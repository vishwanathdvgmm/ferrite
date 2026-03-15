// ================================================================
//  Ferrite v1.4 — Standard Library & Module System
//  v1.3: try/catch/throw, file I/O, f-strings, variadic fns,
//        list/map unpack, line numbers, import, enumerate/zip,
//        ?? operator, mutable closures, multi-line REPL
//  v1.4: native stdlib embedding, module path resolution
// Lexer only relies on standard datatypes
// ================================================================

// ================================================================
// F-STRING PARTS
// ================================================================
#[derive(Debug, Clone, PartialEq)]
pub enum FsPart {
    Text(String),
    Code(String),
}

// ================================================================
// SECTION 1 – LEXER
// ================================================================
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    FStr(Vec<FsPart>),
    Ident(String),
    // keywords
    Let,
    Fn,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Print,
    Break,
    Continue,
    Match,
    Try,
    Catch,
    Throw,
    Import,
    // operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StarStar,
    SlashSlash,
    Eq,
    EqEq,
    BangEq,
    Bang,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    // compound assign
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    // punctuation
    Arrow,
    QuestionQuestion,
    DotDot,
    DotDotDot,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Dot,
    Colon,
    EOF,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: u32,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Lexer {
            input: src.chars().collect(),
            pos: 0,
            line: 1,
        }
    }
    pub fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }
    pub fn peek2(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }
    pub fn advance(&mut self) -> Option<char> {
        let c = self.input.get(self.pos).copied();
        if c == Some('\n') {
            self.line += 1;
        }
        self.pos += 1;
        c
    }

    pub fn skip_ws(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.advance();
            }
            // # line comments (user's choice)
            if self.peek() == Some('#') {
                while matches!(self.peek(), Some(c) if c != '\n') {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    pub fn read_str_content(&mut self) -> Result<String, String> {
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    Some(c) => {
                        s.push('\\');
                        s.push(c);
                    }
                    None => return Err("Unterminated escape".into()),
                },
                Some(c) => s.push(c),
                None => return Err("Unterminated string".into()),
            }
        }
        Ok(s)
    }

    pub fn read_fstr(&mut self) -> Result<Token, String> {
        // 'f' already consumed; consume opening "
        self.advance();
        let mut parts = Vec::new();
        let mut text = String::new();
        loop {
            match self.advance() {
                Some('"') => break,
                Some('{') => {
                    if !text.is_empty() {
                        parts.push(FsPart::Text(text.clone()));
                        text.clear();
                    }
                    let mut code = String::new();
                    let mut depth = 1i32;
                    loop {
                        match self.advance() {
                            Some('{') => {
                                depth += 1;
                                code.push('{');
                            }
                            Some('}') => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                code.push('}');
                            }
                            Some(c) => code.push(c),
                            None => return Err("Unterminated f-string expression".into()),
                        }
                    }
                    parts.push(FsPart::Code(code));
                }
                Some('\\') => match self.advance() {
                    Some('n') => text.push('\n'),
                    Some('t') => text.push('\t'),
                    Some('\\') => text.push('\\'),
                    Some('"') => text.push('"'),
                    Some(c) => {
                        text.push('\\');
                        text.push(c);
                    }
                    None => return Err("Unterminated f-string escape".into()),
                },
                Some(c) => text.push(c),
                None => return Err("Unterminated f-string".into()),
            }
        }
        if !text.is_empty() {
            parts.push(FsPart::Text(text));
        }
        Ok(Token::FStr(parts))
    }

    pub fn read_num(&mut self) -> Token {
        let mut s = String::new();
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            s.push(self.advance().unwrap());
        }
        if self.peek() == Some('.') && matches!(self.peek2(), Some(c) if c.is_ascii_digit()) {
            s.push(self.advance().unwrap());
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                s.push(self.advance().unwrap());
            }
            Token::Float(s.parse().unwrap())
        } else {
            Token::Int(s.parse().unwrap())
        }
    }

    pub fn read_ident(&mut self) -> Token {
        let mut s = String::new();
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            s.push(self.advance().unwrap());
        }
        match s.as_str() {
            "let" => Token::Let,
            "fn" => Token::Fn,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "in" => Token::In,
            "return" => Token::Return,
            "print" => Token::Print,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "match" => Token::Match,
            "try" => Token::Try,
            "catch" => Token::Catch,
            "throw" => Token::Throw,
            "import" => Token::Import,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            "null" => Token::Null,
            _ => Token::Ident(s),
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<(Token, u32)>, String> {
        let mut toks = Vec::new();
        loop {
            self.skip_ws();
            let line = self.line;
            let tok = match self.peek() {
                None => {
                    toks.push((Token::EOF, line));
                    break;
                }
                Some(c) => match c {
                    '"' => {
                        self.advance();
                        Token::Str(self.read_str_content()?)
                    }
                    'f' if self.peek2() == Some('"') => {
                        self.advance();
                        self.read_fstr()?
                    }
                    '0'..='9' => self.read_num(),
                    'a'..='z' | 'A'..='Z' | '_' => self.read_ident(),
                    '+' => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            Token::PlusEq
                        } else {
                            Token::Plus
                        }
                    }
                    '-' => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            Token::MinusEq
                        } else {
                            Token::Minus
                        }
                    }
                    '*' => {
                        self.advance();
                        if self.peek() == Some('*') {
                            self.advance();
                            Token::StarStar
                        } else if self.peek() == Some('=') {
                            self.advance();
                            Token::StarEq
                        } else {
                            Token::Star
                        }
                    }
                    '/' => {
                        self.advance();
                        if self.peek() == Some('/') {
                            self.advance();
                            Token::SlashSlash
                        } else if self.peek() == Some('=') {
                            self.advance();
                            Token::SlashEq
                        } else {
                            Token::Slash
                        }
                    }
                    '%' => {
                        self.advance();
                        Token::Percent
                    }
                    '(' => {
                        self.advance();
                        Token::LParen
                    }
                    ')' => {
                        self.advance();
                        Token::RParen
                    }
                    '{' => {
                        self.advance();
                        Token::LBrace
                    }
                    '}' => {
                        self.advance();
                        Token::RBrace
                    }
                    '[' => {
                        self.advance();
                        Token::LBracket
                    }
                    ']' => {
                        self.advance();
                        Token::RBracket
                    }
                    ',' => {
                        self.advance();
                        Token::Comma
                    }
                    ';' => {
                        self.advance();
                        Token::Semicolon
                    }
                    ':' => {
                        self.advance();
                        Token::Colon
                    }
                    '.' => {
                        self.advance();
                        if self.peek() == Some('.') {
                            self.advance();
                            if self.peek() == Some('.') {
                                self.advance();
                                Token::DotDotDot
                            } else {
                                Token::DotDot
                            }
                        } else {
                            Token::Dot
                        }
                    }
                    '=' => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            Token::EqEq
                        } else if self.peek() == Some('>') {
                            self.advance();
                            Token::Arrow
                        } else {
                            Token::Eq
                        }
                    }
                    '!' => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            Token::BangEq
                        } else {
                            Token::Bang
                        }
                    }
                    '<' => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            Token::LtEq
                        } else {
                            Token::Lt
                        }
                    }
                    '>' => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            Token::GtEq
                        } else {
                            Token::Gt
                        }
                    }
                    '&' => {
                        self.advance();
                        if self.peek() == Some('&') {
                            self.advance();
                            Token::And
                        } else {
                            return Err(format!("line {}: Expected '&&'", self.line));
                        }
                    }
                    '|' => {
                        self.advance();
                        if self.peek() == Some('|') {
                            self.advance();
                            Token::Or
                        } else {
                            return Err(format!("line {}: Expected '||'", self.line));
                        }
                    }
                    '?' => {
                        self.advance();
                        if self.peek() == Some('?') {
                            self.advance();
                            Token::QuestionQuestion
                        } else {
                            return Err(format!("line {}: Expected '??'", self.line));
                        }
                    }
                    c => return Err(format!("line {}: Unexpected character '{}'", self.line, c)),
                },
            };
            toks.push((tok, line));
        }
        Ok(toks)
    }
}

// ================================================================
