pub mod tensor;

use crate::ast::{PrimType, ShapeDim as AstShapeDim, Type as AstType};
use crate::errors::{DiagnosticBag, Span};
use std::collections::HashMap;
use std::fmt;
use tensor::{ShapeDim, TensorShape};

// ── Resolved Type System ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Tensor(Box<Type>, TensorShape),
    Named(String),   // User-defined enum or group type
    Generic(String), // Resolved generic parameter
    Unit,            // () function return
    Never,           // Type of `stop` or `skip` or divergent branches
    Error,           // Represents a failed type check to prevent cascading errors
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Tensor(elem, shape) => write!(f, "Tensor<{}, {}>", elem, shape),
            Type::Named(name) => write!(f, "{}", name),
            Type::Generic(name) => write!(f, "{}", name),
            Type::Unit => write!(f, "()"),
            Type::Never => write!(f, "!"),
            Type::Error => write!(f, "<error>"),
        }
    }
}

// ── Type Environment ─────────────────────────────────────────────

pub struct TypeEnv<'a> {
    pub diag: &'a mut DiagnosticBag,
    scopes: Vec<HashMap<String, Type>>,
    // Global struct/enum declarations for type validation
    pub types: HashMap<String, Type>,
}

impl<'a> TypeEnv<'a> {
    pub fn new(diag: &'a mut DiagnosticBag) -> Self {
        Self {
            diag,
            scopes: vec![HashMap::new()],
            types: HashMap::new(),
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare_var(&mut self, name: String, ty: Type, span: &Span) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(&name) {
                self.diag.error(
                    span.clone(),
                    format!("Variable '{}' is already defined in this scope.", name),
                );
            } else {
                scope.insert(name, ty);
            }
        }
    }

    pub fn lookup_var(&mut self, name: &str, span: &Span) -> Type {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return ty.clone();
            }
        }
        self.diag
            .error(span.clone(), format!("Undefined variable '{}'.", name));
        Type::Error
    }

    pub fn declare_type(&mut self, name: String, ty: Type, span: &Span) {
        if self.types.contains_key(&name) {
            self.diag
                .error(span.clone(), format!("Type '{}' is already defined.", name));
        } else {
            self.types.insert(name, ty);
        }
    }

    // ── AST Resolution ──────────────────────────────────────────

    /// Resolves an AST Type into a strictly typed canonical `Type`.
    pub fn resolve_ast_type(&mut self, ast_ty: &AstType) -> Type {
        match ast_ty {
            AstType::Primitive(prim, _) => match prim {
                PrimType::Int => Type::Int,
                PrimType::Float => Type::Float,
                PrimType::Bool => Type::Bool,
                PrimType::String => Type::String,
            },
            AstType::Tensor { elem, shape, span } => {
                let elem_ty = self.resolve_ast_type(elem);
                // Ensure tensors only contain primitives (float/int) per canonical ML specs
                if !matches!(elem_ty, Type::Float | Type::Int) && elem_ty != Type::Error {
                    self.diag.error(
                        span.clone(),
                        format!(
                            "Tensors can only contain 'int' or 'float', not '{}'",
                            elem_ty
                        ),
                    );
                }

                let dims = shape
                    .iter()
                    .map(|d| match d {
                        AstShapeDim::Const(n) => ShapeDim::Const(*n),
                        AstShapeDim::Symbolic(s) => ShapeDim::Symbolic(s.clone()),
                    })
                    .collect();

                Type::Tensor(Box::new(elem_ty), TensorShape::new(dims))
            }
            AstType::Named(name, _span) => {
                /* For phase 1 we strictly accept defined names, generic stubs, or defer if not found */
                Type::Named(name.clone())
            }
            AstType::Generic {
                name,
                args: _,
                span: _,
            } => {
                // E.g., Option<int>
                Type::Named(name.clone())
            }
        }
    }

    // ── Unification ──────────────────────────────────────────────

    /// Structurally unify two types, emitting a diagnostic if they diverge.
    /// Strict checking: NO implicit coercion, NO implicit broadcasting.
    pub fn unify(&mut self, expected: &Type, actual: &Type, span: &Span) -> bool {
        if expected == &Type::Error || actual == &Type::Error {
            return true; // Suppress cascade errors
        }
        if expected == &Type::Never || actual == &Type::Never {
            return true; // Never unifies with everything
        }

        match (expected, actual) {
            (Type::Tensor(e_elem, e_shape), Type::Tensor(a_elem, a_shape)) => {
                if !self.unify(e_elem, a_elem, span) {
                    return false;
                }
                if !e_shape.exact_match(a_shape) {
                    self.diag.error(
                        span.clone(),
                        format!(
                            "Tensor shape mismatch: expected shape {}, found shape {}.\n\
                            Implicit broadcasting and reshaping are strictly forbidden.",
                            e_shape, a_shape
                        ),
                    );
                    return false;
                }
                true
            }
            _ => {
                if expected != actual {
                    self.diag.error(
                        span.clone(),
                        format!("Type mismatch: expected '{}', found '{}'. Implicit coercion is forbidden.", expected, actual)
                    );
                    return false;
                }
                true
            }
        }
    }
}
