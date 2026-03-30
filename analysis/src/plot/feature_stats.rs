use std::path::Path;

use box_plotters::{box_plot::BoxPlot, quartiles::Quartiles};
use itertools::chain;
use plotters::{chart::ChartBuilder, prelude::{Circle, IntoSegmentedCoord, SegmentValue}, style::{BLACK, IntoFont}};

use crate::{plot::default_root, result::feature_stats::FeatureStats};

pub fn plot(feature_stats: &[FeatureStats], path: impl AsRef<Path>) -> anyhow::Result<()> {
    let caption = "Feature stats";
    let x_axis = ["Features", "Feature Dependencies"];

    let (features, feature_dependencies) = feature_stats
        .iter()
        .map(|s| (s.features as f64, s.feature_dependencies as f64))
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let min = chain!(&features, &feature_dependencies)
        .cloned()
        .min_by(f64::total_cmp)
        .unwrap();
    let max = chain!(&features, &feature_dependencies)
        .cloned()
        .min_by(f64::total_cmp)
        .unwrap();
    let y_axis = min - 10.0..max + 10.0;

    let root = default_root(path.as_ref())?;
    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption(caption, ("sans-serif", 32).into_font())
        .margin(30)
        .build_cartesian_2d(x_axis.into_segmented(), y_axis)?;

    let feature_quartiles = Quartiles::new_iqr(&features);
    let feature_dependency_quartiles = Quartiles::new_iqr(&feature_dependencies);

    let mut feature_box_plot = BoxPlot::vertical_from_key_quartiles(SegmentValue::CenterOf(&x_axis[0]), feature_quartiles);
    feature_box_plot
        .with_width(50.0)
        .with_whisker_width(50.0);
    
    let mut feature_dependency_box_plot = BoxPlot::vertical_from_key_quartiles(SegmentValue::CenterOf(&x_axis[1]), feature_dependency_quartiles);
    feature_dependency_box_plot
        .with_width(50.0)
        .with_whisker_width(50.0);

    chart.draw_series([feature_box_plot, feature_dependency_box_plot])?;

    let feature_outliers = features
        .iter()
        .filter(|&&f| f < feature_quartiles.lower_whisker || f > feature_quartiles.upper_whisker)
        .map(|&f| Circle::new((SegmentValue::CenterOf(&x_axis[0]), f), 2.0, BLACK));

    let feature_dependency_outliers = feature_dependencies
        .iter()
        .filter(|&&d| d < feature_dependency_quartiles.lower_whisker || d > feature_dependency_quartiles.upper_whisker)
        .map(|&d| Circle::new((SegmentValue::CenterOf(&x_axis[1]), d), 2.0, BLACK));

    chart.draw_series(chain!(feature_outliers, feature_dependency_outliers))?;

    Ok(())
}