use crate::weave::compiler::Compiler;
use crate::weave::vm::instruction_pointer::IP;
use crate::weave::vm::traits::Disassemble;
use crate::weave::vm::types::WeaveType;
use crate::weave::vm::types::errors::OpResult;
use crate::weave::{Chunk, Op};
use std::collections::HashMap;
use std::io::{Write, stdout};

pub struct VM {
    chunk: Option<Chunk>,
    stack: Vec<WeaveType>,
    globals: HashMap<String, WeaveType>,
    instruction_counter: usize,
    pub debug_mode: bool,
}

#[derive(Debug)]
pub enum VMError {
    InvalidChunk,
    CompilationError(String),
    RuntimeError { line: usize, msg: String },
}

impl VMError {
    pub fn exit_code(&self) -> i32 {
        match self {
            VMError::InvalidChunk => 60,
            VMError::CompilationError(_) => 70,
            // Probably unnecessary to exit from RuntimeErrors, but here's the code if you want
            VMError::RuntimeError { .. } => 80,
        }
    }
}

pub type VMResult = Result<WeaveType, VMError>;

impl VM {
    pub fn new(debug_mode: bool) -> VM {
        VM {
            chunk: None,
            stack: Vec::with_capacity(255),
            globals: HashMap::new(),
            instruction_counter: 0,
            debug_mode,
        }
    }

    pub fn interpret(&mut self, source: &str) -> VMResult {
        let mut compiler = Compiler::new(source, true);
        self.debug("Compiling...");
        let chunk = match compiler.compile() {
            Ok(c) => c,
            Err(msg) => return Err(VMError::CompilationError(msg)),
        };

        self.debug("Interpreting...");
        if self.debug_mode { chunk.disassemble("chunk dump"); 
        }
        self.chunk = Some(chunk);
        match self.run() {
            Ok(v) => Ok(v),
            Err(e) => {
                match &e {
                    VMError::RuntimeError { line, msg } => {
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
        // TODO: This is the where "malloc" would be in C
        self.debug(&format!("Pushing: {:?} to Stack", value));
        // If we encountered an error in performing an action, we may need to raise an error.
        // We can handle most of that here to make everyone else' lives easier - just return
        // whatever val/error you have and when we try to push it to the stack, determine if
        // something went wrong.
        match value {
            Ok(v) => {
                self.stack.push(v);
                Ok(self._peek())
            }
            Err(msg) => {
                let offset = self.instruction_counter - 1;
                let line = self.chunk.as_ref().unwrap().line_number_at(offset);
                Err(VMError::RuntimeError { line, msg })
            }
        }
    }

    fn _pop(&mut self) -> WeaveType {
        // TODO: This is _nearly_ where the "free" would be in C - basically as soon as the
        //       value returned here is dropped, it should be freed
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

        let mut ip = IP::new(&self.chunk.as_ref().unwrap().code, true);

        loop {
            // until ip offset > chunk size
            self.debug("Executing...");
            let op = Op::at(ip.next());
            self.instruction_counter = ip.idx(0);

            if self.debug_mode {
                // op.disassemble(ip.idx(-1), self.chunk.as_ref().unwrap());
                // TODO: tabular format
                //    offset, line number, opcode, Optional var offset, optional var value, list of stack vars
                println!("  - {:?}", self.stack);
            }

            self.debug(&format!("EVAL({:?})", op));
            match op {
                Op::RETURN | Op::POP => return Ok(self._pop()),
                Op::CONSTANT => {
                    let v = Ok(self._read_constant(ip.next_u16() as usize).clone());
                    self._push(v)?;
                }
                Op::DECL_GLOBAL => {
                    // Previous to this we should have processed an expression (val)
                    // then pushed the name of the global we want to bind it to
                    // and now we need to actually bind it.
                    // So pop the name and value off the stack.
                    let [val, name] = self._poppop();
                    match name {
                        WeaveType::String(name) => {
                            self.debug(&format!("Declaring global: {} = {}", name, val));
                            self.globals.insert(name.to_string(), val);
                        }
                        _ => {
                            let line = self
                                .chunk
                                .as_ref()
                                .unwrap()
                                .line_number_at(self.instruction_counter - 1);
                            return Err(VMError::RuntimeError {
                                line,
                                msg: "Invalid global name".to_string(),
                            });
                        }
                    }
                }
                Op::GET_GLOBAL => {
                    let name = self._pop();
                    match name {
                        WeaveType::String(name) => match self.globals.get(name.as_str()) {
                            Some(v) => {
                                self._push(Ok(v.clone()))?;
                            }
                            None => {
                                let line = self.chunk.as_ref().unwrap().line_number_at(self.instruction_counter - 1);
                                return Err(VMError::RuntimeError { line, msg: format!("Undefined global {}", name) });
                            }
                        },
                        _ => unreachable!("Expected an Identifier: {:?}", name),
                    }
                }
                Op::NEGATE => {
                    let v = -self._pop();
                    self._push(v)?;
                }
                Op::ADD => {
                    let [a, b] = self._poppop();
                    self._push(a + b)?;
                }
                Op::SUB => {
                    let [a, b] = self._poppop();
                    self._push(a - b)?;
                }
                Op::MUL => {
                    let [a, b] = self._poppop();
                    self._push(a * b)?;
                }
                Op::DIV => {
                    let [a, b] = self._poppop();
                    self._push(a / b)?;
                }
                Op::TRUE => {
                    self._push(Ok(WeaveType::Boolean(true)))?;
                }
                Op::FALSE => {
                    self._push(Ok(WeaveType::Boolean(false)))?;
                }
                Op::NOT => {
                    // Everything is truthy in Weave, so we just need to negate
                    // the top value's "truthiness"
                    let val = WeaveType::Boolean(!self._pop().truthy());
                    self._push(Ok(val))?;
                }
                Op::GREATER => {
                    let [a, b] = self._poppop();
                    self._push(Ok(WeaveType::Boolean(a > b)))?;
                }
                Op::LESS => {
                    let [a, b] = self._poppop();
                    self._push(Ok(WeaveType::Boolean(a < b)))?;
                }
                Op::EQUAL => {
                    let [a, b] = self._poppop();
                    self._push(Ok(WeaveType::Boolean(a == b)))?;
                }
                Op::PRINT => {
                    println!("{}", self._pop());
                }
            }
        }
    }

    fn debug(&self, msg: &str) {
        if self.debug_mode {
            println!("{}", msg);
            stdout().flush();
        }
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

    #[test]
    fn test_negate() {
        let mut vm = VM::new(true);
        let res = vm.interpret("-5");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(-5.0));
    }

    #[test]
    fn test_string_literal() {
        let mut vm = VM::new(true);
        let res = vm.interpret("\"hello\"");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from("hello"));
    }
    
    #[test]
    fn test_var_addition() {
        let mut vm = VM::new(true);
        let res = vm.interpret("x = 5\nx + 2");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(7.0));
    }

    #[test]
    fn test_puts_statement() {
        let mut vm = VM::new(true);
        let res = vm.interpret("puts \"hello\";");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
    }

    #[test]
    fn test_using_var() {
        let mut vm = VM::new(true);
        let res = vm.interpret("x = 5; puts x;");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
    }

    #[test]
    fn test_declaring_var() {
        let mut vm = VM::new(true);
        let res = vm.interpret("x = 5");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert!(
            vm.globals.contains_key("x"),
            "Global \"x\" not found in {:?}",
            vm.globals.keys().collect::<Vec<&String>>()
        );
        assert_eq!(vm.globals["x"], WeaveType::from(5.0));
    }
}
