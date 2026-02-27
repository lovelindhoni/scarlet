use std::fmt::{Display, Formatter};

use crate::{
    error::{HeapError, RuntimeError},
    heap::{Heap, Object},
};
use slotmap::DefaultKey;

type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
    Object(DefaultKey),
}

impl Value {
    pub fn add(self, right_operand: Value, line: u64, heap: &mut Heap) -> Result<Value> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Number(left_num + right_num))
            }
            (Value::Object(left_key), Value::Object(right_key)) => {
                let left_object = heap.arena.get(left_key).ok_or(HeapError::ExpiredArenaKey)?;
                let right_object = heap
                    .arena
                    .get(right_key)
                    .ok_or(HeapError::ExpiredArenaKey)?;
                match (left_object, right_object) {
                    (Object::String { value: left }, Object::String { value: right }) => {
                        let concantated_string = left.to_owned() + right;
                        let key = if heap.intern_table.contains_key(&concantated_string) {
                            let interned_key = heap.intern_table[&concantated_string];
                            let object = heap
                                .arena
                                .get(interned_key)
                                .ok_or(HeapError::ExpiredArenaKey)?;
                            match object {
                                Object::String { value } => {
                                    if value != &concantated_string {
                                        return Err(HeapError::InvalidInternedKey {
                                            expected: concantated_string,
                                            found: value.to_owned(),
                                        }
                                        .into());
                                    }
                                }
                            }
                            interned_key
                        } else {
                            let interned_key = heap.arena.insert(Object::String {
                                value: concantated_string.clone(),
                            });
                            heap.intern_table.insert(concantated_string, interned_key);
                            interned_key
                        };
                        Ok(Value::Object(key))
                    }
                }
            }
            _ => Err(RuntimeError::TypeError {
                line,
                message: String::from(
                    "Both of the operands need to be either numbers or strings for addition",
                ),
            }),
        }
    }
    pub fn subtract(self, right_operand: Value, line: u64) -> Result<Value> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Number(left_num - right_num))
            }
            _ => Err(RuntimeError::TypeError {
                line,
                message: String::from("The operands need to be numbers for subtraction"),
            }),
        }
    }
    pub fn multiply(self, right_operand: Value, line: u64) -> Result<Value> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Number(left_num * right_num))
            }
            _ => Err(RuntimeError::TypeError {
                line,
                message: String::from("The operands need to be numbers for multiplication"),
            }),
        }
    }
    pub fn modulo(self, right_operand: Value, line: u64) -> Result<Value> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Number(left_num % right_num))
            }
            _ => Err(RuntimeError::TypeError {
                line,
                message: String::from("The operands need to be numbers for modulus"),
            }),
        }
    }
    pub fn greater(self, right_operand: Value, line: u64) -> Result<Value> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Boolean(left_num > right_num))
            }
            _ => Err(RuntimeError::TypeError {
                line,
                message: String::from(
                    "Greater than(>) comparison can only be done between numbers",
                ),
            }),
        }
    }
    pub fn less(self, right_operand: Value, line: u64) -> Result<Value> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Boolean(left_num < right_num))
            }
            _ => Err(RuntimeError::TypeError {
                line,
                message: String::from("Lesser than(<) comparison can only be done between numbers"),
            }),
        }
    }

    pub fn equal(self, right_operand: Value) -> Result<Value> {
        Ok(match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Value::Boolean(left_num == right_num)
            }
            (Value::Boolean(left_bool), Value::Boolean(right_bool)) => {
                Value::Boolean(left_bool == right_bool)
            }
            (Value::Nil, Value::Nil) => Value::Boolean(true),
            (Value::Object(left_key), Value::Object(right_key)) => {
                Value::Boolean(left_key == right_key)
            }
            _ => Value::Boolean(false),
        })
    }
    pub fn divide(self, right_operand: Value, line: u64) -> Result<Value> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                if right_num == 0.0 {
                    Err(RuntimeError::DivisionByZero {
                        line,
                        left_num,
                        right_num,
                    })
                } else {
                    Ok(Value::Number(left_num / right_num))
                }
            }
            _ => Err(RuntimeError::TypeError {
                line,
                message: String::from("The operands need to be numbers for division"),
            }),
        }
    }
    pub fn not(self, line: u64) -> Result<Value> {
        match self {
            Value::Nil => Ok(Value::Boolean(true)),
            Value::Boolean(boolean) => Ok(Value::Boolean(!boolean)),
            _ => Err(RuntimeError::TypeError {
                line,
                message: String::from("Logical Not(!) can only be done on Nil and Boolean values"),
            }),
        }
    }
    pub fn negate(&mut self, line: u64) -> Result<()> {
        match self {
            Value::Number(num) => {
                *num = -*num;
                Ok(())
            }
            _ => Err(RuntimeError::TypeError {
                message: String::from("Only numbers can be negated"),
                line,
            }),
        }
    }
}

pub enum Instruction {
    Constant(usize),
    DefineGlobal(usize),
    GetGlobal(usize),
    SetGlobal(usize),
    True,
    False,
    Nil,
    Return,
    Negate,
    Add,
    Subtract,
    Multiply,
    Modulo,
    Divide,
    Not,
    Equal,
    Greater,
    Less,
    Print,
    Pop,
}

impl Instruction {
    pub fn opcode(&self) -> &'static str {
        match self {
            Instruction::Constant(_) => "CONSTANT",
            Instruction::DefineGlobal(_) => "DEFINE_GLOBAL",
            Instruction::GetGlobal(_) => "GET_GLOBAL",
            Instruction::SetGlobal(_) => "SET_GLOBAL",

            Instruction::True => "TRUE",
            Instruction::False => "FALSE",
            Instruction::Nil => "NIL",
            Instruction::Return => "RETURN",

            Instruction::Negate => "NEGATE",
            Instruction::Add => "ADD",
            Instruction::Subtract => "SUBTRACT",
            Instruction::Multiply => "MULTIPLY",
            Instruction::Modulo => "MODULO",
            Instruction::Divide => "DIVIDE",

            Instruction::Not => "NOT",
            Instruction::Equal => "EQUAL",
            Instruction::Greater => "GREATER",
            Instruction::Less => "LESS",

            Instruction::Print => "PRINT",
            Instruction::Pop => "POP",
        }
    }
}
