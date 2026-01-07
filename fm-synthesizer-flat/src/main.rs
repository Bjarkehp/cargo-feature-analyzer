use std::{fs::File, io::BufWriter, path::PathBuf};

use anyhow::{Context, anyhow, bail};
use clap::Parser;
use fm_synthesizer_flat::write_uvl;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    destination: PathBuf,

    #[arg(short, long, default_value = None)]
    name: Option<String>,
    #[arg(short, long, default_value = None)]
    path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let cargo_toml_content = if let Some(name) = args.name {
        let cargo_client = cargo_toml::default_cargo_client()?;
        let reqwest_client = cargo_toml::default_reqwest_client()?;
        let version = cargo_toml::latest_version(&name, &cargo_client)?;
        cargo_toml::download_cargo_toml(&reqwest_client, &name, &version.num)?
            .ok_or_else(|| anyhow!("Crate does not contain a Cargo.toml file"))?
    } else if let Some(path) = args.path {
        std::fs::read_to_string(path)?
    } else {
        bail!("Either --name or --path needs to be specified");
    };

    let table = cargo_toml_content.parse::<toml::Table>()?;

    let name = table.get("package")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .with_context(|| "Failed to get crate name")?;

    let constraints = fm_synthesizer_flat::from_cargo_toml(&table)?;

    let file = File::create(args.destination)?;
    let mut writer = BufWriter::new(file);
    write_uvl(&mut writer, name, &constraints)?;

    Ok(())
}