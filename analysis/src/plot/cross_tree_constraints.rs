use std::path::Path;

use itertools::izip;
use plotters::data::fitting_range;

use crate::{plot::{default_chart, default_mesh, default_root, draw_points}, result::model_stats::ModelStats};

pub fn plot(declared: &[ModelStats], fca: &[ModelStats], path: impl AsRef<Path>) -> anyhow::Result<()> {
    let caption = "Cross-tree constraints (Declared & FCA)";
    let x_desc = "Cross-tree constraints (Declared)";
    let y_desc = "Cross-tree constraints (FCA)";

    let points = izip!(declared, fca)
        .map(|(d, f)| (d.cross_tree_constraints as f64, f.cross_tree_constraints as f64))
        .collect::<Vec<_>>();

    let x_range = fitting_range(points.iter().map(|p| &p.0));
    let y_range = fitting_range(points.iter().map(|p| &p.1));

    let root = default_root(path.as_ref(), 1000, 600)?;
    let mut chart = default_chart(&root, caption, x_range, y_range)?;
    default_mesh(&mut chart, x_desc, y_desc).draw()?;
    draw_points(&mut chart, &points)?;
    root.present()?;

    Ok(())
}