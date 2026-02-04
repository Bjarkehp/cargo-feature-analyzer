use std::{fs::File, io::{BufReader, BufWriter, Write}, path::PathBuf};

use anyhow::Context;
use cargo_toml::crate_id::CrateId;
use configuration_scraper::configuration::Configuration;
use fm_synthesizer_fca::{concept, tree_constraints, uvl};
use ordered_float::OrderedFloat;
use petgraph::Direction;
use rand::{SeedableRng, rngs::StdRng};

use crate::{flamapy_client, paths};

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

pub fn create_fca<'a>(id: &CrateId, configurations: &[Configuration<'a>]) -> anyhow::Result<()> {
    let path = PathBuf::from(format!("{}/{}.uvl", paths::FCA_MODEL, id));
    if let Ok(file) = File::create_new(&path) {
        let train_configurations = &configurations[..configurations.len()];
        let mut features = train_configurations.first()
            .expect("Crates are filtered above for number of configs")
            .features.keys()
            .map(|k| k.as_ref())
            .collect::<Vec<_>>();
        features.push(&id.name);

        let ac_poset = concept::ac_poset(train_configurations, &features, &id.name);
        let mut writer = BufWriter::new(file);
        let tree_constraints = tree_constraints::max_depth::find(&ac_poset);
        uvl::write_ac_poset(&mut writer, &ac_poset, &features, &tree_constraints)
            .with_context(|| format!("Failed to write fca feature model to {path:?}"))?;
        writer.flush()
            .with_context(|| format!("Failed to flush file {path:?}"))?;
    }

    Ok(())
}

pub fn create_fca_rng<'a>(id: &CrateId, configurations: &[Configuration<'a>], client: &mut flamapy_client::Client) -> anyhow::Result<()> {
    let path = PathBuf::from(format!("{}/{}.uvl", paths::FCA_MODEL, id));
    if let Ok(file) = File::create_new(&path) {
        let train_configurations = &configurations[..configurations.len() / 10];
        let mut features = train_configurations.first()
            .expect("Crates are filtered above for number of configs")
            .features.keys()
            .map(|k| k.as_ref())
            .collect::<Vec<_>>();
        features.push(&id.name);

        let ac_poset = concept::ac_poset(train_configurations, &features, &id.name);
        let maximal = ac_poset.externals(Direction::Outgoing)
            .next()
            .expect("ac_poset isn't empty");
        
        let mut rng = StdRng::seed_from_u64(123);
        let models = (0..10).map(|i| {
            let tree_constraints = tree_constraints::rng::find(&ac_poset, maximal, &mut rng);
            //let temp_file = NamedTempFile::new()?;
            let path = PathBuf::from(format!("temp_model/{id}/{i}.uvl"));
            std::fs::create_dir_all(format!("temp_model/{id}"))?;
            let temp_file = File::create_new(&path)?;
            //let path = temp_file.path();

            {
                let mut writer = BufWriter::new(&temp_file);
                uvl::write_ac_poset(&mut writer, &ac_poset, &features, &tree_constraints)
                    .with_context(|| format!("Failed to write fca feature model to {path:?}"))?;
                writer.flush()
                    .with_context(|| format!("Failed to flush file {path:?}"))?;
            }

            client.set_model(&path)?;
            let config_number = client.estimated_number_of_configurations()?;

            Ok((path, config_number))
        }).collect::<anyhow::Result<Vec<_>>>()?;

        let best_model = models.iter()
            .min_by_key(|(_file, config_number)| OrderedFloat(*config_number))
            .map(|(file, _config_number)| file)
            .expect("The number of models is always above 0");
        
        let best_model_file = File::open(best_model)?;
        let mut best_model_reader = BufReader::new(best_model_file);
        let mut writer = BufWriter::new(file);

        std::io::copy(&mut best_model_reader, &mut writer)?;

        writer.flush()?;
    }

    Ok(())
}