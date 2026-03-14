use crate::ast::{BinOp, Expr, Stmt};
use crate::codegen::opcodes::{Chunk, Opcode};
use crate::runtime::Value;

pub struct Compiler {
    pub chunk: Chunk,
    locals: Vec<String>,
    scope_depth: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            chunk: Chunk::new(),
            locals: Vec::new(),
            scope_depth: 0,
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
        self.chunk.write(op, 0); // stub line
        idx
    }

    fn emit_jump(&mut self, op: Opcode) -> usize {
        self.emit(op)
    }

    fn patch_jump(&mut self, offset: usize) {
        let current = self.chunk.code.len();
        match self.chunk.code[offset] {
            Opcode::JumpIfFalse(ref mut target) | Opcode::Jump(ref mut target) => {
                *target = current;
            }
            _ => unreachable!(),
        }
    }

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
            Stmt::Let { name, value } => {
                self.compile_expr(value)?;
                if self.scope_depth > 0 {
                    self.locals.push(name.clone());
                } else {
                    let idx = self.chunk.add_constant(Value::Str(name.clone()));
                    self.emit(Opcode::DefineGlobal(idx));
                }
            }
            Stmt::While { cond, body } => {
                let loop_start = self.chunk.code.len();
                self.compile_expr(cond)?;
                let exit_jmp = self.emit_jump(Opcode::JumpIfFalse(0));

                self.compile(body)?;
                self.emit(Opcode::Loop(loop_start));
                self.patch_jump(exit_jmp);
            }
            // Add other statements slowly
            Stmt::Return(e) => {
                if let Some(ex) = e {
                    self.compile_expr(ex)?;
                } else {
                    self.emit(Opcode::Null);
                }
                self.emit(Opcode::Return);
            }
            // Add other statements slowly
            _ => return Err(format!("Unimplemented statement compiler: {:?}", stmt)),
        }
        Ok(())
    }

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
            Expr::Ident(name) => {
                // local check
                if let Some(i) = self.locals.iter().rposition(|l| l == name) {
                    self.emit(Opcode::GetLocal(i));
                } else {
                    let idx = self.chunk.add_constant(Value::Str(name.clone()));
                    self.emit(Opcode::GetGlobal(idx));
                }
            }
            Expr::BinOp { op, left, right } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                match op {
                    BinOp::Add => {
                        self.emit(Opcode::Add);
                    }
                    BinOp::Sub => {
                        self.emit(Opcode::Subtract);
                    }
                    BinOp::Mul => {
                        self.emit(Opcode::Multiply);
                    }
                    BinOp::Div => {
                        self.emit(Opcode::Divide);
                    }
                    BinOp::Mod => {
                        self.emit(Opcode::Modulus);
                    }
                    BinOp::Eq => {
                        self.emit(Opcode::Equal);
                    }
                    BinOp::Ne => {
                        self.emit(Opcode::NotEqual);
                    }
                    BinOp::Lt => {
                        self.emit(Opcode::Less);
                    }
                    BinOp::Le => {
                        self.emit(Opcode::LessEqual);
                    }
                    BinOp::Gt => {
                        self.emit(Opcode::Greater);
                    }
                    BinOp::Ge => {
                        self.emit(Opcode::GreaterEqual);
                    }
                    _ => {
                        self.emit(Opcode::Null);
                    }
                };
            }
            Expr::Call { func, args } => {
                self.compile_expr(func)?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit(Opcode::Call(args.len()));
            }
            Expr::If { cond, then, else_ } => {
                self.compile_expr(cond)?;
                let jump_if_false = self.emit_jump(Opcode::JumpIfFalse(0));
                self.compile(then)?;
                let jump_end = self.emit_jump(Opcode::Jump(0));
                self.patch_jump(jump_if_false);
                if let Some(e) = else_ {
                    self.compile(e)?;
                } else {
                    self.emit(Opcode::Null); // `if` expression returns null if no else block and condition is false
                }
                self.patch_jump(jump_end);
            }
            _ => return Err(format!("Unimplemented expression compiler: {:?}", expr)),
        }
        Ok(())
    }
}
