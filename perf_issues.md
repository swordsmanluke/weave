# Performance Analysis: Function Call Bottlenecks

## Current Performance
- **Current**: ~45 seconds for 5,000 function calls
- **Target**: <1 second (requires ~45x improvement)  
- **Function calls**: 5,001 calls at 201ns average = major bottleneck

## Profiling Results
```
GetLocal           15002 calls,    1105215 ns total,     73 ns avg
Call                5001 calls,    1006773 ns total,    201 ns avg  <- BOTTLENECK
CONSTANT           15005 calls,     962164 ns total,     64 ns avg
ADD                10000 calls,     870495 ns total,     87 ns avg
RETURN              5002 calls,     755541 ns total,    151 ns avg
LESS                5001 calls,     476962 ns total,     95 ns avg
GetUpvalue          5000 calls,     475867 ns total,     95 ns avg  <- BOTTLENECK
SetUpvalue          5000 calls,     361764 ns total,     72 ns avg  <- BOTTLENECK
```

## Root Cause Analysis

### 1. **Excessive Closure Cloning** (CRITICAL)
**Location**: `vm.rs:425-427`
```rust
let closure = f.clone();                          // Clone #1
self.call_stack.push(closure.clone(), func_slot); // Clone #2  
self.call(closure, arg_count)?;
```

**Impact**: 
- 2 clones per function call = 10,002 clones total
- `FnClosure` contains `Rc<WeaveFn>` + `Vec<WeaveUpvalue>`
- Each clone creates new Vec + Rc refcount increment

**Fix Priority**: HIGH - Could reduce Call opcode time by 50%+

### 2. **Argument Vector Allocation** 
**Location**: `vm.rs:430-436`
```rust
let args = if arg_count > 0 {
    let last_arg = self.stack.len() - 1;
    let first_arg = last_arg - arg_count;
    self.stack[first_arg..last_arg].to_vec()  // New Vec every call!
} else {
    vec![]
};
```

**Impact**: 
- New `Vec<WeaveType>` allocation per function call
- 5,001 unnecessary heap allocations

**Fix Priority**: HIGH - Use stack slices instead

### 3. **Upvalue Indirection Overhead**
**Location**: Upvalue access pattern
```rust
WeaveUpvalue → Rc<RefCell<InnerUpvalue>> → pattern match → stack/heap access
```

**Impact**:
- GetUpvalue: 95ns average (should be ~10-20ns for stack access)
- SetUpvalue: 72ns average  
- Multiple pointer dereferences per access

**Fix Priority**: MEDIUM - Direct stack access optimization

### 4. **CallFrame Creation Overhead**
**Location**: `vm.rs:52-55`
```rust
pub fn push(&mut self, closure: FnClosure, slot: usize) {
    let frame = CallFrame::new(closure, slot);  // Heavy allocation
    self.frames.push(frame);
}
```

**Impact**:
- New CallFrame + IP creation per call
- Contains cloned FnClosure (see issue #1)

**Fix Priority**: MEDIUM - CallFrame pooling

## Proposed Solutions (Impact Order)

### **Critical Impact (50%+ improvement)**

1. **Eliminate Double Cloning**
   - Use borrowed references in call stack
   - Single clone max per function call
   - **Estimated improvement**: 40-60% reduction in Call opcode time

2. **Argument Slice Optimization** 
   - Use `&[WeaveType]` instead of `Vec<WeaveType>`
   - Zero-allocation argument passing
   - **Estimated improvement**: 20-30% reduction in Call opcode time

### **High Impact (20-40% improvement)**

3. **Fast Path for Simple Functions**
   - Skip full CallFrame for leaf functions with no upvalues
   - Direct execution for simple arithmetic functions
   - **Estimated improvement**: 25-40% for simple function calls

4. **Upvalue Access Optimization**
   - Direct stack access when upvalue is local
   - Bypass RefCell when possible
   - **Estimated improvement**: 50-70% reduction in upvalue access time

### **Medium Impact (10-20% improvement)**

5. **CallFrame Pooling**
   - Reuse CallFrame objects instead of allocation
   - Pre-allocate common cases
   - **Estimated improvement**: 15-25% reduction in allocation overhead

6. **Inline Trivial Functions**
   - Compiler optimization to inline simple operations
   - Eliminate function calls for basic arithmetic
   - **Estimated improvement**: 20-30% for arithmetic-heavy code

## Implementation Priority

### Phase 1: Critical Fixes (Target: 60-80% improvement)
1. Fix double cloning in Call opcode
2. Implement argument slice optimization  
3. Add fast path for simple functions

### Phase 2: Optimization (Target: additional 20-30% improvement)  
4. Optimize upvalue access patterns
5. Implement CallFrame pooling
6. Add function inlining where beneficial

## Success Metrics
- **Phase 1 Target**: Reduce 45s to 10-15s (3-4.5x improvement)
- **Phase 2 Target**: Reduce to <5s (9x+ improvement)  
- **Ultimate Goal**: <1s (45x improvement) - may require additional architectural changes

## Technical Notes
- All optimizations must maintain correct closure/upvalue semantics
- Memory safety is critical - avoid unsafe code unless absolutely necessary
- Profile after each change to validate improvements
- Regression test all closure and function call scenarios