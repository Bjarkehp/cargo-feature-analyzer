use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

use crate::impl_key_crate_id;

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct RunningTimeStatic {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "Total time")]
    pub total_time: f32,
}

impl_key_crate_id!(RunningTimeStatic);