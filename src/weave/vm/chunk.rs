use std::fmt::{Display, Formatter};
use crate::weave::Op;
use crate::weave::vm::dissassemble::Disassemble;

pub struct Chunk {
    pub code: Vec<u8>
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk { code: vec![] }
    }

    pub fn write(&mut self, op: Op) {
        op.bytecode().iter().for_each(|b| self.code.push(*b));
    }


    pub fn disassemble(&self, name: &str) -> String {
        let mut output = String::from(format!("=== {} ===\n", name));
        let mut offset = 0;
        while offset < self.code.len() {
            offset += Op::from_bytes(&self.code, offset).disassemble(offset, &mut output);
        }
        output
    }

}
