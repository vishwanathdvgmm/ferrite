pub mod token;

use crate::errors::{DiagnosticBag, Span};
use std::path::PathBuf;
pub use token::{lookup_keyword, Token, TokenKind};

// ── Lexer ────────────────────────────────────────────────────────

pub struct Lexer {
    source: Vec<char>,
    file: PathBuf,
    pos: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    pub fn new(source: &str, file: PathBuf) -> Self {
        Self {
            source: source.chars().collect(),
            file,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Tokenize the entire source, returning a Vec of tokens.
    /// Errors are reported into the DiagnosticBag.
    pub fn tokenize(&mut self, diag: &mut DiagnosticBag) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.at_end() {
                tokens.push(Token::new(TokenKind::EOF, self.span(1)));
                break;
            }

            match self.scan_token(diag) {
                Some(tok) => tokens.push(tok),
                None => {
                    // Error already reported; skip the bad character
                    self.advance();
                }
            }
        }

        tokens
    }

    // ── Core Helpers ─────────────────────────────────────────

    fn at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek(&self) -> char {
        if self.at_end() {
            '\0'
        } else {
            self.source[self.pos]
        }
    }

    fn peek_next(&self) -> char {
        if self.pos + 1 >= self.source.len() {
            '\0'
        } else {
            self.source[self.pos + 1]
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.peek();
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        ch
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn span(&self, len: u32) -> Span {
        Span::new(
            self.file.clone(),
            self.line,
            self.col.saturating_sub(len),
            len,
        )
    }

    fn span_from(&self, start_line: u32, start_col: u32, len: u32) -> Span {
        Span::new(self.file.clone(), start_line, start_col, len)
    }

    // ── Whitespace & Comments ────────────────────────────────

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while !self.at_end() && self.peek().is_whitespace() {
                self.advance();
            }

            // Skip line comments: // ...
            if self.peek() == '/' && self.peek_next() == '/' {
                while !self.at_end() && self.peek() != '\n' {
                    self.advance();
                }
                continue;
            }

            break;
        }
    }

    // ── Token Scanner ────────────────────────────────────────

    fn scan_token(&mut self, diag: &mut DiagnosticBag) -> Option<Token> {
        let start_line = self.line;
        let start_col = self.col;
        let ch = self.advance();

        let kind = match ch {
            // ── Single-character delimiters ───────────────
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '.' => TokenKind::Dot,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,

            // ── Multi-character operators ─────────────────
            '-' => {
                if self.match_char('>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '=' => {
                if self.match_char('>') {
                    TokenKind::FatArrow
                } else if self.match_char('=') {
                    TokenKind::EqEq
                } else {
                    TokenKind::Eq
                }
            }
            '!' => {
                if self.match_char('=') {
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                if self.match_char('=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.match_char('=') {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                if self.match_char('&') {
                    TokenKind::And
                } else {
                    diag.error(
                        self.span_from(start_line, start_col, 1),
                        "Unexpected character '&'. Did you mean '&&'?",
                    );
                    return None;
                }
            }
            '|' => {
                if self.match_char('|') {
                    TokenKind::Or
                } else {
                    diag.error(
                        self.span_from(start_line, start_col, 1),
                        "Unexpected character '|'. Did you mean '||'?",
                    );
                    return None;
                }
            }

            // ── String literals ──────────────────────────
            '"' => return Some(self.scan_string(start_line, start_col, diag)),

            // ── Number literals ──────────────────────────
            c if c.is_ascii_digit() => {
                return Some(self.scan_number(c, start_line, start_col));
            }

            // ── Identifiers & Keywords ───────────────────
            c if c.is_alphabetic() || c == '_' => {
                return Some(self.scan_identifier(c, start_line, start_col));
            }

            // ── Unknown character ────────────────────────
            other => {
                diag.error(
                    self.span_from(start_line, start_col, 1),
                    format!("Unexpected character '{}'", other),
                );
                return None;
            }
        };

        let len = (self.col - start_col).max(1);
        Some(Token::new(kind, self.span_from(start_line, start_col, len)))
    }

    // ── String Scanner ───────────────────────────────────────

    fn scan_string(&mut self, start_line: u32, start_col: u32, diag: &mut DiagnosticBag) -> Token {
        let mut value = String::new();

        while !self.at_end() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance();
                match self.peek() {
                    'n' => {
                        value.push('\n');
                        self.advance();
                    }
                    't' => {
                        value.push('\t');
                        self.advance();
                    }
                    'r' => {
                        value.push('\r');
                        self.advance();
                    }
                    '\\' => {
                        value.push('\\');
                        self.advance();
                    }
                    '"' => {
                        value.push('"');
                        self.advance();
                    }
                    other => {
                        diag.warning(
                            self.span_from(self.line, self.col, 1),
                            format!("Unknown escape sequence '\\{}'", other),
                        );
                        value.push(other);
                        self.advance();
                    }
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.at_end() {
            diag.error(
                self.span_from(start_line, start_col, 1),
                "Unterminated string literal",
            );
        } else {
            self.advance(); // consume closing "
        }

        let len = (self.col - start_col).max(1);
        Token::new(
            TokenKind::StringLit(value),
            self.span_from(start_line, start_col, len),
        )
    }

    // ── Number Scanner ───────────────────────────────────────

    fn scan_number(&mut self, first: char, start_line: u32, start_col: u32) -> Token {
        let mut text = String::new();
        text.push(first);
        let mut is_float = false;

        while !self.at_end() && self.peek().is_ascii_digit() {
            text.push(self.advance());
        }

        // Check for decimal point
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            is_float = true;
            text.push(self.advance()); // consume '.'
            while !self.at_end() && self.peek().is_ascii_digit() {
                text.push(self.advance());
            }
        }

        let len = text.len() as u32;
        let span = self.span_from(start_line, start_col, len);

        if is_float {
            let val: f64 = text.parse().unwrap_or(0.0);
            Token::new(TokenKind::FloatLit(val), span)
        } else {
            let val: i64 = text.parse().unwrap_or(0);
            Token::new(TokenKind::IntLit(val), span)
        }
    }

    // ── Identifier / Keyword Scanner ─────────────────────────

    fn scan_identifier(&mut self, first: char, start_line: u32, start_col: u32) -> Token {
        let mut text = String::new();
        text.push(first);

        while !self.at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            text.push(self.advance());
        }

        let len = text.len() as u32;
        let span = self.span_from(start_line, start_col, len);

        let kind = lookup_keyword(&text).unwrap_or(TokenKind::Ident(text));
        Token::new(kind, span)
    }
}
