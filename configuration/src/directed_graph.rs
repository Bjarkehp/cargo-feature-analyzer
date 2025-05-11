use std::collections::{HashMap, HashSet};

use itertools::Itertools;

#[derive(Debug)]
pub struct DirectedGraph<T: Eq + std::hash::Hash> {
    map: HashMap<T, HashSet<T>>
}

impl<T: Eq + std::hash::Hash> DirectedGraph<T> {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    pub fn nodes(&self) -> impl Iterator<Item = &T> {
        self.map.keys()
    }

    pub fn edges(&self) -> impl Iterator<Item = (&T, &T)> {
        self.map.iter()
            .flat_map(|(from, set)| set.iter().map(move |to| (from, to)))
    }

    pub fn insert(&mut self, from: T, to: T) {
        self.map.entry(from).or_default().insert(to);
    }

    pub fn extend(&mut self, from: T, to: impl Iterator<Item = T>) {
        self.map.entry(from).or_default().extend(to);
    }

    pub fn reversed(&self) -> Self where T: Clone {
        let mut reversed_graph: HashMap<T, HashSet<T>> = HashMap::with_capacity(self.map.len());

        for (node, neighbors) in self.map.iter() {
            for neighbor in neighbors {
                reversed_graph.entry(neighbor.clone()).or_default().insert(node.clone());
            }
            reversed_graph.entry(node.clone()).or_default();
        }

        DirectedGraph { map: reversed_graph }
    }

    pub fn depth_first_search(&self, root: T, visited: &mut HashSet<T>) where T: Clone {
        visited.insert(root.clone());

        let set = self.map.get(&root);

        if let Some(set) = set {
            set.iter()
                .filter(|node| !visited.contains(node))
                .collect_vec().into_iter()
                .for_each(|node| self.depth_first_search(node.clone(), visited));
        }
    }
}