use std::ops::Add;

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

impl Precedence {
    pub fn next(&self) -> Precedence {
        use Precedence::*;
        match self {
            NONE => ASSIGNMENT,
            ASSIGNMENT => OR,
            OR => AND,
            AND => EQUALITY,
            EQUALITY => COMPARISON,
            COMPARISON => TERM,
            TERM => FACTOR,
            FACTOR => UNARY,
            UNARY => CALL,
            CALL => PRIMARY,
            PRIMARY => PRIMARY,
        }
    }
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