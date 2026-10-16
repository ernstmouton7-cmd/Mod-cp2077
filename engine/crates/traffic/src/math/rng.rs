//! Deterministic pseudo random generator (Park–Miller minimal standard).

/// Park–Miller "minimal standard" generator, identical to `math/rng.ts`, so a
/// seed produces the same sequence in Rust, JavaScript and Lua.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

const M: u64 = 2_147_483_647;

impl Rng {
    /// Seed with any integer (negative values use their magnitude).
    pub fn new(seed: i64) -> Self {
        let mut s = seed.unsigned_abs() % 2_147_483_646;
        if s == 0 {
            s = 2_147_483_646;
        }
        Rng { state: s }
    }

    /// Uniform in [0, 1).
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        self.state = (self.state * 16807) % M;
        (self.state - 1) as f64 / 2_147_483_646.0
    }

    /// Uniform in [lo, hi).
    #[inline]
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    /// Integer in [lo, hi] inclusive.
    pub fn int(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next_f64() * (hi - lo + 1) as f64).floor() as i64
    }

    #[inline]
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// Uniformly pick one item.
    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.index(items.len())]
    }

    /// Uniform index in `0..len`.
    #[inline]
    pub fn index(&mut self, len: usize) -> usize {
        ((self.next_f64() * len as f64).floor() as usize).min(len.saturating_sub(1))
    }

    /// Approximately normal (sum of four uniforms), within ±3.46 sigma.
    pub fn gauss(&mut self, mean: f64, sigma: f64) -> f64 {
        let u = self.next_f64() + self.next_f64() + self.next_f64() + self.next_f64() - 2.0;
        mean + sigma * u * 1.7320508
    }

    /// Pick from `(item, weight)` entries.
    pub fn weighted<T: Copy>(&mut self, entries: &[(T, f64)]) -> T {
        let total: f64 = entries.iter().map(|e| e.1).sum();
        let mut r = self.next_f64() * total;
        for e in entries {
            r -= e.1;
            if r <= 0.0 {
                return e.0;
            }
        }
        entries[entries.len() - 1].0
    }

    /// A new generator seeded from this one.
    pub fn fork(&mut self) -> Rng {
        Rng::new((self.next_f64() * 2_147_483_000.0).floor() as i64 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn park_miller_reference_values() {
        // 16807^n mod (2^31 - 1) starting at seed 1.
        let mut r = Rng::new(1);
        r.next_f64();
        assert_eq!(r.state, 16807);
        r.next_f64();
        assert_eq!(r.state, 282_475_249);
        let mut z = Rng::new(0);
        assert_eq!(z.state, 2_147_483_646);
        z.next_f64();
    }
}
