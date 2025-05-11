#!/bin/bash
UVL_FILE="$1"
CSVCONF_DIR="$2"

if [[ ! -f "$UVL_FILE" ]]; then
    echo "Error: $UVL_FILE is not a valid file."
    exit 1
fi

if [[ ! -d "$CSVCONF_DIR" ]]; then
    echo "Error: $CSVCONF_DIR is not a valid directory."
    exit 1
fi

for conf in "$CSVCONF_DIR"/*.csvconf; do
    echo "$conf"
    flamapy satisfiable_configuration "$UVL_FILE" "$conf"
done