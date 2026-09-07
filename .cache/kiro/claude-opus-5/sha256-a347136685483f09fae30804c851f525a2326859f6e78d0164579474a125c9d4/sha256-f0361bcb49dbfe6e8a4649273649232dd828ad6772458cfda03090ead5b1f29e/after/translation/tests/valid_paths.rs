//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are reached only through `dlopen`/`dlsym` on their
//! respective `.so` files (see `tests/harness/mod.rs`); the Rust crate is never
//! linked in or called directly.

mod harness;

use harness::*;

/// Iterations per randomized row.
const N: usize = 4000;

// ---------------------------------------------------------------------------
// Triangle builders
// ---------------------------------------------------------------------------

/// A random, comfortably non-degenerate triangle at unit scale.
/// `ccw = true` forces positive winding, `false` forces negative.
fn triangle(rng: &mut Rng, scale: f32, ccw: bool) -> (Vec2, Vec2, Vec2) {
    loop {
        let p1 = Vec2::new(rng.scaled(scale), rng.scaled(scale));
        let p2 = Vec2::new(rng.scaled(scale), rng.scaled(scale));
        let p3 = Vec2::new(rng.scaled(scale), rng.scaled(scale));
        let cross = (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x);
        if !cross.is_finite() || cross.abs() < 0.05 * scale * scale {
            continue; // reject near-degenerate; row 7 covers that deliberately
        }
        return if (cross > 0.0) == ccw {
            (p1, p2, p3)
        } else {
            (p1, p3, p2)
        };
    }
}

/// A point strictly inside the triangle, as a random convex combination.
fn inside(rng: &mut Rng, p1: Vec2, p2: Vec2, p3: Vec2) -> Vec2 {
    let mut a = rng.unit01();
    let mut b = rng.unit01();
    if a + b > 1.0 {
        a = 1.0 - a;
        b = 1.0 - b;
    }
    let c = 1.0 - a - b;
    Vec2::new(
        a * p1.x + b * p2.x + c * p3.x,
        a * p1.y + b * p2.y + c * p3.y,
    )
}

// ---------------------------------------------------------------------------
// Row 1 — non-degenerate CCW, point inside, unit scale
// ---------------------------------------------------------------------------
#[test]
fn row01_ccw_inside_unit() {
    run_row("cfg01", N, |rng| {
        let (p1, p2, p3) = triangle(rng, 1.0, true);
        let p = inside(rng, p1, p2, p3);
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 2 — non-degenerate CW (reversed winding)
// ---------------------------------------------------------------------------
#[test]
fn row02_cw_inside_unit() {
    run_row("cfg02", N, |rng| {
        let (p1, p2, p3) = triangle(rng, 1.0, false);
        let p = inside(rng, p1, p2, p3);
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 3 — point far outside
// ---------------------------------------------------------------------------
#[test]
fn row03_outside() {
    run_row("cfg03", N, |rng| {
        let ccw = rng.bool();
        let (p1, p2, p3) = triangle(rng, 1.0, ccw);
        let p = Vec2::new(rng.scaled(1.0e4), rng.scaled(1.0e4));
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 4 — point exactly on a vertex
// ---------------------------------------------------------------------------
#[test]
fn row04_on_vertex() {
    run_row("cfg04", N, |rng| {
        let ccw = rng.bool();
        let (p1, p2, p3) = triangle(rng, 1.0, ccw);
        let p = match rng.below(3) {
            0 => p1,
            1 => p2,
            _ => p3,
        };
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 5 — point on an edge midpoint
// ---------------------------------------------------------------------------
#[test]
fn row05_on_edge() {
    run_row("cfg05", N, |rng| {
        let ccw = rng.bool();
        let (p1, p2, p3) = triangle(rng, 1.0, ccw);
        let (a, b) = match rng.below(3) {
            0 => (p1, p2),
            1 => (p2, p3),
            _ => (p3, p1),
        };
        let p = Vec2::new(0.5 * (a.x + b.x), 0.5 * (a.y + b.y));
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 6 — right-angled triangle: dot01 is exactly zero
// ---------------------------------------------------------------------------
#[test]
fn row06_right_angled_dot01_zero() {
    run_row("cfg06", N, |rng| {
        let p1 = Vec2::new(rng.scaled(4.0), rng.scaled(4.0));
        let a = rng.pow2(-6, 6);
        let b = rng.pow2(-6, 6);
        // v0 = (a, 0), v1 = (0, b)  =>  dot01 = a*0 + 0*b = +0.0 exactly
        let p3 = Vec2::new(p1.x + a, p1.y);
        let p2 = Vec2::new(p1.x, p1.y + b);
        let p = inside(rng, p1, p2, p3);
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 7 — needle-thin, near-degenerate: catastrophic cancellation in denom
// ---------------------------------------------------------------------------
#[test]
fn row07_needle_thin() {
    run_row("cfg07", N, |rng| {
        let p1 = Vec2::new(rng.scaled(1.0), rng.scaled(1.0));
        let v0 = Vec2::new(rng.scaled(1.0), rng.scaled(1.0));
        // v1 nearly parallel to v0
        let eps = 1.0e-6 * rng.unit();
        let k = 1.0 + rng.unit();
        let v1 = Vec2::new(v0.x * k + eps, v0.y * k - eps);
        let p3 = Vec2::new(p1.x + v0.x, p1.y + v0.y);
        let p2 = Vec2::new(p1.x + v1.x, p1.y + v1.y);
        let p = Vec2::new(rng.scaled(2.0), rng.scaled(2.0));
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 8 — exactly collinear: denom is exactly zero, division by zero
// ---------------------------------------------------------------------------
#[test]
fn row08_exactly_collinear() {
    run_row("cfg08", N, |rng| {
        let p1 = Vec2::new(rng.small_int(8), rng.small_int(8));
        // v0 axis-aligned, v1 = k * v0 with small exact integers so that
        // dot00*dot11 and dot01*dot01 are bit-identical => denom == +0.0
        let a = rng.small_int(8);
        let k = rng.small_int(8);
        let (v0, v1) = if rng.bool() {
            (Vec2::new(a, 0.0), Vec2::new(a * k, 0.0))
        } else {
            (Vec2::new(0.0, a), Vec2::new(0.0, a * k))
        };
        let p3 = Vec2::new(p1.x + v0.x, p1.y + v0.y);
        let p2 = Vec2::new(p1.x + v1.x, p1.y + v1.y);
        let p = Vec2::new(rng.small_int(8), rng.small_int(8));
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 9 — two vertices coincident (all three variants)
// ---------------------------------------------------------------------------
#[test]
fn row09_two_vertices_coincident() {
    run_row("cfg09", N, |rng| {
        let (a, b) = (
            Vec2::new(rng.scaled(4.0), rng.scaled(4.0)),
            Vec2::new(rng.scaled(4.0), rng.scaled(4.0)),
        );
        let p = Vec2::new(rng.scaled(4.0), rng.scaled(4.0));
        match rng.below(3) {
            0 => [a, a, b, p], // p1 == p2  => v1 == 0
            1 => [a, b, a, p], // p1 == p3  => v0 == 0
            _ => [a, b, b, p], // p2 == p3  => v0 == v1
        }
    });
}

// ---------------------------------------------------------------------------
// Row 10 — all three vertices coincident
// ---------------------------------------------------------------------------
#[test]
fn row10_all_vertices_coincident() {
    run_row("cfg10", N, |rng| {
        let a = Vec2::new(rng.scaled(4.0), rng.scaled(4.0));
        let p = Vec2::new(rng.scaled(4.0), rng.scaled(4.0));
        [a, a, a, p]
    });
}

// ---------------------------------------------------------------------------
// Row 11 — exactly representable small integers, no rounding anywhere
// ---------------------------------------------------------------------------
#[test]
fn row11_small_integers() {
    run_row("cfg11", N, |rng| {
        [
            rng.vec_with(|r| r.small_int(16)),
            rng.vec_with(|r| r.small_int(16)),
            rng.vec_with(|r| r.small_int(16)),
            rng.vec_with(|r| r.small_int(16)),
        ]
    });
}

// ---------------------------------------------------------------------------
// Row 12 — powers of two only: exact products, wide exponent range
// ---------------------------------------------------------------------------
#[test]
fn row12_powers_of_two() {
    run_row("cfg12", N, |rng| {
        [
            rng.vec_with(|r| r.pow2(-30, 30)),
            rng.vec_with(|r| r.pow2(-30, 30)),
            rng.vec_with(|r| r.pow2(-30, 30)),
            rng.vec_with(|r| r.pow2(-30, 30)),
        ]
    });
}

// ---------------------------------------------------------------------------
// Row 13 — large magnitudes: dot products overflow to inf, inf-inf = NaN
// ---------------------------------------------------------------------------
#[test]
fn row13_large_magnitude_overflow() {
    run_row("cfg13", N, |rng| {
        [
            rng.vec_with(|r| r.scaled(1.0e18)),
            rng.vec_with(|r| r.scaled(1.0e18)),
            rng.vec_with(|r| r.scaled(1.0e18)),
            rng.vec_with(|r| r.scaled(1.0e18)),
        ]
    });
}

// ---------------------------------------------------------------------------
// Row 14 — tiny magnitudes: products underflow to subnormal then to zero
// ---------------------------------------------------------------------------
#[test]
fn row14_tiny_magnitude_underflow() {
    run_row("cfg14", N, |rng| {
        [
            rng.vec_with(|r| r.scaled(1.0e-20)),
            rng.vec_with(|r| r.scaled(1.0e-20)),
            rng.vec_with(|r| r.scaled(1.0e-20)),
            rng.vec_with(|r| r.scaled(1.0e-20)),
        ]
    });
}

// ---------------------------------------------------------------------------
// Row 15 — mixed huge and tiny in the same call
// ---------------------------------------------------------------------------
#[test]
fn row15_mixed_scales() {
    run_row("cfg15", N, |rng| {
        let mut vs = [
            rng.vec_with(|r| r.scaled(1.0)),
            rng.vec_with(|r| r.scaled(1.0)),
            rng.vec_with(|r| r.scaled(1.0)),
            rng.vec_with(|r| r.scaled(1.0)),
        ];
        let big = rng.below(8) as usize;
        let tiny = rng.below(8) as usize;
        let bv = rng.scaled(1.0e20);
        let tv = rng.scaled(1.0e-20);
        set_slot(&mut vs, big, bv);
        set_slot(&mut vs, tiny, tv);
        vs
    });
}

// ---------------------------------------------------------------------------
// Row 16 — maximal finite / minimal normal magnitudes
// ---------------------------------------------------------------------------
#[test]
fn row16_extreme_finite() {
    const POOL: &[f32] = &[
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0,
        -1.0,
    ];
    run_row("cfg16", N, |rng| {
        let pick = |r: &mut Rng| POOL[r.below(POOL.len() as u32) as usize];
        [
            rng.vec_with(pick),
            rng.vec_with(pick),
            rng.vec_with(pick),
            rng.vec_with(pick),
        ]
    });
}

// ---------------------------------------------------------------------------
// Row 17 — subnormal inputs (detects an FTZ/DAZ mismatch)
// ---------------------------------------------------------------------------
#[test]
fn row17_subnormals() {
    const POOL: &[f32] = &[
        SUBNORMAL_MIN,
        -SUBNORMAL_MIN,
        SUBNORMAL_MAX,
        -SUBNORMAL_MAX,
        f32::MIN_POSITIVE,
        0.0,
    ];
    run_row("cfg17", N, |rng| {
        let pick = |r: &mut Rng| POOL[r.below(POOL.len() as u32) as usize];
        [
            rng.vec_with(pick),
            rng.vec_with(pick),
            rng.vec_with(pick),
            rng.vec_with(pick),
        ]
    });
}

// ---------------------------------------------------------------------------
// Row 18 — exhaustive: all 2^8 signed-zero sign combinations
// ---------------------------------------------------------------------------
#[test]
fn row18_signed_zeros_exhaustive() {
    let mut cases = Vec::with_capacity(256);
    for mask in 0u32..256 {
        let mut vs = [Vec2::new(0.0, 0.0); 4];
        for slot in 0..8 {
            let v = if mask & (1 << slot) != 0 { -0.0f32 } else { 0.0 };
            set_slot(&mut vs, slot, v);
        }
        cases.push(vs);
    }
    run_cases("cfg18", &cases);
}

// ---------------------------------------------------------------------------
// Row 19 — one infinity, every slot, both signs (exhaustive over slot/sign,
// randomized over the remaining components)
// ---------------------------------------------------------------------------
#[test]
fn row19_single_infinity() {
    let mut rng = Rng::for_row("cfg19");
    let mut cases = Vec::new();
    for slot in 0..8usize {
        for &s in &[f32::INFINITY, f32::NEG_INFINITY] {
            for _ in 0..64 {
                let mut vs = [
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                ];
                set_slot(&mut vs, slot, s);
                cases.push(vs);
            }
        }
    }
    run_cases("cfg19", &cases);
}

// ---------------------------------------------------------------------------
// Row 20 — several infinities at once (random slot mask)
// ---------------------------------------------------------------------------
#[test]
fn row20_multiple_infinities() {
    run_row("cfg20", N, |rng| {
        let mut vs = [
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
        ];
        let mask = rng.next_u32() & 0xff;
        for slot in 0..8 {
            if mask & (1 << slot) != 0 {
                let v = if rng.bool() {
                    f32::INFINITY
                } else {
                    f32::NEG_INFINITY
                };
                set_slot(&mut vs, slot, v);
            }
        }
        vs
    });
}

// ---------------------------------------------------------------------------
// Row 21 — a quiet NaN in each slot
// ---------------------------------------------------------------------------
#[test]
fn row21_quiet_nan_each_slot() {
    let mut rng = Rng::for_row("cfg21");
    let mut cases = Vec::new();
    for slot in 0..8usize {
        for _ in 0..128 {
            let mut vs = [
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
            ];
            set_slot(&mut vs, slot, QNAN);
            cases.push(vs);
        }
    }
    run_cases("cfg21", &cases);
}

// ---------------------------------------------------------------------------
// Row 22 — a signalling NaN in each slot (quieting behaviour)
// ---------------------------------------------------------------------------
#[test]
fn row22_signalling_nan_each_slot() {
    let mut rng = Rng::for_row("cfg22");
    let mut cases = Vec::new();
    for slot in 0..8usize {
        for &n in &[SNAN, SNAN_NEG, f32::from_bits(0x7f80_1234)] {
            for _ in 0..48 {
                let mut vs = [
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                ];
                set_slot(&mut vs, slot, n);
                cases.push(vs);
            }
        }
    }
    run_cases("cfg22", &cases);
}

// ---------------------------------------------------------------------------
// Row 23 — negative NaN in each slot (sign propagation)
// ---------------------------------------------------------------------------
#[test]
fn row23_negative_nan_each_slot() {
    let mut rng = Rng::for_row("cfg23");
    let mut cases = Vec::new();
    for slot in 0..8usize {
        for &n in &[QNAN_NEG, NAN_C, f32::from_bits(0xffff_ffff)] {
            for _ in 0..48 {
                let mut vs = [
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                    rng.vec_with(|r| r.scaled(4.0)),
                ];
                set_slot(&mut vs, slot, n);
                cases.push(vs);
            }
        }
    }
    run_cases("cfg23", &cases);
}

// ---------------------------------------------------------------------------
// Row 24 — multiple DISTINCT NaN payloads: the operand-order discriminator
// ---------------------------------------------------------------------------
#[test]
fn row24_distinct_nan_payloads() {
    const NANS: &[f32] = &[QNAN, QNAN_NEG, NAN_A, NAN_B, NAN_C, NAN_D, SNAN, SNAN_NEG];

    // Exhaustive over every ordered pair of slots with two distinct payloads.
    let mut rng = Rng::for_row("cfg24-pairs");
    let mut cases = Vec::new();
    for s1 in 0..8usize {
        for s2 in 0..8usize {
            if s1 == s2 {
                continue;
            }
            let mut vs = [
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
            ];
            set_slot(&mut vs, s1, NAN_A);
            set_slot(&mut vs, s2, NAN_B);
            cases.push(vs);
        }
    }
    run_cases("cfg24-pairs", &cases);

    // Randomized: every slot independently either a random normal or a random
    // distinct NaN payload.
    run_row("cfg24-random", N, |rng| {
        let mut vs = [
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
        ];
        for slot in 0..8 {
            if rng.below(3) == 0 {
                // random NaN with a random payload and random sign
                let payload = rng.next_u32() & 0x007f_ffff;
                let sign = if rng.bool() { 0x8000_0000u32 } else { 0 };
                let bits = sign | 0x7f80_0000 | payload.max(1);
                set_slot(&mut vs, slot, f32::from_bits(bits));
            } else if rng.below(4) == 0 {
                let n = NANS[rng.below(NANS.len() as u32) as usize];
                set_slot(&mut vs, slot, n);
            }
        }
        vs
    });
}

// ---------------------------------------------------------------------------
// Row 25 — fully unstructured: all 32 bits of all 8 components random
// ---------------------------------------------------------------------------
#[test]
fn row25_uniform_random_bits() {
    run_row("cfg25", 40_000, |rng| {
        [
            rng.vec_with(|r| r.any_bits()),
            rng.vec_with(|r| r.any_bits()),
            rng.vec_with(|r| r.any_bits()),
            rng.vec_with(|r| r.any_bits()),
        ]
    });
}

// ---------------------------------------------------------------------------
// Row 26 — lm_sub2 exact-cancellation path
// ---------------------------------------------------------------------------
#[test]
fn row26_sub2_cancellation() {
    run_row("cfg26", N, |rng| {
        let p1 = rng.vec_with(|r| r.scaled(4.0));
        let mut vs = [
            p1,
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
        ];
        // Force one or more components of p2/p3/p to equal the matching
        // component of p1, so that lm_sub2 produces an exact +-0.0 lane.
        for i in 1..4 {
            if rng.bool() {
                vs[i].x = p1.x;
            }
            if rng.bool() {
                vs[i].y = p1.y;
            }
        }
        vs
    });
}

// ---------------------------------------------------------------------------
// Row 27 — lm_dot2 sign-cancellation path (dot01 cancels to +-0.0)
// ---------------------------------------------------------------------------
#[test]
fn row27_dot2_cancellation() {
    run_row("cfg27", N, |rng| {
        let p1 = rng.vec_with(|r| r.small_int(8));
        let a = rng.pow2(-4, 4);
        let t = rng.pow2(-4, 4);
        // v0 = (a, a), v1 = (t, -t)  =>  dot01 = a*t + a*(-t) = +-0.0 exactly
        let p3 = Vec2::new(p1.x + a, p1.y + a);
        let p2 = Vec2::new(p1.x + t, p1.y - t);
        let p = rng.vec_with(|r| r.small_int(8));
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 28 — full-mantissa components: every product and sum is inexact
// ---------------------------------------------------------------------------
#[test]
fn row28_full_mantissa_rounding() {
    run_row("cfg28", N, |rng| {
        // exponent kept moderate, mantissa fully random => 24 significant bits
        let pick = |r: &mut Rng| {
            let exp = 100u32 + r.below(56); // biased exponent 100..155
            let mant = r.next_u32() & 0x007f_ffff;
            let sign = if r.bool() { 0x8000_0000u32 } else { 0 };
            f32::from_bits(sign | (exp << 23) | mant)
        };
        [
            rng.vec_with(pick),
            rng.vec_with(pick),
            rng.vec_with(pick),
            rng.vec_with(pick),
        ]
    });
}

// ---------------------------------------------------------------------------
// Row 29 — reciprocal tail: denom an exact power of two vs. not
// ---------------------------------------------------------------------------
#[test]
fn row29_reciprocal_exact_vs_inexact() {
    // Right-angled triangle: denom = a^2 * b^2 exactly.
    run_row("cfg29-pow2", N, |rng| {
        let p1 = rng.vec_with(|r| r.small_int(4));
        let a = rng.pow2(-8, 8);
        let b = rng.pow2(-8, 8);
        let p3 = Vec2::new(p1.x + a, p1.y);
        let p2 = Vec2::new(p1.x, p1.y + b);
        let p = inside(rng, p1, p2, p3);
        [p1, p2, p3, p]
    });
    run_row("cfg29-inexact", N, |rng| {
        let p1 = rng.vec_with(|r| r.small_int(4));
        // 3 and 7 give denominators whose reciprocal is not representable
        let a = 3.0 * rng.pow2(-4, 4);
        let b = 7.0 * rng.pow2(-4, 4);
        let p3 = Vec2::new(p1.x + a, p1.y);
        let p2 = Vec2::new(p1.x, p1.y + b);
        let p = inside(rng, p1, p2, p3);
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 30 — reciprocal tail: 1.0f/denom overflows (denom subnormal)
// ---------------------------------------------------------------------------
#[test]
fn row30_reciprocal_overflow() {
    run_row("cfg30", N, |rng| {
        let p1 = rng.vec_with(|r| r.scaled(1.0e-11));
        // both legs ~1e-11 => denom ~1e-44, subnormal => 1/denom = +inf
        let a = rng.scaled(1.0e-11);
        let b = rng.scaled(1.0e-11);
        let p3 = Vec2::new(p1.x + a, p1.y);
        let p2 = Vec2::new(p1.x, p1.y + b);
        let p = rng.vec_with(|r| r.scaled(1.0e-11));
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 31 — reciprocal tail: 1.0f/denom underflows to subnormal / +0.0
// ---------------------------------------------------------------------------
#[test]
fn row31_reciprocal_underflow() {
    run_row("cfg31", N, |rng| {
        let p1 = rng.vec_with(|r| r.scaled(1.0e9));
        // legs ~1e9 and ~1e10 => denom ~1e38 => 1/denom subnormal or 0
        let a = rng.scaled(1.0e9) + 1.0e9;
        let b = rng.scaled(1.0e10) + 1.0e10;
        let p3 = Vec2::new(p1.x + a, p1.y);
        let p2 = Vec2::new(p1.x, p1.y + b);
        let p = rng.vec_with(|r| r.scaled(1.0e9));
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 32 — numerator exactly +-0.0 (p == p1 => v2 == 0)
// ---------------------------------------------------------------------------
#[test]
fn row32_zero_numerator() {
    run_row("cfg32", N, |rng| {
        let ccw = rng.bool();
        let (p1, p2, p3) = triangle(rng, 4.0, ccw);
        // p == p1 exactly, optionally with the sign of the zero flipped by
        // using -0.0 components where p1 has 0.0
        [p1, p2, p3, p1]
    });
}

// ---------------------------------------------------------------------------
// Row 33 — numerator +-inf with invDenom == +-0.0  => inf * 0 = NaN
// ---------------------------------------------------------------------------
#[test]
fn row33_inf_times_zero_tail() {
    run_row("cfg33", N, |rng| {
        // Huge legs: denom overflows to +-inf => invDenom == +-0.0, while the
        // u/v numerators also overflow => inf * 0 in the final multiply.
        let p1 = rng.vec_with(|r| r.scaled(1.0e20));
        let a = rng.scaled(1.0e20) + 1.0e20;
        let b = rng.scaled(1.0e20) + 1.0e20;
        let p3 = Vec2::new(a, p1.y);
        let p2 = Vec2::new(p1.x, b);
        let p = rng.vec_with(|r| r.scaled(1.0e20));
        [p1, p2, p3, p]
    });
}

// ---------------------------------------------------------------------------
// Row 34 — argument-slot permutation sweep (no p1/p2/p3/p mix-up)
// ---------------------------------------------------------------------------
#[test]
fn row34_argument_permutations() {
    const PERMS: [[usize; 4]; 24] = [
        [0, 1, 2, 3],
        [0, 1, 3, 2],
        [0, 2, 1, 3],
        [0, 2, 3, 1],
        [0, 3, 1, 2],
        [0, 3, 2, 1],
        [1, 0, 2, 3],
        [1, 0, 3, 2],
        [1, 2, 0, 3],
        [1, 2, 3, 0],
        [1, 3, 0, 2],
        [1, 3, 2, 0],
        [2, 0, 1, 3],
        [2, 0, 3, 1],
        [2, 1, 0, 3],
        [2, 1, 3, 0],
        [2, 3, 0, 1],
        [2, 3, 1, 0],
        [3, 0, 1, 2],
        [3, 0, 2, 1],
        [3, 1, 0, 2],
        [3, 1, 2, 0],
        [3, 2, 0, 1],
        [3, 2, 1, 0],
    ];
    let mut rng = Rng::for_row("cfg34");
    let mut cases = Vec::new();
    for _ in 0..200 {
        let base = [
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
            rng.vec_with(|r| r.scaled(4.0)),
        ];
        for perm in PERMS {
            cases.push([
                base[perm[0]],
                base[perm[1]],
                base[perm[2]],
                base[perm[3]],
            ]);
        }
    }
    run_cases("cfg34", &cases);
}

// ---------------------------------------------------------------------------
// Row 35 — struct/ABI shape parity and dirty XMM upper halves
// ---------------------------------------------------------------------------
#[test]
fn row35_abi_struct_shape() {
    // `struct { float x, y; }` is 8 bytes, 4-byte aligned, no padding.
    assert_eq!(std::mem::size_of::<Vec2>(), 8, "sizeof(lm_vec2)");
    assert_eq!(std::mem::align_of::<Vec2>(), 4, "_Alignof(lm_vec2)");

    #[cfg(target_arch = "x86_64")]
    {
        let im = impls();
        let mut rng = Rng::for_row("cfg35");
        let garbages: [[u32; 2]; 4] = [
            [0, 0],
            [0xffff_ffff, 0xffff_ffff],
            [0x7fc0_0000, 0x7f80_0000],
            [0xdead_beef, 0xcafe_babe],
        ];
        let mut failures = Vec::new();
        for _ in 0..500 {
            let vs = [
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
                rng.vec_with(|r| r.scaled(4.0)),
            ];
            let clean_c = call_c(vs[0], vs[1], vs[2], vs[3]);
            for g in garbages {
                let dirty_c = call_dirty(im.c_addr, vs, g);
                let dirty_r = call_dirty(im.rust_addr, vs, g);
                if dirty_c.to_bits() != dirty_r.to_bits()
                    || dirty_c.to_bits() != clean_c.to_bits()
                {
                    failures.push(format!(
                        "garbage={g:08x?} vs={vs:?} clean_C={clean_c:?} dirty_C={dirty_c:?} dirty_Rust={dirty_r:?}"
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "cfg35: {} dirty-XMM cases diverged\n{}",
            failures.len(),
            failures[..failures.len().min(3)].join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// Row 36 — purity / reentrancy: repeated and concurrent calls agree
// ---------------------------------------------------------------------------
#[test]
fn row36_purity_and_threads() {
    let mut rng = Rng::for_row("cfg36");
    let inputs: Vec<[Vec2; 4]> = (0..500)
        .map(|_| {
            [
                rng.vec_with(|r| r.any_bits()),
                rng.vec_with(|r| r.any_bits()),
                rng.vec_with(|r| r.any_bits()),
                rng.vec_with(|r| r.any_bits()),
            ]
        })
        .collect();

    // Reference results, single-threaded.
    let expected: Vec<(u32, u32)> = inputs
        .iter()
        .map(|&[a, b, c, d]| call_c(a, b, c, d).to_bits())
        .collect();

    // Repeat: no hidden state may change the answer.
    for (i, &[a, b, c, d]) in inputs.iter().enumerate() {
        assert_eq!(call_c(a, b, c, d).to_bits(), expected[i], "C not pure");
        assert_eq!(call_rust(a, b, c, d).to_bits(), expected[i], "Rust != C");
    }

    // Concurrent: 8 threads hammering both .so exports.
    let inputs = std::sync::Arc::new(inputs);
    let expected = std::sync::Arc::new(expected);
    let mut handles = Vec::new();
    for _ in 0..8 {
        let inputs = inputs.clone();
        let expected = expected.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..20 {
                for (i, &[a, b, c, d]) in inputs.iter().enumerate() {
                    assert_eq!(call_c(a, b, c, d).to_bits(), expected[i]);
                    assert_eq!(call_rust(a, b, c, d).to_bits(), expected[i]);
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread");
    }
}

// ---------------------------------------------------------------------------
// Row 37 — feature-combination axis: assert there is exactly one
// ---------------------------------------------------------------------------
#[test]
fn row37_feature_surface_is_empty() {
    // Every Cargo feature becomes a `feature = "..."` cfg. There is no
    // `[features]` table in Cargo.toml, so the crate has exactly one
    // configuration; this test fails loudly if that ever changes and the
    // Phase B/C matrix needs re-running per feature.
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    assert!(
        !manifest.contains("[features]"),
        "Cargo.toml gained a [features] table: re-run Phases B and C for every \
         feature combination and update CONFIGS.md"
    );
    // Sanity: both libraries really were loaded from disk.
    let im = impls();
    assert!(im.c_path.exists() && im.rust_path.exists());
}
