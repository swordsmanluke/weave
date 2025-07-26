use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use crate::weave::vm::types::WeaveType;
use crate::weave::vm::vm::VM;

#[derive(Debug, Clone)]
pub enum InnerUpvalue {
    Open(OpenUpvalue),
    Closed(ClosedUpvalue)
}

impl InnerUpvalue {
    pub fn open(slot: usize) -> Self {
        InnerUpvalue::Open(OpenUpvalue::new(slot))
    }

    pub fn close(&mut self, vm: &mut VM) -> InnerUpvalue {
        match self {
            InnerUpvalue::Open(o) => o.close(vm),
            InnerUpvalue::Closed(c) => InnerUpvalue::Closed(c.clone())
        }
    }
}

impl UpvalAccessor for InnerUpvalue {
    fn get(&self, vm: &VM) -> WeaveType {
        match self {
            InnerUpvalue::Open(o) => o.get(vm),
            InnerUpvalue::Closed(c) => c.get(vm)
        }
    }

    fn set(&mut self, v: WeaveType, vm: &mut VM) -> () {
        match self {
            InnerUpvalue::Open(o) => o.set(v, vm),
            InnerUpvalue::Closed(c) => c.set(v, vm)
        }
    }

    fn close(&self, vm: &mut VM) -> InnerUpvalue {
        match self {
            InnerUpvalue::Open(o) => o.close(vm),
            InnerUpvalue::Closed(c) => InnerUpvalue::Closed(c.clone())
        }
    }
}


// Trait for accessing upvals on either the stack or the heap
pub trait UpvalAccessor {
    fn get(&self, vm: &VM) -> WeaveType;
    fn set(&mut self, v: WeaveType, vm: &mut VM) -> ();
    fn close(&self, vm: &mut VM) -> InnerUpvalue;
}

//*****************
/* Open Upvalue */
//*****************

#[derive(Debug, Clone)]
pub struct OpenUpvalue {
    pub idx: usize
}

impl OpenUpvalue {
    pub fn new(idx: usize) -> Self {
        Self { idx }
    }

    fn slot(&self, _vm: &VM) -> usize {
        // self.idx is now the absolute stack position
        self.idx
    }
}

impl  UpvalAccessor for OpenUpvalue {
    fn get(&self, vm: &VM) -> WeaveType {
        let slot = self.slot(vm);
        let value = vm.get_stack_var(slot).unwrap().clone();
        value
    }

    fn set(&mut self, v: WeaveType, vm: &mut VM) -> () {
        vm.set_stack_var(self.slot(vm), v)
    }

    fn close(&self, vm: &mut VM) -> InnerUpvalue {
        let v = self.get(vm);
        InnerUpvalue::Closed(ClosedUpvalue::new(v))
    }
}


//*****************
/* Closed Upvalue */
//*****************

#[derive(Debug, Clone)]
pub struct ClosedUpvalue {
    value: Rc<RefCell<WeaveType>>
}

impl ClosedUpvalue {
    pub fn new(v: WeaveType) -> Self {
        // Move to the heap and keep a ptr
        let value = Rc::new(RefCell::new(v));
        Self { value }
    }
}

impl UpvalAccessor for ClosedUpvalue {
    fn get(&self, _vm: &VM) -> WeaveType {
        self.value.as_ref().borrow().deref().clone()
    }

    fn set(&mut self, v: WeaveType, vm: &mut VM) -> () {
        self.value.replace(v.clone());
    }

    fn close(&self, _vm: &mut VM) -> InnerUpvalue {
        // We shouldn't call close on a closed upval, but if we do, the cost is
        // just an extra clone call. /shrug
        InnerUpvalue::Closed(self.clone())
    }
}

