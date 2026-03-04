use std::collections::{HashMap, HashSet};

use cargo_toml::{feature_dependencies, toml_util};
use feature_model::{FeatureModel, cross_tree_constraint::{self, CrossTreeConstraint}, feature::Feature, group::Group};
use itertools::Itertools;
use petgraph::{Direction, prelude::DiGraphMap};

pub fn fm_from_cargo_toml(table: &toml::Table) -> Result<FeatureModel, toml_util::Error> {
    let name = table
        .get("package")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| toml_util::Error::KeyMissing("package.name is missing in toml".to_string()))?;

    let mut feature_dependencies = feature_dependencies::from_cargo_toml(table)?;
    
    feature_dependencies.add_node(name);
    for feature in feature_dependencies.nodes().collect::<Vec<_>>() {
        if feature != name {
            feature_dependencies.add_edge(feature, name, ());
        }
    }

    Ok(construct_feature_model(&feature_dependencies, name))
}

fn construct_feature_model<'a, E>(graph: &DiGraphMap<&'a str, E>, root: &'a str) -> FeatureModel {
    let mut tree_edges = vec![];
    let mut cross_tree_edges = vec![];
    let mut stack = graph.neighbors_directed(root, Direction::Incoming)
        .map(|neighbor| (neighbor, root))
        .collect::<Vec<_>>();
    let mut visited = HashSet::new();

    while let Some((feature, dependency)) = stack.pop() {
        if visited.insert(feature) {
            tree_edges.push((feature, dependency));
            for neighbor in graph.neighbors_directed(feature, Direction::Incoming) {
                stack.push((neighbor, feature));
            }
        } else {
            cross_tree_edges.push((feature, dependency));
        }
    }

    let mut tree = tree_edges
        .into_iter()
        .map(|(feature, dependency)| (dependency, feature))
        .into_group_map();

    let root_feature = construct_feature_diagram(root, &mut tree);

    let cross_tree_constraints = cross_tree_edges
        .into_iter()
        .into_group_map()
        .into_iter()
        .map(|(feature, dependencies)| feature_dependencies_implication(feature, &dependencies))
        .collect::<Vec<_>>();

    FeatureModel::new(root_feature, cross_tree_constraints)
}

fn construct_feature_diagram(dependency: &str, tree: &mut HashMap<&str, Vec<&str>>) -> Feature {
    let group_features = tree
        .remove(dependency)
        .unwrap_or_default()
        .into_iter()
        .map(|feature| construct_feature_diagram(feature, tree))
        .collect::<Vec<_>>();

    let groups = if group_features.is_empty() {
        vec![]
    } else {
        vec![Group::optional(group_features)]
    };

    Feature::new(dependency.to_owned(), groups, false)
}

fn feature_dependencies_implication(feature: &str, dependencies: &[&str]) -> CrossTreeConstraint {
    let mut dependency_constraint = CrossTreeConstraint::Feature(dependencies[0].to_owned());
    for &dependency in &dependencies[1..] {
        dependency_constraint = cross_tree_constraint::and(dependency_constraint, dependency);
    }
    cross_tree_constraint::implies(feature, dependency_constraint)
}