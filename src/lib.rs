mod chunk;
mod common;
mod compiler;
mod error;
mod heap;
mod log;
mod native_fns;
mod scanner;
mod vm;

use rapidhash::RapidHashMap;

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
    let mut globals_map = RapidHashMap::default();
    initialize_native_functions(&mut heap, &mut globals_map);

    initialize_native_functions(&mut heap, &mut globals_map);
    let function = compile(source, &mut globals_map, &mut heap)
        .map_err(|e| JsValue::from_str(&format!("Compile Error: {}", e)))?;
    let mut vm = VirtualMachine::new();
    vm.interpret(function, &mut heap)
        .map_err(|e| JsValue::from_str(&format!("Runtime Error: {}", e)))?;
    Ok(())
}
