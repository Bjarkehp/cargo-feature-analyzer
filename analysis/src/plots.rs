use std::{collections::BTreeMap, path::Path};

use analysis::{args::Args, config::config_from_args, plot::{cross_tree_constraints, declared_vs_fca, default_configs, distinct_configs, feature_stats, features_and_dependencies, line_count_and_features, satisfiability, satisfiability_crate, satisfiability_wrt_configs, satisfiability_wrt_features}, result::{Row, configuration_stats::ConfigStats, feature_stats::FeatureStats, line_count::LineCountRow, model_stats::ModelStats, satisfiability_crate_row::SatisfiabilityCrateRow, satisfiability_row::SatisfiabilityRow}};
use anyhow::Context;
use clap::Parser;
use itertools::Itertools;
use serde::de::DeserializeOwned;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = config_from_args(args)?;
    let result = config.result;
    let plot = config.plot;
    
    let satisfiability_crate_result = result.join("satisfiability_crate");
    let satisfiability_crate_plot = plot.join("satisfiability_crate");
    let satisfiability_result = result.join("satisfiability");
    let satisfiability_plot = plot.join("satisfiability");
    let satisfiability_wrt_configs = plot.join("satisfiability_wrt_configs");
    let satisfiability_wrt_features = plot.join("satisfiability_wrt_features");

    std::fs::create_dir_all(&plot)?;
    std::fs::create_dir_all(&satisfiability_plot)?;
    std::fs::create_dir_all(&satisfiability_crate_plot)?;
    std::fs::create_dir_all(&satisfiability_wrt_configs)?;
    std::fs::create_dir_all(&satisfiability_wrt_features)?;

    let feature_stats = load_map::<FeatureStats>(result.join("feature_stats.csv"))?;
    let declared_stats = load_map::<ModelStats>(result.join("flat_model_stats.csv"))?;
    let fca_stats = load_map::<ModelStats>(result.join("fca_model_stats.csv"))?;
    let line_count_rows = load_map::<LineCountRow>(result.join("line_count.csv"))?;
    let config_stats = load_map::<ConfigStats>(result.join("configuration_stats.csv"))?;

    feature_stats::plot(&feature_stats, plot.join("feature_stats.png"))?;
    features_and_dependencies::plot(&feature_stats, plot.join("features_and_dependencies.png"))?;
    declared_vs_fca::plot(&declared_stats, &fca_stats, plot.join("declared_vs_fca.png"))?;
    cross_tree_constraints::plot(&declared_stats, &fca_stats, plot.join("cross_tree_constraints.png"))?;
    line_count_and_features::plot(&line_count_rows, &feature_stats, plot.join("line_count_and_features.png"))?;

    if !config_stats.is_empty() {
        default_configs::plot(&config_stats, plot.join("default_configs.png"))?;
        distinct_configs::plot(&config_stats, plot.join("unique_configs.png"))?;
    }

    

    let satisfiability_crate_table_results = std::fs::read_dir(&satisfiability_crate_result)
        .with_context(|| "Cannot find directory for satisfiability tables")?
        .map_ok(|e| e.path())
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| "Error while listing entries in satisfiability_crate directory")?
        .into_iter()
        .map(std::fs::read_dir)
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .map_ok(|e| e.path())
        .collect::<Result<Vec<_>, _>>()?;

    for path in satisfiability_crate_table_results {
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

        let parent_plot_dir = satisfiability_crate_plot.join(format!("{config_count}"));

        std::fs::create_dir_all(&parent_plot_dir)?;

        let plot_path = parent_plot_dir
            .join(file_stem)
            .with_added_extension("png");

        satisfiability_crate::plot(&rows, plot_path, file_stem)?;
    }

    let satisfiability_table_paths = std::fs::read_dir(&satisfiability_result)
        .with_context(|| "Cannot find directory for satisfiability tables")?
        .map_ok(|e| e.path())
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| "Error while listing entries in satisfiability directory")?;

    for path in satisfiability_table_paths {
        let rows = load_map::<SatisfiabilityRow>(&path)?;

        let configs = path.file_stem()
            .expect("File has a name")
            .to_string_lossy()
            .parse::<usize>()
            .with_context(|| format!("File {path:?} has a non-integer name"))?;

        let plot_path = satisfiability_plot
            .join(format!("{configs}"))
            .with_extension("png");

        satisfiability::plot(&rows, &plot_path)?;

        let plot_path = satisfiability_wrt_configs
            .join(format!("{configs}"))
            .with_extension("png");

        satisfiability_wrt_configs::plot(&rows, &config_stats, &plot_path)?;

        let plot_path = satisfiability_wrt_features
            .join(format!("{configs}"))
            .with_extension("png");

        satisfiability_wrt_features::plot(&rows, &feature_stats, &plot_path)?;
    }

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