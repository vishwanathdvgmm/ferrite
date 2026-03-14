use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::rc::Rc;

use crate::ast::*;
use crate::lexer::{FsPart, Lexer};
use crate::parser::Parser;

pub type Scope = Rc<RefCell<HashMap<String, Value>>>;
pub type Env = Vec<Scope>;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Fn {
        fname: Option<String>,
        params: Vec<String>,
        variadic: Option<String>,
        body: Vec<Stmt>,
        closure: Env,
    },
    Builtin(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{:.1}", n)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::List(l) => {
                write!(f, "[")?;
                for (i, v) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if let Value::Str(s) = v {
                        write!(f, "\"{}\"", s)?;
                    } else {
                        write!(f, "{}", v)?;
                    }
                }
                write!(f, "]")
            }
            Value::Map(m) => {
                write!(f, "{{")?;
                let mut pairs: Vec<_> = m.iter().collect();
                pairs.sort_by_key(|(k, _)| (*k).clone());
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if let Value::Str(s) = v {
                        write!(f, "\"{}\": \"{}\"", k, s)?;
                    } else {
                        write!(f, "\"{}\": {}", k, v)?;
                    }
                }
                write!(f, "}}")
            }
            Value::Fn {
                params, variadic, ..
            } => {
                let mut all = params.clone();
                if let Some(v) = variadic {
                    all.push(format!("...{}", v));
                }
                write!(f, "<fn({})>", all.join(", "))
            }
            Value::Builtin(n) => write!(f, "<builtin:{}>", n),
        }
    }
}

impl Value {
    fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Int(0) => false,
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Map(m) => !m.is_empty(),
            _ => true,
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::Null => "null",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            _ => "function",
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    fn to_map_key(&self) -> Option<String> {
        match self {
            Value::Str(s) => Some(s.clone()),
            Value::Int(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum Sig {
    Ret(Value),
    Brk,
    Cont,
    Throw(Value),
    Err(String),
}

impl Sig {
    fn err(s: impl Into<String>) -> Self {
        Sig::Err(s.into())
    }
}

// ================================================================
// SECTION 5 – INTERPRETER
// ================================================================
pub struct Interp {
    pub env: Env,
    pub current_line: u32,
    pub import_base: Option<PathBuf>,
    pub imported: Vec<PathBuf>,
    pub imported_std: Vec<String>,
}

impl Interp {
    pub fn new() -> Self {
        let mut g: HashMap<String, Value> = HashMap::new();
        for n in &[
            "len",
            "push",
            "pop",
            "str",
            "int",
            "float",
            "type",
            "range",
            "input",
            "sqrt",
            "abs",
            "max",
            "min",
            "floor",
            "ceil",
            "round",
            "assert",
            "keys",
            "values",
            "has_key",
            "delete",
            "sort",
            "reverse",
            "contains",
            "map",
            "filter",
            "reduce",
            "split",
            "join",
            "replace",
            "starts_with",
            "ends_with",
            "trim",
            "upper",
            "lower",
            "chars",
            "substr",
            "pow",
            "log",
            "log2",
            "log10",
            "sin",
            "cos",
            "tan",
            "atan",
            "atan2",
            "pi",
            "e",
            "inf",
            "format",
            "write",
            "exit",
            "enumerate",
            "zip",
            "range",
            "read_file",
            "write_file",
            "append_file",
            "file_exists",
        ] {
            g.insert(n.to_string(), Value::Builtin(n.to_string()));
        }
        g.insert("PI".into(), Value::Float(std::f64::consts::PI));
        g.insert("E".into(), Value::Float(std::f64::consts::E));
        g.insert("INF".into(), Value::Float(f64::INFINITY));
        Interp {
            env: vec![Rc::new(RefCell::new(g))],
            current_line: 0,
            import_base: None,
            imported: Vec::new(),
            imported_std: Vec::new(),
        }
    }

    fn get(&self, n: &str) -> Option<Value> {
        for s in self.env.iter().rev() {
            if let Some(v) = s.borrow().get(n) {
                return Some(v.clone());
            }
        }
        None
    }
    fn set(&self, n: &str, v: Value) {
        for s in self.env.iter().rev() {
            if s.borrow().contains_key(n) {
                s.borrow_mut().insert(n.to_string(), v);
                return;
            }
        }
        self.env
            .last()
            .unwrap()
            .borrow_mut()
            .insert(n.to_string(), v);
    }
    fn def(&self, n: &str, v: Value) {
        self.env
            .last()
            .unwrap()
            .borrow_mut()
            .insert(n.to_string(), v);
    }
    fn push_scope(&mut self) {
        self.env.push(Rc::new(RefCell::new(HashMap::new())));
    }
    fn pop_scope(&mut self) {
        self.env.pop();
    }

    // ── Builtins ──────────────────────────────────────────────────
    fn builtin(&mut self, name: &str, a: Vec<Value>) -> Result<Value, Sig> {
        let n = a.len();
        macro_rules! arity {
            ($k:expr) => {
                if n != $k {
                    return Err(Sig::err(format!(
                        "{}() expects {} arg(s), got {}",
                        name, $k, n
                    )));
                }
            };
        }
        macro_rules! e {
            ($s:expr) => {
                return Err(Sig::err($s))
            };
        }

        match name {
            "len" => {
                arity!(1);
                match &a[0] {
                    Value::List(l) => Ok(Value::Int(l.len() as i64)),
                    Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                    Value::Map(m) => Ok(Value::Int(m.len() as i64)),
                    v => e!(format!("len() not supported for {}", v.kind())),
                }
            }
            "push" => {
                arity!(2);
                match a[0].clone() {
                    Value::List(mut l) => {
                        l.push(a[1].clone());
                        Ok(Value::List(l))
                    }
                    v => e!(format!("push() needs list, got {}", v.kind())),
                }
            }
            "pop" => {
                arity!(1);
                match a[0].clone() {
                    Value::List(mut l) => Ok(l.pop().unwrap_or(Value::Null)),
                    v => e!(format!("pop() needs list, got {}", v.kind())),
                }
            }
            "str" => {
                arity!(1);
                Ok(Value::Str(a[0].to_string()))
            }
            "type" => {
                arity!(1);
                Ok(Value::Str(a[0].kind().to_string()))
            }
            "int" => {
                arity!(1);
                match &a[0] {
                    Value::Int(x) => Ok(Value::Int(*x)),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::Str(s) => s
                        .trim()
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| Sig::err(format!("Cannot parse \"{}\" as int", s))),
                    v => e!(format!("Cannot convert {} to int", v.kind())),
                }
            }
            "float" => {
                arity!(1);
                match &a[0] {
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::Int(x) => Ok(Value::Float(*x as f64)),
                    Value::Str(s) => s
                        .trim()
                        .parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| Sig::err(format!("Cannot parse \"{}\" as float", s))),
                    v => e!(format!("Cannot convert {} to float", v.kind())),
                }
            }
            "sqrt" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Float(f.sqrt()))
                    .ok_or_else(|| Sig::err("sqrt() needs a number"))
            }
            "abs" => {
                arity!(1);
                match a[0] {
                    Value::Int(x) => Ok(Value::Int(x.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => e!("abs() needs a number"),
                }
            }
            "floor" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Int(f.floor() as i64))
                    .ok_or_else(|| Sig::err("floor() needs a number"))
            }
            "ceil" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Int(f.ceil() as i64))
                    .ok_or_else(|| Sig::err("ceil() needs a number"))
            }
            "round" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Int(f.round() as i64))
                    .ok_or_else(|| Sig::err("round() needs a number"))
            }
            "max" => {
                if n < 1 {
                    e!("max() needs at least 1 arg");
                }
                let items: Vec<Value> = if n == 1 {
                    match a[0].clone() {
                        Value::List(l) => l,
                        v => vec![v],
                    }
                } else {
                    a.clone()
                };
                let mut best = items[0].clone();
                for v in &items[1..] {
                    if v.as_f64().ok_or_else(|| Sig::err("max() needs numbers"))?
                        > best
                            .as_f64()
                            .ok_or_else(|| Sig::err("max() needs numbers"))?
                    {
                        best = v.clone();
                    }
                }
                Ok(best)
            }
            "min" => {
                if n < 1 {
                    e!("min() needs at least 1 arg");
                }
                let items: Vec<Value> = if n == 1 {
                    match a[0].clone() {
                        Value::List(l) => l,
                        v => vec![v],
                    }
                } else {
                    a.clone()
                };
                let mut best = items[0].clone();
                for v in &items[1..] {
                    if v.as_f64().ok_or_else(|| Sig::err("min() needs numbers"))?
                        < best
                            .as_f64()
                            .ok_or_else(|| Sig::err("min() needs numbers"))?
                    {
                        best = v.clone();
                    }
                }
                Ok(best)
            }
            "range" => match n {
                1 => match a[0] {
                    Value::Int(x) => Ok(Value::List((0..x).map(Value::Int).collect())),
                    _ => e!("range() needs int"),
                },
                2 => match (&a[0], &a[1]) {
                    (Value::Int(x), Value::Int(y)) => {
                        Ok(Value::List((*x..*y).map(Value::Int).collect()))
                    }
                    _ => e!("range() needs ints"),
                },
                3 => match (&a[0], &a[1], &a[2]) {
                    (Value::Int(x), Value::Int(y), Value::Int(step)) => {
                        let mut v = Vec::new();
                        let mut i = *x;
                        while if *step > 0 { i < *y } else { i > *y } {
                            v.push(Value::Int(i));
                            i += step;
                        }
                        Ok(Value::List(v))
                    }
                    _ => e!("range() needs ints"),
                },
                _ => e!("range() takes 1-3 args"),
            },
            "input" => {
                let prompt = if n == 1 {
                    a[0].to_string()
                } else {
                    String::new()
                };
                print!("{}", prompt);
                io::stdout().flush().unwrap();
                let mut line = String::new();
                io::stdin().read_line(&mut line).unwrap();
                Ok(Value::Str(line.trim_end_matches('\n').to_string()))
            }
            "assert" => {
                if n < 1 || n > 2 {
                    e!("assert() takes 1 or 2 args");
                }
                if !a[0].truthy() {
                    e!(if n == 2 {
                        a[1].to_string()
                    } else {
                        "Assertion failed".into()
                    });
                }
                Ok(Value::Null)
            }
            "exit" => {
                std::process::exit(if n == 1 {
                    match a[0] {
                        Value::Int(c) => c as i32,
                        _ => 0,
                    }
                } else {
                    0
                });
            }
            // math
            "pow" => {
                arity!(2);
                let b = a[0]
                    .as_f64()
                    .ok_or_else(|| Sig::err("pow() needs numbers"))?;
                let x = a[1]
                    .as_f64()
                    .ok_or_else(|| Sig::err("pow() needs numbers"))?;
                Ok(Value::Float(b.powf(x)))
            }
            "log" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Float(f.ln()))
                    .ok_or_else(|| Sig::err("log() needs a number"))
            }
            "log2" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Float(f.log2()))
                    .ok_or_else(|| Sig::err("log2() needs a number"))
            }
            "log10" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Float(f.log10()))
                    .ok_or_else(|| Sig::err("log10() needs a number"))
            }
            "sin" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Float(f.sin()))
                    .ok_or_else(|| Sig::err("sin() needs a number"))
            }
            "cos" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Float(f.cos()))
                    .ok_or_else(|| Sig::err("cos() needs a number"))
            }
            "tan" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Float(f.tan()))
                    .ok_or_else(|| Sig::err("tan() needs a number"))
            }
            "atan" => {
                arity!(1);
                a[0].as_f64()
                    .map(|f| Value::Float(f.atan()))
                    .ok_or_else(|| Sig::err("atan() needs a number"))
            }
            "atan2" => {
                arity!(2);
                let y = a[0]
                    .as_f64()
                    .ok_or_else(|| Sig::err("atan2() needs numbers"))?;
                let x = a[1]
                    .as_f64()
                    .ok_or_else(|| Sig::err("atan2() needs numbers"))?;
                Ok(Value::Float(y.atan2(x)))
            }
            "pi" => {
                arity!(0);
                Ok(Value::Float(std::f64::consts::PI))
            }
            "e" => {
                arity!(0);
                Ok(Value::Float(std::f64::consts::E))
            }
            "inf" => {
                arity!(0);
                Ok(Value::Float(f64::INFINITY))
            }
            "format" => {
                if n < 1 {
                    e!("format() needs at least a template string");
                }
                let tmpl = match &a[0] {
                    Value::Str(s) => s.clone(),
                    _ => e!("format() first arg must be a string"),
                };
                let mut result = String::new();
                let mut idx = 1usize;
                let mut chars = tmpl.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '{' && chars.peek() == Some(&'}') {
                        chars.next();
                        if idx < n {
                            result.push_str(&a[idx].to_string());
                            idx += 1;
                        } else {
                            e!("format(): not enough arguments");
                        }
                    } else {
                        result.push(c);
                    }
                }
                Ok(Value::Str(result))
            }
            "write" => {
                arity!(1);
                print!("{}", a[0]);
                io::stdout().flush().unwrap();
                Ok(Value::Null)
            }
            // map ops
            "keys" => {
                arity!(1);
                match &a[0] {
                    Value::Map(m) => {
                        let mut ks: Vec<Value> = m.keys().map(|k| Value::Str(k.clone())).collect();
                        ks.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
                        Ok(Value::List(ks))
                    }
                    v => e!(format!("keys() needs a map, got {}", v.kind())),
                }
            }
            "values" => {
                arity!(1);
                match &a[0] {
                    Value::Map(m) => {
                        let mut p: Vec<_> = m.iter().collect();
                        p.sort_by_key(|(k, _)| (*k).clone());
                        Ok(Value::List(p.into_iter().map(|(_, v)| v.clone()).collect()))
                    }
                    v => e!(format!("values() needs a map, got {}", v.kind())),
                }
            }
            "has_key" => {
                arity!(2);
                match &a[0] {
                    Value::Map(m) => {
                        let k = a[1]
                            .to_map_key()
                            .ok_or_else(|| Sig::err("Map key must be string/int/bool"))?;
                        Ok(Value::Bool(m.contains_key(&k)))
                    }
                    v => e!(format!("has_key() needs a map, got {}", v.kind())),
                }
            }
            "delete" => {
                arity!(2);
                match a[0].clone() {
                    Value::Map(mut m) => {
                        let k = a[1]
                            .to_map_key()
                            .ok_or_else(|| Sig::err("Map key must be string/int/bool"))?;
                        m.remove(&k);
                        Ok(Value::Map(m))
                    }
                    v => e!(format!("delete() needs a map, got {}", v.kind())),
                }
            }
            // list ops
            "sort" => {
                arity!(1);
                match a[0].clone() {
                    Value::List(mut l) => {
                        let mut err: Option<String> = None;
                        l.sort_by(|a, b| match (a.as_f64(), b.as_f64()) {
                            (Some(x), Some(y)) => {
                                x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            _ => match (a, b) {
                                (Value::Str(x), Value::Str(y)) => x.cmp(y),
                                _ => {
                                    err = Some("sort() requires numbers or strings".into());
                                    std::cmp::Ordering::Equal
                                }
                            },
                        });
                        if let Some(e) = err {
                            return Err(Sig::err(e));
                        }
                        Ok(Value::List(l))
                    }
                    v => e!(format!("sort() needs a list, got {}", v.kind())),
                }
            }
            "reverse" => {
                arity!(1);
                match a[0].clone() {
                    Value::List(mut l) => {
                        l.reverse();
                        Ok(Value::List(l))
                    }
                    Value::Str(s) => Ok(Value::Str(s.chars().rev().collect())),
                    v => e!(format!("reverse() needs list or string, got {}", v.kind())),
                }
            }
            "contains" => {
                arity!(2);
                match &a[0] {
                    Value::List(l) => Ok(Value::Bool(l.iter().any(|v| self.eq_vals(v, &a[1])))),
                    Value::Str(s) => match &a[1] {
                        Value::Str(sub) => Ok(Value::Bool(s.contains(sub.as_str()))),
                        _ => e!("contains() on string needs a string needle"),
                    },
                    Value::Map(m) => {
                        let k = a[1]
                            .to_map_key()
                            .ok_or_else(|| Sig::err("Map key must be string/int/bool"))?;
                        Ok(Value::Bool(m.contains_key(&k)))
                    }
                    v => e!(format!("contains() not supported for {}", v.kind())),
                }
            }
            "map" => {
                if n != 2 {
                    e!("map() takes (list, fn)");
                }
                let list = match a[0].clone() {
                    Value::List(l) => l,
                    v => e!(format!("map() needs a list, got {}", v.kind())),
                };
                let func = a[1].clone();
                let mut out = Vec::new();
                for item in list {
                    out.push(self.call(func.clone(), vec![item])?);
                }
                Ok(Value::List(out))
            }
            "filter" => {
                if n != 2 {
                    e!("filter() takes (list, fn)");
                }
                let list = match a[0].clone() {
                    Value::List(l) => l,
                    v => e!(format!("filter() needs a list, got {}", v.kind())),
                };
                let func = a[1].clone();
                let mut out = Vec::new();
                for item in list {
                    if self.call(func.clone(), vec![item.clone()])?.truthy() {
                        out.push(item);
                    }
                }
                Ok(Value::List(out))
            }
            "reduce" => {
                if n < 2 || n > 3 {
                    e!("reduce() takes (list, fn) or (list, fn, init)");
                }
                let list = match a[0].clone() {
                    Value::List(l) => l,
                    v => e!(format!("reduce() needs a list, got {}", v.kind())),
                };
                let func = a[1].clone();
                let (mut acc, start) = if n == 3 {
                    (a[2].clone(), 0)
                } else if !list.is_empty() {
                    (list[0].clone(), 1)
                } else {
                    e!("reduce() on empty list needs initial value")
                };
                for item in list.into_iter().skip(start) {
                    acc = self.call(func.clone(), vec![acc, item])?;
                }
                Ok(acc)
            }
            "enumerate" => {
                arity!(1);
                let list = match a[0].clone() {
                    Value::List(l) => l,
                    v => e!(format!("enumerate() needs a list, got {}", v.kind())),
                };
                Ok(Value::List(
                    list.into_iter()
                        .enumerate()
                        .map(|(i, v)| Value::List(vec![Value::Int(i as i64), v]))
                        .collect(),
                ))
            }
            "zip" => {
                if n < 2 {
                    e!("zip() takes at least 2 lists");
                }
                let lists: Result<Vec<Vec<Value>>, Sig> = a
                    .iter()
                    .map(|x| match x {
                        Value::List(l) => Ok(l.clone()),
                        v => Err(Sig::err(format!("zip() needs lists, got {}", v.kind()))),
                    })
                    .collect();
                let lists = lists?;
                let min_len = lists.iter().map(|l| l.len()).min().unwrap_or(0);
                Ok(Value::List(
                    (0..min_len)
                        .map(|i| Value::List(lists.iter().map(|l| l[i].clone()).collect()))
                        .collect(),
                ))
            }
            // string ops
            "split" => {
                if n < 1 || n > 2 {
                    e!("split() takes (string) or (string, sep)");
                }
                let s = match &a[0] {
                    Value::Str(s) => s.clone(),
                    v => e!(format!("split() needs a string, got {}", v.kind())),
                };
                let parts: Vec<Value> = if n == 2 {
                    match &a[1] {
                        Value::Str(sep) => s
                            .split(sep.as_str())
                            .map(|p| Value::Str(p.to_string()))
                            .collect(),
                        _ => e!("split() separator must be a string"),
                    }
                } else {
                    s.split_whitespace()
                        .map(|p| Value::Str(p.to_string()))
                        .collect()
                };
                Ok(Value::List(parts))
            }
            "join" => {
                if n != 2 {
                    e!("join() takes (list, sep)");
                }
                let list = match &a[0] {
                    Value::List(l) => l,
                    v => e!(format!("join() needs a list, got {}", v.kind())),
                };
                let sep = match &a[1] {
                    Value::Str(s) => s.clone(),
                    _ => e!("join() separator must be a string"),
                };
                Ok(Value::Str(
                    list.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(&sep),
                ))
            }
            "replace" => {
                if n != 3 {
                    e!("replace() takes (string, from, to)");
                }
                match (&a[0], &a[1], &a[2]) {
                    (Value::Str(s), Value::Str(f2), Value::Str(t)) => {
                        Ok(Value::Str(s.replace(f2.as_str(), t.as_str())))
                    }
                    _ => e!("replace() needs three strings"),
                }
            }
            "starts_with" => {
                if n != 2 {
                    e!("starts_with() takes 2 args");
                }
                match (&a[0], &a[1]) {
                    (Value::Str(s), Value::Str(p)) => Ok(Value::Bool(s.starts_with(p.as_str()))),
                    _ => e!("starts_with() needs two strings"),
                }
            }
            "ends_with" => {
                if n != 2 {
                    e!("ends_with() takes 2 args");
                }
                match (&a[0], &a[1]) {
                    (Value::Str(s), Value::Str(p)) => Ok(Value::Bool(s.ends_with(p.as_str()))),
                    _ => e!("ends_with() needs two strings"),
                }
            }
            "trim" => {
                arity!(1);
                match &a[0] {
                    Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
                    v => e!(format!("trim() needs a string, got {}", v.kind())),
                }
            }
            "upper" => {
                arity!(1);
                match &a[0] {
                    Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
                    v => e!(format!("upper() needs a string, got {}", v.kind())),
                }
            }
            "lower" => {
                arity!(1);
                match &a[0] {
                    Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
                    v => e!(format!("lower() needs a string, got {}", v.kind())),
                }
            }
            "chars" => {
                arity!(1);
                match &a[0] {
                    Value::Str(s) => Ok(Value::List(
                        s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    )),
                    v => e!(format!("chars() needs a string, got {}", v.kind())),
                }
            }
            "substr" => {
                if n != 3 {
                    e!("substr(s, start, len) takes 3 args");
                }
                match (&a[0], &a[1], &a[2]) {
                    (Value::Str(s), Value::Int(start), Value::Int(len)) => {
                        let st = (*start).max(0) as usize;
                        let l = (*len).max(0) as usize;
                        Ok(Value::Str(s.chars().skip(st).take(l).collect()))
                    }
                    _ => e!("substr() needs (string, int, int)"),
                }
            }
            // file I/O
            "read_file" => {
                arity!(1);
                let p = match &a[0] {
                    Value::Str(s) => s.clone(),
                    _ => e!("read_file() needs a string path"),
                };
                std::fs::read_to_string(&p)
                    .map(Value::Str)
                    .map_err(|err| Sig::err(format!("read_file('{}'): {}", p, err)))
            }
            "write_file" => {
                if n != 2 {
                    e!("write_file() takes (path, content)");
                }
                let p = match &a[0] {
                    Value::Str(s) => s.clone(),
                    _ => e!("write_file() needs a string path"),
                };
                std::fs::write(&p, a[1].to_string())
                    .map(|_| Value::Null)
                    .map_err(|err| Sig::err(format!("write_file('{}'): {}", p, err)))
            }
            "append_file" => {
                if n != 2 {
                    e!("append_file() takes (path, content)");
                }
                let p = match &a[0] {
                    Value::Str(s) => s.clone(),
                    _ => e!("append_file() needs a string path"),
                };
                use std::io::Write as IoWrite;
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&p)
                    .map_err(|err| Sig::err(format!("append_file('{}'): {}", p, err)))?;
                f.write_all(a[1].to_string().as_bytes())
                    .map(|_| Value::Null)
                    .map_err(|err| Sig::err(format!("append_file('{}'): {}", p, err)))
            }
            "file_exists" => {
                arity!(1);
                let p = match &a[0] {
                    Value::Str(s) => s.clone(),
                    _ => e!("file_exists() needs a string path"),
                };
                Ok(Value::Bool(std::path::Path::new(&p).exists()))
            }
            _ => e!(format!("Unknown builtin '{}'", name)),
        }
    }

    // ── Eval ──────────────────────────────────────────────────────
    fn eval(&mut self, e: &Expr) -> Result<Value, Sig> {
        match e {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null),
            Expr::Ident(n) => self.get(n).ok_or_else(|| {
                Sig::err(format!(
                    "line {}: Undefined variable '{}'",
                    self.current_line, n
                ))
            }),
            Expr::List(xs) => {
                let mut v = Vec::new();
                for x in xs {
                    v.push(self.eval(x)?);
                }
                Ok(Value::List(v))
            }
            Expr::Map(pairs) => {
                let mut m = HashMap::new();
                for (k, v) in pairs {
                    let kv = self.eval(k)?;
                    let key = kv
                        .to_map_key()
                        .ok_or_else(|| Sig::err("Map key must be string, int, or bool"))?;
                    m.insert(key, self.eval(v)?);
                }
                Ok(Value::Map(m))
            }
            Expr::FStr(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        FsPart::Text(s) => result.push_str(s),
                        FsPart::Code(src) => {
                            let toks = Lexer::new(src)
                                .tokenize()
                                .map_err(|e| Sig::err(format!("In f-string: {}", e)))?;
                            let mut p = Parser::new(toks);
                            let expr = p
                                .parse_expr()
                                .map_err(|e| Sig::err(format!("In f-string: {}", e)))?;
                            result.push_str(&self.eval(&expr)?.to_string());
                        }
                    }
                }
                Ok(Value::Str(result))
            }
            Expr::BinOp { op, left, right } => {
                match op {
                    BinOp::And => {
                        let l = self.eval(left)?;
                        return if !l.truthy() {
                            Ok(Value::Bool(false))
                        } else {
                            self.eval(right)
                        };
                    }
                    BinOp::Or => {
                        let l = self.eval(left)?;
                        return if l.truthy() { Ok(l) } else { self.eval(right) };
                    }
                    BinOp::NullCoal => {
                        let l = self.eval(left)?;
                        return if matches!(l, Value::Null) {
                            self.eval(right)
                        } else {
                            Ok(l)
                        };
                    }
                    _ => {}
                }
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                self.binop(op, l, r)
            }
            Expr::Unary { op, expr } => {
                let v = self.eval(expr)?;
                match op {
                    UnOp::Neg => match v {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err(Sig::err("Unary '-' requires a number")),
                    },
                    UnOp::Not => Ok(Value::Bool(!v.truthy())),
                }
            }
            Expr::Call { func, args } => {
                let fv = self.eval(func)?;
                let mut avs = Vec::new();
                for a in args {
                    avs.push(self.eval(a)?);
                }
                self.call(fv, avs)
            }
            Expr::Index { obj, idx } => {
                let ov = self.eval(obj)?;
                let iv = self.eval(idx)?;
                match (ov, iv) {
                    (Value::List(l), Value::Int(i)) => {
                        let n = l.len() as i64;
                        let i = if i < 0 { n + i } else { i };
                        l.into_iter()
                            .nth(i as usize)
                            .ok_or_else(|| Sig::err(format!("Index {} out of bounds", i)))
                    }
                    (Value::Str(s), Value::Int(i)) => {
                        let ch: Vec<char> = s.chars().collect();
                        let n = ch.len() as i64;
                        let i = if i < 0 { n + i } else { i };
                        ch.get(i as usize)
                            .map(|c| Value::Str(c.to_string()))
                            .ok_or_else(|| Sig::err(format!("String index {} out of bounds", i)))
                    }
                    (Value::Map(m), k) => {
                        let key = k
                            .to_map_key()
                            .ok_or_else(|| Sig::err("Map key must be string/int/bool"))?;
                        Ok(m.get(&key).cloned().unwrap_or(Value::Null))
                    }
                    _ => Err(Sig::err("Invalid index operation")),
                }
            }
            Expr::Field { obj, name } => {
                let ov = self.eval(obj)?;
                match (&ov, name.as_str()) {
                    (Value::Str(s), "len") => Ok(Value::Int(s.chars().count() as i64)),
                    (Value::Str(s), "upper") => Ok(Value::Str(s.to_uppercase())),
                    (Value::Str(s), "lower") => Ok(Value::Str(s.to_lowercase())),
                    (Value::Str(s), "trim") => Ok(Value::Str(s.trim().to_string())),
                    (Value::Str(s), "chars") => Ok(Value::List(
                        s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    )),
                    (Value::List(l), "len") => Ok(Value::Int(l.len() as i64)),
                    (Value::Map(m), "len") => Ok(Value::Int(m.len() as i64)),
                    (Value::Map(m), field) => Ok(m.get(field).cloned().unwrap_or(Value::Null)),
                    _ => Err(Sig::err(format!("No property '{}' on {}", name, ov.kind()))),
                }
            }
            Expr::If { cond, then, else_ } => {
                let cv = self.eval(cond)?;
                self.push_scope();
                let r = if cv.truthy() {
                    self.exec_block(then)
                } else if let Some(eb) = else_ {
                    self.exec_block(eb)
                } else {
                    Ok(Value::Null)
                };
                self.pop_scope();
                r
            }
            Expr::Lambda {
                params,
                variadic,
                body,
            } => Ok(Value::Fn {
                fname: None,
                params: params.clone(),
                variadic: variadic.clone(),
                body: body.clone(),
                closure: self.env.clone(),
            }),
        }
    }

    fn call(&mut self, fv: Value, args: Vec<Value>) -> Result<Value, Sig> {
        match fv {
            Value::Builtin(n) => self.builtin(&n, args),
            Value::Fn {
                fname,
                params,
                variadic,
                body,
                closure,
            } => {
                let required = params.len();
                if variadic.is_none() && args.len() != required {
                    return Err(Sig::err(format!(
                        "Expected {} arg(s), got {}",
                        required,
                        args.len()
                    )));
                }
                if args.len() < required {
                    return Err(Sig::err(format!(
                        "Expected at least {} arg(s), got {}",
                        required,
                        args.len()
                    )));
                }
                // Swap in the closure — shared Rc refs so mutations are visible
                let saved = std::mem::replace(&mut self.env, closure);
                self.push_scope();
                for (p, v) in params.iter().zip(args.iter()) {
                    self.def(p, v.clone());
                }
                if let Some(ref vname) = variadic {
                    self.def(
                        vname,
                        Value::List(args.into_iter().skip(required).collect()),
                    );
                }
                // Self-reference for recursion
                if let Some(ref n) = fname {
                    let fn_closure = self.env[..self.env.len() - 1].to_vec();
                    self.def(
                        n,
                        Value::Fn {
                            fname: fname.clone(),
                            params: params.clone(),
                            variadic: variadic.clone(),
                            body: body.clone(),
                            closure: fn_closure,
                        },
                    );
                }
                let r = self.exec_block(&body);
                self.env = saved;
                match r {
                    Ok(_) => Ok(Value::Null),
                    Err(Sig::Ret(v)) => Ok(v),
                    Err(e) => Err(e),
                }
            }
            _ => Err(Sig::err("Attempted to call a non-function")),
        }
    }

    fn binop(&self, op: &BinOp, l: Value, r: Value) -> Result<Value, Sig> {
        use BinOp::*;
        let e = |s: &str| Err(Sig::err(s.to_string()));
        match op {
            Add => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(a + &b)),
                (Value::Str(a), b) => Ok(Value::Str(a + &b.to_string())),
                (Value::List(mut a), Value::List(b)) => {
                    a.extend(b);
                    Ok(Value::List(a))
                }
                (Value::Map(mut a), Value::Map(b)) => {
                    a.extend(b);
                    Ok(Value::Map(a))
                }
                (l, r) => Err(Sig::err(format!(
                    "Cannot add {} and {}",
                    l.kind(),
                    r.kind()
                ))),
            },
            Sub => self.num2(l, r, |a, b| a - b, |a, b| a - b, "subtract"),
            Mul => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * b as f64)),
                (Value::Str(s), Value::Int(n)) => Ok(Value::Str(s.repeat(n.max(0) as usize))),
                (l, r) => Err(Sig::err(format!(
                    "Cannot multiply {} and {}",
                    l.kind(),
                    r.kind()
                ))),
            },
            Div => {
                match &r {
                    Value::Int(0) => return e("Division by zero"),
                    Value::Float(f) if *f == 0.0 => return e("Division by zero"),
                    _ => {}
                }
                match (l, r) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Float(a as f64 / b as f64)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                    (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 / b)),
                    (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / b as f64)),
                    (l, r) => Err(Sig::err(format!(
                        "Cannot divide {} by {}",
                        l.kind(),
                        r.kind()
                    ))),
                }
            }
            IDiv => match (l, r) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        e("Integer division by zero")
                    } else {
                        Ok(Value::Int(a / b))
                    }
                }
                (Value::Float(a), Value::Float(b)) => Ok(Value::Int((a / b).floor() as i64)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Int((a as f64 / b).floor() as i64)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Int((a / b as f64).floor() as i64)),
                _ => e("Integer division requires numbers"),
            },
            Mod => match (l, r) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        e("Modulo by zero")
                    } else {
                        Ok(Value::Int(a % b))
                    }
                }
                _ => e("Modulo requires integers"),
            },
            Pow => {
                let base = l.as_f64().ok_or_else(|| Sig::err("** requires numbers"))?;
                let exp = r.as_f64().ok_or_else(|| Sig::err("** requires numbers"))?;
                if exp >= 0.0 && exp.fract() == 0.0 {
                    if let Value::Int(b) = &l {
                        return Ok(Value::Int(b.pow(exp as u32)));
                    }
                }
                Ok(Value::Float(base.powf(exp)))
            }
            Eq => Ok(Value::Bool(self.eq_vals(&l, &r))),
            Ne => Ok(Value::Bool(!self.eq_vals(&l, &r))),
            Lt => self.cmp(l, r, |a, b| a < b),
            Le => self.cmp(l, r, |a, b| a <= b),
            Gt => self.cmp(l, r, |a, b| a > b),
            Ge => self.cmp(l, r, |a, b| a >= b),
            And | Or | NullCoal => unreachable!(),
        }
    }

    fn num2(
        &self,
        l: Value,
        r: Value,
        fi: fn(i64, i64) -> i64,
        ff: fn(f64, f64) -> f64,
        op: &str,
    ) -> Result<Value, Sig> {
        match (l, r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(fi(a, b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(ff(a, b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(ff(a as f64, b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(ff(a, b as f64))),
            (l, r) => Err(Sig::err(format!(
                "Cannot {} {} and {}",
                op,
                l.kind(),
                r.kind()
            ))),
        }
    }
    fn cmp(&self, l: Value, r: Value, f: fn(f64, f64) -> bool) -> Result<Value, Sig> {
        if let (Value::Str(a), Value::Str(b)) = (&l, &r) {
            let n: f64 = match a.as_str().cmp(b.as_str()) {
                std::cmp::Ordering::Less => -1.0,
                std::cmp::Ordering::Equal => 0.0,
                std::cmp::Ordering::Greater => 1.0,
            };
            return Ok(Value::Bool(f(n, 0.0)));
        }
        let a = l
            .as_f64()
            .ok_or_else(|| Sig::err(format!("Cannot compare {}", l.kind())))?;
        let b = r
            .as_f64()
            .ok_or_else(|| Sig::err(format!("Cannot compare {}", r.kind())))?;
        Ok(Value::Bool(f(a, b)))
    }
    fn eq_vals(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) == *y,
            (Value::Float(x), Value::Int(y)) => *x == (*y as f64),
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Null, Value::Null) => true,
            (Value::List(x), Value::List(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(a, b)| self.eq_vals(a, b))
            }
            (Value::Map(x), Value::Map(y)) => {
                x.len() == y.len()
                    && x.iter()
                        .all(|(k, v)| y.get(k).map_or(false, |u| self.eq_vals(v, u)))
            }
            _ => false,
        }
    }

    fn match_pattern(
        &mut self,
        pat: &MatchPat,
        val: &Value,
    ) -> Result<Option<Option<(String, Value)>>, Sig> {
        match pat {
            MatchPat::Wildcard => Ok(Some(None)),
            MatchPat::Binding(n) => Ok(Some(Some((n.clone(), val.clone())))),
            MatchPat::Literal(e) => {
                let pv = self.eval(e)?;
                Ok(if self.eq_vals(&pv, val) {
                    Some(None)
                } else {
                    None
                })
            }
            MatchPat::Range(s, e) => {
                let sv = self.eval(s)?;
                let ev = self.eval(e)?;
                let v = val
                    .as_f64()
                    .ok_or_else(|| Sig::err("match range requires a number"))?;
                let s2 = sv
                    .as_f64()
                    .ok_or_else(|| Sig::err("match range start must be a number"))?;
                let e2 = ev
                    .as_f64()
                    .ok_or_else(|| Sig::err("match range end must be a number"))?;
                Ok(if v >= s2 && v < e2 { Some(None) } else { None })
            }
        }
    }

    // ── Import Resolution ─────────────────────────────────────────
    fn resolve_import(&self, path: &str) -> Option<PathBuf> {
        let mut p = PathBuf::from(path);
        if p.extension().is_none() {
            p.set_extension("fe");
        }

        // 1. Relative to current file
        if let Some(ref base) = self.import_base {
            let candidate = base.join(&p);
            if candidate.exists() {
                return Some(candidate);
            }
        } else if p.exists() {
            return Some(p.clone());
        }

        None
    }

    // ── Exec ──────────────────────────────────────────────────────
    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Value, Sig> {
        let mut last = Value::Null;
        for s in stmts {
            last = self.exec(s)?;
        }
        Ok(last)
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<Value, Sig> {
        match stmt {
            Stmt::Expr(e) => self.eval(e),
            Stmt::Break => Err(Sig::Brk),
            Stmt::Continue => Err(Sig::Cont),

            Stmt::Let { name, value } => {
                let v = self.eval(value)?;
                self.def(name, v);
                Ok(Value::Null)
            }

            Stmt::LetList { items, value } => {
                let val = self.eval(value)?;
                let list = match val {
                    Value::List(l) => l,
                    v => {
                        return Err(Sig::err(format!(
                            "List unpack requires a list, got {}",
                            v.kind()
                        )))
                    }
                };
                let mut idx = 0usize;
                for item in items {
                    match item {
                        UnpackItem::Name(n) => {
                            self.def(n, list.get(idx).cloned().unwrap_or(Value::Null));
                            idx += 1;
                        }
                        UnpackItem::Rest(n) => {
                            self.def(n, Value::List(list.into_iter().skip(idx).collect()));
                            return Ok(Value::Null);
                        }
                    }
                }
                Ok(Value::Null)
            }

            Stmt::LetMap { names, value } => {
                let val = self.eval(value)?;
                let map = match val {
                    Value::Map(m) => m,
                    v => {
                        return Err(Sig::err(format!(
                            "Map unpack requires a map, got {}",
                            v.kind()
                        )))
                    }
                };
                for name in names {
                    self.def(name, map.get(name).cloned().unwrap_or(Value::Null));
                }
                Ok(Value::Null)
            }

            Stmt::CompoundAssign { target, op, value } => {
                let rhs = self.eval(value)?;
                match target {
                    Lhs::Ident(n) => {
                        let cur = self.get(n).ok_or_else(|| {
                            Sig::err(format!(
                                "line {}: Undefined variable '{}' — use 'let' first",
                                self.current_line, n
                            ))
                        })?;
                        let result = self.binop(op, cur, rhs)?;
                        self.set(n, result);
                    }
                    Lhs::Index { obj, idx } => {
                        let iv = self.eval(idx)?;
                        let var = lhs_root_name(obj).ok_or_else(|| {
                            Sig::err("Complex compound index assignment not yet supported")
                        })?;
                        let container = self
                            .get(&var)
                            .ok_or_else(|| Sig::err(format!("Undefined variable '{}'", var)))?;
                        match container {
                            Value::List(mut l) => {
                                let i = match iv {
                                    Value::Int(i) => i,
                                    _ => return Err(Sig::err("List index must be an integer")),
                                };
                                let n = l.len() as i64;
                                let i = if i < 0 { n + i } else { i } as usize;
                                if i < l.len() {
                                    let cur = l[i].clone();
                                    l[i] = self.binop(op, cur, rhs)?;
                                    self.set(&var, Value::List(l));
                                } else {
                                    return Err(Sig::err(format!("Index {} out of bounds", i)));
                                }
                            }
                            Value::Map(mut m) => {
                                let k = iv
                                    .to_map_key()
                                    .ok_or_else(|| Sig::err("Map key must be string/int/bool"))?;
                                let cur = m.get(&k).cloned().unwrap_or(Value::Int(0));
                                m.insert(k, self.binop(op, cur, rhs)?);
                                self.set(&var, Value::Map(m));
                            }
                            _ => {
                                return Err(Sig::err(
                                    "Compound index assignment requires a list or map",
                                ))
                            }
                        }
                    }
                    Lhs::Field { obj, name: field } => {
                        let var = lhs_root_name(obj).ok_or_else(|| {
                            Sig::err("Complex compound field assignment not yet supported")
                        })?;
                        let container = self
                            .get(&var)
                            .ok_or_else(|| Sig::err(format!("Undefined variable '{}'", var)))?;
                        match container {
                            Value::Map(mut m) => {
                                let cur = m.get(field).cloned().unwrap_or(Value::Int(0));
                                m.insert(field.clone(), self.binop(op, cur, rhs)?);
                                self.set(&var, Value::Map(m));
                            }
                            _ => return Err(Sig::err("Compound field assignment requires a map")),
                        }
                    }
                }
                Ok(Value::Null)
            }

            Stmt::Assign { target, value } => {
                let v = self.eval(value)?;
                match target {
                    Lhs::Ident(n) => {
                        if self.get(n).is_none() {
                            return Err(Sig::err(format!(
                                "line {}: Undefined variable '{}' — use 'let' first",
                                self.current_line, n
                            )));
                        }
                        self.set(n, v);
                    }
                    Lhs::Index { obj, idx } => {
                        let iv = self.eval(idx)?;
                        let name = lhs_root_name(obj).ok_or_else(|| {
                            Sig::err("Complex index assignment not yet supported")
                        })?;
                        let container = self
                            .get(&name)
                            .ok_or_else(|| Sig::err(format!("Undefined variable '{}'", name)))?;
                        match container {
                            Value::List(mut l) => {
                                let i = match iv {
                                    Value::Int(i) => i,
                                    _ => return Err(Sig::err("List index must be an integer")),
                                };
                                let n = l.len() as i64;
                                let i = if i < 0 { n + i } else { i } as usize;
                                if i < l.len() {
                                    l[i] = v;
                                    self.set(&name, Value::List(l));
                                } else {
                                    return Err(Sig::err(format!("Index {} out of bounds", i)));
                                }
                            }
                            Value::Map(mut m) => {
                                let k = iv
                                    .to_map_key()
                                    .ok_or_else(|| Sig::err("Map key must be string/int/bool"))?;
                                m.insert(k, v);
                                self.set(&name, Value::Map(m));
                            }
                            _ => return Err(Sig::err("Index assignment requires a list or map")),
                        }
                    }
                    Lhs::Field { obj, name: field } => {
                        let var = lhs_root_name(obj).ok_or_else(|| {
                            Sig::err("Complex field assignment not yet supported")
                        })?;
                        let container = self
                            .get(&var)
                            .ok_or_else(|| Sig::err(format!("Undefined variable '{}'", var)))?;
                        match container {
                            Value::Map(mut m) => {
                                m.insert(field.clone(), v);
                                self.set(&var, Value::Map(m));
                            }
                            _ => return Err(Sig::err("Field assignment requires a map")),
                        }
                    }
                }
                Ok(Value::Null)
            }

            Stmt::Print(e) => {
                let v = self.eval(e)?;
                println!("{}", v);
                Ok(Value::Null)
            }
            Stmt::Write(e) => {
                let v = self.eval(e)?;
                print!("{}", v);
                io::stdout().flush().unwrap();
                Ok(Value::Null)
            }

            Stmt::Return(e) => {
                let v = if let Some(ex) = e {
                    self.eval(ex)?
                } else {
                    Value::Null
                };
                Err(Sig::Ret(v))
            }
            Stmt::Throw(e) => {
                let v = self.eval(e)?;
                Err(Sig::Throw(v))
            }

            Stmt::While { cond, body } => {
                loop {
                    if !self.eval(cond)?.truthy() {
                        break;
                    }
                    self.push_scope();
                    let r = self.exec_block(body);
                    self.pop_scope();
                    match r {
                        Ok(_) => {}
                        Err(Sig::Brk) => break,
                        Err(Sig::Cont) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Null)
            }

            Stmt::For { var, iter, body } => {
                let items = match self.eval(iter)? {
                    Value::List(l) => l,
                    Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    Value::Map(m) => {
                        let mut ks: Vec<_> = m.keys().cloned().collect();
                        ks.sort();
                        ks.into_iter().map(Value::Str).collect()
                    }
                    _ => return Err(Sig::err("'for' requires a list, string, or map")),
                };
                'fl: for item in items {
                    self.push_scope();
                    self.def(var, item);
                    let r = self.exec_block(body);
                    self.pop_scope();
                    match r {
                        Ok(_) => {}
                        Err(Sig::Brk) => break 'fl,
                        Err(Sig::Cont) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Null)
            }

            Stmt::FnDef {
                name,
                params,
                variadic,
                body,
            } => {
                let f = Value::Fn {
                    fname: Some(name.clone()),
                    params: params.clone(),
                    variadic: variadic.clone(),
                    body: body.clone(),
                    closure: self.env.clone(),
                };
                self.def(name, f);
                Ok(Value::Null)
            }

            Stmt::Match { subject, arms } => {
                let val = self.eval(subject)?;
                for arm in arms {
                    if let Some(binding) = self.match_pattern(&arm.pattern, &val)? {
                        self.push_scope();
                        if let Some((n, v)) = binding {
                            self.def(&n, v);
                        }
                        let r = self.exec_block(&arm.body);
                        self.pop_scope();
                        return match r {
                            Ok(v) => Ok(v),
                            Err(Sig::Ret(v)) => Err(Sig::Ret(v)),
                            Err(e) => Err(e),
                        };
                    }
                }
                Ok(Value::Null)
            }

            Stmt::TryCatch {
                body,
                catch_var,
                catch_body,
            } => {
                self.push_scope();
                let r = self.exec_block(body);
                self.pop_scope();
                match r {
                    Ok(v) => Ok(v),
                    Err(Sig::Err(msg)) => {
                        self.push_scope();
                        self.def(catch_var, Value::Str(msg));
                        let r2 = self.exec_block(catch_body);
                        self.pop_scope();
                        r2
                    }
                    Err(Sig::Throw(val)) => {
                        self.push_scope();
                        self.def(catch_var, val);
                        let r2 = self.exec_block(catch_body);
                        self.pop_scope();
                        r2
                    }
                    Err(other) => Err(other), // Ret/Brk/Cont propagate normally
                }
            }

            Stmt::Import { path } => {
                // 1. Try internal standard library
                if let Some(embedded_src) = crate::stdlib::get_stdlib_module(path) {
                    if self.imported_std.contains(&path.to_string()) {
                        return Ok(Value::Null);
                    }
                    self.imported_std.push(path.to_string());

                    self.run_src(embedded_src).map_err(Sig::err)?;
                    return Ok(Value::Null);
                }

                // 2. Try filesystem
                let full = self
                    .resolve_import(path)
                    .ok_or_else(|| Sig::err(format!("import '{}': module not found", path)))?;
                // Guard against double-import
                let canonical = full.canonicalize().unwrap_or_else(|_| full.clone());
                if self.imported.contains(&canonical) {
                    return Ok(Value::Null);
                }
                self.imported.push(canonical);
                let src = std::fs::read_to_string(&full)
                    .map_err(|e| Sig::err(format!("import '{}': {}", path, e)))?;
                let old_base = self.import_base.clone();
                self.import_base = full.parent().map(|p| p.to_path_buf());
                let result = self.run_src(&src);
                self.import_base = old_base;
                result.map_err(Sig::err)?;
                Ok(Value::Null)
            }
        }
    }

    fn run_src(&mut self, src: &str) -> Result<(), String> {
        let tokens = Lexer::new(src).tokenize()?;
        let stmts = Parser::new(tokens).parse_program()?;
        for s in &stmts {
            match self.exec(s) {
                Ok(_) => {}
                Err(Sig::Ret(_)) => {
                    return Err(format!(
                        "line {}: 'return' outside of a function",
                        self.current_line
                    ))
                }
                Err(Sig::Brk) => {
                    return Err(format!(
                        "line {}: 'break' outside of a loop",
                        self.current_line
                    ))
                }
                Err(Sig::Cont) => {
                    return Err(format!(
                        "line {}: 'continue' outside of a loop",
                        self.current_line
                    ))
                }
                Err(Sig::Throw(v)) => {
                    return Err(format!(
                        "line {}: Uncaught exception: {}",
                        self.current_line, v
                    ))
                }
                Err(Sig::Err(e)) => return Err(e),
            }
        }
        Ok(())
    }

    pub fn run(&mut self, src: &str) -> Result<(), String> {
        self.run_src(src)
    }
}

fn lhs_root_name(e: &Expr) -> Option<String> {
    match e {
        Expr::Ident(n) => Some(n.clone()),
        Expr::Index { obj, .. } | Expr::Field { obj, .. } => lhs_root_name(obj),
        _ => None,
    }
}
