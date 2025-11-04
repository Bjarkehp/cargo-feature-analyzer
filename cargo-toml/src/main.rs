use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    name: String,

    #[arg(short, long, default_value = None)]
    crate_version: Option<String>,
    #[arg(short, long, default_value = None)]
    destination: Option<PathBuf>
}

fn main() {
    let args = Args::parse();

    let destination = args.destination.unwrap_or_else(|| {
        let parent = std::env::current_dir()
            .expect("Failed to get current directory");
        PathBuf::from(format!("{}/{}.toml", parent.display(), &args.name))
    });

    let crate_version = args.crate_version.unwrap_or_else(|| {
        let client = cargo_toml::default_client().unwrap();
        let version = cargo_toml::latest_version(&args.name, &client).unwrap();
        version.num
    });

    let content = cargo_toml::download(&args.name, &crate_version).unwrap();
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|_| panic!("Failed to create directories for {destination:?}"))
    }
    std::fs::write(&destination, content)
        .unwrap_or_else(|_| panic!("Failed to write the content of {destination:?}"));
}