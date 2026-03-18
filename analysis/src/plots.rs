use std::{collections::BTreeMap, path::Path};

use cargo_toml::crate_id::CrateId;
use plotters::{chart::ChartBuilder, data::{Quartiles, fitting_range}, prelude::{Boxplot, Circle, IntoSegmentedCoord, SegmentValue}, style::{BLACK, Color, IntoFont, WHITE}};
use sorted_iter::SortedPairIterator;
use tokei::Language;

use crate::{ModelStats, bounding_box::BoundingBox, plot::{default_chart, default_log_chart, default_mesh, default_root, draw_linear_regression, draw_linear_regression_log, draw_points}};

pub fn features_and_dependencies(dir: &Path, feature_stats: &BTreeMap<&CrateId, (usize, usize)>, max_features: usize, max_dependencies: usize) -> anyhow::Result<()> {
    let caption = "Features and feature dependencies";
    let x_desc = "Features";
    let y_desc = "Feature dependencies";
    let file_name = "features_and_dependencies.png";
    let path = dir.join(file_name);
    
    let points = feature_stats.iter()
        .map(|(_id, &(f, d))| (f as f64, d as f64))
        .filter(|&(f, d)| f < max_features as f64 && d < max_dependencies as f64)
        .collect::<Vec<_>>();

    let bounding_box = points.iter()
        .cloned()
        .collect::<BoundingBox>();
    let x_range = bounding_box.horizontal_range();
    let y_range = bounding_box.vertical_range();

    let root = default_root(&path)?;
    let mut chart = default_chart(&root, caption, x_range.clone(), y_range)?;
    default_mesh(&mut chart, x_desc, y_desc).draw()?;
    draw_points(&mut chart, &points)?;
    draw_linear_regression(&mut chart, &points, x_range)?;
    root.present()?;

    Ok(())
}

pub fn line_count_and_features(dir: &Path, line_counts: &BTreeMap<&CrateId, Language>, feature_stats: &BTreeMap<&CrateId, (usize, usize)>) -> anyhow::Result<()> {
    let caption = "Line count and features";
    let x_desc = "Line count";
    let y_desc = "Features";
    let file_name = "line_count_and_features.png";
    let path = dir.join(file_name);

    let points = line_counts.iter()
        .join(feature_stats.iter())
        .map(|(_, (language, &(features, _)))| (language.code as f64, features as f64))
        .collect::<Vec<_>>();
    
    let bounding_box = points.iter()
        .cloned()
        .collect::<BoundingBox>();
    let x_range = bounding_box.horizontal_range();
    let y_range = bounding_box.vertical_range();

    let root = default_root(&path)?;
    let mut chart = default_log_chart(&root, caption, x_range.clone(), y_range)?;
    default_mesh(&mut chart, x_desc, y_desc)
        .x_label_formatter(&log_label_formatter)
        .y_label_formatter(&log_label_formatter)
        .draw()?;
    draw_points(&mut chart, &points)?;
    draw_linear_regression_log(&mut chart, &points, x_range)?;
    root.present()?;

    Ok(())
}

pub fn flat_vs_fca_exact(dir: &Path, flat: &BTreeMap<&CrateId, ModelStats>, fca: &BTreeMap<&CrateId, ModelStats>) -> anyhow::Result<()> {
    let caption = "Configuration number (Flat & FCA)";
    let x_desc = "Configuration number (Flat)";
    let y_desc = "Configuration number (FCA)";
    let file_name = "flat_vs_fca_exact.png";
    let path = dir.join(file_name);
    
    let points = flat.iter()
        .join(fca.iter())
        .map(|(_, (flat, fca))| (flat.config_exact, fca.config_exact))
        .collect::<Vec<_>>();

    let bounding_box = points.iter()
        .cloned()
        .collect::<BoundingBox>();
    let x_range = bounding_box.horizontal_range();
    let y_range = bounding_box.vertical_range();

    let root = default_root(&path)?;
    let mut chart = default_log_chart(&root, caption, x_range.clone(), y_range)?;
    default_mesh(&mut chart, x_desc, y_desc)
        .x_label_formatter(&log_label_formatter)
        .y_label_formatter(&log_label_formatter)
        .draw()?;
    draw_points(&mut chart, &points)?;
    root.present()?;

    Ok(())
}

pub fn cross_tree_constraints(dir: &Path, flat: &BTreeMap<&CrateId, ModelStats>, fca: &BTreeMap<&CrateId, ModelStats>) -> anyhow::Result<()> {
    let caption = "Cross-tree constraints (Flat & FCA)";
    let x_desc = "Cross-tree constraints (Flat)";
    let y_desc = "Cross-tree constraints (FCA)";
    let file_name = "cross_tree_constraints_comparison.png";
    let path = dir.join(file_name);
    
    let points = flat.iter()
        .join(fca.iter())
        .map(|(_, (flat, fca))| (flat.cross_tree_constraints as f64, fca.cross_tree_constraints as f64))
        .collect::<Vec<_>>();

    let bounding_box = points.iter()
        .cloned()
        .collect::<BoundingBox>();
    let x_range = bounding_box.horizontal_range();
    let y_range = bounding_box.vertical_range();

    let root = default_root(&path)?;
    let mut chart = default_chart(&root, caption, x_range.clone(), y_range)?;
    default_mesh(&mut chart, x_desc, y_desc).draw()?;
    draw_points(&mut chart, &points)?;
    root.present()?;

    Ok(())
}

pub fn box_plots(dir: &Path, feature_stats: &BTreeMap<&CrateId, (usize, usize)>) -> anyhow::Result<()> {
    let caption = "Global stats";
    let file_name = "box_plots.png";
    let x_axis = ["Features", "Feature Dependencies"];
    let path = dir.join(file_name);

    let (feature_counts, feature_dependency_counts): (Vec<_>, Vec<_>) = feature_stats
        .values()
        .map(|(f, d)| (*f as f32, *d as f32))
        .unzip();
    
    let y_range = fitting_range(
        feature_counts.iter()
            .chain(feature_dependency_counts.iter())
    );

    println!("{:#?}", feature_counts);
    println!("{:#?}", feature_dependency_counts);

    let root = default_root(&path)?;
    
    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption(caption, ("sans-serif", 32).into_font())
        .margin(30)
        .build_cartesian_2d(x_axis.into_segmented(), y_range.start - 10.0..y_range.end + 10.0)?;
    
    chart.configure_mesh()
        .x_label_style(("sans-serif", 18).into_font())
        .y_label_style(("sans-serif", 18).into_font())
        .light_line_style(WHITE)
        .draw()?;

    let a_quartiles = Quartiles::new(&feature_counts);
    let b_quartiles = Quartiles::new(&feature_dependency_counts);

    chart.draw_series(vec![
        Boxplot::new_vertical(SegmentValue::CenterOf(&"Features"), &a_quartiles),
        Boxplot::new_vertical(SegmentValue::CenterOf(&"Feature Dependencies"), &b_quartiles),
    ])?;

    Ok(())
}

fn log_label_formatter(x: &f64) -> String {
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