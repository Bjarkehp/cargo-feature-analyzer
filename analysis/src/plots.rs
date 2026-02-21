use std::{collections::BTreeMap, path::Path};

use cargo_toml::crate_id::CrateId;
use sorted_iter::SortedPairIterator;
use tokei::Language;

use crate::{MAX_DEPENDENCIES, MAX_FEATURES, ModelConfigurationStats, bounding_box::BoundingBox, plot::{default_chart, default_log_chart, default_mesh, default_root, draw_linear_regression, draw_linear_regression_log, draw_points}};

pub fn features_and_dependencies(dir: &Path, feature_stats: &BTreeMap<&CrateId, (usize, usize)>) -> anyhow::Result<()> {
    let caption = "Features and feature dependencies";
    let x_desc = "Features";
    let y_desc = "Feature dependencies";
    let file_name = "features_and_dependencies.png";
    let path = dir.join(file_name);
    
    let points = feature_stats.iter()
        .map(|(_id, &(f, d))| (f as f64, d as f64))
        .filter(|&(f, d)| f < MAX_FEATURES as f64 && d < MAX_DEPENDENCIES as f64)
        .collect::<Vec<_>>();

    let bounding_box = points.iter()
        .cloned()
        .collect::<BoundingBox>();
    let x_range = bounding_box.horizontal_range();
    let y_range = bounding_box.vertical_range();

    let root = default_root(&path)?;
    let mut chart = default_chart(&root, caption, x_range.clone(), y_range)?;
    default_mesh(&mut chart, x_desc, y_desc).draw()?;
    draw_points(&mut chart, &points)?;
    draw_linear_regression(&mut chart, &points, x_range)?;
    root.present()?;

    Ok(())
}

pub fn line_count_and_features(dir: &Path, line_counts: &BTreeMap<&CrateId, Language>, feature_stats: &BTreeMap<&CrateId, (usize, usize)>) -> anyhow::Result<()> {
    let caption = "Line count and features";
    let x_desc = "Line count";
    let y_desc = "Features";
    let file_name = "line_count_and_features.png";
    let path = dir.join(file_name);

    let points = line_counts.iter()
        .join(feature_stats.iter())
        .map(|(_, (language, &(features, _)))| (language.code as f64, features as f64))
        .collect::<Vec<_>>();
    
    let bounding_box = points.iter()
        .cloned()
        .collect::<BoundingBox>();
    let x_range = bounding_box.horizontal_range();
    let y_range = bounding_box.vertical_range();

    let root = default_root(&path)?;
    let mut chart = default_log_chart(&root, caption, x_range.clone(), y_range)?;
    default_mesh(&mut chart, x_desc, y_desc).draw()?;
    draw_points(&mut chart, &points)?;
    draw_linear_regression_log(&mut chart, &points, x_range)?;
    root.present()?;

    Ok(())
}

pub fn flat_vs_fca_exact(dir: &Path, flat: &BTreeMap<&CrateId, ModelConfigurationStats>, fca: &BTreeMap<&CrateId, ModelConfigurationStats>) -> anyhow::Result<()> {
    let caption = "Configuration number (Flat & FCA)";
    let x_desc = "Configuration number (Flat)";
    let y_desc = "Configuration number (FCA)";
    let file_name = "flat_vs_fca_exact.png";
    let path = dir.join(file_name);
    
    let points = flat.iter()
        .join(fca.iter())
        .map(|(_, (flat, fca))| (flat.exact, fca.exact))
        .collect::<Vec<_>>();

    let bounding_box = points.iter()
        .cloned()
        .collect::<BoundingBox>();
    let x_range = bounding_box.horizontal_range();
    let y_range = bounding_box.vertical_range();

    let root = default_root(&path)?;
    let mut chart = default_log_chart(&root, caption, x_range.clone(), y_range)?;
    default_mesh(&mut chart, x_desc, y_desc)
        .x_label_formatter(&|x| format!("{x:e}"))
        .y_label_formatter(&|y| format!("{y:e}"))
        .draw()?;
    draw_points(&mut chart, &points)?;
    root.present()?;

    Ok(())
}