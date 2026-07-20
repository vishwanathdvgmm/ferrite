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
    Module(String, HashMap<String, Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unit => {
                write!(f, "()")?;
                Ok(())
            }
            Value::Int(n) => {
                write!(f, "{}", n)?;
                Ok(())
            }
            Value::Float(n) => {
                write!(f, "{}", n)?;
                Ok(())
            }
            Value::Bool(b) => {
                write!(f, "{}", b)?;
                Ok(())
            }
            Value::String(s) => {
                write!(f, "{}", s)?;
                Ok(())
            }
            Value::Func(decl) => {
                write!(f, "<fun {}>", decl.name)?;
                Ok(())
            }
            Value::Closure(params, _, _) => {
                let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
                write!(f, "<closure ({})>", param_names.join(", "))?;
                Ok(())
            }
            Value::Builtin(name) => {
                write!(f, "<builtin {}>", name)?;
                Ok(())
            }
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
                write!(f, " }}")?;
                Ok(())
            }
            Value::Enum(enum_name, variant, values) => {
                if values.is_empty() {
                    write!(f, "{}::{}", enum_name, variant)?;
                    Ok(())
                } else {
                    write!(f, "{}::{}(", enum_name, variant)?;
                    for (i, v) in values.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, ")")?;
                    Ok(())
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
                write!(f, "]")?;
                Ok(())
            }
            Value::Tensor(data, shape) => {
                let shape_str: Vec<String> = shape.iter().map(|d| d.to_string()).collect();
                write!(
                    f,
                    "Tensor({}, shape=[{}])",
                    data.len(),
                    shape_str.join(", ")
                )?;
                Ok(())
            }
            Value::BoundMethod(_, func) => {
                write!(f, "<bound method {}>", func)?;
                Ok(())
            }
            Value::Module(name, _) => {
                write!(f, "<module {}>", name)?;
                Ok(())
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
