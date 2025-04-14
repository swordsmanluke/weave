use crate::weave::{Chunk, Op};
use crate::weave::vm::vm::VM;

mod weave;

fn main() {
    let mut vm = VM::new(true);

    let mut c = Chunk::new();
    c.add_constant(1.2.into(), 123);
    c.add_constant(2.into(), 123);

    c.write(Op::ADD, 123);

    c.add_constant(5.into(), 123);
    c.write(Op::MUL, 123);

    c.add_constant(2.into(), 123);
    c.write(Op::DIV, 123);

    c.write(Op::RETURN, 123);

    match vm.interpret(&c) {
        Ok(_) => {},
        Err(e) => {
            println!("Error: {:?}", e);
            println!("{}", c.disassemble("test"));
        },
    }

}
