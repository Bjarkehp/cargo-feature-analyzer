pub mod flamapy_client;

use std::{collections::{BTreeMap, BTreeSet}, fs::File, io::{BufWriter, Write}, path::{Path, PathBuf}};

use anyhow::{Context, anyhow};
use cargo_toml::{crate_id::CrateId, feature_dependencies, implied_features};
use chrono::Local;
use configuration_scraper::{configuration::Configuration, postgres};
use crate_scraper::crate_entry::CrateEntry;
use fm_synthesizer_fca::{concept, uvl};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use plotters::{chart::ChartBuilder, prelude::{BitMapBackend, Circle, EmptyElement, IntoDrawingArea, Text}, series::{LineSeries, PointSeries}, style::{IntoFont, RED, WHITE}};
use sorted_iter::{SortedPairIterator, assume::AssumeSortedByKeyExt};

const CRATE_ENTRIES_PATH: &str = "data/crates.txt";
const TOML_PATH: &str = "data/toml";
const CONFIG_PATH: &str = "data/configuration";
const FLAT_MODEL_PATH: &str = "data/model/flat";
const FCA_MODEL_PATH: &str = "data/model/fca";
const RESULT_ROOT_PATH: &str = "data/result";
const PLOT_ROOT_PATH: &str = "data/plot";

const POSTGRES_CONNECTION_STRING: &str = "postgres://crates:crates@localhost:5432/crates_io_db";

fn main() -> anyhow::Result<()> {
    for path in [TOML_PATH, CONFIG_PATH, FLAT_MODEL_PATH, FCA_MODEL_PATH, RESULT_ROOT_PATH, PLOT_ROOT_PATH] {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory {path}"))?;
    }

    let mut postgres_client = postgres::Client::connect(POSTGRES_CONNECTION_STRING, postgres::NoTls)
        .with_context(|| anyhow!("Failed to create postgres client"))?;

    let flamapy_server = Path::new("analysis/src/flamapy_server.py");
    let mut flamapy_client = flamapy_client::Client::new(flamapy_server)
        .with_context(|| "Failed to create flamapy client")?;

    let crate_entries_vec = get_or_scrape_crate_entries(&mut postgres_client)?;

    let crate_entries = crate_entries_vec.iter()
        .map(|e| (&e.id, &e.data))
        .collect::<BTreeMap<_, _>>();

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

    let configuration_sets = dependency_graphs.iter()
        .map(|(id, graph)| get_or_scrape_configurations(id, graph, &mut postgres_client).map(|c| (*id, c)))
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

    for (id, configurations) in configuration_sets.iter().filter(|(_id, configs)| configs.len() > 100) {
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

    println!("Calculating feature and feature dependency counts...");

    let feature_counts = dependency_graphs.iter()
        .map(|(&id, graph)| (id, graph.node_count()))
        .collect::<BTreeMap<_, _>>();

    let dependency_counts = dependency_graphs.iter()
        .map(|(&id, graph)| (id, graph.edge_count()))
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
        .filter(|&(_id, &features)| features < 300)
        .map(|(id, _features)| (id, PathBuf::from(format!("data/model/flat/{id}.uvl"))))
        .map(|(id, path)| get_model_configuration_stats(&mut flamapy_client, &path).map(|s| (id, s)))
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    println!("Calculating config stats for fca models...");

    let fca_models = || feature_counts.iter()
        .filter(|&(_id, &features)| features < 300)
        .map(|(id, _features)| (id, PathBuf::from(format!("data/model/fca/{id}.uvl"))))
        .filter(|(_id, path)| path.exists())
        .assume_sorted_by_key();

    let fca_model_config_stats = fca_models()
        .map(|(id, path)| get_model_configuration_stats(&mut flamapy_client, &path).map(|s| (id, s)))
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
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
    
    let date_time = Local::now().naive_local();
    let result_directory = PathBuf::from(format!("{RESULT_ROOT_PATH}/{date_time}"));
    std::fs::create_dir(&result_directory)
        .with_context(|| "Failed to create directory for results of this analysis")?;

    println!("Creating feature_stats.csv...");
    write_to_csv(
        &result_directory.join("feature_stats.csv"), 
        feature_counts.iter().join(dependency_counts.iter()), 
        &["Crate", "Features", "Feature dependencies"], 
        |writer, (&id, (&features, &dependencies))| writeln!(writer, "{id},{features},{dependencies}")
    )?;

    println!("Creating configuration_stats.csv...");
    write_to_csv(
        &result_directory.join("configuration_stats.csv"), 
        configuration_counts.iter(), 
        &["Crate", "Configurations", "Default configurations", "Unique Configurations"], 
        |writer, (&id, stats)| writeln!(writer, "{id},{},{},{}", 
            stats.configuration_count, 
            stats.default_configurations_count,
            stats.unique_configurations_count,
        )
    )?;

    println!("Creating flat_model_config_stats.csv...");
    write_to_csv(
        &result_directory.join("flat_model_config_stats.csv"), 
        flat_model_config_stats.iter(), 
        &["Crate", "Estimation", "Exact"], 
        |writer, (&id, stats)| writeln!(writer, "{id},{},{}",
            stats.estimation,
            stats.exact,
        )
    )?;

    println!("Creating fca_model_config_stats.csv...");
    write_to_csv(
        &result_directory.join("fca_model_config_stats.csv"), 
        fca_model_config_stats.iter(), 
        &["Crate", "Estimation", "Exact"], 
        |writer, (&id, stats)| writeln!(writer, "{id},{},{}",
            stats.estimation,
            stats.exact,
        )
    )?;

    println!("Creating fca_model_quality.csv...");
    write_to_csv(
        &result_directory.join("fca_model_quality.csv"), 
        fca_model_quality.iter(), 
        &["Crate", "Quality"], 
        |writer, (&id, quality)| writeln!(writer, "{id},{quality}")
    )?;

    let plot_directory = PathBuf::from(format!("{PLOT_ROOT_PATH}/{date_time}"));
    std::fs::create_dir(&plot_directory)
        .with_context(|| "Failed to create directory for plots of this analysis")?;

    println!("Creating features_and_dependencies.png...");
    make_line_chart_plot(
        &plot_directory.join("features_and_dependencies.png"),
        "Features and dependencies",
        feature_counts.iter()
            .join(dependency_counts.iter())
            .map(|(_id, (&f, &d))| (f as f64, d as f64))
            .filter(|&(f, d)| f < 100.0 && d < 1000.0)
    )?;

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

struct ModelConfigurationStats {
    estimation: f64,
    exact: f64,
}

fn get_model_configuration_stats(client: &mut flamapy_client::Client, path: &Path) -> anyhow::Result<ModelConfigurationStats> {
    client.set_model(path)
        .with_context(|| format!("Failed to set model to {path:?}"))?;

    let estimation = client.estimated_number_of_configurations()
        .with_context(|| format!("Failed to get estimated number of configurations for {path:?}"))?;

    let exact = client.configurations_number()
        .with_context(|| format!("Failed to get configration number for {path:?}"))?;

    Ok(ModelConfigurationStats { estimation, exact })
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

fn write_to_csv<T>(
    path: &Path, 
    data: impl Iterator<Item = T>, 
    columns: &[&str], 
    write_fn: impl Fn(&mut BufWriter<File>, T) -> std::io::Result<()>
) -> anyhow::Result<()> {
    {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        
        write!(writer, "{}", columns[0])?;
        for column in &columns[1..] {
            write!(writer, ",{}", column)?;
        }
        writeln!(writer)?;

        for item in data {
            write_fn(&mut writer, item)?;
        }

        writer.flush()?;

        Ok::<(), std::io::Error>(())
    }.with_context(|| format!("Failed to write results into {path:?}"))
}

fn make_line_chart_plot(
    path: &Path,
    caption: &str,
    data_iter: impl Iterator<Item = (f64, f64)>,
) -> anyhow::Result<()> {
    let data = data_iter.sorted_by_key(|&(x, _y)| OrderedFloat(x))
        .collect::<Vec<_>>();

    let x_min = *data.iter()
        .map(|&(x, _y)| OrderedFloat(x))
        .min()
        .expect("Expected some data");
    let x_max = *data.iter()
        .map(|&(x, _y)| OrderedFloat(x))
        .max()
        .expect("Expected some data");
    let y_min = *data.iter()
        .map(|&(_x, y)| OrderedFloat(y))
        .min()
        .expect("Expected some data");
    let y_max = *data.iter()
        .map(|&(_x, y)| OrderedFloat(y))
        .max()
        .expect("Expected some data");

    let root = BitMapBackend::new(path, (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let root = root.margin(10, 10, 10, 10);
    let mut chart = ChartBuilder::on(&root)
        .caption(caption, ("sans-serif", 20).into_font())
        .x_label_area_size(20)
        .y_label_area_size(40)
        .build_cartesian_2d(x_min..x_max + 1.0, y_min..y_max + 1.0)?;
    
    chart.configure_mesh()
        .x_labels(10)
        .y_labels(10)
        .draw()?;

    chart.draw_series(LineSeries::new(
        data.clone(),
        &RED,
    ))?;

    root.present()?;

    Ok(())
}