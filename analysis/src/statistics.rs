use std::{iter::Sum, ops::{Mul, Sub}};

use itertools::Itertools;

pub fn median_mean_dev<T: Sum + Sub + Mul, I: Iterator<Item = T>>(population_iter: I) -> Option<(f64, f64, f64)>
    where f64: From<T>
{
    let population = population_iter
        .map(|x| f64::from(x))
        .sorted_by(|a, b| a.total_cmp(b))
        .collect::<Vec<_>>();

    let n = population.len();

    if n == 0 {
        return None;
    }

    let left = population[(n - 1) / 2];
    let right = population[n / 2];
    let median = (left + right) / 2.0;

    let mean = population
        .iter()
        .sum::<f64>() / n as f64;

    let sd_sq = population
        .iter()
        .map(|x| (x - mean) * (x - mean))
        .sum::<f64>() / n as f64;

    let sd = sd_sq.sqrt();

    Some((median, mean, sd))
}