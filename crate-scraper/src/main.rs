use std::{fs::File, io::{BufWriter, Write}, path::PathBuf};


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

    let file = File::create(&args.destination)
        .unwrap_or_else(|e| panic!("Failed to create file at {:?}: {e}", args.destination));
    let mut writer = BufWriter::new(file);

    for entry in popular_crates {
        writeln!(writer, "{}", entry)
            .unwrap_or_else(|e| panic!("Failed to write into file at {:?}: {e}", args.destination));
    }
}