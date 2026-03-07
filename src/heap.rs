use rapidhash::RapidHashMap;
use slotmap::{DefaultKey, SlotMap};

use crate::{chunk::Chunk, common::Value};

pub type NativeFn = fn(args: &[Value], heap: &mut Heap) -> Result<Value, String>;

#[derive(Debug)]
pub enum Object {
    String(String),
    Function(ObjFunction),
    NativeFunction(NativeFn),
}

#[derive(Debug)]
pub struct ObjFunction {
    pub arity: u64,
    pub chunk: Chunk,
    pub name: Option<DefaultKey>,
}

impl ObjFunction {
    pub fn new(arity: u64, chunk: Chunk, name: Option<DefaultKey>) -> Self {
        Self { arity, chunk, name }
    }
}
pub enum FunctionType {
    Function,
    Script,
}

pub struct Heap {
    pub arena: SlotMap<DefaultKey, Object>,
    pub intern_table: RapidHashMap<String, DefaultKey>,
    pub globals: RapidHashMap<DefaultKey, Value>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            arena: SlotMap::new(),
            intern_table: RapidHashMap::default(),
            globals: RapidHashMap::default(),
        }
    }
    pub fn create_function(&mut self, name: Option<String>) -> DefaultKey {
        let function_name = if let Some(name) = name {
            Some(self.create_or_intern_string(&name))
        } else {
            None
        };
        let function = Object::Function(ObjFunction::new(0, Chunk::new("Function"), function_name));
        self.arena.insert(function)
    }

    pub fn create_native_function(&mut self, function: NativeFn) -> DefaultKey {
        let object = Object::NativeFunction(function);
        self.arena.insert(object)
    }

    pub fn create_or_intern_string(&mut self, string: &str) -> DefaultKey {
        if let Some(&key) = self.intern_table.get(string) {
            key
        } else {
            let key = self.arena.insert(Object::String(string.into()));
            self.intern_table.insert(string.into(), key);
            key
        }
    }

    pub fn concatenate_strings(
        &mut self,
        left_key: DefaultKey,
        right_key: DefaultKey,
    ) -> DefaultKey {
        let left_str = match self.arena.get(left_key) {
            Some(Object::String(value)) => value,
            _ => unreachable!(),
        };
        let right_str = match self.arena.get(right_key) {
            Some(Object::String(value)) => value,
            _ => unreachable!(),
        };
        let result_str = format!("{}{}", left_str, right_str);

        self.create_or_intern_string(&result_str)
    }
}
