use std::fmt::{Debug, Display};
use std::rc::Rc;
use crate::weave::Chunk;
use crate::weave::vm::types::weave_upvalue::WeaveUpvalue;
use crate::weave::vm::types::WeaveType;

#[derive(Clone)]
pub struct WeaveFn {
    pub chunk: Chunk,
    pub name: String,
    pub arity: usize,
    pub upvalue_count: u8,
    params: Vec<FnParam>,
}

#[derive(Clone, Debug)]
pub struct FnParam {
    name: String,
    default: Option<WeaveType>,
}

#[derive(Clone, Debug)]
pub struct FnArg {
    binding: Option<String>,
    value: WeaveType
}

// This upvalue class is the bridge between compiling
// and running. The actual captured upvalues are in
// the WeaveUpvalue struct
#[derive(Clone)]
pub struct Upvalue {
    pub(crate) idx: u8,
    pub(crate) is_local: bool,
    pub(crate) original_idx: u8, // Store the original local variable index for comparison
}

impl Upvalue {
    fn new(idx: u8, is_local: bool) -> Upvalue {
        Upvalue { idx, is_local, original_idx: idx }
    }
    
    pub fn local(idx: u8) -> Upvalue { Upvalue::new(idx, true) }
    
    pub fn remote(idx: u8) -> Upvalue { Upvalue::new(idx, false) }

    pub fn to_bytes(&self) -> Vec<u8> {
        // For local upvalues, store original_idx (the local variable index)
        // For remote upvalues, store idx (the parent upvalue index)
        let stored_idx = if self.is_local { self.original_idx } else { self.idx };
        vec![if self.is_local { 0x01 } else { 0x00 }, stored_idx]
    }
    
    pub fn from_bytes(code: &[u8], offset: usize) -> Upvalue {
        let is_local = code[offset] == 0x01;
        let idx = code[offset + 1];
        // For consistency, we use idx as the stored value and original_idx for deduplication
        Upvalue { idx, is_local, original_idx: idx }
    }
}

impl PartialEq for Upvalue {
    fn eq(&self, other: &Self) -> bool {
        self.original_idx == other.original_idx && 
            self.is_local == other.is_local
    }
}

impl Display for Upvalue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_local {
            write!(f, "local")
        } else {
            write!(f, "upvalue")
        }
    }
}

#[derive(Clone, Debug)]
pub struct FnClosure {
    pub func: Rc<WeaveFn>,
    pub upvalues: Vec<WeaveUpvalue>
}

impl FnClosure {
    pub fn new(func: Rc<WeaveFn>) -> FnClosure {
        FnClosure { func, upvalues: Vec::new() }
    }
}


impl WeaveFn {
    pub fn new(name: String, params: Vec<FnParam>) -> WeaveFn {
        let chunk = Chunk::new();
        let arity = params.len();
        let upvalue_count = 0;
        WeaveFn { name, chunk, params, upvalue_count, arity }
    }
}

impl Display for WeaveFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<fn {}({})>", self.name, self.arity)
    }
}

impl Display for FnClosure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.func)
    }
}

impl Debug for WeaveFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.is_empty() {
            // top level script
            write!(f, "<script>")
        } else {
            write!(f, "<fn {}({})>", self.name, self.arity)
        }
    }
}