use std::path::PathBuf;

use crate::config::Config;

pub struct Paths {
    pub data: PathBuf,
    pub satisfiability_crate: PathBuf,
    pub satisfiability: PathBuf,
    pub crate_entries: PathBuf,
    pub crates: PathBuf,
    pub config: PathBuf,
    pub config_distinct: PathBuf,
    pub static_model: PathBuf,
    pub fca_model: PathBuf,
    pub flamapy_server: PathBuf,

    pub result: PathBuf,
    pub feature_stats: PathBuf,
    pub static_stats: PathBuf,
    pub fca_stats: PathBuf,
    pub line_count: PathBuf,
    pub config_stats: PathBuf,

    pub plot: PathBuf,
    pub feature_stats_plot: PathBuf,
    pub features_and_dependencies: PathBuf,
    pub static_vs_fca: PathBuf,
    pub cross_tree_constraints: PathBuf,
    pub line_count_and_features: PathBuf,
    pub default_configs_plot: PathBuf,
    pub distinct_configs_plot: PathBuf,
    pub satisfiability_crate_plot: PathBuf,
    pub satisfiability_plot: PathBuf,
    pub satisfiability_wrt_configs: PathBuf,
    pub satisfiability_wrt_features: PathBuf,
}

/// Ensures the relevant directories exist, 
/// and returns a Paths instance which stores all relevant paths
pub fn prepare_paths(config: &Config) -> anyhow::Result<Paths> {
    let paths = Paths {
        data: config.data.clone(),
        satisfiability_crate: config.result.join("satisfiability_crate"),
        satisfiability: config.result.join("satisfiability"),
        crate_entries: config.data.join("crates.csv"),
        crates: config.data.join("crate"),
        config: config.data.join("configuration"),
        config_distinct: config.data.join("configuration_distinct"),
        static_model: config.data.join("model/declared"),
        fca_model: config.data.join("model/fca"),
        flamapy_server: PathBuf::from("analysis/src/flamapy_server.py"),

        result: config.result.clone(),
        feature_stats: config.result.join("feature_stats.csv"),
        static_stats: config.result.join("flat_model_stats.csv"),
        fca_stats: config.result.join("fca_model_stats.csv"),
        line_count: config.result.join("line_count.csv"),
        config_stats: config.result.join("configuration_stats.csv"),

        plot: config.plot.clone(),
        feature_stats_plot: config.plot.join("feature_stats.png"),
        features_and_dependencies: config.plot.join("features_and_dependencies.png"),
        static_vs_fca: config.plot.join("declared_vs_fca.png"),
        cross_tree_constraints: config.plot.join("cross_tree_constraints.png"),
        line_count_and_features: config.plot.join("line_count_and_features.png"),
        default_configs_plot: config.plot.join("default_configs.png"),
        distinct_configs_plot: config.plot.join("unique_configs.png"),
        satisfiability_crate_plot: config.plot.join("satisfiability_crate"),
        satisfiability_plot: config.plot.join("satisfiability"),
        satisfiability_wrt_configs: config.plot.join("satisfiability_wrt_configs"),
        satisfiability_wrt_features: config.plot.join("satisfiability_wrt_features"),
    };

    std::fs::create_dir_all(&paths.data)?;
    std::fs::create_dir_all(&paths.result)?;
    std::fs::create_dir_all(&paths.satisfiability_crate)?;
    std::fs::create_dir_all(&paths.satisfiability)?;
    std::fs::create_dir_all(&paths.crates)?;
    std::fs::create_dir_all(&paths.config)?;
    std::fs::create_dir_all(&paths.config_distinct)?;
    std::fs::create_dir_all(&paths.static_model)?;
    std::fs::create_dir_all(&paths.fca_model)?;

    std::fs::create_dir_all(&paths.plot)?;
    std::fs::create_dir_all(&paths.satisfiability_crate_plot)?;
    std::fs::create_dir_all(&paths.satisfiability_plot)?;
    std::fs::create_dir_all(&paths.satisfiability_wrt_configs)?;
    std::fs::create_dir_all(&paths.satisfiability_wrt_features)?;

    Ok(paths)
}