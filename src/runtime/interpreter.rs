use super::environment::Environment;
use super::value::Value;
use crate::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Interpreter {
    pub env: Environment,
    pub module_exports: HashMap<String, Vec<TopDecl>>,
}

impl Interpreter {
    pub fn new(module_exports: HashMap<String, Vec<TopDecl>>) -> Self {
        let mut env = Environment::new();
        // Register builtins
        env.declare("print".to_string(), Value::Builtin("print".to_string()));
        env.declare("println".to_string(), Value::Builtin("println".to_string()));
        env.declare("input".to_string(), Value::Builtin("input".to_string()));
        env.declare("len".to_string(), Value::Builtin("len".to_string()));
        env.declare("str".to_string(), Value::Builtin("str".to_string()));
        env.declare("int".to_string(), Value::Builtin("int".to_string()));
        env.declare("float".to_string(), Value::Builtin("float".to_string()));
        env.declare("assert".to_string(), Value::Builtin("assert".to_string()));
        env.declare("exit".to_string(), Value::Builtin("exit".to_string()));
        env.declare("List".to_string(), Value::Builtin("List".to_string()));
        env.declare("Map".to_string(), Value::Builtin("Map".to_string()));
        env.declare("range".to_string(), Value::Builtin("range".to_string()));
        env.declare("zeros".to_string(), Value::Builtin("zeros".to_string()));
        env.declare("ones".to_string(), Value::Builtin("ones".to_string()));
        env.declare("rand".to_string(), Value::Builtin("rand".to_string()));

        Self {
            env,
            module_exports,
        }
    }

    pub fn run_program(&mut self, program: &Program) -> Result<Value, String> {
        // First pass: register all functions, constants, etc.
        for decl in &program.decls {
            match decl {
                TopDecl::Func(f) => {
                    self.env
                        .declare(f.name.clone(), Value::Func(Rc::new(f.clone())));
                }
                TopDecl::Constant(c) => {
                    let val = self.eval_expr(&c.value)?;
                    self.env.declare(c.name.clone(), val);
                }
                TopDecl::Import(_) => {
                    // Imports are resolved in pass 1.5
                }
                TopDecl::ExternBlock(eb) => {
                    for f in &eb.functions {
                        self.env
                            .declare(f.name.clone(), Value::Builtin(f.name.clone()));
                    }
                }
                TopDecl::Impl(imp) => {
                    for m in &imp.methods {
                        // Store the target type name alongside the method for self-dispatch.
                        let qualified_name = format!("{}::{}", imp.target_type, m.name);
                        let mut fdecl = FuncDecl {
                            visibility: Visibility::Public,
                            effect_params: m.effects.iter().map(|_| "".to_string()).collect(),
                            effects: m.effects.clone(),
                            name: m.name.clone(),
                            generics: vec![],
                            params: m.params.clone(),
                            return_effects: m.return_effects.clone(),
                            return_type: m.return_type.clone(),
                            where_clause: m.where_clause.clone(),
                            body: m.body.clone(),
                            span: m.span.clone(),
                        };
                        if m.has_self {
                            fdecl.params.insert(
                                0,
                                Param {
                                    name: "self".to_string(),
                                    ty: Type::Named(imp.target_type.clone(), m.span.clone()),
                                    span: m.span.clone(),
                                },
                            );
                        }
                        // Register both by simple name and qualified name for dispatch
                        self.env
                            .declare(m.name.clone(), Value::Func(Rc::new(fdecl.clone())));
                        self.env
                            .declare(qualified_name, Value::Func(Rc::new(fdecl)));
                    }
                }
                TopDecl::Enum(e) => {
                    for v in &e.variants {
                        // Register enum variant constructors
                        self.env.declare(
                            v.name.clone(),
                            Value::Builtin(format!("enum_{}::{}", e.name, v.name)),
                        );
                    }
                }
                _ => {}
            }
        }

        // Pass 1.5: Build module values for imports
        for decl in &program.decls {
            if let TopDecl::Import(import_decl) = decl {
                match import_decl {
                    ImportDecl::Simple { path, .. } => {
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
                                    if let Ok(val) = self.env.get(&n) {
                                        exports.insert(n, val.clone());
                                    }
                                }
                            }
                            self.env
                                .declare(path.clone(), Value::Module(module_name.clone(), exports));
                        }
                    }
                    ImportDecl::Aliased { name, alias, .. } => {
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
                                    if let Ok(val) = self.env.get(&n) {
                                        exports.insert(n, val.clone());
                                    }
                                }
                            }
                            self.env.declare(
                                alias.clone(),
                                Value::Module(module_name.clone(), exports),
                            );
                        }
                    }
                    ImportDecl::Selective { .. } => {}
                }
            }
        }

        // Hybrid execution: if main exists, run it. Otherwise execute top-level statements.
        if let Ok(Value::Func(main_func)) = self.env.get("main") {
            let val = self.exec_block(&main_func.body)?;
            Ok(val)
        } else {
            // No main — execute top-level statements directly (script mode).
            // This enables quick playground scripts and small .fe files.
            let last_val = Value::Unit;
            for decl in &program.decls {
                match decl {
                    TopDecl::Func(_)
                    | TopDecl::TestFunc(_)
                    | TopDecl::ExternBlock(_)
                    | TopDecl::Constant(_)
                    | TopDecl::Impl(_)
                    | TopDecl::Enum(_)
                    | TopDecl::Trait(_)
                    | TopDecl::Group(_)
                    | TopDecl::Import(_) => {
                        // Already processed in first pass
                    }
                }
            }
            // Execute any top-level statements from the AST
            // (The parser wraps loose statements into a synthetic main block;
            //  for files with no main, we look for a __top_level__ function)
            if let Ok(Value::Func(top_func)) = self.env.get("__top_level__") {
                let mut last_val = Value::Unit;
                for stmt in &top_func.body.stmts {
                    let val = self.exec_stmt(stmt)?;
                    last_val = val;
                }
                if let Some(expr) = &top_func.body.expr {
                    last_val = self.eval_expr(expr)?;
                }
                return Ok(last_val);
            }
            Ok(last_val)
        }
    }

    // ── Block & Statement Execution ──────────────────────────────

    /// Execute a block, returning (value, signal).
    fn exec_block(&mut self, block: &Block) -> Result<Value, String> {
        let mut last_val = Value::Unit;
        for stmt in &block.stmts {
            last_val = self.exec_stmt(stmt)?;
            if matches!(last_val, Value::Return(_) | Value::Stop | Value::Skip) {
                return Ok(last_val);
            }
        }
        if let Some(expr) = &block.expr {
            last_val = self.eval_expr(expr)?;
        }
        Ok(last_val)
    }

    /// Execute a statement, returning (value, signal).
    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Value, String> {
        match stmt {
            Stmt::Keep { name, value, .. } | Stmt::Param { name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.env.declare(name.clone(), val);
                Ok(Value::Unit)
            }
            Stmt::ExprStmt(expr, _) => {
                let val = self.eval_expr(expr)?;
                if matches!(val, Value::Return(_) | Value::Stop | Value::Skip) {
                    return Ok(val);
                }
                Ok(Value::Unit)
            }
        }
    }

    // ── Pattern Matching ─────────────────────────────────────────

    /// Try to match a value against a pattern.
    /// If matched, binds variables in the current scope and returns true.
    fn match_pattern(&mut self, pat: &Pattern, val: &Value) -> Result<bool, String> {
        match pat {
            Pattern::Wildcard(_) => Ok(true),
            Pattern::Binding(name, _) => {
                self.env.declare(name.clone(), val.clone());
                Ok(true)
            }
            Pattern::Literal(lit) => {
                let lit_val = match lit {
                    Literal::Int(n) => Value::Int(*n),
                    Literal::Float(n) => Value::Float(*n),
                    Literal::Bool(b) => Value::Bool(*b),
                    Literal::String(s) => Value::String(s.clone()),
                };
                Ok(lit_val == *val)
            }
            Pattern::Constructor { name, fields, .. } => {
                if let Value::Enum(_, variant, values) = val {
                    if variant != name {
                        return Ok(false);
                    }
                    if fields.len() != values.len() {
                        return Ok(false);
                    }
                    for (sub_pat, sub_val) in fields.iter().zip(values.iter()) {
                        if !self.match_pattern(sub_pat, sub_val)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Pattern::Struct { name, fields, .. } => {
                if let Value::Group(group_name, group_fields) = val {
                    if group_name != name {
                        return Ok(false);
                    }
                    for (fname, fpat) in fields {
                        if let Some(fval) = group_fields.get(fname) {
                            if !self.match_pattern(fpat, fval)? {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    // ── Expression Evaluation ────────────────────────────────────

    fn is_truthy(&self, val: &Value) -> bool {
        match val {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.borrow().is_empty(),
            Value::Map(m) => !m.borrow().is_empty(),
            Value::Func(_) | Value::Closure(..) | Value::Builtin(_) | Value::BoundMethod(..) => {
                true
            }
            _ => false,
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Block(block) => self.exec_block(block),
            Expr::If {
                condition,
                then_block,
                elif_branches,
                else_block,
                ..
            } => {
                let cond_val = self.eval_expr(condition)?;
                if self.is_truthy(&cond_val) {
                    return self.exec_block(then_block);
                }
                for (elif_cond, elif_block) in elif_branches {
                    let elif_cond_val = self.eval_expr(elif_cond)?;
                    if self.is_truthy(&elif_cond_val) {
                        return self.exec_block(elif_block);
                    }
                }
                if let Some(else_b) = else_block {
                    self.exec_block(else_b)
                } else {
                    Ok(Value::Unit)
                }
            }
            Expr::While {
                condition, body, ..
            } => {
                loop {
                    let cond_val = self.eval_expr(condition)?;
                    if !self.is_truthy(&cond_val) {
                        break;
                    }
                    let res = self.exec_block(body)?;
                    if let Value::Stop = res {
                        break;
                    }
                    if let Value::Skip = res {
                        continue;
                    }
                    if let Value::Return(_) = res {
                        return Ok(res);
                    }
                }
                Ok(Value::Unit)
            }
            Expr::For {
                var,
                iterable,
                body,
                ..
            } => {
                let iter_val = self.eval_expr(iterable)?;
                match iter_val {
                    Value::List(elements) => {
                        let items = elements.borrow().clone();
                        for item in items {
                            self.env.enter_scope();
                            self.env.declare(var.clone(), item);
                            let res = self.exec_block(body)?;
                            self.env.exit_scope();
                            if let Value::Stop = res {
                                break;
                            }
                            if let Value::Skip = res {
                                continue;
                            }
                            if let Value::Return(_) = res {
                                return Ok(res);
                            }
                        }
                    }
                    Value::Map(pairs) => {
                        let entries = pairs.borrow().clone();
                        for (key, _) in entries {
                            self.env.enter_scope();
                            self.env.declare(var.clone(), key);
                            let res = self.exec_block(body)?;
                            self.env.exit_scope();
                            if let Value::Stop = res {
                                break;
                            }
                            if let Value::Skip = res {
                                continue;
                            }
                            if let Value::Return(_) = res {
                                return Ok(res);
                            }
                        }
                    }
                    Value::String(s) => {
                        for ch in s.chars() {
                            self.env.enter_scope();
                            self.env.declare(var.clone(), Value::String(ch.to_string()));
                            let res = self.exec_block(body)?;
                            self.env.exit_scope();
                            if let Value::Stop = res {
                                break;
                            }
                            if let Value::Skip = res {
                                continue;
                            }
                            if let Value::Return(_) = res {
                                return Ok(res);
                            }
                        }
                    }
                    _ => {
                        return Err("for-in requires a List, Map, or String".to_string());
                    }
                }
                Ok(Value::Unit)
            }
            Expr::Match { subject, cases, .. } => {
                let subj_val = self.eval_expr(subject)?;
                for case in cases {
                    if self.match_pattern(&case.pattern, &subj_val)? {
                        if let Some(guard) = &case.guard {
                            let g_val = self.eval_expr(guard)?;
                            if !self.is_truthy(&g_val) {
                                continue;
                            }
                        }
                        return self.exec_block(&case.body);
                    }
                }
                Ok(Value::Unit)
            }
            Expr::Select { .. } => Ok(Value::Unit),
            Expr::Return { value, .. } => {
                let val = value
                    .as_ref()
                    .map(|e| self.eval_expr(e))
                    .unwrap_or(Ok(Value::Unit))?;
                Ok(Value::Return(Box::new(val)))
            }
            Expr::Stop(_) => Ok(Value::Stop),
            Expr::Skip(_) => Ok(Value::Skip),
            Expr::InferBlock(b) | Expr::TrainBlock(b) => {
                let val = self.exec_block(b)?;
                Ok(val)
            }
            Expr::Lit(lit, _) => match lit {
                Literal::Int(i) => Ok(Value::Int(*i)),
                Literal::Float(f) => Ok(Value::Float(*f)),
                Literal::Bool(b) => Ok(Value::Bool(*b)),
                Literal::String(s) => Ok(Value::String(s.clone())),
            },
            Expr::Ident(name, _) => self.env.get(name),
            Expr::BinOp {
                left, op, right, ..
            } => {
                let lval = self.eval_expr(left)?;
                let rval = self.eval_expr(right)?;

                match (lval, rval) {
                    (Value::Int(a), Value::Int(b)) => match op {
                        BinOp::Add => Ok(Value::Int(a + b)),
                        BinOp::Sub => Ok(Value::Int(a - b)),
                        BinOp::Mul => Ok(Value::Int(a * b)),
                        BinOp::Div => {
                            if b == 0 {
                                Err("Runtime Error: Division by zero.".to_string())
                            } else {
                                Ok(Value::Int(a / b))
                            }
                        }
                        BinOp::Mod => {
                            if b == 0 {
                                Err("Runtime Error: Modulo by zero.".to_string())
                            } else {
                                Ok(Value::Int(a % b))
                            }
                        }
                        BinOp::Eq => Ok(Value::Bool(a == b)),
                        BinOp::NotEq => Ok(Value::Bool(a != b)),
                        BinOp::Lt => Ok(Value::Bool(a < b)),
                        BinOp::Gt => Ok(Value::Bool(a > b)),
                        BinOp::LtEq => Ok(Value::Bool(a <= b)),
                        BinOp::GtEq => Ok(Value::Bool(a >= b)),
                        _ => Err(format!("Invalid operator for int: {:?}", op)),
                    },
                    (Value::Float(a), Value::Float(b)) => match op {
                        BinOp::Add => Ok(Value::Float(a + b)),
                        BinOp::Sub => Ok(Value::Float(a - b)),
                        BinOp::Mul => Ok(Value::Float(a * b)),
                        BinOp::Div => {
                            if b == 0.0 {
                                Err("Runtime Error: Float division by zero.".to_string())
                            } else {
                                Ok(Value::Float(a / b))
                            }
                        }
                        BinOp::Eq => Ok(Value::Bool(a == b)),
                        BinOp::NotEq => Ok(Value::Bool(a != b)),
                        BinOp::Lt => Ok(Value::Bool(a < b)),
                        BinOp::Gt => Ok(Value::Bool(a > b)),
                        BinOp::LtEq => Ok(Value::Bool(a <= b)),
                        BinOp::GtEq => Ok(Value::Bool(a >= b)),
                        _ => Err(format!("Invalid operator for float: {:?}", op)),
                    },
                    (Value::Bool(a), Value::Bool(b)) => match op {
                        BinOp::And => Ok(Value::Bool(a && b)),
                        BinOp::Or => Ok(Value::Bool(a || b)),
                        BinOp::Eq => Ok(Value::Bool(a == b)),
                        BinOp::NotEq => Ok(Value::Bool(a != b)),
                        _ => Err(format!("Invalid operator for bool: {:?}", op)),
                    },
                    (Value::String(a), Value::String(b)) => match op {
                        BinOp::Add => Ok(Value::String(format!("{}{}", a, b))),
                        BinOp::Eq => Ok(Value::Bool(a == b)),
                        BinOp::NotEq => Ok(Value::Bool(a != b)),
                        _ => Err(format!("Invalid operator for string: {:?}", op)),
                    },
                    (Value::Tensor(a_data, a_dims), Value::Tensor(b_data, b_dims)) => match op {
                        BinOp::MatMul => {
                            if a_dims.len() != 2 || b_dims.len() != 2 {
                                return Err("MatMul requires 2D tensors".to_string());
                            }
                            let m = a_dims[0] as usize;
                            let n = a_dims[1] as usize;
                            let p = b_dims[0] as usize;
                            let q = b_dims[1] as usize;
                            if n != p {
                                return Err(format!(
                                    "MatMul dimension mismatch: {}x{} @ {}x{}",
                                    m, n, p, q
                                ));
                            }
                            let mut out_data = vec![0.0; m * q];
                            for i in 0..m {
                                for j in 0..q {
                                    let mut sum = 0.0;
                                    for k in 0..n {
                                        sum += a_data[i * n + k] * b_data[k * q + j];
                                    }
                                    out_data[i * q + j] = sum;
                                }
                            }
                            Ok(Value::Tensor(out_data, vec![m as i64, q as i64]))
                        }
                        _ => Err(format!("Invalid operator for Tensor: {:?}", op)),
                    },
                    _ => Err(
                        "Invalid or unsupported binary operation types in interpreter".to_string(),
                    ),
                }
            }
            Expr::UnaryOp { op, operand, .. } => {
                let val = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Neg => match val {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err("Invalid operand for negation".to_string()),
                    },
                    UnaryOp::Not => match val {
                        Value::Bool(b) => Ok(Value::Bool(!b)),
                        _ => Err("Invalid operand for logical NOT".to_string()),
                    },
                    UnaryOp::Await => Ok(val), // Placeholder for future async support
                }
            }
            Expr::Call { callee, args, .. } => {
                let func_val = self.eval_expr(callee)?;
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(self.eval_expr(arg)?);
                }

                match func_val {
                    Value::Builtin(name) => self.execute_builtin(&name, evaluated_args),
                    Value::Func(decl) => {
                        self.env.enter_scope();
                        for (i, param) in decl.params.iter().enumerate() {
                            if i < evaluated_args.len() {
                                self.env
                                    .declare(param.name.clone(), evaluated_args[i].clone());
                            }
                        }
                        let ret = self.exec_block(&decl.body)?;
                        self.env.exit_scope();
                        if let Value::Return(val) = ret {
                            Ok(*val)
                        } else {
                            Ok(ret)
                        }
                    }
                    Value::Closure(params, body, captured_env) => {
                        // Closures use pure snapshot semantics:
                        // The captured environment is a frozen copy of the scope at
                        // lambda creation time. Mutations inside the closure do NOT
                        // propagate back to the outer scope. This is intentional for
                        // stability — it prevents data races and unexpected side effects.
                        let saved_env = self.env.clone();
                        self.env = (*captured_env).clone();
                        self.env.enter_scope();
                        for (i, param) in params.iter().enumerate() {
                            if i < evaluated_args.len() {
                                self.env
                                    .declare(param.name.clone(), evaluated_args[i].clone());
                            }
                        }
                        let result = self.eval_expr(&body);
                        self.env.exit_scope();
                        self.env = saved_env;
                        match result {
                            Ok(Value::Return(val)) => Ok(*val),
                            other => other,
                        }
                    }
                    Value::BoundMethod(receiver, method) => {
                        // Auto-inject `self` (the receiver) as the first argument
                        let mut full_args = vec![*receiver];
                        full_args.extend(evaluated_args);
                        match *method {
                            Value::Func(decl) => {
                                self.env.enter_scope();
                                for (i, param) in decl.params.iter().enumerate() {
                                    if i < full_args.len() {
                                        self.env.declare(param.name.clone(), full_args[i].clone());
                                    }
                                }
                                let ret = self.exec_block(&decl.body)?;
                                self.env.exit_scope();
                                if let Value::Return(val) = ret {
                                    Ok(*val)
                                } else {
                                    Ok(ret)
                                }
                            }
                            Value::Builtin(ref name) => self.execute_builtin(name, full_args),
                            _ => Err("BoundMethod wraps a non-function value".to_string()),
                        }
                    }
                    _ => Err("Attempt to call a non-function".to_string()),
                }
            }
            Expr::Assign { target, value, .. } => {
                let val = self.eval_expr(value)?;
                if let Expr::Ident(name, _) = &**target {
                    self.env.assign(name, val.clone())?;
                    Ok(val)
                } else if let Expr::FieldAccess { object, field, .. } = &**target {
                    // Field assignment: obj.field = value
                    if let Expr::Ident(obj_name, _) = &**object {
                        let obj = self.env.get(obj_name)?;
                        if let Value::Group(group_name, mut fields_map) = obj {
                            fields_map.insert(field.clone(), val.clone());
                            self.env
                                .assign(obj_name, Value::Group(group_name, fields_map))?;
                            Ok(val)
                        } else {
                            Err("Field assignment on non-group type".to_string())
                        }
                    } else {
                        Err("Complex field assignment target not supported".to_string())
                    }
                } else if let Expr::IndexAccess { object, index, .. } = &**target {
                    // Index assignment: list[i] = value
                    if let Expr::Ident(obj_name, _) = &**object {
                        let idx = self.eval_expr(index)?;
                        let obj = self.env.get(obj_name)?;
                        if let (Value::List(items_ref), Value::Int(i)) = (&obj, &idx) {
                            let i = *i;
                            if i < 0 {
                                return Err(format!(
                                    "Runtime Error: Negative index {} is not allowed.",
                                    i
                                ));
                            }
                            let mut items = items_ref.borrow_mut();
                            let idx = i as usize;
                            if idx < items.len() {
                                items[idx] = val.clone();
                                Ok(val)
                            } else {
                                Err(format!(
                                    "Runtime Error: Index {} out of bounds (len {}).",
                                    i,
                                    items.len()
                                ))
                            }
                        } else if let (Value::Map(pairs_ref), key_val) = (&obj, &idx) {
                            let mut pairs = pairs_ref.borrow_mut();
                            if let Some(pair) = pairs.iter_mut().find(|(k, _)| k == key_val) {
                                pair.1 = val.clone();
                            } else {
                                pairs.push((key_val.clone(), val.clone()));
                            }
                            Ok(val)
                        } else {
                            Err("Index assignment on non-list/map type".to_string())
                        }
                    } else {
                        Err("Complex index assignment target not supported".to_string())
                    }
                } else {
                    Err("Complex assignment not supported in simple interpreter".to_string())
                }
            }
            Expr::GroupLiteral { name, fields, .. } => {
                let mut group_fields = HashMap::new();
                for (fname, fexpr) in fields {
                    group_fields.insert(fname.clone(), self.eval_expr(fexpr)?);
                }
                Ok(Value::Group(name.clone(), group_fields))
            }
            Expr::FieldAccess { object, field, .. } => {
                let obj = self.eval_expr(object)?;
                match &obj {
                    Value::Group(group_name, fields) => {
                        // First check if it's a field
                        if let Some(val) = fields.get(field) {
                            Ok(val.clone())
                        } else {
                            // Check if it's a method — look up TypeName::method_name
                            let qualified = format!("{}::{}", group_name, field);
                            if let Ok(method) = self.env.get(&qualified) {
                                // Return a partially-applied method with self bound
                                Ok(Value::BoundMethod(Box::new(obj.clone()), Box::new(method)))
                            } else if let Ok(method) = self.env.get(field) {
                                Ok(Value::BoundMethod(Box::new(obj.clone()), Box::new(method)))
                            } else {
                                Err(format!(
                                    "Field or method '{}' not found on '{}'",
                                    field, group_name
                                ))
                            }
                        }
                    }
                    Value::Module(module_name, exports) => {
                        if let Some(val) = exports.get(field) {
                            Ok(val.clone())
                        } else {
                            Err(format!(
                                "Module '{}' has no public member '{}'",
                                module_name, field
                            ))
                        }
                    }
                    Value::List(_) => {
                        let method_builtin = Value::Builtin(format!("__list_{}", field));
                        Ok(Value::BoundMethod(
                            Box::new(obj.clone()),
                            Box::new(method_builtin),
                        ))
                    }
                    Value::Map(_) => {
                        let method_builtin = Value::Builtin(format!("__map_{}", field));
                        Ok(Value::BoundMethod(
                            Box::new(obj.clone()),
                            Box::new(method_builtin),
                        ))
                    }
                    Value::String(_) => {
                        let method_builtin = Value::Builtin(format!("__str_{}", field));
                        Ok(Value::BoundMethod(
                            Box::new(obj.clone()),
                            Box::new(method_builtin),
                        ))
                    }
                    _ => Err("Field access on non-group type".to_string()),
                }
            }
            Expr::IndexAccess { object, index, .. } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                match (obj, idx) {
                    (Value::List(items_ref), Value::Int(i)) => {
                        let items = items_ref.borrow();
                        if i < 0 {
                            return Err(format!(
                                "Runtime Error: Negative index {} is not allowed.",
                                i
                            ));
                        }
                        let idx = i as usize;
                        if idx < items.len() {
                            Ok(items[idx].clone())
                        } else {
                            Err(format!(
                                "Runtime Error: Index {} out of bounds (len {}).",
                                i,
                                items.len()
                            ))
                        }
                    }
                    (Value::Map(pairs_ref), key_val) => {
                        let pairs = pairs_ref.borrow();
                        if let Some((_, v)) = pairs.iter().find(|(k, _)| k == &key_val) {
                            Ok(v.clone())
                        } else {
                            Err(format!("Runtime Error: Key {} not found in map.", key_val))
                        }
                    }
                    (Value::String(s), Value::Int(i)) => {
                        if i < 0 {
                            return Err(format!(
                                "Runtime Error: Negative string index {} is not allowed.",
                                i
                            ));
                        }
                        let idx = i as usize;
                        if idx < s.len() {
                            Ok(Value::String(s.chars().nth(idx).unwrap().to_string()))
                        } else {
                            Err(format!(
                                "Runtime Error: String index {} out of bounds (len {}).",
                                i,
                                s.len()
                            ))
                        }
                    }
                    _ => Err("Indexing not supported for this type".to_string()),
                }
            }
            Expr::Lambda { params, body, .. } => {
                // Capture the current environment snapshot for the closure
                let captured_env = self.env.clone();
                Ok(Value::Closure(
                    params.clone(),
                    Rc::new(*body.clone()),
                    Rc::new(captured_env),
                ))
            }
            Expr::UnsafeBlock(block, _) => {
                let val = self.exec_block(block)?;
                Ok(val)
            }
        }
    }

    // ── Builtin Function Dispatch ────────────────────────────────

    fn execute_builtin(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            "print" | "println" => {
                if let Some(val) = args.get(0) {
                    if name == "println" {
                        println!("{}", val);
                    } else {
                        print!("{}", val);
                    }
                }
                Ok(Value::Unit)
            }
            "input" => {
                if let Some(Value::String(prompt)) = args.get(0) {
                    use std::io::{self, Write};
                    print!("{}", prompt);
                    io::stdout().flush().unwrap();
                    let mut input = String::new();
                    io::stdin().read_line(&mut input).unwrap();
                    Ok(Value::String(input.trim().to_string()))
                } else {
                    Err("input() requires a string prompt".to_string())
                }
            }
            "len" => {
                if let Some(val) = args.get(0) {
                    match val {
                        Value::String(s) => Ok(Value::Int(s.len() as i64)),
                        Value::List(items) => Ok(Value::Int(items.borrow().len() as i64)),
                        Value::Map(pairs) => Ok(Value::Int(pairs.borrow().len() as i64)),
                        _ => Err("len() expects a string, list or map".to_string()),
                    }
                } else {
                    Err("len() expects an argument".to_string())
                }
            }
            "str" => {
                if let Some(val) = args.get(0) {
                    Ok(Value::String(val.to_string()))
                } else {
                    Err("str() expects an argument".to_string())
                }
            }
            "int" => {
                if let Some(val) = args.get(0) {
                    match val {
                        Value::String(s) => s
                            .parse::<i64>()
                            .map(Value::Int)
                            .map_err(|_| format!("Cannot parse '{}' as int", s)),
                        Value::Float(f) => Ok(Value::Int(*f as i64)),
                        Value::Int(n) => Ok(Value::Int(*n)),
                        _ => Err("int() expects a string, float, or int".to_string()),
                    }
                } else {
                    Err("int() expects an argument".to_string())
                }
            }
            "float" => {
                if let Some(val) = args.get(0) {
                    match val {
                        Value::String(s) => s
                            .parse::<f64>()
                            .map(Value::Float)
                            .map_err(|_| format!("Cannot parse '{}' as float", s)),
                        Value::Int(n) => Ok(Value::Float(*n as f64)),
                        Value::Float(f) => Ok(Value::Float(*f)),
                        _ => Err("float() expects a string, int, or float".to_string()),
                    }
                } else {
                    Err("float() expects an argument".to_string())
                }
            }
            "assert" => {
                if let Some(Value::Bool(cond)) = args.get(0) {
                    if !cond {
                        let msg = args
                            .get(1)
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "Assertion failed".to_string());
                        Err(format!("Assertion failed: {}", msg))
                    } else {
                        Ok(Value::Unit)
                    }
                } else {
                    Err("assert() expects a bool condition".to_string())
                }
            }
            "exit" => {
                let code = if let Some(Value::Int(c)) = args.get(0) {
                    *c as i32
                } else {
                    0
                };
                std::process::exit(code);
            }
            _ => {
                if name == "List" {
                    return Ok(Value::List(Rc::new(RefCell::new(args))));
                }
                if name == "Map" {
                    return Ok(Value::Map(Rc::new(RefCell::new(vec![]))));
                }
                if name.starts_with("__list_") {
                    return self.execute_list_builtin(name, args);
                }
                if name.starts_with("__map_") {
                    return self.execute_map_builtin(name, args);
                }
                if name.starts_with("__str_") {
                    return self.execute_str_builtin(name, args);
                }
                if name == "range" {
                    let (start, end, step) = match args.len() {
                        1 => (
                            0,
                            if let Value::Int(i) = args[0] {
                                i
                            } else {
                                return Err("range expects int".to_string());
                            },
                            1,
                        ),
                        2 => (
                            if let Value::Int(i) = args[0] {
                                i
                            } else {
                                return Err("range expects int".to_string());
                            },
                            if let Value::Int(i) = args[1] {
                                i
                            } else {
                                return Err("range expects int".to_string());
                            },
                            1,
                        ),
                        3 => (
                            if let Value::Int(i) = args[0] {
                                i
                            } else {
                                return Err("range expects int".to_string());
                            },
                            if let Value::Int(i) = args[1] {
                                i
                            } else {
                                return Err("range expects int".to_string());
                            },
                            if let Value::Int(i) = args[2] {
                                i
                            } else {
                                return Err("range expects int".to_string());
                            },
                        ),
                        _ => return Err("range() expects 1-3 arguments".to_string()),
                    };
                    let mut items = Vec::new();
                    let mut i = start;
                    while (step > 0 && i < end) || (step < 0 && i > end) {
                        items.push(Value::Int(i));
                        i += step;
                    }
                    return Ok(Value::List(Rc::new(RefCell::new(items))));
                }
                if name == "zeros" || name == "ones" || name == "rand" {
                    if args.is_empty() {
                        return Err(format!("{}() expects dimensions", name));
                    }
                    let mut dims = Vec::new();
                    let mut total_size = 1;
                    for arg in args {
                        if let Value::Int(i) = arg {
                            dims.push(i);
                            total_size *= i as usize;
                        } else {
                            return Err(format!("{}() dimensions must be integers", name));
                        }
                    }
                    let mut data = vec![0.0; total_size];
                    if name == "ones" {
                        data.fill(1.0);
                    } else if name == "rand" {
                        // VERY simple PRNG for demonstration
                        let mut seed: u64 = 12345;
                        for i in 0..total_size {
                            seed = seed.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
                            data[i] = (seed as f64) / (0x7fffffff as f64);
                        }
                    }
                    return Ok(Value::Tensor(data, dims));
                }
                if name.starts_with("__builtin_math_") {
                    let func = name.strip_prefix("__builtin_math_").unwrap();
                    if let Some(Value::Float(x)) = args.get(0) {
                        match func {
                            "sin" => return Ok(Value::Float(x.sin())),
                            "cos" => return Ok(Value::Float(x.cos())),
                            "tan" => return Ok(Value::Float(x.tan())),
                            "sqrt" => return Ok(Value::Float(x.sqrt())),
                            "log" => return Ok(Value::Float(x.ln())),
                            "log2" => return Ok(Value::Float(x.log2())),
                            "log10" => return Ok(Value::Float(x.log10())),
                            "floor" => return Ok(Value::Float(x.floor())),
                            "ceil" => return Ok(Value::Float(x.ceil())),
                            "round" => return Ok(Value::Float(x.round())),
                            "pow" => {
                                if let Some(Value::Float(y)) = args.get(1) {
                                    return Ok(Value::Float(x.powf(*y)));
                                }
                            }
                            "atan2" => {
                                if let Some(Value::Float(y)) = args.get(1) {
                                    return Ok(Value::Float(x.atan2(*y)));
                                }
                            }
                            _ => {}
                        }
                    }
                    return Err(format!("Invalid arguments to {}", name));
                }

                if name.starts_with("__builtin_io_") {
                    let func = name.strip_prefix("__builtin_io_").unwrap();
                    match func {
                        "read_file" => {
                            if let Some(Value::String(path)) = args.get(0) {
                                return match std::fs::read_to_string(path) {
                                    Ok(content) => Ok(Value::String(content)),
                                    Err(e) => Err(format!("Failed to read file '{}': {}", path, e)),
                                };
                            }
                        }
                        "write_file" => {
                            if let (Some(Value::String(path)), Some(Value::String(content))) =
                                (args.get(0), args.get(1))
                            {
                                return match std::fs::write(path, content) {
                                    Ok(_) => Ok(Value::Unit),
                                    Err(e) => {
                                        Err(format!("Failed to write file '{}': {}", path, e))
                                    }
                                };
                            }
                        }
                        "append_file" => {
                            if let (Some(Value::String(path)), Some(Value::String(content))) =
                                (args.get(0), args.get(1))
                            {
                                use std::fs::OpenOptions;
                                use std::io::Write;
                                return match OpenOptions::new().append(true).create(true).open(path)
                                {
                                    Ok(mut file) => match file.write_all(content.as_bytes()) {
                                        Ok(_) => Ok(Value::Unit),
                                        Err(e) => Err(format!(
                                            "Failed to append to file '{}': {}",
                                            path, e
                                        )),
                                    },
                                    Err(e) => Err(format!("Failed to open file '{}': {}", path, e)),
                                };
                            }
                        }
                        "file_exists" => {
                            if let Some(Value::String(path)) = args.get(0) {
                                return Ok(Value::Bool(std::path::Path::new(path).exists()));
                            }
                        }
                        _ => {}
                    }
                    return Err(format!("Invalid arguments to {}", name));
                }

                if name.starts_with("__builtin_string_") {
                    let func = name.strip_prefix("__builtin_string_").unwrap();
                    match func {
                        "split" => {
                            if let (Some(Value::String(s)), Some(Value::String(delim))) =
                                (args.get(0), args.get(1))
                            {
                                let parts: Vec<Value> = s
                                    .split(delim)
                                    .map(|p| Value::String(p.to_string()))
                                    .collect();
                                return Ok(Value::List(std::rc::Rc::new(std::cell::RefCell::new(
                                    parts,
                                ))));
                            }
                        }
                        "join" => {
                            if let (Some(Value::List(l)), Some(Value::String(delim))) =
                                (args.get(0), args.get(1))
                            {
                                let parts: Vec<String> =
                                    l.borrow().iter().map(|v| v.to_string()).collect();
                                return Ok(Value::String(parts.join(delim)));
                            }
                        }
                        "upper" => {
                            if let Some(Value::String(s)) = args.get(0) {
                                return Ok(Value::String(s.to_uppercase()));
                            }
                        }
                        "lower" => {
                            if let Some(Value::String(s)) = args.get(0) {
                                return Ok(Value::String(s.to_lowercase()));
                            }
                        }
                        "trim" => {
                            if let Some(Value::String(s)) = args.get(0) {
                                return Ok(Value::String(s.trim().to_string()));
                            }
                        }
                        "replace" => {
                            if let (
                                Some(Value::String(s)),
                                Some(Value::String(old)),
                                Some(Value::String(new)),
                            ) = (args.get(0), args.get(1), args.get(2))
                            {
                                return Ok(Value::String(s.replace(old, new)));
                            }
                        }
                        "starts_with" => {
                            if let (Some(Value::String(s)), Some(Value::String(prefix))) =
                                (args.get(0), args.get(1))
                            {
                                return Ok(Value::Bool(s.starts_with(prefix)));
                            }
                        }
                        "ends_with" => {
                            if let (Some(Value::String(s)), Some(Value::String(suffix))) =
                                (args.get(0), args.get(1))
                            {
                                return Ok(Value::Bool(s.ends_with(suffix)));
                            }
                        }
                        "contains" => {
                            if let (Some(Value::String(s)), Some(Value::String(sub))) =
                                (args.get(0), args.get(1))
                            {
                                return Ok(Value::Bool(s.contains(sub)));
                            }
                        }
                        "repeat" => {
                            if let (Some(Value::String(s)), Some(Value::Int(n))) =
                                (args.get(0), args.get(1))
                            {
                                return Ok(Value::String(s.repeat(std::cmp::max(0, *n) as usize)));
                            }
                        }
                        "substr" => {
                            if let (
                                Some(Value::String(s)),
                                Some(Value::Int(start)),
                                Some(Value::Int(length)),
                            ) = (args.get(0), args.get(1), args.get(2))
                            {
                                let start = std::cmp::max(0, *start) as usize;
                                let length = std::cmp::max(0, *length) as usize;
                                let end = std::cmp::min(s.len(), start + length);
                                if start <= s.len() {
                                    return Ok(Value::String(s[start..end].to_string()));
                                }
                                return Ok(Value::String("".to_string()));
                            }
                        }
                        "char_at" => {
                            if let (Some(Value::String(s)), Some(Value::Int(idx))) =
                                (args.get(0), args.get(1))
                            {
                                let idx = *idx as usize;
                                if let Some(c) = s.chars().nth(idx) {
                                    return Ok(Value::String(c.to_string()));
                                }
                                return Ok(Value::String("".to_string()));
                            }
                        }
                        _ => {}
                    }
                    return Err(format!("Invalid arguments to {}", name));
                }

                if name.starts_with("enum_") {
                    let parts: Vec<&str> = name.split("::").collect();
                    let enum_name = parts[0].replace("enum_", "");
                    let variant = parts[1];
                    Ok(Value::Enum(enum_name, variant.to_string(), args))
                } else {
                    Err(format!("Unknown builtin: {}", name))
                }
            }
        }
    }

    fn execute_list_builtin(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let list_val = args.get(0).ok_or("List method requires self")?;
        let items_rc = match list_val {
            Value::List(rc) => rc,
            _ => return Err("Expected List".to_string()),
        };
        match name {
            "__list_push" => {
                let item = args.get(1).ok_or("push requires 1 argument")?;
                items_rc.borrow_mut().push(item.clone());
                Ok(Value::Unit)
            }
            "__list_pop" => items_rc
                .borrow_mut()
                .pop()
                .ok_or("Cannot pop from empty list".to_string()),
            "__list_len" => Ok(Value::Int(items_rc.borrow().len() as i64)),
            "__list_contains" => {
                let item = args.get(1).ok_or("contains requires 1 argument")?;
                Ok(Value::Bool(items_rc.borrow().contains(item)))
            }
            "__list_remove" => {
                let idx_val = args.get(1).ok_or("remove requires 1 argument")?;
                if let Value::Int(i) = idx_val {
                    let idx = *i as usize;
                    let mut items = items_rc.borrow_mut();
                    if idx < items.len() {
                        Ok(items.remove(idx))
                    } else {
                        Err(format!("Index {} out of bounds", idx))
                    }
                } else {
                    Err("remove expects int index".to_string())
                }
            }
            "__list_reverse" => {
                items_rc.borrow_mut().reverse();
                Ok(Value::Unit)
            }
            "__list_clear" => {
                items_rc.borrow_mut().clear();
                Ok(Value::Unit)
            }
            "__list_insert" => {
                let idx_val = args.get(1).ok_or("insert requires 2 arguments")?;
                let item = args.get(2).ok_or("insert requires 2 arguments")?;
                if let Value::Int(i) = idx_val {
                    let idx = *i as usize;
                    let mut items = items_rc.borrow_mut();
                    if idx <= items.len() {
                        items.insert(idx, item.clone());
                        Ok(Value::Unit)
                    } else {
                        Err(format!("Index {} out of bounds", idx))
                    }
                } else {
                    Err("insert expects int index".to_string())
                }
            }
            "__list_slice" => {
                let start_val = args.get(1).ok_or("slice requires 2 arguments")?;
                let end_val = args.get(2).ok_or("slice requires 2 arguments")?;
                if let (Value::Int(start), Value::Int(end)) = (start_val, end_val) {
                    let start = *start as usize;
                    let end = *end as usize;
                    let items = items_rc.borrow();
                    if start <= items.len() && end <= items.len() && start <= end {
                        let sliced = items[start..end].to_vec();
                        Ok(Value::List(Rc::new(RefCell::new(sliced))))
                    } else {
                        Err(format!("Invalid slice {}..{}", start, end))
                    }
                } else {
                    Err("slice expects int indices".to_string())
                }
            }
            _ => Err(format!("Unknown list method: {}", name)),
        }
    }

    fn execute_map_builtin(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let map_val = args.get(0).ok_or("Map method requires self")?;
        let pairs_rc = match map_val {
            Value::Map(rc) => rc,
            _ => return Err("Expected Map".to_string()),
        };
        match name {
            "__map_set" => {
                let key = args.get(1).ok_or("set requires 2 arguments")?;
                let val = args.get(2).ok_or("set requires 2 arguments")?;
                let mut pairs = pairs_rc.borrow_mut();
                if let Some(pair) = pairs.iter_mut().find(|(k, _)| k == key) {
                    pair.1 = val.clone();
                } else {
                    pairs.push((key.clone(), val.clone()));
                }
                Ok(Value::Unit)
            }
            "__map_get" => {
                let key = args.get(1).ok_or("get requires 1 argument")?;
                let pairs = pairs_rc.borrow();
                if let Some((_, v)) = pairs.iter().find(|(k, _)| k == key) {
                    Ok(v.clone())
                } else {
                    Err(format!("Key {} not found in map", key))
                }
            }
            "__map_contains" => {
                let key = args.get(1).ok_or("contains requires 1 argument")?;
                let pairs = pairs_rc.borrow();
                Ok(Value::Bool(pairs.iter().any(|(k, _)| k == key)))
            }
            "__map_remove" => {
                let key = args.get(1).ok_or("remove requires 1 argument")?;
                let mut pairs = pairs_rc.borrow_mut();
                if let Some(idx) = pairs.iter().position(|(k, _)| k == key) {
                    Ok(pairs.remove(idx).1)
                } else {
                    Err(format!("Key {} not found in map", key))
                }
            }
            "__map_keys" => {
                let pairs = pairs_rc.borrow();
                let keys = pairs.iter().map(|(k, _)| k.clone()).collect();
                Ok(Value::List(Rc::new(RefCell::new(keys))))
            }
            "__map_values" => {
                let pairs = pairs_rc.borrow();
                let values = pairs.iter().map(|(_, v)| v.clone()).collect();
                Ok(Value::List(Rc::new(RefCell::new(values))))
            }
            "__map_len" => Ok(Value::Int(pairs_rc.borrow().len() as i64)),
            _ => Err(format!("Unknown map method: {}", name)),
        }
    }

    fn execute_str_builtin(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let str_val = args.get(0).ok_or("String method requires self")?;
        let s = match str_val {
            Value::String(s) => s,
            _ => return Err("Expected String".to_string()),
        };
        match name {
            "__str_len" => Ok(Value::Int(s.len() as i64)),
            "__str_charAt" => {
                let idx_val = args.get(1).ok_or("charAt requires 1 argument")?;
                if let Value::Int(i) = idx_val {
                    let idx = *i as usize;
                    if idx < s.len() {
                        Ok(Value::String(s.chars().nth(idx).unwrap().to_string()))
                    } else {
                        Err(format!("Index {} out of bounds", idx))
                    }
                } else {
                    Err("charAt expects int index".to_string())
                }
            }
            "__str_substring" => {
                let start_val = args.get(1).ok_or("substring requires 2 arguments")?;
                let end_val = args.get(2).ok_or("substring requires 2 arguments")?;
                if let (Value::Int(start), Value::Int(end)) = (start_val, end_val) {
                    let start = *start as usize;
                    let end = *end as usize;
                    if start <= s.len() && end <= s.len() && start <= end {
                        Ok(Value::String(s[start..end].to_string()))
                    } else {
                        Err(format!("Invalid substring {}..{}", start, end))
                    }
                } else {
                    Err("substring expects int indices".to_string())
                }
            }
            "__str_split" => {
                let delim_val = args.get(1).ok_or("split requires 1 argument")?;
                if let Value::String(delim) = delim_val {
                    let parts = s
                        .split(delim)
                        .map(|p| Value::String(p.to_string()))
                        .collect();
                    Ok(Value::List(Rc::new(RefCell::new(parts))))
                } else {
                    Err("split expects string delimiter".to_string())
                }
            }
            "__str_contains" => {
                let substr_val = args.get(1).ok_or("contains requires 1 argument")?;
                if let Value::String(substr) = substr_val {
                    Ok(Value::Bool(s.contains(substr)))
                } else {
                    Err("contains expects string argument".to_string())
                }
            }
            "__str_replace" => {
                let from_val = args.get(1).ok_or("replace requires 2 arguments")?;
                let to_val = args.get(2).ok_or("replace requires 2 arguments")?;
                if let (Value::String(from), Value::String(to)) = (from_val, to_val) {
                    Ok(Value::String(s.replace(from, to)))
                } else {
                    Err("replace expects string arguments".to_string())
                }
            }
            "__str_trim" => Ok(Value::String(s.trim().to_string())),
            "__str_upper" => Ok(Value::String(s.to_uppercase())),
            "__str_lower" => Ok(Value::String(s.to_lowercase())),
            "__str_startsWith" => {
                let prefix_val = args.get(1).ok_or("startsWith requires 1 argument")?;
                if let Value::String(prefix) = prefix_val {
                    Ok(Value::Bool(s.starts_with(prefix)))
                } else {
                    Err("startsWith expects string argument".to_string())
                }
            }
            "__str_endsWith" => {
                let suffix_val = args.get(1).ok_or("endsWith requires 1 argument")?;
                if let Value::String(suffix) = suffix_val {
                    Ok(Value::Bool(s.ends_with(suffix)))
                } else {
                    Err("endsWith expects string argument".to_string())
                }
            }
            _ => Err(format!("Unknown string method: {}", name)),
        }
    }
}
