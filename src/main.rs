mod chunk;
mod common;
mod compiler;
mod debug;
mod scanner;
mod vm;

use std::fs;

use crate::compiler::compile;
use crate::debug::diassemble;
use crate::vm::VirtualMachine;

fn main() {
    let source = fs::read("./main.cia").unwrap();
    let mut vm = VirtualMachine::new();
    let compliation_result = compile(source);
    if let Ok(chunk) = compliation_result {
        diassemble(&chunk);
        vm.interpret(&chunk);
    }
}
