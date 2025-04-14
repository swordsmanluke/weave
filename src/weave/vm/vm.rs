use crate::weave::{Chunk, Op};
use crate::weave::color::green;
use crate::weave::vm::traits::Disassemble;
use crate::weave::vm::types::WeaveType;
use crate::weave::vm::types::errors::OpResult;

pub struct VM<'a> {
    chunk: Option<&'a Chunk>,
    ip: Option<*const u8>,
    stack: Vec<WeaveType>,
    pub debug_mode: bool,
}

#[derive(Debug)]
pub enum VMError {
    InvalidChunk,
    InvalidInstruction,
    InvalidOperand,
    InvalidOperandType,
    InternalError(String)
}

pub type VMResult = Result<(), VMError>;

impl<'a> VM<'a> {
    pub fn new(debug_mode: bool) -> VM<'a> {
        VM {
            chunk: None,
            ip: None,
            stack: Vec::with_capacity(255),
            debug_mode
        }
    }

    pub fn interpret(&mut self, chunk: &'a Chunk) -> VMResult {
        self.chunk = Some(chunk);
        self.ip = Some(chunk.code.as_ptr());
        self.run()
    }

    fn _next(&mut self) -> u8 {
        let b = unsafe { self.ip.unwrap().read() };
        self.ip = unsafe { Some(self.ip.unwrap().offset(1)) };

        b
    }

    fn _offset(&mut self) -> usize {
        let head = self.chunk.unwrap().code.as_ptr();
        (unsafe { self.ip.unwrap().byte_offset_from(head).abs() }) as usize
    }

    fn _read_constant(&mut self) -> &WeaveType {
        let idx = self._next() as usize;
        let chunk = self.chunk.unwrap();
        &chunk.constants.values[idx]
    }

    fn _push(&mut self, value: OpResult) -> VMResult {
        // If we encountered an error in performing an action, we may need to raise an error.
        // We can handle most of that here to make everyone else' lives easier - just return
        // whatever val/error you have and when we try to push it to the stack, determine if
        // something went wrong.
        match value {
            Ok(v) => { self.stack.push(v); Ok(()) }
            Err(msg) => { Err(VMError::InternalError(msg)) }
        }
    }

    fn _pop(&mut self) -> WeaveType {
        self.stack.pop().unwrap_or(WeaveType::None)
    }

    fn _poppop(&mut self) -> [WeaveType; 2] {
        let a = self._pop();
        let b = self._pop();
        [a, b]
    }

    pub fn run(&mut self) -> VMResult {
        if self.ip.is_none() || self.chunk.is_none() {
            return Err(VMError::InvalidChunk);
        }

        loop { // until ip offset > chunk size
            let op = Op::at(self._next());

            if self.debug_mode {
                let mut out = String::new();
                op.disassemble(self._offset() - 1, &self.chunk.unwrap(), &mut out);
                // TODO: tabular format
                //    offset, line number, opcode, Optional var offset, optional var value, list of stack vars
                print!("{}", out);
                println!("       {:?}", self.stack);
            }

            match op {
                Op::RETURN => {
                    print!("{}", green(&format!("{}", self._pop())));
                    return Ok(())
                }
                Op::CONSTANT => {
                    let v = Ok(self._read_constant().clone());
                    self._push(v);
                }
                Op::NEGATE => { let v = -self._pop(); self._push(v)? }
                Op::ADD => {
                    let [a,b] = self._poppop();
                    self._push(a + b)?
                }
                Op::SUB => {
                    let [a,b] = self._poppop();
                    self._push(a - b)?
                }
                Op::MUL => {
                    let [a,b] = self._poppop();
                    self._push(a * b)?
                }
                Op::DIV => {
                    let [a,b] = self._poppop();
                    self._push(a / b)?
                }
            }
        }

    }
}