use std::{error::Error, fs::{self, File}, io::{stdin, BufWriter, Write}, path::{Path, PathBuf}};

use cargo_toml::crate_id::{self, CrateId};
use clap::Parser;
use concept::Concept;
use configuration_scraper::configuration::Configuration;
use fm_synthesizer_fca::{concept, tree_constraints, uvl};
use itertools::Itertools;
use petgraph::{dot::Dot, graph::DiGraph};
use walkdir::WalkDir;

/// Generates an ac-poset from a set of configurations at a specified directory.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    root: String,
    source: PathBuf,
    destination: PathBuf,

    #[arg(short, long, default_value_t = false)]
    force: bool,
    #[arg(short, long, default_value = None)]
    ac_poset: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if !args.force && fs::exists(&args.destination)? && !confirm_overwrite(&args.destination) {
        return Err("User declined operation.".into());
    }

    if let Some(path) = &args.ac_poset {
        if !args.force && fs::exists(path)? && !confirm_overwrite(path) {
            return Err("User declined operation.".into());
        }
    }

    let configurations_files = WalkDir::new(args.source)
        .into_iter()
        .filter_map(|result| result.ok())
        .filter(|e| e.file_name().to_str().unwrap().ends_with(".csvconf"))
        .filter_map(|e| Some((
            e.file_name().to_str()?.trim_end_matches(".csvconf").to_string(), 
            fs::read_to_string(e.path()).ok()?
        )))
        .sorted()
        .collect::<Vec<_>>();

    let configurations = configurations_files.iter()
        .map(|(name, content)| Ok((name.parse::<CrateId>()?, content)))
        .map_ok(|(id, content)| Configuration::from_csv(id.name.to_string(), id.version.clone(), content)
            .ok_or(format!("Failed to parse configuration from {}", id)))
        .collect::<Result<Result<Vec<_>, _>, crate_id::Error>>()??;

    let mut features = configurations.first()
        .ok_or("Crate has no features")?
        .features.keys()
        .map(|k| k.as_ref())
        .collect::<Vec<_>>();
    features.push(&args.root);

    let ac_poset = concept::ac_poset(&configurations, &features, &args.root);

    let uvl_file = File::create(args.destination)?;
    let mut uvl_writer = BufWriter::new(uvl_file);
    let tree_constraints = tree_constraints::max_depth::find(&ac_poset);
    uvl::write_ac_poset(&mut uvl_writer, &ac_poset, &features, &tree_constraints)?;
    uvl_writer.flush()?;

    if let Some(path) = &args.ac_poset {
        write_ac_poset(&ac_poset, path)?;
    }

    Ok(())
}

fn confirm_overwrite(path: impl AsRef<Path>) -> bool {
    println!("Are you sure you want to overwrite {}? [Y/n] ", path.as_ref().display());

    loop {
        let mut buffer = String::new();
        if stdin().read_line(&mut buffer).is_ok() {
            match buffer.trim() {
                "" => return true,
                "y" | "Y" => return true,
                "n" | "N" => return false,
                _ => continue
            }
        } else {
            return false
        }
    }
}

fn write_ac_poset(ac_poset: &DiGraph<Concept, ()>, destination: impl AsRef<Path>) -> std::io::Result<()> {
    let graphviz_config = [
        petgraph::dot::Config::EdgeNoLabel,
        petgraph::dot::Config::RankDir(petgraph::dot::RankDir::BT)
    ];

    let graphviz = Dot::with_attr_getters(
        &ac_poset, 
        &graphviz_config, 
        &|_, _edge| "".to_string(), 
        &|_, _node| "shape=box".to_string()
    );

    fs::write(destination, format!("{:#?}", graphviz))?;

    Ok(())
}