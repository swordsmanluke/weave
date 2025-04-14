use crate::weave::{Chunk, Op};

mod weave;

fn main() {
    let mut c = Chunk::new();
    c.write(Op::CONSTANT, 123);
    c.add_constant(1.2.into(), 123);
    c.write(Op::RETURN, 123);

    println!("{}", c.disassemble("test"));
}
