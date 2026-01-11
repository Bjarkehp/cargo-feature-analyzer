use std::{collections::BTreeMap, path::Path};

use cargo_toml::crate_id::CrateId;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use sorted_iter::SortedPairIterator;
use tokei::Language;

use crate::{MAX_DEPENDENCIES, MAX_FEATURES, plot::{bounding_box::BoundingBox, box_plot}};

pub fn features_and_dependencies(dir: &Path, feature_stats: &BTreeMap<&CrateId, (usize, usize)>) -> anyhow::Result<()> {
    let points = feature_stats.iter()
        .map(|(_id, &(f, d))| (f as f64, d as f64))
        .filter(|&(f, d)| f < MAX_FEATURES as f64 && d < MAX_DEPENDENCIES as f64);

    let boxes = [0.0, 10.0, 25.0, 50.0, 100.0].into_iter();
    
    box_plot::plot(
        &dir.join("features_and_dependencies.png"), 
        "Features and feature dependencies", 
        "Features",
        "Feature dependencies",
        BoundingBox::all(1.1),
        points, 
        boxes,
    )
}

pub fn line_count_and_features(dir: &Path, line_counts: &BTreeMap<&CrateId, Language>, feature_stats: &BTreeMap<&CrateId, (usize, usize)>) -> anyhow::Result<()> {
    let points = line_counts.iter()
        .join(feature_stats.iter())
        .map(|(_, (language, &(features, _)))| (language.code as f64, features as f64))
        .collect::<Vec<_>>();

    let max_lines = points.iter()
        .map(|(l, _)| l)
        .cloned()
        .max_by_key(|&l| OrderedFloat(l))
        .unwrap_or(0.0);

    let boxes = (2..)
        .flat_map(|n| {
            let base = 10.0_f64.powi(n);
            [base, 2.5 * base, 5.0 * base]
        })
        .take_while_inclusive(|&n| n < max_lines)
        .collect::<Vec<_>>();

    println!("{boxes:?}");
    
    box_plot::plot_log(
        &dir.join("line_count_and_features.png"),
        "Line count and features",
        "Line count",
        "Features",
        BoundingBox::all(1.0),
        points.into_iter(),
        boxes.into_iter(),
    )
}