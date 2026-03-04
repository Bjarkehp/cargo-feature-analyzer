use std::path::PathBuf;

use anyhow::{Context, anyhow};
use cargo_toml::default_reqwest_client;
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    name: String,

    #[arg(short, long, default_value = None)]
    crate_version: Option<String>,
    #[arg(short, long, default_value = None)]
    destination: Option<PathBuf>
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let destination = args.destination.unwrap_or_else(|| {
        let parent = std::env::current_dir()
            .expect("Failed to get current directory");
        PathBuf::from(format!("{}/{}.toml", parent.display(), &args.name))
    });

    let crate_version = args.crate_version.unwrap_or_else(|| {
        let client = cargo_toml::default_cargo_client().unwrap();
        let version = cargo_toml::latest_version(&args.name, &client).unwrap();
        version.num
    });

    let client = default_reqwest_client()?;
    cargo_toml::download_and_save(&client, &args.name, &crate_version, &destination)
        .with_context(|| anyhow!("Unable to download and extract {}", args.name))?;

    Ok(())
}