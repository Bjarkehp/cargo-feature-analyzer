use std::{collections::{BTreeMap, BTreeSet, HashMap, HashSet}, io::Write};

use itertools::Itertools;
use petgraph::{graph::{DiGraph, NodeIndex}, Direction};

use crate::concept::Concept;

pub fn write_ac_poset<W: Write>(writer: &mut W, ac_poset: &DiGraph<Concept, ()>, root: &str) -> std::io::Result<()> {
    let mut visited = HashSet::new();
    let mut constraints = BTreeMap::new();

    writeln!(writer, "features")?;
    writeln!(writer, "\t\"{}\"", root)?;
    writeln!(writer, "\t\toptional")?;

    write_unused_features(writer, ac_poset, &mut visited)?;
    write_uvl_tree(writer, ac_poset, &mut visited, &mut constraints)?;

    if !constraints.is_empty() {
        write_uvl_constraints(writer, &constraints)?;
    }

    Ok(())
}

fn write_unused_features<W: Write>(writer: &mut W, ac_poset: &DiGraph<Concept, ()>, visited: &mut HashSet<NodeIndex>) -> std::io::Result<()> {
    for node in ac_poset.externals(Direction::Incoming) {
        let concept = &ac_poset[node];
        if concept.configurations.is_empty() {
            visited.insert(node);
            for feature in concept.features.iter() {
                writeln!(writer, "\t\t\t\"{feature}\"")?;
            }
        }
    }

    Ok(())
}

fn write_uvl_tree<'a, W: Write>(writer: &mut W, ac_poset: &'a DiGraph<Concept, ()>, visited: &mut HashSet<NodeIndex>, constraints: &mut BTreeMap<BTreeSet<&'a str>, BTreeSet<&'a str>>) -> std::io::Result<()>{
    for node in ac_poset.externals(Direction::Outgoing) {
        visit_ac_poset_node(writer, ac_poset, node, visited, constraints, 1)?;
    }

    Ok(())
}

fn write_uvl_constraints<W: Write>(writer: &mut W, constraints: &BTreeMap<BTreeSet<&str>, BTreeSet<&str>>) -> std::io::Result<()> {
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
        if config_histogram(&[child], ac_poset).len() > 1 {
            let child_concept = &ac_poset[child];
            let key = child_concept.features.iter().cloned().collect();
            constraints.entry(key)
                .and_modify(|set| set.extend(concept.features.iter()))
                .or_insert(concept.features.clone());
        }
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