use std::{collections::BTreeMap, path::Path};

use cargo_toml::crate_id::CrateId;
use plotters::data::fitting_range;
use sorted_iter::SortedPairIterator;

use crate::{plot::{default_log_y_chart, default_mesh, default_root, draw_linear_regression, draw_points}, result::{feature_metrics::FeatureMetrics, feature_stats::FeatureStats}};

pub fn plot(
    feature_stats: &BTreeMap<CrateId, FeatureStats>, 
    feature_metrics: &BTreeMap<CrateId, FeatureMetrics>,
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let caption = "NOFC and LOF";
    let x_desc = "NOFC";
    let y_desc = "LOF";

    let points = feature_stats
        .iter()
        .join(feature_metrics.iter())
        .map(|(_id, (s, m))| (s.features as f64, m.lof as f64))
        .collect::<Vec<_>>();

    let x_range = fitting_range(points.iter().map(|p| &p.0));
    let y_range = fitting_range(points.iter().map(|p| &p.1));

    let root = default_root(path.as_ref(), 1000, 600)?;
    let mut chart = default_log_y_chart(&root, caption, x_range.clone(), y_range)?;
    default_mesh(&mut chart, x_desc, y_desc).draw()?;
    draw_points(&mut chart, &points)?;
    draw_linear_regression(&mut chart, &points, x_range)?;
    root.present()?;

    Ok(())
}