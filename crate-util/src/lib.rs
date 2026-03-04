pub mod feature_dependencies;
pub mod implied_features;
pub mod toml_util;
pub mod crate_id;

use std::{fs::File, io::{Cursor, Read}, path::{Path, PathBuf}, time::Duration};

use crates_io_api::SyncClient as CratesIoClient;
use flate2::read::GzDecoder;
use itertools::Itertools;
use tar::Archive;

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
    DownloadCrateArchive(#[from] #[source] reqwest::Error),
    #[error("Could not extract Cargo.toml from crate archive")]
    Extract,
}

pub fn download(client: &reqwest::blocking::Client, name: &str, version: &str) -> Result<Archive<impl Read>, Error> {
    std::thread::sleep(Duration::from_secs(1));
    let crate_archive_bytes = client.get(format!("https://static.crates.io/crates/{name}/{name}-{version}.crate"))
        .send()?
        .bytes()?;
    let cursor = Cursor::new(crate_archive_bytes);
    let gz = GzDecoder::new(cursor);
    Ok(Archive::new(gz))
}

/// Downloads the Cargo.toml content of the specified crate and version.
pub fn download_and_save(client: &reqwest::blocking::Client, name: &str, version: &str, dir: &Path) -> Result<(), Error> {
    let mut archive = download(client, name, version)?;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = dir.join(entry.path()?);
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(path)?;
        std::io::copy(&mut entry, &mut file)?;
    }

    Ok(())
}

pub fn download_cargo_toml(client: &reqwest::blocking::Client, name: &str, version: &str) -> Result<Option<String>, Error> {
    let mut archive = download(client, name, version)?;
    let cargo_toml_path = PathBuf::from(format!("{}-{}/Cargo.toml", name, version));
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path == cargo_toml_path {
            let mut string = String::new();
            entry.read_to_string(&mut string)?;
            return Ok(Some(string));
        }
    }
    Ok(None)
}

/// Creates the default client used when no client is specified.
pub fn default_cargo_client() -> Result<CratesIoClient, Error> {
    let user_agent = "cargo-feature-analysis (bjpal22@student.sdu.dk)";
    let rate_limit = std::time::Duration::from_millis(1000);
    CratesIoClient::new(user_agent, rate_limit)
        .map_err(|_| Error::CreateClient)
}

pub fn default_reqwest_client() -> Result<reqwest::blocking::Client, Error> {
    let client = reqwest::blocking::ClientBuilder::new()
        .user_agent("cargo-feature-analysis (bjpal22@student.sdu.dk)")
        .build()?;
    Ok(client)
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