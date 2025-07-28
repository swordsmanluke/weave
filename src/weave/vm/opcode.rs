use crate::weave::Chunk;
use crate::weave::Op::INVALID;
use crate::weave::vm::traits::disassemble::Disassemble;
use crate::log_debug;
use crate::weave::vm::types::{FnClosure, Upvalue, WeaveType};

#[derive(Debug, PartialEq)]
pub enum Op {
    // Literals
    TRUE,
    FALSE,
    CONSTANT,  // TODO: Always 64 bit double right now. Fix that.
    SetGlobal,
    GetGlobal,
    SetLocal,
    GetLocal,
    SetUpvalue,
    GetUpvalue,
    
    // Boolean
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
    Loop,
    Jump,
    JumpIfFalse,
    Closure,
    Call,
    RETURN,
    POP,

    // IO
    PRINT,
    
    // Error handling
    INVALID(u8),
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
            Op::PRINT => vec![13],
            Op::POP => vec![14],
            Op::SetGlobal => vec![15],
            Op::GetGlobal => vec![16],
            Op::SetLocal => vec![17],
            Op::GetLocal => vec![18],
            Op::Closure => vec![19],
            Op::JumpIfFalse => vec![20],
            Op::Jump => vec![21],
            Op::Loop => vec![22],
            Op::Call => vec![23],
            
            Op::SetUpvalue => vec![24],
            Op::GetUpvalue => vec![25],
            
            Op::INVALID(byte) => vec![255],
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
            13 => Op::PRINT,
            14 => Op::POP,
            15 => Op::SetGlobal,
            16 => Op::GetGlobal,
            17 => Op::SetLocal,
            18 => Op::GetLocal,
            19 => Op::Closure,
            20 => Op::JumpIfFalse,
            21 => Op::Jump,
            22 => Op::Loop,
            23 => Op::Call,
            
            24 => Op::SetUpvalue,
            25 => Op::GetUpvalue,

            _ => INVALID(byte), // Should never happen, but when it does - die.
        }
    }
}

impl From<u8> for Op {
    fn from(byte: u8) -> Op {
        Op::at(byte)
    }
}

impl Disassemble for Op {
    fn disassemble(&self, offset: usize, chunk: &Chunk) -> usize {
        match self {
            Op::CONSTANT => {
                log_debug!("Disassemble CONSTANT/Closure start", offset = format!("{:04x}", offset).as_str(), line = chunk.line_str(offset).as_str());
                let mut offset = offset + 1; // Skip the opcode, already consumed

                // Read two bytes for the index
                let idx = u16::from_be_bytes(chunk.code[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;

                // Now retrieve the value from the constants table and print it
                let value = &chunk.constants[idx];
                log_debug!("Disassemble CONSTANT", idx = idx, value = format!("{}", value).as_str());
                offset
            },
            Op::Closure => {
                #[cfg(debug_assertions)]
                print!("{0:04x}  {1}  {2:?}", offset, chunk.line_str(offset), self);
                let mut offset = offset + 1; // Skip the opcode, already consumed

                // Read two bytes for the function index
                let idx = u16::from_be_bytes(chunk.code[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;

                // retrieve the value from the constants table and print it
                let value = &chunk.constants[idx];
                #[cfg(debug_assertions)]
                println!("\t{0:04x}  {1}", idx, value);
                
                // Now read N upvalues, and show them too.
                match value {
                    WeaveType::Closure(c) => {
                        for i in 0..c.func.upvalue_count {
                            let upvalue = Upvalue::from_bytes(&chunk.code, offset);
                            #[cfg(debug_assertions)]
                            println!("\t\t{0}  {1:04x}", upvalue, i);
                            offset += 2;
                        }
                    }
                    _ => { 
                        #[cfg(debug_assertions)]
                        println!("\t\tNone"); 
                    }
                }
                
                offset
            },
            Op::Call => {
                let mut offset = offset;
                log_debug!("Disassemble Call start", offset = format!("{:04x}", offset).as_str(), line = chunk.line_str(offset).as_str());
                offset += 1; // We've read our opcode, next, get the jump offset
                let slot = (offset as isize - chunk.code[offset] as isize) as usize;
                offset += 1;
                log_debug!("Disassemble Call", slot = slot);
                offset
            }
            Op::Loop => {
                let mut offset = offset;
                log_debug!("Disassemble Loop start", offset = format!("{:04x}", offset).as_str(), line = chunk.line_str(offset).as_str());
                offset += 1; // We've read our opcode, next, get the jump offset
                let jump = u16::from_be_bytes(chunk.code[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                let new_pos = (offset as isize - jump as isize) as usize;
                log_debug!("Disassemble Loop", new_position = format!("{:04x}", new_pos).as_str());

                offset
            }, 
            Op::Jump | Op::JumpIfFalse => {
                let mut offset = offset;
                log_debug!("Disassemble Jump start", offset = format!("{:04x}", offset).as_str(), line = chunk.line_str(offset).as_str(), opcode = format!("{:?}", self).as_str());
                offset += 1; // We've read our opcode, next, get the jump offset
                let jump = u16::from_be_bytes(chunk.code[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                log_debug!("Disassemble Jump", target = format!("{:04x}", offset + jump).as_str());
                
                offset
            }
            Op::GetLocal | Op::SetLocal => {
                log_debug!("Disassemble Local start", offset = format!("{:04x}", offset).as_str(), line = chunk.line_str(offset).as_str(), opcode = format!("{:?}", self).as_str());
                // Lookup the slot and print its contents
                let slot = chunk.code[offset + 1];
                let value = &chunk.constants.get(slot as usize);
                log_debug!("Disassemble Local", slot = slot, value = format!("{:?}", value).as_str());
                offset + 2
            }
            Op::GetUpvalue | Op::SetUpvalue => {
                #[cfg(debug_assertions)]
                println!("{0:04x}  {1}  {2:?}", offset, chunk.line_str(offset), self);
                offset + 2
            }
            op => {
                log_debug!("Disassemble Op", offset = format!("{:04x}", offset).as_str(), line = chunk.line_str(offset).as_str(), opcode = format!("{:?}", op).as_str());
                offset + 1
            }
        }
    }
}
