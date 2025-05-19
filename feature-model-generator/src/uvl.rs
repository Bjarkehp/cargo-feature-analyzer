use std::{collections::{BTreeMap, BTreeSet, HashMap, HashSet}, hash::Hash, io::Write, iter::from_fn};

use itertools::Itertools;
use petgraph::{graph::{DiGraph, NodeIndex}, visit::Dfs, Direction};

use crate::{concept::Concept, max_tree};
use configuration::{dependency::Dependency, directed_graph::DirectedGraph};

pub fn write_ac_poset<W: Write>(writer: &mut W, ac_poset: &DiGraph<Concept, ()>, root: &str) -> std::io::Result<()> {
    let mut visited = HashSet::new();
    let mut constraints = BTreeMap::new();

    writeln!(writer, "features")?;
    writeln!(writer, "\t\"{}\"", root)?;
    writeln!(writer, "\t\toptional")?;
    for node in ac_poset.externals(Direction::Outgoing) {
        visit_ac_poset_node(writer, ac_poset, node, &mut visited, &mut constraints, 1)?;
    }

    if constraints.is_empty() {
        return Ok(());
    }

    writeln!(writer, "constraints")?;
    for (antecedent, consequent) in constraints {
        let left = antecedent.iter().map(|s| format!("\"{s}\"")).join(" & ");
        let right = consequent.iter().map(|s| format!("\"{s}\"")).join(" & ");
        writeln!(writer, "\t{left} => {right}")?;
    }
    Ok(())
}

fn visit_ac_poset_node<'a, W: Write>(
    writer: &mut W, 
    ac_poset: &'a DiGraph<Concept, ()>, 
    node: NodeIndex, 
    visited: &mut HashSet<NodeIndex>,
    constraints: &mut BTreeMap<BTreeSet<&'a str>, BTreeSet<&'a str>>,
    depth: usize
) -> std::io::Result<()> {
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

    let (visited_children, not_visited_children) = ac_poset.neighbors_directed(node, Direction::Incoming)
        .partition::<Vec<_>, _>(|child| visited.contains(child));

    for child in visited_children {
        let child_concept = &ac_poset[child];
        let key = child_concept.features.iter().cloned().collect();
        constraints.entry(key)
            .and_modify(|set| set.extend(concept.features.iter()))
            .or_insert(concept.features.clone());
    }

    if not_visited_children.is_empty() {
        return Ok(());
    }

    let constraint = if concept.configurations.is_empty() && not_visited_children.len() > 1 {
        let histogram = config_histogram(&not_visited_children, ac_poset);
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
    for child in not_visited_children {
        visit_ac_poset_node(writer, ac_poset, child, visited, constraints, depth + 1)?;
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