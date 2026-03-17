use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Args {
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub connection_string: Option<String>,
    #[arg(short, long)]
    pub number_of_crates: Option<usize>,
    #[arg(long)]
    pub max_features: Option<usize>,
    #[arg(long)]
    pub min_configs: Option<usize>,
    #[arg(long)]
    pub max_configs: Option<usize>,
    #[arg(long)]
    pub max_dependencies: Option<usize>,
}