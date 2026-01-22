use std::{collections::{BTreeSet, HashMap, HashSet}, io::Write};

use itertools::Itertools;
use petgraph::{Direction, graph::{DiGraph, EdgeIndex, NodeIndex}, visit::EdgeRef};

use crate::{concept::Concept};

/// Write an ac-poset into a UVL file.
/// 
/// The implementation first traverses the ac-poset from the maximal concept, writing features into the UVL file if not visited.
/// At each concept, mandatory constraints are written for each feature within the concept.
/// Afterwards, the group of inheriting concepts that haven't been visited are checked for whether they form an or-group.
/// Any concepts already visisted will instead be written into the constraints map.
/// Incompatible features are found by looking at the minimal concepts. 
/// If the union of the configurations of any pair of minimal concepts is empty, the features are incompatible.
/// Finally, the constraints are written into the UVL file.
pub fn write_ac_poset<W: Write>(writer: &mut W, ac_poset: &DiGraph<Concept, ()>, features: &[&str], tree_constraints: &HashSet<EdgeIndex>) -> std::io::Result<()> {
    if let Some(maximal) = ac_poset.externals(Direction::Outgoing).next() {
        let used_features = ac_poset.node_indices()
            .flat_map(|node| ac_poset[node].features.iter())
            .cloned()
            .collect::<BTreeSet<_>>();

        let unused_features = features.iter()
            .filter(|&&f| !used_features.contains(f))
            .cloned()
            .collect::<Vec<_>>();

        writeln!(writer, "features")?;
        write_tree_constraints(writer, ac_poset, maximal, tree_constraints, 0)?;
        write_unused_features(writer, &unused_features)?;

        if tree_constraints.len() < ac_poset.edge_count() || !unused_features.is_empty() {
            writeln!(writer, "constraints")?;
            write_cross_tree_constraints(writer, ac_poset, tree_constraints)?;
            for f in unused_features {
                writeln!(writer, "\t!\"{f}\"")?;
            }
        }
    }
    
    Ok(())
}

fn write_tree_constraints<W: Write>(
    writer: &mut W, 
    ac_poset: &DiGraph<Concept, ()>,
    node: NodeIndex,
    tree_constraints: &HashSet<EdgeIndex>, 
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

    let tree_neighbors = ac_poset.edges_directed(node, petgraph::Direction::Incoming)
        .filter(|&e| tree_constraints.contains(&e.id()))
        .map(|e| e.source())
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
    tree_constraints: &HashSet<EdgeIndex>,
) -> std::io::Result<()> {
    for e in ac_poset.edge_indices().filter(|e| !tree_constraints.contains(e)) {
        let (source, target) = ac_poset.edge_endpoints(e)
            .expect("Edge index came from the graph, and the graph has not mutated");

        let left_concept = &ac_poset[source];
        let right_concept = &ac_poset[target];
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

/// Return all pairs of incompatible features.
/// 
/// Incompatible features are found by looking at each pair of minimal concepts.
/// If the union of the configurations of any pair of minimal concepts is empty,
/// then the first features of the two concepts are incompatible.
fn incompatible_features<'a>(ac_poset: &'a DiGraph<Concept, ()>) -> impl Iterator<Item = (&'a str, &'a str)> {
    ac_poset.externals(Direction::Incoming)
        .cartesian_product(ac_poset.externals(Direction::Incoming).collect::<Vec<_>>())
        .map(|(i, j)| (&ac_poset[i], &ac_poset[j]))
        .filter(|(a, b)| (&a.inherited_configurations & &b.inherited_configurations).is_empty())
        .filter_map(|(a, b)| Some((a.features.first()?, b.features.first()?)))
        .map(|(a, b)| (*a, *b))
}

/// Creates a histogram, counting the amount of times a configuration appears in a list of nodes.
pub fn config_histogram<'a>(nodes: &[NodeIndex], ac_poset: &'a DiGraph<Concept, ()>) -> HashMap<&'a str, usize> {
    nodes.iter()
        .flat_map(|&child| ac_poset[child].inherited_configurations.iter())
        .map(|&x| (x, ()))
        .into_grouping_map()
        .aggregate(|acc, _key, _val| Some(acc.unwrap_or(0) + 1))
}

/// Find external concepts with no configurations and writes them directly as top-level features
fn write_unused_features<W: Write>(writer: &mut W, features: &[&str]) -> std::io::Result<()> {
    if features.is_empty() {
        return Ok(());
    }
    
    writeln!(writer, "\t\toptional // Unused features")?;
    for feature in features {
        writeln!(writer, "\t\t\t\"{}\"", feature)?;
    }

    Ok(())
}