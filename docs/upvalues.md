# Upvalues and Closures in Weave

This document explains how closures and upvalues are implemented in the Weave programming language interpreter, with specific attention to the Rust-friendly adaptations made from the traditional C implementation in Crafting Interpreters.

## Overview

Closures in Weave allow functions to capture and access variables from their enclosing scopes, even after those scopes have been exited. This is achieved through a mechanism called "upvalues" which provides a level of indirection for accessing captured variables.

## Key Components

### 1. Upvalue (Compiler Bridge)

Located in `src/weave/vm/types/weave_fn.rs`, this struct bridges the compiler and runtime:

```rust
pub struct Upvalue {
    pub(crate) idx: u8,        // Index in the function's upvalue array
    pub(crate) is_local: bool, // Whether capturing local var or parent upvalue
}
```

**Key Methods:**
- `Upvalue::local(idx)` - Creates upvalue for capturing local variable
- `Upvalue::remote(idx)` - Creates upvalue for capturing parent's upvalue
- `to_bytes()` - Serializes upvalue metadata for bytecode
- `from_bytes()` - Deserializes upvalue metadata during VM execution

### 2. WeaveUpvalue (Runtime Container)

Located in `src/weave/vm/types/weave_upvalue.rs`, this is the runtime container:

```rust
pub struct WeaveUpvalue {
    value: InnerUpvalue,
}
```

**Key Methods:**
- `WeaveUpvalue::open(idx)` - Creates open upvalue pointing to stack slot
- `value(&self, vm)` - Gets the current value of the upvalue
- `set(&mut self, value, vm)` - Sets the upvalue to a new value

### 3. InnerUpvalue (State Management)

Located in `src/weave/vm/types/upvalues/inner.rs`, this enum manages the two states:

```rust
pub enum InnerUpvalue {
    Open(OpenUpvalue),    // Points to stack slot
    Closed(ClosedUpvalue) // Contains heap-allocated value
}
```

#### OpenUpvalue
Points directly to a stack slot. Used when the captured variable is still on the stack.

```rust
struct OpenUpvalue {
    idx: usize  // Absolute stack position
}
```

#### ClosedUpvalue
Contains a heap-allocated value using `Rc<RefCell<WeaveType>>`. Used when the captured variable has moved off the stack.

```rust
struct ClosedUpvalue {
    value: Rc<RefCell<WeaveType>>
}
```

## Execution Flow

### 1. Compilation Phase

During compilation, when a function is encountered:

1. **Variable Resolution**: The compiler tracks which variables are captured from parent scopes
2. **Upvalue Creation**: For each captured variable, an `Upvalue` is created with:
   - `idx`: The position of the variable in the parent's locals/upvalues
   - `is_local`: Whether capturing a local variable or another upvalue
3. **Bytecode Emission**: The `OP_CLOSURE` instruction is emitted with:
   - Function constant index
   - Upvalue metadata bytes for each captured variable

### 2. Runtime Execution

When `OP_CLOSURE` is executed:

1. **Read Function**: The function constant is read from the constants table
2. **Process Upvalues**: For each upvalue in the function:
   - Read the `is_local` and `idx` bytes
   - If `is_local`: Create `WeaveUpvalue::open()` pointing to parent's stack slot
   - If not `is_local`: Clone the upvalue from parent's upvalue array
3. **Create Closure**: The closure is created with all upvalues initialized

### 3. Variable Access

**GetUpvalue**: Reads the current value from an upvalue
```rust
let upvalue = &current_frame.closure.upvalues[slot];
let value = upvalue.value(self);  // Returns the actual value
```

**SetUpvalue**: Updates the value in an upvalue
```rust
let mut upvalue = current_frame.closure.upvalues[slot].clone();
upvalue.set(new_value, self);  // Updates the underlying storage
```

## Stack Frame Layout

Weave uses a specific stack frame layout where slot 0 contains the function object:

```
Stack Frame:
[0] - Function object (reserved)
[1] - First parameter or local variable
[2] - Second parameter or local variable
...
[N] - Last local variable
```

This differs from the Crafting Interpreters book where slot 0 is reserved for future method support.

## Rust-Specific Adaptations

### 1. Index-Based References
Instead of raw pointers, Weave uses stack indices for safety:
```rust
// Safe: Uses index to reference stack slot
struct OpenUpvalue { idx: usize }

// Unsafe C equivalent: ObjUpvalue { Value* location; }
```

### 2. Interior Mutability
Closed upvalues use `Rc<RefCell<>>` for shared mutable access:
```rust
struct ClosedUpvalue {
    value: Rc<RefCell<WeaveType>>  // Safe shared mutability
}
```

### 3. Enum-Based State
The open/closed state is managed through Rust enums rather than C-style unions:
```rust
pub enum InnerUpvalue {
    Open(OpenUpvalue),
    Closed(ClosedUpvalue)
}
```

### 4. Clone-Based Borrowing
To avoid complex borrowing issues, upvalues are cloned when needed:
```rust
let upvalue = current_frame.closure.upvalues[slot].clone();
upvalue.set(value, self);  // No borrowing conflicts
```

## Example: Counter Closure

```weave
fn make_counter() {
    count = 0
    
    fn counter() {
        count = count + 1
        count
    }
    
    counter
}

c = make_counter()
c()  // Returns 1
c()  // Returns 2
```

**Execution Trace:**
1. `make_counter()` creates local variable `count = 0` at stack slot 1
2. `counter` function is compiled with one upvalue capturing `count`
3. During `OP_CLOSURE`, `WeaveUpvalue::open(1)` is created pointing to stack slot 1
4. `c()` calls access and modify the shared `count` variable through the upvalue
5. Multiple calls to `c()` maintain state because they share the same upvalue

## Testing

The implementation includes comprehensive tests covering:

- **Basic closures**: Simple variable capture and access
- **Multiple calls**: State persistence across function calls
- **Shared variables**: Multiple closures accessing the same variable
- **Nested closures**: Multiple levels of variable capture
- **Complex scenarios**: Real-world closure usage patterns

## Future Improvements

### 1. VM-Level Upvalue Tracking
Currently, upvalue deduplication is handled at compile-time. A future improvement would add VM-level tracking to ensure multiple closures capturing the same variable share the same upvalue object.

### 2. Proper Upvalue Closing
The `OP_CLOSE_UPVALUES` opcode exists but needs full implementation to transition open upvalues to closed when variables go out of scope.

### 3. Performance Optimizations
Consider using more efficient data structures for upvalue management, possibly replacing some `Vec` operations with more specialized collections.

## References

- [Crafting Interpreters - Closures](https://craftinginterpreters.com/closures.html)
- Weave source code in `src/weave/vm/types/` and `src/weave/compiler/`
