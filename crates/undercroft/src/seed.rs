use serde::Serialize;

use crate::recipe::GEN_VERSION;
use crate::rng::splitmix64;

/// A generation seed. Build one from any text with [`Seed::from_text`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Seed(pub u64);

impl Seed {
    /// Hashes arbitrary text into a seed, so `"hello"` and `"42"` are both valid seeds.
    pub fn from_text(text: &str) -> Self {
        Seed(finish(fnv1a(FNV_OFFSET, text.as_bytes())))
    }

    /// The shared seed for a `YYYY-MM-DD` date. Includes the generator version
    /// so a new version gets new dailies instead of changing old ones.
    pub fn daily(date: &str) -> Self {
        Seed::from_text(&format!("daily:{date}:v{GEN_VERSION}"))
    }

    /// An independent child seed for a named stage.
    pub fn derive(self, label: &str) -> Self {
        let h = fnv1a(FNV_OFFSET, &self.0.to_le_bytes());
        Seed(finish(fnv1a(h, label.as_bytes())))
    }
}

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0100_0000_01B3);
    }
    hash
}

/// FNV mixes the high bits poorly on its own.
fn finish(hash: u64) -> u64 {
    let mut state = hash;
    splitmix64(&mut state)
}
