//! Uniform grid spatial hash for broad-phase proximity queries.

use std::collections::HashMap;

/// Uniform grid of cells holding copies of `T` (usually an index or id).
/// Items are inserted into every cell their bounding circle touches.
#[derive(Clone, Debug)]
pub struct SpatialHash<T> {
    cells: HashMap<i64, Vec<T>>,
    inv: f64,
    pub cell_size: f64,
}

impl<T: Copy + PartialEq> SpatialHash<T> {
    pub fn new(cell_size: f64) -> Self {
        SpatialHash {
            cells: HashMap::new(),
            inv: 1.0 / cell_size,
            cell_size,
        }
    }

    #[inline]
    fn key(cx: i64, cy: i64) -> i64 {
        (cx + 32768) * 65536 + (cy + 32768)
    }

    pub fn clear(&mut self) {
        for c in self.cells.values_mut() {
            c.clear();
        }
    }

    pub fn insert(&mut self, item: T, x: f64, y: f64, radius: f64) {
        let x0 = ((x - radius) * self.inv).floor() as i64;
        let x1 = ((x + radius) * self.inv).floor() as i64;
        let y0 = ((y - radius) * self.inv).floor() as i64;
        let y1 = ((y + radius) * self.inv).floor() as i64;
        for cx in x0..=x1 {
            for cy in y0..=y1 {
                self.cells.entry(Self::key(cx, cy)).or_default().push(item);
            }
        }
    }

    /// Appends items whose cells overlap the query circle (each at most once,
    /// in first-seen order).
    pub fn query(&self, x: f64, y: f64, radius: f64, out: &mut Vec<T>) {
        let x0 = ((x - radius) * self.inv).floor() as i64;
        let x1 = ((x + radius) * self.inv).floor() as i64;
        let y0 = ((y - radius) * self.inv).floor() as i64;
        let y1 = ((y + radius) * self.inv).floor() as i64;
        for cx in x0..=x1 {
            for cy in y0..=y1 {
                if let Some(cell) = self.cells.get(&Self::key(cx, cy)) {
                    for &it in cell {
                        if !out.contains(&it) {
                            out.push(it);
                        }
                    }
                }
            }
        }
    }
}
