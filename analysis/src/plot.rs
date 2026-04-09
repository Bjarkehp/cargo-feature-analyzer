pub mod features_and_dependencies;
pub mod line_count_and_features;
pub mod declared_vs_fca;
pub mod cross_tree_constraints;
pub mod feature_stats;
pub mod default_configs;
pub mod unique_configs;

use std::{iter::successors, ops::Range, path::Path};

use anyhow::anyhow;
use plotters::{chart::{ChartBuilder, ChartContext, MeshStyle}, coord::{Shift, ranged1d::ValueFormatter, types::RangedCoordf64}, prelude::{BitMapBackend, Cartesian2d, Circle, DrawingArea, DrawingBackend, IntoDrawingArea, IntoLogRange, LogCoord, PathElement, Ranged, SegmentValue}, series::LineSeries, style::{BLACK, BLUE, Color, IntoFont, WHITE}};

use polyfit_rs::polyfit_rs;

use crate::correlation;

pub type DefaultRoot<'a> = DrawingArea<BitMapBackend<'a>, Shift>;
pub type DefaultChartContext<'a> = ChartContext<'a, BitMapBackend<'a>, Cartesian2d<RangedCoordf64, RangedCoordf64>>;
pub type DefaultLogChartContext<'a> = ChartContext<'a, BitMapBackend<'a>, Cartesian2d<LogCoord<f64>, LogCoord<f64>>>;

pub fn default_root(path: &Path, width: u32, height: u32) -> anyhow::Result<DefaultRoot<'_>> {
    let root = BitMapBackend::new(path, (width, height)).into_drawing_area();
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

pub fn default_mesh<'a, 'b, X: Ranged<ValueType = f64> + ValueFormatter<f64>, Y: Ranged<ValueType = f64> + ValueFormatter<f64>, DB: DrawingBackend>(
    chart: &'b mut ChartContext<'a, DB, Cartesian2d<X, Y>>,
    x_desc: &str,
    y_desc: &str,
) -> MeshStyle<'a, 'b, X, Y, DB> {
    let mut mesh_style = chart.configure_mesh();
    mesh_style
        .x_desc(x_desc)
        .y_desc(y_desc)
        .x_label_style(("sans-serif", 18).into_font())
        .y_label_style(("sans-serif", 18).into_font())
        .axis_desc_style(("sans-serif", 24).into_font())
        .max_light_lines(0);
    
    mesh_style
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
    let ticks = ticks(x_range, 500);
    let line = LineSeries::new(ticks.map(|x| (x, a * x + b)), BLUE);

    chart.draw_series(line)?
        .label(format!("r = {r:.2}"))
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart.configure_series_labels()
        .border_style(BLACK)
        .label_font(("sans-serif", 18).into_font())
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
    let ticks = log_ticks(x_range, 500);
    let line = LineSeries::new(ticks.map(|x| (x, a * x + b)), BLUE);

    chart.draw_series(line)?
        .label(format!("r = {r:.2}"))
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart.configure_series_labels()
        .border_style(BLACK)
        .label_font(("sans-serif", 18).into_font())
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

pub fn log_label_formatter(x: &f64) -> String {
    let exponent = x.log10();
    let int_exponent = exponent.round() as u32;
    let exponent_super_script = superscript(int_exponent as i32);

    if (exponent - int_exponent as f64).abs() < 0.01 {
        format!("10{exponent_super_script}")
    } else {
        let coefficient = x / 10_i32.pow(int_exponent) as f64;
        format!("{coefficient} × 10{exponent_super_script}")
    }
}

pub fn integer_formatter(x: &f64) -> String {
    format!("{x:.0}")
}

pub fn segment_formatter(segment: &SegmentValue<&&str>) -> String {
    match segment {
        SegmentValue::Exact(s) => (**s).to_owned(),
        SegmentValue::CenterOf(s) => (**s).to_owned(),
        SegmentValue::Last => format!("{segment:?}"),
    }
}

fn superscript(n: i32) -> String {
    let superscript_map = |c: char| match c {
        '0' => '⁰',
        '1' => '¹', 
        '2' => '²', 
        '3' => '³',
        '4' => '⁴', 
        '5' => '⁵', 
        '6' => '⁶', 
        '7' => '⁷',
        '8' => '⁸', 
        '9' => '⁹', 
        '-' => '⁻',
        _ => panic!("Invalid character for superscript {c}")
    };

    n.to_string()
        .chars()
        .map(superscript_map)
        .collect::<String>()
}