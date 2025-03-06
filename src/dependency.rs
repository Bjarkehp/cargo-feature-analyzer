#[derive(Debug)]
pub enum Dependency {
    And(Vec<Dependency>),
    Or(Vec<Dependency>),
    Xor(Vec<Dependency>),
    Not(Box<Dependency>),

    Feature(String),
    Crate(String),
    Flag { key: String, value: String },
    
    None
}

impl Dependency {
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Dependency::None, r) => r,
            (l, Dependency::None) => l,
            (Dependency::And(mut d), r) => {
                d.push(r);
                Dependency::And(d)
            },
            (l, r) => Dependency::And(vec![l, r])
        }
    }

    pub fn or(self, other: Self) -> Self {
        match self {
            Dependency::Or(mut d) => {
                d.push(other);
                Dependency::Or(d)
            },
            Dependency::None => other,
            _ => Dependency::Or(vec![self, other])
        }
    }

    pub fn not(self) -> Self {
        match self {
            Dependency::Not(d) => *d,
            _ => Dependency::Not(Box::new(self))
        }
    }

    pub fn crates<'a>(&'a self) -> Box<dyn Iterator<Item = &str> + 'a> {
        match self {
            Dependency::And(v) => Box::new(v.iter().flat_map(|d| d.crates())),
            Dependency::Or(v) => Box::new(v.iter().flat_map(|d| d.crates())),
            Dependency::Xor(v) => Box::new(v.iter().flat_map(|d| d.crates())),
            Dependency::Not(d) => d.crates(),
            Dependency::Crate(d) => Box::new(vec![d.as_str()].into_iter()),

            Dependency::Feature(_) | 
            Dependency::Flag { .. } |
            Dependency::None => 
                Box::new(std::iter::empty()),
        }
    }
}