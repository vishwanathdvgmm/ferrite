use crate::ast::*;
use crate::errors::Span;
use std::collections::HashMap;

use crate::types::{operator_trait, ImplDef, TraitDef, TraitMethodDef, Type, TypeEnv};

pub struct SemanticAnalyzer<'a, 'b> {
    pub env: &'a mut TypeEnv<'b>,
    module_exports: HashMap<String, Vec<TopDecl>>,
    in_loop: bool,
    in_func: bool,
    in_unsafe: bool,
    current_return_type: Option<Type>,
    /// The type being implemented in the current `impl` block (for resolving `Self`)
    current_self_type: Option<Type>,
}

impl<'a, 'b> SemanticAnalyzer<'a, 'b> {
    pub fn new(env: &'a mut TypeEnv<'b>, module_exports: HashMap<String, Vec<TopDecl>>) -> Self {
        Self {
            env,
            module_exports,
            in_loop: false,
            in_func: false,
            in_unsafe: false,
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
                TopDecl::Func(f) | TopDecl::TestFunc(f) => {
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
                TopDecl::ExternBlock(eb) => {
                    for f in &eb.functions {
                        let ret_ty = match &f.return_type {
                            Some(t) => self.env.resolve_ast_type(t),
                            None => Type::Unit,
                        };
                        let param_tys: Vec<Type> = f
                            .params
                            .iter()
                            .map(|p| self.env.resolve_ast_type(&p.ty))
                            .collect();

                        let func_ty = Type::ExternFunc(param_tys, Box::new(ret_ty));
                        self.env.declare_var(f.name.clone(), func_ty, &f.span);
                    }
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
                TopDecl::Import(_) => {
                    // Imports are resolved at the very end of pass 1 to ensure all types are known
                }
            }
        }

        // Pass 1.5: Build module types for imports
        for decl in &program.decls {
            if let TopDecl::Import(import_decl) = decl {
                match import_decl {
                    ImportDecl::Simple { path, span } => {
                        let module_name_opt = if self.module_exports.contains_key(path) {
                            Some(path.clone())
                        } else if self
                            .module_exports
                            .contains_key(&format!("<stdlib::{}>", path))
                        {
                            Some(format!("<stdlib::{}>", path))
                        } else {
                            None
                        };
                        let module_name = match module_name_opt {
                            Some(name) => name,
                            None => continue,
                        };
                        if let Some(pub_decls) = self.module_exports.get(&module_name) {
                            let mut exports = HashMap::new();
                            for d in pub_decls {
                                if let Some(n) = d.name() {
                                    let ty = self.env.lookup_var(&n, span);
                                    if ty != Type::Error {
                                        exports.insert(n, Box::new(ty));
                                    }
                                }
                            }
                            let mod_ty = Type::Module(module_name.clone(), exports);
                            self.env.declare_var(path.clone(), mod_ty, span);
                        }
                    }
                    ImportDecl::Aliased { name, alias, span } => {
                        let module_name_opt = if self.module_exports.contains_key(name) {
                            Some(name.clone())
                        } else if self
                            .module_exports
                            .contains_key(&format!("<stdlib::{}>", name))
                        {
                            Some(format!("<stdlib::{}>", name))
                        } else {
                            None
                        };
                        let module_name = match module_name_opt {
                            Some(n) => n,
                            None => continue,
                        };
                        if let Some(pub_decls) = self.module_exports.get(&module_name) {
                            let mut exports = HashMap::new();
                            for d in pub_decls {
                                if let Some(n) = d.name() {
                                    let ty = self.env.lookup_var(&n, span);
                                    if ty != Type::Error {
                                        exports.insert(n, Box::new(ty));
                                    }
                                }
                            }
                            let mod_ty = Type::Module(module_name.clone(), exports);
                            self.env.declare_var(alias.clone(), mod_ty, span);
                        }
                    }
                    ImportDecl::Selective { .. } => {}
                }
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
            TopDecl::ExternBlock(_) => {}
            TopDecl::Func(f) | TopDecl::TestFunc(f) => {
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

                let is_top_level = f.name == "__top_level__";
                if !is_top_level {
                    self.env.enter_scope();
                    for param in &f.params {
                        let pty = self.env.resolve_ast_type(&param.ty);
                        self.env.declare_var(param.name.clone(), pty, &param.span);
                    }
                }

                self.analyze_block(&f.body);

                if !is_top_level {
                    self.env.exit_scope();
                }

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

    fn analyze_block(&mut self, block: &Block) -> Type {
        self.env.enter_scope();
        for stmt in &block.stmts {
            self.analyze_stmt(stmt);
        }
        let ty = if let Some(expr) = &block.expr {
            self.analyze_expr(expr)
        } else {
            Type::Unit
        };
        self.env.exit_scope();
        ty
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
            Stmt::ExprStmt(expr, _) => {
                self.analyze_expr(expr);
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
            Expr::Block(block) => self.analyze_block(block),
            Expr::If {
                condition,
                then_block,
                elif_branches,
                else_block,
                span,
            } => {
                let cond_ty = self.analyze_expr(condition);
                self.env.unify(&Type::Bool, &cond_ty, &condition.span());

                let then_ty = self.analyze_block(then_block);
                let mut overall_ty = then_ty;

                for (elif_cond, elif_block) in elif_branches {
                    let elif_cond_ty = self.analyze_expr(elif_cond);
                    self.env
                        .unify(&Type::Bool, &elif_cond_ty, &elif_cond.span());
                    let branch_ty = self.analyze_block(elif_block);
                    self.env.unify(&overall_ty, &branch_ty, &elif_block.span);
                    overall_ty = branch_ty;
                }

                if let Some(else_b) = else_block {
                    let else_ty = self.analyze_block(else_b);
                    self.env.unify(&overall_ty, &else_ty, &else_b.span);
                    overall_ty = else_ty;
                } else {
                    // If there is no else, the overall type MUST be Unit (or Never).
                    self.env.unify(&Type::Unit, &overall_ty, &span);
                    overall_ty = Type::Unit;
                }
                overall_ty
            }
            Expr::While {
                condition, body, ..
            } => {
                let cond_ty = self.analyze_expr(condition);
                self.env.unify(&Type::Bool, &cond_ty, &condition.span());
                let prev = self.in_loop;
                self.in_loop = true;
                self.analyze_block(body);
                self.in_loop = prev;
                Type::Unit
            }
            Expr::For {
                iterable,
                body,
                var,
                ..
            } => {
                let _iter_ty = self.analyze_expr(iterable);
                // Assume array/list for now
                self.env.enter_scope();
                self.env
                    .declare_var(var.clone(), Type::Error, &iterable.span());
                let prev = self.in_loop;
                self.in_loop = true;
                self.analyze_block(body);
                self.in_loop = prev;
                self.env.exit_scope();
                Type::Unit
            }
            Expr::Match {
                subject,
                cases,
                span: _,
            } => {
                let subj_ty = self.analyze_expr(subject);
                let mut overall_ty = Type::Error;

                for (i, case) in cases.iter().enumerate() {
                    self.env.enter_scope();
                    self.analyze_pattern(&case.pattern, &subj_ty);
                    if let Some(guard) = &case.guard {
                        let g_ty = self.analyze_expr(guard);
                        self.env.unify(&Type::Bool, &g_ty, &guard.span());
                    }
                    let branch_ty = self.analyze_block(&case.body);
                    if i == 0 {
                        overall_ty = branch_ty;
                    } else {
                        self.env.unify(&overall_ty, &branch_ty, &case.span);
                        overall_ty = branch_ty;
                    }
                    self.env.exit_scope();
                }
                overall_ty
            }
            Expr::Select { cases: _, span: _ } => {
                Type::Unit // Simplified for now
            }
            Expr::Return { value, span } => {
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
                        self.env.unify(expected, &ret_ty, &span);
                    }
                }
                Type::Never
            }
            Expr::Stop(span) | Expr::Skip(span) => {
                if !self.in_loop {
                    self.env
                        .diag
                        .error(span.clone(), "Cannot use stop or skip outside of a loop.");
                }
                Type::Never
            }
            Expr::InferBlock(b) | Expr::TrainBlock(b) => {
                self.analyze_block(b);
                Type::Unit
            }
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

                        // String concatenation: string + string -> string
                        if *op == BinOp::Add && lty == Type::String && rty == Type::String {
                            return Type::String;
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
                    BinOp::MatMul => {
                        match (&lty, &rty) {
                            (Type::Tensor(l_elem, l_shape), Type::Tensor(r_elem, r_shape)) => {
                                self.env.unify(l_elem, r_elem, span);
                                if l_shape.dims.len() != 2 || r_shape.dims.len() != 2 {
                                    self.env.diag.error(
                                        span.clone(),
                                        format!(
                                            "MatMul (@) requires 2D tensors, got {}D and {}D",
                                            l_shape.dims.len(),
                                            r_shape.dims.len()
                                        ),
                                    );
                                }
                                // Ideally check shape compatibility, but for now we just return a Tensor type.
                                // We can leave the shape dimensions unvalidated here, or compute the output shape.
                                // For simplicity, we just return a Tensor type (if dimensions are known we could compute it).
                                // Return type is Tensor<E, (L.0, R.1)>
                                let out_shape =
                                    if l_shape.dims.len() == 2 && r_shape.dims.len() == 2 {
                                        crate::types::tensor::TensorShape::new(vec![
                                            l_shape.dims[0].clone(),
                                            r_shape.dims[1].clone(),
                                        ])
                                    } else {
                                        l_shape.clone()
                                    };
                                Type::Tensor(l_elem.clone(), out_shape)
                            }
                            _ => {
                                self.env.diag.error(
                                    span.clone(),
                                    format!("MatMul (@) expects Tensors, got {} and {}", lty, rty),
                                );
                                Type::Error
                            }
                        }
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

                let is_extern = matches!(callee_ty, Type::ExternFunc(_, _));
                if is_extern && !self.in_unsafe {
                    self.env.diag.error(
                        span.clone(),
                        "Call to extern function is unsafe and requires an unsafe block"
                            .to_string(),
                    );
                }

                if let Type::Func(param_tys, func_ret_ty)
                | Type::ExternFunc(param_tys, func_ret_ty) = &callee_ty
                {
                    let builtin_name = if let Expr::Ident(name, _) = callee.as_ref() {
                        Some(name.as_str())
                    } else {
                        None
                    };

                    let is_variadic_builtin =
                        matches!(builtin_name, Some("range" | "zeros" | "ones" | "rand"));

                    if !is_variadic_builtin && args.len() != param_tys.len() {
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
                            let expected_ty = if i < param_tys.len() {
                                &param_tys[i]
                            } else if is_variadic_builtin {
                                &Type::Int
                            } else {
                                &Type::Error
                            };
                            self.env.unify_recursive(
                                expected_ty,
                                &arg_ty,
                                &arg.span(),
                                &mut subst,
                                0,
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
                    Type::Module(mod_name, exports) => {
                        if let Some(field_ty) = exports.get(field) {
                            *field_ty.clone()
                        } else {
                            self.env.diag.error(
                                span.clone(),
                                format!("Module '{}' has no public member '{}'.", mod_name, field),
                            );
                            Type::Error
                        }
                    }
                    Type::GenericInst(name, args) if name == "List" && args.len() == 1 => {
                        self.get_list_method(field, &args[0], span)
                    }
                    Type::GenericInst(name, args) if name == "Map" && args.len() == 2 => {
                        self.get_map_method(field, &args[0], &args[1], span)
                    }
                    Type::String => self.get_str_method(field, span),
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
                    Type::String => {
                        self.env.unify(&Type::Int, &idx_ty, span);
                        Type::String
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
                let mut param_types = Vec::new();
                for param in params {
                    let resolved = self.env.resolve_ast_type(&param.ty);
                    param_types.push(resolved.clone());
                    self.env
                        .declare_var(param.name.clone(), resolved, &param.span);
                }

                let prev_ret = self.current_return_type.clone();
                let prev_func = self.in_func;
                self.in_func = true;
                self.current_return_type = None; // Lambda return type is inferred from body

                let body_ty = self.analyze_expr(body);

                self.in_func = prev_func;
                self.current_return_type = prev_ret;

                self.env.exit_scope();

                Type::Func(param_types, Box::new(body_ty))
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
            Expr::UnsafeBlock(block, _span) => {
                let prev_unsafe = self.in_unsafe;
                self.in_unsafe = true;
                self.env.enter_scope();
                self.analyze_block(block);
                self.env.exit_scope();
                self.in_unsafe = prev_unsafe;
                Type::Unit // For now, unsafe blocks evaluate to Unit. Or could be last expr if we support it.
            }
        }
    }

    fn get_list_method(&mut self, method: &str, elem_ty: &Type, span: &Span) -> Type {
        match method {
            "push" => Type::Func(vec![elem_ty.clone()], Box::new(Type::Unit)),
            "pop" => Type::Func(vec![], Box::new(elem_ty.clone())),
            "len" => Type::Func(vec![], Box::new(Type::Int)),
            "contains" => Type::Func(vec![elem_ty.clone()], Box::new(Type::Bool)),
            "remove" => Type::Func(vec![Type::Int], Box::new(elem_ty.clone())),
            "reverse" => Type::Func(vec![], Box::new(Type::Unit)),
            "clear" => Type::Func(vec![], Box::new(Type::Unit)),
            "insert" => Type::Func(vec![Type::Int, elem_ty.clone()], Box::new(Type::Unit)),
            "slice" => Type::Func(
                vec![Type::Int, Type::Int],
                Box::new(Type::GenericInst("List".to_string(), vec![elem_ty.clone()])),
            ),
            _ => {
                self.env
                    .diag
                    .error(span.clone(), format!("List has no method '{}'", method));
                Type::Error
            }
        }
    }

    fn get_map_method(&mut self, method: &str, key_ty: &Type, val_ty: &Type, span: &Span) -> Type {
        match method {
            "set" => Type::Func(vec![key_ty.clone(), val_ty.clone()], Box::new(Type::Unit)),
            "get" => Type::Func(vec![key_ty.clone()], Box::new(val_ty.clone())),
            "contains" => Type::Func(vec![key_ty.clone()], Box::new(Type::Bool)),
            "remove" => Type::Func(vec![key_ty.clone()], Box::new(val_ty.clone())),
            "keys" => Type::Func(
                vec![],
                Box::new(Type::GenericInst("List".to_string(), vec![key_ty.clone()])),
            ),
            "values" => Type::Func(
                vec![],
                Box::new(Type::GenericInst("List".to_string(), vec![val_ty.clone()])),
            ),
            "len" => Type::Func(vec![], Box::new(Type::Int)),
            _ => {
                self.env
                    .diag
                    .error(span.clone(), format!("Map has no method '{}'", method));
                Type::Error
            }
        }
    }

    fn get_str_method(&mut self, method: &str, span: &Span) -> Type {
        match method {
            "len" => Type::Func(vec![], Box::new(Type::Int)),
            "charAt" => Type::Func(vec![Type::Int], Box::new(Type::String)),
            "substring" => Type::Func(vec![Type::Int, Type::Int], Box::new(Type::String)),
            "split" => Type::Func(
                vec![Type::String],
                Box::new(Type::GenericInst("List".to_string(), vec![Type::String])),
            ),
            "contains" => Type::Func(vec![Type::String], Box::new(Type::Bool)),
            "replace" => Type::Func(vec![Type::String, Type::String], Box::new(Type::String)),
            "trim" => Type::Func(vec![], Box::new(Type::String)),
            "upper" => Type::Func(vec![], Box::new(Type::String)),
            "lower" => Type::Func(vec![], Box::new(Type::String)),
            "startsWith" => Type::Func(vec![Type::String], Box::new(Type::Bool)),
            "endsWith" => Type::Func(vec![Type::String], Box::new(Type::Bool)),
            _ => {
                self.env
                    .diag
                    .error(span.clone(), format!("String has no method '{}'", method));
                Type::Error
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
        BinOp::MatMul => "@",
    }
}

// ── Unit Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::DiagnosticBag;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Helper to parse and type-check a source string, returning error messages.
    fn check_source(source: &str) -> Vec<String> {
        let mut diag = DiagnosticBag::new();
        let mut lexer = Lexer::new(source, PathBuf::from("<test>"));
        let tokens = lexer.tokenize(&mut diag);

        let mut parser = Parser::new(tokens, &mut diag);
        let program = parser.parse_program();

        if !diag.has_errors() {
            let mut type_env = TypeEnv::new(&mut diag);
            let mut semantic = SemanticAnalyzer::new(&mut type_env, HashMap::new());
            semantic.analyze_program(&program);
        }

        diag.errors().iter().map(|e| e.message.clone()).collect()
    }

    #[test]
    fn test_valid_program() {
        let source = r#"
            keep x: int = 42;
            keep y: float = 3.14;
            keep z: bool = true;
            keep s: string = "hello";
        "#;
        let errors = check_source(source);
        assert!(
            errors.is_empty(),
            "Valid program should have no errors, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_type_mismatch() {
        let source = r#"keep x: int = "hello";"#;
        let errors = check_source(source);
        assert!(!errors.is_empty(), "Expected type mismatch error");
        assert!(
            errors[0].contains("Type mismatch"),
            "Expected Type mismatch error, got: {}",
            errors[0]
        );
    }

    #[test]
    fn test_undefined_variable() {
        let source = r#"keep x: int = y;"#;
        let errors = check_source(source);
        assert!(!errors.is_empty(), "Expected undefined variable error");
        assert!(
            errors[0].contains("Undefined variable"),
            "Expected Undefined variable error, got: {}",
            errors[0]
        );
    }

    #[test]
    fn test_function_arity() {
        let source = r#"
            fun add(a: int, b: int) -> int { return a + b; }
            fun main() { add(1); }
        "#;
        let errors = check_source(source);
        assert!(!errors.is_empty(), "Expected arity error");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("Function expects 2 arguments")),
            "Expected argument count error"
        );
    }

    #[test]
    fn test_return_type_check() {
        let source = r#"
            fun f() -> int { return "x"; }
        "#;
        let errors = check_source(source);
        assert!(!errors.is_empty(), "Expected return type error");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("Type mismatch") || e.contains("return type")),
            "Expected return type error"
        );
    }

    #[test]
    fn test_list_index_type() {
        let source = r#"
            fun f() {
                keep l: List<int> = List(1, 2, 3);
                l["key"];
            }
        "#;
        let errors = check_source(source);
        assert!(!errors.is_empty(), "Expected index type error");
        assert!(
            errors.iter().any(|e| e.contains("Type mismatch")),
            "Expected type mismatch on index"
        );
    }
}
