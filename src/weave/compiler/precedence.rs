
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum Precedence {
    NONE,
    ASSIGNMENT,  // =
    OR,          // or
    AND,         // and
    EQUALITY,    // == !=
    COMPARISON,  // < > <= >=
    TERM,        // + -
    FACTOR,      // * /
    UNARY,       // ! -
    CALL,        // . ()
    PRIMARY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality() {
        assert_eq!(Precedence::PRIMARY, Precedence::PRIMARY);
    }
    
    #[test]
    fn comparison() {
        assert!(Precedence::PRIMARY > Precedence::ASSIGNMENT);
    }
}