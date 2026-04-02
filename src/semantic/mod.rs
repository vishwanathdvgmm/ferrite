use crate::ast::*;

use crate::types::{Type, TypeEnv};

pub struct SemanticAnalyzer<'a, 'b> {
    env: &'b mut TypeEnv<'a>,
    in_loop: bool,
    in_func: bool,
    current_return_type: Option<Type>,
}

impl<'a, 'b> SemanticAnalyzer<'a, 'b> {
    pub fn new(env: &'b mut TypeEnv<'a>) -> Self {
        Self {
            env,
            in_loop: false,
            in_func: false,
            current_return_type: None,
        }
    }

    pub fn analyze_program(&mut self, program: &Program) {
        // Pass 1: Declare all top-level types (Groups, Enums) and Functions
        for decl in &program.decls {
            match decl {
                TopDecl::Group(g) => {
                    self.env
                        .declare_type(g.name.clone(), Type::Named(g.name.clone()), &g.span);
                }
                TopDecl::Enum(e) => {
                    self.env
                        .declare_type(e.name.clone(), Type::Named(e.name.clone()), &e.span);
                }
                TopDecl::Constant(c) => {
                    let ty = self.env.resolve_ast_type(&c.ty);
                    self.env.declare_var(c.name.clone(), ty, &c.span);
                }
                TopDecl::Func(f) => {
                    // Register the function name in the variable scope so calls can resolve.
                    // In Phase 1, we use Type::Error as a stub for the function's type
                    // since we don't have full function pointer types yet.
                    let ret_ty = match &f.return_type {
                        Some(t) => self.env.resolve_ast_type(t),
                        None => Type::Unit,
                    };
                    self.env.declare_var(f.name.clone(), ret_ty, &f.span);
                }
                TopDecl::Import(_) => {}
            }
        }

        // Pass 2: Analyze bodies
        for decl in &program.decls {
            self.analyze_decl(decl);
        }
    }

    fn analyze_decl(&mut self, decl: &TopDecl) {
        match decl {
            TopDecl::Import(_) => {}
            TopDecl::Constant(c) => {
                let expr_ty = self.analyze_expr(&c.value);
                let decl_ty = self.env.resolve_ast_type(&c.ty);
                self.env.unify(&decl_ty, &expr_ty, &c.span);
            }
            TopDecl::Group(g) => {
                self.env.enter_scope();
                for method in &g.methods {
                    self.analyze_method(method, &g.name);
                }
                self.env.exit_scope();
            }
            TopDecl::Enum(_) => {}
            TopDecl::Func(f) => {
                let prev_func = self.in_func;
                let prev_ret = self.current_return_type.clone();
                self.in_func = true;

                self.current_return_type = match &f.return_type {
                    Some(t) => Some(self.env.resolve_ast_type(t)),
                    None => Some(Type::Unit),
                };

                self.env.enter_scope();
                for param in &f.params {
                    let pty = self.env.resolve_ast_type(&param.ty);
                    self.env.declare_var(param.name.clone(), pty, &param.span);
                }

                self.analyze_block(&f.body);
                self.env.exit_scope();

                self.in_func = prev_func;
                self.current_return_type = prev_ret;
            }
        }
    }

    fn analyze_method(&mut self, method: &MethodDecl, parent_name: &str) {
        let prev_func = self.in_func;
        let prev_ret = self.current_return_type.clone();
        self.in_func = true;

        self.current_return_type = match &method.return_type {
            Some(t) => Some(self.env.resolve_ast_type(t)),
            None => Some(Type::Unit),
        };

        self.env.enter_scope();
        if method.has_self {
            self.env.declare_var(
                "self".to_string(),
                Type::Named(parent_name.to_string()),
                &method.span,
            );
        }
        for param in &method.params {
            let pty = self.env.resolve_ast_type(&param.ty);
            self.env.declare_var(param.name.clone(), pty, &param.span);
        }

        self.analyze_block(&method.body);
        self.env.exit_scope();

        self.in_func = prev_func;
        self.current_return_type = prev_ret;
    }

    fn analyze_block(&mut self, block: &Block) {
        self.env.enter_scope();
        for stmt in &block.stmts {
            self.analyze_stmt(stmt);
        }
        self.env.exit_scope();
    }

    fn analyze_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Keep {
                name,
                ty,
                value,
                span,
            } => {
                let expr_ty = self.analyze_expr(value);
                let decl_ty = self.env.resolve_ast_type(ty);
                self.env.unify(&decl_ty, &expr_ty, span);
                self.env.declare_var(name.clone(), decl_ty, span);
            }
            Stmt::Param {
                name,
                ty,
                value,
                span,
            } => {
                let expr_ty = self.analyze_expr(value);
                let decl_ty = self.env.resolve_ast_type(ty);
                self.env.unify(&decl_ty, &expr_ty, span);
                self.env.declare_var(name.clone(), decl_ty, span);
            }
            Stmt::ExprStmt(expr) => {
                self.analyze_expr(expr);
            }
            Stmt::Return { value, span } => {
                if !self.in_func {
                    self.env
                        .diag
                        .error(span.clone(), "Cannot return outside of a function.");
                } else {
                    let ret_ty = value
                        .as_ref()
                        .map(|e| self.analyze_expr(e))
                        .unwrap_or(Type::Unit);
                    if let Some(expected) = &self.current_return_type {
                        self.env.unify(expected, &ret_ty, span);
                    }
                }
            }
            Stmt::If {
                condition,
                then_block,
                elif_branches,
                else_block,
                span: _,
            } => {
                let cond_ty = self.analyze_expr(condition);
                self.env.unify(&Type::Bool, &cond_ty, &condition.span());
                self.analyze_block(then_block);
                for (cond, blk) in elif_branches {
                    let ct = self.analyze_expr(cond);
                    self.env.unify(&Type::Bool, &ct, &cond.span());
                    self.analyze_block(blk);
                }
                if let Some(blk) = else_block {
                    self.analyze_block(blk);
                }
            }
            Stmt::While {
                condition,
                body,
                span: _,
            } => {
                let cond_ty = self.analyze_expr(condition);
                self.env.unify(&Type::Bool, &cond_ty, &condition.span());

                let prev_loop = self.in_loop;
                self.in_loop = true;
                self.analyze_block(body);
                self.in_loop = prev_loop;
            }
            Stmt::For {
                var,
                iterable,
                body,
                span,
            } => {
                // Iteration logic checks can go here
                let _iter_ty = self.analyze_expr(iterable);

                let prev_loop = self.in_loop;
                self.in_loop = true;

                self.env.enter_scope();
                self.env.declare_var(var.clone(), Type::Error, span); // stub until traits are fully evaluated
                self.analyze_block(body);
                self.env.exit_scope();

                self.in_loop = prev_loop;
            }
            Stmt::Match {
                subject,
                cases,
                span: _,
            } => {
                let subject_ty = self.analyze_expr(subject);
                for case in cases {
                    self.env.enter_scope();
                    self.analyze_pattern(&case.pattern, &subject_ty);
                    self.analyze_block(&case.body);
                    self.env.exit_scope();
                }
            }
            Stmt::Select { cases, span: _ } => {
                for case in cases {
                    self.env.enter_scope();
                    if let Some((name, expr)) = &case.assignment {
                        let ty = self.analyze_expr(expr);
                        if name != "_" {
                            self.env.declare_var(name.clone(), ty, &expr.span());
                        }
                    }
                    self.analyze_block(&case.body);
                    self.env.exit_scope();
                }
            }
            Stmt::InferBlock(block) | Stmt::TrainBlock(block) => {
                self.analyze_block(block);
            }
            Stmt::Stop(span) | Stmt::Skip(span) => {
                if !self.in_loop {
                    self.env.diag.error(
                        span.clone(),
                        "Cannot break/continue ('stop'/'skip') outside of a loop.",
                    );
                }
            }
        }
    }

    fn analyze_pattern(&mut self, pat: &Pattern, subject_ty: &Type) {
        match pat {
            Pattern::Literal(lit) => {
                let lit_ty = match lit {
                    Literal::Int(_) => Type::Int,
                    Literal::Float(_) => Type::Float,
                    Literal::Bool(_) => Type::Bool,
                    Literal::String(_) => Type::String,
                };
                self.env.unify(&lit_ty, subject_ty, &pat.span());
            }
            Pattern::Wildcard(_) => {}
            Pattern::Binding(name, span) => {
                // Create variable for the match
                self.env.declare_var(name.clone(), subject_ty.clone(), span);
            }
            Pattern::Constructor { .. } => {} // Validate variant exists
            Pattern::Struct { .. } => {}      // Validate struct fields
        }
    }

    fn analyze_expr(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Lit(lit, _) => match lit {
                Literal::Int(_) => Type::Int,
                Literal::Float(_) => Type::Float,
                Literal::Bool(_) => Type::Bool,
                Literal::String(_) => Type::String,
            },
            Expr::Ident(name, span) => self.env.lookup_var(name, span),
            Expr::BinOp {
                left,
                op,
                right,
                span,
            } => {
                let lty = self.analyze_expr(left);
                let rty = self.analyze_expr(right);

                // No introspection rules!
                // e.g., typeof(x) isn't in grammar, but if it were we'd reject it.
                // Binary ops verify identically matching primitives unless operator is overloaded.

                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        // Numeric operation requires uniformity or trait implementations
                        if lty == Type::Float || rty == Type::Float {
                            self.env.unify(&Type::Float, &lty, span);
                            self.env.unify(&Type::Float, &rty, span);
                            Type::Float
                        } else {
                            self.env.unify(&Type::Int, &lty, span);
                            self.env.unify(&Type::Int, &rty, span);
                            Type::Int
                        }
                    }
                    BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                        // For phase 1, assume identical types
                        self.env.unify(&lty, &rty, span);
                        Type::Bool
                    }
                    BinOp::Eq | BinOp::NotEq => {
                        self.env.unify(&lty, &rty, span);
                        Type::Bool
                    }
                    BinOp::And | BinOp::Or => {
                        self.env.unify(&Type::Bool, &lty, span);
                        self.env.unify(&Type::Bool, &rty, span);
                        Type::Bool
                    }
                }
            }
            Expr::UnaryOp { op, operand, span } => {
                let ty = self.analyze_expr(operand);
                match op {
                    UnaryOp::Neg => {
                        if ty != Type::Float && ty != Type::Int {
                            self.env
                                .diag
                                .error(span.clone(), "Negation requires a numeric type.");
                        }
                        ty
                    }
                    UnaryOp::Not => {
                        self.env.unify(&Type::Bool, &ty, span);
                        Type::Bool
                    }
                    UnaryOp::Await => ty, // Extract inner type from future/async
                }
            }
            Expr::Call {
                callee,
                args,
                span: _,
            } => {
                let _callee_ty = self.analyze_expr(callee);
                // Would normally check that callee_ty is a Function and unify args
                for arg in args {
                    self.analyze_expr(arg);
                }
                // Return type is loosely checked for phase 1 via assignments usually
                Type::Error // Stub for arbitrary function return for now
            }
            Expr::FieldAccess {
                object,
                field: _,
                span: _,
            } => {
                let _obj_ty = self.analyze_expr(object);
                // Look up field in struct definition
                Type::Error
            }
            Expr::IndexAccess {
                object,
                index,
                span,
            } => {
                let obj_ty = self.analyze_expr(object);
                let idx_ty = self.analyze_expr(index);
                self.env.unify(&Type::Int, &idx_ty, span); // indices must be ints

                match obj_ty {
                    Type::Tensor(elem, _) => *elem,
                    _ => Type::Error,
                }
            }
            Expr::Lambda {
                params,
                body,
                span: _,
            } => {
                self.env.enter_scope();
                for param in params {
                    let resolved = self.env.resolve_ast_type(&param.ty);
                    self.env
                        .declare_var(param.name.clone(), resolved, &param.span);
                }

                let prev_ret = self.current_return_type.clone();
                let prev_func = self.in_func;
                self.in_func = true;
                self.current_return_type = Some(Type::Error); // Stub lambda return type inference

                let _body_ty = self.analyze_expr(body);

                self.in_func = prev_func;
                self.current_return_type = prev_ret;

                self.env.exit_scope();

                Type::Error // Needs Function trait type mapping
            }
            Expr::GroupLiteral {
                name,
                fields,
                span: _,
            } => {
                for (_, expr) in fields {
                    self.analyze_expr(expr);
                }
                Type::Named(name.clone())
            }
            Expr::Assign {
                target,
                value,
                span,
            } => {
                let target_ty = self.analyze_expr(target);
                let val_ty = self.analyze_expr(value);
                self.env.unify(&target_ty, &val_ty, span);
                val_ty
            }
        }
    }
}
