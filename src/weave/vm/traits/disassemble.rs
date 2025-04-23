use std::fmt::Write;
use crate::weave::Chunk;

pub trait Disassemble {
    fn disassemble(&self, offset: usize, chunk: &Chunk) -> usize;
}