use std::{fs::File, io::{BufWriter, Write}, path::PathBuf};

use anyhow::Context;
use cargo_toml::crate_id::CrateId;
use configuration_scraper::configuration::Configuration;
use feature_model::{FeatureModel, uvl};
use fm_synthesizer_fca::{concept, synthesizer, tree_constraints};

use crate::paths;

/// Create a flat feature model for a crate with the given crate id and Cargo.toml content.
pub fn create_flat(id: &CrateId, table: &toml::Table) -> anyhow::Result<()> {
    let path = PathBuf::from(format!("{}/{}.uvl", paths::FLAT_MODEL, id));
    if let Ok(file) = File::create_new(&path) {
        let feature_model = fm_synthesizer_flat::fm_from_cargo_toml(table)
            .with_context(|| format!("Failed to create flat constraints from {path:?}"))?;
        let mut writer = BufWriter::new(file);
        uvl::write(&mut writer, &feature_model)
            .with_context(|| format!("Failed to write flat feature model to {path:?}"))?;
        writer.flush()
            .with_context(|| format!("Failed to flush file {path:?}"))?;
    }

    Ok(())
}

/// Create an FCA feature model for a crate with the given crate id and set of configurations.
pub fn create_fca<'a>(id: &CrateId, configurations: &[Configuration<'a>]) -> anyhow::Result<FeatureModel> {
    let path = PathBuf::from(format!("{}/{}.uvl", paths::FCA_MODEL, id));
    let file = File::create(&path)?;
    let train_configurations = &configurations[..configurations.len() / 10];
    let mut features = train_configurations.first()
        .expect("Crates are filtered above for number of configs")
        .features.keys()
        .map(|k| k.as_ref())
        .collect::<Vec<_>>();
    features.push(&id.name);

    let ac_poset = concept::ac_poset(train_configurations, &features, &id.name);
    let tree_constraints = tree_constraints::max_depth::find(&ac_poset);
    let feature_model = synthesizer::fm_from_ac_poset(&ac_poset, &features, &tree_constraints);
    let mut writer = BufWriter::new(file);
    uvl::write(&mut writer, &feature_model)
        .with_context(|| format!("Failed to write fca feature model to {path:?}"))?;
    writer.flush()
        .with_context(|| format!("Failed to flush file {path:?}"))?;

    Ok(feature_model)
}