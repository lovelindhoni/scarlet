use rapidhash::RapidHashMap;
use slotmap::{DefaultKey, SlotMap};

use crate::{chunk::Chunk, common::Value};

#[derive(Debug)]
pub enum Object {
    String(String),
    Function(ObjFunction),
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
    pub globals: RapidHashMap<String, Value>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            arena: SlotMap::new(),
            intern_table: RapidHashMap::default(),
            globals: RapidHashMap::default(),
        }
    }
    pub fn new_function(&mut self, name: Option<String>) -> DefaultKey {
        let function_name = if let Some(name) = name {
            if self.intern_table.contains_key(&name) {
                Some(self.intern_table[&name])
            } else {
                let key = self.arena.insert(Object::String(name.clone()));
                self.intern_table.insert(name, key);
                Some(key)
            }
        } else {
            None
        };
        let function = Object::Function(ObjFunction::new(0, Chunk::new("Function"), function_name));
        self.arena.insert(function)
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

        if let Some(&existing_key) = self.intern_table.get(&result_str) {
            existing_key
        } else {
            let new_key = self.arena.insert(Object::String(result_str.clone()));
            self.intern_table.insert(result_str, new_key);
            new_key
        }
    }
}
