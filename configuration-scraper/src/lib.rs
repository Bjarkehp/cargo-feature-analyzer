use std::{borrow::Cow, collections::{BTreeMap, BTreeSet}};

use cargo_toml::{feature_dependencies, implied_features};
use postgres::{Row, types::ToSql};
use semver::{Version, VersionReq};

use crate::configuration::Configuration;

pub use postgres;

pub mod configuration;

pub fn scrape(
    crate_name: &str, 
    crate_version: &Version, 
    feature_dependencies: &feature_dependencies::Graph,
    client: &mut postgres::Client, 
    offset: i64, 
    limit: i64
) -> Result<Vec<Configuration<'static>>, Error> {
    let features = feature_dependencies.nodes()
        .collect::<Vec<_>>();

    let query = include_str!("query.sql");
    let params: &[&(dyn ToSql + Sync)] = &[
        &crate_name, 
        &limit, 
        &offset
    ];

    let rows = client.query(query, params)?;
    
    let configurations = rows.iter()
        .filter_map(|row| row_to_config(row, crate_version, &features, feature_dependencies))
        .collect::<Vec<_>>();

    Ok(configurations)
}

fn row_to_config(
    row: &Row, 
    crate_version: &Version,
    features: &[&str],
    feature_dependencies: &feature_dependencies::Graph,
) -> Option<Configuration<'static>> {
    let dependent_name: String = row.get("dependent_crate");
    let dependency_requirement_str: String = row.get("dependency_requirement");
    let dependency_requirement = VersionReq::parse(&dependency_requirement_str)
        .unwrap_or_else(|e| panic!("Failed to parse version requirement for dependent {dependent_name}: {e}"));

    if !dependency_requirement.matches(crate_version) {
        return None;
    }

    let version_string: String = row.get("dependent_version");
    let version = version_string
        .parse::<Version>()
        .unwrap_or_else(|e| panic!("Failed to parse version of dependent {dependent_name}: {e}"));
    let mut explicit_features: Vec<String> = row.get("features");
    let default_features: bool = row.get("default_features");
    if default_features {
        explicit_features.push("default".to_string());
    }

    let enabled_features = implied_features::from_dependency_graph(explicit_features.iter().map(|f| f.as_str()), feature_dependencies)
        .into_iter()
        .map(|f| Cow::Owned(f.to_string()))
        .collect::<BTreeSet<Cow<str>>>();
    let features = features.iter()
        .map(|&s| s.to_owned())
        .map(Cow::Owned)
        .map(|s| {
            let is_enabled = enabled_features.contains(&s);
            (s, is_enabled)
        })
        .collect::<BTreeMap<_, _>>();

    let configuration = Configuration::new(
        dependent_name,
        version,
        features
    );

    Some(configuration)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
}