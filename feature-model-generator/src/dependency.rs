#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Dependency<'a> {
    Feature(&'a str),
    Crate(&'a str),
    Flag(&'a str, &'a str)
}

impl<'a> Dependency<'a> {
    pub fn representation(self) -> String {
        match self {
            Dependency::Feature(s) => format!("\"{}\"", s),
            Dependency::Crate(s) => format!("\"(Crate) {}\"", s),
            Dependency::Flag(s, _) => format!("\"{}\"", s)
        }
    }

    pub fn name(self) -> &'a str {
        match self {
            Dependency::Feature(s) => s,
            Dependency::Crate(s) => s,
            Dependency::Flag(s, _) => s
        }
    }
}