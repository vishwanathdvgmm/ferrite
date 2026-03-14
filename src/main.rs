// ================================================================
//  Ferrite — A scripting language built in Rust
//  Pipeline:  Source → Lexer → Tokens → Parser → AST → Interpreter
// ================================================================

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};

// ================================================================
// SECTION 1 – LEXER
// ================================================================

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Int(i64), Float(f64), Str(String), Bool(bool), Null,
    Ident(String), Let, Fn, If, Else, While, For, In, Return, Print,
    Plus, Minus, Star, Slash, Percent,
    Eq, EqEq, BangEq, Bang,
    Lt, LtEq, Gt, GtEq,
    And, Or,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Semicolon, Dot,
    EOF,
}

struct Lexer { input: Vec<char>, pos: usize }

impl Lexer {
    fn new(src: &str) -> Self { Lexer { input: src.chars().collect(), pos: 0 } }
    fn peek(&self)  -> Option<char> { self.input.get(self.pos).copied() }
    fn peek2(&self) -> Option<char> { self.input.get(self.pos + 1).copied() }
    fn advance(&mut self) -> Option<char> {
        let c = self.input.get(self.pos).copied(); self.pos += 1; c
    }

    fn skip_ws(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) { self.advance(); }
            if self.peek() == Some('/') && self.peek2() == Some('/') {
                while matches!(self.peek(), Some(c) if c != '\n') { self.advance(); }
            } else { break; }
        }
    }

    fn read_str(&mut self) -> Result<Token, String> {
        self.advance();
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"')  => break,
                Some('\\') => match self.advance() {
                    Some('n')  => s.push('\n'),
                    Some('t')  => s.push('\t'),
                    Some('\\') => s.push('\\'),
                    Some('"')  => s.push('"'),
                    Some(c)    => { s.push('\\'); s.push(c); }
                    None       => return Err("Unterminated escape".into()),
                },
                Some(c) => s.push(c),
                None    => return Err("Unterminated string".into()),
            }
        }
        Ok(Token::Str(s))
    }

    fn read_num(&mut self) -> Token {
        let mut s = String::new();
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) { s.push(self.advance().unwrap()); }
        if self.peek() == Some('.') && matches!(self.peek2(), Some(c) if c.is_ascii_digit()) {
            s.push(self.advance().unwrap());
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) { s.push(self.advance().unwrap()); }
            Token::Float(s.parse().unwrap())
        } else {
            Token::Int(s.parse().unwrap())
        }
    }

    fn read_ident(&mut self) -> Token {
        let mut s = String::new();
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') { s.push(self.advance().unwrap()); }
        match s.as_str() {
            "let"    => Token::Let,    "fn"     => Token::Fn,    "if"     => Token::If,
            "else"   => Token::Else,   "while"  => Token::While, "for"    => Token::For,
            "in"     => Token::In,     "return" => Token::Return,"print"  => Token::Print,
            "true"   => Token::Bool(true), "false" => Token::Bool(false), "null" => Token::Null,
            _        => Token::Ident(s),
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut toks = Vec::new();
        loop {
            self.skip_ws();
            let tok = match self.peek() {
                None => { toks.push(Token::EOF); break; }
                Some(c) => match c {
                    '"'       => self.read_str()?,
                    '0'..='9' => self.read_num(),
                    'a'..='z'|'A'..='Z'|'_' => self.read_ident(),
                    '+' => { self.advance(); Token::Plus }
                    '-' => { self.advance(); Token::Minus }
                    '*' => { self.advance(); Token::Star }
                    '/' => { self.advance(); Token::Slash }
                    '%' => { self.advance(); Token::Percent }
                    '(' => { self.advance(); Token::LParen }
                    ')' => { self.advance(); Token::RParen }
                    '{' => { self.advance(); Token::LBrace }
                    '}' => { self.advance(); Token::RBrace }
                    '[' => { self.advance(); Token::LBracket }
                    ']' => { self.advance(); Token::RBracket }
                    ',' => { self.advance(); Token::Comma }
                    ';' => { self.advance(); Token::Semicolon }
                    '.' => { self.advance(); Token::Dot }
                    '=' => { self.advance(); if self.peek()==Some('=') { self.advance(); Token::EqEq   } else { Token::Eq } }
                    '!' => { self.advance(); if self.peek()==Some('=') { self.advance(); Token::BangEq } else { Token::Bang } }
                    '<' => { self.advance(); if self.peek()==Some('=') { self.advance(); Token::LtEq   } else { Token::Lt } }
                    '>' => { self.advance(); if self.peek()==Some('=') { self.advance(); Token::GtEq   } else { Token::Gt } }
                    '&' => { self.advance(); if self.peek()==Some('&') { self.advance(); Token::And } else { return Err("Expected '&&'".into()); } }
                    '|' => { self.advance(); if self.peek()==Some('|') { self.advance(); Token::Or  } else { return Err("Expected '||'".into()); } }
                    c   => return Err(format!("Unexpected character '{}'", c)),
                },
            };
            toks.push(tok);
        }
        Ok(toks)
    }
}

// ================================================================
// SECTION 2 – AST
// ================================================================

#[derive(Debug, Clone)]
enum Expr {
    Int(i64), Float(f64), Str(String), Bool(bool), Null,
    Ident(String),
    List(Vec<Expr>),
    BinOp  { op: BinOp,   left: Box<Expr>, right: Box<Expr> },
    Unary  { op: UnOp,    expr: Box<Expr> },
    Call   { func: Box<Expr>, args: Vec<Expr> },
    Index  { obj: Box<Expr>,  idx:  Box<Expr> },
    Field  { obj: Box<Expr>,  name: String    },
    If     { cond: Box<Expr>, then: Vec<Stmt>, else_: Option<Vec<Stmt>> },
    Lambda { params: Vec<String>, body: Vec<Stmt> },
}

#[derive(Debug, Clone)]
enum BinOp { Add, Sub, Mul, Div, Mod, Eq, Ne, Lt, Le, Gt, Ge, And, Or }

#[derive(Debug, Clone)]
enum UnOp { Neg, Not }

#[derive(Debug, Clone)]
enum Stmt {
    Expr(Expr),
    Let    { name: String, value: Expr },
    Assign { target: Lhs, value: Expr },
    Print(Expr),
    Return(Option<Expr>),
    While  { cond: Expr, body: Vec<Stmt> },
    For    { var: String, iter: Expr, body: Vec<Stmt> },
    FnDef  { name: String, params: Vec<String>, body: Vec<Stmt> },
}

#[derive(Debug, Clone)]
enum Lhs { Ident(String), Index { name: String, idx: Expr } }

// ================================================================
// SECTION 3 – PARSER
// ================================================================

struct Parser { tokens: Vec<Token>, pos: usize }

impl Parser {
    fn new(tokens: Vec<Token>) -> Self { Parser { tokens, pos: 0 } }
    fn peek(&self) -> &Token { &self.tokens[self.pos] }
    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() { self.pos += 1; }
        t
    }
    fn check(&self, t: &Token) -> bool { std::mem::discriminant(self.peek()) == std::mem::discriminant(t) }
    fn expect(&mut self, t: &Token) -> Result<(), String> {
        if self.check(t) { self.advance(); Ok(()) }
        else { Err(format!("Expected {:?}, got {:?}", t, self.peek())) }
    }
    fn expect_ident(&mut self, ctx: &str) -> Result<String, String> {
        match self.advance() {
            Token::Ident(n) => Ok(n),
            t => Err(format!("Expected identifier {}, got {:?}", ctx, t)),
        }
    }

    fn semi_after(&mut self, e: &Expr) -> Result<(), String> {
        if self.check(&Token::Semicolon) { self.advance(); Ok(()) }
        else if matches!(e, Expr::If { .. } | Expr::Lambda { .. }) { Ok(()) }
        else { Err(format!("Expected ';' after expression, got {:?}", self.peek())) }
    }

    fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut v = Vec::new();
        while self.peek() != &Token::EOF { v.push(self.parse_stmt()?); }
        Ok(v)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(&Token::LBrace)?;
        let mut v = Vec::new();
        while !self.check(&Token::RBrace) && self.peek() != &Token::EOF { v.push(self.parse_stmt()?); }
        self.expect(&Token::RBrace)?;
        Ok(v)
    }

    fn parse_params(&mut self) -> Result<Vec<String>, String> {
        self.expect(&Token::LParen)?;
        let mut p = Vec::new();
        while !self.check(&Token::RParen) {
            p.push(self.expect_ident("in parameter list")?);
            if self.check(&Token::Comma) { self.advance(); }
        }
        self.expect(&Token::RParen)?;
        Ok(p)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            Token::Let => {
                self.advance();
                let name = self.expect_ident("after 'let'")?;
                self.expect(&Token::Eq)?;
                let value = self.parse_expr()?;
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::Let { name, value })
            }
            Token::Print => {
                self.advance(); self.expect(&Token::LParen)?;
                let e = self.parse_expr()?;
                self.expect(&Token::RParen)?; self.expect(&Token::Semicolon)?;
                Ok(Stmt::Print(e))
            }
            Token::Return => {
                self.advance();
                if self.check(&Token::Semicolon) { self.advance(); return Ok(Stmt::Return(None)); }
                let e = self.parse_expr()?; self.expect(&Token::Semicolon)?;
                Ok(Stmt::Return(Some(e)))
            }
            Token::While => {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::While { cond, body })
            }
            Token::For => {
                self.advance();
                let var = self.expect_ident("in 'for'")?;
                self.expect(&Token::In)?;
                let iter = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::For { var, iter, body })
            }
            Token::Fn => {
                self.advance();
                let name   = self.expect_ident("after 'fn'")?;
                let params = self.parse_params()?;
                let body   = self.parse_block()?;
                Ok(Stmt::FnDef { name, params, body })
            }
            Token::Ident(name) => {
                let saved = self.pos;
                self.advance();
                if self.check(&Token::Eq) {
                    self.advance();
                    let value = self.parse_expr()?; self.expect(&Token::Semicolon)?;
                    return Ok(Stmt::Assign { target: Lhs::Ident(name), value });
                }
                if self.check(&Token::LBracket) {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    if self.check(&Token::Eq) {
                        self.advance();
                        let value = self.parse_expr()?; self.expect(&Token::Semicolon)?;
                        return Ok(Stmt::Assign { target: Lhs::Index { name, idx }, value });
                    }
                }
                self.pos = saved;
                let e = self.parse_expr()?; self.semi_after(&e)?;
                Ok(Stmt::Expr(e))
            }
            _ => { let e = self.parse_expr()?; self.semi_after(&e)?; Ok(Stmt::Expr(e)) }
        }
    }

    fn parse_expr(&mut self)  -> Result<Expr, String> { self.parse_or() }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_and()?;
        while self.check(&Token::Or) { self.advance(); let r = self.parse_and()?; l = Expr::BinOp { op: BinOp::Or, left: l.into(), right: r.into() }; }
        Ok(l)
    }
    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_eq()?;
        while self.check(&Token::And) { self.advance(); let r = self.parse_eq()?; l = Expr::BinOp { op: BinOp::And, left: l.into(), right: r.into() }; }
        Ok(l)
    }
    fn parse_eq(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_cmp()?;
        loop {
            let op = match self.peek() { Token::EqEq => BinOp::Eq, Token::BangEq => BinOp::Ne, _ => break };
            self.advance(); let r = self.parse_cmp()?;
            l = Expr::BinOp { op, left: l.into(), right: r.into() };
        }
        Ok(l)
    }
    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_add()?;
        loop {
            let op = match self.peek() { Token::Lt => BinOp::Lt, Token::LtEq => BinOp::Le, Token::Gt => BinOp::Gt, Token::GtEq => BinOp::Ge, _ => break };
            self.advance(); let r = self.parse_add()?;
            l = Expr::BinOp { op, left: l.into(), right: r.into() };
        }
        Ok(l)
    }
    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_mul()?;
        loop {
            let op = match self.peek() { Token::Plus => BinOp::Add, Token::Minus => BinOp::Sub, _ => break };
            self.advance(); let r = self.parse_mul()?;
            l = Expr::BinOp { op, left: l.into(), right: r.into() };
        }
        Ok(l)
    }
    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_unary()?;
        loop {
            let op = match self.peek() { Token::Star => BinOp::Mul, Token::Slash => BinOp::Div, Token::Percent => BinOp::Mod, _ => break };
            self.advance(); let r = self.parse_unary()?;
            l = Expr::BinOp { op, left: l.into(), right: r.into() };
        }
        Ok(l)
    }
    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::Minus => { self.advance(); Ok(Expr::Unary { op: UnOp::Neg, expr: self.parse_unary()?.into() }) }
            Token::Bang  => { self.advance(); Ok(Expr::Unary { op: UnOp::Not, expr: self.parse_unary()?.into() }) }
            _            => self.parse_postfix(),
        }
    }
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_primary()?;
        loop {
            match self.peek() {
                Token::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(&Token::RParen) { args.push(self.parse_expr()?); if self.check(&Token::Comma) { self.advance(); } }
                    self.expect(&Token::RParen)?;
                    e = Expr::Call { func: e.into(), args };
                }
                Token::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?; self.expect(&Token::RBracket)?;
                    e = Expr::Index { obj: e.into(), idx: idx.into() };
                }
                Token::Dot => {
                    self.advance();
                    let name = self.expect_ident("after '.'")?;
                    if self.check(&Token::LParen) {
                        self.advance();
                        let mut args = vec![e];
                        while !self.check(&Token::RParen) { args.push(self.parse_expr()?); if self.check(&Token::Comma) { self.advance(); } }
                        self.expect(&Token::RParen)?;
                        e = Expr::Call { func: Expr::Ident(name).into(), args };
                    } else {
                        e = Expr::Field { obj: e.into(), name };
                    }
                }
                _ => break,
            }
        }
        Ok(e)
    }
    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance() {
            Token::Int(n)   => Ok(Expr::Int(n)),
            Token::Float(f) => Ok(Expr::Float(f)),
            Token::Str(s)   => Ok(Expr::Str(s)),
            Token::Bool(b)  => Ok(Expr::Bool(b)),
            Token::Null     => Ok(Expr::Null),
            Token::Ident(n) => Ok(Expr::Ident(n)),
            Token::LParen   => { let e = self.parse_expr()?; self.expect(&Token::RParen)?; Ok(e) }
            Token::LBracket => {
                let mut items = Vec::new();
                while !self.check(&Token::RBracket) { items.push(self.parse_expr()?); if self.check(&Token::Comma) { self.advance(); } }
                self.expect(&Token::RBracket)?;
                Ok(Expr::List(items))
            }
            Token::If => {
                let cond  = self.parse_expr()?;
                let then  = self.parse_block()?;
                let else_ = if self.check(&Token::Else) { self.advance(); Some(self.parse_block()?) } else { None };
                Ok(Expr::If { cond: cond.into(), then, else_ })
            }
            Token::Fn => {
                let params = self.parse_params()?;
                let body   = self.parse_block()?;
                Ok(Expr::Lambda { params, body })
            }
            tok => Err(format!("Unexpected token: {:?}", tok)),
        }
    }
}

// ================================================================
// SECTION 4 – VALUES  &  CONTROL-FLOW SIGNALS
// ================================================================

type Env = Vec<HashMap<String, Value>>;

#[derive(Debug, Clone)]
enum Value {
    Int(i64), Float(f64), Str(String), Bool(bool), Null,
    List(Vec<Value>),
    Fn { fname: Option<String>, params: Vec<String>, body: Vec<Stmt>, closure: Env },
    Builtin(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n)   => write!(f, "{}", n),
            Value::Float(n) => if n.fract() == 0.0 { write!(f, "{:.1}", n) } else { write!(f, "{}", n) },
            Value::Str(s)   => write!(f, "{}", s),
            Value::Bool(b)  => write!(f, "{}", b),
            Value::Null     => write!(f, "null"),
            Value::List(l)  => {
                write!(f, "[")?;
                for (i, v) in l.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    if let Value::Str(s) = v { write!(f, "\"{}\"", s)?; } else { write!(f, "{}", v)?; }
                }
                write!(f, "]")
            }
            Value::Fn { params, .. } => write!(f, "<fn({})>", params.join(", ")),
            Value::Builtin(n)        => write!(f, "<builtin:{}>", n),
        }
    }
}

impl Value {
    fn truthy(&self) -> bool {
        match self {
            Value::Bool(b)  => *b,
            Value::Null     => false,
            Value::Int(0)   => false,
            Value::Str(s)   => !s.is_empty(),
            Value::List(l)  => !l.is_empty(),
            _               => true,
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Value::Int(_) => "int", Value::Float(_) => "float", Value::Str(_) => "string",
            Value::Bool(_) => "bool", Value::Null => "null", Value::List(_) => "list",
            _ => "function",
        }
    }
    fn as_f64(&self) -> Option<f64> { match self { Value::Int(n) => Some(*n as f64), Value::Float(f) => Some(*f), _ => None } }
}

// A single signal type used by BOTH eval and exec.
// This is the key design: eval returns Result<Value, Sig> so that `return`
// can propagate up through nested expressions (e.g. `return` inside an `if`).
#[derive(Debug)]
enum Sig {
    Ret(Value),    // return <value>
    Err(String),   // runtime error
}

impl Sig {
    fn err(s: impl Into<String>) -> Self { Sig::Err(s.into()) }
}

// ================================================================
// SECTION 5 – INTERPRETER
// ================================================================

struct Interp { env: Env }

impl Interp {
    fn new() -> Self {
        let mut g: HashMap<String, Value> = HashMap::new();
        for n in &["len","push","pop","str","int","float","type","range",
                   "input","sqrt","abs","max","min","floor","ceil"] {
            g.insert(n.to_string(), Value::Builtin(n.to_string()));
        }
        Interp { env: vec![g] }
    }

    fn get(&self, n: &str) -> Option<Value> {
        for s in self.env.iter().rev() { if let Some(v) = s.get(n) { return Some(v.clone()); } }
        None
    }
    fn set(&mut self, n: &str, v: Value) {
        for s in self.env.iter_mut().rev() {
            if s.contains_key(n) { s.insert(n.to_string(), v); return; }
        }
        self.env.last_mut().unwrap().insert(n.to_string(), v);
    }
    fn def(&mut self, n: &str, v: Value) { self.env.last_mut().unwrap().insert(n.to_string(), v); }
    fn push(&mut self) { self.env.push(HashMap::new()); }
    fn pop(&mut self)  { self.env.pop(); }

    // ── Builtins ──────────────────────────────────────────────────
    fn builtin(&mut self, name: &str, a: Vec<Value>) -> Result<Value, Sig> {
        let n = a.len();
        macro_rules! arity {
            ($k:expr) => { if n != $k { return Err(Sig::err(format!("{}() expects {} arg(s), got {}", name, $k, n))); } };
        }
        macro_rules! e { ($s:expr) => { return Err(Sig::err($s)) }; }
        match name {
            "len"   => { arity!(1); match &a[0] {
                Value::List(l) => Ok(Value::Int(l.len() as i64)),
                Value::Str(s)  => Ok(Value::Int(s.chars().count() as i64)),
                v => e!(format!("len() not supported for {}", v.kind()))
            }}
            "push"  => { arity!(2); match a[0].clone() {
                Value::List(mut l) => { l.push(a[1].clone()); Ok(Value::List(l)) }
                v => e!(format!("push() needs list, got {}", v.kind()))
            }}
            "pop"   => { arity!(1); match a[0].clone() {
                Value::List(mut l) => Ok(l.pop().unwrap_or(Value::Null)),
                v => e!(format!("pop() needs list, got {}", v.kind()))
            }}
            "str"   => { arity!(1); Ok(Value::Str(a[0].to_string())) }
            "type"  => { arity!(1); Ok(Value::Str(a[0].kind().to_string())) }
            "int"   => { arity!(1); match &a[0] {
                Value::Int(x)   => Ok(Value::Int(*x)),
                Value::Float(f) => Ok(Value::Int(*f as i64)),
                Value::Str(s)   => s.trim().parse::<i64>().map(Value::Int)
                    .map_err(|_| Sig::err(format!("Cannot parse \"{}\" as int", s))),
                v => e!(format!("Cannot convert {} to int", v.kind()))
            }}
            "float" => { arity!(1); match &a[0] {
                Value::Float(f) => Ok(Value::Float(*f)),
                Value::Int(x)   => Ok(Value::Float(*x as f64)),
                Value::Str(s)   => s.trim().parse::<f64>().map(Value::Float)
                    .map_err(|_| Sig::err(format!("Cannot parse \"{}\" as float", s))),
                v => e!(format!("Cannot convert {} to float", v.kind()))
            }}
            "sqrt"  => { arity!(1); a[0].as_f64().map(|f| Value::Float(f.sqrt())).ok_or_else(|| Sig::err("sqrt() needs a number")) }
            "abs"   => { arity!(1); match a[0] {
                Value::Int(x)   => Ok(Value::Int(x.abs())),
                Value::Float(f) => Ok(Value::Float(f.abs())),
                _ => e!("abs() needs a number")
            }}
            "floor" => { arity!(1); a[0].as_f64().map(|f| Value::Int(f.floor() as i64)).ok_or_else(|| Sig::err("floor() needs a number")) }
            "ceil"  => { arity!(1); a[0].as_f64().map(|f| Value::Int(f.ceil() as i64)).ok_or_else(|| Sig::err("ceil() needs a number")) }
            "max"   => {
                if n < 2 { e!("max() needs at least 2 args"); }
                let mut best = a[0].clone();
                for v in &a[1..] {
                    if v.as_f64().ok_or_else(|| Sig::err("max() needs numbers"))? > best.as_f64().ok_or_else(|| Sig::err("max() needs numbers"))? {
                        best = v.clone();
                    }
                }
                Ok(best)
            }
            "min"   => {
                if n < 2 { e!("min() needs at least 2 args"); }
                let mut best = a[0].clone();
                for v in &a[1..] {
                    if v.as_f64().ok_or_else(|| Sig::err("min() needs numbers"))? < best.as_f64().ok_or_else(|| Sig::err("min() needs numbers"))? {
                        best = v.clone();
                    }
                }
                Ok(best)
            }
            "range" => match n {
                1 => match a[0] {
                    Value::Int(x) => Ok(Value::List((0..x).map(Value::Int).collect())),
                    _ => e!("range() needs int")
                },
                2 => match (&a[0], &a[1]) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::List((*x..*y).map(Value::Int).collect())),
                    _ => e!("range() needs ints")
                },
                3 => match (&a[0], &a[1], &a[2]) {
                    (Value::Int(x), Value::Int(y), Value::Int(step)) => {
                        let mut v = Vec::new(); let mut i = *x;
                        while if *step > 0 { i < *y } else { i > *y } { v.push(Value::Int(i)); i += step; }
                        Ok(Value::List(v))
                    }
                    _ => e!("range() needs ints"),
                },
                _ => e!("range() takes 1-3 args"),
            },
            "input" => {
                let prompt = if n == 1 { a[0].to_string() } else { String::new() };
                print!("{}", prompt); io::stdout().flush().unwrap();
                let mut line = String::new();
                io::stdin().read_line(&mut line).unwrap();
                Ok(Value::Str(line.trim_end_matches('\n').to_string()))
            }
            _ => e!(format!("Unknown builtin '{}'", name)),
        }
    }

    // ── Expression evaluator ──────────────────────────────────────
    // Returns Result<Value, Sig> — Sig::Ret propagates `return` through
    // nested expressions so it correctly exits the enclosing function.
    fn eval(&mut self, e: &Expr) -> Result<Value, Sig> {
        match e {
            Expr::Int(n)    => Ok(Value::Int(*n)),
            Expr::Float(f)  => Ok(Value::Float(*f)),
            Expr::Str(s)    => Ok(Value::Str(s.clone())),
            Expr::Bool(b)   => Ok(Value::Bool(*b)),
            Expr::Null      => Ok(Value::Null),
            Expr::Ident(n)  => self.get(n).ok_or_else(|| Sig::err(format!("Undefined variable '{}'", n))),
            Expr::List(xs)  => {
                let mut vals = Vec::new();
                for x in xs { vals.push(self.eval(x)?); }
                Ok(Value::List(vals))
            }

            Expr::BinOp { op, left, right } => {
                // Short-circuit logic operators
                match op {
                    BinOp::And => { let l = self.eval(left)?; return if !l.truthy() { Ok(Value::Bool(false)) } else { self.eval(right) }; }
                    BinOp::Or  => { let l = self.eval(left)?; return if  l.truthy() { Ok(l) }                 else { self.eval(right) }; }
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
                        Value::Int(n)   => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err(Sig::err("Unary '-' requires a number")),
                    },
                    UnOp::Not => Ok(Value::Bool(!v.truthy())),
                }
            }

            Expr::Call { func, args } => {
                let fv = self.eval(func)?;
                let mut avs = Vec::new();
                for a in args { avs.push(self.eval(a)?); }
                self.call(fv, avs)
            }

            Expr::Index { obj, idx } => {
                let ov = self.eval(obj)?;
                let iv = self.eval(idx)?;
                match (ov, iv) {
                    (Value::List(l), Value::Int(i)) => {
                        let n = l.len() as i64; let i = if i < 0 { n + i } else { i };
                        l.into_iter().nth(i as usize).ok_or_else(|| Sig::err(format!("Index {} out of bounds", i)))
                    }
                    (Value::Str(s), Value::Int(i)) => {
                        let ch: Vec<char> = s.chars().collect();
                        let n = ch.len() as i64; let i = if i < 0 { n + i } else { i };
                        ch.get(i as usize).map(|c| Value::Str(c.to_string()))
                            .ok_or_else(|| Sig::err(format!("String index {} out of bounds", i)))
                    }
                    _ => Err(Sig::err("Invalid index operation")),
                }
            }

            Expr::Field { obj, name } => {
                let ov = self.eval(obj)?;
                match (&ov, name.as_str()) {
                    (Value::Str(s),  "len")   => Ok(Value::Int(s.chars().count() as i64)),
                    (Value::List(l), "len")   => Ok(Value::Int(l.len() as i64)),
                    (Value::Str(s),  "upper") => Ok(Value::Str(s.to_uppercase())),
                    (Value::Str(s),  "lower") => Ok(Value::Str(s.to_lowercase())),
                    (Value::Str(s),  "trim")  => Ok(Value::Str(s.trim().to_string())),
                    (Value::Str(s),  "chars") => Ok(Value::List(s.chars().map(|c| Value::Str(c.to_string())).collect())),
                    _ => Err(Sig::err(format!("No property '{}' on {}", name, ov.kind()))),
                }
            }

            // KEY FIX: Expr::If no longer catches Sig::Ret.
            // It propagates it upward so `return` inside an `if` block correctly
            // exits the enclosing function rather than being silently swallowed.
            Expr::If { cond, then, else_ } => {
                let cv = self.eval(cond)?;
                self.push();
                let r = if cv.truthy() {
                    self.exec_block(then)
                } else if let Some(eb) = else_ {
                    self.exec_block(eb)
                } else {
                    Ok(Value::Null)
                };
                self.pop();
                r  // propagate Sig::Ret straight through — do NOT convert it to Ok
            }

            Expr::Lambda { params, body } => {
                Ok(Value::Fn {
                    fname:   None,
                    params:  params.clone(),
                    body:    body.clone(),
                    closure: self.env.clone(),
                })
            }
        }
    }

    fn call(&mut self, fv: Value, args: Vec<Value>) -> Result<Value, Sig> {
        match fv {
            Value::Builtin(n) => self.builtin(&n, args),

            Value::Fn { fname, params, body, closure } => {
                if params.len() != args.len() {
                    return Err(Sig::err(format!("Expected {} arg(s), got {}", params.len(), args.len())));
                }

                let saved = std::mem::replace(&mut self.env, closure);
                self.push();

                for (p, v) in params.iter().zip(args) { self.def(p, v); }

                // Inject self-reference for recursion using the closure frames only
                // (self.env minus the top call frame) to avoid an infinitely nested
                // Value::Fn inside its own closure.
                if let Some(ref n) = fname {
                    let fn_closure = self.env[..self.env.len() - 1].to_vec();
                    let self_val = Value::Fn {
                        fname:   fname.clone(),
                        params:  params.clone(),
                        body:    body.clone(),
                        closure: fn_closure,
                    };
                    self.def(n, self_val);
                }

                let r = self.exec_block(&body);
                self.env = saved;
                match r {
                    Ok(_)            => Ok(Value::Null),
                    Err(Sig::Ret(v)) => Ok(v),
                    Err(e)           => Err(e),
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
                (Value::Int(a),      Value::Int(b))   => Ok(Value::Int(a + b)),
                (Value::Float(a),    Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a),      Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a),    Value::Int(b))   => Ok(Value::Float(a + b as f64)),
                (Value::Str(a),      Value::Str(b))   => Ok(Value::Str(a + &b)),
                (Value::Str(a),      b)               => Ok(Value::Str(a + &b.to_string())),
                (Value::List(mut a), Value::List(b))  => { a.extend(b); Ok(Value::List(a)) }
                (l, r) => Err(Sig::err(format!("Cannot add {} and {}", l.kind(), r.kind()))),
            },
            Sub => self.num2(l, r, |a,b| a-b, |a,b| a-b, "subtract"),
            Mul => match (l, r) {
                (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a * b as f64)),
                (Value::Str(s),   Value::Int(n))   => Ok(Value::Str(s.repeat(n.max(0) as usize))),
                (l, r) => Err(Sig::err(format!("Cannot multiply {} and {}", l.kind(), r.kind()))),
            },
            Div => {
                match &r {
                    Value::Int(0)   => return e("Division by zero"),
                    Value::Float(f) if *f == 0.0 => return e("Division by zero"),
                    _ => {}
                }
                match (l, r) {
                    (Value::Int(a),   Value::Int(b))   => Ok(Value::Float(a as f64 / b as f64)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                    (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64 / b)),
                    (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a / b as f64)),
                    (l, r) => Err(Sig::err(format!("Cannot divide {} by {}", l.kind(), r.kind()))),
                }
            }
            Mod => match (l, r) {
                (Value::Int(a), Value::Int(b)) =>
                    if b == 0 { e("Modulo by zero") } else { Ok(Value::Int(a % b)) },
                _ => e("Modulo requires integers"),
            },
            Eq  => Ok(Value::Bool(self.eq_vals(&l, &r))),
            Ne  => Ok(Value::Bool(!self.eq_vals(&l, &r))),
            Lt  => self.cmp(l, r, |a,b| a < b),
            Le  => self.cmp(l, r, |a,b| a <= b),
            Gt  => self.cmp(l, r, |a,b| a > b),
            Ge  => self.cmp(l, r, |a,b| a >= b),
            And | Or => unreachable!(),
        }
    }

    fn num2(&self, l: Value, r: Value, fi: fn(i64,i64)->i64, ff: fn(f64,f64)->f64, op: &str) -> Result<Value, Sig> {
        match (l, r) {
            (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(fi(a, b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(ff(a, b))),
            (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(ff(a as f64, b))),
            (Value::Float(a), Value::Int(b))   => Ok(Value::Float(ff(a, b as f64))),
            (l, r) => Err(Sig::err(format!("Cannot {} {} and {}", op, l.kind(), r.kind()))),
        }
    }

    fn cmp(&self, l: Value, r: Value, f: fn(f64,f64)->bool) -> Result<Value, Sig> {
        if let (Value::Str(a), Value::Str(b)) = (&l, &r) {
            let n: f64 = match a.as_str().cmp(b.as_str()) {
                std::cmp::Ordering::Less    => -1.0,
                std::cmp::Ordering::Equal   =>  0.0,
                std::cmp::Ordering::Greater =>  1.0,
            };
            return Ok(Value::Bool(f(n, 0.0)));
        }
        let a = l.as_f64().ok_or_else(|| Sig::err(format!("Cannot compare {}", l.kind())))?;
        let b = r.as_f64().ok_or_else(|| Sig::err(format!("Cannot compare {}", r.kind())))?;
        Ok(Value::Bool(f(a, b)))
    }

    fn eq_vals(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x),   Value::Int(y))   => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Int(x),   Value::Float(y)) => (*x as f64) == *y,
            (Value::Float(x), Value::Int(y))   => *x == (*y as f64),
            (Value::Str(x),   Value::Str(y))   => x == y,
            (Value::Bool(x),  Value::Bool(y))  => x == y,
            (Value::Null,     Value::Null)      => true,
            (Value::List(x),  Value::List(y))  =>
                x.len() == y.len() && x.iter().zip(y).all(|(a,b)| self.eq_vals(a,b)),
            _ => false,
        }
    }

    // ── Statement executor ────────────────────────────────────────
    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Value, Sig> {
        let mut last = Value::Null;
        for s in stmts { last = self.exec(s)?; }
        Ok(last)
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<Value, Sig> {
        match stmt {
            Stmt::Expr(e) => self.eval(e),

            Stmt::Let { name, value } => {
                let v = self.eval(value)?;
                self.def(name, v);
                Ok(Value::Null)
            }

            Stmt::Assign { target, value } => {
                let v = self.eval(value)?;
                match target {
                    Lhs::Ident(n) => {
                        if self.get(n).is_none() {
                            return Err(Sig::err(format!("Undefined variable '{}' — declare with 'let' first", n)));
                        }
                        self.set(n, v);
                    }
                    Lhs::Index { name, idx } => {
                        let i = match self.eval(idx)? {
                            Value::Int(i) => i,
                            _ => return Err(Sig::err("Index must be an integer")),
                        };
                        let list = self.get(name).ok_or_else(|| Sig::err(format!("Undefined variable '{}'", name)))?;
                        if let Value::List(mut l) = list {
                            let n = l.len() as i64;
                            let i = if i < 0 { n + i } else { i } as usize;
                            if i < l.len() { l[i] = v; self.set(name, Value::List(l)); }
                            else { return Err(Sig::err(format!("Index {} out of bounds", i))); }
                        } else {
                            return Err(Sig::err("Index-assign requires a list"));
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

            Stmt::Return(e) => {
                let v = if let Some(ex) = e { self.eval(ex)? } else { Value::Null };
                Err(Sig::Ret(v))
            }

            Stmt::While { cond, body } => {
                loop {
                    if !self.eval(cond)?.truthy() { break; }
                    self.push();
                    let r = self.exec_block(body);
                    self.pop();
                    match r {
                        Ok(_)            => {}
                        Err(Sig::Ret(v)) => return Err(Sig::Ret(v)),
                        Err(e)           => return Err(e),
                    }
                }
                Ok(Value::Null)
            }

            Stmt::For { var, iter, body } => {
                let items = match self.eval(iter)? {
                    Value::List(l) => l,
                    Value::Str(s)  => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    _ => return Err(Sig::err("'for' requires a list or string")),
                };
                for item in items {
                    self.push(); self.def(var, item);
                    let r = self.exec_block(body);
                    self.pop();
                    match r {
                        Ok(_)            => {}
                        Err(Sig::Ret(v)) => return Err(Sig::Ret(v)),
                        Err(e)           => return Err(e),
                    }
                }
                Ok(Value::Null)
            }

            Stmt::FnDef { name, params, body } => {
                let f = Value::Fn {
                    fname:   Some(name.clone()),
                    params:  params.clone(),
                    body:    body.clone(),
                    closure: self.env.clone(),
                };
                self.def(name, f);
                Ok(Value::Null)
            }
        }
    }

    fn run(&mut self, src: &str) -> Result<(), String> {
        let tokens = Lexer::new(src).tokenize()?;
        let stmts  = Parser::new(tokens).parse_program()?;
        for s in &stmts {
            match self.exec(s) {
                Ok(_)            => {}
                Err(Sig::Ret(_)) => return Err("'return' outside of a function".into()),
                Err(Sig::Err(e)) => return Err(e),
            }
        }
        Ok(())
    }
}

// ================================================================
// SECTION 6 – ENTRY POINT
// ================================================================

fn run_main() {
    let args: Vec<String> = std::env::args().collect();
    let mut vm = Interp::new();

    if args.len() > 1 {
        let src = std::fs::read_to_string(&args[1])
            .unwrap_or_else(|e| { eprintln!("ferrite: {}", e); std::process::exit(1); });
        if let Err(e) = vm.run(&src) {
            eprintln!("\x1b[31mError:\x1b[0m {}", e);
            std::process::exit(1);
        }
    } else {
        println!("\x1b[36m╔══════════════════════════════════════╗");
        println!("║   Ferrite v1.0  —  built in Rust     ║");
        println!("║   Type 'exit' or Ctrl+D to quit       ║");
        println!("╚══════════════════════════════════════╝\x1b[0m");
        loop {
            print!("\x1b[33m» \x1b[0m");
            io::stdout().flush().unwrap();
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(0) | Err(_) => { println!(); break; }
                _ => {}
            }
            let line = line.trim();
            if line == "exit" || line == "quit" { break; }
            if line.is_empty() { continue; }
            if let Err(e) = vm.run(line) {
                eprintln!("\x1b[31m  Error: {}\x1b[0m", e);
            }
        }
        println!("Goodbye! 🦀");
    }
}

fn main() {
    // Spawn the interpreter on a thread with a large stack (64 MB).
    // Each recursive call frame in the tree-walking interpreter is large
    // (it destructures Value::Fn which contains closure environments),
    // so the default ~1 MB stack overflows on deeply recursive programs.
    let builder = std::thread::Builder::new()
        .name("ferrite-main".into())
        .stack_size(64 * 1024 * 1024);

    let handler = builder.spawn(run_main).expect("failed to spawn interpreter thread");
    handler.join().expect("interpreter thread panicked");
}
