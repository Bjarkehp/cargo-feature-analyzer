/// Calculates n choose k of two u32's
pub fn n_choose_k_u32(n: u32, k: u32) -> u32 {
    (1..=k).map(|i| (n - k + i) / i)
        .product::<u32>()
}

/// Calculates n choose k of two usize's
pub fn n_choose_k_usize(n: usize, k: usize) -> usize {
    (1..=k).map(|i| (n - k + i) / i)
        .product::<usize>()
}