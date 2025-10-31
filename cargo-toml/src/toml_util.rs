use thiserror::Error;

/// Return the toml table with the given key
pub fn get_table<'a>(parent: &'a toml::Table, key: &str) -> Result<&'a toml::Table> {
    parent.get(key)
        .ok_or(Error::KeyMissing(key.to_string()))?
        .as_table()
        .ok_or(Error::UnexpectedType(key.to_string(), "table"))
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0} was not found")]
    KeyMissing(String),
    #[error("Expected {0} to be of type {1}")]
    UnexpectedType(String, &'static str)
}