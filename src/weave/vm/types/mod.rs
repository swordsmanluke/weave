mod weave_number;
mod weave_type;

pub mod errors;
mod weave_string;
mod weave_fn;
mod native_fn;
mod weave_upvalue;
mod upvalues;

pub use weave_type::WeaveType;
pub use weave_fn::{WeaveFn, FnClosure, Upvalue};
pub use weave_upvalue::WeaveUpvalue;
pub use native_fn::{ NativeFn, NativeFnType };

