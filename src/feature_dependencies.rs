use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use toml::{Table, Value};

use crate::dependency::Dependency;   

type Graph = BTreeMap<String, Dependency>;

pub fn from_cargo_toml(content: &str) -> Result<Graph> {
    let root = content.parse::<toml::Table>()
        .map_err(Error::Deserialization)?;

    let mut graph = Graph::new();
    feature_dependencies(&mut graph, &root)?;
    optional_dependency_features(&mut graph, &root)?;

    Ok(graph)
}

fn feature_dependencies(graph: &mut Graph, root: &Table) -> Result<()> {
    let features = get_table(root, "features")?;

    for (key, value) in features {
        let feature = key.to_string();
        let dependency = dependency_from_feature_value(value)?;
        graph.insert(feature, dependency);
    }

    Ok(())
}

fn optional_dependency_features(graph: &mut Graph, root: &Table) -> Result<()> {
    let features = get_table(root, "features")?;
    let dependencies = get_dependency_tables(root)?
        .into_iter()
        .flat_map(|table| table.into_iter());

    let feature_dependency_set = features.values()
        .map(dependency_from_feature_value)
        .collect::<Result<Vec<Dependency>>>()?
        .into_iter()
        .flat_map(|d| d.crates().map(|s| s.to_string()).collect::<Vec<_>>())
        .collect::<BTreeSet<String>>();

    for (key, value) in dependencies {
        let feature = key.to_string();
        let optional = value.as_table()
            .and_then(|t| t.get("optional"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        if optional && !feature_dependency_set.contains(&feature) {
            graph.insert(feature.clone(), Dependency::Crate(feature));
        }
    }

    Ok(())
}

fn get_table<'a>(parent: &'a Table, key: &str) -> Result<&'a Table> {
    parent.get(key)
        .ok_or(Error::KeyMissing)?
        .as_table()
        .ok_or(Error::WrongType)
}

fn get_dependency_tables(root: &Table) -> Result<Vec<&Table>> {
    let default_table = get_table(root, "dependencies")?;
    let target_table = root.get("target")
        .and_then(|v| v.as_table());

    if let Some(t) = target_table {
        let mut tables = Vec::new();
        let target_tables = t.values()
            .filter_map(|v| v.as_table())
            .filter_map(|t| get_table(t, "dependencies").ok())
            .collect::<Vec<_>>();

        tables.push(default_table);
        tables.extend(target_tables);
        Ok(tables)
    } else {
        Ok(vec![default_table])
    }
}

fn dependency_from_feature_value(value: &Value) -> Result<Dependency> {
    let dependencies = value.as_array()
        .ok_or(Error::WrongType)?
        .iter()
        .map(|d| d.as_str().map(feature_or_dependency))
        .collect::<Option<Vec<Dependency>>>()
        .ok_or(Error::WrongType)?;

    if dependencies.is_empty() {
        Ok(Dependency::None)
    } else {
        Ok(Dependency::And(dependencies))
    }
}

fn feature_or_dependency(s: &str) -> Dependency {
    if let Some(stripped) = s.strip_prefix("dep:") {
        Dependency::Crate(stripped.to_string())
    } else if let Some((left, _)) = s.split_once('/') {
        // Example: "mio/os-poll" depends both on the crate mio, 
        // *and* the feature "os-poll" inside mio
        Dependency::Crate(left.to_string())
            .and(Dependency::Feature(s.to_string()))
    } else {
        Dependency::Feature(s.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Deserialization(#[from] toml::de::Error),
    #[error("Missing key in toml")]
    KeyMissing,
    #[error("Wrong type in toml")]
    WrongType
}