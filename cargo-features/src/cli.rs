use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command
}

#[derive(Subcommand)]
pub enum Command {
    CargoToml {
        name: String,

        #[arg(short, long, default_value = None)]
        crate_version: Option<String>,
        #[arg(short, long, default_value = None)]
        destination: Option<PathBuf>
    },
}