use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

use std::path::PathBuf;
use std::rc::Rc;

pub mod vm;

use crate::lexer::Lexer;
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
        chunk: crate::codegen::opcodes::Chunk,
        captures: Rc<RefCell<HashMap<String, Value>>>,
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
    pub fn truthy(&self) -> bool {
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
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::Null => "null",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Fn { .. } => "function",
            _ => "builtin",
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn to_map_key(&self) -> Option<String> {
        match self {
            Value::Str(s) => Some(s.clone()),
            Value::Int(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }
}

fn run_src(src: &str) -> Result<(), String> {
    let tokens = Lexer::new(src).tokenize()?;
    let stmts = Parser::new(tokens).parse_program()?;

    // Phase 2a: Static Semantic Analysis Pass
    crate::semantic::Resolver::new().resolve(&stmts)?;

    // Phase 2c & 2d: Bytecode Compilation and VM Execution
    let mut compiler = crate::codegen::compiler::Compiler::new();
    compiler.compile(&stmts)?;

    let mut vm = crate::runtime::vm::VM::new();
    vm.interpret(compiler.chunk)?;

    Ok(())
}

pub fn run(src: &str, _import_base: Option<PathBuf>) -> Result<(), String> {
    run_src(src)
}
