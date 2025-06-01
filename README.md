# cargo-feature-analyzer
Contains a set of tools to create and analyze feature models of Rust projects. 

* feature-configuration-scraper fetches dependents of the specified crate from crates.io, and extracts their feature configurations using the crates.io api.
* feature-model-generator uses the extracted feature configurations to create an AC-poset, which is then used to generate a feature model.
* configuration is a crate that provides tools for working with feature configurations.
* validate.sh acts as an integration test that verifies the validity of the feature models based on a set of feature configurations. The script uses flamapy, a tool for analyzing feature models written in UVL.
* analyze.sh is a script which lists the core features, dead features, false optional features and estimated number of configurations in a feature model. This script also uses flamapy.
* scrape.sh runs feature-configuration-scraper with default arguments, putting the configurations in the configurations directory. Only the name of the crate is required as argument.
* generate.sh runs feature-model-generator with default arguments, putting the feature model in the models directory. Only the name of the crate is required as argument.

## Requirements
To compile the tools, you need to have the rust toolchain installed. For analyzing feature models, you need to have flamapy installed.

## Usage
To get started quickly, use the shell scripts.
```bash
scrape.sh \<name of the crate\>
generate.sh \<name of the crate\>
validate.sh \<feature model\> \<csvconf directory\>
analyze.sh \<feature model\>
```

For detailed usage, read the contents of the scripts, or pass --help to either feature-configuration-scraper or feature-model-generator.