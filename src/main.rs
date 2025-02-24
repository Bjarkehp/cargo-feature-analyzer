mod pre_order;
mod feature_dependencies;
mod dependency;

use tree_sitter::{Language, Parser};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_toml = include_str!("tokio.toml");
    let source_rs = include_str!("example.rs");

    println!("DEPENDENCIES:");
    let dependency_graph = feature_dependencies::from_cargo_toml(source_toml)?;
    for (feature, dependency) in dependency_graph.iter() {
        println!("{} = {:?}", feature, dependency);
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