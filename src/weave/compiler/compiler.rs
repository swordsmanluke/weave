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
}

impl Compiler {
    pub fn new(source: &str, debug_mode: bool) -> Compiler {
        Compiler {
            line: 1,
            parser: Parser::new(source),
            had_error: false,
            panic_mode: false,
            chunk: Chunk::new(),
            debug_mode,
        }
    }

    pub fn compile(&mut self) -> CompileResult {
        self.advance();
        self.expression();
        self.consume(TokenType::EOF, "Expected end of file");
        self.emit_opcode(Op::RETURN);

        if self.had_error {
            self.report_err(&self.chunk.disassemble("Chunk Dump"));
            return Err("Compilation error".to_string());
        }
        
        Ok(self.chunk.clone())
    }

    pub fn advance(&mut self) {
        if self.debug_mode { println!("Calling advance"); }
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
        self.had_error = true;
        self.panic_mode = true;
    }

    pub fn expression(&mut self) {
        if self.debug_mode { println!("Parsing Expression"); }
        self.print_progress();
        self.parse_precedence(Precedence::ASSIGNMENT);
    }

    fn print_progress(&mut self) {
        if self.debug_mode { println!("  - Peek: {}\n  - Prev: {}", self.parser.peek(), self.parser.previous()); }
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();

        if self.debug_mode {
            println!("Parsing Precedence {:?} @ {}", precedence, self.parser.previous());
            self.print_progress();
        }

        match ParseRule::for_token(self.parser.previous().token_type).prefix {
            Some(prefix) => prefix(self), // There is a prefix method - , call it
            None => self.report_err("Expected expression"),
        }

        while precedence <= self.parser.peek().token_type.precedence() {
            self.advance();
            match ParseRule::for_token(self.parser.previous().token_type).infix {
                Some(infix) => infix(self),
                None => self.report_err("Expected Infix expression"),
            }
        }
    }

    pub(crate) fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after expression");
    }

    pub(crate) fn unary(&mut self) {
        if self.debug_mode {
            println!("compiling unary @ {}", self.parser.previous());
        }
        let operator = self.parser.previous().token_type;
        self.parse_precedence(Precedence::UNARY);

        match operator {
            TokenType::Minus => self.emit_opcode(Op::NEGATE),
            _ => unreachable!("Not a unary operator"),
        }
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
            TokenType::Plus => self.emit_opcode(Op::ADD),
            TokenType::Minus => self.emit_opcode(Op::SUB),
            TokenType::Slash => self.emit_opcode(Op::DIV),
            TokenType::Star => self.emit_opcode(Op::MUL),
            _ => unreachable!("Not a binary operator"), // Actually, there are several more coming.
        };
    }

    pub fn number(&mut self) {
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

    fn emit_number(&mut self, value: f64) {
        let line = self.line;
        self.current_chunk()
            .add_constant(WeaveType::Number(value.into()), line);
    }

    fn emit_opcode(&mut self, op: Op) {
        let line = self.line;
        self.current_chunk().write_op(op, line);
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
}
