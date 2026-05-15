#!/bin/bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "$SCRIPT_DIR"
curl -L "https://drive.usercontent.google.com/download?id=1NERRdYkTfDdmnQsqTALE5elrwG0FWhNY&confirm=yy" -o db-dump.tar.gz
chmod 755 db-dump.tar.gz