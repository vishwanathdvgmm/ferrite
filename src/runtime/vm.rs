use crate::codegen::opcodes::{Chunk, Opcode};
use crate::runtime::Value;
use std::collections::HashMap;

pub struct VM {
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
    // For simplicity, globals are mapped by name via constant string pool
    globals: HashMap<String, Value>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            chunk: Chunk::new(),
            ip: 0,
            stack: Vec::with_capacity(256),
            globals: HashMap::new(),
        }
    }

    pub fn interpret(&mut self, chunk: Chunk) -> Result<Value, String> {
        self.chunk = chunk;
        self.ip = 0;
        self.run()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap_or(Value::Null)
    }

    fn run(&mut self) -> Result<Value, String> {
        loop {
            if self.ip >= self.chunk.code.len() {
                break;
            }
            let instruction = self.chunk.code[self.ip];
            self.ip += 1;

            match instruction {
                Opcode::Return => {
                    return Ok(self.pop());
                }
                Opcode::Constant(idx) => {
                    let constant = self.chunk.constants[idx].clone();
                    self.push(constant);
                }
                Opcode::Null => self.push(Value::Null),
                Opcode::True => self.push(Value::Bool(true)),
                Opcode::False => self.push(Value::Bool(false)),
                Opcode::Pop => {
                    self.pop();
                }
                Opcode::DefineGlobal(idx) => {
                    let name = match &self.chunk.constants[idx] {
                        Value::Str(s) => s.clone(),
                        _ => return Err("Global name must be string".into()),
                    };
                    let val = self.pop();
                    self.globals.insert(name, val);
                }
                Opcode::GetGlobal(idx) => {
                    let name = match &self.chunk.constants[idx] {
                        Value::Str(s) => s.clone(),
                        _ => return Err("Global name must be string".into()),
                    };
                    let val = self.globals.get(&name).cloned().unwrap_or(Value::Null);
                    self.push(val);
                }
                Opcode::GetLocal(idx) => {
                    let val = self.stack[idx].clone();
                    self.push(val);
                }
                Opcode::JumpIfFalse(offset) => {
                    let val = self.pop();
                    let falsy = match val {
                        Value::Bool(b) => !b,
                        Value::Null => true,
                        Value::Int(0) => true,
                        _ => false,
                    };
                    if falsy {
                        self.ip = offset;
                    }
                }
                Opcode::Jump(offset) => {
                    self.ip = offset;
                }
                Opcode::Loop(offset) => {
                    self.ip = offset;
                }
                Opcode::Add => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x + y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x + y)),
                        (Value::Str(x), Value::Str(y)) => self.push(Value::Str(x + &y)),
                        _ => return Err("Operands must be numbers or strings for Add".into()),
                    }
                }
                Opcode::Subtract => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x - y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x - y)),
                        _ => return Err("Operands must be numbers for Sub".into()),
                    }
                }
                Opcode::Multiply => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x * y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x * y)),
                        _ => return Err("Operands must be numbers for Mul".into()),
                    }
                }
                Opcode::Divide => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => {
                            if y == 0 {
                                return Err("Division by zero".into());
                            }
                            self.push(Value::Float((x as f64) / (y as f64)));
                        }
                        (Value::Float(x), Value::Float(y)) => {
                            self.push(Value::Float(x / y));
                        }
                        _ => return Err("Operands must be numbers for Div".into()),
                    }
                }
                Opcode::Print => {
                    let val = self.pop();
                    println!("{}", val);
                }
                _ => return Err(format!("Unimplemented runtime opcode: {:?}", instruction)),
            }
        }
        Ok(Value::Null)
    }
}
