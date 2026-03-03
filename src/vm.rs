use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::error::{HeapError, InterpretError, RuntimeError};
use crate::heap::{Heap, Object};
#[cfg(feature = "trace")]
use crate::trace::diassemble_instruction;

type Result<T> = std::result::Result<T, InterpretError>;

pub struct VirtualMachine<'a> {
    chunk: Option<&'a Chunk>,
    ip: usize, // The program counter, denotes the next instruction to be executed
    stack: Vec<Value>,
    heap: Option<&'a mut Heap>,
}

impl<'a> VirtualMachine<'a> {
    pub fn new() -> Self {
        VirtualMachine {
            chunk: None,
            ip: 0,
            stack: Vec::new(),
            heap: None,
        }
    }
    fn run(&mut self) -> Result<()> {
        let chunk = *self.chunk.as_ref().ok_or(InterpretError::MissingChunk)?;
        loop {
            let instruction = chunk.instructions.get(self.ip).ok_or(
                InterpretError::InvalidInstructionPointer {
                    ip: self.ip,
                    len: chunk.instructions.len(),
                },
            )?;
            self.ip += 1;
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
                Instruction::Jump(offset) => {
                    self.ip += offset;
                }
                Instruction::JumpIfFalse(offset) => {
                    let is_falsey = match self.stack.last().ok_or(InterpretError::EmptyStack)? {
                        Value::Boolean(boolean) => !*boolean,
                        Value::Nil => true,
                        _ => false,
                    };
                    if is_falsey {
                        self.ip += offset;
                    }
                }
                Instruction::SetLocal(pos) => {
                    self.stack[*pos] = self
                        .stack
                        .last()
                        .ok_or(InterpretError::EmptyStack)?
                        .to_owned();
                }
                Instruction::GetLocal(pos) => {
                    self.stack.push(self.stack[*pos].to_owned());
                }
                Instruction::SetGlobal(pos) => {
                    let heap = self.heap.as_mut().ok_or(InterpretError::MissingHeap)?;
                    if let Value::Object(key) = chunk.values[*pos] {
                        let object = heap
                            .arena
                            .get(key)
                            .ok_or(HeapError::ExpiredArenaKey)
                            .map_err(RuntimeError::from)?;

                        match object {
                            Object::String { value: name } => {
                                if heap.globals.contains_key(name) {
                                    heap.globals.insert(
                                        name.to_owned(),
                                        self.stack
                                            .last()
                                            .ok_or(InterpretError::EmptyStack)?
                                            .to_owned(),
                                    );
                                } else {
                                    return Err(InterpretError::UndefinedVariable {
                                        identifier: name.to_owned(),
                                        line: chunk.get_line(self.ip),
                                    });
                                }
                            }
                        }
                    } else {
                        // unreachable
                    }
                }
                Instruction::GetGlobal(pos) => {
                    let heap = self.heap.as_mut().ok_or(InterpretError::MissingHeap)?;
                    if let Value::Object(key) = chunk.values[*pos] {
                        let object = heap
                            .arena
                            .get(key)
                            .ok_or(HeapError::ExpiredArenaKey)
                            .map_err(RuntimeError::from)?;

                        match object {
                            Object::String { value: name } => {
                                if !heap.globals.contains_key(name) {
                                    return Err(InterpretError::UndefinedVariable {
                                        identifier: name.to_owned(),
                                        line: chunk.get_line(self.ip),
                                    });
                                } else {
                                    self.stack.push(heap.globals[name].clone());
                                }
                            }
                        }
                    } else {
                        // unreachable
                    }
                }
                Instruction::DefineGlobal(pos) => {
                    let heap = self.heap.as_mut().ok_or(InterpretError::MissingHeap)?;
                    if let Value::Object(key) = chunk.values[*pos] {
                        let object = heap
                            .arena
                            .get(key)
                            .ok_or(HeapError::ExpiredArenaKey)
                            .map_err(RuntimeError::from)?;

                        match object {
                            Object::String { value: name } => {
                                let value = self.stack.last().ok_or(InterpretError::EmptyStack)?;
                                heap.globals.insert(name.clone(), value.clone());
                                self.stack.pop().ok_or(InterpretError::EmptyStack)?;
                            }
                        }
                    } else {
                        // unreachable
                    }
                }
                Instruction::Pop => {
                    self.stack.pop().ok_or(InterpretError::EmptyStack)?;
                }
                Instruction::Print => {
                    let value = self.stack.pop().ok_or(InterpretError::EmptyStack)?;
                    if let Value::Object(key) = value {
                        let heap = self.heap.as_ref().ok_or(InterpretError::MissingHeap)?;
                        let object = heap
                            .arena
                            .get(key)
                            .ok_or(HeapError::ExpiredArenaKey)
                            .map_err(RuntimeError::from)?;
                        println!("{:?}", object);
                    } else {
                        println!("{:?}", value);
                    }
                }
                Instruction::Constant(pos) => {
                    let value = &chunk.values[*pos];
                    self.stack.push(value.clone());
                }
                Instruction::True => {
                    self.stack.push(Value::Boolean(true));
                }
                Instruction::False => {
                    self.stack.push(Value::Boolean(false));
                }
                Instruction::Nil => {
                    self.stack.push(Value::Nil);
                }
                Instruction::Not => {
                    let value = self.stack.pop().ok_or(InterpretError::EmptyStack)?;
                    let notted_value = value.not(chunk.get_line(self.ip))?;
                    self.stack.push(notted_value);
                }
                Instruction::Equal => {
                    let result = self.binary_op(Instruction::Equal, chunk.get_line(self.ip))?;
                    self.stack.push(result);
                }
                Instruction::Greater => {
                    let result = self.binary_op(Instruction::Greater, chunk.get_line(self.ip))?;
                    self.stack.push(result);
                }
                Instruction::Less => {
                    let result = self.binary_op(Instruction::Less, chunk.get_line(self.ip))?;
                    self.stack.push(result);
                }
                Instruction::Return => {
                    return Ok(());
                }
                Instruction::Negate => {
                    let value = self.stack.last_mut().ok_or(InterpretError::EmptyStack)?;
                    value.negate(chunk.get_line(self.ip))?;
                }
                Instruction::Add => {
                    let result = self.binary_op(Instruction::Add, chunk.get_line(self.ip))?;
                    self.stack.push(result);
                }
                Instruction::Subtract => {
                    let result = self.binary_op(Instruction::Subtract, chunk.get_line(self.ip))?;
                    self.stack.push(result);
                }
                Instruction::Multiply => {
                    let result = self.binary_op(Instruction::Multiply, chunk.get_line(self.ip))?;
                    self.stack.push(result);
                }
                Instruction::Divide => {
                    let result = self.binary_op(Instruction::Divide, chunk.get_line(self.ip))?;
                    self.stack.push(result);
                }
                Instruction::Modulo => {
                    let result = self.binary_op(Instruction::Modulo, chunk.get_line(self.ip))?;
                    self.stack.push(result);
                }
            }
        }
    }
    pub fn interpret(&mut self, chunk: &'a Chunk, heap: &'a mut Heap) -> Result<()> {
        self.chunk = Some(chunk);
        self.heap = Some(heap);
        self.ip = 0;
        self.run()
    }
    fn binary_op(&mut self, op: Instruction, line: u64) -> Result<Value> {
        let right_operand = self.stack.pop().ok_or(InterpretError::EmptyStack)?;
        let left_operand = self.stack.pop().ok_or(InterpretError::EmptyStack)?;
        match op {
            Instruction::Add => {
                let heap = self.heap.as_mut().ok_or(InterpretError::MissingHeap)?;
                Ok(left_operand.add(right_operand, line, heap)?)
            }
            Instruction::Equal => Ok(left_operand.equal(right_operand)?),
            Instruction::Subtract => Ok(left_operand.subtract(right_operand, line)?),
            Instruction::Multiply => Ok(left_operand.multiply(right_operand, line)?),
            Instruction::Divide => Ok(left_operand.divide(right_operand, line)?),
            Instruction::Modulo => Ok(left_operand.modulo(right_operand, line)?),
            Instruction::Greater => Ok(left_operand.greater(right_operand, line)?),
            Instruction::Less => Ok(left_operand.less(right_operand, line)?),
            _ => Err(InterpretError::InvalidBinaryOp),
        }
    }
}
