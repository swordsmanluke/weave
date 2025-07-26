use std::rc::Rc;
use crate::weave::compiler::token::{Token, TokenType};
use crate::log_debug;

#[derive(Debug, Clone)]
pub struct Scanner {
    code: Rc<String>,
    start: usize,
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new(code: &str, _debug_mode: bool) -> Scanner {
        let code = Rc::new(code.to_string());
        Scanner {
            code: code.clone(),
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn peek(&self) -> char {
        self.code.chars().nth(self.current).unwrap_or('\0')
    }

    pub fn peek_next(&self) -> char {
        self.code.chars().nth(self.current + 1).unwrap_or('\0')
    }

    pub fn advance(&mut self) -> char {
        let c = self.peek();
        self.current += 1;
        c
    }

    pub fn matches(&self, c: char) -> bool {
        self.peek() == c
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
        log_debug!("Scanner emitting token", token_type = format!("{:?}", token_type).as_str(), line = self.line);
        Token::basic_token(token_type, (self.start, self.current), self.line)
    }

    pub fn text_token(&self, token_type: TokenType, lextext: &str) -> Token {
        log_debug!("Scanner emitting text token", token_type = format!("{:?}", token_type).as_str(), lexeme = lextext, line = self.line);
        Token::text_token(token_type, (self.start, self.current), lextext, self.line)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    log_debug!("Scanner encountered newline", line = self.line + 1);
                    self.line += 1;
                    self.advance();
                }
                '#' => {
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    pub fn scan_token(&mut self) -> Token {
        log_debug!("Scanner scanning next token", current_pos = self.current);
        self.skip_whitespace();
        log_debug!("Scanner whitespace skipped", new_pos = self.current);
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
                if self.consume('>') {
                    self.basic_token(TokenType::Map)
                } else {
                    self.basic_token(TokenType::Star)
                }
            }
            '!' => {
                if self.consume('=') {
                    self.basic_token(TokenType::NEqual)
                } else {
                    self.basic_token(TokenType::Bang)
                }
            }
            '=' => {
                if self.consume('=') {
                    self.basic_token(TokenType::EqEqual)
                } else {
                    self.basic_token(TokenType::Equal)
                }
            }
            '<' => {
                if self.consume('=') {
                    self.basic_token(TokenType::LEqual)
                } else {
                    self.basic_token(TokenType::Less)
                }
            }
            '>' => {
                if self.consume('=') {
                    self.basic_token(TokenType::GEqual)
                } else {
                    self.basic_token(TokenType::Greater)
                }
            }
            '&' => {
                if self.consume('&') {
                    self.basic_token(TokenType::AndAnd)
                } else if self.consume('>') {
                    self.basic_token(TokenType::Reduce)
                } else {
                    self.err_token("expected &&")
                }
            }
            '|' => {
                if self.consume('|') {
                    self.basic_token(TokenType::OrOr)
                } else if self.consume('>') {
                    self.basic_token(TokenType::Pipe)
                } else {
                    self.err_token("expected || or |>")
                }
            }

            _ => self.err_token("Unexpected character"),
        }
    }

    fn consume(&mut self, c: char) -> bool {
        if self.matches(c) { self.advance(); true }
        else { false }
    }

    fn scan_string(&mut self) -> Token {
        log_debug!("Scanner scanning string literal", start_pos = self.start, line = self.line);
        // Down the road, we'll want to support interpolation, but for right now, simple string parsing is good enough
        while !self.is_at_end() && !self.matches('"') {
            if self.matches('\n') { self.line += 1; }
            self.advance();
        }
        if self.is_at_end() {
            return self.err_token("Unterminated string");
        }
        self.advance(); // consume the "

        // +1 and -1 to account for the quote markers
        let str_start = self.start + 1;
        let str_end = (self.current as isize - 1) as usize;
        self.text_token(TokenType::String, &self.code[str_start..str_end])
    }

    fn scan_number(&mut self) -> Token {
        while self.peek().is_digit(10) {
            self.advance();
        }

        if self.matches('.') && self.peek_next().is_digit(10) {
            self.advance();
            while self.peek().is_digit(10) {
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

        while is_identifier_part(self.peek()) {
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
            "while" => TokenType::While,
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
        let mut scanner = Scanner::new("\"hello world\"", true);
        let token = scanner.scan_token();
        assert_eq!(token.token_type, TokenType::String);
        assert_eq!(token.lexeme.lexeme(), "hello world");
    }

    #[test]
    fn scan_number() {
        let mut scanner = Scanner::new("123", true);
        let token = scanner.scan_token();
        assert_eq!(token.token_type, TokenType::Number);
        assert_eq!(token.lexeme.lexeme(), "123");
    }

    #[test]
    fn scan_identifier() {
        let mut scanner = Scanner::new("hello", true);
        let token = scanner.scan_token();
        assert_eq!(token.token_type, TokenType::Identifier);
        assert_eq!(token.lexeme.lexeme(), "hello");
    }
}