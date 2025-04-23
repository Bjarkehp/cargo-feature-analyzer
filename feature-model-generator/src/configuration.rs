use std::{collections::HashSet, path::Path};

use derive_new::new;
use thiserror::Error;
use toml::Table;
use walkdir::WalkDir;

use crate::{dependency::Dependency, directed_graph::DirectedGraph};

#[derive(Debug, new)]
pub struct Configuration<'a> {
    name: &'a str,
    features: Vec<Dependency<'a>>
}

impl<'a> Configuration<'a> {
    pub fn name(&self) -> &str {
        self.name
    }
    
    pub fn features(&self) -> &[Dependency<'a>] {
        &self.features
    }
}

pub fn load_tables(path: impl AsRef<Path>) -> Vec<Table> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_name().to_str().unwrap().ends_with(".toml"))
        .filter_map(|file| std::fs::read_to_string(file.path()).ok())
        .filter_map(|toml| toml.as_str().parse().ok())
        .collect::<Vec<_>>()
}

pub fn from<'a>(table: &'a Table, feature: &str) -> Option<Configuration<'a>> {
    let name = name(table)?;
    let features = extract_features(table, feature).ok()?;
    Some(Configuration { name, features })
}

pub fn name(table: &Table) -> Option<&str> {
    table.get("package")?
        .as_table()?
        .get("name")?
        .as_str()
}

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