pub mod pre_order;

use toml::Table;
use tree_sitter::{Language, Parser};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_toml = include_str!("example.toml");
    let source_rs = include_str!("example.rs");

    // Using the crate 'toml' to parse and analyze example.toml
    let table = source_toml.parse::<Table>()?;
    let dependencies = table.get("dependencies")
        .expect("example.toml doesn't have any dependencies")
        .as_table()
        .expect("dependencies in example.toml is not a table");
    
    println!("DEPENDENCIES:");
    for dependency in dependencies.keys() {
        println!("{} = {:?}", dependency, dependencies.get(dependency).unwrap());
    }
    println!();

    // Using tree-sitter and tree-sitter-rust to parse example.rs.
    // The tree can be walked by creating a TreeCursor using tree.walk.
    // The function pre_order::walk creates an iterator of data selected from the cursor at each token.
    // In this example, only the current node and current depth is selected.
    let mut parser = Parser::new();
    parser.set_language(&Language::new(tree_sitter_rust::LANGUAGE))?;
    let tree = parser.parse(source_rs, None)
        .expect("Failed to parse");

    println!("SOURCE CODE:");
    for (node, depth) in pre_order::walk(&tree, |c| (c.node(), c.depth())) {
        for _ in 0..depth {
            print!(".   ");
        }
        println!("{}", node.kind());
    }

    Ok(())
}