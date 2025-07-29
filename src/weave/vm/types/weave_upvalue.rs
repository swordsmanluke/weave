use crate::weave::vm::types::upvalues::inner::UpvalAccessor;
use crate::weave::vm::types::upvalues::InnerUpvalue;
use crate::weave::vm::types::NanBoxedValue;
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

    pub fn value(&self, vm: &VM) -> NanBoxedValue {
        self.value.borrow().get(vm)
    }

    pub fn set(&mut self, v: NanBoxedValue, vm: &mut VM) {
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
    /// Optimized with fast path for closed upvalues (most common in hot loops)
    pub fn get_direct(&self, vm: &VM) -> NanBoxedValue {
        let borrowed = self.value.borrow();
        match &*borrowed {
            InnerUpvalue::Closed(closed) => {
                // Fast path: direct access to closed upvalue without trait dispatch
                *closed.value.borrow()
            }
            InnerUpvalue::Open(open) => {
                // Slower path: open upvalue access through stack
                let slot = open.idx;
                vm.get_stack_value(slot)
            }
        }
    }

    /// Ultra-fast access that returns NanBoxedValue directly
    /// Eliminates conversion overhead in closure hot loops
    pub fn get_fast(&self, vm: &VM) -> NanBoxedValue {
        let borrowed = self.value.borrow();
        match &*borrowed {
            InnerUpvalue::Closed(closed) => {
                // Ultra-fast path: direct NanBoxedValue access with Copy semantics
                closed.get_fast()
            }
            InnerUpvalue::Open(open) => {
                // Open upvalue: get directly from stack
                let slot = open.idx;
                vm.get_stack_value(slot)
            }
        }
    }

    /// Direct set access for performance-critical operations  
    /// Optimized with fast path for closed upvalues (most common in hot loops)
    pub fn set_direct(&self, v: NanBoxedValue, vm: &mut VM) {
        let mut borrowed = self.value.borrow_mut();
        match &mut *borrowed {
            InnerUpvalue::Closed(closed) => {
                // Fast path: direct access to closed upvalue without trait dispatch
                *closed.value.borrow_mut() = v;
            }
            InnerUpvalue::Open(open) => {
                // Slower path: open upvalue access through stack
                vm.set_stack_value(open.idx, v);
            }
        }
    }

    /// Ultra-fast set that works directly with NanBoxedValue
    /// Eliminates conversion overhead in closure hot loops
    pub fn set_fast(&self, v: NanBoxedValue, vm: &mut VM) {
        let mut borrowed = self.value.borrow_mut();
        match &mut *borrowed {
            InnerUpvalue::Closed(closed) => {
                // Ultra-fast path: direct NanBoxedValue set with Copy semantics
                closed.set_fast(v);
            }
            InnerUpvalue::Open(open) => {
                // Open upvalue: set directly on stack
                vm.set_stack_value(open.idx, v);
            }
        }
    }
}
