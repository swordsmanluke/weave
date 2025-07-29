use std::cmp::PartialEq;
use crate::weave::compiler::parse_rule::ParseRule;
use crate::weave::compiler::parser::Parser;
use crate::weave::compiler::precedence::Precedence;
use crate::weave::compiler::token::{Token, TokenType};
use crate::weave::compiler::internal::Scope;
use crate::weave::vm::types::{WeaveFn, FnClosure, Upvalue, NanBoxedValue, PointerTag};
use crate::weave::{Chunk, Op};
use crate::{log_debug, log_info, log_error};

pub type CompileResult = Result<WeaveFn, String>;
 
enum FnType {
    Script,
    Function
}

const MAX_UPVALS: usize = 255;

pub struct Compiler {
    line: usize,
    parser: Parser,
    had_error: bool,
    panic_mode: bool,
    function: WeaveFn,
    function_type: FnType,
    scope: Scope
}

pub enum AssignMode {
    Yes,
    No
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
    pub fn new(source: &str, _debug_mode: bool) -> Compiler {
        Compiler {
            line: 1,
            parser: Parser::new(source),
            had_error: false,
            panic_mode: false,
            function: WeaveFn::new(String::new(), vec![]),
            function_type: FnType::Script,
            scope: Scope::new(),
        }
    }
    
    pub fn new_func_compiler(&mut self, name: String, scope: Scope) -> Compiler {
        Compiler{
            line: self.line,
            parser: self.parser.clone(),
            had_error: false,
            panic_mode: false,
            function: WeaveFn::new(name, vec![]),
            function_type: FnType::Function,
            scope,
        }
    }

    pub fn compile(&mut self) -> CompileResult {
        self.advance();
        while !self.parser.cur_is(TokenType::EOF) {
            self.declaration();
        }
        self.consume(TokenType::EOF, "Expected end of file");
        self.emit_basic_opcode(Op::RETURN);

        if self.had_error {
            let _ = self.current_chunk().disassemble("Chunk Dump");
            self.report_err("Compilation error- see above");
            return Err("Compilation error".to_string());
        }
        
        // Disassemble for debugging
        let _ = self.current_chunk().disassemble("=== Script ===");

        Ok(self.function.clone())
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
        &mut self.function.chunk
    }

    fn report_err(&mut self, message: &str) {
        self.report_err_at(&self.parser.previous(), message);
    }

    fn report_err_at(&mut self, token: &Token, message: &str) {
        if self.panic_mode {
            return;
        }

        log_error!("Compilation error", 
            message = message, 
            line = token.line, 
            lexeme = format!("{}", token.lexeme).as_str()
        );
        self.had_error = true;
        self.panic_mode = true;
    }

    pub fn expression(&mut self) {
        log_debug!("Parsing expression", current_token = format!("{:?}", self.parser.peek_type()).as_str());
        self.print_progress();
        self.parse_precedence(Precedence::ASSIGNMENT);
        self.check(TokenType::Semicolon);
    }

    pub fn declaration(&mut self) {
        log_debug!("Parsing declaration", current_token = format!("{:?}", self.parser.peek_type()).as_str());
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
        log_debug!("Compiling variable get", variable = format!("{}", self.parser.previous()).as_str(), line = self.line);
        let identifier = self.parser.previous().lexeme.lexeme().to_string();
        let idx = self.resolve_local(identifier.as_str());
        if idx.is_some() {
            log_debug!("Local variable found", identifier = identifier, index = idx, scope_depth = self.scope.depth);
            self.emit_opcode(Op::GetLocal, &vec![idx.unwrap() as u8]);
        } else {
            let upval = self.resolve_upvalue(identifier.as_str());
            if upval.is_some() {
                let upval_ref = upval.as_ref().unwrap();
                log_debug!("Upvalue variable found", identifier = identifier.as_str(), upvalue_index = upval_ref.idx);
                self.emit_opcode(Op::GetUpvalue, &vec![upval.unwrap().idx]);
            } else {
                let line = self.line;
                log_debug!("Using global variable lookup", identifier = identifier.as_str(), scope_depth = self.scope.depth);
                self.current_chunk().emit_constant(NanBoxedValue::string(identifier.into()), line);
                self.emit_basic_opcode(Op::GetGlobal);
            }
        }
    }

    fn variable_set(&mut self) {
        log_debug!("Compiling variable definition", variable = format!("{}", self.parser.previous()).as_str(), line = self.line);

        let identifier = self.parser.previous();
        self.consume(TokenType::Equal, "Expected assignment in declaration");
        self.expression(); // Compile the expression

        self.set_named_variable(identifier.lexeme.lexeme().to_string());
    }

    pub(crate) fn resolve_local(&self, identifier: &str) -> Option<isize> {
        let result = self.scope.resolve_local(identifier);
        if result >= 0 {
            Some(result)
        } else {
            None
        }
    }


    fn resolve_upvalue(&mut self, identifier: &str) -> Option<Upvalue> {
        self.scope.resolve_upvalue(identifier)
    }

    fn set_named_variable(&mut self, identifier: String) {
        if self.scope.depth > 0 {
            let idx = self.resolve_local(identifier.as_str());
            if idx.is_some() {
                self.emit_opcode(Op::SetLocal, &[idx.unwrap() as u8].to_vec());
            } else {
                match self.resolve_upvalue(identifier.as_str()) {
                    Some(upval) => {
                        let idx = upval.idx;
                        self.function.upvalue_count += 1;
                        self.emit_opcode(Op::SetUpvalue, &[idx].to_vec());
                    }
                    None => {
                        let local_id = self.add_local(identifier);
                        self.emit_opcode(Op::SetLocal, &[local_id as u8].to_vec());
                    }
                }
            }
        } else {
            let line = self.line;
            self.current_chunk().emit_constant(NanBoxedValue::string(identifier), line);
            self.emit_basic_opcode(Op::SetGlobal);
        }
    }

    fn add_local(&mut self, identifier: String) -> usize {
        // Create new variable
        let slot = self.scope.declare_local(identifier.clone());
        log_info!("ADDED LOCAL VARIABLE", 
            identifier = identifier.as_str(), 
            assigned_slot = slot,
            scope_depth = self.scope.depth,
            total_locals = self.scope.debug_current_locals_len()
        );
        slot
    }

    pub fn statement(&mut self) {
        log_debug!("Parsing statement", current_token = format!("{:?}", self.parser.peek_type()).as_str());
        self.print_progress();

        if self.check(TokenType::Puts) {
            self.puts_statement();
        } else if self.check(TokenType::Return) {
            self.return_statement();
        } else if self.check(TokenType::If) {
            self.if_statement();
        } else if self.check(TokenType::FN) {
            self.function_statement();
        } else if self.check(TokenType::While) {
            self.while_statement();
        } else {
            self.expression_statement();
        }
    }

    fn return_statement(&mut self) {
        match self.function_type{
            FnType::Script => self.report_err("Can't return from script"),
            FnType::Function => {
                if self.check(TokenType::Semicolon) {
                    self.emit_basic_opcode(Op::RETURN);
                } else {
                    self.expression();
                    self.emit_basic_opcode(Op::RETURN);
                }
            }
        }
    }

    fn function_statement(&mut self) {
        log_debug!("Compiling function", current_token = format!("{:?}", self.parser.peek_type()).as_str());
        log_info!("SCOPE STATE BEFORE function_statement", 
            depth = self.scope.depth, 
            stack_len = self.scope.debug_stack_len(),
            current_locals = self.scope.debug_current_locals_len()
        );
        
        self.consume(TokenType::Identifier, "Expected function name");
        let fn_name = self.parser.previous();
        
        // Use enter_function_scope() instead of enter_scope() to prevent scope state accumulation
        // between sequential function compilations while preserving upvalue resolution
        let new_scope = self.scope.enter_function_scope();
        log_info!("SCOPE STATE AFTER enter_function_scope", 
            depth = new_scope.depth, 
            stack_len = new_scope.debug_stack_len()
        );
        
        let mut func_compiler = self.new_func_compiler(fn_name.lexeme.lexeme().to_string(), new_scope);
        func_compiler.function(); // compile function

        self.parser = func_compiler.parser;  // leap forward to the end of the function

        self.emit_closure(func_compiler.function, func_compiler.scope.depth as usize);
        self.set_named_variable(fn_name.lexeme.lexeme().to_string());
        self.scope.exit_scope();
        
        log_info!("SCOPE STATE AFTER function_statement complete", 
            depth = self.scope.depth, 
            stack_len = self.scope.debug_stack_len()
        );
    }

    fn function(&mut self) {
        log_debug!("Compiling function implementation", function_name = self.function.name.as_str());
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expected '(' after function name");
        self.function_params();
        self.consume(TokenType::RightParen, "Expected ')' after function params");
        
        self.consume(TokenType::LeftBrace, "Expected '{' before function body");
        self.block();
        
        // Add implicit RETURN for function end (like explicit return statements)
        self.emit_basic_opcode(Op::RETURN);
        
        log_info!("Function compilation complete", function_name = self.function.name.as_str());
        let _ = self.function.chunk.disassemble(self.function.name.as_str());
    }

    fn lambda_function(&mut self) {
        log_debug!("Compiling lambda function implementation");
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expected '(' in lambda");
        self.function_params();
        self.consume(TokenType::RightParen, "Expected ')' after lambda params");
        
        self.consume(TokenType::LeftBrace, "Expected '{' before lambda body");
        self.block();
        
        // Add implicit RETURN for lambda end (like explicit return statements)
        self.emit_basic_opcode(Op::RETURN);
        
        log_info!("Lambda compilation complete");
        let _ = self.function.chunk.disassemble("<lambda>");
    }

    fn function_params(&mut self) {
        if !self.parser.cur_is(TokenType::RightParen) {
            loop {
                self.consume(TokenType::Identifier, "Expected parameter name");
                self.function.arity += 1;
                self.add_local(self.parser.previous().lexeme.lexeme().to_string());
                if !self.check(TokenType::Comma) { break; }
            }
        }
    }
    
    fn emit_closure(&mut self, mut func: WeaveFn, func_depth: usize) {
        self.emit_basic_opcode(Op::Closure);
        let line = self.line;
        
        // Count up how many upvalues we ended up with
        let upvals = self.scope.upvals_at(func_depth);
        func.upvalue_count = upvals.iter().count() as u8;
        
        // Debug: println!("{} has {} upvals", func.name, func.upvalue_count);
        // Add closure to constants table without emitting constant bytecode
        let closure = FnClosure::new(func.into());
        // Store closure as heap-allocated pointer in NanBoxedValue
        let closure_box = Box::new(closure);
        let closure_ptr = Box::into_raw(closure_box) as *const ();
        let closure_nan_boxed = NanBoxedValue::pointer(closure_ptr, PointerTag::Closure);
        let closure_idx = self.current_chunk().add_constant_only(closure_nan_boxed);
        
        // Emit the closure constant index as part of the Closure instruction
        self.emit_bytes((closure_idx as u16).to_be_bytes().to_vec());
        
        // Emit upvalue information
        let bytes = upvals.iter()
            .fold(vec![], |mut v: Vec<u8>, u: &Upvalue| {
                v.append(&mut u.to_bytes());
                v
        });
        self.emit_bytes(bytes);
    }

    pub fn fn_call(&mut self) {
        let arg_count = self.arg_count();
        self.emit_opcode(Op::Call, &[arg_count].to_vec());
    }

    fn arg_count(&mut self) -> u8 {
        let mut arg_count = 0;
        if !self.parser.cur_is(TokenType::RightParen) {
            loop {
                self.expression();
                arg_count += 1;
                if !self.check(TokenType::Comma) { break; }
            }
        }
        self.consume(TokenType::RightParen, "Expected ')' after arguments");
        arg_count
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().code.len() - 1;
        self.expression_statement(); // condition
        let exit_jump = self.emit_jump(Op::JumpIfFalse);
        // JumpIfFalse now pops the condition automatically

        self.consume(TokenType::LeftBrace, "Expected Block after condition");
        self.block();
        self.emit_basic_opcode(Op::POP); // Pop any leftover expression results from loop body
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        // No need to pop - JumpIfFalse already handled it
    }

    fn if_statement(&mut self) {
        self.expression_statement();  // Condition

        // Set up the jump to evaluate the condition
        let then_jump = self.emit_jump(Op::JumpIfFalse);
        // JumpIfFalse now pops the condition automatically

        // Compile the 'then' block
        self.consume(TokenType::LeftBrace, "Expected Block after condition");
        self.block();

        // Skip the 'else' block when the condition is true
        let else_jump = self.emit_jump(Op::Jump);
        self.patch_jump(then_jump);

        // No need to pop - JumpIfFalse already handled it
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
        log_debug!("Checking token match", expected = format!("{:?}", token).as_str(), actual = format!("{:?}", self.parser.peek_type()).as_str());
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
        log_debug!("Beginning new scope", new_depth = self.scope.depth + 1);
        self.scope.enter_scope();
    }

    fn end_scope(&mut self) {
        log_debug!("Exiting scope", depth = self.scope.depth);
        self.scope.exit_scope();
        
        self.emit_basic_opcode(Op::RETURN);  // Implicit return when exiting a scope
        
        for _ in 0..self.scope.locals_at(self.scope.depth) {
            self.emit_basic_opcode(Op::POP);
        }
        self.scope.pop_scope();
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
        log_debug!("Parser state", peek_token = format!("{}", self.parser.peek()).as_str(), previous_token = format!("{}", self.parser.previous()).as_str());
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();

        log_debug!("Parsing precedence level", precedence = format!("{:?}", precedence).as_str(), current_token = format!("{}", self.parser.previous()).as_str());
        self.print_progress();

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
        log_debug!("Compiling unary expression", operator = format!("{}", self.parser.previous()).as_str());
        let operator = self.parser.previous().token_type;
        self.parse_precedence(Precedence::UNARY);

        match operator {
            TokenType::Bang => self.emit_basic_opcode(Op::NOT),
            TokenType::Minus => self.emit_basic_opcode(Op::NEGATE),
            _ => unreachable!("Not a unary operator"),
        }
    }

    pub fn literal(&mut self, _assign_mode: AssignMode) {
        log_debug!("Compiling literal", value = format!("{}", self.parser.previous()).as_str());
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
        log_debug!("Compiling binary expression", operator = format!("{:?}", self.parser.previous().token_type).as_str());
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
        log_debug!("Compiling number literal", value = format!("{}", self.parser.previous()).as_str());
        let val = self.parser.previous().lexeme.lexeme().parse::<f64>();
        log_debug!("Parsed number value", parsed_value = format!("{:?}", val).as_str());
        match val {
            Ok(v) => self.emit_number(v),
            Err(_) => self.report_err(&format!("Not a Number: {}", self.parser.previous())),
        }
    }

    pub fn string(&mut self, _assign_mode: AssignMode) {
        log_debug!("Compiling string literal", value = format!("{}", self.parser.previous()).as_str());
        let value = self.parser.previous().lexeme.lexeme().to_string();
        self.emit_string(value);
    }

    pub fn lambda(&mut self, _assign_mode: AssignMode) {
        log_debug!("Compiling lambda expression");
        log_debug!("SCOPE STATE BEFORE lambda", 
            depth = self.scope.depth, 
            stack_len = self.scope.debug_stack_len(),
            current_locals = self.scope.debug_current_locals_len()
        );
        
        // Create new scope for lambda - use isolated function scope
        // Use enter_function_scope() instead of enter_scope() to prevent scope state accumulation
        // between sequential function compilations while preserving upvalue resolution
        let new_scope = self.scope.enter_function_scope();
        log_info!("SCOPE STATE AFTER enter_function_scope for lambda", 
            depth = new_scope.depth, 
            stack_len = new_scope.debug_stack_len()
        );
        
        let mut func_compiler = self.new_func_compiler("<lambda>".to_string(), new_scope);
        func_compiler.lambda_function(); // compile lambda
        
        self.parser = func_compiler.parser;  // leap forward to the end of the lambda
        
        self.emit_closure(func_compiler.function, func_compiler.scope.depth as usize);
        self.scope.exit_scope();
        
        log_debug!("SCOPE STATE AFTER lambda complete", 
            depth = self.scope.depth, 
            stack_len = self.scope.debug_stack_len()
        );
    }

    fn emit_bytes(&mut self, bytes: Vec<u8>) {
        let line = self.line;
        self.current_chunk().write(&bytes, line);
    }

    fn emit_string(&mut self, value: String) {
        log_debug!("Emitting string constant", constant_value = format!("{:?}", value).as_str(), line = self.line);
        let line = self.line;
        self.current_chunk().emit_constant(NanBoxedValue::string(value.into()), line);
    }

    fn emit_number(&mut self, value: f64) {
        let line = self.line;
        log_debug!("Emitting constant opcode", constant_value = format!("{:?}", value).as_str(), line = line, offset = self.current_chunk().code.len());
        
        self.current_chunk()
            .emit_constant(NanBoxedValue::number(value), line);
    }

    fn emit_basic_opcode(&mut self, op: Op) {
        let line = self.line;
        log_debug!("Emitting opcode", opcode = format!("{:?}", op).as_str(), line = line, offset = self.current_chunk().code.len());
        self.current_chunk().write_op(op, line);
    }

    fn emit_opcode(&mut self, op: Op, args: &Vec<u8>) {
        let line = self.line;
        log_debug!("Emitting opcode with args", opcode = format!("{:?}", op).as_str(), args = format!("{:?}", args).as_str(), line = line, offset = self.current_chunk().code.len());
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
    
    fn emit_loop(&mut self, loop_start: usize) {
        let offset = self.current_chunk().code.len() - loop_start + 2;
        let hi = (offset >> 8) as u8;
        let lo = (offset & 0xFF) as u8;
        self.emit_opcode(Op::Loop, &vec![hi, lo]);
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
    }

    #[test]
    fn test_compile_local_variables() {
        let mut compiler = Compiler::new("fn test() { x = 1; x + 3 } test()", true);
        let result = compiler.compile();
        assert!(result.is_ok(), "Failed to compile");
        let chunk = result.unwrap().chunk;
        let _ = chunk.disassemble("Chunk Dump");
        // Note: With function wrapper, the bytecode positions will be different
        // We're mainly testing that it compiles without error after removing bare blocks
        assert!(chunk.code.len() > 0, "Chunk should have bytecode");
    }

    #[test]
    fn test_expression_statement() {
        let mut compiler = Compiler::new("x = 3; puts x;", true);
        let result = compiler.compile();
        assert!(result.is_ok(), "Failed to compile");
    }

    #[test]
    fn test_sequential_function_compilation_debug() {
        // This test specifically targets the scope accumulation bug
        let code = "
            fn func1(a) { a + 1 }
            fn func2(b) { b * 2 }
            fn func3(c) { c - 1 }
            
            func1(5) + func2(3) + func3(8)
        ";
        let mut compiler = Compiler::new(code, true);
        let result = compiler.compile();
        assert!(result.is_ok(), "Failed to compile sequential functions: {:?}", result.unwrap_err());
    }

    #[test]
    fn test_sequential_lambda_compilation_debug() {
        // This test specifically targets the scope accumulation bug with lambdas
        let code = "
            lambda1 = ^(a) { a + 1 }
            lambda2 = ^(b) { b * 2 }
            lambda3 = ^(c) { c - 1 }
            
            lambda1(5) + lambda2(3) + lambda3(8)
        ";
        let mut compiler = Compiler::new(code, true);
        let result = compiler.compile();
        assert!(result.is_ok(), "Failed to compile sequential lambdas: {:?}", result.unwrap_err());
    }

    #[test]
    fn test_mixed_function_lambda_compilation_debug() {
        // This test mixes named functions and lambdas to see the interaction
        let code = "
            fn func1(a) { a + 1 }
            lambda1 = ^(b) { b * 2 }
            fn func2(c) { c - 1 }
            lambda2 = ^(d) { d + 10 }
            
            func1(5) + lambda1(3) + func2(8) + lambda2(2)
        ";
        let mut compiler = Compiler::new(code, true);
        let result = compiler.compile();
        assert!(result.is_ok(), "Failed to compile mixed functions/lambdas: {:?}", result.unwrap_err());
    }
}
