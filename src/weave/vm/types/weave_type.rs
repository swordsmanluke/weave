
use std::fmt::Display;
use std::ops::{Add, Div, Mul, Neg, Sub};
use crate::weave::vm::types::weave_number::WeaveNumber;
use crate::weave::vm::types::errors::OpResult;

// Our types for Weave. Detailed type information can be found in the implementation of each type
#[derive(Debug, Clone)]
pub enum WeaveType {
    None,
    Boolean(bool),
    Number(WeaveNumber),
}

impl From<f64> for WeaveType {
    fn from(value: f64) -> Self {
        WeaveType::Number(value.into())
    }
}

impl From<i32> for WeaveType {
    fn from(value: i32) -> Self {
        WeaveType::Number((value as i64).into())
    }
}

impl From<i64> for WeaveType {
    fn from(value: i64) -> Self {
        WeaveType::Number(value.into())
    }
}

impl From<u64> for WeaveType {
    fn from(value: u64) -> Self {
        WeaveType::Number(value.into())
    }
}

impl Display for WeaveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeaveType::Number(n) => write!(f, "{}", n),
            WeaveType::Boolean(b) => write!(f, "{}", b),
            WeaveType::None => {write!(f, "")}
        }
    }
}

impl PartialEq for WeaveType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (WeaveType::Number(a), WeaveType::Number(b)) => a == b,
            (WeaveType::Boolean(a), WeaveType::Boolean(b)) => a == b,
            _ => false
        }
    }
}

impl Neg for WeaveType {
    type Output = OpResult;

    fn neg(self) -> Self::Output {
        match self {
            WeaveType::Number(n) => { Ok(WeaveType::Number(-n)) }
            _ => Err(format!("Cannot negate '{self}'"))
        }
    }
}

impl Add for WeaveType {
    type Output = OpResult;

    fn add(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (WeaveType::Number(a), WeaveType::Number(b)) => Ok(WeaveType::Number(a + b)),
            _ => Err(format!("Cannot add '{}' and '{}'", self, rhs))
        }
    }
}

impl Sub for WeaveType {
    type Output = OpResult;

    fn sub(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (WeaveType::Number(a), WeaveType::Number(b)) => Ok(WeaveType::Number(a - b)),
            _ => Err(format!("Cannot subtract '{}' and '{}'", self, rhs))
        }
    }
}

impl Mul for WeaveType {
    type Output = OpResult;

    fn mul(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (WeaveType::Number(a), WeaveType::Number(b)) => Ok(WeaveType::Number(a * b)),
            _ => Err(format!("Cannot multiply '{}' and '{}'", self, rhs))
        }
    }
}

impl Div for WeaveType {
    type Output = OpResult;

    fn div(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (WeaveType::Number(a), WeaveType::Number(b)) => Ok(WeaveType::Number(a / b)),
            _ => Err(format!("Cannot divide '{}' and '{}'", self, rhs))
        }
    }
}
