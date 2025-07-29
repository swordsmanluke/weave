use std::cell::RefCell;
use std::rc::Rc;
use crate::weave::vm::types::NanBoxedValue;
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
    fn get(&self, vm: &VM) -> NanBoxedValue {
        match self {
            InnerUpvalue::Open(o) => o.get(vm),
            InnerUpvalue::Closed(c) => c.get(vm)
        }
    }

    fn set(&mut self, v: NanBoxedValue, vm: &mut VM) -> () {
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
    fn get(&self, vm: &VM) -> NanBoxedValue;
    fn set(&mut self, v: NanBoxedValue, vm: &mut VM) -> ();
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
    fn get(&self, vm: &VM) -> NanBoxedValue {
        let slot = self.slot(vm);
        // Direct access to the stack
        vm.get_stack_value(slot)
    }

    fn set(&mut self, v: NanBoxedValue, vm: &mut VM) -> () {
        let slot = self.slot(vm);
        vm.set_stack_value(slot, v);
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
    // Single RefCell containing the NanBoxedValue
    pub value: Rc<RefCell<NanBoxedValue>>,
}

impl ClosedUpvalue {
    pub fn new(v: NanBoxedValue) -> Self {
        Self { 
            value: Rc::new(RefCell::new(v))
        }
    }
    
    /// Fast access method that works directly with NanBoxedValue
    pub fn get_fast(&self) -> NanBoxedValue {
        *self.value.borrow()
    }
    
    /// Fast set method that works directly with NanBoxedValue
    pub fn set_fast(&self, v: NanBoxedValue) {
        *self.value.borrow_mut() = v;
    }
}

// Removed conversion functions - now handled by VM methods

impl UpvalAccessor for ClosedUpvalue {
    fn get(&self, _vm: &VM) -> NanBoxedValue {
        *self.value.borrow()
    }

    fn set(&mut self, v: NanBoxedValue, _vm: &mut VM) -> () {
        *self.value.borrow_mut() = v;
    }

    fn close(&self, _vm: &mut VM) -> InnerUpvalue {
        // We shouldn't call close on a closed upval, but if we do, the cost is
        // just an extra clone call. /shrug
        InnerUpvalue::Closed(self.clone())
    }
}

