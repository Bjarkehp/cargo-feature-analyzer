use std::{collections::{BTreeMap, BTreeSet}, fmt::Debug};

use derive_new::new;
use itertools::Itertools;
use petgraph::{graph::DiGraph, visit::{Dfs, EdgeRef, VisitMap}};

use configuration::Configuration;

/// A Concept consists of a set of configurations and features,
/// where the configurations share that same set of features.
/// implied_configurations is a set of all configurations with a set of features that are a superset of this concept's features.
/// configurations on the other hand, only contains a configuration if it isn't implied by the ordering of other concepts.
#[derive(PartialEq, Eq, PartialOrd, Ord, new, Default)]
pub struct Concept<'a> {
    pub features: BTreeSet<&'a str>,
    pub configurations: BTreeSet<&'a str>,
    pub inherited_configurations: BTreeSet<&'a str>,
}

impl Debug for Concept<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Concept")
            .field("features", &self.features)
            .field("configurations", &self.configurations)
            .finish()
    }
}

/// Create an Attribute-Concept partially ordered set from a set of configurations.
/// 
/// The function can be split into 5 steps:
/// 1. Extract all concepts from the configurations by grouping together features with the same set of configurations.
/// 2. Find all pairs of concepts where the first's configurations are a subset of the second's.
/// 3. Remove duplicate configurations from the concepts that are already inherited by parent concepts.
/// 4. Create a graph where the nodes are concepts and the edges represent the partial order of concepts.
/// 5. Remove all redundant edges that don't effect the partial order.
pub fn ac_poset<'a>(configurations: &'a [Configuration], features: &'a [&str]) -> DiGraph<Concept<'a>, ()> {
    let mut concepts = extract_concepts(configurations, features);
    let edges = subset_edges(&concepts);
    remove_duplicate_configurations(&mut concepts, &edges);
    let mut graph = create_graph(concepts, &edges);
    transitive_reduction(&mut graph);
    graph
}

/// Extract all concepts from the configurations by grouping together features with the same set of set of configurations.
fn extract_concepts<'a>(configurations: &'a [Configuration], features: &'a [&str], ) -> Vec<Concept<'a>> {
    let configurations_with_feature = |feature: &str| {
        configurations.iter()
            .filter(|config| config.features().contains(&feature))
            .map(|config| config.name())
            .collect::<BTreeSet<_>>()
    };

    features.iter()
        .map(|feature| (configurations_with_feature(feature), feature))
        .into_grouping_map()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|(configurations, features)| Concept::new(features, configurations.clone(), configurations))
        .filter(|c| !c.configurations.is_empty())
        .sorted_by(|a, b| a.configurations.cmp(&b.configurations))
        .collect()
}

/// Find all pairs of concepts where one's configurations are a subset of the other's.
/// 
/// The result is a Vec of indices of the concepts.
fn subset_edges(concepts: &[Concept]) -> Vec<(u32, u32)> {
    concepts.iter().enumerate()
        .cartesian_product(concepts.iter().enumerate())
        .filter(|((_, a), (_, b))| a != b && a.configurations.is_subset(&b.configurations))
        .map(|((i, _), (j, _))| (i as u32, j as u32))
        .collect()
}

/// Remove duplicate configurations from the concepts that are already inherited by parent concepts.
fn remove_duplicate_configurations(concepts: &mut [Concept], edges: &[(u32, u32)]) {
    let mut differences: BTreeMap<u32, BTreeSet<&str>> = BTreeMap::new();
    for &(i, j) in edges {
        let a = &concepts[i as usize];
        let b = &concepts[j as usize];

        let config_diff = differences.entry(j)
            .or_insert(b.configurations.clone())
            .difference(&a.configurations)
            .cloned()
            .collect::<BTreeSet<_>>();
        differences.entry(j)
            .and_modify(|set| set.retain(|c| config_diff.contains(c)))
            .or_insert(config_diff);
    }

    for (i, set) in differences {
        concepts[i as usize].configurations = set;
    }
}

/// Remove all redundant edges that are implied by the partial order.
fn transitive_reduction(graph: &mut DiGraph<Concept, ()>) {
    let edges = graph.edge_references()
        .map(|e| (e.source(), e.target()))
        .collect::<Vec<_>>();

    for (u, v) in edges {
        let edge_index = graph.find_edge(u, v).unwrap();
        graph.remove_edge(edge_index).unwrap();
        let mut dfs = Dfs::new(&*graph, u);
        while dfs.next(&*graph).is_some() {}
        if !dfs.discovered.is_visited(&v) {
            graph.add_edge(u, v, ());
        }
    }
}

/// Create a graph from a vector of nodes (concepts) and a slice of edges.
fn create_graph<'a>(concepts: Vec<Concept<'a>>, edges: &[(u32, u32)]) -> DiGraph<Concept<'a>, ()> {
    let mut graph = DiGraph::<Concept, ()>::default();
    for concept in concepts.into_iter() {
        graph.add_node(concept);
    }
    for &(i, j) in edges {
        graph.add_edge(i.into(), j.into(), ());
    }
    graph
}