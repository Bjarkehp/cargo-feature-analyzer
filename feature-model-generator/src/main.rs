mod feature_dependencies;
mod dependency;
mod uvl;
mod directed_graph;
mod max_tree;
mod configuration;
mod concept;

use std::{collections::{BTreeSet, HashMap}, error::Error, fs::{self, File}, io::{BufWriter, Write}};

use dependency::Dependency;
use itertools::Itertools;
use petgraph::dot::Dot;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn Error>> {
    let configuration_tables = configuration::load_tables("../feature-configuration-scraper/example-configurations");
    let configurations = configuration_tables.iter()
        .filter_map(|table| configuration::from(table, "tokio"))
        .collect::<Vec<_>>();
    let graph = concept::ac_poset(&configurations);

    let graphviz_config = [
        petgraph::dot::Config::EdgeNoLabel,
        petgraph::dot::Config::RankDir(petgraph::dot::RankDir::BT)
    ];

    let graphviz = Dot::with_attr_getters(
        &graph, 
        &graphviz_config, 
        &|_, _edge| "".to_string(), 
        &|_, _node| "shape=box".to_string()
    );

    fs::write("dot.dot", format!("{:#?}", graphviz))?;

    Ok(())
}

#[allow(dead_code)]
fn feature_model_from_only_cargo_toml() -> Result<(), Box<dyn Error>> {
    let source_toml = include_str!("../examples/toml/tokio.toml");
    let toml_table = source_toml.parse()?;
    let graph = feature_dependencies::from_cargo_toml(&toml_table)?;
    
    let tokio_uvl = File::create("tokio.uvl")?;
    let mut tokio_uvl_writer = BufWriter::new(tokio_uvl);
    uvl::write(&mut tokio_uvl_writer, &graph)?;
    tokio_uvl_writer.flush()?;

    Ok(())
}

#[allow(dead_code)]
fn map_of_occurrences() -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();
    let configurations = WalkDir::new("../feature-configuration-scraper/configurations")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_name().to_str().unwrap().ends_with(".toml"))
        .filter_map(|file| std::fs::read_to_string(file.path()).ok())
        .filter_map(|toml| toml.as_str().parse().ok())
        .collect::<Vec<_>>();

    // Normal dependencies
    configurations.iter()
        .filter_map(|table| configuration::extract_features(table, "tokio").ok())
        .map(|hset| hset.into_iter().map(Dependency::name).collect::<BTreeSet<_>>())
        .for_each(|set| { map.entry(set).and_modify(|n| *n += 1).or_insert(1); });

    // Dev dependencies
    configurations.iter()
        .filter_map(|table| configuration::extract_dev_features(table, "tokio").ok())
        .map(|hset| hset.into_iter().map(Dependency::name).collect::<BTreeSet<_>>())
        .for_each(|set| { map.entry(set).and_modify(|n| *n += 1).or_insert(1); });

    map.into_iter()
        .sorted_by_key(|(_k, v)| -v)
        .for_each(|(k, v)| println!("{:?}: {}", k, v));

    Ok(())
}