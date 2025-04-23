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
            TokenType::Equal => ParseRule::new(),
            TokenType::Comma => ParseRule::new(),
            TokenType::Semicolon => ParseRule::new(),
            TokenType::Bang => ParseRuleBuilder::p_none().prefix(Compiler::unary).rule,
            TokenType::NEqual => ParseRuleBuilder::p_equality().infix(Compiler::binary).rule,
            TokenType::EqEqual => ParseRuleBuilder::p_equality().infix(Compiler::binary).rule,
            TokenType::Greater => ParseRuleBuilder::p_comparison().infix(Compiler::binary).rule,
            TokenType::GEqual => ParseRuleBuilder::p_comparison().infix(Compiler::binary).rule,
            TokenType::Less => ParseRuleBuilder::p_comparison().infix(Compiler::binary).rule,
            TokenType::LEqual => ParseRuleBuilder::p_comparison().infix(Compiler::binary).rule,

            // Term
            TokenType::Minus => ParseRuleBuilder::p_term().prefix(Compiler::unary).infix(Compiler::binary).rule,
            TokenType::Plus => ParseRuleBuilder::p_term().infix(Compiler::binary).rule,

            // Product
            TokenType::Slash => ParseRuleBuilder::p_factor().infix(Compiler::binary).rule,
            TokenType::Star => ParseRuleBuilder::p_factor().infix(Compiler::binary).rule,

            // Literals
            TokenType::True => ParseRuleBuilder::p_none().prefix(Compiler::literal).rule,
            TokenType::False => ParseRuleBuilder::p_none().prefix(Compiler::literal).rule,
            TokenType::Number => ParseRuleBuilder::p_none().prefix(Compiler::number).rule,
            TokenType::String => ParseRuleBuilder::p_none().prefix(Compiler::string).rule,
            TokenType::Identifier => ParseRuleBuilder::p_none().prefix(Compiler::variable).rule,

            // TODO
            TokenType::AndAnd => ParseRule::new(),
            TokenType::OrOr => ParseRule::new(),
            TokenType::Pipe => ParseRule::new(),
            TokenType::Map => ParseRule::new(),
            TokenType::Reduce => ParseRule::new(),
            TokenType::Container => ParseRule::new(),
            TokenType::If => ParseRule::new(),
            TokenType::Else => ParseRule::new(),
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