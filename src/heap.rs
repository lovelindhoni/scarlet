use rapidhash::RapidHashMap;
use slotmap::{DefaultKey, SlotMap};

use crate::common::Value;

#[derive(Debug)]
pub enum Object {
    String { value: String },
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
}
