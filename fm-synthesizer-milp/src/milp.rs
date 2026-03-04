use std::collections::{HashMap, HashSet};

use configuration_scraper::configuration::Configuration;
use good_lp::{constraint, variable, Constraint, Expression, ProblemVariables, Variable};

use crate::util::{binary, make_variables, n_choose_k, natural, VariableMap};

use itertools::{chain, iproduct as p};

/// Stores all variables regarding the MILP formulation.
pub struct FeatureModelMilp {
    pub problem: ProblemVariables,
    
    pub rows: usize,
    pub rows_f: f64,
    pub columns: usize,
    pub columns_f: f64,

    pub context: HashSet<(usize, usize)>,
    pub log_choice_table: HashMap<(usize, usize, usize), f64>,

    pub feature_group_relation: VariableMap<(usize, usize)>,
    pub config_group_count: VariableMap<(usize, usize)>,

    pub cardinality_min: VariableMap<usize>,
    pub cardinality_max: VariableMap<usize>,
    pub group_size: VariableMap<usize>,

    pub cardinality_min_binary: VariableMap<(usize, usize)>,
    pub cardinality_max_binary: VariableMap<(usize, usize)>,
    pub group_size_binary: VariableMap<(usize, usize)>,

    pub group_has_feature: VariableMap<usize>,
    pub group_count: Variable,

    pub is_mandatory: VariableMap<usize>,
    pub is_optional: VariableMap<usize>,

    pub feature_parent_relation: VariableMap<(usize, usize)>,
    pub group_parent_relation: VariableMap<(usize, usize)>,
    pub config_group_relation: VariableMap<(usize, usize)>,
    pub flow: VariableMap<(usize, usize)>,

    pub group_min_max_size: VariableMap<(usize, usize, usize, usize)>,
}

/// Creates the problem variables for MILP formulation.
pub fn create_problem(features: &[&str], configurations: &[Configuration]) -> FeatureModelMilp {
    let mut problem = ProblemVariables::new();
    let rows = configurations.len();
    let columns = features.len();

    let context = p!(0..rows, 0..columns)
        .filter(|&(i, j)| configurations[i].features.get(features[j]) == Some(&true))
        .collect::<HashSet<_>>();

    let n_choose_k_table = p!(0..columns, 0..columns)
        .filter(|&(n, k)| n >= k)
        .map(|(n, k)| ((n, k), n_choose_k(n as u64, k as u64) as f64))
        .collect::<HashMap<_, _>>();

    let log_choice_table = p!(0..columns, 0..columns, 0..columns)
        .filter(|&(min, max, n) | min <= max && max <= n)
        .map(|key @ (min, max, n)| {
            let sum = (min..=max)
                .map(|k| n_choose_k_table[&(n, k)])
                .sum::<f64>();
            let log_sum = sum.ln();
            (key, log_sum)
        })
        .collect::<HashMap<_, _>>();

    FeatureModelMilp { 
        rows, 
        rows_f: rows as f64, 
        columns, 
        columns_f: columns as f64, 
        context,
        log_choice_table,
        cardinality_min: make_variables(&mut problem, 0..columns, natural), 
        cardinality_max: make_variables(&mut problem, 0..columns, natural),
        cardinality_min_binary: make_variables(&mut problem, p!(0..columns, 0..columns), binary),
        cardinality_max_binary: make_variables(&mut problem, p!(0..columns, 0..columns), binary),
        feature_group_relation: make_variables(&mut problem, p!(0..columns, 0..columns), binary), 
        config_group_count: make_variables(&mut problem, p!(0..rows, 0..columns), natural),
        group_size: make_variables(&mut problem, 0..columns, natural),
        group_size_binary: make_variables(&mut problem, p!(0..columns, 0..columns), binary),
        group_has_feature: make_variables(&mut problem, 0..columns, binary), 
        group_count: problem.add(variable().integer().min(1)), 
        is_mandatory: make_variables(&mut problem, 0..columns, binary),
        is_optional: make_variables(&mut problem, 0..columns, binary),
        feature_parent_relation: make_variables(&mut problem, p!(1..columns, 0..columns), binary),
        group_parent_relation: make_variables(&mut problem, p!(1..columns, 0..columns), binary),
        config_group_relation: make_variables(&mut problem, p!(0..rows, 0..columns), binary),
        flow: make_variables(&mut problem, p!(0..columns, 0..columns), |_| variable().min(0)),
        group_min_max_size: make_variables(&mut problem, p!(0..columns, 0..columns, 0..columns, 0..columns), binary),
        problem, 
    }
}

/// Creates the MILP objective expression as a reward (should be maximized).
pub fn create_objective(milp: &FeatureModelMilp) -> Expression {
    group_choice_objective(milp)
}

/// Creates the constraints on the variables in the formulation.
pub fn create_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    chain!(
        feature_group_constraints(milp),
        config_group_count_constraints(milp),
        cardinality_min_constraints(milp),
        cardinality_max_constraints(milp),
        std::iter::once(root_in_group_0_constraint(milp)),
        std::iter::once(root_group_size_constraints(milp)),
        group_size_constraints(milp),
        group_not_empty_constraints(milp),
        std::iter::once(group_count_constraint(milp)),
        is_mandatory_constraints(milp),
        is_optional_constraints(milp),
        config_parent_relation_constraints(milp),
        feature_parent_same_as_group_constraints(milp),
        feature_depends_on_parent_constraints(milp),
        std::iter::once(root_flow_constraint(milp)),
        capacity_constraints(milp),
        flow_constraints(milp),
        one_parent_per_feature_constraints(milp),
        one_parent_per_group_constraints(milp),
        group_symmetry_constraints(milp),

        cardinality_min_binary_constraints(milp),
        cardinality_min_binary_select_1_constraints(milp),
        cardinality_max_binary_constraints(milp),
        cardinality_max_binary_select_1_constraints(milp),
        group_size_binary_constraints(milp),
        group_size_binary_select_1_constraints(milp),
        group_min_max_size_constraints(milp),

        group_dependency_constraints(milp),
        multiple_mandatory_groups_constraints(milp),
        chained_mandatory_groups_constraints(milp),
        multiple_optional_groups_constraints(milp),
    )
}

/// Calculates the approximate number of possible configurations as a cost,
/// the number of mandatory groups as a reward 
/// and the number of groups as a reward.
/// 
/// The approximate number of possible configurations is calculated using a table of n_choose_k values.
/// The final value in the table has the natural logarithm applied.
/// This is because summing the values results in the same order as finding the product of the values without the logarithm:
/// Πx ~ Σ ln x
fn group_choice_objective(milp: &FeatureModelMilp) -> Expression {
    let choice_cost = p!(0..milp.columns, 0..milp.columns, 0..milp.columns, 0..milp.columns)
        .filter(|&(_group, min, max, size)| min <= max && max <= size)
        .map(|(group, min, max, size)| milp.log_choice_table[&(min, max, size)] * milp.group_min_max_size[&(group, min, max, size)])
        .sum::<Expression>();
    let mandatory_reward = (0..milp.columns)
        .map(|group| milp.is_mandatory[&group])
        .sum::<Expression>();
    let group_count_reward = 0.1 * milp.group_count;
    mandatory_reward + group_count_reward - choice_cost
}

/// Each feature must be part of exactly one group
fn feature_group_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|feature| {
        let sum = (0..milp.columns)
            .map(|group| milp.feature_group_relation[&(feature, group)])
            .sum::<Expression>();
        constraint!(sum == 1)
    })
}

/// Determines the number of active features within some group for some config.
fn config_group_count_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(0..milp.rows, 0..milp.columns).map(|(config, group)| {
        let sum = (0..milp.columns)
            .filter(|&feature| milp.context.contains(&(config, feature)))
            .map(|feature| milp.feature_group_relation[&(feature, group)])
            .sum::<Expression>();
        constraint!(milp.config_group_count[&(config, group)] == sum)
    })
}

/// For every config and group, create a constraint 
/// such that cardinality_min of the group must be no more 
/// than the number of active features within the group for the config.
/// The constraint only applies if the parent of the group is enabled in the config.
fn cardinality_min_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(0..milp.rows, 0..milp.columns).map(|(config, group)| {
        let min = milp.cardinality_min[&group];
        let count = milp.config_group_count[&(config, group)];
        let parent_enabled = milp.config_group_relation[&(config, group)];
        constraint!(min <= count + milp.columns_f * (1 - parent_enabled))
    })
}

/// For every config and group, create a constraint 
/// such that cardinality_max of the group must be atleast 
/// the number of active features within the group for the config.
/// The constraint only applies if the parent of the group is enabled in the config.
fn cardinality_max_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(0..milp.rows, 0..milp.columns).map(|(config, group)| {
        let max = milp.cardinality_max[&group];
        let count = milp.config_group_count[&(config, group)];
        let parent_enabled = milp.config_group_relation[&(config, group)];
        constraint!(max >= count - milp.columns_f * (1 - parent_enabled))
    })
}

/// Only the root feature can be in the root group.
fn root_group_size_constraints(milp: &FeatureModelMilp) -> Constraint {
    constraint!(milp.group_size[&0] == 1)
}

/// Determines the size of each group.
fn group_size_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|group| {
        let sum = (0..milp.columns)
            .map(|feature| milp.feature_group_relation[&(feature, group)])
            .sum::<Expression>();
        constraint!(milp.group_size[&group] == sum)
    })
}

/// Determines whether a group has any features. 
fn group_not_empty_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).flat_map(|group| [
        constraint!(milp.group_has_feature[&group] <= milp.group_size[&group]),
        constraint!(milp.group_has_feature[&group] * milp.columns_f >= milp.group_size[&group]),
    ])
}

/// Determines the number of groups used for the current solution.
fn group_count_constraint(milp: &FeatureModelMilp) -> Constraint {
    let sum = (0..milp.columns)
        .map(|group| milp.group_has_feature[&group])
        .sum::<Expression>();
    constraint!(milp.group_count == sum)
}

/// Determines if a group is considered a mandatory group.
/// If a group is mandatory, then the cardinality is [size..size], 
/// i.e. if the parent is chosen, then all features within the group must be chosen as well.
fn is_mandatory_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).flat_map(|group| [
        // group_size[k] == cardinality_min[k]
        constraint!(milp.group_size[&group] - milp.cardinality_min[&group] <= milp.columns_f * (1 - milp.is_mandatory[&group])),
        // group_size[k] == cardinality_max[k]
        constraint!(milp.group_size[&group] - milp.cardinality_max[&group] <= milp.columns_f * (1 - milp.is_mandatory[&group])),
        // A group is not mandatory if it is empty
        constraint!(milp.group_has_feature[&group] >= milp.is_mandatory[&group]),
        // If all conditions hold, it must be mandatory
        constraint!(milp.group_size[&group] - milp.cardinality_min[&group] + milp.group_size[&group] - milp.cardinality_max[&group] - milp.group_has_feature[&group] >= -milp.is_mandatory[&group])
    ])
}

/// Determines if a group is considered an optional group.
/// If a group is optional, then the cardinality is [0..size], 
/// i.e. there is no restriction on how many features can be enabled in the group.
fn is_optional_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).flat_map(|group| [
        constraint!(milp.cardinality_min[&group] <=  milp.columns_f * (1 - milp.is_optional[&group])),
        constraint!(milp.group_size[&group] - milp.cardinality_max[&group] <= milp.columns_f * (1 - milp.is_optional[&group])),
        constraint!(milp.group_size[&group] - milp.cardinality_max[&group] + milp.cardinality_min[&group] - milp.group_has_feature[&group] >= -milp.is_optional[&group]),
    ])
}

/// Determines if the parent of a specific group is enabled in a specific config.
fn config_parent_relation_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(0..milp.rows, 1..milp.columns).map(|(config, group)| {
        let sum = (0..milp.columns)
            .filter(|&parent| milp.context.contains(&(config, parent)))
            .map(|parent| milp.group_parent_relation[&(group, parent)])
            .sum::<Expression>();
        constraint!(milp.config_group_relation[&(config, group)] == sum)
    })
}

/// The root feature must be in group 0.
fn root_in_group_0_constraint(milp: &FeatureModelMilp) -> Constraint {
    constraint!(milp.feature_group_relation[&(0, 0)] == 1)
}

/// The parent of a feature must be the same as the parent of its group.
fn feature_parent_same_as_group_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(1..milp.columns, 1..milp.columns, 0..milp.columns).map(|(feature, group, parent)| {
        constraint!(milp.feature_parent_relation[&(feature, parent)] >= milp.feature_group_relation[&(feature, group)] + milp.group_parent_relation[&(group, parent)] - 1)
    })
}

/// For all configurations, a parent must be enabled when a child is enabled.
fn feature_depends_on_parent_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    let not_dependencies = p!(1..milp.columns, 0..milp.columns).filter(|&(child, parent)| {
        !(0..milp.rows).filter(|&config| milp.context.contains(&(config, child)))
            .all(|config| milp.context.contains(&(config, parent)))
    });

    not_dependencies.map(|(feature, parent)| constraint!(milp.feature_parent_relation[&(feature, parent)] == 0))
}

/// The root feature generates |features| - 1 flow, 
/// so that each feature excluding root can consume one unit of flow.
fn root_flow_constraint(milp: &FeatureModelMilp) -> Constraint {
    let out_flow = (0..milp.columns)
        .map(|child| milp.flow[&(child, 0)])
        .sum::<Expression>();
    constraint!(out_flow == milp.columns_f - 1.0)
}

/// Flow can only pass through two features if one is a parent and the other is a child.
fn capacity_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(1..milp.columns, 0..milp.columns).map(|(child, parent)| {
        constraint!(milp.flow[&(child, parent)] <= (milp.columns_f - 1.0) * milp.feature_parent_relation[&(child, parent)])
    })
}

/// For each feature, the amount of flow going in must be equal to the amount of flow going out minus 1.
/// In other words, one unit of flow is consumed for each feature.
fn flow_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (1..milp.columns).map(|feature| {
        let in_flow = (0..milp.columns)
            .map(|parent| milp.flow[&(feature, parent)])
            .sum::<Expression>();
        let out_flow = (0..milp.columns)
            .map(|child| milp.flow[&(child, feature)])
            .sum::<Expression>();
        constraint!(in_flow == out_flow + 1)
    })
}

/// Each feature must only have one parent.
fn one_parent_per_feature_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (1..milp.columns).map(|feature| {
        let sum = (0..milp.columns)
            .map(|parent| milp.feature_parent_relation[&(feature, parent)])
            .sum::<Expression>();
        constraint!(sum == 1)
    })
}

/// Each group must only have one parent.
fn one_parent_per_group_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (1..milp.columns).map(|group| {
        let sum = (0..milp.columns)
            .map(|parent| milp.group_parent_relation[&(group, parent)])
            .sum::<Expression>();
        constraint!(sum == 1)
    })
}

/// For each feature, if it is group n, then every group m < n
/// must contain a previous feature (according to the order given when constructing the formulation).
/// This simplifies the output, so that the group numbers are ordered.
/// The constraints also reduce symmetry, leading to faster solves.
fn group_symmetry_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(0..milp.columns, 1..milp.columns).map(|(feature, group)| {
        let features_in_previous_group = (0..feature)
            .map(|feature_before| milp.feature_group_relation[&(feature_before, group - 1)])
            .sum::<Expression>();
        constraint!(features_in_previous_group - milp.feature_group_relation[&(feature, group)] >= 0)
    })
}

/// Chooses the correct binary variable based on cardinality_min.
fn cardinality_min_binary_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|group| {
        let sum = (0..milp.columns)
            .map(|i| i as f64 * milp.cardinality_min_binary[&(group, i)])
            .sum::<Expression>();
        constraint!(milp.cardinality_min[&group] == sum)
    })
}

/// Only one binary variable can be chosen.
fn cardinality_min_binary_select_1_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|group| {
        let sum = (0..milp.columns)
            .map(|i| milp.cardinality_min_binary[&(group, i)])
            .sum::<Expression>();
        constraint!(sum == 1)
    })
}

/// Chooses the correct binary variable based on cardinality_max.
fn cardinality_max_binary_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|group| {
        let sum = (0..milp.columns)
            .map(|i| i as f64 * milp.cardinality_max_binary[&(group, i)])
            .sum::<Expression>();
        constraint!(milp.cardinality_max[&group] == sum)
    })
}

/// Only one binary variable can be chosen.
fn cardinality_max_binary_select_1_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|group| {
        let sum = (0..milp.columns)
            .map(|i| milp.cardinality_max_binary[&(group, i)])
            .sum::<Expression>();
        constraint!(sum == 1)
    })
}

/// Chooses the correct binary variable based on group_size.
fn group_size_binary_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|group| {
        let sum = (0..milp.columns)
            .map(|i| i as f64 * milp.group_size_binary[&(group, i)])
            .sum::<Expression>();
        constraint!(milp.group_size[&group] == sum)
    })
}

/// Only one binary variable can be chosen.
fn group_size_binary_select_1_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|group| {
        let sum = (0..milp.columns)
            .map(|size| milp.group_size_binary[&(group, size)])
            .sum::<Expression>();
        constraint!(sum == 1)
    })
}

/// Chooses the correct (group, min, max, size) binary variable.
fn group_min_max_size_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(0..milp.columns, 0..milp.columns, 0..milp.columns, 0..milp.columns).flat_map(|(group, min, max, n)| [
        constraint!(milp.group_min_max_size[&(group, min, max, n)] <= milp.cardinality_min_binary[&(group, min)]),
        constraint!(milp.group_min_max_size[&(group, min, max, n)] <= milp.cardinality_max_binary[&(group, max)]),
        constraint!(milp.group_min_max_size[&(group, min, max, n)] <= milp.group_size_binary[&(group, n)]),
        constraint!(milp.group_min_max_size[&(group, min, max, n)] >= milp.cardinality_min_binary[&(group, min)] + milp.cardinality_max_binary[&(group, max)] + milp.group_size_binary[&(group, n)] - 2),
    ])
}  

/// Unless the group is mandatory, no features within the group can depend on each other.
/// This forces the solver to create nested groups, which lowers the number of possible configurations.
fn group_dependency_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    let dependencies = p!(0..milp.columns, 0..milp.columns)
        .filter(|&(feature1, feature2)| feature1 != feature2)
        .filter(|&(feature1, feature2)| {
            (0..milp.rows)
                .filter(|&config| milp.context.contains(&(config, feature1)))
                .all(|config| milp.context.contains(&(config, feature2)))
        });
    
    p!(dependencies, 0..milp.columns).map(|((feature1, feature2), group)| 
        constraint!(milp.feature_group_relation[&(feature1, group)] + milp.feature_group_relation[&(feature2, group)] - milp.is_mandatory[&group] <= 1)
    )
}

/// There cannot be multiple mandatory groups with the same parent.
/// This forces the solver to merge the two groups into one mandatory group.
/// This also reduces symmetry.
fn multiple_mandatory_groups_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(1..milp.columns, 1..milp.columns, 0..milp.columns)
        .filter(|&(group1, group2, _parent)| group1 > group2)
        .map(|(group1, group2, parent)| {
            constraint!(
                milp.group_parent_relation[&(group1, parent)] + 
                milp.group_parent_relation[&(group2, parent)] + 
                milp.is_mandatory[&group1] + 
                milp.is_mandatory[&group2] <= 3
            )
        })
}

/// A mandatory group cannot have a parent within a mandatory group.
/// This forces the solver to merge the two groups into one mandatory group.
/// This also reduces symmetry.
fn chained_mandatory_groups_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(1..milp.columns, 1..milp.columns, 1..milp.columns).map(|(parent_group, child_group, parent)| {
        constraint!(
            milp.feature_group_relation[&(parent, parent_group)] + 
            milp.group_parent_relation[&(child_group, parent)] + 
            milp.is_mandatory[&parent_group] + 
            milp.is_mandatory[&child_group] <= 3
        )
    })
}

/// There cannot be two optional groups with the same parent.
/// This forces the solver to merge the two groups into one optional group.
/// This also reduces symmetry.
fn multiple_optional_groups_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(1..milp.columns, 1..milp.columns, 0..milp.columns)
        .filter(|&(group1, group2, _parent)| group1 > group2)
        .map(|(group1, group2, parent)| {
            constraint!(
                milp.group_parent_relation[&(group1, parent)] + 
                milp.group_parent_relation[&(group2, parent)] + 
                milp.is_optional[&group1] + 
                milp.is_optional[&group2] <= 3
            )
        })
}