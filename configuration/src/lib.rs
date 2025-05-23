use std::{collections::BTreeSet, path::Path};

use thiserror::Error;
use toml_util::get_table;
use walkdir::WalkDir;

pub mod feature_dependencies;

mod toml_util; 

#[derive(Debug)]
pub enum Configuration<'a> {
    Standard {
        name: &'a str,
        features: BTreeSet<&'a str>
    },
    Dev {
        name: &'a str,
        features: BTreeSet<&'a str>
    }
}

impl<'a> Configuration<'a> {
    pub fn new_standard(root: &'a toml::Table, feature: &str, feature_dependencies: &'a feature_dependencies::Map) -> Result<Self> {
        let name = name(root)
            .ok_or(Error::NoName)?;
        let dependency_table = get_table(root, "dependencies")
            .map_err(|_| Error::NoDependencies)?;
        let features = implied_features(dependency_table, feature, feature_dependencies)?;

        Ok(Self::Standard {
            name,
            features
        })
    }

    pub fn new_dev(root: &'a toml::Table, feature: &str, feature_dependencies: &'a feature_dependencies::Map) -> Result<Self> {
        let name = name(root)
            .ok_or(Error::NoName)?;
        let dependency_table = get_table(root, "dependencies")
            .map_err(|_| Error::NoDependencies)?;
        let features = implied_features(dependency_table, feature, feature_dependencies)?;

        Ok(Self::Dev {
            name,
            features
        })
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Standard { name, .. } => name,
            Self::Dev { name, .. } => name
        }
    }
    
    pub fn features(&self) -> &BTreeSet<&str> {
        match self {
            Self::Standard { features, .. } => features,
            Self::Dev { features, .. } => features
        }
    }
}

fn implied_features<'a>(
    dependency_table: &'a toml::Table, 
    feature: &str, 
    feature_dependencies: &'a feature_dependencies::Map
) -> Result<BTreeSet<&'a str>> {
    let features_value = dependency_table.get(feature)
        .and_then(|v| v.as_table())
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
                .ok_or(Error::InvalidFeatureDependencies)?;
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
    #[error("Unable to parse a specific feature in provided Cargo.toml")]
    InvalidFeature,
    #[error("The passed feature dependencies are invalid")]
    InvalidFeatureDependencies,
}