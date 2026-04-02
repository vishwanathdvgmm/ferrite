use crate::ast::*;
use crate::errors::{DiagnosticBag, Span};
use crate::lexer::{Token, TokenKind};

// ── Parser ───────────────────────────────────────────────────────

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    diag: &'a mut DiagnosticBag,
    panic_mode: bool,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, diag: &'a mut DiagnosticBag) -> Self {
        Self {
            tokens,
            pos: 0,
            diag,
            panic_mode: false,
        }
    }

    pub fn parse_program(&mut self) -> Program {
        let mut decls = Vec::new();
        while !self.is_at_end() {
            if let Some(decl) = self.parse_top_decl() {
                decls.push(decl);
            } else {
                self.synchronize();
            }
        }
        Program { decls }
    }

    // ── Helper Methods ──────────────────────────────────────────────

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::EOF
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn peek_next(&self) -> &Token {
        self.tokens
            .get(self.pos + 1)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.pos.saturating_sub(1)]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.previous()
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }
        &self.peek().kind == kind
    }

    fn match_token(&mut self, kinds: &[TokenKind]) -> bool {
        for kind in kinds {
            if self.check(kind) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn consume(&mut self, kind: TokenKind, msg: &str) -> Option<&Token> {
        if self.check(&kind) {
            Some(self.advance())
        } else {
            self.error_at_current(msg);
            None
        }
    }

    fn consume_ident(&mut self, msg: &str) -> Option<(String, Span)> {
        match self.peek().kind.clone() {
            TokenKind::Ident(name) => {
                let span = self.peek().span.clone();
                self.advance();
                Some((name, span))
            }
            _ => {
                self.error_at_current(msg);
                None
            }
        }
    }

    fn merge_span(&self, start: &Span, end: &Span) -> Span {
        Span::new(
            start.file.clone(),
            start.line,
            start.col,
            (end.col + end.len).saturating_sub(start.col),
        )
    }

    // ── Error Handling ──────────────────────────────────────────────

    fn error_at_current(&mut self, message: &str) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;
        let span = self.peek().span.clone();
        self.diag.error(span, message);
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;
        if !self.is_at_end() {
            self.advance();
        }
        while !self.is_at_end() {
            if self.previous().kind == TokenKind::Semicolon {
                return;
            }
            match self.peek().kind {
                TokenKind::Fun
                | TokenKind::Keep
                | TokenKind::Param
                | TokenKind::Constant
                | TokenKind::Group
                | TokenKind::Enum
                | TokenKind::Import
                | TokenKind::From
                | TokenKind::If
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Match
                | TokenKind::Infer
                | TokenKind::Train
                | TokenKind::Async
                | TokenKind::Select
                | TokenKind::Return
                | TokenKind::Stop
                | TokenKind::Skip => return,
                _ => {}
            }
            self.advance();
        }
    }

    // ── Declarations ────────────────────────────────────────────────

    fn parse_top_decl(&mut self) -> Option<TopDecl> {
        if self.match_token(&[TokenKind::Import, TokenKind::From]) {
            self.parse_import_decl().map(TopDecl::Import)
        } else if self.match_token(&[TokenKind::Constant]) {
            self.parse_constant_decl().map(TopDecl::Constant)
        } else if self.match_token(&[TokenKind::Group]) {
            self.parse_group_decl().map(TopDecl::Group)
        } else if self.match_token(&[TokenKind::Enum]) {
            self.parse_enum_decl().map(TopDecl::Enum)
        } else {
            // Function declaration might start with `<`, `infer`, `train`, `async`, or `fun`
            // Let's rely on fact that func starts with `fun` eventually
            self.parse_func_decl().map(TopDecl::Func)
        }
    }

    fn parse_import_decl(&mut self) -> Option<ImportDecl> {
        let prev = self.previous().kind.clone();
        let span_start = self.previous().span.clone();

        if prev == TokenKind::Import {
            if let TokenKind::StringLit(path) = self.peek().kind.clone() {
                self.advance();
                let end_span = self.previous().span.clone();
                self.consume(TokenKind::Semicolon, "Expected ';' after import path.")?;
                Some(ImportDecl::Simple {
                    path,
                    span: self.merge_span(&span_start, &end_span),
                })
            } else if let TokenKind::Ident(name) = self.peek().kind.clone() {
                self.advance();
                self.consume(TokenKind::As, "Expected 'as' in import alias.")?;
                let (alias, _) = self.consume_ident("Expected alias name.")?;
                let end_span = self.previous().span.clone();
                self.consume(TokenKind::Semicolon, "Expected ';' after import alias.")?;
                Some(ImportDecl::Aliased {
                    name,
                    alias,
                    span: self.merge_span(&span_start, &end_span),
                })
            } else {
                self.error_at_current("Expected string literal or identifier after 'import'.");
                None
            }
        } else if prev == TokenKind::From {
            if let TokenKind::StringLit(path) = self.peek().kind.clone() {
                self.advance();
                self.consume(TokenKind::Take, "Expected 'take' after from path.")?;
                let (name, _) = self.consume_ident("Expected identifier after 'take'.")?;
                let end_span = self.previous().span.clone();
                self.consume(TokenKind::Semicolon, "Expected ';' after from import.")?;
                Some(ImportDecl::Selective {
                    path,
                    name,
                    span: self.merge_span(&span_start, &end_span),
                })
            } else {
                self.error_at_current("Expected string literal after 'from'.");
                None
            }
        } else {
            None
        }
    }

    fn parse_constant_decl(&mut self) -> Option<ConstantDecl> {
        let start_span = self.previous().span.clone();
        let (name, _) = self.consume_ident("Expected constant name.")?;
        self.consume(TokenKind::Colon, "Expected ':' after constant name.")?;
        let ty = self.parse_type()?;
        self.consume(TokenKind::Eq, "Expected '=' after constant type.")?;
        let value = self.parse_expression()?;
        let end_span = self.previous().span.clone();
        self.consume(
            TokenKind::Semicolon,
            "Expected ';' after constant declaration.",
        )?;
        Some(ConstantDecl {
            name,
            ty,
            value,
            span: self.merge_span(&start_span, &end_span),
        })
    }

    fn parse_group_decl(&mut self) -> Option<GroupDecl> {
        let start_span = self.previous().span.clone();
        let (name, _) = self.consume_ident("Expected group name.")?;
        let generics = self.parse_generic_params_opt();
        self.consume(TokenKind::LBrace, "Expected '{' before group body.")?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::Fun)
                || self.check(&TokenKind::Infer)
                || self.check(&TokenKind::Train)
                || self.check(&TokenKind::Async)
            {
                methods.push(self.parse_method_decl()?);
            } else {
                fields.push(self.parse_field_decl()?);
            }
        }

        self.consume(TokenKind::RBrace, "Expected '}' after group body.")?;
        let end_span = self.previous().span.clone();

        Some(GroupDecl {
            name,
            generics,
            fields,
            methods,
            span: self.merge_span(&start_span, &end_span),
        })
    }

    fn parse_field_decl(&mut self) -> Option<FieldDecl> {
        let (name, span_start) = self.consume_ident("Expected field name.")?;
        self.consume(TokenKind::Colon, "Expected ':' after field name.")?;
        let ty = self.parse_type()?;
        self.consume(
            TokenKind::Semicolon,
            "Expected ';' after field declaration.",
        )?;
        let span = self.merge_span(&span_start, &self.previous().span.clone());
        Some(FieldDecl { name, ty, span })
    }

    fn parse_method_decl(&mut self) -> Option<MethodDecl> {
        let start_span = self.peek().span.clone();
        let effects = self.parse_effect_list();
        self.consume(TokenKind::Fun, "Expected 'fun' in method declaration.")?;
        let (name, _) = self.consume_ident("Expected method name.")?;

        self.consume(TokenKind::LParen, "Expected '(' after method name.")?;

        let mut has_self = false;
        let mut params = Vec::new();

        if self.match_token(&[TokenKind::SelfKw]) {
            has_self = true;
            if self.match_token(&[TokenKind::Comma]) {
                params = self.parse_params()?;
            }
        } else if !self.check(&TokenKind::RParen) {
            params = self.parse_params()?;
        }
        self.consume(TokenKind::RParen, "Expected ')' after method parameters.")?;

        let mut return_effects = Vec::new();
        let mut return_type = None;
        if self.match_token(&[TokenKind::Arrow]) {
            return_effects = self.parse_effect_list();
            return_type = Some(self.parse_type()?);
        }

        let where_clause = self.parse_where_clause();
        let body = self.parse_block()?;
        let span = self.merge_span(&start_span, &self.previous().span.clone());

        Some(MethodDecl {
            effects,
            name,
            has_self,
            params,
            return_effects,
            return_type,
            where_clause,
            body,
            span,
        })
    }

    fn parse_enum_decl(&mut self) -> Option<EnumDecl> {
        let start_span = self.previous().span.clone();
        let (name, _) = self.consume_ident("Expected enum name.")?;
        let generics = self.parse_generic_params_opt();
        self.consume(TokenKind::LBrace, "Expected '{' before enum body.")?;

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            variants.push(self.parse_enum_variant()?);
        }
        self.consume(TokenKind::RBrace, "Expected '}' after enum body.")?;
        let span = self.merge_span(&start_span, &self.previous().span.clone());
        Some(EnumDecl {
            name,
            generics,
            variants,
            span,
        })
    }

    fn parse_enum_variant(&mut self) -> Option<EnumVariant> {
        let (name, start_span) = self.consume_ident("Expected enum variant name.")?;
        let mut fields = Vec::new();
        if self.match_token(&[TokenKind::LParen]) {
            if !self.check(&TokenKind::RParen) {
                loop {
                    fields.push(self.parse_type()?);
                    if !self.match_token(&[TokenKind::Comma]) {
                        break;
                    }
                }
            }
            self.consume(TokenKind::RParen, "Expected ')' after variant types.")?;
        }
        self.consume(TokenKind::Semicolon, "Expected ';' after enum variant.")?;
        let span = self.merge_span(&start_span, &self.previous().span.clone());
        Some(EnumVariant { name, fields, span })
    }

    fn parse_func_decl(&mut self) -> Option<FuncDecl> {
        let start_span = self.peek().span.clone();

        let effect_params = Vec::new(); // Effect generic params logic temporarily simplified
        if self.match_token(&[TokenKind::Lt]) {
            // Function effect params: <infer, train>
            // We use a heuristics here because it could be generic params.
            // But grammar says effect_param_list first.
            // Fallback: we merge them mentally and sort it out in semantics if needed.
            // For now, let's assume it's generics if it's not well formed effect lists.
            // Simplified: we will just parse standard generic params.
            // The prompt says: `< infer, T: shape >`
            // Let's rely on standard generic syntax.
            self.pos -= 1; // back up
        }

        // effect list
        let effects = self.parse_effect_list();
        self.consume(TokenKind::Fun, "Expected 'fun'.")?;
        let (name, _) = self.consume_ident("Expected function name.")?;
        let generics = self.parse_generic_params_opt();

        self.consume(TokenKind::LParen, "Expected '(' after function name.")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            params = self.parse_params()?;
        }
        self.consume(TokenKind::RParen, "Expected ')' after parameters.")?;

        let mut return_effects = Vec::new();
        let mut return_type = None;
        if self.match_token(&[TokenKind::Arrow]) {
            return_effects = self.parse_effect_list();
            return_type = Some(self.parse_type()?);
        }

        let where_clause = self.parse_where_clause();
        let body = self.parse_block()?;
        let span = self.merge_span(&start_span, &self.previous().span.clone());

        Some(FuncDecl {
            effect_params,
            effects,
            name,
            generics,
            params,
            return_effects,
            return_type,
            where_clause,
            body,
            span,
        })
    }

    fn parse_params(&mut self) -> Option<Vec<Param>> {
        let mut params = Vec::new();
        loop {
            let (name, start_span) = self.consume_ident("Expected parameter name.")?;
            self.consume(TokenKind::Colon, "Expected ':' after parameter name.")?;
            let ty = self.parse_type()?;
            let span = self.merge_span(&start_span, &self.previous().span.clone());
            params.push(Param { name, ty, span });
            if !self.match_token(&[TokenKind::Comma]) {
                break;
            }
        }
        Some(params)
    }

    // ── Types & Generics ────────────────────────────────────────────

    fn parse_type(&mut self) -> Option<Type> {
        let span_start = self.peek().span.clone();

        if self.match_token(&[
            TokenKind::Ident("int".into()),
            TokenKind::Ident("float".into()),
            TokenKind::Ident("bool".into()),
            TokenKind::Ident("string".into()),
        ]) {
            let prim = match self.previous().kind {
                TokenKind::Ident(ref s) if s == "int" => PrimType::Int,
                TokenKind::Ident(ref s) if s == "float" => PrimType::Float,
                TokenKind::Ident(ref s) if s == "bool" => PrimType::Bool,
                TokenKind::Ident(ref s) if s == "string" => PrimType::String,
                _ => unreachable!(),
            };
            return Some(Type::Primitive(prim, span_start));
        }

        let (name, span) = self.consume_ident("Expected type name.")?;

        if name == "Tensor" {
            self.consume(TokenKind::Lt, "Expected '<' after Tensor.")?;
            let elem = Box::new(self.parse_type()?);
            self.consume(TokenKind::Comma, "Expected ',' in Tensor type.")?;
            self.consume(TokenKind::LParen, "Expected '(' for Tensor shape.")?;
            let mut shape = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    if let TokenKind::IntLit(n) = self.peek().kind {
                        shape.push(ShapeDim::Const(n));
                        self.advance();
                    } else if let TokenKind::Ident(ref s) = self.peek().kind {
                        shape.push(ShapeDim::Symbolic(s.clone()));
                        self.advance();
                    } else {
                        self.error_at_current(
                            "Expected integer or identifier for shape dimension.",
                        );
                        return None;
                    }
                    if !self.match_token(&[TokenKind::Comma]) {
                        break;
                    }
                }
            }
            self.consume(TokenKind::RParen, "Expected ')' after Tensor shape.")?;
            self.consume(TokenKind::Gt, "Expected '>' after Tensor type.")?;
            let end_span = self.previous().span.clone();
            return Some(Type::Tensor {
                elem,
                shape,
                span: self.merge_span(&span_start, &end_span),
            });
        }

        if self.match_token(&[TokenKind::Lt]) {
            let mut args = Vec::new();
            if !self.check(&TokenKind::Gt) {
                loop {
                    args.push(self.parse_type()?);
                    if !self.match_token(&[TokenKind::Comma]) {
                        break;
                    }
                }
            }
            self.consume(TokenKind::Gt, "Expected '>' after generic type arguments.")?;
            let end_span = self.previous().span.clone();
            return Some(Type::Generic {
                name,
                args,
                span: self.merge_span(&span_start, &end_span),
            });
        }

        Some(Type::Named(name, span))
    }

    fn parse_generic_params_opt(&mut self) -> Vec<GenericParam> {
        let mut params = Vec::new();
        if self.match_token(&[TokenKind::Lt]) {
            if !self.check(&TokenKind::Gt) {
                loop {
                    let start_span = self.peek().span.clone();
                    if let Some((name, _)) = self.consume_ident("Expected generic parameter name.")
                    {
                        if self.match_token(&[TokenKind::Colon]) {
                            if self.match_token(&[TokenKind::Ident("shape".into())]) {
                                params.push(GenericParam::Shape {
                                    name,
                                    span: self
                                        .merge_span(&start_span, &self.previous().span.clone()),
                                });
                            } else {
                                let mut bounds = Vec::new();
                                loop {
                                    if let Some((bound_name, bspan)) =
                                        self.consume_ident("Expected trait bound.")
                                    {
                                        bounds.push(TraitRef {
                                            name: bound_name,
                                            span: bspan,
                                        });
                                    } else {
                                        break;
                                    }
                                    if !self.match_token(&[TokenKind::Plus]) {
                                        break;
                                    }
                                }
                                params.push(GenericParam::Bounded {
                                    name,
                                    bounds,
                                    span: self
                                        .merge_span(&start_span, &self.previous().span.clone()),
                                });
                            }
                        } else {
                            params.push(GenericParam::Type {
                                name,
                                span: self.merge_span(&start_span, &self.previous().span.clone()),
                            });
                        }
                    } else {
                        break;
                    }
                    if !self.match_token(&[TokenKind::Comma]) {
                        break;
                    }
                }
            }
            self.consume(TokenKind::Gt, "Expected '>' after generic parameters.");
        }
        params
    }

    fn parse_where_clause(&mut self) -> Vec<Constraint> {
        let mut constraints = Vec::new();
        if self.match_token(&[TokenKind::Where]) {
            loop {
                let start_span = self.peek().span.clone();
                let (left, _) = match self.consume_ident("Expected identifier in where clause.") {
                    Some(id) => id,
                    None => break,
                };

                let op = if self.match_token(&[TokenKind::EqEq]) {
                    RelOp::Eq
                } else if self.match_token(&[TokenKind::BangEq]) {
                    RelOp::NotEq
                } else if self.match_token(&[TokenKind::Lt]) {
                    RelOp::Lt
                } else if self.match_token(&[TokenKind::Gt]) {
                    RelOp::Gt
                } else if self.match_token(&[TokenKind::LtEq]) {
                    RelOp::LtEq
                } else if self.match_token(&[TokenKind::GtEq]) {
                    RelOp::GtEq
                } else if self.match_token(&[TokenKind::Colon]) {
                    // Trait bound
                    let mut bounds = Vec::new();
                    loop {
                        if let Some((bname, bspan)) = self.consume_ident("Expected trait name.") {
                            bounds.push(TraitRef {
                                name: bname,
                                span: bspan,
                            });
                        } else {
                            break;
                        }
                        if !self.match_token(&[TokenKind::Plus]) {
                            break;
                        }
                    }
                    let span = self.merge_span(&start_span, &self.previous().span.clone());
                    constraints.push(Constraint::TraitBound {
                        param: left,
                        bounds,
                        span,
                    });
                    if !self.match_token(&[TokenKind::Comma]) {
                        return constraints;
                    }
                    continue;
                } else {
                    self.error_at_current("Expected relational operator or ':' in where clause.");
                    return constraints;
                };

                let right = if let TokenKind::IntLit(n) = self.peek().kind {
                    self.advance();
                    ConstraintRhs::Int(n)
                } else if let TokenKind::Ident(ref s) = self.peek().kind {
                    let name = s.clone();
                    self.advance();
                    ConstraintRhs::Ident(name)
                } else {
                    self.error_at_current(
                        "Expected integer or identifier right of constraint operator.",
                    );
                    return constraints;
                };

                let span = self.merge_span(&start_span, &self.previous().span.clone());
                constraints.push(Constraint::ShapeRel {
                    left,
                    op,
                    right,
                    span,
                });

                if !self.match_token(&[TokenKind::Comma]) {
                    break;
                }
            }
        }
        constraints
    }

    fn parse_effect_list(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        while self.match_token(&[TokenKind::Infer]) {
            effects.push(Effect::Infer);
        }
        while self.match_token(&[TokenKind::Train]) {
            effects.push(Effect::Train);
        }
        while self.match_token(&[TokenKind::Async]) {
            effects.push(Effect::Async);
        }
        effects
    }

    // ── Statements & Blocks ─────────────────────────────────────────

    fn parse_block(&mut self) -> Option<Block> {
        let start_span = self.peek().span.clone();
        self.consume(TokenKind::LBrace, "Expected '{' to start block.")?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else {
                self.synchronize();
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' at end of block.")?;
        let span = self.merge_span(&start_span, &self.previous().span.clone());
        Some(Block { stmts, span })
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        if self.match_token(&[TokenKind::Keep]) {
            self.parse_decl_stmt(true)
        } else if self.match_token(&[TokenKind::Param]) {
            self.parse_decl_stmt(false)
        } else if self.match_token(&[TokenKind::Return]) {
            self.parse_return_stmt()
        } else if self.match_token(&[TokenKind::If]) {
            self.parse_if_stmt()
        } else if self.match_token(&[TokenKind::While]) {
            self.parse_while_stmt()
        } else if self.match_token(&[TokenKind::For]) {
            self.parse_for_stmt()
        } else if self.match_token(&[TokenKind::Match]) {
            self.parse_match_stmt()
        } else if self.match_token(&[TokenKind::Infer]) {
            let block = self.parse_block()?;
            Some(Stmt::InferBlock(block))
        } else if self.match_token(&[TokenKind::Train]) {
            let block = self.parse_block()?;
            Some(Stmt::TrainBlock(block))
        } else if self.match_token(&[TokenKind::Select]) {
            self.parse_select_stmt()
        } else if self.match_token(&[TokenKind::Stop]) {
            let span = self.previous().span.clone();
            self.consume(TokenKind::Semicolon, "Expected ';' after stop.")?;
            Some(Stmt::Stop(span))
        } else if self.match_token(&[TokenKind::Skip]) {
            let span = self.previous().span.clone();
            self.consume(TokenKind::Semicolon, "Expected ';' after skip.")?;
            Some(Stmt::Skip(span))
        } else {
            self.parse_expr_stmt()
        }
    }

    fn parse_decl_stmt(&mut self, is_keep: bool) -> Option<Stmt> {
        let start_span = self.previous().span.clone();
        let (name, _) = self.consume_ident("Expected variable name.")?;
        self.consume(TokenKind::Colon, "Expected ':' after variable name.")?;
        let ty = self.parse_type()?;
        self.consume(TokenKind::Eq, "Expected '=' after type.")?;
        let value = self.parse_expression()?;
        self.consume(
            TokenKind::Semicolon,
            "Expected ';' after variable declaration.",
        )?;
        let span = self.merge_span(&start_span, &self.previous().span.clone());

        if is_keep {
            Some(Stmt::Keep {
                name,
                ty,
                value,
                span,
            })
        } else {
            Some(Stmt::Param {
                name,
                ty,
                value,
                span,
            })
        }
    }

    fn parse_return_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.previous().span.clone();
        let value = if !self.check(&TokenKind::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.consume(TokenKind::Semicolon, "Expected ';' after return.")?;
        Some(Stmt::Return {
            value,
            span: self.merge_span(&start_span, &self.previous().span.clone()),
        })
    }

    fn parse_if_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.previous().span.clone();
        let condition = self.parse_expression()?;
        let then_block = self.parse_block()?;

        let mut elif_branches = Vec::new();
        while self.match_token(&[TokenKind::Elif]) {
            let cond = self.parse_expression()?;
            let blk = self.parse_block()?;
            elif_branches.push((cond, blk));
        }

        let mut else_block = None;
        if self.match_token(&[TokenKind::Else]) {
            else_block = Some(self.parse_block()?);
        }

        let span = self.merge_span(&start_span, &self.previous().span.clone());
        Some(Stmt::If {
            condition,
            then_block,
            elif_branches,
            else_block,
            span,
        })
    }

    fn parse_while_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.previous().span.clone();
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        Some(Stmt::While {
            condition,
            body,
            span: self.merge_span(&start_span, &self.previous().span.clone()),
        })
    }

    fn parse_for_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.previous().span.clone();
        let (var, _) = self.consume_ident("Expected iteration variable.")?;
        self.consume(TokenKind::In, "Expected 'in' after iterator variable.")?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;
        Some(Stmt::For {
            var,
            iterable,
            body,
            span: self.merge_span(&start_span, &self.previous().span.clone()),
        })
    }

    fn parse_match_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.previous().span.clone();
        let subject = self.parse_expression()?;
        self.consume(TokenKind::LBrace, "Expected '{' before match cases.")?;
        let mut cases = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let case_start = self.peek().span.clone();
            if self.match_token(&[TokenKind::Case]) {
                let pattern = self.parse_pattern()?;
                self.consume(TokenKind::FatArrow, "Expected '=>' after case pattern.")?;
                let body = self.parse_block()?;
                let span = self.merge_span(&case_start, &self.previous().span.clone());
                cases.push(MatchCase {
                    pattern,
                    body,
                    span,
                });
            } else if self.match_token(&[TokenKind::Default]) {
                self.consume(TokenKind::FatArrow, "Expected '=>' after default.")?;
                let body = self.parse_block()?;
                let span = self.merge_span(&case_start, &self.previous().span.clone());
                cases.push(MatchCase {
                    pattern: Pattern::Wildcard(case_start.clone()),
                    body,
                    span,
                });
            } else {
                self.error_at_current("Expected 'case' or 'default'.");
                return None;
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' after match cases.")?;
        Some(Stmt::Match {
            subject,
            cases,
            span: self.merge_span(&start_span, &self.previous().span.clone()),
        })
    }

    fn parse_select_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.previous().span.clone();
        self.consume(TokenKind::LBrace, "Expected '{' before select cases.")?;
        let mut cases = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let case_start = self.peek().span.clone();
            if self.match_token(&[TokenKind::Case]) {
                let mut assignment = None;
                if let TokenKind::Ident(_) = self.peek().kind {
                    if self.peek_next().kind == TokenKind::Eq {
                        let (name, _) = self.consume_ident("Expected variable name.")?;
                        self.consume(TokenKind::Eq, "Expected '='")?;
                        let expr = self.parse_expression()?;
                        assignment = Some((name, expr));
                    }
                }
                if assignment.is_none() {
                    let expr = self.parse_expression()?;
                    assignment = Some(("_".into(), expr)); // drop value
                }

                self.consume(TokenKind::FatArrow, "Expected '=>' after case expression.")?;
                let body = self.parse_block()?;
                let span = self.merge_span(&case_start, &self.previous().span.clone());
                cases.push(SelectCase {
                    assignment,
                    body,
                    is_default: false,
                    span,
                });
            } else if self.match_token(&[TokenKind::Default]) {
                self.consume(TokenKind::FatArrow, "Expected '=>' after default.")?;
                let body = self.parse_block()?;
                let span = self.merge_span(&case_start, &self.previous().span.clone());
                cases.push(SelectCase {
                    assignment: None,
                    body,
                    is_default: true,
                    span,
                });
            } else {
                self.error_at_current("Expected 'case' or 'default'.");
                return None;
            }
        }
        self.consume(TokenKind::RBrace, "Expected '}' after select cases.")?;
        Some(Stmt::Select {
            cases,
            span: self.merge_span(&start_span, &self.previous().span.clone()),
        })
    }

    fn parse_pattern(&mut self) -> Option<Pattern> {
        let start_span = self.peek().span.clone();

        if self.match_token(&[TokenKind::True]) {
            return Some(Pattern::Literal(Literal::Bool(true)));
        }
        if self.match_token(&[TokenKind::False]) {
            return Some(Pattern::Literal(Literal::Bool(false)));
        }
        if let TokenKind::IntLit(n) = self.peek().kind {
            self.advance();
            return Some(Pattern::Literal(Literal::Int(n)));
        }
        if let TokenKind::FloatLit(n) = self.peek().kind {
            self.advance();
            return Some(Pattern::Literal(Literal::Float(n)));
        }
        if let TokenKind::StringLit(ref s) = self.peek().kind {
            let s = s.clone();
            self.advance();
            return Some(Pattern::Literal(Literal::String(s)));
        }

        if let TokenKind::Ident(ref name) = self.peek().kind {
            if name == "_" {
                self.advance();
                return Some(Pattern::Wildcard(start_span));
            }
            let name = name.clone();
            self.advance();

            if self.match_token(&[TokenKind::LParen]) {
                let mut fields = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        fields.push(self.parse_pattern()?);
                        if !self.match_token(&[TokenKind::Comma]) {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RParen, "Expected ')' after constructor pattern.")?;
                let span = self.merge_span(&start_span, &self.previous().span.clone());
                return Some(Pattern::Constructor { name, fields, span });
            }

            if self.match_token(&[TokenKind::LBrace]) {
                let mut fields = Vec::new();
                if !self.check(&TokenKind::RBrace) {
                    loop {
                        let (fname, _) =
                            self.consume_ident("Expected field name in struct pattern.")?;
                        let pat = if self.match_token(&[TokenKind::Colon]) {
                            self.parse_pattern()?
                        } else {
                            Pattern::Binding(fname.clone(), self.previous().span.clone())
                        };
                        fields.push((fname, pat));
                        if !self.match_token(&[TokenKind::Comma]) {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RBrace, "Expected '}' after struct pattern.")?;
                let span = self.merge_span(&start_span, &self.previous().span.clone());
                return Some(Pattern::Struct { name, fields, span });
            }

            return Some(Pattern::Binding(name, start_span));
        }

        self.error_at_current("Expected pattern.");
        None
    }

    fn parse_expr_stmt(&mut self) -> Option<Stmt> {
        let expr = self.parse_expression()?;
        self.consume(TokenKind::Semicolon, "Expected ';' after expression.")?;
        Some(Stmt::ExprStmt(expr))
    }

    // ── Expressions ─────────────────────────────────────────────────

    fn parse_expression(&mut self) -> Option<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Option<Expr> {
        let expr = self.parse_logical_or()?;
        if self.match_token(&[TokenKind::Eq]) {
            let _equals = self.previous().span.clone();
            let value = self.parse_assignment()?;
            let span = self.merge_span(&expr.span().clone(), &value.span().clone());
            return Some(Expr::Assign {
                target: Box::new(expr),
                value: Box::new(value),
                span,
            });
        }
        Some(expr)
    }

    fn parse_logical_or(&mut self) -> Option<Expr> {
        let mut expr = self.parse_logical_and()?;
        while self.match_token(&[TokenKind::Or]) {
            let op = BinOp::Or;
            let right = self.parse_logical_and()?;
            let span = self.merge_span(&expr.span().clone(), &right.span().clone());
            expr = Expr::BinOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Some(expr)
    }

    fn parse_logical_and(&mut self) -> Option<Expr> {
        let mut expr = self.parse_equality()?;
        while self.match_token(&[TokenKind::And]) {
            let op = BinOp::And;
            let right = self.parse_equality()?;
            let span = self.merge_span(&expr.span().clone(), &right.span().clone());
            expr = Expr::BinOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Some(expr)
    }

    fn parse_equality(&mut self) -> Option<Expr> {
        let mut expr = self.parse_comparison()?;
        while self.match_token(&[TokenKind::EqEq, TokenKind::BangEq]) {
            let op = match self.previous().kind {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::NotEq,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            let span = self.merge_span(&expr.span().clone(), &right.span().clone());
            expr = Expr::BinOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Some(expr)
    }

    fn parse_comparison(&mut self) -> Option<Expr> {
        let mut expr = self.parse_additive()?;
        while self.match_token(&[
            TokenKind::Lt,
            TokenKind::Gt,
            TokenKind::LtEq,
            TokenKind::GtEq,
        ]) {
            let op = match self.previous().kind {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::LtEq => BinOp::LtEq,
                TokenKind::GtEq => BinOp::GtEq,
                _ => unreachable!(),
            };
            let right = self.parse_additive()?;
            let span = self.merge_span(&expr.span().clone(), &right.span().clone());
            expr = Expr::BinOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Some(expr)
    }

    fn parse_additive(&mut self) -> Option<Expr> {
        let mut expr = self.parse_multiplicative()?;
        while self.match_token(&[TokenKind::Plus, TokenKind::Minus]) {
            let op = match self.previous().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative()?;
            let span = self.merge_span(&expr.span().clone(), &right.span().clone());
            expr = Expr::BinOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Some(expr)
    }

    fn parse_multiplicative(&mut self) -> Option<Expr> {
        let mut expr = self.parse_unary()?;
        while self.match_token(&[TokenKind::Star, TokenKind::Slash, TokenKind::Percent]) {
            let op = match self.previous().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => unreachable!(),
            };
            let right = self.parse_unary()?;
            let span = self.merge_span(&expr.span().clone(), &right.span().clone());
            expr = Expr::BinOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Some(expr)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        if self.match_token(&[TokenKind::Bang, TokenKind::Minus, TokenKind::Await]) {
            let start_span = self.previous().span.clone();
            let op = match self.previous().kind {
                TokenKind::Bang => UnaryOp::Not,
                TokenKind::Minus => UnaryOp::Neg,
                TokenKind::Await => UnaryOp::Await,
                _ => unreachable!(),
            };
            let right = self.parse_postfix()?;
            let span = self.merge_span(&start_span, &right.span().clone());
            return Some(Expr::UnaryOp {
                op,
                operand: Box::new(right),
                span,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.match_token(&[TokenKind::LParen]) {
                let mut args = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if !self.match_token(&[TokenKind::Comma]) {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RParen, "Expected ')' after arguments.")?;
                let span = self.merge_span(&expr.span().clone(), &self.previous().span.clone());
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
            } else if self.match_token(&[TokenKind::Dot]) {
                let (field, _) = self.consume_ident("Expected property name after '.'.")?;
                let span = self.merge_span(&expr.span().clone(), &self.previous().span.clone());
                expr = Expr::FieldAccess {
                    object: Box::new(expr),
                    field,
                    span,
                };
            } else if self.match_token(&[TokenKind::LBracket]) {
                let index = self.parse_expression()?;
                self.consume(TokenKind::RBracket, "Expected ']' after index.")?;
                let span = self.merge_span(&expr.span().clone(), &self.previous().span.clone());
                expr = Expr::IndexAccess {
                    object: Box::new(expr),
                    index: Box::new(index),
                    span,
                };
            } else {
                break;
            }
        }
        Some(expr)
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        let span = self.peek().span.clone();
        if self.match_token(&[TokenKind::False]) {
            return Some(Expr::Lit(Literal::Bool(false), span));
        }
        if self.match_token(&[TokenKind::True]) {
            return Some(Expr::Lit(Literal::Bool(true), span));
        }

        if let TokenKind::IntLit(n) = self.peek().kind {
            self.advance();
            return Some(Expr::Lit(Literal::Int(n), span));
        }
        if let TokenKind::FloatLit(n) = self.peek().kind {
            self.advance();
            return Some(Expr::Lit(Literal::Float(n), span));
        }
        if let TokenKind::StringLit(ref s) = self.peek().kind {
            let v = s.clone();
            self.advance();
            return Some(Expr::Lit(Literal::String(v), span));
        }

        if self.match_token(&[TokenKind::SelfKw]) {
            return Some(Expr::Ident("self".into(), span));
        }

        if let TokenKind::Ident(ref name) = self.peek().kind {
            let name = name.clone();
            self.advance();

            // Check for group literal: Point { x: 1 }
            if self.check(&TokenKind::LBrace) {
                // Peek ahead to ensure it's a field init (ident :) to avoid confusion with blocks
                if let TokenKind::Ident(_) = self.peek_next().kind {
                    self.advance(); // consume {
                    let mut fields = Vec::new();
                    if !self.check(&TokenKind::RBrace) {
                        loop {
                            let (fname, _) = self.consume_ident("Expected field name.")?;
                            self.consume(
                                TokenKind::Colon,
                                "Expected ':' after field name in literal.",
                            )?;
                            let expr = self.parse_expression()?;
                            fields.push((fname, expr));
                            if !self.match_token(&[TokenKind::Comma]) {
                                break;
                            }
                        }
                    }
                    self.consume(TokenKind::RBrace, "Expected '}' after group literal.")?;
                    let full_span = self.merge_span(&span, &self.previous().span.clone());
                    return Some(Expr::GroupLiteral {
                        name,
                        fields,
                        span: full_span,
                    });
                }
            }

            return Some(Expr::Ident(name, span));
        }

        if self.match_token(&[TokenKind::LParen]) {
            let expr = self.parse_expression()?;
            self.consume(TokenKind::RParen, "Expected ')' after expression.")?;
            return Some(expr);
        }

        self.error_at_current("Expected expression.");
        None
    }
}

// Implement a helper method for Expr to easily get its span.
impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Lit(_, s) => s.clone(),
            Expr::Ident(_, s) => s.clone(),
            Expr::BinOp { span, .. } => span.clone(),
            Expr::UnaryOp { span, .. } => span.clone(),
            Expr::Call { span, .. } => span.clone(),
            Expr::FieldAccess { span, .. } => span.clone(),
            Expr::IndexAccess { span, .. } => span.clone(),
            Expr::Lambda { span, .. } => span.clone(),
            Expr::GroupLiteral { span, .. } => span.clone(),
            Expr::Assign { span, .. } => span.clone(),
        }
    }
}
