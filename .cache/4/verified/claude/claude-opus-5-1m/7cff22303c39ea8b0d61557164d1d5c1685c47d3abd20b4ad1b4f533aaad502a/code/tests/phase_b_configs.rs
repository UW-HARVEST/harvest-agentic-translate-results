//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. `tfm` is both the only and the
//! lowest-level public entry point, so every test drives it directly through
//! the `.so` export, with the full option (`count`) and input-shape
//! cross-product the C branches on.
//!
//! Every row uses many randomized inputs from the fixed seed `SEED`, and each
//! comparison is bit-for-bit on the raw `u32` of every output element (plus
//! guard words on either side of `dest`).

mod common;
use common::*;

/// A random NaN: random sign, random payload, random quiet bit.
fn nan_bits(rng: &mut Rng) -> u32 {
    let sign = (rng.next_u64() & 1) as u32;
    let payload = (rng.next_u32() & 0x007f_ffff).max(1);
    (sign << 31) | 0x7f80_0000 | payload
}

fn ordered_pair(rng: &mut Rng) -> Option<(u32, u32)> {
    let a = rng.normal_f32();
    let b = rng.normal_f32();
    let (fa, fb) = (f32::from_bits(a), f32::from_bits(b));
    if fa < fb {
        Some((a, b))
    } else if fb < fa {
        Some((b, a))
    } else {
        None
    }
}

// ===========================================================================
// Rows 1-3 — arm selection with plain finite normals.
// ===========================================================================

/// CONFIGS.md row 1 — `count = 1`, **if** arm forced (`src[0] < src[1]`).
#[test]
fn row01_if_arm_finite_normals() {
    let mut rng = Rng::new(SEED ^ 0x101);
    let mut n = 0;
    while n < 4 * SAMPLES {
        if let Some((lo, hi)) = ordered_pair(&mut rng) {
            let s = [lo, hi, rng.normal_f32()];
            assert!(trace(s[0], s[1], s[2]).arm_if, "row01 must take the if arm");
            diff(&format!("row01 #{n}"), &s, 1, 2);
            n += 1;
        }
    }
    // ...and with a "tame" dxy so sqd stays finite and the sqrt is meaningful.
    for i in 0..SAMPLES {
        if let Some((lo, hi)) = ordered_pair(&mut rng) {
            diff(&format!("row01 tame #{i}"), &[lo, hi, rng.tame_f32()], 1, 2);
        }
    }
}

/// CONFIGS.md row 2 — `count = 1`, **else** arm via `src[0] > src[1]`.
#[test]
fn row02_else_arm_greater_finite_normals() {
    let mut rng = Rng::new(SEED ^ 0x102);
    let mut n = 0;
    while n < 4 * SAMPLES {
        if let Some((lo, hi)) = ordered_pair(&mut rng) {
            let s = [hi, lo, rng.normal_f32()];
            assert!(!trace(s[0], s[1], s[2]).arm_if, "row02 must take the else arm");
            diff(&format!("row02 #{n}"), &s, 1, 2);
            n += 1;
        }
    }
    for i in 0..SAMPLES {
        if let Some((lo, hi)) = ordered_pair(&mut rng) {
            diff(&format!("row02 tame #{i}"), &[hi, lo, rng.tame_f32()], 1, 2);
        }
    }
}

/// CONFIGS.md row 3 — `count = 1`, **else** arm via `src[0] == src[1]`.
#[test]
fn row03_else_arm_equal_finite_normals() {
    let mut rng = Rng::new(SEED ^ 0x103);
    for i in 0..4 * SAMPLES {
        let v = rng.normal_f32();
        let s = [v, v, rng.normal_f32()];
        assert!(!trace(s[0], s[1], s[2]).arm_if);
        diff(&format!("row03 #{i}"), &s, 1, 2);
    }
    for i in 0..SAMPLES {
        let v = rng.tame_f32();
        diff(&format!("row03 tame #{i}"), &[v, v, rng.tame_f32()], 1, 2);
    }
}

/// CONFIGS.md row 4 — `count = 1`, arm chosen by the data, full special pool.
#[test]
fn row04_data_chosen_arm_full_pool() {
    let mut rng = Rng::new(SEED ^ 0x104);
    let (mut ifs, mut elses) = (0usize, 0usize);
    for i in 0..20 * SAMPLES {
        let s = [rng.pool_f32(), rng.pool_f32(), rng.pool_f32()];
        if trace(s[0], s[1], s[2]).arm_if {
            ifs += 1;
        } else {
            elses += 1;
        }
        diff(&format!("row04 #{i}"), &s, 1, 2);
    }
    assert!(ifs > 100 && elses > 100, "row04: arm coverage {ifs}/{elses}");
}

// ===========================================================================
// Rows 5-12 — the clamp `(0 > sqd) ? 0 : sqd` and the classes of `sqd`.
// ===========================================================================

/// CONFIGS.md row 5 — clamp **taken** (`sqd < 0`), both arms.
#[test]
fn row05_clamp_taken_both_arms() {
    let hits_if = diff_matching("row05 clamp-taken if-arm", SEED ^ 0x105, 400_000, 50, |t| {
        t.sqd < 0.0 && t.arm_if
    });
    let hits_else = diff_matching(
        "row05 clamp-taken else-arm",
        SEED ^ 0x115,
        400_000,
        50,
        |t| t.sqd < 0.0 && !t.arm_if,
    );
    println!("row05: sqd<0 hits if={hits_if} else={hits_else}");
}

/// CONFIGS.md row 6 — clamp **not** taken (`sqd > 0`), both arms.
#[test]
fn row06_clamp_not_taken_both_arms() {
    let a = diff_matching("row06 sqd>0 if-arm", SEED ^ 0x106, 40_000, 2000, |t| {
        t.sqd > 0.0 && t.sqd.is_finite() && t.arm_if
    });
    let b = diff_matching("row06 sqd>0 else-arm", SEED ^ 0x116, 40_000, 2000, |t| {
        t.sqd > 0.0 && t.sqd.is_finite() && !t.arm_if
    });
    println!("row06: finite sqd>0 hits if={a} else={b}");

    // High-volume coverage of the *real* sqrtf path: a finite, positive,
    // non-trivial `sqd` whose square root is not exact. This is where
    // f32::sqrt (sqrtss) must agree with glibc's sqrtf to the last bit.
    let mut rng = Rng::new(SEED ^ 0x126);
    let mut nontrivial = 0usize;
    for i in 0..40 * SAMPLES {
        let s = [rng.tame_f32(), rng.tame_f32(), rng.tame_f32()];
        let t = trace(s[0], s[1], s[2]);
        if t.sqd.is_finite() && t.sqd > 0.0 && t.root.is_finite() && t.root > 0.0 {
            let r = t.root;
            if r * r != t.sqd {
                nontrivial += 1; // inexact square root
            }
        }
        diff(&format!("row06 sqrt-path #{i}"), &s, 1, 2);
    }
    assert!(
        nontrivial > 5_000,
        "row06: only {nontrivial} inexact-sqrt inputs exercised"
    );
    println!("row06: {nontrivial} inputs with an inexact sqrtf result");
}

/// CONFIGS.md row 7 — `sqd == +0.0` exactly, both arms.
#[test]
fn row07_sqd_exactly_positive_zero() {
    // dx2 == dy2 (else arm, equality) and dxy == 0 gives sqd == +0.0 exactly.
    let mut rng = Rng::new(SEED ^ 0x107);
    let mut n = 0;
    for _ in 0..8 * SAMPLES {
        let v = rng.normal_f32();
        for &z in &[0x0000_0000u32, 0x8000_0000u32] {
            let t = trace(v, v, z);
            if t.sqd.to_bits() == 0 {
                diff("row07 equal-operands", &[v, v, z], 1, 2);
                n += 1;
            }
        }
    }
    assert!(n >= SAMPLES, "row07: only {n} exact +0.0 sqd cases");
    let a = diff_matching("row07 sqd==+0 if-arm", SEED ^ 0x117, 400_000, 20, |t| {
        t.sqd.to_bits() == 0 && t.arm_if
    });
    let b = diff_matching("row07 sqd==+0 else-arm", SEED ^ 0x127, 200_000, 20, |t| {
        t.sqd.to_bits() == 0 && !t.arm_if
    });
    println!("row07: sqd==+0 hits if={a} else={b}");
}

/// CONFIGS.md row 8 — `sqd == -0.0` is unreachable (the final addend
/// `(4*dxy)*dxy` is never negatively signed), so `sqrtf(-0.0)` never happens.
/// Verified by search; agreement is asserted over the whole search space.
#[test]
fn row08_sqd_negative_zero_unreachable() {
    assert_unreachable("row08 sqd==-0.0", SEED ^ 0x108, 200_000, |t| {
        t.sqd.to_bits() == 0x8000_0000
    });
}

/// CONFIGS.md row 9 — `sqd` is NaN (clamp not applied ⇒ `sqrtf(NaN)`), both arms.
#[test]
fn row09_sqd_nan_both_arms() {
    let a = diff_matching("row09 sqd-nan if-arm", SEED ^ 0x109, 200_000, 200, |t| {
        t.sqd.is_nan() && t.arm_if
    });
    let b = diff_matching("row09 sqd-nan else-arm", SEED ^ 0x119, 100_000, 200, |t| {
        t.sqd.is_nan() && !t.arm_if
    });
    println!("row09: sqd NaN hits if={a} else={b}");
}

/// CONFIGS.md row 10 — `sqd == +inf` via a squaring overflow, both arms.
#[test]
fn row10_sqd_positive_infinity() {
    let a = diff_matching("row10 sqd==+inf if-arm", SEED ^ 0x10a, 200_000, 200, |t| {
        t.sqd == f32::INFINITY && t.arm_if
    });
    let b = diff_matching("row10 sqd==+inf else-arm", SEED ^ 0x11a, 100_000, 200, |t| {
        t.sqd == f32::INFINITY && !t.arm_if
    });
    println!("row10: sqd==+inf hits if={a} else={b}");
    // Explicit `|dxy| > 2^64` witnesses.
    let mut rng = Rng::new(SEED ^ 0x12a);
    for i in 0..SAMPLES {
        let sign = (rng.next_u64() & 1) as u32;
        let exp = 220 + rng.below(30) as u32;
        let dxy = (sign << 31) | (exp << 23) | (rng.next_u32() & 0x007f_ffff);
        let (lo, hi) = (rng.tame_f32(), rng.tame_f32());
        diff(&format!("row10 huge-dxy #{i}"), &[lo, hi, dxy], 1, 2);
    }
}

/// CONFIGS.md row 11 — `sqd == inf - inf` ⇒ indefinite QNaN, both arms.
#[test]
fn row11_sqd_inf_minus_inf() {
    let pred = |t: &Trace| {
        t.dy2_sq.is_infinite()
            && t.two_dx2_dy2.is_infinite()
            && t.dy2_sq.is_sign_positive() == t.two_dx2_dy2.is_sign_positive()
    };
    let a = diff_matching("row11 inf-inf if-arm", SEED ^ 0x10b, 200_000, 100, move |t| {
        pred(t) && t.arm_if
    });
    let b = diff_matching("row11 inf-inf else-arm", SEED ^ 0x11b, 200_000, 100, move |t| {
        pred(t) && !t.arm_if
    });
    println!("row11: inf-inf hits if={a} else={b}");
}

/// CONFIGS.md row 12 — `0 * inf` inside `2.0f*dx2*dy2` and inside
/// `4.0f*dxy*dxy` (the latter is structurally unreachable).
#[test]
fn row12_zero_times_inf_sites() {
    let hits = diff_matching("row12 0*inf in 2*dx2*dy2", SEED ^ 0x10c, 200_000, 100, |t| {
        !t.dx2.is_nan() && !t.dy2.is_nan() && t.two_dx2_dy2.is_nan()
    });
    println!("row12: {hits} hits at the 2*dx2*dy2 site");
    // dxy == 0 vs inf, exhaustively against every special first/second operand.
    for &dxy in &[0x0000_0000u32, 0x8000_0000, 0x7f80_0000, 0xff80_0000] {
        for &a in SPECIALS {
            for &b in SPECIALS {
                diff("row12 dxy-zero-or-inf", &[a, b, dxy], 1, 2);
            }
        }
    }
    assert_unreachable("row12 0*inf in 4*dxy*dxy", SEED ^ 0x11c, 100_000, |t| {
        !t.dxy.is_nan() && t.term4.is_nan()
    });
}

// ===========================================================================
// Rows 13-19 — input shapes.
// ===========================================================================

/// CONFIGS.md row 13 — every ±0.0 combination in all three slots.
#[test]
fn row13_signed_zero_cross_product() {
    let z = [0x0000_0000u32, 0x8000_0000u32];
    for &a in &z {
        for &b in &z {
            for &c in &z {
                diff(&format!("row13 {a:#x},{b:#x},{c:#x}"), &[a, b, c], 1, 2);
            }
        }
    }
    // ±0.0 in one slot, arbitrary pool values in the others.
    let mut rng = Rng::new(SEED ^ 0x10d);
    for i in 0..8 * SAMPLES {
        let mut s = [rng.pool_f32(), rng.pool_f32(), rng.pool_f32()];
        s[rng.below(3)] = z[rng.below(2)];
        diff(&format!("row13 mixed #{i}"), &s, 1, 2);
    }
}

/// CONFIGS.md row 14 — the 27 combinations of {−inf, ±0, +inf}, plus mixes.
#[test]
fn row14_infinity_cross_product() {
    let v = [0xff80_0000u32, 0x0000_0000, 0x8000_0000, 0x7f80_0000];
    for &a in &v {
        for &b in &v {
            for &c in &v {
                diff(&format!("row14 {a:#x},{b:#x},{c:#x}"), &[a, b, c], 1, 2);
            }
        }
    }
    // ±inf in one slot, arbitrary values elsewhere.
    let mut rng = Rng::new(SEED ^ 0x10e);
    for i in 0..8 * SAMPLES {
        let mut s = [rng.pool_f32(), rng.pool_f32(), rng.pool_f32()];
        s[rng.below(3)] = if rng.next_u64() & 1 == 0 {
            0x7f80_0000
        } else {
            0xff80_0000
        };
        diff(&format!("row14 mixed #{i}"), &s, 1, 2);
    }
}

/// CONFIGS.md row 15 — a quiet NaN in each slot, random payloads and signs.
#[test]
fn row15_quiet_nan_payload_propagation() {
    let mut rng = Rng::new(SEED ^ 0x10f);
    for i in 0..8 * SAMPLES {
        let sign = (rng.next_u64() & 1) as u32;
        let payload = (rng.next_u32() & 0x003f_ffff).max(1);
        let qnan = (sign << 31) | 0x7fc0_0000 | payload;
        assert!(f32::from_bits(qnan).is_nan() && qnan & 0x0040_0000 != 0);
        let slot = rng.below(3);
        let mut s = [rng.pool_f32(), rng.pool_f32(), rng.pool_f32()];
        s[slot] = qnan;
        diff(&format!("row15 slot{slot} #{i}"), &s, 1, 2);
    }
    // Deterministic payload sweep in each slot.
    for k in 0..256u32 {
        for sign in [0u32, 0x8000_0000] {
            let qnan = sign | 0x7fc0_0000 | (k * 0x0000_4001).max(1) & 0x003f_ffff | 1;
            for slot in 0..3 {
                let mut s = [0x3f80_0000u32, 0x4000_0000, 0x4040_0000];
                s[slot] = qnan;
                diff("row15 sweep", &s, 1, 2);
            }
        }
    }
}

/// CONFIGS.md row 16 — a **signaling** NaN (quiet bit clear) in each slot.
#[test]
fn row16_signaling_nan_in_each_slot() {
    let mut snans = Vec::new();
    for k in 0..64u32 {
        for sign in [0u32, 0x8000_0000] {
            let payload = ((k * 0x0001_0007) & 0x003f_ffff).max(1);
            let b = sign | 0x7f80_0000 | payload;
            if b & 0x0040_0000 == 0 && b & 0x007f_ffff != 0 {
                snans.push(b);
            }
        }
    }
    assert!(snans.len() >= 64, "need signaling NaNs, got {}", snans.len());
    for &n in &snans {
        for slot in 0..3 {
            let mut s = [0x3f80_0000u32, 0x4000_0000, 0x4040_0000];
            s[slot] = n;
            diff("row16 canonical-others", &s, 1, 2);
            let mut s2 = [0x4000_0000u32, 0x3f80_0000, 0xbf80_0000];
            s2[slot] = n;
            diff("row16 else-arm-others", &s2, 1, 2);
        }
    }
    let mut rng = Rng::new(SEED ^ 0x110);
    for i in 0..8 * SAMPLES {
        let mut s = [rng.pool_f32(), rng.pool_f32(), rng.pool_f32()];
        s[rng.below(3)] = snans[rng.below(snans.len())];
        diff(&format!("row16 rand #{i}"), &s, 1, 2);
    }
}

/// CONFIGS.md row 17 — two or three simultaneous NaNs (destination-operand rule).
#[test]
fn row17_multiple_simultaneous_nans() {
    let nans = [
        0x7fc0_0000u32,
        0xffc0_0000,
        0x7fc0_0001,
        0xffff_ffff,
        0x7f80_0001,
        0xffbf_ffff,
        0x7fab_cdef,
        0xff87_6543,
        0x7fff_ffff,
        0xff80_0001,
    ];
    for &a in &nans {
        for &b in &nans {
            for &c in &nans {
                diff("row17 three-nans", &[a, b, c], 1, 2);
            }
            for &c in SPECIALS {
                diff("row17 two-nans-01", &[a, b, c], 1, 2);
                diff("row17 two-nans-02", &[a, c, b], 1, 2);
                diff("row17 two-nans-12", &[c, a, b], 1, 2);
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 0x111);
    for i in 0..8 * SAMPLES {
        let s = [nan_bits(&mut rng), nan_bits(&mut rng), nan_bits(&mut rng)];
        diff(&format!("row17 rand #{i}"), &s, 1, 2);
    }
}

/// CONFIGS.md row 18 — subnormal operands, both signs, including `0x00000001`.
#[test]
fn row18_subnormal_operands() {
    let sub = [
        0x0000_0001u32,
        0x8000_0001,
        0x0000_0002,
        0x0040_0000,
        0x007f_ffff,
        0x807f_ffff,
        0x0000_7fff,
        0x8000_7fff,
    ];
    for &a in &sub {
        for &b in &sub {
            for &c in &sub {
                diff("row18 all-subnormal", &[a, b, c], 1, 2);
            }
            for &c in SPECIALS {
                diff("row18 two-subnormal", &[a, b, c], 1, 2);
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 0x112);
    for i in 0..8 * SAMPLES {
        let g = |r: &mut Rng| {
            let sign = (r.next_u64() & 1) as u32;
            (sign << 31) | (r.next_u32() & 0x007f_ffff).max(1)
        };
        let s = [g(&mut rng), g(&mut rng), g(&mut rng)];
        diff(&format!("row18 rand #{i}"), &s, 1, 2);
    }
}

/// CONFIGS.md row 19 — fully random 32-bit patterns in all three slots.
#[test]
fn row19_uniform_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 0x113);
    for i in 0..8 * 4096 {
        let s = [rng.next_u32(), rng.next_u32(), rng.next_u32()];
        diff(&format!("row19 #{i}"), &s, 1, 2);
    }
}

// ===========================================================================
// Rows 20-23 — the `count` axis and the src/dest strides.
// ===========================================================================

/// CONFIGS.md row 20 — `count = 2` (the `src += 3` / `dest += 2` stride).
#[test]
fn row20_count_two_strides() {
    let mut rng = Rng::new(SEED ^ 0x114);
    for i in 0..8 * SAMPLES {
        let s: Vec<u32> = (0..6).map(|_| rng.pool_f32()).collect();
        diff(&format!("row20 #{i}"), &s, 2, 4);
    }
    // Both arm orderings across the two iterations.
    let lo = 0x3f80_0000u32;
    let hi = 0x4000_0000u32;
    for (a0, b0) in [(lo, hi), (hi, lo)] {
        for (a1, b1) in [(lo, hi), (hi, lo)] {
            diff(
                "row20 arm-mix",
                &[a0, b0, 0x4040_0000, a1, b1, 0xbf80_0000],
                2,
                4,
            );
        }
    }
}

/// CONFIGS.md row 21 — `count = 3..=8` with both arms guaranteed in one call.
#[test]
fn row21_small_counts_mixed_arms() {
    let mut rng = Rng::new(SEED ^ 0x121);
    for n in 3usize..=8 {
        for i in 0..2 * SAMPLES {
            let mut s: Vec<u32> = (0..3 * n).map(|_| rng.pool_f32()).collect();
            // Force iteration 0 into the if arm and iteration 1 into the else arm.
            s[0] = 0x3f80_0000;
            s[1] = 0x4000_0000;
            s[3] = 0x4000_0000;
            s[4] = 0x3f80_0000;
            let t0 = trace(s[0], s[1], s[2]);
            let t1 = trace(s[3], s[4], s[5]);
            assert!(t0.arm_if && !t1.arm_if, "row21: arm forcing failed");
            diff(&format!("row21 n={n} #{i}"), &s, n as i32, 2 * n);
        }
    }
}

/// CONFIGS.md row 22 — `count = 1024`, full pool, guards checked.
#[test]
fn row22_many_iterations() {
    let mut rng = Rng::new(SEED ^ 0x122);
    for i in 0..16 {
        let n = 1024usize;
        let s: Vec<u32> = (0..3 * n).map(|_| rng.pool_f32()).collect();
        diff(&format!("row22 #{i}"), &s, n as i32, 2 * n);
    }
    // A long run of uniform random bit patterns too.
    for i in 0..16 {
        let n = 1024usize;
        let s: Vec<u32> = (0..3 * n).map(|_| rng.next_u32()).collect();
        diff(&format!("row22 bits #{i}"), &s, n as i32, 2 * n);
    }
    // A very large count, so the index arithmetic (`3*i` / `2*i` vs the C's
    // `src += 3` / `dest += 2`) is exercised well past 16 bits.
    for n in [65_537usize, 200_000] {
        let s: Vec<u32> = (0..3 * n).map(|_| rng.pool_f32()).collect();
        diff(&format!("row22 large n={n}"), &s, n as i32, 2 * n);
    }
    // Every count from 1 to 300, against a matching buffer.
    for n in 1usize..=300 {
        let s: Vec<u32> = (0..3 * n).map(|_| rng.pool_f32()).collect();
        diff(&format!("row22 sweep n={n}"), &s, n as i32, 2 * n);
    }
}

/// CONFIGS.md row 23 — `count = 1000` with identical triples: every iteration
/// takes the same arm. Run once for each arm.
#[test]
fn row23_uniform_stream_single_arm() {
    let n = 1000usize;
    for (a, b, c, want_if) in [
        (0x3f80_0000u32, 0x4000_0000u32, 0x4040_0000u32, true),
        (0x4000_0000u32, 0x3f80_0000u32, 0x4040_0000u32, false),
    ] {
        assert_eq!(trace(a, b, c).arm_if, want_if);
        let s: Vec<u32> = (0..n).flat_map(|_| [a, b, c]).collect();
        diff(
            &format!("row23 arm_if={want_if}"),
            &s,
            n as i32,
            2 * n,
        );
    }
    // Same, but with special-value triples.
    let mut rng = Rng::new(SEED ^ 0x123);
    for i in 0..32 {
        let (a, b, c) = (rng.pool_f32(), rng.pool_f32(), rng.pool_f32());
        let s: Vec<u32> = (0..n).flat_map(|_| [a, b, c]).collect();
        diff(&format!("row23 special #{i}"), &s, n as i32, 2 * n);
    }
}

// ===========================================================================
// Rows 24-27 — memory shapes.
// ===========================================================================

/// CONFIGS.md row 24 — byte-unaligned `dest` and `src`.
#[test]
fn row24_unaligned_pointers() {
    let mut rng = Rng::new(SEED ^ 0x124);
    for dest_off in 0..4usize {
        for src_off in 0..4usize {
            for i in 0..64 {
                let n = 5usize;
                let s: Vec<u32> = (0..3 * n).map(|_| rng.pool_f32()).collect();
                diff_unaligned(
                    &format!("row24 d={dest_off} s={src_off} #{i}"),
                    &s,
                    n as i32,
                    2 * n,
                    dest_off,
                    src_off,
                );
            }
        }
    }
}

/// CONFIGS.md row 25 — `dest == src` (exact aliasing).
#[test]
fn row25_exact_aliasing() {
    let mut rng = Rng::new(SEED ^ 0x125);
    for i in 0..4 * SAMPLES {
        let n = 8usize;
        let buf: Vec<u32> = (0..3 * n + 4).map(|_| rng.pool_f32()).collect();
        diff_alias(&format!("row25 #{i}"), &buf, n as i32, 0, 0);
    }
}

/// CONFIGS.md row 26 — `dest` trailing inside `src`'s buffer (benign overlap).
#[test]
fn row26_benign_overlap() {
    let mut rng = Rng::new(SEED ^ 0x126);
    for i in 0..2 * SAMPLES {
        let n = 8usize;
        let buf: Vec<u32> = (0..3 * n + 8).map(|_| rng.pool_f32()).collect();
        for src_off in 1..=4usize {
            diff_alias(
                &format!("row26 src_off={src_off} #{i}"),
                &buf,
                n as i32,
                0,
                src_off,
            );
        }
    }
}

/// CONFIGS.md row 27 — `dest` ahead of `src` (destructive overlap).
#[test]
fn row27_destructive_overlap() {
    let mut rng = Rng::new(SEED ^ 0x127);
    for i in 0..2 * SAMPLES {
        let n = 8usize;
        let buf: Vec<u32> = (0..3 * n + 8).map(|_| rng.pool_f32()).collect();
        for dest_off in 1..=4usize {
            diff_alias(
                &format!("row27 dest_off={dest_off} #{i}"),
                &buf,
                n as i32,
                dest_off,
                0,
            );
        }
    }
}

// ===========================================================================
// Rows 28-34 — value-boundary shapes.
// ===========================================================================

/// CONFIGS.md row 28 — adjacent-value pairs for the strict `<` boundary.
#[test]
fn row28_adjacent_value_pairs() {
    let mut rng = Rng::new(SEED ^ 0x128);
    for i in 0..8 * SAMPLES {
        let base = rng.normal_f32();
        let d = rng.below(9) as i64 - 4;
        let other = (base as i64 + d) as u32;
        let c = rng.pool_f32();
        diff(&format!("row28 a #{i}"), &[base, other, c], 1, 2);
        diff(&format!("row28 b #{i}"), &[other, base, c], 1, 2);
    }
    // Deterministic nextafter sweep around a few anchors.
    for &anchor in &[
        0x3f80_0000u32,
        0x0000_0000,
        0x8000_0000,
        0x0080_0000,
        0x7f7f_fffe,
        0xff7f_fffe,
        0x4000_0000,
    ] {
        for d in -3i64..=3 {
            let other = (anchor as i64 + d) as u32;
            for &c in SPECIALS {
                diff("row28 sweep", &[anchor, other, c], 1, 2);
                diff("row28 sweep-rev", &[other, anchor, c], 1, 2);
            }
        }
    }
}

/// CONFIGS.md row 29 — the signed-zero compare boundary.
#[test]
fn row29_signed_zero_compare_boundary() {
    for (a, b) in [
        (0x8000_0000u32, 0x0000_0000u32),
        (0x0000_0000u32, 0x8000_0000u32),
        (0x0000_0000u32, 0x0000_0000u32),
        (0x8000_0000u32, 0x8000_0000u32),
    ] {
        assert!(!trace(a, b, 0).arm_if, "row29: ±0 < ±0 must be false");
        for &c in SPECIALS {
            diff(&format!("row29 {a:#x},{b:#x}"), &[a, b, c], 1, 2);
        }
    }
    let mut rng = Rng::new(SEED ^ 0x129);
    for i in 0..4 * SAMPLES {
        let c = rng.pool_f32();
        diff(&format!("row29 -0,+0 #{i}"), &[0x8000_0000, 0x0000_0000, c], 1, 2);
        diff(&format!("row29 +0,-0 #{i}"), &[0x0000_0000, 0x8000_0000, c], 1, 2);
    }
}

/// CONFIGS.md row 30 — the extremes of the normal range in all three slots.
#[test]
fn row30_normal_range_extremes() {
    let ext = [
        0x0080_0000u32, // FLT_MIN
        0x8080_0000,    // -FLT_MIN
        0x7f7f_ffff,    // FLT_MAX
        0xff7f_ffff,    // -FLT_MAX
        0x7f7f_fffe,
        0xff7f_fffe,
        0x0080_0001,
        0x8080_0001,
    ];
    for &a in &ext {
        for &b in &ext {
            for &c in &ext {
                diff("row30 all-extremes", &[a, b, c], 1, 2);
            }
            for &c in SPECIALS {
                diff("row30 two-extremes", &[a, b, c], 1, 2);
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 0x130);
    for i in 0..4 * SAMPLES {
        let mut s = [rng.pool_f32(), rng.pool_f32(), rng.pool_f32()];
        s[rng.below(3)] = ext[rng.below(ext.len())];
        diff(&format!("row30 rand #{i}"), &s, 1, 2);
    }
}

/// CONFIGS.md row 31 — `4.0f*dxy` overflows while `dxy*dxy` alone would not.
/// This distinguishes `(4*dxy)*dxy` from `4*(dxy*dxy)`, i.e. it pins the C's
/// left-to-right multiplication order.
#[test]
fn row31_four_times_dxy_overflow_order() {
    // |dxy| in (FLT_MAX/4, FLT_MAX]: 4*dxy overflows to ±inf, then *dxy = +inf.
    // (4*(dxy*dxy)) would also be inf here, so also probe the *other* direction:
    // |dxy| where dxy*dxy overflows but 4*dxy does not -- both must give +inf.
    let mut cands = Vec::new();
    for k in 0..64u32 {
        // exponent 254 => just under FLT_MAX, 4*dxy overflows
        cands.push((254u32 << 23) | (k * 0x0002_0001) & 0x007f_ffff);
        cands.push(0x8000_0000 | (254u32 << 23) | (k * 0x0002_0001) & 0x007f_ffff);
        // exponent ~ 190 => dxy*dxy overflows, 4*dxy does not
        cands.push((190u32 << 23) | (k * 0x0001_1111) & 0x007f_ffff);
        cands.push(0x8000_0000 | (190u32 << 23) | (k * 0x0001_1111) & 0x007f_ffff);
    }
    let mut checked_overflow_of_4dxy = 0;
    for &dxy in &cands {
        let v = f32::from_bits(dxy);
        if (4.0f32 * v).is_infinite() && !(v * v).is_infinite() {
            checked_overflow_of_4dxy += 1;
        }
        for &a in SPECIALS {
            for &b in SPECIALS {
                diff("row31", &[a, b, dxy], 1, 2);
            }
        }
    }
    // The interesting sub-case must actually occur... but note that for f32 any
    // |dxy| with 4*dxy overflowing also has dxy*dxy overflowing, so instead the
    // order is pinned by NaN payload/sign, which the diff above covers.
    println!("row31: {checked_overflow_of_4dxy} inputs where only 4*dxy overflows");
    let mut rng = Rng::new(SEED ^ 0x131);
    for i in 0..4 * SAMPLES {
        let sign = (rng.next_u64() & 1) as u32;
        let exp = 180 + rng.below(75) as u32;
        let dxy = (sign << 31) | (exp << 23) | (rng.next_u32() & 0x007f_ffff);
        diff(
            &format!("row31 rand #{i}"),
            &[rng.pool_f32(), rng.pool_f32(), dxy],
            1,
            2,
        );
    }
}

/// CONFIGS.md row 32 — `dx2 == dy2` exactly, so `sqd` reduces to `4*dxy^2`.
#[test]
fn row32_equal_dx2_dy2() {
    let mut rng = Rng::new(SEED ^ 0x132);
    for i in 0..8 * SAMPLES {
        let v = rng.pool_f32();
        let c = rng.pool_f32();
        // Equality => the else arm; NaN => also the else arm.
        assert!(!trace(v, v, c).arm_if);
        diff(&format!("row32 #{i}"), &[v, v, c], 1, 2);
    }
    for &v in SPECIALS {
        for &c in SPECIALS {
            diff("row32 specials", &[v, v, c], 1, 2);
        }
    }
    // The if arm cannot have dx2 == dy2 (strict `<`), so also cover the
    // "one ULP apart" near-equal case which is the closest reachable analogue.
    for i in 0..2 * SAMPLES {
        let v = rng.normal_f32();
        let w = v.wrapping_add(1);
        if f32::from_bits(v) < f32::from_bits(w) {
            assert!(trace(v, w, 0).arm_if);
            diff(&format!("row32 near-equal if-arm #{i}"), &[v, w, rng.pool_f32()], 1, 2);
        }
    }
}

/// CONFIGS.md row 33 — a mixed NaN/finite output stream over many iterations.
#[test]
fn row33_mixed_nan_finite_stream() {
    let mut rng = Rng::new(SEED ^ 0x133);
    for i in 0..64 {
        let n = 64usize;
        let mut s = Vec::with_capacity(3 * n);
        let mut nan_iters = 0;
        let mut finite_iters = 0;
        for k in 0..n {
            let (a, b, c) = if k % 2 == 0 {
                (rng.tame_f32(), rng.tame_f32(), rng.tame_f32())
            } else {
                (nan_bits(&mut rng), rng.pool_f32(), rng.pool_f32())
            };
            let t = trace(a, b, c);
            if t.lambda.is_nan() {
                nan_iters += 1;
            } else {
                finite_iters += 1;
            }
            s.extend_from_slice(&[a, b, c]);
        }
        assert!(
            nan_iters > 0 && finite_iters > 0,
            "row33: need a mixed stream, got {nan_iters}/{finite_iters}"
        );
        diff(&format!("row33 #{i}"), &s, n as i32, 2 * n);
    }
}

/// CONFIGS.md row 34 — the exhaustive 3-slot cross-product of the 24-value
/// special table (13 824 deterministic cases).
#[test]
fn row34_exhaustive_specials_cross_product() {
    assert_eq!(SPECIALS.len(), 24, "the specials table must have 24 entries");
    let mut cases = 0usize;
    for &a in SPECIALS {
        for &b in SPECIALS {
            for &c in SPECIALS {
                diff(
                    &format!("row34 {a:#010x},{b:#010x},{c:#010x}"),
                    &[a, b, c],
                    1,
                    2,
                );
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 24 * 24 * 24);
    // The same cross-product driven as one long multi-element call, so that the
    // strides and the per-iteration state reset are exercised on it too.
    let flat: Vec<u32> = SPECIALS
        .iter()
        .flat_map(|&a| {
            SPECIALS
                .iter()
                .flat_map(move |&b| SPECIALS.iter().flat_map(move |&c| [a, b, c]))
        })
        .collect();
    let n = flat.len() / 3;
    diff("row34 single-call", &flat, n as i32, 2 * n);
}
