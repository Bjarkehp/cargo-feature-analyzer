mod flamapy_client;
mod feature_model;
mod retry;

use std::{collections::BTreeSet, path::Path};

use analysis::{args::Args, config::config_from_args, paths::{self, Paths}, result::{configuration_stats::ConfigStats, feature_stats::FeatureStats, line_count::LineCountRow, model_stats::ModelStats, satisfiability_crate_row::SatisfiabilityCrateRow, satisfiability_row::SatisfiabilityRow}};
use anyhow::Context;
use cargo_toml::{crate_id::CrateId, feature_dependencies, implied_features};
use clap::Parser;
use configuration_scraper::{configuration::Configuration, postgres};
use crate_scraper::crate_entry::CrateEntry;
use ::feature_model::FeatureModel;
use itertools::Itertools;
use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use tokei::{LanguageType, Languages};

use crate::retry::retry;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = config_from_args(args)?;
    let paths = paths::prepare_paths(&config)?;
    let tokei_config = tokei::Config::default();
    let mut rng = StdRng::seed_from_u64(123);

    let mut postgres_client = postgres::Client::connect(&config.connection_string, postgres::NoTls)
        .with_context(|| "Failed to create postgres client")?;
    let mut flamapy_client = flamapy_client::Client::new(&paths.flamapy_server)
        .with_context(|| "Failed to create flamapy client")?;
    let reqwest_client = cargo_toml::default_reqwest_client()
        .with_context(|| "Failed to create reqwest client")?;

    let mut feature_stats_writer = csv::Writer::from_path(paths.result.join("feature_stats.csv"))?;
    let mut static_model_stats_writer = csv::Writer::from_path(paths.result.join("flat_model_stats.csv"))?;
    let mut fca_model_stats_writer = csv::Writer::from_path(paths.result.join("fca_model_stats.csv"))?;
    let mut config_stats_writer = csv::Writer::from_path(paths.result.join("configuration_stats.csv"))?;
    let mut line_count_writer = csv::Writer::from_path(paths.result.join("line_count.csv"))?;
    let mut running_time_fca_writer = csv::Writer::from_path(paths.result.join("running_time_fca.csv"))?;
    let mut running_time_static_writer = csv::Writer::from_path(paths.result.join("running_time_static.csv"))?;

    for &count in &config.synthesis_configs {
        std::fs::create_dir_all(paths.satisfiability_crate.join(count.to_string()))?;
    }

    let mut satisfiability_writers = config.synthesis_configs
        .iter()
        .map(|count| paths.result.join("satisfiability").join(format!("{count}.csv")))
        .map(csv::Writer::from_path)
        .collect::<Result<Vec<_>, _>>()?;

    let crate_entries = get_or_scrape_crate_entries(&mut postgres_client, config.number_of_crates, &paths)?
        .into_iter()
        .sorted_by(|a, b| a.id.cmp(&b.id));

    for entry in crate_entries {
        let id = entry.id;
        let id_str = id.to_string();

        println!("Analyzing {id_str}");

        download_crate(&reqwest_client, &id, &paths)?;

        let line_count_row = get_line_count(&id, &tokei_config, &paths)?;
        let cargo_toml = get_cargo_toml(&id, &paths)?;
        let dependency_graph = feature_dependencies::from_cargo_toml(&cargo_toml)
            .with_context(|| format!("Failed to create dependency graph for {id}"))?;

        let feature_count = dependency_graph.node_count();
        let feature_dependency_count = dependency_graph.edge_count();
        let default_features = implied_features::from_dependency_graph(["default"].into_iter(), &dependency_graph);
        let feature_stats = FeatureStats::new(id.clone(), feature_count, feature_dependency_count);

        if feature_count > config.max_features || feature_count < config.min_features {
            continue;
        }
        
        let crate_configs = if !config.distinct_configs {
            Some(get_or_scrape_configurations(&mut postgres_client, &id, &dependency_graph, &paths, config.max_configs, &mut rng)?)
        } else {
            None
        };
        
        let mut distinct_crate_configs = get_or_scrape_distinct_configurations(&mut postgres_client, &id, &dependency_graph, &paths, config.max_configs, &mut rng)?;
        let config_stats = if let Some(configs) = &crate_configs {
            get_configuration_stats(&id, configs, &default_features)
        } else {
            ConfigStats::new(id.clone(), 0, 0, distinct_crate_configs.len())
        };

        if distinct_crate_configs.len() < config.min_configs {
            continue;
        }

        let (static_model, running_time_static) = feature_model::create_static(&id, &cargo_toml, &paths)?;
        let static_model_path = paths.static_model.join(format!("{id_str}.uvl"));
        let static_model_stats = get_model_stats(&mut flamapy_client, &id, &static_model_path, &static_model)?;

        let (fca_model, running_time_fca) = feature_model::create_fca(&id, &distinct_crate_configs, &paths)?;
        let fca_model_path = paths.fca_model.join(format!("{id_str}.uvl"));
        let fca_model_stats = get_model_stats(&mut flamapy_client, &id, &fca_model_path, &fca_model)?;

        #[allow(clippy::needless_range_loop)]
        for synthesis_config_index in 0..config.synthesis_configs.len() {
            let synthesis_config_count = config.synthesis_configs[synthesis_config_index];

            if synthesis_config_count as f32 > distinct_crate_configs.len() as f32 * config.train_test_split {
                continue;
            }

            let train_range = 0..synthesis_config_count;
            let test_range = synthesis_config_count..distinct_crate_configs.len();

            let satisfiablity_path = paths.satisfiability_crate
                .join(synthesis_config_count.to_string())
                .join(id.to_string())
                .with_extension("csv");
            let mut satisfiability_crate_writer = csv::Writer::from_path(satisfiablity_path)?;
            let mut total_satisfied_test_configurations = 0;

            for _ in 0..config.permutations {
                distinct_crate_configs.shuffle(&mut rng);
                let crate_train_configs = &distinct_crate_configs[train_range.clone()];
                let crate_test_configs = &distinct_crate_configs[test_range.clone()];
                feature_model::create_fca(&id, crate_train_configs, &paths)?;
                let satisfied_test_configurations = number_of_satisfied_configurations(&mut flamapy_client, &id, &fca_model_path, crate_test_configs, &paths)?;
                let satisfiability = satisfied_test_configurations as f64 / test_range.len() as f64;
                let row = SatisfiabilityCrateRow::new(satisfiability);
                satisfiability_crate_writer.serialize(row)?;
                total_satisfied_test_configurations += satisfied_test_configurations;
            }

            satisfiability_crate_writer.flush()?;

            let average_satisfiability = total_satisfied_test_configurations as f64 / (test_range.len() as f64 * config.permutations as f64);
            let satisfiability_row = SatisfiabilityRow::new(id.clone(), average_satisfiability);
            satisfiability_writers[synthesis_config_index].serialize(satisfiability_row)?;
        }

        feature_stats_writer.serialize(feature_stats)?;
        static_model_stats_writer.serialize(static_model_stats)?;
        fca_model_stats_writer.serialize(fca_model_stats)?;
        config_stats_writer.serialize(config_stats)?;
        line_count_writer.serialize(line_count_row)?;
        running_time_fca_writer.serialize(running_time_fca)?;
        running_time_static_writer.serialize(running_time_static)?;
    }

    feature_stats_writer.flush()?;
    static_model_stats_writer.flush()?;
    fca_model_stats_writer.flush()?;
    config_stats_writer.flush()?;
    line_count_writer.flush()?;
    running_time_fca_writer.flush()?;
    running_time_static_writer.flush()?;

    for mut writer in satisfiability_writers {
        writer.flush()?;
    }

    Ok(())
}

fn get_or_scrape_crate_entries(client: &mut postgres::Client, number_of_crates: usize, paths: &Paths) -> anyhow::Result<Vec<CrateEntry>> {
    if let Ok(reader) = csv::Reader::from_path(&paths.crate_entries) {
        let crate_entries = reader
            .into_deserialize()
            .collect::<csv::Result<Vec<CrateEntry>>>()
            .with_context(|| format!("Expected to parse {:?} as a list of crates", paths.crate_entries))?;
 
        if number_of_crates == crate_entries.len() {
            return Ok(crate_entries);
        }
    }

    println!("Scraping {} popular crates from crates.io...", number_of_crates);

    let entries = crate_scraper::scrape_popular_by_configurations(client, number_of_crates as i64)
        .expect("Failed to scrape popular crates");

    let mut writer = csv::Writer::from_path(&paths.crate_entries)?;

    for entry in entries.iter() {
        writer
            .serialize(entry)
            .with_context(|| format!("Failed to write to file {:?}", paths.crate_entries))?;
    }

    writer.flush()?;

    Ok(entries)
}

fn download_crate(client: &reqwest::blocking::Client, id: &CrateId, paths: &Paths) -> anyhow::Result<()> {
    let path = paths.crates.join(id.to_string());
    if !std::fs::exists(&path)? {
        println!("Downloading {}", id);
        let version_str = id.version.to_string();
        let request = || cargo_toml::download(client, &id.name, &version_str);
        let error_reporter = |attempt, _error| println!("Failed attempt {} at downloading Cargo.toml for {id}, {} attempts left", attempt, 3 - attempt);
        let mut archive = retry(5, request, error_reporter)
            .with_context(|| format!("Failed to download Cargo.toml for {id}"))?;
        archive.unpack(&paths.crates)?;
        let unpack_path = paths.crates.join(format!("{}-{}", id.name, id.version));
        std::fs::rename(unpack_path, path)?;
    }

    Ok(())
}

fn get_line_count(id: &CrateId, config: &tokei::Config, paths: &Paths) -> anyhow::Result<LineCountRow> {
    let mut tokei_language = Languages::new();
    tokei_language.get_statistics(&[paths.crates.join(id.to_string())], &[], config);
    let tokei_stats = tokei_language.get(&LanguageType::Rust)
        .with_context(|| format!("Failed to get line count statistics for {id}"))?
        .summarise();
    let line_count = tokei_stats.lines();
    Ok(LineCountRow::new(id.clone(), line_count))
}

fn get_cargo_toml(id: &CrateId, paths: &Paths) -> anyhow::Result<toml::Table> {
    let path = paths.crates
        .join(id.to_string())
        .join("Cargo.toml");
    let content = std::fs::read_to_string(&path)?;
    content.parse()
        .with_context(|| format!("Failed to parse Cargo.toml for {id}"))
}

fn get_or_scrape_configurations<R: Rng>(
    client: &mut postgres::Client, 
    id: &CrateId, 
    dependency_graph: &feature_dependencies::Graph,
    paths: &Paths,
    max_configs: usize,
    rng: &mut R
) -> anyhow::Result<Vec<Configuration<'static>>> {
    let path = paths.config.join(id.to_string());
    let mut configurations = if let Ok(entries) = std::fs::read_dir(&path) {
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
            max_configs
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
    configurations.shuffle(rng);
    Ok(configurations)
}

fn get_or_scrape_distinct_configurations<R: Rng>(
    client: &mut postgres::Client, 
    id: &CrateId, 
    dependency_graph: &feature_dependencies::Graph,
    paths: &Paths,
    max_configs: usize,
    rng: &mut R
) -> anyhow::Result<Vec<Configuration<'static>>> {
    let path = paths.config_distinct.join(id.to_string());
    let mut configurations = if let Ok(entries) = std::fs::read_dir(&path) {
        let all_configurations = entries.map(|r| r.with_context(|| format!("Failed to get entry in {path:?}")))
            .map(|r| r.and_then(|entry| read_configuration(&entry.path())))
            .collect::<anyhow::Result<Vec<_>>>()?;
        
        all_configurations
            .iter()
            .unique_by(|c| &c.features)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        std::fs::create_dir(&path)
            .with_context(|| format!("Failed to create directory {path:?}"))?;

        println!("Scraping configurations for {id}...");

        let configurations = configuration_scraper::scrape(
            &id.name, 
            &id.version, 
            dependency_graph, 
            client, 
            max_configs
        ).with_context(|| format!("Failed to query for configuration for {id}"))?;

        let distinct_configurations = configurations
            .iter()
            .unique_by(|c| &c.features)
            .cloned()
            .collect::<Vec<_>>();

        println!("Found {} distinct configurations", distinct_configurations.len());

        for configuration in distinct_configurations.iter() {
            let config_path = path.join(format!("{}@{}.csvconf", configuration.name, configuration.version));
            std::fs::write(&config_path, configuration.to_csv())
                .with_context(|| format!("Failed to write to configuration file {path:?}"))?;
        }

        distinct_configurations
    };

    configurations.sort_by(|a, b| a.name.cmp(&b.name));
    configurations.shuffle(rng);
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

fn get_configuration_stats(id: &CrateId, configs: &[Configuration<'static>], default_features: &BTreeSet<&str>) -> ConfigStats {
    let configuration_count = configs.len();

    let default_configuration_count = configs
        .iter()
        .filter(|config| config.features.iter().all(|(feature, &enabled)| default_features.contains(feature.as_ref()) == enabled))
        .count();

    let distinct_configuration_count = configs.iter()
        .into_group_map_by(|config| &config.features)
        .len();

    ConfigStats::new(id.clone(), configuration_count, default_configuration_count, distinct_configuration_count)
}

fn get_model_stats(client: &mut flamapy_client::Client, id: &CrateId, path: &Path, model: &FeatureModel) -> anyhow::Result<ModelStats> {
    client.set_model(path)
        .with_context(|| format!("Failed to set model to {path:?}"))?;

    let features = model.count_features();
    let cross_tree_constraints = model.cross_tree_constraints.len();

    let config_estimation = client.estimated_number_of_configurations()
        .with_context(|| format!("Failed to get estimated number of configurations for {path:?}"))?;

    let config_exact = client.configurations_number()
        .with_context(|| format!("Failed to get configuration number for {path:?}"))?;

    Ok(ModelStats::new(id.clone(), features, cross_tree_constraints, config_estimation, config_exact))
}

fn number_of_satisfied_configurations(client: &mut flamapy_client::Client, id: &CrateId, model: &Path, configurations: &[Configuration], paths: &Paths) -> anyhow::Result<usize> {
    client.set_model(model)
        .with_context(|| format!("Failed to set model to {model:?}"))?;

    configurations
        .iter()
        .map(|configuration| {
            let path = paths.config_distinct
                .join(format!("{id}/{}@{}.csvconf", configuration.name, configuration.version));

            client.satisfiable_configuration(&path)
                .map(|b| b as usize)
                .with_context(|| format!("Failed to check for satisfiable configuration (model: {}, configuration: {:?})", id, path))
        })
        .fold_ok(0, |a, b| a + b)
}