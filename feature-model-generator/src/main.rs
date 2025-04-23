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

fn concept_latice_feature_model() -> Result<(), Box<dyn Error>> {
    let configurations = WalkDir::new("../feature-configuration-scraper/configurations")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_name().to_str().unwrap().ends_with(".toml"))
        .filter_map(|file| std::fs::read_to_string(file.path()).ok())
        .filter_map(|toml| toml.as_str().parse().ok())
        .collect::<Vec<Table>>();

    let configuration_features = configurations.iter()
        .filter_map(|table| Some((table.get("name")?.as_str()?, feature_configuration::extract_features(table, "tokio").ok()?)))
        .map(|(name, hset)| (name, hset.into_iter().map(Dependency::name).collect::<BTreeSet<_>>()))
        .collect::<HashMap<_, _>>();

    let concepts = configurations.iter()
        .filter_map(|table| Some((table.get("name")?.as_str()?, feature_configuration::extract_features(table, "tokio").ok()?)))
        .map(|(name, hset)| (hset.into_iter().map(Dependency::name).collect::<BTreeSet<_>>(), name))
        .into_grouping_map().collect::<BTreeSet<&str>>()
        .into_iter()
        .map(|(intent, extent)| Concept::new(intent, extent))
        .collect::<Vec<_>>();
        
    //let mut graph = DiGraph::default();
    
    // for concept in concepts.iter() {
    //     graph.add_node(1);
    // }

    let mut edges = Vec::new();
    let test = &[
        (1, 2), (2, 3), (3, 4),
        (1, 4)];

    for (i, a) in concepts.iter().enumerate() {
        for (j, b) in concepts.iter().enumerate() {
            if a != b && a.intent.is_subset(&b.intent) && b.extent.is_subset(&a.extent) {
                edges.push((i as u32, j as u32));
            }
        }
    }

    let graph = DiGraph::<u32, ()>::from_edges(edges);

    Ok(())
}

#[derive(PartialEq, Eq, new, Default)]
pub struct Concept<'a> {
    pub intent: BTreeSet<&'a str>,
    pub extent: BTreeSet<&'a str>
}