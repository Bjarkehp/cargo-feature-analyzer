use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

use crate::impl_key_crate_id;

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct FeatureStats {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "Features")]
    pub features: usize, 
    #[serde(rename = "Feature dependencies")]
    pub feature_dependencies: usize,
}

impl_key_crate_id!(FeatureStats);