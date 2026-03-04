use crate::{cross_tree_constraint::CrossTreeConstraint, feature::Feature};

pub mod feature;
pub mod group;
pub mod cross_tree_constraint;

/// Stores a root feature and a collection of cross tree constraints.
pub struct FeatureModel {
    pub root_feature: Feature,
    pub cross_tree_constraints: Vec<CrossTreeConstraint>,
}

impl FeatureModel {
    pub fn count_features(&self) -> usize {
        self.root_feature.count_features()
    }
}