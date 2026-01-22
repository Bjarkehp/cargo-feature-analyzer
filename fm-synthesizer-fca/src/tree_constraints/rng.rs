use std::collections::HashSet;

use petgraph::{Direction, graph::{DiGraph, EdgeIndex, NodeIndex}, visit::EdgeRef};
use rand::{Rng, seq::IndexedRandom};

use crate::concept::Concept;

pub fn find<R: Rng>(ac_poset: &DiGraph<Concept, ()>, maximal: NodeIndex, rng: &mut R) -> HashSet<EdgeIndex> {
    let choose_random_edge = |n: NodeIndex| *ac_poset.edges_directed(n, Direction::Outgoing)
        .map(|e| e.id())
        .collect::<Vec<_>>()
        .choose(rng)
        .expect("Every concept except the maximal has atleast one outgoing edge");
    
    ac_poset.node_indices()
        .filter(|&n| n != maximal)
        .map(choose_random_edge)
        .collect::<HashSet<_>>()
}