use std::path::Path;

use itertools::izip;
use plotters::data::fitting_range;

use crate::{plot::{default_chart, default_mesh, default_root, draw_linear_regression, draw_points}, result::{feature_stats::FeatureStats, line_count::LineCountRow}};

pub fn plot(line_count_rows: &[LineCountRow], feature_stats: &[FeatureStats], path: impl AsRef<Path>) -> anyhow::Result<()> {
    let caption = "Line count and features";
    let x_desc = "Line count";
    let y_desc = "Features";

    let points = izip!(line_count_rows, feature_stats)
        .map(|(l, f)| (l.line_count as f64, f.features as f64))
        .collect::<Vec<_>>();

    let x_range = fitting_range(points.iter().map(|p| &p.0));
    let x_range = 0.0..x_range.end;
    let y_range = fitting_range(points.iter().map(|p| &p.1));
    let y_range = 0.0..y_range.end;

    let root = default_root(path.as_ref(), 1000, 600)?;
    let mut chart = default_chart(&root, caption, x_range.clone(), y_range)?;
    default_mesh(&mut chart, x_desc, y_desc).draw()?;
    draw_points(&mut chart, &points)?;
    draw_linear_regression(&mut chart, &points, x_range)?;
    root.present()?;

    Ok(())
}