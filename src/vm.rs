use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
#[cfg(feature = "trace")]
use crate::debug::diassemble_instruction;

pub struct VirtualMachine<'a> {
    chunk: Option<&'a Chunk>,
    ip: usize, // The program counter, denotes the next instruction to be executed
    stack: Vec<Value>,
}

#[repr(u8)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

impl<'a> VirtualMachine<'a> {
    pub fn new() -> Self {
        VirtualMachine {
            chunk: None,
            ip: 0,
            stack: Vec::new(),
        }
    }
    fn run(&mut self) -> InterpretResult {
        let chunk = *self
            .chunk
            .as_ref()
            .expect("Chunk Instance should've been filled here!");
        loop {
            if let Some(instruction) = chunk.instructions.get(self.ip) {
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
                        if let Some(value) = self.stack.pop() {
                            println!("{:?}", value);
                        } else {
                            // should be handling the case where the stack is empty
                        }
                        return InterpretResult::Ok;
                    }
                    Instruction::Negate => {
                        if let Some(value) = self.stack.last_mut() {
                            if let Err(e) = value.negate() {
                                eprintln!("{e}");
                            }
                        } else {
                            eprintln!("Negate couldn't find no operand on top of stack");
                        }
                    }
                    Instruction::Add => {
                        let result = self.binary_op(Instruction::Add);
                        match result {
                            Ok(result) => self.stack.push(result),
                            Err(e) => {
                                eprintln!("{e}");
                            }
                        }
                    }
                    Instruction::Subtract => {
                        let result = self.binary_op(Instruction::Subtract);
                        match result {
                            Ok(result) => self.stack.push(result),
                            Err(e) => {
                                eprintln!("{e}");
                            }
                        }
                    }
                    Instruction::Multiply => {
                        let result = self.binary_op(Instruction::Multiply);
                        match result {
                            Ok(result) => self.stack.push(result),
                            Err(e) => {
                                eprintln!("{e}");
                            }
                        }
                    }
                    Instruction::Divide => {
                        let result = self.binary_op(Instruction::Divide);
                        match result {
                            Ok(result) => self.stack.push(result),
                            Err(e) => {
                                eprintln!("{e}");
                            }
                        }
                    }
                    Instruction::Modulo => {
                        let result = self.binary_op(Instruction::Modulo);
                        match result {
                            Ok(result) => self.stack.push(result),
                            Err(e) => {
                                eprintln!("{e}");
                            }
                        }
                    }
                }
            }
            self.ip += 1;
        }
    }
    pub fn interpret(&mut self, chunk: &'a Chunk) -> InterpretResult {
        self.chunk = Some(&chunk);
        self.ip = 0;
        let result = self.run();
        result
    }
    fn binary_op(&mut self, op: Instruction) -> Result<Value, String> {
        if self.stack.len() >= 2 {
            let right_operand = self.stack.pop().expect("Stack length checked above!");
            let left_operand = self.stack.pop().expect("Stack length checked above");
            match op {
                Instruction::Add => left_operand.add(right_operand),
                Instruction::Subtract => left_operand.subtract(right_operand),
                Instruction::Multiply => left_operand.multiply(right_operand),
                Instruction::Divide => left_operand.divide(right_operand),
                Instruction::Modulo => left_operand.modulo(right_operand),
                _ => Err(format!("Not a binary operation")),
            }
        } else {
            Err(format!("Stack doesn't have operands for binary operation"))
        }
    }
}
