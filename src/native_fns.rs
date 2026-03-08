use crate::{
    common::Value,
    heap::{Heap, Object},
};

use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Result = std::result::Result<Value, String>;

pub fn clock(args: &[Value], _heap: &mut Heap) -> Result {
    let fn_name = "clock";
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

pub fn print_ln(args: &[Value], _heap: &mut Heap) -> Result {
    for (i, value) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{:?}", value);
    }
    println!();
    Ok(Value::Nil)
}

pub fn print(args: &[Value], _heap: &mut Heap) -> Result {
    for (i, value) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{:?}", value);
    }
    Ok(Value::Nil)
}

pub fn len(args: &[Value], heap: &mut Heap) -> Result {
    let fn_name = "len";
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

pub fn type_of(args: &[Value], heap: &mut Heap) -> Result {
    let fn_name = "type";
    check_arguments_len(1, args.len(), fn_name)?;

    let value_type = match args[0] {
        Value::Number(_) => "number",
        Value::Boolean(_) => "boolean",
        Value::Nil => "nil",
        Value::Object(key) => {
            let object = heap.arena.get(key).unwrap();
            match object {
                Object::NativeFunction(_) => "native-function",
                Object::String(_) => "string",
                Object::Function(_) => "function",
            }
        }
    };
    let key = heap.allocate_or_intern_string(value_type);
    Ok(Value::Object(key))
}

pub fn sleep(args: &[Value], _heap: &mut Heap) -> Result {
    let fn_name = "sleep";
    check_arguments_len(1, args.len(), fn_name)?;

    if let Value::Number(duration) = args[0] {
        thread::sleep(Duration::from_millis(duration as u64));
        Ok(Value::Nil)
    } else {
        Err(format!(
            "{}() takes an argument of type 'number' - found '{:?}'",
            fn_name, args[0]
        ))
    }
}

pub fn to_string(args: &[Value], heap: &mut Heap) -> Result {
    let fn_name = "to_string";
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

pub fn to_number(args: &[Value], heap: &mut Heap) -> Result {
    let fn_name = "to_number";
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
            let object = heap.arena.get(key).unwrap();
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
