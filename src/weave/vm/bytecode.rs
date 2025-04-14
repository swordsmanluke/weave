use std::fmt::Write;
use std::fmt::Display;
use crate::weave::Chunk;
use crate::weave::vm::dissassemble::Disassemble;

pub enum Op {
    CONSTANT,  // Always 64 bit, but the type is variable
    RETURN,
}

impl Op {
    pub fn bytecode(&self) -> Vec<u8> {
        match self {
            Op::RETURN => vec![0],
            Op::CONSTANT => vec![1],
        }
    }

    /// Used for deassembling, reads the opcode at the given offset to determine what it is
    pub fn at(byte: u8) -> Op {
        match byte {
            0 => Op::RETURN,
            1 => Op::CONSTANT,
            _ => panic!("Unknown opcode"), // Should never happen, but when it does - die.
        }
    }
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::RETURN => write!(f, "RETURN"),
            Op::CONSTANT => write!(f, "CONSTANT"),
        }
    }
}

impl Disassemble for Op {
    fn disassemble(&self, offset: usize, chunk: &Chunk, f: &mut String) -> usize {
        match self {
            Op::RETURN => {
                write!(f, "{0:04x}  {1}  RETURN\n", offset, chunk.line_str(offset)).unwrap();
                offset + 1  // Return our size - 1 byte
            },
            Op::CONSTANT => {
                write!(f, "{0:04x}  {1}  CONSTANT", offset, chunk.line_str(offset)).unwrap();
                // Next ,grab the following 8 bytes and convert them to a value... might need
                // some tracking to do this right. Separate Opcodes for encoding [u]int|floats?
                let mut offset = offset + 1; // Skip the opcode, already consumed
                let idx = chunk.code[offset] as usize;

                // Convert the next 8 bytes to a f64
                let value = &chunk.constants.values[idx];
                write!(f, "\t{0:04x}  {1}\n", idx, value).unwrap();
                offset += 1;
                offset
            },
        }
    }
}
