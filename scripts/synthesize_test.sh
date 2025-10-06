#!/bin/bash

POSITIONAL_ARGS=()

# Default arguments
synthesize=100
test=100

while [[ $# -gt 0 ]]; do
    case "$1" in
        -s|--synthesize)
            synthesize="$2"
            shift 2
            ;;
        -t|--test)
            test="$2"
            shift 2
            ;;
        *)
            POSITIONAL_ARGS+=("$1")
            shift
            ;;
    esac
done

crate="${POSITIONAL_ARGS[0]}"
toml="${POSITIONAL_ARGS[1]}"
destination="${POSITIONAL_ARGS[2]}"

set -- "${POSITIONAL_ARGS[@]}"

connection_string="postgresql://crates:crates@localhost:5432/crates_db"

cargo run --bin feature-configuration-postgres "$toml" "$connection_string" "$destination"/synthesize --limit "$synthesize"
cargo run --bin feature-configuration-postgres "$toml" "$connection_string" "$destination"/test --offset "$synthesize" --limit "$test"
cargo run --bin feature-model-generator "$destination"/synthesize models/"$crate".uvl
scripts/validate.sh models/"$crate".uvl "$destination"/test