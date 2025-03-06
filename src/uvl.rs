use std::{collections::HashMap, io::Write};

use itertools::Itertools;

use crate::{dependency::{Dependencies, Dependency}, feature_dependencies::Graph};

pub fn to_universal_variability_language<W: Write>(graph: &Graph, writer: &mut W) -> std::io::Result<()> {
    let reference_count = graph.values()
        .flat_map(|d| d.leafs())
        .counts();

    let top_level = graph.keys()
        .filter(|d| !reference_count.contains_key(d))
        .collect::<Vec<_>>();

    writeln!(writer, "features")?;
    for &d in top_level.iter() {
        visit_dependency(graph, d, &reference_count, writer, 1)?;
    }

    for d in graph.keys().filter(|d| *reference_count.get(d).unwrap_or(&0) >= 2) {
        writeln!(writer, "\t{}", d.name())?;
    }

    writeln!(writer, "constraints")?;
    for &d in top_level.iter() {
        visit_constraint(graph, d, &reference_count, writer)?;
    }

    Ok(())
}

fn visit_dependency<W: Write>(graph: &Graph, dependency: &Dependency, reference_count: &HashMap<&Dependency, usize>, writer: &mut W, depth: usize) -> std::io::Result<()> {
    tab(writer, depth)?;
    writeln!(writer, "{}", dependency.name())?;
    
    let empty = Dependencies::empty();
    let children = graph.get(dependency)
        .unwrap_or(&empty)
        .mandatory()
        .filter(|d| *reference_count.get(d).unwrap_or(&0) == 1)
        .collect::<Vec<_>>();

    if children.is_empty() {
        return Ok(());
    }

    tab(writer, depth + 1)?;
    writeln!(writer, "mandatory")?;

    for child in children {
        visit_dependency(graph, child, reference_count, writer, depth + 2)?;
    }

    Ok(())
}

fn visit_constraint<W: Write>(graph: &Graph, dependency: &Dependency, reference_count: &HashMap<&Dependency, usize>, writer: &mut W) -> std::io::Result<()> {
    let empty = Dependencies::empty();
    let children = graph.get(dependency)
        .unwrap_or(&empty)
        .mandatory()
        .filter(|d| *reference_count.get(d).unwrap_or(&0) > 1)
        .collect::<Vec<_>>();

    for child in children {
        writeln!(writer, "\t{} => {}", dependency.name(), child.name())?;
        visit_constraint(graph, child, reference_count, writer)?;
    }

    Ok(())
}

fn tab<W: Write>(writer: &mut W, depth: usize) -> std::io::Result<()> {
    for _ in 0..depth {
        write!(writer, "\t")?;
    }

    Ok(())
}