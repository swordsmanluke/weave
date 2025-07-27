# Weave Scope Resolution System

## Overview

The Weave programming language implements a stack-based virtual machine with a hierarchical scope system for variable resolution. This document details how scopes are created, managed, and how variables are resolved during compilation and execution.

## Architecture Components

### Core Types

- **`Scope`** (`src/weave/compiler/internal/scope.rs`): Main scope management structure
- **`InnerScope`**: Individual scope level containing locals and upvalues
- **`Local`**: Local variable representation with name and depth
- **`Upvalue`**: Closure variable capture mechanism

### Scope Structure

```rust
pub struct Scope {
    pub depth: u8,                    // Current scope nesting level
    stack: ScopeStack                 // Shared stack of InnerScope levels
}

struct InnerScope {
    pub locals: Vec<Local>,           // Local variables (slot 0 reserved for function)
    pub upvalues: Vec<Upvalue>        // Captured variables from outer scopes
}
```

## Scope Management

### Scope Creation and Nesting

1. **Global Scope (depth=0)**: Created at startup, contains global variables
2. **Function Scopes (depth≥1)**: Created for each function/lambda compilation
3. **Block Scopes**: Currently share the function scope depth

### Scope Operations

#### `enter_scope()` - Create Child Scope
```rust
pub fn enter_scope(&mut self) -> Self { 
    self.incr() 
}

fn incr(&mut self) -> Self {
    let mut child = self.clone();
    child.depth += 1;
    child.stack.borrow_mut().push(InnerScope::new());
    child
}
```

**Critical Issue Identified**: `enter_scope()` accumulates state by cloning the entire scope stack. This causes sequential function compilations to inherit previous function's scope state.

#### `exit_scope()` - Return to Parent
```rust
pub fn exit_scope(&mut self) { 
    self.decr(); 
}

fn decr(&mut self) {
    self.pop_scope();
}
```

## Variable Resolution Process

### Local Variable Resolution

Variables are resolved in this priority order:
1. **Local variables** in current scope
2. **Upvalues** from parent scopes  
3. **Global variables** if not found locally

#### `resolve_local()` Implementation
```rust
pub fn resolve_local(&self, identifier: &str) -> isize {
    let current_depth = self.depth as usize;
    let locals = &self.stack.borrow()[current_depth].locals;
    
    // Search in reverse order (most recent declaration first)
    for (i, l) in locals.iter().enumerate().rev() {
        if l.name.as_str() == identifier {
            return i as isize;  // Returns slot index
        }
    }
    -1  // Not found
}
```

### Upvalue Resolution (Closures)

For variables not found in current scope, the system searches parent scopes recursively:

```rust
fn recursive_resolve_upvalue(&mut self, identifier: &str, depth: usize) -> Option<Upvalue> {
    if depth == 0 { return None; }  // Global scope has no upvalues
    
    let parent_depth = depth - 1;
    
    // Check parent's local variables
    let parent_local = self.stack.borrow_mut()[parent_depth].resolve_local(identifier);
    if let Some(i) = parent_local {
        return Some(self.add_upvalue(Upvalue::local(i as u8), depth))
    }
    
    // Recursively check parent's upvalues
    self.recursive_resolve_upvalue(identifier, parent_depth).map(|parent_resolved| {
        let remote_upvalue = Upvalue { 
            idx: parent_resolved.idx,
            is_local: false, 
            original_idx: parent_resolved.original_idx 
        };
        self.add_upvalue(remote_upvalue, depth)
    })
}
```

## Stack-Based Execution Model

### CallFrame Structure
Each function call creates a CallFrame with:
- **`slot`**: Base stack index for this function's local variables
- **`ip`**: Instruction pointer in function's bytecode
- **Function reference**: The executing function

### Local Variable Indexing

**Reserved Slot Layout**:
```
slot[0]: Function object (reserved)
slot[1]: First parameter/local variable  
slot[2]: Second parameter/local variable
...
```

**VM Local Access**:
```rust
Op::GetLocal => {
    let relative_slot = self.call_stack.next_byte() as usize;
    let absolute_slot = frame.slot + relative_slot;
    let value = self.stack.get(absolute_slot).unwrap();
    self.stack.push(value.clone());
}
```

## Identified Issues

### 1. Sequential Function Compilation Bug

**Problem**: The `enter_scope()` method accumulates scope state across sequential function compilations instead of providing fresh scope contexts.

**Manifestation**:
```weave
lambda1 = ^(x) { x + 1 }  // Compiles correctly, gets slot indices 0,1
temp = 5                  // Global variable  
lambda2 = ^(y) { y * 2 }  // BUG: Gets slot indices 0,1,2 instead of 0,1
```

**Root Cause**: `scope.enter_scope()` at `compiler.rs:592` clones accumulated scope state:
```rust
let new_scope = self.scope.enter_scope();  // Inherits previous function's locals
```

**Impact**: 
- Second lambda gets `relative_slot=2` instead of `1` for parameter `y`
- VM runtime error: "Stack index out of bounds: slot=3, stack_len=3"
- Affects both lambdas AND named functions

### 2. Upvalue Resolution Dependencies

**Challenge**: Creating completely fresh scopes breaks upvalue resolution because parent context is lost.

**Attempted Fix**:
```rust
// This breaks upvalue resolution
let new_scope = Scope::new();  // Fresh scope loses parent context
```

**Error**: `index out of bounds: the len is 1 but the index is 1` in `scope.rs:80`

## Integration with Compilation

### Function Compilation Flow

1. **Named Functions** (`function_statement()` at `compiler.rs:276`):
   ```rust
   let new_scope = self.scope.enter_scope();          // Create child scope
   let mut func_compiler = self.new_func_compiler(name, new_scope);
   func_compiler.function();                          // Compile function body
   self.emit_closure(func_compiler.function, ...);    // Emit closure bytecode
   self.scope.exit_scope();                           // Clean up scope
   ```

2. **Lambda Expressions** (`lambda()` at `compiler.rs:588`):
   ```rust
   let new_scope = self.scope.enter_scope();          // Same pattern as functions
   let mut func_compiler = self.new_func_compiler("<lambda>", new_scope);
   func_compiler.lambda_function();                   // Compile lambda body  
   self.emit_closure(func_compiler.function, ...);    // Emit closure bytecode
   self.scope.exit_scope();                           // Clean up scope
   ```

### Variable Compilation

Variables are compiled differently based on scope context:

```rust
fn set_named_variable(&mut self, identifier: String) {
    if self.scope.depth > 0 {
        // Local scope - try local first, then upvalue, finally create new local
        let idx = self.resolve_local(identifier.as_str());
        if idx.is_some() {
            self.emit_opcode(Op::SetLocal, &[idx.unwrap() as u8]);
        } else {
            // Check for upvalue or create new local
            // ...
        }
    } else {
        // Global scope - emit global variable access
        self.emit_basic_opcode(Op::SetGlobal);
    }
}
```

## Performance Characteristics

### Scope Operations
- **Local variable lookup**: O(n) linear search in current scope
- **Upvalue resolution**: O(d×n) where d=depth, n=locals per scope
- **Scope creation**: O(1) but accumulates state incorrectly

### Memory Usage
- Each `InnerScope`: ~24 bytes + locals vector + upvalues vector
- Scope stack shared via `Rc<RefCell<>>` for efficient cloning
- Memory leak potential due to accumulated scope state

## Testing Coverage

### Current Test Cases
From `vm.rs`, lambda tests reveal the scope issue:

```rust
#[test]
fn test_lambda_with_intermediate_variables() {
    let code = "
        lambda1 = ^(x) { x + 1 }  // Works: slot indices 0,1
        temp = 5                  // Global variable
        lambda2 = ^(y) { y * 2 }  // Fails: wrong slot indexing
        
        result1 = lambda1(temp)   // Success: 6
        result2 = lambda2(temp)   // Runtime error
        result1 + result2
    ";
    // Expected: 16 (6 + 10), Actual: Stack index error
}
```

### Debug Instrumentation
Added debug output reveals the issue:

```
GetLocal: slot=1, stack_len=3  // lambda1 parameter - correct
GetLocal: slot=2, stack_len=3  // lambda2 parameter - WRONG! Should be slot=1
```

## Recommended Fixes

### 1. Scope Isolation Fix
Modify `enter_scope()` to create truly independent scopes for function compilation:

```rust
pub fn enter_scope_for_function(&mut self) -> Self {
    // Create fresh scope that maintains upvalue resolution capability
    // but doesn't inherit accumulated local variable state
}
```

### 2. Test-Driven Development
Expand test coverage to catch scope isolation issues:

```rust
#[test]
fn test_sequential_function_compilation() {
    // Test both lambdas and named functions
    let code = "
        fn func1(x) { x + 1 }
        temp = 5
        fn func2(y) { y * 2 }
        func1(temp) + func2(temp)
    ";
    // Should compile without scope accumulation
}
```

### 3. Scope State Validation
Add validation to detect scope accumulation during compilation:

```rust
fn validate_scope_isolation(&self) {
    assert_eq!(self.scope.stack.borrow()[self.scope.depth].locals.len(), 1,
               "Function scope should start with only reserved slot");
}
```

## Future Improvements

1. **Block Scope Support**: Implement proper block-level scoping for `if`/`while` statements
2. **Scope Caching**: Cache local variable lookups for performance
3. **Memory Management**: Implement scope cleanup to prevent memory leaks
4. **Cross-Scope Analysis**: Add compile-time analysis to detect variable capture patterns

## Conclusion

The Weave scope resolution system provides a solid foundation for variable management and closure support. However, the sequential function compilation bug represents a critical issue that affects both lambda expressions and named functions. The bug stems from scope state accumulation in the `enter_scope()` method, causing incorrect local variable slot assignments in the VM's stack-based execution model.

The fix requires careful balance between providing isolated scope contexts for function compilation while maintaining the upvalue resolution mechanism that enables proper closure behavior.