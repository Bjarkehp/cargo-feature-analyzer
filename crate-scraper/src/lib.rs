pub mod crate_data;
pub mod crate_entry;

use cargo_toml::crate_id::CrateId;
use postgres::{Row, types::ToSql};
use semver::Version;

use crate::{crate_data::CrateData, crate_entry::CrateEntry};  

pub fn scrape_popular_by_configurations(client: &mut postgres::Client, count: i64) -> Result<Vec<CrateEntry>, Error> {
    let query = include_str!("popular_by_configurations.sql");
    let params: &[&(dyn ToSql + Sync)] = &[&count];
    let crates = client.query(query, params)?
        .into_iter()
        .map(row_to_crate_id_and_data)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(crates)
}

pub fn scrape_popular_by_downloads(client: &mut postgres::Client, count: i64) -> Result<Vec<CrateEntry>, Error> {
    let query = include_str!("popular_by_downloads.sql");
    let params: &[&(dyn ToSql + Sync)] = &[&count];
    let crates = client.query(query, params)?
        .into_iter()
        .map(row_to_crate_id_and_data)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(crates)
}

pub fn scrape_with_download_limit(client: &mut postgres::Client, count: i64, download_limit: i64) -> Result<Vec<CrateEntry>, Error> {
    let query = include_str!("download_limit.sql");
    let params: &[&(dyn ToSql + Sync)] = &[&count, &download_limit];
    let crates = client.query(query, params)?
        .into_iter()
        .map(row_to_crate_id_and_data)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(crates)
}

fn row_to_crate_id_and_data(row: Row) -> Result<CrateEntry, Error> {
    let name: String = row.get("crate_name");
    let num: String = row.get("num");
    let downloads: i64 = row.get("downloads");

    let version: Version = num.parse()?;
    let id = CrateId::new(name, version);
    let data = CrateData { downloads };
    let entry = CrateEntry::new(id, data);
    Ok(entry)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error while querying for crates: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("Error while parsing crate version: {0}")]
    ParseSemver(#[from] semver::Error)
}