use rapidhash::{HashMapExt, RapidHashMap};
use slotmap::{SecondaryMap, SlotMap, new_key_type};

use crate::{chunk::Chunk, common::Value};

pub const BASE_GC_TRIGGER: usize = 10 * 1024 * 1024;

pub type NativeFn =
    fn(name: &'static str, args: &[Value], heap: &mut Heap) -> Result<Value, String>;

new_key_type! {
    pub struct HeapKey;
}

#[derive(Debug, Clone)]
pub struct Upvalue {
    pub index: usize,
    pub is_local: bool,
}

impl Upvalue {
    pub fn new(index: usize, is_local: bool) -> Self {
        Self { index, is_local }
    }
}

#[derive(Debug)]
pub enum Object {
    String(String),
    Function(ObjFunction),
    NativeFunction(NativeObjFunction),
    Closure(ObjClosure),
    Upvalue(ObjUpvalue),
    Class(ObjClass),
    Instance(ObjInstance),
}

#[derive(Debug)]
pub struct ObjFunction {
    pub arity: u64,
    pub chunk: Chunk,
    pub name: Option<HeapKey>,
}

#[derive(Debug, Clone)]
pub struct ObjClosure {
    pub function: HeapKey,
    pub upvalues: Vec<HeapKey>, // each points to objupvalue in the heap
}

#[derive(Debug)]
pub struct NativeObjFunction {
    pub name: &'static str,
    pub function: NativeFn,
}

impl ObjFunction {
    pub fn new(arity: u64, chunk: Chunk, name: Option<HeapKey>) -> Self {
        Self { arity, chunk, name }
    }
}

#[derive(Debug)]
pub struct ObjClass {
    pub name: HeapKey,
}

#[derive(Debug)]
pub struct ObjInstance {
    pub class: HeapKey,
    pub fields: RapidHashMap<HeapKey, Value>, // heapkey is objstring heapkey
}

#[derive(Debug)]
#[repr(u8)]
pub enum UpvalueState {
    Open(usize),   // stack slot index
    Closed(Value), // owns the value after variable leaves stack
}

#[derive(Debug)]
pub struct ObjUpvalue {
    pub state: UpvalueState,
}

#[repr(u8)]
pub enum FunctionType {
    Function,
    Script,
}

pub fn mark_value(
    arena: &SlotMap<HeapKey, Object>,
    marked_objects: &mut SecondaryMap<HeapKey, bool>,
    value: &Value,
) {
    if let Value::Object(object_key) = value {
        mark_object(arena, marked_objects, object_key);
    }
}

pub fn mark_object(
    arena: &SlotMap<HeapKey, Object>,
    marked_objects: &mut SecondaryMap<HeapKey, bool>,
    root: &HeapKey,
) {
    // a recursive dfs apporach came to naturally, but i might blow up the call stack for deep graphs of references. so switched to an iterative dfs
    let mut dfs_stack = vec![*root];

    while let Some(key) = dfs_stack.pop() {
        if marked_objects.contains_key(key) {
            continue;
        }
        marked_objects.insert(key, true);

        let object = arena.get(key).unwrap();
        match object {
            Object::Function(function) => {
                if let Some(function_name_key) = &function.name {
                    dfs_stack.push(*function_name_key);
                }
                for value in &function.chunk.values {
                    if let Value::Object(child_key) = value {
                        dfs_stack.push(*child_key);
                    }
                }
            }
            Object::Closure(closure) => {
                dfs_stack.push(closure.function);
                for upvalue in &closure.upvalues {
                    dfs_stack.push(*upvalue);
                }
            }
            Object::Upvalue(upvalue) => {
                if let UpvalueState::Closed(Value::Object(child_key)) = &upvalue.state {
                    dfs_stack.push(*child_key);
                }
            }
            Object::Class(class) => {
                dfs_stack.push(class.name);
            }
            Object::Instance(instance) => {
                dfs_stack.push(instance.class);
                for (identifier_key, value) in &instance.fields {
                    dfs_stack.push(*identifier_key);
                    if let Value::Object(object_key) = value {
                        dfs_stack.push(*object_key);
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct Heap {
    pub arena: SlotMap<HeapKey, Object>,
    pub marked_objects: SecondaryMap<HeapKey, bool>, // marks the reachable objects
    pub intern_table: RapidHashMap<String, HeapKey>,
    pub globals: RapidHashMap<HeapKey, Value>,
    pub bytes_allocated: usize,
    pub next_gc_run: usize,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            arena: SlotMap::with_key(),
            intern_table: RapidHashMap::new(),
            globals: RapidHashMap::new(),
            marked_objects: SecondaryMap::new(),
            bytes_allocated: 0,
            next_gc_run: BASE_GC_TRIGGER,
        }
    }

    pub fn mark_globals(&mut self) {
        for (identifier_key, value) in &self.globals {
            // the key here points to the variable identifier string, so we mark it
            mark_object(&self.arena, &mut self.marked_objects, identifier_key);
            mark_value(&self.arena, &mut self.marked_objects, value);
        }
    }

    pub fn mark_interned_strings(&mut self) {
        self.intern_table
            .retain(|_, key| self.marked_objects.contains_key(*key));
    }

    pub fn sweep(&mut self) {
        let freed: usize = self
            .arena
            .iter()
            .filter(|(key, _)| !self.marked_objects.contains_key(*key))
            .map(|(_, obj)| match obj {
                Object::String(s) => std::mem::size_of::<Object>() + s.capacity(),
                Object::Closure(c) => {
                    std::mem::size_of::<Object>()
                        + (c.upvalues.capacity() * std::mem::size_of::<HeapKey>())
                }
                _ => std::mem::size_of::<Object>(),
            })
            .sum();
        self.bytes_allocated -= freed;

        self.arena.retain(|heap_key, _| {
            let is_marked = self.marked_objects.contains_key(heap_key);
            if is_marked {
                self.marked_objects.remove(heap_key);
            }
            is_marked
        });
    }

    pub fn allocate_function(&mut self, name: Option<String>) -> HeapKey {
        let function_name = if let Some(name) = name {
            Some(self.allocate_or_intern_string(&name))
        } else {
            None
        };
        self.bytes_allocated += std::mem::size_of::<Object>();
        let function = Object::Function(ObjFunction::new(0, Chunk::new(), function_name));
        self.arena.insert(function)
    }

    pub fn allocate_closure(&mut self, function: HeapKey, upvalues: Vec<HeapKey>) -> HeapKey {
        self.bytes_allocated +=
            std::mem::size_of::<Object>() + (upvalues.capacity() * std::mem::size_of::<HeapKey>());
        // takes a normal function key and returns a closure key
        let closure = ObjClosure { function, upvalues };
        self.arena.insert(Object::Closure(closure))
    }

    pub fn allocate_native_function(&mut self, name: &'static str, function: NativeFn) -> HeapKey {
        self.bytes_allocated += std::mem::size_of::<Object>();
        let object = Object::NativeFunction(NativeObjFunction { name, function });
        self.arena.insert(object)
    }

    pub fn allocate_or_intern_string(&mut self, string: &str) -> HeapKey {
        if let Some(&key) = self.intern_table.get(string) {
            key
        } else {
            let string = string.to_owned();
            self.bytes_allocated += std::mem::size_of::<Object>() + string.capacity();
            let key = self.arena.insert(Object::String(string.clone()));
            self.intern_table.insert(string, key);
            key
        }
    }

    pub fn allocate_upvalue(&mut self, slot: usize) -> HeapKey {
        self.bytes_allocated += std::mem::size_of::<Object>();
        self.arena.insert(Object::Upvalue(ObjUpvalue {
            state: UpvalueState::Open(slot),
        }))
    }

    pub fn allocate_class(&mut self, name: HeapKey) -> HeapKey {
        let class = ObjClass { name };
        self.arena.insert(Object::Class(class))
    }

    pub fn allocate_instance(&mut self, class: HeapKey) -> HeapKey {
        let instance = ObjInstance {
            class,
            fields: RapidHashMap::new(),
        };
        self.arena.insert(Object::Instance(instance))
    }

    pub fn concatenate_strings(&mut self, left_key: HeapKey, right_key: HeapKey) -> HeapKey {
        let left_str = match self.arena.get(left_key) {
            Some(Object::String(value)) => value,
            _ => unreachable!(),
        };
        let right_str = match self.arena.get(right_key) {
            Some(Object::String(value)) => value,
            _ => unreachable!(),
        };
        let result_str = format!("{}{}", left_str, right_str);

        self.allocate_or_intern_string(&result_str)
    }
}
