# cargo-feature-analyzer
A set of tools for analyzing variability of Rust crates.

This repository provides tools for synthesizing feature models for Rust crates and analyzing the variability of those feature models. The feature models are either synthesized based on the crates' Cargo.toml (flat model), or configurations of the crate, which are scraped from a crates.io database dump (FCA model).

The analysis checks for correlation between several properties of the crates (e.g. correlation between line count and number of features), and compares the flat feature model with the FCA feature model.

## Packages
* **analysis**: Scrapes crates and configurations for a specified number of crates, and builds plots showing showing the correlation of several properties of those crates.
* **cargo-toml**: Contains several utility types and functions for handling Rust crates.
* **configuration-scraper**: Connects to a postgres database containing the crates.io dump, and queries for crates that have a specific dependency.
* **crate-scraper**: Connects to a postgres database containing the crates.io dump, and finds popular crates determined by different parameters.
* **feature-model-generator-milp**: Experiment on synthesizing feature models using a MILP solver (SCIP).
* **fm-synthesizer-fca**: Synthesizes a feature model based on a set of configurations. The crate using Formal Concept Analysis to generate an Attribute-Concept Partially Ordered Set (AC-poset), which is then turned into a feature model. This feature model represents the practical configuration space of a crate, based on how it is used by other crates.
* **fm-synthesizer-flat**: Synthesizes a feature model based on the Cargo.toml of a crate. This model represents the theoretical configuration space of a crate.

## Requirements

### Rust
To run any of the crates, you must have a working [Rust toolchain](https://rust-lang.org/) installed.

### Flamapy
The analysis uses Flamapy to calculate certain properties of feature models. We recommend using version 2.1.0.dev1, which can be installed using pip (or pipx):

```bash
pip install flamapy==2.1.0.dev1
```

### crates.io postgres database
Many crates use the crates.io database dump, to avoid sending too much traffic to crates.io. We have provided a docker container in this repository as well as a few scripts to get the database quickly up and running. The docker container also contains small modifications to import.sql and some extra index tables in schema.sql make scraping faster. This of course requires that the host machine has docker installed.

First, the content of the database dump must be downloaded. Either download [the newest archive](https://static.crates.io/db-dump.tar.gz) and place it in the path ```docker/db-dump.tar.gz```, or run ```docker/download.sh``` to get the dump used to get our results.

To build the docker container and the database, run ```docker/build.sh```.

Finally, to start the database, run ```docker/start.sh```

### SCIP (optional)
To run the feature model synthesizer utilizing MILP, you need to download SCIP. The crate uses a solver-agnostic MILP library, so using a different solver is possible with a few small changes to the source code, if necessary.

## Running the analysis
To run the analysis, simply run the following command:

```bash 
cargo run --bin analysis
```

## Running other crates
All other crates provide a nice list of parameters when passing the ```-h``` or ```--help``` argument. If you use cargo to compile and run the crates, there are a few things to keep in mind. First, the name of the binary crate doesn't always match the name of the package. For example, to run the binary crate inside fm-synthesizer-fca, you must use the name fm_synthesizer_fca_bin, which can be found in the package's Cargo.toml. Second, you must seperate the cargo arguments from the arguments provided to the crate using ```--```. This example shows how to compile and run the fm_synthesizer_fca_bin crate, assuming you have configurations for tokio stored in the directory ```configurations/tokio```:

```bash
cargo run --bin fm_synthesizer_fca_bin -- --ac-poset ac-poset.dot tokio configurations/tokio tokio.uvl
```