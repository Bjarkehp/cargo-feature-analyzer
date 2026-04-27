use std::{fs::File, io::{BufWriter, Write}, time::Instant};

use analysis::result::{running_time_fca::RunningTimeFca, running_time_static::RunningTimeStatic};
use anyhow::Context;
use cargo_toml::crate_id::CrateId;
use configuration_scraper::configuration::Configuration;
use feature_model::{FeatureModel, uvl};
use fm_synthesizer_fca::{concept, synthesizer, tree_constraints};

use crate::paths::Paths;

/// Create a declared feature model for a crate with the given crate id and Cargo.toml content.
pub fn create_static(id: &CrateId, table: &toml::Table, paths: &Paths) -> anyhow::Result<(FeatureModel, RunningTimeStatic)> {
    let start = Instant::now();
    let feature_model = fm_synthesizer_flat::fm_from_cargo_toml(table)
        .with_context(|| format!("Failed to create flat constraints for {id}"))?;
    let elapsed = start.elapsed().as_secs_f32();
    let running_time = RunningTimeStatic::new(id.clone(), elapsed);

    let path = paths.static_model.join(format!("{id}.uvl"));
    let file = File::create(&path)?;
    let mut writer = BufWriter::new(file);
    uvl::write(&mut writer, &feature_model)
        .with_context(|| format!("Failed to write flat feature model to {path:?}"))?;
    writer.flush()
        .with_context(|| format!("Failed to flush file {path:?}"))?;

    Ok((feature_model, running_time))
}

/// Create an FCA feature model for a crate with the given crate id and set of configurations.
pub fn create_fca<'a>(id: &CrateId, configurations: &[Configuration<'a>], paths: &Paths) -> anyhow::Result<(FeatureModel, RunningTimeFca)> {
    let path = paths.fca_model.join(format!("{id}.uvl"));
    let file = File::create(&path)?;
    let train_configurations = &configurations[..configurations.len() / 10];
    let mut features = train_configurations.first()
        .expect("Crates are filtered above for number of configs")
        .features.keys()
        .map(|k| k.as_ref())
        .collect::<Vec<_>>();
    features.push(&id.name);

    let start = Instant::now();

    let ac_poset = concept::ac_poset(train_configurations, &features, &id.name);
    let ac_poset_time = Instant::now();
    let tree_constraints = tree_constraints::max_depth::find(&ac_poset);
    let tree_constraints_time = Instant::now();
    let feature_model = synthesizer::fm_from_ac_poset(&ac_poset, &features, &tree_constraints);
    let group_time = Instant::now();

    let mut writer = BufWriter::new(file);
    uvl::write(&mut writer, &feature_model)
        .with_context(|| format!("Failed to write fca feature model to {path:?}"))?;
    writer.flush()
        .with_context(|| format!("Failed to flush file {path:?}"))?;

    let running_time = RunningTimeFca::new(
        id.clone(), 
        (ac_poset_time - start).as_secs_f32(),
        (tree_constraints_time - ac_poset_time).as_secs_f32(),
        (group_time - tree_constraints_time).as_secs_f32(),
        (group_time - start).as_secs_f32(),
    );

    Ok((feature_model, running_time))
}