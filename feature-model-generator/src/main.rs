mod feature_dependencies;
mod dependency;
mod uvl;
mod directed_graph;
mod max_tree;
mod feature_configuration;

use std::{fs::File, io::{BufWriter, Write}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_toml = include_str!("../examples/toml/tokio.toml");
    let tokio_uvl = File::create("tokio.uvl")?;
    let mut tokio_uvl_writer = BufWriter::new(tokio_uvl);

    let toml_table = source_toml.parse()?;
    let graph = feature_dependencies::from_cargo_toml(&toml_table)?;
    uvl::write(&mut tokio_uvl_writer, &graph)?;
    tokio_uvl_writer.flush()?;

    Ok(())
}