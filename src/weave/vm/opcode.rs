use std::fmt::Write;
use std::fmt::Display;
use crate::weave::Chunk;
use crate::weave::vm::traits::disassemble::Disassemble;

pub enum Op {
    CONSTANT,  // Always 64 bit, but the type is variable
    NEGATE,
    ADD,
    SUB,
    MUL,
    DIV,
    RETURN,
}

impl Op {
    pub fn bytecode(&self) -> Vec<u8> {
        match self {
            Op::RETURN => vec![0],
            Op::CONSTANT => vec![1],
            Op::NEGATE => vec![2],
            Op::ADD => vec![3],
            Op::SUB => vec![4],
            Op::MUL => vec![5],
            Op::DIV => vec![6]
        }
    }

    /// Used for deassembling, reads the opcode at the given offset to determine what it is
    pub fn at(byte: u8) -> Op {
        match byte {
            0 => Op::RETURN,
            1 => Op::CONSTANT,
            2 => Op::NEGATE,
            3 => Op::ADD,
            4 => Op::SUB,
            5 => Op::MUL,
            6 => Op::DIV,

            _ => panic!("Unknown opcode"), // Should never happen, but when it does - die.
        }
    }
}

impl Disassemble for Op {
    fn disassemble(&self, offset: usize, chunk: &Chunk, f: &mut String) -> usize {
        match self {
            Op::RETURN => {
                write!(f, "{0:04x}  {1}  RETURN", offset, chunk.line_str(offset)).unwrap();
                offset + 1  // Return our size - 1 byte
            },
            Op::NEGATE => {
                write!(f, "{0:04x}  {1}  NEGATE", offset, chunk.line_str(offset)).unwrap();
                offset + 1
            },
            Op::CONSTANT => {
                write!(f, "{0:04x}  {1}  CONSTANT", offset, chunk.line_str(offset)).unwrap();
                // Next ,grab the following 8 bytes and convert them to a value... might need
                // some tracking to do this right. Separate Opcodes for encoding [u]int|floats?
                let mut offset = offset + 1; // Skip the opcode, already consumed
                let idx = chunk.code[offset] as usize;

                // Convert the next 8 bytes to a f64
                let value = &chunk.constants.values[idx];
                write!(f, "\t{0:04x}  {1}", idx, value).unwrap();
                offset += 1;
                offset
            },
            Op::ADD => {
                write!(f, "{0:04x}  {1}  ADD", offset, chunk.line_str(offset)).unwrap();
                offset + 1
            }
            Op::SUB => {
                write!(f, "{0:04x}  {1}  SUB", offset, chunk.line_str(offset)).unwrap();
                offset + 1
            }
            Op::MUL => {
                write!(f, "{0:04x}  {1}  MUL", offset, chunk.line_str(offset)).unwrap();
                offset + 1
            }
            Op::DIV => {
                write!(f, "{0:04x}  {1}  DIV", offset, chunk.line_str(offset)).unwrap();
                offset + 1
            }
        }
    }
}
