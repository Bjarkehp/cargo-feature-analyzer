use itertools::Itertools;
use petgraph::prelude::DiGraphMap;

use crate::toml_util::{get_table, Error, Result};

pub type Graph<'a> = DiGraphMap<&'a str, ()>;

/// Create a map between features and their dependencies from a toml table.
pub fn from_cargo_toml(root: &toml::Table) -> Result<Graph<'_>> {
    let feature_table = get_table(root, "features")?;
    let dependency_tables = get_dependency_tables(root);

    let mut feature_dependencies = explicit_feature_dependencies(feature_table)?;
    dependency_tables.into_iter()
        .flat_map(optional_dependencies)
        .filter(|d| !feature_dependencies.contains_node(d))
        .collect::<Vec<_>>().into_iter()
        .for_each(|d| { feature_dependencies.add_node(d); });

    Ok(feature_dependencies)
}

/// Find all features and their dependencies that are explicitly listed in the feature table.
fn explicit_feature_dependencies(table: &toml::Table) -> Result<Graph<'_>> {
    let mut map = Graph::new();

    for (key, value) in table {
        let feature = key.as_str();
        let dependencies = value.as_array()
            .ok_or(Error::UnexpectedType(key.to_string(), "array"))?
            .iter()
            .enumerate()
            .map(|(i, d)| d.as_str()
                .map(trim_feature)
                .ok_or(Error::UnexpectedType(format!("{}[{}]", key, i), "str"))
            )
            .filter_ok(|d| !d.ends_with('?'))
            .collect::<Result<Vec<&str>>>()?;

        map.add_node(feature);
        for dependency in dependencies {
            map.add_edge(feature, dependency, ());
        }
    }

    Ok(map)
}

/// Find all dependencies marked as optional
fn optional_dependencies(table: &toml::Table) -> impl Iterator<Item = &str> {
    table.iter()
        .filter(|&(_k, v)| dependency_is_optional(v))
        .map(|(k, _v)| k.as_str())
}

// Determine if a dependency's toml value indicates it is optional.
fn dependency_is_optional(dependency: &toml::Value) -> bool {
    dependency.as_table()
        .and_then(|t| t.get("optional"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Find all tables with dependencies or dev-dependencies
pub fn get_dependency_tables(root: &toml::Table) -> Vec<&toml::Table> {
    let default_table = get_table(root, "dependencies").ok();
    let dev_table = get_table(root, "dev-dependencies").ok();
    let target_table = root.get("target")
        .and_then(|v| v.as_table());

    let target_dependency_tables = if let Some(target) = target_table {
        target.values()
            .filter_map(|v| v.as_table())
            .filter_map(|t| get_table(t, "dependencies").ok())
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    let target_dev_dependency_tables = if let Some(target) = target_table {
        target.values()
            .filter_map(|v| v.as_table())
            .filter_map(|t| get_table(t, "dev-dependencies").ok())
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    let mut tables = vec![];
    
    if let Some(table) = default_table {
        tables.push(table);
    }
    if let Some(table) = dev_table {
        tables.push(table);
    }

    tables.extend(target_dependency_tables);
    tables.extend(target_dev_dependency_tables);

    tables
}

/// Trims off the optional 'dep:' prefix and the '/<feature>' suffix.
fn trim_feature(mut s: &str) -> &str {
    s = s.split_once("dep:")
        .map(|(_l, r)| r)
        .unwrap_or(s);
    s = s.split_once('/')
        .map(|(l, _r)| l)
        .unwrap_or(s);
    s
}