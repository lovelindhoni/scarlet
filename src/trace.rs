use crate::chunk::Chunk;
use crate::common::Instruction;
use crate::error::TraceError;
use crate::heap::{Heap, HeapKey, Object};

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

fn constant_instruction(
    idx: usize,
    chunk: &Chunk,
    pos: &usize,
    instruction: &Instruction,
    heap: &Heap,
) {
    let line = print_prefix(idx, chunk);
    println!(
        "{:04} {:>4} {:<16} {:4} '{:?}'",
        idx,
        line,
        instruction.opcode(),
        pos,
        chunk.values[*pos].display(heap)
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

pub fn diassemble(function_key: HeapKey, heap: &Heap) -> Result<()> {
    let (chunk, function_name_key) = match heap.arena.get(function_key).unwrap() {
        Object::Function(function) => (&function.chunk, &function.name),
        _ => unreachable!(),
    };
    println!(
        "Disassembling Function: {}",
        if let Some(function_name_key) = function_name_key {
            match heap.arena.get(*function_name_key).unwrap() {
                Object::String(value) => value,
                _ => unreachable!(),
            }
        } else {
            "<script>"
        }
    );
    if chunk.instructions.is_empty() {
        return Err(TraceError::EmptyChunk);
    }
    for idx in 0..chunk.instructions.len() {
        diassemble_instruction(chunk, idx, heap)?;
    }
    Ok(())
}

pub fn diassemble_instruction(chunk: &Chunk, idx: usize, heap: &Heap) -> Result<()> {
    let instruction = chunk
        .instructions
        .get(idx)
        .ok_or(TraceError::InvalidInstructionPointer {
            ip: idx,
            len: chunk.instructions.len(),
        })?;
    match instruction {
        Instruction::SetLocal(pos)
        | Instruction::GetLocal(pos)
        | Instruction::SetUpvalue(pos)
        | Instruction::GetUpvalue(pos) => {
            byte_instruction(idx, chunk, pos, instruction);
        }
        Instruction::Call(arg_count) => {
            byte_instruction(idx, chunk, arg_count, instruction);
        }
        Instruction::Jump(offset)
        | Instruction::JumpIfFalse(offset)
        | Instruction::Loop(offset) => {
            jump_instruction(idx, chunk, offset, instruction);
        }
        Instruction::GetGlobal(pos)
        | Instruction::SetGlobal(pos)
        | Instruction::Constant(pos)
        | Instruction::Class(pos)
        | Instruction::GetProperty(pos)
        | Instruction::SetProperty(pos)
        | Instruction::Method(pos)
        | Instruction::DefineGlobal(pos) => {
            constant_instruction(idx, chunk, pos, instruction, heap);
        }

        Instruction::Invoke(pos, arg_count) => {
            let line = print_prefix(idx, chunk);
            println!(
                "{:04} {:>4} {:<16} {:4} ({} args) '{:?}'",
                idx,
                line,
                instruction.opcode(),
                pos,
                arg_count,
                chunk.values[*pos].display(heap)
            );
        }

        Instruction::Closure(pos, upvalues) => {
            let line = print_prefix(idx, chunk);

            println!(
                "{:04} {:>4} {:<16} {:4} '{:?}'",
                idx,
                line,
                instruction.opcode(),
                pos,
                chunk.values[*pos].display(heap)
            );
            for (i, upvalue) in upvalues.iter().enumerate() {
                let location = if upvalue.is_local { "local" } else { "upvalue" };

                println!(
                    "      |                     {:<8} {:4} (slot {})",
                    location, upvalue.index, i
                );
            }
        }

        _ => {
            simple_instruction(idx, chunk, instruction);
        }
    }
    Ok(())
}
