use anyhow::{Context, Result as AnyhowResult, anyhow};

use crate::chunk::Chunk;
use crate::common::Instruction;

fn simple_instruction(idx: usize, chunk: &Chunk, opcode: &str) {
    if idx > 0 && chunk.get_line(idx - 1) == chunk.get_line(idx) {
        println!("{} | {}", idx, opcode);
    } else {
        println!("{} {} {}", idx, chunk.get_line(idx), opcode);
    }
}

pub fn diassemble(chunk: &Chunk) -> AnyhowResult<()> {
    println!("Disassembling Chunk: {}", chunk.name);
    if chunk.instructions.is_empty() {
        return Err(anyhow!("Chunk Empty"));
    }
    for idx in 0..chunk.instructions.len() {
        diassemble_instruction(chunk, idx)?;
    }
    Ok(())
}

pub fn diassemble_instruction(chunk: &Chunk, idx: usize) -> AnyhowResult<()> {
    let instruction = chunk
        .instructions
        .get(idx)
        .with_context(|| format!("Instruction not present on chunk in index: {}", idx))?;
    match instruction {
        Instruction::Negate => {
            simple_instruction(idx, &chunk, "NEGATE");
        }
        Instruction::Return => {
            simple_instruction(idx, &chunk, "RETURN");
        }
        Instruction::Add => {
            simple_instruction(idx, &chunk, "ADD");
        }
        Instruction::Subtract => {
            simple_instruction(idx, &chunk, "SUBTRACT");
        }
        Instruction::Multiply => {
            simple_instruction(idx, &chunk, "MULTIPLY");
        }
        Instruction::Divide => {
            simple_instruction(idx, &chunk, "DIVIDE");
        }
        Instruction::Modulo => {
            simple_instruction(idx, &chunk, "MODULO");
        }

        Instruction::Constant(pos) => {
            if idx > 0 && chunk.get_line(idx - 1) == chunk.get_line(idx) {
                println!("{} | CONSTANT {}'{:?}'", idx, pos, chunk.values[*pos]);
            } else {
                // pos = index in constants array
                println!(
                    "{} {} CONSTANT {}'{:?}'",
                    idx,
                    chunk.get_line(idx),
                    pos,
                    chunk.values[*pos]
                );
            }
        }
    }
    Ok(())
}
