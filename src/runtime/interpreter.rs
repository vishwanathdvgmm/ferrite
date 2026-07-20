use super::environment::Environment;
use super::value::Value;
use crate::ast::*;
use std::collections::HashMap;
use std::rc::Rc;

/// Control flow signals that propagate up through the interpreter.
/// These allow `return`, `stop` (break), and `skip` (continue) to
/// correctly unwind through nested blocks.
enum Signal {
    /// Normal execution, no control flow change.
    None,
    /// A `return` statement was executed with a value.
    Return(Value),
    /// A `stop` (break) statement was executed.
    Break,
    /// A `skip` (continue) statement was executed.
    Continue,
}

pub struct Interpreter {
    pub env: Environment,
    module_exports: HashMap<String, Vec<TopDecl>>,
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
            let (val, _) = self.exec_block(&main_func.body)?;
            Ok(val)
        } else {
            // No main — execute top-level statements directly (script mode).
            // This enables quick playground scripts and small .fe files.
            let last_val = Value::Unit;
            for decl in &program.decls {
                match decl {
                    TopDecl::Func(_)
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
                let (val, _) = self.exec_block(&top_func.body)?;
                return Ok(val);
            }
            Ok(last_val)
        }
    }

    // ── Block & Statement Execution ──────────────────────────────

    /// Execute a block, returning (value, signal).
    fn exec_block(&mut self, block: &Block) -> Result<(Value, Signal), String> {
        self.env.enter_scope();
        let mut last_val = Value::Unit;
        for stmt in &block.stmts {
            let (val, sig) = self.exec_stmt(stmt)?;
            match sig {
                Signal::None => {
                    last_val = val;
                }
                Signal::Return(v) => {
                    self.env.exit_scope();
                    return Ok((Value::Unit, Signal::Return(v)));
                }
                Signal::Break => {
                    self.env.exit_scope();
                    return Ok((Value::Unit, Signal::Break));
                }
                Signal::Continue => {
                    self.env.exit_scope();
                    return Ok((Value::Unit, Signal::Continue));
                }
            }
        }
        self.env.exit_scope();
        Ok((last_val, Signal::None))
    }

    /// Execute a statement, returning (value, signal).
    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<(Value, Signal), String> {
        match stmt {
            Stmt::Keep { name, value, .. } | Stmt::Param { name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.env.declare(name.clone(), val);
                Ok((Value::Unit, Signal::None))
            }
            Stmt::ExprStmt(expr) => {
                self.eval_expr(expr)?;
                Ok((Value::Unit, Signal::None))
            }
            Stmt::Return { value, .. } => {
                let ret_val = if let Some(expr) = value {
                    self.eval_expr(expr)?
                } else {
                    Value::Unit
                };
                Ok((Value::Unit, Signal::Return(ret_val)))
            }
            Stmt::If {
                condition,
                then_block,
                elif_branches,
                else_block,
                ..
            } => {
                let cond_val = self.eval_expr(condition)?;
                if cond_val == Value::Bool(true) {
                    return self.exec_block(then_block);
                }

                for (elif_cond, elif_block) in elif_branches {
                    let elif_cond_val = self.eval_expr(elif_cond)?;
                    if elif_cond_val == Value::Bool(true) {
                        return self.exec_block(elif_block);
                    }
                }

                if let Some(else_b) = else_block {
                    return self.exec_block(else_b);
                }
                Ok((Value::Unit, Signal::None))
            }
            Stmt::While {
                condition, body, ..
            } => {
                loop {
                    let cond_val = self.eval_expr(condition)?;
                    if cond_val != Value::Bool(true) {
                        break;
                    }
                    let (_, sig) = self.exec_block(body)?;
                    match sig {
                        Signal::Break => break,
                        Signal::Continue => continue,
                        Signal::Return(v) => return Ok((Value::Unit, Signal::Return(v))),
                        Signal::None => {}
                    }
                }
                Ok((Value::Unit, Signal::None))
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                let iter_val = self.eval_expr(iterable)?;
                let items = match iter_val {
                    Value::List(items) => items,
                    Value::String(s) => {
                        // Iterate over characters
                        s.chars().map(|c| Value::String(c.to_string())).collect()
                    }
                    other => {
                        return Err(format!(
                            "Cannot iterate over type '{}'. Expected a List or String.",
                            other
                        ))
                    }
                };

                for item in items {
                    self.env.enter_scope();
                    self.env.declare(var.clone(), item);
                    let (_, sig) = self.exec_block(body)?;
                    self.env.exit_scope();
                    match sig {
                        Signal::Break => break,
                        Signal::Continue => continue,
                        Signal::Return(v) => return Ok((Value::Unit, Signal::Return(v))),
                        Signal::None => {}
                    }
                }
                Ok((Value::Unit, Signal::None))
            }
            Stmt::Match { subject, cases, .. } => {
                let subject_val = self.eval_expr(subject)?;
                for case in cases {
                    self.env.enter_scope();
                    if self.match_pattern(&case.pattern, &subject_val)? {
                        // Check guard clause if present
                        if let Some(ref guard) = case.guard {
                            let guard_val = self.eval_expr(guard)?;
                            if guard_val != Value::Bool(true) {
                                self.env.exit_scope();
                                continue;
                            }
                        }
                        let (val, sig) = self.exec_block(&case.body)?;
                        self.env.exit_scope();
                        return Ok((val, sig));
                    }
                    self.env.exit_scope();
                }
                Ok((Value::Unit, Signal::None))
            }
            Stmt::Select { cases, .. } => {
                for case in cases {
                    self.env.enter_scope();
                    if case.is_default {
                        let (val, sig) = self.exec_block(&case.body)?;
                        self.env.exit_scope();
                        return Ok((val, sig));
                    }
                    if let Some((name, expr)) = &case.assignment {
                        let val = self.eval_expr(expr)?;
                        if name != "_" {
                            self.env.declare(name.clone(), val);
                        }
                    }
                    let (val, sig) = self.exec_block(&case.body)?;
                    self.env.exit_scope();
                    return Ok((val, sig));
                }
                Ok((Value::Unit, Signal::None))
            }
            Stmt::InferBlock(block) | Stmt::TrainBlock(block) => self.exec_block(block),
            Stmt::Stop(_) => Ok((Value::Unit, Signal::Break)),
            Stmt::Skip(_) => Ok((Value::Unit, Signal::Continue)),
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

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
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
                        let (ret, sig) = self.exec_block(&decl.body)?;
                        self.env.exit_scope();
                        match sig {
                            Signal::Return(v) => Ok(v),
                            _ => Ok(ret),
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
                        result
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
                                let (ret, sig) = self.exec_block(&decl.body)?;
                                self.env.exit_scope();
                                match sig {
                                    Signal::Return(v) => Ok(v),
                                    _ => Ok(ret),
                                }
                            }
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
                        if let (Value::List(mut items), Value::Int(i)) = (obj, idx) {
                            if i < 0 {
                                return Err(format!(
                                    "Runtime Error: Negative index {} is not allowed.",
                                    i
                                ));
                            }
                            let idx = i as usize;
                            if idx < items.len() {
                                items[idx] = val.clone();
                                self.env.assign(obj_name, Value::List(items))?;
                                Ok(val)
                            } else {
                                Err(format!(
                                    "Runtime Error: Index {} out of bounds (len {}).",
                                    i,
                                    items.len()
                                ))
                            }
                        } else {
                            Err("Index assignment on non-list type".to_string())
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
                    _ => Err("Field access on non-group type".to_string()),
                }
            }
            Expr::IndexAccess { object, index, .. } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                match (obj, idx) {
                    (Value::List(items), Value::Int(i)) => {
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
                        Value::List(items) => Ok(Value::Int(items.len() as i64)),
                        _ => Err("len() expects a string or list".to_string()),
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
}
