use std::{collections::BTreeMap, path::Path};

use crate::result::model_stats::ModelStats;
use cargo_toml::crate_id::CrateId;
use plotters::data::fitting_range;
use sorted_iter::SortedPairIterator;

use crate::plot::{default_log_chart, default_mesh, default_root, draw_points, log_label_formatter};

pub fn plot(
    declared: &BTreeMap<CrateId, ModelStats>, 
    fca: &BTreeMap<CrateId, ModelStats>, 
    path: impl AsRef<Path>
) -> anyhow::Result<()> {
    let caption = "Configuration number (Declared & FCA)";
    let x_desc = "Configuration number (Declared)";
    let y_desc = "Configuration number (FCA)";

    let points = declared
        .iter()
        .join(fca.iter())
        .map(|(_id, (d, f))| (d.config_exact, f.config_exact))
        .collect::<Vec<_>>();

    let x_range = fitting_range(points.iter().map(|p| &p.0));
    let y_range = fitting_range(points.iter().map(|p| &p.1));

    let root = default_root(path.as_ref(), 1000, 600)?;
    let mut chart = default_log_chart(&root, caption, x_range, y_range)?;
    default_mesh(&mut chart, x_desc, y_desc)
        .x_label_formatter(&log_label_formatter)
        .y_label_formatter(&log_label_formatter)
        .draw()?;
    draw_points(&mut chart, &points)?;
    root.present()?;

    Ok(())
}