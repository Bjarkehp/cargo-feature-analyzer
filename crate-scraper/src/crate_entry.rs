use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateEntry {
    pub id: CrateId,
    pub crate_type: CrateType,
    pub downloads: i64,
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub enum CrateType {
    Binary,
    Library,
    Both
}

impl CrateEntry {
    pub fn new(id: CrateId, crate_type: CrateType, downloads: i64) -> Self {
        CrateEntry { id, crate_type, downloads }
    }
}