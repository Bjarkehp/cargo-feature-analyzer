use std::{collections::HashSet, hash::Hash};

use thiserror::Error;
use toml::Table;

use crate::{dependency::Dependency, directed_graph::DirectedGraph};

pub fn extract_all<'a>(root: &'a Table, dependency: &str, dependency_graph: &'a DirectedGraph<Dependency>) -> Result<HashSet<Dependency<'a>>> {
    let features = extract(root, dependency)?;
    let mut visited = HashSet::new();

    for feature in features {
        dependency_graph.depth_first_search(feature, &mut visited);
    }

    Ok(visited)
}

pub fn extract_features<'a>(root: &'a Table, dependency: &str) -> Result<Vec<Dependency<'a>>> {
    extract(get_table(root, "dependencies")?, dependency)
}

pub fn extract_dev_features<'a>(root: &'a Table, dependency: &str) -> Result<Vec<Dependency<'a>>> {
    extract(get_table(root, "dev-dependencies")?, dependency)
}

fn extract<'a>(table: &'a Table, dependency: &str) -> Result<Vec<Dependency<'a>>> {
    let dependency_node = table.get(dependency)
        .ok_or(Error::KeyMissing)?;

    let feature_node = dependency_node.get("features")
        .and_then(|v| v.as_array());

    let features = if let Some(feature_node) = feature_node {
        feature_node.iter()
            .map(|f| f.as_str().map(Dependency::Feature))
            .collect::<Option<Vec<_>>>()
            .ok_or(Error::WrongType)?
    } else {
        Vec::new()
    };

    Ok(features)
}

fn get_table<'a>(parent: &'a Table, key: &str) -> Result<&'a Table> {
    parent.get(key)
        .ok_or(Error::KeyMissing)?
        .as_table()
        .ok_or(Error::WrongType)
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Missing key in toml")]
    KeyMissing,
    #[error("Wrong type in toml")]
    WrongType
}

pub type Result<T> = std::result::Result<T, Error>;