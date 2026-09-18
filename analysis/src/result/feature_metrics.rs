use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

use crate::impl_key_crate_id;

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct FeatureMetrics {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "LOC")]
    pub loc: usize,
    #[serde(rename = "LOF")]
    pub lof: usize,
}

impl_key_crate_id!(FeatureMetrics);