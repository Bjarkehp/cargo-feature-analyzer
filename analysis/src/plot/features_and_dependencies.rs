use std::path::Path;

use crate::result::feature_stats::FeatureStats;
use plotters::data::fitting_range;

use crate::plot::{default_chart, default_mesh, default_root, draw_linear_regression, draw_points};

pub fn plot(feature_stats: &[FeatureStats], path: impl AsRef<Path>) -> anyhow::Result<()> {
    let caption = "Features and feature dependencies";
    let x_desc = "Features";
    let y_desc = "Feature dependencies";

    let points = feature_stats.iter()
        .map(|s| (s.features as f64, s.feature_dependencies as f64))
        .collect::<Vec<_>>();

    let x_range = fitting_range(points.iter().map(|p| &p.0));
    let y_range = fitting_range(points.iter().map(|p| &p.1));

    let root = default_root(path.as_ref())?;
    let mut chart = default_chart(&root, caption, x_range.clone(), y_range)?;
    default_mesh(&mut chart, x_desc, y_desc).draw()?;
    draw_points(&mut chart, &points)?;
    draw_linear_regression(&mut chart, &points, x_range)?;
    root.present()?;

    Ok(())
}