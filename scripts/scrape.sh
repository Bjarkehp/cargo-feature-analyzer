#!/bin/bash
mkdir -p configurations/"$1"
cargo run --bin feature-configuration-scraper -- "$1" configurations/"$1".toml configurations/"$1"