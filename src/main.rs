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

    if args.len() < 3 {
        println!("Ferrite v2.0 Compiler (AOT ML Language)");
        println!("Usage:");
        println!("  ferrite check   <file.fe>   # Parse and Type-check only");
        println!("  ferrite compile <file.fe>   # Compile to native LLVM IR / Object");
        return;
    }

    let cmd = &args[1];
    let path = PathBuf::from(&args[2]);

    let mut diag = errors::DiagnosticBag::new();
    let mut resolver = imports::ImportResolver::new(&mut diag);

    if let Some(entry_path) = resolver.resolve_entry(&path) {
        let modules = resolver.into_modules();
        let entry_module = &modules[&entry_path];

        // Semantic Analysis Pass
        let mut type_env = types::TypeEnv::new(&mut diag);
        let mut semantic = semantic::SemanticAnalyzer::new(&mut type_env);

        semantic.analyze_program(&entry_module.ast);
        if diag.has_errors() {
            diag.emit_all();
            std::process::exit(1);
        }

        if cmd == "check" {
            println!("✅ Type-checking successful.");
            return;
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
