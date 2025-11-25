use std::{collections::{HashMap, HashSet}, io::Write};

use cargo_toml::{feature_dependencies, toml_util};
use itertools::Itertools;
use petgraph::{Direction, prelude::DiGraphMap};

pub struct Constraints<'a> {
    pub tree: HashMap<&'a str, Vec<&'a str>>,
    pub cross_tree: HashMap<&'a str, Vec<&'a str>>,
}

pub fn from_cargo_toml(table: &toml::Table) -> Result<Constraints<'_>, toml_util::Error> {
    let mut feature_dependencies = feature_dependencies::from_cargo_toml(table)?;

    let name = table.get("package")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| toml_util::Error::KeyMissing("package.name is missing in toml".to_string()))?;

    feature_dependencies.add_node(name);
    for feature in feature_dependencies.nodes().collect::<Vec<_>>() {
        if feature != name {
            feature_dependencies.add_edge(feature, name, ());
        }
    }

    Ok(dependency_graph_constraints(&feature_dependencies, name))
}

fn dependency_graph_constraints<'a, E>(graph: &DiGraphMap<&'a str, E>, root: &'a str) -> Constraints<'a> {
    let mut tree_edges = vec![];
    let mut cross_tree_edges = vec![];
    let mut stack = graph.neighbors_directed(root, Direction::Incoming)
        .map(|neighbor| (root, neighbor))
        .collect::<Vec<_>>();
    let mut visited = HashSet::new();

    while let Some((feature, dependency)) = stack.pop() {
        if visited.insert(dependency) {
            tree_edges.push((feature, dependency));
            for neighbor in graph.neighbors_directed(dependency, Direction::Incoming) {
                stack.push((dependency, neighbor));
            }
        } else {
            cross_tree_edges.push((dependency, feature));
        }
    }

    let tree = tree_edges.into_iter().into_group_map();
    let cross_tree = cross_tree_edges.into_iter().into_group_map();

    Constraints { tree, cross_tree }
}

pub fn write_uvl<W: Write>(writer: &mut W, root: &str, constraints: &Constraints) -> std::io::Result<()> {
    writeln!(writer, "features")?;
    let mut visited_features = HashSet::new();
    write_tree_constraints(writer, root, &constraints.tree, &mut visited_features, 1)?;
    write_cross_tree_constraints(writer, &constraints.cross_tree)?;
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