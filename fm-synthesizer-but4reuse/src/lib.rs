use std::{borrow::Cow, collections::{HashMap, HashSet}};

use anyhow::Context;
use cargo_toml::feature_dependencies::{self};
use configuration_scraper::configuration::Configuration;
use feature_model::{FeatureModel, cross_tree_constraint, feature::Feature};
use itertools::{Either, Itertools, iproduct};
use petgraph::Direction;
use slotmap::{SecondaryMap, SlotMap, new_key_type};
use smallvec::{SmallVec, smallvec};

new_key_type! { struct FeatureKey; }

type FeatureVec = SmallVec<[FeatureKey; 8]>;

pub fn synthesize<'a>(cargo_toml: &'a toml::Table, configurations: &[Configuration]) -> anyhow::Result<FeatureModel> {
    let mut features = SlotMap::<FeatureKey, Cow<'a, str>>::default();
    
    let mut name_to_feature = HashMap::new();
    let mut cardinality = SecondaryMap::<FeatureKey, Cardinality>::default();
    let mut feature_dependencies = SecondaryMap::<FeatureKey, FeatureVec>::default();
    let mut feature_parents = SecondaryMap::<FeatureKey, FeatureKey>::default();
    let mut feature_children = SecondaryMap::<FeatureKey, FeatureVec>::default();
    let mut is_ancestor = HashSet::new();
    let mut is_dependency = HashSet::new();
    let mut is_mandatory = HashSet::new();
    
    let root_name = cargo_toml
        .get("package")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .context("package.name is missing in toml")?;
    
    let root = features.insert(root_name.into());
    name_to_feature.insert(root_name, root);
    feature_dependencies.insert(root, smallvec![]);
    feature_children.insert(root, smallvec![]);

    let feature_str_dependencies = feature_dependencies::from_cargo_toml(cargo_toml)?;
    for name in feature_str_dependencies.nodes() {
        let feature = features.insert(name.into());
        name_to_feature.insert(name, feature);
        feature_children.insert(feature, smallvec![]);
    }

    for name in feature_str_dependencies.nodes() {
        let feature = name_to_feature[name];
        let mut dependencies = feature_str_dependencies
            .neighbors_directed(name, Direction::Outgoing)
            .map(|s| name_to_feature[s])
            .collect::<FeatureVec>();

        if dependencies.is_empty() {
            dependencies.push(root);
        }

        for &dependency in dependencies.iter() {
            is_dependency.insert((feature, dependency));
        }
        
        feature_dependencies.insert(feature, dependencies);
    }

    for &(f1, f2) in is_dependency.iter() {
        if f1 < f2 && !feature_parents.contains_key(f2) && is_dependency.contains(&(f2, f1)) {
            feature_children[f1].push(f2);
            feature_parents.insert(f2, f1);
            is_mandatory.insert(f2);
        }
    }

    for ((f1, f1_name), (f2, f2_name)) in iproduct!(features.iter(), features.iter()) {
        if f1 == f2 || feature_dependencies[f1].contains(&f2) || is_mandatory.contains(&f1) || is_mandatory.contains(&f2) {
            continue;
        }

        let exists_in_configurations = configurations
            .iter()
            .any(|c| c.is_enabled(f1_name));

        let configuration_implication = configurations
            .iter()
            .filter(|c| c.is_enabled(f1_name))
            .all(|c| c.is_enabled(f2_name));

        if exists_in_configurations && configuration_implication {
            feature_dependencies[f1].push(f2);
            is_dependency.insert((f1, f2));
        }
    }

    let features_without_root = features
        .iter()
        .filter(|&(f, _)| f != root);

    let exclude_constraints = iproduct!(features_without_root.clone(), features_without_root)
        .filter(|((f1, _), (f2, _))| f1 < f2)
        .filter(|((_, f1_name), (_, f2_name))| {
            configurations.iter()
                .all(|c| !(c.is_enabled(f1_name) && c.is_enabled(f2_name)))
        })
        .map(|((f1, _), (f2, _))| (f1, f2))
        .collect::<HashSet<_>>();

    let mut remaining_exclude_constraints = exclude_constraints.clone();
    let mut remaining_implication_constraints = feature_dependencies
        .iter()
        .flat_map(|(f, ds)| ds.iter().map(move |&d| (f, d)))
        .collect::<HashSet<_>>();

    let mut id = 0;

    for &(f1, f2) in exclude_constraints.iter() {
        let alt1 = feature_parents.get(f1);
        let alt2 = feature_parents.get(f2);

        #[allow(clippy::collapsible_match)]
        match (alt1, alt2) {
            (None, None) => {
                id += 1;
                let parent = features.insert(format!("Alternative_{id}").into());
                cardinality.insert(parent, Cardinality::Alternative);
                feature_children.insert(parent, smallvec![]);
                try_add_to_alt_group(&mut feature_children, &mut feature_parents, &mut feature_dependencies, &mut is_ancestor, &mut remaining_exclude_constraints, &exclude_constraints, f1, parent);
                try_add_to_alt_group(&mut feature_children, &mut feature_parents, &mut feature_dependencies, &mut is_ancestor, &mut remaining_exclude_constraints, &exclude_constraints, f2, parent);
            },
            (None, Some(&parent)) => {
                try_add_to_alt_group(&mut feature_children, &mut feature_parents, &mut feature_dependencies, &mut is_ancestor, &mut remaining_exclude_constraints, &exclude_constraints, f1, parent);
            },
            (Some(&parent), None) => {
                try_add_to_alt_group(&mut feature_children, &mut feature_parents, &mut feature_dependencies, &mut is_ancestor, &mut remaining_exclude_constraints, &exclude_constraints, f2, parent);
            },
            _ => {},
        }
    }

    for feature in features.keys() {
        if feature == root || feature_parents.contains_key(feature) {
            continue;
        }

        let mut parent_candidates = feature_dependencies[feature]
            .iter()
            .filter(|&&dependency| !is_dependency.contains(&(dependency, feature)))
            .copied()
            .collect::<HashSet<_>>();

        let to_filter = parent_candidates
            .iter()
            .filter(|&&f1| {
                parent_candidates
                    .iter()
                    .filter(|&&f2| f1 < f2)
                    .any(|&f2| is_ancestor.contains(&(f1, f2)))
            })
            .copied()
            .collect::<Vec<_>>();

        for candidate in to_filter {
            parent_candidates.remove(&candidate);
        }

        if let Some(&candidate) = parent_candidates.iter().next() {
            let alt_candidate = parent_candidates
                .iter()
                .find(|&&c| feature_parents.get(c).and_then(|&pc| cardinality.get(pc)) == Some(&Cardinality::Alternative))
                .copied();

            let parent = alt_candidate.unwrap_or(candidate);

            feature_children[parent].push(feature);
            feature_parents.insert(feature, parent);
            remaining_implication_constraints.remove(&(feature, parent));
        } else if feature != root {
            feature_children[root].push(feature);
            feature_parents.insert(feature, root);
            remaining_implication_constraints.remove(&(feature, root));
        }
    }

    let fm = construct_fm(root, &features, &cardinality, &feature_children, &is_mandatory, &remaining_exclude_constraints, &remaining_implication_constraints);
    Ok(fm)
}

fn construct_fm(
    root: FeatureKey,
    features: &SlotMap<FeatureKey, Cow<str>>,
    cardinality: &SecondaryMap<FeatureKey, Cardinality>,
    feature_children: &SecondaryMap<FeatureKey, FeatureVec>,
    is_mandatory: &HashSet<FeatureKey>,
    exclude_constraints: &HashSet<(FeatureKey, FeatureKey)>,
    implication_constraints: &HashSet<(FeatureKey, FeatureKey)>,
) -> FeatureModel {
    let feature = construct_feature(root, features, cardinality, feature_children, is_mandatory);

    let exclude_constraints = exclude_constraints
        .iter()
        .map(|&(f1, f2)| (features[f1].as_ref(), features[f2].as_ref()))
        .map(|(f1, f2)| cross_tree_constraint::exclusive(f1, f2));
    
    let implication_constraints = implication_constraints
        .iter()
        .map(|&(f1, f2)| (features[f1].as_ref(), features[f2].as_ref()))
        .map(|(f1, f2)| cross_tree_constraint::implies(f1, f2));

    let constraints = exclude_constraints
        .chain(implication_constraints)
        .collect();

    FeatureModel::new(feature, constraints)
}

fn construct_feature(
    key: FeatureKey,
    features: &SlotMap<FeatureKey, Cow<str>>,
    cardinality: &SecondaryMap<FeatureKey, Cardinality>,
    feature_children: &SecondaryMap<FeatureKey, FeatureVec>,
    is_mandatory: &HashSet<FeatureKey>,
) -> Feature {
    let name = features[key].clone().into_owned();
    let (mandatory_children, children): (Vec<_>, Vec<_>) = feature_children[key]
        .iter()
        .partition_map(|&f| {
            let feature = construct_feature(f, features, cardinality, feature_children, is_mandatory);
            if is_mandatory.contains(&f) {
                Either::Left(feature)
            } else {
                Either::Right(feature)
            }
        });
    let n = children.len();
    let (min, max) = match cardinality.get(key) {
        Some(Cardinality::Alternative) => (1, 1),
        None => (0, n),
    };

    let mut groups = if children.is_empty() {
        vec![]
    } else {
        vec![feature_model::group::Group::new(children, min, max)]
    };

    if !mandatory_children.is_empty() {
        let n = mandatory_children.len();
        let group = feature_model::group::Group::new(mandatory_children, n, n);
        groups.push(group);
    }

    let is_abstract = false;

    Feature::new(
        name,
        groups,
        is_abstract
    )
}

#[allow(clippy::too_many_arguments)]
fn try_add_to_alt_group(
    feature_children: &mut SecondaryMap<FeatureKey, FeatureVec>, 
    feature_parents: &mut SecondaryMap<FeatureKey, FeatureKey>,
    feature_dependencies: &mut SecondaryMap<FeatureKey, FeatureVec>,
    is_ancestor: &mut HashSet<(FeatureKey, FeatureKey)>, 
    remaining_exclude_constraints: &mut HashSet<(FeatureKey, FeatureKey)>,
    exclude_constraints: &HashSet<(FeatureKey, FeatureKey)>,
    f1: FeatureKey, 
    parent: FeatureKey,
) {
    let all_found = feature_children[parent]
        .iter()
        .all(|&f2| exclude_constraints.contains(&(f1, f2)) || exclude_constraints.contains(&(f2, f1)));

    if all_found {
        for &f2 in feature_children[parent].iter() {
            is_ancestor.insert((f1, f2));
            is_ancestor.insert((f2, f1));
            let query = if f1 < f2 {
                (f1, f2)
            } else {
                (f2, f1)
            };
            remaining_exclude_constraints.remove(&query);
        }

        feature_children[parent].push(f1);
        feature_parents.insert(f1, parent);

        let dependencies = feature_dependencies[f1]
            .iter()
            .copied()
            .collect::<HashSet<_>>();

        if let Some(parent_dependencies) = feature_dependencies.get_mut(parent) {
            parent_dependencies.retain(|d| dependencies.contains(d));
        } else {
            let dependencies = feature_dependencies[f1].clone();
            feature_dependencies.insert(parent, dependencies);
        }
    }

}

#[derive(PartialEq, Eq)]
enum Cardinality {
    Alternative,
}