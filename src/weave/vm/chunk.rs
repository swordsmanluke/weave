use std::fmt::{Display, Write};
use crate::weave::Op;
use crate::weave::vm::dissassemble::Disassemble;
use crate::weave::vm::types::WeaveType;
use crate::weave::vm::values::ValueArray;

pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: ValueArray, // May be replaceable with a vec
    pub lines: Vec<usize>
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk { code: vec![], constants: ValueArray::new(), lines: vec![] }
    }

    pub fn write(&mut self, op: Op, line: usize) {
        self._write(&op.bytecode(), line);
    }

    /// TODO: Helper for the dissassembler - this should probably move elsewhere
    pub fn line_str(&self, offset: usize) -> String {
        let is_newline = offset == 0 || self.lines[offset] != self.lines[offset - 1];
        if is_newline { format!("{:4 }", self.lines[offset]) } else { "   | ".to_string() }
    }

    fn _write(&mut self, bytes: &Vec<u8>, line: usize) {
        bytes.iter().for_each(|b| self.code.push(*b));
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: WeaveType, line: usize) -> usize {
        self.constants.push(value);
        let idx = (self.constants.values.len() - 1) as u8;
        self._write(&vec![idx], line);
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