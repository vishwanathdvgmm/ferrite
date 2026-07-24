pub mod ast;
#[cfg(feature = "llvm")]
pub mod codegen;
pub mod errors;
pub mod fmt;
pub mod imports;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod pkg;
pub mod repl;
pub mod runtime;
pub mod semantic;
pub mod stdlib;
pub mod types;

use std::collections::HashMap;
use std::path::PathBuf;

use ast::{ImportDecl, TopDecl, Visibility};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let version_str = "Ferrite v3.0.0-dev Compiler (AOT ML Language)";
    let usage_str = "Usage:
  ferrite run     <file.fe>   # Execute via Tree-Walk Interpreter
  ferrite check   <file.fe>   # Parse and Type-check only
  ferrite compile <file.fe>   # Compile to native LLVM IR / Object
  ferrite init                # Initialize a Ferrite project in the current directory
  ferrite new     <name>      # Create a new Ferrite project
  ferrite build               # Build the Ferrite project in the current directory
  ferrite clean               # Clean the Ferrite project target directory
  ferrite add     <pkg>       # Add a dependency to ferrite.toml
  ferrite remove  <pkg>       # Remove a dependency from ferrite.toml
  ferrite fmt     <file>      # Format a Ferrite source file in-place
  ferrite repl                # Start the interactive Read-Eval-Print Loop
  ferrite lsp                 # Start the Language Server Protocol process
  ferrite --version           # Print compiler version
  ferrite --help              # Print this help message";

    if args.len() == 1 {
        // If no arguments, launch REPL by default for better DevX
        repl::start_repl();
        return;
    }

    if args.len() == 2 {
        let arg = &args[1];
        if arg == "--version" || arg == "-V" || arg == "-v" {
            println!("{}", version_str);
            return;
        } else if arg == "--help" || arg == "-h" {
            println!("{}\n\n{}", version_str, usage_str);
            return;
        } else if arg == "init" {
            // `ferrite init` — initialize project in current directory
            let cwd = std::env::current_dir().unwrap_or_else(|e| {
                eprintln!("Error: Cannot determine current directory: {}", e);
                std::process::exit(1);
            });
            match pkg::scaffold::init_project(&cwd) {
                Ok(()) => {
                    println!("✅ Initialized Ferrite project in '{}'", cwd.display());
                    println!("   Created: ferrite.toml");
                    println!("   Created: src/main.fe");
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            return;
        } else if arg == "build" {
            let cwd = std::env::current_dir().unwrap_or_else(|e| {
                eprintln!("Error: Cannot determine current directory: {}", e);
                std::process::exit(1);
            });
            if let Err(e) = pkg::build::build_project(&cwd) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            return;
        } else if arg == "clean" {
            let cwd = std::env::current_dir().unwrap_or_else(|e| {
                eprintln!("Error: Cannot determine current directory: {}", e);
                std::process::exit(1);
            });
            if let Err(e) = pkg::build::clean_project(&cwd) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            return;
        } else if arg == "repl" {
            repl::start_repl();
            return;
        } else if arg == "lsp" {
            lsp::run_lsp_server();
            return;
        }
    }

    // Commands that take a single argument (not a .fe file)
    if args.len() == 3 && args[1] == "new" {
        let name = &args[2];
        let cwd = std::env::current_dir().unwrap_or_else(|e| {
            eprintln!("Error: Cannot determine current directory: {}", e);
            std::process::exit(1);
        });
        match pkg::scaffold::new_project(&cwd, name) {
            Ok(()) => {
                println!("✅ Created new Ferrite project '{}'", name);
                println!("   Created: {}/ferrite.toml", name);
                println!("   Created: {}/src/main.fe", name);
                println!("   Created: {}/tests/test_main.fe", name);
                println!("\n   Get started:");
                println!("     cd {}", name);
                println!("     ferrite run src/main.fe");
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if args.len() == 3 {
        let cmd = &args[1];
        let arg2 = &args[2];
        let cwd = std::env::current_dir().unwrap_or_else(|e| {
            eprintln!("Error: Cannot determine current directory: {}", e);
            std::process::exit(1);
        });

        if cmd == "add" {
            if let Err(e) = pkg::deps::add_dependency(&cwd, arg2, "*") {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            println!("✅ Added dependency '{}' to ferrite.toml", arg2);
            return;
        } else if cmd == "remove" {
            if let Err(e) = pkg::deps::remove_dependency(&cwd, arg2) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            println!("✅ Removed dependency '{}' from ferrite.toml", arg2);
            return;
        } else if cmd == "fmt" {
            let path = PathBuf::from(arg2);
            if !path.exists() {
                eprintln!("Error: File not found: {}", arg2);
                std::process::exit(1);
            }
            let source = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };
            let mut diag = errors::DiagnosticBag::new();
            let mut lexer = lexer::Lexer::new(&source, path.clone());
            let tokens = lexer.tokenize(&mut diag);
            if diag.has_errors() {
                diag.emit_all();
                std::process::exit(1);
            }
            let mut parser = parser::Parser::new(tokens, &mut diag);
            let program = parser.parse_program();
            if diag.has_errors() {
                diag.emit_all();
                std::process::exit(1);
            }
            let mut formatter = fmt::Formatter::new(lexer.comments);
            let formatted = formatter.format_program(&program);
            if let Err(e) = std::fs::write(&path, formatted) {
                eprintln!("Error writing formatted file: {}", e);
                std::process::exit(1);
            }
            println!("Formatted {}", arg2);
            return;
        }
    }

    if args.len() < 3 {
        println!("{}\n\n{}", version_str, usage_str);
        return;
    }

    let cmd = &args[1];
    let path = PathBuf::from(&args[2]);

    let mut diag = errors::DiagnosticBag::new();
    let mut resolver = imports::ImportResolver::new(&mut diag);

    if let Some(entry_path) = resolver.resolve_entry(&path) {
        let modules = resolver.into_modules();
        let entry_module = &modules[&entry_path];

        // ── Module-Aware AST Merge ─────────────────────────────────
        // Instead of blindly flattening all module ASTs, we build a
        // module export map and only inject symbols that are explicitly
        // imported and publicly visible.

        // Step 1: Build an export map for each module (module_name -> pub decls)
        let mut module_exports: HashMap<String, Vec<TopDecl>> = HashMap::new();
        for (_mod_path, module) in &modules {
            let pub_decls: Vec<TopDecl> = module
                .ast
                .decls
                .iter()
                .filter(|d| match d {
                    TopDecl::Func(f) => f.visibility == Visibility::Public,
                    TopDecl::Constant(c) => c.visibility == Visibility::Public,
                    TopDecl::Group(g) => g.visibility == Visibility::Public,
                    TopDecl::Enum(e) => e.visibility == Visibility::Public,
                    TopDecl::Trait(t) => t.visibility == Visibility::Public,
                    TopDecl::Impl(_) => true, // impl blocks are always public
                    TopDecl::Import(_) => false, // don't re-export imports
                })
                .cloned()
                .collect();
            module_exports.insert(module.name.clone(), pub_decls);
        }

        // Step 2: Start with the entry module's own AST
        let mut merged_ast = entry_module.ast.clone();

        // Step 3: For each import in the entry module, inject the relevant
        // public declarations from the imported module.
        for decl in &entry_module.ast.decls {
            if let TopDecl::Import(import_decl) = decl {
                let module_name = match import_decl {
                    ImportDecl::Simple { path, .. } => path.clone(),
                    ImportDecl::Aliased { name, .. } => name.clone(),
                    ImportDecl::Selective { path, .. } => path.clone(),
                };

                let module_name_opt = if module_exports.contains_key(&module_name) {
                    Some(module_name.clone())
                } else if module_exports.contains_key(&format!("<stdlib::{}>", module_name)) {
                    Some(format!("<stdlib::{}>", module_name))
                } else {
                    None
                };
                let resolved_name = match module_name_opt {
                    Some(name) => name,
                    None => continue,
                };

                if let Some(pub_decls) = module_exports.get(&resolved_name) {
                    match import_decl {
                        ImportDecl::Simple { .. } | ImportDecl::Aliased { .. } => {
                            // Import entire module: inject all pub decls
                            merged_ast.decls.extend(pub_decls.clone());
                        }
                        ImportDecl::Selective { names, span, .. } => {
                            // Import specific symbols: only inject matching pub decls
                            for name in names {
                                let found = pub_decls.iter().any(|d| match d {
                                    TopDecl::Func(f) => &f.name == name,
                                    TopDecl::Constant(c) => &c.name == name,
                                    TopDecl::Group(g) => &g.name == name,
                                    TopDecl::Enum(e) => &e.name == name,
                                    TopDecl::Trait(t) => &t.name == name,
                                    _ => false,
                                });
                                if found {
                                    for d in pub_decls {
                                        let matches = match d {
                                            TopDecl::Func(f) => &f.name == name,
                                            TopDecl::Constant(c) => &c.name == name,
                                            TopDecl::Group(g) => &g.name == name,
                                            TopDecl::Enum(e) => &e.name == name,
                                            TopDecl::Trait(t) => &t.name == name,
                                            _ => false,
                                        };
                                        if matches {
                                            merged_ast.decls.push(d.clone());
                                        }
                                    }
                                } else {
                                    diag.error(
                                        span.clone(),
                                        format!(
                                            "'{}' is not a public symbol in module '{}'.",
                                            name, module_name
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Semantic Analysis Pass
        let mut type_env = types::TypeEnv::new(&mut diag);
        let mut semantic = semantic::SemanticAnalyzer::new(&mut type_env, module_exports.clone());

        semantic.analyze_program(&merged_ast);
        if diag.has_errors() {
            diag.emit_all();
            std::process::exit(1);
        }

        if cmd == "check" {
            println!("✅ Type-checking successful.");
            return;
        } else if cmd == "run" {
            let mut interpreter = runtime::interpreter::Interpreter::new(module_exports);
            match interpreter.run_program(&merged_ast) {
                Ok(val) => {
                    if val != runtime::value::Value::Unit {
                        println!("{}", val);
                    }
                }
                Err(e) => {
                    eprintln!("Runtime Error: {}", e);
                    std::process::exit(1);
                }
            }
        } else if cmd == "compile" {
            #[cfg(feature = "llvm")]
            {
                // LLVM Codegen Pass
                let llvm_ctx = inkwell::context::Context::create();
                let mut llvm_codegen =
                    codegen::llvm::LLVMCodegen::new(&llvm_ctx, "ferrite_module", &type_env);

                if let Err(e) = llvm_codegen.compile_program(&entry_module.ast) {
                    eprintln!("LLVM Codegen Error: {}", e);
                    std::process::exit(1);
                }

                // Output to .ll text
                let out_path = entry_path.with_extension("ll");
                if let Err(e) = llvm_codegen.emit_to_file(&out_path) {
                    eprintln!("Failed to write LLVM IR: {}", e);
                } else {
                    println!("✅ Compiled native IR to {}", out_path.display());
                }
            }
            #[cfg(not(feature = "llvm"))]
            {
                eprintln!("Ferrite was compiled without the 'llvm' backend feature enabled.");
                eprintln!(
                    "Please install LLVM 15 and recompile the compiler with --features llvm."
                );
                std::process::exit(1);
            }
        } else {
            eprintln!("Unknown command: {}", cmd);
            std::process::exit(1);
        }
    } else {
        diag.emit_all();
        std::process::exit(1);
    }
}
