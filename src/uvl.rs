use std::{collections::{BTreeSet, HashMap, HashSet}, io::Write};

use itertools::Itertools;

use crate::{dependency::Dependency, directed_graph::DirectedGraph, max_tree};

pub fn write<W: Write>(writer: &mut W, graph: &DirectedGraph<Dependency>) -> std::io::Result<()> {
    let reversed = graph.reversed();
    let mut visited = HashSet::new();
    let mut visited_edges = HashSet::new();

    writeln!(writer, "features")?;
    while let Some((root, tree_edges)) = max_tree_first_root_candidate(&reversed, &mut visited) {
        let tree = tree_edges.iter().cloned().into_group_map();
        write_tree(writer, &tree, root, 1)?;

        for (from, to) in tree_edges {
            visited_edges.insert((to, from));
        }
    }

    writeln!(writer, "constraints")?;
    for (from, to) in graph.edges().filter(|(&from, &to)| !visited_edges.contains(&(from, to))) {
        writeln!(writer, "\t{} => {}", from.representation(), to.representation())?;
    }

    Ok(())
}

fn root_candidates<T: Eq + std::hash::Hash + Ord>(graph: &DirectedGraph<T>) -> BTreeSet<&T> {
    let mut nodes = graph.nodes().collect::<BTreeSet<_>>();

    for (_from, to) in graph.edges() {
        nodes.remove(to);
    }

    nodes
}

fn max_tree_first_root_candidate<'a>(graph: &'a DirectedGraph<Dependency>, visited: &mut HashSet<&'a Dependency<'a>>) -> Option<(Dependency<'a>, Vec<(Dependency<'a>, Dependency<'a>)>)> {
    let root = root_candidates(graph)
        .into_iter()
        .find(|root| !visited.contains(root))?;
    
    let tree = max_tree::find(graph, root, visited)
        .map(|(&from, &to)| (from, to))
        .collect::<Vec<_>>();

    Some((*root, tree))
}

fn write_tree<W: Write>(writer: &mut W, tree: &HashMap<Dependency, Vec<Dependency>>, node: Dependency, depth: usize) -> std::io::Result<()> {
    writeln!(writer, "{}{}", "\t".repeat(depth), node.representation())?;

    let empty = vec![];
    let children = tree.get(&node).unwrap_or(&empty);

    if children.is_empty() {
        return Ok(());
    }

    writeln!(writer, "{}optional", "\t".repeat(depth + 1))?;

    for child in children {
        write_tree(writer, tree, *child, depth + 2)?;
    }
    
    Ok(())
}