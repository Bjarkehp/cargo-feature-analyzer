pub type Client = crates_io_api::SyncClient;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not create client")]
    CreateClient,
}

/// Creates the default client for communicating with crates.io.
pub fn default_client() -> Result<Client, Error> {
    let user_agent = "feature-configuration-scraper (bjpal22@student.sdu.dk)";
    let rate_limit = std::time::Duration::from_millis(1000);
    Client::new(user_agent, rate_limit)
        .map_err(|_| Error::CreateClient)
}