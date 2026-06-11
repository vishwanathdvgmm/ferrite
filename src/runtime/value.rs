use crate::ast::FuncDecl;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Func(FuncDecl),
    Builtin(String),
    Group(String, HashMap<String, Value>),
    Enum(String, String, Vec<Value>),
    Tensor(Vec<f64>, Vec<i64>), // basic flat tensor for interpreter fallback
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Func(decl) => write!(f, "<fun {}>", decl.name),
            Value::Builtin(name) => write!(f, "<builtin {}>", name),
            Value::Group(name, fields) => {
                write!(f, "{} {{ ", name)?;
                let mut first = true;
                for (k, v) in fields {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                    first = false;
                }
                write!(f, " }}")
            }
            Value::Enum(enum_name, variant, values) => {
                if values.is_empty() {
                    write!(f, "{}::{}", enum_name, variant)
                } else {
                    write!(f, "{}::{}(", enum_name, variant)?;
                    for (i, v) in values.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, ")")
                }
            }
            Value::Tensor(data, shape) => {
                write!(f, "Tensor(shape={:?}, data={:?})", shape, data)
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Unit, Value::Unit) => true,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            // Function and complex equality is intentionally simplified for the interpreter
            _ => false,
        }
    }
}
