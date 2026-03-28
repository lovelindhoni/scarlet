use rapidhash::RapidHashMap;

use crate::{
    common::Value,
    heap::{Heap, HeapKey, NativeFn, Object},
    log_print, log_println,
};

#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Result = std::result::Result<Value, String>;

#[cfg(not(target_arch = "wasm32"))]
const NATIVES: &[(&str, NativeFn)] = &[
    ("clock", clock),
    ("print", print),
    ("println", print_ln),
    ("len", len),
    ("type", type_of),
    ("sleep", sleep),
    ("to_string", to_string),
    ("to_number", to_number),
    ("read", read),
];

#[cfg(target_arch = "wasm32")]
const NATIVES: &[(&str, NativeFn)] = &[
    ("print", print),
    ("println", print_ln),
    ("len", len),
    ("type", type_of),
    ("to_string", to_string),
    ("to_number", to_number),
];

pub fn initialize_native_functions(
    heap: &mut Heap,
    globals_map: &mut RapidHashMap<HeapKey, usize>,
) {
    for (name, func) in NATIVES {
        let name_key = heap.allocate_or_intern_string(name);
        let fn_key = heap.allocate_native_function(name, *func);
        globals_map.insert(name_key, globals_map.len());
        heap.globals.identifiers.push(name_key);
        heap.globals.values.push(Value::Object(fn_key));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn clock(fn_name: &'static str, args: &[Value], _heap: &mut Heap) -> Result {
    check_arguments_len(0, args.len(), fn_name)?;

    let time = Value::Number(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            * 1000.0,
    );

    Ok(time)
}

fn print_values(args: &[Value], heap: &mut Heap) -> String {
    args.iter()
        .enumerate()
        .map(|(i, value)| {
            if i > 0 {
                format!(" {}", value.display(heap))
            } else {
                value.display(heap).to_string()
            }
        })
        .collect()
}

fn print(_fn_name: &'static str, args: &[Value], heap: &mut Heap) -> Result {
    let output = print_values(args, heap);
    log_print!("{}", output);
    Ok(Value::Nil)
}

fn print_ln(_fn_name: &'static str, args: &[Value], heap: &mut Heap) -> Result {
    let output = print_values(args, heap);
    log_println!("{}", output);
    Ok(Value::Nil)
}

fn len(fn_name: &'static str, args: &[Value], heap: &mut Heap) -> Result {
    check_arguments_len(1, args.len(), fn_name)?;
    if let Value::Object(key) = args[0] {
        let object = heap.arena.get(key).unwrap();

        if let Object::String(string) = object {
            Ok(Value::Number(string.len() as f64))
        } else {
            Err(format!("{}() works only on string values", fn_name))
        }
    } else {
        Err(format!("{}() works only on string values", fn_name))
    }
}

fn type_of(fn_name: &'static str, args: &[Value], heap: &mut Heap) -> Result {
    check_arguments_len(1, args.len(), fn_name)?;

    let value_type = match args[0] {
        Value::Number(_) => "number",
        Value::Boolean(_) => "boolean",
        Value::Nil => "nil",
        Value::Object(key) => {
            let object = heap.arena.get(key).unwrap();
            match object {
                Object::NativeFunction(_) => "native-function",
                Object::Upvalue(_) => "upvalue",
                Object::String(_) => "string",
                Object::Class(_) => "class",
                Object::Function(_) | Object::Closure(_) | Object::BoundMethod(_) => "function",
                Object::Instance(_) => "instance",
            }
        }
    };
    let key = heap.allocate_or_intern_string(value_type);
    Ok(Value::Object(key))
}

#[cfg(not(target_arch = "wasm32"))]
fn sleep(fn_name: &'static str, args: &[Value], heap: &mut Heap) -> Result {
    check_arguments_len(1, args.len(), fn_name)?;

    if let Value::Number(duration) = args[0] {
        thread::sleep(Duration::from_millis(duration as u64));
        Ok(Value::Nil)
    } else {
        Err(format!(
            "{}() takes an argument of type 'number' - found '{}'",
            fn_name,
            args[0].display(heap)
        ))
    }
}

fn to_string(fn_name: &'static str, args: &[Value], heap: &mut Heap) -> Result {
    check_arguments_len(1, args.len(), fn_name)?;

    let string = match args[0] {
        Value::Number(num) => num.to_string(),
        Value::Boolean(boolean) => boolean.to_string(),
        Value::Nil => String::from("nil"),
        Value::Object(key) => {
            let object = heap.arena.get(key).unwrap();
            match object {
                Object::String(string) => string.to_owned(),
                _ => {
                    return Err(format!(
                        "{}() can't convert functions into string type",
                        fn_name
                    ));
                }
            }
        }
    };

    let key = heap.allocate_or_intern_string(&string);
    Ok(Value::Object(key))
}

fn to_number(fn_name: &'static str, args: &[Value], heap: &mut Heap) -> Result {
    check_arguments_len(1, args.len(), fn_name)?;

    let num = match args[0] {
        Value::Number(num) => num,
        Value::Boolean(boolean) => {
            if boolean {
                1.0
            } else {
                0.0
            }
        }
        Value::Nil => {
            return Err(format!("{}() can't convert 'nil' into 'number'", fn_name));
        }
        Value::Object(key) => {
            let object = heap.get_obj(key);
            match object {
                Object::String(string) => match string.parse::<f64>() {
                    Ok(n) => n,
                    Err(_) => {
                        return Err(format!("couldn't convert {} into number", string));
                    }
                },
                _ => {
                    return Err(format!("{}() can't convert functions into number", fn_name));
                }
            }
        }
    };

    Ok(Value::Number(num))
}

#[cfg(not(target_arch = "wasm32"))]
fn read(fn_name: &'static str, args: &[Value], heap: &mut Heap) -> Result {
    check_arguments_len(1, args.len(), fn_name)?;

    if let Value::Object(key) = args[0] {
        let prompt = heap.get_string(key);
        print!("{}", prompt);
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| format!("{}() failed to read input: {}", fn_name, e))?;

        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        let key = heap.allocate_or_intern_string(trimmed);
        return Ok(Value::Object(key));
    }

    Err(format!("{}() takes a string argument as prompt", fn_name))
}

fn check_arguments_len(
    expected: usize,
    found: usize,
    fn_name: &str,
) -> std::result::Result<(), String> {
    if found != expected {
        Err(format!(
            "{}() takes {}  arguments - found {}",
            fn_name, expected, found
        ))
    } else {
        Ok(())
    }
}
