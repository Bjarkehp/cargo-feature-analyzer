use std::{fmt::Display, num::ParseIntError, str::FromStr};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct CrateData {
    pub downloads: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error while parsing downloads: {0}")]
    ParseInt(ParseIntError),
}

impl FromStr for CrateData {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let downloads = s.parse()
            .map_err(Error::ParseInt)?;
        Ok(CrateData { downloads })
    }
}

impl Display for CrateData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.downloads)
    }
}