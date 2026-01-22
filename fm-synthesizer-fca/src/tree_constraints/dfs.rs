use std::collections::HashSet;

use petgraph::{Direction, graph::{DiGraph, EdgeIndex, NodeIndex}, visit::EdgeRef};

use crate::concept::Concept;

pub fn find(ac_poset: &DiGraph<Concept, ()>, maximal: NodeIndex) -> HashSet<EdgeIndex> {
    let mut edges = HashSet::new();
    let mut visisted = HashSet::new();
    let mut stack = vec![maximal];

    while let Some(node) = stack.pop() {
        if visisted.contains(&node) {
            continue;
        }

        visisted.insert(node);

        for edge in ac_poset.edges_directed(node, Direction::Incoming) {
            stack.push(edge.source());
            edges.insert(edge.id());
        }
    }
    
    edges
}