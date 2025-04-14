mod weave_number;

use std::fmt::Display;
pub use weave_number::WeaveNumber;

// Our types for Weave. Detailed type information can be found in the implementation of each type
pub enum WeaveType {
    Number(WeaveNumber),
}

impl From<f64> for WeaveType {
    fn from(value: f64) -> Self {
        WeaveType::Number(value.into())
    }
}

impl Display for WeaveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeaveType::Number(n) => write!(f, "{}", n),
        }
    }
}
