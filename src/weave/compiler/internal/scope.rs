use crate::log_debug;
use std::cell::RefCell;
use std::rc::Rc;
use crate::weave::vm::types::Upvalue;

#[derive(PartialEq, Clone)]
enum ScopeType {
    If,
    Fn,
    General,
}

#[derive(Clone)]
pub(crate) struct Local {
    name: Box<String>,
    depth: u8
}

type ScopeStack = Rc<RefCell<Vec<InnerScope>>>;
pub struct Scope {
    pub depth: u8,
    stack: ScopeStack
}

struct InnerScope {
    pub locals: Vec<Local>,
    pub upvalues: Vec<Upvalue>
}

impl Local {
    pub fn new(name: String, depth: u8) -> Local {
        Local { name: name.into(), depth }
    }

    pub fn empty() -> Local { Local { name: "".to_string().into(), depth: 0 } }
}

impl InnerScope {
    pub fn new() -> InnerScope {
        InnerScope {
            locals: vec![Local::empty()],  // First value is reserved for the function object!
            upvalues: vec![]
        }
    }

    pub fn resolve_local(&self, identifier: &str) -> Option<usize> {
        self.locals.iter().enumerate().find_map(|(i, l)|{
            if l.name.as_str() == identifier {
                Some(i)
            } else {
                None
            }
        })
    }
}

impl Clone for Scope {
    fn clone(&self) -> Self {
        Scope {
            depth: self.depth,
            stack: self.stack.clone()
        }
    }
}


impl Scope {
    pub fn new() -> Self {
        Scope {
            depth: 0,
            stack: Rc::new(RefCell::new(vec![InnerScope::new()]))  // Top level, owning scope
        }
    }

    pub fn locals_at(&self, depth: u8) -> u8 {
        self.stack.borrow()[depth as usize].locals.len() as u8
    }
    
    pub fn upvals_at(&self, depth: usize) -> Vec<Upvalue> {
        let scope = &self.stack.borrow()[depth];
        scope.upvalues.clone()
    }
    
    pub fn pop_scope(&mut self) {
        if self.stack.borrow().is_empty() { return; }
        self.stack.borrow_mut().pop();
    }

    pub fn declare_local(&mut self, identifier: String) -> usize {
        let local = Local::new(identifier, self.depth);
        if self.stack.borrow().is_empty() {
            return 0;
        }
        let current_depth = self.depth as usize;
        if current_depth >= self.stack.borrow().len() {
            return 0;
        }
        self.stack.borrow_mut()[current_depth].locals.push(local);
        self.stack.borrow()[current_depth].locals.len() - 1
    }
    
    pub fn resolve_local(&self, identifier: &str) -> isize {
        if self.stack.borrow().is_empty() {
            log_debug!("No local variables in scope", identifier = identifier);
            return -1;
        }
        
        // For now, only search the current scope depth to match the original behavior
        // This maintains stack index consistency until we can properly implement 
        // cross-scope variable resolution with upvalues
        let current_depth = self.depth as usize;
        if current_depth >= self.stack.borrow().len() {
            return -1;
        }
        
        let locals = &self.stack.borrow()[current_depth].locals;
        log_debug!("Searching current scope for local variable", 
            identifier = identifier, 
            current_depth = self.depth,
            local_count = locals.len()
        );
        
        // Search locals in current scope (reverse order to find most recent declaration)
        for (i, l) in locals.iter().enumerate().rev() {
            if l.name.as_str() == identifier {
                log_debug!("Found local variable in current scope", 
                    variable = l.name.as_str(), 
                    index = i,
                    scope_depth = current_depth,
                    variable_depth = l.depth
                );
                return i as isize;
            }
        }
        
        log_debug!("Local variable not found in current scope", identifier = identifier);
        -1
    }

    pub fn resolve_upvalue(&mut self, identifier: &str) -> Option<Upvalue> {
        // Call our recursive function which will search up the call stack for upvals
        self.recursive_resolve_upvalue(identifier, self.depth as usize)

    }

    fn recursive_resolve_upvalue(&mut self, identifier: &str, depth: usize) -> Option<Upvalue> {
        // Top level (e.g. Global) scope has no upvalues
        if depth == 0 { return None; }
        
        let parent_depth = depth - 1;

        // Get our parent's local variables
        let parent_local = self.stack.borrow_mut()[parent_depth].resolve_local(identifier);
        if let Some(i) = parent_local {
            return Some(self.add_upvalue(Upvalue::local(i as u8), depth))
        }

        // Get any upvalues threaded from upstream and create a new "local" upvalue for it
        self.recursive_resolve_upvalue(identifier, parent_depth).map(|parent_resolved_upvalue| {
            // The parent_resolved_upvalue.idx is already the correct index in parent's upvalue array
            // We just need to create a remote upvalue pointing to that index
            let remote_upvalue = Upvalue { 
                idx: parent_resolved_upvalue.idx, // Use the resolved index from parent
                is_local: false, 
                original_idx: parent_resolved_upvalue.original_idx 
            };
            self.add_upvalue(remote_upvalue, depth)
        })
    }


    // Removed find_upvalue_index - no longer needed since we use the resolved index directly

    fn add_upvalue(&mut self, upvalue: Upvalue, depth: usize) -> Upvalue {
        // TODO: Check for too many upvalues
        // if self.upvals.len() >= crate::weave::compiler::compiler::MAX_UPVALS {
        //     self.report_err("Too many closure values in function");
        //     return 0;
        // }

        // Check to see if upvalue already exists
        let upvals = self.upvals_at(depth);
        // Check if upvalue already exists
        for (i, u) in upvals.iter().enumerate() {
            if *u == upvalue {
                // Return an upvalue with the array index, not the source index
                return Upvalue { 
                    idx: i as u8, // Position in the upvalue array
                    is_local: upvalue.is_local, 
                    original_idx: upvalue.original_idx 
                };
            }
        }

        // Otherwise, add it
        let new_index = upvals.len() as u8;
        let new_upvalue = Upvalue { 
            idx: upvalue.idx, // Keep the original idx for storage in the upvalue array
            is_local: upvalue.is_local, 
            original_idx: upvalue.original_idx 
        };
        // Add new upvalue to the scope
        self.stack.borrow_mut()[depth].upvalues.push(new_upvalue.clone());
        
        // Return an upvalue with the array index for the compiler to use
        Upvalue { 
            idx: new_index, // Position in the upvalue array
            is_local: upvalue.is_local, 
            original_idx: upvalue.original_idx 
        }
    }
    
    fn incr(&mut self) -> Self {
        let mut child = self.clone();
        child.depth += 1;
        child.stack.borrow_mut().push(InnerScope::new());
        child
    }

    fn decr(&mut self) {
        self.pop_scope();
    }

    pub fn enter_scope(&mut self) -> Self { self.incr() }
    pub fn exit_scope(&mut self) { self.decr(); }
}