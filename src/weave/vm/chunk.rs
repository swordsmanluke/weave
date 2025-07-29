use std::fmt::{Error};
use crate::weave::Op;
use crate::weave::vm::traits::disassemble::Disassemble;
use crate::weave::vm::types::NanBoxedValue;
use crate::log_debug;

#[derive(Clone, Debug)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<NanBoxedValue>, // Now using NanBoxedValue for 4x memory reduction
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
            .find(|(l_offset, _line)| *l_offset >= offset)
            .unwrap_or(&(0,0));

        let is_newline = offset == line_offset;
        if is_newline { format!("{:4 }", line) } else { "   |".to_string() }
    }

    pub(crate) fn line_number_at(&self, offset: usize) -> usize {
        let (_line_offset, line) = *self.lines.iter()
            .find(|(l_offset, _line)| *l_offset >= offset)
            .unwrap_or(&(0,0));
        line
    }

    pub fn write(&mut self, bytes: &Vec<u8>, line: usize) {
        bytes.iter().for_each(|b| self.write_byte(*b, line));
    }
    
    pub fn write_byte(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        if self.lines.last().unwrap_or(&(0,0)).1 != line {
            let offset = self.code.len() - 1;
            self.lines.push((offset, line))
        }
    }

    pub fn emit_constant(&mut self, value: NanBoxedValue, line: usize) -> usize {
        self.write_op(Op::CONSTANT, line);
        self.add_constant(value, line)
    }

    pub fn add_constant(&mut self, value: NanBoxedValue, line: usize) -> usize {
        let idx = self.add_constant_only(value);
        self.write(&(idx as u16).to_be_bytes().to_vec(), line); // Write BigEndian bytes to the chunk
        idx
    }

    /// Add a constant to the constants table without emitting bytecode
    pub fn add_constant_only(&mut self, value: NanBoxedValue) -> usize {
        // NanBoxedValue implements Copy and PartialEq, so this is efficient
        if let Some(pos) = self.constants.iter().position(|&v| v == value) {
            pos
        } else {
            self.constants.push(value);
            self.constants.len() - 1
        }
    }

    pub fn get_constant(&self, idx: usize) -> NanBoxedValue {
        self.constants[idx] // Copy, not reference - NanBoxedValue is Copy
    }

    pub fn disassemble(&self, name: &str) -> Result<(), Error> {
        log_debug!("Disassemble chunk", chunk_name = name);
        let mut offset = 0;
        while offset < self.code.len() {
            offset = Op::at(self.code[offset]).disassemble(offset, self);
        }
        Ok(())
    }
}