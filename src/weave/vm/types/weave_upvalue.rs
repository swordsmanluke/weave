use crate::weave::vm::types::upvalues::inner::UpvalAccessor;
use crate::weave::vm::types::upvalues::InnerUpvalue;
use crate::weave::vm::types::WeaveType;
use crate::weave::vm::vm::VM;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Clone)]
pub struct WeaveUpvalue {
    value: Rc<RefCell<InnerUpvalue>>,
}

impl WeaveUpvalue {
    pub fn open(idx: usize) -> WeaveUpvalue {
        let value = InnerUpvalue::open(idx);
        WeaveUpvalue { value: Rc::new(RefCell::new(value)) }
    }

    pub fn value(&self, vm: &VM) -> WeaveType {
        self.value.borrow().get(vm)
    }

    pub fn set(&mut self, v: WeaveType, vm: &mut VM) {
        self.value.borrow_mut().set(v, vm)
    }

    pub fn close(&mut self, vm: &mut VM) {
        let closed_value = self.value.borrow().close(vm);
        *self.value.borrow_mut() = closed_value;
    }

    pub fn is_open(&self) -> bool {
        matches!(*self.value.borrow(), InnerUpvalue::Open(_))
    }

    pub fn get_stack_index(&self) -> usize {
        match &*self.value.borrow() {
            InnerUpvalue::Open(open_upvalue) => open_upvalue.idx,
            InnerUpvalue::Closed(_) => 0, // Closed upvalues don't have stack index
        }
    }

    /// Direct access to the inner value for performance-critical operations
    pub fn get_direct(&self, vm: &VM) -> WeaveType {
        self.value.borrow().get(vm)
    }

    /// Direct set access for performance-critical operations
    pub fn set_direct(&self, v: WeaveType, vm: &mut VM) {
        self.value.borrow_mut().set(v, vm)
    }
}
