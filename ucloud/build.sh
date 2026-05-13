#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
POSTGRES_BIN="/usr/lib/postgresql/16/bin"

sudo apt update
sudo apt install postgresql postgresql-contrib

sudo mkdir /crates_db
sudo -u postgres "$POSTGRES_BIN/initdb" /usr/local/pgsql/data
postgres -D /usr/local/pgsql/data >"$SCRIPT_DIR/postgres.log" 2>&1 &

tar -xzf "$SCRIPT_DIR/db-dump.tar.gz" -C "$SCRIPT_DIR/dump" --strip-components=1
mv "$SCRIPT_DIR/dump/data" /usr/local/pgsql/data
rm -r "$SCRIPT_DIR/dump"

psql -f "$SCRIPT_DIR/config.sql"
psql -f "$SCRIPT_DIR/schema.sql"
psql -f "$SCRIPT_DIR/import.sql"

echo "crates.io database is ready to start"