use std::fmt::Display;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Add;

#[derive(Clone, Debug)]
pub struct WeaveString {
    hashcode: u64,
    value: Box<String>
}

fn hash_str(key: &str) -> u64 {
    let mut s = DefaultHasher::new();
    key.hash(&mut s);
    s.finish()
}

impl WeaveString {
    pub fn new(value: String) -> Self {
        WeaveString {
            hashcode: hash_str(&value),
            value: Box::new(value)
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
    
    pub fn len(&self) -> usize {
        self.value.len()
    }
    
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl PartialEq for WeaveString {
    fn eq(&self, other: &Self) -> bool {
        // TODO: Intern strings to improve performance?
        self.hashcode == other.hashcode && self.value == other.value
    }
}

impl Display for WeaveString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl PartialOrd for WeaveString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl Add for &WeaveString {
    type Output = WeaveString;

    fn add(self, rhs: Self) -> Self::Output {
        WeaveString::new(format!("{}{}", self.value, rhs.value))
    }
}

impl Add for WeaveString {
    type Output = WeaveString;

    fn add(self, rhs: Self) -> Self::Output {
        WeaveString::new(format!("{}{}", self.value, rhs.value))
    }
}

impl From<String> for WeaveString {
    fn from(value: String) -> Self {
        WeaveString::new(value)
    }
}

impl From<&str> for WeaveString {
    fn from(value: &str) -> Self {
        WeaveString::new(value.to_string())
    }
}