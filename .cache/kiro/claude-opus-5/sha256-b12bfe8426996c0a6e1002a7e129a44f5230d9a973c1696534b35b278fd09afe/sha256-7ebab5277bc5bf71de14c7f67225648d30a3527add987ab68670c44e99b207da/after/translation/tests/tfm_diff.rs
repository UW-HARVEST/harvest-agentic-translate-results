//! Differential tests for `tfm`, comparing the C `.so` against the Rust `.so`.
//!
//! Ordered from the lowest-level behaviour (the scalar arithmetic on a single
//! entry, which exercises the `sqd`/`lambda`/clamp helpers) up to whole-array
//! iteration and pointer advancement.

mod common;

use common::{check, check_one, check_with, Pair, Rng, EDGE_VALUES};

/// The Rust `.so` must export everything the C `.so` exports.
#[test]
fn exports_present() {
    // `Pair::load` resolves `tfm` in both libraries and panics otherwise.
    let _ = Pair::load();
}

// ---------------------------------------------------------------------------
// Level 0: count handling / no-op cases
// ---------------------------------------------------------------------------

#[test]
fn count_zero_and_negative_write_nothing() {
    let pair = Pair::load();
    let src = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
    for count in [0i32, -1, -7, i32::MIN] {
        // Destination is all sentinel and must stay that way.
        check_with(&pair, &format!("count={count}"), &src, count, 8);
    }
}

#[test]
fn count_zero_with_null_pointers() {
    let pair = Pair::load();
    // Neither implementation may dereference the pointers when count <= 0.
    for count in [0i32, -3] {
        unsafe {
            (pair.c_tfm)(std::ptr::null_mut(), std::ptr::null(), count);
            (pair.rust_tfm)(std::ptr::null_mut(), std::ptr::null(), count);
        }
    }
}

// ---------------------------------------------------------------------------
// Level 1: single entry, both branches of `src[0] < src[1]`
// ---------------------------------------------------------------------------

#[test]
fn single_entry_then_branch() {
    let pair = Pair::load();
    // src[0] < src[1] -> dest = [dx2 - lambda, dxy]
    check_one(&pair, "then/simple", [1.0, 4.0, 2.0]);
    check_one(&pair, "then/zero_dxy", [1.0, 4.0, 0.0]);
    check_one(&pair, "then/neg", [-5.0, -1.0, 3.5]);
    check_one(&pair, "then/tiny_gap", [1.0, 1.0 + f32::EPSILON, 1.0]);
}

#[test]
fn single_entry_else_branch() {
    let pair = Pair::load();
    // src[0] >= src[1] -> operands swap and dest = [dxy, dx2 - lambda]
    check_one(&pair, "else/equal", [2.0, 2.0, 1.0]);
    check_one(&pair, "else/greater", [9.0, 4.0, 2.0]);
    check_one(&pair, "else/neg", [-1.0, -5.0, 3.5]);
    // -0.0 < 0.0 is false in C, so this must take the else branch.
    check_one(&pair, "else/neg_zero_vs_zero", [-0.0, 0.0, 1.0]);
    check_one(&pair, "else/zero_vs_neg_zero", [0.0, -0.0, 1.0]);
}

/// A NaN in either comparison operand makes `<` false, forcing the else branch.
#[test]
fn nan_comparison_takes_else_branch() {
    let pair = Pair::load();
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_1234),
    ];
    for (i, &n) in nans.iter().enumerate() {
        check_one(&pair, &format!("nan_lhs_{i}"), [n, 1.0, 2.0]);
        check_one(&pair, &format!("nan_rhs_{i}"), [1.0, n, 2.0]);
        check_one(&pair, &format!("nan_dxy_{i}"), [1.0, 2.0, n]);
        check_one(&pair, &format!("nan_both_{i}"), [n, n, n]);
    }
}

// ---------------------------------------------------------------------------
// Level 2: the arithmetic helpers, isolated
// ---------------------------------------------------------------------------

/// `sqd = (dy2-dx2)^2 + 4*dxy^2` is non-negative when computed exactly, but the
/// rounded `f32` evaluation order in the C can produce a negative value, which
/// is then clamped to 0 before `sqrtf`. Feed inputs that land on that path.
#[test]
fn negative_sqd_is_clamped() {
    let pair = Pair::load();
    // dx2 == dy2 and dxy == 0 gives sqd == 0 exactly.
    check_one(&pair, "clamp/exact_zero", [3.0, 3.0, 0.0]);
    // Cancellation: (dy2*dy2) - (2*dx2*dy2) + (dx2*dx2) with nearly equal
    // large operands rounds to a negative result.
    for &v in &[
        1.0e18f32, 1.0e20, 3.3554432e7, 1.6777216e7, 1.0e30, 5.0e-20, 1.0,
    ] {
        let near = f32::from_bits(v.to_bits() + 1);
        check_one(&pair, "clamp/cancel_a", [v, near, 0.0]);
        check_one(&pair, "clamp/cancel_b", [near, v, 0.0]);
        check_one(&pair, "clamp/cancel_c", [v, v, f32::from_bits(1)]);
    }
    // Negative-zero sqd must survive as -0.0 through sqrt (0 > -0.0 is false).
    check_one(&pair, "clamp/neg_zero_sqd", [0.0, -0.0, 0.0]);
    check_one(&pair, "clamp/neg_zero_sqd2", [-0.0, -0.0, -0.0]);
}

/// Overflow inside `sqd` (and hence `sqrtf`) must match, including infinities
/// and inf - inf = NaN in `dx2 - lambda`.
#[test]
fn overflow_and_infinities() {
    let pair = Pair::load();
    let big = [
        f32::MAX,
        f32::MIN,
        1e30f32,
        -1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e38,
        -1e38,
    ];
    for (i, &a) in big.iter().enumerate() {
        for (j, &b) in big.iter().enumerate() {
            for (k, &c) in big.iter().enumerate() {
                check_one(&pair, &format!("big/{i}_{j}_{k}"), [a, b, c]);
            }
        }
    }
}

/// Subnormal and underflow behaviour.
#[test]
fn subnormals_and_underflow() {
    let pair = Pair::load();
    let small = [
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-40,
        -1e-40,
        1e-45,
    ];
    for (i, &a) in small.iter().enumerate() {
        for (j, &b) in small.iter().enumerate() {
            for (k, &c) in small.iter().enumerate() {
                check_one(&pair, &format!("small/{i}_{j}_{k}"), [a, b, c]);
            }
        }
    }
}

/// Full cartesian sweep over the interesting single-float values.
#[test]
fn edge_value_cartesian() {
    let pair = Pair::load();
    for (i, &a) in EDGE_VALUES.iter().enumerate() {
        for (j, &b) in EDGE_VALUES.iter().enumerate() {
            for (k, &c) in EDGE_VALUES.iter().enumerate() {
                check_one(&pair, &format!("edge/{i}_{j}_{k}"), [a, b, c]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 3: randomized differential testing
// ---------------------------------------------------------------------------

#[test]
fn random_bit_patterns() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xC0FFEE);
    // Arbitrary bit patterns: mostly NaN/inf/huge, good for the clamp and the
    // NaN-propagation paths.
    for round in 0..2000 {
        let count = 32i32;
        let src: Vec<f32> = (0..count as usize * 3)
            .map(|_| rng.next_f32_bits())
            .collect();
        check(&pair, &format!("randbits/{round}"), &src, count);
    }
}

#[test]
fn random_realistic_values() {
    let pair = Pair::load();
    let mut rng = Rng::new(42);
    for round in 0..2000 {
        // Vary the magnitude scale so both the well-conditioned and the
        // catastrophic-cancellation regimes get hit.
        let scale = match round % 6 {
            0 => 1.0f32,
            1 => 1e-20,
            2 => 1e20,
            3 => 1e38,
            4 => 1e-38,
            _ => 1e6,
        };
        let count = 16i32;
        let src: Vec<f32> = (0..count as usize * 3)
            .map(|_| rng.next_f32_scaled(scale))
            .collect();
        check(&pair, &format!("randreal/{round}/{scale:e}"), &src, count);
    }
}

/// Structure-tensor-like inputs: dx2, dy2 non-negative, dxy correlated. This is
/// the regime the function is actually used in.
#[test]
fn random_structure_tensor_like() {
    let pair = Pair::load();
    let mut rng = Rng::new(7);
    for round in 0..2000 {
        let count = 16i32;
        let mut src = Vec::with_capacity(count as usize * 3);
        for _ in 0..count {
            let gx = rng.next_f32_scaled(4.0);
            let gy = rng.next_f32_scaled(4.0);
            src.push(gx * gx);
            src.push(gy * gy);
            src.push(gx * gy);
        }
        check(&pair, &format!("tensor/{round}"), &src, count);
    }
}

/// Near-equal dx2/dy2 pairs, where the branch decision and the cancellation in
/// `sqd` are both most delicate.
#[test]
fn near_equal_pairs() {
    let pair = Pair::load();
    let mut rng = Rng::new(99);
    for round in 0..2000 {
        let count = 8i32;
        let mut src = Vec::with_capacity(count as usize * 3);
        for _ in 0..count {
            let base = rng.next_f32_scaled(1e6);
            let delta = (rng.next_u32() % 5) as i32 - 2;
            let other = f32::from_bits((base.to_bits() as i64 + delta as i64) as u32);
            src.push(base);
            src.push(other);
            src.push(rng.next_f32_scaled(1e-3));
        }
        check(&pair, &format!("neareq/{round}"), &src, count);
    }
}

// ---------------------------------------------------------------------------
// Level 4: iteration, pointer advancement, buffer sizes
// ---------------------------------------------------------------------------

/// `src` advances by 3 and `dest` by 2 per iteration; verify for many counts
/// that every slot lands in the right place and nothing past `2*count` is
/// written.
#[test]
fn pointer_advancement_various_counts() {
    let pair = Pair::load();
    let mut rng = Rng::new(2024);
    for count in 1..=64i32 {
        let src: Vec<f32> = (0..count as usize * 3)
            .map(|_| rng.next_f32_scaled(10.0))
            .collect();
        // Extra guard slots catch any overrun of the destination.
        check_with(
            &pair,
            &format!("advance/{count}"),
            &src,
            count,
            count as usize * 2 + 8,
        );
    }
}

/// A single mixed array whose entries alternate between the two branches.
#[test]
fn alternating_branches() {
    let pair = Pair::load();
    let count = 100i32;
    let mut src = Vec::new();
    for i in 0..count {
        if i % 2 == 0 {
            src.extend_from_slice(&[1.0, 5.0, 0.25 * i as f32]);
        } else {
            src.extend_from_slice(&[5.0, 1.0, -0.25 * i as f32]);
        }
    }
    check(&pair, "alternating", &src, count);
}

#[test]
fn large_count() {
    let pair = Pair::load();
    let mut rng = Rng::new(555);
    let count = 100_000i32;
    let src: Vec<f32> = (0..count as usize * 3)
        .map(|_| rng.next_f32_scaled(100.0))
        .collect();
    check(&pair, "large", &src, count);
}

/// Unaligned-ish starts: run from an offset inside a bigger allocation so the
/// pointers are not 16-byte aligned, in case either side vectorizes.
#[test]
fn offset_pointers() {
    let pair = Pair::load();
    let mut rng = Rng::new(13);
    let count = 37i32;
    let full: Vec<f32> = (0..count as usize * 3 + 4)
        .map(|_| rng.next_f32_scaled(3.0))
        .collect();
    for skip in 0..4usize {
        let src = &full[skip..skip + count as usize * 3];
        check(&pair, &format!("offset/{skip}"), src, count);
    }
}

// ---------------------------------------------------------------------------
// Level 5: NaN payload propagation
// ---------------------------------------------------------------------------

/// Distinct NaN payloads plus magnitudes that make intermediate terms overflow
/// to infinity. When two different NaNs (or a freshly generated invalid-op NaN
/// and an input NaN) meet in one operation, the surviving payload depends on the
/// operand order of the underlying SSE instruction, so this is the sharpest
/// probe of the arithmetic transcription.
#[test]
fn nan_payload_propagation() {
    let pair = Pair::load();
    let nans = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0xFFED_4C32),
        f32::from_bits(0x7F80_0001), // signalling
        f32::from_bits(0xFF80_0001), // signalling, negative
        f32::from_bits(0x7FFF_FFFF),
        f32::from_bits(0xFFFF_FFFF),
    ];
    // Values that overflow when squared or cross-multiplied, so that the
    // non-NaN part of `sqd` becomes an invalid `inf - inf`.
    let overflowing = [
        1.0f32,
        0.0,
        -0.0,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e30,
        -1e30,
        7.386899e27,
        1.07478574e21,
    ];

    // One NaN, in each of the three slots, against every overflowing pair.
    for (n_i, &n) in nans.iter().enumerate() {
        for (i, &a) in overflowing.iter().enumerate() {
            for (j, &b) in overflowing.iter().enumerate() {
                check_one(&pair, &format!("nanpay/dxy/{n_i}_{i}_{j}"), [a, b, n]);
                check_one(&pair, &format!("nanpay/dx2/{n_i}_{i}_{j}"), [n, a, b]);
                check_one(&pair, &format!("nanpay/dy2/{n_i}_{i}_{j}"), [a, n, b]);
            }
        }
    }

    // Two or three different NaNs at once.
    for (i, &a) in nans.iter().enumerate() {
        for (j, &b) in nans.iter().enumerate() {
            for (k, &c) in nans.iter().enumerate() {
                check_one(&pair, &format!("nanpay/triple/{i}_{j}_{k}"), [a, b, c]);
            }
            for (k, &c) in overflowing.iter().enumerate() {
                check_one(&pair, &format!("nanpay/pair_c/{i}_{j}_{k}"), [a, b, c]);
                check_one(&pair, &format!("nanpay/pair_a/{i}_{j}_{k}"), [c, a, b]);
                check_one(&pair, &format!("nanpay/pair_b/{i}_{j}_{k}"), [a, c, b]);
            }
        }
    }
}

/// Randomized NaN-payload fuzzing: every input slot is either a random NaN, an
/// infinity, or a huge finite value, maximizing the chance of two competing
/// payloads inside one operation.
#[test]
fn random_nan_payload_fuzz() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xDEADBEEF);
    for round in 0..4000 {
        let count = 8i32;
        let mut src = Vec::with_capacity(count as usize * 3);
        for _ in 0..count as usize * 3 {
            let v = match rng.next_u32() % 5 {
                0 => f32::from_bits(0x7F80_0000 | (rng.next_u32() & 0x807F_FFFF)), // NaN/inf
                1 => f32::from_bits(0xFF80_0000 | (rng.next_u32() & 0x007F_FFFF)),
                2 => f32::INFINITY,
                3 => f32::NEG_INFINITY,
                _ => f32::from_bits(rng.next_u32() | 0x7000_0000), // huge magnitudes
            };
            src.push(v);
        }
        check(&pair, &format!("nanfuzz/{round}"), &src, count);
    }
}

/// Exhaustive sweep of the exponent field with a fixed mantissa: catches any
/// overflow/underflow threshold handled differently by the two builds.
#[test]
fn exponent_sweep() {
    let pair = Pair::load();
    let mantissas = [0x0000_0000u32, 0x0040_0000, 0x007F_FFFF, 0x0012_3456];
    for &m in mantissas.iter() {
        for e in 0u32..=255 {
            for &sign in &[0u32, 0x8000_0000] {
                let v = f32::from_bits(sign | (e << 23) | m);
                // Pair it with a few partners covering both branches.
                for (p, &other) in [1.0f32, -1.0, 0.0, v, f32::MAX].iter().enumerate() {
                    check_one(&pair, &format!("expsweep/{m:x}_{e}_{sign:x}_{p}"), [v, other, v]);
                    check_one(&pair, &format!("expsweep_r/{m:x}_{e}_{sign:x}_{p}"), [other, v, v]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 6: overlapping buffers
// ---------------------------------------------------------------------------

/// The C reads three floats and then writes two per iteration, with `src`
/// advancing faster than `dest`. Callers may therefore transform in place; make
/// sure the Rust does the reads and writes in the same order.
#[test]
fn overlapping_dest_and_src() {
    let pair = Pair::load();
    let mut rng = Rng::new(31337);
    let count = 24i32;
    let n = count as usize;

    for offset in 0..6usize {
        let base: Vec<f32> = (0..n * 3 + offset)
            .map(|_| rng.next_f32_scaled(5.0))
            .collect();

        // dest starts `offset` floats before src's start, inside one buffer.
        let mut c_buf = base.clone();
        let mut rust_buf = base.clone();
        unsafe {
            let p = c_buf.as_mut_ptr();
            (pair.c_tfm)(p, p.add(offset), count);
            let p = rust_buf.as_mut_ptr();
            (pair.rust_tfm)(p, p.add(offset), count);
        }
        for i in 0..base.len() {
            assert_eq!(
                c_buf[i].to_bits(),
                rust_buf[i].to_bits(),
                "overlap/{offset}: mismatch at [{i}]: C={:?} Rust={:?}",
                c_buf[i],
                rust_buf[i]
            );
        }
    }
}

/// Fully in-place: `dest == src`.
#[test]
fn in_place_dest_equals_src() {
    let pair = Pair::load();
    let mut rng = Rng::new(4242);
    for round in 0..50 {
        let count = 1 + (round % 20) as i32;
        let base: Vec<f32> = (0..count as usize * 3)
            .map(|_| rng.next_f32_scaled(2.0))
            .collect();
        let mut c_buf = base.clone();
        let mut rust_buf = base.clone();
        unsafe {
            (pair.c_tfm)(c_buf.as_mut_ptr(), c_buf.as_ptr(), count);
            (pair.rust_tfm)(rust_buf.as_mut_ptr(), rust_buf.as_ptr(), count);
        }
        for i in 0..base.len() {
            assert_eq!(
                c_buf[i].to_bits(),
                rust_buf[i].to_bits(),
                "inplace/{round}: mismatch at [{i}]"
            );
        }
    }
}
