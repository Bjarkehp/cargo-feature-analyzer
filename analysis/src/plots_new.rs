use std::path::{Path, PathBuf};

use analysis::{plot::{declared_vs_fca, features_and_dependencies}, result::{feature_stats::FeatureStats, model_stats::ModelStats}};
use serde::de::DeserializeOwned;

fn main() -> anyhow::Result<()> {
    let results = PathBuf::from("data/result/2026-03-29 16:14:11.948147478");
    let plots = PathBuf::from("data/plots");

    let feature_stats = load::<FeatureStats>(results.join("feature_stats.csv"))?;
    let declared_stats = load::<ModelStats>(results.join("flat_model_stats.csv"))?;
    let fca_stats = load::<ModelStats>(results.join("fca_model_stats.csv"))?;

    features_and_dependencies::plot(&feature_stats, plots.join("features_and_dependencies.png"))?;
    declared_vs_fca::plot(&declared_stats, &fca_stats, plots.join("declared_vs_fca.png"))?;

    Ok(())
}

fn load<T: DeserializeOwned>(path: impl AsRef<Path>) -> anyhow::Result<Vec<T>> {
    let rows = csv::Reader::from_path(path)?
        .into_deserialize()
        .collect::<csv::Result<Vec<T>>>()?;
    Ok(rows)
}