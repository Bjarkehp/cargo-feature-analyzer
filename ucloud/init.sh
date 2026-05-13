#!/usr/bin/env bash
set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
POSTGRES_BIN="/usr/lib/postgresql/16/bin"

sudo apt update
sudo apt install -y postgresql-16 postgresql-contrib

sudo chmod +x /work

mkdir -p /work/crates_db/data 

if [ ! -f /work/crates_db/data/PG_VERSION ]; then
    sudo -u postgres $POSTGRES_BIN/initdb -D /work/crates_db/data
fi

sudo -u postgres $POSTGRES_BIN/pg_ctl -D /work/crates_db/data -l /work/crates_db/logfile start

sudo chown -R postgres:postgres /work/crates_db

sudo -u postgres touch /work/crates_db/logfile