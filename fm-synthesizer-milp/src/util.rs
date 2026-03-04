use std::collections::HashMap;

use good_lp::{variable, ProblemVariables, Variable, VariableDefinition};

pub type VariableMap<K> = HashMap<K, Variable>;

pub fn make_variables<K: std::hash::Hash + Eq>(
    problem: &mut ProblemVariables,
    keys: impl Iterator<Item = K>, 
    definition: impl Fn(&K) -> VariableDefinition
) -> HashMap<K, Variable> {
    keys.map(|k| (problem.add(definition(&k)), k))
        .map(|(v, k)| (k, v)) // The above line needs to borrow k before moving it, hence the second map
        .collect()
}

pub fn binary<K>(_k: &K) -> VariableDefinition {
    variable().binary()
}

pub fn natural<K>(_k: &K) -> VariableDefinition {
    variable().integer().min(0)
}

pub fn continous<K>(_k: &K) -> VariableDefinition {
    variable()
}

pub fn n_choose_k(n: u64, k: u64) -> u64 {
    (1..=k).map(|i| (n - k + i) / i).product::<u64>()
}