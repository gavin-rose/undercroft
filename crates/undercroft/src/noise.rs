//! Value noise using only `+ - * /` and `floor`, which are bit-identical on
//! every platform. (Library `sin`/`pow` can differ between native and WASM.)

use crate::rng::splitmix64;

/// Smooth noise in `0..1` with features about one unit apart.
pub fn value(seed: u64, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (tx, ty) = (smoothstep(x - x0), smoothstep(y - y0));
    let (xi, yi) = (x0 as i64, y0 as i64);
    let top = lerp(lattice(seed, xi, yi), lattice(seed, xi + 1, yi), tx);
    let bottom = lerp(lattice(seed, xi, yi + 1), lattice(seed, xi + 1, yi + 1), tx);
    lerp(top, bottom, ty)
}

/// Several octaves of [`value`] noise, each twice the frequency and half the
/// weight of the last. Still in `0..1`.
pub fn fractal(seed: u64, x: f64, y: f64, octaves: u32) -> f64 {
    let (mut sum, mut norm, mut amp, mut freq) = (0.0, 0.0, 1.0, 1.0);
    for octave in 0..octaves.max(1) {
        sum += amp * value(seed.wrapping_add(octave as u64), x * freq, y * freq);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

fn lattice(seed: u64, x: i64, y: i64) -> f64 {
    let mut state = seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    (splitmix64(&mut state) >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}

fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
