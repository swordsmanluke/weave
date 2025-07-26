# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Weave is a dynamically-typed programming language interpreter written in Rust. It features a bytecode compiler and stack-based virtual machine with support for closures, functions, and an interactive REPL.

Samples of Weave syntax can be found in docs/syntax.md

## Development Strategy

### Phase 0: Get Ready
1. Run the Unit tests with `cargo test` - is anything broken? If so, STOP and ask the User if they would like you to fix the tests.
2. If there are no ready Tasks: Move to Phase 1 to discover the User's next ask.
3. If there is a ready Task: Move to Phase 2 to continue delivery of the in-progress work.

### Phase 1: Discover Requirements
1. Use the /discovery process to ask questions of the User to discover the new requirements.
2. Use task-master to capture the new Tasks
3. Use task-master to examine to break down the new Tasks into subtasks and look for hidden complexity.

### Phase 2: Deliver a Task
1. Select the next task from task-master
2. reflect - do you have all information necessary to implement this? if not, abort and ask for help.
3. Iterate this process:
  - Create test(s) to verify the _desired_ behavior - these tests are expected to fail until the implementation is correct.
  - Use task-master's smart-implemntation workflow to develop the task.
  - Run all of the unit tests regularly to verify both that progress is being made and that no regressions were introduced.
  - Reflect: if progress is stalling, is our approach correct? Use subtasks to consider alternative architectures and approaches. Proceed with the best approach - even if it is to stay on the current course.
  - Repeat this process until the desired behavior has been implemented and all unit tests are passing.

## Development Commands

### Build and Run
- `cargo build` - Build the project in debug mode
- `cargo build --release` - Build optimized release version
- `cargo run` - Start the REPL (interactive shell)
- `cargo run <filename.wv>` - Execute a Weave script file

### Testing and Quality
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test
- `cargo test -- --nocapture` - Run tests with println! output visible
- `cargo run ./sample_weave/<weave file>` - run a sample Weave program to verify compilation
- `cargo check` - Quick syntax and type checking
- `cargo fmt` - Format code according to Rust standards
- `cargo clippy` - Run Rust linter for code improvements

## Documentation
Documentation of this project can be found in docs/

## Architecture Overview

### Compiler Pipeline
The compiler (`src/weave/compiler/`) transforms source code into bytecode:
1. **Scanner** (`scanner.rs`) - Tokenizes source into lexemes
2. **Parser** (`parser.rs`) - Builds AST using Pratt parsing with precedence rules
3. **Compiler** (`compiler.rs`) - Generates bytecode chunks with scope management

### Virtual Machine
The VM (`src/weave/vm/`) executes bytecode:
- **Stack-based execution** with instruction pointer
- **Native function** support for built-in operations
- **Closure support** with upvalues (currently being implemented)

### Type System
Located in `src/weave/vm/types/`:
- `WeaveType` - Base enum for all value types
- Primitive types: `WeaveNumber`, `WeaveString`, boolean
- Function types: `WeaveFn` (user functions), `NativeFn` (built-ins)
- `WeaveUpvalue` - For closure variable capture

### Language Features

**Supported Syntax:**
- Variables and assignment
- Functions with `fn` keyword, closures, and lambdas
- Control flow: `if`/`else`, `while` loops
- Operators: arithmetic, comparison, logical
- Comments starting with `#`
- Special operators: `|>` (pipe), `*>` (map), `&>` (reduce)

**Built-in Functions:**
- `puts(value)` - Print to stdout (temporary until print() is implemented)

**Functions and Lambdas**
- **Function**: `fn foo(a, b) { a + b }` 
- **Lambda**: `foo = ^(a, b) { a + b }`
- 

## Testing Strategy

Tests are embedded in source files using Rust's `#[test]` attribute. Key test locations:
- VM execution tests: `src/weave/vm/vm.rs`
- Compiler tests: `src/weave/compiler/compiler.rs`
- Scanner tests: `src/weave/compiler/scanner.rs`
- Sample Weave programs: `sample_weave/*.wv`

When adding features:
1. Add unit tests covering the new behavior
2. Test edge cases and error conditions
3. Use descriptive test names
4. Use TDD practices to enhance

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
- Run `cargo test` regularly to avoid regressions
- Use `cargo fmt` before committing
- Address `cargo clippy` warnings
- Keep modules small, focused and well-documented
- Use descriptive variable and function names
- Implement `Display` traits for user-facing types

## Task Master AI Instructions
**Import Task Master's development workflow commands and guidelines, treat as if import is in the main CLAUDE.md file.**
@./.taskmaster/CLAUDE.md
