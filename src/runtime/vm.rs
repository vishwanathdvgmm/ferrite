use crate::codegen::compiler::Compiler;
use crate::codegen::opcodes::{Chunk, Opcode};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::runtime::Value;
use std::collections::HashMap;

struct ExceptionHandler {
    handler_ip: usize,
    stack_depth: usize,
}

pub struct VM {
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    handler_stack: Vec<ExceptionHandler>,
    last_thrown: Option<Value>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            chunk: Chunk::new(),
            ip: 0,
            stack: Vec::with_capacity(256),
            globals: VM::init_globals(),
            handler_stack: Vec::new(),
            last_thrown: None,
        }
    }

    fn init_globals() -> HashMap<String, Value> {
        let mut g = HashMap::new();
        let builtins = [
            "len", "push", "pop", "str", "int", "float", "type", "range", "input",
            "sqrt", "abs", "max", "min", "floor", "ceil", "round", "assert",
            "keys", "values", "has_key", "delete", "sort", "reverse", "contains",
            "map", "filter", "reduce", "split", "join", "replace",
            "starts_with", "ends_with", "trim", "upper", "lower", "chars", "substr",
            "pow", "log", "log2", "log10", "sin", "cos", "tan", "atan", "atan2",
            "format", "write", "exit", "enumerate", "zip",
            "read_file", "write_file", "append_file", "file_exists",
            "__slice",
        ];
        for n in &builtins {
            g.insert(n.to_string(), Value::Builtin(n.to_string()));
        }
        g.insert("PI".into(), Value::Float(std::f64::consts::PI));
        g.insert("E".into(), Value::Float(std::f64::consts::E));
        g.insert("INF".into(), Value::Float(f64::INFINITY));
        g
    }

    pub fn interpret(&mut self, chunk: Chunk) -> Result<Value, String> {
        self.chunk = chunk;
        self.ip = 0;
        self.run()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap_or(Value::Null)
    }

    fn error(&mut self, err_val: Value) -> Result<Value, String> {
        if !self.handler_stack.is_empty() {
            let handler = self.handler_stack.pop().unwrap();
            self.ip = handler.handler_ip;
            self.stack.truncate(handler.stack_depth);
            self.push(err_val);
            Ok(Value::Null)
        } else {
            let msg = err_val.to_string();
            self.last_thrown = Some(err_val);
            Err(msg)
        }
    }

    fn run(&mut self) -> Result<Value, String> {
        loop {
            if self.ip >= self.chunk.code.len() {
                break;
            }
            let instruction = self.chunk.code[self.ip];
            self.ip += 1;

            match instruction {
                Opcode::Constant(idx) => {
                    let constant = self.chunk.constants[idx].clone();
                    self.push(constant);
                }
                Opcode::Null => self.push(Value::Null),
                Opcode::True => self.push(Value::Bool(true)),
                Opcode::False => self.push(Value::Bool(false)),
                Opcode::Pop => { self.pop(); }
                Opcode::Swap => {
                    let a = self.pop();
                    let b = self.pop();
                    self.push(a);
                    self.push(b);
                }
                Opcode::Dup => {
                    let val = self.stack.last().cloned().unwrap_or(Value::Null);
                    self.push(val);
                }

                // ── Variables ──────────────────────────────────────────
                Opcode::DefineGlobal(idx) => {
                    let name = self.chunk.constants[idx].to_string();
                    let val = self.pop();
                    self.globals.insert(name, val);
                }
                Opcode::GetGlobal(idx) => {
                    let name = self.chunk.constants[idx].to_string();
                    if let Some(val) = self.globals.get(&name) {
                        self.push(val.clone());
                    } else {
                        let err = Value::Str(format!("Undefined variable '{}'", name));
                        if self.error(err).is_ok() {
                            continue;
                        } else {
                            return Err(format!("Undefined variable '{}'", name));
                        }
                    }
                }
                Opcode::SetGlobal(idx) => {
                    let name = self.chunk.constants[idx].to_string();
                    let val = self.stack.last().cloned().unwrap_or(Value::Null);
                    self.globals.insert(name, val);
                }
                Opcode::GetLocal(idx) => {
                    let val = self.stack.get(idx).cloned().unwrap_or(Value::Null);
                    self.push(val);
                }
                Opcode::SetLocal(idx) => {
                    let val = self.stack.last().cloned().unwrap_or(Value::Null);
                    if idx < self.stack.len() {
                        self.stack[idx] = val;
                    }
                }
                Opcode::CaptureLocal => {
                    let val = self.pop();
                    let name_val = self.pop();
                    let name = name_val.to_string();
                    if let Some(Value::Fn { captures, .. }) = self.stack.last_mut() {
                        captures.borrow_mut().insert(name, val);
                    }
                }

                // ── Control Flow ──────────────────────────────────────
                Opcode::JumpIfFalse(offset) => {
                    let val = self.pop();
                    if !val.truthy() {
                        self.ip = offset;
                    }
                }
                Opcode::JumpIfTrue(offset) => {
                    let val = self.pop();
                    if val.truthy() {
                        self.ip = offset;
                    }
                }
                Opcode::JumpIfNull(offset) => {
                    let val = self.stack.last().cloned().unwrap_or(Value::Null);
                    if matches!(val, Value::Null) {
                        self.pop();
                        self.ip = offset;
                    }
                }
                Opcode::Jump(offset) => {
                    self.ip = offset;
                }
                Opcode::Loop(offset) => {
                    self.ip = offset;
                }

                // ── Comparison ────────────────────────────────────────
                Opcode::Equal => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::Bool(self.vals_equal(&a, &b)));
                }
                Opcode::NotEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::Bool(!self.vals_equal(&a, &b)));
                }
                Opcode::Greater => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a.as_f64(), b.as_f64()) {
                        (Some(x), Some(y)) => self.push(Value::Bool(x > y)),
                        _ => return Err("Operands must be numbers for >".into()),
                    }
                }
                Opcode::GreaterEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a.as_f64(), b.as_f64()) {
                        (Some(x), Some(y)) => self.push(Value::Bool(x >= y)),
                        _ => return Err("Operands must be numbers for >=".into()),
                    }
                }
                Opcode::Less => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a.as_f64(), b.as_f64()) {
                        (Some(x), Some(y)) => self.push(Value::Bool(x < y)),
                        _ => return Err("Operands must be numbers for <".into()),
                    }
                }
                Opcode::LessEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a.as_f64(), b.as_f64()) {
                        (Some(x), Some(y)) => self.push(Value::Bool(x <= y)),
                        _ => return Err("Operands must be numbers for <=".into()),
                    }
                }

                // ── Arithmetic ────────────────────────────────────────
                Opcode::Add => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x + y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x + y)),
                        (Value::Int(x), Value::Float(y)) => self.push(Value::Float(x as f64 + y)),
                        (Value::Float(x), Value::Int(y)) => self.push(Value::Float(x + y as f64)),
                        (Value::Str(x), Value::Str(y)) => self.push(Value::Str(x + &y)),
                        (Value::Str(x), y) => self.push(Value::Str(x + &y.to_string())),
                        (Value::List(mut x), Value::List(y)) => { x.extend(y); self.push(Value::List(x)); }
                        _ => return Err("Invalid operands for +".into()),
                    }
                }
                Opcode::Subtract => {
                    let b = self.pop();
                    let a = self.pop();
                    match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x - y)),
                        _ => match (a.as_f64(), b.as_f64()) {
                            (Some(x), Some(y)) => self.push(Value::Float(x - y)),
                            _ => return Err("Invalid operands for -".into()),
                        }
                    }
                }
                Opcode::Multiply => {
                    let b = self.pop();
                    let a = self.pop();
                    match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x * y)),
                        _ => match (a.as_f64(), b.as_f64()) {
                            (Some(x), Some(y)) => self.push(Value::Float(x * y)),
                            _ => return Err("Invalid operands for *".into()),
                        }
                    }
                }
                Opcode::Divide => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a.as_f64(), b.as_f64()) {
                        (Some(x), Some(y)) => {
                            if y == 0.0 {
                                let err = Value::Str("Division by zero".into());
                                if self.error(err).is_ok() { continue; }
                                else { return Err("Division by zero".into()); }
                            }
                            self.push(Value::Float(x / y));
                        }
                        _ => return Err("Invalid operands for /".into()),
                    }
                }
                Opcode::Modulus => {
                    let b = self.pop();
                    let a = self.pop();
                    match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x % y)),
                        _ => match (a.as_f64(), b.as_f64()) {
                            (Some(x), Some(y)) => self.push(Value::Float(x % y)),
                            _ => return Err("Invalid operands for %".into()),
                        }
                    }
                }
                Opcode::IntDivide => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a.as_f64(), b.as_f64()) {
                        (Some(x), Some(y)) => self.push(Value::Int((x / y).floor() as i64)),
                        _ => return Err("Invalid operands for //".into()),
                    }
                }
                Opcode::Power => {
                    let b = self.pop();
                    let a = self.pop();
                    match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) if *y >= 0 => {
                            self.push(Value::Int((*x as f64).powi(*y as i32) as i64));
                        }
                        _ => match (a.as_f64(), b.as_f64()) {
                            (Some(x), Some(y)) => self.push(Value::Float(x.powf(y))),
                            _ => return Err("Invalid operands for **".into()),
                        }
                    }
                }
                Opcode::Negate => {
                    let a = self.pop();
                    match a {
                        Value::Int(n) => self.push(Value::Int(-n)),
                        Value::Float(n) => self.push(Value::Float(-n)),
                        _ => return Err("Cannot negate non-number".into()),
                    }
                }
                Opcode::Not => {
                    let a = self.pop();
                    self.push(Value::Bool(!a.truthy()));
                }

                // ── Collections ───────────────────────────────────────
                Opcode::BuildList(count) => {
                    let mut list = Vec::with_capacity(count);
                    for _ in 0..count {
                        list.push(self.pop());
                    }
                    list.reverse();
                    self.push(Value::List(list));
                }
                Opcode::BuildMap(count) => {
                    let mut pairs = Vec::with_capacity(count);
                    for _ in 0..count {
                        let v = self.pop();
                        let k = self.pop();
                        pairs.push((k, v));
                    }
                    pairs.reverse();
                    let mut map = HashMap::new();
                    for (k, v) in pairs {
                        if let Some(key) = k.to_map_key() {
                            map.insert(key, v);
                        } else {
                            return Err(format!("Invalid map key: {}", k));
                        }
                    }
                    self.push(Value::Map(map));
                }
                Opcode::IndexGet => {
                    let idx = self.pop();
                    let obj = self.pop();
                    match (obj, idx) {
                        (Value::List(l), Value::Int(i)) => {
                            let index = if i < 0 { l.len() as i64 + i } else { i } as usize;
                            self.push(l.get(index).cloned().unwrap_or(Value::Null));
                        }
                        (Value::Map(m), val) => {
                            if let Some(k) = val.to_map_key() {
                                self.push(m.get(&k).cloned().unwrap_or(Value::Null));
                            } else {
                                return Err(format!("Invalid map key: {}", val));
                            }
                        }
                        (Value::Str(s), Value::Int(i)) => {
                            let index = if i < 0 { s.len() as i64 + i } else { i } as usize;
                            self.push(
                                s.chars()
                                    .nth(index)
                                    .map(|c| Value::Str(c.to_string()))
                                    .unwrap_or(Value::Null),
                            );
                        }
                        _ => return Err("Indexing not supported for this type".into()),
                    }
                }
                Opcode::IndexSet => {
                    let val = self.pop();
                    let idx = self.pop();
                    let obj = self.pop();
                    match (obj, idx) {
                        (Value::List(mut l), Value::Int(i)) => {
                            let index = if i < 0 { l.len() as i64 + i } else { i } as usize;
                            if index < l.len() {
                                l[index] = val;
                            } else {
                                while l.len() <= index {
                                    l.push(Value::Null);
                                }
                                l[index] = val;
                            }
                            // We need to write back — but in a stack machine, the object was consumed.
                            // The compiler must handle this by re-storing.
                        }
                        (Value::Map(mut m), key_val) => {
                            if let Some(k) = key_val.to_map_key() {
                                m.insert(k, val);
                                // same note about write-back
                            }
                        }
                        _ => return Err("IndexSet not supported for this type".into()),
                    }
                }
                Opcode::FieldGet(idx) => {
                    let name = self.chunk.constants[idx].to_string();
                    let obj = self.pop();
                    match obj {
                        Value::Map(m) => {
                            self.push(m.get(&name).cloned().unwrap_or(Value::Null));
                        }
                        _ => return Err(format!("Cannot access field '{}' on {}", name, obj.kind())),
                    }
                }
                Opcode::FieldSet(idx) => {
                    let name = self.chunk.constants[idx].to_string();
                    let val = self.pop();
                    let obj = self.pop();
                    match obj {
                        Value::Map(mut m) => {
                            m.insert(name, val);
                            self.push(Value::Map(m));
                        }
                        _ => return Err("Cannot set field on non-map".into()),
                    }
                }

                // ── Functions ─────────────────────────────────────────
                Opcode::Call(arg_count) => {
                    let mut args = Vec::with_capacity(arg_count);
                    for _ in 0..arg_count {
                        args.push(self.pop());
                    }
                    args.reverse();
                    let callee = self.pop();
                    match callee {
                        Value::Builtin(name) => match self.call_builtin(&name, args) {
                            Ok(val) => self.push(val),
                            Err(msg) => {
                                let err = Value::Str(msg.clone());
                                if self.error(err).is_ok() {
                                    continue;
                                } else {
                                    return Err(msg);
                                }
                            }
                        },
                        Value::Fn {
                            params,
                            variadic,
                            chunk,
                            captures,
                            ..
                        } => {
                            let mut nested_vm = VM::new();
                            nested_vm.globals = self.globals.clone();
                            // Load captures
                            for (k, v) in captures.borrow().iter() {
                                nested_vm.globals.insert(k.clone(), v.clone());
                            }

                            // Push positional args
                            let param_count = params.len();
                            for i in 0..param_count {
                                nested_vm.push(args.get(i).cloned().unwrap_or(Value::Null));
                            }

                            // If variadic, collect remaining args into a list
                            if variadic.is_some() {
                                let rest: Vec<Value> = if args.len() > param_count {
                                    args[param_count..].to_vec()
                                } else {
                                    Vec::new()
                                };
                                nested_vm.push(Value::List(rest));
                            }

                            match nested_vm.interpret(chunk) {
                                Ok(v) => {
                                    self.globals = nested_vm.globals.clone();
                                    let mut caps = captures.borrow_mut();
                                    for k in caps.keys().cloned().collect::<Vec<_>>() {
                                        if let Some(new_val) = self.globals.get(&k) {
                                            caps.insert(k, new_val.clone());
                                        }
                                    }
                                    self.push(v);
                                }
                                Err(msg) => {
                                    self.globals = nested_vm.globals.clone();
                                    let mut caps = captures.borrow_mut();
                                    for k in caps.keys().cloned().collect::<Vec<_>>() {
                                        if let Some(new_val) = self.globals.get(&k) {
                                            caps.insert(k, new_val.clone());
                                        }
                                    }
                                    let err = nested_vm.last_thrown.take()
                                        .unwrap_or_else(|| Value::Str(msg.clone()));
                                    if self.error(err).is_ok() {
                                        continue;
                                    } else {
                                        return Err(msg);
                                    }
                                }
                            }
                        }
                        _ => return Err("Attempted to call non-function".into()),
                    }
                }
                Opcode::Return => {
                    return Ok(self.pop());
                }

                // ── I/O ───────────────────────────────────────────────
                Opcode::Print => {
                    println!("{}", self.pop());
                }
                Opcode::Throw => {
                    let err = self.pop();
                    let display = err.to_string();
                    if self.error(err).is_ok() {
                        continue;
                    } else {
                        return Err(format!("Uncaught exception: {}", display));
                    }
                }

                // ── Exception Handling ─────────────────────────────────
                Opcode::BeginTry(offset) => {
                    self.handler_stack.push(ExceptionHandler {
                        handler_ip: offset,
                        stack_depth: self.stack.len(),
                    });
                }
                Opcode::EndTry => {
                    self.handler_stack.pop();
                }
                Opcode::Import => {
                    let path_val = self.pop();
                    let path = path_val.to_string();

                    if let Some(src) = crate::stdlib::get_stdlib_module(&path) {
                        let tokens = Lexer::new(src).tokenize()?;
                        let stmts = Parser::new(tokens).parse_program()?;

                        let mut compiler = Compiler::new();
                        compiler.compile(&stmts)?;

                        let mut nested_vm = VM::new();
                        nested_vm.globals = self.globals.clone();
                        nested_vm.interpret(compiler.chunk)?;
                        self.globals = nested_vm.globals;
                    } else {
                        return Err(format!("Module '{}' not found", path));
                    }
                }
            }
        }
        Ok(Value::Null)
    }

    // ══════════════════════════════════════════════════════════════════
    //  Built-in Functions
    // ══════════════════════════════════════════════════════════════════

    fn call_builtin(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            // ── Type Conversion ───────────────────────────────────
            "int" => {
                let a = args.first().ok_or("int() needs 1 arg")?;
                match a {
                    Value::Int(n) => Ok(Value::Int(*n)),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                    Value::Str(s) => s
                        .trim()
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| format!("Cannot parse \"{}\" as int", s)),
                    _ => Err("Cannot convert to int".into()),
                }
            }
            "float" => {
                let a = args.first().ok_or("float() needs 1 arg")?;
                match a {
                    Value::Int(n) => Ok(Value::Float(*n as f64)),
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::Str(s) => s
                        .trim()
                        .parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| format!("Cannot parse \"{}\" as float", s)),
                    _ => Err("Cannot convert to float".into()),
                }
            }
            "str" => {
                if args.is_empty() {
                    Ok(Value::Str("".into()))
                } else {
                    Ok(Value::Str(args[0].to_string()))
                }
            }
            "type" => {
                let a = args.first().ok_or("type() needs 1 arg")?;
                Ok(Value::Str(a.kind().to_string()))
            }

            // ── Collections ───────────────────────────────────────
            "len" => {
                let a = args.first().ok_or("len() needs 1 arg")?;
                match a {
                    Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                    Value::List(l) => Ok(Value::Int(l.len() as i64)),
                    Value::Map(m) => Ok(Value::Int(m.len() as i64)),
                    _ => Err("len() not supported for this type".into()),
                }
            }
            "push" => {
                if args.len() < 2 {
                    return Err("push() needs 2 args".into());
                }
                match &args[0] {
                    Value::List(l) => {
                        let mut new_list = l.clone();
                        new_list.push(args[1].clone());
                        Ok(Value::List(new_list))
                    }
                    _ => Err("push() needs a list".into()),
                }
            }
            "pop" => {
                let a = args.first().ok_or("pop() needs 1 arg")?;
                match a {
                    Value::List(l) => {
                        let mut new_list = l.clone();
                        new_list.pop();
                        Ok(Value::List(new_list))
                    }
                    _ => Err("pop() needs a list".into()),
                }
            }
            "range" => {
                let (start, end, step) = match args.len() {
                    1 => (0i64, self.as_int(&args[0])?, 1i64),
                    2 => (self.as_int(&args[0])?, self.as_int(&args[1])?, 1i64),
                    3 => (
                        self.as_int(&args[0])?,
                        self.as_int(&args[1])?,
                        self.as_int(&args[2])?,
                    ),
                    _ => return Err("range() needs 1-3 args".into()),
                };
                let mut list = Vec::new();
                let mut i = start;
                if step > 0 {
                    while i < end { list.push(Value::Int(i)); i += step; }
                } else if step < 0 {
                    while i > end { list.push(Value::Int(i)); i += step; }
                }
                Ok(Value::List(list))
            }
            "sort" => {
                let a = args.first().ok_or("sort() needs 1 arg")?;
                match a {
                    Value::List(l) => {
                        let mut sorted = l.clone();
                        sorted.sort_by(|a, b| {
                            a.as_f64()
                                .unwrap_or(0.0)
                                .partial_cmp(&b.as_f64().unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        Ok(Value::List(sorted))
                    }
                    _ => Err("sort() needs a list".into()),
                }
            }
            "reverse" => {
                let a = args.first().ok_or("reverse() needs 1 arg")?;
                match a {
                    Value::List(l) => {
                        let mut rev = l.clone();
                        rev.reverse();
                        Ok(Value::List(rev))
                    }
                    Value::Str(s) => Ok(Value::Str(s.chars().rev().collect())),
                    _ => Err("reverse() not supported".into()),
                }
            }
            "contains" => {
                if args.len() < 2 {
                    return Err("contains() needs 2 args".into());
                }
                match &args[0] {
                    Value::List(l) => Ok(Value::Bool(
                        l.iter().any(|v| self.vals_equal(v, &args[1])),
                    )),
                    Value::Str(s) => match &args[1] {
                        Value::Str(sub) => Ok(Value::Bool(s.contains(sub.as_str()))),
                        _ => Ok(Value::Bool(false)),
                    },
                    Value::Map(m) => match &args[1] {
                        Value::Str(k) => Ok(Value::Bool(m.contains_key(k))),
                        _ => Ok(Value::Bool(false)),
                    },
                    _ => Err("contains() not supported".into()),
                }
            }
            "keys" => {
                let a = args.first().ok_or("keys() needs 1 arg")?;
                match a {
                    Value::Map(m) => Ok(Value::List(
                        m.keys().map(|k| Value::Str(k.clone())).collect(),
                    )),
                    _ => Err("keys() needs a map".into()),
                }
            }
            "values" => {
                let a = args.first().ok_or("values() needs 1 arg")?;
                match a {
                    Value::Map(m) => Ok(Value::List(m.values().cloned().collect())),
                    _ => Err("values() needs a map".into()),
                }
            }
            "has_key" => {
                if args.len() < 2 {
                    return Err("has_key() needs 2 args".into());
                }
                match &args[0] {
                    Value::Map(m) => {
                        let key = args[1].to_string();
                        Ok(Value::Bool(m.contains_key(&key)))
                    }
                    _ => Err("has_key() needs a map".into()),
                }
            }
            "delete" => {
                if args.len() < 2 {
                    return Err("delete() needs 2 args".into());
                }
                match &args[0] {
                    Value::Map(m) => {
                        let mut new_map = m.clone();
                        let key = args[1].to_string();
                        new_map.remove(&key);
                        Ok(Value::Map(new_map))
                    }
                    _ => Err("delete() needs a map".into()),
                }
            }
            "enumerate" => {
                let a = args.first().ok_or("enumerate() needs 1 arg")?;
                match a {
                    Value::List(l) => Ok(Value::List(
                        l.iter()
                            .enumerate()
                            .map(|(i, v)| Value::List(vec![Value::Int(i as i64), v.clone()]))
                            .collect(),
                    )),
                    _ => Err("enumerate() needs a list".into()),
                }
            }
            "zip" => {
                if args.len() < 2 {
                    return Err("zip() needs 2 args".into());
                }
                match (&args[0], &args[1]) {
                    (Value::List(a), Value::List(b)) => {
                        let pairs: Vec<Value> = a
                            .iter()
                            .zip(b.iter())
                            .map(|(x, y)| Value::List(vec![x.clone(), y.clone()]))
                            .collect();
                        Ok(Value::List(pairs))
                    }
                    _ => Err("zip() needs two lists".into()),
                }
            }

            // ── Higher-Order Functions ────────────────────────────
            "map" => {
                if args.len() < 2 {
                    return Err("map() needs 2 args".into());
                }
                let list = match &args[0] {
                    Value::List(l) => l.clone(),
                    _ => return Err("map() first arg must be a list".into()),
                };
                let func = args[1].clone();
                let mut result = Vec::new();
                for item in list {
                    let val = self.call_value(func.clone(), vec![item])?;
                    result.push(val);
                }
                Ok(Value::List(result))
            }
            "filter" => {
                if args.len() < 2 {
                    return Err("filter() needs 2 args".into());
                }
                let list = match &args[0] {
                    Value::List(l) => l.clone(),
                    _ => return Err("filter() first arg must be a list".into()),
                };
                let func = args[1].clone();
                let mut result = Vec::new();
                for item in list {
                    let val = self.call_value(func.clone(), vec![item.clone()])?;
                    if val.truthy() {
                        result.push(item);
                    }
                }
                Ok(Value::List(result))
            }
            "reduce" => {
                if args.len() < 3 {
                    return Err("reduce() needs 3 args (list, fn, initial)".into());
                }
                let list = match &args[0] {
                    Value::List(l) => l.clone(),
                    _ => return Err("reduce() first arg must be a list".into()),
                };
                let func = args[1].clone();
                let mut acc = args[2].clone();
                for item in list {
                    acc = self.call_value(func.clone(), vec![acc, item])?;
                }
                Ok(acc)
            }

            // ── String Functions ──────────────────────────────────
            "split" => {
                if args.len() < 2 {
                    return Err("split() needs 2 args".into());
                }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(sep)) => Ok(Value::List(
                        s.split(sep.as_str())
                            .map(|p| Value::Str(p.to_string()))
                            .collect(),
                    )),
                    _ => Err("split() needs strings".into()),
                }
            }
            "join" => {
                if args.len() < 2 {
                    return Err("join() needs 2 args".into());
                }
                match (&args[0], &args[1]) {
                    (Value::List(l), Value::Str(sep)) => {
                        let parts: Vec<String> = l.iter().map(|v| v.to_string()).collect();
                        Ok(Value::Str(parts.join(sep)))
                    }
                    _ => Err("join() needs a list and a string".into()),
                }
            }
            "replace" => {
                if args.len() < 3 {
                    return Err("replace() needs 3 args".into());
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::Str(s), Value::Str(from), Value::Str(to)) => {
                        Ok(Value::Str(s.replace(from.as_str(), to.as_str())))
                    }
                    _ => Err("replace() needs strings".into()),
                }
            }
            "starts_with" => {
                if args.len() < 2 { return Err("starts_with() needs 2 args".into()); }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(p)) => Ok(Value::Bool(s.starts_with(p.as_str()))),
                    _ => Err("starts_with() needs strings".into()),
                }
            }
            "ends_with" => {
                if args.len() < 2 { return Err("ends_with() needs 2 args".into()); }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(p)) => Ok(Value::Bool(s.ends_with(p.as_str()))),
                    _ => Err("ends_with() needs strings".into()),
                }
            }
            "trim" => {
                let a = args.first().ok_or("trim() needs 1 arg")?;
                match a {
                    Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
                    _ => Err("trim() needs a string".into()),
                }
            }
            "upper" => {
                let a = args.first().ok_or("upper() needs 1 arg")?;
                match a {
                    Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
                    _ => Err("upper() needs a string".into()),
                }
            }
            "lower" => {
                let a = args.first().ok_or("lower() needs 1 arg")?;
                match a {
                    Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
                    _ => Err("lower() needs a string".into()),
                }
            }
            "chars" => {
                let a = args.first().ok_or("chars() needs 1 arg")?;
                match a {
                    Value::Str(s) => Ok(Value::List(
                        s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    )),
                    _ => Err("chars() needs a string".into()),
                }
            }
            "substr" => {
                if args.len() < 3 {
                    return Err("substr() needs 3 args".into());
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::Str(s), Value::Int(start), Value::Int(len)) => {
                        let start = *start as usize;
                        let len = *len as usize;
                        let result: String = s.chars().skip(start).take(len).collect();
                        Ok(Value::Str(result))
                    }
                    _ => Err("substr() needs (string, int, int)".into()),
                }
            }
            "format" => {
                // format(template, ...args) — replaces {} in order
                if args.is_empty() {
                    return Err("format() needs at least 1 arg".into());
                }
                let template = args[0].to_string();
                let mut result = template;
                for arg in &args[1..] {
                    if let Some(pos) = result.find("{}") {
                        result = format!("{}{}{}", &result[..pos], arg, &result[pos + 2..]);
                    }
                }
                Ok(Value::Str(result))
            }

            // ── Math ──────────────────────────────────────────────
            "sqrt" => {
                let a = args.first().ok_or("sqrt() needs 1 arg")?;
                Ok(Value::Float(a.as_f64().ok_or("sqrt() needs a number")?.sqrt()))
            }
            "abs" => {
                let a = args.first().ok_or("abs() needs 1 arg")?;
                match a {
                    Value::Int(n) => Ok(Value::Int(n.abs())),
                    Value::Float(n) => Ok(Value::Float(n.abs())),
                    _ => Err("abs() needs a number".into()),
                }
            }
            "max" => {
                if args.len() == 1 {
                    // max(list)
                    match &args[0] {
                        Value::List(l) => {
                            if l.is_empty() { return Err("max() on empty list".into()); }
                            let mut best = l[0].as_f64().unwrap_or(f64::NEG_INFINITY);
                            let mut best_val = l[0].clone();
                            for v in &l[1..] {
                                let f = v.as_f64().unwrap_or(f64::NEG_INFINITY);
                                if f > best { best = f; best_val = v.clone(); }
                            }
                            Ok(best_val)
                        }
                        _ => Ok(args[0].clone()),
                    }
                } else {
                    // max(a, b)
                    let a = args[0].as_f64().ok_or("max() needs numbers")?;
                    let b = args[1].as_f64().ok_or("max() needs numbers")?;
                    if a >= b { Ok(args[0].clone()) } else { Ok(args[1].clone()) }
                }
            }
            "min" => {
                if args.len() == 1 {
                    match &args[0] {
                        Value::List(l) => {
                            if l.is_empty() { return Err("min() on empty list".into()); }
                            let mut best = l[0].as_f64().unwrap_or(f64::INFINITY);
                            let mut best_val = l[0].clone();
                            for v in &l[1..] {
                                let f = v.as_f64().unwrap_or(f64::INFINITY);
                                if f < best { best = f; best_val = v.clone(); }
                            }
                            Ok(best_val)
                        }
                        _ => Ok(args[0].clone()),
                    }
                } else {
                    let a = args[0].as_f64().ok_or("min() needs numbers")?;
                    let b = args[1].as_f64().ok_or("min() needs numbers")?;
                    if a <= b { Ok(args[0].clone()) } else { Ok(args[1].clone()) }
                }
            }
            "floor" => {
                let a = args.first().ok_or("floor() needs 1 arg")?;
                Ok(Value::Int(a.as_f64().ok_or("floor() needs a number")?.floor() as i64))
            }
            "ceil" => {
                let a = args.first().ok_or("ceil() needs 1 arg")?;
                Ok(Value::Int(a.as_f64().ok_or("ceil() needs a number")?.ceil() as i64))
            }
            "round" => {
                let a = args.first().ok_or("round() needs 1 arg")?;
                Ok(Value::Int(a.as_f64().ok_or("round() needs a number")?.round() as i64))
            }
            "pow" => {
                if args.len() < 2 { return Err("pow() needs 2 args".into()); }
                let base = args[0].as_f64().ok_or("pow() needs numbers")?;
                let exp = args[1].as_f64().ok_or("pow() needs numbers")?;
                Ok(Value::Float(base.powf(exp)))
            }
            "log" => {
                let a = args.first().ok_or("log() needs 1 arg")?;
                Ok(Value::Float(a.as_f64().ok_or("log() needs a number")?.ln()))
            }
            "log2" => {
                let a = args.first().ok_or("log2() needs 1 arg")?;
                Ok(Value::Float(a.as_f64().ok_or("log2() needs a number")?.log2()))
            }
            "log10" => {
                let a = args.first().ok_or("log10() needs 1 arg")?;
                Ok(Value::Float(a.as_f64().ok_or("log10() needs a number")?.log10()))
            }
            "sin" => {
                let a = args.first().ok_or("sin() needs 1 arg")?;
                Ok(Value::Float(a.as_f64().ok_or("sin() needs a number")?.sin()))
            }
            "cos" => {
                let a = args.first().ok_or("cos() needs 1 arg")?;
                Ok(Value::Float(a.as_f64().ok_or("cos() needs a number")?.cos()))
            }
            "tan" => {
                let a = args.first().ok_or("tan() needs 1 arg")?;
                Ok(Value::Float(a.as_f64().ok_or("tan() needs a number")?.tan()))
            }
            "atan" => {
                let a = args.first().ok_or("atan() needs 1 arg")?;
                Ok(Value::Float(a.as_f64().ok_or("atan() needs a number")?.atan()))
            }
            "atan2" => {
                if args.len() < 2 { return Err("atan2() needs 2 args".into()); }
                let y = args[0].as_f64().ok_or("atan2() needs numbers")?;
                let x = args[1].as_f64().ok_or("atan2() needs numbers")?;
                Ok(Value::Float(y.atan2(x)))
            }

            // ── I/O Functions ─────────────────────────────────────
            "input" => {
                if !args.is_empty() {
                    print!("{}", args[0]);
                }
                use std::io::{self, Write};
                io::stdout().flush().ok();
                let mut buf = String::new();
                io::stdin().read_line(&mut buf).ok();
                Ok(Value::Str(buf.trim_end_matches('\n').trim_end_matches('\r').to_string()))
            }
            "write" => {
                // Print without newline
                if !args.is_empty() {
                    print!("{}", args[0]);
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
                Ok(Value::Null)
            }
            "exit" => {
                let code = if args.is_empty() {
                    0
                } else {
                    self.as_int(&args[0]).unwrap_or(0) as i32
                };
                std::process::exit(code);
            }
            "assert" => {
                if args.is_empty() {
                    return Err("assert() needs at least 1 arg".into());
                }
                if !args[0].truthy() {
                    let msg = if args.len() > 1 { args[1].to_string() } else { "Assertion failed".to_string() };
                    return Err(msg);
                }
                Ok(Value::Null)
            }

            // ── File I/O ──────────────────────────────────────────
            "read_file" => {
                let path = args.first().ok_or("read_file() needs 1 arg")?.to_string();
                std::fs::read_to_string(&path)
                    .map(Value::Str)
                    .map_err(|e| format!("read_file error: {}", e))
            }
            "write_file" => {
                if args.len() < 2 { return Err("write_file() needs 2 args".into()); }
                let path = args[0].to_string();
                let content = args[1].to_string();
                std::fs::write(&path, &content)
                    .map(|_| Value::Null)
                    .map_err(|e| format!("write_file error: {}", e))
            }
            "append_file" => {
                if args.len() < 2 { return Err("append_file() needs 2 args".into()); }
                let path = args[0].to_string();
                let content = args[1].to_string();
                use std::io::Write;
                let mut f = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&path)
                    .map_err(|e| format!("append_file error: {}", e))?;
                f.write_all(content.as_bytes())
                    .map(|_| Value::Null)
                    .map_err(|e| format!("append_file error: {}", e))
            }
            "file_exists" => {
                let path = args.first().ok_or("file_exists() needs 1 arg")?.to_string();
                Ok(Value::Bool(std::path::Path::new(&path).exists()))
            }

            // ── Internal Helpers ──────────────────────────────────
            "__slice" => {
                // __slice(list, start) — returns list[start..]
                if args.len() < 2 { return Err("__slice() needs 2 args".into()); }
                match (&args[0], &args[1]) {
                    (Value::List(l), Value::Int(start)) => {
                        let start = *start as usize;
                        if start >= l.len() {
                            Ok(Value::List(Vec::new()))
                        } else {
                            Ok(Value::List(l[start..].to_vec()))
                        }
                    }
                    _ => Err("__slice() needs (list, int)".into()),
                }
            }

            _ => Err(format!("Builtin '{}' not yet implemented in VM", name)),
        }
    }

    // ── Helper: call a Value::Fn or Value::Builtin ────────────────
    fn call_value(&mut self, callee: Value, args: Vec<Value>) -> Result<Value, String> {
        match callee {
            Value::Builtin(name) => self.call_builtin(&name, args),
            Value::Fn {
                params,
                variadic,
                chunk,
                ..
            } => {
                let mut nested_vm = VM::new();
                nested_vm.globals = self.globals.clone();

                let param_count = params.len();
                for i in 0..param_count {
                    nested_vm.push(args.get(i).cloned().unwrap_or(Value::Null));
                }
                if variadic.is_some() {
                    let rest: Vec<Value> = if args.len() > param_count {
                        args[param_count..].to_vec()
                    } else {
                        Vec::new()
                    };
                    nested_vm.push(Value::List(rest));
                }

                let result = nested_vm.interpret(chunk)?;
                self.globals = nested_vm.globals;
                Ok(result)
            }
            _ => Err("Attempted to call non-function".into()),
        }
    }

    fn vals_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) == *y,
            (Value::Float(x), Value::Int(y)) => *x == (*y as f64),
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }

    fn as_int(&self, v: &Value) -> Result<i64, String> {
        match v {
            Value::Int(n) => Ok(*n),
            Value::Float(f) => Ok(*f as i64),
            _ => Err("Expected a number".into()),
        }
    }
}
