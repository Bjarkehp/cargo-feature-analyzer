use std::{iter::successors, ops::Range, path::Path};

use anyhow::anyhow;
use plotters::{chart::{ChartBuilder, ChartContext}, coord::{Shift, ranged1d::ValueFormatter, types::RangedCoordf64}, prelude::{BitMapBackend, Cartesian2d, Circle, DrawingArea, IntoDrawingArea, IntoLogRange, LogCoord, PathElement, Ranged}, series::LineSeries, style::{BLACK, BLUE, Color, IntoFont, WHITE}};

use polyfit_rs::polyfit_rs;

use crate::correlation;

pub type DefaultRoot<'a> = DrawingArea<BitMapBackend<'a>, Shift>;
pub type DefaultChartContext<'a> = ChartContext<'a, BitMapBackend<'a>, Cartesian2d<RangedCoordf64, RangedCoordf64>>;
pub type DefaultLogChartContext<'a> = ChartContext<'a, BitMapBackend<'a>, Cartesian2d<LogCoord<f64>, LogCoord<f64>>>;

pub fn default_root(path: &Path) -> anyhow::Result<DefaultRoot<'_>> {
    let root = BitMapBackend::new(path, (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;
    Ok(root)
}

pub fn default_chart<'a>(
    root: &DefaultRoot<'a>, 
    caption: &str, 
    x_range: Range<f64>, 
    y_range: Range<f64>,
) -> anyhow::Result<DefaultChartContext<'a>> {
    let chart = ChartBuilder::on(root)
        .caption(caption, ("sans-serif", 32).into_font())
        .margin(30)
        .x_label_area_size(50)
        .y_label_area_size(70)
        .build_cartesian_2d(x_range, y_range)?;

    Ok(chart)
}

pub fn default_log_chart<'a>(
    root: &DefaultRoot<'a>, 
    caption: &str, 
    x_range: Range<f64>, 
    y_range: Range<f64>,
) -> anyhow::Result<DefaultLogChartContext<'a>> {
    let chart = ChartBuilder::on(root)
        .caption(caption, ("sans-serif", 32).into_font())
        .margin(30)
        .x_label_area_size(50)
        .y_label_area_size(70)
        .build_cartesian_2d(x_range.log_scale(), y_range.log_scale())?;
    
    Ok(chart)
}

pub fn default_mesh<X: Ranged<ValueType = f64> + ValueFormatter<f64>, Y: Ranged<ValueType = f64> + ValueFormatter<f64>>(
    chart: &mut ChartContext<'_, BitMapBackend<'_>, Cartesian2d<X, Y>>,
    x_desc: &str,
    y_desc: &str,
) -> anyhow::Result<()> {
    chart.configure_mesh()
        .x_desc(x_desc)
        .y_desc(y_desc)
        .x_label_style(("sans-serif", 18).into_font())
        .y_label_style(("sans-serif", 18).into_font())
        .axis_desc_style(("sans-serif", 24).into_font())
        .max_light_lines(0)
        .draw()?;

    Ok(())
}

pub fn draw_points<X: Ranged<ValueType = f64> + ValueFormatter<f64>, Y: Ranged<ValueType = f64> + ValueFormatter<f64>>(
    chart: &mut ChartContext<'_, BitMapBackend<'_>, Cartesian2d<X, Y>>,
    points: &[(f64, f64)],
) -> anyhow::Result<()> {
    let circles = points.iter()
        .cloned()
        .map(|p| Circle::new(p, 2.0, BLACK.filled()));

    chart.draw_series(circles)?;

    Ok(())
}

pub fn draw_linear_regression<'a, X: Ranged<ValueType = f64> + ValueFormatter<f64>, Y: Ranged<ValueType = f64> + ValueFormatter<f64>>(
    chart: &mut ChartContext<'a, BitMapBackend<'a>, Cartesian2d<X, Y>>,
    points: &[(f64, f64)],
    x_range: Range<f64>,
) -> anyhow::Result<()> {
    let (x, y): (Vec<_>, Vec<_>) = points.iter().cloned().unzip();

    let parameters = polyfit_rs::polyfit(&x, &y, 1)
        .map_err(|e| anyhow!("{e}"))?;
    let b = parameters[0];
    let a = parameters[1];
    let r = correlation::pearson(points);
    let ticks = ticks(x_range, 100);
    let line = LineSeries::new(ticks.map(|x| (x, a * x + b)), BLUE);

    chart.draw_series(line)?
        .label(format!("r = {r:.2}"))
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart.configure_series_labels()
        .border_style(BLACK)
        .draw()?;

    Ok(())
}

pub fn draw_linear_regression_log<'a, X: Ranged<ValueType = f64> + ValueFormatter<f64>, Y: Ranged<ValueType = f64> + ValueFormatter<f64>>(
    chart: &mut ChartContext<'a, BitMapBackend<'a>, Cartesian2d<X, Y>>,
    points: &[(f64, f64)],
    x_range: Range<f64>,
) -> anyhow::Result<()> {
    let (x, y): (Vec<_>, Vec<_>) = points.iter().cloned().unzip();

    let parameters = polyfit_rs::polyfit(&x, &y, 1)
        .map_err(|e| anyhow!("{e}"))?;
    let b = parameters[0];
    let a = parameters[1];
    let r = correlation::pearson_log(points);
    let ticks = log_ticks(x_range, 100);
    let line = LineSeries::new(ticks.map(|x| (x, a * x + b)), BLUE);

    chart.draw_series(line)?
        .label(format!("r = {r:.2}"))
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart.configure_series_labels()
        .border_style(BLACK)
        .draw()?;

    Ok(())
}

fn log_ticks(r: Range<f64>, count: usize) -> impl Iterator<Item = f64> {
    let r_log = r.start.log10()..r.end.log10();
    ticks(r_log, count).map(|x| 10_f64.powf(x))
}

fn ticks(r: Range<f64>, count: usize) -> impl Iterator<Item = f64> {
    let step = (r.end - r.start) / (count - 1) as f64;
    successors(Some(r.start), move |x| Some(x + step)).take(count)
}