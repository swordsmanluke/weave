use crate::weave::compiler::Compiler;
use crate::weave::compiler::precedence::Precedence;
use crate::weave::compiler::token::TokenType;

pub struct ParseRule {
    pub prefix: Option<fn(&mut Compiler) -> ()>,
    pub infix: Option<fn(&mut Compiler) -> ()>,
    pub precedence: Precedence,
}

impl ParseRule {
    pub fn new() -> ParseRule {
        ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::NONE,
        }
    }
    
    pub fn for_token(token_type: TokenType) -> ParseRule {
        match token_type {
            // No precedence
            TokenType::LeftParen => ParseRuleBuilder::p_none().prefix(Compiler::grouping).rule,
            TokenType::RightParen => ParseRule::new(),
            TokenType::LeftBrace => ParseRule::new(),
            TokenType::RightBrace => ParseRule::new(),
            TokenType::LeftBracket => ParseRule::new(),
            TokenType::RightBracket => ParseRule::new(),
            TokenType::Comma => ParseRule::new(),
            TokenType::Semicolon => ParseRule::new(),
            TokenType::Bang => ParseRule::new(),
            TokenType::NEqual => ParseRule::new(),
            TokenType::Equal => ParseRule::new(),
            TokenType::EqEqual => ParseRule::new(),
            TokenType::Greater => ParseRule::new(),
            TokenType::GEqual => ParseRule::new(),
            TokenType::Less => ParseRule::new(),
            TokenType::LEqual => ParseRule::new(),

            // Term
            TokenType::Minus => ParseRuleBuilder::p_term().prefix(Compiler::unary).infix(Compiler::binary).rule,
            TokenType::Plus => ParseRuleBuilder::p_term().infix(Compiler::binary).rule,

            // Product
            TokenType::Slash => ParseRuleBuilder::p_factor().infix(Compiler::binary).rule,
            TokenType::Star => ParseRuleBuilder::p_factor().infix(Compiler::binary).rule,

            // Literal
            TokenType::Number => ParseRuleBuilder::p_none().prefix(Compiler::number).rule,

            // TODO
            TokenType::AndAnd => ParseRule::new(),
            TokenType::OrOr => ParseRule::new(),

            TokenType::Pipe => ParseRule::new(),
            TokenType::Map => ParseRule::new(),
            TokenType::Reduce => ParseRule::new(),
            TokenType::Identifier => ParseRule::new(),
            TokenType::String => ParseRule::new(),
            
            TokenType::Container => ParseRule::new(),
            TokenType::If => ParseRule::new(),
            TokenType::Else => ParseRule::new(),
            TokenType::True => ParseRule::new(),
            TokenType::False => ParseRule::new(),
            TokenType::FN => ParseRule::new(),
            TokenType::Return => ParseRule::new(),
            TokenType::Puts => ParseRule::new(),
            TokenType::ERROR => ParseRule::new(),
            TokenType::EOF => ParseRule::new(),
        }
        
    }
}

struct ParseRuleBuilder {
    rule: ParseRule
}

impl ParseRuleBuilder {
    pub fn new() -> ParseRuleBuilder {
        ParseRuleBuilder {
            rule: ParseRule::new()
        }
    }
    
    pub fn prefix(mut self, prefix: fn(&mut Compiler) -> ()) -> ParseRuleBuilder {
        self.rule.prefix = Some(prefix);
        self
    }
    
    pub fn infix(mut self, infix: fn(&mut Compiler) -> ()) -> ParseRuleBuilder {
        self.rule.infix = Some(infix);
        self
    }
    
    pub fn precedence(mut self, precedence: Precedence) -> ParseRuleBuilder {
        self.rule.precedence = precedence;
        self
    }
    
    pub fn p_none() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::NONE)
    }
    
    pub fn p_assignment() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::ASSIGNMENT)
    }
    
    pub fn p_or() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::OR)
    }
    
    pub fn p_and() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::AND)
    }
    
    pub fn p_equality() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::EQUALITY)
    }
    
    pub fn p_comparison() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::COMPARISON)
    }
    
    pub fn p_term() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::TERM)
    }
    
    pub fn p_factor() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::FACTOR)
    }
    
    pub fn p_unary() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::UNARY)
    }
    
    pub fn p_call() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::CALL)
    }
    
    pub fn p_primary() -> ParseRuleBuilder {
        Self::new().precedence(Precedence::PRIMARY)
    }
}