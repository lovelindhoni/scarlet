use crate::heap::{Heap, HeapKey, Object};

#[derive(Debug, Copy, Clone)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
    Object(HeapKey),
}

impl Value {
    pub fn display(&self, heap: &Heap) -> String {
        match self {
            Value::Number(num) => num.to_string(),
            Value::Boolean(boolean) => boolean.to_string(),
            Value::Nil => "nil".to_string(),
            Value::Object(heap_key) => {
                let object = heap.arena.get(*heap_key).unwrap();
                match object {
                    Object::String(string) => string.to_owned(),
                    Object::Function(function) => {
                        let fn_name = if let Some(fn_name_key) = function.name {
                            let object = heap.arena.get(fn_name_key).unwrap();
                            if let Object::String(fn_name) = object {
                                fn_name
                            } else {
                                unreachable!()
                            }
                        } else {
                            "<script>"
                        };
                        format!("fn {}()", fn_name)
                    }
                    Object::NativeFunction(_) => "<native fn>".to_string(),
                }
            }
        }
    }
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

            Instruction::Pop => "POP",
        }
    }
}
