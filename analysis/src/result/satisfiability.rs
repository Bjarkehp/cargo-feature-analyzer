use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

use crate::impl_key_crate_id;

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct SatisfiabilityRow {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "Satisfiability")]
    pub satisfiability: f64,
}

impl_key_crate_id!(SatisfiabilityRow);