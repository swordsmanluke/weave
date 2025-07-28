use std::fmt::Display;
use crate::weave::vm::types::{WeaveType, NanBoxedValue};
use crate::weave::vm::vm::VMError;
use std::time::SystemTime;
use crate::log_debug;

#[derive(Debug, Clone)]
pub enum NativeFnType {
    Input,
    Print,
    Clock,
    ReadFile,
    WriteFile,
}

impl NativeFnType {
    pub fn variants() -> Vec<NativeFnType> {
        vec![NativeFnType::Input, 
             NativeFnType::Print, 
             NativeFnType::Clock, 
             NativeFnType::ReadFile, 
             NativeFnType::WriteFile]
    }
}

#[derive(Debug, Clone)]
pub struct NativeFn {
    pub name: NativeFnType,
    pub arity: usize,
    pub func: fn(&[NanBoxedValue]) -> Result<NanBoxedValue, VMError>,
}

impl NativeFn {
    pub fn get(fn_type: NativeFnType) -> NativeFn {
        match fn_type {
            NativeFnType::Input => NativeFn {
                name: NativeFnType::Input,
                arity: 0,
                func: input,
            },
            NativeFnType::Print => NativeFn {
                name: NativeFnType::Print,
                arity: 1,
                func: print,
            },
            NativeFnType::Clock => NativeFn {
                name: NativeFnType::Clock,
                arity: 0,
                func: clock,
            },
            NativeFnType::ReadFile => NativeFn {
                name: NativeFnType::ReadFile,
                arity: 1,
                func: read_file,
            },
            NativeFnType::WriteFile => NativeFn {
                name: NativeFnType::WriteFile,
                arity: 2,
                func: write_file,
            },
        }
    }
}

impl Display for NativeFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<n.fn: {}({})>", self.name, self.arity)
    }
}

impl Display for NativeFnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeFnType::Input => write!(f, "input"),
            NativeFnType::Print => write!(f, "print"),
            NativeFnType::Clock => write!(f, "clock"),
            NativeFnType::ReadFile => write!(f, "read"),
            NativeFnType::WriteFile => write!(f, "write"),
        }
    }
}

fn print(args: &[NanBoxedValue]) -> Result<NanBoxedValue, VMError> {
    let printable = args
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<String>>()
        .join("");
    log_debug!("Native puts function call", output = printable.as_str());
    println!("{}", printable);
    Ok(NanBoxedValue::null())
}

fn input(_args: &[NanBoxedValue]) -> Result<NanBoxedValue, VMError> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    // TODO: Implement string support in NanBoxedValue
    // For now, return null as placeholder
    Ok(NanBoxedValue::null())
}

fn clock(_args: &[NanBoxedValue]) -> Result<NanBoxedValue, VMError> {
    // Get system time (ms since epoch)
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    Ok(NanBoxedValue::number(time as f64))
}

fn read_file(args: &[NanBoxedValue]) -> Result<NanBoxedValue, VMError> {
    let path = args[0].to_string();
    let contents = std::fs::read_to_string(path).unwrap();
    // TODO: this should be a Container of bytes which we can convert to a Weave String
    //       and/or format with a desired 'formatter' function
    // For now, return null as placeholder until string support is added
    Ok(NanBoxedValue::null())
}

fn write_file(args: &[NanBoxedValue]) -> Result<NanBoxedValue, VMError> {
    let path = args[0].to_string();
    let contents = args[1].to_string();
    std::fs::write(path, contents).unwrap();
    Ok(NanBoxedValue::null())
}
