mod chunk;
mod cli;
mod common;
mod compiler;
mod error;
mod heap;
mod log;
mod native_fns;
mod scanner;
mod trace;
mod vm;

use std::io::{self, Write};
use std::process;

use crate::cli::ScarletCli;
use crate::compiler::compile;
use crate::heap::Heap;
use crate::native_fns::initialize_native_functions;
use crate::trace::diassemble;
use crate::vm::VirtualMachine;

fn main() {
    let cli: ScarletCli = argh::from_env();

    let mut heap = Heap::new();
    initialize_native_functions(&mut heap);

    if cli.repl {
        run_repl(&mut heap, cli.debug);
    } else if let Some(script_path) = cli.run {
        let source = match std::fs::read(script_path) {
            Ok(source) => source,
            Err(e) => {
                eprintln!("IO Error: {}", e);
                process::exit(1);
            }
        };
        let function = match compile(source, &mut heap) {
            Ok(function) => function,
            Err(e) => {
                eprintln!("Compile Error: {}", e);
                process::exit(1);
            }
        };
        if cli.debug
            && let Err(e) = diassemble(function, &heap)
        {
            eprintln!("Trace Error: {}", e);
            process::exit(1);
        }
        let mut vm = VirtualMachine::new();
        if let Err(e) = vm.interpret(function, &mut heap) {
            eprintln!("Runtime Error: {}", e);
            process::exit(1);
        }
    } else if cli.version {
        println!("Scarlet {}", env!("CARGO_PKG_VERSION"));
    } else {
        run_repl(&mut heap, cli.debug);
    }
}

fn run_repl(heap: &mut Heap, debug_mode: bool) {
    println!(
        "Scarlet {} — REPL (Ctrl-D to exit)",
        env!("CARGO_PKG_VERSION")
    );

    loop {
        print!(">> ");
        if let Err(e) = io::stdout().flush() {
            eprintln!("IO Error: {e}");
            break;
        }
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(0) => {
                println!("Byie!");
                break;
            }
            Ok(_) => {
                let source = line.trim().to_owned().into_bytes();
                if source.is_empty() {
                    continue;
                }
                let function = match compile(source, heap) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Compile Error: {e}");
                        continue;
                    }
                };
                if debug_mode && let Err(e) = diassemble(function, heap) {
                    eprintln!("Trace Error: {e}");
                    continue;
                }
                if let Err(e) = VirtualMachine::new().interpret(function, heap) {
                    eprintln!("Runtime Error: {e}");
                }
            }
            Err(e) => {
                eprintln!("Input error: {e}");
                break;
            }
        }
    }
}
