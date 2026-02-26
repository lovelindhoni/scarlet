use slotmap::{DefaultKey, SlotMap};
use std::collections::hash_map::HashMap;

use crate::common::Value;

#[derive(Debug)]
pub enum Object {
    String { value: String },
}

pub struct Heap {
    pub arena: SlotMap<DefaultKey, Object>,
    pub intern_table: HashMap<String, DefaultKey>,
    pub globals: HashMap<String, Value>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            arena: SlotMap::new(),
            intern_table: HashMap::new(),
            globals: HashMap::new(),
        }
    }
}
