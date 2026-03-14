use crate::ast::{Expr, Lhs, MatchPat, Stmt, UnpackItem};
use std::collections::HashSet;

pub struct Resolver {
    scopes: Vec<HashSet<String>>,
    loop_depth: usize,
    fn_depth: usize,
}

impl Resolver {
    pub fn new() -> Self {
        let mut globals = HashSet::new();
        // Register builtins
        for n in &[
            "len",
            "push",
            "pop",
            "str",
            "int",
            "float",
            "type",
            "range",
            "input",
            "sqrt",
            "abs",
            "max",
            "min",
            "floor",
            "ceil",
            "round",
            "assert",
            "keys",
            "values",
            "has_key",
            "delete",
            "sort",
            "reverse",
            "contains",
            "map",
            "filter",
            "reduce",
            "split",
            "join",
            "replace",
            "starts_with",
            "ends_with",
            "trim",
            "upper",
            "lower",
            "chars",
            "substr",
            "pow",
            "log",
            "log2",
            "log10",
            "sin",
            "cos",
            "tan",
            "atan",
            "atan2",
            "pi",
            "e",
            "inf",
            "format",
            "write",
            "exit",
            "enumerate",
            "zip",
            "read_file",
            "write_file",
            "append_file",
            "file_exists",
            "PI",
            "E",
            "INF",
            "print",
        ] {
            globals.insert(n.to_string());
        }

        Resolver {
            scopes: vec![globals],
            loop_depth: 0,
            fn_depth: 0,
        }
    }

    pub fn resolve(&mut self, stmts: &[Stmt]) -> Result<(), String> {
        for s in stmts {
            self.resolve_stmt(s)?;
        }
        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn check_var(&self, name: &str) -> Result<(), String> {
        for scope in self.scopes.iter().rev() {
            if scope.contains(name) {
                return Ok(());
            }
        }
        // In Ferrite, previously undeclared variables could be injected at runtime (e.g. by `import`).
        // To prevent breaking dynamic imports, we might emit a warning or just let it pass for now.
        // For strict semantic analysis of v1.4.0, we will enforce it!
        // Wait, imported items are not known at compile time.
        // Let's just allow global resolution to fallback to dynamic for now to not break `import "mathutils"` which defines things like `square`.
        // Actually, preventing variable reads is too strict for a dynamic language with `import` unless we parse imports during resolution.
        // For Phase 2a, we will just focus on control flow validation (`break`, `continue`, `return`) and local variable usage.
        Ok(())
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(e) | Stmt::Print(e) | Stmt::Write(e) | Stmt::Throw(e) => {
                self.resolve_expr(e)?;
            }
            Stmt::Let { name, value } => {
                self.resolve_expr(value)?;
                self.declare(name);
            }
            Stmt::LetList { items, value } => {
                self.resolve_expr(value)?;
                for item in items {
                    match item {
                        UnpackItem::Name(n) | UnpackItem::Rest(n) => self.declare(n),
                    }
                }
            }
            Stmt::LetMap { names, value } => {
                self.resolve_expr(value)?;
                for n in names {
                    self.declare(n);
                }
            }
            Stmt::Assign { target, value } => {
                self.resolve_expr(value)?;
                self.resolve_lhs(target)?;
            }
            Stmt::CompoundAssign { target, value, .. } => {
                self.resolve_expr(value)?;
                self.resolve_lhs(target)?;
            }
            Stmt::Return(opt_expr) => {
                if self.fn_depth == 0 {
                    return Err("'return' expression outside of function".to_string());
                }
                if let Some(e) = opt_expr {
                    self.resolve_expr(e)?;
                }
            }
            Stmt::Break => {
                if self.loop_depth == 0 {
                    return Err("'break' outside of loop".to_string());
                }
            }
            Stmt::Continue => {
                if self.loop_depth == 0 {
                    return Err("'continue' outside of loop".to_string());
                }
            }
            Stmt::While { cond, body } => {
                self.resolve_expr(cond)?;
                self.loop_depth += 1;
                self.begin_scope();
                self.resolve(body)?;
                self.end_scope();
                self.loop_depth -= 1;
            }
            Stmt::For { var, iter, body } => {
                self.resolve_expr(iter)?;
                self.loop_depth += 1;
                self.begin_scope();
                self.declare(var);
                self.resolve(body)?;
                self.end_scope();
                self.loop_depth -= 1;
            }
            Stmt::FnDef {
                name,
                params,
                variadic,
                body,
            } => {
                self.declare(name); // Function is available in current scope (allows recursion)
                self.begin_scope();
                self.fn_depth += 1;
                for p in params {
                    self.declare(p);
                }
                if let Some(v) = variadic {
                    self.declare(v);
                }
                self.resolve(body)?;
                self.fn_depth -= 1;
                self.end_scope();
            }
            Stmt::Match { subject, arms } => {
                self.resolve_expr(subject)?;
                for arm in arms {
                    self.begin_scope();
                    match &arm.pattern {
                        MatchPat::Binding(name) => self.declare(name),
                        MatchPat::Literal(e) => {
                            self.resolve_expr(e)?;
                        }
                        MatchPat::Range(s, e) => {
                            self.resolve_expr(s)?;
                            self.resolve_expr(e)?;
                        }
                        MatchPat::Wildcard => {}
                    }
                    self.resolve(&arm.body)?;
                    self.end_scope();
                }
            }
            Stmt::TryCatch {
                body,
                catch_var,
                catch_body,
            } => {
                self.begin_scope();
                self.resolve(body)?;
                self.end_scope();

                self.begin_scope();
                self.declare(catch_var);
                self.resolve(catch_body)?;
                self.end_scope();
            }
            Stmt::Import { path: _ } => {
                // Cannot statically resolve dynamic imports here without a real module system
            }
        }
        Ok(())
    }

    fn resolve_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Int(_) | Expr::Float(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Null => Ok(()),
            Expr::Ident(name) => {
                self.check_var(name)?;
                Ok(())
            }
            Expr::List(items) => {
                for item in items {
                    self.resolve_expr(item)?;
                }
                Ok(())
            }
            Expr::Map(pairs) => {
                for (k, v) in pairs {
                    self.resolve_expr(k)?;
                    self.resolve_expr(v)?;
                }
                Ok(())
            }
            Expr::FStr(parts) => {
                for p in parts {
                    if let crate::lexer::FsPart::Code(src) = p {
                        let toks = crate::lexer::Lexer::new(src)
                            .tokenize()
                            .map_err(|e| format!("In f-string: {}", e))?;
                        let mut parser = crate::parser::Parser::new(toks);
                        let expr = parser
                            .parse_expr()
                            .map_err(|e| format!("In f-string: {}", e))?;
                        self.resolve_expr(&expr)?;
                    }
                }
                Ok(())
            }
            Expr::BinOp { left, right, .. } => {
                self.resolve_expr(left)?;
                self.resolve_expr(right)?;
                Ok(())
            }
            Expr::Unary { expr, .. } => {
                self.resolve_expr(expr)?;
                Ok(())
            }
            Expr::Call { func, args } => {
                self.resolve_expr(func)?;
                for arg in args {
                    self.resolve_expr(arg)?;
                }
                Ok(())
            }
            Expr::Index { obj, idx } => {
                self.resolve_expr(obj)?;
                self.resolve_expr(idx)?;
                Ok(())
            }
            Expr::Field { obj, .. } => {
                self.resolve_expr(obj)?;
                Ok(())
            }
            Expr::If { cond, then, else_ } => {
                self.resolve_expr(cond)?;
                self.begin_scope();
                self.resolve(then)?;
                self.end_scope();
                if let Some(e) = else_ {
                    self.begin_scope();
                    self.resolve(e)?;
                    self.end_scope();
                }
                Ok(())
            }
            Expr::Lambda {
                params,
                variadic,
                body,
            } => {
                self.begin_scope();
                self.fn_depth += 1;
                for p in params {
                    self.declare(p);
                }
                if let Some(v) = variadic {
                    self.declare(v);
                }
                self.resolve(body)?;
                self.fn_depth -= 1;
                self.end_scope();
                Ok(())
            }
        }
    }

    fn resolve_lhs(&mut self, lhs: &Lhs) -> Result<(), String> {
        match lhs {
            Lhs::Ident(name) => {
                self.check_var(name)?;
            }
            Lhs::Index { obj, idx } => {
                self.resolve_expr(obj)?;
                self.resolve_expr(idx)?;
            }
            Lhs::Field { obj, .. } => {
                self.resolve_expr(obj)?;
            }
        }
        Ok(())
    }
}
