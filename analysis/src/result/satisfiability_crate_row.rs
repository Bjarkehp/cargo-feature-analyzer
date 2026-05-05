use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, derive_new::new)]
pub struct SatisfiabilityCrateRow {
    #[serde(rename = "Satisfiability")]
    pub satisfiability: f64,
}