use std::path::PathBuf;

use analysis::config::Config;

pub struct Paths {
    pub data: PathBuf,
    pub result: PathBuf,
    pub satisfiability_crate: PathBuf,
    pub satisfiability: PathBuf,
    pub crate_entries: PathBuf,
    pub crates: PathBuf,
    pub config: PathBuf,
    pub config_distinct: PathBuf,
    pub static_model: PathBuf,
    pub fca_model: PathBuf,
    pub flamapy_server: PathBuf,
}

/// Ensures the relevant directories exist, 
/// and returns a Paths instance which stores all relevant paths
pub fn prepare_paths(config: &Config) -> anyhow::Result<Paths> {
    let paths = Paths {
        data: config.data.clone(),
        result: config.result.clone(),
        satisfiability_crate: config.result.join("satisfiability_crate"),
        satisfiability: config.result.join("satisfiability"),
        crate_entries: config.data.join("crates.csv"),
        crates: config.data.join("crate"),
        config: config.data.join("configuration"),
        config_distinct: config.data.join("configuration_distinct"),
        static_model: config.data.join("model/declared"),
        fca_model: config.data.join("model/fca"),
        flamapy_server: PathBuf::from("analysis/src/flamapy_server.py")
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

    Ok(paths)
}