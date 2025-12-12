pub mod util;
pub mod milp;

use std::{fs::File, io::BufWriter, path::PathBuf};
use anyhow::{Context, anyhow};
use cargo_toml::crate_id::CrateId;
use configuration_scraper::configuration::Configuration;
use clap::Parser;
use good_lp::{scip, Solution, SolverModel};
use itertools::Itertools;
use petgraph::{graph::DiGraph, visit::EdgeRef, Direction};
use walkdir::WalkDir;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    source: PathBuf,
    destination: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let configurations_files = WalkDir::new(args.source)
        .into_iter()
        .filter_map(|result| result.ok())
        .filter(|e| e.file_name().to_str().unwrap().ends_with(".csvconf"))
        .filter_map(|e| Some((
            e.file_name().to_str()?.to_string(), 
            std::fs::read_to_string(e.path()).ok()?
        )))
        .sorted()
        .collect::<Vec<_>>();

    let configurations = configurations_files.iter()
        .map(|(name, content)| {
            let crate_id: CrateId = name.parse()
                .with_context(|| format!("Failed to parse {name} to crate id"))?;
            let configuration = Configuration::from_csv_owned(crate_id.name, crate_id.version, content)
                .ok_or_else(|| anyhow!("Failed to parse file content as csv"))?;
            Ok(configuration)
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    let features = configurations[0].features.keys()
        .filter(|&f| configurations.iter().any(|c| c.features.get(f) == Some(&true)))
        .map(|f| f.as_ref())
        .collect::<Vec<_>>();

    let milp = milp::create_problem(&features, &configurations);
    let objective = milp::create_objective(&milp);
    let constraints = milp::create_constraints(&milp)
        .collect::<Vec<_>>();
    let solution = milp.problem.maximise(objective)
        .using(scip)
        .set_option("display/verblevel", 3)
        .set_time_limit(3600)
        .with_all(constraints)
        .solve()
        .expect("Failed to solve MILP");

    let mut graph = DiGraph::new();
    let feature_vertices = features.iter()
        .map(|f| graph.add_node(Vertex::Feature(f)))
        .collect::<Vec<_>>();
    let group_vertices = (0..features.len())
        .map(|g| graph.add_node(Vertex::Group { 
            min: solution.value(milp.cardinality_min[&g]).round() as usize, 
            max: solution.value(milp.cardinality_max[&g]).round() as usize, 
            size: solution.value(milp.group_size[&g]).round() as usize   
        }))
        .collect::<Vec<_>>();

    milp.feature_group_relation.iter()
        .filter(|&(_key, &variable)| solution.value(variable) >= 0.5)
        .for_each(|(&(feature, group), _variable)| {
            graph.add_edge(group_vertices[group], feature_vertices[feature], ());
        });

    milp.group_parent_relation.iter()
        .filter(|&(_key, &variable)| solution.value(variable) >= 0.5)
        .for_each(|(&(group, parent), _variable)| {
            graph.add_edge(feature_vertices[parent], group_vertices[group], ());
        });

    let file = File::create(args.destination)?;
    let mut writer = BufWriter::new(file);
    let mut stack = vec![(feature_vertices[0], 1)];

    writeln!(writer, "features")?;

    while let Some((vertex_id, depth)) = stack.pop() {
        if !matches!(graph[vertex_id], Vertex::Group { min: 0, max: 0, size: 0 }) {
            write!(writer, "{}", "    ".repeat(depth))?;
        }
        match graph[vertex_id] {
            Vertex::Feature(name) => writeln!(writer, "\"{}\"", name)?,
            Vertex::Group { min: 0, max: 0, size: 0 } => {},
            Vertex::Group { min, max, size } if min == max && max == size => writeln!(writer, "mandatory")?,
            Vertex::Group { min: 1, max: 1, .. } => writeln!(writer, "alternative")?,
            Vertex::Group { min: 1, max, size } if max == size => writeln!(writer, "or")?,
            Vertex::Group { min: 0, max, size } if max == size => writeln!(writer, "optional")?,
            Vertex::Group { min, max, .. } => writeln!(writer, "[{}..{}]", min, max)?,
        };

        for edge in graph.edges_directed(vertex_id, Direction::Outgoing) {
            stack.push((edge.target(), depth + 1));
        }
    }

    Ok(())
}

enum Vertex<'a> {
    Feature(&'a str),
    Group {
        min: usize,
        max: usize,
        size: usize,
    }
}