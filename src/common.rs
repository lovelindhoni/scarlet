use slotmap::DefaultKey;

#[derive(Debug, Copy, Clone)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
    Object(DefaultKey),
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Constant(usize),
    DefineGlobal(usize),
    GetGlobal(usize),
    SetGlobal(usize),
    GetLocal(usize),
    SetLocal(usize),
    JumpIfFalse(usize),
    Jump(usize),
    Loop(usize),
    Call(usize),
    True,
    False,
    Nil,
    Return,
    Negate,
    Add,
    Subtract,
    Multiply,
    Modulo,
    Divide,
    Not,
    Equal,
    Greater,
    Less,
    Print,
    Pop,
}

impl Instruction {
    pub fn opcode(&self) -> &'static str {
        match self {
            Instruction::Constant(_) => "CONSTANT",
            Instruction::DefineGlobal(_) => "DEFINE_GLOBAL",
            Instruction::GetGlobal(_) => "GET_GLOBAL",
            Instruction::SetGlobal(_) => "SET_GLOBAL",
            Instruction::GetLocal(_) => "GET_LOCAL",
            Instruction::SetLocal(_) => "SET_LOCAL",

            Instruction::Jump(_) => "JUMP",
            Instruction::JumpIfFalse(_) => "JUMP_IF_FALSE",
            Instruction::Loop(_) => "LOOP",

            Instruction::Call(_) => "CALL",

            Instruction::True => "TRUE",
            Instruction::False => "FALSE",
            Instruction::Nil => "NIL",
            Instruction::Return => "RETURN",

            Instruction::Negate => "NEGATE",
            Instruction::Add => "ADD",
            Instruction::Subtract => "SUBTRACT",
            Instruction::Multiply => "MULTIPLY",
            Instruction::Modulo => "MODULO",
            Instruction::Divide => "DIVIDE",

            Instruction::Not => "NOT",
            Instruction::Equal => "EQUAL",
            Instruction::Greater => "GREATER",
            Instruction::Less => "LESS",

            Instruction::Print => "PRINT",
            Instruction::Pop => "POP",
        }
    }
}
