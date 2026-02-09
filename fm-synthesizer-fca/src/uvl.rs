use std::{collections::{BTreeSet, HashMap, HashSet}, io::Write};

use cargo_toml::crate_id::CrateId;
use itertools::Itertools;
use petgraph::{Direction, graph::{DiGraph, EdgeIndex, NodeIndex}, visit::EdgeRef};

use crate::{concept::Concept, indent::tab, optimal_groups};

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

        let mut abstract_feature_index = 1;

        writeln!(writer, "features")?;
        write_tree_constraints(writer, ac_poset, maximal, tree_constraints, &mut abstract_feature_index, 0)?;
        write_unused_features(writer, &unused_features)?;

        if tree_constraints.len() < ac_poset.edge_count() {
            writeln!(writer, "constraints")?;
            write_cross_tree_constraints(writer, ac_poset, tree_constraints)?;
        }
    }
    
    Ok(())
}

/// Writes the tree constraints of the UVL model.
/// 
/// The function recursively travels through the ac-poset only using edges in tree_constraints.
/// Depth is used to indent the feature and group cardinality correctly.
/// [optimal_groups::find] is used for parents with a reasonable low number of children (15),
/// to partition the children into groups that locally minimize the number of configurations for the feature model.
fn write_tree_constraints<W: Write>(
    writer: &mut W, 
    ac_poset: &DiGraph<Concept, ()>,
    node: NodeIndex,
    tree_constraints: &HashSet<EdgeIndex>, 
    abstract_feature_index: &mut usize,
    depth: usize,
) -> std::io::Result<()> {
    let mandatory_features_count = write_mandatory_features(writer, ac_poset, node, depth)?;

    let tree_neighbors = ac_poset.edges_directed(node, Direction::Incoming)
        .filter(|&e| tree_constraints.contains(&e.id()))
        .map(|e| e.source())
        .collect::<Vec<_>>();

    let n = tree_neighbors.len();

    if n == 0 {
        Ok(())
    } else if n < 15 {
        write_optimal_groups(writer, ac_poset, node, &tree_neighbors, tree_constraints, mandatory_features_count, abstract_feature_index, depth)
    } else {
        write_group(writer, ac_poset, &tree_neighbors, tree_constraints, abstract_feature_index, 0, n, depth)
    }    
}

/// Writes mandatory features of a concept into the feature model, if any exist.
fn write_mandatory_features<W: Write>(
    writer: &mut W,
    ac_poset: &DiGraph<Concept, ()>,
    node: NodeIndex,
    depth: usize
) -> std::io::Result<usize> {
    let concept = &ac_poset[node];
    let features = concept.features.iter()
        .cloned()
        .collect::<Vec<_>>();
    let parent_feature = features[0];

    tab(writer, 2 * depth + 1)?;
    writeln!(writer, "\"{parent_feature}\"")?;

    if features.len() > 1 {
        tab(writer, 2 * depth + 2)?;
        writeln!(writer, "mandatory")?;
        for &child_feature in &features[1..] {
            tab(writer, 2 * depth + 3)?;
            writeln!(writer, "\"{child_feature}\"")?;
        }
    }

    let mandatory_features_count = features.len() - 1;
    Ok(mandatory_features_count)
}

/// Calculates and writes optimal groups of some concept into the feature model.
/// 
/// See [optimal_groups::find] for more info on optimal groups.
#[allow(clippy::too_many_arguments)]
fn write_optimal_groups<W: Write>(
    writer: &mut W,
    ac_poset: &DiGraph<Concept, ()>,
    node: NodeIndex,
    tree_neighbors: &[NodeIndex],
    tree_constraints: &HashSet<EdgeIndex>,
    mandatory_features_count: usize,
    abstract_feature_index: &mut usize,
    depth: usize,
) -> std::io::Result<()> {
    let optimal_groups = optimal_groups::find(ac_poset, node, tree_neighbors)
        .collect::<Vec<_>>();
    
    let (mut non_abstract_groups, mut abstract_groups) = optimal_groups.iter()
        .partition::<Vec<_>, _>(|(nodes, min, max)| *min == 0 && *max == nodes.len());

    if abstract_groups.len() == 1 {
        let group = abstract_groups.pop()
            .expect("Length of abstract groups is checked before hand");
        non_abstract_groups.push(group);
    }

    // No need to create another mandatory group, if one already exists,
    // or if there are no abstract groups.
    if mandatory_features_count == 0 && !abstract_groups.is_empty() {
        tab(writer, 2 * depth + 2)?;
        writeln!(writer, "mandatory")?;
    }

    for (group_nodes, min, max) in abstract_groups {
        write_abstract_group(writer, ac_poset, group_nodes, tree_constraints, abstract_feature_index, *min, *max, depth)?;
    }

    for (group_nodes, min, max) in non_abstract_groups {   
        write_group(writer, ac_poset, group_nodes, tree_constraints, abstract_feature_index, *min, *max, depth)?;
    }

    Ok(())
}

/// Writes a group of concepts into the feature model. 
#[allow(clippy::too_many_arguments)]
fn write_group<W: Write>(
    writer: &mut W,
    ac_poset: &DiGraph<Concept, ()>,
    group_nodes: &[NodeIndex],
    tree_constraints: &HashSet<EdgeIndex>,
    abstract_feature_index: &mut usize,
    min: usize,
    max: usize,
    depth: usize,
) -> std::io::Result<()> {
    tab(writer, 2 * depth + 2)?;
    match (min, max) {
        (0, n) if n == group_nodes.len() => writeln!(writer, "optional")?,
        (1, n) if n == group_nodes.len() => writeln!(writer, "or")?,
        (1, 1) => writeln!(writer, "alternative")?,
        (m, n) => writeln!(writer, "[{m}..{n}]")?,
    }

    for &node in group_nodes {
        write_tree_constraints(writer, ac_poset, node, tree_constraints, abstract_feature_index, depth + 1)?;
    }

    Ok(())
}

/// Writes an abstract group (a group with an abstract parent).
/// 
/// This is used to make multiple groups in a feature model distinct in a visualization.
#[allow(clippy::too_many_arguments)]
fn write_abstract_group<W: Write>(
    writer: &mut W,
    ac_poset: &DiGraph<Concept, ()>,
    group_nodes: &[NodeIndex],
    tree_constraints: &HashSet<EdgeIndex>,
    abstract_feature_index: &mut usize,
    min: usize,
    max: usize,
    depth: usize,
) -> std::io::Result<()> {
    tab(writer, 2 * depth + 3)?;
    writeln!(writer, "abstract_{} {{abstract}}", *abstract_feature_index)?;
    *abstract_feature_index += 1;
    write_group(writer, ac_poset, group_nodes, tree_constraints, abstract_feature_index, min, max, depth + 1)?;

    Ok(())
}

/// Writes the cross tree constraints found in the ac-poset into the model.
/// 
/// The function goes through every edge in the ac-poset that is not a tree constraint,
/// and writes it into the model as an implication.
/// Incompatible features, i.e. features where no configuration has both enabled,
/// are also written using an implication statement.
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

/// Returns pairs of incompatible features.
/// 
/// Incompatible features are found by looking at each pair of minimal concepts.
/// If the union of the configurations of any pair of minimal concepts is empty,
/// then the first features of the two concepts are incompatible.
/// 
/// This implies that some redundant pairs aren't returned. If a => !b and b => c,
/// then the pair (a, b) is returned, but not (a, c) because it is implied by the previous pair.
fn incompatible_features<'a>(ac_poset: &'a DiGraph<Concept, ()>) -> impl Iterator<Item = (&'a str, &'a str)> {
    ac_poset.externals(Direction::Incoming)
        .cartesian_product(ac_poset.externals(Direction::Incoming).collect::<Vec<_>>())
        .map(|(i, j)| (&ac_poset[i], &ac_poset[j]))
        .filter(|(a, b)| (&a.inherited_configurations & &b.inherited_configurations).is_empty())
        .filter_map(|(a, b)| Some((a.features.first()?, b.features.first()?)))
        .map(|(a, b)| (*a, *b))
        .filter(|(a, b)| a < b) // Removes symmetric pairs
}

/// Creates a histogram, counting the amount of times a configuration appears in a list of nodes.
pub fn config_histogram<'a>(nodes: &[NodeIndex], ac_poset: &'a DiGraph<Concept, ()>) -> HashMap<&'a CrateId, usize> {
    nodes.iter()
        .flat_map(|&child| ac_poset[child].inherited_configurations.iter())
        .map(|x| (x, ()))
        .into_grouping_map()
        .aggregate(|acc, _key, _val| Some(acc.unwrap_or(0) + 1))
}

/// Find external concepts with no configurations and writes them directly as top-level features
fn write_unused_features<W: Write>(writer: &mut W, features: &[&str]) -> std::io::Result<()> {
    if features.is_empty() {
        return Ok(());
    }
    
    writeln!(writer, "\t\tmandatory")?;
    writeln!(writer, "\t\t\tunused_features {{abstract}}")?;
    writeln!(writer, "\t\t\t\t[0..0]")?;
    for feature in features {
        writeln!(writer, "\t\t\t\t\t\"{}\"", feature)?;
    }

    // Flamapy, as of version 2.1.0.dev1, has a bug with feature models, 
    // where a tree constraint of [0..0] only has one feature inside.
    // To mitigate this, a dummy unused feature is added in those cases.
    if features.len() == 1 {
        writeln!(writer, "\t\t\t\t\tabstract_unused_feature {{abstract}}")?;
    }

    Ok(())
}