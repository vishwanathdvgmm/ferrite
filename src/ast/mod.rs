// ================================================================
// Ferrite v2.0 — Abstract Syntax Tree
// Maps directly to ferrite_grammar.txt
// ================================================================

use crate::errors::Span;

// ── Program ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Program {
    pub decls: Vec<TopDecl>,
}

// ── Top-Level Declarations ───────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TopDecl {
    Import(ImportDecl),
    Constant(ConstantDecl),
    Group(GroupDecl),
    Enum(EnumDecl),
    Func(FuncDecl),
}

// ── Imports ──────────────────────────────────────────────────────
// import "path";
// import name as alias;
// from "path" take name;

#[derive(Debug, Clone)]
pub enum ImportDecl {
    Simple {
        path: String,
        span: Span,
    },
    Aliased {
        name: String,
        alias: String,
        span: Span,
    },
    Selective {
        path: String,
        name: String,
        span: Span,
    },
}

// ── Constants ────────────────────────────────────────────────────
// constant PI: float = 3.14;

#[derive(Debug, Clone)]
pub struct ConstantDecl {
    pub name: String,
    pub ty: Type,
    pub value: Expr,
    pub span: Span,
}

// ── Effects ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Infer,
    Train,
    Async,
    Named(String),
}

// ── Generics & Traits ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum GenericParam {
    /// Simple type parameter: `T`
    Type { name: String, span: Span },
    /// Shape parameter: `N: shape`
    Shape { name: String, span: Span },
    /// Trait-bounded parameter: `T: Add + Mul`
    Bounded {
        name: String,
        bounds: Vec<TraitRef>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct TraitRef {
    pub name: String,
    pub span: Span,
}

// ── Where Constraints ────────────────────────────────────────────
// where N > 0, M == N, T: Serialize

#[derive(Debug, Clone)]
pub enum Constraint {
    /// Shape constraint: `N > 0` or `N == M`
    ShapeRel {
        left: String,
        op: RelOp,
        right: ConstraintRhs,
        span: Span,
    },
    /// Trait bound: `T: Add + Mul`
    TraitBound {
        param: String,
        bounds: Vec<TraitRef>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum ConstraintRhs {
    Int(i64),
    Ident(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelOp {
    Eq,    // ==
    NotEq, // !=
    Lt,    // <
    Gt,    // >
    LtEq,  // <=
    GtEq,  // >=
}

// ── Groups (Structs) ─────────────────────────────────────────────
// group Vector<T> { x: float; fun length(self) -> float { ... } }

#[derive(Debug, Clone)]
pub struct GroupDecl {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<FieldDecl>,
    pub methods: Vec<MethodDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub effects: Vec<Effect>,
    pub name: String,
    pub has_self: bool,
    pub params: Vec<Param>,
    pub return_effects: Vec<Effect>,
    pub return_type: Option<Type>,
    pub where_clause: Vec<Constraint>,
    pub body: Block,
    pub span: Span,
}

// ── Enums (ADTs) ─────────────────────────────────────────────────
// enum Option<T> { Some(T); None; }

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<Type>,
    pub span: Span,
}

// ── Functions ────────────────────────────────────────────────────
// fun add(a: int, b: int) -> int { ... }

#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub effect_params: Vec<String>,
    pub effects: Vec<Effect>,
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_effects: Vec<Effect>,
    pub return_type: Option<Type>,
    pub where_clause: Vec<Constraint>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

// ── Types ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Type {
    /// int, float, bool, string
    Primitive(PrimType, Span),
    /// Tensor<float, (784, 128)>
    Tensor {
        elem: Box<Type>,
        shape: Vec<ShapeDim>,
        span: Span,
    },
    /// List<T>, Option<int>
    Generic {
        name: String,
        args: Vec<Type>,
        span: Span,
    },
    /// User-defined type name
    Named(String, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrimType {
    Int,
    Float,
    Bool,
    String,
}

#[derive(Debug, Clone)]
pub enum ShapeDim {
    Const(i64),
    Symbolic(String),
}

// ── Blocks & Statements ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// keep x: int = 5;
    Keep {
        name: String,
        ty: Type,
        value: Expr,
        span: Span,
    },
    /// param w: Tensor<float, (512)> = init();
    Param {
        name: String,
        ty: Type,
        value: Expr,
        span: Span,
    },
    /// expression;
    ExprStmt(Expr),
    /// return [expr];
    Return { value: Option<Expr>, span: Span },
    /// if cond { ... } elif cond { ... } else { ... }
    If {
        condition: Expr,
        then_block: Block,
        elif_branches: Vec<(Expr, Block)>,
        else_block: Option<Block>,
        span: Span,
    },
    /// while cond { ... }
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    /// for x in expr { ... }
    For {
        var: String,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    /// match expr { case pat => { ... } default => { ... } }
    Match {
        subject: Expr,
        cases: Vec<MatchCase>,
        span: Span,
    },
    /// infer { ... }
    InferBlock(Block),
    /// train { ... }
    TrainBlock(Block),
    /// select { case x = expr => { ... } default => { ... } }
    Select { cases: Vec<SelectCase>, span: Span },
    /// stop;
    Stop(Span),
    /// skip;
    Skip(Span),
}

// ── Match ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    /// Literal value: 42, "hello", true
    Literal(Literal),
    /// Wildcard: _
    Wildcard(Span),
    /// Variable binding: x
    Binding(String, Span),
    /// Constructor: Some(x, y)
    Constructor {
        name: String,
        fields: Vec<Pattern>,
        span: Span,
    },
    /// Struct pattern: Point { x, y }
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Literal(lit) => match lit {
                Literal::Int(_) | Literal::Float(_) | Literal::Bool(_) | Literal::String(_) => {
                    Span::dummy()
                } // Lit spans shouldn't ideally be detached from AST, but we handle it in SemanticAnalyzer
            },
            Pattern::Wildcard(s) => s.clone(),
            Pattern::Binding(_, s) => s.clone(),
            Pattern::Constructor { span, .. } => span.clone(),
            Pattern::Struct { span, .. } => span.clone(),
        }
    }
}

// ── Select ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SelectCase {
    pub assignment: Option<(String, Expr)>,
    pub body: Block,
    pub is_default: bool,
    pub span: Span,
}

// ── Expressions ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal value
    Lit(Literal, Span),
    /// Variable reference
    Ident(String, Span),
    /// Binary operation: a + b, x == y
    BinOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },
    /// Unary operation: !x, -y, await expr
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    /// Function call: foo(a, b)
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Field access: obj.field
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Index access: list[0]
    IndexAccess {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// Lambda: (a: int, b: int) => a + b
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
        span: Span,
    },
    /// Group literal: Point { x: 1.0, y: 2.0 }
    GroupLiteral {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    /// Assignment: x = expr, obj.field = expr, list[i] = expr
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
}

// ── Literals ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
}

// ── Operators ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,   // +
    Sub,   // -
    Mul,   // *
    Div,   // /
    Mod,   // %
    Eq,    // ==
    NotEq, // !=
    Lt,    // <
    Gt,    // >
    LtEq,  // <=
    GtEq,  // >=
    And,   // &&
    Or,    // ||
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,   // -
    Not,   // !
    Await, // await
}
