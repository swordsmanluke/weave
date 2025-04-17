use crate::weave::compiler::token::{Token, TokenType};
use std::str::Chars;

struct CharStream<'a> {
    stream: Chars<'a>,
    cur: Option<char>,
    next: Option<char>,
}

impl<'a> CharStream<'a> {
    fn new(code: &'a str) -> CharStream<'a> {
        let mut stream = code.chars();
        let cur = stream.next();
        let next = stream.next();
        CharStream { stream, cur, next }
    }

    pub fn peek(&self) -> char {
        self.cur.unwrap_or('\0')
    }
    
    pub fn peek_next(&self) -> char { 
        self.next.unwrap_or('\0')
    }

    pub fn advance(&mut self) -> char {
        let c = self.peek();
        println!("advancing: {}", c);
        self.cur = self.next;
        self.next = self.stream.next();
        c
    }

    pub fn matches(&self, c: char) -> bool {
        self.cur == Some(c)
    }

    pub fn next_matches(&self, c: char) -> bool {
        self.next == Some(c)
    }
}

pub struct Scanner<'a> {
    code: &'a str,
    char_iter: CharStream<'a>,
    start: usize,
    current: usize,
    line: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(code: &'a str) -> Scanner<'a> {
        Scanner {
            code,
            char_iter: CharStream::new(&code),
            start: 0,
            current: 0,
            line: 1,
        }
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.char_iter.advance()
    }

    fn cur_lexeme(&self) -> &str {
        &self.code[self.start..self.current]
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.code.len()
    }

    pub fn err_token(&self, message: &'static str) -> Token {
        Token::new(TokenType::ERROR, message, self.line)
    }

    pub fn new_token(&self, token_type: TokenType) -> Token {
        Token::new(token_type, self.cur_lexeme(), self.line)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.char_iter.peek() {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                '#' => {
                    while !self.is_at_end() && self.char_iter.peek() != '\n' {
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    pub fn scan_token(&mut self) -> Token {
        self.skip_whitespace();
        self.start = self.current; // Reset the self/scanner

        if self.is_at_end() {
            return Token::new(TokenType::EOF, "", self.line);
        }

        match self.advance() {
            c if Scanner::is_alpha(c) => self.scan_identifier(),
            c if Scanner::is_digit(c) => self.scan_number(),
            
            '(' => self.new_token(TokenType::LeftParen),
            ')' => self.new_token(TokenType::RightParen),
            '{' => self.new_token(TokenType::LeftBrace),
            '}' => self.new_token(TokenType::RightBrace),
            '[' => self.new_token(TokenType::LeftBracket),
            ']' => self.new_token(TokenType::RightBracket),
            ',' => self.new_token(TokenType::Comma),

            '-' => self.new_token(TokenType::Minus),
            '+' => self.new_token(TokenType::Plus),
            ';' => self.new_token(TokenType::Semicolon),
            '/' => self.new_token(TokenType::Slash),

            '"' => self.scan_string(),

            '*' => {
                if self.char_iter.next_matches('>') {
                    self.new_token(TokenType::Map)
                } else {
                    self.new_token(TokenType::Star)
                }
            }
            '!' => {
                if self.char_iter.next_matches('=') {
                    self.new_token(TokenType::NEqual)
                } else {
                    self.new_token(TokenType::Bang)
                }
            }
            '=' => {
                if self.char_iter.next_matches('=') {
                    self.new_token(TokenType::EqEqual)
                } else {
                    self.new_token(TokenType::Equal)
                }
            }
            '<' => {
                if self.char_iter.next_matches('=') {
                    self.new_token(TokenType::LEqual)
                } else {
                    self.new_token(TokenType::Less)
                }
            }
            '>' => {
                if self.char_iter.next_matches('=') {
                    self.new_token(TokenType::GEqual)
                } else {
                    self.new_token(TokenType::Greater)
                }
            }
            '&' => {
                if self.char_iter.next_matches('&') {
                    self.new_token(TokenType::AndAnd)
                } else if self.char_iter.next_matches('>') {
                    self.new_token(TokenType::Reduce)
                } else {
                    self.err_token("expected &&")
                }
            }
            '|' => {
                if self.char_iter.next_matches('|') {
                    self.new_token(TokenType::OrOr)
                } else if self.char_iter.next_matches('>') {
                    self.new_token(TokenType::Pipe)
                } else {
                    self.err_token("expected || or |>")
                }
            }

            _ => self.err_token("Unexpected character"),
        }
    }

    fn scan_string(&mut self) -> Token {
        println!("scanning string");
        // Down the road, we'll want to support interpolation, but for right now, simple string parsing is good enough
        while !self.is_at_end() && !self.char_iter.matches('"') {
            if self.char_iter.matches('\n') { self.line += 1; }
            self.advance();
        }
        if self.is_at_end() {
            return self.err_token("Unterminated string");
        }
        self.advance(); // consume the "

        // +1 and -1 to account for the quote markers
        Token::new(TokenType::String, &&self.code[self.start+1..self.current-1], self.line)
    }

    fn scan_number(&mut self) -> Token {
        while self.char_iter.peek().is_digit(10) {
            self.advance();
        }

        if self.char_iter.matches('.') && self.char_iter.peek_next().is_digit(10) {
            self.advance();
            while self.char_iter.peek().is_digit(10) {
                self.advance();
            }
        }

        self.new_token(TokenType::Number)
    }

    fn is_alpha(c: char) -> bool {
        println!("is_alpha: '{}'? {}", c, c.is_alphabetic());
        c.is_alphabetic()
    }
    
    fn is_digit(c: char) -> bool {
        c.is_digit(10)
    }

    fn scan_identifier(&mut self) -> Token {
        fn is_identifier_part(c: char) -> bool {
            c.is_alphanumeric() || c.is_digit(10) || c == '_'
        }
        
        while is_identifier_part(self.char_iter.peek()) {
            self.advance();
        }

        let token_type = self.identifier_type();
        self.new_token(token_type)
    }

    fn identifier_type(&self) -> TokenType {
        match self.cur_lexeme() {
            // Keywords
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "true" => TokenType::True,
            "false" => TokenType::False,
            "fn" => TokenType::FN,
            "return" => TokenType::Return,
            "puts" => TokenType::Puts,

            // Okay, just a normal identifier
            _ => TokenType::Identifier,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn scan_string() {
        let mut scanner = Scanner::new("\"hello world\"");
        let token = scanner.scan_token();
        assert_eq!(token.token_type, TokenType::String);
        assert_eq!(token.lexeme, "hello world");
    }   
    
    #[test]
    fn scan_number() {
        let mut scanner = Scanner::new("123");
        let token = scanner.scan_token();
        assert_eq!(token.token_type, TokenType::Number);
        assert_eq!(token.lexeme, "123");
    }
    
    #[test]
    fn scan_identifier() {
        let mut scanner = Scanner::new("hello");
        let token = scanner.scan_token();
        assert_eq!(token.token_type, TokenType::Identifier);
        assert_eq!(token.lexeme, "hello");
    }
}