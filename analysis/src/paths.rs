use std::path::PathBuf;

use analysis::config::Config;

pub struct Paths {
    pub data: PathBuf,
    pub result: PathBuf,
    pub crate_entries: PathBuf,
    pub crates: PathBuf,
    pub config: PathBuf,
    pub declared_model: PathBuf,
    pub fca_model: PathBuf,
    pub flamapy_server: PathBuf,
}

/// Ensures the relevant directories exist, 
/// and returns a Paths instance which stores all relevant paths
pub fn prepare_paths(config: &Config) -> anyhow::Result<Paths> {
    let paths = Paths {
        data: config.data.clone(),
        result: config.result.clone(),
        crate_entries: config.data.join("crates.txt"),
        crates: config.data.join("crate"),
        config: config.data.join("configuration"),
        declared_model: config.data.join("model/declared"),
        fca_model: config.data.join("model/fca_model"),
        flamapy_server: PathBuf::from("analysis/src/flamapy_server.py")
    };

    std::fs::create_dir_all(&paths.data)?;
    std::fs::create_dir_all(&paths.result)?;
    std::fs::create_dir_all(&paths.crates)?;
    std::fs::create_dir_all(&paths.declared_model)?;
    std::fs::create_dir_all(&paths.fca_model)?;

    Ok(paths)
}