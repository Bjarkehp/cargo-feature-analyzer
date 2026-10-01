use cargo_toml::crate_id::CrateId;
use serde::{Deserialize, Serialize};

use crate::{impl_key_crate_id, scanner::ScanResult};

#[derive(Debug, Serialize, Deserialize)]
pub struct FeatureMetrics {
    #[serde(rename = "Crate")]
    pub crate_id: CrateId,
    #[serde(rename = "LOC")]
    pub loc: usize,
    #[serde(rename = "LOF")]
    pub lof: usize,
    #[serde(rename = "SD median")]
    pub sd_median: f64,
    #[serde(rename = "SD mean")]
    pub sd_mean: f64,
    #[serde(rename = "SD deviation")]
    pub sd_dev: f64,
    #[serde(rename = "TD median")]
    pub td_median: f64,
    #[serde(rename = "TD mean")]
    pub td_mean: f64,
    #[serde(rename = "TD deviation")]
    pub td_dev: f64,
    #[serde(rename = "AND median")]
    pub and_median: f64,
    #[serde(rename = "AND mean")]
    pub and_mean: f64,
    #[serde(rename = "AND deviation")]
    pub and_dev: f64,
}

impl FeatureMetrics {
    pub fn from_crate_scan(crate_id: CrateId, scan: ScanResult) -> Self {
        Self {
            crate_id,
            loc: scan.loc,
            lof: scan.lof,
            sd_median: scan.sd_median,
            sd_mean: scan.sd_mean,
            sd_dev: scan.sd_dev,
            td_median: scan.td_median,
            td_mean: scan.td_mean,
            td_dev: scan.td_dev,
            and_median: scan.and_median,
            and_mean: scan.and_mean,
            and_dev: scan.and_dev,
        }
    }
}

impl_key_crate_id!(FeatureMetrics);