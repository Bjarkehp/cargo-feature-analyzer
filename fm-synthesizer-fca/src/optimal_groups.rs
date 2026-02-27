use std::iter::successors;

use itertools::MultiUnzip;

use crate::min_max::MinMaxExt;

type Mask = u32;
type Cost = f64;

/// Finds the optimal set of groups which (locally) minimizes the number of configurations for a feature model.
/// 
/// The algorithm uses dynamic programming with a bit mask as the key.
/// First, the cost of all possible groups are calculated, and stored in the cost vector.
/// Once the costs are calculated, the dp table can be constructed and populated.
/// The core of the algorithm is the recurrence: dp(S) = mininimize for all subsets G in S : dp(S \ G) * cost(G).
pub fn find(n: usize, assignments: &[Mask], weight: impl Fn(usize) -> Cost) -> impl Iterator<Item = (Vec<usize>, usize, usize)> {
    let full: Mask = 1 << n;
    let (cost, group_min, group_max): (Vec<Cost>, Vec<u32>, Vec<u32>) = (0..full)
        .map(|group| (group, group_cardinality(group, assignments)))
        .map(|(group, (min, max))| (group_cost(mask_indices(group), group.count_ones() as usize, min, max, &weight), min, max))
        .multiunzip();

    let mut dp = vec![Cost::MAX; full as usize];
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

/// Calcuates the cost of a specified group, its cardinality 
/// and a weight function for every item in the group.
pub fn group_cost(group: impl Iterator<Item = usize>, n: usize, min: u32, max: u32, weight: impl Fn(usize) -> f64) -> f64 {
    let mut dp = vec![0.0; n + 1];
    dp[0] = 1.0;

    for i in group {
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