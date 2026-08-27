use crate::ast::*;
use crate::errors;
use crate::lexer;
use crate::parser;
use std::path::PathBuf;

pub fn format_source(source: &str) -> Result<String, String> {
    let mut diag = errors::DiagnosticBag::new();
    let mut lexer = lexer::Lexer::new(source, PathBuf::from("lsp_fmt.fe"));
    let tokens = lexer.tokenize(&mut diag);
    if diag.has_errors() {
        return Err("Syntax errors prevent formatting".to_string());
    }
    let mut parser = parser::Parser::new(tokens, &mut diag);
    let program = parser.parse_program();
    if diag.has_errors() {
        return Err("Syntax errors prevent formatting".to_string());
    }
    let mut formatter = Formatter::new(lexer.comments);
    Ok(formatter.format_program(&program))
}

pub struct Formatter {
    indent_level: usize,
    output: String,
    comments: Vec<(u32, String)>,
}

impl Formatter {
    pub fn new(mut comments: Vec<(u32, String)>) -> Self {
        // Sort comments by line just in case, though they are inherently ordered
        comments.sort_by_key(|c| c.0);
        Self {
            indent_level: 0,
            output: String::new(),
            comments,
        }
    }

    fn indent(&mut self) {
        self.output.push_str(&"    ".repeat(self.indent_level));
    }

    fn flush_comments(&mut self, up_to_line: u32, is_trailing: bool) {
        let mut i = 0;
        while i < self.comments.len() {
            let line = self.comments[i].0;
            if line <= up_to_line {
                let text = self.comments[i].1.clone();
                if is_trailing && line == up_to_line {
                    // Prepend space for trailing comment
                    self.output.push_str(" ");
                    self.output.push_str(&text);
                    self.output.push('\n');
                } else if !is_trailing && line < up_to_line {
                    // Indent and print standalone comment
                    self.indent();
                    self.output.push_str(&text);
                    self.output.push('\n');
                } else if !is_trailing && line == up_to_line {
                    // Standalone comment exactly on the line
                    self.indent();
                    self.output.push_str(&text);
                    self.output.push('\n');
                } else {
                    // Skip trailing condition if not matched
                    i += 1;
                    continue;
                }
                self.comments.remove(i);
            } else {
                break;
            }
        }
    }

    pub fn format_program(&mut self, program: &Program) -> String {
        for (i, decl) in program.decls.iter().enumerate() {
            if i > 0 {
                self.output.push_str("\n\n");
            }
            self.format_top_decl(decl);
        }
        self.flush_comments(u32::MAX, false);
        self.output.push('\n');
        self.output.clone()
    }

    fn format_type(&self, ty: &Type) -> String {
        match ty {
            Type::Primitive(p, _) => format!("{:?}", p).to_lowercase(),
            Type::Named(n, _) => n.clone(),
            Type::SelfType(_) => "Self".to_string(),
            Type::Generic { name, args, .. } => {
                let args_str: Vec<String> = args.iter().map(|t| self.format_type(t)).collect();
                format!("{}<{}>", name, args_str.join(", "))
            }
            Type::Tensor { elem, shape, .. } => {
                let shape_str: Vec<String> = shape
                    .iter()
                    .map(|s| match s {
                        ShapeDim::Const(i) => i.to_string(),
                        ShapeDim::Symbolic(id) => id.clone(),
                    })
                    .collect();
                format!(
                    "Tensor<{}, ({})>",
                    self.format_type(elem),
                    shape_str.join(", ")
                )
            }
        }
    }

    fn format_generics(&self, generics: &[GenericParam]) -> String {
        if generics.is_empty() {
            return String::new();
        }
        let params: Vec<String> = generics
            .iter()
            .map(|p| match p {
                GenericParam::Type { name, .. } => name.clone(),
                GenericParam::Shape { name, .. } => format!("{}: shape", name),
                GenericParam::Bounded { name, bounds, .. } => {
                    let bs: Vec<String> = bounds.iter().map(|b| b.name.clone()).collect();
                    format!("{}: {}", name, bs.join(" + "))
                }
            })
            .collect();
        format!("<{}>", params.join(", "))
    }

    fn format_where_clause(&self, where_clause: &[Constraint]) -> String {
        if where_clause.is_empty() {
            return String::new();
        }
        let constraints: Vec<String> = where_clause
            .iter()
            .map(|c| match c {
                Constraint::ShapeRel {
                    left, op, right, ..
                } => {
                    let op_str = match op {
                        RelOp::Eq => "==",
                        RelOp::NotEq => "!=",
                        RelOp::Lt => "<",
                        RelOp::Gt => ">",
                        RelOp::LtEq => "<=",
                        RelOp::GtEq => ">=",
                    };
                    let rhs_str = match right {
                        ConstraintRhs::Int(i) => i.to_string(),
                        ConstraintRhs::Ident(id) => id.clone(),
                    };
                    format!("{} {} {}", left, op_str, rhs_str)
                }
                Constraint::TraitBound { param, bounds, .. } => {
                    let bs: Vec<String> = bounds.iter().map(|b| b.name.clone()).collect();
                    format!("{}: {}", param, bs.join(" + "))
                }
            })
            .collect();
        format!("\n    where {}", constraints.join(", "))
    }

    fn format_effects(&self, effects: &[Effect]) -> String {
        if effects.is_empty() {
            return String::new();
        }
        let effs: Vec<String> = effects
            .iter()
            .map(|e| match e {
                Effect::Infer => "infer".to_string(),
                Effect::Train => "train".to_string(),
                Effect::Async => "async".to_string(),
                Effect::Named(n) => n.clone(),
            })
            .collect();
        format!("{} ", effs.join(" "))
    }

    fn format_method(&mut self, method: &MethodDecl) {
        self.flush_comments(method.span.line - 1, false);
        self.indent();
        self.output.push_str(&format!(
            "{}fun {}(",
            self.format_effects(&method.effects),
            method.name
        ));

        let mut first = true;
        if method.has_self {
            self.output.push_str("self");
            first = false;
        }

        for param in &method.params {
            if !first {
                self.output.push_str(", ");
            }
            first = false;
            self.output
                .push_str(&format!("{}: {}", param.name, self.format_type(&param.ty)));
        }
        self.output.push_str(")");

        if !method.return_effects.is_empty() {
            self.output.push_str(&format!(
                " ! {}",
                self.format_effects(&method.return_effects).trim()
            ));
        }

        if let Some(ret) = &method.return_type {
            self.output
                .push_str(&format!(" -> {}", self.format_type(ret)));
        }

        self.output
            .push_str(&self.format_where_clause(&method.where_clause));
        self.output.push_str(" {\n");
        self.indent_level += 1;
        for stmt in &method.body.stmts {
            self.format_stmt(stmt);
        }
        self.indent_level -= 1;
        self.indent();
        self.output.push_str("}\n");
        self.flush_comments(method.span.line, true);
    }

    fn format_trait_method(&mut self, method: &TraitMethodSig) {
        self.flush_comments(method.span.line - 1, false);
        self.indent();
        self.output.push_str(&format!("fun {}(", method.name));

        let mut first = true;
        if method.has_self {
            self.output.push_str("self");
            first = false;
        }

        for param in &method.params {
            if !first {
                self.output.push_str(", ");
            }
            first = false;
            self.output
                .push_str(&format!("{}: {}", param.name, self.format_type(&param.ty)));
        }
        self.output.push_str(")");

        if let Some(ret) = &method.return_type {
            self.output
                .push_str(&format!(" -> {}", self.format_type(ret)));
        }
        self.output.push_str(";\n");
        self.flush_comments(method.span.line, true);
    }

    fn format_top_decl(&mut self, decl: &TopDecl) {
        let span = match decl {
            TopDecl::Import(ImportDecl::Simple { span, .. }) => span,
            TopDecl::Import(ImportDecl::Aliased { span, .. }) => span,
            TopDecl::Import(ImportDecl::Selective { span, .. }) => span,
            TopDecl::Constant(c) => &c.span,
            TopDecl::Group(g) => &g.span,
            TopDecl::Enum(e) => &e.span,
            TopDecl::Func(f) => &f.span,
            TopDecl::TestFunc(f) => &f.span,
            TopDecl::ExternBlock(eb) => &eb.span,
            TopDecl::Trait(t) => &t.span,
            TopDecl::Impl(i) => &i.span,
        };
        self.flush_comments(span.line - 1, false);

        match decl {
            TopDecl::Import(import) => match import {
                ImportDecl::Simple { path, .. } => {
                    self.output.push_str(&format!("import \"{}\";", path));
                }
                ImportDecl::Aliased { name, alias, .. } => {
                    self.output
                        .push_str(&format!("import {} as {};", name, alias));
                }
                ImportDecl::Selective { path, names, .. } => {
                    self.output.push_str(&format!(
                        "from \"{}\" take {{ {} }};",
                        path,
                        names.join(", ")
                    ));
                }
            },
            TopDecl::Constant(c) => {
                let vis = if c.visibility == Visibility::Public {
                    "pub "
                } else {
                    ""
                };
                self.output.push_str(&format!(
                    "{}keep {}: {} = ",
                    vis,
                    c.name,
                    self.format_type(&c.ty)
                ));
                self.format_expr(&c.value);
                self.output.push(';');
            }
            TopDecl::Group(g) => {
                let vis = if g.visibility == Visibility::Public {
                    "pub "
                } else {
                    ""
                };
                self.output.push_str(&format!(
                    "{}group {}{} {{\n",
                    vis,
                    g.name,
                    self.format_generics(&g.generics)
                ));
                self.indent_level += 1;
                for field in &g.fields {
                    self.flush_comments(field.span.line - 1, false);
                    self.indent();
                    self.output.push_str(&format!(
                        "{}: {};",
                        field.name,
                        self.format_type(&field.ty)
                    ));
                    self.flush_comments(field.span.line, true);
                    if !self.output.ends_with('\n') {
                        self.output.push('\n');
                    }
                }
                for method in &g.methods {
                    self.format_method(method);
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            TopDecl::Enum(e) => {
                let vis = if e.visibility == Visibility::Public {
                    "pub "
                } else {
                    ""
                };
                self.output.push_str(&format!(
                    "{}enum {}{} {{\n",
                    vis,
                    e.name,
                    self.format_generics(&e.generics)
                ));
                self.indent_level += 1;
                for variant in &e.variants {
                    self.flush_comments(variant.span.line - 1, false);
                    self.indent();
                    self.output.push_str(&variant.name);
                    if !variant.fields.is_empty() {
                        let types: Vec<String> =
                            variant.fields.iter().map(|t| self.format_type(t)).collect();
                        self.output.push_str(&format!("({})", types.join(", ")));
                    }
                    self.output.push_str(";");
                    self.flush_comments(variant.span.line, true);
                    if !self.output.ends_with('\n') {
                        self.output.push('\n');
                    }
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            TopDecl::Func(f) => {
                let vis = if f.visibility == Visibility::Public {
                    "pub "
                } else {
                    ""
                };
                let effect_params = if f.effect_params.is_empty() {
                    String::new()
                } else {
                    format!("[{}] ", f.effect_params.join(", "))
                };

                self.output.push_str(&format!(
                    "{}{}{}fun {}{}(",
                    vis,
                    effect_params,
                    self.format_effects(&f.effects),
                    f.name,
                    self.format_generics(&f.generics)
                ));
                for (i, param) in f.params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&format!(
                        "{}: {}",
                        param.name,
                        self.format_type(&param.ty)
                    ));
                }
                self.output.push_str(")");
                if !f.return_effects.is_empty() {
                    self.output.push_str(&format!(
                        " ! {}",
                        self.format_effects(&f.return_effects).trim()
                    ));
                }
                if let Some(ret) = &f.return_type {
                    self.output
                        .push_str(&format!(" -> {}", self.format_type(ret)));
                }
                self.output
                    .push_str(&self.format_where_clause(&f.where_clause));
                self.output.push_str(" {\n");
                self.indent_level += 1;
                for stmt in &f.body.stmts {
                    self.format_stmt(stmt);
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            TopDecl::Trait(t) => {
                let vis = if t.visibility == Visibility::Public {
                    "pub "
                } else {
                    ""
                };
                self.output.push_str(&format!(
                    "{}trait {}{} {{\n",
                    vis,
                    t.name,
                    self.format_generics(&t.generics)
                ));
                self.indent_level += 1;
                for method in &t.methods {
                    self.format_trait_method(method);
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            TopDecl::Impl(i) => {
                self.output.push_str("impl");
                let gen = self.format_generics(&i.generics);
                if !gen.is_empty() {
                    self.output.push_str(&gen);
                }
                if let Some(tr) = &i.trait_name {
                    self.output.push_str(&format!(" {} for", tr));
                }
                self.output.push_str(&format!(" {}", i.target_type));
                self.output
                    .push_str(&self.format_where_clause(&i.where_clause));
                self.output.push_str(" {\n");
                self.indent_level += 1;
                for method in &i.methods {
                    self.format_method(method);
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            TopDecl::TestFunc(f) => {
                self.output.push_str(&format!("test fun {}() {{\n", f.name));
                self.indent_level += 1;
                for stmt in &f.body.stmts {
                    self.format_stmt(stmt);
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            TopDecl::ExternBlock(eb) => {
                self.output.push_str(&format!("extern \"{}\" {{\n", eb.abi));
                self.indent_level += 1;
                for f in &eb.functions {
                    self.indent();
                    self.output.push_str(&format!("fun {}(", f.name));
                    for (i, param) in f.params.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.output.push_str(&format!(
                            "{}: {}",
                            param.name,
                            self.format_type(&param.ty)
                        ));
                    }
                    self.output.push_str(")");
                    if let Some(ret) = &f.return_type {
                        self.output
                            .push_str(&format!(" -> {}", self.format_type(ret)));
                    }
                    self.output.push_str(";\n");
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
        }
        self.flush_comments(span.line, true);
    }

    fn format_stmt(&mut self, stmt: &Stmt) {
        let span = match stmt {
            Stmt::Keep { span, .. } => span,
            Stmt::Param { span, .. } => span,

            Stmt::ExprStmt(expr, _) => {
                // Approximate span of expr stmt for comments
                match expr {
                    Expr::Lit(_, span) => span,
                    Expr::Ident(_, span) => span,
                    Expr::BinOp { span, .. } => span,
                    Expr::UnaryOp { span, .. } => span,
                    Expr::Call { span, .. } => span,
                    Expr::FieldAccess { span, .. } => span,
                    Expr::IndexAccess { span, .. } => span,
                    Expr::Lambda { span, .. } => span,
                    Expr::GroupLiteral { span, .. } => span,
                    Expr::Assign { span, .. } => span,
                    Expr::UnsafeBlock(_, span) => span,
                    Expr::Block(b) => &b.span,
                    Expr::If { span, .. } => span,
                    Expr::While { span, .. } => span,
                    Expr::For { span, .. } => span,
                    Expr::Match { span, .. } => span,
                    Expr::InferBlock(b) => &b.span,
                    Expr::TrainBlock(b) => &b.span,
                    Expr::Select { span, .. } => span,
                    Expr::Return { span, .. } => span,
                    Expr::Stop(span) => span,
                    Expr::Skip(span) => span,
                }
            }
        };

        if span.line > 0 {
            self.flush_comments(span.line - 1, false);
        }
        self.indent();
        match stmt {
            Stmt::Keep {
                name, ty, value, ..
            } => {
                self.output
                    .push_str(&format!("keep {}: {} = ", name, self.format_type(ty)));
                self.format_expr(value);
                self.output.push_str(";");
            }
            Stmt::ExprStmt(expr, has_semi) => {
                self.format_expr(expr);
                if *has_semi {
                    self.output.push_str(";");
                }
            }

            Stmt::Param {
                name, ty, value, ..
            } => {
                self.output
                    .push_str(&format!("param {}: {} = ", name, self.format_type(ty)));
                self.format_expr(value);
                self.output.push_str(";");
            }
        }
        if span.line > 0 {
            self.flush_comments(span.line, true);
        }
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn format_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Literal(lit) => match lit {
                Literal::Int(i) => self.output.push_str(&i.to_string()),
                Literal::Float(f) => self.output.push_str(&format!("{:?}", f)),
                Literal::Bool(b) => self.output.push_str(&b.to_string()),
                Literal::String(s) => self.output.push_str(&format!("\"{}\"", s)),
            },
            Pattern::Wildcard(_) => self.output.push_str("_"),
            Pattern::Binding(name, _) => self.output.push_str(name),
            Pattern::Constructor { name, fields, .. } => {
                self.output.push_str(name);
                if !fields.is_empty() {
                    self.output.push('(');
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.format_pattern(f);
                    }
                    self.output.push(')');
                }
            }
            Pattern::Struct { name, fields, .. } => {
                self.output.push_str(name);
                self.output.push_str(" { ");
                for (i, (fname, fpat)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    if let Pattern::Binding(bname, _) = fpat {
                        if fname == bname {
                            self.output.push_str(fname);
                            continue;
                        }
                    }
                    self.output.push_str(&format!("{}: ", fname));
                    self.format_pattern(fpat);
                }
                self.output.push_str(" }");
            }
        }
    }

    fn format_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Lit(lit, _) => match lit {
                Literal::Int(i) => self.output.push_str(&i.to_string()),
                Literal::Float(f) => self.output.push_str(&format!("{:?}", f)),
                Literal::Bool(b) => self.output.push_str(&b.to_string()),
                Literal::String(s) => self.output.push_str(&format!("\"{}\"", s)),
            },
            Expr::Ident(id, _) => self.output.push_str(id),
            Expr::BinOp {
                left, op, right, ..
            } => {
                self.format_expr(left);
                let op_str = match op {
                    BinOp::Add => " + ",
                    BinOp::Sub => " - ",
                    BinOp::Mul => " * ",
                    BinOp::Div => " / ",
                    BinOp::Mod => " % ",
                    BinOp::Eq => " == ",
                    BinOp::NotEq => " != ",
                    BinOp::Lt => " < ",
                    BinOp::Gt => " > ",
                    BinOp::LtEq => " <= ",
                    BinOp::GtEq => " >= ",
                    BinOp::And => " && ",
                    BinOp::Or => " || ",
                };
                self.output.push_str(op_str);
                self.format_expr(right);
            }
            Expr::UnaryOp { op, operand, .. } => {
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                    UnaryOp::Await => "await ",
                };
                self.output.push_str(op_str);
                self.format_expr(operand);
            }
            Expr::Call { callee, args, .. } => {
                self.format_expr(callee);
                self.output.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_expr(arg);
                }
                self.output.push(')');
            }
            Expr::FieldAccess { object, field, .. } => {
                self.format_expr(object);
                self.output.push_str(&format!(".{}", field));
            }
            Expr::IndexAccess { object, index, .. } => {
                self.format_expr(object);
                self.output.push('[');
                self.format_expr(index);
                self.output.push(']');
            }
            Expr::Assign { target, value, .. } => {
                self.format_expr(target);
                self.output.push_str(" = ");
                self.format_expr(value);
            }
            Expr::Lambda { params, body, .. } => {
                self.output.push_str("(");
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&format!(
                        "{}: {}",
                        param.name,
                        self.format_type(&param.ty)
                    ));
                }
                self.output.push_str(") => ");
                self.format_expr(body);
            }
            Expr::GroupLiteral { name, fields, .. } => {
                self.output.push_str(name);
                self.output.push_str(" { ");
                for (i, (field_name, field_expr)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&format!("{}: ", field_name));
                    self.format_expr(field_expr);
                }
                self.output.push_str(" }");
            }
            Expr::UnsafeBlock(b, _) => {
                self.output.push_str("unsafe {\n");
                self.indent_level += 1;
                for stmt in &b.stmts {
                    self.format_stmt(stmt);
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            Expr::Block(b) => {
                self.output.push_str("{\n");
                self.indent_level += 1;
                for stmt in &b.stmts {
                    self.format_stmt(stmt);
                }
                if let Some(e) = &b.expr {
                    self.flush_comments(e.span().line - 1, false);
                    self.indent();
                    self.format_expr(e);
                    self.output.push('\n');
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            Expr::If {
                condition,
                then_block,
                elif_branches,
                else_block,
                ..
            } => {
                self.output.push_str("if ");
                self.format_expr(condition);
                self.output.push_str(" ");
                self.format_expr(&Expr::Block(then_block.clone()));
                for (cond, blk) in elif_branches {
                    self.output.push_str(" else if ");
                    self.format_expr(cond);
                    self.output.push_str(" ");
                    self.format_expr(&Expr::Block(blk.clone()));
                }
                if let Some(eb) = else_block {
                    self.output.push_str(" else ");
                    self.format_expr(&Expr::Block(eb.clone()));
                }
            }
            Expr::While {
                condition, body, ..
            } => {
                self.output.push_str("while ");
                self.format_expr(condition);
                self.output.push_str(" ");
                self.format_expr(&Expr::Block(body.clone()));
            }
            Expr::For {
                var,
                iterable,
                body,
                ..
            } => {
                self.output.push_str(&format!("for {} in ", var));
                self.format_expr(iterable);
                self.output.push_str(" ");
                self.format_expr(&Expr::Block(body.clone()));
            }
            Expr::Match { subject, cases, .. } => {
                self.output.push_str("match ");
                self.format_expr(subject);
                self.output.push_str(" {\n");
                self.indent_level += 1;
                for case in cases {
                    self.flush_comments(case.span.line - 1, false);
                    self.indent();
                    if let Pattern::Wildcard(_) = case.pattern {
                        self.output.push_str("default");
                    } else {
                        self.output.push_str("case ");
                        self.format_pattern(&case.pattern);
                        if let Some(guard) = &case.guard {
                            self.output.push_str(" if ");
                            self.format_expr(guard);
                        }
                    }
                    self.output.push_str(" => ");
                    self.format_expr(&Expr::Block(case.body.clone()));
                    self.output.push('\n');
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            Expr::Select { cases, .. } => {
                self.output.push_str("select {\n");
                self.indent_level += 1;
                for case in cases {
                    self.flush_comments(case.span.line - 1, false);
                    self.indent();
                    if case.is_default {
                        self.output.push_str("default => ");
                    } else {
                        self.output.push_str("case ");
                        if let Some((name, expr)) = &case.assignment {
                            if name == "_" {
                                self.format_expr(expr);
                            } else {
                                self.output.push_str(&format!("{} = ", name));
                                self.format_expr(expr);
                            }
                        }
                        self.output.push_str(" => ");
                    }
                    self.format_expr(&Expr::Block(case.body.clone()));
                    self.output.push('\n');
                }
                self.indent_level -= 1;
                self.indent();
                self.output.push('}');
            }
            Expr::InferBlock(b) => {
                self.output.push_str("infer ");
                self.format_expr(&Expr::Block(b.clone()));
            }
            Expr::TrainBlock(b) => {
                self.output.push_str("train ");
                self.format_expr(&Expr::Block(b.clone()));
            }
            Expr::Return { value, .. } => {
                self.output.push_str("return");
                if let Some(v) = value {
                    self.output.push_str(" ");
                    self.format_expr(v);
                }
            }
            Expr::Stop(_) => {
                self.output.push_str("stop");
            }
            Expr::Skip(_) => {
                self.output.push_str("skip");
            }
        }
    }
}
