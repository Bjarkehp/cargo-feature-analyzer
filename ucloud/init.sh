#!/usr/bin/env bash
set -e

sudo apt update

# Install Postgres
sudo apt install -y postgresql-16 postgresql-contrib

# Install Python
sudo apt install -y python3

# Install flamapy
pip install flamapy

# Install Rust
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"