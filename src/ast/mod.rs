// SECTION 2 – AST
// ================================================================
use crate::lexer::FsPart;

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    Ident(String),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    FStr(Vec<FsPart>),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    Index {
        obj: Box<Expr>,
        idx: Box<Expr>,
    },
    Field {
        obj: Box<Expr>,
        name: String,
    },
    If {
        cond: Box<Expr>,
        then: Vec<Stmt>,
        else_: Option<Vec<Stmt>>,
    },
    Lambda {
        params: Vec<String>,
        variadic: Option<String>,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    IDiv,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    NullCoal,
}

#[derive(Debug, Clone)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPat,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum MatchPat {
    Literal(Expr),
    Range(Expr, Expr),
    Wildcard,
    Binding(String),
}

#[derive(Debug, Clone)]
pub enum UnpackItem {
    Name(String),
    Rest(String),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Let {
        name: String,
        value: Expr,
    },
    LetList {
        items: Vec<UnpackItem>,
        value: Expr,
    },
    LetMap {
        names: Vec<String>,
        value: Expr,
    },
    Assign {
        target: Lhs,
        value: Expr,
    },
    CompoundAssign {
        target: Lhs,
        op: BinOp,
        value: Expr,
    },
    Print(Expr),
    Write(Expr),
    Return(Option<Expr>),
    Throw(Expr),
    Break,
    Continue,
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    For {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    FnDef {
        name: String,
        params: Vec<String>,
        variadic: Option<String>,
        body: Vec<Stmt>,
    },
    Match {
        subject: Expr,
        arms: Vec<MatchArm>,
    },
    TryCatch {
        body: Vec<Stmt>,
        catch_var: String,
        catch_body: Vec<Stmt>,
    },
    Import {
        path: String,
    },
}

#[derive(Debug, Clone)]
pub enum Lhs {
    Ident(String),
    Index { obj: Box<Expr>, idx: Expr },
    Field { obj: Box<Expr>, name: String },
}

// ================================================================
