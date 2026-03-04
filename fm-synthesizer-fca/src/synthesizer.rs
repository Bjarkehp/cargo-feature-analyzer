use std::collections::{HashMap, HashSet};

use feature_model::{FeatureModel, cross_tree_constraint::CrossTreeConstraint, feature::Feature, group::Group};
use itertools::Itertools;
use petgraph::{Direction, graph::{DiGraph, EdgeIndex, NodeIndex}, visit::EdgeRef};

use crate::{concept::Concept, optimal_groups};

pub fn fm_from_ac_poset(ac_poset: &DiGraph<Concept, ()>, features: &[&str], tree_constraints: &HashSet<EdgeIndex>) -> FeatureModel {
    let maximal = ac_poset.externals(Direction::Outgoing)
        .next()
        .expect("An ac-poset should always have one maximal concept");

    let mut synthesizer = Synthesizer {
        ac_poset,
        tree_constraints,
        abstract_feature_index: 1,
    };
    
    let mut root_feature = synthesizer.construct_feature_diagram(maximal);
    
    let used_features = ac_poset.node_weights()
        .flat_map(|concept| concept.features.iter())
        .cloned()
        .collect::<HashSet<_>>();

    let unused_features = features.iter()
        .cloned()
        .filter(|f| !used_features.contains(f))
        .map(|name| Feature::new_leaf(name.to_owned(), false))
        .collect::<Vec<_>>();

    let cross_tree_constraints_from_edges = ac_poset.edge_indices()
        .filter(|e| !tree_constraints.contains(e))
        .map(|e| edge_to_concept_pair(ac_poset, e))
        .filter_map(|(a, b)| Some((*a.features.first()?, *b.features.first()?)))
        .map(|(l, r)| CrossTreeConstraint::Implies(l.to_owned(), r.to_owned()));

    let cross_tree_constraints_from_minimal_concepts = ac_poset.externals(Direction::Incoming)
        .cartesian_product(ac_poset.externals(Direction::Incoming).collect::<Vec<_>>())
        .map(|(i, j)| (&ac_poset[i], &ac_poset[j]))
        .filter(|(a, b)| (&a.inherited_configurations & &b.inherited_configurations).is_empty())
        .filter_map(|(a, b)| Some((*a.features.first()?, *b.features.first()?)))
        .filter(|(a, b)| a < b)
        .map(|(l, r)| CrossTreeConstraint::Exclusive(l.to_owned(), r.to_owned()));

    let cross_tree_constraints_unused_features = unused_features.iter()
        .map(|feature| CrossTreeConstraint::Not(feature.name.to_owned()));

    let cross_tree_constraints = cross_tree_constraints_from_edges
        .chain(cross_tree_constraints_from_minimal_concepts)
        .chain(cross_tree_constraints_unused_features)
        .collect::<Vec<_>>();

    if !unused_features.is_empty() {
        let unused_features_group = Group::optional(unused_features);
        let unused_features_abstract_feature = Feature::new(
            "unused_features".to_owned(), 
            vec![unused_features_group], 
            true,
        );
        let mandatory_group = Group::mandatory(vec![unused_features_abstract_feature]);
        root_feature.groups.push(mandatory_group);
    }
    
    FeatureModel { root_feature, cross_tree_constraints }
}

struct Synthesizer<'a> {
    ac_poset: &'a DiGraph<Concept<'a>, ()>,
    tree_constraints: &'a HashSet<EdgeIndex>,
    abstract_feature_index: usize,
}

impl<'a> Synthesizer<'a> {
    fn construct_feature_diagram(&mut self, node: NodeIndex) -> Feature {
        let tree_neighbors = self.ac_poset
            .edges_directed(node, Direction::Incoming)
            .filter(|&e| self.tree_constraints.contains(&e.id()))
            .map(|e| e.source())
            .collect::<Vec<_>>();

        let name = root_feature_name(self.ac_poset, node);

        let n = tree_neighbors.len();
        let groups = if n > 0 && n <= 12 {
            self.construct_optimal_groups(node, &tree_neighbors)
        } else {
            self.construct_simple_groups(node, &tree_neighbors)
        };

        Feature::new(name, groups, false)
    }

    fn construct_optimal_groups(&mut self, node: NodeIndex, tree_neighbors: &[NodeIndex]) -> Vec<Group> {
        let mut mandatory_features = mandatory_features(self.ac_poset, node);

        let features = tree_neighbors.iter()
            .map(|&neighbor| self.construct_feature_diagram(neighbor))
            .collect::<Vec<_>>();

        let assignments = construct_assignment_masks(self.ac_poset, node, tree_neighbors);
        let weight = |i: usize| features[i].config_count;
        let partition = optimal_groups::find(features.len(), &assignments, weight)
            .collect::<Vec<_>>();

        let mut groups_content = partition
            .iter()
            .map(|(_, min, max)| (vec![], *min, *max))
            .collect::<Vec<_>>();

        let feature_to_group_map = partition
            .iter()
            .enumerate()
            .flat_map(|(j, (indices, _, _))| indices.iter().map(move |&i| (i, j)))
            .collect::<HashMap<_, _>>();

        for (i, feature) in features.into_iter().enumerate() {
            groups_content[feature_to_group_map[&i]].0.push(feature);
        }

        let mut groups = groups_content.into_iter()
            .map(|(features, min, max)| Group::new(features, min, max))
            .collect::<Vec<_>>();

        if groups.len() == 1 && mandatory_features.is_empty() {
            groups
        } else if groups.len() == 1 && groups[0].is_optional() {
            let mandatory_group = Group::mandatory(mandatory_features);
            groups.push(mandatory_group);
            groups
        } else {
            for group in groups {
                let name = format!("abstract_{}", self.abstract_feature_index);
                let abstract_feature = Feature::new(name, vec![group], true);
                mandatory_features.push(abstract_feature);
                self.abstract_feature_index += 1;
            }
            
            if mandatory_features.is_empty() {
                vec![]
            } else {
                vec![Group::mandatory(mandatory_features)]
            }
        }
    }

    fn construct_simple_groups(&mut self, node: NodeIndex, tree_neighbors: &[NodeIndex]) -> Vec<Group> {
        let mandatory_features = mandatory_features(self.ac_poset, node);
        let optional_features = tree_neighbors.iter()
            .map(|&neighbor| self.construct_feature_diagram(neighbor))
            .collect::<Vec<_>>();

        let mandatory_group = Some(mandatory_features)
            .filter(|features| !features.is_empty())
            .map(Group::mandatory);
        let optional_group = Some(optional_features)
            .filter(|features| !features.is_empty())
            .map(Group::optional);
        [mandatory_group, optional_group].into_iter()
            .flatten()
            .collect::<Vec<_>>()
    }
}

fn construct_assignment_masks(
    ac_poset: &DiGraph<Concept, ()>,
    node: NodeIndex,
    tree_neighbors: &[NodeIndex],
) -> Vec<u32> {
    let mut assignments = tree_neighbors.iter()
        .enumerate()
        .flat_map(|(i, &node)| ac_poset[node].inherited_configurations.iter().map(move |c| (i, c)))
        .map(|(i, config)| (config, i))
        .into_grouping_map()
        .fold(0, |acc: u32, _, i| acc | (1 << i))
        .values()
        .cloned()
        .collect::<Vec<_>>();

    let has_cross_tree_neighbors = tree_neighbors.len() != ac_poset.edges_directed(node, Direction::Incoming).count();
    let empty_concept = ac_poset[node].configurations.is_empty();
    let empty_assignment = !empty_concept || has_cross_tree_neighbors;
    if empty_assignment {
        assignments.push(0);
    }

    assignments
}

fn root_feature_name(ac_poset: &DiGraph<Concept, ()>, node: NodeIndex) -> String {
    ac_poset[node].features.first()
        .cloned()
        .expect("A concept must have one feature")
        .to_owned()
}

fn mandatory_features(ac_poset: &DiGraph<Concept, ()>, node: NodeIndex) -> Vec<Feature> {
    ac_poset[node].features.iter()
        .skip(1) // skip root feature
        .cloned()
        .map(|name| Feature::new_leaf(name.to_owned(), false))
        .collect::<Vec<_>>()
}

fn edge_to_concept_pair<'a, 'b>(ac_poset: &'a DiGraph<Concept<'b>, ()>, edge: EdgeIndex) -> (&'a Concept<'b>, &'a Concept<'b>) {
    let (source, target) = ac_poset.edge_endpoints(edge)
        .expect("Edge index came from the graph, and the graph has not been mutated since");
    
    let left = &ac_poset[source];
    let right = &ac_poset[target];
    (left, right)
}