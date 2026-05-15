#!/usr/bin/env bash
set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

echo "Extracting dump..."
mkdir -p /tmp/db-dump
tar -xzf db-dump.tar.gz -C "/tmp/db-dump" --strip-components=1
mv /tmp/db-dump/data data
sudo chown -R postgres:postgres data
rm -r "/tmp/db-dump"

echo "Importing dump..."
sudo -u postgres psql -f config.sql
sudo -u postgres psql -d crates_io_db -f schema.sql
sudo -u postgres psql -d crates_io_db -f import.sql

sudo rm -r data

echo "Data from crates.io has been imported"