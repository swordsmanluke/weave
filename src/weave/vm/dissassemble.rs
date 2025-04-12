
pub trait Disassemble {
    fn disassemble(&self, offset: usize, f: &mut String) -> usize;
}