use std::{error::Error, io::BufWriter, path::{Path, PathBuf}};

use clap::Parser;
use configuration::Configuration;
use postgres::{types::ToSql, Client, NoTls};
use std::io::Write;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    cargo_toml_path: PathBuf,
    database_str: String,
    config_destination: PathBuf,

    #[arg(short, long, default_value_t = 0)]
    offset: i64,
    #[arg(short, long, default_value_t = 100)]
    limit: i64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let cargo_toml_content = std::fs::read_to_string(&args.cargo_toml_path)
        .expect("Failed to read Cargo.toml");
    let table: toml::Table = cargo_toml_content.parse()?;
    let crate_name = table.get("package")
        .and_then(|pkg| pkg.get("name"))
        .and_then(|name| name.as_str())
        .ok_or("Failed to get crate name from Cargo.toml")?;
    let feature_dependencies = configuration::feature_dependencies::from_cargo_toml(&table)?;
    let features = feature_dependencies.nodes()
        .collect::<Vec<_>>();

    std::fs::create_dir_all(&args.config_destination)?;

    // let url = "postgresql://postgres@localhost:5432/cratesio";
    let mut client = Client::connect(&args.database_str, NoTls)?;
    let query = include_str!("query.sql");
    let params: &[&(dyn ToSql + Sync)] = &[&crate_name, &args.limit, &args.offset];
    let rows = client.query(query, params)?;
    for row in rows {
        let dependent_name: String = row.get("dependent_crate");
        let version: String = row.get("dependent_version");
        let mut explicit_features: Vec<String> = row.get("features");
        let default_features: bool = row.get("default_features");
        if default_features {
            explicit_features.push("default".to_string());
        }

        let implicit_features = configuration::implied_features(explicit_features.iter().map(|f| f.as_str()), &feature_dependencies)?;
        let configuration = configuration::Configuration::new(
            format!("{}-{}.csvconf", dependent_name, version),
            implicit_features,
        );
        let config_path = args.config_destination.join(configuration.name());
        write_configuration(&configuration, config_path, &features, crate_name)?;
    };

    Ok(())
}

/// Write a configuration to a .csvconf file
fn write_configuration(
    config: &Configuration, 
    destination: impl AsRef<Path>,
    features: &[&str], 
    crate_name: &str
) -> std::io::Result<()> {
    let file = std::fs::File::create(destination)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "\"{}\",True", crate_name)?;
    for feature in features {
        if config.features().contains(feature) {
            writeln!(writer, "\"{}\",True", feature)?;
        } else {
            writeln!(writer, "\"{}\",False", feature)?;
        }
    }

    Ok(())
}