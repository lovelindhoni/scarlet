use crate::common::{Instruction, Value};
use crate::error::InterpretError;
use crate::heap::{BASE_GC_TRIGGER, Heap, HeapKey, Object, UpvalueState, mark_object, mark_value};
#[cfg(feature = "trace")]
use crate::trace::diassemble_instruction;

const GC_HEAP_GROW_FACTOR: u32 = 2;

type Result<T> = std::result::Result<T, InterpretError>;

struct CallFrame {
    ip: usize,
    closure: HeapKey,
    slot_start: usize,
}

impl CallFrame {
    pub fn new(ip: usize, closure: HeapKey, slot_start: usize) -> Self {
        Self {
            ip,
            closure,
            slot_start,
        }
    }
}

pub struct VirtualMachine<'a> {
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    heap: Option<&'a mut Heap>,
    open_upvalues: Vec<HeapKey>,
}

impl<'a> VirtualMachine<'a> {
    pub fn new() -> Self {
        Self {
            frames: Vec::with_capacity(64),
            stack: Vec::with_capacity(256),
            heap: None,
            open_upvalues: Vec::new(),
        }
    }

    fn collect_garbage(&mut self) {
        let (bytes_allocated, next_gc_run) = {
            let heap = self.heap.as_ref().unwrap();
            (heap.bytes_allocated, heap.next_gc_run)
        };
        if bytes_allocated >= next_gc_run {
            self.heap.as_mut().unwrap().mark_globals();
            self.mark_vm_roots();
            self.heap.as_mut().unwrap().mark_interned_strings();
            self.heap.as_mut().unwrap().sweep();
            let heap = self.heap.as_mut().unwrap();
            heap.next_gc_run =
                (heap.bytes_allocated * GC_HEAP_GROW_FACTOR as usize).max(BASE_GC_TRIGGER);
        }
    }

    fn mark_vm_roots(&mut self) {
        let heap = self.heap.as_mut().unwrap();
        for value in &self.stack {
            mark_value(&heap.arena, &mut heap.marked_objects, value);
        }
        for frame in &self.frames {
            mark_object(&heap.arena, &mut heap.marked_objects, &frame.closure);
        }
        for upvalue_key in &self.open_upvalues {
            mark_object(&heap.arena, &mut heap.marked_objects, upvalue_key);
        }
    }

    fn capture_upvalue(&mut self, stack_idx: usize) -> HeapKey {
        for &key in &self.open_upvalues {
            if let Object::Upvalue(uv) = self.heap.as_ref().unwrap().arena.get(key).unwrap() {
                if let UpvalueState::Open(loc) = uv.state {
                    if loc == stack_idx {
                        return key;
                    }
                }
            }
        }
        let key = self.heap.as_mut().unwrap().allocate_upvalue(stack_idx);
        let pos = self
            .open_upvalues
            .iter()
            .position(|&k| {
                if let Object::Upvalue(uv) = self.heap.as_ref().unwrap().arena.get(k).unwrap() {
                    if let UpvalueState::Open(loc) = uv.state {
                        loc < stack_idx
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .unwrap_or(self.open_upvalues.len());
        self.open_upvalues.insert(pos, key);
        key
    }
    #[inline]
    // TODO: might let call_value absorb this function within itself, because it does an extra heap lookup
    fn call(&mut self, closure_key: HeapKey, arg_count: usize) -> Result<()> {
        let heap = self.heap.as_ref().unwrap();

        let function_key = if let Object::Closure(closure) = heap.arena.get(closure_key).unwrap() {
            closure.function
        } else {
            unreachable!()
        };

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
                        })
                    };
                }

                let slot_start = self.stack.len().checked_sub(arg_count + 1).unwrap();
                let frame = CallFrame::new(0, closure_key, slot_start);
                self.frames.push(frame);
                Ok(())
            }
            _ => Err(InterpretError::UncallableObject {}),
        }
    }

    #[inline]
    fn call_value(&mut self, arg_count: usize) -> Result<()> {
        let callee_index = self.stack.len().checked_sub(arg_count + 1).unwrap();
        let callee = self.stack[callee_index];

        if let Value::Object(key) = callee {
            let heap = self.heap.as_mut().unwrap();
            let object = heap.arena.get(key).unwrap();

            match object {
                Object::Function(_) => {
                    // because every user defined function is now wrapped in a closure
                    unreachable!()
                }
                Object::Closure(_closure) => {
                    // its okay to clone this here, bcoz, the objclosure doesn't have much data as of now?
                    self.call(key, arg_count)?;
                }
                Object::NativeFunction(native_function) => {
                    let stack_len = self.stack.len();
                    let args_start = stack_len - arg_count;
                    let result = (native_function.function)(
                        native_function.name,
                        &self.stack[args_start..],
                        heap,
                    )
                    .map_err(|message| InterpretError::NativeFunctionError { message })?;
                    self.stack.truncate(stack_len - (arg_count + 1));
                    self.stack.push(result);
                }
                _ => {
                    return Err(InterpretError::UncallableObject);
                }
            }
        } else {
            return Err(InterpretError::UncallableObject);
        }
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        loop {
            self.collect_garbage();

            let frame_index = self.frames.len() - 1;
            let frame = &mut self.frames[frame_index];

            let heap = self.heap.as_ref().unwrap();

            let closure = match heap.arena.get(frame.closure).unwrap() {
                Object::Closure(c) => c,
                _ => unreachable!(),
            };

            let function = match heap.arena.get(closure.function).unwrap() {
                Object::Function(f) => f,
                _ => unreachable!(),
            };

            let chunk = &function.chunk;
            let stack = &mut self.stack;

            let instruction = &chunk.instructions[frame.ip];
            frame.ip += 1;

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
            diassemble_instruction(chunk, frame.ip);

            match instruction {
                Instruction::GetUpvalue(slot) => {
                    let upvalue_key = if let Object::Closure(c) = self
                        .heap
                        .as_ref()
                        .unwrap()
                        .arena
                        .get(frame.closure)
                        .unwrap()
                    {
                        c.upvalues[*slot]
                    } else {
                        unreachable!()
                    };

                    let val = match &self.heap.as_ref().unwrap().arena.get(upvalue_key).unwrap() {
                        Object::Upvalue(uv) => match uv.state {
                            UpvalueState::Open(idx) => self.stack[idx],
                            UpvalueState::Closed(v) => v,
                        },
                        _ => unreachable!(),
                    };
                    self.stack.push(val);
                }

                Instruction::SetUpvalue(slot) => {
                    let upvalue_key = if let Object::Closure(c) = self
                        .heap
                        .as_ref()
                        .unwrap()
                        .arena
                        .get(frame.closure)
                        .unwrap()
                    {
                        c.upvalues[*slot]
                    } else {
                        unreachable!()
                    };

                    let val = *self.stack.last().unwrap();
                    if let Object::Upvalue(uv) = self
                        .heap
                        .as_mut()
                        .unwrap()
                        .arena
                        .get_mut(upvalue_key)
                        .unwrap()
                    {
                        match &mut uv.state {
                            UpvalueState::Open(idx) => self.stack[*idx] = val,
                            UpvalueState::Closed(v) => *v = val,
                        }
                    }
                }
                Instruction::CloseUpvalue => {
                    let top_idx = self.stack.len() - 1;
                    self.close_upvalues(top_idx);
                    self.stack.pop();
                }
                Instruction::Closure(pos, upvalues) => {
                    let value = chunk.values[*pos];
                    let upvalues = upvalues.clone();
                    let slot_start = frame.slot_start;
                    let closure_key = frame.closure;

                    if let Value::Object(function_key) = value {
                        let mut upvalue_keys = Vec::new();
                        for uv in upvalues.iter() {
                            let key = if uv.is_local {
                                let stack_idx = slot_start + uv.index;
                                self.capture_upvalue(stack_idx)
                            } else {
                                let heap = self.heap.as_ref().unwrap();
                                if let Object::Closure(c) = heap.arena.get(closure_key).unwrap() {
                                    c.upvalues[uv.index]
                                } else {
                                    unreachable!()
                                }
                            };
                            upvalue_keys.push(key);
                        }
                        let closure_key = self
                            .heap
                            .as_mut()
                            .unwrap()
                            .allocate_closure(function_key, upvalue_keys);
                        self.stack.push(Value::Object(closure_key));
                    }
                }
                Instruction::Call(arg_count) => {
                    self.call_value(*arg_count)?;
                }
                Instruction::Loop(offset) => {
                    frame.ip -= offset;
                }

                Instruction::Jump(offset) => {
                    frame.ip += offset;
                }

                Instruction::JumpIfFalse(offset) => {
                    let is_falsey = match stack.last().unwrap() {
                        Value::Boolean(boolean) => !*boolean,
                        Value::Nil => true,
                        _ => false,
                    };

                    if is_falsey {
                        frame.ip += offset;
                    }
                }

                Instruction::SetLocal(pos) => {
                    stack[frame.slot_start + pos] = *stack.last().unwrap();
                }

                Instruction::GetLocal(pos) => {
                    stack.push(stack[frame.slot_start + pos]);
                }

                Instruction::SetGlobal(pos) => {
                    if let Value::Object(key) = chunk.values[*pos] {
                        let val = *stack.last().unwrap();
                        let heap = self.heap.as_mut().unwrap();
                        match heap.globals.get_mut(&key) {
                            Some(slot) => *slot = val,
                            None => {
                                return Err(InterpretError::UndefinedVariable {
                                    identifier: self.key_to_string(key),
                                });
                            }
                        }
                    }
                }

                Instruction::DefineGlobal(pos) => {
                    if let Value::Object(key) = chunk.values[*pos] {
                        let val = stack.pop().unwrap();
                        self.heap.as_mut().unwrap().globals.insert(key, val);
                    }
                }

                Instruction::GetGlobal(pos) => {
                    if let Value::Object(key) = chunk.values[*pos] {
                        match self.heap.as_ref().unwrap().globals.get(&key) {
                            Some(&val) => stack.push(val),
                            None => {
                                return Err(InterpretError::UndefinedVariable {
                                    identifier: self.key_to_string(key),
                                });
                            }
                        }
                    }
                }

                Instruction::Pop => {
                    stack.pop().unwrap();
                }

                Instruction::Constant(pos) => {
                    stack.push(chunk.values[*pos]);
                }

                Instruction::True => {
                    stack.push(Value::Boolean(true));
                }

                Instruction::False => {
                    stack.push(Value::Boolean(false));
                }

                Instruction::Nil => {
                    stack.push(Value::Nil);
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
                    let result = stack.pop().unwrap();
                    let frame = self.frames.pop().unwrap();
                    if self.frames.is_empty() {
                        stack.pop().unwrap();
                        return Ok(());
                    }
                    let slot_start = frame.slot_start;
                    self.close_upvalues(slot_start);
                    self.stack.truncate(slot_start);
                    self.stack.push(result);
                }
            }
        }
    }
    fn close_upvalues(&mut self, from_slot: usize) {
        let heap = self.heap.as_mut().unwrap();
        self.open_upvalues.retain(|&key| {
            if let Object::Upvalue(uv) = heap.arena.get_mut(key).unwrap() {
                if let UpvalueState::Open(loc) = uv.state {
                    if loc >= from_slot {
                        let val = self.stack[loc];
                        uv.state = UpvalueState::Closed(val);
                        return false;
                    }
                }
            }
            true
        });
    }
    pub fn interpret(&mut self, function_key: HeapKey, heap: &'a mut Heap) -> Result<()> {
        let closure_key = heap.allocate_closure(function_key, Vec::new());
        self.stack.push(Value::Object(closure_key));

        self.heap = Some(heap);
        self.call(closure_key, 0)?;

        match self.run() {
            Ok(v) => Ok(v),
            Err(err) => {
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
                        message: "Operands must be two numbers or two strings.".to_string(),
                    });
                }
            },
            Instruction::Subtract => self.num_op(a, b, |x, y| x - y)?,
            Instruction::Multiply => self.num_op(a, b, |x, y| x * y)?,
            Instruction::Divide => {
                if let Value::Number(0.0) = b {
                    return Err(InterpretError::DivisionByZero {
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
            let function_key =
                if let Object::Closure(closure) = heap.arena.get(frame.closure).unwrap() {
                    closure.function
                } else {
                    unreachable!()
                };

            let object = heap.arena.get(function_key).unwrap();
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
    fn key_to_string(&self, key: HeapKey) -> String {
        match self.heap.as_ref().unwrap().arena.get(key) {
            Some(Object::String(s)) => s.clone(),
            _ => "<unknown>".to_owned(),
        }
    }
}
