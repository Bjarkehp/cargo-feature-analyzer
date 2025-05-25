use std::{error::Error, fs, io::BufWriter, path::{Path, PathBuf}};

use clap::Parser;
use colored::Colorize;
use configuration::{feature_dependencies, Configuration};
use crates_io_api::{ReverseDependency, SyncClient};
use std::io::Write;

/// Program for scraping the top dependents of a specified crate from crates.io
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    crate_name: String,
    cargo_destination: PathBuf,
    config_destination: PathBuf,

    #[arg(short, long, default_value_t = 100)]
    count: u32
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let user_agent = "feature-configuration-scraper (bjpal22@student.sdu.dk)";
    let rate_limit = std::time::Duration::from_millis(1000);
    let crates_client = SyncClient::new(user_agent, rate_limit)
        .map_err(|_| "Failed to create client")?;
    let reqwest_client = reqwest::blocking::Client::new();

    let repository = crates_client.get_crate(&args.crate_name)
        .map(|c| c.crate_data.repository.ok_or(format!("Failed to get repository for {}", c.crate_data.name)))??;
    let cargo_toml = download_cargo_toml(&reqwest_client, &repository, &args.crate_name)?;
    std::fs::write(args.cargo_destination, &cargo_toml)?;
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
                eprintln!("{}", format!("Failed to get crate: {}", e).red());
                continue;
            },
            None => break,
        };

        println!();
        println!("{}:", c.name);

        let repository = if let Some(r) = c.repository {
            r
        } else {
            eprintln!("{}", format!("Crate {} has no reposity", c.name).red());
            continue;
        };

        let config_cargo_toml = if let Ok(content) = download_cargo_toml(&reqwest_client, &repository, &c.name) {
            content
        } else {
            println!("Couldn't find Cargo.toml for {}", c.name);
            continue;
        };

        let config_table = config_cargo_toml.parse()?;
        
        let mut destination = PathBuf::new();
        destination.push(&args.config_destination);
        destination.push(format!("{}.toml", c.name));
        fs::write(destination, &config_cargo_toml)?;

        let configurations = feature_dependencies::get_dependency_tables(&config_table)
            .into_iter()
            .filter_map(|table| configuration::implied_features(table, &args.crate_name, &feature_dependencies).ok())
            .enumerate()
            .map(|(i, features)| Configuration::new(format!("{}-({})", c.name, i + 1), features))
            .take((args.count - configuration_count) as usize)
            .collect::<Vec<_>>();
        
        for configuration in configurations {
            let mut destination = PathBuf::new();
            destination.push(&args.config_destination);
            destination.push(format!("{}.csvconf", configuration.name()));
            write_configuration(&configuration, destination, &features, &args.crate_name)?;
            configuration_count += 1;
            println!("({}/{}) Created configuration {}", configuration_count, args.count, configuration.name())
        }
    }

    Ok(())
}

fn write_configuration(
    config: &Configuration, 
    destination: impl AsRef<Path>,
    features: &[&str], 
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