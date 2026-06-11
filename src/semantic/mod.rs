use crate::ast::*;

use crate::types::{operator_trait, ImplDef, TraitDef, TraitMethodDef, Type, TypeEnv};

pub struct SemanticAnalyzer<'a, 'b> {
    env: &'b mut TypeEnv<'a>,
    in_loop: bool,
    in_func: bool,
    current_return_type: Option<Type>,
    /// The type being implemented in the current `impl` block (for resolving `Self`)
    current_self_type: Option<Type>,
}

impl<'a, 'b> SemanticAnalyzer<'a, 'b> {
    pub fn new(env: &'b mut TypeEnv<'a>) -> Self {
        Self {
            env,
            in_loop: false,
            in_func: false,
            current_return_type: None,
            current_self_type: None,
        }
    }

    pub fn analyze_program(&mut self, program: &Program) {
        // Pass 1: Declare all top-level types (Groups, Enums), Functions, Traits, Impls
        for decl in &program.decls {
            match decl {
                TopDecl::Group(g) => {
                    self.env
                        .declare_type(g.name.clone(), Type::Named(g.name.clone()), &g.span);

                    // Register group fields for field access resolution
                    let fields: Vec<(String, Type)> = g
                        .fields
                        .iter()
                        .map(|f| {
                            let ty = self.env.resolve_ast_type(&f.ty);
                            (f.name.clone(), ty)
                        })
                        .collect();
                    self.env.register_group_fields(g.name.clone(), fields);

                    // Register inline group methods as impl methods
                    for method in &g.methods {
                        let ret_ty = match &method.return_type {
                            Some(t) => self.env.resolve_ast_type(t),
                            None => Type::Unit,
                        };
                        let mut param_tys: Vec<Type> = method
                            .params
                            .iter()
                            .map(|p| self.env.resolve_ast_type(&p.ty))
                            .collect();
                        if method.has_self {
                            param_tys.insert(0, Type::Named(g.name.clone()));
                        }
                        let func_ty = Type::Func(param_tys, Box::new(ret_ty));
                        // Register as a global function for now (simple dispatch)
                        self.env
                            .declare_var(method.name.clone(), func_ty, &method.span);
                    }
                }
                TopDecl::Enum(e) => {
                    self.env
                        .declare_type(e.name.clone(), Type::Named(e.name.clone()), &e.span);

                    // Register enum variants
                    let variants: Vec<(String, Vec<Type>)> = e
                        .variants
                        .iter()
                        .map(|v| {
                            let field_tys: Vec<Type> = v
                                .fields
                                .iter()
                                .map(|t| self.env.resolve_ast_type(t))
                                .collect();
                            (v.name.clone(), field_tys)
                        })
                        .collect();
                    self.env
                        .register_enum_variants(e.name.clone(), variants.clone());

                    // Register each variant as a constructor function
                    for (vname, vtypes) in &variants {
                        if vtypes.is_empty() {
                            // Unit variant — register as a named constant
                            self.env.declare_var(
                                vname.clone(),
                                Type::Named(e.name.clone()),
                                &e.span,
                            );
                        } else {
                            // Constructor variant — register as a function
                            let func_ty =
                                Type::Func(vtypes.clone(), Box::new(Type::Named(e.name.clone())));
                            self.env.declare_var(vname.clone(), func_ty, &e.span);
                        }
                    }
                }
                TopDecl::Constant(c) => {
                    let ty = self.env.resolve_ast_type(&c.ty);
                    self.env.declare_var(c.name.clone(), ty, &c.span);
                }
                TopDecl::Func(f) => {
                    // Register the function name in the variable scope with its parameter types.
                    let generic_names: Vec<String> = f
                        .generics
                        .iter()
                        .map(|g| match g {
                            GenericParam::Type { name, .. } => name.clone(),
                            GenericParam::Shape { name, .. } => name.clone(),
                            GenericParam::Bounded { name, .. } => name.clone(),
                        })
                        .collect();

                    self.env.push_generics(generic_names.clone());

                    let ret_ty = match &f.return_type {
                        Some(t) => self.env.resolve_ast_type(t),
                        None => Type::Unit,
                    };
                    let param_tys: Vec<Type> = f
                        .params
                        .iter()
                        .map(|p| self.env.resolve_ast_type(&p.ty))
                        .collect();

                    self.env.pop_generics(generic_names.len());

                    let func_ty = Type::Func(param_tys, Box::new(ret_ty));
                    self.env.declare_var(f.name.clone(), func_ty, &f.span);
                }
                TopDecl::Trait(t) => {
                    // Register trait definition
                    let method_sigs: Vec<TraitMethodDef> = t
                        .methods
                        .iter()
                        .map(|m| {
                            let param_types: Vec<Type> = m
                                .params
                                .iter()
                                .map(|p| self.env.resolve_ast_type(&p.ty))
                                .collect();
                            let return_type = match &m.return_type {
                                Some(ty) => self.env.resolve_ast_type(ty),
                                None => Type::Unit,
                            };
                            TraitMethodDef {
                                name: m.name.clone(),
                                param_types,
                                return_type,
                                has_self: m.has_self,
                            }
                        })
                        .collect();
                    let trait_def = TraitDef {
                        name: t.name.clone(),
                        method_sigs,
                    };
                    self.env.register_trait(t.name.clone(), trait_def, &t.span);
                }
                TopDecl::Impl(imp) => {
                    // Register impl definition — validate trait methods are present
                    let method_names: Vec<String> =
                        imp.methods.iter().map(|m| m.name.clone()).collect();
                    let impl_def = ImplDef {
                        trait_name: imp.trait_name.clone(),
                        target_type: imp.target_type.clone(),
                        method_names,
                    };
                    self.env.register_impl(impl_def, &imp.span);

                    // Register impl methods as callable functions
                    let self_type = Type::Named(imp.target_type.clone());
                    for method in &imp.methods {
                        let ret_ty = match &method.return_type {
                            Some(t) => {
                                let resolved = self.env.resolve_ast_type(t);
                                resolved.resolve_self(&self_type)
                            }
                            None => Type::Unit,
                        };
                        let mut param_tys: Vec<Type> = method
                            .params
                            .iter()
                            .map(|p| {
                                let resolved = self.env.resolve_ast_type(&p.ty);
                                resolved.resolve_self(&self_type)
                            })
                            .collect();
                        if method.has_self {
                            param_tys.insert(0, self_type.clone());
                        }
                        let func_ty = Type::Func(param_tys, Box::new(ret_ty));
                        self.env
                            .declare_var(method.name.clone(), func_ty, &method.span);
                    }
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

                let generic_names: Vec<String> = f
                    .generics
                    .iter()
                    .map(|g| match g {
                        GenericParam::Type { name, .. } => name.clone(),
                        GenericParam::Shape { name, .. } => name.clone(),
                        GenericParam::Bounded { name, .. } => name.clone(),
                    })
                    .collect();
                self.env.push_generics(generic_names.clone());

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

                self.env.pop_generics(generic_names.len());
                self.in_func = prev_func;
                self.current_return_type = prev_ret;
            }
            TopDecl::Trait(_) => {
                // Trait method signatures are validated during registration.
                // No bodies to analyze.
            }
            TopDecl::Impl(imp) => {
                // Analyze each impl method body
                let prev_self = self.current_self_type.clone();
                self.current_self_type = Some(Type::Named(imp.target_type.clone()));

                self.env.enter_scope();
                for method in &imp.methods {
                    self.analyze_method(method, &imp.target_type);
                }
                self.env.exit_scope();

                self.current_self_type = prev_self;
            }
        }
    }

    fn analyze_method(&mut self, method: &MethodDecl, parent_name: &str) {
        let prev_func = self.in_func;
        let prev_ret = self.current_return_type.clone();
        let prev_self = self.current_self_type.clone();
        self.in_func = true;
        self.current_self_type = Some(Type::Named(parent_name.to_string()));

        let self_type = Type::Named(parent_name.to_string());
        self.current_return_type = match &method.return_type {
            Some(t) => {
                let resolved = self.env.resolve_ast_type(t);
                Some(resolved.resolve_self(&self_type))
            }
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
            let resolved = pty.resolve_self(&self_type);
            self.env
                .declare_var(param.name.clone(), resolved, &param.span);
        }

        self.analyze_block(&method.body);
        self.env.exit_scope();

        self.in_func = prev_func;
        self.current_return_type = prev_ret;
        self.current_self_type = prev_self;
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
                span,
            } => {
                let subject_ty = self.analyze_expr(subject);

                // Exhaustiveness checking for enum types
                self.check_match_exhaustiveness(&subject_ty, cases, span);

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

    /// Check if a match statement on an enum type is exhaustive.
    /// Emits a warning (not error) if variants are missing and no wildcard is present.
    fn check_match_exhaustiveness(
        &mut self,
        subject_ty: &Type,
        cases: &[MatchCase],
        span: &crate::errors::Span,
    ) {
        let type_name = match subject_ty {
            Type::Named(name) => name,
            _ => return, // Only check enums
        };

        let variants = match self.env.enum_variants.get(type_name) {
            Some(v) => v.clone(),
            None => return, // Not an enum type — could be a group, skip
        };

        // Check if there's a wildcard/default pattern
        let has_wildcard = cases
            .iter()
            .any(|c| matches!(&c.pattern, Pattern::Wildcard(_)));
        if has_wildcard {
            return; // Wildcard covers everything
        }

        // Check if there's a catch-all binding (single variable without constructor)
        let has_binding_catchall = cases
            .iter()
            .any(|c| matches!(&c.pattern, Pattern::Binding(_, _)));
        if has_binding_catchall {
            return; // A binding pattern catches everything
        }

        // Collect matched variant names
        let matched_variants: Vec<&str> = cases
            .iter()
            .filter_map(|c| match &c.pattern {
                Pattern::Constructor { name, .. } => Some(name.as_str()),
                Pattern::Binding(name, _) => {
                    // Check if this binding name is actually a unit variant
                    if variants.iter().any(|(vn, vf)| vn == name && vf.is_empty()) {
                        Some(name.as_str())
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        // Find missing variants
        let missing: Vec<&String> = variants
            .iter()
            .filter(|(vname, _)| !matched_variants.contains(&vname.as_str()))
            .map(|(vname, _)| vname)
            .collect();

        if !missing.is_empty() {
            let missing_list = missing
                .iter()
                .map(|s| format!("'{}'", s))
                .collect::<Vec<_>>()
                .join(", ");
            self.env.diag.warning(
                span.clone(),
                format!(
                    "Non-exhaustive match on enum '{}'. Missing variants: {}. \
                     Consider adding a 'default' case.",
                    type_name, missing_list
                ),
            );
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
            Pattern::Constructor { name, fields, span } => {
                // Validate variant exists and bind inner fields
                let mut found = false;
                // Find which enum this variant belongs to
                for (enum_name, variants) in &self.env.enum_variants.clone() {
                    for (vname, vtypes) in variants {
                        if vname == name {
                            found = true;
                            // Verify subject type matches enum
                            self.env
                                .unify(&Type::Named(enum_name.clone()), subject_ty, span);
                            // Verify field count
                            if fields.len() != vtypes.len() {
                                self.env.diag.error(
                                    span.clone(),
                                    format!(
                                        "Variant '{}' expects {} field(s), but pattern has {}.",
                                        name,
                                        vtypes.len(),
                                        fields.len()
                                    ),
                                );
                            } else {
                                // Recursively analyze sub-patterns
                                for (i, field_pat) in fields.iter().enumerate() {
                                    self.analyze_pattern(field_pat, &vtypes[i]);
                                }
                            }
                            break;
                        }
                    }
                    if found {
                        break;
                    }
                }
                if !found {
                    self.env
                        .diag
                        .error(span.clone(), format!("Unknown enum variant '{}'.", name));
                }
            }
            Pattern::Struct { name, fields, span } => {
                // Validate struct fields exist
                self.env.unify(&Type::Named(name.clone()), subject_ty, span);
                if let Some(group_fields) = self.env.group_fields.get(name).cloned() {
                    for (fname, fpat) in fields {
                        let field_ty = group_fields
                            .iter()
                            .find(|(n, _)| n == fname)
                            .map(|(_, t)| t.clone())
                            .unwrap_or_else(|| {
                                self.env.diag.error(
                                    span.clone(),
                                    format!("Group '{}' has no field '{}'.", name, fname),
                                );
                                Type::Error
                            });
                        self.analyze_pattern(fpat, &field_ty);
                    }
                }
            }
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

                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        // Check for operator overloading on user-defined types
                        if let Type::Named(type_name) = &lty {
                            if let Some(trait_name) = operator_trait(op) {
                                if self.env.has_trait_impl(type_name, trait_name) {
                                    // Operator is valid via trait — ensure both operands match
                                    self.env.unify(&lty, &rty, span);
                                    return lty;
                                } else {
                                    self.env.diag.error(
                                        span.clone(),
                                        format!(
                                            "Type '{}' does not implement trait '{}' required for operator '{}'.",
                                            type_name, trait_name, op_symbol(op)
                                        ),
                                    );
                                    return Type::Error;
                                }
                            }
                        }

                        // Standard numeric operation
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
            Expr::Call { callee, args, span } => {
                let callee_ty = self.analyze_expr(callee);
                let mut ret_ty = Type::Error;

                if let Type::Func(param_tys, func_ret_ty) = &callee_ty {
                    if args.len() != param_tys.len() {
                        self.env.diag.error(
                            span.clone(),
                            format!(
                                "Function expects {} arguments, but got {}",
                                param_tys.len(),
                                args.len()
                            ),
                        );
                        for arg in args {
                            self.analyze_expr(arg);
                        }
                    } else {
                        let mut subst = std::collections::HashMap::new();
                        for (i, arg) in args.iter().enumerate() {
                            let arg_ty = self.analyze_expr(arg);
                            self.env.unify_recursive(
                                &param_tys[i],
                                &arg_ty,
                                &arg.span(),
                                &mut subst,
                            );
                        }
                        ret_ty = func_ret_ty.substitute(&subst);
                    }
                } else if callee_ty != Type::Error {
                    self.env.diag.error(
                        span.clone(),
                        format!("Cannot call non-function type '{}'", callee_ty),
                    );
                    for arg in args {
                        self.analyze_expr(arg);
                    }
                } else {
                    for arg in args {
                        self.analyze_expr(arg);
                    }
                }
                ret_ty
            }
            Expr::FieldAccess {
                object,
                field,
                span,
            } => {
                let obj_ty = self.analyze_expr(object);

                match &obj_ty {
                    Type::Named(type_name) => {
                        // Look up field in registered group fields
                        if let Some(field_ty) = self.env.lookup_field(type_name, field) {
                            field_ty
                        } else {
                            // Field not found — might be a method (methods are registered as globals)
                            // For now, report field not found
                            if self.env.group_fields.contains_key(type_name) {
                                self.env.diag.error(
                                    span.clone(),
                                    format!("Group '{}' has no field '{}'.", type_name, field),
                                );
                            }
                            Type::Error
                        }
                    }
                    Type::Error => Type::Error,
                    _ => {
                        self.env.diag.error(
                            span.clone(),
                            format!("Cannot access field '{}' on type '{}'.", field, obj_ty),
                        );
                        Type::Error
                    }
                }
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
                    Type::Tensor(elem, _) => {
                        self.env.unify(&Type::Int, &idx_ty, span);
                        *elem
                    }
                    Type::GenericInst(name, args) if name == "Map" && args.len() == 2 => {
                        self.env.unify(&args[0], &idx_ty, span);
                        args[1].clone()
                    }
                    Type::GenericInst(name, args) if name == "List" && args.len() == 1 => {
                        self.env.unify(&Type::Int, &idx_ty, span);
                        args[0].clone()
                    }
                    _ => {
                        if obj_ty != Type::Error {
                            self.env.diag.error(
                                span.clone(),
                                format!("Indexing not supported for type '{}'", obj_ty),
                            );
                        }
                        Type::Error
                    }
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

/// Helper to get the human-readable operator symbol
fn op_symbol(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::NotEq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}
