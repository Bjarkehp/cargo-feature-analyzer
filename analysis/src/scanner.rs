use std::{cmp::max, ops::{Add, Range}, path::{Path, PathBuf}};

use itertools::Itertools;
use walkdir::WalkDir;

#[derive(Default, Debug)]
pub struct ScanResult {
    loc: usize,
    lof: usize
}

impl Add for ScanResult {
    type Output = ScanResult;

    fn add(self, rhs: Self) -> Self::Output {
        ScanResult {
            loc: self.loc + rhs.loc,
            lof: self.lof + rhs.lof
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to set parser language")]
    LanguageError(#[from] tree_sitter::LanguageError),
    #[error("failed to read source")]
    ReadSource(std::io::Error),
    #[error("failed to parse source")]
    ParseSource,
}

pub fn scan<P: AsRef<Path>>(path: P) -> Result<ScanResult, Error> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;

    let result = rust_file_paths(path)
        .map(|p| -> Result<_, Error> {
            let source = std::fs::read_to_string(&p)
                .map_err(Error::ReadSource)?;
            let lof = scan_file(&mut parser, &source)?;
            Ok(lof)
        })
        .fold_ok(ScanResult::default(), |a, b| a + b)?;

    Ok(result)
}

fn rust_file_paths(path: impl AsRef<Path>) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".rs"))
        .map(|e| e.into_path())
}

fn scan_file(parser: &mut tree_sitter::Parser, source: &str) -> Result<ScanResult, Error> {
    let tree = parser.parse(source, None)
        .ok_or(Error::ParseSource)?;
    let root = tree.root_node();
    let mut lof_ranges = vec![];
    scan_node(root, source, &mut lof_ranges);
    let union = range_union(lof_ranges);
    let lof: usize = union
        .into_iter()
        .map(|r| r.end - r.start)
        .sum();
    let loc = root.end_position().row - root.start_position().row + 1;
    Ok(ScanResult { loc, lof })
}

fn scan_node(node: tree_sitter::Node, source: &str, lof: &mut Vec<Range<usize>>) {
    let mut cursor = node.walk();
    let mut children = node.children(&mut cursor).peekable();
    let mut start = node.start_position().row + 1;
    let mut is_feature_child = false;

    while let Some(child) = children.next() {
        match child.kind() {
            "attribute_item" => {
                let is_cfg = scan_attribute_item_for_cfg(child, source);
                let is_cfg_attr = scan_attribute_item_for_cfg_attr(child, source);
                let has_feature = scan_attribute_item_for_feature(child, source);

                if is_cfg_attr && has_feature {
                    let start = child.start_position().row + 1;
                    let end = child.end_position().row + 2;
                    lof.push(start..end)
                } else if is_cfg && has_feature {
                    is_feature_child = true
                }
            },
            _ => {
                scan_node(child, source, lof);
                if is_feature_child {
                    let end = child.end_position().row + 2;
                    lof.push(start..end);
                }
                start = children.peek().map(|n| n.start_position().row + 1).unwrap_or(0);
                is_feature_child = false;
            },
        }
    }
}

fn scan_attribute_item_for_cfg_attr(node: tree_sitter::Node, source: &str) -> bool {    
    scan_attribute_item_for_identifier(node, source, |s| s == "cfg_attr")
}

fn scan_attribute_item_for_cfg(node: tree_sitter::Node, source: &str) -> bool {    
    scan_attribute_item_for_identifier(node, source, |s| s == "cfg")
}

fn scan_attribute_item_for_feature(node: tree_sitter::Node, source: &str) -> bool {    
    scan_attribute_item_for_identifier(node, source, |s| s == "feature")
}

fn scan_attribute_item_for_identifier(node: tree_sitter::Node, source: &str, predicate: impl (Fn(&str) -> bool) + Clone) -> bool {    
    if node.kind() == "identifier" {
        let identifier = node.utf8_text(source.as_bytes());
        identifier.map(predicate).unwrap_or(false)
    } else {
        let mut cursor = node.walk();
        let mut children = node.children(&mut cursor);
        children.any(|child| scan_attribute_item_for_identifier(child, source, predicate.clone()))
    }
}

fn range_union(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    if ranges.is_empty() {
        return ranges;
    }

    let mut union = vec![];
    ranges.sort_by_key(|r| (r.start, r.end));
    union.push(ranges[0].clone());

    for r in ranges {
        if let Some(u) = union.last_mut() && r.start <= u.end {
            u.end = max(r.end, u.end);
        } else {
            union.push(r)
        }
    }

    union
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_on_test_src() {
        let result = scan("test/lof.rs")
            .map_err(|e| format!("scan failed: {e}"))
            .unwrap();

        assert_eq!(result.loc, 40);
        assert_eq!(result.lof, 21);
    }
}