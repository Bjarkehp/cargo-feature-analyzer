use std::{fmt::Display, str::FromStr};

use semver::Version;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrateId {
    pub name: String,
    pub version: Version,
}

impl CrateId {
    pub fn new(name: String, version: Version) -> Self {
        Self { name, version }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error parsing '{0}' as crate: {1}")]
    Semver(String, semver::Error),
    #[error("Error parsing '{0}' as crate: Crate is missing a version")]
    MissingVersion(String)
}

impl FromStr for CrateId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (crate_name, version_str) = s.split_once('@')
            .ok_or_else(|| Error::MissingVersion(s.to_string()))?;
        let version = version_str.parse()
            .map_err(|e| Error::Semver(s.to_string(), e))?;
        Ok(CrateId::new(crate_name.to_owned(), version))
    }
}

impl Display for CrateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}