pub mod flamapy_client;

use std::{collections::BTreeMap, fs::File, io::{BufWriter, Write}, path::{Path, PathBuf}};

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

fn main() {
    std::fs::create_dir_all(TOML_PATH)
        .unwrap_or_else(|e| panic!("Failed to create directory {TOML_PATH}: {e}"));
    std::fs::create_dir_all(CONFIG_PATH)
        .unwrap_or_else(|e| panic!("Failed to create directory {CONFIG_PATH}: {e}"));
    std::fs::create_dir_all(FLAT_MODEL_PATH)
        .unwrap_or_else(|e| panic!("Failed to create directory {FLAT_MODEL_PATH}: {e}"));
    std::fs::create_dir_all(FCA_MODEL_PATH)
        .unwrap_or_else(|e| panic!("Failed to create directory {FCA_MODEL_PATH}: {e}"));
    std::fs::create_dir_all("data/result")
        .unwrap_or_else(|e| panic!("UFailed to create directory data/result: {e}"));

    let url = "postgres://crates:crates@localhost:5432/crates_io_db";
    let mut client = postgres::Client::connect(url, postgres::NoTls)
        .unwrap_or_else(|e| panic!("Failed to create postgres client: {e}"));

    let crate_entries = get_or_scrape_crate_entries(&mut client)
        .into_iter()
        .map(|e| (e.id, e.data))
        .collect::<BTreeMap<_, _>>();

    let cargo_tomls = crate_entries.keys()
        .map(|id| (id, get_or_scrape_cargo_toml(id)))
        .collect::<BTreeMap<_, _>>();

    let dependency_graphs = cargo_tomls.iter()
        .map(|(id, table)| {
            let table = feature_dependencies::from_cargo_toml(table)
                .unwrap_or_else(|e| panic!("Failed to create dependency graph for {id}: {e}"));
            (*id, table)
        })
        .collect::<BTreeMap<_, _>>();

    let configurations = dependency_graphs.iter()
        .map(|(id, graph)| (*id, get_or_scrape_configurations(id, graph, &mut client)))
        .filter(|(_, configurations)| !configurations.is_empty())
        .collect::<BTreeMap<_, _>>();

    for (id, table) in cargo_tomls.iter() {
        let path = PathBuf::from(format!("{FLAT_MODEL_PATH}/{id}.uvl"));
        if let Ok(file) = File::create_new(&path) {
            let constraints = fm_synthesizer_flat::from_cargo_toml(table)
                .unwrap_or_else(|e| panic!("Failed to create flat constraints from {path:?}: {e}"));
            let mut writer = BufWriter::new(file);
            fm_synthesizer_flat::write_uvl(&mut writer, &id.name, &constraints)
                .unwrap_or_else(|e| panic!("Failed to write flat feature model to {path:?}: {e}"));
            writer.flush()
                .unwrap_or_else(|e| panic!("Failed to flush file {path:?}: {e}"));
        }
    }

    for (id, configurations) in configurations.iter().filter(|(_id, configs)| configs.len() > 100) {
        let path = PathBuf::from(format!("{FCA_MODEL_PATH}/{id}.uvl"));
        if let Ok(file) = File::create_new(&path) {
            let train_configurations = &configurations[..configurations.len() / 10];
            let mut features = train_configurations.first()
                .unwrap() // Crates are filtered above for number of configs.
                .features.keys()
                .map(|k| k.as_ref())
                .collect::<Vec<_>>();
            features.push(&id.name);

            let ac_poset = concept::ac_poset(train_configurations, &features, &id.name);
            let mut writer = BufWriter::new(file);
            uvl::write_ac_poset(&mut writer, &ac_poset, &features)
                .unwrap_or_else(|e| panic!("Failed to write fca feature model to {path:?}: {e}"));
            writer.flush()
                .unwrap_or_else(|e| panic!("Failed to flush file {path:?}: {e}"));
        }
    }

    let date_time = Local::now().naive_local();
    let csv_path = PathBuf::from(format!("data/result/{}.csv", date_time));
    let csv_file = File::create(&csv_path)
        .unwrap_or_else(|e| panic!("Failed to create file result table at {csv_path:?}: {e}"));
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
    ).unwrap_or_else(|e| panic!("Failed to write column names to {csv_path:?}: {e}"));    

    let flamapy_server = Path::new("analysis/src/flamapy_server.py");
    let mut flamapy_client = flamapy_client::Client::new(flamapy_server)
        .unwrap_or_else(|e| panic!("Failed to create flamapy client: {e}"));

    for id in crate_entries.keys() {
        let flat = PathBuf::from(format!("data/model/flat/{}.uvl", id));
        let fca = PathBuf::from(format!("data/model/fca/{}.uvl", id));

        write!(csv_writer, "{id},")
            .unwrap_or_else(|e| panic!("Failed to write to {csv_path:?}: {e}"));

        let dependencies = dependency_graphs.get(id)
            .unwrap_or_else(|| panic!("Expected a dependency graph for {id}"));
        let features = dependencies.node_count();
        let feature_dependencies = dependencies.edge_count();
        write!(csv_writer, "{features},{feature_dependencies},")
            .unwrap_or_else(|e| panic!("Failed to write to {csv_path:?}: {e}"));

        let configs = configurations.get(id);
        let default_features = implied_features::from_dependency_graph(std::iter::once("default"), dependencies);
        let (configuration_count, default_configurations_count, unique_configurations_count) = configs.map(|configs| {
            let configuration_count = configs.len();
            let default_configurations_count = configs
                .iter()
                .filter(|config| config.features.iter().all(|(feature, &enabled)| default_features.contains(feature.as_ref()) == enabled))
                .count();
            let unique_configurations_count = configs.iter()
                .into_group_map_by(|config| &config.features)
                .len();
            (configuration_count, default_configurations_count, unique_configurations_count)
        }).unwrap_or((0, 0, 0));
        write!(csv_writer, "{configuration_count},{default_configurations_count},{unique_configurations_count},")
            .unwrap_or_else(|e| panic!("Failed to write to {csv_path:?}: {e}"));

        if features < 300 {
            flamapy_client.set_model(&flat)
                .unwrap_or_else(|e| panic!("Failed to set model to {flat:?}: {e}"));
            let estimated_number_of_configurations_flat = flamapy_client.estimated_number_of_configurations()
                .unwrap_or_else(|e| panic!("Failed to get estimated number of configurations for {flat:?}: {e}"));
            let configuration_number_flat = flamapy_client.configurations_number()
                .unwrap_or_else(|e| panic!("Failed to get configration number for {flat:?}: {e}"));
            write!(csv_writer, "{estimated_number_of_configurations_flat},{configuration_number_flat},")
                .unwrap_or_else(|e| panic!("Failed to write to {csv_path:?}: {e}"));
        } else {
            write!(csv_writer, ",,")
                .unwrap_or_else(|e| panic!("Failed to write to {csv_path:?}: {e}"));
        }
        

        if fca.exists() && features < 300 {
            flamapy_client.set_model(&fca)
                .unwrap_or_else(|e| panic!("Failed to set model to {fca:?}: {e}"));
            let estimated_number_of_configurations_fca = flamapy_client.estimated_number_of_configurations()
                .unwrap_or_else(|e| panic!("Failed to get estimated number of configurations for {fca:?}: {e}"));
            let configuration_number_fca = flamapy_client.configurations_number()
                .unwrap_or_else(|e| panic!("Failed to get configuration number for {fca:?}: {e}"));
            let configs = configs.unwrap_or_else(|| panic!("If an FCA model exist, there should also be configurations"));
            let test_configs = &configs[configs.len() / 10..];
            let satified_configurations = test_configs.iter()
                .filter(|config| {
                    let path = PathBuf::from(format!("data/configuration/{id}/{}@{}.csvconf", config.name, config.version));
                    flamapy_client.satisfiable_configuration(&path)
                        .unwrap_or_else(|e| panic!("Failed to check for satisfiable configuration for {}@{} for {id}: {e}", config.name, config.version))
                })
                .count();
            let quality = satified_configurations as f64 / test_configs.len() as f64;
            writeln!(csv_writer, "{estimated_number_of_configurations_fca},{configuration_number_fca},{quality}")
                .unwrap_or_else(|e| panic!("Failed to write to {csv_path:?}: {e}"));
        } else {
            writeln!(csv_writer, ",,")
                .unwrap_or_else(|e| panic!("Failed to write to {csv_path:?}: {e}"));
        }
    }

    csv_writer.flush()
        .unwrap_or_else(|e| panic!("Failed to flush {csv_path:?}: {e}"));
}

fn get_or_scrape_crate_entries(client: &mut postgres::Client) -> Vec<CrateEntry> {
    if let Ok(content) = std::fs::read_to_string(CRATE_ENTRIES_PATH) {
        content.lines()
            .map(|line| line.parse())
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| panic!("Expected to parse {CRATE_ENTRIES_PATH} as a list of crates: {e}"))
    } else {
        println!("Scraping 300 popular crates from crates.io...");
        let entries = crate_scraper::scrape_popular(client, 300)
            .expect("Failed to scrape popular crates");
        let file = File::create(CRATE_ENTRIES_PATH)
            .unwrap_or_else(|e| panic!("Failed to create file {CRATE_ENTRIES_PATH}: {e}"));
        let mut writer = BufWriter::new(file);
        for entry in entries.iter() {
            println!("{}", entry);
            writeln!(writer, "{}", entry)
                .unwrap_or_else(|e| panic!("Failed to write to file {CRATE_ENTRIES_PATH}: {e}"));
        }
        entries
    }
}

fn get_or_scrape_cargo_toml(id: &CrateId) -> toml::Table {
    let path = PathBuf::from(format!("{TOML_PATH}/{id}.toml"));
    let content = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        println!("Downloading Cargo.toml for {}", id);
        let toml_content = cargo_toml::download(&id.name, &id.version.to_string())
            .unwrap_or_else(|e| panic!("Failed to download Cargo.toml for {id}: {e}"));
        std::fs::write(&path, &toml_content)
            .unwrap_or_else(|e| panic!("Failed to write Cargo.toml for {id} to {path:?}: {e}"));
        toml_content
    });

    content.parse().unwrap_or_else(|e| panic!("Failed to parse Cargo.toml for {id}: {e}"))
}

fn get_or_scrape_configurations(id: &CrateId, dependency_graph: &feature_dependencies::Graph, client: &mut postgres::Client) -> Vec<Configuration<'static>> {
    let path = PathBuf::from(format!("{CONFIG_PATH}/{id}"));
    if let Ok(entries) = std::fs::read_dir(&path) {
        entries.map(|r| r.unwrap_or_else(|e| panic!("Failed to get entry in {path:?}: {e}")))
            .map(|entry| read_configuration(&entry.path()))
            .collect::<Vec<_>>()
    } else {
        std::fs::create_dir(&path)
            .unwrap_or_else(|e| panic!("Failed to create directory {path:?}: {e}"));
        println!("Scraping configurations for {id}...");
        let configurations = configuration_scraper::scrape(
            &id.name, 
            &id.version, 
            dependency_graph, 
            client, 
            0, 
            1000
        ).unwrap_or_else(|e| panic!("Failed to query for configuration for {id}: {e}"));
        println!("Found {} configurations", configurations.len());

        for configuration in configurations.iter() {
            let config_path = path.join(format!("{}@{}.csvconf", configuration.name, configuration.version));
            std::fs::write(&config_path, configuration.to_csv())
                .unwrap_or_else(|e| panic!("Failed to write to configuration file {path:?}: {e}"));
        }

        configurations
    }
}

fn read_configuration(path: &Path) -> Configuration<'static> {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read configuration file at {path:?}: {e}"));
    let file_name = path.file_stem()
        .unwrap_or_else(|| panic!("Failed to get name of file at {path:?}"))
        .to_str()
        .unwrap_or_else(|| panic!("Failed to convert path {path:?} to utf8"));
    let config_id: CrateId = file_name.parse()
        .unwrap_or_else(|e| panic!("Failed to parse configuration id for file at {path:?}: {e}"));
    Configuration::from_csv_owned(config_id.name, config_id.version, &content)
        .unwrap_or_else(|| panic!("Failed to parse configuration file at {path:?}"))
}