use std::fmt::Write;
use crate::weave::Chunk;
use crate::weave::vm::traits::disassemble::Disassemble;

#[derive(Debug, PartialEq)]
pub enum Op {
    // Numeric Constants
    CONSTANT,  // TODO: Always 64 bit double right now. Fix that.
    
    // Boolean
    TRUE,
    FALSE,
    NOT,
    
    // Comparison
    GREATER,
    LESS,
    EQUAL,
    
    // Arithmetic
    NEGATE,
    ADD,
    SUB,
    MUL,
    DIV,
    
    // Control
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
            Op::DIV => vec![6],
            Op::TRUE => vec![7],
            Op::FALSE => vec![8],
            Op::NOT => vec![9],
            Op::GREATER => vec![10],
            Op::LESS => vec![11],
            Op::EQUAL => vec![12],
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
            7 => Op::TRUE,
            8 => Op::FALSE,
            9 => Op::NOT,
            10 => Op::GREATER,
            11 => Op::LESS,
            12 => Op::EQUAL,

            _ => panic!("Unknown opcode"), // Should never happen, but when it does - die.
        }
    }
}

impl From<u8> for Op {
    fn from(byte: u8) -> Op {
        Op::at(byte)
    }
}

impl Disassemble for Op {
    fn disassemble(&self, offset: usize, chunk: &Chunk, f: &mut String) -> usize {
        match self {
            Op::CONSTANT => {
                writeln!(f, "{0:04x}  {1}  CONSTANT", offset, chunk.line_str(offset)).unwrap();
                // Next ,grab the following 8 bytes and convert them to a value... might need
                // some tracking to do this right. Separate Opcodes for encoding [u]int|floats?
                let mut offset = offset + 1; // Skip the opcode, already consumed
                let idx = chunk.code[offset] as usize;

                // Convert the next 8 bytes to a f64
                let value = &chunk.constants[idx];
                writeln!(f, "\t{0:04x}  {1}", idx, value).unwrap();
                offset += 1;
                offset
            },
            op => {
                writeln!(f, "{:04x}  {}  {:?}", offset, chunk.line_str(offset), op).unwrap();
                offset + 1
            }
        }
    }
}
