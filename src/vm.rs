use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::error::InterpretError;
#[cfg(feature = "trace")]
use crate::trace::diassemble_instruction;

type Result<T> = std::result::Result<T, InterpretError>;

pub struct VirtualMachine<'a> {
    chunk: Option<&'a Chunk>,
    ip: usize, // The program counter, denotes the next instruction to be executed
    stack: Vec<Value>,
}

impl<'a> VirtualMachine<'a> {
    pub fn new() -> Self {
        VirtualMachine {
            chunk: None,
            ip: 0,
            stack: Vec::new(),
        }
    }
    fn run(&mut self) -> Result<()> {
        let chunk = *self
            .chunk
            .as_ref()
            .expect("Chunk Instance should've been filled here!");
        loop {
            let instruction = chunk.instructions.get(self.ip).ok_or(
                InterpretError::InvalidInstructionPointer {
                    ip: self.ip,
                    len: chunk.instructions.len(),
                },
            )?;
            #[cfg(feature = "trace")]
            println!("Gutting VM's stack");
            #[cfg(feature = "trace")]
            if self.stack.is_empty() {
                println!("Stack is Empty!");
            } else {
                for value in &self.stack {
                    println!("[ {:?} ]", value);
                }
            }
            #[cfg(feature = "trace")]
            diassemble_instruction(chunk, self.ip);
            match instruction {
                Instruction::Constant(pos) => {
                    let value = &chunk.values[*pos];
                    self.stack.push(value.clone());
                }
                Instruction::Return => {
                    let value = self.stack.pop().ok_or(InterpretError::EmptyStack)?;
                    println!("{:?}", value);
                    return Ok(());
                }
                Instruction::Negate => {
                    let value = self.stack.last_mut().ok_or(InterpretError::EmptyStack)?;
                    value.negate()?;
                }
                Instruction::Add => {
                    let result = self.binary_op(Instruction::Add)?;
                    self.stack.push(result);
                }
                Instruction::Subtract => {
                    let result = self.binary_op(Instruction::Subtract)?;
                    self.stack.push(result);
                }
                Instruction::Multiply => {
                    let result = self.binary_op(Instruction::Multiply)?;
                    self.stack.push(result);
                }
                Instruction::Divide => {
                    let result = self.binary_op(Instruction::Divide)?;
                    self.stack.push(result);
                }
                Instruction::Modulo => {
                    let result = self.binary_op(Instruction::Modulo)?;
                    self.stack.push(result);
                }
            }
            self.ip += 1;
        }
    }
    pub fn interpret(&mut self, chunk: &'a Chunk) -> Result<()> {
        self.chunk = Some(chunk);
        self.ip = 0;
        self.run()
    }
    fn binary_op(&mut self, op: Instruction) -> Result<Value> {
        let right_operand = self.stack.pop().ok_or(InterpretError::EmptyStack)?;
        let left_operand = self.stack.pop().ok_or(InterpretError::EmptyStack)?;
        match op {
            Instruction::Add => Ok(left_operand.add(right_operand)?),
            Instruction::Subtract => Ok(left_operand.subtract(right_operand)?),
            Instruction::Multiply => Ok(left_operand.multiply(right_operand)?),
            Instruction::Divide => Ok(left_operand.divide(right_operand)?),
            Instruction::Modulo => Ok(left_operand.modulo(right_operand)?),
            _ => Err(InterpretError::InvalidBinaryOp),
        }
    }
}
