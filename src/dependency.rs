#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Dependencies {
    mandatory: Vec<Dependency>,
    optional: Vec<Dependency>,
    or: Vec<Dependency>,
    alternative: Vec<Dependency>
}

impl Dependencies {
    pub fn new(mandatory: Vec<Dependency>, optional: Vec<Dependency>, or: Vec<Dependency>, alternative: Vec<Dependency>) -> Self {
        Self { mandatory, optional, or, alternative }
    }

    pub fn from_mandatory(mandatory: Vec<Dependency>) -> Self {
        Self::new(mandatory, vec![], vec![], vec![])
    }

    pub fn empty() -> Self {
        Self::new(vec![], vec![], vec![], vec![])
    }

    pub fn is_empty(&self) -> bool {
        self.mandatory.is_empty() &&
        self.optional.is_empty() &&
        self.or.is_empty() &&
        self.alternative.is_empty()
    }

    pub fn leafs(&self) -> impl Iterator<Item = &Dependency> {
        self.mandatory.iter()
            .chain(self.optional.iter())
            .chain(self.or.iter())
            .chain(self.alternative.iter())
    }

    pub fn crates(&self) -> impl Iterator<Item = &str> {
        self.mandatory.iter()
            .chain(self.optional.iter())
            .chain(self.alternative.iter())
            .filter_map(|d| match d {
                Dependency::Crate(s) => Some(s.as_str()),
                _ => None
            })
    }

    pub fn mandatory(&self) -> impl Iterator<Item = &Dependency> {
        self.mandatory.iter()
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Dependency {
    Feature(String),
    Crate(String),
    Flag(String, String)
}

impl Dependency {
    pub fn name(&self) -> String {
        match self {
            Dependency::Feature(s) => format!("\"{}\"", s),
            Dependency::Crate(s) => format!("\"{}\"", s),
            Dependency::Flag(s, _) => format!("\"{}\"", s)
        }
    }
}