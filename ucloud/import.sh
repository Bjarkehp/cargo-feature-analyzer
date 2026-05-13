#!/bin/bash
set -e

DUMP_URL="https://static.crates.io/db-dump.tar.gz"
DUMP_DIR="/tmp/dump"

echo "Downloading crates.io dump..."
mkdir -p "$DUMP_DIR"
# curl -SL "$DUMP_URL" -o "$DUMP_DIR/dump.tar.gz" --progress-bar
wget -O "$DUMP_DIR/dump.tar.gz" "$DUMP_URL" --progress=dot:giga

echo "Unpacking dump..."
tar -xzf "$DUMP_DIR/dump.tar.gz" -C "$DUMP_DIR" --strip-components=1

cp /import.sql "$DUMP_DIR/import.sql"

cd "$DUMP_DIR"

echo "Initializing schema..."
psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$DUMP_DIR/schema.sql"

echo "Importing data..."
psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$DUMP_DIR/import.sql"

echo "Cleaning up dump files..."
rm -rf "$DUMP_DIR"

echo "Database setup complete."