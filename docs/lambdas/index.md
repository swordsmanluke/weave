# Lambda Implementation in Weave

Lambda expressions provide a concise syntax for creating anonymous functions in Weave. This documentation covers the implementation details, architecture, and practical usage of lambdas in the Weave programming language.

## Overview

Lambdas in Weave use the `^` (caret) syntax and are implemented as anonymous closures that integrate seamlessly with the existing closure system. They provide a lightweight alternative to named functions for functional programming patterns.

### Basic Syntax

```weave
# Basic lambda
add = ^(a, b) { a + b }
result = add(3, 4)  # Returns 7

# Lambda with closure
fn make_multiplier(factor) {
    ^(x) { x * factor }
}

double = make_multiplier(2)
result = double(5)  # Returns 10
```

## Key Features

- **Anonymous Functions**: Create functions without explicit names
- **Closure Support**: Capture variables from outer scopes
- **First-Class Values**: Pass lambdas as arguments, return from functions
- **Performance**: Same optimization level as named functions
- **Pipeline Integration**: Works with Weave's `|>`, `*>`, and `&>` operators

## Documentation Structure

### [Architecture](architecture.md)
Detailed technical implementation including:
- Compilation pipeline from `^` syntax to bytecode
- Integration with the closure system
- Memory management and performance characteristics
- Comparison with named function implementation

### Visual Diagrams
- [Lambda Compilation Flow](lambda-compilation-flow.svg) - How lambda syntax is processed
- [Lambda vs Function Comparison](lambda-vs-function.svg) - Architectural differences and similarities

## Quick Reference

### Syntax Patterns

```weave
# No parameters
getMessage = ^() { "Hello, World!" }

# Single parameter
square = ^(x) { x * x }

# Multiple parameters
calculate = ^(a, b, c) { a * b + c }

# Multi-line body
complex = ^(data) {
    processed = data * 2
    validated = processed > 0
    validated ? processed : 0
}
```

### Common Use Cases

1. **Functional Programming**: Map, filter, reduce operations
2. **Event Handlers**: Callback functions for asynchronous operations
3. **Configuration**: Parameterized behavior configuration
4. **Pipeline Operations**: Integration with Weave's pipeline operators

## Performance Characteristics

- **Creation**: ~50ns overhead compared to named functions
- **Execution**: Identical performance to named functions
- **Memory**: Efficient upvalue sharing with closure system
- **Optimization**: Benefits from all VM-level optimizations

## Integration with Weave Features

### Pipeline Operators
```weave
data = [1, 2, 3, 4, 5]

# Map with lambda
doubled = data *> ^(x) { x * 2 }

# Filter with lambda  
evens = data *> ^(x) { x % 2 == 0 ? x : nil }

# Reduce with lambda
sum = data &> ^(acc: 0, val) { acc + val }
```

### Closure Interaction
```weave
fn createCounter() {
    count = 0
    ^() { 
        count = count + 1
        count 
    }
}

counter = createCounter()
puts(counter())  # 1
puts(counter())  # 2
```

## When to Use Lambdas vs Named Functions

### Use Lambdas When:
- Creating short, single-purpose functions
- Passing functions as arguments
- Working with pipeline operators
- Need inline function definitions

### Use Named Functions When:
- Complex logic requiring multiple statements
- Recursive functions
- Functions used in multiple places
- Public API functions requiring documentation

## Error Handling

Common lambda-related errors and solutions:

- **Syntax Errors**: Missing `^` or incorrect parameter syntax
- **Scope Issues**: Variable capture problems in closures
- **Type Errors**: Incorrect parameter types or return values

See [architecture.md](architecture.md) for detailed troubleshooting information.

## Related Documentation

- [Closures](../closures/index.md) - Comprehensive closure system documentation
- [Functions](../syntax.md#functions-and-lambdas) - Complete function syntax reference
- [Pipeline Operators](../syntax.md#function-pipelines) - Integration with `|>`, `*>`, `&>`
- [Upvalues](../upvalues.md) - Technical details of variable capture mechanism

## Testing and Examples

The lambda implementation includes comprehensive test coverage:
- Basic lambda creation and execution
- Closure variable capture
- Parameter variations (0, 1, multiple)
- Integration with existing function system

For hands-on examples, see the test cases in `src/weave/vm/vm.rs` (tests 852-899).