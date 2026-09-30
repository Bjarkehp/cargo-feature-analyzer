#!/bin/bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "$SCRIPT_DIR"

docker build -t crates_io_db --no-cache .

if [ "$(docker ps -a -q -f name=crates_io_db$)" ]; then
    # Container exists
    if [ "$(docker ps -q -f name=crates_io_db$)" ]; then
        docker stop crates_io_db
    fi
    docker rm crates_io_db
fi

docker create --name crates_io_db -p 5432:5432 crates_io_db