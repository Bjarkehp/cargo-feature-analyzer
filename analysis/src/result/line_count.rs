use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

use crate::impl_key_crate_id;

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct LineCountRow {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "Line count")]
    pub line_count: usize,
}

impl_key_crate_id!(LineCountRow);