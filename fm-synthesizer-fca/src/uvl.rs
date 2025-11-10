use std::{collections::{BTreeMap, BTreeSet, HashMap, HashSet}, fmt::Display, io::Write};

use itertools::Itertools;
use petgraph::{graph::{DiGraph, NodeIndex}, Direction};

use crate::concept::Concept;

/// Stores a feature and whether it is enabled or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ConstraintItem<'a> {
    enabled: bool,
    feature: &'a str,
}

impl<'a> ConstraintItem<'a> {
    fn new(enabled: bool, feature: &'a str) -> Self {
        Self { enabled, feature }
    }

    fn enabled(feature: &'a str) -> Self {
        Self::new(true, feature)
    }

    fn disabled(feature: &'a str) -> Self {
        Self::new(false, feature)
    }
}

impl Display for ConstraintItem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.enabled {
            write!(f, "\"{}\"", self.feature.replace('-', "_"))
        } else {
            write!(f, "!\"{}\"", self.feature.replace('-', "_"))
        }
    }
}

/// Write an ac-poset into a UVL file.
/// 
/// The implementation first traverses the ac-poset from the maximal concept, writing features into the UVL file if not visited.
/// At each concept, mandatory constraints are written for each feature within the concept.
/// Afterwards, the group of inheriting concepts that haven't been visited are checked for whether they form an or-group.
/// Any concepts already visisted will instead be written into the constraints map.
/// Incompatible features are found by looking at the minimal concepts. 
/// If the union of the configurations of any pair of minimal concepts is empty, the features are incompatible.
/// Finally, the constraints are written into the UVL file.
pub fn write_ac_poset<W: Write>(writer: &mut W, ac_poset: &DiGraph<Concept, ()>, features: &[&str]) -> std::io::Result<()> {
    let mut visited = HashSet::new();
    let mut constraints = BTreeMap::new();

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

        visit_ac_poset_node(writer, ac_poset, maximal, &mut visited, &mut constraints, 0)?;
        write_unused_features(writer, &unused_features)?;

        for (a, b) in incompatible_features(ac_poset) {
            constraints.entry(a)
                .or_default()
                .insert(ConstraintItem::disabled(b));
        }

        write_uvl_constraints(writer, &constraints)?;
    }

    Ok(())
}

/// Find external concepts with no configurations and writes them directly as top-level features
fn write_unused_features<W: Write>(writer: &mut W, features: &[&str]) -> std::io::Result<()> {
    if features.is_empty() {
        return Ok(());
    }
    
    writeln!(writer, "\t\toptional // Unused features")?;
    for feature in features {
        writeln!(writer, "\t\t\t\"{}\"", feature.replace('-', "_"))?;
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

/// Write the constraints into a UVL file
fn write_uvl_constraints<W: Write>(writer: &mut W, constraints: &BTreeMap<&str, BTreeSet<ConstraintItem>>) -> std::io::Result<()> {
    if constraints.is_empty() {
        return Ok(());
    }
    
    writeln!(writer, "constraints")?;
    for (antecedent, consequent) in constraints {
        let left = format!("\"{}\"", antecedent.replace('-', "_"));
        let right = consequent.iter()
            .map(|item| item.to_string())
            .join(" & ");
        writeln!(writer, "\t{left} => {right}")?;
    }

    Ok(())
}

/// Recursively traverse from a single node in the ac-poset, writing features into the UVL file if not visited.
/// Any visited concepts will instead be written into the constraints map.
fn visit_ac_poset_node<'a, W: Write>(
    writer: &mut W, 
    ac_poset: &'a DiGraph<Concept, ()>, 
    node: NodeIndex, 
    visited: &mut HashSet<NodeIndex>,
    constraints: &mut BTreeMap<&'a str, BTreeSet<ConstraintItem<'a>>>,
    depth: usize
) -> std::io::Result<()> {
    visited.insert(node);

    let concept = &ac_poset[node];
    let features = concept.features.iter()
        .cloned()
        .collect::<Vec<_>>();

    let parent_feature = features[0];
    writeln!(writer, "{}\"{}\"", "\t".repeat(2 * depth + 1), parent_feature.replace('-', "_"))?;
    if features.len() > 1 {
        writeln!(writer, "{}mandatory", "\t".repeat(2 * depth + 2))?;
        for &child_feature in &features[1..] {
            writeln!(writer, "{}\"{}\"", "\t".repeat(2 * depth + 3), child_feature.replace('-', "_"))?;
        }
    }

    let (visited_neighbors, not_visited_neighbors) = ac_poset.neighbors_directed(node, Direction::Incoming)
        .partition::<Vec<_>, _>(|child| visited.contains(child));

    for child in visited_neighbors {
        let child_concept = &ac_poset[child];
        let constraint_items = concept.features.iter()
            .cloned()
            .map(ConstraintItem::enabled)
            .collect::<BTreeSet<_>>();
        
        if let Some(key) = child_concept.features.first() {
            constraints.entry(key)
                .and_modify(|set| set.extend(constraint_items.iter()))
                .or_insert(constraint_items);
        }
    }

    if not_visited_neighbors.is_empty() {
        return Ok(());
    }

    let constraint = if concept.configurations.is_empty() && not_visited_neighbors.len() > 1 {
        let histogram = config_histogram(&not_visited_neighbors, ac_poset);
        let min = *histogram.values().min().unwrap();
        let max = *histogram.values().max().unwrap();
        
        match (min, max) {
            (1, 1) => "alternative".to_string(),
            (1, _) => "or".to_string(),
            (_, _) => format!("[{}..{}]", min, max),
        }
    } else {
        "optional".to_string()
    };
    
    writeln!(writer, "{}{}", "\t".repeat(2 * depth + 2), constraint)?;
    for child in not_visited_neighbors {
        visit_ac_poset_node(writer, ac_poset, child, visited, constraints, depth + 1)?;
    }

    Ok(())
}

/// Creates a histogram, counting the amount of times a configuration appears in a list of nodes.
fn config_histogram<'a>(nodes: &[NodeIndex], ac_poset: &'a DiGraph<Concept, ()>) -> HashMap<&'a str, usize> {
    nodes.iter()
        .flat_map(|&child| ac_poset[child].inherited_configurations.iter())
        .map(|&x| (x, ()))
        .into_grouping_map()
        .aggregate(|acc, _key, _val| Some(acc.unwrap_or(0) + 1))
}