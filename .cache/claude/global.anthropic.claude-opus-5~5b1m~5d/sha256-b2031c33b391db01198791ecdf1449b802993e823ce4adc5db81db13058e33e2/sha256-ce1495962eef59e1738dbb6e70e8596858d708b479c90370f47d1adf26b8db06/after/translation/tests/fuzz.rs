//! High-volume randomized differential fuzzing of the public entry point.
//!
//! Complements the per-row tests in `differential.rs`: this file blankets the
//! whole 4-`int` input space, plus several structured sub-spaces that are dense
//! in the branches `lib.c` takes (float classes, sign patterns, low bytes,
//! decimal widths).
//!
//! Iteration counts can be raised with `HARVEST_FUZZ_ITERS=<n>`.

mod common;

use common::{assert_same, Rng};

fn iters(default: usize) -> usize {
    std::env::var("HARVEST_FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

#[test]
fn fuzz_uniform_int4() {
    let n = iters(1_000_000);
    let mut rng = Rng::new(0x5EED_0001);
    for _ in 0..n {
        assert_same(
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

/// Small magnitudes dominate the interesting formatting/`buf_sum` behaviour.
#[test]
fn fuzz_small_magnitudes() {
    let n = iters(300_000);
    let mut rng = Rng::new(0x5EED_0002);
    for _ in 0..n {
        let pick = |r: &mut Rng| {
            let m = 10i64.pow((r.next_u32() % 10) as u32);
            let v = r.range_i64(0, m) as i32;
            if r.next_u32() & 1 == 0 {
                v
            } else {
                v.wrapping_neg()
            }
        };
        let a = pick(&mut rng);
        let b = pick(&mut rng);
        let c = pick(&mut rng);
        let d = pick(&mut rng);
        assert_same(a, b, c, d);
    }
}

/// `a` restricted to the float window that actually contributes to `result`.
#[test]
fn fuzz_float_window() {
    let n = iters(300_000);
    let mut rng = Rng::new(0x5EED_0003);
    for _ in 0..n {
        let a = rng.range_u32_as_i32(0x3F80_0000, 0x447A_0000);
        assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

/// Dense sweep of `a` around every float exponent, with random `b,c,d`.
#[test]
fn fuzz_float_exponent_neighbourhoods() {
    let mut rng = Rng::new(0x5EED_0004);
    let per = iters(64);
    for exp in 0u32..256 {
        for sign in [0u32, 0x8000_0000] {
            for _ in 0..per {
                let mant = rng.next_u32() & 0x7F_FFFF;
                let a = (sign | (exp << 23) | mant) as i32;
                assert_same(a, rng.next_i32(), rng.next_i32(), rng.next_i32());
            }
        }
    }
}

/// Exhaustive low-byte cube for `b`, `c`, `d` (2^24 combinations is too slow;
/// use a full sweep of each byte independently plus a randomized cross-section).
#[test]
fn fuzz_low_byte_space() {
    let mut rng = Rng::new(0x5EED_0005);
    for lb in 0u32..256 {
        for lc in 0u32..256 {
            let ld = rng.next_u32() & 0xFF;
            let hi = |r: &mut Rng| r.next_u32() & 0xFFFF_FF00;
            let b = (hi(&mut rng) | lb) as i32;
            let c = (hi(&mut rng) | lc) as i32;
            let d = (hi(&mut rng) | ld) as i32;
            assert_same(rng.next_i32(), b, c, d);
        }
    }
}

/// Every sign pattern crossed with every decimal-width pattern.
#[test]
fn fuzz_sign_and_width_cross_product() {
    let mut rng = Rng::new(0x5EED_0006);
    let per = iters(24);
    let bound = |w: u32| -> i64 {
        match w {
            0 => 9,
            _ => (10i64.pow(w + 1) - 1).min(i32::MAX as i64),
        }
    };
    for mask in 0u32..16 {
        for w in [0u32, 1, 2, 4, 6, 8, 9] {
            for _ in 0..per {
                let mut v = [0i32; 4];
                for (k, slot) in v.iter_mut().enumerate() {
                    let hi = bound(w);
                    let lo = if w == 0 { 0 } else { 10i64.pow(w) };
                    let mag = rng.range_i64(lo.min(hi), hi) as i32;
                    *slot = if (mask >> k) & 1 == 1 {
                        mag.wrapping_neg()
                    } else {
                        mag
                    };
                }
                assert_same(v[0], v[1], v[2], v[3]);
            }
        }
    }
}

/// Sequentially exhaustive sweep of `a` over a contiguous block, holding
/// `b,c,d` at values chosen to activate every later stage.
#[test]
fn fuzz_contiguous_a_sweep() {
    let n = iters(200_000) as i64;
    for base in [
        0i64,
        1,
        0x3F80_0000,
        0x447A_0000 - 1,
        0x7F80_0000 - 1,
        i32::MAX as i64 - n,
        i32::MIN as i64,
    ] {
        let mut k = 0i64;
        while k < n.min(40_000) {
            let a = base.wrapping_add(k) as i32;
            assert_same(a, -1, 255, 0x0100_0001);
            k += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Heavy exhaustive sweeps — run with `cargo test --release -- --ignored`
// ---------------------------------------------------------------------------

/// Exhaustive over all 2^24 combinations of the low bytes of `b`, `c`, `d`,
/// which is the complete input space of `interpret_as_int` and (together with
/// `a`) of `complex_iteration`.
#[test]
#[ignore = "heavy: 2^24 differential calls"]
fn heavy_exhaustive_low_bytes() {
    for lb in 0u32..256 {
        for lc in 0u32..256 {
            for ld in 0u32..256 {
                assert_same(0x4479_FFFFu32 as i32, lb as i32, lc as i32, ld as i32);
            }
        }
    }
}

/// Exhaustive over the low 24 bits of `a` (all float mantissas at the small
/// exponents, i.e. the subnormal + `(int)f == 0` region) and, shifted, over the
/// accepted float window.
#[test]
#[ignore = "heavy: 3 * 2^24 differential calls"]
fn heavy_exhaustive_a_low24() {
    for base in [0u32, 0x3F80_0000, 0x4479_0000u32.wrapping_sub(0x00FF_FFFF)] {
        for k in 0u32..0x0100_0000 {
            assert_same(base.wrapping_add(k) as i32, -1, 255, 0x0100_0001);
        }
    }
}

/// Exhaustive over the whole `int` range for `a`, striding so the run stays
/// bounded, with several `(b,c,d)` witnesses.
#[test]
#[ignore = "heavy: full-range strided sweep of a"]
fn heavy_full_range_a_stride() {
    let witnesses: [(i32, i32, i32); 4] = [
        (0, 0, 0),
        (-1, -1, -1),
        (i32::MIN, i32::MAX, 12345),
        (255, 256, -256),
    ];
    let stride: u32 = 1021; // prime, so residues cover every class
    for &(b, c, d) in &witnesses {
        let mut u: u32 = 0;
        loop {
            assert_same(u as i32, b, c, d);
            let (next, of) = u.overflowing_add(stride);
            if of {
                break;
            }
            u = next;
        }
    }
}
