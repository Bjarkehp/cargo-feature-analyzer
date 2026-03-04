use std::fmt::Display;

pub enum CrossTreeConstraint {
    Feature(String),
    And(Box<CrossTreeConstraint>, Box<CrossTreeConstraint>),
    Or(Box<CrossTreeConstraint>, Box<CrossTreeConstraint>),
    Implies(Box<CrossTreeConstraint>, Box<CrossTreeConstraint>),
    Not(Box<CrossTreeConstraint>),
}

pub fn implies(a: impl Into<CrossTreeConstraint>, b: impl Into<CrossTreeConstraint>) -> CrossTreeConstraint {
    CrossTreeConstraint::Implies(
        Box::new(a.into()), 
        Box::new(b.into()), 
    )
}

pub fn and(a: impl Into<CrossTreeConstraint>, b: impl Into<CrossTreeConstraint>) -> CrossTreeConstraint {
    CrossTreeConstraint::And(
        Box::new(a.into()), 
        Box::new(b.into()), 
    )
}

pub fn or(a: impl Into<CrossTreeConstraint>, b: impl Into<CrossTreeConstraint>) -> CrossTreeConstraint {
    CrossTreeConstraint::Or(
        Box::new(a.into()), 
        Box::new(b.into()), 
    )
}

pub fn not(constraint: impl Into<CrossTreeConstraint>) -> CrossTreeConstraint {
    CrossTreeConstraint::Not(Box::new(constraint.into()))
}

pub fn exclusive(a: impl Into<CrossTreeConstraint>, b: impl Into<CrossTreeConstraint>) -> CrossTreeConstraint {
    or(not(a.into()), not(b.into()))
}

impl From<String> for CrossTreeConstraint {
    fn from(value: String) -> Self {
        CrossTreeConstraint::Feature(value)
    }
}

impl From<&str> for CrossTreeConstraint {
    fn from(value: &str) -> Self {
        CrossTreeConstraint::Feature(value.to_owned())
    }
}

impl Display for CrossTreeConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrossTreeConstraint::Feature(name) => write!(f, "\"{name}\""),
            CrossTreeConstraint::And(a, b) => write!(f, "{a} & {b}"),
            CrossTreeConstraint::Or(a, b) => write!(f, "{a} | {b}"),
            CrossTreeConstraint::Implies(a, b) => write!(f, "{a} => {b}"),
            CrossTreeConstraint::Not(constraint) => write!(f, "!{constraint}"),
        }
    }
}