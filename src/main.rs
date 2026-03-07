mod chunk;
mod common;
mod compiler;
mod error;
mod heap;
mod scanner;
mod trace;
mod vm;

use mimalloc::MiMalloc;
use std::fs;
use std::process;

use crate::compiler::compile;
use crate::heap::Heap;
use crate::trace::diassemble;
use crate::vm::VirtualMachine;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    let source = match fs::read("./main.cia") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("IO Error: {}", e);
            process::exit(1);
        }
    };

    let mut heap = Heap::new();

    let function = match compile(source, &mut heap) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Compile Error: {}", e);
            process::exit(1);
        }
    };

    // if let Err(e) = diassemble(function, &heap) {
    //     eprintln!("Trace Error: {}", e);
    //     process::exit(1);
    // }

    let mut vm = VirtualMachine::new();
    if let Err(e) = vm.interpret(function, &mut heap) {
        eprintln!("Runtime Error: {}", e);
        process::exit(1);
    }
}
