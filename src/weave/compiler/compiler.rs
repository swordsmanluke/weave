use std::cmp::PartialEq;
use std::io::{stdout, Write};
use crate::weave::compiler::parse_rule::ParseRule;
use crate::weave::compiler::parser::Parser;
use crate::weave::compiler::precedence::Precedence;
use crate::weave::compiler::token::{Token, TokenType};
use crate::weave::vm::types::WeaveType;
use crate::weave::{Chunk, Op};

pub type CompileResult = Result<Chunk, String>;

pub struct Compiler {
    line: usize,
    parser: Parser,
    had_error: bool,
    panic_mode: bool,
    chunk: Chunk,
    debug_mode: bool,
    scope: Scope
}

struct Local {
    name: Box<String>,
    depth: u8
}

struct Scope {
    locals: Vec<Local>,
    scope_type: Vec<ScopeType>,
    depth: u8
}


impl Scope {
    fn new() -> Scope {
        Scope {
            locals: Vec::new(),
            scope_type: Vec::new(),
            depth: 0
        }
    }

    fn incr(&mut self, scope_type: ScopeType) {
        self.scope_type.push(scope_type);
        self.depth += 1;
    }

    fn decr(&mut self) {
        self.scope_type.pop();
        self.depth -= 1;
    }

    pub fn enter_if_scope(&mut self) { self.incr(ScopeType::If); }
    pub fn enter_fn_scope(&mut self) { self.incr(ScopeType::Fn); }
    pub fn enter_gen_scope(&mut self) { self.incr(ScopeType::General); }
    pub fn exit_scope(&mut self) { self.decr(); }
    pub fn should_shadow(&self) -> bool { self.scope_type.last() == Some(&ScopeType::Fn) }
}

pub enum AssignMode {
    Yes,
    No
}

#[derive(PartialEq)]
enum ScopeType {
    If,
    Fn,
    General,
}

impl PartialEq for AssignMode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AssignMode::Yes, AssignMode::Yes) => true,
            (AssignMode::No, AssignMode::No) => true,
            _ => false
        }
    }
}

impl Compiler {
    pub fn new(source: &str, debug_mode: bool) -> Compiler {
        Compiler {
            line: 1,
            parser: Parser::new(source),
            had_error: false,
            panic_mode: false,
            chunk: Chunk::new(),
            scope: Scope::new(),
            debug_mode,
        }
    }

    pub fn compile(&mut self) -> CompileResult {
        self.advance();
        while !self.parser.cur_is(TokenType::EOF) {
            self.declaration();
        }
        self.consume(TokenType::EOF, "Expected end of file");
        self.emit_basic_opcode(Op::EXIT);

        if self.had_error {
            self.chunk.disassemble("Chunk Dump");
            self.report_err("Compilation error- see above");
            return Err("Compilation error".to_string());
        }

        Ok(self.chunk.clone())
    }

    pub fn advance(&mut self) {
        loop {
            if let Some(token) = self.parser.next() {
                match token.token_type {
                    TokenType::ERROR => self.report_err_at(&token, "Parsing error"),
                    _ => {
                        self.line = token.line;
                        break;
                    }
                }
            } else { break; }
        }
    }

    pub fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.parser.cur_is(token_type) {
            self.advance();
        } else {
            self.report_err(message);
        }
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.chunk
    }

    fn report_err(&mut self, message: &str) {
        self.report_err_at(&self.parser.previous(), message);
    }

    fn report_err_at(&mut self, token: &Token, message: &str) {
        if self.panic_mode {
            return;
        }

        println!("{}", message);
        println!("Error on line {}:\n\t{}", token.line, token.lexeme);
        stdout().flush();
        self.had_error = true;
        self.panic_mode = true;
    }

    pub fn expression(&mut self) {
        if self.debug_mode { println!("Parsing Expression"); }
        self.print_progress();
        self.parse_precedence(Precedence::ASSIGNMENT);
        self.check(TokenType::Semicolon);
    }

    pub fn declaration(&mut self) {
        if self.debug_mode { println!("Parsing Declaration"); }
        self.print_progress();

        if self.panic_mode { self.synchronize(); }

        self.statement();
    }

    pub fn variable(&mut self, assign_mode: AssignMode) {
        // This could be a few cases
        //   1 - declaring a new variable
        //   2 - assigning to an existing var
        //   3 - evaling an existing var
        // In Weave, cases 1&2 have the same syntax: x = y
        // So we need to emit a single opcode 'assign' for both and let the VM handle it.
        // The third case requires that there is _not_ an equal sign after the identifier.
        // So we have to consume the identifier... then see what comes next to know what
        // to emit!
        if self.parser.peek_type() == TokenType::Equal && assign_mode == AssignMode::Yes {
            self.variable_set();
        } else {
            self.variable_get();
        }
    }

    fn variable_get(&mut self) {
        if self.debug_mode { println!("compiling variable GET @ {}", self.parser.previous()); }
        let identifier = self.parser.previous().lexeme.lexeme().to_string();
        let idx = self.resolve_local(identifier.as_str());
        if idx != -1 {
            self.emit_opcode(Op::GetLocal, &vec![idx as u8]);
        } else {
            self.chunk.add_constant(WeaveType::String(identifier.into()), self.line);
            self.emit_basic_opcode(Op::GetGlobal);
        }
    }

    fn variable_set(&mut self) {
        if self.debug_mode { println!("compiling variable DEF @ {}", self.parser.previous()); }

        let identifier = self.parser.previous();
        self.consume(TokenType::Equal, "Expected assignment in declaration");
        self.expression(); // Compile the expression

        self.set_named_variable(identifier.lexeme.lexeme().to_string());
    }

    fn resolve_local(&self, identifier: &str) -> isize {
        println!("Looking for local var: {}", identifier);
        println!("Locals: {}", self.scope.locals.iter().map(|l| l.name.as_str()).collect::<Vec<&str>>().join(", "));
        if self.scope.locals.is_empty() {
            println!("No local variables");
            return -1;
        }
        
        for (i, l) in self.scope.locals.iter().enumerate().rev() {
            if l.name.as_str() == identifier {
                print!("Found local variable {}", l.name);
                // Found the variable, but we can only assign to variables in our _immediate_ scope
                if self.scope.should_shadow() {
                    println!("....but we're shadowing, so create a new var!");
                    return -1;
                }
                println!("... and we're not shadowing, so we can use it!");
                return i as isize; 
            }
        }
        
        return -1;
    }

    fn set_named_variable(&mut self, identifier: String) {
        if self.scope.depth > 0 {
            let idx = self.resolve_local(identifier.as_str());
            if idx != -1 {
                self.emit_opcode(Op::SetLocal, &[idx as u8].to_vec());
            } else {
                // Create new variable
                let local = Local { name: identifier.into(), depth: self.scope.depth };
                self.scope.locals.push(local);
                self.emit_opcode(Op::SetLocal, &[self.scope.locals.len() as u8  - 1].to_vec());
            }
        } else {
            self.chunk.add_constant(WeaveType::String(identifier.into()), self.line);
            self.emit_basic_opcode(Op::SetGlobal);
        }
    }

    pub fn statement(&mut self) {
        if self.debug_mode { println!("Parsing Statement"); }
        self.print_progress();

        if self.check(TokenType::Puts) {
            self.puts_statement();
        } else if self.check(TokenType::If) {
            self.if_statement();
        } else if self.check(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn if_statement(&mut self) {
        self.expression_statement();  // Condition

        // Set up the jump to evaluate the condition
        let then_jump = self.emit_jump(Op::JumpIfFalse);
        self.emit_basic_opcode(Op::POP);  // Pop the condition off the stack

        // Compile the 'then' block
        self.consume(TokenType::LeftBrace, "Expected Block after condition");
        self.block();

        // Skip the 'else' block when the condition is true
        let else_jump = self.emit_jump(Op::Jump);
        self.patch_jump(then_jump);

        // Another place we may need to pop the condition
        self.emit_basic_opcode(Op::POP);
        if self.check(TokenType::Else) {
            // Compile the 'else' block
            self.consume(TokenType::LeftBrace, "Expected Block after 'else'");
            self.block();
        }
        self.patch_jump(else_jump);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.check(TokenType::Semicolon);
    }

    fn check(&mut self, token: TokenType) -> bool {
        if self.debug_mode { println!("Checking: {:?} == {:?}", token, self.parser.peek_type()); }
        if self.parser.cur_is(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn puts_statement(&mut self) {
        self.expression();
        self.emit_basic_opcode(Op::PRINT);
    }

    fn begin_scope(&mut self) {
        if self.debug_mode { println!("Begin Scope {}", self.scope.depth + 1); }
        match self.parser.previous().token_type {
            TokenType::If => self.scope.enter_if_scope(),
            TokenType::FN => self.scope.enter_fn_scope(),
            _ => self.scope.enter_gen_scope(),  // just a local scoped statement like "a=1; { a += 2; }" we should shadow here.
        }
    }

    fn end_scope(&mut self) {
        if self.debug_mode { println!("Exit Scope {}", self.scope.depth); }
        self.scope.exit_scope();
        
        self.emit_basic_opcode(Op::RETURN);  // Set the last value of the stack for returning to reference of scope 

        while !self.scope.locals.is_empty() && self.scope.locals.last().unwrap().depth > self.scope.depth {
            self.emit_basic_opcode(Op::POP);
            self.scope.locals.pop();
        }
    }

    fn block(&mut self) {
        while !self.parser.cur_is(TokenType::RightBrace) && !self.parser.cur_is(TokenType::EOF) {
            self.declaration();
        }

        self.consume(TokenType::RightBrace, "Expected '}' after block");
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;

        while !self.parser.cur_is(TokenType::EOF) {
            if self.parser.previous().token_type == TokenType::Semicolon {
                return;
            }

            match self.parser.peek_type() {
                TokenType::FN | TokenType::Puts | TokenType::If | TokenType::Return => return,
                _ => (),
            }

            self.advance();
        }
    }

    fn print_progress(&mut self) {
        if self.debug_mode { println!("  - Peek: {}\n  - Prev: {}\n", self.parser.peek(), self.parser.previous()); }
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();

        if self.debug_mode {
            println!("Parsing Precedence {:?} @ {}", precedence, self.parser.previous());
            self.print_progress();
        }

        let assign_mode = if precedence > Precedence::ASSIGNMENT { AssignMode::No } else { AssignMode::Yes }; // if precedence is higher than ASSIGNMENT, then it is an assignment expression. Otherwise, it is not.AssignMode::No;

        match ParseRule::for_token(self.parser.previous().token_type).prefix {
            Some(prefix) => prefix(self, assign_mode), // There is a prefix method - , call it
            None => self.report_err(&format!("Expected prefix expression for token {}", self.parser.previous())),
        }

        while precedence <= self.parser.peek().token_type.precedence() {
            self.advance();
            match ParseRule::for_token(self.parser.previous().token_type).infix {
                Some(infix) => infix(self),
                None => self.report_err("Expected Infix expression"),
            }
        }
    }

    pub(crate) fn grouping(&mut self, _assign_mode: AssignMode) {
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after expression");
    }

    pub(crate) fn unary(&mut self, _assign_mode: AssignMode) {
        if self.debug_mode {
            println!("compiling unary @ {}", self.parser.previous());
        }
        let operator = self.parser.previous().token_type;
        self.parse_precedence(Precedence::UNARY);

        match operator {
            TokenType::Bang => self.emit_basic_opcode(Op::NOT),
            TokenType::Minus => self.emit_basic_opcode(Op::NEGATE),
            _ => unreachable!("Not a unary operator"),
        }
    }

    pub fn literal(&mut self, _assign_mode: AssignMode) {
        if self.debug_mode {
            println!("compiling literal @ {}", self.parser.previous());
        }
        match self.parser.previous().token_type {
            TokenType::True => self.emit_basic_opcode(Op::TRUE),
            TokenType::False => self.emit_basic_opcode(Op::FALSE),
            _ => unreachable!("Not a literal"),
        }
    }
    
    pub fn log_and(&mut self) {
        let end_jump = self.emit_jump(Op::JumpIfFalse);
        self.emit_basic_opcode(Op::POP);  // Pop the condition off the stack()
        self.parse_precedence(Precedence::AND);
        self.patch_jump(end_jump);
    }
    
    pub fn log_or(&mut self) {
        let else_jump = self.emit_jump(Op::JumpIfFalse);
        let end_jump = self.emit_jump(Op::Jump);
        self.patch_jump(else_jump);
        self.emit_basic_opcode(Op::POP);  // Pop the condition off the stack()
        self.parse_precedence(Precedence::OR);
        self.patch_jump(end_jump);
    }

    pub(crate) fn binary(&mut self) {
        if self.debug_mode {
            println!("compiling binary");
            self.print_progress();
        }
        let operator = self.parser.previous().token_type;
        let rule = ParseRule::for_token(operator);

        self.parse_precedence(rule.precedence.next());

        match operator {
            TokenType::Plus => self.emit_basic_opcode(Op::ADD),
            TokenType::Minus => self.emit_basic_opcode(Op::SUB),
            TokenType::Slash => self.emit_basic_opcode(Op::DIV),
            TokenType::Star => self.emit_basic_opcode(Op::MUL),
            TokenType::Greater => self.emit_basic_opcode(Op::GREATER),
            TokenType::Less => self.emit_basic_opcode(Op::LESS),
            TokenType::EqEqual => self.emit_basic_opcode(Op::EQUAL),
            TokenType::GEqual => {
                self.emit_basic_opcode(Op::LESS);
                self.emit_basic_opcode(Op::NOT)
            },
            TokenType::LEqual => {
                self.emit_basic_opcode(Op::GREATER);
                self.emit_basic_opcode(Op::NOT)
            },
            TokenType::NEqual => {
                self.emit_basic_opcode(Op::EQUAL);
                self.emit_basic_opcode(Op::NOT)
            }
            _ => unreachable!("Not a binary operator"), // Actually, there are several more coming.
        };
    }

    pub fn number(&mut self, _assign_mode: AssignMode) {
        if self.debug_mode {
            println!("compiling number @ {}", self.parser.previous());
        }
        let val = self.parser.previous().lexeme.lexeme().parse::<f64>();
        if self.debug_mode {
            println!("val: {:?}", val);
        }
        match val {
            Ok(v) => self.emit_number(v),
            Err(_) => self.report_err(&format!("Not a Number: {}", self.parser.previous())),
        }
    }

    pub fn string(&mut self, _assign_mode: AssignMode) {
        if self.debug_mode { println!("compiling string @ {}", self.parser.previous()); }
        let value = self.parser.previous().lexeme.lexeme().to_string();
        self.emit_string(value);
    }

    fn emit_string(&mut self, value: String) {
        if self.debug_mode { println!("Emitting opcode CONSTANT: {} at line {}", value, self.line); }
        let line = self.line;
        self.current_chunk().add_constant(WeaveType::String(value.into()), line);
    }

    fn emit_number(&mut self, value: f64) {
        let line = self.line;
        if self.debug_mode { println!("Emitting opcode CONSTANT: {} at line {} offset {}", value, line, self.current_chunk().code.len()); }
        self.current_chunk()
            .add_constant(WeaveType::Number(value.into()), line);
    }

    fn emit_basic_opcode(&mut self, op: Op) {
        let line = self.line;
        if self.debug_mode { println!("Emitting opcode: {:?} at line {} offset {}", op, line, self.current_chunk().code.len()); }
        self.current_chunk().write_op(op, line);
    }

    fn emit_opcode(&mut self, op: Op, args: &Vec<u8>) {
        let line = self.line;
        if self.debug_mode { println!("Emitting opcode: {:?} {:?} at line {} offset {}", op, args, line, self.current_chunk().code.len()); }
        self.current_chunk().write_op(op, line);
        self.current_chunk().write(args, line);
    }

    fn emit_jump(&mut self, op: Op) -> usize {
        self.emit_opcode(op, &vec![0xFF, 0xFF]);
        self.current_chunk().code.len() - 2
    }

    fn patch_jump(&mut self, jmp_param: usize) {
        let jump = self.current_chunk().code.len() - jmp_param - 2;
        self.current_chunk().code[jmp_param] = (jump >> 8) as u8;
        self.current_chunk().code[jmp_param + 1] = (jump & 0xFF) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile() {
        let mut compiler = Compiler::new("1 + 2", true);
        assert!(compiler.compile().is_ok(), "Failed to compile");
    }

    #[test]
    fn test_compile_global_variables() {
        let mut compiler = Compiler::new("x = 1", true);
        let result = compiler.compile();
        assert!(result.is_ok(), "Failed to compile");
        assert!(result.unwrap().constants.len() > 0, "Global \"x\" not created");
    }

    #[test]
    fn test_compile_local_variables() {
        let mut compiler = Compiler::new("{ x = 1; x + 3 }", true);
        let result = compiler.compile();
        assert!(result.is_ok(), "Failed to compile");
        let chunk = result.unwrap();
        chunk.disassemble("Chunk Dump");
        let bytecode = chunk.code[3];
        assert!(bytecode == Op::SetLocal.bytecode()[0], "{} != {}", bytecode, Op::SetLocal.bytecode()[0]);

        let bytecode = chunk.code[5];
        assert!(bytecode == Op::GetLocal.bytecode()[0], "{} != {}", bytecode, Op::GetLocal.bytecode()[0]);
    }

    #[test]
    fn test_expression_statement() {
        let mut compiler = Compiler::new("x = 3; puts x;", true);
        let result = compiler.compile();
        assert!(result.is_ok(), "Failed to compile");
    }
}
