#[derive(Debug)]
pub enum Dependency {
    And(Vec<Dependency>),
    Or(Vec<Dependency>),
    Xor(Vec<Dependency>),
    Feature(String),
    Dependency(String),
    Flag { key: String, value: String },
    None
}

impl Dependency {
    pub fn and(self, dependency: Dependency) -> Dependency {
        match self {
            Dependency::And(mut d) => {
                d.push(dependency);
                Dependency::And(d)
            },
            Dependency::None => dependency,
            _ => Dependency::And(vec![self, dependency])
        }
    }

    pub fn or(self, dependency: Dependency) -> Dependency {
        match self {
            Dependency::Or(mut d) => {
                d.push(dependency);
                Dependency::Or(d)
            },
            Dependency::None => dependency,
            _ => Dependency::Or(vec![self, dependency])
        }
    }

    pub fn dependencies<'a>(&'a self) -> Box<dyn Iterator<Item = &str> + 'a> {
        match self {
            Dependency::And(v) => Box::new(v.iter().flat_map(|d| d.dependencies())),
            Dependency::Or(v) => Box::new(v.iter().flat_map(|d| d.dependencies())),
            Dependency::Xor(v) => Box::new(v.iter().flat_map(|d| d.dependencies())),
            Dependency::Dependency(d) => Box::new(vec![d.as_str()].into_iter()),

            Dependency::Feature(_) | 
            Dependency::Flag { .. } | 
            Dependency::None => 
                Box::new(std::iter::empty()),
        }
    }
}