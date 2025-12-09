use std::{fmt::Display, str::FromStr};

use cargo_toml::crate_id::{self, CrateId};

use crate::crate_data::{self, CrateData};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct CrateEntry {
    pub id: CrateId,
    pub data: CrateData,
}

impl CrateEntry {
    pub fn new(id: CrateId, data: CrateData) -> Self {
        CrateEntry { id, data }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error while parsing crate entry: {0}")]
    ParseId(#[from] crate_id::Error),
    #[error("Error while parsing crate entry: {0}")]
    ParseData(#[from] crate_data::Error),
    #[error("Error wahile parsing crate entry: No ':' found")]
    Split,
}

impl FromStr for CrateEntry {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (id_str, data_str) = s.split_once(':')
            .ok_or(Error::Split)?;
        let id = id_str.parse()?;
        let data = data_str.parse()?;
        Ok(CrateEntry { id, data })
    }
}

impl Display for CrateEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.id, self.data)
    }
}