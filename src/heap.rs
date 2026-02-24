use slotmap::{DefaultKey, SlotMap};

#[derive(Debug)]
pub enum Object {
    String { value: String },
}

pub struct Heap {
    pub arena: SlotMap<DefaultKey, Object>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            arena: SlotMap::new(),
        }
    }
}
