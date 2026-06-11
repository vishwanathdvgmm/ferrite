pub mod ast;
#[cfg(feature = "llvm")]
pub mod codegen;
pub mod errors;
pub mod imports;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod semantic;
pub mod stdlib;
pub mod types;

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let version_str = "Ferrite v2.2.1 Compiler (AOT ML Language)";
    let usage_str = "Usage:
  ferrite run     <file.fe>   # Execute via Tree-Walk Interpreter
  ferrite check   <file.fe>   # Parse and Type-check only
  ferrite compile <file.fe>   # Compile to native LLVM IR / Object
  ferrite --version           # Print compiler version
  ferrite --help              # Print this help message";

    if args.len() == 2 {
        let arg = &args[1];
        if arg == "--version" || arg == "-V" || arg == "-v" {
            println!("{}", version_str);
            return;
        } else if arg == "--help" || arg == "-h" {
            println!("{}\n\n{}", version_str, usage_str);
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

        // Merge all module ASTs into one contiguous program
        let mut merged_ast = entry_module.ast.clone();
        for (module_path, module) in &modules {
            if module_path != &entry_path {
                merged_ast.decls.extend(module.ast.decls.clone());
            }
        }

        // Semantic Analysis Pass
        let mut type_env = types::TypeEnv::new(&mut diag);
        let mut semantic = semantic::SemanticAnalyzer::new(&mut type_env);

        semantic.analyze_program(&merged_ast);
        if diag.has_errors() {
            diag.emit_all();
            std::process::exit(1);
        }

        if cmd == "check" {
            println!("✅ Type-checking successful.");
            return;
        } else if cmd == "run" {
            let mut interpreter = runtime::interpreter::Interpreter::new();
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
