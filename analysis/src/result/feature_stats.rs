use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct FeatureStats {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "Features")]
    pub features: usize, 
    #[serde(rename = "Feature Dependencies")]
    pub feature_dependencies: usize,
}