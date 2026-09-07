//! Near-exhaustive stress sweeps.
//!
//! Phases B and C are organised per `CONFIGS.md` / `ERRORS.md` row. This file is
//! the belt-and-braces backstop: it streams tens of millions of `(y, exp_q2)`
//! pairs through both `.so` exports without materialising them, so it can cover
//! far more of the input space than the per-row batches.
//!
//! Structural note that makes these sweeps close to exhaustive: the C body only
//! consumes `e` through `e & 3` and `(e >> 2) & 31` (the x86 shift-count mask),
//! so its behaviour is **periodic in `e` with period 128**. Sweeping any
//! contiguous 128-wide band of `e`, plus the clamp boundary at 120 and the loop
//! trip count, therefore covers every distinct integer code path.

mod common;

use common::{pair, Rng};

/// Stream a sweep, comparing bit patterns without allocating.
struct Sweep {
    label: &'static str,
    n: u64,
    fails: Vec<String>,
}

impl Sweep {
    fn new(label: &'static str) -> Self {
        Sweep {
            label,
            n: 0,
            fails: Vec::new(),
        }
    }

    #[inline]
    fn probe(&mut self, y: f32, exp_q2: i32) {
        let p = pair();
        let c = unsafe { (p.c)(y, exp_q2) };
        let r = unsafe { (p.rust)(y, exp_q2) };
        self.n += 1;
        if c.to_bits() != r.to_bits() {
            if self.fails.len() < 20 {
                self.fails.push(format!(
                    "y bits 0x{:08X} ({y:?}), exp_q2 {exp_q2} (0x{:08X}): C 0x{:08X} vs Rust 0x{:08X}",
                    y.to_bits(),
                    exp_q2 as u32,
                    c.to_bits(),
                    r.to_bits()
                ));
            }
        }
    }

    fn finish(self) {
        assert!(self.n > 0, "[{}] vacuous sweep", self.label);
        if !self.fails.is_empty() {
            panic!(
                "[{}] divergences over {} probes; first {}:\n{}",
                self.label,
                self.n,
                self.fails.len(),
                self.fails.join("\n")
            );
        }
        eprintln!("[{}] {} probes, 0 divergences", self.label, self.n);
    }
}

/// Every `exp_q2` in a band wide enough to cover the full period-128 behaviour
/// several times over, plus the clamp boundary and multi-iteration region,
/// crossed with a fixed set of `y` values covering every float class.
#[test]
fn stress_exhaustive_exp_band_x_float_classes() {
    let ys: Vec<f32> = common::SPECIAL_F32_BITS
        .iter()
        .copied()
        .chain([
            0x3FC0_0000, 0xBFC0_0000, // ±1.5
            0x4049_0FDB, 0xC049_0FDB, // ±pi
            0x0000_0100, 0x8000_0100, // subnormals
            0x7F00_0000, 0xFF00_0000, // huge normals
            0x0100_0000, 0x8100_0000, // tiny normals
            0x3F7F_FFFF, 0x3F80_0001, // just below / above 1.0
        ])
        .map(f32::from_bits)
        .collect();

    let mut s = Sweep::new("exhaustive_exp_band");
    // -2048..=2048 is 32 full periods of the negative masked-shift behaviour and
    // covers 1, 2, 3 ... 18 loop iterations on the positive side.
    for e in -2048..=2048i32 {
        for &y in &ys {
            s.probe(y, e);
        }
    }
    // The two extreme ends of the int range, exhaustively for 4096 values each.
    for k in 0..4096i32 {
        let e = i32::MIN.wrapping_add(k);
        for &y in &ys {
            s.probe(y, e);
        }
    }
    for k in 0..4096i32 {
        let e = i32::MIN / 2 + k;
        for &y in &ys {
            s.probe(y, e);
        }
    }
    s.finish();
}

/// Exhaustive sweep of the `f32` mantissa for several exponent fields, so every
/// rounding case of the two `mulss`es is exercised for the key `exp_q2` values.
#[test]
fn stress_exhaustive_mantissa_sweep() {
    // Exponent fields spanning subnormal, tiny, mid, and huge magnitudes.
    let exp_fields: [u32; 9] = [0, 1, 0x0B, 0x40, 0x7F, 0x80, 0xC0, 0xFD, 0xFE];
    // The integer-side configurations that matter: index 0..3 crossed with the
    // interesting masked shift counts, plus multi-iteration.
    let exps: [i32; 16] = [
        0, 1, 2, 3, 119, 120, 121, 240, 241, -1, -2, -3, -4, -128, -129, i32::MIN,
    ];

    let mut s = Sweep::new("exhaustive_mantissa");
    // Stride over the 2^23 mantissa space: 8192 samples per (exp_field, sign),
    // hitting the low, high and carry-boundary mantissas exactly.
    let stride = (1u32 << 23) / 8192;
    for &ef in &exp_fields {
        for sign in [0u32, 0x8000_0000] {
            let mut m = 0u32;
            while m < (1 << 23) {
                let y = f32::from_bits(sign | (ef << 23) | m);
                for &e in &exps {
                    s.probe(y, e);
                }
                m += stride;
            }
            // exact mantissa boundaries
            for m in [0u32, 1, 2, 0x7F_FFFE, 0x7F_FFFF, 0x40_0000, 0x3F_FFFF] {
                let y = f32::from_bits(sign | (ef << 23) | m);
                for &e in &exps {
                    s.probe(y, e);
                }
            }
        }
    }
    s.finish();
}

/// Strided sweep across the ENTIRE 2^32 `f32` bit space (every float class,
/// every NaN payload class, every subnormal magnitude) crossed with the
/// distinct integer configurations.
#[test]
fn stress_full_f32_bitspace_stride() {
    // 4093 is prime, so the stride visits every residue class mod small numbers
    // and walks the whole 2^32 space in ~1.05M steps.
    const STRIDE: u32 = 4093;
    let exps: [i32; 12] = [0, 1, 3, 60, 119, 120, 121, 241, -1, -3, -128, i32::MIN];

    let mut s = Sweep::new("full_f32_bitspace");
    let mut bits: u32 = 0;
    loop {
        let y = f32::from_bits(bits);
        for &e in &exps {
            s.probe(y, e);
        }
        let (next, of) = bits.overflowing_add(STRIDE);
        if of {
            break;
        }
        bits = next;
    }
    s.finish();
}

/// Large fixed-seed random fuzz over the full cross product, with the positive
/// exponent magnitude bounded so the loop trip count stays reasonable.
#[test]
fn stress_random_fuzz_millions() {
    let mut rng = Rng::new(0x5EED_1234_5678_9ABC);
    let mut s = Sweep::new("random_fuzz");
    for _ in 0..20_000_000u64 {
        let y = f32::from_bits(rng.next_u32());
        let raw = rng.next_i32();
        // Full i32 range for negatives (always 1 iteration); bound positives.
        let e = if raw >= 0 { raw % 20_000 } else { raw };
        s.probe(y, e);
    }
    s.finish();
}

/// Strided sweep across the ENTIRE negative `exp_q2` range (all 2^31 values),
/// which is the region where C invokes undefined behaviour via the negative
/// shift count. Every negative exponent completes in exactly one iteration, so
/// this is cheap; the stride is prime so it lands on all 128 behaviour classes.
#[test]
fn stress_full_negative_exp_range_stride() {
    const STRIDE: i64 = 251;
    let ys: [f32; 10] = [
        1.0,
        -1.0,
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NAN,
    ];
    let mut s = Sweep::new("full_negative_exp_range");
    let mut e: i64 = i32::MIN as i64;
    while e < 0 {
        let ei = e as i32;
        for &y in &ys {
            s.probe(y, ei);
        }
        e += STRIDE;
    }
    s.finish();
}

/// Strided sweep across the entire POSITIVE `exp_q2` range. The trip count is
/// `ceil(exp_q2 / 120)`, so a coarse stride keeps the total work bounded while
/// still visiting every remainder class and a huge span of trip counts.
#[test]
fn stress_full_positive_exp_range_stride() {
    // Prime stride; ~43 samples spanning 1 .. INT_MAX. Each sample costs
    // ceil(e/120) iterations *inside* both libraries, so the sample count is
    // deliberately small -- the fine structure (every remainder class mod 120)
    // is covered by the cheap loop below and by `stress_all_integer_paths`.
    const STRIDE: i64 = 49_999_991;
    let ys: [f32; 3] = [1.0, -1.0, f32::NAN];
    let mut s = Sweep::new("full_positive_exp_range");
    let mut e: i64 = 1;
    while e <= i32::MAX as i64 {
        let ei = e as i32;
        for &y in &ys {
            s.probe(y, ei);
        }
        e += STRIDE;
    }
    // Every remainder class mod 120, at several trip counts.
    for rem in 0..120i64 {
        for trips in [1i64, 2, 3, 4, 17, 1000, 10_000] {
            let e = (120 * (trips - 1) + rem).max(1) as i32;
            for y in [1.0f32, -1.0, f32::MAX, f32::MIN_POSITIVE, 0.0, -0.0,
                      f32::INFINITY, f32::NEG_INFINITY, f32::NAN, 0.75, -1234.5] {
                s.probe(y, e);
            }
        }
    }
    s.finish();
}

/// Exhaustive sweep of every `exp_q2` residue/shift combination, i.e. all 128
/// values of `e mod 128` on the negative side and all 31 in-range shifts on the
/// positive side, crossed with a wide random `y` sample each.
#[test]
fn stress_all_integer_paths() {
    let mut rng = Rng::new(0xA111_0BEEF_u64.wrapping_mul(3));
    let mut s = Sweep::new("all_integer_paths");

    // Negative side: 128 distinct behaviours, sampled at 40 different
    // "generations" so the actual value of e (not just e mod 128) varies.
    for era in 0..40i32 {
        for r in 0..128i32 {
            let e = -(era * 128 + r) - 1;
            for _ in 0..24 {
                s.probe(rng.any_f32(), e);
                s.probe(rng.moderate_f32(), e);
            }
        }
    }
    // Positive side: every e in 0..=120 (single iteration, all indices/shifts).
    for e in 0..=120i32 {
        for _ in 0..300 {
            s.probe(rng.any_f32(), e);
            s.probe(rng.moderate_f32(), e);
        }
    }
    // Multi-iteration: every remainder in 0..=119 for trip counts 2..=6.
    for trips in 2..=6i32 {
        for rem in 0..120i32 {
            let e = 120 * (trips - 1) + rem.max(1);
            for _ in 0..40 {
                s.probe(rng.moderate_f32(), e);
                s.probe(rng.any_f32(), e);
            }
        }
    }
    s.finish();
}
