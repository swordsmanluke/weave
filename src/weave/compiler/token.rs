use std::fmt::{Display, Formatter};

pub struct Token<'src> {
    pub token_type: TokenType,
    pub lexeme: &'src str,
    pub line: usize
}

impl<'src> Display for Token<'src> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let out = format!("{}   ({:?})", self.lexeme, self.token_type);
        write!(f, "{}", out)
    }
}

impl <'src> Token<'src> {
    pub fn new(token_type: TokenType, lexeme: &'src str, line: usize) -> Token<'src> {
        Token {
            token_type,
            lexeme,
            line
        }
    }
    
    pub fn length(&self) -> usize {
        self.lexeme.len()
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TokenType {
    // Single-character tokens.
    LeftParen, RightParen,
    LeftBrace, RightBrace,
    LeftBracket, RightBracket,
    Comma, Minus, Plus,
    Semicolon, Slash, Star,
    // One or two character tokens.
    Bang, NEqual,
    Equal, EqEqual,
    Greater, GEqual,
    Less, LEqual,
    
    // Logical operators
    AndAnd, OrOr,
    
    // Pipe tokens
    Pipe, Map, Reduce,
    
    // Literals.
    Identifier, String, Number, Container,
    // Keywords.
    //  - flow control
    If, Else,
    True, False,
    //  - functions
    FN, Return,
    
    // Print helper until print() is implemented
    Puts, 
    // END Keywords
    // Terminations
    ERROR, EOF,
}