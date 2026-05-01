use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

use crate::impl_key_crate_id;

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct ConfigStats {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "Configurations")]
    pub configuration_count: usize,
    #[serde(rename = "Default Configurations")]
    pub default_configuration_count: usize,
    #[serde(rename = "Unique Configurations")]
    pub distinct_configuration_count: usize,
}

impl_key_crate_id!(ConfigStats);