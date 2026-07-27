use crate::ast::TopDecl;
use crate::errors::DiagnosticBag;
use crate::parser::Parser;
use crate::runtime::interpreter::Interpreter;
use crate::semantic::SemanticAnalyzer;
use crate::types::TypeEnv;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::collections::HashMap;

pub fn start_repl() {
    println!("Ferrite v{} Interactive REPL", env!("CARGO_PKG_VERSION"));
    println!("Type 'exit' or press Ctrl-C to quit.\n");

    let mut rl = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(e) => {
            eprintln!("Failed to initialize REPL: {}", e);
            return;
        }
    };

    let module_exports: HashMap<String, Vec<TopDecl>> = HashMap::new();
    let mut interpreter = Interpreter::new(module_exports.clone());
    // We also need to maintain the type environment for the semantic analyzer across lines
    // For a simple REPL, we'll just instantiate a new one each time for now, but
    // ideally it should be persistent if we want full type tracking across inputs.

    loop {
        let readline = rl.readline("ferrite> ");
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                if input == "exit" || input == "quit" {
                    break;
                }
                let _ = rl.add_history_entry(input);

                let mut diag = DiagnosticBag::new();
                let mut lexer = crate::lexer::Lexer::new(input, std::path::PathBuf::from("<repl>"));
                let tokens = lexer.tokenize(&mut diag);
                let mut parser = Parser::new(tokens, &mut diag);
                let program = parser.parse_program();

                if diag.has_errors() {
                    diag.emit_all();
                    continue;
                }

                let mut type_env = TypeEnv::new(&mut diag);
                let mut semantic = SemanticAnalyzer::new(&mut type_env, module_exports.clone());
                semantic.analyze_program(&program);

                if diag.has_errors() {
                    diag.emit_all();
                    continue;
                }

                match interpreter.run_program(&program) {
                    Ok(val) => {
                        if val != crate::runtime::value::Value::Unit {
                            println!("{}", val);
                        }
                    }
                    Err(e) => {
                        eprintln!("Runtime Error: {}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("REPL Error: {:?}", err);
                break;
            }
        }
    }
}
