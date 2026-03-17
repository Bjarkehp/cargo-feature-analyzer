use nameof::name_of;

use crate::args::Args;

pub struct Config {
    pub connection_string: String,
    pub number_of_crates: usize,
    pub max_features: usize,
    pub min_configs: usize,
    pub max_configs: usize,
    pub max_dependencies: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self { 
            connection_string: "postgres://crates:crates@localhost:5432/crates_io_db".to_owned(), 
            number_of_crates: 100, 
            max_features: 100, 
            min_configs: 100, 
            max_configs: 1000, 
            max_dependencies: 1000
        }
    }
}

macro_rules! config_replace {
    ($config:expr, $args:expr, $map:ident, $ident:ident) => {
        $config.$ident = $args.$ident
            .or_else(|| $map(name_of!($ident in Args)))
            .unwrap_or($config.$ident);
    };
}

pub fn config_from_args(args: Args) -> anyhow::Result<Config> {
    let maybe_toml_config = args.config
        .as_ref()
        .map(std::fs::read_to_string)
        .transpose()?
        .map(|s| s.parse::<toml::Table>())
        .transpose()?;

    let mut config = Config::default();

    if let Some(toml_config) = maybe_toml_config {
        let usize_map = |k: &str| toml_config.get(k)
            .and_then(|v| v.as_integer().filter(|&i| i >= 0).map(|i| i as usize));

        let str_map = |k: &str| toml_config.get(k)
            .and_then(|v| v.as_str().map(str::to_string));

        config_replace!(config, args, str_map, connection_string);
        config_replace!(config, args, usize_map, number_of_crates);
        config_replace!(config, args, usize_map, max_features);
        config_replace!(config, args, usize_map, min_configs);
        config_replace!(config, args, usize_map, max_configs);
        config_replace!(config, args, usize_map, max_dependencies);
    }

    Ok(config)
}