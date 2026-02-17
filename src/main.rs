mod chunk;
mod common;
mod compiler;
mod debug;
mod scanner;
mod vm;

use std::fs;

use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::compiler::compile;
use crate::debug::diassemble;
use crate::vm::VirtualMachine;

fn main() {
    let source_bytes = fs::read("./main.cia").unwrap();
    compile(&source_bytes);
    // let mut vm = VirtualMachine::new();
    // let mut chunk = Chunk::new("Master");
    // chunk.write_constant(Value::Number(1.2), 132);
    // chunk.write_constant(Value::Number(3.4), 132);
    // chunk.write_instruction(Instruction::Add, 132);
    // chunk.write_constant(Value::Number(5.6), 132);
    // chunk.write_instruction(Instruction::Divide, 132);
    // chunk.write_instruction(Instruction::Negate, 132);
    // chunk.write_instruction(Instruction::Return, 132);
    // diassemble(&chunk);
    // vm.interpret(&chunk);
}
