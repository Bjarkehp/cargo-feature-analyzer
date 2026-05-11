use std::{collections::BTreeMap, path::{Path, PathBuf}};

use analysis::{args::Args, config::config_from_args, paths::{self, Paths}, plot::{cross_tree_constraints, declared_vs_fca, default_configs, distinct_configs, feature_stats, features_and_dependencies, line_count_and_features, satisfiability, satisfiability_crate, satisfiability_wrt_configs, satisfiability_wrt_features}, result::{Row, configuration_stats::ConfigStats, feature_stats::FeatureStats, line_count::LineCountRow, model_stats::ModelStats, satisfiability_crate_row::SatisfiabilityCrateRow, satisfiability_row::SatisfiabilityRow}};
use anyhow::Context;
use cargo_toml::crate_id::CrateId;
use clap::Parser;
use itertools::Itertools;
use serde::de::DeserializeOwned;
use walkdir::WalkDir;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = config_from_args(args)?;
    let paths = paths::prepare_paths(&config)?;

    let feature_stats = load_map::<FeatureStats>(&paths.feature_stats)?;
    let declared_stats = load_map::<ModelStats>(&paths.static_stats)?;
    let fca_stats = load_map::<ModelStats>(&paths.fca_stats)?;
    let line_count_rows = load_map::<LineCountRow>(&paths.line_count)?;
    let config_stats = load_map::<ConfigStats>(&paths.config_stats)?;

    feature_stats::plot(&feature_stats, &paths.feature_stats_plot)?;
    features_and_dependencies::plot(&feature_stats, &paths.features_and_dependencies)?;
    declared_vs_fca::plot(&declared_stats, &fca_stats, &paths.static_vs_fca)?;
    cross_tree_constraints::plot(&declared_stats, &fca_stats, &paths.cross_tree_constraints)?;
    line_count_and_features::plot(&line_count_rows, &feature_stats, &paths.line_count_and_features)?;

    if !config_stats.is_empty() {
        default_configs::plot(&config_stats, &paths.default_configs_plot)?;
        distinct_configs::plot(&config_stats, &paths.distinct_configs_plot)?;
    }

    for path in walk_files(&paths.satisfiability_crate) {
        satisfiability_crate_result_to_plot(path, &paths)?;
    }

    for path in walk_files(&paths.satisfiability) {
        satisfiability_result_to_plot(path, &config_stats, &feature_stats, &paths)?;
    }

    Ok(())
}

fn satisfiability_crate_result_to_plot(path: impl AsRef<Path>, paths: &Paths) -> anyhow::Result<()> {
    let path = path.as_ref();
    let file_name = path
        .file_name()
        .expect("File has a name")
        .to_string_lossy();
    let file_stem = file_name
        .rsplit_once('.')
        .with_context(|| format!("Path {path:?} has no extension in name"))?
        .0;

    let config_count = path
        .parent()
        .expect("Path must have a parent")
        .file_name()
        .expect("Directory is not root")
        .to_string_lossy()
        .parse::<usize>()
        .with_context(|| format!("Directory {path:?} has a non-integer name"))?;

    let rows = load_vec::<SatisfiabilityCrateRow>(&path)?;

    let parent_plot_dir = paths.satisfiability_crate_plot.join(format!("{config_count}"));

    std::fs::create_dir_all(&parent_plot_dir)?;

    let plot_path = parent_plot_dir
        .join(file_stem)
        .with_added_extension("png");

    satisfiability_crate::plot(&rows, plot_path, file_stem)?;

    Ok(())
}

fn satisfiability_result_to_plot(path: impl AsRef<Path>, config_stats: &BTreeMap<CrateId, ConfigStats>, feature_stats: &BTreeMap<CrateId, FeatureStats>, paths: &Paths) -> anyhow::Result<()> {
    let path = path.as_ref();
    let rows = load_map::<SatisfiabilityRow>(&path)?;

    let configs = path
        .file_stem()
        .expect("File has a name")
        .to_string_lossy()
        .parse::<usize>()
        .with_context(|| format!("File {path:?} has a non-integer name"))?;

    let plot_path = paths.satisfiability_plot
        .join(format!("{configs}"))
        .with_extension("png");

    satisfiability::plot(&rows, &plot_path)?;

    let plot_path = paths.satisfiability_wrt_configs
        .join(format!("{configs}"))
        .with_extension("png");

    satisfiability_wrt_configs::plot(&rows, config_stats, &plot_path)?;

    let plot_path = paths.satisfiability_wrt_features
        .join(format!("{configs}"))
        .with_extension("png");

    satisfiability_wrt_features::plot(&rows, feature_stats, &plot_path)?;

    Ok(())
}

fn load_map<T: DeserializeOwned + Row>(path: impl AsRef<Path>) -> anyhow::Result<BTreeMap<T::Key, T>> {
    let rows = csv::Reader::from_path(path)?
        .into_deserialize()
        .map_ok(|r: T| (r.key(), r))
        .collect::<csv::Result<BTreeMap<T::Key, T>>>()?;
    Ok(rows)
}

fn load_vec<T: DeserializeOwned>(path: impl AsRef<Path>) -> anyhow::Result<Vec<T>> {
    let rows = csv::Reader::from_path(path)?
        .into_deserialize()
        .collect::<csv::Result<Vec<T>>>()?;
    Ok(rows)
}

fn walk_files(path: impl AsRef<Path>) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|e| e.is_file())
}