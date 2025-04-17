use crate::weave::Chunk;
use crate::weave::compiler::scanner::{Scanner};
use crate::weave::compiler::token::{Token, TokenType};

pub type CompileResult = Result<Chunk, String>;

pub struct Compiler<'src> {
    line: usize,
    parser: Parser<'src>
}


impl<'src> Compiler<'src> {
    pub fn new(source: &'src str) -> Compiler<'src> {
        Compiler {
            line: 1,
            parser: Parser::new(source)
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
                        report_err(&token);
                    }
                },
                None => { return }
            }
        }
    }

    // pub fn expression(&self) -> Result<(), String> {
    //     Ok(())
    // }
    //
    // pub fn consume(&mut self, token_type: TokenType, message: &str) -> Result<(), String> {
    //     Ok(())
    // }
}

fn report_err(token: &Token) {
    println!("Error on line {}:\n\t{}", token.line, token.lexeme);
}

struct Parser<'src> {
    scanner: Scanner<'src>,
    tokens: Vec<Token>,
}

impl <'src> Parser<'src> {
    pub fn new(code: &'src str) -> Parser<'src> {
        Parser {
            scanner: Scanner::new(code),
            tokens: Vec::new(),
        }
    }
}

impl Iterator for Parser<'_> {
    type Item = Token;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.tokens.push(self.scanner.scan_token());
        self.tokens.get(0).cloned()
    }
}


