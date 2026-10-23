//! xoshiro256** seeded via splitmix64. Implemented here rather than taken from
//! `rand` so a dependency upgrade can never change what a seed produces.

pub struct Rng {
    s: [u64; 4],
}

pub(crate) fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut sm = seed;
        Rng {
            s: [
                splitmix64(&mut sm),
                splitmix64(&mut sm),
                splitmix64(&mut sm),
                splitmix64(&mut sm),
            ],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform integer in `lo..hi`. Uses rejection sampling, so it's unbiased.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo < hi, "empty range {lo}..{hi}");
        let span = (hi as i64 - lo as i64) as u64;
        let limit = u64::MAX - u64::MAX % span;
        loop {
            let x = self.next_u64();
            if x < limit {
                return (lo as i64 + (x % span) as i64) as i32;
            }
        }
    }

    pub fn index(&mut self, len: usize) -> usize {
        self.range(0, len as i32) as usize
    }

    /// Basic float arithmetic only, so it's bit-identical on every platform.
    pub fn chance(&mut self, p: f64) -> bool {
        let unit = (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
        unit < p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_stays_in_bounds() {
        let mut rng = Rng::new(7);
        for _ in 0..10_000 {
            let x = rng.range(-3, 4);
            assert!((-3..4).contains(&x));
        }
    }

    #[test]
    fn same_seed_same_stream() {
        let (mut a, mut b) = (Rng::new(42), Rng::new(42));
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }
}
