use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct SatisfiabilityRow {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "Satisfiability")]
    pub satisfiability: f64,
}