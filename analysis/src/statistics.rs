use std::{iter::Sum, ops::{Mul, Sub}};

pub fn mean_dev<T: Sum + Sub + Mul + Into<f64>, I: Iterator<Item = T>>(population: impl Fn() -> I) -> (f64, f64) {
    let n = population().count() as f64;
    let mean = population()
        .sum::<T>()
        .into() / n;

    let sd_sq = population()
        .map(Into::into)
        .map(|x| (x - mean) * (x - mean))
        .sum::<f64>() / n;

    let sd = sd_sq.sqrt();

    (mean, sd)
}