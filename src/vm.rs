use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::error::InterpretError;
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
        let chunk = *self.chunk.as_ref().unwrap();

        loop {
            // TODO: might skip bounds checking here via unsafe
            let instruction = &chunk.instructions[self.ip];
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
                Instruction::Loop(offset) => {
                    self.ip -= offset;
                }

                Instruction::Jump(offset) => {
                    self.ip += offset;
                }

                Instruction::JumpIfFalse(offset) => {
                    let is_falsey = match self.stack.last().unwrap() {
                        Value::Boolean(boolean) => !*boolean,
                        Value::Nil => true,
                        _ => false,
                    };

                    if is_falsey {
                        self.ip += offset;
                    }
                }

                Instruction::SetLocal(pos) => {
                    self.stack[*pos] = *self.stack.last().unwrap();
                }

                Instruction::GetLocal(pos) => {
                    self.stack.push(self.stack[*pos]);
                }

                Instruction::SetGlobal(pos) => {
                    let heap = self.heap.as_mut().unwrap();

                    if let Value::Object(key) = chunk.values[*pos] {
                        let object = heap.arena.get(key).unwrap();

                        match object {
                            Object::String { value: name } => {
                                heap.globals
                                    .insert(name.to_owned(), *self.stack.last().unwrap());
                            }
                        }
                    }
                }

                Instruction::GetGlobal(pos) => {
                    let heap = self.heap.as_mut().unwrap();

                    if let Value::Object(key) = chunk.values[*pos] {
                        let object = heap.arena.get(key).unwrap();

                        match object {
                            Object::String { value: name } => {
                                self.stack.push(heap.globals[name]);
                            }
                        }
                    }
                }

                Instruction::DefineGlobal(pos) => {
                    let heap = self.heap.as_mut().unwrap();

                    if let Value::Object(key) = chunk.values[*pos] {
                        let object = heap.arena.get(key).unwrap();

                        match object {
                            Object::String { value: name } => {
                                let value = *self.stack.last().unwrap();
                                heap.globals.insert(name.clone(), value);
                                self.stack.pop().unwrap();
                            }
                        }
                    }
                }

                Instruction::Pop => {
                    self.stack.pop().unwrap();
                }

                Instruction::Print => {
                    let value = self.stack.pop().unwrap();

                    if let Value::Object(key) = value {
                        let heap = self.heap.as_ref().unwrap();
                        let object = heap.arena.get(key).unwrap();
                        println!("{:?}", object);
                    } else {
                        println!("{:?}", value);
                    }
                }

                Instruction::Constant(pos) => {
                    self.stack.push(chunk.values[*pos]);
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
                    let value = self.stack.pop().unwrap();
                    let notted_value = value.not(chunk, self.ip)?;
                    self.stack.push(notted_value);
                }

                Instruction::Equal => {
                    let result = self.binary_op(Instruction::Equal)?;
                    self.stack.push(result);
                }

                Instruction::Greater => {
                    let result = self.binary_op(Instruction::Greater)?;
                    self.stack.push(result);
                }

                Instruction::Less => {
                    let result = self.binary_op(Instruction::Less)?;
                    self.stack.push(result);
                }

                Instruction::Return => {
                    return Ok(());
                }

                Instruction::Negate => {
                    let value = self.stack.last_mut().unwrap();
                    value.negate(chunk, self.ip)?;
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
        }
    }
    pub fn interpret(&mut self, chunk: &'a Chunk, heap: &'a mut Heap) -> Result<()> {
        self.chunk = Some(chunk);
        self.heap = Some(heap);
        self.ip = 0;
        self.run()
    }

    #[inline(always)]
    fn binary_op(&mut self, op: Instruction) -> Result<Value> {
        let right_operand = self.stack.pop().unwrap();
        let left_operand = self.stack.pop().unwrap();
        let chunk = self.chunk.unwrap();
        match op {
            Instruction::Add => {
                let heap = self.heap.as_mut().unwrap();
                Ok(left_operand.add(right_operand, chunk, self.ip, heap)?)
            }
            Instruction::Equal => Ok(left_operand.equal(right_operand)?),
            Instruction::Subtract => Ok(left_operand.subtract(right_operand, chunk, self.ip)?),
            Instruction::Multiply => Ok(left_operand.multiply(right_operand, chunk, self.ip)?),
            Instruction::Divide => Ok(left_operand.divide(right_operand, chunk, self.ip)?),
            Instruction::Modulo => Ok(left_operand.modulo(right_operand, chunk, self.ip)?),
            Instruction::Greater => Ok(left_operand.greater(right_operand, chunk, self.ip)?),
            Instruction::Less => Ok(left_operand.less(right_operand, chunk, self.ip)?),
            _ => Err(InterpretError::InvalidBinaryOp),
        }
    }
}
