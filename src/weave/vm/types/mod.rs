mod weave_number;
mod nan_boxed_value;

mod weave_string;
mod weave_fn;
mod native_fn;
mod weave_upvalue;
mod upvalues;
pub use weave_fn::{WeaveFn, FnClosure, Upvalue};
pub use weave_upvalue::WeaveUpvalue;
pub use native_fn::{ NativeFn, NativeFnType };
pub use nan_boxed_value::{NanBoxedValue, PointerTag};
pub use weave_string::WeaveString;
pub use weave_number::WeaveNumber;

// Arena type aliases for VM use
use crate::weave::vm::arena::{Arena, Handle};
pub type ClosureArena = Arena<FnClosure>;
pub type ClosureHandle = Handle<FnClosure>;
pub type StringArena = Arena<WeaveString>;
pub type StringHandle = Handle<WeaveString>;

