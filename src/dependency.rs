#[derive(Debug)]
pub enum Dependency {
    And(Vec<Dependency>),
    Or(Vec<Dependency>),
    Xor(Vec<Dependency>),

    Feature(String),
    Crate(String),
    Flag { key: String, value: String },
}

/// Methods on Dependency are declared in a trait,
/// which enables Option<Dependency> to also implement this trait.
pub trait DependencyTrait {
    fn and(self, other: Self) -> Self;
    fn or(self, other: Self) -> Self;

    fn crates<'a>(&'a self) -> Box<dyn Iterator<Item = &str> + 'a>;
}

impl DependencyTrait for Dependency {
    fn and(self, other: Self) -> Self {
        match self {
            Dependency::And(mut d) => {
                d.push(other);
                Dependency::And(d)
            },
            _ => Dependency::And(vec![self, other])
        }
    }

    fn or(self, other: Self) -> Self {
        match self {
            Dependency::Or(mut d) => {
                d.push(other);
                Dependency::Or(d)
            },
            _ => Dependency::Or(vec![self, other])
        }
    }

    fn crates<'a>(&'a self) -> Box<dyn Iterator<Item = &str> + 'a> {
        match self {
            Dependency::And(v) => Box::new(v.iter().flat_map(|d| d.crates())),
            Dependency::Or(v) => Box::new(v.iter().flat_map(|d| d.crates())),
            Dependency::Xor(v) => Box::new(v.iter().flat_map(|d| d.crates())),
            Dependency::Crate(d) => Box::new(vec![d.as_str()].into_iter()),

            Dependency::Feature(_) | 
            Dependency::Flag { .. } => 
                Box::new(std::iter::empty()),
        }
    }
}

impl DependencyTrait for Option<Dependency> {
    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Some(l), Some(r)) => Some(l.and(r)),
            (Some(l), None) => Some(l),
            (None, Some(r)) => Some(r),
            (None, None) => None
        }
    }

    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Some(l), Some(r)) => Some(l.or(r)),
            _ => None
        }
    }

    fn crates<'a>(&'a self) -> Box<dyn Iterator<Item = &str> + 'a> {
        if let Some(d) = self {
            d.crates()
        } else {
            Box::new(std::iter::empty())
        }
    }
}