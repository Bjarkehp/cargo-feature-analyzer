#!/bin/bash
UVL_FILE="$1"
CSVCONF_DIR="$2"

GREEN='\033[0;32m'
RED='\033[0;31m'
NO_COLOR='\033[0m'

if [[ ! -f "$UVL_FILE" ]]; then
    echo "Error: $UVL_FILE is not a valid file."
    exit 1
fi

if [[ ! -d "$CSVCONF_DIR" ]]; then
    echo "Error: $CSVCONF_DIR is not a valid directory."
    exit 1
fi

for conf in "$CSVCONF_DIR"/*.csvconf; do
    result=$(flamapy satisfiable_configuration "$UVL_FILE" "$conf")
    
    if [[ "$result" == "True" ]]; then
        echo -e "${GREEN}$conf${NO_COLOR}"
    else
        echo -e "${RED}$conf${NO_COLOR}"
    fi
done