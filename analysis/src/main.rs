pub mod flamapy_client;

use std::{collections::{BTreeMap, BTreeSet}, fs::File, io::{BufWriter, Write}, path::{Path, PathBuf}};

use anyhow::{Context, anyhow};
use cargo_toml::{crate_id::CrateId, feature_dependencies, implied_features};
use chrono::Local;
use configuration_scraper::{configuration::Configuration, postgres};
use crate_scraper::crate_entry::CrateEntry;
use fm_synthesizer_fca::{concept, uvl};
use itertools::Itertools;

const CRATE_ENTRIES_PATH: &str = "data/crates.txt";
const TOML_PATH: &str = "data/toml";
const CONFIG_PATH: &str = "data/configuration";
const FLAT_MODEL_PATH: &str = "data/model/flat";
const FCA_MODEL_PATH: &str = "data/model/fca";
const RESULT_PATH: &str = "data/result";

const POSTGRES_CONNECTION_STRING: &str = "postgres://crates:crates@localhost:5432/crates_io_db";

fn main() -> anyhow::Result<()> {
    for path in [TOML_PATH, CONFIG_PATH, FLAT_MODEL_PATH, FCA_MODEL_PATH, RESULT_PATH] {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory {path}"))?;
    }

    let mut client = postgres::Client::connect(POSTGRES_CONNECTION_STRING, postgres::NoTls)
        .with_context(|| anyhow!("Failed to create postgres client"))?;

    let crate_entries = get_or_scrape_crate_entries(&mut client)?
        .into_iter()
        .map(|e| (e.id, e.data))
        .collect::<BTreeMap<_, _>>();

    let cargo_tomls = crate_entries.keys()
        .map(|id| get_or_scrape_cargo_toml(id).map(|table| (id, table)))
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    let dependency_graphs = cargo_tomls.iter()
        .map(|(id, table)| {
            feature_dependencies::from_cargo_toml(table)
                .map(|table| (*id, table))
                .with_context(|| format!("Failed to create dependency graph for {id}"))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    let configurations = dependency_graphs.iter()
        .map(|(id, graph)| get_or_scrape_configurations(id, graph, &mut client).map(|c| (*id, c)))
        .filter_ok(|(_, configs)| !configs.is_empty())
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    for (id, table) in cargo_tomls.iter() {
        let path = PathBuf::from(format!("{FLAT_MODEL_PATH}/{id}.uvl"));
        if let Ok(file) = File::create_new(&path) {
            let constraints = fm_synthesizer_flat::from_cargo_toml(table)
                .with_context(|| format!("Failed to create flat constraints from {path:?}"))?;
            let mut writer = BufWriter::new(file);
            fm_synthesizer_flat::write_uvl(&mut writer, &id.name, &constraints)
                .with_context(|| format!("Failed to write flat feature model to {path:?}"))?;
            writer.flush()
                .with_context(|| format!("Failed to flush file {path:?}"))?;
        }
    }

    for (id, configurations) in configurations.iter().filter(|(_id, configs)| configs.len() > 100) {
        let path = PathBuf::from(format!("{FCA_MODEL_PATH}/{id}.uvl"));
        if let Ok(file) = File::create_new(&path) {
            let train_configurations = &configurations[..configurations.len() / 10];
            let mut features = train_configurations.first()
                .expect("Crates are filtered above for number of configs")
                .features.keys()
                .map(|k| k.as_ref())
                .collect::<Vec<_>>();
            features.push(&id.name);

            let ac_poset = concept::ac_poset(train_configurations, &features, &id.name);
            let mut writer = BufWriter::new(file);
            uvl::write_ac_poset(&mut writer, &ac_poset, &features)
                .with_context(|| format!("Failed to write fca feature model to {path:?}"))?;
            writer.flush()
                .with_context(|| format!("Failed to flush file {path:?}"))?;
        }
    }

    let date_time = Local::now().naive_local();
    let csv_path = PathBuf::from(format!("data/result/{}.csv", date_time));
    let csv_file = File::create(&csv_path)
        .with_context(|| format!("Failed to create file result table at {csv_path:?}"))?;
    let mut csv_writer = BufWriter::new(csv_file);
    
    let columns = [
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
    ];

    writeln!(csv_writer, "{}", columns.join(","))
        .with_context(|| format!("Failed to write column names to {csv_path:?}"))?; 

    let flamapy_server = Path::new("analysis/src/flamapy_server.py");
    let mut flamapy_client = flamapy_client::Client::new(flamapy_server)
        .with_context(|| "Failed to create flamapy client")?;

    for id in crate_entries.keys() {
        println!("Writing {id} into result...");

        let flat = PathBuf::from(format!("data/model/flat/{}.uvl", id));
        let fca = PathBuf::from(format!("data/model/fca/{}.uvl", id));

        write!(csv_writer, "{id},")
            .with_context(|| format!("Failed to write to {csv_path:?}"))?;

        let dependencies = dependency_graphs.get(id)
            .with_context(|| format!("Expected a dependency graph for {id}"))?;
        let features = dependencies.node_count();
        let feature_dependencies = dependencies.edge_count();
        write!(csv_writer, "{features},{feature_dependencies},")
            .with_context(|| format!("Failed to write to {csv_path:?}"))?;

        let configs = configurations.get(id).map(|v| v.as_slice());
        let default_features = implied_features::from_dependency_graph(std::iter::once("default"), dependencies);
        let config_stats = get_configuration_stats(configs, &default_features);

        write!(csv_writer, "{},{},{},", 
            config_stats.configuration_count, 
            config_stats.default_configurations_count, 
            config_stats.unique_configurations_count
        ).with_context(|| format!("Failed to write to {csv_path:?}"))?;

        if features < 300 {
            flamapy_client.set_model(&flat)
                .with_context(|| format!("Failed to set model to {flat:?}"))?;

            let estimated_number_of_configurations_flat = flamapy_client.estimated_number_of_configurations()
                .with_context(|| format!("Failed to get estimated number of configurations for {flat:?}"))?;

            let configuration_number_flat = flamapy_client.configurations_number()
                .with_context(|| format!("Failed to get configration number for {flat:?}"))?;

            write!(csv_writer, "{estimated_number_of_configurations_flat},{configuration_number_flat},")
        } else {
            write!(csv_writer, ",,")
        }.with_context(|| format!("Failed to write to {csv_path:?}"))?;

        if let Some(configs) = configs && fca.exists() && features < 300 {
            flamapy_client.set_model(&fca)
                .with_context(|| format!("Failed to set model to {fca:?}"))?;

            let estimated_number_of_configurations_fca = flamapy_client.estimated_number_of_configurations()
                .with_context(|| format!("Failed to get estimated number of configurations for {fca:?}"))?;

            let configuration_number_fca = flamapy_client.configurations_number()
                .with_context(|| format!("Failed to get configuration number for {fca:?}"))?;

            let test_configs = &configs[configs.len() / 10..];
            let satified_configurations = number_of_satisfied_configurations(&mut flamapy_client, id, test_configs)?;
            let quality = satified_configurations as f64 / test_configs.len() as f64;

            writeln!(csv_writer, "{estimated_number_of_configurations_fca},{configuration_number_fca},{quality}")
        } else {
            writeln!(csv_writer, ",,")
        }.with_context(|| format!("Failed to write to {csv_path:?}"))?;
    }

    csv_writer.flush().with_context(|| format!("Failed to flush {csv_path:?}"))?;

    Ok(())
}

fn get_or_scrape_crate_entries(client: &mut postgres::Client) -> anyhow::Result<Vec<CrateEntry>> {
    if let Ok(content) = std::fs::read_to_string(CRATE_ENTRIES_PATH) {
        content.lines()
            .map(|line| line.parse())
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("Expected to parse {CRATE_ENTRIES_PATH} as a list of crates"))
    } else {
        println!("Scraping 300 popular crates from crates.io...");

        let entries = crate_scraper::scrape_popular(client, 300)
            .expect("Failed to scrape popular crates");

        let file = File::create(CRATE_ENTRIES_PATH)
            .with_context(|| format!("Failed to create file {CRATE_ENTRIES_PATH}"))?;

        let mut writer = BufWriter::new(file);

        for entry in entries.iter() {
            println!("{}", entry);
            writeln!(writer, "{}", entry)
                .with_context(|| format!("Failed to write to file {CRATE_ENTRIES_PATH}"))?;
        }

        Ok(entries)
    }
}

fn get_or_scrape_cargo_toml(id: &CrateId) -> anyhow::Result<toml::Table> {
    let path = PathBuf::from(format!("{TOML_PATH}/{id}.toml"));
    let content = std::fs::read_to_string(&path).or_else(|_| {
        println!("Downloading Cargo.toml for {}", id);
        let toml_content = cargo_toml::download(&id.name, &id.version.to_string())
            .with_context(|| format!("Failed to download Cargo.toml for {id}"))?;
        std::fs::write(&path, &toml_content)
            .with_context(|| format!("Failed to write Cargo.toml for {id} to {path:?}"))?;
        Ok::<_, anyhow::Error>(toml_content)
    })?;

    content.parse().with_context(|| format!("Failed to parse Cargo.toml for {id}"))
}

fn get_or_scrape_configurations(id: &CrateId, dependency_graph: &feature_dependencies::Graph, client: &mut postgres::Client) -> anyhow::Result<Vec<Configuration<'static>>> {
    let path = PathBuf::from(format!("{CONFIG_PATH}/{id}"));
    if let Ok(entries) = std::fs::read_dir(&path) {
        println!("Collecting configurations for {id}...");

        entries.map(|r| r.with_context(|| format!("Failed to get entry in {path:?}")))
            .map(|r| r.and_then(|entry| read_configuration(&entry.path())))
            .collect::<anyhow::Result<Vec<_>>>()
    } else {
        std::fs::create_dir(&path)
            .with_context(|| format!("Failed to create directory {path:?}"))?;

        println!("Scraping configurations for {id}...");

        let configurations = configuration_scraper::scrape(
            &id.name, 
            &id.version, 
            dependency_graph, 
            client, 
            0, 
            1000
        ).with_context(|| format!("Failed to query for configuration for {id}"))?;

        println!("Found {} configurations", configurations.len());

        for configuration in configurations.iter() {
            let config_path = path.join(format!("{}@{}.csvconf", configuration.name, configuration.version));
            std::fs::write(&config_path, configuration.to_csv())
                .with_context(|| format!("Failed to write to configuration file {path:?}"))?;
        }

        Ok(configurations)
    }
}

fn read_configuration(path: &Path) -> anyhow::Result<Configuration<'static>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read configuration file at {path:?}"))?;
    
    let file_name = path.file_stem()
        .with_context(|| format!("Failed to get name of file at {path:?}"))?
        .to_str()
        .with_context(|| format!("Failed to convert path {path:?} to utf8"))?;

    let config_id: CrateId = file_name.parse()
        .with_context(|| format!("Failed to parse configuration id for file at {path:?}"))?;

    Configuration::from_csv_owned(config_id.name, config_id.version, &content)
        .with_context(|| format!("Failed to parse configuration file at {path:?}"))
}

struct ConfigStats {
    configuration_count: usize,
    default_configurations_count: usize, 
    unique_configurations_count: usize,
}

fn get_configuration_stats(configs: Option<&[Configuration<'static>]>, default_features: &BTreeSet<&str>) -> ConfigStats {
    if let Some(configs) = configs {
        let configuration_count = configs.len();

        let default_configurations_count = configs
            .iter()
            .filter(|config| config.features.iter().all(|(feature, &enabled)| default_features.contains(feature.as_ref()) == enabled))
            .count();

        let unique_configurations_count = configs.iter()
            .into_group_map_by(|config| &config.features)
            .len();

        ConfigStats { 
            configuration_count, 
            default_configurations_count, 
            unique_configurations_count 
        }
    } else {
        ConfigStats { 
            configuration_count: 0, 
            default_configurations_count: 0, 
            unique_configurations_count: 0 
        }
    }
}

fn number_of_satisfied_configurations(client: &mut flamapy_client::Client, id: &CrateId, configurations: &[Configuration<'static>]) -> anyhow::Result<usize> {
    configurations.iter()
        .map(|config| {
            let path = PathBuf::from(format!("data/configuration/{id}/{}@{}.csvconf", config.name, config.version));
            client.satisfiable_configuration(&path)
                .map(|b| b as usize)
                .with_context(|| format!("Failed to check for satisfiable configuration for {}@{} for {id}", config.name, config.version))
        })
        .fold_ok(0, |acc, x| acc + x)
}