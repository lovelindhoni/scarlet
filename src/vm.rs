use std::mem::MaybeUninit;

use crate::common::{Instruction, Value};
use crate::error::InterpretError;
use crate::heap::{BASE_GC_TRIGGER, Heap, HeapKey, Object, UpvalueState, mark_object, mark_value};
#[cfg(feature = "trace")]
use crate::trace::diassemble_instruction;

const GC_HEAP_GROW_FACTOR: u32 = 2;
const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = FRAMES_MAX * 256;

type Result<T> = std::result::Result<T, InterpretError>;

struct CallFrame {
    ip: usize,
    closure: HeapKey,
    slot_start: usize,
    function_key: HeapKey,
}

impl CallFrame {
    pub fn new(ip: usize, closure: HeapKey, function_key: HeapKey, slot_start: usize) -> Self {
        Self {
            ip,
            closure,
            function_key,
            slot_start,
        }
    }
}

pub struct VirtualMachine<'a> {
    frames: [MaybeUninit<CallFrame>; FRAMES_MAX],
    frame_count: usize,
    stack: [Value; STACK_MAX],
    stack_top: usize,
    heap: Option<&'a mut Heap>,
    open_upvalues: Vec<HeapKey>,
    init_string: Option<HeapKey>,
}

impl<'a> VirtualMachine<'a> {
    pub fn new() -> Self {
        Self {
            frames: unsafe { MaybeUninit::uninit().assume_init() },
            frame_count: 0,
            stack: [Value::Nil; STACK_MAX],
            stack_top: 0,
            heap: None,
            open_upvalues: Vec::new(),
            init_string: None,
        }
    }

    #[inline(always)]
    fn push(&mut self, value: Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
    }

    #[inline(always)]
    fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        self.stack[self.stack_top]
    }

    #[inline(always)]
    fn push_frame(&mut self, frame: CallFrame) {
        self.frames[self.frame_count].write(frame);
        self.frame_count += 1;
    }

    #[inline(always)]
    fn pop_frame(&mut self) -> CallFrame {
        self.frame_count -= 1;
        unsafe { self.frames[self.frame_count].assume_init_read() }
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

        mark_object(
            &heap.arena,
            &mut heap.marked_objects,
            &self.init_string.unwrap(),
        );
        for value in &self.stack[..self.stack_top] {
            mark_value(&heap.arena, &mut heap.marked_objects, value);
        }
        for i in 0..self.frame_count {
            let frame = unsafe { self.frames[i].assume_init_ref() };
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

        let function_key = match heap
            .arena
            .get(closure_key)
            .expect("closure missing from arena")
        {
            Object::Closure(closure) => closure.function,
            _ => unreachable!(),
        };

        match heap
            .arena
            .get(function_key)
            .expect("function missing from arena")
        {
            Object::Function(function) => {
                if function.arity as usize != arg_count {
                    return Err(InterpretError::ArgumentsCountMismatch {
                        message: format!(
                            "Expected {} arguments but got {}.",
                            function.arity, arg_count
                        ),
                    });
                }

                let slot_start = self.stack_top.checked_sub(arg_count + 1).unwrap();
                let frame = CallFrame::new(0, closure_key, function_key, slot_start);
                self.push_frame(frame);
                Ok(())
            }
            _ => Err(InterpretError::UncallableObject {}),
        }
    }

    #[inline]
    fn call_value(&mut self, arg_count: usize) -> Result<()> {
        let callee_index = self.stack_top.checked_sub(arg_count + 1).unwrap();
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
                    self.call(key, arg_count)?;
                }
                Object::NativeFunction(native_function) => {
                    let args_start = self.stack_top - arg_count;
                    let result = (native_function.function)(
                        native_function.name,
                        &self.stack[args_start..self.stack_top],
                        heap,
                    )
                    .map_err(|message| InterpretError::NativeFunctionError { message })?;
                    self.stack_top -= arg_count + 1;
                    self.push(result);
                }
                Object::Class(_) => {
                    let maybe_init: Option<HeapKey> = {
                        let heap = self.heap.as_ref().unwrap();
                        let Object::Class(class) = heap.arena.get(key).unwrap() else {
                            unreachable!()
                        };
                        let init_key = self.init_string.unwrap();
                        class.methods.get(&init_key).and_then(|v| {
                            if let Value::Object(k) = v {
                                Some(*k)
                            } else {
                                None
                            }
                        })
                    };

                    let instance = self.heap.as_mut().unwrap().allocate_instance(key);
                    self.stack[self.stack_top - arg_count - 1] = Value::Object(instance);

                    if let Some(closure_key) = maybe_init {
                        self.call(closure_key, arg_count)?;
                    } else if arg_count != 0 {
                        return Err(InterpretError::ArgumentsCountMismatch {
                            message: format!("Expected 0 arguments but got {}.", arg_count),
                        });
                    }
                }
                Object::BoundMethod(bound_method) => {
                    let method = bound_method.method;
                    self.stack[self.stack_top - arg_count - 1] = bound_method.receiver;
                    self.call(method, arg_count)?;
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

    fn bind_method(&mut self, class: HeapKey, name: HeapKey) -> Result<()> {
        let method_val = {
            let heap = self.heap.as_ref().unwrap();
            let Object::Class(obj_class) = heap.arena.get(class).unwrap() else {
                unreachable!()
            };
            obj_class.methods.get(&name).copied() // Value is Copy
        };

        let method_val = method_val.ok_or_else(|| InterpretError::UndefinedProperty {
            identifier: self.key_to_string(name),
        })?;

        let Value::Object(method_key) = method_val else {
            unreachable!("method must be a closure object")
        };

        let receiver = self.stack[self.stack_top - 1];
        let bound = self
            .heap
            .as_mut()
            .unwrap()
            .allocate_bound_method(receiver, method_key);

        self.pop();
        self.push(Value::Object(bound));
        Ok(())
    }

    fn define_method(&mut self, method_name: HeapKey) -> Result<()> {
        let method = self.stack[self.stack_top - 1];
        if let Value::Object(obj_key) = &self.stack[self.stack_top - 2] {
            if let Object::Class(class) =
                self.heap.as_mut().unwrap().arena.get_mut(*obj_key).unwrap()
            {
                class.methods.insert(method_name, method);
                self.pop();
            }
        }
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        loop {
            self.collect_garbage();

            let frame_index = self.frame_count - 1;
            let frame = unsafe { self.frames[frame_index].assume_init_mut() };

            let heap = self.heap.as_ref().unwrap();
            let function = unsafe {
                match heap.arena.get_unchecked(frame.function_key) {
                    Object::Function(f) => f,
                    _ => core::hint::unreachable_unchecked(),
                }
            };

            let chunk = &function.chunk;
            let stack = &mut self.stack;
            let instruction = unsafe { chunk.instructions.get_unchecked(frame.ip) };

            frame.ip += 1;

            #[cfg(feature = "trace")]
            println!("Gutting VM's stack");

            #[cfg(feature = "trace")]
            if self.stack_top == 0 {
                println!("Stack is Empty!");
            } else {
                for value in &self.stack[..self.stack_top] {
                    println!("[ {:?} ]", value);
                }
            }

            #[cfg(feature = "trace")]
            diassemble_instruction(chunk, frame.ip);

            match instruction {
                Instruction::Invoke(name_pos, arg_count) => {
                    let arg_count = *arg_count;

                    let name_key = {
                        let chunk_val = {
                            let heap = self.heap.as_ref().unwrap();
                            let frame =
                                unsafe { self.frames[self.frame_count - 1].assume_init_ref() };
                            let function = match heap.arena.get(frame.function_key).unwrap() {
                                Object::Function(f) => f,
                                _ => unreachable!(),
                            };
                            function.chunk.values[*name_pos]
                        };
                        if let Value::Object(k) = chunk_val {
                            k
                        } else {
                            unreachable!()
                        }
                    };

                    let receiver = stack[self.stack_top - arg_count - 1];

                    let Value::Object(instance_key) = receiver else {
                        return Err(InterpretError::TypeError {
                            message: "Only instances have methods.".to_string(),
                        });
                    };

                    let (field_val, class_key) = {
                        let heap = self.heap.as_ref().unwrap();
                        match heap.arena.get(instance_key).unwrap() {
                            Object::Instance(instance) => {
                                (instance.fields.get(&name_key).copied(), instance.class)
                            }
                            _ => {
                                return Err(InterpretError::TypeError {
                                    message: "Only instances have methods.".to_string(),
                                });
                            }
                        }
                    };

                    if let Some(field_val) = field_val {
                        self.stack[self.stack_top - arg_count - 1] = field_val;
                        self.call_value(arg_count)?;
                    } else {
                        let closure_key = {
                            let heap = self.heap.as_ref().unwrap();
                            let Object::Class(class) = heap.arena.get(class_key).unwrap() else {
                                unreachable!()
                            };
                            class.methods.get(&name_key).and_then(|v| {
                                if let Value::Object(k) = v {
                                    Some(*k)
                                } else {
                                    None
                                }
                            })
                        };

                        let closure_key =
                            closure_key.ok_or_else(|| InterpretError::UndefinedProperty {
                                identifier: self.key_to_string(name_key),
                            })?;

                        self.call(closure_key, arg_count)?;
                    }
                }

                Instruction::Method(pos) => {
                    if let Value::Object(method_name_key) = chunk.values[*pos] {
                        self.define_method(method_name_key)?;
                    }
                }
                Instruction::GetProperty(pos) => {
                    if let Value::Object(instance_key) = stack[self.stack_top - 1] {
                        let field_name_key = if let Value::Object(k) = chunk.values[*pos] {
                            k
                        } else {
                            unreachable!()
                        };

                        let (field_val, class_key) = {
                            let heap = self.heap.as_ref().unwrap();
                            match heap.arena.get(instance_key).unwrap() {
                                Object::Instance(instance) => (
                                    instance.fields.get(&field_name_key).copied(),
                                    instance.class,
                                ),
                                _ => {
                                    return Err(InterpretError::TypeError {
                                        message: "Only instances have properties.".to_string(),
                                    });
                                }
                            }
                        };

                        if let Some(val) = field_val {
                            self.stack_top -= 1;
                            self.push(val);
                        } else {
                            self.bind_method(class_key, field_name_key)?;
                        }
                    } else {
                        return Err(InterpretError::TypeError {
                            message: "Only instances have properties.".to_string(),
                        });
                    }
                }

                Instruction::SetProperty(pos) => {
                    let instance_val = stack[self.stack_top - 2];

                    if let Value::Object(instance_key) = instance_val {
                        let field_name_key = if let Value::Object(k) = chunk.values[*pos] {
                            k
                        } else {
                            unreachable!()
                        };

                        let heap = self.heap.as_mut().unwrap();

                        match heap.arena.get_mut(instance_key).unwrap() {
                            Object::Instance(instance) => {
                                let value = stack[self.stack_top - 1];
                                instance.fields.insert(field_name_key, value);

                                let value = self.pop();
                                self.stack_top -= 1;
                                self.push(value);
                            }
                            _ => {
                                return Err(InterpretError::TypeError {
                                    message: "Only instances have fields.".to_string(),
                                });
                            }
                        }
                    } else {
                        return Err(InterpretError::TypeError {
                            message: "Only instances have fields.".to_string(),
                        });
                    }
                }
                Instruction::Class(slot) => {
                    let class_name = chunk.values[*slot];
                    if let Value::Object(name_key) = class_name {
                        let class_key = self.heap.as_mut().unwrap().allocate_class(name_key);
                        self.push(Value::Object(class_key));
                    }
                }
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
                            UpvalueState::Open(idx) => stack[idx],
                            UpvalueState::Closed(v) => v,
                        },
                        _ => unreachable!(),
                    };
                    stack[self.stack_top] = val;
                    self.stack_top += 1;
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

                    let val = stack[self.stack_top - 1];
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
                    let top_idx = self.stack_top - 1;
                    self.close_upvalues(top_idx);
                    self.stack_top -= 1;
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
                        self.push(Value::Object(closure_key));
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
                    let is_falsey = match stack[self.stack_top - 1] {
                        Value::Boolean(boolean) => !boolean,
                        Value::Nil => true,
                        _ => false,
                    };

                    if is_falsey {
                        frame.ip += offset;
                    }
                }

                Instruction::SetLocal(pos) => {
                    let val = stack[self.stack_top - 1];
                    stack[frame.slot_start + pos] = val;
                }

                Instruction::GetLocal(pos) => {
                    let val = stack[frame.slot_start + pos];
                    stack[self.stack_top] = val;
                    self.stack_top += 1;
                }

                Instruction::SetGlobal(pos) => {
                    if let Value::Object(key) = chunk.values[*pos] {
                        let val = stack[self.stack_top - 1];
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
                        self.stack_top -= 1;
                        let val = stack[self.stack_top];
                        self.heap.as_mut().unwrap().globals.insert(key, val);
                    }
                }

                Instruction::GetGlobal(pos) => {
                    if let Value::Object(key) = chunk.values[*pos] {
                        match self.heap.as_ref().unwrap().globals.get(&key) {
                            Some(&val) => {
                                stack[self.stack_top] = val;
                                self.stack_top += 1;
                            }
                            None => {
                                return Err(InterpretError::UndefinedVariable {
                                    identifier: self.key_to_string(key),
                                });
                            }
                        }
                    }
                }

                Instruction::Pop => {
                    self.stack_top -= 1;
                }

                Instruction::Constant(pos) => {
                    let val = chunk.values[*pos];
                    stack[self.stack_top] = val;
                    self.stack_top += 1;
                }

                Instruction::True => {
                    stack[self.stack_top] = Value::Boolean(true);
                    self.stack_top += 1;
                }

                Instruction::False => {
                    stack[self.stack_top] = Value::Boolean(false);
                    self.stack_top += 1;
                }

                Instruction::Nil => {
                    stack[self.stack_top] = Value::Nil;
                    self.stack_top += 1;
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
                    self.stack_top -= 1;
                    let result = self.stack[self.stack_top];
                    let frame = self.pop_frame();
                    if self.frame_count == 0 {
                        self.stack_top -= 1;
                        return Ok(());
                    }
                    let slot_start = frame.slot_start;
                    self.close_upvalues(slot_start);
                    self.stack_top = slot_start;
                    self.stack[self.stack_top] = result;
                    self.stack_top += 1;
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
        self.push(Value::Object(closure_key));

        let init_string_key = heap.allocate_or_intern_string("init");

        self.heap = Some(heap);
        self.init_string = Some(init_string_key);

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
        let val = self.pop();

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

        self.push(result);
        Ok(())
    }

    #[inline(always)]
    fn binary_op(&mut self, op: Instruction) -> Result<()> {
        let b = self.pop();
        let a = self.pop();

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

        self.push(result);
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
        for i in (0..self.frame_count).rev() {
            let frame = unsafe { self.frames[i].assume_init_ref() };
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
