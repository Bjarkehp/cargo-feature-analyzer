pub mod pre_order;

use tree_sitter::{Language, Parser, TreeCursor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new();
    parser.set_language(&Language::new(tree_sitter_rust::LANGUAGE))?;
    let source = include_bytes!("example.rs");
    let tree = parser.parse(source, None)
        .ok_or("Failed to parse")?;

    for (node, depth) in pre_order::walk(tree.walk(), |c| (c.node(), c.depth())) {
        tab(depth);
        println!("{}: {}", node.kind(), node.kind_id());
    }

    Ok(())
}

fn tab(depth: u32) {
    for _ in 0..depth {
        print!("    ");
    }
}