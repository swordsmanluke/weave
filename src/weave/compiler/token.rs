use std::fmt::{Display, Formatter};
use crate::weave::compiler::parse_rule::ParseRule;
use crate::weave::compiler::precedence::Precedence;

#[derive(PartialEq, Clone, Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: Lexeme,
    pub line: usize
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct Lexeme {
    start: usize,
    end: usize,
    txt: Option<String>
}

impl Lexeme {
    pub fn new(start: usize, end: usize, txt: Option<String>) -> Lexeme {
        Lexeme {
            start,
            end,
            txt
        }
    }

    pub fn lexeme(&self) -> &str {
        &self.txt.as_ref().unwrap()
    }
}

impl Display for Lexeme {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.txt {
            Some(txt) => {
                write!(f, "{}", txt)
            }
            None => {
                write!(f, "")
            }
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}    {}", self.token_type, self.lexeme)
    }
}

impl  Token {
    
    pub fn basic_token(token_type: TokenType, span: (usize, usize), line: usize) -> Token {
        let lex = Lexeme::new(span.0, span.1, None);
        Token::new(token_type, lex, line)
    }
    
    pub fn text_token(token_type: TokenType, span: (usize, usize), lextext: &str, line: usize) -> Token {
        let lex = Lexeme::new(span.0, span.1, Some(lextext.to_string()));
        Token::new(token_type, lex, line)
    }
    
    fn new(token_type: TokenType, lexeme: Lexeme, line: usize) -> Token {
        Token {
            token_type,
            lexeme,
            line
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TokenType {
    // Single-character tokens.
    LeftParen, RightParen,
    LeftBrace, RightBrace,
    LeftBracket, RightBracket,
    Comma, Minus, Plus,
    Semicolon, Slash, Star, Caret,
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
    If, Else, While,
    True, False,
    //  - functions
    FN, Return,
    
    // Print helper until print() is implemented
    Puts, 
    // END Keywords
    // Terminations
    ERROR, EOF,
}

impl TokenType {
    pub fn precedence(&self) -> Precedence {
        ParseRule::for_token(*self).precedence
    }
}