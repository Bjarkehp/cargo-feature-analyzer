mod feature_dependencies;
mod dependency;
mod uvl;
mod directed_graph;
mod max_tree;
mod feature_configuration;

use std::{collections::{BTreeSet, HashMap}, fs::File, io::{BufWriter, Write}};

use dependency::Dependency;
use itertools::Itertools;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_toml = include_str!("../examples/toml/tokio.toml");
    let tokio_uvl = File::create("tokio.uvl")?;
    let mut tokio_uvl_writer = BufWriter::new(tokio_uvl);

    let toml_table = source_toml.parse()?;
    let graph = feature_dependencies::from_cargo_toml(&toml_table)?;
    // uvl::write(&mut tokio_uvl_writer, &graph)?;
    // tokio_uvl_writer.flush()?;

    let mut map = HashMap::new();
    let configurations = WalkDir::new("../feature-configuration-scraper/configurations")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_name().to_str().unwrap().ends_with(".toml"))
        .filter_map(|file| std::fs::read_to_string(file.path()).ok())
        .filter_map(|toml| toml.as_str().parse().ok())
        .collect::<Vec<_>>();

    configurations.iter()
        .filter_map(|table| feature_configuration::extract_features(table, "tokio").ok())
        .map(|hset| hset.into_iter().map(Dependency::name).collect::<BTreeSet<_>>())
        .for_each(|set| { map.entry(set).and_modify(|n| *n += 1).or_insert(1); });

    configurations.iter()
        .filter_map(|table| feature_configuration::extract_dev_features(table, "tokio").ok())
        .map(|hset| hset.into_iter().map(Dependency::name).collect::<BTreeSet<_>>())
        .for_each(|set| { map.entry(set).and_modify(|n| *n += 1).or_insert(1); });

    map.into_iter()
        .sorted_by_key(|(_k, v)| -v)
        .for_each(|(k, v)| println!("{:?}: {}", k, v));

    Ok(())
}