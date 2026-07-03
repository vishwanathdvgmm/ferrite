use super::environment::Environment;
use crate::ast::{Expr, FuncDecl, Param};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Func(Rc<FuncDecl>),
    /// A closure captures its lexical environment at creation time.
    Closure(Vec<Param>, Rc<Expr>, Rc<Environment>),
    Builtin(String),
    Group(String, HashMap<String, Value>),
    Enum(String, String, Vec<Value>),
    List(Vec<Value>),
    Tensor(Vec<f64>, Vec<i64>), // basic flat tensor for interpreter fallback
    /// A method bound to its receiver object (self, method_func).
    BoundMethod(Box<Value>, Box<Value>),
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
            Value::Closure(params, _, _) => {
                let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
                write!(f, "<closure ({})>", param_names.join(", "))
            }
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
            Value::List(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Tensor(data, shape) => {
                write!(f, "Tensor(shape={:?}, data={:?})", shape, data)
            }
            Value::BoundMethod(_, method) => {
                write!(f, "<bound method {:?}>", method)
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
