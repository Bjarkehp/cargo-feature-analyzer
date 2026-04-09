use std::path::Path;

use analysis::{args::Args, config::config_from_args, plot::{cross_tree_constraints, declared_vs_fca, default_configs, feature_stats, features_and_dependencies, line_count_and_features, unique_configs}, result::{configuration_stats::ConfigStats, feature_stats::FeatureStats, line_count::LineCountRow, model_stats::ModelStats}};
use clap::Parser;
use serde::de::DeserializeOwned;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = config_from_args(args)?;
    let result = config.result;
    let plot = config.plot;

    std::fs::create_dir_all(&plot)?;

    let feature_stats = load::<FeatureStats>(result.join("feature_stats.csv"))?;
    let declared_stats = load::<ModelStats>(result.join("flat_model_stats.csv"))?;
    let fca_stats = load::<ModelStats>(result.join("fca_model_stats.csv"))?;
    let line_count_rows = load::<LineCountRow>(result.join("line_count.csv"))?;
    let config_stats = load::<ConfigStats>(result.join("configuration_stats.csv"))?;

    feature_stats::plot(&feature_stats, plot.join("feature_stats.png"))?;
    features_and_dependencies::plot(&feature_stats, plot.join("features_and_dependencies.png"))?;
    declared_vs_fca::plot(&declared_stats, &fca_stats, plot.join("declared_vs_fca.png"))?;
    cross_tree_constraints::plot(&declared_stats, &fca_stats, plot.join("cross_tree_constraints.png"))?;
    line_count_and_features::plot(&line_count_rows, &feature_stats, plot.join("line_count_and_features.png"))?;
    default_configs::plot(&config_stats, plot.join("default_configs.png"))?;
    unique_configs::plot(&config_stats, plot.join("unique_configs.png"))?;

    Ok(())
}

fn load<T: DeserializeOwned>(path: impl AsRef<Path>) -> anyhow::Result<Vec<T>> {
    let rows = csv::Reader::from_path(path)?
        .into_deserialize()
        .collect::<csv::Result<Vec<T>>>()?;
    Ok(rows)
}