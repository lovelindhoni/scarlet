use slotmap::DefaultKey;

use crate::chunk::Chunk;
use crate::common::{Instruction, Value};
use crate::error::InterpretError;
use crate::heap::{Heap, Object};
#[cfg(feature = "trace")]
use crate::trace::diassemble_instruction;

type Result<T> = std::result::Result<T, InterpretError>;

struct CallFrame {
    ip: usize,
    function: DefaultKey,
    slot_start: usize,
}

impl CallFrame {
    pub fn new(ip: usize, function: DefaultKey, slot_start: usize) -> Self {
        Self {
            ip,
            function,
            slot_start,
        }
    }
}

pub struct VirtualMachine<'a> {
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    heap: Option<&'a mut Heap>,
}

impl<'a> VirtualMachine<'a> {
    pub fn new() -> Self {
        VirtualMachine {
            frames: Vec::new(),
            stack: Vec::new(),
            heap: None,
        }
    }
    #[inline(always)]
    fn get_current_chunk(&self) -> &Chunk {
        let frame = self.frames.last().expect("No frames on stack");
        let heap = self.heap.as_ref().expect("Heap not initialized");
        match heap
            .arena
            .get(frame.function)
            .expect("Function missing from arena")
        {
            Object::Function(function) => &function.chunk,
            _ => unreachable!("CallFrame pointed to non-function object"),
        }
    }
    #[inline(always)]
    fn get_mut_top_frame(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }
    #[inline(always)]
    fn get_top_frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    #[inline]
    fn call(&mut self, function_key: DefaultKey, arg_count: usize) -> Result<()> {
        let heap = self.heap.as_ref().unwrap();

        match heap
            .arena
            .get(function_key)
            .expect("function missing from arena")
        {
            Object::Function(function) => {
                if function.arity as usize != arg_count {
                    return {
                        Err(InterpretError::ArgumentsCountMismatch {
                            message: format!(
                                "Expected {} arguments but got {}.",
                                function.arity, arg_count
                            ),
                            line: self.current_line(),
                        })
                    };
                }

                let slot_start = self.stack.len().checked_sub(arg_count + 1).unwrap();
                let frame = CallFrame::new(0, function_key, slot_start);
                self.frames.push(frame);
                Ok(())
            }
            _ => Err(InterpretError::UncallableObject {
                line: self.current_line(),
            }),
        }
    }

    #[inline]
    fn call_value(&mut self, arg_count: usize) -> Result<()> {
        let callee_index = self.stack.len().checked_sub(arg_count + 1).unwrap();
        let callee = self.stack[callee_index];

        if let Value::Object(key) = callee {
            let object = self.heap.as_ref().unwrap().arena.get(key).unwrap();

            match object {
                Object::Function(_) => {
                    self.call(key, arg_count)?;
                }
                _ => {
                    return {
                        Err(InterpretError::UncallableObject {
                            line: self.current_line(),
                        })
                    };
                }
            }
        } else {
            return Err(InterpretError::UncallableObject {
                line: self.current_line(),
            });
        }
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        loop {
            let instruction = self.get_current_chunk().instructions[self.frames.last().unwrap().ip];
            self.get_mut_top_frame().ip += 1;

            let slot_start = self.frames.last().unwrap().slot_start;

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
            diassemble_instruction(self.get_current_chunk(), self.get_top_frame().ip);

            match instruction {
                Instruction::Call(arg_count) => {
                    self.call_value(arg_count)?;
                }
                Instruction::Loop(offset) => {
                    self.get_mut_top_frame().ip -= offset;
                }

                Instruction::Jump(offset) => {
                    self.get_mut_top_frame().ip += offset;
                }

                Instruction::JumpIfFalse(offset) => {
                    let is_falsey = match self.stack.last().unwrap() {
                        Value::Boolean(boolean) => !*boolean,
                        Value::Nil => true,
                        _ => false,
                    };

                    if is_falsey {
                        self.get_mut_top_frame().ip += offset;
                    }
                }

                Instruction::SetLocal(pos) => {
                    self.stack[slot_start + pos] = *self.stack.last().unwrap();
                }

                Instruction::GetLocal(pos) => {
                    self.stack.push(self.stack[slot_start + pos]);
                }

                Instruction::SetGlobal(pos) => {
                    if let Value::Object(key) = self.get_current_chunk().values[pos] {
                        let heap = self.heap.as_mut().unwrap();
                        let object = heap.arena.get(key).unwrap();

                        match object {
                            Object::String(name) => {
                                if heap.globals.contains_key(name) {
                                    heap.globals
                                        .insert(name.to_owned(), *self.stack.last().unwrap());
                                } else {
                                    return Err(InterpretError::UndefinedVariable {
                                        identifier: name.to_owned(),
                                        line: self.current_line(),
                                    });
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                }

                Instruction::GetGlobal(pos) => {
                    if let Value::Object(key) = self.get_current_chunk().values[pos] {
                        let heap = self.heap.as_ref().unwrap();
                        let object = heap.arena.get(key).unwrap();

                        match object {
                            Object::String(name) => {
                                if !heap.globals.contains_key(name) {
                                    return Err(InterpretError::UndefinedVariable {
                                        identifier: name.to_owned(),
                                        line: self.current_line(),
                                    });
                                } else {
                                    self.stack.push(heap.globals[name]);
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                }

                Instruction::DefineGlobal(pos) => {
                    if let Value::Object(key) = self.get_current_chunk().values[pos] {
                        let heap = self.heap.as_mut().unwrap();
                        let object = heap.arena.get(key).unwrap();

                        match object {
                            Object::String(name) => {
                                let value = *self.stack.last().unwrap();
                                heap.globals.insert(name.clone(), value);
                                self.stack.pop().unwrap();
                            }
                            _ => unreachable!(),
                        }
                    }
                }

                Instruction::Pop => {
                    self.stack.pop().unwrap();
                }

                Instruction::Print => {
                    let value = self.stack.pop().unwrap();

                    if let Value::Object(key) = value {
                        let object = self.heap.as_ref().unwrap().arena.get(key).unwrap();
                        println!("{:?}", object);
                    } else {
                        println!("{:?}", value);
                    }
                }

                Instruction::Constant(pos) => {
                    self.stack.push(self.get_current_chunk().values[pos]);
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
                    self.unary_op(Instruction::Not)?;
                }

                Instruction::Equal => {
                    self.binary_op(Instruction::Equal)?;
                }

                Instruction::Greater => {
                    self.binary_op(Instruction::Greater)?;
                }

                Instruction::Less => {
                    self.binary_op(Instruction::Less)?;
                }

                Instruction::Negate => {
                    self.unary_op(Instruction::Negate)?;
                }

                Instruction::Add => {
                    self.binary_op(Instruction::Add)?;
                }

                Instruction::Subtract => {
                    self.binary_op(Instruction::Subtract)?;
                }

                Instruction::Multiply => {
                    self.binary_op(Instruction::Multiply)?;
                }

                Instruction::Divide => {
                    self.binary_op(Instruction::Divide)?;
                }

                Instruction::Modulo => {
                    self.binary_op(Instruction::Modulo)?;
                }

                Instruction::Return => {
                    let result = self.stack.pop().unwrap();
                    let frame = self.frames.pop().unwrap();
                    if self.frames.is_empty() {
                        self.stack.pop().unwrap();
                        return Ok(());
                    }
                    self.stack.truncate(frame.slot_start);
                    self.stack.push(result);
                }
            }
        }
    }
    pub fn interpret(&mut self, function_key: DefaultKey, heap: &'a mut Heap) -> Result<()> {
        self.heap = Some(heap);
        self.stack.push(Value::Object(function_key));
        self.call(function_key, 0)?;

        match self.run() {
            Ok(v) => Ok(v),
            Err(err) => {
                eprintln!("{}", err);
                self.print_stack_trace();
                Err(err)
            }
        }
    }

    #[inline(always)]
    fn unary_op(&mut self, op: Instruction) -> Result<()> {
        let val = self.stack.pop().expect("Stack underflow");

        let result = match op {
            Instruction::Negate => match val {
                Value::Number(num) => Value::Number(-num),
                _ => {
                    return Err(InterpretError::TypeError {
                        message: "Only numbers can be negated".to_string(),
                        line: self.current_line(),
                    });
                }
            },
            Instruction::Not => match val {
                Value::Nil => Value::Boolean(true),
                Value::Boolean(b) => Value::Boolean(!b),
                _ => Value::Boolean(false),
            },
            _ => unreachable!("Invalid unary operation"),
        };

        self.stack.push(result);
        Ok(())
    }

    #[inline(always)]
    fn binary_op(&mut self, op: Instruction) -> Result<()> {
        let b = self.stack.pop().expect("Stack underflow");
        let a = self.stack.pop().expect("Stack underflow");

        let result = match op {
            Instruction::Add => match (a, b) {
                (Value::Number(n1), Value::Number(n2)) => Value::Number(n1 + n2),
                (Value::Object(k1), Value::Object(k2)) => {
                    let heap = self.heap.as_mut().unwrap();
                    Value::Object(heap.concatenate_strings(k1, k2))
                }
                _ => {
                    return Err(InterpretError::TypeError {
                        line: self.current_line(),
                        message: "Operands must be two numbers or two strings.".to_string(),
                    });
                }
            },
            Instruction::Subtract => self.num_op(a, b, |x, y| x - y)?,
            Instruction::Multiply => self.num_op(a, b, |x, y| x * y)?,
            Instruction::Divide => {
                if let Value::Number(0.0) = b {
                    return Err(InterpretError::DivisionByZero {
                        line: self.current_line(),
                        left_num: match a {
                            Value::Number(n) => n,
                            _ => 0.0,
                        },
                        right_num: 0.0,
                    });
                }
                self.num_op(a, b, |x, y| x / y)?
            }
            Instruction::Modulo => self.num_op(a, b, |x, y| x % y)?,
            Instruction::Greater => self.bool_op(a, b, |x, y| x > y)?,
            Instruction::Less => self.bool_op(a, b, |x, y| x < y)?,
            Instruction::Equal => self.equal_op(a, b)?,
            _ => return Err(InterpretError::InvalidBinaryOp),
        };

        self.stack.push(result);
        Ok(())
    }

    #[inline(always)]
    fn num_op<F>(&self, a: Value, b: Value, f: F) -> Result<Value>
    where
        F: FnOnce(f64, f64) -> f64,
    {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => Ok(Value::Number(f(n1, n2))),
            _ => Err(InterpretError::TypeError {
                line: self.current_line(),
                message: "Operands must be numbers.".to_string(),
            }),
        }
    }

    #[inline(always)]
    fn bool_op<F>(&self, a: Value, b: Value, f: F) -> Result<Value>
    where
        F: FnOnce(f64, f64) -> bool,
    {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => Ok(Value::Boolean(f(n1, n2))),
            _ => Err(InterpretError::TypeError {
                line: self.current_line(),
                message: "Operands must be numbers for comparison.".to_string(),
            }),
        }
    }

    #[inline(always)]
    fn equal_op(&self, a: Value, b: Value) -> Result<Value> {
        Ok(match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => Value::Boolean(n1 == n2),
            (Value::Boolean(b1), Value::Boolean(b2)) => Value::Boolean(b1 == b2),
            (Value::Nil, Value::Nil) => Value::Boolean(true),
            (Value::Object(k1), Value::Object(k2)) => Value::Boolean(k1 == k2),
            _ => Value::Boolean(false),
        })
    }
    fn print_stack_trace(&self) {
        let heap = self.heap.as_ref().unwrap();
        for frame in self.frames.iter().rev() {
            let object = heap.arena.get(frame.function).unwrap();

            if let Object::Function(function) = object {
                let instruction = frame.ip.saturating_sub(1);
                let line = function.chunk.get_line(instruction);

                if let Some(name_key) = function.name {
                    let name_obj = heap.arena.get(name_key).unwrap();
                    if let Object::String(value) = name_obj {
                        eprintln!("[line {}] in {}()", line, value);
                    }
                } else {
                    eprintln!("[line {}] in script", line);
                }
            }
        }
    }
    #[inline(always)]
    fn current_line(&self) -> u64 {
        let ip = self.get_top_frame().ip - 1;
        self.get_current_chunk().get_line(ip)
    }
}
