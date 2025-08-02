use crate::weave::compiler::Compiler;
use crate::weave::vm::instruction_pointer::IP;
use crate::weave::vm::types::{FnClosure, NanBoxedValue, NativeFn, NativeFnType, PointerTag, Upvalue, WeaveUpvalue};
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
    
    // Arena allocators for memory management
    closure_arena: crate::weave::vm::types::ClosureArena,
    upvalue_arena: crate::weave::vm::types::UpvalueArena,
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
    pub closure: *const FnClosure,
    pub slot: usize,
    ip: IP,
}

impl CallFrame {
    pub fn new(closure_ptr: *const FnClosure, slot: usize) -> CallFrame {
        let closure = unsafe { &*closure_ptr };
        let ip = IP::new(&closure.func.chunk.code);
        CallFrame { closure: closure_ptr, ip, slot}
    }
    
    /// Reuse this frame for a new function call (avoids allocation)
    pub fn reset(&mut self, closure_ptr: *const FnClosure, slot: usize) {
        let closure = unsafe { &*closure_ptr };
        self.closure = closure_ptr;
        self.slot = slot;
        self.ip = IP::new(&closure.func.chunk.code);
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
    
    pub fn push(&mut self, closure_ptr: *const FnClosure, slot: usize) {
        // Try to reuse a frame from the pool first
        if let Some(mut frame) = self.frame_pool.pop() {
            frame.reset(closure_ptr, slot);
            self.frames.push(frame);
        } else {
            // Create new frame only if pool is empty
            let frame = CallFrame::new(closure_ptr, slot);
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
        let closure = unsafe { &*self.frames.last().unwrap().closure };
        closure.func.chunk.disassemble(name).unwrap();
    }
    
    pub fn constants(&self) -> &Vec<NanBoxedValue> {
        let closure = unsafe { &*self.frames.last().unwrap().closure };
        &closure.func.chunk.constants
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
        let absolute_slot = self.cur_frame().i(relative_slot);
        absolute_slot
    }
    
    pub fn jump(&mut self, offset: u16) {
        self.cur_frame().ip.jump(offset);
    }
    
    pub fn jump_back(&mut self, offset: u16) {
        self.cur_frame().ip.jump_back(offset);
    }

    pub fn line_number_at(&mut self, offset: isize) -> usize {
        let point = self.cur_frame().ip.idx(offset);
        let closure = unsafe { &*self.cur_frame().closure };
        closure.func.chunk.line_number_at(point)
    }

    pub fn get_constant(&mut self, idx: usize) -> NanBoxedValue {
        let closure = unsafe { &*self.cur_frame().closure };
        closure.func.chunk.get_constant(idx)
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn is_at_end(&self) -> bool {
        self.frames.is_empty() || self.frames.last().unwrap().ip.is_at_end()
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
            closure_arena: crate::weave::vm::types::ClosureArena::with_capacity(64),
            upvalue_arena: crate::weave::vm::types::UpvalueArena::with_capacity(128),
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
        
        let top_frame = FnClosure::new(Rc::new(func));

        // Store closure in arena and create handle
        let closure_handle = self.closure_arena.insert(top_frame);
        let closure_nan_boxed = NanBoxedValue::closure_handle(closure_handle.clone());
        self.stack.push(closure_nan_boxed);
        
        // TODO: Update CallStack to use handles instead of raw pointers
        // For now, we need to get a raw pointer for compatibility
        let closure_ref = self.closure_arena.get(closure_handle).unwrap();
        let closure_ptr = closure_ref as *const FnClosure;
        self.call_stack.push(closure_ptr, 0);

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
    
    pub fn get_stack_value(&self, slot: usize) -> NanBoxedValue {
        self.stack[slot]
    }

    pub fn set_stack_value(&mut self, slot: usize, value: NanBoxedValue) {
        self.stack[slot] = value;
    }
    
    pub fn current_frame(&self) -> &CallFrame {
        self.call_stack.frames.last().unwrap()
    }
    
    pub fn add_local_upvalue(&mut self, closure: &mut FnClosure, uv: Upvalue) {
        // Creates a new upvalue for a local variable, storing it in the arena
        // uv.idx is the local variable index in the PARENT frame where the variable is defined
        
        // For local upvalues, they come from the current frame where local variables exist
        // The Closure operation runs within the frame that contains the captured variables
        let current_frame_idx = self.call_stack.frames.len() - 1; // Current frame
        let current_frame = &self.call_stack.frames[current_frame_idx];
        let current_frame_slot = current_frame.slot;
        let absolute_slot = current_frame_slot + uv.idx as usize;
        
        // Check if we already have an open upvalue for this stack slot
        // We'll scan the stack for upvalue NanBoxedValues that might reference this slot
        // let mut existing_handle = None;
        
        // For now, we'll create a new upvalue in the arena
        // TODO: Implement upvalue sharing/deduplication if needed
        
        // Create new upvalue and store it in the arena
        let new_upvalue = WeaveUpvalue::open(absolute_slot);
        let upvalue_handle = self.upvalue_arena.insert(new_upvalue);
        
        
        // Store the arena handle in the closure
        closure.upvalues.push(upvalue_handle.clone());
    }
    
    pub fn add_remote_upvalue(&mut self, closure: &mut FnClosure, uv: Upvalue) {
        // Remote upvalues reference an upvalue from the current frame's closure
        let current_closure = unsafe { &*self.current_frame().closure };
        let current_upvalues = &current_closure.upvalues;
        
        // Bounds check
        if (uv.idx as usize) >= current_upvalues.len() {
            panic!("Remote upvalue index {} out of bounds (upvalues length: {})", 
                   uv.idx, current_upvalues.len());
        }
        
        // Get the upvalue handle from the parent closure
        let source_upvalue_handle = current_upvalues[uv.idx as usize].clone();
        closure.upvalues.push(source_upvalue_handle);
    }
    

    pub fn close_upvalues(&mut self, last_slot: usize) {
        // Close all upvalues that reference stack slots >= last_slot
        // We need to collect handles and their current values first to avoid borrow checker issues
        let mut upvalues_to_close = Vec::new();
        
        log_debug!("CLOSE_UPVALUES DEBUG", last_slot = last_slot, stack_len = self.stack.len());
        
        for (handle, upvalue) in self.upvalue_arena.iter() {
            if upvalue.is_open() && upvalue.get_stack_index() >= last_slot {
                let slot = upvalue.get_stack_index();
                log_debug!("UPVALUE TO CLOSE", slot = slot, stack_len = self.stack.len());
                
                if slot >= self.stack.len() {
                    log_debug!("UPVALUE SLOT OUT OF BOUNDS", slot = slot, stack_len = self.stack.len());
                    // Skip this upvalue - it's already invalid
                    continue;
                }
                
                let value = self.stack[slot]; // Copy the current stack value
                upvalues_to_close.push((handle.clone(), value));
            }
        }
        
        // Now close each upvalue using the copied values
        for (handle, value) in upvalues_to_close {
            if let Some(upvalue) = self.upvalue_arena.get(handle) {
                upvalue.close_with_value(value);
            }
        }
    }

    fn _read_constant(&mut self, idx: usize) -> NanBoxedValue {
        self.call_stack.get_constant(idx)
    }


    pub fn run(&mut self) -> VMResult {
        if self.call_stack.is_empty() { return Err(VMError::InvalidChunk); }

        self.debug("Executing...");
        log_debug!("Starting VM execution", function = "main");

        #[cfg(feature = "vm-profiling")]
        let mut opcode_times: std::collections::HashMap<String, (u64, u64)> = std::collections::HashMap::new(); // (total_ns, count)
        
        #[cfg(feature = "vm-profiling")]
        let mut memory_samples: Vec<(usize, usize, usize, usize, usize, usize)> = Vec::new(); // (iteration, stack_len, upvalues_len, arena_len, globals_len, frames_len)
        
        #[cfg(feature = "vm-profiling")]
        let mut iteration_count = 0;
        while !self.call_stack.is_at_end() {
            // until ip offset > chunk size
            let op = self.call_stack.next_op();

            #[cfg(feature = "vm-profiling")]
            {
                iteration_count += 1;
                // Sample memory usage every 100 iterations to avoid overhead
                if iteration_count % 100 == 0 {
                    memory_samples.push((
                        iteration_count,
                        self.stack.len(),
                        self.upvalue_arena.len(),
                        self.closure_arena.len(),
                        self.globals.len(),
                        self.call_stack.frames.len(),
                    ));
                }
            }

            #[cfg(feature = "vm-profiling")]
            let start_time = std::time::Instant::now();

            // self.debug(&format!("EVAL({:?})", op));
            match op {
                Op::INVALID(_) => {
                    return Err(VMError::InvalidChunk);
                }
                Op::RETURN => {
                    let result = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    // Close upvalues before cleaning up the stack
                    let current_frame_slot = self.current_frame().slot;
                    self.close_upvalues(current_frame_slot);
                    
                    // Now we can clean up the stack - remove everything from the frame slot onwards
                    let old_len = self.stack.len();
                    self.stack.truncate(current_frame_slot);
                    if old_len != current_frame_slot {
                        log_debug!("STACK TRUNCATE", old_len = old_len, new_len = current_frame_slot, opcode = "RETURN", ip = format!("{:x}", self.call_stack.cur_frame().ip.ip).as_str());
                    }
                    
                    // TODO: Implement proper closure cleanup to prevent memory leaks
                    
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
                            
                            // Memory usage analysis
                            if !memory_samples.is_empty() {
                                eprintln!("Memory Usage Analysis:");
                                eprintln!("  Total samples: {}", memory_samples.len());
                                
                                if memory_samples.len() > 1 {
                                    let first = &memory_samples[0];
                                    let last = &memory_samples[memory_samples.len() - 1];
                                    
                                    eprintln!("  Growth from iteration {} to {}:", first.0, last.0);
                                    eprintln!("    Stack:      {} -> {} (+{})", first.1, last.1, last.1 as i32 - first.1 as i32);
                                    eprintln!("    Upvalues:   {} -> {} (+{})", first.2, last.2, last.2 as i32 - first.2 as i32);
                                    eprintln!("    Arena:      {} -> {} (+{})", first.3, last.3, last.3 as i32 - first.3 as i32);
                                    eprintln!("    Globals:    {} -> {} (+{})", first.4, last.4, last.4 as i32 - first.4 as i32);
                                    eprintln!("    Frames:     {} -> {} (+{})", first.5, last.5, last.5 as i32 - first.5 as i32);
                                    
                                    // Find the component with the highest growth
                                    let stack_growth = last.1 as i32 - first.1 as i32;
                                    let upvalues_growth = last.2 as i32 - first.2 as i32;
                                    let arena_growth = last.3 as i32 - first.3 as i32;
                                    let globals_growth = last.4 as i32 - first.4 as i32;
                                    let frames_growth = last.5 as i32 - first.5 as i32;
                                    
                                    let growths = [stack_growth, upvalues_growth, arena_growth, globals_growth, frames_growth];
                                    let max_growth = *growths.iter().max().unwrap();
                                    
                                    if max_growth > 0 {
                                        eprintln!("  Largest growth component: {}", 
                                            if stack_growth == max_growth { "Stack" }
                                            else if upvalues_growth == max_growth { "Upvalues" }
                                            else if arena_growth == max_growth { "Arena" }
                                            else if globals_growth == max_growth { "Globals" }
                                            else { "Frames" }
                                        );
                                    }
                                }
                                eprintln!();
                            }
                        }
                        // Don't pop from empty stack
                        return Ok(result);
                    }
                    
                    // Place return value by pushing it onto the truncated stack
                    // The stack was truncated to function_slot, so pushing the result
                    // places it where the function call was (replacing the closure)
                    self.stack.push(result);
                    log_debug!("STACK PUSH", value = format!("{:?}", result).as_str(), stack_len = self.stack.len(), opcode = "RETURN", ip = format!("{:x}", self.call_stack.cur_frame().ip.ip).as_str());
                },
                Op::POP => { 
                    if let Some(value) = self.stack.pop() {
                        self.last_value = value;
                        log_debug!("STACK POP", value = format!("{:?}", value).as_str(), stack_len = self.stack.len(), opcode = "POP", ip = format!("{:x}", self.call_stack.cur_frame().ip.ip).as_str());
                    }
                },
                Op::CloseUpvalues => {
                    let slot = self.call_stack.next_byte() as usize;
                    self.close_upvalues(slot);
                },
                Op::CONSTANT => {
                    let idx = self.call_stack.next_u16() as usize;
                    #[cfg(debug_assertions)]
                    self.debug(&format!("Reading constant @ {:0x}", idx));
                    // Push constant directly - NanBoxedValue is Copy, no clone needed!
                    let constant = self.call_stack.get_constant(idx);
                    self.stack.push(constant);
                    log_debug!("STACK PUSH", value = format!("{:?}", constant).as_str(), stack_len = self.stack.len(), opcode = "CONSTANT", ip = format!("{:x}", self.call_stack.cur_frame().ip.ip).as_str());
                }
                Op::Closure => {
                    let idx = self.call_stack.next_u16() as usize;
                    self.debug(&format!("Reading closure @ {:0x}", idx));
                    let val = self._read_constant(idx);
                    
                    if val.is_pointer() {
                        let (ptr, tag) = val.as_pointer();
                        match tag {
                            PointerTag::Closure => {
                                // Cast pointer back to FnClosure and clone it for modification
                                let closure_ref = unsafe { &*(ptr as *const FnClosure) };
                                let mut closure = closure_ref.clone();
                                
                                // Process upvalues that follow the closure constant
                                for _ in 0..closure.func.upvalue_count {
                                    let frame = self.call_stack.cur_frame();
                                    let frame_closure = unsafe { &*frame.closure };
                                    let bytecode = &frame_closure.func.chunk.code;
                                    let offset = frame.ip.ip;
                                    let upvalue = Upvalue::from_bytes(bytecode, offset);
                                    // Skip the upvalue bytes we just read
                                    let _ = frame; // Explicitly drop to release borrow
                                    self.call_stack.cur_frame().ip.ip += 2;
                                    
                                    if upvalue.is_local {
                                        // Create upvalue from local variable in current frame
                                        self.add_local_upvalue(&mut closure, upvalue);
                                    } else {
                                        // Copy upvalue from parent frame
                                        self.add_remote_upvalue(&mut closure, upvalue);
                                    }
                                }
                                
                                // Store the modified closure in arena
                                let closure_handle = self.closure_arena.insert(closure);
                                #[cfg(feature = "vm-debug")]
                                let debug_handle = closure_handle.clone();
                                let closure_nan_boxed = NanBoxedValue::closure_handle(closure_handle);
                                #[cfg(feature = "vm-debug")]
                                log_debug!("CLOSURE CREATED WITH UPVALUES", handle = format!("{:?}", debug_handle).as_str(), is_closure_handle = closure_nan_boxed.is_closure_handle());
                                self.stack.push(closure_nan_boxed);
                            }
                            _ => {
                                return Err(VMError::CompilationError(format!("Expected closure pointer, found {:?} pointer", tag)));
                            }
                        }
                    } else {
                        return Err(VMError::CompilationError(format!("Expected closure pointer, found non-pointer value")));
                    }
                }
                Op::Call => {
                    let arg_count = self.call_stack.next_byte() as usize;
                    let func_slot = (self.stack.len() - 1) - arg_count;
                    let func_nan_boxed = *self.stack.get(func_slot).unwrap();
                    
                    #[cfg(feature = "vm-debug")]
                    log_debug!("CALL DEBUG", is_closure_handle = func_nan_boxed.is_closure_handle(), is_pointer = func_nan_boxed.is_pointer(), func_value = format!("{:?}", func_nan_boxed).as_str());
                    
                    if func_nan_boxed.is_closure_handle() {
                        // New arena-based closure handle
                        let closure_handle = func_nan_boxed.as_closure_handle();
                        let closure = self.closure_arena.get(closure_handle).unwrap();
                        
                        // Inline validation
                        if closure.func.arity != arg_count {
                            return Err(VMError::RuntimeError { 
                                line: self.call_stack.line_number_at(-1), 
                                msg: format!("{} Expected {} arguments but got {}", closure.func.name, closure.func.arity, arg_count) 
                            });
                        }
                        if self.call_stack.frames.len() > 100 {
                            return Err(VMError::RuntimeError { 
                                line: self.call_stack.line_number_at(-1), 
                                msg: "Stack overflow".to_string() 
                            });
                        }
                        
                        // Get raw pointer for CallStack compatibility (temporary)
                        let closure_ptr = closure as *const FnClosure;
                        self.call_stack.push(closure_ptr, func_slot);
                    } else if func_nan_boxed.is_pointer() {
                        let (ptr, tag) = func_nan_boxed.as_pointer();
                        match tag {
                            PointerTag::Closure => {
                                // Legacy closure pointer (during transition)
                                let closure_ptr = ptr as *const FnClosure;
                                let closure = unsafe { &*closure_ptr };
                                
                                // Inline validation to eliminate double cloning
                                if closure.func.arity != arg_count {
                                    return Err(VMError::RuntimeError { 
                                        line: self.call_stack.line_number_at(-1), 
                                        msg: format!("{} Expected {} arguments but got {}", closure.func.name, closure.func.arity, arg_count) 
                                    });
                                }
                                if self.call_stack.frames.len() > 100 {
                                    return Err(VMError::RuntimeError { 
                                        line: self.call_stack.line_number_at(-1), 
                                        msg: "Stack overflow".to_string() 
                                    });
                                }
                                
                                // Pass closure pointer directly - NO CLONING!
                                self.call_stack.push(closure_ptr, func_slot);
                            }
                            PointerTag::NativeFn => {
                                // Cast pointer back to NativeFn
                                let native_fn = unsafe { &*(ptr as *const Rc<NativeFn>) };
                                
                                // Call native function directly with NanBoxedValue args
                                let result = if arg_count > 0 {
                                    let last_arg = self.stack.len() - 1;
                                    let first_arg = last_arg - arg_count;
                                    let nan_boxed_args = &self.stack[first_arg..last_arg];
                                    (native_fn.func)(nan_boxed_args)?
                                } else {
                                    (native_fn.func)(&[])?
                                };
                                
                                // Pop function and args from stack, push result
                                for _ in 0..=arg_count {
                                    self.stack.pop();
                                }
                                self.stack.push(result);
                            }
                            _ => {
                                return Err(VMError::RuntimeError { 
                                    line: self.call_stack.line_number_at(-1), 
                                    msg: "Only functions can be called".to_string() 
                                })
                            }
                        }
                    } else {
                        return Err(VMError::RuntimeError { 
                            line: self.call_stack.line_number_at(-1), 
                            msg: "Only functions can be called".to_string() 
                        });
                    }
                }
                Op::SetLocal => {
                    let relative_slot = self.call_stack.next_byte() as usize;
                    let slot = self.call_stack.cur_frame().i(relative_slot);
                    let value = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    #[cfg(feature = "vm-debug")]
                    log_debug!("SET LOCAL", slot = slot, value = format!("{:?}", nan_boxed_value).as_str());
                    // Ensure stack is large enough for the slot - use exponential growth
                    if self.stack.len() <= slot {
                        // Use exponential growth strategy to avoid O(n²) resize behavior
                        let new_size = std::cmp::max(slot + 1, self.stack.len() * 2);
                        self.stack.resize(new_size, NanBoxedValue::null());
                    }
                    self.stack[slot] = value;
                    // Value stays on stack since assignments are expressions in Weave
                }
                Op::GetLocal => {
                    let relative_slot = self.call_stack.next_byte() as usize;
                    let slot = self.call_stack.cur_frame().i(relative_slot);
                    // Ensure stack is large enough for the slot - use exponential growth
                    if self.stack.len() <= slot {
                        // Use exponential growth strategy to avoid O(n²) resize behavior
                        let new_size = std::cmp::max(slot + 1, self.stack.len() * 2);
                        self.stack.resize(new_size, NanBoxedValue::null());
                    }
                    // Use reference to avoid cloning during push
                    let value = self.stack[slot];
                    #[cfg(feature = "vm-debug")]
                    log_debug!("GET LOCAL", slot = slot, value = format!("{:?}", value).as_str());
                    self.stack.push(value);
                }
                Op::GetUpvalue => {
                    let slot = self.call_stack.next_byte() as usize;
                    // Get upvalue from arena using the handle
                    let closure = unsafe { &*self.call_stack.cur_frame().closure };
                    
                    // DEBUG: Check the bounds
                    if slot >= closure.upvalues.len() {
                        panic!("GetUpvalue: slot {} out of bounds, upvalues.len() = {}, expected upvalue_count = {}", 
                               slot, closure.upvalues.len(), closure.func.upvalue_count);
                    }
                    
                    let upvalue_handle = &closure.upvalues[slot];
                    let upvalue = self.upvalue_arena.get(upvalue_handle.clone()).unwrap();
                    let nan_boxed_value = upvalue.get_fast(self);
                    self.stack.push(nan_boxed_value);
                }
                Op::SetUpvalue => {
                    let slot = self.call_stack.next_byte() as usize;
                    // Set upvalue using the arena handle
                    let nan_boxed_value = self.stack[self.stack.len() - 1]; // peek top of stack
                    let closure = unsafe { &*self.call_stack.frames.last().unwrap().closure };
                    let upvalue_handle = closure.upvalues[slot].clone();
                    
                    // We need to work around the borrow checker here
                    // The issue is that set_fast needs &mut self, but we also have a mutable borrow from upvalue_arena
                    // Solution: Get the upvalue, check if it's open, and handle accordingly
                    let upvalue = self.upvalue_arena.get(upvalue_handle.clone()).unwrap();
                    if upvalue.is_open() {
                        let stack_index = upvalue.get_stack_index();
                        self.stack[stack_index] = nan_boxed_value;
                    } else {
                        // For closed upvalues, we need to update the stored value
                        // Use the same approach as close_upvalues to avoid borrow checker issues
                        let upvalue_handle_clone = upvalue_handle.clone();
                        drop(upvalue); // Release immutable borrow
                        
                        if let Some(upvalue) = self.upvalue_arena.get(upvalue_handle_clone) {
                            upvalue.close_with_value(nan_boxed_value);
                        }
                    }
                }
                Op::SetGlobal => {
                    // Previous to this we should have processed an expression (val)
                    // then pushed the name of the global we want to bind it to
                    // and now we need to actually bind it.
                    // So pop the name and value off the stack.
                    let name = self.stack.pop().unwrap();
                    let val = self.stack.pop().unwrap();
                    
                    if name.is_string() {
                        let name_str = name.as_string();
                        self.debug(&format!("Declaring global: {} = {}", name_str, val));
                        self.globals.insert(name_str.to_string(), val);
                        self.stack.push(val); // Push the assigned value back for expression semantics
                    } else {
                        unreachable!("Only strings can become globals - how did you get here?");
                    }
                }
                Op::GetGlobal => {
                    let name = self.stack.pop().unwrap();
                    
                    if name.is_string() {
                        let name_str = name.as_string();
                        match self.globals.get(name_str) {
                            Some(v) => {
                                self.stack.push(*v);
                            }
                            None => {
                                let line = self.call_stack.line_number_at(-1);
                                return Err(VMError::RuntimeError { line, msg: format!("Undefined global {}", name_str) });
                            }
                        }
                    } else {
                        unreachable!("Expected an Identifier: {:?}", name);
                    }
                }
                Op::NEGATE => {
                    let v = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    if v.is_number() {
                        self.stack.push(NanBoxedValue::number(-v.as_number()));
                    } else {
                        return Err(VMError::RuntimeError { 
                            line: self.call_stack.line_number_at(-1), 
                            msg: "Can only negate numbers".to_string() 
                        });
                    }
                }
                Op::ADD => {
                    // Fast-path NaN-boxed arithmetic
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_add(b) {
                        self.stack.push(result);
                    } else {
                        // Handle string concatenation
                        if a.is_string() && b.is_string() {
                            let a_str = a.as_string();
                            let b_str = b.as_string();
                            let result = format!("{}{}", a_str, b_str);
                            self.stack.push(NanBoxedValue::string(result));
                        } else if a.is_string() || b.is_string() {
                            // String + non-string = convert to string and concatenate
                            let a_str = if a.is_string() { a.as_string().to_string() } else { format!("{}", a) };
                            let b_str = if b.is_string() { b.as_string().to_string() } else { format!("{}", b) };
                            let result = format!("{}{}", a_str, b_str);
                            self.stack.push(NanBoxedValue::string(result));
                        } else {
                            return Err(VMError::RuntimeError { 
                                line: self.call_stack.line_number_at(-1), 
                                msg: format!("Cannot add {} and {}", a, b) 
                            });
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
                        return Err(VMError::RuntimeError { 
                            line: self.call_stack.line_number_at(-1), 
                            msg: format!("Cannot subtract {} from {}", b, a) 
                        });
                    }
                }
                Op::MUL => {
                    // Fast-path NaN-boxed arithmetic
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_mul(b) {
                        self.stack.push(result);
                    } else {
                        return Err(VMError::RuntimeError { 
                            line: self.call_stack.line_number_at(-1), 
                            msg: format!("Cannot multiply {} and {}", a, b) 
                        });
                    }
                }
                Op::DIV => {
                    // Fast-path NaN-boxed arithmetic
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_div(b) {
                        self.stack.push(result);
                    } else {
                        return Err(VMError::RuntimeError { 
                            line: self.call_stack.line_number_at(-1), 
                            msg: format!("Cannot divide {} by {}", a, b) 
                        });
                    }
                }
                Op::TRUE => {
                    self.stack.push(NanBoxedValue::boolean(true));
                }
                Op::FALSE => {
                    self.stack.push(NanBoxedValue::boolean(false));
                }
                Op::NOT => {
                    // Everything is truthy in Weave, so we just need to negate
                    // the top value's "truthiness"
                    let val = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let is_truthy = val.is_truthy();
                    self.stack.push(NanBoxedValue::boolean(!is_truthy));
                }
                Op::GREATER => {
                    // Fast-path NaN-boxed comparison
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_greater(b) {
                        self.stack.push(result);
                    } else {
                        // For non-numeric comparisons, return false
                        self.stack.push(NanBoxedValue::boolean(false));
                    }
                }
                Op::LESS => {
                    // Fast-path NaN-boxed comparison
                    let b = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    let a = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    
                    if let Some(result) = a.fast_less(b) {
                        self.stack.push(result);
                    } else {
                        // For non-numeric comparisons, return false
                        self.stack.push(NanBoxedValue::boolean(false));
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
                    let value = *self.stack.last().unwrap_or(&NanBoxedValue::null());
                    println!("{}", green(&format!("{}", value)));
                    log_debug!("VM print instruction", value = format!("{}", value).as_str(), stack_depth = self.stack.len());
                }
                Op::Jump => {
                    let jmp_target = self.call_stack.next_u16();
                    self.call_stack.jump(jmp_target);
                }
                Op::JumpIfFalse => {
                    let jmp_offset = self.call_stack.next_u16();
                    let value = self.stack.pop().unwrap_or(NanBoxedValue::null());
                    if !value.is_truthy() {
                        self.call_stack.jump(jmp_offset);
                    }
                    // Value is already popped - no need to do anything else
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
            
            // Memory usage analysis
            if !memory_samples.is_empty() {
                eprintln!("Memory Usage Analysis:");
                eprintln!("  Total samples: {}", memory_samples.len());
                
                if memory_samples.len() > 1 {
                    let first = &memory_samples[0];
                    let last = &memory_samples[memory_samples.len() - 1];
                    
                    eprintln!("  Growth from iteration {} to {}:", first.0, last.0);
                    eprintln!("    Stack:      {} -> {} (+{})", first.1, last.1, last.1 as i32 - first.1 as i32);
                    eprintln!("    Upvalues:   {} -> {} (+{})", first.2, last.2, last.2 as i32 - first.2 as i32);
                    eprintln!("    Arena:      {} -> {} (+{})", first.3, last.3, last.3 as i32 - first.3 as i32);
                    eprintln!("    Globals:    {} -> {} (+{})", first.4, last.4, last.4 as i32 - first.4 as i32);
                    eprintln!("    Frames:     {} -> {} (+{})", first.5, last.5, last.5 as i32 - first.5 as i32);
                    
                    // Find the component with the highest growth
                    let stack_growth = last.1 as i32 - first.1 as i32;
                    let upvalues_growth = last.2 as i32 - first.2 as i32;
                    let arena_growth = last.3 as i32 - first.3 as i32;
                    let globals_growth = last.4 as i32 - first.4 as i32;
                    let frames_growth = last.5 as i32 - first.5 as i32;
                    
                    let growths = [stack_growth, upvalues_growth, arena_growth, globals_growth, frames_growth];
                    let max_growth = *growths.iter().max().unwrap();
                    
                    if max_growth > 0 {
                        eprintln!("  Largest growth component: {}", 
                            if stack_growth == max_growth { "Stack" }
                            else if upvalues_growth == max_growth { "Upvalues" }
                            else if arena_growth == max_growth { "Arena" }
                            else if globals_growth == max_growth { "Globals" }
                            else { "Frames" }
                        );
                    }
                }
                eprintln!();
            }
        }

        // Return the top value on the stack as the result
        Ok(self.stack.last().copied().unwrap_or(NanBoxedValue::null()))
    }

    fn debug(&self, msg: &str) {
        #[cfg(debug_assertions)]
        log_debug!("VM debug", message = msg, stack_depth = self.stack.len());
    }

    fn runtime_error(&mut self, line: usize, msg: &String) {
        let callstack = self.call_stack.frames.iter().rev();
        for frame in callstack {
            let closure = unsafe { &*frame.closure };
            let func = &closure.func;
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
        let nan_boxed_func = NanBoxedValue::pointer(Box::into_raw(Box::new(func)) as *const (), PointerTag::NativeFn);
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
        // String values are stored as pointers in NanBoxedValue
        let result = res.unwrap();
        assert!(result.is_string());
        assert_eq!(result.as_string(), "hello");
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
