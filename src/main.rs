use crate::weave::{Chunk, Op};

mod weave;

fn main() {
    let mut c = Chunk::new();
    c.write(Op::RETURN);

    println!("{}", c.disassemble("test"));
}
