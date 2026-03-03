use crate::chunk::Chunk;
use crate::common::Instruction;
use crate::error::TraceError;

type Result<T> = std::result::Result<T, TraceError>;

fn print_prefix(idx: usize, chunk: &Chunk) -> String {
    let same_line = idx > 0 && chunk.get_line(idx - 1) == chunk.get_line(idx);
    if same_line {
        "|".to_string()
    } else {
        chunk.get_line(idx).to_string()
    }
}

fn simple_instruction(idx: usize, chunk: &Chunk, instruction: &Instruction) {
    let line = print_prefix(idx, chunk);
    println!("{:04} {:>4} {:<16}", idx, line, instruction.opcode());
}

fn constant_instruction(idx: usize, chunk: &Chunk, pos: &usize, instruction: &Instruction) {
    let line = print_prefix(idx, chunk);
    println!(
        "{:04} {:>4} {:<16} {:4} '{:?}'",
        idx,
        line,
        instruction.opcode(),
        pos,
        chunk.values[*pos]
    );
}

fn jump_instruction(idx: usize, chunk: &Chunk, offset: &usize, instruction: &Instruction) {
    let line = print_prefix(idx, chunk);
    let destination = idx + 1 + *offset;

    println!(
        "{:04} {:>4} {:<16} {:4} -> {:04}",
        idx,
        line,
        instruction.opcode(),
        offset,
        destination
    );
}

fn byte_instruction(idx: usize, chunk: &Chunk, pos: &usize, instruction: &Instruction) {
    let line = print_prefix(idx, chunk);
    println!(
        "{:04} {:>4} {:<16} {:4}",
        idx,
        line,
        instruction.opcode(),
        pos
    );
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
        Instruction::Jump(offset) | Instruction::JumpIfFalse(offset) => {
            jump_instruction(idx, chunk, offset, instruction);
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
