use std::collections::{HashMap, HashSet};

use petgraph::{Direction, algo::toposort, graph::{DiGraph, EdgeIndex}, visit::EdgeRef};

use crate::concept::Concept;

pub fn find(ac_poset: &DiGraph<Concept, ()>) -> HashSet<EdgeIndex> {
    let toposort = toposort(&ac_poset, None)
        .expect("AC poset has no cycles");

    let mut edges = HashSet::new();
    let mut depths = HashMap::new();
    let maximal = *toposort.last()
        .expect("AC poset is not empty");
    depths.insert(maximal, 0_usize);

    for &node in toposort[..toposort.len() - 1].iter().rev() {
        let (edge, depth) = ac_poset.edges_directed(node, Direction::Outgoing)
            .map(|e| (e, depths[&e.target()] + 1))
            .max_by_key(|(_e, d)| *d)
            .map(|(e, d)| (e.id(), d))
            .expect("Every concept except the maximal has atleast one outgoing edge");

        edges.insert(edge);
        depths.insert(node, depth);
    }

    edges
}