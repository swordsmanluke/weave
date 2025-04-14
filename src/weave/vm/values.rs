use crate::weave::vm::types::WeaveType;

pub(crate) struct ValueArray {
    pub values: Vec<WeaveType>
}

impl ValueArray {
    pub fn new() -> ValueArray {
        ValueArray { values: vec![] }
    }

    pub fn push(&mut self, value: WeaveType) {
        self.values.push(value);
    }
}
