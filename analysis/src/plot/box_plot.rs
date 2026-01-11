use std::{ops::Range, path::Path};

use itertools::Itertools;
use ordered_float::OrderedFloat;
use plotters::{chart::ChartBuilder, prelude::{BitMapBackend, IntoDrawingArea, IntoLogRange, PathElement, Rectangle}, style::{BLACK, BLUE, Color, IntoFont, ShapeStyle, WHITE}};

use crate::plot::bounding_box::BoundingBox;

pub fn plot(
    path: &Path,
    caption: &str,
    x_desc: &str,
    y_desc: &str,
    margin: BoundingBox,
    data_iter: impl Iterator<Item = (f64, f64)>,
    box_iter: impl Iterator<Item = f64>,
) -> anyhow::Result<()> {
    let mut data = data_iter.sorted_by_key(|&(x, _y)| OrderedFloat(x))
        .collect::<Vec<_>>();
    let ranges = box_iter.tuple_windows()
        .map(|(start, end)| start..end);
    let plot_boxes = group_data_by_bounds(&mut data, ranges)
        .collect::<Vec<_>>();

    let root = BitMapBackend::new(path, (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let bounding_box = plot_boxes.iter()
        .flat_map(|b| [(b.range.start, b.min), (b.range.end, b.max)])
        .collect::<BoundingBox>() * margin;

    let mut chart = ChartBuilder::on(&root)
        .caption(caption, ("sans-serif", 32).into_font())
        .margin(30)
        .x_label_area_size(50)
        .y_label_area_size(70)
        .build_cartesian_2d(bounding_box.horizontal_range(), bounding_box.vertical_range())?;

    chart.configure_mesh()
        .x_desc(x_desc)
        .y_desc(y_desc)
        .x_label_style(("sans-serif", 18).into_font())
        .y_label_style(("sans-serif", 18).into_font())
        .axis_desc_style(("sans-serif", 24).into_font())
        .max_light_lines(0)
        .draw()?;

    let line_style = BLACK;
    let box_style = BLUE.mix(0.5).filled();

    let rectangles = plot_boxes.iter()
        .flat_map(|b| plot_box_rectangles(b, box_style, line_style));
    let paths = plot_boxes.iter()
        .flat_map(|b| plot_box_paths(b, line_style));

    chart.draw_series(rectangles)?;
    chart.draw_series(paths)?;

    root.present()?;
    Ok(())
}

pub fn plot_log(
    path: &Path,
    caption: &str,
    x_desc: &str,
    y_desc: &str,
    margin: BoundingBox,
    data_iter: impl Iterator<Item = (f64, f64)>,
    box_iter: impl Iterator<Item = f64>,
) -> anyhow::Result<()> {
    let mut data = data_iter.sorted_by_key(|&(x, _y)| OrderedFloat(x))
        .collect::<Vec<_>>();
    let ranges = box_iter.tuple_windows()
        .map(|(start, end)| start..end);
    let plot_boxes = group_data_by_bounds(&mut data, ranges)
        .collect::<Vec<_>>();

    let root = BitMapBackend::new(path, (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let bounding_box = plot_boxes.iter()
        .flat_map(|b| [(b.range.start, b.min), (b.range.end, b.max)])
        .collect::<BoundingBox>() * margin;

    println!("{:?}", bounding_box.horizontal_range());

    let mut chart = ChartBuilder::on(&root)
        .caption(caption, ("sans-serif", 32).into_font())
        .margin(30)
        .x_label_area_size(50)
        .y_label_area_size(70)
        .build_cartesian_2d(bounding_box.horizontal_range().log_scale(), bounding_box.vertical_range().log_scale())?;

    chart.configure_mesh()
        .x_desc(x_desc)
        .y_desc(y_desc)
        .x_label_style(("sans-serif", 18).into_font())
        .y_label_style(("sans-serif", 18).into_font())
        .axis_desc_style(("sans-serif", 24).into_font())
        .max_light_lines(0)
        .draw()?;

    let line_style = BLACK;
    let box_style = BLUE.mix(0.5).filled();

    let rectangles = plot_boxes.iter()
        .flat_map(|b| plot_box_rectangles(b, box_style, line_style));
    let paths = plot_boxes.iter()
        .flat_map(|b| plot_box_paths(b, line_style));

    chart.draw_series(rectangles)?;
    chart.draw_series(paths)?;

    root.present()?;
    Ok(())
}

#[derive(derive_new::new, Debug)]
struct PlotBox {
    range: Range<f64>,
    min: f64,
    max: f64,
    median: f64,
    first_quartile: f64,
    third_quartile: f64,
}

impl PlotBox {
    pub fn from_range_and_data(range: Range<f64>, data: &mut [(f64, f64)]) -> Option<PlotBox> {
        data.sort_by_key(|&(_x, y)| OrderedFloat(y));
        let min = data.first()?.1;
        let max = data.last()?.1;
        let median = data[data.len() / 2].1;
        let first_quartile = data[data.len() / 4].1;
        let third_quartile = data[3 * data.len() / 4].1;
        Some(PlotBox::new(range, min, max, median, first_quartile, third_quartile))
    }
}

fn group_data_by_bounds(data: &mut [(f64, f64)], ranges: impl Iterator<Item = Range<f64>>) -> impl Iterator<Item = PlotBox> {
    let mut ranges = ranges.peekable();
    
    let min = ranges.peek()
        .expect("There must be some bound to group by")
        .start;
    
    let start = data.iter()
        .position(|&(x, _y)| x >= min)
        .unwrap_or(data.len());

    ranges.scan(start, move |prev_j, range| {
        if *prev_j >= data.len() {
            return None;
        }

        let skips = data[*prev_j..].iter()
            .position(|&(x, _y)| x >= range.start)
            .unwrap_or(data.len() - *prev_j);
        
        let i = *prev_j + skips;

        if i >= data.len() {
            return None;
        }

        let len = data[i..].iter()
            .position(|&(x, _y)| x >= range.end)
            .unwrap_or(data.len() - i);
        
        let j = i + len;
        let slice = &mut data[i..j];
        *prev_j = j;
        PlotBox::from_range_and_data(range, slice)
    })
}

fn plot_box_rectangles(plot_box: &PlotBox, box_style: impl Into<ShapeStyle>, line_style: impl Into<ShapeStyle>) -> [Rectangle<(f64, f64)>; 2] {
    [
        Rectangle::new([(plot_box.range.start, plot_box.first_quartile), (plot_box.range.end, plot_box.third_quartile)], box_style),
        Rectangle::new([(plot_box.range.start, plot_box.first_quartile), (plot_box.range.end, plot_box.third_quartile)], line_style),
    ]
}

fn plot_box_paths(plot_box: &PlotBox, line_style: impl Into<ShapeStyle> + Copy) -> [PathElement<(f64, f64)>; 5] {
    let x0 = plot_box.range.start;
    let x1 = plot_box.range.end;
    let xc = (x0 + x1) / 2.0;
    let cap_width = (x1 - x0) * 0.4;
    [
        PathElement::new([(x0, plot_box.median), (x1, plot_box.median)], line_style),
        PathElement::new([(xc, plot_box.min), (xc, plot_box.first_quartile)], line_style),
        PathElement::new([(xc, plot_box.third_quartile), (xc, plot_box.max)],line_style),
        PathElement::new([(xc - cap_width / 2.0, plot_box.min), (xc + cap_width / 2.0, plot_box.min)], line_style),
        PathElement::new([(xc - cap_width / 2.0, plot_box.max), (xc + cap_width / 2.0, plot_box.max)], line_style),
    ]
}