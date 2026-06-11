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
    Named(String),                  // User-defined enum or group type
    Generic(String),                // Resolved generic parameter
    GenericInst(String, Vec<Type>), // Generic instantiation, e.g., List<int>
    Func(Vec<Type>, Box<Type>),     // Function signature: param types and return type
    Unit,                           // () function return
    Never,                          // Type of `stop` or `skip` or divergent branches
    Error,                          // Represents a failed type check to prevent cascading errors
    SelfType,                       // Resolves to implementing type inside trait/impl blocks
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
            Type::GenericInst(name, args) => {
                let args_str = args
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}<{}>", name, args_str)
            }
            Type::Func(params, ret) => {
                let params_str = params
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "fun({}) -> {}", params_str, ret)
            }
            Type::Unit => write!(f, "()"),
            Type::Never => write!(f, "!"),
            Type::Error => write!(f, "<error>"),
            Type::SelfType => write!(f, "Self"),
        }
    }
}

impl Type {
    pub fn substitute(&self, subst: &std::collections::HashMap<String, Type>) -> Type {
        match self {
            Type::Generic(name) => subst.get(name).cloned().unwrap_or_else(|| self.clone()),
            Type::GenericInst(name, args) => {
                let new_args = args.iter().map(|a| a.substitute(subst)).collect();
                Type::GenericInst(name.clone(), new_args)
            }
            Type::Tensor(elem, shape) => {
                Type::Tensor(Box::new(elem.substitute(subst)), shape.clone())
            }
            Type::Func(params, ret) => {
                let new_params = params.iter().map(|p| p.substitute(subst)).collect();
                Type::Func(new_params, Box::new(ret.substitute(subst)))
            }
            _ => self.clone(),
        }
    }

    /// Replace `SelfType` with the given concrete type.
    pub fn resolve_self(&self, concrete: &Type) -> Type {
        match self {
            Type::SelfType => concrete.clone(),
            Type::GenericInst(name, args) => {
                let new_args = args.iter().map(|a| a.resolve_self(concrete)).collect();
                Type::GenericInst(name.clone(), new_args)
            }
            Type::Tensor(elem, shape) => {
                Type::Tensor(Box::new(elem.resolve_self(concrete)), shape.clone())
            }
            Type::Func(params, ret) => {
                let new_params = params.iter().map(|p| p.resolve_self(concrete)).collect();
                Type::Func(new_params, Box::new(ret.resolve_self(concrete)))
            }
            _ => self.clone(),
        }
    }
}

// ── Trait Definitions ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    pub method_sigs: Vec<TraitMethodDef>,
}

#[derive(Debug, Clone)]
pub struct TraitMethodDef {
    pub name: String,
    pub param_types: Vec<Type>, // excluding self
    pub return_type: Type,
    pub has_self: bool,
}

// ── Impl Definitions ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ImplDef {
    pub trait_name: Option<String>, // None = inherent impl
    pub target_type: String,
    pub method_names: Vec<String>,
}

// ── Operator → Trait Mapping ──────────────────────────────────────

use crate::ast::BinOp;

pub fn operator_trait(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::Add => Some("Add"),
        BinOp::Sub => Some("Sub"),
        BinOp::Mul => Some("Mul"),
        BinOp::Div => Some("Div"),
        _ => None,
    }
}

// ── Type Environment ─────────────────────────────────────────────

pub struct TypeEnv<'a> {
    pub diag: &'a mut DiagnosticBag,
    scopes: Vec<HashMap<String, Type>>,
    // Global struct/enum declarations for type validation
    pub types: HashMap<String, Type>,
    pub active_generics: Vec<String>,
    // v2.2: Trait & impl registries
    pub traits: HashMap<String, TraitDef>,
    pub impls: Vec<ImplDef>,
    pub group_fields: HashMap<String, Vec<(String, Type)>>,
    pub enum_variants: HashMap<String, Vec<(String, Vec<Type>)>>,
}

impl<'a> TypeEnv<'a> {
    pub fn new(diag: &'a mut DiagnosticBag) -> Self {
        let mut globals = HashMap::new();
        // Register core builtins
        globals.insert(
            "print".to_string(),
            Type::Func(vec![Type::String], Box::new(Type::Unit)),
        );
        globals.insert(
            "println".to_string(),
            Type::Func(vec![Type::String], Box::new(Type::Unit)),
        );
        globals.insert(
            "input".to_string(),
            Type::Func(vec![Type::String], Box::new(Type::String)),
        );
        globals.insert(
            "len".to_string(),
            Type::Func(vec![Type::String], Box::new(Type::Int)),
        );
        globals.insert(
            "str".to_string(),
            Type::Func(vec![Type::Int], Box::new(Type::String)),
        ); // Ideally overloaded, using int for now
        globals.insert(
            "int".to_string(),
            Type::Func(vec![Type::String], Box::new(Type::Int)),
        );
        globals.insert(
            "float".to_string(),
            Type::Func(vec![Type::String], Box::new(Type::Float)),
        );
        globals.insert(
            "assert".to_string(),
            Type::Func(vec![Type::Bool, Type::String], Box::new(Type::Unit)),
        );
        globals.insert(
            "exit".to_string(),
            Type::Func(vec![Type::Int], Box::new(Type::Never)),
        );
        globals.insert(
            "zeros".to_string(),
            Type::Func(vec![], Box::new(Type::Error)),
        );

        Self {
            diag,
            scopes: vec![globals],
            types: HashMap::new(),
            active_generics: Vec::new(),
            traits: HashMap::new(),
            impls: Vec::new(),
            group_fields: HashMap::new(),
            enum_variants: HashMap::new(),
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn push_generics(&mut self, generics: Vec<String>) {
        self.active_generics.extend(generics);
    }

    pub fn pop_generics(&mut self, count: usize) {
        let new_len = self.active_generics.len().saturating_sub(count);
        self.active_generics.truncate(new_len);
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

    // ── Trait Registry ──────────────────────────────────────────

    pub fn register_trait(&mut self, name: String, def: TraitDef, span: &Span) {
        if self.traits.contains_key(&name) {
            self.diag.error(
                span.clone(),
                format!("Trait '{}' is already defined.", name),
            );
        } else {
            self.traits.insert(name, def);
        }
    }

    pub fn register_impl(&mut self, impl_def: ImplDef, span: &Span) {
        // Validate trait existence if this is a trait impl
        if let Some(ref trait_name) = impl_def.trait_name {
            if let Some(trait_def) = self.traits.get(trait_name).cloned() {
                // Check that all required methods are implemented
                for required_method in &trait_def.method_sigs {
                    if !impl_def.method_names.contains(&required_method.name) {
                        self.diag.error(
                            span.clone(),
                            format!(
                                "Missing method '{}' required by trait '{}' for type '{}'.",
                                required_method.name, trait_name, impl_def.target_type
                            ),
                        );
                    }
                }
            } else {
                self.diag.error(
                    span.clone(),
                    format!("Trait '{}' is not defined.", trait_name),
                );
            }
        }
        self.impls.push(impl_def);
    }

    pub fn has_trait_impl(&self, type_name: &str, trait_name: &str) -> bool {
        self.impls.iter().any(|imp| {
            imp.target_type == type_name && imp.trait_name.as_deref() == Some(trait_name)
        })
    }

    // ── Group & Enum Registry ───────────────────────────────────

    pub fn register_group_fields(&mut self, name: String, fields: Vec<(String, Type)>) {
        self.group_fields.insert(name, fields);
    }

    pub fn register_enum_variants(&mut self, name: String, variants: Vec<(String, Vec<Type>)>) {
        self.enum_variants.insert(name, variants);
    }

    pub fn lookup_field(&self, type_name: &str, field_name: &str) -> Option<Type> {
        // Check group fields
        if let Some(fields) = self.group_fields.get(type_name) {
            for (fname, ftype) in fields {
                if fname == field_name {
                    return Some(ftype.clone());
                }
            }
        }
        None
    }

    /// Look up a method registered on a type via impl blocks or inline group methods.
    /// Returns the function type if found.
    pub fn lookup_method(&self, type_name: &str, method_name: &str) -> Option<Type> {
        // Check variable scope for methods registered as functions like "TypeName::method"
        // We don't use namespaced names in the current simple model.
        // Instead, methods are found from impl blocks' method signatures.
        // For now, return None — the semantic analyzer handles method calls through field access.
        let _ = type_name;
        let _ = method_name;
        None
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
                if self.active_generics.contains(name) {
                    Type::Generic(name.clone())
                } else {
                    /* For phase 1 we strictly accept defined names, generic stubs, or defer if not found */
                    Type::Named(name.clone())
                }
            }
            AstType::Generic {
                name,
                args,
                span: _,
            } => {
                let resolved_args = args.iter().map(|a| self.resolve_ast_type(a)).collect();
                Type::GenericInst(name.clone(), resolved_args)
            }
            AstType::SelfType(_) => Type::SelfType,
        }
    }

    // ── Unification ──────────────────────────────────────────────

    /// Structurally unify two types, emitting a diagnostic if they diverge.
    /// Strict checking: NO implicit coercion, NO implicit broadcasting.
    pub fn unify(&mut self, expected: &Type, actual: &Type, span: &Span) -> bool {
        self.unify_recursive(expected, actual, span, &mut HashMap::new())
    }

    pub fn unify_recursive(
        &mut self,
        expected: &Type,
        actual: &Type,
        span: &Span,
        subst: &mut HashMap<String, Type>,
    ) -> bool {
        if expected == &Type::Error || actual == &Type::Error {
            return true; // Suppress cascade errors
        }
        if expected == &Type::Never || actual == &Type::Never {
            return true; // Never unifies with everything
        }
        // SelfType unifies with anything (will be resolved contextually)
        if expected == &Type::SelfType || actual == &Type::SelfType {
            return true;
        }

        match (expected, actual) {
            (Type::Tensor(e_elem, e_shape), Type::Tensor(a_elem, a_shape)) => {
                if !self.unify_recursive(e_elem, a_elem, span, subst) {
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
            (Type::GenericInst(n1, a1), Type::GenericInst(n2, a2)) => {
                if n1 != n2 || a1.len() != a2.len() {
                    self.diag.error(
                        span.clone(),
                        format!("Generic type mismatch: '{}' vs '{}'", expected, actual),
                    );
                    return false;
                }
                for i in 0..a1.len() {
                    if !self.unify_recursive(&a1[i], &a2[i], span, subst) {
                        return false;
                    }
                }
                true
            }
            (Type::GenericInst(n1, _), Type::Named(n2))
            | (Type::Named(n2), Type::GenericInst(n1, _)) => {
                if n1 != n2 {
                    self.diag.error(
                        span.clone(),
                        format!("Type mismatch: '{}' vs '{}'", expected, actual),
                    );
                    return false;
                }
                true
            }
            (Type::Generic(name), other) | (other, Type::Generic(name)) => {
                if let Some(existing) = subst.get(name) {
                    if existing != other && other != &Type::Error {
                        self.diag.error(
                            span.clone(),
                            format!(
                                "Generic type conflict for '{}': already bound to '{}', but found '{}'",
                                name, existing, other
                            ),
                        );
                        return false;
                    }
                } else {
                    subst.insert(name.clone(), other.clone());
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
