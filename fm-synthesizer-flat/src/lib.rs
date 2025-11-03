use std::collections::HashSet;
use std::hash::Hash;

use cargo_toml::{feature_dependencies, toml_util};
use petgraph::{Direction, prelude::DiGraphMap};

pub struct Constraints<'a> {
    tree: Vec<(&'a str, &'a str)>,
    cross_tree: Vec<(&'a str, &'a str)>,
}

pub fn from_cargo_toml<'a>(table: &'a toml::Table) -> Result<Constraints<'a>, toml_util::Error> {
    let mut feature_dependencies = feature_dependencies::from_cargo_toml(&table)?;

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
            cross_tree_edges.push((feature, dependency));
        }
    }

    Constraints { tree: tree_edges, cross_tree: cross_tree_edges }
}