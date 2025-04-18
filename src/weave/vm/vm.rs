use crate::weave::{Chunk, Op};
use crate::weave::compiler::Compiler;
use crate::weave::vm::traits::Disassemble;
use crate::weave::vm::types::WeaveType;
use crate::weave::vm::types::errors::OpResult;

struct IP {
    ip: *const u8,
    head: *const u8
}

impl IP {
    pub fn new(ptr: *const u8) -> IP {
        IP {
            ip: ptr,
            head: ptr
        }
    }
    
    pub fn offset(&self) -> usize {
        unsafe { self.ip.offset_from(self.head) as usize } 
    }
    
    pub fn next(&mut self) -> u8 {
        let b = unsafe { self.ip.read() };
        self.ip = unsafe { self.ip.offset(1) };

        b.clone()
    }
}

pub struct VM {
    chunk: Option<Chunk>,
    ip: Option<IP>,
    stack: Vec<WeaveType>,
    pub debug_mode: bool,
}

#[derive(Debug)]
pub enum VMError {
    InvalidChunk,
    InvalidOperandType,
    CompilationError(String),
    InternalError(String)
}

impl VMError {
    pub fn exit_code(&self) -> i32 {
        match self {
            VMError::InvalidChunk => 60,
            VMError::InvalidOperandType => 62,
            VMError::CompilationError(_) => 70,
            VMError::InternalError(_) => 80,
        }
    }
}

pub type VMResult = Result<WeaveType, VMError>;

impl VM {
    pub fn new(debug_mode: bool) -> VM {
        VM {
            chunk: None,
            ip: None,
            stack: Vec::with_capacity(255),
            debug_mode
        }
    }

    pub fn interpret(&mut self, source: &str) -> VMResult {
        let mut compiler = Compiler::new(source, true);
        let chunk = match compiler.compile() {
            Ok(c) => c,
            Err(msg) => return Err(VMError::CompilationError(msg))
        };
        
        self.ip = Some(IP::new(chunk.code.as_ptr()));
        self.chunk = Some(chunk);
        self.run()
    }

    fn _read_constant(&mut self) -> &WeaveType {
        let idx = self.ip.as_mut().unwrap().next().clone() as usize;
        self.chunk.as_ref().unwrap().get_constant(idx)
    }

    fn _push(&mut self, value: OpResult) -> VMResult {
        // If we encountered an error in performing an action, we may need to raise an error.
        // We can handle most of that here to make everyone else' lives easier - just return
        // whatever val/error you have and when we try to push it to the stack, determine if
        // something went wrong.
        match value {
            Ok(v) => { self.stack.push(v); Ok(self._peek()) }
            Err(msg) => { Err(VMError::InternalError(msg)) }
        }
    }

    fn _pop(&mut self) -> WeaveType {
        self.stack.pop().unwrap_or(WeaveType::None)
    }
    
    fn _peek(&self) -> WeaveType {
        self.stack.last().unwrap_or(&WeaveType::None).clone()
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
            let op = Op::at(self.ip().next());

            if self.debug_mode {
                let mut out = String::new();
                op.disassemble(self.ip().offset() - 1, self.chunk.as_ref().unwrap(), &mut out);
                // TODO: tabular format
                //    offset, line number, opcode, Optional var offset, optional var value, list of stack vars
                print!("{}", out);
                println!("       {:?}", self.stack);
            }

            match op {
                Op::RETURN => {
                    return Ok(self._pop())
                }
                Op::CONSTANT => {
                    let v = Ok(self._read_constant().clone());
                    self._push(v);
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
            }
        }
    }

    fn ip(&mut self) -> &mut IP {
        self.ip.as_mut().unwrap()
    }
}