//! Phase C — error/rejection-path differential tests.
//!
//! One test (or clearly-labelled block) per row of `ERRORS.md`. Each asserts
//! that C and Rust produce the SAME error code / sentinel / early-out value —
//! not merely that both "failed somehow".

#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};

const INT_MIN: c_int = i32::MIN;

// ===========================================================================
// Rows 1–4: f2 — out-of-range C enum values across the FFI boundary
// ===========================================================================

/// Every `C2_TYPE` value with no valid variant. A C enum accepts any `int`, so
/// these are real inputs the C `switch default:` must handle, and the sentinel
/// must be exactly `0`.
const BAD_TYPES: &[c_uint] = &[
    2,
    3,
    4,
    7,
    100,
    0x7FFF_FFFF,
    0x8000_0000, // INT_MIN reinterpreted as unsigned
    0xFFFF_FFFF, // -1 reinterpreted as unsigned
    0xFFFF_FFFE,
    0x0000_FFFF,
    0xDEAD_BEEF,
];

fn sample_shapes(rng: &mut Rng) -> (c2Circle, c2AABB) {
    (
        c2Circle {
            p: c2v {
                x: rng.finite_f32(10.0),
                y: rng.finite_f32(10.0),
            },
            r: rng.finite_f32(5.0),
        },
        c2AABB {
            min: c2v {
                x: rng.finite_f32(10.0),
                y: rng.finite_f32(10.0),
            },
            max: c2v {
                x: rng.finite_f32(10.0),
                y: rng.finite_f32(10.0),
            },
        },
    )
}

/// Row 1 — `typeA` out of range → outer `switch default:` → `0`.
#[test]
fn err01_f2_bad_typeA() {
    let a = apis();
    let mut rng = Rng::new();
    for &bad in BAD_TYPES {
        for _ in 0..200 {
            let (ci, bi) = sample_shapes(&mut rng);
            let pc = &ci as *const c2Circle as *const c_void;
            let pb = &bi as *const c2AABB as *const c_void;
            for &tb in &[C2_TYPE_CIRCLE, C2_TYPE_AABB] {
                let (rc, rr) = unsafe { ((a.c.f2)(pc, bad, pc, tb), (a.r.f2)(pc, bad, pc, tb)) };
                eq_i32("f2/bad-typeA", &(bad, tb), rc, rr);
                assert_eq!(rc, 0, "C must return the 0 sentinel for typeA={bad:#x}");
                let (rc2, rr2) = unsafe { ((a.c.f2)(pb, bad, pb, tb), (a.r.f2)(pb, bad, pb, tb)) };
                eq_i32("f2/bad-typeA/aabb", &(bad, tb), rc2, rr2);
                assert_eq!(rc2, 0);
            }
        }
    }
}

/// Row 2 — `typeA == CIRCLE`, `typeB` out of range → inner `default:` → `0`.
#[test]
fn err02_f2_bad_typeB_circle() {
    let a = apis();
    let mut rng = Rng::new();
    for &bad in BAD_TYPES {
        for _ in 0..200 {
            let (ci, _) = sample_shapes(&mut rng);
            let pc = &ci as *const c2Circle as *const c_void;
            let (rc, rr) = unsafe {
                (
                    (a.c.f2)(pc, C2_TYPE_CIRCLE, pc, bad),
                    (a.r.f2)(pc, C2_TYPE_CIRCLE, pc, bad),
                )
            };
            eq_i32("f2/bad-typeB-circle", &bad, rc, rr);
            assert_eq!(rc, 0, "C must return 0 for typeB={bad:#x}");
        }
    }
}

/// Row 3 — `typeA == AABB`, `typeB` out of range → inner `default:` → `0`.
#[test]
fn err03_f2_bad_typeB_aabb() {
    let a = apis();
    let mut rng = Rng::new();
    for &bad in BAD_TYPES {
        for _ in 0..200 {
            let (_, bi) = sample_shapes(&mut rng);
            let pb = &bi as *const c2AABB as *const c_void;
            let (rc, rr) = unsafe {
                (
                    (a.c.f2)(pb, C2_TYPE_AABB, pb, bad),
                    (a.r.f2)(pb, C2_TYPE_AABB, pb, bad),
                )
            };
            eq_i32("f2/bad-typeB-aabb", &bad, rc, rr);
            assert_eq!(rc, 0, "C must return 0 for typeB={bad:#x}");
        }
    }
}

/// Row 4 — both type tags out of range → outer `default:` wins → `0`.
#[test]
fn err04_f2_both_bad() {
    let a = apis();
    let mut rng = Rng::new();
    for &ba in BAD_TYPES {
        for &bb in BAD_TYPES {
            let (ci, _) = sample_shapes(&mut rng);
            let pc = &ci as *const c2Circle as *const c_void;
            let (rc, rr) = unsafe { ((a.c.f2)(pc, ba, pc, bb), (a.r.f2)(pc, ba, pc, bb)) };
            eq_i32("f2/both-bad", &(ba, bb), rc, rr);
            assert_eq!(rc, 0);
        }
    }
    // One step past each valid variant, in both slots.
    for &(ta, tb) in &[(0u32, 2u32), (2, 0), (1, 2), (2, 1), (2, 2)] {
        let (ci, _) = sample_shapes(&mut rng);
        let pc = &ci as *const c2Circle as *const c_void;
        let (rc, rr) = unsafe { ((a.c.f2)(pc, ta, pc, tb), (a.r.f2)(pc, ta, pc, tb)) };
        eq_i32("f2/one-past", &(ta, tb), rc, rr);
        assert_eq!(rc, 0);
    }
}

// ===========================================================================
// Rows 5–11: f3 — every guarded / overflow-avoidance branch
// ===========================================================================

/// Row 5 — `v2 == 0`: the explicit divide-by-zero guard must return `0`
/// (and must NOT execute an `idiv`, which would raise SIGFPE).
#[test]
fn err05_f3_div_by_zero() {
    let a = apis();
    let mut rng = Rng::new();
    let mut probes: Vec<c_int> = SPECIAL_I32.to_vec();
    for _ in 0..20_000 {
        probes.push(rng.next_i32());
    }
    for v1 in probes {
        let (rc, rr) = unsafe { ((a.c.f3)(v1, 0), (a.r.f3)(v1, 0)) };
        eq_i32("f3/v2=0", &v1, rc, rr);
        assert_eq!(rc, 0, "C returns the 0 sentinel for v2 == 0 (v1={v1})");
    }
}

/// Row 6 — `v1 >= 0 && v2 == INT_MIN`: `q = 0, r = v1`.
#[test]
fn err06_f3_v2_intmin() {
    let a = apis();
    let mut rng = Rng::new();
    let mut probes: Vec<c_int> = vec![0, 1, 2, 7, i32::MAX, i32::MAX - 1, 0x4000_0000];
    for _ in 0..20_000 {
        probes.push((rng.next_u32() >> 1) as i32); // always >= 0
    }
    for v1 in probes {
        assert!(v1 >= 0);
        let (rc, rr) = unsafe { ((a.c.f3)(v1, INT_MIN), (a.r.f3)(v1, INT_MIN)) };
        eq_i32("f3/v1>=0,v2=INT_MIN", &v1, rc, rr);
        // r = v1 >= 0 -> returns q == 0
        assert_eq!(rc, 0, "expected q=0 for v1={v1}, v2=INT_MIN");
    }
}

/// Row 7 — `v1 < 0 && v1 != INT_MIN && v2 == INT_MIN`: `q = 1`,
/// `r = v1 - q*v2` wraps on signed overflow.
#[test]
fn err07_f3_v1neg_v2_intmin() {
    let a = apis();
    let mut rng = Rng::new();
    let mut probes: Vec<c_int> = vec![-1, -2, -7, INT_MIN + 1, INT_MIN + 2, -0x4000_0000];
    for _ in 0..20_000 {
        let v = -((rng.next_u32() >> 1) as i32);
        if v != INT_MIN && v < 0 {
            probes.push(v);
        }
    }
    for v1 in probes {
        let (rc, rr) = unsafe { ((a.c.f3)(v1, INT_MIN), (a.r.f3)(v1, INT_MIN)) };
        eq_i32("f3/v1<0,v2=INT_MIN", &v1, rc, rr);
        // Reproduce the C arithmetic exactly (wrapping) to pin the value.
        let r = v1.wrapping_sub(1i32.wrapping_mul(INT_MIN));
        let expect = if r >= 0 { 1 } else { 1i32.wrapping_add(1) };
        assert_eq!(rc, expect, "v1={v1}");
    }
}

/// Row 8 — `v1 == INT_MIN && v2 >= 1`: the `-(v1 + v2)` branch.
#[test]
fn err08_f3_v1_intmin_v2pos() {
    let a = apis();
    let mut rng = Rng::new();
    let mut probes: Vec<c_int> = (1..=300).collect();
    probes.extend([i32::MAX, i32::MAX - 1, 0x4000_0000, 65536]);
    for _ in 0..20_000 {
        let v = (rng.next_u32() >> 1) as i32;
        if v > 0 {
            probes.push(v);
        }
    }
    for v2 in probes {
        let (rc, rr) = unsafe { ((a.c.f3)(INT_MIN, v2), (a.r.f3)(INT_MIN, v2)) };
        eq_i32("f3/v1=INT_MIN,v2>0", &v2, rc, rr);
        let t = INT_MIN.wrapping_add(v2).wrapping_neg();
        let q = t.wrapping_div(v2).wrapping_neg().wrapping_sub(1);
        let r = t.wrapping_rem(v2).wrapping_neg();
        let expect = if r >= 0 { q } else { q.wrapping_add(-1) };
        assert_eq!(rc, expect, "v2={v2}");
    }
}

/// Row 9 — `v1 == INT_MIN && v2 < 0 && v2 != INT_MIN`: the `-(v1 - v2)` branch.
#[test]
fn err09_f3_v1_intmin_v2neg() {
    let a = apis();
    let mut rng = Rng::new();
    let mut probes: Vec<c_int> = (-300..=-1).collect();
    probes.extend([INT_MIN + 1, INT_MIN + 2, -0x4000_0000, -65536]);
    for _ in 0..20_000 {
        let v = -((rng.next_u32() >> 1) as i32);
        if v < 0 && v != INT_MIN {
            probes.push(v);
        }
    }
    for v2 in probes {
        let (rc, rr) = unsafe { ((a.c.f3)(INT_MIN, v2), (a.r.f3)(INT_MIN, v2)) };
        eq_i32("f3/v1=INT_MIN,v2<0", &v2, rc, rr);
        let t = INT_MIN.wrapping_sub(v2).wrapping_neg();
        let q = t.wrapping_div(v2.wrapping_neg()).wrapping_add(1);
        let r = t.wrapping_rem(v2.wrapping_neg()).wrapping_neg();
        let expect = if r >= 0 { q } else { q.wrapping_add(1) };
        assert_eq!(rc, expect, "v2={v2}");
    }
}

/// Row 10 — `v1 == INT_MIN && v2 == INT_MIN`: final `else`, `q = 1, r = 0` → `1`.
#[test]
fn err10_f3_both_intmin() {
    let a = apis();
    let (rc, rr) = unsafe { ((a.c.f3)(INT_MIN, INT_MIN), (a.r.f3)(INT_MIN, INT_MIN)) };
    eq_i32("f3/both-INT_MIN", &(INT_MIN, INT_MIN), rc, rr);
    assert_eq!(rc, 1, "C's final else yields q=1, r=0 -> 1");
}

/// Row 11 — the `r < 0` floor correction: `q + (v2 > 0 ? -1 : 1)`.
///
/// Asserts the correction actually fires (so the row is genuinely exercised)
/// and that both libraries agree on the corrected value.
#[test]
fn err11_f3_negative_remainder() {
    let a = apis();
    let mut fired_pos_v2 = 0usize;
    let mut fired_neg_v2 = 0usize;
    for v1 in -200i32..=200 {
        for v2 in -200i32..=200 {
            if v2 == 0 {
                continue;
            }
            let (rc, rr) = unsafe { ((a.c.f3)(v1, v2), (a.r.f3)(v1, v2)) };
            eq_i32("f3/floor", &(v1, v2), rc, rr);
            // The C correction is `q + (v2 > 0 ? -1 : 1)`, applied to the
            // truncating quotient. NOTE: for `v2 < 0` that ADDS one, so `f3`
            // is not a mathematical floor there — reproduce, do not "fix".
            let trunc = v1.wrapping_div(v2);
            if rc != trunc {
                let delta = if v2 > 0 {
                    fired_pos_v2 += 1;
                    -1
                } else {
                    fired_neg_v2 += 1;
                    1
                };
                assert_eq!(
                    rc,
                    trunc.wrapping_add(delta),
                    "corrected quotient must be trunc{delta:+} for v1={v1}, v2={v2}"
                );
            }
        }
    }
    assert!(
        fired_pos_v2 > 0 && fired_neg_v2 > 0,
        "the r<0 correction must fire for both signs of v2 (pos={fired_pos_v2}, neg={fired_neg_v2})"
    );
}

// ===========================================================================
// Rows 12, 31, 36, 41: null pointers — C has NO checks (documented, not run)
// ===========================================================================

/// The C source contains no null check anywhere: `grep -c 'NULL' lib.c` == 0.
/// `f4`, `f11`, `f12` and `f13` dereference their pointer arguments
/// unconditionally, so a null argument is undefined behaviour that faults the
/// process on both sides. Executing it would abort the test runner, so this
/// row is verified structurally instead: the Rust wrappers must be equally
/// unchecked (no `if ptr.is_null()` early-out that C lacks).
#[test]
fn err_null_pointers_documented() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read translation/src/lib.rs");
    assert!(
        !src.contains("is_null"),
        "Rust must not add a null check the C does not have — that would make \
         Rust return where C faults"
    );
    let c = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");
    assert!(
        !c.contains("NULL"),
        "premise changed: the C source now mentions NULL; re-derive ERRORS.md rows 12/31/36/41"
    );
}

// ===========================================================================
// Rows 13–14: f4 — degenerate and maximal seeds
// ===========================================================================

/// Row 13 — `state = {0, 0}` → `value == 0` → result exactly `0.0`.
#[test]
fn err13_f4_zero_state() {
    let a = apis();
    let mut sc = cn_rnd_t { state: [0, 0] };
    let mut sr = cn_rnd_t { state: [0, 0] };
    let (vc, vr) = unsafe { ((a.c.f4)(&mut sc), (a.r.f4)(&mut sr)) };
    eq_f64("f4/zero-state", &"{0,0}", vc, vr);
    assert_eq!(vc.to_bits(), 0f64.to_bits(), "C yields exactly +0.0");
    assert_eq!(sc.state, sr.state);
    assert_eq!(sc.state, [0, 0], "the all-zero state is a fixed point");
    // repeated calls stay at 0
    for _ in 0..64 {
        let (vc, vr) = unsafe { ((a.c.f4)(&mut sc), (a.r.f4)(&mut sr)) };
        eq_f64("f4/zero-state-rep", &"{0,0}", vc, vr);
        assert_eq!(sc.state, sr.state);
    }
}

/// Row 14 — maximal seed, and every `{0,x}` / `{x,0}` shape.
#[test]
fn err14_f4_max_state() {
    let a = apis();
    let mut seeds: Vec<(u64, u64)> = vec![
        (u64::MAX, u64::MAX),
        (u64::MAX, 0),
        (0, u64::MAX),
        (1, u64::MAX),
        (u64::MAX, 1),
        (1 << 63, 1 << 63),
        (1, 0),
        (0, 1),
    ];
    let mut rng = Rng::new();
    for _ in 0..5_000 {
        seeds.push((0, rng.next_u64()));
        seeds.push((rng.next_u64(), 0));
    }
    for (s0, s1) in seeds {
        let mut sc = cn_rnd_t { state: [s0, s1] };
        let mut sr = cn_rnd_t { state: [s0, s1] };
        let (vc, vr) = unsafe { ((a.c.f4)(&mut sc), (a.r.f4)(&mut sr)) };
        eq_f64("f4/extreme-seed", &(s0, s1), vc, vr);
        assert_eq!(sc.state, sr.state);
        assert!(!vc.is_nan() && (0.0..1.0).contains(&vc), "C: {vc}");
    }
}

// ===========================================================================
// Rows 15–16: f5 — silent truncation of bits above bit 15
// ===========================================================================

/// Row 15 — high bits are silently discarded (no error, no rejection).
#[test]
fn err15_f5_high_bits_discarded() {
    let a = apis();
    let mut rng = Rng::new();
    for _ in 0..200_000 {
        let low = rng.below(0x1_0000);
        let high = rng.next_u32() & 0xFFFF_0000;
        let (rc, rr) = unsafe { ((a.c.f5)(low | high), (a.r.f5)(low | high)) };
        eq_u32("f5/high-bits", &(low, high), rc, rr);
        // The C result must equal the result for the low half alone.
        let base = unsafe { (a.c.f5)(low) };
        assert_eq!(rc, base, "high bits must not affect the result (low={low:#x}, high={high:#x})");
        assert!(rc < 0x1_0000, "result must fit in 16 bits: {rc:#x}");
    }
}

/// Row 16 — the extremes.
#[test]
fn err16_f5_extremes() {
    let a = apis();
    for &(v, expect) in &[
        (0u32, 0u32),
        (0xFFFF_FFFF, 0x0000_FFFF),
        (0xFFFF_0000, 0x0000_0000),
        (0x0000_FFFF, 0x0000_FFFF),
        (1, 0x8000),
        (0x8000, 1),
        (0x8000_0000, 0),
    ] {
        let (rc, rr) = unsafe { ((a.c.f5)(v), (a.r.f5)(v)) };
        eq_u32("f5/extreme", &v, rc, rr);
        assert_eq!(rc, expect, "f5({v:#x})");
    }
}

// ===========================================================================
// Rows 17–20: f7 — the predicate switches and unsigned wrap
// ===========================================================================

/// Row 17 — `channels == 2` flips which terms contribute.
#[test]
fn err17_f7_channels_two() {
    let a = apis();
    let mut rng = Rng::new();
    for _ in 0..100_000 {
        let bs = rng.next_u32();
        let bd = rng.next_u32();
        let (rc, rr) = unsafe { ((a.c.f7)(bs, 2, bd), (a.r.f7)(bs, 2, bd)) };
        eq_u32("f7/ch=2", &(bs, bd), rc, rr);
    }
    // one step either side of the special value
    for ch in [0u32, 1, 2, 3] {
        for bd in [0u32, 8, 31, 32, 33, u32::MAX] {
            for bs in [0u32, 1, 4096, u32::MAX] {
                let (rc, rr) = unsafe { ((a.c.f7)(bs, ch, bd), (a.r.f7)(bs, ch, bd)) };
                eq_u32("f7/ch-boundary", &(bs, ch, bd), rc, rr);
            }
        }
    }
}

/// Row 18 — `bitdepth == 32` removes the `+1`.
#[test]
fn err18_f7_bitdepth_32() {
    let a = apis();
    let mut rng = Rng::new();
    for _ in 0..100_000 {
        let bs = rng.next_u32();
        let ch = rng.next_u32();
        let (rc, rr) = unsafe { ((a.c.f7)(bs, ch, 32), (a.r.f7)(bs, ch, 32)) };
        eq_u32("f7/bd=32", &(bs, ch), rc, rr);
    }
    for bd in [31u32, 32, 33] {
        for ch in [1u32, 2, 3] {
            for bs in [0u32, 1, 192, 4096, u32::MAX] {
                let (rc, rr) = unsafe { ((a.c.f7)(bs, ch, bd), (a.r.f7)(bs, ch, bd)) };
                eq_u32("f7/bd-boundary", &(bs, ch, bd), rc, rr);
            }
        }
    }
}

/// Row 19 — oversized arguments: unsigned wrap-around, no range check.
#[test]
fn err19_f7_overflow_wrap() {
    let a = apis();
    let big: &[u32] = &[
        u32::MAX,
        u32::MAX - 1,
        0x8000_0000,
        0xFFFF_FFF8,
        0x1_0000 - 1,
        0x1_0000,
        0x7FFF_FFFF,
    ];
    for &bs in big {
        for &ch in big {
            for &bd in big {
                let (rc, rr) = unsafe { ((a.c.f7)(bs, ch, bd), (a.r.f7)(bs, ch, bd)) };
                eq_u32("f7/overflow", &(bs, ch, bd), rc, rr);
            }
        }
    }
    let mut rng = Rng::new();
    for _ in 0..200_000 {
        let (bs, ch, bd) = (rng.next_u32(), rng.next_u32(), rng.next_u32());
        let (rc, rr) = unsafe { ((a.c.f7)(bs, ch, bd), (a.r.f7)(bs, ch, bd)) };
        eq_u32("f7/overflow-rand", &(bs, ch, bd), rc, rr);
    }
}

/// Row 20 — all-zero arguments: `18 + 0 + (0+7)/8 == 18`.
#[test]
fn err20_f7_zeros() {
    let a = apis();
    let (rc, rr) = unsafe { ((a.c.f7)(0, 0, 0), (a.r.f7)(0, 0, 0)) };
    eq_u32("f7/zeros", &"(0,0,0)", rc, rr);
    assert_eq!(rc, 18, "18 + 0 + (0+7)/8 == 18");
    // each argument zero in turn
    for &(bs, ch, bd) in &[
        (0u32, 2u32, 16u32),
        (4096, 0, 16),
        (4096, 2, 0),
        (0, 0, 16),
        (0, 2, 0),
        (4096, 0, 0),
    ] {
        let (rc, rr) = unsafe { ((a.c.f7)(bs, ch, bd), (a.r.f7)(bs, ch, bd)) };
        eq_u32("f7/one-zero", &(bs, ch, bd), rc, rr);
    }
}

// ===========================================================================
// Rows 21–23: f9 — unchecked division by zero
// ===========================================================================

fn lm(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

#[track_caller]
fn f9_both(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> (lm_vec2, lm_vec2) {
    let a = apis();
    unsafe { ((a.c.f9)(p1, p2, p3, p), (a.r.f9)(p1, p2, p3, p)) }
}

/// Row 21 — degenerate denominator → `invDenom = ±inf`, no rejection.
#[test]
fn err21_f9_degenerate_denominator() {
    let mut rng = Rng::new();
    let mut saw_inf = 0usize;
    let mut saw_nan = 0usize;
    for _ in 0..50_000 {
        // collinear p1/p2/p3 makes dot00*dot11 == dot01*dot01 exactly for many
        // integral inputs
        let ox = (rng.next_u32() % 21) as i32 as f32 - 10.0;
        let oy = (rng.next_u32() % 21) as i32 as f32 - 10.0;
        let dx = (rng.next_u32() % 11) as i32 as f32 - 5.0;
        let dy = (rng.next_u32() % 11) as i32 as f32 - 5.0;
        let s = (rng.next_u32() % 9) as i32 as f32 - 4.0;
        let t = (rng.next_u32() % 9) as i32 as f32 - 4.0;
        let p1 = lm(ox, oy);
        let p2 = lm(ox + dx * s, oy + dy * s);
        let p3 = lm(ox + dx * t, oy + dy * t);
        let p = lm(
            (rng.next_u32() % 21) as i32 as f32 - 10.0,
            (rng.next_u32() % 21) as i32 as f32 - 10.0,
        );
        let (c, r) = f9_both(p1, p2, p3, p);
        eq_lm("f9/degenerate", &"collinear", c, r);
        if c.x.is_infinite() || c.y.is_infinite() {
            saw_inf += 1;
        }
        if c.x.is_nan() || c.y.is_nan() {
            saw_nan += 1;
        }
    }
    assert!(
        saw_inf + saw_nan > 0,
        "the degenerate branch must actually be reached (inf={saw_inf}, nan={saw_nan})"
    );
    // Exactly-zero denominator, deterministic.
    let (c, r) = f9_both(lm(0.0, 0.0), lm(1.0, 0.0), lm(2.0, 0.0), lm(3.0, 4.0));
    eq_lm("f9/exact-degenerate", &"axis", c, r);
    assert!(
        c.x.is_nan() || c.x.is_infinite() || c.y.is_nan() || c.y.is_infinite(),
        "C: {c:?}"
    );
}

/// Row 22 — all four points identical: `0 * inf` → `NaN` (both components).
#[test]
fn err22_f9_all_points_equal() {
    for &v in &[0.0f32, -0.0, 1.0, -1.0, 1e20, 1e-20] {
        let q = lm(v, v);
        let (c, r) = f9_both(q, q, q, q);
        eq_lm("f9/all-equal", &Bits(v), c, r);
        assert!(
            c.x.is_nan() && c.y.is_nan(),
            "C must yield NaN for identical points: {c:?} (v={v})"
        );
    }
    let mut rng = Rng::new();
    for _ in 0..20_000 {
        let q = lm(rng.finite_f32(1e3), rng.finite_f32(1e3));
        let (c, r) = f9_both(q, q, q, q);
        eq_lm("f9/all-equal-rand", &(Bits(q.x), Bits(q.y)), c, r);
    }
}

/// Row 23 — `NaN`/`±inf` coordinates propagate unchecked.
#[test]
fn err23_f9_nan_inf_inputs() {
    for &nb in NAN_ZOO {
        let n = f32::from_bits(nb);
        for slot in 0..8 {
            let mut co = [1.0f32, 0.0, 0.0, 1.0, 2.0, 3.0, 5.0, 7.0];
            co[slot] = n;
            let (c, r) = f9_both(
                lm(co[0], co[1]),
                lm(co[2], co[3]),
                lm(co[4], co[5]),
                lm(co[6], co[7]),
            );
            eq_lm("f9/nan-inf", &(nb, slot), c, r);
        }
    }
    // Every combination of two NaN/inf values in the first two slots.
    for &n1 in NAN_ZOO {
        for &n2 in NAN_ZOO {
            let (c, r) = f9_both(
                lm(f32::from_bits(n1), f32::from_bits(n2)),
                lm(1.0, 0.0),
                lm(0.0, 1.0),
                lm(0.25, 0.25),
            );
            eq_lm("f9/nan-pair", &(n1, n2), c, r);
        }
    }
}

// ===========================================================================
// Row 24: f10 — every uint16_t is in range (no bounds check needed)
// ===========================================================================

#[test]
fn err24_f10_exhaustive_all_65536() {
    let a = apis();
    // Also asserts the C index arithmetic never leaves the table: n = h>>10
    // is at most 63, and (h & 0x3ff) + m__offset[n] is at most 2047.
    for h in 0u16..=u16::MAX {
        let n = (h >> 10) as usize;
        assert!(n <= 63);
        let (rc, rr) = unsafe { ((a.c.f10)(h), (a.r.f10)(h)) };
        eq_f32("f10", &format!("{h:#06x}"), rc, rr);
        if h == u16::MAX {
            break;
        }
    }
    // Named special half-floats: ±0, ±Inf, qNaN, sNaN, min subnormal, max normal
    for &h in &[
        0x0000u16, 0x8000, 0x7C00, 0xFC00, 0x7E00, 0xFE00, 0x7C01, 0xFC01, 0x0001, 0x8001, 0x03FF,
        0x0400, 0x7BFF, 0xFBFF, 0x3C00, 0xBC00, 0x7FFF, 0xFFFF,
    ] {
        let (rc, rr) = unsafe { ((a.c.f10)(h), (a.r.f10)(h)) };
        eq_f32("f10/special", &format!("{h:#06x}"), rc, rr);
    }
}

// ===========================================================================
// Rows 25–30: f11 error / out-of-range paths
// ===========================================================================

/// Row 25 — `s == 0` (and `-0.0`) early-out: `dest[0..3] = l`.
#[test]
fn err25_f11_s_zero() {
    let a = apis();
    let mut rng = Rng::new();
    for &s in &[0.0f32, -0.0] {
        let mut probes: Vec<f32> = zoo_f32();
        for _ in 0..20_000 {
            probes.push(rng.any_f32());
        }
        for l in probes {
            let src = [rng.any_f32(), s, l];
            let mut dc = [0.0f32; 3];
            let mut dr = [0.0f32; 3];
            unsafe {
                (a.c.f11)(dc.as_mut_ptr(), src.as_ptr());
                (a.r.f11)(dr.as_mut_ptr(), src.as_ptr());
            }
            eq_tri("f11/s0", &BitsTri(src), &dc, &dr);
            // the C early-out copies `l` verbatim into all three slots
            for k in 0..3 {
                assert_eq!(
                    dc[k].to_bits(),
                    l.to_bits(),
                    "f11 s==0 must copy l verbatim (k={k}, l={:?})",
                    Bits(l)
                );
            }
        }
    }
}

/// Row 26 — negative hue takes the *third* branch (`h < 120 && h < 180`),
/// which is the C bug. Pinned explicitly.
#[test]
fn err26_f11_negative_hue() {
    let a = apis();
    let mut rng = Rng::new();
    let mut probes: Vec<f32> = vec![-1e-45, -f32::MIN_POSITIVE, -0.001, -1.0, -60.0, -359.0, -1e30, f32::MIN];
    for _ in 0..20_000 {
        probes.push(-rng.finite_f32(1e4).abs() - f32::MIN_POSITIVE);
    }
    for h in probes {
        assert!(h < 0.0);
        let src = [h, 0.75f32, 0.4f32];
        let mut dc = [0.0f32; 3];
        let mut dr = [0.0f32; 3];
        unsafe {
            (a.c.f11)(dc.as_mut_ptr(), src.as_ptr());
            (a.r.f11)(dr.as_mut_ptr(), src.as_ptr());
        }
        eq_tri("f11/negative-hue", &BitsTri(src), &dc, &dr);
        // branch 3 writes {m, c+m, x+m}; with s=0.75, l=0.4:
        //   c = (1 - |2*0.4 - 1|) * 0.75, m = 0.4 - 0.5*c
        // dest[0] must be m (NOT the final-else m,m,m — they coincide in value
        // for dest[0], so distinguish via dest[1] != dest[0]).
        assert_ne!(
            dc[1].to_bits(),
            dc[0].to_bits(),
            "negative hue must hit the buggy third branch, not the final else \
             (h={:?}, got {:?})",
            Bits(h),
            BitsTri(dc)
        );
    }
    // -0.0 is NOT < 0.0, so it takes branch 1 instead.
    let src = [-0.0f32, 0.75, 0.4];
    let mut dc = [0.0f32; 3];
    let mut dr = [0.0f32; 3];
    unsafe {
        (a.c.f11)(dc.as_mut_ptr(), src.as_ptr());
        (a.r.f11)(dr.as_mut_ptr(), src.as_ptr());
    }
    eq_tri("f11/neg-zero-hue", &BitsTri(src), &dc, &dr);
}

/// Row 27 — `h >= 360` → final `else` → `{m, m, m}`.
#[test]
fn err27_f11_hue_ge_360() {
    let a = apis();
    let mut rng = Rng::new();
    let mut probes: Vec<f32> = vec![360.0, 360.00003, 361.0, 720.0, 1e30, f32::MAX];
    for _ in 0..20_000 {
        probes.push(360.0 + rng.finite_f32(1e4).abs());
    }
    for h in probes {
        let src = [h, 0.75f32, 0.4f32];
        let mut dc = [0.0f32; 3];
        let mut dr = [0.0f32; 3];
        unsafe {
            (a.c.f11)(dc.as_mut_ptr(), src.as_ptr());
            (a.r.f11)(dr.as_mut_ptr(), src.as_ptr());
        }
        eq_tri("f11/h>=360", &BitsTri(src), &dc, &dr);
        assert_eq!(dc[0].to_bits(), dc[1].to_bits(), "final else -> {{m,m,m}}");
        assert_eq!(dc[1].to_bits(), dc[2].to_bits(), "final else -> {{m,m,m}}");
    }
}

/// Row 28 — `h` NaN → all comparisons false → final `else`.
#[test]
fn err28_f11_nan_hue() {
    let a = apis();
    for &nb in NAN_ZOO {
        let h = f32::from_bits(nb);
        if !h.is_nan() {
            continue;
        }
        for &(s, l) in &[(0.75f32, 0.4f32), (1.0, 0.5), (-1.0, 2.0), (2.0, -1.0)] {
            let src = [h, s, l];
            let mut dc = [0.0f32; 3];
            let mut dr = [0.0f32; 3];
            unsafe {
                (a.c.f11)(dc.as_mut_ptr(), src.as_ptr());
                (a.r.f11)(dr.as_mut_ptr(), src.as_ptr());
            }
            eq_tri("f11/nan-hue", &BitsTri(src), &dc, &dr);
            assert_eq!(dc[0].to_bits(), dc[1].to_bits());
            assert_eq!(dc[1].to_bits(), dc[2].to_bits());
        }
    }
}

/// Row 29 — `h = ±inf`: `+inf` → final else, `-inf` → the buggy third branch.
#[test]
fn err29_f11_inf_hue() {
    let a = apis();
    for &(s, l) in &[(0.75f32, 0.4f32), (1.0, 0.5), (0.5, 1.0), (-1.0, -1.0)] {
        for &h in &[f32::INFINITY, f32::NEG_INFINITY] {
            let src = [h, s, l];
            let mut dc = [0.0f32; 3];
            let mut dr = [0.0f32; 3];
            unsafe {
                (a.c.f11)(dc.as_mut_ptr(), src.as_ptr());
                (a.r.f11)(dr.as_mut_ptr(), src.as_ptr());
            }
            eq_tri("f11/inf-hue", &BitsTri(src), &dc, &dr);
        }
        // +inf must land in the final else -> all three equal
        let src = [f32::INFINITY, s, l];
        let mut dc = [0.0f32; 3];
        unsafe {
            (a.c.f11)(dc.as_mut_ptr(), src.as_ptr());
        }
        assert_eq!(dc[0].to_bits(), dc[2].to_bits(), "+inf -> {{m,m,m}}");
    }
}

/// Row 30 — `h ∈ [120, 180)`: the range the C bug makes unreachable, so it
/// falls all the way through to the final `else`.
#[test]
fn err30_f11_dead_range_120_180() {
    let a = apis();
    let mut rng = Rng::new();
    let mut probes: Vec<f32> = vec![120.0, 120.00001, 150.0, 179.0, 179.99998];
    for _ in 0..20_000 {
        let frac = rng.next_u32() as f64 / (u32::MAX as f64 + 1.0);
        probes.push((120.0 + frac * 60.0) as f32);
    }
    for h in probes {
        assert!((120.0..180.0).contains(&h), "h={h}");
        let src = [h, 0.75f32, 0.4f32];
        let mut dc = [0.0f32; 3];
        let mut dr = [0.0f32; 3];
        unsafe {
            (a.c.f11)(dc.as_mut_ptr(), src.as_ptr());
            (a.r.f11)(dr.as_mut_ptr(), src.as_ptr());
        }
        eq_tri("f11/dead-range", &BitsTri(src), &dc, &dr);
        assert_eq!(
            dc[0].to_bits(),
            dc[1].to_bits(),
            "[120,180) must reach the final else (h={h}, got {:?})",
            BitsTri(dc)
        );
        assert_eq!(dc[1].to_bits(), dc[2].to_bits());
    }
}

// ===========================================================================
// Rows 32–35: f12 error / out-of-range paths
// ===========================================================================

/// Row 32 — `s == 0` early-out: `dest[0..3] = v`.
#[test]
fn err32_f12_s_zero() {
    let a = apis();
    let mut rng = Rng::new();
    for &s in &[0.0f32, -0.0] {
        let mut probes: Vec<f32> = zoo_f32();
        for _ in 0..20_000 {
            probes.push(rng.any_f32());
        }
        for v in probes {
            let src = [rng.any_f32(), s, v];
            let mut dc = [0.0f32; 3];
            let mut dr = [0.0f32; 3];
            unsafe {
                (a.c.f12)(dc.as_mut_ptr(), src.as_ptr());
                (a.r.f12)(dr.as_mut_ptr(), src.as_ptr());
            }
            eq_tri("f12/s0", &BitsTri(src), &dc, &dr);
            for k in 0..3 {
                assert_eq!(dc[k].to_bits(), v.to_bits(), "f12 s==0 copies v verbatim");
            }
        }
    }
}

/// Row 33 — `i` outside `0..=4` → `switch default:` → `{v, p, q}`.
#[test]
fn err33_f12_i_default_branch() {
    let a = apis();
    // i == 5 (h in [300,360)), i == 6, i negative, and far out.
    let mut hs: Vec<f32> = vec![
        300.0, 330.0, 359.999, 360.0, 420.0, 600.0, -0.001, -60.0, -1e6, 1e6,
    ];
    let mut rng = Rng::new();
    for _ in 0..20_000 {
        hs.push(if rng.next_u32() % 2 == 0 {
            300.0 + (rng.next_u32() as f64 / u32::MAX as f64 * 60.0) as f32
        } else {
            -(rng.next_u32() as f64 / u32::MAX as f64 * 1e5) as f32 - 1.0
        });
    }
    for h in hs {
        let src = [h, 0.6f32, 0.9f32];
        let mut dc = [0.0f32; 3];
        let mut dr = [0.0f32; 3];
        unsafe {
            (a.c.f12)(dc.as_mut_ptr(), src.as_ptr());
            (a.r.f12)(dr.as_mut_ptr(), src.as_ptr());
        }
        eq_tri("f12/default", &BitsTri(src), &dc, &dr);
        // default arm sets r = v exactly
        assert_eq!(
            dc[0].to_bits(),
            0.9f32.to_bits(),
            "default arm sets r = v (h={h}, got {:?})",
            BitsTri(dc)
        );
    }
}

/// Row 34 — `h` NaN → `(int)floorf(NaN)` is UB; x86 `cvttss2si` gives
/// `INT_MIN` → `default:`.
#[test]
fn err34_f12_nan_hue() {
    let a = apis();
    for &nb in NAN_ZOO {
        let h = f32::from_bits(nb);
        if !h.is_nan() {
            continue;
        }
        for &(s, v) in &[(0.6f32, 0.9f32), (1.0, 1.0), (-1.0, 2.0), (2.0, -1.0)] {
            let src = [h, s, v];
            let mut dc = [0.0f32; 3];
            let mut dr = [0.0f32; 3];
            unsafe {
                (a.c.f12)(dc.as_mut_ptr(), src.as_ptr());
                (a.r.f12)(dr.as_mut_ptr(), src.as_ptr());
            }
            eq_tri("f12/nan-hue", &BitsTri(src), &dc, &dr);
            assert_eq!(dc[0].to_bits(), v.to_bits(), "NaN hue -> default arm, r = v");
        }
    }
}

/// Row 35 — `h/60` not representable as `int` → `cvttss2si` "integer
/// indefinite" (`INT_MIN`) → `default:`.
#[test]
fn err35_f12_hue_out_of_int_range() {
    let a = apis();
    // Values whose /60 is at, just below, and beyond INT_MAX / INT_MIN.
    let probes: &[f32] = &[
        2147483520.0 * 60.0,
        2147483648.0 * 60.0,
        -2147483648.0 * 60.0,
        -2147483904.0 * 60.0,
        f32::MAX,
        f32::MIN,
        1e30,
        -1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        2147483648.0,
        -2147483648.0,
        2147483520.0 * 60.0 + 60.0,
    ];
    for &h in probes {
        for &(s, v) in &[(0.6f32, 0.9f32), (1.0, 1.0), (-2.0, -3.0)] {
            let src = [h, s, v];
            let mut dc = [0.0f32; 3];
            let mut dr = [0.0f32; 3];
            unsafe {
                (a.c.f12)(dc.as_mut_ptr(), src.as_ptr());
                (a.r.f12)(dr.as_mut_ptr(), src.as_ptr());
            }
            eq_tri("f12/out-of-int-range", &BitsTri(src), &dc, &dr);
        }
    }
}

// ===========================================================================
// Rows 37–40: f13 error / out-of-range paths
// ===========================================================================

/// Row 37 — `delta == 0` (`r == g == b`) → `{0, 0, max}`.
#[test]
fn err37_f13_delta_zero() {
    let a = apis();
    let mut rng = Rng::new();
    let mut probes: Vec<f32> = zoo_f32();
    for _ in 0..20_000 {
        probes.push(rng.finite_f32(1e6));
    }
    for v in probes {
        let src = [v, v, v];
        let mut dc = [0.0f32; 3];
        let mut dr = [0.0f32; 3];
        unsafe {
            (a.c.f13)(dc.as_mut_ptr(), src.as_ptr());
            (a.r.f13)(dr.as_mut_ptr(), src.as_ptr());
        }
        eq_tri("f13/delta0", &BitsTri(src), &dc, &dr);
        // NaN inputs make `delta` NaN, which is NOT == 0 -> not the early-out.
        if !v.is_nan() && v.is_finite() {
            assert_eq!(dc[0].to_bits(), 0f32.to_bits(), "h == +0.0");
            assert_eq!(dc[1].to_bits(), 0f32.to_bits(), "s == +0.0");
            assert_eq!(dc[2].to_bits(), v.to_bits(), "v == max");
        }
    }
}

/// Row 38 — `max == 0` → early out `{0, 0, 0}`.
#[test]
fn err38_f13_max_zero() {
    let a = apis();
    let mut rng = Rng::new();
    for _ in 0..40_000 {
        let x = -rng.finite_f32(1e3).abs();
        let y = -rng.finite_f32(1e3).abs();
        for src in [[0.0f32, x, y], [x, 0.0, y], [x, y, 0.0], [-0.0f32, x, y]] {
            let mut dc = [0.0f32; 3];
            let mut dr = [0.0f32; 3];
            unsafe {
                (a.c.f13)(dc.as_mut_ptr(), src.as_ptr());
                (a.r.f13)(dr.as_mut_ptr(), src.as_ptr());
            }
            eq_tri("f13/max0", &BitsTri(src), &dc, &dr);
            if x < 0.0 && y < 0.0 {
                assert_eq!(dc[0].to_bits(), 0f32.to_bits());
                assert_eq!(dc[1].to_bits(), 0f32.to_bits());
            }
        }
    }
    // Pure black.
    for src in [[0.0f32; 3], [-0.0f32; 3], [0.0, -0.0, 0.0]] {
        let mut dc = [0.0f32; 3];
        let mut dr = [0.0f32; 3];
        unsafe {
            (a.c.f13)(dc.as_mut_ptr(), src.as_ptr());
            (a.r.f13)(dr.as_mut_ptr(), src.as_ptr());
        }
        eq_tri("f13/black", &BitsTri(src), &dc, &dr);
    }
}

/// Row 39 — NaN component: the min/max ternaries pick the *second* operand on
/// an unordered compare, `delta` becomes NaN, and the early-out is skipped.
#[test]
fn err39_f13_nan_component() {
    let a = apis();
    for &nb in NAN_ZOO {
        let n = f32::from_bits(nb);
        for &o1 in &[0.0f32, 1.0, -1.0, 0.5, f32::INFINITY, f32::NEG_INFINITY] {
            for &o2 in &[0.0f32, 1.0, -1.0, 0.5, f32::INFINITY] {
                for src in [[n, o1, o2], [o1, n, o2], [o1, o2, n], [n, n, o1], [n, o1, n]] {
                    let mut dc = [0.0f32; 3];
                    let mut dr = [0.0f32; 3];
                    unsafe {
                        (a.c.f13)(dc.as_mut_ptr(), src.as_ptr());
                        (a.r.f13)(dr.as_mut_ptr(), src.as_ptr());
                    }
                    eq_tri("f13/nan-component", &BitsTri(src), &dc, &dr);
                }
            }
        }
    }
}

/// Row 40 — `h < 0` after `h *= 60` → `h += 360` wrap. Must actually fire.
#[test]
fn err40_f13_negative_hue_wrap() {
    let a = apis();
    let mut rng = Rng::new();
    let mut fired = 0usize;
    for _ in 0..40_000 {
        // r == max and g < b makes (g-b)/delta negative -> h < 0.
        let r = rng.finite_f32(10.0).abs() + 1.0;
        let g = rng.finite_f32(1.0).abs() * 0.1;
        let b = g + rng.finite_f32(1.0).abs() * 0.5 + 1e-6;
        let src = [r, g, b.min(r)];
        let mut dc = [0.0f32; 3];
        let mut dr = [0.0f32; 3];
        unsafe {
            (a.c.f13)(dc.as_mut_ptr(), src.as_ptr());
            (a.r.f13)(dr.as_mut_ptr(), src.as_ptr());
        }
        eq_tri("f13/hue-wrap", &BitsTri(src), &dc, &dr);
        if dc[0] > 180.0 {
            fired += 1;
            assert!(dc[0] <= 360.0, "wrapped hue must be <= 360: {:?}", dc[0]);
        }
    }
    assert!(fired > 0, "the h += 360 wrap must actually fire");
    // Deterministic: r=1, g=0, b=0.5 -> h = (0-0.5)/1*60 = -30 -> 330
    let src = [1.0f32, 0.0, 0.5];
    let mut dc = [0.0f32; 3];
    let mut dr = [0.0f32; 3];
    unsafe {
        (a.c.f13)(dc.as_mut_ptr(), src.as_ptr());
        (a.r.f13)(dr.as_mut_ptr(), src.as_ptr());
    }
    eq_tri("f13/hue-wrap-exact", &BitsTri(src), &dc, &dr);
    assert_eq!(dc[0], 330.0f32, "expected 330 degrees, got {:?}", dc[0]);
}

// ===========================================================================
// Rows 42–48: agglom's isnan guards, and that ±inf is NOT skipped
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn call_agglom(f: FnAgglom, v: &AgglomArgs) -> f64 {
    unsafe {
        f(
            v.f[0], v.f[1], v.f[2], v.f[3], v.f[4], v.f[5], v.f[6], v.i[0], v.i[1], v.u64[0],
            v.u64[1], v.u32[0], v.u32[1], v.u32[2], v.u32[3], v.f[7], v.f[8], v.f[9], v.f[10],
            v.f[11], v.f[12], v.f[13], v.f[14], v.h, v.f[15], v.f[16], v.f[17], v.f[18], v.f[19],
            v.f[20], v.f[21], v.f[22], v.f[23],
        )
    }
}

#[derive(Copy, Clone, Debug)]
struct AgglomArgs {
    f: [f32; 24],
    i: [c_int; 2],
    u64: [u64; 2],
    u32: [u32; 4],
    h: u16,
}

impl AgglomArgs {
    fn zeros() -> AgglomArgs {
        AgglomArgs {
            f: [0.0; 24],
            i: [0; 2],
            u64: [0; 2],
            u32: [0; 4],
            h: 0,
        }
    }
}

#[track_caller]
fn diff_agglom(tag: &str, v: &AgglomArgs) -> f64 {
    let a = apis();
    let c = call_agglom(a.c.agglom, v);
    let r = call_agglom(a.r.agglom, v);
    eq_f64(tag, v, c, r);
    c
}

/// Rows 42–47 — every `isnan(...)` guard: the NaN contribution is skipped, so
/// the accumulated `ret` stays finite.
#[test]
fn err42_47_agglom_nan_skips() {
    // Row 44: f10 decoding to a half-float NaN.
    for &h in &[0x7E00u16, 0x7FFF, 0xFE00, 0xFFFF, 0x7C01, 0xFC01] {
        let mut v = AgglomArgs::zeros();
        v.h = h;
        let c = diff_agglom("agglom/f10-nan-skip", &v);
        assert!(
            !c.is_nan(),
            "the isnan guard must skip the f10 NaN (h={h:#06x}, got {c})"
        );
    }
    // Rows 43, 45–47: f9 degenerate + NaN-producing f11/f12/f13 inputs.
    for &nb in NAN_ZOO {
        let n = f32::from_bits(nb);
        if !n.is_nan() {
            continue;
        }
        // f9: all four points equal -> u, v both NaN -> both skipped
        let mut v = AgglomArgs::zeros();
        for k in 7..15 {
            v.f[k] = if k % 2 == 0 { 1.0 } else { 2.0 };
        }
        let c = diff_agglom("agglom/f9-nan-skip", &v);
        assert!(!c.is_nan(), "f9 NaN components must be skipped (got {c})");

        // f11 / f12 / f13 with a NaN argument each
        for slot in 15..24 {
            let mut v = AgglomArgs::zeros();
            v.f[16] = 0.5; // f11_3 (s) != 0
            v.f[19] = 0.5; // f12_3 (s) != 0
            v.f[slot] = n;
            let c = diff_agglom("agglom/colour-nan-skip", &v);
            assert!(
                !c.is_nan(),
                "every colour NaN must be skipped (slot={slot}, nan={nb:#010x}, got {c})"
            );
        }
        // All 24 floats NaN at once: every contribution skipped, leaving only
        // the integer subsystems.
        let mut v = AgglomArgs::zeros();
        v.f = [n; 24];
        let c = diff_agglom("agglom/all-nan-skip", &v);
        assert!(!c.is_nan(), "all-NaN floats must all be skipped (got {c})");
    }
    // Row 42: the f4 guard. `f4` mathematically cannot produce NaN (it builds
    // a double with exponent 1023 and subtracts 1.0), so the guard is dead.
    // Verify that claim across many seeds rather than assuming it.
    let a = apis();
    let mut rng = Rng::new();
    for _ in 0..200_000 {
        let (s0, s1) = (rng.next_u64(), rng.next_u64());
        let mut sc = cn_rnd_t { state: [s0, s1] };
        let x = unsafe { (a.c.f4)(&mut sc) };
        assert!(!x.is_nan(), "f4 produced NaN for seed {s0:#x},{s1:#x}");
    }
}

/// Row 48 — `±inf` contributions are NOT skipped (only NaN is), so `ret` can
/// become `±inf` or even `NaN` when both signs are added.
#[test]
fn err48_agglom_inf_not_skipped() {
    let mut saw_inf = false;
    let mut saw_nan_from_inf = false;
    // f13 with r=+inf, b=-inf: delta = inf - (-inf) = inf, s = inf/inf = NaN...
    // drive several shapes and check both libs agree, and that at least one
    // makes `ret` infinite (proving inf is not filtered).
    for &(p, q) in &[
        (f32::INFINITY, 1.0f32),
        (f32::NEG_INFINITY, 1.0),
        (f32::INFINITY, f32::NEG_INFINITY),
        (f32::MAX, f32::MAX),
    ] {
        let mut v = AgglomArgs::zeros();
        v.f[16] = 0.5; // f11 s
        v.f[19] = 0.5; // f12 s
        v.f[17] = p; // f11 l
        v.f[20] = q; // f12 v
        let c = diff_agglom("agglom/inf", &v);
        if c.is_infinite() {
            saw_inf = true;
        }
        if c.is_nan() {
            saw_nan_from_inf = true;
        }

        // f12 with v = +inf and s finite -> p/q/t = ±inf
        let mut w = AgglomArgs::zeros();
        w.f[18] = 90.0; // f12 h
        w.f[19] = 0.5; // f12 s
        w.f[20] = f32::INFINITY;
        let c2 = diff_agglom("agglom/inf-f12", &w);
        if c2.is_infinite() {
            saw_inf = true;
        }
        if c2.is_nan() {
            saw_nan_from_inf = true;
        }
    }
    assert!(
        saw_inf,
        "at least one configuration must leave ret infinite, proving the \
         isnan guards do not filter infinities"
    );
    // Both +inf and -inf added -> inf + (-inf) = NaN escapes into the result.
    let mut v = AgglomArgs::zeros();
    v.f[16] = 0.5;
    v.f[17] = f32::INFINITY; // f11 l -> +inf contributions
    v.f[19] = 0.5;
    v.f[20] = f32::NEG_INFINITY; // f12 v -> -inf contributions
    let c = diff_agglom("agglom/inf-cancel", &v);
    if c.is_nan() {
        saw_nan_from_inf = true;
    }
    // Not asserting NaN must happen (depends on which branches fire), but the
    // C/Rust agreement above is the actual requirement.
    let _ = saw_nan_from_inf;
}

// ===========================================================================
// Rows 49–52: NaN/unordered behaviour of the collision predicates
// ===========================================================================

/// Row 49 — `d2 < r2` is false when unordered → the predicates return `0`.
#[test]
fn err49_c2_nan_compare() {
    let a = apis();
    for &nb in NAN_ZOO {
        let n = f32::from_bits(nb);
        if !n.is_nan() {
            continue;
        }
        // NaN radius
        let A = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: n,
        };
        let B = c2Circle {
            p: c2v { x: 1.0, y: 1.0 },
            r: 1.0,
        };
        let (rc, rr) = unsafe { ((a.c.c2CircletoCircle)(A, B), (a.r.c2CircletoCircle)(A, B)) };
        eq_i32("c2CircletoCircle/nan-r", &nb, rc, rr);
        assert_eq!(rc, 0, "unordered compare -> 0");

        // NaN centre coordinate
        let A2 = c2Circle {
            p: c2v { x: n, y: 0.0 },
            r: 5.0,
        };
        let (rc, rr) = unsafe { ((a.c.c2CircletoCircle)(A2, B), (a.r.c2CircletoCircle)(A2, B)) };
        eq_i32("c2CircletoCircle/nan-p", &nb, rc, rr);
        assert_eq!(rc, 0);

        // c2CircletoAABB with a NaN radius
        let box_ = c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        let A3 = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: n,
        };
        let (rc, rr) = unsafe { ((a.c.c2CircletoAABB)(A3, box_), (a.r.c2CircletoAABB)(A3, box_)) };
        eq_i32("c2CircletoAABB/nan-r", &nb, rc, rr);
        assert_eq!(rc, 0);
    }
}

/// Row 50 — `c2AABBtoAABB` with NaN coordinates reports overlap (`1`), because
/// every `<` is false and the result is `!(0|0|0|0)`.
#[test]
fn err50_c2_aabb_nan_reports_overlap() {
    let a = apis();
    for &nb in NAN_ZOO {
        let n = f32::from_bits(nb);
        if !n.is_nan() {
            continue;
        }
        let all_nan = c2AABB {
            min: c2v { x: n, y: n },
            max: c2v { x: n, y: n },
        };
        let far = c2AABB {
            min: c2v { x: 1e9, y: 1e9 },
            max: c2v { x: 2e9, y: 2e9 },
        };
        let (rc, rr) = unsafe {
            (
                (a.c.c2AABBtoAABB)(all_nan, far),
                (a.r.c2AABBtoAABB)(all_nan, far),
            )
        };
        eq_i32("c2AABBtoAABB/all-nan", &nb, rc, rr);
        assert_eq!(rc, 1, "all-NaN box must report overlap even with a far box");
        let (rc, rr) = unsafe {
            (
                (a.c.c2AABBtoAABB)(far, all_nan),
                (a.r.c2AABBtoAABB)(far, all_nan),
            )
        };
        eq_i32("c2AABBtoAABB/all-nan-rev", &nb, rc, rr);
        assert_eq!(rc, 1);
        // One NaN coordinate at a time.
        for slot in 0..4 {
            let mut co = [0.0f32, 0.0, 1.0, 1.0];
            co[slot] = n;
            let A = c2AABB {
                min: c2v { x: co[0], y: co[1] },
                max: c2v { x: co[2], y: co[3] },
            };
            let (rc, rr) = unsafe { ((a.c.c2AABBtoAABB)(A, far), (a.r.c2AABBtoAABB)(A, far)) };
            eq_i32("c2AABBtoAABB/one-nan", &(nb, slot), rc, rr);
        }
    }
}

/// Row 51 — `c2Maxv`/`c2Minv` NaN asymmetry: the ternary yields `b` when the
/// compare is unordered, so `c2Maxv(NaN, x) == x` but `c2Maxv(x, NaN) == NaN`.
#[test]
fn err51_c2_minmax_nan_asymmetry() {
    let a = apis();
    for &nb in NAN_ZOO {
        let n = f32::from_bits(nb);
        if !n.is_nan() {
            continue;
        }
        let nan_v = c2v { x: n, y: n };
        let one = c2v { x: 1.0, y: 1.0 };
        unsafe {
            let (c1, r1) = ((a.c.c2Maxv)(nan_v, one), (a.r.c2Maxv)(nan_v, one));
            eq_v2("c2Maxv(NaN,1)", &nb, c1, r1);
            assert_eq!(c1.x.to_bits(), 1.0f32.to_bits(), "picks b on unordered");

            let (c2, r2) = ((a.c.c2Maxv)(one, nan_v), (a.r.c2Maxv)(one, nan_v));
            eq_v2("c2Maxv(1,NaN)", &nb, c2, r2);
            assert!(c2.x.is_nan(), "picks b == NaN on unordered");

            let (c3, r3) = ((a.c.c2Minv)(nan_v, one), (a.r.c2Minv)(nan_v, one));
            eq_v2("c2Minv(NaN,1)", &nb, c3, r3);
            assert_eq!(c3.x.to_bits(), 1.0f32.to_bits());

            let (c4, r4) = ((a.c.c2Minv)(one, nan_v), (a.r.c2Minv)(one, nan_v));
            eq_v2("c2Minv(1,NaN)", &nb, c4, r4);
            assert!(c4.x.is_nan());

            // and through c2Clampv, where the asymmetry composes
            for (p, lo, hi) in [
                (nan_v, one, one),
                (one, nan_v, one),
                (one, one, nan_v),
                (nan_v, nan_v, one),
            ] {
                let (cc, cr) = ((a.c.c2Clampv)(p, lo, hi), (a.r.c2Clampv)(p, lo, hi));
                eq_v2("c2Clampv/nan", &nb, cc, cr);
            }
        }
    }
}

/// Row 52 — `c2Clampv` with an inverted box: no validation, `lo` wins.
#[test]
fn err52_c2_clampv_inverted_box() {
    let a = apis();
    let mut rng = Rng::new();
    let lo = c2v { x: 5.0, y: 6.0 };
    let hi = c2v { x: -5.0, y: -6.0 };
    let mut probes: Vec<c2v> = vec![
        c2v { x: -100.0, y: -100.0 },
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 100.0, y: 100.0 },
        c2v { x: 5.0, y: 6.0 },
        c2v { x: -5.0, y: -6.0 },
    ];
    for _ in 0..20_000 {
        probes.push(c2v {
            x: rng.finite_f32(100.0),
            y: rng.finite_f32(100.0),
        });
    }
    for p in probes {
        let (c, r) = unsafe { ((a.c.c2Clampv)(p, lo, hi), (a.r.c2Clampv)(p, lo, hi)) };
        eq_v2("c2Clampv/inverted", &(Bits(p.x), Bits(p.y)), c, r);
        // max(lo, min(p, hi)) with lo > hi always yields lo for finite input
        if p.x.is_finite() {
            assert_eq!(c.x.to_bits(), lo.x.to_bits(), "lo must win (p.x={:?})", p.x);
            assert_eq!(c.y.to_bits(), lo.y.to_bits());
        }
    }
    // Also through c2CircletoAABB, which clamps internally.
    for _ in 0..20_000 {
        let A = c2Circle {
            p: c2v {
                x: rng.finite_f32(100.0),
                y: rng.finite_f32(100.0),
            },
            r: rng.finite_f32(50.0),
        };
        let B = c2AABB { min: lo, max: hi };
        let (c, r) = unsafe { ((a.c.c2CircletoAABB)(A, B), (a.r.c2CircletoAABB)(A, B)) };
        eq_i32("c2CircletoAABB/inverted-box", &(Bits(A.p.x), Bits(A.r)), c, r);
    }
}
