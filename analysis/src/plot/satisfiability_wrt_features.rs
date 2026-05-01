use std::{collections::BTreeMap, path::Path};

use cargo_toml::crate_id::CrateId;
use plotters::data::fitting_range;
use sorted_iter::SortedPairIterator;

use crate::{plot::{default_chart, default_mesh, default_root, draw_points, integer_formatter}, result::{feature_stats::FeatureStats, satisfiability::SatisfiabilityRow}};

pub fn plot(
    satisfiability: &BTreeMap<CrateId, SatisfiabilityRow>,
    feature_stats: &BTreeMap<CrateId, FeatureStats>,
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let caption = "Satisfiability w.r.t. features";
    let x_desc = "Number of features";
    let y_desc = "Satisfiability";

    let points = feature_stats
        .iter()
        .join(satisfiability.iter())
        .map(|(_id, (f, s))| (f.features as f64, s.satisfiability))
        .collect::<Vec<_>>();

    let x_range = fitting_range(points.iter().map(|p| &p.0));
    let y_range = fitting_range(points.iter().map(|p| &p.1));

    let root = default_root(path.as_ref(), 1000, 600)?;
    let mut chart = default_chart(&root, caption, x_range, y_range)?;
    default_mesh(&mut chart, x_desc, y_desc)
        .x_label_formatter(&integer_formatter)
        .draw()?;

    draw_points(&mut chart, &points)?;
    root.present()?;

    Ok(())
}