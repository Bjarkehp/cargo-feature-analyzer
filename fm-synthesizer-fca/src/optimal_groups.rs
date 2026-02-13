use std::{collections::HashMap, iter::successors};

use itertools::MultiUnzip;
use petgraph::{Direction, graph::{DiGraph, NodeIndex}};

use crate::{binomial::n_choose_k_u32, concept::Concept, min_max::MinMaxExt};

type Mask = u32;
type Cost = u32;

/// Finds the optimal set of groups which (locally) minimizes the number of configurations for a feature model.
/// 
/// The algorithm uses dynamic programming with a bit mask as the key.
/// First, the problem is reduces the input into bit masks which can be effeciently used later.
/// Then the cost of all possible groups are calculated, and stored in the cost vector.
/// Once the costs are calculated, the dp table can be constructed an populated.
/// The core of the algorithm is the recurrence: dp(S) = mininimize for all subsets G in S : dp(S \ G) * cost(G).
pub fn find<'a>(ac_poset: &DiGraph<Concept, ()>, node: NodeIndex, tree_neighbors: &'a [NodeIndex]) -> impl Iterator<Item = (Vec<NodeIndex>, usize, usize)> + 'a {
    let n = tree_neighbors.len() as Mask;

    let tree_neighbors_reverse_map = (0..n)
        .map(|i| (tree_neighbors[i as usize], i))
        .collect::<HashMap<_, _>>();

    let mut assignment_map: HashMap<String, Mask> = HashMap::new();
    for &node in tree_neighbors.iter() {
        for config in ac_poset[node].inherited_configurations.iter() {
            assignment_map.entry(config.to_string())
                .and_modify(|x| *x |= 1 << tree_neighbors_reverse_map[&node])
                .or_insert(1 << tree_neighbors_reverse_map[&node]);
        }
    }

    let mut assignments = assignment_map.values()
        .cloned()
        .collect::<Vec<_>>();

    let has_cross_tree_neighbors = tree_neighbors.len() != ac_poset.edges_directed(node, Direction::Incoming).count();
    let empty_assignment = !ac_poset[node].configurations.is_empty() || has_cross_tree_neighbors;
    if empty_assignment {
        assignments.push(0);
    }

    let full: Mask = 1 << n;
    let (cost, group_min, group_max): (Vec<u32>, Vec<u32>, Vec<u32>) = (0..full)
        .map(|group| (group, group_cardinality(group, &assignments)))
        .map(|(group, (min, max))| (group_cost(group.count_ones(), min, max), min, max))
        .multiunzip();

    let mut dp = vec![Cost::MAX; full as usize];
    let mut choice = vec![0; full as usize];
    dp[0] = 1;

    for subset in 1..full {
        for group in enumerate_groups(subset) {
            let rest = (subset ^ group) as usize;
            let val = dp[rest] * cost[group as usize];
            if val < dp[subset as usize] {
                dp[subset as usize] = val;
                choice[subset as usize] = group;
            }
        }
    }

    let mut groups = vec![];
    let mut mask = full - 1;
    while mask != 0 {
        let sub = choice[mask as usize];
        groups.push(sub);
        mask ^= sub;
    };

    groups.into_iter().map(move |group| {
        let nodes = (0..Mask::BITS)
            .filter(|i| group & (1 << i) != 0)
            .map(|i| tree_neighbors[i as usize])
            .collect::<Vec<_>>();
        
        let min = group_min[group as usize] as usize;
        let max = group_max[group as usize] as usize;
        (nodes, min, max)
    })
}

pub fn find2(n: usize, assignments: &[Mask], weight: impl Fn(usize) -> f64) -> impl Iterator<Item = (Vec<usize>, usize, usize)> {
    let full: Mask = 1 << n;
    let (cost, group_min, group_max): (Vec<f64>, Vec<u32>, Vec<u32>) = (0..full)
        .map(|group| (group, group_cardinality(group, assignments)))
        .map(|(group, (min, max))| (group_cost2(group, min, max, &weight), min, max))
        //.map(|(group, (min, max))| (group_cost(group.count_ones(), min, max) as f64, min, max))
        .multiunzip();

    let mut dp = vec![f64::MAX; full as usize];
    let mut choice = vec![0; full as usize];
    dp[0] = 1.0;

    for subset in 1..full {
        for group in enumerate_groups(subset) {
            let rest = (subset ^ group) as usize;
            let val = dp[rest] * cost[group as usize];
            if val < dp[subset as usize] {
                dp[subset as usize] = val;
                choice[subset as usize] = group;
            }
        }
    }

    let mut groups = vec![];
    let mut mask = full - 1;
    while mask != 0 {
        let sub = choice[mask as usize];
        groups.push(sub);
        mask ^= sub;
    };

    groups.into_iter().map(move |group| {
        let nodes = (0..Mask::BITS)
            .filter(|i| group & (1 << i) != 0)
            .map(|i| i as usize)
            .collect::<Vec<_>>();
        
        let min = group_min[group as usize] as usize;
        let max = group_max[group as usize] as usize;
        (nodes, min, max)
    })
}

/// Calculates the minimum and maximum cardinality of a group
/// with regards to a set of assignments.
fn group_cardinality(group: Mask, assignments: &[Mask]) -> (u32, u32) {
    assignments.iter()
        .map(|assignment| (group & assignment).count_ones() as Mask)
        .min_max()
        .expect("There is atleast one assignement")
}

/// Calculates the cost of a group with the specified size (n), 
/// minimum cardinality and maximum cardinality.
fn group_cost(n: u32, min: u32, max: u32) -> u32 {
    (min..=max)
        .map(|k| n_choose_k_u32(n, k))
        .sum()
}

fn group_cost2(group: Mask, min: u32, max: u32, weight: impl Fn(usize) -> f64) -> f64 {
    if group == 0 {
        return 1.0;
    }

    let n = group.count_ones() as usize;

    let mut dp = vec![0.0; n + 1];
    dp[0] = 1.0;

    for i in mask_indices(group) {
        for k in (1..=n).rev() {
            dp[k] += dp[k - 1] * weight(i);
        }
    }

    (min..=max)
        .map(|k| dp[k as usize])
        .sum()
}

/// Returns an iterator of bitmasks representing suubsets of the given bitmask.
/// 
/// To reduce symmetry, it is assumed that 
/// the Least Significant Bit (LSB) is always in the subset.
fn enumerate_groups(s: u32) -> impl Iterator<Item = u32> {
    let lsb = s & (!s + 1);
    let groups = successors(Some(s), move |&g| {
        if g != 0 {
            Some((g - 1) & s)    
        } else {
            None
        }
    });
    
    groups.filter(move |g| g & lsb != 0)
}

fn mask_indices(mask: Mask) -> impl Iterator<Item = usize> {
    (0..Mask::BITS)
        .filter(move |i| mask & (1 << i) != 0)
        .map(|i| i as usize)
}