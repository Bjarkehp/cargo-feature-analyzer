use std::{fs::File, io::BufWriter, path::PathBuf};

use anyhow::Context;
use cargo_toml::feature_dependencies;
use clap::Parser;
use configuration_scraper::postgres;

pub mod crates;

#[derive(Parser)]
struct Args {
    #[arg(long, action)]
    overwrite_configurations: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let crates_content = std::fs::read_to_string("crates.txt")
        .context("Could not find crates.txt, try running analysis from root directory")?;
    let crates = crates_content.lines()
        .map(crates::parse)
        .collect::<Result<Vec<_>, _>>()?;

    for c in crates.iter() {
        let path = PathBuf::from(format!("data/toml/{}.toml", c));
        if !std::fs::exists(&path)? {
            println!("Downloading Cargo.toml for {}", c);
            let toml_content = cargo_toml::download(c.name, &c.version.to_string())?;
            std::fs::write(&path, toml_content)?;
        }
    }

    let url = "postgres://crates:crates@localhost:5432/crates_db";
    if let Ok(mut client) = postgres::Client::connect(url, postgres::NoTls) {
        for c in crates.iter() {
            let directory = PathBuf::from(format!("data/configuration/{}", c));
            if !std::fs::exists(&directory)? || args.overwrite_configurations {
                std::fs::create_dir_all(&directory)?;
                println!("Scraping configurations for {}", c);
                let cargo_toml_content = std::fs::read_to_string(format!("data/toml/{}.toml", c))?;
                let table: toml::Table = cargo_toml_content.parse()?;
                let dependency_graph = feature_dependencies::from_cargo_toml(&table)?;
                let configurations = configuration_scraper::scrape(
                    c.name, 
                    &c.version, 
                    &dependency_graph, 
                    &mut client, 
                    0, 
                    1000
                )?;

                println!("Found {} configurations", configurations.len());

                for conf in configurations {
                    let path = PathBuf::from(format!("{}/{}@{}.csvconf", directory.display(), conf.name, conf.version));
                    let conf_content = conf.to_csv();
                    std::fs::write(path, conf_content)?;
                }
            }
        }
    } else {
        println!("Warning: Could not connect to crates_db, skipping updating configurations");
    }

    for c in crates.iter() {
        let path = PathBuf::from(format!("data/model/flat/{}.uvl", c));
        if !std::fs::exists(&path)? {
            let cargo_toml_content = std::fs::read_to_string(format!("data/toml/{}.toml", c))?;
            let table: toml::Table = cargo_toml_content.parse()?;
            let constraints = fm_synthesizer_flat::from_cargo_toml(&table)?;

            let file = File::create(&path)?;
            let mut writer = BufWriter::new(file);

            fm_synthesizer_flat::write_uvl(&mut writer, c.name, &constraints)?;
        }
    }

    Ok(())
}