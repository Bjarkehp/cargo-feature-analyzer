#!/usr/bin/env bash
set -e

PG_BIN="/usr/lib/postgresql/16/bin"
PG_DIR="/work/crates_db"
PG_DATA="$PG_DIR/data"
PG_LOG="$PG_DIR/logfile"

sudo chmod +x /work
sudo chown -R postgres:postgres "$PG_DIR"

if ! sudo test -f "$PG_DATA/PG_VERSION"; then
    sudo -u postgres mkdir -p "$PG_DATA"
    sudo -u postgres touch "$PG_LOG"
    sudo -u postgres "$PG_BIN/initdb" -D "$PG_DATA"
fi

sudo -u postgres "$PG_BIN/pg_ctl" -D "$PG_DATA" -l "$PG_LOG" start
