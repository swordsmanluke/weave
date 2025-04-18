use crate::weave::compiler::token::{Token, TokenType};

struct CharStream {
    chars: Vec<char>,
    idx: usize,
}

impl CharStream {
    fn new(string: &String) -> CharStream {
        let idx = 0;
        let chars = string.chars().collect::<Vec<char>>();
        CharStream { chars, idx }
    }

    pub fn peek(&self) -> char {
        self.or_not(self.chars.get(self.idx))
    }
    
    pub fn peek_next(&self) -> char { 
        self.or_not(self.chars.get(self.idx + 1))
    }

    pub fn advance(&mut self) -> char {
        let c = self.peek();
        self.idx += 1;
        c
    }

    pub fn matches(&self, c: char) -> bool {
        self.peek() == c
    }

    pub fn next_matches(&self, c: char) -> bool {
        self.peek_next() == c
    }

    fn or_not(&self, optc: Option<&char>) -> char {
        match optc {
            Some(c) => *c,
            None => '\0',
        }
    }
}

pub struct Scanner {
    code: String,
    char_iter: CharStream,
    start: usize,
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new(code: &str) -> Scanner {
        let code = code.to_string();
        let code2 = code.clone();
        Scanner {
            code,
            char_iter: CharStream::new(&code2),
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
        Token::text_token(TokenType::ERROR, (self.start, self.current), message, self.line)
    }

    pub fn basic_token(&self, token_type: TokenType) -> Token {
        Token::basic_token(token_type, (self.start, self.current), self.line)
    }

    pub fn text_token(&self, token_type: TokenType, lextext: &str) -> Token {
        Token::text_token(token_type, (self.start, self.current), lextext, self.line)
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
            return Token::basic_token(TokenType::EOF, (self.start, self.current), self.line);
        }

        match self.advance() {
            c if Scanner::is_alpha(c) => self.scan_identifier(),
            c if Scanner::is_digit(c) => self.scan_number(),

            '(' => self.basic_token(TokenType::LeftParen),
            ')' => self.basic_token(TokenType::RightParen),
            '{' => self.basic_token(TokenType::LeftBrace),
            '}' => self.basic_token(TokenType::RightBrace),
            '[' => self.basic_token(TokenType::LeftBracket),
            ']' => self.basic_token(TokenType::RightBracket),
            ',' => self.basic_token(TokenType::Comma),

            '-' => self.basic_token(TokenType::Minus),
            '+' => self.basic_token(TokenType::Plus),
            ';' => self.basic_token(TokenType::Semicolon),
            '/' => self.basic_token(TokenType::Slash),

            '"' => self.scan_string(),

            '*' => {
                if self.char_iter.next_matches('>') {
                    self.basic_token(TokenType::Map)
                } else {
                    self.basic_token(TokenType::Star)
                }
            }
            '!' => {
                if self.char_iter.next_matches('=') {
                    self.basic_token(TokenType::NEqual)
                } else {
                    self.basic_token(TokenType::Bang)
                }
            }
            '=' => {
                if self.char_iter.next_matches('=') {
                    self.basic_token(TokenType::EqEqual)
                } else {
                    self.basic_token(TokenType::Equal)
                }
            }
            '<' => {
                if self.char_iter.next_matches('=') {
                    self.basic_token(TokenType::LEqual)
                } else {
                    self.basic_token(TokenType::Less)
                }
            }
            '>' => {
                if self.char_iter.next_matches('=') {
                    self.basic_token(TokenType::GEqual)
                } else {
                    self.basic_token(TokenType::Greater)
                }
            }
            '&' => {
                if self.char_iter.next_matches('&') {
                    self.basic_token(TokenType::AndAnd)
                } else if self.char_iter.next_matches('>') {
                    self.basic_token(TokenType::Reduce)
                } else {
                    self.err_token("expected &&")
                }
            }
            '|' => {
                if self.char_iter.next_matches('|') {
                    self.basic_token(TokenType::OrOr)
                } else if self.char_iter.next_matches('>') {
                    self.basic_token(TokenType::Pipe)
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
        self.text_token(TokenType::String, &self.code[self.start+1..self.current-1])
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

        self.text_token(TokenType::Number, &self.code[self.start..self.current])
    }

    fn is_alpha(c: char) -> bool {
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
        self.text_token(token_type, &self.code[self.start..self.current])
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
        assert_eq!(token.lexeme.lexeme(), "hello world");
    }

    #[test]
    fn scan_number() {
        let mut scanner = Scanner::new("123");
        let token = scanner.scan_token();
        assert_eq!(token.token_type, TokenType::Number);
        assert_eq!(token.lexeme.lexeme(), "123");
    }

    #[test]
    fn scan_identifier() {
        let mut scanner = Scanner::new("hello");
        let token = scanner.scan_token();
        assert_eq!(token.token_type, TokenType::Identifier);
        assert_eq!(token.lexeme.lexeme(), "hello");
    }
}