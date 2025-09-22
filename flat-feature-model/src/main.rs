use std::{collections::{HashMap, HashSet}, error::Error, fs::File, hash::Hash, io::{BufWriter, Write}, path::PathBuf};

use clap::Parser;
use configuration::feature_dependencies;
use itertools::Itertools;
use petgraph::{prelude::DiGraphMap, Direction};


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    cargo_toml: PathBuf,
    destination: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let cargo_toml_content = std::fs::read_to_string(&args.cargo_toml)?;
    let table = cargo_toml_content.parse::<toml::Table>()?;
    let mut feature_dependencies = feature_dependencies::from_cargo_toml(&table)?;

    let name = table.get("package")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .ok_or("Failed to get crate name")?;

    feature_dependencies.add_node(name);
    for feature in feature_dependencies.nodes().collect::<Vec<_>>() {
        if feature != name {
            feature_dependencies.add_edge(feature, name, ());
        }
    }

    let (tree_constraints, cross_tree_constraints) = 
        dependency_graph_constraints(&feature_dependencies, name);

    println!("{:?}", tree_constraints);
    
    let tree_constraints_grouped = tree_constraints.into_iter()
        .into_group_map();
    let cross_tree_constraints_grouped = cross_tree_constraints.into_iter()
        .map(|(a, b)| (b, a))
        .into_group_map();

    let file = File::create(args.destination)?;
    let mut writer = BufWriter::new(file);
    write_uvl(&mut writer, name, &tree_constraints_grouped, &cross_tree_constraints_grouped)?;

    Ok(())
}

#[allow(clippy::type_complexity)] // The return type is a partition into in-tree constraints and cross-tree constraints.
fn dependency_graph_constraints<N: Hash + Eq + Copy + Ord, E>(graph: &DiGraphMap<N, E>, node: N) -> (Vec<(N, N)>, Vec<(N, N)>) {
    let mut tree_edges = vec![];
    let mut cross_tree_edges = vec![];
    let mut stack = graph.neighbors_directed(node, Direction::Incoming)
        .map(|neighbor| (node, neighbor))
        .collect::<Vec<_>>();
    let mut visited = HashSet::new();

    while let Some((feature, dependency)) = stack.pop() {
        if visited.insert(dependency) {
            tree_edges.push((feature, dependency));
            for neighbor in graph.neighbors_directed(dependency, Direction::Incoming) {
                stack.push((dependency, neighbor));
            }
        } else {
            cross_tree_edges.push((feature, dependency));
        }
    }

    (tree_edges, cross_tree_edges)
}

fn write_uvl<W: Write>(writer: &mut W, root: &str, tree_constraints: &HashMap<&str, Vec<&str>>, cross_tree_constraints: &HashMap<&str, Vec<&str>>) -> std::io::Result<()> {
    writeln!(writer, "features")?;
    let mut visited_features = HashSet::new();
    write_tree_constraints(writer, root, tree_constraints, &mut visited_features, 1)?;
    write_cross_tree_constraints(writer, cross_tree_constraints)?;
    Ok(())
}

fn write_tree_constraints<'a, W: Write>(
    writer: &mut W, 
    current: &'a str, 
    constraints: &'a HashMap<&str, Vec<&str>>, 
    visited: &mut HashSet<&'a str>,
    indentation: usize,
) -> std::io::Result<()> {
    if !visited.insert(current) {
        return Ok(());
    }

    writeln!(writer, "{}\"{}\"", " ".repeat(4 * indentation), current)?;
    if let Some(dependents) = constraints.get(current) {
        writeln!(writer, "{}optional", " ".repeat(4 * (indentation + 1)))?;
        for feature in dependents {
            write_tree_constraints(writer, feature, constraints, visited, indentation + 2)?;
        }
    }
    
    Ok(())
}

fn write_cross_tree_constraints<W: Write>(writer: &mut W, constraints: &HashMap<&str, Vec<&str>>) -> std::io::Result<()> {
    writeln!(writer, "constraints")?;
    for (feature, dependencies) in constraints {
        let dependencies_str = dependencies.iter()
            .map(|s| format!("\"{}\"", s))
            .join(" & ");
        writeln!(writer, "    \"{}\" => {}", feature, dependencies_str)?;
    }

    Ok(())
}