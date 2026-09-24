use std::{cmp::max, collections::{HashMap, HashSet}, ops::{Add, Range}, path::{Path, PathBuf}};

use itertools::Itertools;
use walkdir::WalkDir;

use crate::statistics;

#[derive(Debug)]
pub struct ScanResult {
    pub loc: usize,
    pub lof: usize,
    pub sd_mean: f64,
    pub sd_dev: f64,
    pub td_mean: f64,
    pub td_dev: f64,
    pub and_mean: f64,
    pub and_dev: f64,
}

#[derive(Default, Debug)]
struct FileScanResult {
    pub loc: usize,
    pub lof: usize,
}

impl Add for FileScanResult {
    type Output = FileScanResult;

    fn add(self, rhs: Self) -> Self::Output {
        FileScanResult {
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

    let mut sd = HashMap::new();
    let mut td = vec![];
    let mut and = vec![];

    let file_result = rust_file_paths(path)
        .map(|p| -> Result<_, Error> {
            let source = std::fs::read_to_string(&p)
                .map_err(Error::ReadSource)?;
            let lof = scan_file(&mut parser, &source, &mut sd, &mut td, &mut and)?;
            Ok(lof)
        })
        .fold_ok(FileScanResult::default(), |a, b| a + b)?;

    let (sd_mean, sd_dev) = statistics::mean_dev(|| sd.values().copied());
    let (td_mean, td_dev) = statistics::mean_dev(|| td.iter().copied());
    let (and_mean, and_dev) = statistics::mean_dev(|| and.iter().copied());

    let result = ScanResult {
        loc: file_result.loc,
        lof: file_result.lof,
        sd_mean,
        sd_dev,
        td_mean,
        td_dev,
        and_mean,
        and_dev,
    };

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

fn scan_file(
    parser: &mut tree_sitter::Parser, 
    source: &str,
    sd: &mut HashMap<String, u32>,
    td: &mut Vec<u32>,
    and: &mut Vec<u32>,
) -> Result<FileScanResult, Error> {
    let tree = parser.parse(source, None)
        .ok_or(Error::ParseSource)?;
    let root = tree.root_node();

    let mut lof = vec![];
    scan_node(root, source, &mut lof, sd, td, and, 1);

    let loc = root.end_position().row - root.start_position().row + 1;

    let lof_union = range_union(lof);
    let lof: usize = lof_union
        .into_iter()
        .map(|r| r.end - r.start)
        .sum();

    Ok(FileScanResult { loc, lof })
}

fn scan_node(
    node: tree_sitter::Node, 
    source: &str, 
    lof: &mut Vec<Range<usize>>,
    sd: &mut HashMap<String, u32>,
    td: &mut Vec<u32>,
    and: &mut Vec<u32>,
    depth: u32,
) {
    let mut cursor = node.walk();
    let mut children = node.children(&mut cursor).peekable();
    let mut start = node.start_position().row + 1;
    let mut is_feature_child = false;

    while let Some(child) = children.next() {
        match child.kind() {
            "attribute_item" => {
                let is_cfg = scan_attribute_item_for_cfg(child, source);
                let is_cfg_attr = scan_attribute_item_for_cfg_attr(child, source);

                if !is_cfg && !is_cfg_attr {
                    continue;
                }

                let mut visited = HashSet::new();
                let feature_count = scan_attribute_item_for_features(child, source, sd, &mut visited);
                if feature_count > 0 {
                    td.push(feature_count);
                } else {
                    continue;
                }

                if is_cfg_attr {
                    let start = child.start_position().row + 1;
                    let end = child.end_position().row + 2;
                    lof.push(start..end)
                } else if is_cfg {
                    is_feature_child = true
                }
            },
            _ => {
                if is_feature_child {
                    scan_node(child, source, lof, sd, td, and, depth + 1);
                    let end = child.end_position().row + 2;
                    lof.push(start..end);
                    and.push(depth);
                } else {
                    scan_node(child, source, lof, sd, td, and, depth);
                }
                start = children.peek().map(|n| n.start_position().row + 1).unwrap_or(0);
                is_feature_child = false;
            },
        }
    }
}

fn scan_attribute_item_for_cfg(node: tree_sitter::Node, source: &str) -> bool {    
    scan_attribute_item_for_identifier(node, source, |s| s == "cfg")
}

fn scan_attribute_item_for_cfg_attr(node: tree_sitter::Node, source: &str) -> bool {    
    scan_attribute_item_for_identifier(node, source, |s| s == "cfg_attr")
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

fn scan_attribute_item_for_features<'a>(
    node: tree_sitter::Node, 
    source: &'a str, 
    sd: &mut HashMap<String, u32>, 
    visited: &mut HashSet<&'a str>
) -> u32 {
    let mut sum = 0;

    let mut cursor = node.walk();
    let identifiers = node.children(&mut cursor)
        .filter(|c| c.kind() == "identifier" && c.utf8_text(source.as_bytes()) == Ok("feature"));

    for identifier in identifiers {
        let Some(string_literal) = identifier.next_sibling().and_then(|n| n.next_sibling()) else { continue };
        let Some(string_content) = find_child(string_literal, "string_content") else { continue };
        let feature = string_content.utf8_text(source.as_bytes()).expect("source is utf8");
        if visited.insert(feature) {
            sd.entry(feature.to_string())
                .and_modify(|c| *c += 1)
                .or_insert(1);
            sum += 1;
        }
    }

    let mut cursor = node.walk();
    let children = node.children(&mut cursor);
    sum += children
        .map(|child| scan_attribute_item_for_features(child, source, sd, visited))
        .sum::<u32>();

    sum
}

fn find_child<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    let mut children = node.children(&mut cursor);
    children.find(|c| c.kind() == kind)
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
        assert_eq!(result.sd_mean, 4.0);
        assert_eq!(result.sd_dev, 3.0);
        assert_eq!(result.td_mean, 1.0);
        assert_eq!(result.td_dev, 0.0);
    }
}