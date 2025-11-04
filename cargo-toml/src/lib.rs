pub mod feature_dependencies;
pub mod implied_features;
pub mod toml_util;

use crates_io_api::SyncClient as CratesIoClient;
use crates_tools::CrateArchive;
use itertools::Itertools;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    CratesIoError(#[from] crates_io_api::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Could not create client")]
    CreateClient,
    #[error("No versions found for crate")]
    NoVersionsFound,
    #[error("Could not download crate archive")]
    DownloadCrateArchive,
    #[error("Could not extract Cargo.toml from crate archive")]
    Extract,
}

/// Downloads the Cargo.toml content of the specified crate and version.
pub fn download(crate_name: &str, version: &str) -> Result<String, Error> {
    //let version = latest_version(crate_name, client)?;
    let crate_archive = CrateArchive::download_crates_io(crate_name, version)
        .map_err(|_| Error::DownloadCrateArchive)?;
    let cargo_toml_path = format!("{}-{}/Cargo.toml", crate_name, version);
    let cargo_toml_bytes = crate_archive.content_bytes(cargo_toml_path)
        .ok_or(Error::Extract)?;
    let cargo_toml_string = String::from_utf8_lossy(cargo_toml_bytes).into_owned();
    Ok(cargo_toml_string)
}

/// Creates the default client used when no client is specified.
pub fn default_client() -> Result<CratesIoClient, Error> {
    let user_agent = "feature-configuration-scraper (bjpal22@student.sdu.dk)";
    let rate_limit = std::time::Duration::from_millis(1000);
    CratesIoClient::new(user_agent, rate_limit)
        .map_err(|_| Error::CreateClient)
}

/// Finds the latest version of a crate by using crates_io_api.
pub fn latest_version(crate_name: &str, client: &CratesIoClient) -> Result<crates_io_api::Version, Error> {
    let _crate = client.get_crate(crate_name)?;
    let latest_version = _crate.versions.into_iter()
        .sorted_by_key(|v| semver::Version::parse(&v.num).expect("Crate version should be able to be parsed"))
        .next_back()
        .ok_or(Error::NoVersionsFound)?;
    Ok(latest_version)
}