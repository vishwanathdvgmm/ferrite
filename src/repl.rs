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
    let mut saved_type_env_state: Option<(
        Vec<HashMap<String, crate::types::Type>>,
        HashMap<String, crate::types::Type>,
        HashMap<String, crate::types::TraitDef>,
        Vec<crate::types::ImplDef>,
        HashMap<String, Vec<(String, crate::types::Type)>>,
        HashMap<String, Vec<(String, Vec<crate::types::Type>)>>,
    )> = None;

    let mut input = String::new();
    let mut prompt = "ferrite> ";

    loop {
        match rl.readline(prompt) {
            Ok(line) => {
                input.push_str(&line);
                input.push('\n');

                if input.trim().is_empty() {
                    input.clear();
                    prompt = "ferrite> ";
                    continue;
                }
                if input.trim() == "exit" || input.trim() == "quit" {
                    break;
                }

                let mut diag = DiagnosticBag::new();
                let mut lexer =
                    crate::lexer::Lexer::new(&input, std::path::PathBuf::from("<repl>"));
                let tokens = lexer.tokenize(&mut diag);
                let eof_span = tokens.last().map(|t| t.span.clone());

                let mut parser = Parser::new(tokens, &mut diag);
                let program = parser.parse_program();

                if diag.has_errors() {
                    let is_eof_error = if let Some(eof) = &eof_span {
                        diag.has_error_at(eof)
                    } else {
                        false
                    };

                    if is_eof_error {
                        prompt = "... > ";
                        continue; // Wait for more input
                    } else {
                        diag.emit_all();
                        input.clear();
                        prompt = "ferrite> ";
                        continue;
                    }
                }

                let _ = rl.add_history_entry(input.trim());

                let mut type_env = TypeEnv::new(&mut diag);
                if let Some(state) = &saved_type_env_state {
                    type_env.scopes = state.0.clone();
                    type_env.types = state.1.clone();
                    type_env.traits = state.2.clone();
                    type_env.impls = state.3.clone();
                    type_env.group_fields = state.4.clone();
                    type_env.enum_variants = state.5.clone();
                }

                let mut semantic = SemanticAnalyzer::new(&mut type_env, module_exports.clone());
                semantic.analyze_program(&program);

                if type_env.diag.has_errors() {
                    type_env.diag.emit_all();
                    input.clear();
                    prompt = "ferrite> ";
                    continue;
                }

                saved_type_env_state = Some((
                    type_env.scopes.clone(),
                    type_env.types.clone(),
                    type_env.traits.clone(),
                    type_env.impls.clone(),
                    type_env.group_fields.clone(),
                    type_env.enum_variants.clone(),
                ));

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

                // Reset for next command
                input.clear();
                prompt = "ferrite> ";
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
