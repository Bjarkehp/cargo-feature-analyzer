
use tokio::process::Command;

pub async fn is_installed() -> Result<(), tokio::io::Error> {
    Command::new("flamapy").output().await?;
    Ok(())
}