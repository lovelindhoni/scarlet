#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
}

impl Value {
    pub fn add(self, right_operand: Value) -> Result<Value, String> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Number(left_num + right_num))
            }
        }
    }
    pub fn subtract(self, right_operand: Value) -> Result<Value, String> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Number(left_num - right_num))
            }
        }
    }
    pub fn multiply(self, right_operand: Value) -> Result<Value, String> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Number(left_num * right_num))
            }
        }
    }
    pub fn modulo(self, right_operand: Value) -> Result<Value, String> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Number(left_num % right_num))
            }
        }
    }
    pub fn divide(self, right_operand: Value) -> Result<Value, String> {
        match (self, right_operand) {
            (Value::Number(left_num), Value::Number(right_num)) => {
                Ok(Value::Number(left_num / right_num))
            }
        }
    }
    pub fn negate(&mut self) -> Result<String, String> {
        match self {
            Value::Number(num) => {
                *num = -*num;
                Ok(format!("Success"))
            }
        }
    }
}

pub enum Instruction {
    Constant(usize), // Constant variant holds the index of the constant value in the values array
    Return,
    Negate,
    Add,
    Subtract,
    Multiply,
    Modulo,
    Divide,
}
