use crate::error::RuntimeError;

type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
}

impl Value {
    pub fn add(self, right_operand: Value, line: u64) -> Result<Value> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Number(left_num + right_num))
            }
            _ => Err(RuntimeError::TypeError {
                line,
                message: String::from("The operands need to be numbers for division"),
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
                message: String::from("The operands need to be numbers for division"),
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
                message: String::from("The operands need to be numbers for division"),
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
                message: String::from("The operands need to be numbers for division"),
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
            Value::Boolean(_) => Err(RuntimeError::TypeError {
                message: String::from("Can't negate boolean value"),
                line,
            }),
            Value::Nil => Err(RuntimeError::TypeError {
                message: String::from("Can't negate nil value"),
                line,
            }),
        }
    }
}

pub enum Instruction {
    Constant(usize), // Constant variant holds the index of the constant value in the values array
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
}
