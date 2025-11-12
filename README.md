# Weave

A dynamically-typed programming language interpreter written in Rust, featuring a bytecode compiler and stack-based virtual machine.

## Overview

Weave is an interpreter for the Weave programming language, designed for simple, data-pipeline-oriented scripting. The project implements a compiler that transforms Weave source code into bytecode, which is then executed by a custom virtual machine with support for closures, recursion, and first-class functions.

**Current Status**: Active development - Core language features are implemented including functions, closures, control flow, and recursion. Advanced features like containers, pipeline operators, and file I/O are planned.

## Features

### Currently Implemented

- **Variables and Assignment**: Dynamic typing with automatic type conversions
- **Functions**: Named functions with the `fn` keyword
- **Lambdas**: Anonymous functions using `^` syntax
- **Closures**: Proper lexical scoping with upvalue capture
- **Recursion**: Full support for recursive function calls
- **Control Flow**: `if`/`else` conditionals and `while` loops
- **Operators**: Arithmetic (`+`, `-`, `*`, `/`), comparison (`<`, `>`, `<=`, `>=`, `==`, `!=`), logical (`and`, `or`, `not`)
- **Native Functions**: Built-in functions for I/O and system operations
- **Interactive REPL**: Read-eval-print loop with multi-line support and command history

### Planned Features

See `docs/syntax.md` for the complete language specification including planned features such as:
- Containers (unified list/map data structure)
- Pipeline operators (`|>`, `*>`, `&>`)
- Symbol types
- File I/O with format parsing (CSV, JSON, YAML, etc.)
- Shell command execution
- String manipulation

## Quick Start

### Installation

Requires Rust 2024 edition or later.

```bash
# Clone the repository
git clone <repository-url>
cd weave

# Build the project
cargo build --release

# Run the REPL
cargo run

# Execute a Weave script
cargo run sample_programs/test_simple_factorial.wv
```

## Language Examples

### Hello World

```weave
puts("Hello World")
```

### Variables and Arithmetic

```weave
# Comments begin with #
a = 10
b = 20
sum = a + b
puts(sum)  # prints: 30
```

### Functions

```weave
fn add(x, y) {
    # Last expression is implicitly returned
    x + y
}

result = add(5, 3)
puts(result)  # prints: 8
```

### Factorial with Recursion

```weave
fn factorial(n) {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

puts(factorial(5))  # prints: 120
```

### Closures

```weave
fn make_counter() {
    count = 0

    fn increment() {
        count = count + 1
        count
    }

    increment
}

counter = make_counter()
puts(counter())  # prints: 1
puts(counter())  # prints: 2
puts(counter())  # prints: 3
```

### Lambdas

```weave
# Lambda syntax uses ^ instead of fn
square = ^(x) { x * x }

puts(square(5))  # prints: 25

# Lambdas can capture variables from outer scopes
fn outer() {
    x = 42

    inner = ^(y) { x + y }
    inner(8)
}

puts(outer())  # prints: 50
```

### Control Flow

```weave
fn fizzbuzz(n) {
    i = 1
    while i <= n {
        if i % 15 == 0 {
            puts("FizzBuzz")
        } else {
            if i % 3 == 0 {
                puts("Fizz")
            } else {
                if i % 5 == 0 {
                    puts("Buzz")
                } else {
                    puts(i)
                }
            }
        }
        i = i + 1
    }
}

fizzbuzz(15)
```

## Built-in Functions

### Currently Available

- **`puts(value)`** - Print a value to stdout (temporary implementation)
- **`print(value)`** - Print a value to stdout
- **`input()`** - Read a line from stdin
- **`clock()`** - Get current Unix timestamp
- **`read_file(path)`** - Read file contents as string
- **`write_file(path, content)`** - Write content to file

## Development

### Building and Running

```bash
# Build in debug mode
cargo build

# Build in release mode (optimized)
cargo build --release

# Build with VM profiling
cargo build --features vm-profiling

# Run with debug logging
cargo run -- --log-level debug

# Run with console logging
cargo run -- --log-console

# Run a specific script
cargo run <filename.wv>
```

### Testing

```bash
# Run all tests
cargo test

# Run a specific test
cargo test <test_name>

# Run tests with output visible
cargo test -- --nocapture

# Run sample programs
cargo run sample_programs/test_simple_factorial.wv
```

### Code Quality

```bash
# Quick syntax and type checking
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Architecture

### Compiler Pipeline

The compiler transforms source code into bytecode in three stages:

1. **Scanner** (`src/weave/compiler/scanner.rs`) - Tokenizes source into lexemes
2. **Parser** (`src/weave/compiler/parser.rs`) - Builds abstract syntax using Pratt parsing
3. **Compiler** (`src/weave/compiler/compiler.rs`) - Generates bytecode chunks with scope management

### Virtual Machine

The VM (`src/weave/vm/`) executes bytecode:

- **Stack-based execution** with instruction pointer tracking
- **Call stack** for function invocation and return
- **Upvalue system** for closure variable capture
- **Native function** integration for built-in operations
- **NaN-boxed values** for efficient value representation

### Type System

Located in `src/weave/vm/types/`:

- **NanBoxedValue** - Efficient value representation using NaN-boxing
- **WeaveNumber** - 64-bit numeric values (u64/i64/f64)
- **WeaveString** - String type
- **WeaveFn** - User-defined functions
- **FnClosure** - Function closures with upvalue capture
- **NativeFn** - Built-in native functions

## REPL

The interactive REPL provides:

- Multi-line input support for incomplete expressions
- Command history via arrow keys
- Tab completion (planned)
- Exit with `exit` command or Ctrl+C/Ctrl+D

```bash
$ cargo run
wv> fn greet(name) {
...     puts("Hello, " + name)
... }
wv> greet("World")
Hello, World
wv> exit
```

## Contributing

This is an active development project. Key areas for contribution:

1. Implementing planned language features (see `docs/syntax.md`)
2. Performance optimizations
3. Additional built-in functions
4. Improved error messages
5. Test coverage

Please run `cargo test` and `cargo clippy` before submitting changes.

## Documentation

- **`docs/syntax.md`** - Complete language syntax specification
- **`sample_programs/`** - Example Weave programs for testing

## Project Structure

```
weave/
├── src/
│   ├── main.rs                  # CLI entry point
│   └── weave/
│       ├── compiler/            # Compiler (scanner, parser, codegen)
│       ├── vm/                  # Virtual machine and types
│       ├── shell/               # REPL implementation
│       └── logging/             # Logging infrastructure
├── sample_programs/             # Test programs
├── docs/                        # Documentation
└── tests/                       # Integration tests
```
