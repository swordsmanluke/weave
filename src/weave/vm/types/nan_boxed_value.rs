use std::fmt;

/// NaN-boxing implementation for efficient value representation
///
/// Uses IEEE 754 double-precision floating-point representation to encode
/// multiple value types in a single 64-bit value:
///
/// - Numbers: Stored directly as f64 values
/// - Boolean true: 0x7FF8000000000003
/// - Boolean false: 0x7FF8000000000002
/// - Null: 0x7FF8000000000004
/// - Pointers: Use 48-bit payload space with tag bits for type discrimination
#[derive(Clone, Copy, PartialEq)]
pub struct NanBoxedValue {
    bits: u64,
}

// NaN-boxing bit patterns and constants
const QUIET_NAN_MASK: u64 = 0x7FF8000000000000;
// const PAYLOAD_MASK: u64 = 0x0007FFFFFFFFFFFF;
// const SIGN_BIT: u64 = 0x8000000000000000;

// Special value encodings in the quiet NaN space
const NULL_BITS: u64 = QUIET_NAN_MASK | 0x0004;
const TRUE_BITS: u64 = QUIET_NAN_MASK | 0x0003;
const FALSE_BITS: u64 = QUIET_NAN_MASK | 0x0002;

// Pointer tag encodings (using upper bits of 48-bit payload)
const STRING_TAG: u64 = 0x0001000000000000;
const FUNCTION_TAG: u64 = 0x0002000000000000;
const CLOSURE_TAG: u64 = 0x0003000000000000;
const NATIVE_FN_TAG: u64 = 0x0004000000000000;
const UPVALUE_TAG: u64 = 0x0005000000000000;
const CLOSURE_HANDLE_TAG: u64 = 0x0006000000000000;

impl NanBoxedValue {
    /// Creates a new NanBoxedValue from a number
    #[inline]
    pub fn number(value: f64) -> Self {
        Self {
            bits: value.to_bits(),
        }
    }

    /// Creates a new NanBoxedValue from a boolean
    #[inline]
    pub fn boolean(value: bool) -> Self {
        Self {
            bits: if value { TRUE_BITS } else { FALSE_BITS },
        }
    }

    /// Creates a new NanBoxedValue representing null
    #[inline]
    pub fn null() -> Self {
        Self { bits: NULL_BITS }
    }

    /// Creates a new NanBoxedValue from a string (heap-allocated as pointer)
    #[inline]
    pub fn string(value: String) -> Self {
        use crate::weave::vm::types::WeaveString;
        let weave_string = WeaveString::new(value);
        let string_box = Box::new(weave_string);
        let string_ptr = Box::into_raw(string_box) as *const ();
        Self::pointer(string_ptr, PointerTag::String)
    }

    /// Creates a new NanBoxedValue from a closure handle (arena-allocated)
    #[inline]
    pub fn closure_handle(handle: crate::weave::vm::types::ClosureHandle) -> Self {
        let packed = handle.to_u64();
        // Pack the handle data directly into the NaN-boxed value's payload
        Self {
            bits: QUIET_NAN_MASK | CLOSURE_HANDLE_TAG | (packed & 0x0000FFFFFFFFFFFF),
        }
    }

    /// Creates a new NanBoxedValue from a raw pointer with type tag
    #[inline]
    pub fn pointer(ptr: *const (), tag: PointerTag) -> Self {
        debug_assert!(
            (ptr as u64) < (1u64 << 48),
            "Pointer must fit in 48 bits for NaN-boxing"
        );

        let tag_bits = match tag {
            PointerTag::String => STRING_TAG,
            PointerTag::Function => FUNCTION_TAG,
            PointerTag::Closure => CLOSURE_TAG,
            PointerTag::NativeFn => NATIVE_FN_TAG,
            PointerTag::Upvalue => UPVALUE_TAG,
            PointerTag::ClosureHandle => CLOSURE_HANDLE_TAG,
        };

        Self {
            bits: QUIET_NAN_MASK | tag_bits | (ptr as u64),
        }
    }

    /// Fast type checking - returns true if this value represents a number
    #[inline]
    pub fn is_number(self) -> bool {
        // If it's not in the quiet NaN range, it's a number
        if (self.bits & QUIET_NAN_MASK) != QUIET_NAN_MASK {
            true
        } else {
            // In the quiet NaN range - check if it's a special encoded value
            // f64::NAN is 0x7FF8000000000000, which is just the QUIET_NAN_MASK
            // Our special values have additional payload bits set
            self.bits == QUIET_NAN_MASK || // f64::NAN case
            (!self.is_null() && !self.is_boolean() && !self.is_pointer())
        }
    }

    /// Fast type checking - returns true if this value represents null
    #[inline]
    pub fn is_null(self) -> bool {
        self.bits == NULL_BITS
    }

    /// Fast type checking - returns true if this value represents a boolean
    #[inline]
    pub fn is_boolean(self) -> bool {
        self.bits == TRUE_BITS || self.bits == FALSE_BITS
    }

    /// Fast type checking - returns true if this value represents a pointer
    #[inline]
    pub fn is_pointer(self) -> bool {
        (self.bits & QUIET_NAN_MASK) == QUIET_NAN_MASK && !self.is_null() && !self.is_boolean()
    }

    /// Fast type checking - returns true if this value represents a string
    #[inline]
    pub fn is_string(self) -> bool {
        if self.is_pointer() {
            let (_, tag) = self.as_pointer();
            tag == PointerTag::String
        } else {
            false
        }
    }

    /// Fast type checking - returns true if this value represents a closure handle
    #[inline]
    pub fn is_closure_handle(self) -> bool {
        if self.is_pointer() {
            let (_, tag) = self.as_pointer();
            tag == PointerTag::ClosureHandle
        } else {
            false
        }
    }

    /// Extracts the number value (assumes is_number() == true)
    #[inline]
    pub fn as_number(self) -> f64 {
        debug_assert!(self.is_number(), "Value is not a number");
        f64::from_bits(self.bits)
    }

    /// Extracts the boolean value (assumes is_boolean() == true)
    #[inline]
    pub fn as_boolean(self) -> bool {
        debug_assert!(self.is_boolean(), "Value is not a boolean");
        self.bits == TRUE_BITS
    }

    /// Extracts the string value (assumes is_string() == true)
    #[inline]
    pub fn as_string(self) -> &'static str {
        debug_assert!(self.is_string(), "Value is not a string");
        let (ptr, _) = self.as_pointer();
        use crate::weave::vm::types::WeaveString;
        unsafe { &*(ptr as *const WeaveString) }.as_str()
    }

    /// Extracts the closure handle (assumes is_closure_handle() == true)
    #[inline]
    pub fn as_closure_handle(self) -> crate::weave::vm::types::ClosureHandle {
        debug_assert!(self.is_closure_handle(), "Value is not a closure handle");
        // Extract the packed handle data from the payload
        let handle_data = self.bits & 0x0000FFFFFFFFFFFF;
        crate::weave::vm::types::ClosureHandle::from_u64(handle_data)
    }

    /// Fast type checking - returns true if this value represents an upvalue
    #[inline]
    pub fn is_upvalue(self) -> bool {
        (self.bits & QUIET_NAN_MASK) == QUIET_NAN_MASK
            && (self.bits & 0x0007000000000000) == UPVALUE_TAG
    }

    /// Extracts the upvalue pointer (assumes is_upvalue() == true)
    #[inline]
    pub fn as_upvalue(self) -> *const crate::weave::vm::types::WeaveUpvalue {
        debug_assert!(self.is_upvalue(), "Value is not an upvalue");
        (self.bits & 0x0000FFFFFFFFFFFF) as *const crate::weave::vm::types::WeaveUpvalue
    }

    /// Extracts the pointer value and tag (assumes is_pointer() == true)
    #[inline]
    pub fn as_pointer(self) -> (*const (), PointerTag) {
        debug_assert!(self.is_pointer(), "Value is not a pointer");

        let ptr = (self.bits & 0x0000FFFFFFFFFFFF) as *const ();
        let tag_bits = self.bits & 0x0007000000000000;

        let tag = match tag_bits {
            STRING_TAG => PointerTag::String,
            FUNCTION_TAG => PointerTag::Function,
            CLOSURE_TAG => PointerTag::Closure,
            NATIVE_FN_TAG => PointerTag::NativeFn,
            UPVALUE_TAG => PointerTag::Upvalue,
            CLOSURE_HANDLE_TAG => PointerTag::ClosureHandle,
            _ => panic!("Invalid pointer tag: {:#x}", tag_bits),
        };

        (ptr, tag)
    }

    /// Get the raw bits for debugging
    #[inline]
    pub fn bits(self) -> u64 {
        self.bits
    }

    // Fast-path arithmetic operations for NaN-boxed values
    // These operate directly on the bit representation for maximum performance

    /// Fast addition of two NaN-boxed values
    /// Returns None if the operation cannot be performed (e.g., non-numeric operands)
    #[inline]
    pub fn fast_add(self, other: NanBoxedValue) -> Option<NanBoxedValue> {
        if self.is_number() && other.is_number() {
            let result = self.as_number() + other.as_number();
            Some(NanBoxedValue::number(result))
        } else {
            None
        }
    }

    /// Fast subtraction of two NaN-boxed values
    #[inline]
    pub fn fast_sub(self, other: NanBoxedValue) -> Option<NanBoxedValue> {
        if self.is_number() && other.is_number() {
            let result = self.as_number() - other.as_number();
            Some(NanBoxedValue::number(result))
        } else {
            None
        }
    }

    /// Fast multiplication of two NaN-boxed values
    #[inline]
    pub fn fast_mul(self, other: NanBoxedValue) -> Option<NanBoxedValue> {
        if self.is_number() && other.is_number() {
            let result = self.as_number() * other.as_number();
            Some(NanBoxedValue::number(result))
        } else {
            None
        }
    }

    /// Fast division of two NaN-boxed values
    #[inline]
    pub fn fast_div(self, other: NanBoxedValue) -> Option<NanBoxedValue> {
        if self.is_number() && other.is_number() {
            let result = self.as_number() / other.as_number();
            Some(NanBoxedValue::number(result))
        } else {
            None
        }
    }

    /// Fast comparison - greater than
    #[inline]
    pub fn fast_greater(self, other: NanBoxedValue) -> Option<NanBoxedValue> {
        if self.is_number() && other.is_number() {
            let result = self.as_number() > other.as_number();
            Some(NanBoxedValue::boolean(result))
        } else {
            None
        }
    }

    /// Fast comparison - less than
    #[inline]
    pub fn fast_less(self, other: NanBoxedValue) -> Option<NanBoxedValue> {
        if self.is_number() && other.is_number() {
            let result = self.as_number() < other.as_number();
            Some(NanBoxedValue::boolean(result))
        } else {
            None
        }
    }

    /// Fast equality comparison
    #[inline]
    pub fn fast_equal(self, other: NanBoxedValue) -> NanBoxedValue {
        // Handle numeric equality first to respect IEEE 754 NaN != NaN
        if self.is_number() && other.is_number() {
            let a = self.as_number();
            let b = other.as_number();
            // Use IEEE 754 equality (handles NaN correctly)
            return NanBoxedValue::boolean(a == b);
        }

        // Fast path for exact bit equality (works for booleans, null, pointers)
        if self.bits == other.bits {
            return NanBoxedValue::boolean(true);
        } else {
            // Different types or bit patterns are not equal
            return NanBoxedValue::boolean(false);
        }
    }

    /// Check if this value is "truthy" in Weave semantics
    #[inline]
    pub fn is_truthy(self) -> bool {
        if self.is_null() {
            false
        } else if self.is_boolean() {
            self.as_boolean()
        } else if self.is_number() {
            // Numbers are truthy except for 0.0 and NaN
            let n = self.as_number();
            !n.is_nan() && n != 0.0
        } else {
            // Pointers are always truthy
            true
        }
    }
}

/// Type tags for pointer values stored in NaN-boxed values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerTag {
    String,
    Function,
    Closure,
    NativeFn,
    Upvalue,
    ClosureHandle,
}

impl fmt::Display for NanBoxedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_number() {
            write!(f, "{}", self.as_number())
        } else if self.is_boolean() {
            write!(f, "{}", self.as_boolean())
        } else if self.is_null() {
            write!(f, "null")
        } else if self.is_string() {
            write!(f, "{}", self.as_string())
        } else if self.is_closure_handle() {
            let handle = self.as_closure_handle();
            let index = handle.clone().index();
            let generation = handle.generation();
            write!(f, "<closure handle {}:{}>", index, generation)
        } else if self.is_pointer() {
            // For Display, we can't safely dereference non-string pointers without more context
            // So we'll just show pointer info
            let (_, tag) = self.as_pointer();
            write!(f, "<{:?} pointer>", tag)
        } else {
            write!(f, "<invalid>")
        }
    }
}

impl fmt::Debug for NanBoxedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_number() {
            write!(f, "{}", self.as_number())
        } else if self.is_boolean() {
            write!(f, "{}", self.as_boolean())
        } else if self.is_null() {
            write!(f, "<NULL>")
        } else if self.is_pointer() {
            let (ptr, tag) = self.as_pointer();
            if tag == PointerTag::String {
                write!(f, "<str \"{}\">", &self.as_string())
            } else if tag == PointerTag::Function {
                write!(f, "<fn {:?}>", ptr)
            } else if tag == PointerTag::Closure {
                write!(f, "<cl {:?}>", ptr)
            } else if tag == PointerTag::NativeFn {
                write!(f, "<fn {:?}>", ptr)
            } else if tag == PointerTag::Upvalue {
                write!(f, "<upval {:?}>", ptr)
            } else if tag == PointerTag::ClosureHandle {
                write!(f, "<clh {:?}>", ptr)
            } else {
                write!(f, "{:?}, {:p})", tag, ptr)
            }
        } else {
            write!(f, "Invalid({:#x})", self.bits)
        }
    }
}

impl From<f64> for NanBoxedValue {
    #[inline]
    fn from(value: f64) -> Self {
        Self::number(value)
    }
}

impl From<bool> for NanBoxedValue {
    #[inline]
    fn from(value: bool) -> Self {
        Self::boolean(value)
    }
}

impl From<()> for NanBoxedValue {
    #[inline]
    fn from(_: ()) -> Self {
        Self::null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_encoding() {
        let val = NanBoxedValue::number(42.0);
        assert!(val.is_number());
        assert!(!val.is_boolean());
        assert!(!val.is_null());
        assert!(!val.is_pointer());
        assert_eq!(val.as_number(), 42.0);
    }

    #[test]
    fn test_special_float_values() {
        // Test infinity
        let inf = NanBoxedValue::number(f64::INFINITY);
        assert!(inf.is_number());
        assert!(inf.as_number().is_infinite());

        // Test negative zero
        let neg_zero = NanBoxedValue::number(-0.0);
        assert!(neg_zero.is_number());
        assert_eq!(neg_zero.as_number().to_bits(), (-0.0f64).to_bits());

        // Test signaling NaN (should be preserved as number)
        // Note: f64::NAN produces a quiet NaN that conflicts with our encoding scheme
        // So we use a signaling NaN instead which has different bit pattern
        let signaling_nan_bits = 0x7FF0000000000001u64; // Signaling NaN
        let signaling_nan = f64::from_bits(signaling_nan_bits);
        let nan = NanBoxedValue::number(signaling_nan);
        assert!(nan.is_number());
        assert!(nan.as_number().is_nan());
    }

    #[test]
    fn test_boolean_encoding() {
        let true_val = NanBoxedValue::boolean(true);
        assert!(true_val.is_boolean());
        assert!(!true_val.is_number());
        assert!(!true_val.is_null());
        assert!(!true_val.is_pointer());
        assert_eq!(true_val.as_boolean(), true);
        assert_eq!(true_val.bits(), TRUE_BITS);

        let false_val = NanBoxedValue::boolean(false);
        assert!(false_val.is_boolean());
        assert_eq!(false_val.as_boolean(), false);
        assert_eq!(false_val.bits(), FALSE_BITS);
    }

    #[test]
    fn test_null_encoding() {
        let null_val = NanBoxedValue::null();
        assert!(null_val.is_null());
        assert!(!null_val.is_number());
        assert!(!null_val.is_boolean());
        assert!(!null_val.is_pointer());
        assert_eq!(null_val.bits(), NULL_BITS);
    }

    #[test]
    fn test_pointer_encoding() {
        let test_string = "hello";
        let ptr = test_string.as_ptr() as *const ();

        let val = NanBoxedValue::pointer(ptr, PointerTag::String);
        assert!(val.is_pointer());
        assert!(!val.is_number());
        assert!(!val.is_boolean());
        assert!(!val.is_null());

        let (extracted_ptr, tag) = val.as_pointer();
        assert_eq!(extracted_ptr, ptr);
        assert_eq!(tag, PointerTag::String);
    }

    #[test]
    fn test_from_traits() {
        let num_val: NanBoxedValue = 3.14.into();
        assert!(num_val.is_number());
        assert_eq!(num_val.as_number(), 3.14);

        let bool_val: NanBoxedValue = true.into();
        assert!(bool_val.is_boolean());
        assert_eq!(bool_val.as_boolean(), true);

        let null_val: NanBoxedValue = ().into();
        assert!(null_val.is_null());
    }

    #[test]
    fn test_bit_patterns() {
        // Verify specific bit patterns match specification
        assert_eq!(NanBoxedValue::null().bits(), 0x7FF8000000000004);
        assert_eq!(NanBoxedValue::boolean(true).bits(), 0x7FF8000000000003);
        assert_eq!(NanBoxedValue::boolean(false).bits(), 0x7FF8000000000002);

        // Test that numbers don't interfere with special values
        let zero = NanBoxedValue::number(0.0);
        assert!(zero.is_number());
        assert!(!zero.is_null());

        let one = NanBoxedValue::number(1.0);
        assert!(one.is_number());
        assert!(!one.is_boolean());
    }

    #[test]
    fn test_fast_arithmetic() {
        let a = NanBoxedValue::number(5.0);
        let b = NanBoxedValue::number(3.0);

        // Test fast addition
        let sum = a.fast_add(b).unwrap();
        assert!(sum.is_number());
        assert_eq!(sum.as_number(), 8.0);

        // Test fast subtraction
        let diff = a.fast_sub(b).unwrap();
        assert!(diff.is_number());
        assert_eq!(diff.as_number(), 2.0);

        // Test fast multiplication
        let prod = a.fast_mul(b).unwrap();
        assert!(prod.is_number());
        assert_eq!(prod.as_number(), 15.0);

        // Test fast division
        let quot = a.fast_div(b).unwrap();
        assert!(quot.is_number());
        assert_eq!(quot.as_number(), 5.0 / 3.0);

        // Test with non-numeric operands (should return None)
        let bool_val = NanBoxedValue::boolean(true);
        assert!(a.fast_add(bool_val).is_none());
        assert!(bool_val.fast_mul(a).is_none());
    }

    #[test]
    fn test_fast_comparisons() {
        let a = NanBoxedValue::number(5.0);
        let b = NanBoxedValue::number(3.0);
        let c = NanBoxedValue::number(5.0);

        // Test greater than
        let gt = a.fast_greater(b).unwrap();
        assert!(gt.is_boolean());
        assert_eq!(gt.as_boolean(), true);

        let gt2 = b.fast_greater(a).unwrap();
        assert_eq!(gt2.as_boolean(), false);

        // Test less than
        let lt = b.fast_less(a).unwrap();
        assert_eq!(lt.as_boolean(), true);

        // Test equality
        let eq = a.fast_equal(c);
        assert_eq!(eq.as_boolean(), true);

        let eq2 = a.fast_equal(b);
        assert_eq!(eq2.as_boolean(), false);

        // Test with non-numeric operands
        let bool_val = NanBoxedValue::boolean(true);
        assert!(a.fast_greater(bool_val).is_none());
    }

    #[test]
    fn test_fast_equal_edge_cases() {
        // Test identical bit patterns
        let a = NanBoxedValue::boolean(true);
        let b = NanBoxedValue::boolean(true);
        assert_eq!(a.fast_equal(b).as_boolean(), true);

        // Test different types
        let num = NanBoxedValue::number(1.0);
        let bool_val = NanBoxedValue::boolean(true);
        assert_eq!(num.fast_equal(bool_val).as_boolean(), false);

        // Test NaN handling
        let nan1 = NanBoxedValue::number(f64::NAN);
        let nan2 = NanBoxedValue::number(f64::NAN);
        assert_eq!(nan1.fast_equal(nan2).as_boolean(), false); // NaN != NaN
    }

    #[test]
    fn test_is_truthy() {
        // Numbers
        assert_eq!(NanBoxedValue::number(1.0).is_truthy(), true);
        assert_eq!(NanBoxedValue::number(-1.0).is_truthy(), true);
        assert_eq!(NanBoxedValue::number(0.0).is_truthy(), false);
        assert_eq!(NanBoxedValue::number(f64::NAN).is_truthy(), false);

        // Booleans
        assert_eq!(NanBoxedValue::boolean(true).is_truthy(), true);
        assert_eq!(NanBoxedValue::boolean(false).is_truthy(), false);

        // Null
        assert_eq!(NanBoxedValue::null().is_truthy(), false);

        // Pointers (should be truthy)
        let test_string = "hello";
        let ptr = test_string.as_ptr() as *const ();
        let val = NanBoxedValue::pointer(ptr, PointerTag::String);
        assert_eq!(val.is_truthy(), true);
    }

    #[test]
    fn test_round_trip_conversion() {
        // Numbers
        let original_numbers = [0.0, 1.0, -1.0, 3.14159, f64::INFINITY, -f64::INFINITY];
        for &num in &original_numbers {
            let boxed = NanBoxedValue::number(num);
            assert_eq!(boxed.as_number(), num);
        }

        // Booleans
        for &bool_val in &[true, false] {
            let boxed = NanBoxedValue::boolean(bool_val);
            assert_eq!(boxed.as_boolean(), bool_val);
        }

        // Null
        let null_boxed = NanBoxedValue::null();
        assert!(null_boxed.is_null());
    }
}

impl NanBoxedValue {
    /// Manually deallocate heap-allocated pointer if this value owns it
    /// This should be called when a value is being discarded and won't be used again
    pub unsafe fn deallocate(&self) {
        if self.is_pointer() {
            let (ptr, tag) = self.as_pointer();
            if !ptr.is_null() {
                match tag {
                    PointerTag::String => {
                        unsafe {
                            let _ = Box::from_raw(ptr as *mut crate::weave::vm::types::WeaveString);
                        }
                    }
                    PointerTag::Closure => {
                        unsafe {
                            let _ = Box::from_raw(ptr as *mut crate::weave::vm::types::FnClosure);
                        }
                    }
                    PointerTag::NativeFn => {
                        unsafe {
                            let _ = Box::from_raw(ptr as *mut std::rc::Rc<crate::weave::vm::types::NativeFn>);
                        }
                    }
                    PointerTag::Upvalue => {
                        unsafe {
                            let _ = Box::from_raw(ptr as *mut crate::weave::vm::types::WeaveUpvalue);
                        }
                    }
                    PointerTag::ClosureHandle => {
                        // Closure handles don't need manual deallocation - they're managed by the arena
                        // This is the whole point of using arena allocation!
                    }
                    _ => {
                        // Unknown pointer types - don't deallocate to avoid crashes
                    }
                }
            }
        }
    }
}
