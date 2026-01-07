use anyhow::Context;

pub const CRATE_ENTRIES: &str = "data/crates.txt";
pub const TOML: &str = "data/toml";
pub const CRATE: &str = "data/crate";
pub const CONFIG: &str = "data/configuration";
pub const FLAT_MODEL: &str = "data/model/flat";
pub const FCA_MODEL: &str = "data/model/fca";
pub const RESULT_ROOT: &str = "data/result";
pub const PLOT_ROOT: &str = "data/plot";

pub const FLAMAPY_SERVER: &str = "analysis/src/flamapy_server.py";

pub fn prepare_directories() -> anyhow::Result<()> {
    for path in [TOML, CONFIG, FLAT_MODEL, FCA_MODEL, RESULT_ROOT, PLOT_ROOT] {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory {path}"))?;
    }

    Ok(())
}