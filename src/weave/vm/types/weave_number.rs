use std::fmt::Display;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone)]
pub enum WeaveNumber{
    Float(f64),
    Int(i64),
    UInt(u64),
}

impl WeaveNumber {
    pub fn to_shared_type(&self, rhs: &WeaveNumber) -> Self {
        match (self, rhs) {
            (WeaveNumber::UInt(a), WeaveNumber::Int(_)) => WeaveNumber::Int(*a as i64),
            (WeaveNumber::UInt(a), WeaveNumber::Float(_)) => WeaveNumber::Float(*a as f64),
            (WeaveNumber::Int(a), WeaveNumber::Float(_)) => WeaveNumber::Float(*a as f64),
            _ => self.clone(),
        }
    }
}

impl From<f64> for WeaveNumber{
    fn from(value: f64) -> Self {
        WeaveNumber::Float(value)
    }
}

impl From<u64> for WeaveNumber{
    fn from(value: u64) -> Self {
        WeaveNumber::UInt(value)
    }
}

impl From<i64> for WeaveNumber{
    fn from(value: i64) -> Self {
        WeaveNumber::Int(value)
    }
}

impl Display for WeaveNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeaveNumber::Float(n) => write!(f, "{:.2}", n),
            WeaveNumber::Int(n) => write!(f, "{}", n),
            WeaveNumber::UInt(n) => write!(f, "{}", n),
        }
    }
}

impl PartialEq for WeaveNumber {
    fn eq(&self, rhs: &Self) -> bool {
        let a = self.to_shared_type(&rhs);
        let b = rhs.to_shared_type(&self);

        match (&a, &b) {
            (WeaveNumber::UInt(a), WeaveNumber::UInt(b)) => a == b,
            (WeaveNumber::Int(a), WeaveNumber::Int(b)) => a == b,
            (WeaveNumber::Float(a), WeaveNumber::Float(b)) => a == b,
            _ => false
        }
    }
}

impl Neg for WeaveNumber {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            WeaveNumber::Float(x) => { WeaveNumber::Float(-x) }
            WeaveNumber::Int(i) => { WeaveNumber::Int(-i) }
            WeaveNumber::UInt(u) => { WeaveNumber::Int(-(u as i64)) }
        }
    }
}

impl Add for &WeaveNumber {
    type Output = WeaveNumber;

    fn add(self, rhs: Self) -> Self::Output {
        let a = self.to_shared_type(&rhs);
        let b = rhs.to_shared_type(&self);

        match (&a, &b) {
            (WeaveNumber::UInt(a), WeaveNumber::UInt(b)) => WeaveNumber::UInt(a + b),
            (WeaveNumber::Int(a), WeaveNumber::Int(b)) => WeaveNumber::Int(a + b),
            (WeaveNumber::Float(a), WeaveNumber::Float(b)) => WeaveNumber::Float(a + b),
            _ => unreachable!("Can't add {} and {} - but you shouldn't be here", a, b),
        }
    }
}

impl Sub for &WeaveNumber {
    type Output = WeaveNumber;

    fn sub(self, rhs: Self) -> Self::Output {
        let a = self.to_shared_type(&rhs);
        let b = rhs.to_shared_type(&self);
        
        println!("{:?} - {:?}", a, b);

        match (&a, &b) {
            (WeaveNumber::UInt(a), WeaveNumber::UInt(b)) => WeaveNumber::UInt(a - b),
            (WeaveNumber::Int(a), WeaveNumber::Int(b)) => WeaveNumber::Int(a - b),
            (WeaveNumber::Float(a), WeaveNumber::Float(b)) => WeaveNumber::Float(a - b),
            _ => unreachable!("Can't subtract {} and {} - but you shouldn't be here", a, b),
        }
    }
}

impl Mul for &WeaveNumber {
    type Output = WeaveNumber;

    fn mul(self, rhs: Self) -> Self::Output {
        let a = self.to_shared_type(&rhs);
        let b = rhs.to_shared_type(&self);

        match (&a, &b) {
            (WeaveNumber::UInt(a), WeaveNumber::UInt(b)) => WeaveNumber::UInt(a * b),
            (WeaveNumber::Int(a), WeaveNumber::Int(b)) => WeaveNumber::Int(a * b),
            (WeaveNumber::Float(a), WeaveNumber::Float(b)) => WeaveNumber::Float(a * b),
            _ => unreachable!("Can't multiply {} and {} - but you shouldn't be here", a, b),
        }
    }
}

impl Div for &WeaveNumber {
    type Output = WeaveNumber;

    fn div(self, rhs: Self) -> Self::Output {
        let a = self.to_shared_type(&rhs);
        let b = rhs.to_shared_type(&self);

        match (&a, &b) {
            (WeaveNumber::UInt(a), WeaveNumber::UInt(b)) => WeaveNumber::UInt(a / b),
            (WeaveNumber::Int(a), WeaveNumber::Int(b)) => WeaveNumber::Int(a / b),
            (WeaveNumber::Float(a), WeaveNumber::Float(b)) => WeaveNumber::Float(a / b),
            _ => unreachable!("Can't divide {} and {} - but you shouldn't be here", a, b),
        }
    }
}
