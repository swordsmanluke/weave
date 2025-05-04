use std::fmt::{Debug, Display};
use crate::weave::Chunk;
use crate::weave::vm::types::WeaveType;

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

#[derive(Clone)]
pub struct WeaveFn {
    pub chunk: Chunk,
    pub name: String,
    pub arity: usize,
    params: Vec<FnParam>,   // TODO: change fn/arg parsing
}

impl WeaveFn {
    pub fn new(name: String, params: Vec<FnParam>) -> WeaveFn {
        let chunk = Chunk::new();
        let arity = params.len();
        WeaveFn { name, chunk, params, arity }
    }
}

impl Display for WeaveFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<fn {}({})>", self.name, self.arity)
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