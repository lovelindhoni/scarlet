use crate::chunk::Chunk;
use crate::common::Instruction;
use crate::error::TraceError;

type Result<T> = std::result::Result<T, TraceError>;

fn simple_instruction(idx: usize, chunk: &Chunk, instruction: &Instruction) {
    if idx > 0 && chunk.get_line(idx - 1) == chunk.get_line(idx) {
        println!("{:04} | {:<16}", idx, instruction.opcode());
    } else {
        println!(
            "{:04} {:4} {:<16}",
            idx,
            chunk.get_line(idx),
            instruction.opcode()
        );
    }
}

fn constant_instruction(idx: usize, chunk: &Chunk, pos: &usize, instruction: &Instruction) {
    // TODO: need to account for objects properly here
    if idx > 0 && chunk.get_line(idx - 1) == chunk.get_line(idx) {
        println!(
            "{:04} | {:<16} {:4} '{:?}'",
            idx,
            instruction.opcode(),
            pos,
            chunk.values[*pos]
        );
    } else {
        println!(
            "{:04} {:4} {:<16} {:4} '{:?}'",
            idx,
            chunk.get_line(idx),
            instruction.opcode(),
            pos,
            chunk.values[*pos]
        );
    }
}

fn byte_instruction(idx: usize, chunk: &Chunk, pos: &usize, instruction: &Instruction) {
    if idx > 0 && chunk.get_line(idx - 1) == chunk.get_line(idx) {
        println!("{:04} | {:<16} {:4}", idx, instruction.opcode(), pos);
    } else {
        println!(
            "{:04} {:4} {:<16} {:4}",
            idx,
            chunk.get_line(idx),
            instruction.opcode(),
            pos
        );
    }
}

pub fn diassemble(chunk: &Chunk) -> Result<()> {
    println!("Disassembling Chunk: {}", chunk.name);
    if chunk.instructions.is_empty() {
        return Err(TraceError::EmptyChunk {
            name: chunk.name.clone(),
        });
    }
    for idx in 0..chunk.instructions.len() {
        diassemble_instruction(chunk, idx)?;
    }
    Ok(())
}

pub fn diassemble_instruction(chunk: &Chunk, idx: usize) -> Result<()> {
    let instruction = chunk
        .instructions
        .get(idx)
        .ok_or(TraceError::InvalidInstructionPointer {
            ip: idx,
            len: chunk.instructions.len(),
        })?;
    match instruction {
        Instruction::SetLocal(pos) | Instruction::GetLocal(pos) => {
            byte_instruction(idx, chunk, pos, instruction);
        }
        Instruction::GetGlobal(pos)
        | Instruction::SetGlobal(pos)
        | Instruction::Constant(pos)
        | Instruction::DefineGlobal(pos) => {
            constant_instruction(idx, chunk, pos, instruction);
        }
        _ => {
            simple_instruction(idx, chunk, instruction);
        }
    }
    Ok(())
}
