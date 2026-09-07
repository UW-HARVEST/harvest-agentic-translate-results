//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md` (C1..C30). Every row drives BOTH the C
//! `.so` and the Rust `.so` through `libloading` and compares the 3 output
//! floats bit-for-bit. Randomized rows use a fixed SplitMix64 seed so failures
//! reproduce exactly.

mod common;
use common::{pair, Rng, N, SEED};

/// C1 — achromatic r==g==b in (0,1] -> delta == 0 early return.
#[test]
fn c01_achromatic_unit_range() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 1);
    p.assert_batch(
        "C1",
        (0..N).map(|_| {
            let v = rng.range(f32::MIN_POSITIVE, 1.0);
            [v, v, v]
        }),
    );
}

/// C2 — achromatic r==g==b at large magnitude.
#[test]
fn c02_achromatic_large() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 2);
    p.assert_batch(
        "C2",
        (0..N).map(|_| {
            let v = rng.range(1.0, 1e30);
            [v, v, v]
        }),
    );
}

/// C3 — all channels exactly +0.0: delta == 0 AND max == 0.
#[test]
fn c03_all_positive_zero() {
    let p = pair();
    p.assert_batch("C3", [[0.0f32, 0.0, 0.0]]);
}

/// C4 — r strict max, g > b: `r == max` branch, h in (0,60), no wrap.
#[test]
fn c04_r_max_g_gt_b() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 4);
    p.assert_batch(
        "C4",
        (0..N).map(|_| {
            let r = rng.range(0.5, 1.0);
            let g = rng.range(0.2, 0.5);
            let b = rng.range(0.0, 0.2);
            [r, g, b]
        }),
    );
}

/// C5 — r strict max, g < b: `h < 0` wrap (+360) taken.
#[test]
fn c05_r_max_g_lt_b_wrap() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 5);
    p.assert_batch(
        "C5",
        (0..N).map(|_| {
            let r = rng.range(0.5, 1.0);
            let b = rng.range(0.2, 0.5);
            let g = rng.range(0.0, 0.2);
            [r, g, b]
        }),
    );
}

/// C6 — r strict max, g == b exactly: h computed as +-0.0 before scaling.
#[test]
fn c06_r_max_g_eq_b() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 6);
    p.assert_batch(
        "C6",
        (0..N).map(|_| {
            let r = rng.range(0.5, 1.0);
            let gb = rng.range(0.0, 0.5);
            [r, gb, gb]
        }),
    );
}

/// C7 — g strict max: `2 + (b-r)/delta` branch.
#[test]
fn c07_g_max() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 7);
    p.assert_batch(
        "C7",
        (0..N).map(|_| {
            let g = rng.range(0.5, 1.0);
            let r = rng.range(0.0, 0.5);
            let b = rng.range(0.0, 0.5);
            [r, g, b]
        }),
    );
}

/// C8 — b strict max: `else` / `4 + (r-g)/delta` branch.
#[test]
fn c08_b_max() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 8);
    p.assert_batch(
        "C8",
        (0..N).map(|_| {
            let b = rng.range(0.5, 1.0);
            let r = rng.range(0.0, 0.5);
            let g = rng.range(0.0, 0.5);
            [r, g, b]
        }),
    );
}

/// C9 — tie r == g > b: the if-chain must resolve to the `r == max` branch.
#[test]
fn c09_tie_r_eq_g() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 9);
    p.assert_batch(
        "C9",
        (0..N).map(|_| {
            let rg = rng.range(0.5, 1.0);
            let b = rng.range(0.0, 0.5);
            [rg, rg, b]
        }),
    );
}

/// C10 — tie g == b > r: resolves to the `g == max` branch.
#[test]
fn c10_tie_g_eq_b() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 10);
    p.assert_batch(
        "C10",
        (0..N).map(|_| {
            let gb = rng.range(0.5, 1.0);
            let r = rng.range(0.0, 0.5);
            [r, gb, gb]
        }),
    );
}

/// C11 — tie r == b > g: resolves to the `r == max` branch.
#[test]
fn c11_tie_r_eq_b() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 11);
    p.assert_batch(
        "C11",
        (0..N).map(|_| {
            let rb = rng.range(0.5, 1.0);
            let g = rng.range(0.0, 0.5);
            [rb, g, rb]
        }),
    );
}

/// C12 — uniformly random normalized triple in [0,1].
#[test]
fn c12_random_unit_cube() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 12);
    p.assert_batch(
        "C12",
        (0..N * 4).map(|_| [rng.unit(), rng.unit(), rng.unit()]),
    );
}

/// C13 — uniformly random triple in [0,255].
#[test]
fn c13_random_0_255() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 13);
    p.assert_batch(
        "C13",
        (0..N * 4).map(|_| {
            [
                rng.range(0.0, 255.0),
                rng.range(0.0, 255.0),
                rng.range(0.0, 255.0),
            ]
        }),
    );
}

/// C14 — mixed sign with max > 0: delta > max, so s > 1 (unchecked out of range).
#[test]
fn c14_mixed_sign_positive_max() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 14);
    p.assert_batch(
        "C14",
        (0..N).map(|_| {
            let mut v = [
                rng.range(-1.0, 0.0),
                rng.range(-1.0, 0.0),
                rng.range(-1.0, 0.0),
            ];
            // Force exactly one channel strictly positive so max > 0.
            v[rng.below(3)] = rng.range(f32::MIN_POSITIVE, 2.0);
            v
        }),
    );
}

/// C15 — all channels negative: max < 0, delta != 0, max != 0 -> negative s and v.
#[test]
fn c15_all_negative() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 15);
    p.assert_batch(
        "C15",
        (0..N).map(|_| {
            [
                rng.range(-100.0, -f32::MIN_POSITIVE),
                rng.range(-100.0, -f32::MIN_POSITIVE),
                rng.range(-100.0, -f32::MIN_POSITIVE),
            ]
        }),
    );
}

/// C16 — max lands on exactly 0 while delta != 0: the `max == 0` disjunct
/// short-circuits a chromatic input onto the achromatic early return.
#[test]
fn c16_max_exactly_zero_delta_nonzero() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 16);
    p.assert_batch(
        "C16",
        (0..N).map(|_| {
            let mut v = [
                rng.range(-50.0, -f32::MIN_POSITIVE),
                rng.range(-50.0, -f32::MIN_POSITIVE),
                rng.range(-50.0, -f32::MIN_POSITIVE),
            ];
            // One channel is exactly zero (+0.0 or -0.0) -> max == 0, delta > 0.
            v[rng.below(3)] = if rng.bool() { 0.0 } else { -0.0 };
            v
        }),
    );
}

/// C17 — exhaustive signed-zero shapes (all 8 combinations of +-0.0).
#[test]
fn c17_signed_zero_exhaustive() {
    let p = pair();
    let zeros = [0.0f32, -0.0f32];
    let mut inputs = Vec::new();
    for &r in &zeros {
        for &g in &zeros {
            for &b in &zeros {
                inputs.push([r, g, b]);
            }
        }
    }
    assert_eq!(inputs.len(), 8);
    p.assert_batch("C17", inputs);
}

/// C18 — exactly one NaN, in r.
#[test]
fn c18_nan_in_r() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 18);
    p.assert_batch(
        "C18",
        (0..N).map(|_| [rng.nan(), rng.range(-2.0, 2.0), rng.range(-2.0, 2.0)]),
    );
}

/// C19 — exactly one NaN, in g.
#[test]
fn c19_nan_in_g() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 19);
    p.assert_batch(
        "C19",
        (0..N).map(|_| [rng.range(-2.0, 2.0), rng.nan(), rng.range(-2.0, 2.0)]),
    );
}

/// C20 — exactly one NaN, in b.
#[test]
fn c20_nan_in_b() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 20);
    p.assert_batch(
        "C20",
        (0..N).map(|_| [rng.range(-2.0, 2.0), rng.range(-2.0, 2.0), rng.nan()]),
    );
}

/// C21 — all remaining NaN masks (two or three NaNs), random payloads/signs.
#[test]
fn c21_multi_nan_masks() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 21);
    // masks with >= 2 bits set: 0b011, 0b101, 0b110, 0b111
    let masks = [0b011u8, 0b101, 0b110, 0b111];
    let mut inputs = Vec::new();
    for &mask in &masks {
        for _ in 0..N {
            let mut v = [0.0f32; 3];
            for i in 0..3 {
                v[i] = if mask & (1 << i) != 0 {
                    rng.nan()
                } else {
                    rng.range(-2.0, 2.0)
                };
            }
            inputs.push(v);
        }
    }
    p.assert_batch("C21", inputs);
}

/// C22 — +inf in one channel, finite elsewhere.
#[test]
fn c22_plus_inf() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 22);
    p.assert_batch(
        "C22",
        (0..N).map(|_| {
            let mut v = [
                rng.range(-10.0, 10.0),
                rng.range(-10.0, 10.0),
                rng.range(-10.0, 10.0),
            ];
            v[rng.below(3)] = f32::INFINITY;
            v
        }),
    );
}

/// C23 — -inf in one channel, finite elsewhere.
#[test]
fn c23_minus_inf() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 23);
    p.assert_batch(
        "C23",
        (0..N).map(|_| {
            let mut v = [
                rng.range(-10.0, 10.0),
                rng.range(-10.0, 10.0),
                rng.range(-10.0, 10.0),
            ];
            v[rng.below(3)] = f32::NEG_INFINITY;
            v
        }),
    );
}

/// C24 — both +inf and -inf present: delta == inf and inf/inf == NaN.
#[test]
fn c24_both_infinities() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 24);
    let mut inputs = Vec::new();
    // Exhaustive placement of (+inf, -inf) over the 3 slots, third slot random,
    // plus all-infinity combinations.
    for i in 0..3usize {
        for j in 0..3usize {
            if i == j {
                continue;
            }
            for _ in 0..N / 6 {
                let mut v = [rng.range(-10.0, 10.0); 3];
                v[0] = rng.range(-10.0, 10.0);
                v[1] = rng.range(-10.0, 10.0);
                v[2] = rng.range(-10.0, 10.0);
                v[i] = f32::INFINITY;
                v[j] = f32::NEG_INFINITY;
                inputs.push(v);
            }
        }
    }
    let infs = [f32::INFINITY, f32::NEG_INFINITY];
    for &r in &infs {
        for &g in &infs {
            for &b in &infs {
                inputs.push([r, g, b]);
            }
        }
    }
    p.assert_batch("C24", inputs);
}

/// C25 — subnormal components: delta underflows to a subnormal or to 0.
#[test]
fn c25_subnormals() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 25);
    p.assert_batch(
        "C25",
        (0..N * 2).map(|_| [rng.subnormal(), rng.subnormal(), rng.subnormal()]),
    );
}

/// C26 — f32::MAX-scale opposite signs: `max - min` overflows to +inf while max is finite.
#[test]
fn c26_delta_overflows_to_inf() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 26);
    p.assert_batch(
        "C26",
        (0..N).map(|_| {
            let big = f32::MAX * rng.range(0.6, 1.0);
            let small = -f32::MAX * rng.range(0.6, 1.0);
            let mut v = [rng.range(-1.0, 1.0); 3];
            let i = rng.below(3);
            let j = (i + 1 + rng.below(2)) % 3;
            v[i] = big;
            v[j] = small;
            v
        }),
    );
}

/// C27 — min = nextafter(max, -inf): smallest possible non-zero delta.
#[test]
fn c27_minimal_delta() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 27);
    p.assert_batch(
        "C27",
        (0..N).map(|_| {
            let m = rng.range(0.25, 4.0);
            let lower = f32::from_bits(m.to_bits() - 1);
            let lower2 = f32::from_bits(m.to_bits() - 2);
            match rng.below(3) {
                0 => [m, lower, lower2],
                1 => [lower, m, lower2],
                _ => [lower2, lower, m],
            }
        }),
    );
}

/// C28 — exhaustive 10^3 sweep over one-step-past-range boundary values.
#[test]
fn c28_boundary_value_sweep() {
    let p = pair();
    let vals: [f32; 10] = [
        -0.0,
        0.0,
        f32::from_bits(0x8000_0001), // nextafter(0, -inf) : -smallest subnormal
        f32::from_bits(0x0000_0001), // nextafter(0, +inf) : +smallest subnormal
        1.0,
        f32::from_bits(1.0f32.to_bits() - 1), // nextafter(1, -inf)
        f32::from_bits(1.0f32.to_bits() + 1), // nextafter(1, +inf)
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
    ];
    let mut inputs = Vec::with_capacity(1000);
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                inputs.push([r, g, b]);
            }
        }
    }
    assert_eq!(inputs.len(), 1000);
    p.assert_batch("C28", inputs);
}

/// C29 — fully random raw 32-bit patterns (NaNs, infs, denormals, huge/tiny).
#[test]
fn c29_random_raw_bits() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 29);
    p.assert_batch(
        "C29",
        (0..N * 25).map(|_| [rng.raw_f32(), rng.raw_f32(), rng.raw_f32()]),
    );
}

/// C30 — tiny value alphabet so ties/equalities are frequent; stresses the
/// position-based tie-break and the `delta == 0 || max == 0` disjunction jointly.
#[test]
fn c30_tiny_alphabet_ties() {
    let p = pair();
    let alphabet: [f32; 7] = [-2.0, -1.0, -0.0, 0.0, 0.5, 1.0, 2.0];
    // Exhaustive 7^3 = 343 plus randomized draws from the same alphabet.
    let mut inputs = Vec::new();
    for &r in &alphabet {
        for &g in &alphabet {
            for &b in &alphabet {
                inputs.push([r, g, b]);
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..N {
        inputs.push([
            alphabet[rng.below(7)],
            alphabet[rng.below(7)],
            alphabet[rng.below(7)],
        ]);
    }
    p.assert_batch("C30", inputs);
}
