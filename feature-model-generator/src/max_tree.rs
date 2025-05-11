use std::collections::{HashMap, HashSet};

use configuration::directed_graph::DirectedGraph;

pub fn find<'a, T: Eq + std::hash::Hash>(graph: &'a DirectedGraph<T>, root: &'a T, visited: &mut HashSet<&'a T>) -> impl Iterator<Item = (&'a T, &'a T)> {
    let mut parent: HashMap<&T, &T> = HashMap::new();
    let mut tree_edges = Vec::new();

    visited.insert(root);

    for node in graph.nodes() {
        if node == root {
            continue;
        }
        
        let incoming_edge = graph.edges().find(|&(_, to)| to == node);
        if let Some((from, _)) = incoming_edge {
            parent.insert(node, from);
            tree_edges.push((from, node));
        }
    }

    let mut cycles = Vec::new();
    for node in graph.nodes() {
        if node == root || visited.contains(&node) {
            continue;
        }
        
        let mut cycle = Vec::new();
        let mut current = node;
        while let Some(&p) = parent.get(&current) {
            if visited.contains(&current) {
                break;
            }
            cycle.push(current);
            visited.insert(current);
            current = p;
            
            if current == node {
                cycles.push(cycle.clone());
                break;
            }
        }
    }
    
    for cycle in cycles {
        if let Some(node) = cycle.first() {
            parent.remove(node);
            tree_edges.retain(|&(_, to)| to != *node);
        }
    }

    tree_edges.into_iter()
}