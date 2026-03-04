use crate::{cross_tree_constraint::CrossTreeConstraint, feature::Feature};

pub mod feature;
pub mod group;
pub mod cross_tree_constraint;
pub mod uvl;
pub mod indent;

/// Stores a root feature and a collection of cross tree constraints.
pub struct FeatureModel {
    pub root_feature: Feature,
    pub cross_tree_constraints: Vec<CrossTreeConstraint>,
}

impl FeatureModel {
    pub fn new(root_feature: Feature, cross_tree_constraints: Vec<CrossTreeConstraint>) -> FeatureModel {
        FeatureModel { root_feature, cross_tree_constraints }
    }

    pub fn count_features(&self) -> usize {
        self.root_feature.count_features()
    }
}