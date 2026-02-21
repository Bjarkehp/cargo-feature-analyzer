use std::io::Write;

use crate::{feature_model::{CrossTreeConstraint, Feature, FeatureModel, Group}, indent::tab};

pub fn write<W: Write>(writer: &mut W, feature_model: &FeatureModel) -> std::io::Result<()> {
    writeln!(writer, "features")?;
    write_feature(writer, &feature_model.root_feature, 1)?;

    if !feature_model.cross_tree_constraints.is_empty() {
        writeln!(writer, "constraints")?;
        write_constraints(writer, &feature_model.cross_tree_constraints)?;
    }

    Ok(())
}

fn write_feature<W: Write>(writer: &mut W, feature: &Feature, depth: usize) -> std::io::Result<()> {
    tab(writer, depth)?;
    if feature.is_abstract {
        writeln!(writer, "\"{}\" {{abstract}}", feature.name)?;
    } else {
        writeln!(writer, "\"{}\"", feature.name)?;
    }

    for group in &feature.groups {
        write_group(writer, group, depth + 1)?;
    }

    Ok(())
}

fn write_group<W: Write>(writer: &mut W, group: &Group, depth: usize) -> std::io::Result<()> {
    tab(writer, depth)?;
    write_group_cardinality(writer, group)?;

    for feature in &group.features {
        write_feature(writer, feature, depth + 1)?;
    }

    Ok(())
}

fn write_group_cardinality<W: Write>(writer: &mut W, group: &Group) -> std::io::Result<()> {
    let n = group.features.len();

    match (group.min, group.max) {
        (a, b) if a == b && b == n => writeln!(writer, "mandatory")?,
        (0, m) if m == n => writeln!(writer, "optional")?,
        (1, m) if m == n => writeln!(writer, "or")?,
        (1, 1) => writeln!(writer, "alternative")?,
        (a, b) => writeln!(writer, "[{a}..{b}]")?,
    }

    Ok(())
} 

fn write_constraints<W: Write>(writer: &mut W, constraints: &[CrossTreeConstraint]) -> std::io::Result<()> {
    for constraint in constraints {
        write_constraint(writer, constraint)?;
    }

    Ok(())
}

fn write_constraint<W: Write>(writer: &mut W, constraint: &CrossTreeConstraint) -> std::io::Result<()> {
    match constraint {
        CrossTreeConstraint::Implies(l, r) => writeln!(writer, "\t\"{l}\" => \"{r}\""),
        CrossTreeConstraint::Exclusive(l, r) => writeln!(writer, "\t!\"{l}\" | !\"{r}\""),
        CrossTreeConstraint::Not(feature) => writeln!(writer, "\t!\"{feature}\""),
    }
}