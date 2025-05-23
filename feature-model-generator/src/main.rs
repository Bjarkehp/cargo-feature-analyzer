mod uvl;
mod max_tree;
mod concept;

use std::{collections::{BTreeSet, HashMap}, error::Error, fs::{self, File}, io::{stdin, BufWriter, Write}, path::{Path, PathBuf}};

use clap::Parser;
use concept::Concept;
use configuration::dependency::Dependency;
use itertools::Itertools;
use petgraph::{dot::Dot, graph::DiGraph};
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

    let source_toml = fs::read_to_string(&args.cargo_toml)?;
    let toml_table = source_toml.parse()?;
    let crate_name = configuration::name(&toml_table)
        .ok_or("Specified cargo file does not have a name")?;
    let dependency_graph = configuration::feature_dependencies::from_cargo_toml(&toml_table)?;
    let features = dependency_graph.nodes()
        .filter(|d| matches!(d, Dependency::Feature(_)))
        .map(|d| d.name())
        .collect::<Vec<_>>();

    let configuration_tables = configuration::load_tables(args.source);
    let configurations = configuration_tables.iter()
        .filter_map(|table| configuration::from(table, crate_name, &dependency_graph))
        .collect::<Vec<_>>();

    let ac_poset = concept::ac_poset(&configurations, &features);
    
    let uvl_file = File::create(args.destination)?;
    let mut uvl_writer = BufWriter::new(uvl_file);
    uvl::write_ac_poset(&mut uvl_writer, &ac_poset, crate_name)?;
    uvl_writer.flush()?;

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

#[allow(dead_code)]
fn write_ac_poset(ac_poset: &DiGraph<Concept, ()>, destination: impl AsRef<Path>) -> std::io::Result<()> {
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

    fs::write(destination, format!("{:#?}", graphviz))?;

    Ok(())
}