use std::{collections::{BTreeSet, HashMap, HashSet}, hash::Hash, io::Write, iter::from_fn};

use itertools::Itertools;
use petgraph::{graph::{DiGraph, NodeIndex}, visit::Dfs, Direction};

use crate::{concept::Concept, dependency::Dependency, directed_graph::DirectedGraph, max_tree};

pub fn write_ac_poset<W: Write>(writer: &mut W, ac_poset: &DiGraph<Concept, ()>) -> std::io::Result<()> {
    let mut visited = HashSet::new();
    writeln!(writer, "features")?;
    for node in ac_poset.externals(Direction::Outgoing) {
        write_ac_poset_tree(writer, ac_poset, node, &mut visited, 0)?;
    }
    Ok(())
}

fn write_ac_poset_tree<W: Write>(writer: &mut W, ac_poset: &DiGraph<Concept, ()>, node: NodeIndex, visited: &mut HashSet<NodeIndex>, depth: usize) -> std::io::Result<()> {
    visited.insert(node);

    let concept = &ac_poset[node];
    let features = concept.features.iter()
        .cloned()
        .collect::<Vec<_>>();

    let parent_feature = features[0];
    writeln!(writer, "{}\"{parent_feature}\"", "\t".repeat(2 * depth + 1))?;
    if features.len() > 1 {
        writeln!(writer, "{}mandatory", "\t".repeat(2 * depth + 2))?;
        for &child_feature in &features[1..] {
            writeln!(writer, "{}\"{child_feature}\"", "\t".repeat(2 * depth + 3))?;
        }
    }

    let child_concepts = ac_poset.neighbors_directed(node, Direction::Incoming)
        .filter(|child| !visited.contains(child))
        .collect::<Vec<_>>();

    if child_concepts.is_empty() {
        return Ok(());
    }

    let constraint = if concept.configurations.is_empty() && child_concepts.len() > 1 {
        let histogram = config_histogram(&child_concepts, ac_poset);
        let min = *histogram.values().min().unwrap();
        let max = *histogram.values().max().unwrap();
        
        if min == 1 && max == 1 {
            "alternative".to_string()
        } else if min == 1 {
            "or".to_string()
        } else {
            format!("[{}..{}]", min, max)
        }
    } else {
        "optional".to_string()
    };

    writeln!(writer, "{}{}", "\t".repeat(2 * depth + 2), constraint)?;
    for child in child_concepts {
        write_ac_poset_tree(writer, ac_poset, child, visited, depth + 1)?;
    }

    Ok(())
}

fn config_histogram<'a>(nodes: &[NodeIndex], ac_poset: &'a DiGraph<Concept, ()>) -> HashMap<&'a str, usize> {
    nodes.iter()
        .flat_map(|&child| inheriting_concepts(child, ac_poset).into_iter().flat_map(|c| c.configurations.iter()))
        .map(|&x| (x, ()))
        .into_grouping_map()
        .aggregate(|acc, _key, _val| Some(acc.unwrap_or(0) + 1))
}

fn inheriting_concepts<'a>(node: NodeIndex, ac_poset: &'a DiGraph<Concept, ()>) -> BTreeSet<&'a Concept<'a>> {
    let mut concepts = BTreeSet::new();
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        if !concepts.contains(&ac_poset[node]) {
            concepts.insert(&ac_poset[node]);
            for neighbor in ac_poset.neighbors_directed(node, Direction::Incoming) {
                stack.push(neighbor);
            }
        }
    }
    concepts
}

pub fn write<W: Write>(writer: &mut W, graph: &DirectedGraph<Dependency>) -> std::io::Result<()> {
    let reversed = graph.reversed();
    let mut visited = HashSet::new();
    let mut visited_edges = HashSet::new();

    writeln!(writer, "features")?;
    while let Some((root, tree_edges)) = max_tree_first_root_candidate(&reversed, &mut visited) {
        let tree = tree_edges.iter().cloned().into_group_map();
        write_tree(writer, &tree, root, 1)?;

        for (from, to) in tree_edges {
            visited_edges.insert((to, from));
        }
    }

    writeln!(writer, "constraints")?;
    for (from, to) in graph.edges().filter(|(&from, &to)| !visited_edges.contains(&(from, to))) {
        writeln!(writer, "\t{} => {}", from.representation(), to.representation())?;
    }

    Ok(())
}

fn root_candidates<T: Eq + std::hash::Hash + Ord>(graph: &DirectedGraph<T>) -> BTreeSet<&T> {
    let mut nodes = graph.nodes().collect::<BTreeSet<_>>();

    for (_from, to) in graph.edges() {
        nodes.remove(to);
    }

    nodes
}

fn max_tree_first_root_candidate<'a>(graph: &'a DirectedGraph<Dependency>, visited: &mut HashSet<&'a Dependency<'a>>) -> Option<(Dependency<'a>, Vec<(Dependency<'a>, Dependency<'a>)>)> {
    let root = root_candidates(graph)
        .into_iter()
        .find(|root| !visited.contains(root))?;
    
    let tree = max_tree::find(graph, root, visited)
        .map(|(&from, &to)| (from, to))
        .collect::<Vec<_>>();

    Some((*root, tree))
}

fn write_tree<W: Write>(writer: &mut W, tree: &HashMap<Dependency, Vec<Dependency>>, node: Dependency, depth: usize) -> std::io::Result<()> {
    writeln!(writer, "{}{}", "\t".repeat(depth), node.representation())?;

    let empty = vec![];
    let children = tree.get(&node).unwrap_or(&empty);

    if children.is_empty() {
        return Ok(());
    }

    writeln!(writer, "{}optional", "\t".repeat(depth + 1))?;

    for child in children {
        write_tree(writer, tree, *child, depth + 2)?;
    }
    
    Ok(())
}