pub mod flamapy_client;

use std::{fs::File, io::{BufWriter, Write}, path::{Path, PathBuf}};

use anyhow::Context;
use cargo_toml::{crate_id::{self, CrateId}, feature_dependencies, implied_features};
use chrono::Local;
use clap::Parser;
use configuration_scraper::{configuration::Configuration, postgres};
use fm_synthesizer_fca::{concept, uvl};
use itertools::Itertools;

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
    std::fs::create_dir_all("data/configuration/train")?;
    std::fs::create_dir_all("data/configuration/test")?;
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
    let mut client = postgres::Client::connect(url, postgres::NoTls)?;
    for c in crates.iter() {
        let train_directory = PathBuf::from(format!("data/configuration/train/{}", c));
        let test_directory = PathBuf::from(format!("data/configuration/test/{}", c));
        if !std::fs::exists(&train_directory)? || !std::fs::exists(&test_directory)? || args.overwrite_configurations {
            if !args.overwrite_configurations {
                std::fs::create_dir_all(&train_directory)?;
                std::fs::create_dir_all(&test_directory)?;
            }
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
                200
            )?;

            println!("Found {} configurations", configurations.len());

            for (i, conf) in configurations.iter().enumerate() {
                let directory = if i < configurations.len() / 2 {
                    &train_directory
                } else {
                    &test_directory
                };
                let path = PathBuf::from(format!("{}/{}@{}.csvconf", directory.display(), conf.name, conf.version));
                let conf_content = conf.to_csv();
                std::fs::write(path, conf_content)?;
            }
        }
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
            let train_configurations = get_train_configurations(c)?;

            let mut features = train_configurations.first()
                .with_context(|| format!("Crate {} has no configurations", c))?
                .features.keys()
                .map(|k| k.as_ref())
                .collect::<Vec<_>>();
            features.push(c.name);

            let ac_poset = concept::ac_poset(&train_configurations, &features, c.name);

            let uvl_file = File::create(&path)?;
            let mut uvl_writer = BufWriter::new(uvl_file);
            uvl::write_ac_poset(&mut uvl_writer, &ac_poset, &features)?;
            uvl_writer.flush()?;
        }
    }

    let date_time = Local::now().naive_local();
    let csv_file = File::create(format!("data/result/{}.csv", date_time))?;
    let mut csv_writer = BufWriter::new(csv_file);
    
    #[allow(clippy::write_literal)]
    writeln!(csv_writer, "{},{},{},{},{},{},{},{},{},{},{}",
        "Crate",
        "Features",
        "Feature dependencies",
        "Configurations",
        "Default only configurations",
        "Unique configurations",
        "Estimated number of configurations (flat)",
        "Estimated number of configurations (FCA)",
        "Configuration number (flat)",
        "Configuration number (FCA)",
        "FCA Quality",
    )?;    

    let flamapy_server = Path::new("analysis/src/flamapy_server.py");
    let mut flamapy_client = flamapy_client::Client::new(flamapy_server)?;

    for c in crates.iter() {
        let toml = PathBuf::from(format!("data/toml/{}.toml", c));
        let flat = PathBuf::from(format!("data/model/flat/{}.uvl", c));
        let fca = PathBuf::from(format!("data/model/fca/{}.uvl", c));

        let table: toml::Table = std::fs::read_to_string(&toml)?.parse()?;

        let dependencies = feature_dependencies::from_cargo_toml(&table)?;
        let features = dependencies.node_count();
        let feature_dependencies = dependencies.edge_count();
        let default_features = implied_features::from_dependency_graph(std::iter::once("default"), &dependencies);
        let train_configurations = get_train_configurations(c)?;
        let test_configurations = get_test_configurations(c)?;
        let configurations = train_configurations.iter()
            .chain(test_configurations.iter())
            .collect::<Vec<_>>();
        let configuration_count = configurations.len();
        let default_configurations_count = configurations
            .iter()
            .filter(|config| config.features.iter().all(|(feature, &enabled)| default_features.contains(feature.as_ref()) == enabled))
            .count();
        let unique_configurations_count = configurations.iter()
            .into_group_map_by(|config| &config.features)
            .len();

        flamapy_client.set_model(&flat)?;

        let estimated_number_of_configurations_flat = 
            flamapy_client.estimated_number_of_configurations()?;
        let configuration_number_flat = 
            flamapy_client.configurations_number()?;

        flamapy_client.set_model(&fca)?;

        let estimated_number_of_configurations_fca = 
            flamapy_client.estimated_number_of_configurations()?;
        let configuration_number_fca = 
            flamapy_client.configurations_number()?;

        let satified_configurations = test_configurations.iter()
            .filter(|config| {
                let path = PathBuf::from(format!("data/configuration/test/{}/{}@{}.csvconf", c, config.name, config.version));
                let output = flamapy_client.satisfiable_configuration(&path);
                if let Ok(result) = output {
                    result
                } else {
                    println!("Warning ({}@{}): {:?}", config.name, config.version, output);
                    false
                }
            })
            .count();
        let quality = satified_configurations as f64 / test_configurations.len() as f64;
        
        writeln!(csv_writer, "{},{},{},{},{},{},{},{},{},{},{}",
            c,
            features,
            feature_dependencies,
            configuration_count,
            default_configurations_count,
            unique_configurations_count,
            estimated_number_of_configurations_flat,
            estimated_number_of_configurations_fca,
            configuration_number_flat,
            configuration_number_fca,
            quality
        )?;
    }

    csv_writer.flush()?;

    Ok(())
}

fn get_train_configurations(crate_id: &CrateId) -> anyhow::Result<Vec<Configuration<'static>>> {
    get_configurations(Path::new("data/configuration/train"), crate_id)
}

fn get_test_configurations(crate_id: &CrateId) -> anyhow::Result<Vec<Configuration<'static>>> {
    get_configurations(Path::new("data/configuration/test"), crate_id)
}

fn get_configurations(path: &Path, crate_id: &CrateId) -> anyhow::Result<Vec<Configuration<'static>>> {
    let config_path = PathBuf::from(format!("{}/{}", path.display(), crate_id));
    let mut configurations = vec![];

    for entry in std::fs::read_dir(&config_path)? {
        let entry = entry?;
        let filename = entry.file_name();
        let name = filename.to_str()
            .context("File name was not valid utf-8")?
            .trim_end_matches(".csvconf");
        let dependency_crate_id = crate_id::parse(name)?;
        let content = std::fs::read_to_string(format!("{}/{}/{}", path.display(), crate_id, filename.display()))?;
        let configuration = Configuration::from_csv_owned(dependency_crate_id.name.to_string(), dependency_crate_id.version.clone(), &content)
            .with_context(|| format!("Configuration {}/{} could not be parsed", crate_id, filename.display()))?;
        configurations.push(configuration);
    }

    Ok(configurations)
}