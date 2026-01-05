use std::{collections::BTreeMap, path::Path};

use cargo_toml::crate_id::CrateId;

use crate::{MAX_DEPENDENCIES, MAX_FEATURES, plot::{bounding_box::BoundingBox, box_plot}};

pub fn features_and_dependencies(dir: &Path, feature_stats: &BTreeMap<&CrateId, (usize, usize)>) -> anyhow::Result<()> {
    box_plot::plot(
        &dir.join("features_and_dependencies_box_plot.png"), 
        "Features and feature dependencies", 
        "Features",
        "Feature dependencies",
        BoundingBox::all(1.1),
        feature_stats.iter()
            .map(|(_id, &(f, d))| (f as f64, d as f64))
            .filter(|&(f, d)| f < MAX_FEATURES as f64 && d < MAX_DEPENDENCIES as f64), 
        [0.0, 10.0, 25.0, 50.0, 100.0].into_iter()
    )
}