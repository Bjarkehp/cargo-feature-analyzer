use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

use crate::impl_key_crate_id;

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct ModelStats {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "Features")]
    pub features: usize,
    #[serde(rename = "Cross-tree constraints")]
    pub cross_tree_constraints: usize,
    #[serde(rename = "Estimated configurations")]
    pub config_estimation: f64,
    #[serde(rename = "Exact configurations")]
    pub config_exact: f64,
}

impl_key_crate_id!(ModelStats);