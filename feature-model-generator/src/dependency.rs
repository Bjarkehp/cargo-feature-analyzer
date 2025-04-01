#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Dependency<'a> {
    Feature(&'a str),
    Crate(&'a str),
    Flag(&'a str, &'a str)
}

impl Dependency<'_> {
    pub fn representation(self) -> String {
        match self {
            Dependency::Feature(s) => format!("\"{}\"", s),
            Dependency::Crate(s) => format!("\"(Crate) {}\"", s),
            Dependency::Flag(s, _) => format!("\"{}\"", s)
        }
    }
}