use std::collections::HashMap;
use std::fmt::{Write};
use crate::weave::Op;
use crate::weave::vm::traits::disassemble::Disassemble;
use crate::weave::vm::types::WeaveType;

#[derive(Clone)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<WeaveType>, // May be replaceable with a vec
    pub lines: Vec<(usize, usize)>
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk { code: vec![], constants: vec![], lines: Vec::new() }
    }
    
    pub fn write_op(&mut self, op: Op, line: usize) {
        self.write(&op.bytecode(), line);
    }
    
    /// TODO: Helper for the dissassembler - this should probably move elsewhere
    pub fn line_str(&self, offset: usize) -> String {
        let (line_offset, line) = *self.lines.iter()
            .find(|(l_offset, line)| *l_offset >= offset)
            .unwrap_or(&(0,0));
        
        let is_newline = offset == line_offset;
        if is_newline { format!("{:4 }", line) } else { "   | ".to_string() }
    }
    
    pub(crate) fn line_number_at(&self, offset: usize) -> usize {
        let (_line_offset, line) = *self.lines.iter()
            .find(|(l_offset, line)| *l_offset >= offset)
            .unwrap_or(&(0,0));
        line
    }

    pub fn write(&mut self, bytes: &Vec<u8>, line: usize) {
        bytes.iter().for_each(|b| self.code.push(*b));
        if self.lines.last().unwrap_or(&(0,0)).1 != line {
            let offset = self.code.len() - 1;
            self.lines.push((offset, line)) 
        }
    }

    pub fn add_constant(&mut self, value: WeaveType, line: usize) -> usize {
        self.write_op(Op::CONSTANT, line);
        self.constants.push(value);
        let idx = (self.constants.len() - 1) as u16;
        self.write(&idx.to_be_bytes().to_vec(), line); // Write BigEndian bytes to the chunk
        idx as usize
    }

    pub fn get_constant(&self, idx: usize) -> &WeaveType {
        &self.constants[idx]
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