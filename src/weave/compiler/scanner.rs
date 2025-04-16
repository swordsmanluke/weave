use crate::weave::compiler::token::{Token, TokenType};

pub struct Scanner {
}

pub struct Span {
    code: String,
    start: usize,
    current: usize,
    line: usize,
}

impl Span {
    pub fn new(code: &str) -> Span {
        let code = code.to_string();
        Span {
            code,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn cur_lexeme(&self) -> &str {
        &self.code[self.start..self.current]
    }

    pub fn is_at_end(&self) -> bool {
        self.current >= self.code.len()
    }

    pub fn advance(&mut self) -> &str {
        self.current += 1;
        self.code.get(self.current - 1..self.current).unwrap()
    }

    pub fn peek(&self) -> Option<&str> {
        self.peeknth(0)
    }

    pub fn peeknth(&self, n: usize) -> Option<&str> {
        if self.is_at_end() { return None; }
        self.code.get(self.current+n..self.current + n + 1)
    }

    pub fn peekc(&self) -> Option<char> {
        self.peeknthc(0)
    }

    pub fn peeknthc(&self, n: usize) -> Option<char> {
        if self.is_at_end() { return None; }
        self.code.chars().nth(self.current + n)
    }

    pub fn consume(&mut self, pat: &str) -> bool {
        match self.peek() {
            None => false,
            Some(p) => {
                if p == pat {
                    self.advance();
                    true
                } else {
                    false
                }
            }
        }
    }
    
    pub fn err_token(&self, message: &str) -> Token {
        Token::new(TokenType::ERROR, message, self.line)
    } 
    
    pub fn new_token(&self, token_type: TokenType) -> Token {
        Token::new(token_type, self.cur_lexeme(), self.line)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                None => return,
                Some(c) => {
                    match c {
                        " " |
                        "\t" |
                        "\r" => { self.advance(); }
                        "\n" => { self.line += 1; self.advance(); }
                        "#"  => { while !self.is_at_end() && self.peek() != Some("\n") { self.advance(); } }
                        _ => return
                    }
                }
            }
        }
    }
}

impl Scanner {

    pub fn new() -> Scanner {
        Scanner { }
    }

    pub fn scan_token(&self, span: &mut Span) -> Token {
        span.skip_whitespace();
        span.start = span.current; // Reset the span/scanner

        if span.is_at_end() {
            return Token::new(TokenType::EOF, "", span.line);
        }

        span.advance();

        match self.try_identifier(span) {
            Some(it) => return it,
            None => {}
        };

        match self.try_number(span) {
            Some(nt) => return nt,
            None => {}
        };

        match span.peek().unwrap_or("") {
            "(" => span.new_token(TokenType::LeftParen),
            ")" => span.new_token(TokenType::RightParen),
            "{" => span.new_token(TokenType::LeftBrace),
            "}" => span.new_token(TokenType::RightBrace),
            "[" => span.new_token(TokenType::LeftBracket),
            "]" => span.new_token(TokenType::RightBracket),
            "," => span.new_token(TokenType::Comma),

            "-" => span.new_token(TokenType::Minus),
            "+" => span.new_token(TokenType::Plus),
            ";" => span.new_token(TokenType::Semicolon),
            "/" => span.new_token(TokenType::Slash),

            "\"" => self.scan_string(span),

            "*" => {
                if span.consume(">") {
                    span.new_token(TokenType::Map)
                } else {
                    span.new_token(TokenType::Star)
                }
            }
            "!" => {
                if span.consume("=") {
                    span.new_token(TokenType::NEqual)
                } else {
                    span.new_token(TokenType::Bang)
                }
            }
            "=" => {
                if span.consume("=") {
                    span.new_token(TokenType::EqEqual)
                } else {
                    span.new_token(TokenType::Equal)
                }
            }
            "<" => {
                if span.consume("=") {
                    span.new_token(TokenType::LEqual)
                } else {
                    span.new_token(TokenType::Less)
                }
            }
            ">" => {
                if span.consume("=") {
                    span.new_token(TokenType::GEqual)
                } else {
                    span.new_token(TokenType::Greater)
                }
            }
            "&" => {
                if span.consume("&") {
                    span.new_token(TokenType::AndAnd)
                } else if span.consume(">") {
                    span.new_token(TokenType::Reduce)
                } else {
                    span.err_token("expected &&")
                }
            }
            "|" => {
                if span.consume("|") {
                    span.new_token(TokenType::OrOr)
                } else if span.consume(">") {
                    span.new_token(TokenType::Pipe)
                } else {
                    span.err_token("expected || or |>")
                }
            }

            _ => {
                span.err_token("Unexpected character")
            }
        }
    }


    fn scan_string(&self, span: &mut Span) -> Token {
        // Down the road, we'll want to support interpolation, but for right now, simple string parsing is good enough
        while !span.is_at_end() && span.peek() != Some("\"") {
            if span.peek() == Some("\n") { span.line += 1; }
            span.advance();
        }
        if span.is_at_end() { return span.err_token("Unterminated string"); }
        span.advance();

        // +1 and -1 to account for the quote markers
        Token::new(TokenType::String, span.cur_lexeme(), span.line)
    }

    fn try_number(&self, span: &mut Span) -> Option<Token> {
        if span.peekc()?.is_digit(10){
            self.scan_number(span)
        } else {
            None
        }
    }

    fn scan_number(&self, span: &mut Span) -> Option<Token> {
        while span.peekc().unwrap_or('_').is_digit(10) { span.advance(); }

        if span.peek() == Some(".") && span.peeknthc(1).unwrap_or('_').is_digit(10) {
            span.advance();
            while span.peekc().unwrap_or('_').is_digit(10) { span.advance(); }
        }

        Some(span.new_token(TokenType::Number))
    }

    fn try_identifier(&self, span: &mut Span) -> Option<Token> {
        if span.peekc()?.is_alphabetic() || span.peek() == Some("_") {
            Some(self.scan_identifier(span))
        } else { None }
    }

    fn scan_identifier(&self, span: &mut Span) -> Token {
        let c = span.peekc().unwrap_or(' ');
        while c.is_alphanumeric() ||
            c.is_digit(10) ||
            c == '_' { span.advance(); }

        let token_type = self.identifier_type(span);
        span.new_token(token_type)
    }

    fn identifier_type(&self, span: &mut Span) -> TokenType {
        match span.cur_lexeme() {
            // Keywords
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "true" => TokenType::True,
            "false" => TokenType::False,
            "fn" => TokenType::FN,
            "return" => TokenType::Return,
            "puts" => TokenType::Puts,

            // Okay, just a normal identifier
            _ => TokenType::Identifier
        }
    }

}
