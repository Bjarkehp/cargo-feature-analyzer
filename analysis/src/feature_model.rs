use std::{fs::File, io::{BufWriter, Write}, path::PathBuf};

use anyhow::Context;
use cargo_toml::crate_id::CrateId;
use configuration_scraper::configuration::Configuration;
use fm_synthesizer_fca::{concept, feature_model::{self, FeatureModel}, tree_constraints, uvl, uvl_writer};

use crate::paths;

pub fn create_flat(id: &CrateId, table: &toml::Table) -> anyhow::Result<()> {
    let path = PathBuf::from(format!("{}/{}.uvl", paths::FLAT_MODEL, id));
    if let Ok(file) = File::create_new(&path) {
        let constraints = fm_synthesizer_flat::from_cargo_toml(table)
            .with_context(|| format!("Failed to create flat constraints from {path:?}"))?;
        let mut writer = BufWriter::new(file);
        fm_synthesizer_flat::write_uvl(&mut writer, &id.name, &constraints)
            .with_context(|| format!("Failed to write flat feature model to {path:?}"))?;
        writer.flush()
            .with_context(|| format!("Failed to flush file {path:?}"))?;
    }

    Ok(())
}

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
    let feature_model = feature_model::from_ac_poset(&ac_poset, &features, &tree_constraints);
    let mut writer = BufWriter::new(file);
    uvl_writer::write(&mut writer, &feature_model)
        .with_context(|| format!("Failed to write fca feature model to {path:?}"))?;
    writer.flush()
        .with_context(|| format!("Failed to flush file {path:?}"))?;

    Ok(feature_model)
}