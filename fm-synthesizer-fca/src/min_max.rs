use std::cmp::{max, min};

pub trait MinMaxExt: Iterator {
    fn min_max(self) -> Option<(Self::Item, Self::Item)> where Self::Item: Ord + Copy;
}

impl<T: Ord, I: Iterator<Item = T>> MinMaxExt for I {
    /// Calculates the minimum and maximum value of the [Iterator].
    fn min_max(self) -> Option<(Self::Item, Self::Item)> where Self::Item: Ord + Copy {
        self.fold(None, |min_max, value| {
            if let Some((min_value, max_value)) = min_max {
                let new_min_value = min(min_value, value);
                let new_max_value = max(max_value, value);
                Some((new_min_value, new_max_value))
            } else {
                Some((value, value))
            }
        })
    }
}