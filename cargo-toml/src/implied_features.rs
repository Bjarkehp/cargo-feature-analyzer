use std::collections::BTreeSet;

use petgraph::Direction;

use crate::feature_dependencies;

pub fn from_dependency_graph<'a>(
    explicit_features: impl Iterator<Item = &'a str>,
    dependency_graph: &'a feature_dependencies::Graph
) -> BTreeSet<&'a str> {
    let mut stack = explicit_features.collect::<Vec<_>>();
    let mut visited_features = BTreeSet::new();
    while let Some(feature) = stack.pop() {
        if !visited_features.contains(feature) {
            visited_features.insert(feature);
            let new_features = dependency_graph.neighbors_directed(feature, Direction::Outgoing)
                .filter(|&f| dependency_graph.contains_node(f));
            stack.extend(new_features);
        }
    }

    visited_features
}
