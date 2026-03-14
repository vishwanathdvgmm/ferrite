// SECTION 3 – PARSER
// ================================================================
use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<(Token, u32)>,
    pos: usize,
    current_line: u32,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, u32)>) -> Self {
        Parser {
            tokens,
            pos: 0,
            current_line: 1,
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].0
    }

    fn peek_tok_at(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.pos + offset)
            .map(|(t, _)| t)
            .unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) -> Token {
        let (t, line) = self.tokens[self.pos].clone();
        self.current_line = line;
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn check(&self, t: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(t)
    }

    fn expect(&mut self, t: &Token) -> Result<(), String> {
        if self.check(t) {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "line {}: Expected {:?}, got {:?}",
                self.current_line,
                t,
                self.peek()
            ))
        }
    }

    fn expect_ident(&mut self, ctx: &str) -> Result<String, String> {
        match self.advance() {
            Token::Ident(n) => Ok(n),
            t => Err(format!(
                "line {}: Expected identifier {}, got {:?}",
                self.current_line, ctx, t
            )),
        }
    }

    fn semi_after(&mut self, e: &Expr) -> Result<(), String> {
        if self.check(&Token::Semicolon) {
            self.advance();
            Ok(())
        } else if matches!(e, Expr::If { .. } | Expr::Lambda { .. }) {
            Ok(())
        } else if self.check(&Token::RBrace) {
            Ok(())
        } else {
            Err(format!(
                "line {}: Expected ';' after expression, got {:?}",
                self.current_line,
                self.peek()
            ))
        }
    }

    fn compound_op(tok: &Token) -> Option<BinOp> {
        match tok {
            Token::PlusEq => Some(BinOp::Add),
            Token::MinusEq => Some(BinOp::Sub),
            Token::StarEq => Some(BinOp::Mul),
            Token::SlashEq => Some(BinOp::Div),
            _ => None,
        }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut v = Vec::new();
        while self.peek() != &Token::EOF {
            v.push(self.parse_stmt()?);
        }
        Ok(v)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(&Token::LBrace)?;
        let mut v = Vec::new();
        while !self.check(&Token::RBrace) && self.peek() != &Token::EOF {
            v.push(self.parse_stmt()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(v)
    }

    fn parse_params(&mut self) -> Result<(Vec<String>, Option<String>), String> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        let mut variadic = None;
        while !self.check(&Token::RParen) {
            if self.check(&Token::DotDotDot) {
                self.advance();
                variadic = Some(self.expect_ident("after '...'")?);
                break;
            }
            params.push(self.expect_ident("in parameter list")?);
            if self.check(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;
        Ok((params, variadic))
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        // Two-token lookahead: name <op>= expr
        if let Token::Ident(name) = self.peek().clone() {
            if let Some(op) = Self::compound_op(self.peek_tok_at(1)) {
                self.advance();
                self.advance();
                let value = self.parse_expr()?;
                self.expect(&Token::Semicolon)?;
                return Ok(Stmt::CompoundAssign {
                    target: Lhs::Ident(name),
                    op,
                    value,
                });
            }
        }

        match self.peek().clone() {
            Token::Let => {
                self.advance();
                // let [a, b, ...rest] = expr
                if self.check(&Token::LBracket) {
                    self.advance();
                    let mut items = Vec::new();
                    while !self.check(&Token::RBracket) {
                        if self.check(&Token::DotDotDot) {
                            self.advance();
                            items.push(UnpackItem::Rest(self.expect_ident("after '...'")?));
                            break;
                        }
                        items.push(UnpackItem::Name(self.expect_ident("in list unpack")?));
                        if self.check(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RBracket)?;
                    self.expect(&Token::Eq)?;
                    let value = self.parse_expr()?;
                    self.expect(&Token::Semicolon)?;
                    return Ok(Stmt::LetList { items, value });
                }
                // let {a, b} = expr
                if self.check(&Token::LBrace) {
                    self.advance();
                    let mut names = Vec::new();
                    while !self.check(&Token::RBrace) {
                        names.push(self.expect_ident("in map unpack")?);
                        if self.check(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RBrace)?;
                    self.expect(&Token::Eq)?;
                    let value = self.parse_expr()?;
                    self.expect(&Token::Semicolon)?;
                    return Ok(Stmt::LetMap { names, value });
                }
                let name = self.expect_ident("after 'let'")?;
                self.expect(&Token::Eq)?;
                let value = self.parse_expr()?;
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::Let { name, value })
            }
            Token::Print => {
                self.advance();
                self.expect(&Token::LParen)?;
                let e = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::Print(e))
            }
            Token::Ident(ref s) if s == "write" => {
                self.advance();
                self.expect(&Token::LParen)?;
                let e = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::Write(e))
            }
            Token::Return => {
                self.advance();
                if self.check(&Token::Semicolon) {
                    self.advance();
                    return Ok(Stmt::Return(None));
                }
                let e = self.parse_expr()?;
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::Return(Some(e)))
            }
            Token::Throw => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::Throw(e))
            }
            Token::Break => {
                self.advance();
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::Break)
            }
            Token::Continue => {
                self.advance();
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::Continue)
            }
            Token::While => {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::While { cond, body })
            }
            Token::For => {
                self.advance();
                // for [a, b] in ... — desugar to for __item in ... { let [a,b] = __item; ... }
                if self.check(&Token::LBracket) {
                    self.advance();
                    let mut items = Vec::new();
                    while !self.check(&Token::RBracket) {
                        if self.check(&Token::DotDotDot) {
                            self.advance();
                            items.push(UnpackItem::Rest(self.expect_ident("after '...'")?));
                            break;
                        }
                        items.push(UnpackItem::Name(self.expect_ident("in for destructure")?));
                        if self.check(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RBracket)?;
                    self.expect(&Token::In)?;
                    let iter = self.parse_expr()?;
                    let mut body = self.parse_block()?;
                    // Prepend destructuring
                    body.insert(
                        0,
                        Stmt::LetList {
                            items,
                            value: Expr::Ident("__item".to_string()),
                        },
                    );
                    return Ok(Stmt::For {
                        var: "__item".to_string(),
                        iter,
                        body,
                    });
                }
                let var = self.expect_ident("in 'for'")?;
                self.expect(&Token::In)?;
                let iter = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::For { var, iter, body })
            }
            Token::Fn => {
                self.advance();
                let name = self.expect_ident("after 'fn'")?;
                let (params, variadic) = self.parse_params()?;
                let body = self.parse_block()?;
                Ok(Stmt::FnDef {
                    name,
                    params,
                    variadic,
                    body,
                })
            }
            Token::Match => {
                self.advance();
                let subject = self.parse_expr()?;
                self.expect(&Token::LBrace)?;
                let mut arms = Vec::new();
                while !self.check(&Token::RBrace) && self.peek() != &Token::EOF {
                    let pattern = self.parse_match_pat()?;
                    self.expect(&Token::Arrow)?;
                    let body = self.parse_block()?;
                    if self.check(&Token::Comma) {
                        self.advance();
                    }
                    arms.push(MatchArm { pattern, body });
                }
                self.expect(&Token::RBrace)?;
                Ok(Stmt::Match { subject, arms })
            }
            Token::Try => {
                self.advance();
                let body = self.parse_block()?;
                self.expect(&Token::Catch)?;
                let has_paren = self.check(&Token::LParen);
                if has_paren {
                    self.advance();
                }
                let catch_var = self.expect_ident("in catch")?;
                if has_paren {
                    self.expect(&Token::RParen)?;
                }
                let catch_body = self.parse_block()?;
                Ok(Stmt::TryCatch {
                    body,
                    catch_var,
                    catch_body,
                })
            }
            Token::Import => {
                self.advance();
                match self.advance() {
                    Token::Str(path) => {
                        self.expect(&Token::Semicolon)?;
                        Ok(Stmt::Import { path })
                    }
                    t => Err(format!(
                        "line {}: import expects a string path, got {:?}",
                        self.current_line, t
                    )),
                }
            }
            Token::Ident(name) => {
                let saved = self.pos;
                self.advance();

                if self.check(&Token::Eq) {
                    self.advance();
                    let value = self.parse_expr()?;
                    self.expect(&Token::Semicolon)?;
                    return Ok(Stmt::Assign {
                        target: Lhs::Ident(name),
                        value,
                    });
                }

                if self.check(&Token::LBracket) {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    if self.check(&Token::Eq) {
                        self.advance();
                        let value = self.parse_expr()?;
                        self.expect(&Token::Semicolon)?;
                        let obj = Box::new(Expr::Ident(name));
                        return Ok(Stmt::Assign {
                            target: Lhs::Index { obj, idx },
                            value,
                        });
                    }
                    if let Some(op) = Self::compound_op(self.peek()) {
                        self.advance();
                        let value = self.parse_expr()?;
                        self.expect(&Token::Semicolon)?;
                        let obj = Box::new(Expr::Ident(name));
                        return Ok(Stmt::CompoundAssign {
                            target: Lhs::Index { obj, idx },
                            op,
                            value,
                        });
                    }
                }

                self.pos = saved;
                let e = self.parse_expr()?;
                if self.check(&Token::Eq) {
                    self.advance();
                    let value = self.parse_expr()?;
                    self.expect(&Token::Semicolon)?;
                    let target = expr_to_lhs(e)?;
                    return Ok(Stmt::Assign { target, value });
                }
                if let Some(op) = Self::compound_op(self.peek()) {
                    self.advance();
                    let value = self.parse_expr()?;
                    self.expect(&Token::Semicolon)?;
                    let target = expr_to_lhs(e)?;
                    return Ok(Stmt::CompoundAssign { target, op, value });
                }
                self.semi_after(&e)?;
                Ok(Stmt::Expr(e))
            }
            _ => {
                let e = self.parse_expr()?;
                self.semi_after(&e)?;
                Ok(Stmt::Expr(e))
            }
        }
    }

    // Kept from user's v1.2 — uses parse_primary to avoid consuming `..` as Dot
    fn parse_match_pat(&mut self) -> Result<MatchPat, String> {
        if let Token::Ident(ref s) = self.peek().clone() {
            if s == "_" {
                self.advance();
                return Ok(MatchPat::Wildcard);
            }
        }
        let neg = if self.check(&Token::Minus) {
            self.advance();
            true
        } else {
            false
        };
        let mut e = self.parse_primary()?;
        if neg {
            e = Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(e),
            };
        }
        // Range: expr .. expr
        if self.check(&Token::DotDot) {
            self.advance();
            let neg2 = if self.check(&Token::Minus) {
                self.advance();
                true
            } else {
                false
            };
            let mut end = self.parse_primary()?;
            if neg2 {
                end = Expr::Unary {
                    op: UnOp::Neg,
                    expr: Box::new(end),
                };
            }
            return Ok(MatchPat::Range(e, end));
        }
        if let Expr::Ident(n) = &e {
            return Ok(MatchPat::Binding(n.clone()));
        }
        Ok(MatchPat::Literal(e))
    }

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_null_coal()
    }

    fn parse_null_coal(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_or()?;
        while self.check(&Token::QuestionQuestion) {
            self.advance();
            let r = self.parse_or()?;
            l = Expr::BinOp {
                op: BinOp::NullCoal,
                left: l.into(),
                right: r.into(),
            };
        }
        Ok(l)
    }
    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_and()?;
        while self.check(&Token::Or) {
            self.advance();
            let r = self.parse_and()?;
            l = Expr::BinOp {
                op: BinOp::Or,
                left: l.into(),
                right: r.into(),
            };
        }
        Ok(l)
    }
    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_eq()?;
        while self.check(&Token::And) {
            self.advance();
            let r = self.parse_eq()?;
            l = Expr::BinOp {
                op: BinOp::And,
                left: l.into(),
                right: r.into(),
            };
        }
        Ok(l)
    }
    fn parse_eq(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_cmp()?;
        loop {
            let op = match self.peek() {
                Token::EqEq => BinOp::Eq,
                Token::BangEq => BinOp::Ne,
                _ => break,
            };
            self.advance();
            let r = self.parse_cmp()?;
            l = Expr::BinOp {
                op,
                left: l.into(),
                right: r.into(),
            };
        }
        Ok(l)
    }
    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_add()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::LtEq => BinOp::Le,
                Token::Gt => BinOp::Gt,
                Token::GtEq => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let r = self.parse_add()?;
            l = Expr::BinOp {
                op,
                left: l.into(),
                right: r.into(),
            };
        }
        Ok(l)
    }
    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let r = self.parse_mul()?;
            l = Expr::BinOp {
                op,
                left: l.into(),
                right: r.into(),
            };
        }
        Ok(l)
    }
    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut l = self.parse_pow()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                Token::SlashSlash => BinOp::IDiv,
                _ => break,
            };
            self.advance();
            let r = self.parse_pow()?;
            l = Expr::BinOp {
                op,
                left: l.into(),
                right: r.into(),
            };
        }
        Ok(l)
    }
    fn parse_pow(&mut self) -> Result<Expr, String> {
        let base = self.parse_unary()?;
        if self.check(&Token::StarStar) {
            self.advance();
            let exp = self.parse_pow()?;
            Ok(Expr::BinOp {
                op: BinOp::Pow,
                left: base.into(),
                right: exp.into(),
            })
        } else {
            Ok(base)
        }
    }
    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::Minus => {
                self.advance();
                Ok(Expr::Unary {
                    op: UnOp::Neg,
                    expr: self.parse_unary()?.into(),
                })
            }
            Token::Bang => {
                self.advance();
                Ok(Expr::Unary {
                    op: UnOp::Not,
                    expr: self.parse_unary()?.into(),
                })
            }
            _ => self.parse_postfix(),
        }
    }
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_primary()?;
        loop {
            match self.peek() {
                Token::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(&Token::RParen) {
                        args.push(self.parse_expr()?);
                        if self.check(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RParen)?;
                    e = Expr::Call {
                        func: e.into(),
                        args,
                    };
                }
                Token::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    e = Expr::Index {
                        obj: e.into(),
                        idx: idx.into(),
                    };
                }
                Token::Dot => {
                    self.advance();
                    let name = self.expect_ident("after '.'")?;
                    if self.check(&Token::LParen) {
                        self.advance();
                        let mut args = vec![e];
                        while !self.check(&Token::RParen) {
                            args.push(self.parse_expr()?);
                            if self.check(&Token::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(&Token::RParen)?;
                        e = Expr::Call {
                            func: Expr::Ident(name).into(),
                            args,
                        };
                    } else {
                        e = Expr::Field {
                            obj: e.into(),
                            name,
                        };
                    }
                }
                _ => break,
            }
        }
        Ok(e)
    }
    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance() {
            Token::Int(n) => Ok(Expr::Int(n)),
            Token::Float(f) => Ok(Expr::Float(f)),
            Token::Str(s) => Ok(Expr::Str(s)),
            Token::Bool(b) => Ok(Expr::Bool(b)),
            Token::Null => Ok(Expr::Null),
            Token::Ident(n) => Ok(Expr::Ident(n)),
            Token::FStr(p) => Ok(Expr::FStr(p)),
            Token::LParen => {
                let e = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(e)
            }
            Token::LBracket => {
                let mut items = Vec::new();
                while !self.check(&Token::RBracket) {
                    items.push(self.parse_expr()?);
                    if self.check(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::List(items))
            }
            Token::LBrace => {
                let mut pairs = Vec::new();
                while !self.check(&Token::RBrace) {
                    let k = self.parse_expr()?;
                    self.expect(&Token::Colon)?;
                    let v = self.parse_expr()?;
                    pairs.push((k, v));
                    if self.check(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBrace)?;
                Ok(Expr::Map(pairs))
            }
            Token::If => self.parse_if_body(),
            Token::Fn => {
                let (params, variadic) = self.parse_params()?;
                let body = self.parse_block()?;
                Ok(Expr::Lambda {
                    params,
                    variadic,
                    body,
                })
            }
            tok => Err(format!(
                "line {}: Unexpected token: {:?}",
                self.current_line, tok
            )),
        }
    }
    fn parse_if_body(&mut self) -> Result<Expr, String> {
        let cond = self.parse_expr()?;
        let then = self.parse_block()?;
        let else_ = if self.check(&Token::Else) {
            self.advance();
            if self.check(&Token::If) {
                self.advance();
                let inner = self.parse_if_body()?;
                Some(vec![Stmt::Expr(inner)])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        Ok(Expr::If {
            cond: cond.into(),
            then,
            else_,
        })
    }
}

fn expr_to_lhs(e: Expr) -> Result<Lhs, String> {
    match e {
        Expr::Ident(n) => Ok(Lhs::Ident(n)),
        Expr::Index { obj, idx } => Ok(Lhs::Index { obj, idx: *idx }),
        Expr::Field { obj, name } => Ok(Lhs::Field { obj, name }),
        _ => Err("Invalid assignment target".into()),
    }
}

// ================================================================
// SECTION 4 – VALUES  &  SIGNALS
