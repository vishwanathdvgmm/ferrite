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
    pub comments: Vec<(u32, String)>,
}

impl Lexer {
    pub fn new(source: &str, file: PathBuf) -> Self {
        Self {
            source: source.chars().collect(),
            file,
            pos: 0,
            line: 1,
            col: 1,
            comments: Vec::new(),
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
                let comment_line = self.line;
                let mut comment_text = String::new();
                while !self.at_end() && self.peek() != '\n' {
                    comment_text.push(self.peek());
                    self.advance();
                }
                self.comments.push((comment_line, comment_text));
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

// ── Unit Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Helper: tokenize source and return a Vec of (TokenKind, line, col) for easy snapshot comparison.
    fn tokenize_to_strings(source: &str) -> (Vec<String>, Vec<String>) {
        let mut diag = DiagnosticBag::new();
        let mut lexer = Lexer::new(source, PathBuf::from("<test>"));
        let tokens = lexer.tokenize(&mut diag);

        let token_strs: Vec<String> = tokens
            .iter()
            .map(|t| format!("{:?} @ {}:{}", t.kind, t.span.line, t.span.col))
            .collect();

        let error_strs: Vec<String> = if diag.has_errors() {
            // Collect error messages (without ANSI rendering)
            vec![format!("{} error(s) reported", diag.error_count())]
        } else {
            vec![]
        };

        (token_strs, error_strs)
    }

    #[test]
    fn test_empty_input() {
        let (tokens, errors) = tokenize_to_strings("");
        insta::assert_debug_snapshot!("empty_input_tokens", tokens);
        assert!(errors.is_empty(), "Empty input should produce no errors");
    }

    #[test]
    fn test_single_char_delimiters() {
        let (tokens, errors) = tokenize_to_strings("( ) { } [ ] , : ; .");
        insta::assert_debug_snapshot!("single_char_delimiters", tokens);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_arithmetic_operators() {
        let (tokens, _) = tokenize_to_strings("+ - * / %");
        insta::assert_debug_snapshot!("arithmetic_operators", tokens);
    }

    #[test]
    fn test_comparison_operators() {
        let (tokens, _) = tokenize_to_strings("< <= > >= == != = !");
        insta::assert_debug_snapshot!("comparison_operators", tokens);
    }

    #[test]
    fn test_logical_operators() {
        let (tokens, _) = tokenize_to_strings("&& ||");
        insta::assert_debug_snapshot!("logical_operators", tokens);
    }

    #[test]
    fn test_arrow_operators() {
        let (tokens, _) = tokenize_to_strings("-> =>");
        insta::assert_debug_snapshot!("arrow_operators", tokens);
    }

    #[test]
    fn test_integer_literals() {
        let (tokens, _) = tokenize_to_strings("0 42 1024 999999");
        insta::assert_debug_snapshot!("integer_literals", tokens);
    }

    #[test]
    fn test_float_literals() {
        let (tokens, _) = tokenize_to_strings("3.14 0.5 100.0");
        insta::assert_debug_snapshot!("float_literals", tokens);
    }

    #[test]
    fn test_string_literal_simple() {
        let (tokens, _) = tokenize_to_strings(r#""hello world""#);
        insta::assert_debug_snapshot!("string_literal_simple", tokens);
    }

    #[test]
    fn test_string_literal_escapes() {
        let (tokens, _) = tokenize_to_strings(r#""line\nnew\ttab\\slash\"quote""#);
        insta::assert_debug_snapshot!("string_literal_escapes", tokens);
    }

    #[test]
    fn test_string_literal_unterminated() {
        let (tokens, errors) = tokenize_to_strings(r#""hello"#);
        insta::assert_debug_snapshot!("string_unterminated_tokens", tokens);
        assert!(
            !errors.is_empty(),
            "Unterminated string should produce an error"
        );
    }

    #[test]
    fn test_keywords() {
        let source = "fun keep param constant group enum import from take as if elif else while for in return stop skip match case default infer train async await spawn select where self trait impl pub extern unsafe test true false";
        let (tokens, _) = tokenize_to_strings(source);
        insta::assert_debug_snapshot!("all_keywords", tokens);
    }

    #[test]
    fn test_identifiers() {
        let (tokens, _) = tokenize_to_strings("foo _bar baz_42 MyType");
        insta::assert_debug_snapshot!("identifiers", tokens);
    }

    #[test]
    fn test_mixed_ferrite_code() {
        let source = r#"fun main() -> int {
    keep x: int = 42;
    return x;
}"#;
        let (tokens, _) = tokenize_to_strings(source);
        insta::assert_debug_snapshot!("mixed_ferrite_code", tokens);
    }

    #[test]
    fn test_unknown_character() {
        let (_, errors) = tokenize_to_strings("@");
        assert!(
            !errors.is_empty(),
            "Unknown character should produce an error"
        );
    }

    #[test]
    fn test_single_ampersand_error() {
        let (_, errors) = tokenize_to_strings("&");
        assert!(!errors.is_empty(), "Single '&' should produce an error");
    }

    #[test]
    fn test_single_pipe_error() {
        let (_, errors) = tokenize_to_strings("|");
        assert!(!errors.is_empty(), "Single '|' should produce an error");
    }

    #[test]
    fn test_comments_skipped() {
        let source = "keep x: int = 5; // this is a comment\nkeep y: int = 10;";
        let (tokens, _) = tokenize_to_strings(source);
        insta::assert_debug_snapshot!("comments_skipped", tokens);
    }

    #[test]
    fn test_multiline_spans() {
        let source = "keep\nx\n:\nint\n=\n5\n;";
        let (tokens, _) = tokenize_to_strings(source);
        insta::assert_debug_snapshot!("multiline_spans", tokens);
    }
}
