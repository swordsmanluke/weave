use crate::weave::compiler::Compiler;
use crate::weave::vm::instruction_pointer::IP;
use crate::weave::vm::types::{FnClosure, NanBoxedValue, NativeFn, NativeFnType, Upvalue, WeaveNumber, WeaveString, WeaveType, WeaveUpvalue};
use crate::weave::vm::types::errors::OpResult;
use crate::weave::{Op};
use std::collections::HashMap;
use std::rc::Rc;
use crate::weave::color::green;
use crate::{log_debug, log_error};

pub struct VM {
    call_stack: CallStack,
    stack: Vec<NanBoxedValue>,
    globals: HashMap<String, NanBoxedValue>,
    last_value: NanBoxedValue,
    open_upvalues: Vec<WeaveUpvalue>,
}

#[derive(Debug, Clone)]
pub enum VMError {
    InvalidChunk,
    CompilationError(String),
    RuntimeError { line: usize, msg: String },
}

struct CallStack  {
    frames: Vec<CallFrame>,
    // Simple frame pool to avoid allocations in hot loops
    frame_pool: Vec<CallFrame>,
}

pub struct CallFrame {
    pub closure: FnClosure,
    pub slot: usize,
    ip: IP,
}

impl CallFrame {
    pub fn new(closure: FnClosure, slot: usize) -> CallFrame {
        let ip = IP::new(&closure.func.chunk.code);
        CallFrame { closure, ip, slot}
    }
    
    /// Reuse this frame for a new function call (avoids allocation)
    pub fn reset(&mut self, closure: FnClosure, slot: usize) {
        self.closure = closure;
        self.slot = slot;
        self.ip = IP::new(&self.closure.func.chunk.code);
    }

    pub fn i(&self, idx: usize) -> usize {
        self.slot + idx
    }
}

impl CallStack {
    pub fn new() -> CallStack {
        CallStack { 
            frames: Vec::new(),
            frame_pool: Vec::new(),
        }
    }
    
    pub fn push(&mut self, closure: FnClosure, slot: usize) {
        // Try to reuse a frame from the pool first
        if let Some(mut frame) = self.frame_pool.pop() {
            frame.reset(closure, slot);
            self.frames.push(frame);
        } else {
            // Create new frame only if pool is empty
            let frame = CallFrame::new(closure, slot);
            self.frames.push(frame);
        }
    }
    
    pub fn pop(&mut self) {
        // Return the frame to the pool for reuse instead of dropping it
        if let Some(frame) = self.frames.pop() {
            // Keep a reasonable pool size to avoid unbounded memory growth
            if self.frame_pool.len() < 16 {
                self.frame_pool.push(frame);
            }
            // If pool is full, just drop the frame (normal behavior)
        }
    }
    
    pub fn disassemble(&self, name: &str) {
        self.frames.last().unwrap().closure.func.chunk.disassemble(name).unwrap();
    }
    
    pub fn constants(&self) -> &Vec<WeaveType> {
        &self.frames.last().unwrap().closure.func.chunk.constants
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
        self.cur_frame().closure.func.chunk.line_number_at(point)
    }

    pub fn get_constant(&mut self, idx: usize) -> &WeaveType {
        self.cur_frame().closure.func.chunk.get_constant(idx)
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

pub type VMResult = Result<NanBoxedValue, VMError>;

impl VM {
    pub fn new() -> VM {
        let mut vm = VM {
            call_stack: CallStack::new(),
            stack: Vec::with_capacity(255),
            globals: HashMap::new(),
            last_value: NanBoxedValue::null(),
            open_upvalues: Vec::new(),
        };

        NativeFnType::variants().iter().for_each(|fn_type| {
            vm.define_native(Rc::new(NativeFn::get(fn_type.clone())));
        });

        vm
    }

    // Temporary conversion helpers - will be removed when WeaveType is fully eliminated
    pub fn weave_type_to_nan_boxed(&self, wt: WeaveType) -> NanBoxedValue {
        use crate::weave::vm::types::PointerTag;
        match wt {
            WeaveType::None => NanBoxedValue::null(),
            WeaveType::Boolean(b) => NanBoxedValue::boolean(b),
            WeaveType::Number(n) => NanBoxedValue::number(n.to_f64()),
            WeaveType::String(s) => {
                // For now, leak string to get a stable pointer - this is temporary
                let leaked = Box::leak(Box::new(s));
                NanBoxedValue::pointer(leaked as *const _ as *const (), PointerTag::String)
            },
            WeaveType::Closure(c) => {
                let leaked = Box::leak(Box::new(c));
                NanBoxedValue::pointer(leaked as *const _ as *const (), PointerTag::Closure)
            },
            WeaveType::NativeFn(f) => {
                let leaked = Box::leak(Box::new(f));
                NanBoxedValue::pointer(leaked as *const _ as *const (), PointerTag::NativeFn)
            },
            WeaveType::Upvalue(u) => {
                let leaked = Box::leak(Box::new(u));
                NanBoxedValue::pointer(leaked as *const _ as *const (), PointerTag::Upvalue)
            },
        }
    }

    pub fn nan_boxed_to_weave_type(&self, nb: NanBoxedValue) -> WeaveType {
        use crate::weave::vm::types::PointerTag;
        if nb.is_null() {
            WeaveType::None
        } else if nb.is_boolean() {
            WeaveType::Boolean(nb.as_boolean())
        } else if nb.is_number() {
            WeaveType::Number(nb.as_number().into())
        } else if nb.is_pointer() {
            let (ptr, tag) = nb.as_pointer();
            match tag {
                PointerTag::String => {
                    unsafe {
                        let string_ref = &*(ptr as *const WeaveString);
                        WeaveType::String(string_ref.clone())
                    }
                }
                PointerTag::Closure => {
                    unsafe {
                        let closure_ref = &*(ptr as *const FnClosure);
                        WeaveType::Closure(closure_ref.clone())
                    }
                }
                PointerTag::NativeFn => {
                    unsafe {
                        let fn_ref = &*(ptr as *const Rc<NativeFn>);
                        WeaveType::NativeFn(fn_ref.clone())
                    }
                }
                PointerTag::Upvalue => {
                    unsafe {
                        let upvalue_ref = &*(ptr as *const WeaveUpvalue);
                        WeaveType::Upvalue(upvalue_ref.clone())
                    }
                }
                _ => WeaveType::None,
            }
        } else {
            WeaveType::None
        }
    }

    pub fn interpret(&mut self, source: &str) -> VMResult {
        let mut compiler = Compiler::new(source, false);
        self.debug(&format!("Compiling...\n{}", source));
        let func = match compiler.compile() {
            Ok(c) => c,
            Err(msg) => return Err(VMError::CompilationError(msg)),
        };
        
        let top_frame = FnClosure::new(Rc::new(func));

        self._push_weave_type(WeaveType::Closure(top_frame.clone()));
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
    
    pub fn get_stack_var(&self, slot: usize) -> Option<WeaveType> {
        self.stack.get(slot).map(|nb| self.nan_boxed_to_weave_type(*nb))
    }
    
    pub fn clone_stack_var(&mut self, slot: usize) -> WeaveType { 
        self.nan_boxed_to_weave_type(self.stack[slot])
    }

    pub fn set_stack_var(&mut self, slot: usize, value: WeaveType) {
        self.stack[slot] = self.weave_type_to_nan_boxed(value);
    }
    
    pub fn stack_len(&self) -> usize {
        self.stack.len()
    }
    
    pub fn current_frame(&self) -> &CallFrame {
        self.call_stack.frames.last().unwrap()
    }
    
    pub fn add_local_upvalue(&mut self, closure: &mut FnClosure, uv: Upvalue) {
        // Creates a new (open) upvalue in the given frame, using the given local index as the slot
        // uv.idx is the local variable index in the current frame (where the closure is being created)
        // We need to convert it to an absolute stack position
        
        // For local upvalues, they come from the current frame (not parent)
        let current_frame_idx = self.call_stack.frames.len() - 1;
        let current_frame = &self.call_stack.frames[current_frame_idx];
        let current_frame_slot = current_frame.slot;
        // Local variables are indexed starting from 0 (which is the function itself)
        // So local at index N is at stack position: frame_slot + N
        let absolute_slot = current_frame_slot + uv.idx as usize;
        
        // Check if we already have an open upvalue for this slot
        let existing_upvalue = self.open_upvalues.iter()
            .find(|uv| uv.is_open() && uv.get_stack_index() == absolute_slot)
            .cloned();
            
        let upvalue = if let Some(existing) = existing_upvalue {
            // Reuse existing upvalue
            existing
        } else {
            // Create new upvalue and register it
            let new_upvalue = WeaveUpvalue::open(absolute_slot);
            self.open_upvalues.push(new_upvalue.clone());
            new_upvalue
        };
        
        closure.upvalues.push(upvalue);
    }
    
    pub fn add_remote_upvalue(&mut self, closure: &mut FnClosure, uv: Upvalue) {
        // Remote upvalues reference an upvalue in the current frame's closure
        let current_upvalues = &self.current_frame().closure.upvalues;
        
        // Bounds check
        if (uv.idx as usize) >= current_upvalues.len() {
            panic!("Remote upvalue index {} out of bounds (upvalues length: {})", 
                   uv.idx, current_upvalues.len());
        }
        
        let source_upvalue = current_upvalues[uv.idx as usize].clone();
        closure.upvalues.push(source_upvalue);
    }
    
    pub fn get_upvalue(&self, idx: usize) -> Option<WeaveType> {
        self.current_frame().closure.upvalues.get(idx).map(|uv| uv.value(self))
    }

    pub fn close_upvalues(&mut self, last_slot: usize) {
        // Close all open upvalues that reference stack slots at or above last_slot
        let mut indices_to_close = Vec::new();
        
        
        for (i, upvalue) in self.open_upvalues.iter().enumerate() {
            if upvalue.is_open() && upvalue.get_stack_index() >= last_slot {
                indices_to_close.push(i);
            }
        }
        
        for i in indices_to_close.iter().rev() {
            // Close the upvalue - this updates the shared reference for all closures
            // We need to clone to avoid borrowing issues
            let mut upvalue_clone = self.open_upvalues[*i].clone();
            upvalue_clone.close(self);
            // The Rc<RefCell<>> ensures all references are updated
        }
        
        // Remove closed upvalues from the open_upvalues list
        self.open_upvalues.retain(|uv| uv.is_open());
    }

    fn _read_constant(&mut self, idx: usize) -> &WeaveType {
        self.call_stack.get_constant(idx)
    }

    fn _push_weave_type(&mut self, value: WeaveType) {
        let nan_boxed = self.weave_type_to_nan_boxed(value);
        self.stack.push(nan_boxed);
    }

    fn _push(&mut self, value: OpResult) -> Result<(), VMError> {
        // TODO: This is the where "malloc" would be in C
        // self.debug(&format!("PUSH: {:?}", value));
        // If we encountered an error in performing an action, we may need to raise an error.
        // We can handle most of that here to make everyone else' lives easier - just return
        // whatever val/error you have and when we try to push it to the stack, determine if
        // something went wrong.
        match value {
            Ok(v) => {
                // Convert WeaveType to NanBoxedValue for now - will be removed in later subtasks
                let nan_boxed = self.weave_type_to_nan_boxed(v);
                self.stack.push(nan_boxed);
                Ok(())
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
        // self.debug(&format!("POP: {:?}", self._peek()));
        let nan_boxed = self.stack.pop().unwrap_or(NanBoxedValue::null());
        self.nan_boxed_to_weave_type(nan_boxed)
    }

    fn _peek(&self) -> WeaveType {
        let nan_boxed = self.stack.last().copied().unwrap_or(NanBoxedValue::null());
        self.nan_boxed_to_weave_type(nan_boxed)
    }
    
    fn _peek_ref(&self) -> WeaveType {
        // Note: Can't return reference to converted value, so return owned value
        self._peek()
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

        #[cfg(feature = "vm-profiling")]
        let mut opcode_times: std::collections::HashMap<String, (u64, u64)> = std::collections::HashMap::new(); // (total_ns, count)
        while !self.call_stack.is_at_end() {
            // until ip offset > chunk size
            let op = self.call_stack.next_op();

            #[cfg(feature = "vm-profiling")]
            let start_time = std::time::Instant::now();

            // self.debug(&format!("EVAL({:?})", op));
            match op {
                Op::INVALID(_) => {
                    return Err(VMError::InvalidChunk);
                }
                Op::RETURN => {
                    let result = self._pop();
                    
                    // Close upvalues before cleaning up the stack
                    let current_frame_slot = self.current_frame().slot;
                    self.close_upvalues(current_frame_slot);
                    
                    // Now we can clean up the stack - remove everything from the frame slot onwards
                    self.stack.truncate(current_frame_slot);
                    
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        #[cfg(feature = "vm-profiling")]
                        {
                            // Track the final opcode before early return
                            let elapsed = start_time.elapsed().as_nanos() as u64;
                            let opcode_name = format!("{:?}", op);
                            let entry = opcode_times.entry(opcode_name).or_insert((0, 0));
                            entry.0 += elapsed;
                            entry.1 += 1;
                            // Print profiling before early return
                            eprintln!("VM execution completed (early return). Opcodes tracked: {}", opcode_times.len());
                            if !opcode_times.is_empty() {
                                eprintln!("Opcode Performance Profile:");
                                let mut sorted_opcodes: Vec<_> = opcode_times.iter().collect();
                                sorted_opcodes.sort_by(|a, b| b.1.0.cmp(&a.1.0)); // Sort by total time desc
                                for (opcode, (total_ns, count)) in sorted_opcodes.iter().take(10) {
                                    let avg_ns = *total_ns / *count;
                                    eprintln!("  {:15} {:8} calls, {:10} ns total, {:6} ns avg", 
                                             opcode, count, total_ns, avg_ns);
                                }
                                eprintln!();
                            }
                        }
                        // Don't pop from empty stack
                        let nan_boxed_result = self.weave_type_to_nan_boxed(result);
                        return Ok(nan_boxed_result);
                    }
                    
                    // self.debug(&format!("Returning: {} from depth {}", result, self.stack.len()));
                    self._push(Ok(result))?;
                },
                Op::POP => { self._pop(); },
                Op::CONSTANT => {
                    let idx = self.call_stack.next_u16() as usize;
                    #[cfg(debug_assertions)]
                    self.debug(&format!("Reading constant @ {:0x}", idx));
                    // Push constant directly (still need to clone for ownership)
                    let constant = self.call_stack.get_constant(idx).clone();
                    self._push_weave_type(constant);
                }
                Op::Closure => {
                    let idx = self.call_stack.next_u16() as usize;
                    self.debug(&format!("Reading closure @ {:0x}", idx));
                    let val = self._read_constant(idx).clone();
                    match val {
                        WeaveType::Closure(mut closure) => {
                            // Process upvalues that follow the closure constant
                            for _ in 0..closure.func.upvalue_count {
                                let frame = self.call_stack.cur_frame();
                                let bytecode = &frame.closure.func.chunk.code;
                                let offset = frame.ip.ip;
                                let upvalue = Upvalue::from_bytes(bytecode, offset);
                                // Skip the upvalue bytes we just read
                                drop(frame); // Explicitly drop to release borrow
                                self.call_stack.cur_frame().ip.ip += 2;
                                
                                if upvalue.is_local {
                                    // Create upvalue from local variable in current frame
                                    self.add_local_upvalue(&mut closure, upvalue);
                                } else {
                                    // Copy upvalue from parent frame
                                    self.add_remote_upvalue(&mut closure, upvalue);
                                }
                            }
                            self._push(Ok(WeaveType::Closure(closure)))?;
                        }
                        x => {
                            return Err(VMError::CompilationError(format!("Expected callable closure, found {:?}", x)));
                        }
                    }
                }
                Op::Call => {
                    let arg_count= self.call_stack.next_byte() as usize;
                    let func_slot = (self.stack.len() - 1) - arg_count;
                    let func_nan_boxed = self.stack.get(func_slot).unwrap();
                    let func = self.nan_boxed_to_weave_type(*func_nan_boxed);
                    #[cfg(debug_assertions)]
                    self.debug(&format!("Taking {} @ {}", func, func_slot));
                    match func {
                        WeaveType::Closure(f) => {
                            // Inline validation to eliminate double cloning
                            if f.func.arity != arg_count {
                                return Err(VMError::RuntimeError { 
                                    line: self.call_stack.line_number_at(-1), 
                                    msg: format!("{} Expected {} arguments but got {}", f.func.name, f.func.arity, arg_count) 
                                });
                            }
                            if self.call_stack.frames.len() > 100 {
                                return Err(VMError::RuntimeError { 
                                    line: self.call_stack.line_number_at(-1), 
                                    msg: "Stack overflow".to_string() 
                                });
                            }
                            
                            // Standard path: full CallFrame creation  
                            self.call_stack.push(f.clone(), func_slot);
                        }
                        WeaveType::NativeFn(f) => {
                            // Convert NanBoxedValue args to WeaveType for native function call
                            let args = if arg_count > 0 {
                                let last_arg = self.stack.len() - 1;
                                let first_arg = last_arg - arg_count;
                                let nan_boxed_args = &self.stack[first_arg..last_arg];
                                let converted_args: Vec<WeaveType> = nan_boxed_args.iter()
                                    .map(|nb| self.nan_boxed_to_weave_type(*nb))
                                    .collect();
                                (f.func)(&converted_args)?
                            } else {
                                (f.func)(&[])?
                            };
                        }
                        _ => {
                            return Err(VMError::RuntimeError { line: self.call_stack.line_number_at(-1), msg: "Only functions can be called".to_string() })
                        }
                    }
                }
                Op::SetLocal => {
                    let slot = self.call_stack.next_slot();
                    let value = self._pop();
                    let nan_boxed_value = self.weave_type_to_nan_boxed(value.clone());
                    // Ensure stack is large enough for the slot
                    while self.stack.len() <= slot {
                        self.stack.push(NanBoxedValue::null());
                    }
                    self.stack[slot] = nan_boxed_value;
                    self._push_weave_type(value); // Push the assigned value back for expression semantics
                }
                Op::GetLocal => {
                    let slot = self.call_stack.next_slot();
                    if slot >= self.stack.len() {
                        return Err(VMError::RuntimeError { 
                            line: self.call_stack.line_number_at(-1), 
                            msg: format!("Stack index out of bounds: slot={}, stack_len={}", slot, self.stack.len()) 
                        });
                    }
                    // Use reference to avoid cloning during push
                    let value = self.stack[slot];
                    self.stack.push(value);
                }
                Op::GetUpvalue => {
                    let slot = self.call_stack.next_byte() as usize;
                    // Ultra-fast path: direct NanBoxedValue access with Copy semantics
                    let upvalue = self.call_stack.cur_frame().closure.upvalues[slot].clone();
                    let nan_boxed_value = upvalue.get_fast(self);
                    self.stack.push(nan_boxed_value);
                }
                Op::SetUpvalue => {
                    let slot = self.call_stack.next_byte() as usize;
                    // Ultra-fast path: direct NanBoxedValue access with Copy semantics
                    let nan_boxed_value = self.stack[self.stack.len() - 1]; // peek top of stack
                    let upvalue = self.call_stack.frames.last().unwrap()
                        .closure.upvalues[slot].clone();
                    upvalue.set_fast(nan_boxed_value, self);
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
                            let nan_boxed_val = self.weave_type_to_nan_boxed(val.clone());
                            self.globals.insert(name.to_string(), nan_boxed_val);
                            self._push_weave_type(val); // Push the assigned value back for expression semantics
                        }
                        _ => { unreachable!("Only strings can become globals - how did you get here?"); }
                    }
                }
                Op::GetGlobal => {
                    let name = self._pop();
                    match name {
                        WeaveType::String(name) => match self.globals.get(name.as_str()) {
                            Some(v) => {
                                let value = self.nan_boxed_to_weave_type(*v);
                                self._push(Ok(value))?;
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
                    // Fast-path NaN-boxed arithmetic
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_add(b) {
                        self.stack.push(result);
                    } else {
                        // Fallback to WeaveType operations for string concatenation, etc.
                        let a_weave = self.nan_boxed_to_weave_type(a);
                        let b_weave = self.nan_boxed_to_weave_type(b);
                        match a_weave + b_weave {
                            Ok(result) => self._push_weave_type(result),
                            Err(e) => return Err(VMError::RuntimeError { 
                                line: self.call_stack.line_number_at(-1), 
                                msg: format!("Addition failed: {}", e) 
                            }),
                        }
                    }
                }
                Op::SUB => {
                    // Fast-path NaN-boxed arithmetic
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_sub(b) {
                        self.stack.push(result);
                    } else {
                        // Fallback to WeaveType operations
                        let a_weave = self.nan_boxed_to_weave_type(a);
                        let b_weave = self.nan_boxed_to_weave_type(b);
                        match a_weave - b_weave {
                            Ok(result) => self._push_weave_type(result),
                            Err(e) => return Err(VMError::RuntimeError { 
                                line: self.call_stack.line_number_at(-1), 
                                msg: format!("Subtraction failed: {}", e) 
                            }),
                        }
                    }
                }
                Op::MUL => {
                    // Fast-path NaN-boxed arithmetic
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_mul(b) {
                        self.stack.push(result);
                    } else {
                        // Fallback to WeaveType operations
                        let a_weave = self.nan_boxed_to_weave_type(a);
                        let b_weave = self.nan_boxed_to_weave_type(b);
                        match a_weave * b_weave {
                            Ok(result) => self._push_weave_type(result),
                            Err(e) => return Err(VMError::RuntimeError { 
                                line: self.call_stack.line_number_at(-1), 
                                msg: format!("Multiplication failed: {}", e) 
                            }),
                        }
                    }
                }
                Op::DIV => {
                    // Fast-path NaN-boxed arithmetic
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_div(b) {
                        self.stack.push(result);
                    } else {
                        // Fallback to WeaveType operations
                        let a_weave = self.nan_boxed_to_weave_type(a);
                        let b_weave = self.nan_boxed_to_weave_type(b);
                        match a_weave / b_weave {
                            Ok(result) => self._push_weave_type(result),
                            Err(e) => return Err(VMError::RuntimeError { 
                                line: self.call_stack.line_number_at(-1), 
                                msg: format!("Division failed: {}", e) 
                            }),
                        }
                    }
                }
                Op::TRUE => {
                    self._push_weave_type(WeaveType::Boolean(true));
                }
                Op::FALSE => {
                    self._push_weave_type(WeaveType::Boolean(false));
                }
                Op::NOT => {
                    // Everything is truthy in Weave, so we just need to negate
                    // the top value's "truthiness"
                    let val = WeaveType::Boolean(!self._pop().truthy());
                    self._push(Ok(val))?;
                }
                Op::GREATER => {
                    // Fast-path NaN-boxed comparison
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_greater(b) {
                        self.stack.push(result);
                    } else {
                        // Fallback to WeaveType comparison
                        let a_weave = self.nan_boxed_to_weave_type(a);
                        let b_weave = self.nan_boxed_to_weave_type(b);
                        self._push_weave_type(WeaveType::Boolean(a_weave > b_weave));
                    }
                }
                Op::LESS => {
                    // Fast-path NaN-boxed comparison
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_less(b) {
                        self.stack.push(result);
                    } else {
                        // Fallback to WeaveType comparison
                        let a_weave = self.nan_boxed_to_weave_type(a);
                        let b_weave = self.nan_boxed_to_weave_type(b);
                        self._push_weave_type(WeaveType::Boolean(a_weave < b_weave));
                    }
                }
                Op::EQUAL => {
                    // Fast-path NaN-boxed comparison
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    let result = a.fast_equal(b);
                    self.stack.push(result);
                }
                Op::PRINT => {
                    // Don't remove the top value from the stack - printing a value evaluates
                    // to the value itself. e.g. "print(1) == 1"
                    let value = self._peek_ref();
                    println!("{}", green(&format!("{}", value)));
                    log_debug!("VM print instruction", value = format!("{}", value).as_str(), stack_depth = self.stack.len());
                }
                Op::Jump => {
                    let jmp_target = self.call_stack.next_u16();
                    self.call_stack.jump(jmp_target);
                }
                Op::JumpIfFalse => {
                    let jmp_offset = self.call_stack.next_u16();
                    if !self._peek_ref().truthy() {
                        self.call_stack.jump(jmp_offset);
                    }
                }
                Op::Loop => {
                    let jmp_offset = self.call_stack.next_u16();
                    self.call_stack.jump_back(jmp_offset);
                }
            }

            #[cfg(feature = "vm-profiling")]
            {
                let elapsed = start_time.elapsed().as_nanos() as u64;
                let opcode_name = format!("{:?}", op);
                let entry = opcode_times.entry(opcode_name).or_insert((0, 0));
                entry.0 += elapsed;
                entry.1 += 1;
            }

            self.debug(&format!("  - {:?}", self.stack));
            self.debug(&format!("  - {:?}", self.call_stack.constants()));
        }

        #[cfg(feature = "vm-profiling")]
        {
            eprintln!("VM execution completed. Opcodes tracked: {}", opcode_times.len());
            if !opcode_times.is_empty() {
                eprintln!("Opcode Performance Profile:");
                let mut sorted_opcodes: Vec<_> = opcode_times.iter().collect();
                sorted_opcodes.sort_by(|a, b| b.1.0.cmp(&a.1.0)); // Sort by total time desc
                for (opcode, (total_ns, count)) in sorted_opcodes.iter().take(10) {
                    let avg_ns = *total_ns / *count;
                    eprintln!("  {:15} {:8} calls, {:10} ns total, {:6} ns avg", 
                             opcode, count, total_ns, avg_ns);
                }
                eprintln!();
            } else {
                eprintln!("No opcodes were executed!");
            }
        }

        Ok(self.last_value)
    }

    fn debug(&self, msg: &str) {
        #[cfg(debug_assertions)]
        log_debug!("VM debug", message = msg, stack_depth = self.stack.len());
    }

    fn runtime_error(&mut self, line: usize, msg: &String) {
        let callstack = self.call_stack.frames.iter().rev();
        for frame in callstack {
            let func = &frame.closure.func;
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
        let nan_boxed_func = self.weave_type_to_nan_boxed(WeaveType::NativeFn(func));
        self.globals.insert(name, nan_boxed_func);
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
        self.call_stack.reset();
    }
    
    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_math() {
        let mut vm = VM::new();
        let res = vm.interpret("5 + 2 * 3");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        let result = res.unwrap();
        assert!(result.is_number());
        assert_eq!(result.as_number(), 11.0);
    }

    #[test]
    fn test_parenthesis() {
        let mut vm = VM::new();
        let res = vm.interpret("(5 + 2) * 3");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::number(21.0));
    }

    #[test]
    fn test_negate() {
        let mut vm = VM::new();
        let res = vm.interpret("-5");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::number(-5.0));
    }

    #[test]
    fn test_string_literal() {
        let mut vm = VM::new();
        let res = vm.interpret("\"hello\"");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        // String values are stored as pointers in NanBoxedValue, so we can't directly compare
        // Instead, let's convert back to WeaveType to verify the value
        let result_as_weave = vm.nan_boxed_to_weave_type(res.unwrap());
        assert_eq!(result_as_weave, WeaveType::from("hello"));
    }
    
    #[test]
    fn test_var_addition() {
        let mut vm = VM::new();
        let res = vm.interpret("x = 5\nx + 2");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(7.0));
    }

    #[test]
    fn test_puts_statement() {
        let mut vm = VM::new();
        let res = vm.interpret("puts \"hello\";");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
    }

    #[test]
    fn test_using_var() {
        let mut vm = VM::new();
        let res = vm.interpret("x = 5; puts x;");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
    }

    #[test]
    fn test_declaring_var() {
        let mut vm = VM::new();
        let res = vm.interpret("x = 5");
        assert_eq!(vm.stack.len(), 0);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert!(
            vm.globals.contains_key("x"),
            "Global \"x\" not found in {:?}",
            vm.globals.keys().collect::<Vec<&String>>()
        );
        assert_eq!(vm.globals["x"], NanBoxedValue::number(5.0));
    }
    
    #[test]
    fn test_invalid_assignment_doesnt_parse() {
        let mut vm = VM::new();
        let res = vm.interpret("a= 1; a + b = 5");
        assert!(res.is_err());
    }

    #[test]
    fn test_shadowing_self() {
        let mut vm = VM::new();
        let res = vm.interpret("
        a = 1;  # Global var
        fn foo() {
            a = a   # Shadows global with a local
            a = a + 2  # Increments local
        }
        foo()
        
        a ");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(1.0));
    }

    #[test]
    fn test_bad_initializer() {
        let mut vm = VM::new();
        let res = vm.interpret("a = a");
        assert!(res.is_err());
    }

    #[test]
    fn test_local_variables() {
        let mut vm = VM::new();
        let res = vm.interpret("fn test() { x = 1; x + 3 } test()");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(4.0));
    }

    #[test]
    fn test_nested_scopes() {
        let mut vm = VM::new();
        // Note: Updated to use functions instead of bare blocks 
        // This test now verifies closure variable capture instead of nested blocks
        let res = vm.interpret("fn outer() { x = 2; fn inner() { x = x + 3; x } inner() } outer()");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(5.0)); // 2 + 3 = 5
    }

    #[test]
    fn test_if_true_condition() {
        let mut vm = VM::new();
        let res = vm.interpret("fn test() {
        a = 1;
        if (true) { a = a + 1 }
        a} test()");
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(2.0));
    }

    #[test]
    fn test_if_false_condition() {
        let code = "fn test() {
        a = 1;
        if false { a = a + 1 }
        a
        } test()";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(1.0));
    }

    #[test]
    fn test_if_else_condition() {
        let code = "fn test() {
        a = 1;
        if false {
            a = a + 1
        } else {
            a = a + 2
        }
        a
        } test()";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(3.0));
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
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
    }
    
    #[test]
    fn test_while_syntax() {
        let code = "fn test() {
            a = 1  
            while a < 3 {
                a = a + 1
            }
            a
        } test()";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(3.0));
    }
    
    #[test]
    fn test_fn_definition() {
        let code = "
            fn add(a, b) { 
                a + b 
            }
            add(-1, 4)
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(3.0));
    }
    
    #[test]
    fn test_simple_closure() {
        let code = "
            fn make_counter() {
              count = 0
              fn counter() {
                count = count + 1
                count
              }
              counter
            }
            c = make_counter()
            c()
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(1.0));
    }

    #[test]
    fn test_closures() {
        let code = "
            fn outer() {
              a = 1;
              b = 2;
              fn middle() {
                c = 3;
                d = 4;
                fn inner() {
                  a + c + b + d
                }
                inner()
              }
              middle()
            }
            outer()
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(10.0));
    }

    #[test]
    fn test_basic_lambda() {
        let code = "
            add = ^(a, b) { a + b }
            add(3, 4)
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(7.0));
    }

    #[test]
    fn test_lambda_with_closure() {
        let code = "
            fn make_adder(x) {
                ^(y) { x + y }
            }
            add5 = make_adder(5)
            add5(10)
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(15.0));
    }

    #[test]
    fn test_lambda_no_params() {
        let code = "
            getValue = ^() { 42 }
            getValue()
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(42.0));
    }

    #[test]
    fn test_lambda_single_param() {
        let code = "
            square = ^(x) { x * x }
            square(6)
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(36.0));
    }

    #[test]
    fn test_multiple_lambdas_sequential() {
        let code = "
            add = ^(a, b) { a + b }
            result1 = add(3, 4)
            
            square = ^(x) { x * x }
            result2 = square(5)
            
            result1 + result2
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(32.0)); // 7 + 25 = 32
    }

    #[test]
    fn test_multiple_lambdas_same_line() {
        let code = "
            add = ^(a, b) { a + b }
            mul = ^(x, y) { x * y }
            add(3, 4) + mul(5, 6)
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(37.0)); // 7 + 30 = 37
    }

    #[test]
    fn test_lambda_sequence_with_strings() {
        let code = "
            add = ^(a, b) { a + b }
            getMessage = ^() { \"Hello\" }
            
            result = add(3, 4)
            msg = getMessage()
            result
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(7.0));
    }

    #[test]
    fn test_lambda_with_intermediate_variables() {
        let code = "
            lambda1 = ^(x) { x + 1 }
            temp = 5
            lambda2 = ^(y) { y * 2 }
            
            result1 = lambda1(temp)
            result2 = lambda2(temp)
            result1 + result2
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(16.0)); // 6 + 10 = 16
    }

    #[test]
    fn test_sequential_named_functions() {
        let code = "
            fn func1(x) { x + 1 }
            temp = 5
            fn func2(y) { y * 2 }
            
            result1 = func1(temp)
            result2 = func2(temp)
            result1 + result2
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(16.0)); // 6 + 10 = 16
    }

    #[test]
    fn test_nested_lambda_calls() {
        let code = "
            add = ^(a, b) { a + b }
            mul = ^(x, y) { x * y }
            
            add(mul(2, 3), mul(4, 5))
        ";
        let mut vm = VM::new();
        let res = vm.interpret(code);
        assert!(res.is_ok(), "Failed to interpret: {:?}", res.unwrap_err());
        assert_eq!(res.unwrap(), NanBoxedValue::from(26.0)); // add(6, 20) = 26
    }
}
