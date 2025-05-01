use crate::weave::Op;

#[derive(PartialEq)]
enum ScopeType {
    If,
    Fn,
    General,
}

pub(crate) struct Local {
    name: Box<String>,
    depth: u8
}

pub(crate) struct Scope {
    locals: Vec<Local>,
    scope_type: Vec<ScopeType>,
    pub depth: u8
}


impl Local {
    pub fn new(name: String, depth: u8) -> Local {
        Local { name: name.into(), depth }
    }

    pub fn empty() -> Local { Local { name: "".to_string().into(), depth: 0 } }
}


impl Scope {
    pub fn new() -> Scope {
        Scope {
            locals: vec![Local::empty()], // First local slot is reserved for the VM
            scope_type: Vec::new(),
            depth: 0
        }
    }
    
    pub fn locals_at(&self, depth: u8) -> u8 {
        // TODO[Optimize]: reverse search and terminate once we're below the current stack depth
        self.locals.iter().filter(|l| l.depth == depth).count() as u8
    }
    
    pub fn pop_scope(&mut self) {
        while !self.locals.is_empty() && self.locals.last().unwrap().depth > self.depth {
            self.locals.pop();
        }
    }

    pub fn declare_local(&mut self, identifier: String) -> usize {
        let local = Local::new(identifier, self.depth);
        self.locals.push(local);
        self.locals.len() - 1
    }
    
    pub fn resolve_local(&self, identifier: &str) -> isize {
        println!("Looking for local var: {}", identifier);
        println!("Locals: {}", self.locals.iter().map(|l| l.name.as_str()).collect::<Vec<&str>>().join(", "));
        if self.locals.is_empty() {
            println!("No local variables");
            return -1;
        }

        for (i, l) in self.locals.iter().enumerate().rev() {
            if l.name.as_str() == identifier && l.depth == self.depth {
                print!("Found local variable {}", l.name);
                // Found the variable, but we can only assign to variables in our _immediate_ scope
                if self.should_shadow() {
                    println!("....but we're shadowing, so create a new var!");
                    return -1;
                }
                println!("... and we're not shadowing, so we can use it!");
                return i as isize;
            }
        }
        
        // No local found 
        -1
    }

    fn incr(&mut self, scope_type: ScopeType) {
        self.scope_type.push(scope_type);
        self.depth += 1;
    }

    fn decr(&mut self) {
        self.scope_type.pop();
        self.depth -= 1;
    }

    pub fn enter_if_scope(&mut self) { self.incr(ScopeType::If); }
    pub fn enter_fn_scope(&mut self) { self.incr(ScopeType::Fn); }
    pub fn enter_gen_scope(&mut self) { self.incr(ScopeType::General); }
    pub fn exit_scope(&mut self) { self.decr(); }
    pub fn should_shadow(&self) -> bool { self.scope_type.last() == Some(&ScopeType::Fn) }
}