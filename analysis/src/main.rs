mod flamapy_client;
mod csv;
mod plot;
mod paths;
mod feature_model;
mod tables;
pub mod plots;
mod retry;
mod correlation;
mod bounding_box;

use std::{collections::{BTreeMap, BTreeSet}, fs::File, io::{BufWriter, Write}, path::{Path, PathBuf}};

use anyhow::{Context, anyhow};
use cargo_toml::{crate_id::CrateId, feature_dependencies, implied_features};
use chrono::Local;
use configuration_scraper::{configuration::Configuration, postgres};
use crate_scraper::crate_entry::CrateEntry;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use sorted_iter::{SortedPairIterator, assume::AssumeSortedByKeyExt};
use tokei::{LanguageType, Languages};

use crate::retry::retry;

const POSTGRES_CONNECTION_STRING: &str = "postgres://crates:crates@localhost:5432/crates_io_db";
const NUMBER_OF_CRATES: usize = 2;
const MAX_FEATURES: usize = 100;
const MIN_CONFIGS: usize = 100;
const MAX_CONFIGS: usize = 1000;
const MAX_DEPENDENCIES: usize = 1000;

fn main() -> anyhow::Result<()> {
    paths::prepare_directories()?;

    let mut postgres_client = postgres::Client::connect(POSTGRES_CONNECTION_STRING, postgres::NoTls)
        .with_context(|| "Failed to create postgres client")?;
    let mut flamapy_client = flamapy_client::Client::new(paths::FLAMAPY_SERVER)
        .with_context(|| "Failed to create flamapy client")?;
    let reqwest_client = cargo_toml::default_reqwest_client()
        .with_context(|| "Failed to create reqwest client")?;

    let crate_entries_vec = get_or_scrape_crate_entries(&mut postgres_client)?;

    let crate_entries = crate_entries_vec.iter()
        .map(|e| (&e.id, &e.data))
        .collect::<BTreeMap<_, _>>();

    for id in crate_entries.keys() {
        download_crate(&reqwest_client, id)?;
    }

    let line_counts = crate_entries.keys()
        .map(|&id| {
            let mut languages = Languages::new();
            let config = tokei::Config::default();
            languages.get_statistics(&[format!("{}/{id}", paths::CRATE)], &[], &config);
            let stats = languages.get(&LanguageType::Rust)
                .ok_or_else(|| anyhow!("Failed to get line count statistics for {id}"))?
                .summarise();
            Ok((id, stats))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    let cargo_tomls = crate_entries.keys()
        .map(|&id| get_or_scrape_cargo_toml(id).map(|table| (id, table)))
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    let dependency_graphs = cargo_tomls.iter()
        .map(|(id, table)| {
            feature_dependencies::from_cargo_toml(table)
                .map(|table| (*id, table))
                .with_context(|| format!("Failed to create dependency graph for {id}"))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    let mut rng = StdRng::seed_from_u64(123);
    let configuration_sets = dependency_graphs.iter()
        .map(|(id, graph)| get_or_scrape_configurations(id, graph, &mut postgres_client, &mut rng).map(|c| (*id, c)))
        .filter_ok(|(_, configs)| !configs.is_empty())
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    println!("Creating flat models...");

    for (id, table) in cargo_tomls.iter() {
        feature_model::create_flat(id, table)?;
    }

    println!("Creating fca models...");

    for (id, configurations) in configuration_sets.iter().filter(|(_id, configs)| configs.len() >= MIN_CONFIGS) {
        feature_model::create_fca(id, configurations)?
    }

    println!("Calculating feature and feature dependency counts...");

    let feature_stats = dependency_graphs.iter()
        .map(|(&id, graph)| (id, (graph.node_count(), graph.edge_count())))
        .collect::<BTreeMap<_, _>>();

    let feature_counts = dependency_graphs.iter()
        .map(|(&id, graph)| (id, graph.node_count()))
        .collect::<BTreeMap<_, _>>();

    println!("Calculating configuration counts...");

    let configuration_counts = dependency_graphs.iter()
        .left_join(configuration_sets.iter())
        .map(|(&id, (graph, configs))| {
            let default_features = implied_features::from_dependency_graph(std::iter::once("default"), graph);
            let configs_slice = configs.map(|c| c.as_slice());
            let stats = get_configuration_stats(configs_slice, &default_features);
            (id, stats)
        })
        .collect::<BTreeMap<_, _>>();

    println!("Calculating config stats for flat models...");

    let flat_model_config_stats = feature_counts.iter()
        .filter(|&(_id, &features)| features < MAX_FEATURES)
        .map(|(&id, _features)| (id, PathBuf::from(format!("data/model/flat/{id}.uvl"))))
        .map(|(id, path)| get_model_configuration_stats(&mut flamapy_client, &path).map(|s| (id, s)))
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    println!("Calculating config stats for fca models...");

    let fca_models = || feature_counts.iter()
        .filter(|&(_id, &features)| features < MAX_FEATURES)
        .map(|(id, _features)| (id, PathBuf::from(format!("data/model/fca/{id}.uvl"))))
        .filter(|(_id, path)| path.exists())
        .assume_sorted_by_key();

    let fca_model_config_stats = fca_models()
        .map(|(&id, path)| get_model_configuration_stats(&mut flamapy_client, &path).map(|s| (id, s)))
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    println!("Calculating fca quality...");

    let fca_model_quality = fca_models()
        .join(configuration_sets.iter())
        .map(|(&id, (path, configs))| {
            flamapy_client.set_model(&path)
                .with_context(|| format!("Failed to set model to {path:?}"))?;
            let test_configs = &configs[configs.len() / 10..];
            let satified_configurations = number_of_satisfied_configurations(&mut flamapy_client, id, test_configs)?;
            let quality = satified_configurations as f64 / test_configs.len() as f64;
            Ok((id, quality))
        })
        .map_ok(|(id, quality)| {
            println!("{id}");
            (id, quality)
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
    
    let date_time = Local::now().naive_local();
    
    println!("Creating csv files...");
    let result_directory = PathBuf::from(format!("{}/{}", paths::RESULT_ROOT, date_time));
    std::fs::create_dir(&result_directory)
        .with_context(|| "Failed to create directory for results of this analysis")?;

    tables::write_feature_stats(&result_directory, &feature_stats)?;
    tables::write_configuration_stats(&result_directory, &configuration_counts)?;
    tables::write_flat_model_config_stats(&result_directory, &flat_model_config_stats)?;
    tables::write_fca_model_config_stats(&result_directory, &fca_model_config_stats)?;
    tables::write_fca_model_quality(&result_directory, &fca_model_quality)?;

    let plot_dir = PathBuf::from(format!("{}/{}", paths::PLOT_ROOT, date_time));
    std::fs::create_dir(&plot_dir)
        .with_context(|| "Failed to create directory for plots of this analysis")?;

    println!("Creating features_and_dependencies.png...");
    plots::features_and_dependencies(&plot_dir, &feature_stats)?;
    println!("Creating line_count_and_features.png...");
    plots::line_count_and_features(&plot_dir, &line_counts, &feature_stats)?;
    println!("Creating flat_vs_fca.png...");
    plots::flat_vs_fca_exact(&plot_dir, &flat_model_config_stats, &fca_model_config_stats)?;

    println!("Average number of features: {}", feature_counts.values().sum::<usize>() as f64 / feature_counts.len() as f64);
    println!("Average number of feature dependencies: {}", feature_stats.values().map(|x| x.1).sum::<usize>() as f64 / feature_stats.len() as f64);
    println!("Average quality: {}", fca_model_quality.values().sum::<f64>() / fca_model_quality.len() as f64);

    Ok(())
}

fn get_or_scrape_crate_entries(client: &mut postgres::Client) -> anyhow::Result<Vec<CrateEntry>> {
    if let Ok(content) = std::fs::read_to_string(paths::CRATE_ENTRIES) {
        content.lines()
            .map(|line| line.parse())
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("Expected to parse {} as a list of crates", paths::CRATE_ENTRIES))
    } else {
        println!("Scraping {} popular crates from crates.io...", NUMBER_OF_CRATES);

        let entries = crate_scraper::scrape_popular_by_configurations(client, NUMBER_OF_CRATES as i64)
            .expect("Failed to scrape popular crates");

        let file = File::create(paths::CRATE_ENTRIES)
            .with_context(|| format!("Failed to create file {}", paths::CRATE_ENTRIES))?;

        let mut writer = BufWriter::new(file);

        for entry in entries.iter() {
            writeln!(writer, "{}", entry)
                .with_context(|| format!("Failed to write to file {}", paths::CRATE_ENTRIES))?;
        }

        Ok(entries)
    }
}

fn download_crate(client: &reqwest::blocking::Client, id: &CrateId) -> anyhow::Result<()> {
    let path = PathBuf::from(format!("{}/{id}", paths::CRATE));
    if !std::fs::exists(&path)? {
        println!("Downloading {}", id);
        let version_str = id.version.to_string();
        let request = || cargo_toml::download(client, &id.name, &version_str);
        let error_reporter = |attempt, _error| println!("Failed attempt {} at downloading Cargo.toml for {id}, {} attempts left", attempt, 3 - attempt);
        let mut archive = retry(5, request, error_reporter)
            .with_context(|| format!("Failed to download Cargo.toml for {id}"))?;
        archive.unpack(paths::CRATE)?;
        let unpack_path = Path::new(paths::CRATE).join(format!("{}-{}", id.name, id.version));
        std::fs::rename(unpack_path, path)?;
    }

    Ok(())
}

fn get_or_scrape_cargo_toml(id: &CrateId) -> anyhow::Result<toml::Table> {
    let path = PathBuf::from(format!("{}/{id}/Cargo.toml", paths::CRATE));
    let content = std::fs::read_to_string(&path)?;
    content.parse()
        .with_context(|| format!("Failed to parse Cargo.toml for {id}"))
}

fn get_or_scrape_configurations<R: Rng>(id: &CrateId, dependency_graph: &feature_dependencies::Graph, client: &mut postgres::Client, rng: &mut R) -> anyhow::Result<Vec<Configuration<'static>>> {
    let path = PathBuf::from(format!("{}/{id}", paths::CONFIG));
    let mut configurations = if let Ok(entries) = std::fs::read_dir(&path) {
        println!("Collecting configurations for {id}...");

        entries.map(|r| r.with_context(|| format!("Failed to get entry in {path:?}")))
            .map(|r| r.and_then(|entry| read_configuration(&entry.path())))
            .collect::<anyhow::Result<Vec<_>>>()?
    } else {
        std::fs::create_dir(&path)
            .with_context(|| format!("Failed to create directory {path:?}"))?;

        println!("Scraping configurations for {id}...");

        let configurations = configuration_scraper::scrape(
            &id.name, 
            &id.version, 
            dependency_graph, 
            client, 
            MAX_CONFIGS
        ).with_context(|| format!("Failed to query for configuration for {id}"))?;

        println!("Found {} configurations", configurations.len());

        for configuration in configurations.iter() {
            let config_path = path.join(format!("{}@{}.csvconf", configuration.name, configuration.version));
            std::fs::write(&config_path, configuration.to_csv())
                .with_context(|| format!("Failed to write to configuration file {path:?}"))?;
        }

        configurations
    };

    configurations.sort_by(|a, b| a.name.cmp(&b.name));
    // configurations.shuffle(rng);
    Ok(configurations)
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

pub struct ModelConfigurationStats {
    estimation: f64,
    exact: f64,
}

fn get_model_configuration_stats(client: &mut flamapy_client::Client, path: &Path) -> anyhow::Result<ModelConfigurationStats> {
    client.set_model(path)
        .with_context(|| format!("Failed to set model to {path:?}"))?;

    let estimation = client.estimated_number_of_configurations()
        .with_context(|| format!("Failed to get estimated number of configurations for {path:?}"))?;

    let exact = client.configurations_number()
        .with_context(|| format!("Failed to get configuration number for {path:?}"))?;

    Ok(ModelConfigurationStats { estimation, exact })
}

fn number_of_satisfied_configurations(client: &mut flamapy_client::Client, id: &CrateId, configurations: &[Configuration<'static>]) -> anyhow::Result<usize> {
    configurations.iter()
        .inspect(|c| println!("\t{}@{}", c.name, c.version))
        .map(|config| {
            let path = PathBuf::from(format!("data/configuration/{id}/{}@{}.csvconf", config.name, config.version));
            client.satisfiable_configuration(&path)
                .map(|b| b as usize)
                .with_context(|| format!("Failed to check for satisfiable configuration for {}@{} for {id}", config.name, config.version))
        })
        .fold_ok(0, |acc, x| acc + x)
}