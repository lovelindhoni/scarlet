use crate::heap::{Heap, HeapKey, ObjFunction, Object, Upvalue};

#[derive(Copy, Clone)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
    Object(HeapKey),
}

impl Value {
    fn print_function(&self, function: &ObjFunction, heap: &Heap) -> String {
        let fn_name = if let Some(fn_name_key) = function.name {
            let object = heap.arena.get(fn_name_key).unwrap();
            if let Object::String(fn_name) = object {
                fn_name
            } else {
                unreachable!()
            }
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
                        if let Object::Function(function) =
                            heap.arena.get(closure.function).unwrap()
                        {
                            self.print_function(function, heap)
                        } else {
                            unreachable!() // hopefully :|
                        }
                    }
                    Object::NativeFunction(native_function) => {
                        format!("fn {}()", native_function.name)
                    }
                    Object::Instance(instance) => {
                        let name =
                            if let Object::Class(class) = heap.arena.get(instance.class).unwrap() {
                                if let Object::String(name) = heap.arena.get(class.name).unwrap() {
                                    name
                                } else {
                                    unreachable!()
                                }
                            } else {
                                unreachable!();
                            };
                        format!("{} instance", name)
                    }
                    Object::Class(obj_class) => {
                        if let Object::String(class_name) = heap.arena.get(obj_class.name).unwrap()
                        {
                            format!("class {} {{}}", class_name)
                        } else {
                            unreachable!()
                        }
                    }
                    Object::BoundMethod(bound_method) => {
                        if let Object::Closure(closure) =
                            heap.arena.get(bound_method.method).unwrap()
                        {
                            if let Object::Function(function) =
                                heap.arena.get(closure.function).unwrap()
                            {
                                return self.print_function(function, heap);
                            }
                        }
                        unreachable!()
                    }
                }
            }
        }
    }
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
