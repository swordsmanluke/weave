use crate::weave::compiler::scanner::Scanner;
use crate::weave::compiler::token::{Token, TokenType};

pub(crate) struct Parser {
    scanner: Scanner,
    tokens: Vec<Token>,
}

impl  Parser {
    pub fn new(code: &str) -> Parser {
        Parser {
            scanner: Scanner::new(code),
            tokens: Vec::new(),
        }
    }

    pub fn cur_is(&self, token_type: TokenType) -> bool {
        self.peek_type() == token_type
    }

    pub fn peek(&self) -> Token {
        self.prev(0)
    }
    
    pub fn previous(&self) -> Token {
        self.prev(1)
    }
    
    fn prev(&self, i: usize) -> Token {
        self.tokens.get(i).cloned().unwrap_or(Token::text_token(TokenType::ERROR, (0, 0), "Out of bounds", 0))
    }
    
    pub fn peek_type(&self) -> TokenType {
        self.peek().token_type
    }

}

impl Iterator for Parser {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let next_tok = self.scanner.scan_token();
        
        if next_tok.token_type == TokenType::EOF {
            if self.peek().token_type == TokenType::EOF { return None }
        }
        
        if next_tok.token_type == TokenType::ERROR { return Some(next_tok); }
        
        // Add it to our history, then return it as the next token
        self.tokens.insert(0, next_tok);
        self.tokens.get(0).cloned()
    }
}


