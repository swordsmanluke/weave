use std::fmt::{Display, Write};
use crate::weave::Op;
use crate::weave::vm::dissassemble::Disassemble;
use crate::weave::vm::types::WeaveType;
use crate::weave::vm::values::ValueArray;

pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: ValueArray, // May be replaceable with a vec
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk { code: vec![], constants: ValueArray::new() }
    }

    pub fn write(&mut self, op: Op) {
        op.bytecode().iter().for_each(|b| self.code.push(*b));
    }

    pub fn add_constant(&mut self, value: WeaveType) -> usize {
        self.constants.push(value);
        let idx = (self.constants.values.len() - 1) as u8;
        self.code.push(idx);
        idx as usize
    }

    pub fn disassemble(&self, name: &str) -> String {
        let mut f = String::new();
        write!(f, "=== {0} ===\n", name).unwrap();
        let mut offset = 0;
        while offset < self.code.len() {
            offset = Op::at(self.code[offset]).disassemble(offset, self, &mut f);
        }
        f
    }
}