use crate::ast::{BinOp, Expr, Lhs, MatchPat, Stmt, UnOp, UnpackItem};
use crate::codegen::opcodes::{Chunk, Opcode};
use crate::lexer::{FsPart, Lexer};
use crate::parser::Parser;
use crate::runtime::Value;

pub struct Compiler {
    pub chunk: Chunk,
    locals: Vec<String>,
    scope_depth: usize,
    loop_starts: Vec<usize>,
    loop_exit_jumps: Vec<Vec<usize>>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            chunk: Chunk::new(),
            locals: Vec::new(),
            scope_depth: 0,
            loop_starts: Vec::new(),
            loop_exit_jumps: Vec::new(),
        }
    }

    pub fn compile(&mut self, stmts: &[Stmt]) -> Result<(), String> {
        for s in stmts {
            self.compile_stmt(s)?;
        }
        Ok(())
    }

    fn emit(&mut self, op: Opcode) -> usize {
        let idx = self.chunk.code.len();
        self.chunk.write(op, 0);
        idx
    }

    fn emit_jump(&mut self, op: Opcode) -> usize {
        self.emit(op)
    }

    fn patch_jump(&mut self, offset: usize) {
        let current = self.chunk.code.len();
        match self.chunk.code[offset] {
            Opcode::JumpIfFalse(ref mut target)
            | Opcode::JumpIfTrue(ref mut target)
            | Opcode::JumpIfNull(ref mut target)
            | Opcode::Jump(ref mut target)
            | Opcode::BeginTry(ref mut target) => {
                *target = current;
            }
            _ => unreachable!(),
        }
    }

    // ── Statement Compiler ──────────────────────────────────────────

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(e) => {
                self.compile_expr(e)?;
                self.emit(Opcode::Pop);
            }
            Stmt::Print(e) => {
                self.compile_expr(e)?;
                self.emit(Opcode::Print);
            }
            Stmt::Write(e) => {
                self.compile_expr(e)?;
                // Use a builtin call for write (print without newline)
                let name_idx = self.chunk.add_constant(Value::Str("write".into()));
                self.emit(Opcode::GetGlobal(name_idx));
                self.emit(Opcode::Swap);
                self.emit(Opcode::Call(1));
                self.emit(Opcode::Pop);
            }
            Stmt::Let { name, value } => {
                self.compile_expr(value)?;
                if self.scope_depth > 0 {
                    self.locals.push(name.clone());
                } else {
                    let idx = self.chunk.add_constant(Value::Str(name.clone()));
                    self.emit(Opcode::DefineGlobal(idx));
                }
            }
            Stmt::LetList { items, value } => {
                self.compile_expr(value)?;
                let mut named_count = 0;
                for (i, item) in items.iter().enumerate() {
                    match item {
                        UnpackItem::Name(name) => {
                            self.emit(Opcode::Dup);
                            let idx_const = self.chunk.add_constant(Value::Int(i as i64));
                            self.emit(Opcode::Constant(idx_const));
                            self.emit(Opcode::IndexGet);
                            if self.scope_depth > 0 {
                                self.locals.push(name.clone());
                            } else {
                                let name_idx =
                                    self.chunk.add_constant(Value::Str(name.clone()));
                                self.emit(Opcode::DefineGlobal(name_idx));
                            }
                            named_count += 1;
                        }
                        UnpackItem::Rest(name) => {
                            // Slice from current index to end
                            // Use a builtin __slice(list, start)
                            self.emit(Opcode::Dup);
                            let slice_name =
                                self.chunk.add_constant(Value::Str("__slice".into()));
                            self.emit(Opcode::GetGlobal(slice_name));
                            self.emit(Opcode::Swap);
                            let start_idx =
                                self.chunk.add_constant(Value::Int(named_count as i64));
                            self.emit(Opcode::Constant(start_idx));
                            self.emit(Opcode::Call(2));
                            if self.scope_depth > 0 {
                                self.locals.push(name.clone());
                            } else {
                                let name_idx =
                                    self.chunk.add_constant(Value::Str(name.clone()));
                                self.emit(Opcode::DefineGlobal(name_idx));
                            }
                        }
                    }
                }
                self.emit(Opcode::Pop); // pop the original list
            }
            Stmt::LetMap { names, value } => {
                self.compile_expr(value)?;
                for name in names {
                    self.emit(Opcode::Dup);
                    let name_const = self.chunk.add_constant(Value::Str(name.clone()));
                    self.emit(Opcode::Constant(name_const));
                    self.emit(Opcode::IndexGet);
                    if self.scope_depth > 0 {
                        self.locals.push(name.clone());
                    } else {
                        let name_idx = self.chunk.add_constant(Value::Str(name.clone()));
                        self.emit(Opcode::DefineGlobal(name_idx));
                    }
                }
                self.emit(Opcode::Pop);
            }
            Stmt::Assign { target, value } => {
                self.compile_assign(target, value)?;
            }
            Stmt::CompoundAssign { target, op, value } => {
                self.compile_compound_assign(target, op, value)?;
            }
            Stmt::While { cond, body } => {
                let loop_start = self.chunk.code.len();
                self.loop_starts.push(loop_start);
                self.loop_exit_jumps.push(Vec::new());

                self.compile_expr(cond)?;
                let exit_jmp = self.emit_jump(Opcode::JumpIfFalse(0));

                let body_locals = self.locals.len();
                self.compile(body)?;
                // Pop any locals created in body
                while self.locals.len() > body_locals {
                    self.locals.pop();
                    self.emit(Opcode::Pop);
                }

                self.emit(Opcode::Loop(loop_start));
                self.patch_jump(exit_jmp);

                self.loop_starts.pop();
                let exits = self.loop_exit_jumps.pop().unwrap_or_default();
                for e in exits {
                    self.patch_jump(e);
                }
            }
            Stmt::For { var, iter, body } => {
                self.compile_expr(iter)?;
                let iter_loc = self.locals.len();
                self.locals.push("__iter".into());

                let zero_idx = self.chunk.add_constant(Value::Int(0));
                self.emit(Opcode::Constant(zero_idx));
                let i_loc = self.locals.len();
                self.locals.push("__i".into());

                let loop_start = self.chunk.code.len();
                self.loop_starts.push(loop_start);
                self.loop_exit_jumps.push(Vec::new());

                // len(__iter)
                let len_idx = self.chunk.add_constant(Value::Str("len".into()));
                self.emit(Opcode::GetGlobal(len_idx));
                self.emit(Opcode::GetLocal(iter_loc));
                self.emit(Opcode::Call(1));
                // __i < len
                self.emit(Opcode::GetLocal(i_loc));
                self.emit(Opcode::Swap);
                self.emit(Opcode::Less);

                let exit_jmp = self.emit_jump(Opcode::JumpIfFalse(0));

                // var = __iter[__i]
                self.emit(Opcode::GetLocal(iter_loc));
                self.emit(Opcode::GetLocal(i_loc));
                self.emit(Opcode::IndexGet);
                self.locals.push(var.clone());

                let body_locals = self.locals.len();
                self.compile(body)?;
                // Pop any locals created in body
                while self.locals.len() > body_locals {
                    self.locals.pop();
                    self.emit(Opcode::Pop);
                }

                // pop loop var
                self.locals.pop();
                self.emit(Opcode::Pop);

                // __i += 1
                self.emit(Opcode::GetLocal(i_loc));
                let one_idx = self.chunk.add_constant(Value::Int(1));
                self.emit(Opcode::Constant(one_idx));
                self.emit(Opcode::Add);
                self.emit(Opcode::SetLocal(i_loc));
                self.emit(Opcode::Pop);

                self.emit(Opcode::Loop(loop_start));
                self.patch_jump(exit_jmp);

                self.loop_starts.pop();
                let exits = self.loop_exit_jumps.pop().unwrap_or_default();
                for e in exits {
                    self.patch_jump(e);
                }

                // pop __i and __iter
                self.locals.pop();
                self.locals.pop();
                self.emit(Opcode::Pop);
                self.emit(Opcode::Pop);
            }
            Stmt::Break => {
                // Jump to end of loop — patched when loop ends
                let jmp = self.emit_jump(Opcode::Jump(0));
                if let Some(exits) = self.loop_exit_jumps.last_mut() {
                    exits.push(jmp);
                }
            }
            Stmt::Continue => {
                if let Some(&start) = self.loop_starts.last() {
                    self.emit(Opcode::Loop(start));
                }
            }
            Stmt::Return(e) => {
                if let Some(ex) = e {
                    self.compile_expr(ex)?;
                } else {
                    self.emit(Opcode::Null);
                }
                self.emit(Opcode::Return);
            }
            Stmt::Throw(e) => {
                self.compile_expr(e)?;
                self.emit(Opcode::Throw);
            }
            Stmt::TryCatch {
                body,
                catch_var,
                catch_body,
            } => {
                let try_jump = self.emit_jump(Opcode::BeginTry(0));
                self.compile(body)?;
                self.emit(Opcode::EndTry);
                let catch_jump = self.emit_jump(Opcode::Jump(0));

                self.patch_jump(try_jump);
                if self.scope_depth > 0 {
                    self.locals.push(catch_var.clone());
                } else {
                    let idx = self.chunk.add_constant(Value::Str(catch_var.clone()));
                    self.emit(Opcode::DefineGlobal(idx));
                }

                self.compile(catch_body)?;
                self.patch_jump(catch_jump);
            }
            Stmt::Import { path } => {
                let idx = self.chunk.add_constant(Value::Str(path.clone()));
                self.emit(Opcode::Constant(idx));
                self.emit(Opcode::Import);
            }
            Stmt::FnDef {
                name,
                params,
                variadic,
                body,
            } => {
                let mut fn_compiler = Compiler::new();
                fn_compiler.scope_depth = 1;
                for p in params {
                    fn_compiler.locals.push(p.clone());
                }
                if let Some(v) = variadic {
                    fn_compiler.locals.push(v.clone());
                }

                fn_compiler.compile(body)?;
                fn_compiler.emit(Opcode::Null);
                fn_compiler.emit(Opcode::Return);

                let val = Value::Fn {
                    fname: Some(name.clone()),
                    params: params.clone(),
                    variadic: variadic.clone(),
                    chunk: fn_compiler.chunk,
                    captures: std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::new())),
                };

                let idx = self.chunk.add_constant(val);
                self.emit(Opcode::Constant(idx));

                let current_locals = self.locals.clone();
                for (i, local_name) in current_locals.iter().enumerate() {
                    let name_idx = self.chunk.add_constant(Value::Str(local_name.clone()));
                    self.emit(Opcode::Constant(name_idx));
                    self.emit(Opcode::GetLocal(i));
                    self.emit(Opcode::CaptureLocal);
                }

                if self.scope_depth > 0 {
                    self.locals.push(name.clone());
                } else {
                    let name_idx = self.chunk.add_constant(Value::Str(name.clone()));
                    self.emit(Opcode::DefineGlobal(name_idx));
                }
            }
            Stmt::Match { subject, arms } => {
                self.compile_expr(subject)?;
                let mut end_jumps = Vec::new();

                for arm in arms {
                    match &arm.pattern {
                        MatchPat::Wildcard => {
                            self.emit(Opcode::Pop); // consume subject
                            self.compile(&arm.body)?;
                        }
                        MatchPat::Binding(name) => {
                            // Bind subject to name
                            if self.scope_depth > 0 {
                                self.locals.push(name.clone());
                            } else {
                                let idx =
                                    self.chunk.add_constant(Value::Str(name.clone()));
                                self.emit(Opcode::DefineGlobal(idx));
                            }
                            self.compile(&arm.body)?;
                        }
                        MatchPat::Literal(lit) => {
                            self.emit(Opcode::Dup);
                            self.compile_expr(lit)?;
                            self.emit(Opcode::Equal);
                            let skip = self.emit_jump(Opcode::JumpIfFalse(0));
                            self.emit(Opcode::Pop); // pop subject
                            self.compile(&arm.body)?;
                            let end = self.emit_jump(Opcode::Jump(0));
                            end_jumps.push(end);
                            self.patch_jump(skip);
                        }
                        MatchPat::Range(lo, hi) => {
                            // subject >= lo && subject <= hi
                            self.emit(Opcode::Dup);
                            self.compile_expr(lo)?;
                            self.emit(Opcode::GreaterEqual);
                            let skip_lo = self.emit_jump(Opcode::JumpIfFalse(0));
                            self.emit(Opcode::Dup);
                            self.compile_expr(hi)?;
                            self.emit(Opcode::LessEqual);
                            let skip_hi = self.emit_jump(Opcode::JumpIfFalse(0));
                            self.emit(Opcode::Pop); // pop subject
                            self.compile(&arm.body)?;
                            let end = self.emit_jump(Opcode::Jump(0));
                            end_jumps.push(end);
                            self.patch_jump(skip_lo);
                            self.patch_jump(skip_hi);
                        }
                    }
                }

                for j in end_jumps {
                    self.patch_jump(j);
                }
            }
        }
        Ok(())
    }

    // ── Assignment Helpers ──────────────────────────────────────────

    fn compile_assign(&mut self, target: &Lhs, value: &Expr) -> Result<(), String> {
        match target {
            Lhs::Ident(name) => {
                self.compile_expr(value)?;
                if let Some(i) = self.locals.iter().rposition(|l| l == name) {
                    self.emit(Opcode::SetLocal(i));
                } else {
                    let idx = self.chunk.add_constant(Value::Str(name.clone()));
                    self.emit(Opcode::SetGlobal(idx));
                }
                self.emit(Opcode::Pop);
            }
            Lhs::Index { obj, idx } => {
                self.compile_expr(obj)?;
                self.compile_expr(idx)?;
                self.compile_expr(value)?;
                self.emit(Opcode::IndexSet);
            }
            Lhs::Field { obj, name } => {
                self.compile_expr(obj)?;
                self.compile_expr(value)?;
                let name_idx = self.chunk.add_constant(Value::Str(name.clone()));
                self.emit(Opcode::FieldSet(name_idx));
            }
        }
        Ok(())
    }

    fn compile_compound_assign(
        &mut self,
        target: &Lhs,
        op: &BinOp,
        value: &Expr,
    ) -> Result<(), String> {
        match target {
            Lhs::Ident(name) => {
                // get current value
                if let Some(i) = self.locals.iter().rposition(|l| l == name) {
                    self.emit(Opcode::GetLocal(i));
                } else {
                    let idx = self.chunk.add_constant(Value::Str(name.clone()));
                    self.emit(Opcode::GetGlobal(idx));
                }
                self.compile_expr(value)?;
                self.emit_binop(op);
                // set back
                if let Some(i) = self.locals.iter().rposition(|l| l == name) {
                    self.emit(Opcode::SetLocal(i));
                } else {
                    let idx = self.chunk.add_constant(Value::Str(name.clone()));
                    self.emit(Opcode::SetGlobal(idx));
                }
                self.emit(Opcode::Pop);
            }
            Lhs::Index { obj, idx } => {
                // obj[idx] op= value  →  obj[idx] = obj[idx] op value
                self.compile_expr(obj)?;
                self.compile_expr(idx)?;
                self.emit(Opcode::Dup); // dup key
                // get obj[idx]
                // We need obj on stack too, so let's use a different strategy
                // Actually compile: obj, idx are on stack
                // We need to: get obj[idx], compute op value, set obj[idx] = result
                // Stack: [obj, idx]
                // Dup both: we can't easily. Use Swap/Dup:
                // Better approach: compile obj and idx twice
                self.emit(Opcode::Pop); // pop dup
                self.emit(Opcode::Pop); // pop idx
                self.emit(Opcode::Pop); // pop obj
                // Recompile for get
                self.compile_expr(obj)?;
                self.compile_expr(idx)?;
                self.emit(Opcode::IndexGet);
                self.compile_expr(value)?;
                self.emit_binop(op);
                // Now set: need obj, idx, result on stack
                // Result is on top. Recompile obj and idx under it.
                self.compile_expr(obj)?;
                self.emit(Opcode::Swap);
                self.compile_expr(idx)?;
                self.emit(Opcode::Swap);
                self.emit(Opcode::IndexSet);
            }
            Lhs::Field { obj, name } => {
                self.compile_expr(obj)?;
                let name_idx = self.chunk.add_constant(Value::Str(name.clone()));
                self.emit(Opcode::FieldGet(name_idx));
                self.compile_expr(value)?;
                self.emit_binop(op);
                self.compile_expr(obj)?;
                self.emit(Opcode::Swap);
                let name_idx2 = self.chunk.add_constant(Value::Str(name.clone()));
                self.emit(Opcode::FieldSet(name_idx2));
            }
        }
        Ok(())
    }

    fn emit_binop(&mut self, op: &BinOp) {
        match op {
            BinOp::Add => { self.emit(Opcode::Add); }
            BinOp::Sub => { self.emit(Opcode::Subtract); }
            BinOp::Mul => { self.emit(Opcode::Multiply); }
            BinOp::Div => { self.emit(Opcode::Divide); }
            BinOp::Mod => { self.emit(Opcode::Modulus); }
            BinOp::Pow => { self.emit(Opcode::Power); }
            BinOp::IDiv => { self.emit(Opcode::IntDivide); }
            _ => { self.emit(Opcode::Add); } // fallback (shouldn't happen for compound)
        }
    }

    // ── Expression Compiler ─────────────────────────────────────────

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Int(n) => {
                let idx = self.chunk.add_constant(Value::Int(*n));
                self.emit(Opcode::Constant(idx));
            }
            Expr::Float(n) => {
                let idx = self.chunk.add_constant(Value::Float(*n));
                self.emit(Opcode::Constant(idx));
            }
            Expr::Str(s) => {
                let idx = self.chunk.add_constant(Value::Str(s.clone()));
                self.emit(Opcode::Constant(idx));
            }
            Expr::Bool(b) => {
                self.emit(if *b { Opcode::True } else { Opcode::False });
            }
            Expr::Null => {
                self.emit(Opcode::Null);
            }
            Expr::List(elements) => {
                for el in elements {
                    self.compile_expr(el)?;
                }
                self.emit(Opcode::BuildList(elements.len()));
            }
            Expr::Map(pairs) => {
                for (k, v) in pairs {
                    self.compile_expr(k)?;
                    self.compile_expr(v)?;
                }
                self.emit(Opcode::BuildMap(pairs.len()));
            }
            Expr::Index { obj, idx } => {
                self.compile_expr(obj)?;
                self.compile_expr(idx)?;
                self.emit(Opcode::IndexGet);
            }
            Expr::Field { obj, name } => {
                self.compile_expr(obj)?;
                let name_idx = self.chunk.add_constant(Value::Str(name.clone()));
                self.emit(Opcode::FieldGet(name_idx));
            }
            Expr::Ident(name) => {
                if let Some(i) = self.locals.iter().rposition(|l| l == name) {
                    self.emit(Opcode::GetLocal(i));
                } else {
                    let idx = self.chunk.add_constant(Value::Str(name.clone()));
                    self.emit(Opcode::GetGlobal(idx));
                }
            }
            Expr::Unary { op, expr } => {
                self.compile_expr(expr)?;
                match op {
                    UnOp::Neg => { self.emit(Opcode::Negate); }
                    UnOp::Not => { self.emit(Opcode::Not); }
                }
            }
            Expr::BinOp { op, left, right } => {
                match op {
                    BinOp::And => {
                        // Short-circuit: if left is falsy, keep it as result
                        self.compile_expr(left)?;
                        self.emit(Opcode::Dup);
                        let jump = self.emit_jump(Opcode::JumpIfFalse(0));
                        self.emit(Opcode::Pop); // pop the dup (left was truthy)
                        self.compile_expr(right)?;
                        self.patch_jump(jump);
                    }
                    BinOp::Or => {
                        // Short-circuit: if left is truthy, keep it as result
                        self.compile_expr(left)?;
                        self.emit(Opcode::Dup);
                        let jump = self.emit_jump(Opcode::JumpIfTrue(0));
                        self.emit(Opcode::Pop); // pop the dup (left was falsy)
                        self.compile_expr(right)?;
                        self.patch_jump(jump);
                    }
                    BinOp::NullCoal => {
                        self.compile_expr(left)?;
                        let jump_null = self.emit_jump(Opcode::JumpIfNull(0));
                        let jump_end = self.emit_jump(Opcode::Jump(0));
                        self.patch_jump(jump_null);
                        self.compile_expr(right)?;
                        self.patch_jump(jump_end);
                    }
                    _ => {
                        // Normal binary ops: compile both operands first
                        self.compile_expr(left)?;
                        self.compile_expr(right)?;
                        match op {
                            BinOp::Add => { self.emit(Opcode::Add); }
                            BinOp::Sub => { self.emit(Opcode::Subtract); }
                            BinOp::Mul => { self.emit(Opcode::Multiply); }
                            BinOp::Div => { self.emit(Opcode::Divide); }
                            BinOp::Mod => { self.emit(Opcode::Modulus); }
                            BinOp::Pow => { self.emit(Opcode::Power); }
                            BinOp::IDiv => { self.emit(Opcode::IntDivide); }
                            BinOp::Eq => { self.emit(Opcode::Equal); }
                            BinOp::Ne => { self.emit(Opcode::NotEqual); }
                            BinOp::Lt => { self.emit(Opcode::Less); }
                            BinOp::Le => { self.emit(Opcode::LessEqual); }
                            BinOp::Gt => { self.emit(Opcode::Greater); }
                            BinOp::Ge => { self.emit(Opcode::GreaterEqual); }
                            _ => {} // And/Or/NullCoal handled above
                        }
                    }
                }
            }
            Expr::Call { func, args } => {
                self.compile_expr(func)?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit(Opcode::Call(args.len()));
            }
            Expr::Lambda {
                params,
                variadic,
                body,
            } => {
                let mut fn_compiler = Compiler::new();
                fn_compiler.scope_depth = 1;
                for p in params {
                    fn_compiler.locals.push(p.clone());
                }
                if let Some(v) = variadic {
                    fn_compiler.locals.push(v.clone());
                }

                fn_compiler.compile(body)?;
                fn_compiler.emit(Opcode::Null);
                fn_compiler.emit(Opcode::Return);

                let val = Value::Fn {
                    fname: None,
                    params: params.clone(),
                    variadic: variadic.clone(),
                    chunk: fn_compiler.chunk,
                    captures: std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::new())),
                };

                let idx = self.chunk.add_constant(val);
                self.emit(Opcode::Constant(idx));

                let current_locals = self.locals.clone();
                for (i, local_name) in current_locals.iter().enumerate() {
                    let name_idx = self.chunk.add_constant(Value::Str(local_name.clone()));
                    self.emit(Opcode::Constant(name_idx));
                    self.emit(Opcode::GetLocal(i));
                    self.emit(Opcode::CaptureLocal);
                }
            }
            Expr::FStr(parts) => {
                for (i, part) in parts.iter().enumerate() {
                    match part {
                        FsPart::Text(s) => {
                            let idx = self.chunk.add_constant(Value::Str(s.clone()));
                            self.emit(Opcode::Constant(idx));
                        }
                        FsPart::Code(code) => {
                            let str_idx = self.chunk.add_constant(Value::Str("str".into()));
                            self.emit(Opcode::GetGlobal(str_idx));
                            let toks = Lexer::new(code).tokenize()?;
                            let mut pars = Parser::new(toks);
                            let expr = pars.parse_expr()?;
                            self.compile_expr(&expr)?;
                            self.emit(Opcode::Call(1));
                        }
                    }
                    if i > 0 {
                        self.emit(Opcode::Add);
                    }
                }
            }
            Expr::If { cond, then, else_ } => {
                self.compile_expr(cond)?;
                let jump_if_false = self.emit_jump(Opcode::JumpIfFalse(0));
                self.compile(then)?;
                self.emit(Opcode::Null); // ensure then-branch pushes a value
                let jump_end = self.emit_jump(Opcode::Jump(0));
                self.patch_jump(jump_if_false);
                if let Some(e) = else_ {
                    self.compile(e)?;
                    self.emit(Opcode::Null); // ensure else-branch pushes a value
                } else {
                    self.emit(Opcode::Null);
                }
                self.patch_jump(jump_end);
            }
        }
        Ok(())
    }
}
