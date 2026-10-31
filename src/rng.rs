//! Deterministic `SplitMix64` PRNG. In-crate on purpose: output must be byte-identical
//! across platforms and releases, and the dependency tree stays small.

#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Independent stream per `(seed, label)` pair, so adding a tuple never shifts another's text.
    pub fn for_label(seed: u64, label: &str) -> Self {
        // FNV-1a over the label, folded into the seed.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in label.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        let mut r = Self(seed ^ h.rotate_left(17));
        r.next_u64();
        r
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n`. Returns 0 when `n == 0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        usize::try_from(self.next_u64() % (n as u64)).unwrap_or(0)
    }

    /// Weighted index. Zero total weight falls back to uniform.
    pub fn weighted(&mut self, weights: &[u32]) -> usize {
        let total: u64 = weights.iter().map(|w| u64::from(*w)).sum();
        if total == 0 {
            return self.below(weights.len());
        }
        let mut roll = self.next_u64() % total;
        for (i, w) in weights.iter().enumerate() {
            let w = u64::from(*w);
            if roll < w {
                return i;
            }
            roll -= w;
        }
        weights.len().saturating_sub(1)
    }

    /// Fisher-Yates.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn labels_give_independent_streams() {
        let mut a = Rng::for_label(7, "tuple-a");
        let mut b = Rng::for_label(7, "tuple-b");
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn known_vector_is_stable() {
        // Pins the stream so a refactor cannot silently change every generated dataset.
        let mut r = Rng::new(0);
        assert_eq!(r.next_u64(), 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn weighted_respects_zero_weights() {
        let mut r = Rng::new(1);
        for _ in 0..200 {
            assert_eq!(r.weighted(&[0, 5, 0]), 1);
        }
    }
}
