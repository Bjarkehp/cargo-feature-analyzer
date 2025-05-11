#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Dependency<'a> {
    Feature(&'a str),
    Crate(&'a str),
}

impl<'a> Dependency<'a> {
    pub fn representation(self) -> String {
        match self {
            Dependency::Feature(s) => format!("\"{}\"", s),
            Dependency::Crate(s) => format!("\"(Crate) {}\"", s),
        }
    }

    pub fn name(self) -> &'a str {
        match self {
            Dependency::Feature(s) => s,
            Dependency::Crate(s) => s,
        }
    }
}