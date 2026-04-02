use crate::errors::Span;
use std::fmt;

// ── Token Type ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ── Keywords ─────────────────────────────────────────────
    Fun,
    Keep,
    Param,
    Constant,
    Group,
    Enum,
    Import,
    From,
    Take,
    As,
    If,
    Elif,
    Else,
    While,
    For,
    In,
    Return,
    Stop, // `break` equivalent
    Skip, // `continue` equivalent
    Match,
    Case,
    Default,
    Infer,
    Train,
    Async,
    Await,
    Spawn,
    Select,
    Where,
    SelfKw, // `self`
    Extern,
    Unsafe,
    True,
    False,

    // ── Identifiers & Literals ───────────────────────────────
    Ident(String),
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    StringLit(String),

    // ── Operators ────────────────────────────────────────────
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Percent,  // %
    Eq,       // =
    EqEq,     // ==
    BangEq,   // !=
    Lt,       // <
    Gt,       // >
    LtEq,     // <=
    GtEq,     // >=
    And,      // &&
    Or,       // ||
    Bang,     // !
    Arrow,    // ->
    FatArrow, // =>

    // ── Delimiters ───────────────────────────────────────────
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;
    Dot,       // .

    // ── Special ──────────────────────────────────────────────
    EOF,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Fun => write!(f, "fun"),
            TokenKind::Keep => write!(f, "keep"),
            TokenKind::Param => write!(f, "param"),
            TokenKind::Constant => write!(f, "constant"),
            TokenKind::Group => write!(f, "group"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::From => write!(f, "from"),
            TokenKind::Take => write!(f, "take"),
            TokenKind::As => write!(f, "as"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Elif => write!(f, "elif"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::While => write!(f, "while"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Stop => write!(f, "stop"),
            TokenKind::Skip => write!(f, "skip"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Case => write!(f, "case"),
            TokenKind::Default => write!(f, "default"),
            TokenKind::Infer => write!(f, "infer"),
            TokenKind::Train => write!(f, "train"),
            TokenKind::Async => write!(f, "async"),
            TokenKind::Await => write!(f, "await"),
            TokenKind::Spawn => write!(f, "spawn"),
            TokenKind::Select => write!(f, "select"),
            TokenKind::Where => write!(f, "where"),
            TokenKind::SelfKw => write!(f, "self"),
            TokenKind::Extern => write!(f, "extern"),
            TokenKind::Unsafe => write!(f, "unsafe"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::IntLit(n) => write!(f, "{}", n),
            TokenKind::FloatLit(n) => write!(f, "{}", n),
            TokenKind::BoolLit(b) => write!(f, "{}", b),
            TokenKind::StringLit(s) => write!(f, "\"{}\"", s),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::BangEq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::And => write!(f, "&&"),
            TokenKind::Or => write!(f, "||"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::EOF => write!(f, "EOF"),
        }
    }
}

// ── Token ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

// ── Keyword Lookup ───────────────────────────────────────────────

pub fn lookup_keyword(word: &str) -> Option<TokenKind> {
    match word {
        "fun" => Some(TokenKind::Fun),
        "keep" => Some(TokenKind::Keep),
        "param" => Some(TokenKind::Param),
        "constant" => Some(TokenKind::Constant),
        "group" => Some(TokenKind::Group),
        "enum" => Some(TokenKind::Enum),
        "import" => Some(TokenKind::Import),
        "from" => Some(TokenKind::From),
        "take" => Some(TokenKind::Take),
        "as" => Some(TokenKind::As),
        "if" => Some(TokenKind::If),
        "elif" => Some(TokenKind::Elif),
        "else" => Some(TokenKind::Else),
        "while" => Some(TokenKind::While),
        "for" => Some(TokenKind::For),
        "in" => Some(TokenKind::In),
        "return" => Some(TokenKind::Return),
        "stop" => Some(TokenKind::Stop),
        "skip" => Some(TokenKind::Skip),
        "match" => Some(TokenKind::Match),
        "case" => Some(TokenKind::Case),
        "default" => Some(TokenKind::Default),
        "infer" => Some(TokenKind::Infer),
        "train" => Some(TokenKind::Train),
        "async" => Some(TokenKind::Async),
        "await" => Some(TokenKind::Await),
        "spawn" => Some(TokenKind::Spawn),
        "select" => Some(TokenKind::Select),
        "where" => Some(TokenKind::Where),
        "self" => Some(TokenKind::SelfKw),
        "extern" => Some(TokenKind::Extern),
        "unsafe" => Some(TokenKind::Unsafe),
        "true" => Some(TokenKind::True),
        "false" => Some(TokenKind::False),
        _ => None,
    }
}
