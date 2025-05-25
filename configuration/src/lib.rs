use std::{collections::BTreeSet, path::Path};

use derive_new::new;
use thiserror::Error;
use walkdir::WalkDir;

pub mod feature_dependencies;

mod toml_util; 

#[derive(Debug, new)]
pub struct Configuration<'a> {
    name: String,
    features: BTreeSet<&'a str>
}

impl<'a> Configuration<'a> {
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn features(&self) -> &BTreeSet<&str> {
        &self.features
    }

    pub fn from_csvconf(name: String, content: &'a str) -> Self {
        let features = content.lines()
            .filter_map(|l| l.split_once(','))
            .filter(|&(_l, r)| r == "True")
            .map(|(l, _r)| &l[1..l.len() - 1]) // Assume quotation marks
            .collect();
        Configuration::new(name, features)
    }
}

pub fn implied_features<'a>(
    dependency_table: &'a toml::Table, 
    dependency: &str, 
    feature_dependencies: &'a feature_dependencies::Map
) -> Result<BTreeSet<&'a str>> {
    let features_value = dependency_table.get(dependency)
        .ok_or(Error::DependencyNotFound(dependency.to_string()))?
        .as_table()
        .and_then(|t| t.get("features"))
        .and_then(|v| v.as_array());

    let mut features = if let Some(v) = features_value {
        v.iter()
            .map(|v| v.as_str())
            .collect::<Option<Vec<_>>>()
            .ok_or(Error::InvalidFeature)?
    } else {
        return Ok(BTreeSet::new());
    };  

    let mut visited_features = BTreeSet::new();
    while let Some(feature) = features.pop() {
        if !visited_features.contains(feature) {
            visited_features.insert(feature);
            let new_features = feature_dependencies.get(feature)
                .ok_or(Error::InvalidFeatureDependencies(feature.to_string()))?
                .iter()
                .filter(|&f| feature_dependencies.contains_key(f));
            features.extend(new_features);
        }
    }

    Ok(visited_features)
}

pub fn name(root: &toml::Table) -> Option<&str> {
    root.get("package")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
}

pub fn load_tables(path: impl AsRef<Path>) -> Vec<toml::Table> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|result| result.ok())
        .filter(|entry| entry.file_name().to_str().unwrap().ends_with(".toml"))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|content| content.as_str().parse().ok())
        .collect::<Vec<_>>()
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("The passed Cargo.toml has no name")]
    NoName,
    #[error("The passed Cargo.toml has no dependencies")]
    NoDependencies,
    #[error("The passed Cargo.toml has no dev-dependencies")]
    NoDevDependencies,
    #[error("{0} is not in dependency table")]
    DependencyNotFound(String),
    #[error("Unable to parse a specific feature in provided Cargo.toml")]
    InvalidFeature,
    #[error("The passed feature dependencies doesn't contain {0}")]
    InvalidFeatureDependencies(String),
}