use std::{cmp::{max, min}, collections::HashMap, iter::successors};

use petgraph::{Direction, graph::{DiGraph, NodeIndex}};

use crate::concept::Concept;

type Mask = u32;
type Cost = u32;

/// Finds the optimal set of groups which (locally) minimizes the number of configurations for a feature model.
/// 
/// The algorithm uses dynamic programming with a bit mask as the key.
/// First, the problem is reduces the input into bit masks which can be effeciently used later.
/// Then the cost of all possible groups are calculated, and stored in the cost vector.
/// Once the costs are calculated, the dp table can be constructed an populated.
/// The core of the algorithm is the recurrence: dp(S) = mininimize for all subsets G in S : dp(S \ G) * cost(G).
pub fn find<'a>(ac_poset: &DiGraph<Concept, ()>, node: NodeIndex, tree_neighbors: &'a [NodeIndex]) -> impl Iterator<Item = Vec<NodeIndex>> + 'a {
    let has_cross_tree_neighbors = tree_neighbors.len() != ac_poset.edges_directed(node, Direction::Incoming).count();
    let empty_assignment = !ac_poset[node].configurations.is_empty() || has_cross_tree_neighbors;
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

    if empty_assignment {
        assignments.push(0);
    }

    let full: Mask = 1 << n;
    let mut cost = vec![0; full as usize];
    cost[0] = 1;

    for s in 1..full {
        let mut min_c = n;
        let mut max_c = 0;
        for &a in assignment_map.values() {
            let c = (s & a).count_ones() as Mask;
            min_c = min(min_c, c);
            max_c = max(max_c, c);
        }
        let size = s.count_ones() as Mask;
        cost[s as usize] = (min_c..=max_c)
            .map(|k| n_choose_k(size, k))
            .sum();
    }

    let mut dp = vec![Cost::MAX; full as usize];
    let mut choice = vec![0; full as usize];
    dp[0] = 1;

    for s in 1..full {
        for group in enumerate_groups(s) {
            let val = dp[(s ^ group) as usize] * cost[group as usize];
            if val < dp[s as usize] {
                dp[s as usize] = val;
                choice[s as usize] = group;
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

    groups.into_iter().map(|group_mask| {
        (0..Mask::BITS)
            .filter(|i| group_mask & (1 << i) != 0)
            .map(|i| tree_neighbors[i as usize])
            .collect::<Vec<_>>()
    })
}

/// Returns an iterator of bitmasks representing suubsets of the given bitmask.
/// 
/// To reduce symmetry, it is assumed that 
/// the Least Significant Bit (LSB) is always in the subset.
fn enumerate_groups(s: u32) -> impl Iterator<Item = u32> {
    let lsb = s & (!s + 1);
    successors(Some(s), move |&g| {
            if g != 0 {
                Some((g - 1) & s)    
            } else {
                None
            }
        })
        .filter(move |g| g & lsb != 0)
}

/// Calculates n choose k of two u32's
fn n_choose_k(n: u32, k: u32) -> u32 {
    (1..=k).map(|i| (n - k + i) / i)
        .product::<u32>()
}