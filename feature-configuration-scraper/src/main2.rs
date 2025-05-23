use std::{env, error::Error, fs, io::BufWriter, path::{Path, PathBuf}};

use clap::Parser;
use colored::Colorize;
use configuration::Configuration;
use crates_io_api::{ReverseDependency, SyncClient};
use std::io::Write;

use crate::create_client;

/// Program for scraping the top dependents of a specified crate from crates.io
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    crate_name: String,
    crate_destination: PathBuf,
    config_destination: PathBuf,

    #[arg(short, long, default_value_t = 100)]
    count: u32
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let crates_client = create_client()?;
    let reqwest_client = reqwest::blocking::Client::new();

    let repository = crates_client.get_crate(&args.crate_name)
        .map(|c| c.crate_data.repository.ok_or(format!("Failed to get repository for {}", c.crate_data.name)))??;
    let cargo_toml = download_cargo_toml(&reqwest_client, &repository, &args.crate_name)?;
    std::fs::File::create(args.crate_destination)?;
    let table = cargo_toml.parse()?;
    let feature_dependencies = configuration::feature_dependencies::from_cargo_toml(&table)?;
    let features = feature_dependencies.keys()
        .cloned()
        .collect::<Vec<_>>();

    let mut dependents = top_dependents(&args.crate_name, &crates_client)
        .map(|d| crates_client.get_crate(&d.crate_version.crate_name).map(|c| c.crate_data));

    let mut configuration_count = 0;
    while configuration_count < args.count {
        let c = match dependents.next() {
            Some(Ok(c)) => c,
            Some(Err(e)) => {
                eprintln!("Failed to get crate: {}", e);
                continue;
            },
            None => break,
        };

        let name = c.name;
        let repository = c.repository.ok_or(format!("Failed to get repository for {}", name))?;
        let config_cargo_toml = download_cargo_toml(&reqwest_client, &repository, &name)?;
        let config_table = config_cargo_toml.parse()?;
        
        let mut destination = PathBuf::new();
        destination.push(&args.config_destination);
        destination.push(format!("{}.toml", name));
        fs::write(destination, &config_cargo_toml)?;
        
        if let Ok(config) = Configuration::new_standard(&config_table, &args.crate_name, &feature_dependencies) {
            let mut destination = PathBuf::new();
            destination.push(&args.config_destination);
            destination.push(format!("{}.csvconf", name));
            write_configuration(&config, destination, features.iter().cloned(), args.crate_name.as_str())?;
            configuration_count += 1;
            println!("{}", "Created standard configuration".green());
        } else {
            println!("{}", "Failed to create standard configuration".yellow());
        }
        
        if configuration_count == args.count {
            return Ok(());
        }

        if let Ok(config) = Configuration::new_dev(&config_table, &args.crate_name, &feature_dependencies) {
            let mut destination = PathBuf::new();
            destination.push(&args.config_destination);
            destination.push(format!("{}.dev.csvconf", name));
            write_configuration(&config, destination, features.iter().cloned(), args.crate_name.as_str())?;
            configuration_count += 1;
            println!("{}", "Created dev configuration".green());
        } else {
            println!("{}", "Failed to create standard configuration".yellow());
        }
    }

    Ok(())
}

fn write_configuration<'a>(
    config: &Configuration, 
    destination: impl AsRef<Path>,
    features: impl Iterator<Item = &'a str>, 
    crate_name: &str
) -> std::io::Result<()> {
    let file = fs::File::create(destination)?;
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

fn download_cargo_toml(client: &reqwest::blocking::Client, repository: &str, crate_name: &str) -> Result<String, Box<dyn Error>> {
    download_github_file(client, repository, "Cargo.toml").ok()
        .filter(|content| content.contains("[package]"))
        .or_else(|| download_github_file(client, repository, &format!("{}/Cargo.toml", crate_name)).ok())
        .ok_or(format!("Failed to download Cargo.toml from {}", repository).into())
}

fn top_dependents(crate_name: &str, client: &SyncClient) -> impl Iterator<Item = ReverseDependency> {
    (1..).map(|i| client.crate_reverse_dependencies_page(crate_name, i).map(|page| page.dependencies.into_iter()))
        .scan((), |_, page| page.ok())
        .flatten()
}

fn download_github_file(client: &reqwest::blocking::Client, repository: &str, path: &str) -> Result<String, Box<dyn Error>> {
    let url = github_path(repository, path)
        .ok_or("Invalid repository or path")?;
    Ok(client.get(url).send()?.error_for_status()?.text()?)
}

fn github_path(repository_url: &str, path: &str) -> Option<String> {
    repository_url.strip_prefix("https://github.com/")
        .map(|repository| format!("https://raw.githubusercontent.com/{}/refs/heads/master/{}", repository, path))
}