use crate::group::Group;

/// Represents a feature inside a feature model.
/// Stores a name and a collection of groups.
/// A feature can be marked as abstract.
pub struct Feature {
    pub name: String,
    pub groups: Vec<Group>,
    pub is_abstract: bool,
    pub config_count: f64,
}

impl Feature {
    pub fn new(name: String, groups: Vec<Group>, is_abstract: bool) -> Feature {
        let config_count = groups.iter()
            .map(|g| g.config_count)
            .product();

        Feature { name, groups, is_abstract, config_count }
    }

    pub fn new_leaf(name: String, is_abstract: bool) -> Feature {
        Feature::new(name, vec![], is_abstract)
    }

    pub fn count_features(&self) -> usize {
        self.groups.iter()
            .flat_map(|g| g.features.iter())
            .count()
    }
}
