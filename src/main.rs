// ================================================================
//  Ferrite v1.3 — built on the user's clean v1.2 base
//  Added: try/catch/throw, file I/O, f-strings, variadic fns,
//         list/map unpack, line numbers, import, enumerate/zip,
//         ?? operator, mutable closures, multi-line REPL
// ================================================================

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::rc::Rc;

// ================================================================
// F-STRING PARTS
// ================================================================
#[derive(Debug, Clone, PartialEq)]
enum FsPart {
    Text(String),
    Code(String),
}

// ================================================================
// SECTION 1 – LEXER
// ================================================================
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    FStr(Vec<FsPart>),
    Ident(String),
    // keywords
    Let, Fn, If, Else, While, For, In, Return, Print,
    Break, Continue, Match, Try, Catch, Throw, Import,
    // operators
    Plus, Minus, Star, Slash, Percent, StarStar, SlashSlash,
    Eq, EqEq, BangEq, Bang, Lt, LtEq, Gt, GtEq, And, Or,
    // compound assign
    PlusEq, MinusEq, StarEq, SlashEq,
    // punctuation
    Arrow, QuestionQuestion, DotDot, DotDotDot,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Semicolon, Dot, Colon,
    EOF,
}

struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: u32,
}

impl Lexer {
    fn new(src: &str) -> Self {
        Lexer { input: src.chars().collect(), pos: 0, line: 1 }
    }
    fn peek(&self) -> Option<char> { self.input.get(self.pos).copied() }
    fn peek2(&self) -> Option<char> { self.input.get(self.pos + 1).copied() }
    fn advance(&mut self) -> Option<char> {
        let c = self.input.get(self.pos).copied();
        if c == Some('\n') { self.line += 1; }
        self.pos += 1;
        c
    }

    fn skip_ws(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) { self.advance(); }
            // # line comments (user's choice)
            if self.peek() == Some('#') {
                while matches!(self.peek(), Some(c) if c != '\n') { self.advance(); }
            } else {
                break;
            }
        }
    }

    fn read_str_content(&mut self) -> Result<String, String> {
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
                None => return Err("Unterminated string".into()),
            }
        }
        Ok(s)
    }

    fn read_fstr(&mut self) -> Result<Token, String> {
        // 'f' already consumed; consume opening "
        self.advance();
        let mut parts = Vec::new();
        let mut text = String::new();
        loop {
            match self.advance() {
                Some('"') => break,
                Some('{') => {
                    if !text.is_empty() { parts.push(FsPart::Text(text.clone())); text.clear(); }
                    let mut code = String::new();
                    let mut depth = 1i32;
                    loop {
                        match self.advance() {
                            Some('{') => { depth += 1; code.push('{'); }
                            Some('}') => { depth -= 1; if depth == 0 { break; } code.push('}'); }
                            Some(c)   => code.push(c),
                            None      => return Err("Unterminated f-string expression".into()),
                        }
                    }
                    parts.push(FsPart::Code(code));
                }
                Some('\\') => match self.advance() {
                    Some('n')  => text.push('\n'),
                    Some('t')  => text.push('\t'),
                    Some('\\') => text.push('\\'),
                    Some('"')  => text.push('"'),
                    Some(c)    => { text.push('\\'); text.push(c); }
                    None       => return Err("Unterminated f-string escape".into()),
                },
                Some(c) => text.push(c),
                None => return Err("Unterminated f-string".into()),
            }
        }
        if !text.is_empty() { parts.push(FsPart::Text(text)); }
        Ok(Token::FStr(parts))
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
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            s.push(self.advance().unwrap());
        }
        match s.as_str() {
            "let"      => Token::Let,      "fn"       => Token::Fn,
            "if"       => Token::If,       "else"     => Token::Else,
            "while"    => Token::While,    "for"      => Token::For,
            "in"       => Token::In,       "return"   => Token::Return,
            "print"    => Token::Print,    "break"    => Token::Break,
            "continue" => Token::Continue, "match"    => Token::Match,
            "try"      => Token::Try,      "catch"    => Token::Catch,
            "throw"    => Token::Throw,    "import"   => Token::Import,
            "true"     => Token::Bool(true), "false"  => Token::Bool(false),
            "null"     => Token::Null,
            _          => Token::Ident(s),
        }
    }

    fn tokenize(&mut self) -> Result<Vec<(Token, u32)>, String> {
        let mut toks = Vec::new();
        loop {
            self.skip_ws();
            let line = self.line;
            let tok = match self.peek() {
                None => { toks.push((Token::EOF, line)); break; }
                Some(c) => match c {
                    '"' => { self.advance(); Token::Str(self.read_str_content()?) }
                    'f' if self.peek2() == Some('"') => { self.advance(); self.read_fstr()? }
                    '0'..='9' => self.read_num(),
                    'a'..='z' | 'A'..='Z' | '_' => self.read_ident(),
                    '+' => { self.advance(); if self.peek()==Some('=') { self.advance(); Token::PlusEq  } else { Token::Plus   } }
                    '-' => { self.advance(); if self.peek()==Some('=') { self.advance(); Token::MinusEq } else { Token::Minus  } }
                    '*' => {
                        self.advance();
                        if self.peek() == Some('*') { self.advance(); Token::StarStar }
                        else if self.peek() == Some('=') { self.advance(); Token::StarEq }
                        else { Token::Star }
                    }
                    '/' => {
                        self.advance();
                        if self.peek() == Some('/') { self.advance(); Token::SlashSlash }
                        else if self.peek() == Some('=') { self.advance(); Token::SlashEq }
                        else { Token::Slash }
                    }
                    '%' => { self.advance(); Token::Percent }
                    '(' => { self.advance(); Token::LParen }
                    ')' => { self.advance(); Token::RParen }
                    '{' => { self.advance(); Token::LBrace }
                    '}' => { self.advance(); Token::RBrace }
                    '[' => { self.advance(); Token::LBracket }
                    ']' => { self.advance(); Token::RBracket }
                    ',' => { self.advance(); Token::Comma }
                    ';' => { self.advance(); Token::Semicolon }
                    ':' => { self.advance(); Token::Colon }
                    '.' => {
                        self.advance();
                        if self.peek() == Some('.') {
                            self.advance();
                            if self.peek() == Some('.') { self.advance(); Token::DotDotDot }
                            else { Token::DotDot }
                        } else { Token::Dot }
                    }
                    '=' => {
                        self.advance();
                        if self.peek() == Some('=')  { self.advance(); Token::EqEq  }
                        else if self.peek() == Some('>') { self.advance(); Token::Arrow }
                        else { Token::Eq }
                    }
                    '!' => { self.advance(); if self.peek()==Some('=') { self.advance(); Token::BangEq } else { Token::Bang } }
                    '<' => { self.advance(); if self.peek()==Some('=') { self.advance(); Token::LtEq   } else { Token::Lt } }
                    '>' => { self.advance(); if self.peek()==Some('=') { self.advance(); Token::GtEq   } else { Token::Gt } }
                    '&' => { self.advance(); if self.peek()==Some('&') { self.advance(); Token::And } else { return Err(format!("line {}: Expected '&&'", self.line)); } }
                    '|' => { self.advance(); if self.peek()==Some('|') { self.advance(); Token::Or  } else { return Err(format!("line {}: Expected '||'", self.line)); } }
                    '?' => { self.advance(); if self.peek()==Some('?') { self.advance(); Token::QuestionQuestion } else { return Err(format!("line {}: Expected '??'", self.line)); } }
                    c   => return Err(format!("line {}: Unexpected character '{}'", self.line, c)),
                },
            };
            toks.push((tok, line));
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
    Ident(String), List(Vec<Expr>), Map(Vec<(Expr, Expr)>),
    FStr(Vec<FsPart>),
    BinOp  { op: BinOp,   left: Box<Expr>, right: Box<Expr> },
    Unary  { op: UnOp,    expr: Box<Expr> },
    Call   { func: Box<Expr>, args: Vec<Expr> },
    Index  { obj:  Box<Expr>, idx:  Box<Expr> },
    Field  { obj:  Box<Expr>, name: String },
    If     { cond: Box<Expr>, then: Vec<Stmt>, else_: Option<Vec<Stmt>> },
    Lambda { params: Vec<String>, variadic: Option<String>, body: Vec<Stmt> },
}

#[derive(Debug, Clone)]
enum BinOp { Add, Sub, Mul, Div, Mod, Pow, IDiv, Eq, Ne, Lt, Le, Gt, Ge, And, Or, NullCoal }

#[derive(Debug, Clone)]
enum UnOp { Neg, Not }

#[derive(Debug, Clone)]
struct MatchArm { pattern: MatchPat, body: Vec<Stmt> }

#[derive(Debug, Clone)]
enum MatchPat {
    Literal(Expr),
    Range(Expr, Expr),
    Wildcard,
    Binding(String),
}

#[derive(Debug, Clone)]
enum UnpackItem { Name(String), Rest(String) }

#[derive(Debug, Clone)]
enum Stmt {
    Expr(Expr),
    Let    { name: String, value: Expr },
    LetList { items: Vec<UnpackItem>, value: Expr },
    LetMap  { names: Vec<String>,     value: Expr },
    Assign { target: Lhs, value: Expr },
    CompoundAssign { target: Lhs, op: BinOp, value: Expr },
    Print(Expr),
    Write(Expr),
    Return(Option<Expr>),
    Throw(Expr),
    Break,
    Continue,
    While  { cond: Expr, body: Vec<Stmt> },
    For    { var: String, iter: Expr, body: Vec<Stmt> },
    FnDef  { name: String, params: Vec<String>, variadic: Option<String>, body: Vec<Stmt> },
    Match  { subject: Expr, arms: Vec<MatchArm> },
    TryCatch { body: Vec<Stmt>, catch_var: String, catch_body: Vec<Stmt> },
    Import { path: String },
}

#[derive(Debug, Clone)]
enum Lhs {
    Ident(String),
    Index { obj: Box<Expr>, idx: Expr },
    Field { obj: Box<Expr>, name: String },
}

// ================================================================
// SECTION 3 – PARSER
// ================================================================
struct Parser {
    tokens: Vec<(Token, u32)>,
    pos: usize,
    current_line: u32,
}

impl Parser {
    fn new(tokens: Vec<(Token, u32)>) -> Self {
        Parser { tokens, pos: 0, current_line: 1 }
    }

    fn peek(&self) -> &Token { &self.tokens[self.pos].0 }

    fn peek_tok_at(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).map(|(t,_)| t).unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) -> Token {
        let (t, line) = self.tokens[self.pos].clone();
        self.current_line = line;
        if self.pos + 1 < self.tokens.len() { self.pos += 1; }
        t
    }

    fn check(&self, t: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(t)
    }

    fn expect(&mut self, t: &Token) -> Result<(), String> {
        if self.check(t) { self.advance(); Ok(()) }
        else { Err(format!("line {}: Expected {:?}, got {:?}", self.current_line, t, self.peek())) }
    }

    fn expect_ident(&mut self, ctx: &str) -> Result<String, String> {
        match self.advance() {
            Token::Ident(n) => Ok(n),
            t => Err(format!("line {}: Expected identifier {}, got {:?}", self.current_line, ctx, t)),
        }
    }

    fn semi_after(&mut self, e: &Expr) -> Result<(), String> {
        if self.check(&Token::Semicolon) {
            self.advance();
            Ok(())
        } else if matches!(e, Expr::If { .. } | Expr::Lambda { .. }) {
            Ok(())
        } else if self.check(&Token::RBrace) {
            Ok(())
        } else {
            Err(format!("line {}: Expected ';' after expression, got {:?}", self.current_line, self.peek()))
        }
    }

    fn compound_op(tok: &Token) -> Option<BinOp> {
        match tok {
            Token::PlusEq  => Some(BinOp::Add),
            Token::MinusEq => Some(BinOp::Sub),
            Token::StarEq  => Some(BinOp::Mul),
            Token::SlashEq => Some(BinOp::Div),
            _ => None,
        }
    }

    fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut v = Vec::new();
        while self.peek() != &Token::EOF { v.push(self.parse_stmt()?); }
        Ok(v)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(&Token::LBrace)?;
        let mut v = Vec::new();
        while !self.check(&Token::RBrace) && self.peek() != &Token::EOF {
            v.push(self.parse_stmt()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(v)
    }

    fn parse_params(&mut self) -> Result<(Vec<String>, Option<String>), String> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        let mut variadic = None;
        while !self.check(&Token::RParen) {
            if self.check(&Token::DotDotDot) {
                self.advance();
                variadic = Some(self.expect_ident("after '...'")?);
                break;
            }
            params.push(self.expect_ident("in parameter list")?);
            if self.check(&Token::Comma) { self.advance(); }
        }
        self.expect(&Token::RParen)?;
        Ok((params, variadic))
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        // Two-token lookahead: name <op>= expr
        if let Token::Ident(name) = self.peek().clone() {
            if let Some(op) = Self::compound_op(self.peek_tok_at(1)) {
                self.advance(); self.advance();
                let value = self.parse_expr()?;
                self.expect(&Token::Semicolon)?;
                return Ok(Stmt::CompoundAssign { target: Lhs::Ident(name), op, value });
            }
        }

        match self.peek().clone() {
            Token::Let => {
                self.advance();
                // let [a, b, ...rest] = expr
                if self.check(&Token::LBracket) {
                    self.advance();
                    let mut items = Vec::new();
                    while !self.check(&Token::RBracket) {
                        if self.check(&Token::DotDotDot) {
                            self.advance();
                            items.push(UnpackItem::Rest(self.expect_ident("after '...'")?));
                            break;
                        }
                        items.push(UnpackItem::Name(self.expect_ident("in list unpack")?));
                        if self.check(&Token::Comma) { self.advance(); }
                    }
                    self.expect(&Token::RBracket)?;
                    self.expect(&Token::Eq)?;
                    let value = self.parse_expr()?;
                    self.expect(&Token::Semicolon)?;
                    return Ok(Stmt::LetList { items, value });
                }
                // let {a, b} = expr
                if self.check(&Token::LBrace) {
                    self.advance();
                    let mut names = Vec::new();
                    while !self.check(&Token::RBrace) {
                        names.push(self.expect_ident("in map unpack")?);
                        if self.check(&Token::Comma) { self.advance(); }
                    }
                    self.expect(&Token::RBrace)?;
                    self.expect(&Token::Eq)?;
                    let value = self.parse_expr()?;
                    self.expect(&Token::Semicolon)?;
                    return Ok(Stmt::LetMap { names, value });
                }
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
            Token::Ident(ref s) if s == "write" => {
                self.advance(); self.expect(&Token::LParen)?;
                let e = self.parse_expr()?;
                self.expect(&Token::RParen)?; self.expect(&Token::Semicolon)?;
                Ok(Stmt::Write(e))
            }
            Token::Return => {
                self.advance();
                if self.check(&Token::Semicolon) { self.advance(); return Ok(Stmt::Return(None)); }
                let e = self.parse_expr()?; self.expect(&Token::Semicolon)?;
                Ok(Stmt::Return(Some(e)))
            }
            Token::Throw => {
                self.advance();
                let e = self.parse_expr()?; self.expect(&Token::Semicolon)?;
                Ok(Stmt::Throw(e))
            }
            Token::Break    => { self.advance(); self.expect(&Token::Semicolon)?; Ok(Stmt::Break) }
            Token::Continue => { self.advance(); self.expect(&Token::Semicolon)?; Ok(Stmt::Continue) }
            Token::While => {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::While { cond, body })
            }
            Token::For => {
                self.advance();
                // for [a, b] in ... — desugar to for __item in ... { let [a,b] = __item; ... }
                if self.check(&Token::LBracket) {
                    self.advance();
                    let mut items = Vec::new();
                    while !self.check(&Token::RBracket) {
                        if self.check(&Token::DotDotDot) {
                            self.advance();
                            items.push(UnpackItem::Rest(self.expect_ident("after '...'")?));
                            break;
                        }
                        items.push(UnpackItem::Name(self.expect_ident("in for destructure")?));
                        if self.check(&Token::Comma) { self.advance(); }
                    }
                    self.expect(&Token::RBracket)?;
                    self.expect(&Token::In)?;
                    let iter = self.parse_expr()?;
                    let mut body = self.parse_block()?;
                    // Prepend destructuring
                    body.insert(0, Stmt::LetList {
                        items,
                        value: Expr::Ident("__item".to_string()),
                    });
                    return Ok(Stmt::For { var: "__item".to_string(), iter, body });
                }
                let var = self.expect_ident("in 'for'")?;
                self.expect(&Token::In)?;
                let iter = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::For { var, iter, body })
            }
            Token::Fn => {
                self.advance();
                let name = self.expect_ident("after 'fn'")?;
                let (params, variadic) = self.parse_params()?;
                let body = self.parse_block()?;
                Ok(Stmt::FnDef { name, params, variadic, body })
            }
            Token::Match => {
                self.advance();
                let subject = self.parse_expr()?;
                self.expect(&Token::LBrace)?;
                let mut arms = Vec::new();
                while !self.check(&Token::RBrace) && self.peek() != &Token::EOF {
                    let pattern = self.parse_match_pat()?;
                    self.expect(&Token::Arrow)?;
                    let body = self.parse_block()?;
                    if self.check(&Token::Comma) { self.advance(); }
                    arms.push(MatchArm { pattern, body });
                }
                self.expect(&Token::RBrace)?;
                Ok(Stmt::Match { subject, arms })
            }
            Token::Try => {
                self.advance();
                let body = self.parse_block()?;
                self.expect(&Token::Catch)?;
                let has_paren = self.check(&Token::LParen);
                if has_paren { self.advance(); }
                let catch_var = self.expect_ident("in catch")?;
                if has_paren { self.expect(&Token::RParen)?; }
                let catch_body = self.parse_block()?;
                Ok(Stmt::TryCatch { body, catch_var, catch_body })
            }
            Token::Import => {
                self.advance();
                match self.advance() {
                    Token::Str(path) => { self.expect(&Token::Semicolon)?; Ok(Stmt::Import { path }) }
                    t => Err(format!("line {}: import expects a string path, got {:?}", self.current_line, t)),
                }
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
                        let obj = Box::new(Expr::Ident(name));
                        return Ok(Stmt::Assign { target: Lhs::Index { obj, idx }, value });
                    }
                    if let Some(op) = Self::compound_op(self.peek()) {
                        self.advance();
                        let value = self.parse_expr()?; self.expect(&Token::Semicolon)?;
                        let obj = Box::new(Expr::Ident(name));
                        return Ok(Stmt::CompoundAssign { target: Lhs::Index { obj, idx }, op, value });
                    }
                }

                self.pos = saved;
                let e = self.parse_expr()?;
                if self.check(&Token::Eq) {
                    self.advance();
                    let value = self.parse_expr()?; self.expect(&Token::Semicolon)?;
                    let target = expr_to_lhs(e)?;
                    return Ok(Stmt::Assign { target, value });
                }
                if let Some(op) = Self::compound_op(self.peek()) {
                    self.advance();
                    let value = self.parse_expr()?; self.expect(&Token::Semicolon)?;
                    let target = expr_to_lhs(e)?;
                    return Ok(Stmt::CompoundAssign { target, op, value });
                }
                self.semi_after(&e)?;
                Ok(Stmt::Expr(e))
            }
            _ => {
                let e = self.parse_expr()?;
                self.semi_after(&e)?;
                Ok(Stmt::Expr(e))
            }
        }
    }

    // Kept from user's v1.2 — uses parse_primary to avoid consuming `..` as Dot
    fn parse_match_pat(&mut self) -> Result<MatchPat, String> {
        if let Token::Ident(ref s) = self.peek().clone() {
            if s == "_" { self.advance(); return Ok(MatchPat::Wildcard); }
        }
        let neg = if self.check(&Token::Minus) { self.advance(); true } else { false };
        let mut e = self.parse_primary()?;
        if neg { e = Expr::Unary { op: UnOp::Neg, expr: Box::new(e) }; }
        // Range: expr .. expr
        if self.check(&Token::DotDot) {
            self.advance();
            let neg2 = if self.check(&Token::Minus) { self.advance(); true } else { false };
            let mut end = self.parse_primary()?;
            if neg2 { end = Expr::Unary { op: UnOp::Neg, expr: Box::new(end) }; }
            return Ok(MatchPat::Range(e, end));
        }
        if let Expr::Ident(n) = &e { return Ok(MatchPat::Binding(n.clone())); }
        Ok(MatchPat::Literal(e))
    }

    fn parse_expr(&mut self) -> Result<Expr, String> { self.parse_null_coal() }

    fn parse_null_coal(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_or()?;
        while self.check(&Token::QuestionQuestion) {
            self.advance(); let r = self.parse_or()?;
            l = Expr::BinOp { op: BinOp::NullCoal, left: l.into(), right: r.into() };
        }
        Ok(l)
    }
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
        let mut l = self.parse_pow()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul, Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod, Token::SlashSlash => BinOp::IDiv,
                _ => break,
            };
            self.advance(); let r = self.parse_pow()?;
            l = Expr::BinOp { op, left: l.into(), right: r.into() };
        }
        Ok(l)
    }
    fn parse_pow(&mut self) -> Result<Expr, String> {
        let base = self.parse_unary()?;
        if self.check(&Token::StarStar) {
            self.advance();
            let exp = self.parse_pow()?;
            Ok(Expr::BinOp { op: BinOp::Pow, left: base.into(), right: exp.into() })
        } else { Ok(base) }
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
                    } else { e = Expr::Field { obj: e.into(), name }; }
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
            Token::FStr(p)  => Ok(Expr::FStr(p)),
            Token::LParen   => { let e = self.parse_expr()?; self.expect(&Token::RParen)?; Ok(e) }
            Token::LBracket => {
                let mut items = Vec::new();
                while !self.check(&Token::RBracket) { items.push(self.parse_expr()?); if self.check(&Token::Comma) { self.advance(); } }
                self.expect(&Token::RBracket)?;
                Ok(Expr::List(items))
            }
            Token::LBrace => {
                let mut pairs = Vec::new();
                while !self.check(&Token::RBrace) {
                    let k = self.parse_expr()?; self.expect(&Token::Colon)?; let v = self.parse_expr()?;
                    pairs.push((k, v));
                    if self.check(&Token::Comma) { self.advance(); }
                }
                self.expect(&Token::RBrace)?;
                Ok(Expr::Map(pairs))
            }
            Token::If => self.parse_if_body(),
            Token::Fn => {
                let (params, variadic) = self.parse_params()?;
                let body = self.parse_block()?;
                Ok(Expr::Lambda { params, variadic, body })
            }
            tok => Err(format!("line {}: Unexpected token: {:?}", self.current_line, tok)),
        }
    }
    fn parse_if_body(&mut self) -> Result<Expr, String> {
        let cond = self.parse_expr()?;
        let then = self.parse_block()?;
        let else_ = if self.check(&Token::Else) {
            self.advance();
            if self.check(&Token::If) {
                self.advance();
                let inner = self.parse_if_body()?;
                Some(vec![Stmt::Expr(inner)])
            } else { Some(self.parse_block()?) }
        } else { None };
        Ok(Expr::If { cond: cond.into(), then, else_ })
    }
}

fn expr_to_lhs(e: Expr) -> Result<Lhs, String> {
    match e {
        Expr::Ident(n)           => Ok(Lhs::Ident(n)),
        Expr::Index { obj, idx } => Ok(Lhs::Index { obj, idx: *idx }),
        Expr::Field { obj, name }=> Ok(Lhs::Field { obj, name }),
        _                        => Err("Invalid assignment target".into()),
    }
}

// ================================================================
// SECTION 4 – VALUES  &  SIGNALS
// ================================================================

// Mutable closures: scopes are Rc<RefCell<HashMap>> so closures share
// references rather than copying, enabling counter/accumulator patterns.
type Scope = Rc<RefCell<HashMap<String, Value>>>;
type Env   = Vec<Scope>;

#[derive(Debug, Clone)]
enum Value {
    Int(i64), Float(f64), Str(String), Bool(bool), Null,
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Fn { fname: Option<String>, params: Vec<String>, variadic: Option<String>, body: Vec<Stmt>, closure: Env },
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
            Value::Map(m) => {
                write!(f, "{{")?;
                let mut pairs: Vec<_> = m.iter().collect();
                pairs.sort_by_key(|(k, _)| (*k).clone());
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    if let Value::Str(s) = v { write!(f, "\"{}\": \"{}\"", k, s)?; }
                    else { write!(f, "\"{}\": {}", k, v)?; }
                }
                write!(f, "}}")
            }
            Value::Fn { params, variadic, .. } => {
                let mut all = params.clone();
                if let Some(v) = variadic { all.push(format!("...{}", v)); }
                write!(f, "<fn({})>", all.join(", "))
            }
            Value::Builtin(n) => write!(f, "<builtin:{}>", n),
        }
    }
}

impl Value {
    fn truthy(&self) -> bool {
        match self {
            Value::Bool(b)  => *b, Value::Null => false, Value::Int(0) => false,
            Value::Str(s)   => !s.is_empty(), Value::List(l) => !l.is_empty(),
            Value::Map(m)   => !m.is_empty(), _ => true,
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Value::Int(_)  => "int",   Value::Float(_) => "float", Value::Str(_)  => "string",
            Value::Bool(_) => "bool",  Value::Null     => "null",  Value::List(_) => "list",
            Value::Map(_)  => "map",   _               => "function",
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self { Value::Int(n) => Some(*n as f64), Value::Float(f) => Some(*f), _ => None }
    }
    fn to_map_key(&self) -> Option<String> {
        match self {
            Value::Str(s)  => Some(s.clone()),
            Value::Int(n)  => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum Sig { Ret(Value), Brk, Cont, Throw(Value), Err(String) }

impl Sig { fn err(s: impl Into<String>) -> Self { Sig::Err(s.into()) } }

// ================================================================
// SECTION 5 – INTERPRETER
// ================================================================
struct Interp {
    env: Env,
    current_line: u32,
    import_base: Option<PathBuf>,
}

impl Interp {
    fn new() -> Self {
        let mut g: HashMap<String, Value> = HashMap::new();
        for n in &[
            "len","push","pop","str","int","float","type","range",
            "input","sqrt","abs","max","min","floor","ceil","round",
            "assert","keys","values","has_key","delete",
            "sort","reverse","contains","map","filter","reduce",
            "split","join","replace","starts_with","ends_with","trim","upper","lower","chars",
            "pow","log","log2","log10","sin","cos","tan","atan","atan2","pi","e","inf",
            "format","write","exit",
            "enumerate","zip",
            "read_file","write_file","append_file","file_exists",
        ] { g.insert(n.to_string(), Value::Builtin(n.to_string())); }
        g.insert("PI".into(),  Value::Float(std::f64::consts::PI));
        g.insert("E".into(),   Value::Float(std::f64::consts::E));
        g.insert("INF".into(), Value::Float(f64::INFINITY));
        Interp { env: vec![Rc::new(RefCell::new(g))], current_line: 0, import_base: None }
    }

    fn get(&self, n: &str) -> Option<Value> {
        for s in self.env.iter().rev() {
            if let Some(v) = s.borrow().get(n) { return Some(v.clone()); }
        }
        None
    }
    fn set(&self, n: &str, v: Value) {
        for s in self.env.iter().rev() {
            if s.borrow().contains_key(n) { s.borrow_mut().insert(n.to_string(), v); return; }
        }
        self.env.last().unwrap().borrow_mut().insert(n.to_string(), v);
    }
    fn def(&self, n: &str, v: Value) { self.env.last().unwrap().borrow_mut().insert(n.to_string(), v); }
    fn push_scope(&mut self) { self.env.push(Rc::new(RefCell::new(HashMap::new()))); }
    fn pop_scope(&mut self)  { self.env.pop(); }

    // ── Builtins ──────────────────────────────────────────────────
    fn builtin(&mut self, name: &str, a: Vec<Value>) -> Result<Value, Sig> {
        let n = a.len();
        macro_rules! arity { ($k:expr) => { if n != $k { return Err(Sig::err(format!("{}() expects {} arg(s), got {}", name, $k, n))); } }; }
        macro_rules! e { ($s:expr) => { return Err(Sig::err($s)) }; }

        match name {
            "len" => { arity!(1); match &a[0] {
                Value::List(l) => Ok(Value::Int(l.len() as i64)),
                Value::Str(s)  => Ok(Value::Int(s.chars().count() as i64)),
                Value::Map(m)  => Ok(Value::Int(m.len() as i64)),
                v => e!(format!("len() not supported for {}", v.kind()))
            }}
            "push" => { arity!(2); match a[0].clone() { Value::List(mut l) => { l.push(a[1].clone()); Ok(Value::List(l)) } v => e!(format!("push() needs list, got {}", v.kind())) }}
            "pop"  => { arity!(1); match a[0].clone() { Value::List(mut l) => Ok(l.pop().unwrap_or(Value::Null)), v => e!(format!("pop() needs list, got {}", v.kind())) }}
            "str"  => { arity!(1); Ok(Value::Str(a[0].to_string())) }
            "type" => { arity!(1); Ok(Value::Str(a[0].kind().to_string())) }
            "int"  => { arity!(1); match &a[0] {
                Value::Int(x)   => Ok(Value::Int(*x)),
                Value::Float(f) => Ok(Value::Int(*f as i64)),
                Value::Str(s)   => s.trim().parse::<i64>().map(Value::Int).map_err(|_| Sig::err(format!("Cannot parse \"{}\" as int", s))),
                v => e!(format!("Cannot convert {} to int", v.kind()))
            }}
            "float" => { arity!(1); match &a[0] {
                Value::Float(f) => Ok(Value::Float(*f)),
                Value::Int(x)   => Ok(Value::Float(*x as f64)),
                Value::Str(s)   => s.trim().parse::<f64>().map(Value::Float).map_err(|_| Sig::err(format!("Cannot parse \"{}\" as float", s))),
                v => e!(format!("Cannot convert {} to float", v.kind()))
            }}
            "sqrt"  => { arity!(1); a[0].as_f64().map(|f| Value::Float(f.sqrt())).ok_or_else(|| Sig::err("sqrt() needs a number")) }
            "abs"   => { arity!(1); match a[0] { Value::Int(x) => Ok(Value::Int(x.abs())), Value::Float(f) => Ok(Value::Float(f.abs())), _ => e!("abs() needs a number") } }
            "floor" => { arity!(1); a[0].as_f64().map(|f| Value::Int(f.floor() as i64)).ok_or_else(|| Sig::err("floor() needs a number")) }
            "ceil"  => { arity!(1); a[0].as_f64().map(|f| Value::Int(f.ceil()  as i64)).ok_or_else(|| Sig::err("ceil() needs a number")) }
            "round" => { arity!(1); a[0].as_f64().map(|f| Value::Int(f.round() as i64)).ok_or_else(|| Sig::err("round() needs a number")) }
            "max" => {
                if n < 1 { e!("max() needs at least 1 arg"); }
                let items: Vec<Value> = if n == 1 { match a[0].clone() { Value::List(l) => l, v => vec![v] } } else { a.clone() };
                let mut best = items[0].clone();
                for v in &items[1..] { if v.as_f64().ok_or_else(|| Sig::err("max() needs numbers"))? > best.as_f64().ok_or_else(|| Sig::err("max() needs numbers"))? { best = v.clone(); } }
                Ok(best)
            }
            "min" => {
                if n < 1 { e!("min() needs at least 1 arg"); }
                let items: Vec<Value> = if n == 1 { match a[0].clone() { Value::List(l) => l, v => vec![v] } } else { a.clone() };
                let mut best = items[0].clone();
                for v in &items[1..] { if v.as_f64().ok_or_else(|| Sig::err("min() needs numbers"))? < best.as_f64().ok_or_else(|| Sig::err("min() needs numbers"))? { best = v.clone(); } }
                Ok(best)
            }
            "range" => match n {
                1 => match a[0] { Value::Int(x) => Ok(Value::List((0..x).map(Value::Int).collect())), _ => e!("range() needs int") },
                2 => match (&a[0], &a[1]) { (Value::Int(x), Value::Int(y)) => Ok(Value::List((*x..*y).map(Value::Int).collect())), _ => e!("range() needs ints") },
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
                let mut line = String::new(); io::stdin().read_line(&mut line).unwrap();
                Ok(Value::Str(line.trim_end_matches('\n').to_string()))
            }
            "assert" => {
                if n < 1 || n > 2 { e!("assert() takes 1 or 2 args"); }
                if !a[0].truthy() { e!(if n == 2 { a[1].to_string() } else { "Assertion failed".into() }); }
                Ok(Value::Null)
            }
            "exit" => { std::process::exit(if n == 1 { match a[0] { Value::Int(c) => c as i32, _ => 0 } } else { 0 }); }
            // math
            "pow"   => { arity!(2); let b = a[0].as_f64().ok_or_else(|| Sig::err("pow() needs numbers"))?; let x = a[1].as_f64().ok_or_else(|| Sig::err("pow() needs numbers"))?; Ok(Value::Float(b.powf(x))) }
            "log"   => { arity!(1); a[0].as_f64().map(|f| Value::Float(f.ln()   )).ok_or_else(|| Sig::err("log() needs a number")) }
            "log2"  => { arity!(1); a[0].as_f64().map(|f| Value::Float(f.log2() )).ok_or_else(|| Sig::err("log2() needs a number")) }
            "log10" => { arity!(1); a[0].as_f64().map(|f| Value::Float(f.log10())).ok_or_else(|| Sig::err("log10() needs a number")) }
            "sin"   => { arity!(1); a[0].as_f64().map(|f| Value::Float(f.sin()  )).ok_or_else(|| Sig::err("sin() needs a number")) }
            "cos"   => { arity!(1); a[0].as_f64().map(|f| Value::Float(f.cos()  )).ok_or_else(|| Sig::err("cos() needs a number")) }
            "tan"   => { arity!(1); a[0].as_f64().map(|f| Value::Float(f.tan()  )).ok_or_else(|| Sig::err("tan() needs a number")) }
            "atan"  => { arity!(1); a[0].as_f64().map(|f| Value::Float(f.atan() )).ok_or_else(|| Sig::err("atan() needs a number")) }
            "atan2" => { arity!(2); let y = a[0].as_f64().ok_or_else(|| Sig::err("atan2() needs numbers"))?; let x = a[1].as_f64().ok_or_else(|| Sig::err("atan2() needs numbers"))?; Ok(Value::Float(y.atan2(x))) }
            "pi"    => { arity!(0); Ok(Value::Float(std::f64::consts::PI)) }
            "e"     => { arity!(0); Ok(Value::Float(std::f64::consts::E)) }
            "inf"   => { arity!(0); Ok(Value::Float(f64::INFINITY)) }
            "format" => {
                if n < 1 { e!("format() needs at least a template string"); }
                let tmpl = match &a[0] { Value::Str(s) => s.clone(), _ => e!("format() first arg must be a string") };
                let mut result = String::new(); let mut idx = 1usize;
                let mut chars = tmpl.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '{' && chars.peek() == Some(&'}') { chars.next(); if idx < n { result.push_str(&a[idx].to_string()); idx += 1; } else { e!("format(): not enough arguments"); } }
                    else { result.push(c); }
                }
                Ok(Value::Str(result))
            }
            "write" => { arity!(1); print!("{}", a[0]); io::stdout().flush().unwrap(); Ok(Value::Null) }
            // map ops
            "keys"    => { arity!(1); match &a[0] { Value::Map(m) => { let mut ks: Vec<Value> = m.keys().map(|k| Value::Str(k.clone())).collect(); ks.sort_by(|a,b| a.to_string().cmp(&b.to_string())); Ok(Value::List(ks)) } v => e!(format!("keys() needs a map, got {}", v.kind())) }}
            "values"  => { arity!(1); match &a[0] { Value::Map(m) => { let mut p: Vec<_> = m.iter().collect(); p.sort_by_key(|(k,_)| (*k).clone()); Ok(Value::List(p.into_iter().map(|(_,v)| v.clone()).collect())) } v => e!(format!("values() needs a map, got {}", v.kind())) }}
            "has_key" => { arity!(2); match &a[0] { Value::Map(m) => { let k = a[1].to_map_key().ok_or_else(|| Sig::err("Map key must be string/int/bool"))?; Ok(Value::Bool(m.contains_key(&k))) } v => e!(format!("has_key() needs a map, got {}", v.kind())) }}
            "delete"  => { arity!(2); match a[0].clone() { Value::Map(mut m) => { let k = a[1].to_map_key().ok_or_else(|| Sig::err("Map key must be string/int/bool"))?; m.remove(&k); Ok(Value::Map(m)) } v => e!(format!("delete() needs a map, got {}", v.kind())) }}
            // list ops
            "sort" => { arity!(1); match a[0].clone() {
                Value::List(mut l) => {
                    let mut err: Option<String> = None;
                    l.sort_by(|a, b| match (a.as_f64(), b.as_f64()) {
                        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
                        _ => match (a, b) { (Value::Str(x), Value::Str(y)) => x.cmp(y), _ => { err = Some("sort() requires numbers or strings".into()); std::cmp::Ordering::Equal } }
                    });
                    if let Some(e) = err { return Err(Sig::err(e)); }
                    Ok(Value::List(l))
                }
                v => e!(format!("sort() needs a list, got {}", v.kind()))
            }}
            "reverse"  => { arity!(1); match a[0].clone() { Value::List(mut l) => { l.reverse(); Ok(Value::List(l)) } Value::Str(s) => Ok(Value::Str(s.chars().rev().collect())), v => e!(format!("reverse() needs list or string, got {}", v.kind())) }}
            "contains" => { arity!(2); match &a[0] {
                Value::List(l) => Ok(Value::Bool(l.iter().any(|v| self.eq_vals(v, &a[1])))),
                Value::Str(s)  => match &a[1] { Value::Str(sub) => Ok(Value::Bool(s.contains(sub.as_str()))), _ => e!("contains() on string needs a string needle") },
                Value::Map(m)  => { let k = a[1].to_map_key().ok_or_else(|| Sig::err("Map key must be string/int/bool"))?; Ok(Value::Bool(m.contains_key(&k))) }
                v => e!(format!("contains() not supported for {}", v.kind()))
            }}
            "map" => {
                if n != 2 { e!("map() takes (list, fn)"); }
                let list = match a[0].clone() { Value::List(l) => l, v => e!(format!("map() needs a list, got {}", v.kind())) };
                let func = a[1].clone(); let mut out = Vec::new();
                for item in list { out.push(self.call(func.clone(), vec![item])?); }
                Ok(Value::List(out))
            }
            "filter" => {
                if n != 2 { e!("filter() takes (list, fn)"); }
                let list = match a[0].clone() { Value::List(l) => l, v => e!(format!("filter() needs a list, got {}", v.kind())) };
                let func = a[1].clone(); let mut out = Vec::new();
                for item in list { if self.call(func.clone(), vec![item.clone()])?.truthy() { out.push(item); } }
                Ok(Value::List(out))
            }
            "reduce" => {
                if n < 2 || n > 3 { e!("reduce() takes (list, fn) or (list, fn, init)"); }
                let list = match a[0].clone() { Value::List(l) => l, v => e!(format!("reduce() needs a list, got {}", v.kind())) };
                let func = a[1].clone();
                let (mut acc, start) = if n == 3 { (a[2].clone(), 0) } else if !list.is_empty() { (list[0].clone(), 1) } else { e!("reduce() on empty list needs initial value") };
                for item in list.into_iter().skip(start) { acc = self.call(func.clone(), vec![acc, item])?; }
                Ok(acc)
            }
            "enumerate" => {
                arity!(1);
                let list = match a[0].clone() { Value::List(l) => l, v => e!(format!("enumerate() needs a list, got {}", v.kind())) };
                Ok(Value::List(list.into_iter().enumerate().map(|(i, v)| Value::List(vec![Value::Int(i as i64), v])).collect()))
            }
            "zip" => {
                if n < 2 { e!("zip() takes at least 2 lists"); }
                let lists: Result<Vec<Vec<Value>>, Sig> = a.iter().map(|x| match x { Value::List(l) => Ok(l.clone()), v => Err(Sig::err(format!("zip() needs lists, got {}", v.kind()))) }).collect();
                let lists = lists?;
                let min_len = lists.iter().map(|l| l.len()).min().unwrap_or(0);
                Ok(Value::List((0..min_len).map(|i| Value::List(lists.iter().map(|l| l[i].clone()).collect())).collect()))
            }
            // string ops
            "split" => {
                if n < 1 || n > 2 { e!("split() takes (string) or (string, sep)"); }
                let s = match &a[0] { Value::Str(s) => s.clone(), v => e!(format!("split() needs a string, got {}", v.kind())) };
                let parts: Vec<Value> = if n == 2 { match &a[1] { Value::Str(sep) => s.split(sep.as_str()).map(|p| Value::Str(p.to_string())).collect(), _ => e!("split() separator must be a string") } } else { s.split_whitespace().map(|p| Value::Str(p.to_string())).collect() };
                Ok(Value::List(parts))
            }
            "join"        => { if n != 2 { e!("join() takes (list, sep)"); } let list = match &a[0] { Value::List(l) => l, v => e!(format!("join() needs a list, got {}", v.kind())) }; let sep = match &a[1] { Value::Str(s) => s.clone(), _ => e!("join() separator must be a string") }; Ok(Value::Str(list.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(&sep))) }
            "replace"     => { if n != 3 { e!("replace() takes (string, from, to)"); } match (&a[0], &a[1], &a[2]) { (Value::Str(s), Value::Str(f2), Value::Str(t)) => Ok(Value::Str(s.replace(f2.as_str(), t.as_str()))), _ => e!("replace() needs three strings") } }
            "starts_with" => { if n != 2 { e!("starts_with() takes 2 args"); } match (&a[0], &a[1]) { (Value::Str(s), Value::Str(p)) => Ok(Value::Bool(s.starts_with(p.as_str()))), _ => e!("starts_with() needs two strings") } }
            "ends_with"   => { if n != 2 { e!("ends_with() takes 2 args");   } match (&a[0], &a[1]) { (Value::Str(s), Value::Str(p)) => Ok(Value::Bool(s.ends_with(p.as_str()))),   _ => e!("ends_with() needs two strings") } }
            "trim"  => { arity!(1); match &a[0] { Value::Str(s) => Ok(Value::Str(s.trim().to_string())),       v => e!(format!("trim() needs a string, got {}",  v.kind())) } }
            "upper" => { arity!(1); match &a[0] { Value::Str(s) => Ok(Value::Str(s.to_uppercase())),           v => e!(format!("upper() needs a string, got {}", v.kind())) } }
            "lower" => { arity!(1); match &a[0] { Value::Str(s) => Ok(Value::Str(s.to_lowercase())),           v => e!(format!("lower() needs a string, got {}", v.kind())) } }
            "chars" => { arity!(1); match &a[0] { Value::Str(s) => Ok(Value::List(s.chars().map(|c| Value::Str(c.to_string())).collect())), v => e!(format!("chars() needs a string, got {}", v.kind())) } }
            // file I/O
            "read_file"   => { arity!(1); let p = match &a[0] { Value::Str(s) => s.clone(), _ => e!("read_file() needs a string path") }; std::fs::read_to_string(&p).map(Value::Str).map_err(|err| Sig::err(format!("read_file('{}'): {}", p, err))) }
            "write_file"  => { if n != 2 { e!("write_file() takes (path, content)"); } let p = match &a[0] { Value::Str(s) => s.clone(), _ => e!("write_file() needs a string path") }; std::fs::write(&p, a[1].to_string()).map(|_| Value::Null).map_err(|err| Sig::err(format!("write_file('{}'): {}", p, err))) }
            "append_file" => {
                if n != 2 { e!("append_file() takes (path, content)"); }
                let p = match &a[0] { Value::Str(s) => s.clone(), _ => e!("append_file() needs a string path") };
                use std::io::Write as IoWrite;
                let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&p).map_err(|err| Sig::err(format!("append_file('{}'): {}", p, err)))?;
                f.write_all(a[1].to_string().as_bytes()).map(|_| Value::Null).map_err(|err| Sig::err(format!("append_file('{}'): {}", p, err)))
            }
            "file_exists" => { arity!(1); let p = match &a[0] { Value::Str(s) => s.clone(), _ => e!("file_exists() needs a string path") }; Ok(Value::Bool(std::path::Path::new(&p).exists())) }
            _ => e!(format!("Unknown builtin '{}'", name)),
        }
    }

    // ── Eval ──────────────────────────────────────────────────────
    fn eval(&mut self, e: &Expr) -> Result<Value, Sig> {
        match e {
            Expr::Int(n)    => Ok(Value::Int(*n)),
            Expr::Float(f)  => Ok(Value::Float(*f)),
            Expr::Str(s)    => Ok(Value::Str(s.clone())),
            Expr::Bool(b)   => Ok(Value::Bool(*b)),
            Expr::Null      => Ok(Value::Null),
            Expr::Ident(n)  => self.get(n).ok_or_else(|| Sig::err(format!("line {}: Undefined variable '{}'", self.current_line, n))),
            Expr::List(xs)  => { let mut v = Vec::new(); for x in xs { v.push(self.eval(x)?); } Ok(Value::List(v)) }
            Expr::Map(pairs)=> {
                let mut m = HashMap::new();
                for (k, v) in pairs {
                    let kv = self.eval(k)?;
                    let key = kv.to_map_key().ok_or_else(|| Sig::err("Map key must be string, int, or bool"))?;
                    m.insert(key, self.eval(v)?);
                }
                Ok(Value::Map(m))
            }
            Expr::FStr(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        FsPart::Text(s) => result.push_str(s),
                        FsPart::Code(src) => {
                            let toks = Lexer::new(src).tokenize().map_err(|e| Sig::err(format!("In f-string: {}", e)))?;
                            let mut p = Parser::new(toks);
                            let expr = p.parse_expr().map_err(|e| Sig::err(format!("In f-string: {}", e)))?;
                            result.push_str(&self.eval(&expr)?.to_string());
                        }
                    }
                }
                Ok(Value::Str(result))
            }
            Expr::BinOp { op, left, right } => {
                match op {
                    BinOp::And     => { let l = self.eval(left)?; return if !l.truthy() { Ok(Value::Bool(false)) } else { self.eval(right) }; }
                    BinOp::Or      => { let l = self.eval(left)?; return if  l.truthy() { Ok(l) }                 else { self.eval(right) }; }
                    BinOp::NullCoal=> { let l = self.eval(left)?; return if matches!(l, Value::Null) { self.eval(right) } else { Ok(l) }; }
                    _ => {}
                }
                let l = self.eval(left)?; let r = self.eval(right)?;
                self.binop(op, l, r)
            }
            Expr::Unary { op, expr } => {
                let v = self.eval(expr)?;
                match op {
                    UnOp::Neg => match v { Value::Int(n) => Ok(Value::Int(-n)), Value::Float(f) => Ok(Value::Float(-f)), _ => Err(Sig::err("Unary '-' requires a number")) },
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
                let ov = self.eval(obj)?; let iv = self.eval(idx)?;
                match (ov, iv) {
                    (Value::List(l), Value::Int(i)) => { let n = l.len() as i64; let i = if i < 0 { n + i } else { i }; l.into_iter().nth(i as usize).ok_or_else(|| Sig::err(format!("Index {} out of bounds", i))) }
                    (Value::Str(s),  Value::Int(i)) => { let ch: Vec<char> = s.chars().collect(); let n = ch.len() as i64; let i = if i < 0 { n + i } else { i }; ch.get(i as usize).map(|c| Value::Str(c.to_string())).ok_or_else(|| Sig::err(format!("String index {} out of bounds", i))) }
                    (Value::Map(m),  k) => { let key = k.to_map_key().ok_or_else(|| Sig::err("Map key must be string/int/bool"))?; Ok(m.get(&key).cloned().unwrap_or(Value::Null)) }
                    _ => Err(Sig::err("Invalid index operation")),
                }
            }
            Expr::Field { obj, name } => {
                let ov = self.eval(obj)?;
                match (&ov, name.as_str()) {
                    (Value::Str(s),  "len")   => Ok(Value::Int(s.chars().count() as i64)),
                    (Value::Str(s),  "upper") => Ok(Value::Str(s.to_uppercase())),
                    (Value::Str(s),  "lower") => Ok(Value::Str(s.to_lowercase())),
                    (Value::Str(s),  "trim")  => Ok(Value::Str(s.trim().to_string())),
                    (Value::Str(s),  "chars") => Ok(Value::List(s.chars().map(|c| Value::Str(c.to_string())).collect())),
                    (Value::List(l), "len")   => Ok(Value::Int(l.len() as i64)),
                    (Value::Map(m),  "len")   => Ok(Value::Int(m.len() as i64)),
                    (Value::Map(m),  field)   => Ok(m.get(field).cloned().unwrap_or(Value::Null)),
                    _ => Err(Sig::err(format!("No property '{}' on {}", name, ov.kind()))),
                }
            }
            Expr::If { cond, then, else_ } => {
                let cv = self.eval(cond)?;
                self.push_scope();
                let r = if cv.truthy() { self.exec_block(then) }
                        else if let Some(eb) = else_ { self.exec_block(eb) }
                        else { Ok(Value::Null) };
                self.pop_scope();
                r
            }
            Expr::Lambda { params, variadic, body } => {
                Ok(Value::Fn { fname: None, params: params.clone(), variadic: variadic.clone(), body: body.clone(), closure: self.env.clone() })
            }
        }
    }

    fn call(&mut self, fv: Value, args: Vec<Value>) -> Result<Value, Sig> {
        match fv {
            Value::Builtin(n) => self.builtin(&n, args),
            Value::Fn { fname, params, variadic, body, closure } => {
                let required = params.len();
                if variadic.is_none() && args.len() != required {
                    return Err(Sig::err(format!("Expected {} arg(s), got {}", required, args.len())));
                }
                if args.len() < required {
                    return Err(Sig::err(format!("Expected at least {} arg(s), got {}", required, args.len())));
                }
                // Swap in the closure — shared Rc refs so mutations are visible
                let saved = std::mem::replace(&mut self.env, closure);
                self.push_scope();
                for (p, v) in params.iter().zip(args.iter()) { self.def(p, v.clone()); }
                if let Some(ref vname) = variadic {
                    self.def(vname, Value::List(args.into_iter().skip(required).collect()));
                }
                // Self-reference for recursion
                if let Some(ref n) = fname {
                    let fn_closure = self.env[..self.env.len() - 1].to_vec();
                    self.def(n, Value::Fn { fname: fname.clone(), params: params.clone(), variadic: variadic.clone(), body: body.clone(), closure: fn_closure });
                }
                let r = self.exec_block(&body);
                self.env = saved;
                match r { Ok(_) => Ok(Value::Null), Err(Sig::Ret(v)) => Ok(v), Err(e) => Err(e) }
            }
            _ => Err(Sig::err("Attempted to call a non-function")),
        }
    }

    fn binop(&self, op: &BinOp, l: Value, r: Value) -> Result<Value, Sig> {
        use BinOp::*;
        let e = |s: &str| Err(Sig::err(s.to_string()));
        match op {
            Add => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(a + &b)),
                (Value::Str(a), b) => Ok(Value::Str(a + &b.to_string())),
                (Value::List(mut a), Value::List(b)) => { a.extend(b); Ok(Value::List(a)) }
                (Value::Map(mut a), Value::Map(b)) => { a.extend(b); Ok(Value::Map(a)) }
                (l, r) => Err(Sig::err(format!("Cannot add {} and {}", l.kind(), r.kind()))),
            },
            Sub  => self.num2(l, r, |a,b| a-b, |a,b| a-b, "subtract"),
            Mul  => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * b as f64)),
                (Value::Str(s), Value::Int(n)) => Ok(Value::Str(s.repeat(n.max(0) as usize))),
                (l, r) => Err(Sig::err(format!("Cannot multiply {} and {}", l.kind(), r.kind()))),
            },
            Div  => {
                match &r { Value::Int(0) => return e("Division by zero"), Value::Float(f) if *f == 0.0 => return e("Division by zero"), _ => {} }
                match (l, r) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Float(a as f64 / b as f64)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                    (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 / b)),
                    (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / b as f64)),
                    (l, r) => Err(Sig::err(format!("Cannot divide {} by {}", l.kind(), r.kind()))),
                }
            }
            IDiv => match (l, r) {
                (Value::Int(a), Value::Int(b)) => if b == 0 { e("Integer division by zero") } else { Ok(Value::Int(a / b)) },
                (Value::Float(a), Value::Float(b)) => Ok(Value::Int((a / b).floor() as i64)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Int((a as f64 / b).floor() as i64)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Int((a / b as f64).floor() as i64)),
                _ => e("Integer division requires numbers"),
            },
            Mod  => match (l, r) { (Value::Int(a), Value::Int(b)) => if b == 0 { e("Modulo by zero") } else { Ok(Value::Int(a % b)) }, _ => e("Modulo requires integers") },
            Pow  => {
                let base = l.as_f64().ok_or_else(|| Sig::err("** requires numbers"))?;
                let exp  = r.as_f64().ok_or_else(|| Sig::err("** requires numbers"))?;
                if exp >= 0.0 && exp.fract() == 0.0 { if let Value::Int(b) = &l { return Ok(Value::Int(b.pow(exp as u32))); } }
                Ok(Value::Float(base.powf(exp)))
            }
            Eq   => Ok(Value::Bool(self.eq_vals(&l, &r))),
            Ne   => Ok(Value::Bool(!self.eq_vals(&l, &r))),
            Lt   => self.cmp(l, r, |a,b| a < b),
            Le   => self.cmp(l, r, |a,b| a <= b),
            Gt   => self.cmp(l, r, |a,b| a > b),
            Ge   => self.cmp(l, r, |a,b| a >= b),
            And | Or | NullCoal => unreachable!(),
        }
    }

    fn num2(&self, l: Value, r: Value, fi: fn(i64,i64)->i64, ff: fn(f64,f64)->f64, op: &str) -> Result<Value, Sig> {
        match (l, r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(fi(a, b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(ff(a, b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(ff(a as f64, b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(ff(a, b as f64))),
            (l, r) => Err(Sig::err(format!("Cannot {} {} and {}", op, l.kind(), r.kind()))),
        }
    }
    fn cmp(&self, l: Value, r: Value, f: fn(f64,f64)->bool) -> Result<Value, Sig> {
        if let (Value::Str(a), Value::Str(b)) = (&l, &r) {
            let n: f64 = match a.as_str().cmp(b.as_str()) { std::cmp::Ordering::Less => -1.0, std::cmp::Ordering::Equal => 0.0, std::cmp::Ordering::Greater => 1.0 };
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
            (Value::List(x),  Value::List(y))  => x.len() == y.len() && x.iter().zip(y).all(|(a,b)| self.eq_vals(a,b)),
            (Value::Map(x),   Value::Map(y))   => x.len() == y.len() && x.iter().all(|(k,v)| y.get(k).map_or(false, |u| self.eq_vals(v,u))),
            _ => false,
        }
    }

    fn match_pattern(&mut self, pat: &MatchPat, val: &Value) -> Result<Option<Option<(String,Value)>>, Sig> {
        match pat {
            MatchPat::Wildcard    => Ok(Some(None)),
            MatchPat::Binding(n)  => Ok(Some(Some((n.clone(), val.clone())))),
            MatchPat::Literal(e)  => { let pv = self.eval(e)?; Ok(if self.eq_vals(&pv, val) { Some(None) } else { None }) }
            MatchPat::Range(s, e) => {
                let sv = self.eval(s)?; let ev = self.eval(e)?;
                let v  = val.as_f64().ok_or_else(|| Sig::err("match range requires a number"))?;
                let s2 = sv.as_f64().ok_or_else(|| Sig::err("match range start must be a number"))?;
                let e2 = ev.as_f64().ok_or_else(|| Sig::err("match range end must be a number"))?;
                Ok(if v >= s2 && v < e2 { Some(None) } else { None })
            }
        }
    }

    // ── Exec ──────────────────────────────────────────────────────
    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Value, Sig> {
        let mut last = Value::Null;
        for s in stmts { last = self.exec(s)?; }
        Ok(last)
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<Value, Sig> {
        match stmt {
            Stmt::Expr(e)  => self.eval(e),
            Stmt::Break    => Err(Sig::Brk),
            Stmt::Continue => Err(Sig::Cont),

            Stmt::Let { name, value } => { let v = self.eval(value)?; self.def(name, v); Ok(Value::Null) }

            Stmt::LetList { items, value } => {
                let val = self.eval(value)?;
                let list = match val { Value::List(l) => l, v => return Err(Sig::err(format!("List unpack requires a list, got {}", v.kind()))) };
                let mut idx = 0usize;
                for item in items {
                    match item {
                        UnpackItem::Name(n) => { self.def(n, list.get(idx).cloned().unwrap_or(Value::Null)); idx += 1; }
                        UnpackItem::Rest(n) => { self.def(n, Value::List(list.into_iter().skip(idx).collect())); return Ok(Value::Null); }
                    }
                }
                Ok(Value::Null)
            }

            Stmt::LetMap { names, value } => {
                let val = self.eval(value)?;
                let map = match val { Value::Map(m) => m, v => return Err(Sig::err(format!("Map unpack requires a map, got {}", v.kind()))) };
                for name in names { self.def(name, map.get(name).cloned().unwrap_or(Value::Null)); }
                Ok(Value::Null)
            }

            Stmt::CompoundAssign { target, op, value } => {
                let rhs = self.eval(value)?;
                match target {
                    Lhs::Ident(n) => {
                        let cur = self.get(n).ok_or_else(|| Sig::err(format!("line {}: Undefined variable '{}' — use 'let' first", self.current_line, n)))?;
                        let result = self.binop(op, cur, rhs)?; self.set(n, result);
                    }
                    Lhs::Index { obj, idx } => {
                        let iv = self.eval(idx)?;
                        let var = lhs_root_name(obj).ok_or_else(|| Sig::err("Complex compound index assignment not yet supported"))?;
                        let container = self.get(&var).ok_or_else(|| Sig::err(format!("Undefined variable '{}'", var)))?;
                        match container {
                            Value::List(mut l) => { let i = match iv { Value::Int(i) => i, _ => return Err(Sig::err("List index must be an integer")) }; let n = l.len() as i64; let i = if i<0{n+i}else{i} as usize; if i<l.len() { let cur=l[i].clone(); l[i]=self.binop(op,cur,rhs)?; self.set(&var,Value::List(l)); } else { return Err(Sig::err(format!("Index {} out of bounds",i))); } }
                            Value::Map(mut m) => { let k = iv.to_map_key().ok_or_else(|| Sig::err("Map key must be string/int/bool"))?; let cur = m.get(&k).cloned().unwrap_or(Value::Int(0)); m.insert(k, self.binop(op, cur, rhs)?); self.set(&var, Value::Map(m)); }
                            _ => return Err(Sig::err("Compound index assignment requires a list or map")),
                        }
                    }
                    Lhs::Field { obj, name: field } => {
                        let var = lhs_root_name(obj).ok_or_else(|| Sig::err("Complex compound field assignment not yet supported"))?;
                        let container = self.get(&var).ok_or_else(|| Sig::err(format!("Undefined variable '{}'", var)))?;
                        match container {
                            Value::Map(mut m) => { let cur = m.get(field).cloned().unwrap_or(Value::Int(0)); m.insert(field.clone(), self.binop(op, cur, rhs)?); self.set(&var, Value::Map(m)); }
                            _ => return Err(Sig::err("Compound field assignment requires a map")),
                        }
                    }
                }
                Ok(Value::Null)
            }

            Stmt::Assign { target, value } => {
                let v = self.eval(value)?;
                match target {
                    Lhs::Ident(n) => {
                        if self.get(n).is_none() { return Err(Sig::err(format!("line {}: Undefined variable '{}' — use 'let' first", self.current_line, n))); }
                        self.set(n, v);
                    }
                    Lhs::Index { obj, idx } => {
                        let iv = self.eval(idx)?;
                        let name = lhs_root_name(obj).ok_or_else(|| Sig::err("Complex index assignment not yet supported"))?;
                        let container = self.get(&name).ok_or_else(|| Sig::err(format!("Undefined variable '{}'", name)))?;
                        match container {
                            Value::List(mut l) => { let i = match iv { Value::Int(i) => i, _ => return Err(Sig::err("List index must be an integer")) }; let n = l.len() as i64; let i = if i<0{n+i}else{i} as usize; if i<l.len() { l[i]=v; self.set(&name,Value::List(l)); } else { return Err(Sig::err(format!("Index {} out of bounds",i))); } }
                            Value::Map(mut m) => { let k = iv.to_map_key().ok_or_else(|| Sig::err("Map key must be string/int/bool"))?; m.insert(k, v); self.set(&name, Value::Map(m)); }
                            _ => return Err(Sig::err("Index assignment requires a list or map")),
                        }
                    }
                    Lhs::Field { obj, name: field } => {
                        let var = lhs_root_name(obj).ok_or_else(|| Sig::err("Complex field assignment not yet supported"))?;
                        let container = self.get(&var).ok_or_else(|| Sig::err(format!("Undefined variable '{}'", var)))?;
                        match container {
                            Value::Map(mut m) => { m.insert(field.clone(), v); self.set(&var, Value::Map(m)); }
                            _ => return Err(Sig::err("Field assignment requires a map")),
                        }
                    }
                }
                Ok(Value::Null)
            }

            Stmt::Print(e) => { let v = self.eval(e)?; println!("{}", v); Ok(Value::Null) }
            Stmt::Write(e) => { let v = self.eval(e)?; print!("{}", v); io::stdout().flush().unwrap(); Ok(Value::Null) }

            Stmt::Return(e) => { let v = if let Some(ex) = e { self.eval(ex)? } else { Value::Null }; Err(Sig::Ret(v)) }
            Stmt::Throw(e)  => { let v = self.eval(e)?; Err(Sig::Throw(v)) }

            Stmt::While { cond, body } => {
                loop {
                    if !self.eval(cond)?.truthy() { break; }
                    self.push_scope(); let r = self.exec_block(body); self.pop_scope();
                    match r { Ok(_) => {}, Err(Sig::Brk) => break, Err(Sig::Cont) => continue, Err(e) => return Err(e) }
                }
                Ok(Value::Null)
            }

            Stmt::For { var, iter, body } => {
                let items = match self.eval(iter)? {
                    Value::List(l) => l,
                    Value::Str(s)  => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    Value::Map(m)  => { let mut ks: Vec<_> = m.keys().cloned().collect(); ks.sort(); ks.into_iter().map(Value::Str).collect() }
                    _ => return Err(Sig::err("'for' requires a list, string, or map")),
                };
                'fl: for item in items {
                    self.push_scope(); self.def(var, item);
                    let r = self.exec_block(body); self.pop_scope();
                    match r { Ok(_) => {}, Err(Sig::Brk) => break 'fl, Err(Sig::Cont) => continue, Err(e) => return Err(e) }
                }
                Ok(Value::Null)
            }

            Stmt::FnDef { name, params, variadic, body } => {
                let f = Value::Fn { fname: Some(name.clone()), params: params.clone(), variadic: variadic.clone(), body: body.clone(), closure: self.env.clone() };
                self.def(name, f); Ok(Value::Null)
            }

            Stmt::Match { subject, arms } => {
                let val = self.eval(subject)?;
                for arm in arms {
                    if let Some(binding) = self.match_pattern(&arm.pattern, &val)? {
                        self.push_scope();
                        if let Some((n, v)) = binding { self.def(&n, v); }
                        let r = self.exec_block(&arm.body); self.pop_scope();
                        return match r { Ok(v) => Ok(v), Err(Sig::Ret(v)) => Err(Sig::Ret(v)), Err(e) => Err(e) };
                    }
                }
                Ok(Value::Null)
            }

            Stmt::TryCatch { body, catch_var, catch_body } => {
                self.push_scope(); let r = self.exec_block(body); self.pop_scope();
                match r {
                    Ok(v) => Ok(v),
                    Err(Sig::Err(msg)) => {
                        self.push_scope(); self.def(catch_var, Value::Str(msg));
                        let r2 = self.exec_block(catch_body); self.pop_scope(); r2
                    }
                    Err(Sig::Throw(val)) => {
                        self.push_scope(); self.def(catch_var, val);
                        let r2 = self.exec_block(catch_body); self.pop_scope(); r2
                    }
                    Err(other) => Err(other), // Ret/Brk/Cont propagate normally
                }
            }

            Stmt::Import { path } => {
                let full = if let Some(ref base) = self.import_base {
                    base.join(path)
                } else { PathBuf::from(path) };
                let src = std::fs::read_to_string(&full)
                    .map_err(|e| Sig::err(format!("import '{}': {}", path, e)))?;
                let old_base = self.import_base.clone();
                self.import_base = full.parent().map(|p| p.to_path_buf());
                let result = self.run_src(&src);
                self.import_base = old_base;
                result.map_err(Sig::err)?;
                Ok(Value::Null)
            }
        }
    }

    fn run_src(&mut self, src: &str) -> Result<(), String> {
        let tokens = Lexer::new(src).tokenize()?;
        let stmts  = Parser::new(tokens).parse_program()?;
        for s in &stmts {
            match self.exec(s) {
                Ok(_)              => {}
                Err(Sig::Ret(_))   => return Err(format!("line {}: 'return' outside of a function", self.current_line)),
                Err(Sig::Brk)      => return Err(format!("line {}: 'break' outside of a loop", self.current_line)),
                Err(Sig::Cont)     => return Err(format!("line {}: 'continue' outside of a loop", self.current_line)),
                Err(Sig::Throw(v)) => return Err(format!("line {}: Uncaught exception: {}", self.current_line, v)),
                Err(Sig::Err(e))   => return Err(e),
            }
        }
        Ok(())
    }

    fn run(&mut self, src: &str) -> Result<(), String> { self.run_src(src) }
}

fn lhs_root_name(e: &Expr) -> Option<String> {
    match e { Expr::Ident(n) => Some(n.clone()), Expr::Index{obj,..}|Expr::Field{obj,..} => lhs_root_name(obj), _ => None }
}

// ================================================================
// SECTION 6 – ENTRY POINT
// ================================================================
fn run_main() {
    let args: Vec<String> = std::env::args().collect();
    let mut vm = Interp::new();

    if args.len() > 1 {
        let path = &args[1];
        let src = std::fs::read_to_string(path)
            .unwrap_or_else(|e| { eprintln!("ferrite: {}", e); std::process::exit(1); });
        vm.import_base = std::path::Path::new(path).parent().map(|p| p.to_path_buf());
        if let Err(e) = vm.run(&src) {
            eprintln!("\x1b[31mError:\x1b[0m {}", e);
            std::process::exit(1);
        }
    } else {
        println!("\x1b[36m╔══════════════════════════════════════╗");
        println!("║   Ferrite v1.3  —  built in Rust     ║");
        println!("║   Type 'exit' or Ctrl+D to quit       ║");
        println!("╚══════════════════════════════════════╝\x1b[0m");

        // Multi-line REPL: buffer lines until braces are balanced
        let mut buffer = String::new();
        let mut depth: i32 = 0;
        loop {
            let prompt = if depth > 0 { "\x1b[90m... \x1b[0m" } else { "\x1b[33m»   \x1b[0m" };
            print!("{}", prompt); io::stdout().flush().unwrap();
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(0) | Err(_) => { println!(); break; }
                _ => {}
            }
            let trimmed = line.trim();
            if depth == 0 && (trimmed == "exit" || trimmed == "quit") { break; }
            if trimmed.is_empty() && depth == 0 { continue; }
            for c in line.chars() {
                match c { '{' | '(' | '[' => depth += 1, '}' | ')' | ']' => depth -= 1, _ => {} }
            }
            buffer.push_str(&line);
            if depth <= 0 {
                depth = 0;
                let src = buffer.trim().to_string();
                buffer.clear();
                if src.is_empty() { continue; }
                if let Err(e) = vm.run(&src) {
                    eprintln!("\x1b[31m  Error: {}\x1b[0m", e);
                }
            }
        }
        println!("Goodbye! 🦀");
    }
}

fn main() {
    let builder = std::thread::Builder::new()
        .name("ferrite-main".into())
        .stack_size(64 * 1024 * 1024);
    let handler = builder.spawn(run_main).expect("failed to spawn interpreter thread");
    handler.join().expect("interpreter thread panicked");
}
