use std::fmt::Display;

use semver::Version;

pub struct CrateId<'a> {
    pub name: &'a str,
    pub version: Version,
}

impl<'a> CrateId<'a> {
    pub fn new(name: &'a str, version: Version) -> Self {
        Self { name, version }
    }
}

impl Display for CrateId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error parsing '{0}' as crate: {1}")]
    Semver(String, semver::Error),
    #[error("Error parsing '{0}' as crate: Crate is missing a version")]
    MissingVersion(String)
}

pub fn parse(s: &str) -> Result<CrateId<'_>, Error> {
    let (crate_name, version_str) = s.split_once('@')
        .ok_or_else(|| Error::MissingVersion(s.to_string()))?;
    let version = version_str.parse()
        .map_err(|e| Error::Semver(s.to_string(), e))?;
    Ok(CrateId::new(crate_name, version))
} 
