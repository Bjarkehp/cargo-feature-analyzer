use std::{collections::HashSet, hash::Hash};

use thiserror::Error;
use toml::Table;

use crate::{dependency::Dependency, directed_graph::DirectedGraph};

pub fn extract<'a>(root: &'a Table, dependency: &'a str, dependency_graph: &'a DirectedGraph<Dependency>) -> Result<HashSet<Dependency<'a>>> {
    let dependencies = get_table(root, "dependencies")?;
    let dependency_node = dependencies.get(dependency)
        .ok_or(Error::KeyMissing)?;

    let feature_node = dependency_node.get("features")
        .and_then(|v| v.as_array());

    let features = if let Some(feature_node) = feature_node {
        feature_node.iter()
            .map(|f| f.as_str().map(|s| Dependency::Feature(s)))
            .collect::<Option<HashSet<_>>>()
            .ok_or(Error::WrongType)?
    } else {
        HashSet::new()
    };

    let mut visited = HashSet::new();

    for feature in features {
        dependency_graph.depth_first_search(feature, &mut visited);
    }

    Ok(visited)
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