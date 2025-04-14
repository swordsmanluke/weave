use std::fmt::Display;

pub enum WeaveNumber{
    Float(f64),
    Int(i64),
    UInt(u64),
}

impl From<f64> for WeaveNumber{
    fn from(value: f64) -> Self {
        WeaveNumber::Float(value)
    }
}

impl Display for WeaveNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeaveNumber::Float(n) => write!(f, "{:02}", n),
            WeaveNumber::Int(n) => write!(f, "{}", n),
            WeaveNumber::UInt(n) => write!(f, "{}", n),
        }
    }
}