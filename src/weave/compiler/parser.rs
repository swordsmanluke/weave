use crate::weave::compiler::scanner::Scanner;
use crate::weave::compiler::token::{Token, TokenType};

pub(crate) struct Parser<'src> {
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

    pub fn cur_is(&self, token_type: TokenType) -> bool {
        self.peek_type() == token_type
    }

    pub fn next_is(&self, token_type: TokenType) -> bool {
        self.peek_next_type() == token_type
    }
    
    pub fn peek(&self) -> Token {
        self.tokens.get(0).cloned().unwrap_or(Token::basic_token(TokenType::EOF, (0, 0), 0))
    }

    pub fn peek_type(&self) -> TokenType {
        self.tokens.get(0).map_or(TokenType::EOF, |token| token.token_type)
    }

    pub fn peek_next_type(&self) -> TokenType {
        self.tokens.get(1).map_or(TokenType::EOF, |token| token.token_type)
    }
}

impl Iterator for Parser<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.tokens.push(self.scanner.scan_token());
        self.tokens.get(0).cloned()
    }
}


