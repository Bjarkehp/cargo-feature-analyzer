use std::{collections::{BTreeSet, HashSet}, io::Write};

use petgraph::{Direction, graph::{DiGraph, NodeIndex}, visit::EdgeRef};
use rand::{Rng, seq::IndexedRandom};

use crate::{concept::Concept, uvl::{config_histogram, incompatible_features, write_unused_features}};

pub fn write_ac_poset<W: Write, R: Rng>(writer: &mut W, ac_poset: &DiGraph<Concept, ()>, features: &[&str], rng: &mut R) -> std::io::Result<()> {
    if let Some(maximal) = ac_poset.externals(Direction::Outgoing).next() {
        let used_features = ac_poset.node_indices()
            .flat_map(|node| ac_poset[node].features.iter())
            .cloned()
            .collect::<BTreeSet<_>>();

        let unused_features = features.iter()
            .filter(|&&f| !used_features.contains(f))
            .cloned()
            .collect::<Vec<_>>();

        let mut tree_constraints = HashSet::new();

        for node in ac_poset.node_indices().filter(|&n| n != maximal) {
            let neighbors = ac_poset.neighbors_directed(node, Direction::Outgoing)
                .collect::<Vec<_>>();

            let tree_constraint_neighbor = *neighbors.choose(rng)
                .expect("Every concept except the maximal has atleast one outgoing edge");

            tree_constraints.insert((node, tree_constraint_neighbor));
        }

        writeln!(writer, "features")?;
        write_tree_constraints(writer, ac_poset, maximal, &tree_constraints, 0)?;
        write_unused_features(writer, &unused_features)?;

        if tree_constraints.len() < ac_poset.edge_count() {
            writeln!(writer, "constraints")?;
            write_cross_tree_constraints(writer, ac_poset, &tree_constraints)?;
        }
    }
    
    Ok(())
}

fn write_tree_constraints<W: Write>(
    writer: &mut W, 
    ac_poset: &DiGraph<Concept, ()>,
    node: NodeIndex,
    tree_constraints: &HashSet<(NodeIndex, NodeIndex)>, 
    depth: usize,
) -> std::io::Result<()> {
    let tab1 = "\t".repeat(2 * depth + 1);
    let tab2 = "\t".repeat(2 * depth + 2);

    let concept = &ac_poset[node];
    let features = concept.features.iter()
        .cloned()
        .collect::<Vec<_>>();

    let parent_feature = features[0];
    writeln!(writer, "{tab1}\"{parent_feature}\"")?;
    if features.len() > 1 {
        let tab3 = "\t".repeat(2 * depth + 3);
        writeln!(writer, "{tab2}mandatory")?;
        for &child_feature in &features[1..] {
            writeln!(writer, "{tab3}\"{child_feature}\"")?;
        }
    }

    let tree_neighbors = ac_poset.neighbors_directed(node, petgraph::Direction::Incoming)
        .filter(|&n| tree_constraints.contains(&(n, node)))
        .collect::<Vec<_>>();

    if tree_neighbors.is_empty() {
        return Ok(());
    }

    if concept.configurations.is_empty() && tree_neighbors.len() > 1 {
        let histogram = config_histogram(&tree_neighbors, ac_poset);
        let min = *histogram.values().min().unwrap();
        let max = *histogram.values().max().unwrap();
        
        match (min, max) {
            (1, 1) => writeln!(writer, "{tab2}alternative")?,
            (1, _) => writeln!(writer, "{tab2}or")?,
            (_, _) => writeln!(writer, "{tab2}[{min}..{max}]")?,
        }
    } else {
        writeln!(writer, "{tab2}optional")?;
    };

    for neighbor in tree_neighbors {
        write_tree_constraints(writer, ac_poset, neighbor, tree_constraints, depth + 1)?;
    }

    Ok(())
}

fn write_cross_tree_constraints<W: Write>(
    writer: &mut W,
    ac_poset: &DiGraph<Concept, ()>,
    tree_constraints: &HashSet<(NodeIndex, NodeIndex)>,
) -> std::io::Result<()> {
    for e in ac_poset.edge_references() {
        if tree_constraints.contains(&(e.source(), e.target())) {
            continue;
        }

        let left_concept = &ac_poset[e.source()];
        let right_concept = &ac_poset[e.target()];
        let a = left_concept.features.iter().next()
            .expect("Concepts have atleast one feature");
        let b = right_concept.features.iter().next()
            .expect("Concepts have atleast one feature");
        writeln!(writer, "\t\"{a}\" => \"{b}\"")?;
    }

    for (a, b) in incompatible_features(ac_poset) {
        writeln!(writer, "\t\"{a}\" => !\"{b}\"")?;
    }

    Ok(())
}