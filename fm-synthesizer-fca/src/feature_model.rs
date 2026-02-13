use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use petgraph::{Direction, graph::{DiGraph, EdgeIndex, NodeIndex}, visit::EdgeRef};

use crate::{binomial::n_choose_k_usize, concept::Concept, optimal_groups};

#[derive(derive_new::new)]
pub struct Feature {
    pub name: String,
    pub groups: Vec<Group>,
    pub estimated_number_of_configurations: f64,
    pub is_abstract: bool,
}

impl Feature {
    pub fn new_leaf(name: String, is_abstract: bool) -> Feature {
        Feature::new(name, vec![], 1.0, is_abstract)
    }
}

pub struct Group {
    pub features: Vec<Feature>,
    pub min: usize,
    pub max: usize,
    pub estimated_number_of_configurations: f64,
}

impl Group {
    pub fn new(features: Vec<Feature>, min: usize, max: usize) -> Group {
        let n = features.len();
        let mut dp = vec![0.0; n + 1];
        dp[0] = 1.0;

        for feature in features.iter() {
            for k in (1..=n).rev() {
                dp[k] += dp[k - 1] * feature.estimated_number_of_configurations;
            }
        }

        let estimated_number_of_configurations = (min..=max)
            .map(|k| dp[k])
            .sum();

        Group { features, min, max, estimated_number_of_configurations }
    }

    pub fn mandatory(features: Vec<Feature>) -> Group {
        let n = features.len();
        Group::new(features, n, n)
    }

    pub fn optional(features: Vec<Feature>) -> Group {
        let n = features.len();
        Group::new(features, 0, n)
    }

    pub fn is_mandatory(&self) -> bool {
        let n = self.features.len();
        self.min == n && self.max == n
    }

    pub fn is_optional(&self) -> bool {
        self.min == 0 && self.max == self.features.len()
    }
}

pub enum CrossTreeConstraint {
    Implies(String, String),
    Exclusive(String, String),
}

pub struct FeatureModel {
    pub root_feature: Feature,
    pub cross_tree_constraints: Vec<CrossTreeConstraint>,
}

pub fn from_ac_poset(ac_poset: &DiGraph<Concept, ()>, features: &[&str], tree_constraints: &HashSet<EdgeIndex>) -> FeatureModel {
    let maximal = ac_poset.externals(Direction::Outgoing)
        .next()
        .expect("An ac-poset should always have one maximal concept");

    let mut abstract_feature_index = 1;

    let mut root_feature = construct_feature_tree(ac_poset, maximal, tree_constraints, &mut abstract_feature_index);
    
    let used_features = ac_poset.node_weights()
        .flat_map(|concept| concept.features.iter())
        .cloned()
        .collect::<HashSet<_>>();

    let mut unused_features = features.iter()
        .cloned()
        .filter(|f| !used_features.contains(f))
        .map(|name| Feature::new_leaf(name.to_owned(), false))
        .collect::<Vec<_>>();

    // Flamapy, as of version 2.1.0.dev1, has a bug with feature models, 
    // where a tree constraint of [0..0] only has one feature inside.
    // To mitigate this, a dummy unused feature is added in those cases.
    if unused_features.len() == 1 {
        unused_features.push(Feature::new_leaf("abstract_unused_feature".to_owned(), true));
    }

    if !unused_features.is_empty() {
        let unused_features_group = Group::new(unused_features, 0, 0);
        let unused_features_abstract_feature = Feature::new(
            "unused_features".to_owned(), 
            vec![unused_features_group], 
            1.0, 
            true,
        );
        let mandatory_group = Group::mandatory(vec![unused_features_abstract_feature]);
        root_feature.groups.push(mandatory_group);
    }

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

    let cross_tree_constraints = cross_tree_constraints_from_edges
        .chain(cross_tree_constraints_from_minimal_concepts)
        .collect::<Vec<_>>();
    
    FeatureModel { root_feature, cross_tree_constraints }
}

fn construct_feature_tree(
    ac_poset: &DiGraph<Concept, ()>, 
    node: NodeIndex,
    tree_constraints: &HashSet<EdgeIndex>,
    abstract_feature_index: &mut usize,
) -> Feature {    
    let tree_neighbors = ac_poset.edges_directed(node, Direction::Incoming)
        .filter(|&e| tree_constraints.contains(&e.id()))
        .map(|e| e.source())
        .collect::<Vec<_>>();

    let n = tree_neighbors.len();

    let name = root_feature_name(ac_poset, node);
    let groups = if n > 0 && n <= 12 {
        construct_optimal_groups(ac_poset, node, &tree_neighbors, tree_constraints, abstract_feature_index)
    } else {
        construct_simple_groups(ac_poset, node, &tree_neighbors, tree_constraints, abstract_feature_index)
    };
    let estimated_number_of_configurations = groups.iter()
        .map(|g| g.estimated_number_of_configurations)
        .product::<f64>();
    Feature::new(name, groups, estimated_number_of_configurations, false)
}

fn construct_optimal_groups(
    ac_poset: &DiGraph<Concept, ()>,
    node: NodeIndex,
    tree_neighbors: &[NodeIndex],
    tree_constraints: &HashSet<EdgeIndex>,
    abstract_feature_index: &mut usize,
) -> Vec<Group> {
    let mut mandatory_features = mandatory_features(ac_poset, node);

    let features = ac_poset.edges_directed(node, Direction::Incoming)
        .filter(|&e| tree_constraints.contains(&e.id()))
        .map(|e| e.source())
        .map(|neighbor| construct_feature_tree(ac_poset, neighbor, tree_constraints, abstract_feature_index))
        .collect::<Vec<_>>();

    let assignments = construct_assignment_masks(ac_poset, node, tree_neighbors);
    let weight = |i: usize| features[i].estimated_number_of_configurations;
    let partition = optimal_groups::find2(features.len(), &assignments, weight)
        .collect::<Vec<_>>();

    let mut groups_content = partition.iter()
        .map(|(_, min, max)| (vec![], *min, *max))
        .collect::<Vec<_>>();
    let feature_to_group_map = partition.iter()
        .enumerate()
        .flat_map(|(j, (indices, _, _))| indices.iter().map(move |&i| (i, j)))
        .collect::<HashMap<_, _>>();
    for (i, feature) in features.into_iter().enumerate() {
        groups_content[feature_to_group_map[&i]].0.push(feature);
    }
    let mut groups = groups_content.into_iter()
        .map(|(features, min, max)| Group::new(features, min, max))
        .collect::<Vec<_>>();

    if mandatory_features.is_empty() && groups.len() == 1 {
        groups
    } else if groups.len() == 1 && groups[0].is_optional() {
        let mandatory_group = Group::mandatory(mandatory_features);
        groups.push(mandatory_group);
        groups
    } else {
        for group in groups {
            let name = format!("abstract_{abstract_feature_index}");
            let estimated_number_of_configurations = 
                group.estimated_number_of_configurations;
            let abstract_feature = Feature::new(name, vec![group], estimated_number_of_configurations, true);
            mandatory_features.push(abstract_feature);
            *abstract_feature_index += 1;
        }
        
        if mandatory_features.is_empty() {
            vec![]
        } else {
            vec![Group::mandatory(mandatory_features)]
        }
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

fn construct_simple_groups(
    ac_poset: &DiGraph<Concept, ()>,
    node: NodeIndex,
    tree_neighbors: &[NodeIndex],
    tree_constraints: &HashSet<EdgeIndex>,
    abstract_feature_index: &mut usize,
) -> Vec<Group> {
    let mandatory_features = mandatory_features(ac_poset, node);
    let optional_features = tree_neighbors.iter()
        .map(|&neighbor| construct_feature_tree(ac_poset, neighbor, tree_constraints, abstract_feature_index))
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