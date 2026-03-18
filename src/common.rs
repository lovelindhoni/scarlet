use crate::{
    error::InterpretError,
    heap::{Heap, HeapKey, ObjFunction, Object, Upvalue},
};

#[derive(Copy, Clone)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
    Object(HeapKey),
}

#[inline(always)]
pub fn get_obj_key(value: &Value) -> HeapKey {
    let Value::Object(obj_key) = value else {
        unreachable!("should have been a Object Value here!")
    };
    *obj_key
}

impl Value {
    fn print_function(&self, function: &ObjFunction, heap: &Heap) -> String {
        let fn_name = if let Some(fn_name_key) = function.name {
            heap.get_string(fn_name_key)
        } else {
            "<script>"
        };
        format!("fn {}()", fn_name)
    }
    pub fn display(&self, heap: &Heap) -> String {
        match self {
            Value::Number(num) => num.to_string(),
            Value::Boolean(boolean) => boolean.to_string(),
            Value::Nil => "nil".to_string(),
            Value::Object(heap_key) => {
                let object = heap.arena.get(*heap_key).unwrap();
                match object {
                    Object::Upvalue(_) => "upvalue".to_string(),
                    Object::String(string) => string.to_owned(),
                    Object::Function(function) => self.print_function(function, heap),
                    Object::Closure(closure) => {
                        let function = heap.get_function(closure.function);
                        self.print_function(function, heap)
                    }
                    Object::NativeFunction(native_function) => {
                        format!("fn {}()", native_function.name)
                    }
                    Object::Instance(instance) => {
                        let class = heap.get_class(instance.class);
                        let name = heap.get_string(class.name);
                        format!("{} instance", name)
                    }
                    Object::Class(obj_class) => {
                        let class_name = heap.get_string(obj_class.name);
                        format!("class {} {{}}", class_name)
                    }
                    Object::BoundMethod(bound_method) => {
                        let closure = heap.get_closure(bound_method.method);
                        let function = heap.get_function(closure.function);
                        self.print_function(function, heap)
                    }
                }
            }
        }
    }
}

pub fn validate_int(val: f64) -> Result<i64, InterpretError> {
    if !val.is_finite() {
        return Err(InterpretError::TypeError {
            message: "Operand must be finite for bitwise operations".to_string(),
        });
    }

    if val.fract() != 0.0 {
        return Err(InterpretError::TypeError {
            message: "Operands must be numbers with no frational part for bitwise operations"
                .to_string(),
        });
    }

    if val < i64::MIN as f64 || val > i64::MAX as f64 {
        return Err(InterpretError::TypeError {
            message: "Number out of range for bitwise operations".to_string(),
        });
    }

    Ok(val as i64)
}

#[derive(Clone)]
pub enum Instruction {
    Constant(usize),
    DefineGlobal(usize),
    GetGlobal(usize),
    SetGlobal(usize),
    GetLocal(usize),
    SetLocal(usize),
    JumpIfFalse(usize),
    Jump(usize),
    Loop(usize),
    Call(usize),
    Closure(usize, Box<[Upvalue]>),
    SetUpvalue(usize),
    GetUpvalue(usize),
    Class(usize),
    GetProperty(usize),
    SetProperty(usize),
    Method(usize),
    Invoke(usize, usize),
    GetSuper(usize),
    SuperInvoke(usize, usize),
    ShiftLeft,
    ShiftRight,
    BitXor,
    BitAnd,
    BitOr,
    CloseUpvalue,
    Inherit,
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
    Pop,
}

impl Instruction {
    pub fn opcode(&self) -> &'static str {
        match self {
            Instruction::Constant(_) => "CONSTANT",
            Instruction::DefineGlobal(_) => "DEFINE_GLOBAL",
            Instruction::GetGlobal(_) => "GET_GLOBAL",
            Instruction::SetGlobal(_) => "SET_GLOBAL",
            Instruction::GetLocal(_) => "GET_LOCAL",
            Instruction::SetLocal(_) => "SET_LOCAL",

            Instruction::GetUpvalue(_) => "GET_UPVALUE",
            Instruction::SetUpvalue(_) => "SET_UPVALUE",

            Instruction::Jump(_) => "JUMP",
            Instruction::JumpIfFalse(_) => "JUMP_IF_FALSE",
            Instruction::Loop(_) => "LOOP",

            Instruction::Call(_) => "CALL",
            Instruction::Closure(_, _) => "CLOSURE",
            Instruction::Invoke(_, _) => "INVOKE",
            Instruction::SuperInvoke(_, _) => "SUPER_INVOKE",

            Instruction::Class(_) => "CLASS",
            Instruction::GetProperty(_) => "GET_PROPERTY",
            Instruction::SetProperty(_) => "SET_PROPERTY",

            Instruction::Method(_) => "METHOD",
            Instruction::GetSuper(_) => "GET_SUPER",

            Instruction::CloseUpvalue => "CLOSE_UPVALUE",

            Instruction::True => "TRUE",
            Instruction::False => "FALSE",
            Instruction::Nil => "NIL",
            Instruction::Return => "RETURN",

            Instruction::BitOr => "BITWISE_OR",
            Instruction::BitXor => "BITWISE_XOR",
            Instruction::BitAnd => "BITWISE_AND",
            Instruction::ShiftLeft => "BITWISE_SHIFTLEFT",
            Instruction::ShiftRight => "BITWISE_SHIFTRIGHT",

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

            Instruction::Pop => "POP",

            Instruction::Inherit => "INHERIT",
        }
    }
}
