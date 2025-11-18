use std::{error::Error, io::BufWriter, path::{Path, PathBuf}};

use cargo_toml::feature_dependencies;

use clap::Parser;
use configuration_scraper::configuration::Configuration;
use postgres::NoTls;
use semver::Version;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    cargo_toml_path: PathBuf,
    config_destination: PathBuf,

    #[arg(short, long, default_value = None)]
    database_str: Option<String>,
    #[arg(short, long, default_value_t = 0)]
    offset: i64,
    #[arg(short, long, default_value_t = 100)]
    limit: i64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let cargo_toml_content = std::fs::read_to_string(args.cargo_toml_path)
        .expect("Failed to read Cargo.toml");
    let table: toml::Table = cargo_toml_content.parse()?;
    let crate_name = table.get("package")
        .and_then(|package| package.get("name"))
        .and_then(|name| name.as_str())
        .ok_or("Failed to get crate name from Cargo.toml")?;
    let crate_version_str = table.get("package")
        .and_then(|package| package.get("version"))
        .and_then(|version| version.as_str())
        .ok_or("Failed to get crate version from Cargo.toml")?;
    let crate_version: Version = crate_version_str.parse()
        .expect("Failed to parse crate version from Cargo.toml");
    let feature_dependencies = feature_dependencies::from_cargo_toml(&table)?;
    let features = feature_dependencies.nodes()
        .collect::<Vec<_>>();
    let mut client = postgres::Client::connect(args.database_str.as_deref().unwrap_or("postgres://crates:crates@localhost:5432/crates_db"), NoTls)?;

    let configurations = configuration_scraper::scrape(
        crate_name,
        &crate_version,
        &feature_dependencies,
        &mut client,
        args.offset,
        args.limit,
    )?;

    let dir = PathBuf::from(format!("{}/{}@{}", args.config_destination.display(), crate_name, crate_version));
    std::fs::create_dir_all(&dir)?;

    for config in configurations {
        let path = PathBuf::from(format!("{}/{}@{}", dir.display(), config.name, config.version));
        write_configuration(&config, &path, &features)?;
    }

    Ok(())
}

/// Write a configuration to a .csvconf file
fn write_configuration(
    config: &Configuration, 
    destination: impl AsRef<Path>,
    features: &[&str], 
) -> std::io::Result<()> {
    let file = std::fs::File::create(destination)?;
    let mut writer = BufWriter::new(file);
    for &feature in features {
        if config.features[feature] {
            writeln!(writer, "\"{}\",True", feature.replace('-', "_"))?;
        } else {
            writeln!(writer, "\"{}\",False", feature.replace('-', "_"))?;
        }
    }

    Ok(())
}