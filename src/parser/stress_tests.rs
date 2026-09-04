#[cfg(test)]
mod tests {
    use crate::errors::DiagnosticBag;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::panic;
    use std::path::PathBuf;

    /// Helper to parse a source string and ensure it does not panic.
    fn assert_no_panic(source: &str) {
        let result = panic::catch_unwind(|| {
            let mut diag = DiagnosticBag::new();
            let mut lexer = Lexer::new(source, PathBuf::from("<test>"));
            let tokens = lexer.tokenize(&mut diag);

            let mut parser = Parser::new(tokens, &mut diag);
            let _program = parser.parse_program();
            // We don't care if there are errors in `diag`, just that it doesn't panic.
        });

        assert!(result.is_ok(), "Parser panicked on input: {}", source);
    }

    #[test]
    fn test_unmatched_braces() {
        assert_no_panic("fun f() { { { }");
    }

    #[test]
    fn test_unmatched_parens() {
        assert_no_panic("fun f(a: int, b: int { }");
    }

    #[test]
    fn test_missing_semicolons() {
        assert_no_panic("keep x: int = 5 keep y: int = 10");
    }

    #[test]
    fn test_random_tokens() {
        assert_no_panic("+++ --- *** ;;; ::: >>>");
    }

    #[test]
    fn test_deeply_nested() {
        let mut source = String::new();
        for _ in 0..25 {
            source.push_str("if true { ");
        }
        source.push_str("}");
        assert_no_panic(&source);
    }

    #[test]
    fn test_empty_constructs() {
        assert_no_panic("fun() {}");
        assert_no_panic("group {}");
        assert_no_panic("enum {}");
        assert_no_panic("match {} {}");
    }

    #[test]
    fn test_very_long_identifier() {
        let mut source = String::from("keep ");
        for _ in 0..10000 {
            source.push('a');
        }
        source.push_str(": int = 5;");
        assert_no_panic(&source);
    }

    #[test]
    fn test_huge_number_literal() {
        assert_no_panic("keep x: int = 99999999999999999999999999999999;");
    }

    #[test]
    fn test_binary_garbage() {
        assert_no_panic("\x00\u{FF}\u{FE}");
    }
}
