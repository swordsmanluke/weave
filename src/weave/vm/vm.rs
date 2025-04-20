use std::slice::Iter;
use crate::weave::{Chunk, Op};
use crate::weave::compiler::Compiler;
use crate::weave::vm::instruction_pointer::IP;
use crate::weave::vm::traits::Disassemble;
use crate::weave::vm::types::WeaveType;
use crate::weave::vm::types::errors::OpResult;

pub struct VM {
    chunk: Option<Chunk>,
    stack: Vec<WeaveType>,
    instruction_counter: usize,
    pub debug_mode: bool,
}

#[derive(Debug)]
pub enum VMError {
    InvalidChunk,
    InvalidOperandType,
    CompilationError(String),
    RuntimeError{line: usize, msg: String},
    InternalError(String)
}

impl VMError {
    pub fn exit_code(&self) -> i32 {
        match self {
            VMError::InvalidChunk => 60,
            VMError::InvalidOperandType => 62,
            VMError::CompilationError(_) => 70,
            VMError::RuntimeError{..} => 75,
            VMError::InternalError(_) => 80,
        }
    }
}

pub type VMResult = Result<WeaveType, VMError>;

impl VM {
    pub fn new(debug_mode: bool) -> VM {
        VM {
            chunk: None,
            stack: Vec::with_capacity(255),
            instruction_counter: 0,
            debug_mode
        }
    }

    pub fn interpret(&mut self, source: &str) -> VMResult {
        let mut compiler = Compiler::new(source, true);
        self.debug("Compiling...");
        let chunk = match compiler.compile() {
            Ok(c) => c,
            Err(msg) => return Err(VMError::CompilationError(msg))
        };

        self.debug("Interpreting...");
        self.debug(&format!("Chunk: {}", chunk.disassemble("Chunk Dump")));
        self.chunk = Some(chunk);
        match self.run() {
            Ok(v) => Ok(v),
            Err(e) => {
                match &e {
                    VMError::RuntimeError{line, msg} => {
                        self.runtime_error(*line, &msg);
                    }
                    _ => {}
                }
                Err(e)
            }
        }
    }

    fn _read_constant(&mut self, idx: usize) -> &WeaveType {
        self.chunk.as_ref().unwrap().get_constant(idx)
    }

    fn _push(&mut self, value: OpResult) -> VMResult {
        self.debug(&format!("Pushing: {:?} to Stack", value));
        // If we encountered an error in performing an action, we may need to raise an error.
        // We can handle most of that here to make everyone else' lives easier - just return
        // whatever val/error you have and when we try to push it to the stack, determine if
        // something went wrong.
        match value {
            Ok(v) => { self.stack.push(v); Ok(self._peek()) }
            Err(msg) => { 
                let line = self.chunk.as_ref().unwrap().lines[self.instruction_counter - 1];
                Err(VMError::RuntimeError{ line, msg}) }
        }
    }

    fn _pop(&mut self) -> WeaveType {
        self.stack.pop().unwrap_or(WeaveType::None)
    }

    fn _peek(&self) -> WeaveType {
        self.stack.last().unwrap_or(&WeaveType::None).clone()
    }

    fn _poppop(&mut self) -> [WeaveType; 2] {
        let b = self._pop();
        let a = self._pop();
        [a, b]
    }

    pub fn run(&mut self) -> VMResult {
        if self.chunk.is_none() {
            return Err(VMError::InvalidChunk);
        }
        
        fn read(ip: &mut Iter<u8>) -> u8 {
            match ip.next() { Some(v) => *v, None => 0 }
        }

        let mut ip = IP::new(&self.chunk.as_ref().unwrap().code, true);

        loop { // until ip offset > chunk size
            self.debug("Executing...");
            let op = Op::at(ip.next());
            self.instruction_counter = ip.idx(0);

            if self.debug_mode {
                let mut out = String::new();
                op.disassemble(ip.idx(-1), self.chunk.as_ref().unwrap(), &mut out);
                // TODO: tabular format
                //    offset, line number, opcode, Optional var offset, optional var value, list of stack vars
                print!("{}", out);
                println!("       {:?}", self.stack);
            }

            self.debug(&format!("EVAL({:?})", op));
            match op {
                Op::RETURN => {
                    return Ok(self._pop())
                }
                Op::CONSTANT => {
                    let v = Ok(self._read_constant(ip.next() as usize).clone());
                    self._push(v)?;
                }
                Op::NEGATE => { let v = -self._pop(); self._push(v)?; }
                Op::ADD => {
                    let [a,b] = self._poppop();
                    self._push(a + b)?;
                }
                Op::SUB => {
                    let [a,b] = self._poppop();
                    self._push(a - b)?;
                }
                Op::MUL => {
                    let [a,b] = self._poppop();
                    self._push(a * b)?;
                }
                Op::DIV => {
                    let [a,b] = self._poppop();
                    self._push(a / b)?;
                }
                Op::TRUE => { self._push(Ok(WeaveType::Boolean(true)))?; }
                Op::FALSE => { self._push(Ok(WeaveType::Boolean(false)))?; }
            }
        }
    }

    fn debug(&self, msg: &str) {
        if self.debug_mode { println!("{}", msg); }
    }

    fn runtime_error(&mut self, line: usize, msg: &String) {
        println!("{}", msg);
        println!("[line {}] in script\n", line);
        self.reset_stack();
    }
    
    fn reset_stack(&mut self) {
        self.stack.clear();
        self.instruction_counter = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_math() {
        let mut vm = VM::new(true);
        let res = vm.interpret("5 + 2 * 3");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(11.0));
    }
    
    #[test]
    fn test_parenthesis() {
        let mut vm = VM::new(true);
        let res = vm.interpret("(5 + 2) * 3");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(21.0));
    }
}