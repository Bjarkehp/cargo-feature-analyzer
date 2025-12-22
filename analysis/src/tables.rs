use std::collections::BTreeMap;
use std::path::Path;
use std::io::Write;

use cargo_toml::crate_id::CrateId;

use crate::{ConfigStats, ModelConfigurationStats, csv};

pub fn write_feature_stats(dir: impl AsRef<Path>, data: &BTreeMap<&CrateId, (usize, usize)>) -> anyhow::Result<()> {
    csv::write(
        &dir.as_ref().join("feature_stats.csv"), 
        data.iter(), 
        &["Crate", "Features", "Feature dependencies"], 
        |writer, (id, (features, dependencies))| writeln!(writer, "{id},{features},{dependencies}")
    )
}

pub fn write_configuration_stats(dir: impl AsRef<Path>, data: &BTreeMap<&CrateId, ConfigStats>) -> anyhow::Result<()> {
    csv::write(
        &dir.as_ref().join("configuration_stats.csv"), 
        data.iter(), 
        &["Crate", "Configurations", "Default configurations", "Unique Configurations"], 
        |writer, (&id, stats)| writeln!(writer, "{id},{},{},{}", 
            stats.configuration_count, 
            stats.default_configurations_count,
            stats.unique_configurations_count,
        )
    )
}

pub fn write_flat_model_config_stats(dir: impl AsRef<Path>, data: &BTreeMap<&CrateId, ModelConfigurationStats>) -> anyhow::Result<()> {
    csv::write(
        &dir.as_ref().join("flat_model_config_stats.csv"), 
        data.iter(), 
        &["Crate", "Estimation", "Exact"], 
        |writer, (&id, stats)| writeln!(writer, "{id},{},{}",
            stats.estimation,
            stats.exact,
        )
    )
}

pub fn write_fca_model_config_stats(dir: impl AsRef<Path>, data: &BTreeMap<&CrateId, ModelConfigurationStats>) -> anyhow::Result<()> {
    csv::write(
        &dir.as_ref().join("fca_model_config_stats.csv"), 
        data.iter(), 
        &["Crate", "Estimation", "Exact"], 
        |writer, (&id, stats)| writeln!(writer, "{id},{},{}",
            stats.estimation,
            stats.exact,
        )
    )
}

pub fn write_fca_model_quality(dir: impl AsRef<Path>, data: &BTreeMap<&CrateId, f64>) -> anyhow::Result<()> {
    csv::write(
        &dir.as_ref().join("fca_model_quality.csv"), 
        data.iter(), 
        &["Crate", "Quality"], 
        |writer, (&id, quality)| writeln!(writer, "{id},{quality}")
    )
}