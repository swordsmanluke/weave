use std::fmt::Write;
use std::fmt::Display;
use crate::weave::vm::dissassemble::Disassemble;

pub enum Op {
    RETURN,
}

impl Op {
    pub fn bytecode(&self) -> Vec<u8> {
        match self {
            Op::RETURN => vec![0],
        }
    }

    pub fn from_bytes(bytes: &Vec<u8>, offset: usize) -> Op {
        let opcode = match bytes[offset] {
            0 => Op::RETURN,
            _ => panic!("Unknown opcode"),
        };
        opcode
    }
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::RETURN => write!(f, "RETURN"),
        }
    }
}

impl Disassemble for Op {
    fn disassemble(&self, offset: usize, f: &mut String) -> usize {
        match self {
            Op::RETURN => {
                write!(f, "{0:04x}    RETURN\n", offset).unwrap();
                1  // Return our size - 1 byte
            },
        }
    }
}
