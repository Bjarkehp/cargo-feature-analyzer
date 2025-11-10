pub mod flamapy;

use std::{fs::File, io::{BufWriter, Write}, path::PathBuf};

use anyhow::Context;
use cargo_toml::{crate_id, feature_dependencies};
use chrono::Local;
use clap::Parser;
use configuration_scraper::{configuration::Configuration, postgres};
use fm_synthesizer_fca::{concept, uvl};

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
        .map(crate_id::parse)
        .collect::<Result<Vec<_>, _>>()?;

    std::fs::create_dir_all("data/toml")?;
    std::fs::create_dir_all("data/configuration")?;
    std::fs::create_dir_all("data/model/flat")?;
    std::fs::create_dir_all("data/model/fca")?;
    std::fs::create_dir_all("data/result")?;

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

    for c in crates.iter() {
        let path = PathBuf::from(format!("data/model/fca/{}.uvl", c));
        if !std::fs::exists(&path)? {
            let config_path = PathBuf::from(format!("data/configuration/{}", c));
            let mut configurations = vec![];

            for entry in std::fs::read_dir(&config_path)? {
                let entry = entry?;
                let filename = entry.file_name();
                let name = filename.to_str()
                    .context("File name was not valid utf-8")?
                    .trim_end_matches(".csvconf");
                let crate_id = crate_id::parse(name)?;
                let content = std::fs::read_to_string(format!("data/configuration/{}/{}", c, filename.display()))?;
                let configuration = Configuration::from_csv_owned(crate_id.name.to_string(), crate_id.version.clone(), &content)
                    .with_context(|| format!("Configuration {}/{} could not be parsed", c, filename.display()))?;
                configurations.push(configuration);
            }

            let mut features = configurations.first()
                .with_context(|| format!("Crate {} has no configurations", c))?
                .features.keys()
                .map(|k| k.as_ref())
                .collect::<Vec<_>>();
            features.push(c.name);

            let ac_poset = concept::ac_poset(&configurations, &features, c.name);

            let uvl_file = File::create(&path)?;
            let mut uvl_writer = BufWriter::new(uvl_file);
            uvl::write_ac_poset(&mut uvl_writer, &ac_poset, &features)?;
            uvl_writer.flush()?;
        }
    }

    let date_time = Local::now().naive_local();
    let csv_file = File::create(format!("data/result/{}.csv", date_time))?;
    let mut csv_writer = BufWriter::new(csv_file);
    let columns = [
        "Crate",
        "Estimated number of configurations (flat)",
        "Estimated number of configurations (fca)",
        "Configuration number (flat)",
        "Configuration number (fca)",
    ];
    writeln!(csv_writer, "{}", columns.join(","))?;

    for c in crates.iter() {
        let flat = PathBuf::from(format!("data/model/flat/{}.uvl", c));
        let fca = PathBuf::from(format!("data/model/fca/{}.uvl", c));

        let estimated_number_of_configurations_flat = 
            flamapy::estimated_number_of_configurations(&flat)?;
        let estimated_number_of_configurations_fca = 
            flamapy::estimated_number_of_configurations(&fca)?;
        let configuration_number_flat = 0;
            // flamapy::configurations_number(&flat)?;
        let configuration_number_fca = 0;
            // flamapy::configurations_number(&fca)?;
        
        writeln!(csv_writer, "{},{},{},{},{}",
            c,
            estimated_number_of_configurations_flat,
            estimated_number_of_configurations_fca,
            configuration_number_flat,
            configuration_number_fca
        )?;
    }

    csv_writer.flush()?;

    Ok(())
}