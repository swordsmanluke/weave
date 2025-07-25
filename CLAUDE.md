# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Weaver is a dynamically-typed programming language interpreter written in Rust. It features a bytecode compiler and stack-based virtual machine with support for closures, functions, and an interactive REPL.

## Development Strategy

### Phase 0: Get Ready
1. Run the Unit tests with `cargo test` - is anything broken? If so, STOP and ask the User if they would like you to fix the tests.
2. If there are no ready Tasks: Move to Phase 1 to discover the User's next ask.
3. If there are 

### Phase 1: Discover Requirements
1. Use the /discovery process to ask questions of the User to discover the new requirements.
2. Use task-master to capture the new Tasks
3. Use task-master to examine to break down the new Tasks into subtasks and look for hidden complexity.

### Phase 2: Deliver a Task
1. Select the next task from task-master
2. reflect - are you ready to implement this? if not, abort and ask for help.
3. Begin by creating tests to verify the desired behavior
4. Use task-master's smart-implemntation workflow to develop the task. 
5. Run the unit tests regularly to verify that progress is being made and that no regressions were introduced.

## Development Commands

### Build and Run
- `cargo build` - Build the project in debug mode
- `cargo build --release` - Build optimized release version
- `cargo run` - Start the REPL (interactive shell)
- `cargo run <filename.wv>` - Execute a Weaver script file

### Testing and Quality
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test
- `cargo test -- --nocapture` - Run tests with println! output visible
- `cargo check` - Quick syntax and type checking
- `cargo fmt` - Format code according to Rust standards
- `cargo clippy` - Run Rust linter for code improvements

## Architecture Overview

### Compiler Pipeline
The compiler (`src/weave/compiler/`) transforms source code into bytecode:
1. **Scanner** (`scanner.rs`) - Tokenizes source into lexemes
2. **Parser** (`parser.rs`) - Builds AST using Pratt parsing with precedence rules
3. **Compiler** (`compiler.rs`) - Generates bytecode chunks with scope management

### Virtual Machine
The VM (`src/weave/vm/`) executes bytecode:
- **Stack-based execution** with instruction pointer
- **Garbage collected** heap for dynamic values
- **Native function** support for built-in operations
- **Closure support** with upvalues (currently being enhanced)

### Type System
Located in `src/weave/vm/types/`:
- `WeaveType` - Base enum for all value types
- Primitive types: `WeaveNumber`, `WeaveString`, boolean
- Function types: `WeaveFn` (user functions), `NativeFn` (built-ins)
- `WeaveUpvalue` - For closure variable capture

### Language Features

**Supported Syntax:**
- Variables and assignment
- Functions with `fn` keyword and closures
- Control flow: `if`/`else`, `while` loops
- Operators: arithmetic, comparison, logical
- Comments starting with `#`
- Special operators: `|>` (pipe), `*>` (map), `&>` (reduce)

**Built-in Functions:**
- `puts(value)` - Print to stdout (temporary until print() is implemented)

## Testing Strategy

Tests are embedded in source files using Rust's `#[test]` attribute. Key test locations:
- VM execution tests: `src/weave/vm/vm.rs`
- Compiler tests: `src/weave/compiler/compiler.rs`
- Scanner tests: `src/weave/compiler/scanner.rs`

When adding features:
1. Add unit tests in the relevant module
2. Test edge cases and error conditions
3. Use descriptive test names

## Current Development Focus

The project is actively developing closure support with upvalues. Recent commits show work on:
- Implementing proper upvalue capture
- REPL improvements
- Native function integration

See `src/weave/TODO.md` for planned features including:
- Additional types (datetime, streams)
- Built-in functions (string manipulation, math, network)
- Random number generation

## REPL Usage

The REPL provides an interactive environment:
- Prompt: `wv>`
- Multi-line support for unclosed expressions
- Command history via arrow keys
- Exit with `exit` command or Ctrl+C/Ctrl+D

## Code Style Guidelines

- Follow Rust idioms and conventions
- Use `cargo fmt` before committing
- Address `cargo clippy` warnings
- Keep modules focused and well-documented
- Use descriptive variable and function names
- Implement `Display` traits for user-facing types

## Task Master AI Instructions
**Import Task Master's development workflow commands and guidelines, treat as if import is in the main CLAUDE.md file.**
@./.taskmaster/CLAUDE.md
