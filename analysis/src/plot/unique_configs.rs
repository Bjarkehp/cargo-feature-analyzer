use std::path::Path;

use box_plotters::{box_plot::BoxPlot, quartiles::Quartiles};
use plotters::{chart::ChartBuilder, prelude::Circle, style::{BLACK, IntoFont}};

use crate::{plot::{default_root, integer_formatter}, result::configuration_stats::ConfigStats};

pub fn plot(config_stats: &[ConfigStats], path: impl AsRef<Path>) -> anyhow::Result<()> {
    let caption = "Unique configurations";

    let values = config_stats
        .iter()
        .map(|s| s.unique_configuration_count as f64)
        .collect::<Vec<_>>();

    let max = values
        .iter()
        .cloned()
        .max_by(f64::total_cmp)
        .unwrap();
    let x_axis = 0.0..max;

    let root = default_root(path.as_ref(), 1000, 200)?;
    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption(caption, ("sans-serif", 32).into_font())
        .margin(30)
        .build_cartesian_2d(x_axis, -1.0..1.0)?;

    chart
        .configure_mesh()
        .x_label_style(("sans-serif", 18).into_font())
        .x_label_formatter(&integer_formatter)
        .disable_y_axis()
        .disable_mesh()
        .draw()?;

    let quartiles = Quartiles::new_iqr(&values);

    let mut box_plot = BoxPlot::horizontal_from_key_quartiles(0.0, quartiles);
    box_plot
        .with_width(50.0)
        .with_whisker_width(50.0);

    chart.draw_series([box_plot])?;

    let outliers = values
        .iter()
        .filter(|&&c| c < quartiles.lower_whisker || c > quartiles.upper_whisker)
        .map(|&c| Circle::new((c, 0.0), 2.0, BLACK));

    chart.draw_series(outliers)?;

    Ok(())
}