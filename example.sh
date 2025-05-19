#!/bin/bash

# https://stackoverflow.com/a/34676160
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
WORK_DIR=`mktemp -d -p "$DIR"`
if [[ ! "$WORK_DIR" || ! -d "$WORK_DIR" ]]; then
  echo "Could not create temp dir"
  exit 1
fi

function cleanup {      
  rm -r "$WORK_DIR"
}

trap cleanup EXIT

echo "Extracting dependents of tokio to $WORK_DIR..."
cd "$DIR/feature-configuration-scraper"
cargo run -- tokio -d "$WORK_DIR" -c 20

echo "Generating feature model for tokio..."
cd "$DIR/feature-model-generator"
cargo run examples/toml/tokio.toml "$WORK_DIR" "$DIR/tokio.uvl" 

echo "Validating the model..."
cd "$DIR"
./validate_uvl.sh "$DIR/tokio.uvl" "$WORK_DIR"