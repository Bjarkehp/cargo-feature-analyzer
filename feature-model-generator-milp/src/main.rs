pub mod util;
pub mod milp;

use std::{error::Error, path::PathBuf};

use clap::Parser;
use configuration::Configuration;
use good_lp::{scip, Solution, SolverModel};
use itertools::Itertools;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    source: PathBuf,
    destination: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
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

    let features = configuration::all_features(&configurations_files[0].1)
        .ok_or(format!("Failed to parse features from {}", configurations_files[0].0))?;

    let configurations = configurations_files.iter()
        .map(|(name, content)| Configuration::from_csvconf(name.clone(), content)
            .ok_or(format!("Failed to parse configuration from {}", name)))
        .collect::<Result<Vec<_>, _>>()?;

    let milp = milp::create_problem(&features, &configurations);
    let objective = milp::create_objective(&milp);
    let constraints = milp::create_constraints(&milp)
        .collect::<Vec<_>>();
    let solution = milp.problem.maximise(objective)
        .using(scip)
        .set_option("display/verblevel", 3)
        .set_time_limit(300)
        .with_all(constraints)
        .solve()
        .expect("Failed to solve MILP");

    for feature_index in 1..milp.columns {
        let group = (0..milp.columns).find(|&g| solution.value(milp.feature_group_relation[&(feature_index, g)]) >= 0.5)
            .expect("Every feature should be in exactly one group");
        let parent_index = (0..milp.columns).find(|&p| solution.value(milp.feature_parent_relation[&(feature_index, p)]) >= 0.5)
            .expect("Every non-root feature should have exactly one parent");
        let feature = features[feature_index];
        let parent = features[parent_index];
        println!("{feature} is in group {group} with parent {parent}");
    }

    for group in 0..milp.columns {
        let cardinality_min = solution.value(milp.cardinality_min[&group]).round() as i32;
        let cardinality_max = solution.value(milp.cardinality_max[&group]).round() as i32;
        let is_used = solution.value(milp.group_not_empty[&group]) >= 0.5;
        let is_mandatory = solution.value(milp.is_mandatory[&group]) >= 0.5;
        let is_alternative = solution.value(milp.is_alternative[&group]) >= 0.5;
        let is_or = solution.value(milp.is_or[&group]) >= 0.5;
        
        if is_used {
            print!("{group}: {cardinality_min}..{cardinality_max} ");

            if is_mandatory {
                println!("Mandatory");
            } else if is_alternative {
                println!("Alternative");
            } else if is_or {
                println!("Or");
            } else {
                println!("Optional");
            }
        }
    }

    Ok(())
}
