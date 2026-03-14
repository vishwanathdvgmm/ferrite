use crate::runtime::Value;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Opcode {
    Constant(usize),
    Null,
    True,
    False,

    Pop,
    
    // Variables
    DefineGlobal(usize),
    GetGlobal(usize),
    SetGlobal(usize),
    GetLocal(usize),
    SetLocal(usize),

    // Control Flow
    JumpIfFalse(usize),
    Jump(usize),
    Loop(usize),

    // Operations
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
    IntDivide,
    Power,
    Not,
    Negate,

    // Collections
    BuildList(usize),
    BuildMap(usize),
    IndexGet,
    IndexSet,
    FieldGet(usize),
    FieldSet(usize),

    // Functions
    Call(usize),
    Return,

    Print,
    Throw,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<Opcode>,
    pub constants: Vec<Value>,
    pub lines: Vec<usize>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn write(&mut self, op: Opcode, line: usize) {
        self.code.push(op);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }
}
