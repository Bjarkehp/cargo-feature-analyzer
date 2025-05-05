use std::collections::HashSet;
use thiserror::Error;
use toml::{Table, Value};

use crate::{dependency::Dependency, directed_graph::DirectedGraph};   

pub fn from_cargo_toml(root: &toml::Table) -> Result<DirectedGraph<Dependency>> {
    let mut graph = DirectedGraph::new();
    feature_dependencies(&mut graph, root)?;
    optional_dependency_features(&mut graph, root)?;

    Ok(graph)
}

fn feature_dependencies<'a>(graph: &mut DirectedGraph<Dependency<'a>>, root: &'a Table) -> Result<()> {
    let feature_table = get_table(root, "features")?;

    for (key, value) in feature_table {
        let feature = Dependency::Feature(key);
        let dependencies = dependencies_from_feature_value(value)?;
        graph.extend(feature, dependencies);
    }

    Ok(())
}

fn optional_dependency_features<'a>(graph: &mut DirectedGraph<Dependency<'a>>, root: &'a Table) -> Result<()> {
    let feature_table = get_table(root, "features")?;
    let dependencies = get_dependency_tables(root)
        .unwrap_or_default()
        .into_iter()
        .flat_map(|table| table.into_iter());

    let feature_dependency_set = feature_table.values()
        .map(dependencies_from_feature_value)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .filter_map(|d| match d {
            Dependency::Crate(s) => Some(s),
            _ => None
        })
        .collect::<HashSet<&str>>();

    for (key, value) in dependencies {
        let key = key.as_str();
        let optional = value.as_table()
            .and_then(|t| t.get("optional"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        if optional && !feature_dependency_set.contains(key) {
            let feature = Dependency::Feature(key);
            let dependency = Dependency::Crate(key);
            graph.insert(feature, dependency);
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

fn dependencies_from_feature_value(value: &Value) -> Result<impl Iterator<Item = Dependency>> {
    let dependencies = value.as_array()
        .ok_or(Error::WrongType)?
        .iter()
        .map(|d| d.as_str().map(parse_dependency))
        .collect::<Option<Vec<Dependency>>>()
        .ok_or(Error::WrongType)?;

    Ok(dependencies.into_iter())
}

fn parse_dependency(s: &str) -> Dependency {
    if let Some(stripped) = s.strip_prefix("dep:") {
        Dependency::Crate(stripped)
    } else {
        Dependency::Feature(s)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Missing key in toml")]
    KeyMissing,
    #[error("Wrong type in toml")]
    WrongType
}