use std::path::PathBuf;

use clap::Parser;

pub mod cli;
pub mod cargo_toml;
pub mod crates_io;
pub mod flamapy;
 
fn main() {
    match cli::Args::parse().command {
        cli::Command::CargoToml { name, crate_version, destination } => 
            cargo_toml(name, crate_version, destination),
    }
}

pub fn cargo_toml(name: String, version: Option<String>, destination: Option<PathBuf>) {
    let destination = destination.unwrap_or_else(|| {
        std::env::current_dir().expect("Failed to get current directory")
    });

    let crate_version = version.unwrap_or_else(|| {
        let client = crates_io::default_client().unwrap();
        let version = cargo_toml::latest_version(&name, &client).unwrap();
        version.num
    });

    let content = cargo_toml::download(&name, &crate_version).unwrap();
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|_| panic!("Failed to create directories for {destination:?}"))
    }
    std::fs::write(&destination, content)
        .unwrap_or_else(|_| panic!("Failed to write the content of {destination:?}"));
}