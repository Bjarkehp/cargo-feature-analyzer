use std::{collections::BTreeSet, path::Path};

use derive_new::new;
use petgraph::Direction;
use thiserror::Error;
use walkdir::WalkDir;

pub mod feature_dependencies;

pub mod toml_util; 

/// Stores the name of a configuration and a set of its enabled features.
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

    /// Create a new configuration from the contents of a .csvconf file.
    pub fn from_csvconf(name: String, content: &'a str) -> Option<Self> {
        let features = content.lines()
            .map(|l| l.split_once(','))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .filter(|&(_l, r)| r == "True")
            .map(|(l, _r)| l.trim_matches('"'))
            .collect();
        Some(Configuration::new(name, features))
    }
}

/// Return a list of all features in a .csvconf file.
pub fn all_features(content: &str) -> Option<Vec<&str>> {
    let features = content.lines()
        .map(|l| l.split_once(','))
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .map(|(l, _r)| l.trim_matches('"'))
        .collect();
    Some(features)
}

/// Return the full set of features enabled by a dependency using its toml value.
/// The features listed in the dependency's 'features' field are included.
/// 
/// The features field does not list all features enabled by the dependency.
/// There are default features that are enabled by default,
/// and features might also enable other features, if they depend on them.
/// Therefore, it is necessary to recursively resolve the full set of features enabled by the dependency.
pub fn implied_features_from_table<'a>(
    dependency_table: &'a toml::Table, 
    dependency: &str, 
    feature_dependencies: &'a feature_dependencies::Graph
) -> Result<BTreeSet<&'a str>> {
    let default_features = dependency_table.get(dependency)
        .ok_or(Error::DependencyNotFound(dependency.to_string()))?
        .as_table()
        .and_then(|t| t.get("default-features"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

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
        Vec::new()
    };  

    if default_features {
        features.push("default");
    }

    implied_features(features.into_iter(), feature_dependencies)
}

pub fn implied_features<'a>(
    explicit_features: impl Iterator<Item = &'a str>,
    feature_dependencies: &'a feature_dependencies::Graph
) -> Result<BTreeSet<&'a str>> {
    let mut stack = explicit_features.collect::<Vec<_>>();
    let mut visited_features = BTreeSet::new();
    while let Some(feature) = stack.pop() {
        if !visited_features.contains(feature) {
            visited_features.insert(feature);
            let new_features = feature_dependencies.neighbors_directed(feature, Direction::Outgoing)
                .filter(|&f| feature_dependencies.contains_node(f));
            stack.extend(new_features);
        }
    }

    Ok(visited_features)
}

/// Return the name of a Cargo.toml
pub fn name(root: &toml::Table) -> Option<&str> {
    root.get("package")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
}

/// Return all Cargo.toml files in a directory
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