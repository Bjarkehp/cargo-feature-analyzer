mod pre_order;
mod feature_dependencies;
mod dependency;
mod uvl;

use std::{fs::File, io::{BufWriter, Write}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_toml = include_str!("../examples/toml/tokio.toml");
    let tokio_uvl = File::create("tokio.uvl")?;
    let mut tokio_uvl_writer = BufWriter::new(tokio_uvl);

    let graph = feature_dependencies::from_cargo_toml(source_toml)?;
    uvl::to_universal_variability_language(&graph, &mut tokio_uvl_writer)?;
    tokio_uvl_writer.flush()?;

    Ok(())
}