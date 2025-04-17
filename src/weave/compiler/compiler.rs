use crate::weave::Chunk;
use crate::weave::compiler::parser::Parser;
use crate::weave::compiler::token::{Token, TokenType};

pub type CompileResult = Result<Chunk, String>;

pub struct Compiler<'src> {
    line: usize,
    parser: Parser<'src>,
    had_error: bool,
    panic_mode: bool,
}


impl<'src> Compiler<'src> {
    pub fn new(source: &'src str) -> Compiler<'src> {
        Compiler {
            line: 1,
            parser: Parser::new(source),
            had_error: false,
            panic_mode: false
        }
    }

    pub fn compile(&mut self) -> CompileResult {
        self.advance();
        // compiler.expression();
        // compiler.consume(TokenType::EOF, "Expected end of expression")?;

        Ok(Chunk::new())
    }

    pub fn advance(&mut self) {
        loop {
            match self.parser.next() {
                Some(token) => {
                    if token.token_type == TokenType::ERROR {
                        self.report_err_at(&token, "Compiler error");
                    }
                },
                None => { return }
            }
        }
    }
    
    pub fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.parser.cur_is(token_type) {
            self.advance();
        } else {
            self.report_err(&format!("Expected {:?} but found {:?}", token_type, self.parser.peek_type()));
        }
    }
    
    fn report_err(&mut self, message: &str) {
        self.report_err_at(&self.parser.peek(), message);
    }

    fn report_err_at(&mut self, token: &Token, message: &str) {
        if self.panic_mode { return }
        
        println!("{}", message);
        println!("Error on line {}:\n\t{}", token.line, token.lexeme);
        self.had_error = true;
        self.panic_mode = true;
    }

    // pub fn expression(&self) -> Result<(), String> {
    //     Ok(())
    // }
    //
    // pub fn consume(&mut self, token_type: TokenType, message: &str) -> Result<(), String> {
    //     Ok(())
    // }
}

