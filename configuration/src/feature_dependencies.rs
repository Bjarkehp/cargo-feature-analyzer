
use std::collections::BTreeMap;

use crate::{toml_util::{get_table,Error, Result}};

pub type Map<'a> = BTreeMap<&'a str, Vec<&'a str>>;

pub fn from_cargo_toml(root: &toml::Table) -> Result<Map> {
    let feature_table = get_table(root, "features")?;
    let dependency_tables = get_dependency_tables(root)?;

    let mut feature_dependencies = explicit_feature_dependencies(feature_table)?;
    let optional_dependencies = dependency_tables.into_iter()
        .flat_map(optional_dependencies)
        .filter(|d| !feature_dependencies.contains_key(d))
        .collect::<Vec<_>>();

    for dependency in optional_dependencies {
        feature_dependencies.insert(dependency, vec![]);
    }

    Ok(feature_dependencies)
}

fn explicit_feature_dependencies(table: &toml::Table) -> Result<Map> {
    let mut map = Map::new();

    for (key, value) in table {
        if key != "default" {
            let feature = key.as_str();
            let dependencies = value.as_array()
                .ok_or(Error::UnexpectedType(key.to_string(), "array"))?
                .iter()
                .enumerate()
                .map(|(i, d)| d.as_str()
                    .ok_or(Error::UnexpectedType(format!("{}[{}]", key, i), "str")))
                .collect::<Result<Vec<&str>>>()?;

            map.insert(feature, dependencies);
        }
    }

    Ok(map)
}

fn optional_dependencies(table: &toml::Table) -> impl Iterator<Item = &str> {
    table.iter()
        .filter(|&(_k, v)| dependency_is_optional(v))
        .map(|(k, _v)| k.as_str())
}

fn get_dependency_tables(root: &toml::Table) -> Result<Vec<&toml::Table>> {
    let default_table = get_table(root, "dependencies")?;
    let target_table = root.get("target")
        .and_then(|v| v.as_table());

    if let Some(t) = target_table {
        let mut tables = t.values()
            .filter_map(|v| v.as_table())
            .filter_map(|t| get_table(t, "dependencies").ok())
            .collect::<Vec<_>>();

        tables.push(default_table);
        Ok(tables)
    } else {
        Ok(vec![default_table])
    }
}

fn dependency_is_optional(dependency: &toml::Value) -> bool {
    dependency.as_table()
        .and_then(|t| t.get("optional"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}