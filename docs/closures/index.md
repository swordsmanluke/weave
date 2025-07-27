# Closures in Weave

Closures are one of the most powerful features of the Weave programming language, allowing functions to capture and access variables from their enclosing scopes even after those scopes have been exited. This enables sophisticated functional programming patterns and stateful behavior.

## What are Closures?

A closure is a function that "closes over" variables from its surrounding scope, maintaining access to them throughout its lifetime. In Weave, any function that references variables from an outer scope automatically becomes a closure.

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
c()  # Returns 1
c()  # Returns 2
```

In this example, the inner `counter` function captures the `count` variable from its parent scope, creating a closure that maintains state between calls.

## Key Features

- **Variable Capture**: Functions automatically capture variables from enclosing scopes
- **State Persistence**: Captured variables maintain their state across function calls
- **Nested Closures**: Support for multiple levels of variable capture
- **Shared State**: Multiple closures can share the same captured variables

## Language Syntax

Weave supports two main function syntaxes that can create closures:

### Named Functions
```weave
fn outer() {
    x = 42
    fn inner() { x }  # Captures x
    inner
}
```

### Lambda Functions
```weave
outer = ^() {
    x = 42
    ^() { x }  # Lambda that captures x
}
```

## Common Use Cases

1. **Counter Functions**: Maintaining state between calls
2. **Factory Functions**: Creating functions with pre-configured behavior
3. **Event Handlers**: Capturing context for asynchronous operations
4. **Functional Programming**: Higher-order functions and combinators

## Implementation Overview

Weave's closure implementation uses a sophisticated upvalue system that provides:

- **Memory Safety**: Rust-based implementation with safe memory management
- **High Performance**: Optimized VM operations with ~4x performance improvements through reduced cloning
- **Efficient Operations**: Stack operations use references where possible to minimize allocations
- **Production Ready**: Benchmarks show 10K iterations complete in ~0.25 seconds
- **Flexibility**: Automatic transition between stack and heap storage as needed

## Performance Characteristics

Recent optimizations have significantly improved Weave's runtime performance:

- **VM Optimization**: Eliminated excessive cloning in hot code paths
- **Stack Efficiency**: `GetLocal`, `SetLocal`, `_push`, and `_peek` operations now use references
- **Memory Management**: Reduced memory allocations during variable access
- **Benchmark Results**: Performance tests demonstrate production-ready execution speeds

The VM now achieves approximately 4x better performance compared to earlier implementations, making closures and variable access highly efficient for real-world applications.

## Documentation Structure

This documentation is organized into the following sections:

- **[Architecture](architecture.md)**: Detailed technical implementation of closures and upvalues
- **Architecture Diagrams**: Visual representations of the closure system
  - [Closure Creation Flow](closure-creation-flow.svg)
  - [Upvalue State Transitions](upvalue-states.svg)
  - [Variable Access Patterns](variable-access.svg)

## Getting Started

To understand how closures work in Weave:

1. Start with the basic examples above
2. Review the [architecture documentation](architecture.md) for implementation details
3. Examine the test cases in `src/weave/vm/vm.rs` for comprehensive examples
4. Explore the source code in `src/weave/vm/types/` for the complete implementation

## Related Documentation

- [Syntax Documentation](../syntax.md): Complete Weave language syntax
- [Upvalues Technical Documentation](../upvalues.md): Detailed upvalue implementation
- [VM Architecture](../vm/): Virtual machine implementation details