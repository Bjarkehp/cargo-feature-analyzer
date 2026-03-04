use crate::feature::Feature;

/// Represents a group inside a feature model.
/// A group stores a collection of features,
/// and a minimum and maximum cardinality.
pub struct Group {
    pub features: Vec<Feature>,
    pub min: usize,
    pub max: usize,
    pub config_count: f64,
}

impl Group {
    pub fn new(features: Vec<Feature>, min: usize, max: usize) -> Group {
        let n = features.len();
        let mut dp = vec![0.0; n + 1];
        dp[0] = 1.0;

        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            for k in (1..=n).rev() {
                dp[k] += dp[k - 1] * features[i].config_count;
            }
        }

        let config_count = (min..=max)
            .map(|k| dp[k])
            .sum();

        Group { features, min, max, config_count }
    }

    /// Creates a new mandatory group.
    pub fn mandatory(features: Vec<Feature>) -> Group {
        let n = features.len();
        Group::new(features, n, n)
    }

    /// Creates a new optional group.
    pub fn optional(features: Vec<Feature>) -> Group {
        let n = features.len();
        Group::new(features, 0, n)
    }

    /// Determines if the group is mandatory.
    pub fn is_mandatory(&self) -> bool {
        let n = self.features.len();
        self.min == n && self.max == n
    }

    /// Determines if the group is optional.
    pub fn is_optional(&self) -> bool {
        self.min == 0 && self.max == self.features.len()
    }
}