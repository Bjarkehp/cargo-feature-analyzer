use std::collections::HashSet;

use configuration::Configuration;
use good_lp::{constraint, variable, Constraint, Expression, IntoAffineExpression, ProblemVariables, Variable};

use crate::util::{binary, float, make_variables, natural, VariableMap};

use itertools::{chain, iproduct as p};

const MANDATORY_BASE: f64 = 8.0;
const ALTERNATIVE_BASE: f64 = 4.0;
const OR_BASE: f64 = 2.0;
const GROUP_COUNT_BASE: f64 = 1.0;

pub struct FeatureModelMilp {
    pub problem: ProblemVariables,
    
    pub rows: usize,
    pub rows_f: f64,
    pub columns: usize,
    pub columns_f: f64,

    pub context: HashSet<(usize, usize)>,

    pub cardinality_min: VariableMap<usize>,
    pub cardinality_max: VariableMap<usize>,

    pub feature_group_relation: VariableMap<(usize, usize)>,
    pub config_group_count: VariableMap<(usize, usize)>,

    pub group_size: VariableMap<usize>,
    pub group_not_empty: VariableMap<usize>,
    pub group_count: Variable,

    pub is_mandatory: VariableMap<usize>,
    pub mandatory_rewards: VariableMap<usize>,

    pub is_alternative: VariableMap<usize>,
    pub alternative_rewards: VariableMap<usize>,

    pub is_or: VariableMap<usize>,
    pub or_rewards: VariableMap<usize>,

    pub feature_parent_relation: VariableMap<(usize, usize)>,
    pub group_parent_relation: VariableMap<(usize, usize)>,
    pub config_group_relation: VariableMap<(usize, usize)>,
    pub feature_depth: VariableMap<usize>,
}

pub fn create_problem(features: &[&str], configurations: &[Configuration]) -> FeatureModelMilp {
    let mut problem = ProblemVariables::new();
    let rows = configurations.len();
    let columns = features.len();

    let context = p!(0..rows, 0..columns)
        .filter(|&(i, j)| configurations[i].features().contains(features[j]))
        .collect::<HashSet<_>>();

    FeatureModelMilp { 
        rows, 
        rows_f: rows as f64, 
        columns, 
        columns_f: columns as f64, 
        context,
        cardinality_min: make_variables(&mut problem, 0..columns, natural), 
        cardinality_max: make_variables(&mut problem, 0..columns, natural),
        feature_group_relation: make_variables(&mut problem, p!(0..columns, 0..columns), binary), 
        config_group_count: make_variables(&mut problem, p!(0..rows, 0..columns), natural),
        group_size: make_variables(&mut problem, 0..columns, natural),
        group_not_empty: make_variables(&mut problem, 0..columns, binary), 
        group_count: problem.add(variable().integer().min(1)), 
        is_mandatory: make_variables(&mut problem, 0..columns, binary),
        mandatory_rewards: make_variables(&mut problem, 0..columns, float),
        is_alternative: make_variables(&mut problem, 0..columns, binary),
        alternative_rewards: make_variables(&mut problem, 0..columns, float),
        is_or: make_variables(&mut problem, 0..columns, binary),
        or_rewards: make_variables(&mut problem, 0..columns, float),
        feature_parent_relation: make_variables(&mut problem, p!(1..columns, 0..columns), binary),
        group_parent_relation: make_variables(&mut problem, p!(1..columns, 0..columns), binary),
        config_group_relation: make_variables(&mut problem, p!(0..rows, 0..columns), binary),
        feature_depth: make_variables(&mut problem, 0..columns, natural), 
        problem, 
    }
}

pub fn create_objective(milp: &FeatureModelMilp) -> Expression {
    let mandatory_rewards = (0..milp.columns)
        .map(|group| milp.mandatory_rewards[&group])
        .sum::<Expression>();
    let alternative_rewards = (0..milp.columns)
        .map(|group| milp.alternative_rewards[&group])
        .sum::<Expression>();
    let or_rewards = (0..milp.columns)
        .map(|group| milp.or_rewards[&group])
        .sum::<Expression>();
    let cardinality_cost = (0..milp.columns)
        .map(|group| milp.cardinality_max[&group] - milp.cardinality_min[&group])
        .sum::<Expression>();
    let group_count_cost = GROUP_COUNT_BASE * milp.group_count;

    mandatory_rewards + 
    alternative_rewards + 
    or_rewards - 
    cardinality_cost -
    group_count_cost
}

pub fn create_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    chain!(
        feature_group_constraints(milp),
        config_group_count_constraints(milp),
        cardinality_min_constraints(milp),
        cardinality_max_constraints(milp),
        cardinality_root_constraints(milp),
        cardinality_min_max_constraints(milp),
        group_size_constraints(milp),
        group_not_empty_constraints(milp),
        group_symmetry_constraints(milp),
        std::iter::once(group_count_constraint(milp)),
        is_mandatory_constraints(milp),
        mandatory_rewards_constraints(milp),
        is_alternative_constraints(milp),
        alternative_rewards_constraints(milp),
        is_or_constraints(milp),
        or_rewards_constraints(milp),
        group_types_constraints(milp),
        config_parent_relation_constraints(milp),
        std::iter::once(root_in_group_0_constraint(milp)),
        std::iter::once(group_0_size_1_constraint(milp)),
        feature_parent_same_as_group_constraints(milp),
        feature_depends_on_parent_constraints(milp),
        std::iter::once(root_depth_0_constraint(milp)),
        parent_depth_above_child_constraints(milp),
        one_parent_per_feature_constraints(milp),
        one_parent_per_group_constraints(milp),
    )
}

fn feature_group_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|feature| {
        let sum = (0..milp.columns)
            .map(|group| milp.feature_group_relation[&(feature, group)])
            .sum::<Expression>();
        constraint!(sum == 1)
    })
}

fn config_group_count_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(0..milp.rows, 0..milp.columns).map(|(config, group)| {
        let sum = (0..milp.columns).map(|feature| {
            if milp.context.contains(&(config, feature)) {
                milp.feature_group_relation[&(feature, group)].into_expression()
            } else {
                0.into_expression()
            }
        }).sum::<Expression>();
        constraint!(milp.config_group_count[&(config, group)] == sum)
    })
}

fn cardinality_min_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(0..milp.rows, 0..milp.columns)
        .map(|(config, group)| constraint!(milp.cardinality_min[&group] <= milp.config_group_count[&(config, group)] + milp.columns_f * (1 - milp.config_group_relation[&(config, group)])))
}

fn cardinality_max_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(0..milp.rows, 0..milp.columns)
        .map(|(config, group)| constraint!(milp.cardinality_max[&group] >= milp.config_group_count[&(config, group)] - milp.columns_f * (1 - milp.config_group_relation[&(config, group)])))
}

fn cardinality_root_constraints(milp: &FeatureModelMilp) -> [Constraint; 2] {
    [
        constraint!(milp.cardinality_min[&0] == 1),
        constraint!(milp.cardinality_max[&0] == 1),
    ]
}

fn cardinality_min_max_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|group| constraint!(milp.cardinality_min[&group] <= milp.cardinality_max[&group]))
}

fn group_size_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).map(|group| {
        let sum = (0..milp.columns)
            .map(|feature| milp.feature_group_relation[&(feature, group)])
            .sum::<Expression>();
        constraint!(milp.group_size[&group] == sum)
    })
}

fn group_not_empty_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).flat_map(|group| [
        constraint!(milp.group_not_empty[&group] <= milp.group_size[&group]),
        constraint!(milp.group_not_empty[&group] * milp.columns_f >= milp.group_size[&group]),
    ])
}

fn group_symmetry_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (1..milp.columns)
        .map(|group| constraint!(milp.group_not_empty[&(group - 1)] >= milp.group_not_empty[&group]))
}

fn group_count_constraint(milp: &FeatureModelMilp) -> Constraint {
    let sum = (0..milp.columns)
        .map(|group| milp.group_not_empty[&group])
        .sum::<Expression>();
    constraint!(milp.group_count == sum)
}

fn is_mandatory_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).flat_map(|group| [
        // cardinality_min[k] == cardinality_max[k]
        constraint!(milp.cardinality_max[&group] - milp.cardinality_min[&group] <= milp.columns_f * (1 - milp.is_mandatory[&group])),
        // group_size[k] == cardinality_max[k]
        constraint!(milp.group_size[&group] - milp.cardinality_max[&group] <= milp.columns_f * (1 - milp.is_mandatory[&group])),
        // A group is not mandatory if it is empty
        constraint!(milp.group_not_empty[&group] >= milp.is_mandatory[&group]),
    ])
}

fn mandatory_rewards_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).flat_map(|group| [
        constraint!(milp.mandatory_rewards[&group] <= MANDATORY_BASE * milp.columns_f * milp.is_mandatory[&group]),
        constraint!(milp.mandatory_rewards[&group] <= MANDATORY_BASE * milp.group_size[&group]),
    ])
}

fn is_alternative_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).flat_map(|group| [
        constraint!(milp.cardinality_min[&group] >= 1 - milp.columns_f * (1 - milp.is_alternative[&group])),
        constraint!(milp.cardinality_min[&group] <= 1 + milp.columns_f * (1 - milp.is_alternative[&group])),
        constraint!(milp.cardinality_max[&group] >= 1 - milp.columns_f * (1 - milp.is_alternative[&group])),
        constraint!(milp.cardinality_max[&group] <= 1 + milp.columns_f * (1 - milp.is_alternative[&group])),
    ])
}

fn alternative_rewards_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).flat_map(|group| [
        constraint!(milp.alternative_rewards[&group] <= ALTERNATIVE_BASE * milp.columns_f * milp.is_alternative[&group]),
        constraint!(milp.alternative_rewards[&group] <= ALTERNATIVE_BASE * milp.group_size[&group]),
    ])
}

fn is_or_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).flat_map(|group| [
        constraint!(milp.cardinality_min[&group] >= 1 - milp.columns_f * (1 - milp.is_or[&group])),
        constraint!(milp.cardinality_min[&group] <= 1 + milp.columns_f * (1 - milp.is_or[&group])),
    ])
}

fn or_rewards_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns).flat_map(|group| [
        constraint!(milp.or_rewards[&group] <= OR_BASE * milp.columns_f * milp.is_or[&group]),
        constraint!(milp.or_rewards[&group] <= OR_BASE * milp.group_size[&group]),
    ])
}

fn group_types_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (0..milp.columns)
        .map(|group| constraint!(milp.is_mandatory[&group] + milp.is_alternative[&group] + milp.is_or[&group] + (1 - milp.group_not_empty[&group]) <= 1))
}

fn config_parent_relation_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(0..milp.rows, 1..milp.columns).map(|(config, group)| {
        let sum = (0..milp.columns)
            .map(|parent| if milp.context.contains(&(config, parent)) { 1 } else { 0 } * milp.group_parent_relation[&(group, parent)])
            .sum::<Expression>();
        constraint!(milp.config_group_relation[&(config, group)] == sum)
    })
}

fn root_in_group_0_constraint(milp: &FeatureModelMilp) -> Constraint {
    constraint!(milp.feature_group_relation[&(0, 0)] == 1)
}

fn group_0_size_1_constraint(milp: &FeatureModelMilp) -> Constraint {
    constraint!(milp.group_size[&0] == 1)
}

fn feature_parent_same_as_group_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(1..milp.columns, 1..milp.columns, 0..milp.columns).map(|(feature, group, parent)| {
        constraint!(milp.feature_parent_relation[&(feature, parent)] >= milp.feature_group_relation[&(feature, group)] + milp.group_parent_relation[&(group, parent)] - 1)
    })
}

fn feature_depends_on_parent_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    let not_dependencies = p!(1..milp.columns, 0..milp.columns).filter(|&(child, parent)| {
        !(0..milp.rows).filter(|&config| milp.context.contains(&(config, child)))
            .all(|config| milp.context.contains(&(config, parent)))
    });

    not_dependencies.map(|(feature, parent)| constraint!(milp.feature_parent_relation[&(feature, parent)] == 0))
}

fn root_depth_0_constraint(milp: &FeatureModelMilp) -> Constraint {
    constraint!(milp.feature_depth[&0] == 0)
}

fn parent_depth_above_child_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    p!(1..milp.columns, 0..milp.columns).map(|(feature, parent)| {
        constraint!(milp.feature_depth[&feature] >= milp.feature_depth[&parent] - milp.columns_f * (1 - milp.feature_parent_relation[&(feature, parent)]) + 1)
    })
}

fn one_parent_per_feature_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (1..milp.columns).map(|feature| {
        let sum = (0..milp.columns)
            .map(|parent| milp.feature_parent_relation[&(feature, parent)])
            .sum::<Expression>();
        constraint!(sum == 1)
    })
}

fn one_parent_per_group_constraints(milp: &FeatureModelMilp) -> impl Iterator<Item = Constraint> {
    (1..milp.columns).map(|group| {
        let sum = (0..milp.columns)
            .map(|parent| milp.group_parent_relation[&(group, parent)])
            .sum::<Expression>();
        constraint!(sum == 1)
    })
}