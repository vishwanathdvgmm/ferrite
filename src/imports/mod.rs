use crate::ast::{ImportDecl, Program, TopDecl};
use crate::errors::{DiagnosticBag, Span};
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// ── Module Representation ────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Module {
    pub path: PathBuf,
    pub name: String,
    pub ast: Program,
}

// ── Import Resolver ──────────────────────────────────────────────

pub struct ImportResolver<'a> {
    diag: &'a mut DiagnosticBag,
    // Maps canonical paths to fully loaded modules
    loaded_modules: HashMap<PathBuf, Module>,
    // Tracks the current visit stack to detect circular imports
    visiting: HashSet<PathBuf>,
}

impl<'a> ImportResolver<'a> {
    pub fn new(diag: &'a mut DiagnosticBag) -> Self {
        Self {
            diag,
            loaded_modules: HashMap::new(),
            visiting: HashSet::new(),
        }
    }

    /// Resolves and loads the entry file and all its dependencies recursively.
    pub fn resolve_entry(&mut self, entry_path: &Path) -> Option<PathBuf> {
        let canonical_entry = match entry_path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // For error reporting without a span, we just use a dummy
                self.diag.error(
                    Span::dummy(),
                    format!("Could not find entry file: {}", entry_path.display()),
                );
                return None;
            }
        };

        if self.load_module(&canonical_entry).is_some() {
            Some(canonical_entry)
        } else {
            None
        }
    }

    pub fn get_module(&self, path: &PathBuf) -> Option<&Module> {
        self.loaded_modules.get(path)
    }

    pub fn into_modules(self) -> HashMap<PathBuf, Module> {
        self.loaded_modules
    }

    // ── Internal Loading Logic ────────────────────────────────────

    fn load_module(&mut self, canonical_path: &PathBuf) -> Option<()> {
        if self.loaded_modules.contains_key(canonical_path) {
            return Some(()); // Already loaded
        }

        if self.visiting.contains(canonical_path) {
            self.diag.error(
                Span::dummy(),
                format!("Circular import detected: {}", canonical_path.display()),
            );
            return None;
        }

        self.visiting.insert(canonical_path.clone());

        // 1. Read the source file
        let is_stdlib = canonical_path.to_string_lossy().starts_with("<stdlib::");

        let src = if is_stdlib {
            let name = canonical_path
                .to_string_lossy()
                .replace("<stdlib::", "")
                .replace(">", "");
            crate::stdlib::get_stdlib_module(&name).unwrap().to_string()
        } else {
            match std::fs::read_to_string(canonical_path) {
                Ok(s) => s,
                Err(e) => {
                    self.diag.error(
                        Span::dummy(),
                        format!("Failed to read {}: {}", canonical_path.display(), e),
                    );
                    self.visiting.remove(canonical_path);
                    return None;
                }
            }
        };

        // Cache the source for error reporting
        self.diag
            .register_source(canonical_path.clone(), src.clone());

        // 2. Lex
        let mut lexer = Lexer::new(&src, canonical_path.clone());
        let tokens = lexer.tokenize(self.diag);

        if self.diag.has_errors() {
            self.visiting.remove(canonical_path);
            return None;
        }

        // 3. Parse
        let mut parser = Parser::new(tokens, self.diag);
        let ast = parser.parse_program();

        if self.diag.has_errors() {
            self.visiting.remove(canonical_path);
            return None;
        }

        // 4. Resolve dependencies recursively
        self.resolve_dependencies(canonical_path, &ast)?;

        // 5. Build and store the module
        let mut module_name = canonical_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // special case for stdlib
        if canonical_path
            .components()
            .any(|c| c.as_os_str() == "stdlib")
        {
            module_name = format!("std::{}", module_name);
        }

        let module = Module {
            path: canonical_path.clone(),
            name: module_name,
            ast,
        };

        self.loaded_modules.insert(canonical_path.clone(), module);
        self.visiting.remove(canonical_path);

        Some(())
    }

    fn resolve_dependencies(&mut self, current_file: &PathBuf, ast: &Program) -> Option<()> {
        let parent_dir = current_file.parent().unwrap_or(Path::new(""));

        for decl in &ast.decls {
            if let TopDecl::Import(import_decl) = decl {
                let (raw_path, span) = match import_decl {
                    ImportDecl::Simple { path, span } => (path, span),
                    ImportDecl::Aliased { name, span, .. } => (name, span),
                    ImportDecl::Selective { path, span, .. } => (path, span),
                };

                // Check if it's a standard library import
                let canonical_target = if crate::stdlib::get_stdlib_module(raw_path).is_some() {
                    PathBuf::from(format!("<stdlib::{}>", raw_path))
                } else {
                    let mut p = parent_dir.join(raw_path);
                    if p.extension().is_none() {
                        p.set_extension("fe");
                    }
                    match p.canonicalize() {
                        Ok(p) => p,
                        Err(_) => {
                            self.diag.error(
                                span.clone(),
                                format!("Cannot resolve import: {}", raw_path),
                            );
                            continue; // Keep trying other imports to collect errors
                        }
                    }
                };

                // Recursive load
                self.load_module(&canonical_target);
            }
        }

        if self.diag.has_errors() {
            None
        } else {
            Some(())
        }
    }
}
