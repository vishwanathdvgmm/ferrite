use super::environment::Environment;
use super::value::Value;
use crate::ast::*;
use std::collections::HashMap;

pub struct Interpreter {
    pub env: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
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

        Self { env }
    }

    pub fn run_program(&mut self, program: &Program) -> Result<Value, String> {
        // First pass: register all functions, constants, etc.
        for decl in &program.decls {
            match decl {
                TopDecl::Func(f) => {
                    self.env.declare(f.name.clone(), Value::Func(f.clone()));
                }
                TopDecl::Constant(c) => {
                    let val = self.eval_expr(&c.value)?;
                    self.env.declare(c.name.clone(), val);
                }
                TopDecl::Impl(imp) => {
                    for m in &imp.methods {
                        // Very simplified dispatch: we just inject impl methods into global scope by name.
                        // In a real VM, method dispatch is bound to the type.
                        let mut fdecl = FuncDecl {
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
                        self.env.declare(m.name.clone(), Value::Func(fdecl));
                    }
                }
                TopDecl::Enum(e) => {
                    for v in &e.variants {
                        // Register enum variant constructors
                        // For a tree-walk interpreter, we just treat them as generic builtins
                        self.env.declare(
                            v.name.clone(),
                            Value::Builtin(format!("enum_{}::{}", e.name, v.name)),
                        );
                    }
                }
                _ => {}
            }
        }

        // Run main
        if let Ok(Value::Func(main_func)) = self.env.get("main") {
            self.eval_block(&main_func.body)
        } else {
            Err("No main function found.".to_string())
        }
    }

    fn eval_block(&mut self, block: &Block) -> Result<Value, String> {
        self.env.enter_scope();
        let mut ret = Value::Unit;
        for stmt in &block.stmts {
            match self.eval_stmt(stmt)? {
                Some(r) => {
                    ret = r;
                    break; // Early return encountered
                }
                None => {}
            }
        }
        self.env.exit_scope();
        Ok(ret)
    }

    /// Returns Some(Value) if a return statement was executed, otherwise None
    fn eval_stmt(&mut self, stmt: &Stmt) -> Result<Option<Value>, String> {
        match stmt {
            Stmt::Keep { name, value, .. } | Stmt::Param { name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.env.declare(name.clone(), val);
                Ok(None)
            }
            Stmt::ExprStmt(expr) => {
                self.eval_expr(expr)?;
                Ok(None)
            }
            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    Ok(Some(self.eval_expr(expr)?))
                } else {
                    Ok(Some(Value::Unit))
                }
            }
            Stmt::If {
                condition,
                then_block,
                elif_branches,
                else_block,
                ..
            } => {
                let cond_val = self.eval_expr(condition)?;
                if let Value::Bool(true) = cond_val {
                    let ret = self.eval_block(then_block)?;
                    // We need a better way to propagate returns through blocks.
                    // For now, if the block returns something other than unit, we propagate it.
                    if ret != Value::Unit {
                        return Ok(Some(ret));
                    }
                    return Ok(None);
                }

                for (elif_cond, elif_block) in elif_branches {
                    let elif_cond_val = self.eval_expr(elif_cond)?;
                    if let Value::Bool(true) = elif_cond_val {
                        let ret = self.eval_block(elif_block)?;
                        if ret != Value::Unit {
                            return Ok(Some(ret));
                        }
                        return Ok(None);
                    }
                }

                if let Some(else_b) = else_block {
                    let ret = self.eval_block(else_b)?;
                    if ret != Value::Unit {
                        return Ok(Some(ret));
                    }
                }
                Ok(None)
            }
            Stmt::While {
                condition, body, ..
            } => {
                loop {
                    let cond_val = self.eval_expr(condition)?;
                    if let Value::Bool(false) = cond_val {
                        break;
                    }
                    let ret = self.eval_block(body)?;
                    if ret != Value::Unit {
                        return Ok(Some(ret));
                    }
                }
                Ok(None)
            }
            Stmt::For { .. } | Stmt::Match { .. } | Stmt::Select { .. } => {
                Err("For/Match/Select not yet implemented in Tree-Walk Interpreter".to_string())
            }
            Stmt::InferBlock(block) | Stmt::TrainBlock(block) => {
                let ret = self.eval_block(block)?;
                if ret != Value::Unit {
                    return Ok(Some(ret));
                }
                Ok(None)
            }
            Stmt::Stop(_) | Stmt::Skip(_) => Err("Stop/Skip not implemented".to_string()),
        }
    }

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
                        BinOp::Div => Ok(Value::Int(a / b)),
                        BinOp::Mod => Ok(Value::Int(a % b)),
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
                        BinOp::Div => Ok(Value::Float(a / b)),
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
                    UnaryOp::Await => Ok(val), // Dummy await
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
                            self.env
                                .declare(param.name.clone(), evaluated_args[i].clone());
                        }
                        let ret = self.eval_block(&decl.body)?;
                        self.env.exit_scope();
                        Ok(ret)
                    }
                    _ => Err("Attempt to call a non-function".to_string()),
                }
            }
            Expr::Assign { target, value, .. } => {
                let val = self.eval_expr(value)?;
                if let Expr::Ident(name, _) = &**target {
                    self.env.assign(name, val.clone())?;
                    Ok(val)
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
                match obj {
                    Value::Group(_, fields) => {
                        if let Some(val) = fields.get(field) {
                            Ok(val.clone())
                        } else {
                            Err(format!("Field '{}' not found", field))
                        }
                    }
                    _ => Err("Field access on non-group type".to_string()),
                }
            }
            _ => Err("Expression not supported in simple interpreter".to_string()),
        }
    }

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
                if let Some(Value::String(s)) = args.get(0) {
                    Ok(Value::Int(s.len() as i64))
                } else {
                    Err("len() expects a string".to_string())
                }
            }
            "str" => {
                if let Some(val) = args.get(0) {
                    Ok(Value::String(val.to_string()))
                } else {
                    Err("str() expects an argument".to_string())
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
