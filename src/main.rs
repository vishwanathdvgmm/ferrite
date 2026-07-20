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

use std::collections::HashMap;
use std::path::PathBuf;

use ast::{ImportDecl, TopDecl, Visibility};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let version_str = "Ferrite v2.4.0 Compiler (AOT ML Language)";
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
