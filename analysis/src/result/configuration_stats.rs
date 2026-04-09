use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct ConfigStats {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "Configurations")]
    pub configuration_count: usize,
    #[serde(rename = "Default Configurations")]
    pub default_configuration_count: usize,
    #[serde(rename = "Unique Configurations")]
    pub unique_configuration_count: usize,
}