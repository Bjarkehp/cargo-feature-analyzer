use std::path::PathBuf;


use clap::Parser;
use crate_scraper::scrape_popular_by_configurations;
use postgres::NoTls;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    destination: PathBuf
}

fn main() {
    let args = Args::parse();

    let connection_string = "postgres://crates:crates@localhost:5432/crates_io_db";
    let mut client = postgres::Client::connect(connection_string, NoTls).unwrap();
    let popular_crates = scrape_popular_by_configurations(&mut client, 1000).unwrap();

    let mut writer = csv::Writer::from_path(&args.destination)
        .unwrap_or_else(|e| panic!("Failed to create csv writer at {:?}: {e}", args.destination));


    for entry in popular_crates {
        writer.serialize(entry)
            .unwrap_or_else(|e| panic!("Failed to write into file at {:?}: {e}", args.destination));
    }

    writer.flush().unwrap_or_else(|e| panic!("Failed to flush writer at {:?}: {e}", args.destination))
}