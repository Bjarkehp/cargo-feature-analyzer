use std::path::Path;

use itertools::Itertools;
use ordered_float::OrderedFloat;
use plotters::{chart::ChartBuilder, prelude::{BitMapBackend, IntoDrawingArea}, series::LineSeries, style::{IntoFont, RED, WHITE}};

pub fn line_chart(
    path: &Path,
    caption: &str,
    data_iter: impl Iterator<Item = (f64, f64)>,
) -> anyhow::Result<()> {
    let data = data_iter.sorted_by_key(|&(x, _y)| OrderedFloat(x))
        .collect::<Vec<_>>();

    let x_min = *data.iter()
        .map(|&(x, _y)| OrderedFloat(x))
        .min()
        .expect("Expected some data");
    let x_max = *data.iter()
        .map(|&(x, _y)| OrderedFloat(x))
        .max()
        .expect("Expected some data");
    let y_min = *data.iter()
        .map(|&(_x, y)| OrderedFloat(y))
        .min()
        .expect("Expected some data");
    let y_max = *data.iter()
        .map(|&(_x, y)| OrderedFloat(y))
        .max()
        .expect("Expected some data");

    let root = BitMapBackend::new(path, (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let root = root.margin(10, 10, 10, 10);
    let mut chart = ChartBuilder::on(&root)
        .caption(caption, ("sans-serif", 20).into_font())
        .x_label_area_size(20)
        .y_label_area_size(40)
        .build_cartesian_2d(x_min..x_max + 1.0, y_min..y_max + 1.0)?;
    
    chart.configure_mesh()
        .x_labels(10)
        .y_labels(10)
        .draw()?;

    chart.draw_series(LineSeries::new(
        data.clone(),
        &RED,
    ))?;

    root.present()?;

    Ok(())
}