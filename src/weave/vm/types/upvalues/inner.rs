use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use crate::weave::vm::types::{WeaveType, NanBoxedValue};
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
pub struct UpvalueData {
    pub weave_value: WeaveType,
    pub fast_value: NanBoxedValue,
    pub is_fast_dirty: bool, // Track if fast_value needs sync
}

#[derive(Debug, Clone)]
pub struct ClosedUpvalue {
    // Single RefCell containing both values for atomic operations
    pub data: Rc<RefCell<UpvalueData>>,
}

impl ClosedUpvalue {
    pub fn new(v: WeaveType) -> Self {
        let data = UpvalueData {
            weave_value: v,
            fast_value: NanBoxedValue::null(),
            is_fast_dirty: true, // Needs initial sync
        };
        Self { 
            data: Rc::new(RefCell::new(data))
        }
    }
    
    pub fn new_with_fast(v: WeaveType, fast_v: NanBoxedValue) -> Self {
        let data = UpvalueData {
            weave_value: v,
            fast_value: fast_v,
            is_fast_dirty: false, // Already synced
        };
        Self { 
            data: Rc::new(RefCell::new(data))
        }
    }
    
    /// Fast access method that works directly with NanBoxedValue
    /// Eliminates conversion overhead in hot loops - SINGLE RefCell borrow!
    pub fn get_fast(&self) -> NanBoxedValue {
        let mut data = self.data.borrow_mut();
        if data.is_fast_dirty {
            // Lazy initialization - sync fast_value from weave_value
            data.fast_value = match &data.weave_value {
                WeaveType::Number(n) => NanBoxedValue::number(n.to_f64()),
                WeaveType::Boolean(b) => NanBoxedValue::boolean(*b),
                WeaveType::None => NanBoxedValue::null(),
                _ => NanBoxedValue::null(), // Default for other types
            };
            data.is_fast_dirty = false;
        }
        data.fast_value // Copy - no clone needed!
    }
    
    /// Fast set method that works directly with NanBoxedValue
    /// Eliminates conversion overhead in hot loops - SINGLE RefCell borrow!
    pub fn set_fast(&self, v: NanBoxedValue) {
        let mut data = self.data.borrow_mut();
        data.fast_value = v;
        // Keep WeaveType in sync atomically
        data.weave_value = if v.is_null() {
            WeaveType::None
        } else if v.is_boolean() {
            WeaveType::Boolean(v.as_boolean())
        } else if v.is_number() {
            WeaveType::Number(crate::weave::vm::types::WeaveNumber::Float(v.as_number()))
        } else {
            WeaveType::None // Default for other types
        };
        data.is_fast_dirty = false; // Both values are in sync
    }
}

// Removed conversion functions - now handled by VM methods

impl UpvalAccessor for ClosedUpvalue {
    fn get(&self, _vm: &VM) -> WeaveType {
        self.data.borrow().weave_value.clone()
    }

    fn set(&mut self, v: WeaveType, _vm: &mut VM) -> () {
        let mut data = self.data.borrow_mut();
        data.weave_value = v;
        data.is_fast_dirty = true; // Fast value needs sync
    }

    fn close(&self, _vm: &mut VM) -> InnerUpvalue {
        // We shouldn't call close on a closed upval, but if we do, the cost is
        // just an extra clone call. /shrug
        InnerUpvalue::Closed(self.clone())
    }
}

