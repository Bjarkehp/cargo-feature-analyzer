use std::collections::{BTreeSet, HashMap};

use derive_new::new;
use itertools::Itertools;
use petgraph::{graph::DiGraph, visit::{Dfs, EdgeRef, VisitMap}};

use crate::{configuration::Configuration, dependency::Dependency};

/// A Concept consists of a set of configurations and features,
/// where the configurations share that same set of features.
#[derive(PartialEq, Eq, new, Default, Debug)]
pub struct Concept<'a> {
    pub configurations: BTreeSet<&'a str>,
    pub features: BTreeSet<&'a str>,
}

/// Create an Attribute-Concept partially ordered set from a set of configurations.
/// 
/// The function can be split into 5 steps:
/// 1. Extract all concepts from the configurations by grouping together concepts with the same set of features.
/// 2. Find all pairs of concepts where one's features are a subset of the other's.
/// 3. Remove duplicate features from the concepts that are already inherited by parent concepts.
/// 4. Create a graph where the nodes are concepts and the edges represent the partial order of concepts.
/// 5. Remove all redundant edges that don't effect the partial order.
pub fn ac_poset<'a>(configurations: &'a [Configuration<'a>]) -> DiGraph<Concept<'a>, ()> {
    let mut concepts = extract_concepts(configurations);
    let edges = subset_edges(&concepts);
    remove_duplicate_features(&mut concepts, &edges);
    let mut graph = create_graph(concepts, &edges);
    transitive_reduction(&mut graph);
    graph
}

/// Extract all concepts from the configurations by grouping together concepts with the same set of features.
fn extract_concepts<'a>(configurations: &'a [Configuration<'a>]) -> Vec<Concept<'a>> {
    configurations.iter()
        .map(|config| (config.features().iter().cloned().map(Dependency::name).collect::<BTreeSet<_>>(), config.name()))
        .into_grouping_map().collect::<BTreeSet<&str>>()
        .into_iter()
        .map(|(features, configurations)| Concept::new(configurations, features))
        .collect()
}

/// Find all pairs of concepts where one's features are a subset of the other's.
/// 
/// The result is a Vec of indices of the concepts.
fn subset_edges(concepts: &[Concept]) -> Vec<(u32, u32)> {
    concepts.iter().enumerate()
        .cartesian_product(concepts.iter().enumerate())
        .filter(|((_, a), (_, b))| a != b && a.features.is_subset(&b.features))
        .map(|((i, _), (j, _))| (i as u32, j as u32))
        .collect()
}

/// Remove all redundant edges that don't effect the partial order.
fn remove_duplicate_features(concepts: &mut [Concept], edges: &[(u32, u32)]) {
    let mut differences: HashMap<u32, BTreeSet<&str>> = HashMap::new();
    for &(i, j) in edges {
        let a = &concepts[i as usize];
        let b = &concepts[j as usize];
        let diff = differences.entry(j)
            .or_insert(b.features.clone())
            .difference(&a.features)
            .cloned()
            .collect::<BTreeSet<_>>();
        differences.entry(j)
            .and_modify(|set| *set = set.intersection(&diff).cloned().collect::<BTreeSet<_>>())
            .or_insert(diff);
    }

    for (i, set) in differences {
        concepts[i as usize].features = set;
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