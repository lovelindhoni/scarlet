use rapidhash::RapidHashMap;
use slotmap::{SlotMap, new_key_type};

use crate::{chunk::Chunk, common::Value};

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
pub struct ObjUpvalue {
    pub location: usize,
}

#[repr(u8)]
pub enum FunctionType {
    Function,
    Script,
}

pub struct Heap {
    pub arena: SlotMap<HeapKey, Object>,
    pub intern_table: RapidHashMap<String, HeapKey>,
    pub globals: RapidHashMap<HeapKey, Value>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            arena: SlotMap::with_key(),
            intern_table: RapidHashMap::default(),
            globals: RapidHashMap::default(),
        }
    }
    pub fn allocate_function(&mut self, name: Option<String>) -> HeapKey {
        let function_name = if let Some(name) = name {
            Some(self.allocate_or_intern_string(&name))
        } else {
            None
        };
        let function = Object::Function(ObjFunction::new(0, Chunk::new("Function"), function_name));
        self.arena.insert(function)
    }

    pub fn allocate_closure(&mut self, function: HeapKey, upvalues: Vec<HeapKey>) -> HeapKey {
        // takes a normal function pointer and returns a closure pointer
        let closure = ObjClosure { function, upvalues };
        self.arena.insert(Object::Closure(closure))
    }

    pub fn allocate_native_function(&mut self, name: &'static str, function: NativeFn) -> HeapKey {
        let object = Object::NativeFunction(NativeObjFunction { name, function });
        self.arena.insert(object)
    }

    pub fn allocate_or_intern_string(&mut self, string: &str) -> HeapKey {
        if let Some(&key) = self.intern_table.get(string) {
            key
        } else {
            let key = self.arena.insert(Object::String(string.into()));
            self.intern_table.insert(string.into(), key);
            key
        }
    }

    pub fn allocate_upvalue(&mut self, slot: usize) -> HeapKey {
        let upvalue = ObjUpvalue { location: slot };
        let object = Object::Upvalue(upvalue);
        self.arena.insert(object)
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
