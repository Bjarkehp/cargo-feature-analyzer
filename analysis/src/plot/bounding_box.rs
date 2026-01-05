use std::{cmp::{max, min}, ops::{Mul, Range}};

use ordered_float::OrderedFloat;

const REVERSE_INFINITE: BoundingBox = BoundingBox {
    left: f64::INFINITY,
    right: f64::NEG_INFINITY,
    bottom: f64::INFINITY,
    top: f64::NEG_INFINITY,
};

#[derive(Debug, derive_new::new)]
pub struct BoundingBox {
    pub left: f64,
    pub right: f64,
    pub bottom: f64,
    pub top: f64,
}

impl BoundingBox {
    pub fn all(f: f64) -> BoundingBox {
        BoundingBox::new(f, f, f, f)
    }

    pub fn with(self, (x, y): (f64, f64)) -> BoundingBox {
        let left = *min(OrderedFloat(self.left), OrderedFloat(x));
        let right = *max(OrderedFloat(self.right), OrderedFloat(x));
        let bottom = *min(OrderedFloat(self.bottom), OrderedFloat(y));
        let top = *max(OrderedFloat(self.top), OrderedFloat(y));
        BoundingBox { left, right, bottom, top }
    }

    pub fn horizontal_range(&self) -> Range<f64> {
        self.left..self.right
    }

    pub fn vertical_range(&self) -> Range<f64> {
        self.bottom..self.top
    }
}

impl Mul for BoundingBox {
    type Output = BoundingBox;

    fn mul(self, rhs: Self) -> Self::Output {
        let len_h = self.right - self.left;
        let len_v = self.top - self.bottom;
        let center_h = (self.right + self.left) / 2.0;
        let center_v = (self.top + self.bottom) / 2.0;
        let left = center_h - len_h * rhs.left / 2.0;
        let right = center_h + len_h * rhs.right / 2.0;
        let bottom = center_v - len_v * rhs.bottom / 2.0;
        let top = center_v + len_v * rhs.top / 2.0;
        BoundingBox { left, right, bottom, top }
    }
}

impl FromIterator<(f64, f64)> for BoundingBox {
    fn from_iter<T: IntoIterator<Item = (f64, f64)>>(iter: T) -> Self {
        iter.into_iter().fold(REVERSE_INFINITE, BoundingBox::with)
    }
}