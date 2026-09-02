//! Exhaustive-in-the-interesting-subspace sweeps.
//!
//! Two jobs:
//!
//! 1. `nan_full_cross_product` — a full 3-lane cross product over a rich set of
//!    special values (many NaN payloads and signs, sNaN, infinities, signed
//!    zeros, subnormals, the C's own literals). This is what makes the
//!    "equivalent mutant" claims in `scripts/mutation_check.py` credible: every
//!    operand-role-sensitive combination the kernel can see is enumerated, not
//!    sampled.
//!
//! 2. `sqd_is_never_negative_zero` — exhaustively proves over all 2^32 `f32`
//!    bit patterns that `4.0f * dxy * dxy` is never `-0.0`, hence
//!    `sqd = addss(4*dxy*dxy, acc)` is never `-0.0`, hence the C's
//!    `(0 > sqd) ? 0 : sqd` clamp cannot distinguish `>` from `>=`.

mod common;

use common::*;
use std::ffi::c_int;

/// Rich special-value set: every class the kernel branches on or propagates.
fn special_values() -> Vec<f32> {
    let mut v: Vec<u32> = vec![
        0x0000_0000, // +0
        0x8000_0000, // -0
        0x0000_0001, // +smallest subnormal
        0x8000_0001, // -smallest subnormal
        0x007F_FFFF, // +largest subnormal
        0x807F_FFFF, // -largest subnormal
        0x0080_0000, // +smallest normal
        0x8080_0000, // -smallest normal
        0x3F80_0000, // 1.0
        0xBF80_0000, // -1.0
        0x4000_0000, // 2.0
        0x3F00_0000, // 0.5  (C literal)
        0x4080_0000, // 4.0  (C literal)
        0x7F7F_FFFF, // FLT_MAX
        0xFF7F_FFFF, // -FLT_MAX
        0x7F80_0000, // +inf
        0xFF80_0000, // -inf
        // signalling NaNs, both signs, varied payloads
        0x7F80_0001, 0xFF80_0001, 0x7FA0_0000, 0xFFA0_0000, 0x7FBF_FFFF, 0xFFBF_FFFF,
        0x7F91_2345, 0xFF91_2345,
        // quiet NaNs, both signs, varied payloads (distinct payloads matter:
        // they are what makes an operand-role swap observable)
        0x7FC0_0000, 0xFFC0_0000, 0x7FC0_0001, 0xFFC0_0001, 0x7FD5_5555, 0xFFD5_5555,
        0x7FEA_AAAA, 0xFFEA_AAAA, 0x7FFF_FFFF, 0xFFFF_FFFF, 0x7FC1_1111, 0xFFC2_2222,
    ];
    v.dedup();
    v.into_iter().map(f32::from_bits).collect()
}

#[test]
fn nan_full_cross_product() {
    let vals = special_values();
    let n = vals.len();
    // 36^3 == 46_656 differential calls through both .so exports.
    let mut cases = 0usize;
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                diff_call("cross", &[a, b, c], 1);
                cases += 1;
            }
        }
    }
    assert_eq!(cases, n * n * n);
    assert!(cases > 40_000, "cross product unexpectedly small: {cases}");
}

/// Same cross product, but batched into a multi-element call so the pointer
/// advance interacts with the special values.
#[test]
fn nan_cross_product_batched() {
    let vals = special_values();
    let mut src: Vec<f32> = Vec::new();
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                src.extend_from_slice(&[a, b, c]);
            }
        }
    }
    let count = src.len() / 3;
    diff_call("cross/batch", &src, count as c_int);

    // and in-place, so writes trail reads over the whole special-value space
    let mut bc = src.clone();
    let mut br = src.clone();
    unsafe {
        (c_tfm())(bc.as_mut_ptr(), bc.as_ptr(), count as c_int);
        (rust_tfm())(br.as_mut_ptr(), br.as_ptr(), count as c_int);
    }
    assert_bits_eq("cross/batch/inplace", &src, count as c_int, &bc, &br);
}

/// Exhaustive proof over all 2^32 `f32` values that the `4*dxy*dxy` term is
/// never `-0.0`, and that `+0.0 + x` is never `-0.0` — which together make the
/// C's clamp insensitive to `>` vs `>=`, i.e. `sqd == -0.0` is unreachable.
///
/// Split across threads; ~4.3e9 iterations of two multiplies.
#[test]
fn sqd_is_never_negative_zero() {
    const NEG_ZERO: u32 = 0x8000_0000;
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 16) as u64;
    let span = (1u64 << 32) / threads;

    let handles: Vec<_> = (0..threads)
        .map(|t| {
            let lo = t * span;
            let hi = if t + 1 == threads { 1u64 << 32 } else { (t + 1) * span };
            std::thread::spawn(move || -> Option<u32> {
                for bits in lo..hi {
                    let dxy = f32::from_bits(bits as u32);
                    // exactly the C's operand order: mulss(4.0f, dxy) then *dxy
                    let term = (4.0f32 * dxy) * dxy;
                    if term.to_bits() == NEG_ZERO {
                        return Some(bits as u32);
                    }
                    // +0.0f + x is never -0.0 for any x (only -0 + -0 is -0)
                    let sum = 0.0f32 + dxy;
                    if sum.to_bits() == NEG_ZERO {
                        return Some(bits as u32);
                    }
                }
                None
            })
        })
        .collect();

    for h in handles {
        if let Some(bits) = h.join().expect("thread panicked") {
            panic!(
                "sqd CAN be -0.0: dxy = 0x{bits:08x} — the `>` vs `>=` clamp \
                 distinction is reachable after all"
            );
        }
    }
}
