use std::{env, error::Error, fs, io::BufWriter, path::PathBuf};

use clap::Parser;
use colored::Colorize;
use crates_io_api::{ReverseDependency, SyncClient};
use std::io::Write;

mod main2;

/// Program for scraping the top dependents of a specified crate from crates.io
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    crate_name: String,

    #[arg(short, long, default_value = None)]
    destination: Option<PathBuf>,

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
    let table = cargo_toml.parse()?;
    let feature_dependencies = configuration::feature_dependencies::from_cargo_toml(&table)?;
    let features = feature_dependencies.keys()
        .collect::<Vec<_>>();

    let dependents = top_dependents(&args.crate_name, &crates_client)
        .map(|d| crates_client.get_crate(&d.crate_version.crate_name).map(|c| c.crate_data));

    let mut writes = dependents.map(|c| {
        let c = c?;
        let name = c.name;
        let repository = c.repository.ok_or(format!("Failed to get repository for {}", name))?;
        let config_cargo_toml = download_cargo_toml(&reqwest_client, &repository, &name)?;

        let config_table = config_cargo_toml.parse()?;
        let config = configuration::from(&config_table, &args.crate_name, &feature_dependencies)
            .ok_or(format!("Failed to create configuration for {}", name))?;

        let mut toml_destination = PathBuf::new();
        toml_destination.push(args.destination.as_deref().unwrap_or(&env::current_dir()?));
        toml_destination.push(format!("{}.toml", name));

        fs::write(toml_destination, &config_cargo_toml)?;

        let mut csvconf_destination = PathBuf::new();
        csvconf_destination.push(args.destination.as_deref().unwrap_or(&env::current_dir()?));
        csvconf_destination.push(format!("{}.csvconf", name));
        let csvconf_file = fs::File::create(csvconf_destination)?;
        let mut csvconf_writer = BufWriter::new(csvconf_file);
        
        writeln!(csvconf_writer, "\"{}\",True", args.crate_name)?;
        for feature in features.iter() {
            if config.features().contains(feature) {
                writeln!(csvconf_writer, "\"{}\",True", feature)?;
            } else {
                writeln!(csvconf_writer, "\"{}\",False", feature)?;
            }
        }

        csvconf_writer.flush()?;

        Result::<String, Box<dyn Error>>::Ok(name)
    });

    let count = args.count;
    let mut current = 0;

    while current < count {
        match writes.next() {
            Some(Ok(name)) => {
                current += 1;
                println!("({current}/{count}) {}", format!("Downloaded {}", name).green());
            },
            Some(Err(e)) => println!("({current}/{count}) {}", format!("{e}").red()),
            None => break
        }
    }

    Ok(())
}

fn create_client() -> Result<SyncClient, Box<dyn Error>> {
    let user_agent = "feature-configuration-scraper (bjpal22@student.sdu.dk)";
    let rate_limit = std::time::Duration::from_millis(1000);
    SyncClient::new(user_agent, rate_limit)
        .map_err(|_| "Failed to create client".into())
}

fn top_dependents(crate_name: &str, client: &SyncClient) -> impl Iterator<Item = ReverseDependency> {
    (1..).map(|i| client.crate_reverse_dependencies_page(crate_name, i).map(|page| page.dependencies.into_iter()))
        .scan((), |_, page| page.ok())
        .flatten()
}

fn download_cargo_toml(client: &reqwest::blocking::Client, repository: &str, crate_name: &str) -> Result<String, Box<dyn Error>> {
    download_github_file(client, repository, "Cargo.toml").ok()
        .filter(|content| content.contains("[package]"))
        .or_else(|| download_github_file(client, repository, &format!("{}/Cargo.toml", crate_name)).ok())
        .ok_or(format!("Failed to download Cargo.toml from {}", repository).into())
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