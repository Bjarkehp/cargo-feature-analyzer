use std::{fmt::Display, str::FromStr};

use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl<'de> Deserialize<'de> for CrateId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let content = String::deserialize(deserializer)?;
        CrateId::from_str(&content)
            .map_err(serde::de::Error::custom)
    }
}

impl Serialize for CrateId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}