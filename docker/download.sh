#!/bin/bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
curl -L "https://drive.usercontent.google.com/download?id=1NERRdYkTfDdmnQsqTALE5elrwG0FWhNY&confirm=yy" -o "$SCRIPT_DIR/db-dump2.tar.gz"
