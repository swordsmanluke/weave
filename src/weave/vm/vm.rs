use crate::weave::compiler::Compiler;
use crate::weave::vm::instruction_pointer::IP;
use crate::weave::vm::types::{NativeFn, NativeFnType, WeaveFn, WeaveType};
use crate::weave::vm::types::errors::OpResult;
use crate::weave::{Op};
use std::collections::HashMap;
use std::io::{Write, stdout};
use std::rc::Rc;
use crate::weave::color::green;
use crate::{log_debug, log_info, log_warn, log_error};

pub struct VM {
    call_stack: CallStack,
    stack: Vec<WeaveType>,
    globals: HashMap<String, WeaveType>,
    last_value: WeaveType,
}

#[derive(Debug, Clone)]
pub enum VMError {
    InvalidChunk,
    CompilationError(String),
    RuntimeError { line: usize, msg: String },
}

struct CallStack  {
    frames: Vec<CallFrame>
}

struct CallFrame {
    pub func: Rc<WeaveFn>,
    ip: IP,
    slot: usize
}

impl CallFrame {
    pub fn new(func: Rc<WeaveFn>, slot: usize) -> CallFrame {
        let ip = IP::new(&func.chunk.code, true);
        CallFrame { func, ip, slot}
    }

    pub fn i(&self, idx: usize) -> usize {
        self.slot + idx
    }
}

impl CallStack {
    pub fn new() -> CallStack {
        CallStack { frames: Vec::new() }
    }
    
    pub fn push(&mut self, func: Rc<WeaveFn>, slot: usize) {
        let frame = CallFrame::new(func, slot);
        self.frames.push(frame);
    }
    
    pub fn pop(&mut self) {
        self.frames.pop();
    }
    
    pub fn disassemble(&self, name: &str) {
        self.frames.last().unwrap().func.chunk.disassemble(name).unwrap();
    }
    
    pub fn constants(&self) -> &Vec<WeaveType> {
        &self.frames.last().unwrap().func.chunk.constants
    }

    pub fn next_op(&mut self) -> Op {
        Op::at(self.cur_frame().ip.next())
    }

    pub fn next_u16(&mut self) -> u16 {
        self.cur_frame().ip.next_u16()
    }

    pub fn next_byte(&mut self) -> u8 {
        self.cur_frame().ip.next()
    }
    
    pub fn next_slot(&mut self) -> usize {
        let relative_slot = self.next_byte() as usize;
        self.cur_frame().i(relative_slot)
    }
    
    pub fn jump(&mut self, offset: u16) {
        self.cur_frame().ip.jump(offset);
    }
    
    pub fn jump_back(&mut self, offset: u16) {
        self.cur_frame().ip.jump_back(offset);
    }

    pub fn line_number_at(&mut self, offset: isize) -> usize {
        let point = self.cur_frame().ip.idx(offset);
        self.cur_frame().func.chunk.line_number_at(point)
    }

    pub fn get_constant(&mut self, idx: usize) -> &WeaveType {
        self.cur_frame().func.chunk.get_constant(idx)
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn is_at_end(&self) -> bool {
        self.frames.iter().all(|f| f.ip.is_at_end())
    }
    
    pub fn reset(&mut self) {
        self.frames.clear();
    }

    fn cur_frame(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }
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
    pub fn new(_debug_mode: bool) -> VM {
        let mut vm = VM {
            call_stack: CallStack::new(),
            stack: Vec::with_capacity(255),
            globals: HashMap::new(),
            last_value: WeaveType::None,
        };

        NativeFnType::variants().iter().for_each(|fn_type| {
            vm.define_native(Rc::new(NativeFn::get(fn_type.clone())));
        });

        vm
    }

    pub fn interpret(&mut self, source: &str) -> VMResult {
        let mut compiler = Compiler::new(source, false);
        self.debug(&format!("Compiling...\n{}", source));
        let func = match compiler.compile() {
            Ok(c) => c,
            Err(msg) => return Err(VMError::CompilationError(msg)),
        };
        
        let top_frame = Rc::new(func);

        self.stack.push(WeaveType::Fn(top_frame.clone()));
        self.call_stack.push(top_frame, 0);

        self.debug("Interpreting...");
        
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
        self.call_stack.get_constant(idx)
    }

    fn _push(&mut self, value: OpResult) -> VMResult {
        // TODO: This is the where "malloc" would be in C
        self.debug(&format!("PUSH: {:?}", value));
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
                let line = self.call_stack.line_number_at(-1);
                Err(VMError::RuntimeError { line, msg })
            }
        }
    }

    fn _pop(&mut self) -> WeaveType {
        // TODO: This is _nearly_ where the "free" would be in C - basically as soon as the
        //       value returned here is dropped, it should be freed
        self.debug(&format!("POP: {:?}", self._peek()));
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
        if self.call_stack.is_empty() { return Err(VMError::InvalidChunk); }

        self.debug("Executing...");
        log_debug!("Starting VM execution", function = "main");

        while !self.call_stack.is_at_end() {
            // until ip offset > chunk size
            let op = self.call_stack.next_op();

            self.debug(&format!("EVAL({:?})", op));
            match op {
                Op::INVALID(_) => {
                    return Err(VMError::InvalidChunk);
                }
                Op::RETURN => {
                    let result = self._pop();
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        self._pop();
                        return Ok(result);
                    }
                    
                    self.debug(&format!("Returning: {} from depth {}", result, self.stack.len()));
                    self._push(Ok(result))?;
                },
                Op::POP => { self._pop(); },
                Op::CONSTANT => {
                    let idx = self.call_stack.next_u16() as usize;
                    self.debug(&format!("Reading constant @ {:0x}", idx));
                    let v = Ok(self._read_constant(idx).clone());
                    self._push(v)?;
                }
                Op::Call => {
                    let arg_count= self.call_stack.next_byte() as usize;
                    let func_slot = (self.stack.len() - 1) - arg_count;
                    let func = self.stack.get(func_slot).unwrap();
                    self.debug(&format!("Taking {} @ {}", func, func_slot));
                    match func {
                        WeaveType::Fn(f) => {
                            self.debug(&format!("Calling {} with {} arguments", f, arg_count));
                            self.call_stack.push(f.clone(), func_slot);
                            self.call(f.clone(), arg_count)?;
                        }
                        WeaveType::NativeFn(f) => {
                            let args = if arg_count > 0 {
                                let last_arg = self.stack.len() - 1;
                                let first_arg = last_arg - arg_count;
                                self.stack[first_arg..last_arg].to_vec()
                            } else {
                                vec![]
                            };
                            self.debug(&format!("Calling {} with {} arguments", f, arg_count));

                            (f.func)(args)?;
                        }
                        _ => {
                            return Err(VMError::RuntimeError { line: self.call_stack.line_number_at(-1), msg: "Only functions can be called".to_string() })
                        }
                    }
                }
                Op::SetLocal => {
                    let slot = self.call_stack.next_slot();
                    
                    self.stack[slot] = self._peek();
                }
                Op::GetLocal => {
                    let slot = self.call_stack.next_slot();
                    self._push(Ok(self.stack[slot].clone()))?;
                }
                Op::SetGlobal => {
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
                        _ => { unreachable!("Only strings can become globals - how did you get here?"); }
                    }
                }
                Op::GetGlobal => {
                    let name = self._pop();
                    match name {
                        WeaveType::String(name) => match self.globals.get(name.as_str()) {
                            Some(v) => {
                                self._push(Ok(v.clone()))?;
                            }
                            None => {
                                let line = self.call_stack.line_number_at(-1);
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
                    // Don't remove the top value from the stack - printing a value evaluates
                    // to the value itself. e.g. "print(1) == 1"
                    let value = self._peek();
                    println!("{}", green(&format!("{}", value)));
                    log_debug!("VM print instruction", value = format!("{}", value).as_str(), stack_depth = self.stack.len());
                }
                Op::Jump => {
                    let jmp_target = self.call_stack.next_u16();
                    self.call_stack.jump(jmp_target);
                }
                Op::JumpIfFalse => {
                    let jmp_offset = self.call_stack.next_u16();
                    if !self._peek().truthy() {
                        self.call_stack.jump(jmp_offset);
                    }
                }
                Op::Loop => {
                    let jmp_offset = self.call_stack.next_u16();
                    self.call_stack.jump_back(jmp_offset);
                }
            }

            self.debug(&format!("  - {:?}", self.stack));
            self.debug(&format!("  - {:?}", self.call_stack.constants()));
        }

        Ok(self.last_value.clone())
    }

    fn debug(&self, msg: &str) {
        log_debug!("VM debug", message = msg, stack_depth = self.stack.len());
    }

    fn runtime_error(&mut self, line: usize, msg: &String) {
        let callstack = self.call_stack.frames.iter().rev();
        for frame in callstack {
            let func = &frame.func;
            let line = func.chunk.line_number_at(frame.ip.idx(-1));
            
            log_error!("Runtime error in function", 
                line = line, 
                function = func.name.as_str(), 
                message = msg.as_str(),
                code = func.chunk.line_str(frame.ip.idx(0)).as_str()
            );
        }

        self.reset_stack();
    }

    fn define_native(&mut self, func: Rc<NativeFn>) {
        let name = func.name.to_string();
        self.globals.insert(name, WeaveType::NativeFn(func));
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
        self.call_stack.reset();
    }
    
    fn call(&mut self, func: Rc<WeaveFn>, arg_count: usize) -> VMResult {
        if func.arity != arg_count {
            Err(VMError::RuntimeError { line: 0, msg: format!("{} Expected {} arguments but got {}", func.name, func.arity, arg_count) })
        } else if self.call_stack.frames.len() > 100 {
            Err(VMError::RuntimeError { line: 0, msg: "Stack overflow".to_string() })
        } else {
            Ok(self._peek())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_math() {
        let mut vm = VM::new(true);
        let res = vm.interpret("5 + 2 * 3");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(11.0));
    }

    #[test]
    fn test_parenthesis() {
        let mut vm = VM::new(true);
        let res = vm.interpret("(5 + 2) * 3");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(21.0));
    }

    #[test]
    fn test_negate() {
        let mut vm = VM::new(true);
        let res = vm.interpret("-5");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(-5.0));
    }

    #[test]
    fn test_string_literal() {
        let mut vm = VM::new(true);
        let res = vm.interpret("\"hello\"");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from("hello"));
    }
    
    #[test]
    fn test_var_addition() {
        let mut vm = VM::new(true);
        let res = vm.interpret("x = 5\nx + 2");
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
    
    #[test]
    fn test_invalid_assignment_doesnt_parse() {
        let mut vm = VM::new(true);
        let res = vm.interpret("a= 1; a + b = 5");
        assert!(res.is_err());
    }

    #[test]
    fn test_shadowing_self() {
        let mut vm = VM::new(true);
        let res = vm.interpret("
        a = 1;  # Global var
        fn foo() {
            a = a   # Shadows global with a local
            a = a + 2  # Increments local
        }
        foo()
        
        a ");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(1.0));
    }

    #[test]
    fn test_bad_initializer() {
        let mut vm = VM::new(true);
        let res = vm.interpret("a = a");
        assert!(res.is_err());
    }

    #[test]
    fn test_local_variables() {
        let mut vm = VM::new(true);
        let res = vm.interpret("{ x = 1; x + 3 }");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(4.0));
    }

    #[test]
    fn test_nested_scopes() {
        let mut vm = VM::new(true);
        let res = vm.interpret("{ x = 2; { x = 1; x = x + 3 } puts x; }");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(4.0));
    }

    #[test]
    fn test_if_true_condition() {
        let mut vm = VM::new(true);
        let res = vm.interpret("{
        a = 1;
        if (true) { a = a + 1 }
        a}");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(2.0));
    }

    #[test]
    fn test_if_false_condition() {
        let code = "{
        a = 1;
        if false { a = a + 1 }
        a
        }";
        let mut vm = VM::new(true);
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(1.0));
    }

    #[test]
    fn test_if_else_condition() {
        let code = "{
        a = 1;
        if false {
            a = a + 1
        } else {
            a = a + 2
        }
        a
        }";
        let mut vm = VM::new(true);
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(3.0));
    }


    #[test]
    fn test_if_syntax() {
        let code = "
            if false {
                puts 1
            } else {
                puts 2
            }
        ";
        let mut vm = VM::new(true);
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
    }
    
    #[test]
    fn test_while_syntax() {
        let code = "{
            a = 1  
            while a < 3 {
                a = a + 1
            }
            a
        }";
        let mut vm = VM::new(true);
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(3.0));
    }
    
    #[test]
    fn test_fn_definition() {
        let code = "
            fn add(a, b) { 
                a + b 
            }
            add(-1, 4)
        ";
        let mut vm = VM::new(true);
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), WeaveType::from(3.0));
    }
}
