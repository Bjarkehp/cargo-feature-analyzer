use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct Runningtime {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "AC-Poset time")]
    pub ac_poset_time: f32,
    #[serde(rename = "Tree constraints")]
    pub tree_constraints_time: f32,
    #[serde(rename = "Group time")]
    pub group_time: f32, 
    #[serde(rename = "Total time")]
    pub total_time: f32,
}