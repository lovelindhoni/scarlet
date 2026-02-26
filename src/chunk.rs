use crate::common::{Instruction, Value};

pub struct Chunk {
    pub name: String,
    pub instructions: Vec<Instruction>,
    pub values: Vec<Value>,
    pub lines: Vec<(usize, u64)>, // run length encoding via AOS
}

impl Chunk {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            instructions: Vec::new(),
            values: Vec::new(),
            lines: Vec::new(),
        }
    }
    fn add_line(&mut self, line: u64) {
        if let Some(last_line) = self.lines.last_mut()
            && last_line.1 == line
        {
            last_line.0 += 1;
        } else {
            self.lines.push((1, line));
        }
    }
    pub fn get_line(&self, instruction_idx: usize) -> u64 {
        let mut count = 0;
        for line in &self.lines {
            count += line.0;
            if count > instruction_idx {
                return line.1;
            }
        }
        // we are updating line information for every new instruction we add, so we would never reach here on good assumption
        u64::MAX
    }
    pub fn write_instruction(&mut self, instruction: Instruction, line: u64) {
        self.instructions.push(instruction);
        self.add_line(line);
    }
    pub fn add_constant(&mut self, value: Value) -> usize {
        self.values.push(value);
        self.values.len() - 1
    }
}
