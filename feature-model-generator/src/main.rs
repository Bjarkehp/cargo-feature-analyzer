mod feature_dependencies;
mod dependency;
mod uvl;
mod directed_graph;
mod max_tree;
mod configuration;
mod concept;

use std::{collections::{BTreeSet, HashMap}, error::Error, fs::{self, File}, io::{stdin, BufWriter, Write}, path::{Path, PathBuf}};

use clap::Parser;
use dependency::Dependency;
use itertools::Itertools;
use petgraph::dot::Dot;
use walkdir::WalkDir;

/// Generates an ac-poset from a set of configurations at a specified directory.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    cargo_toml: PathBuf,
    source: PathBuf,
    destination: PathBuf,

    #[arg(short, long, default_value_t = false)]
    force: bool
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if !args.force && fs::exists(&args.destination)? && !confirm_overwrite(&args.destination) {
        return Err("User declined operation.".into());
    }
    let timer = std::time::Instant::now();
    let source_toml = fs::read_to_string(&args.cargo_toml)?;
    let toml_table = source_toml.parse()?;
    let feature = configuration::name(&toml_table)
        .ok_or("Specified cargo file does not have a name")?;
    let dependency_graph = feature_dependencies::from_cargo_toml(&toml_table)?;
    println!("{} ms: Created dependency graph", timer.elapsed().as_millis());
    let configuration_tables = configuration::load_tables(args.source);
    let configurations = configuration_tables.iter()
        .filter_map(|table| configuration::from(table, feature, &dependency_graph))
        .collect::<Vec<_>>();
    println!("{} ms: Parsed configurations", timer.elapsed().as_millis());
    let ac_poset = concept::ac_poset(&configurations);
    println!("{} ms: Created ac-poset", timer.elapsed().as_millis());
    
    let uvl_file = File::create(args.destination.with_extension("uvl"))?;
    let mut uvl_writer = BufWriter::new(uvl_file);
    uvl::write_ac_poset(&mut uvl_writer, &ac_poset)?;
    uvl_writer.flush()?;
    println!("{} ms: Wrote feature model", timer.elapsed().as_millis());

    let graphviz_config = [
        petgraph::dot::Config::EdgeNoLabel,
        petgraph::dot::Config::RankDir(petgraph::dot::RankDir::BT)
    ];

    let graphviz = Dot::with_attr_getters(
        &ac_poset, 
        &graphviz_config, 
        &|_, _edge| "".to_string(), 
        &|_, _node| "shape=box".to_string()
    );

    fs::write(args.destination.with_extension("acposet.dot"), format!("{:#?}", graphviz))?;
    println!("{} ms: Wrote graphviz representation of ac-poset", timer.elapsed().as_millis());

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

fn confirm_overwrite(path: impl AsRef<Path>) -> bool {
    println!("Are you sure you want to overwrite {}? [Y/n] ", path.as_ref().display());

    loop {
        let mut buffer = String::new();
        if stdin().read_line(&mut buffer).is_ok() {
            match buffer.trim() {
                "" => return true,
                "y" | "Y" => return true,
                "n" | "N" => return false,
                _ => continue
            }
        } else {
            return false
        }
    }
}