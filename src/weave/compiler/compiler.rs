use crate::weave::Chunk;
use crate::weave::compiler::scanner::{Scanner, Span};
use crate::weave::compiler::token::TokenType;

pub fn compile(source: &str) -> Chunk {
    // sentinel value to ensure that the first line number gets printed
    let mut line = usize::MAX;
    let scanner = Scanner::new();
    let mut span = Span::new(source);

    loop {
        let token = scanner.scan_token(&mut span);
        if token.token_type == TokenType::EOF {
            break;
        }

        if token.line != line {
            line = token.line;
            print!("{:04} ", line);
        } else {
            print!("   | ");
        }

        println!("{}", token);
    }

    Chunk::new()
}
