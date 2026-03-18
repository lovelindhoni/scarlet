mod chunk;
mod common;
mod compiler;
mod error;
mod heap;
mod log;
mod native_fns;
mod scanner;
mod vm;

use crate::compiler::compile;
use crate::heap::Heap;
use crate::native_fns::initialize_native_functions;
use crate::vm::VirtualMachine;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn interpret(source: String) -> Result<(), wasm_bindgen::JsValue> {
    let source = source.as_bytes().to_vec();
    let mut heap = Heap::new();
    initialize_native_functions(&mut heap);
    let function = compile(source, &mut heap)
        .map_err(|e| JsValue::from_str(&format!("Compile Error: {}", e)))?;
    let mut vm = VirtualMachine::new();
    vm.interpret(function, &mut heap)
        .map_err(|e| JsValue::from_str(&format!("Runtime Error: {}", e)))?;
    Ok(())
}
