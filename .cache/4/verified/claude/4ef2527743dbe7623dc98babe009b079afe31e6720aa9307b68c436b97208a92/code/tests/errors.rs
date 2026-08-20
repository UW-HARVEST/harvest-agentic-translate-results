//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`. Both `.so`s are loaded via `libloading`; every assertion
//! compares the *exact* returned error code / sentinel / bit pattern.

#![allow(non_snake_case)]

mod common;

use std::ffi::c_void;

use common::*;

/// Values that are not valid `C2_TYPE` variants (C enums accept any `int`).
const BAD_TYPES: &[i32] = &[
    -2147483648,
    -2147483647,
    -1000,
    -2,
    -1,
    2,
    3,
    4,
    5,
    255,
    256,
    65536,
    1000000,
    2147483646,
    2147483647,
];

// ---------------------------------------------------------------------------
// E01–E04 — `f2` dispatch rejection paths
// ---------------------------------------------------------------------------

/// E01 — `typeA` out of range ⇒ outer `switch` `default:` ⇒ `return 0`
#[test]
fn e01_f2_bad_typeA() {
    let p = both();
    let mut rng = Rng::new(0xE01);
    for i in 0..2_000 {
        let a = rng.wild_circle();
        let b = rng.wild_aabb();
        let pa = &a as *const C2Circle as *const c_void;
        let pb = &b as *const C2Aabb as *const c_void;
        for &ta in BAD_TYPES {
            for &tb in &[C2_TYPE_CIRCLE, C2_TYPE_AABB, 2, -1] {
                let c = unsafe { (p.c.f2)(pa, ta, pb, tb) };
                let r = unsafe { (p.r.f2)(pa, ta, pb, tb) };
                eq_i32(&format!("e01[{i}] f2(typeA={ta}, typeB={tb})"), c, r);
                assert_eq!(c, 0, "e01: C must reject typeA={ta} with 0, got {c}");
            }
        }
    }
}

/// E02 — `typeA == CIRCLE`, `typeB` out of range ⇒ inner `default:` ⇒ `0`
#[test]
fn e02_f2_circle_bad_typeB() {
    let p = both();
    let mut rng = Rng::new(0xE02);
    for i in 0..2_000 {
        let a = rng.wild_circle();
        let b = rng.wild_circle();
        let pa = &a as *const C2Circle as *const c_void;
        let pb = &b as *const C2Circle as *const c_void;
        for &tb in BAD_TYPES {
            let c = unsafe { (p.c.f2)(pa, C2_TYPE_CIRCLE, pb, tb) };
            let r = unsafe { (p.r.f2)(pa, C2_TYPE_CIRCLE, pb, tb) };
            eq_i32(&format!("e02[{i}] f2(CIRCLE, typeB={tb})"), c, r);
            assert_eq!(c, 0, "e02: C must reject typeB={tb} with 0, got {c}");
        }
    }
}

/// E03 — `typeA == AABB`, `typeB` out of range ⇒ inner `default:` ⇒ `0`
#[test]
fn e03_f2_aabb_bad_typeB() {
    let p = both();
    let mut rng = Rng::new(0xE03);
    for i in 0..2_000 {
        let a = rng.wild_aabb();
        let b = rng.wild_aabb();
        let pa = &a as *const C2Aabb as *const c_void;
        let pb = &b as *const C2Aabb as *const c_void;
        for &tb in BAD_TYPES {
            let c = unsafe { (p.c.f2)(pa, C2_TYPE_AABB, pb, tb) };
            let r = unsafe { (p.r.f2)(pa, C2_TYPE_AABB, pb, tb) };
            eq_i32(&format!("e03[{i}] f2(AABB, typeB={tb})"), c, r);
            assert_eq!(c, 0, "e03: C must reject typeB={tb} with 0, got {c}");
        }
    }
}

/// E04 — out-of-range `typeA` makes even `NULL` pointers safe (never deref'd)
#[test]
fn e04_f2_bad_type_null_ptrs() {
    let p = both();
    for &ta in BAD_TYPES {
        for &tb in BAD_TYPES {
            let c = unsafe { (p.c.f2)(std::ptr::null(), ta, std::ptr::null(), tb) };
            let r = unsafe { (p.r.f2)(std::ptr::null(), ta, std::ptr::null(), tb) };
            eq_i32(&format!("e04 f2(NULL, {ta}, NULL, {tb})"), c, r);
            assert_eq!(c, 0);
        }
    }
    // typeA valid but typeB invalid: the C still returns before dereferencing.
    for &tb in BAD_TYPES {
        for &ta in &[C2_TYPE_CIRCLE, C2_TYPE_AABB] {
            let c = unsafe { (p.c.f2)(std::ptr::null(), ta, std::ptr::null(), tb) };
            let r = unsafe { (p.r.f2)(std::ptr::null(), ta, std::ptr::null(), tb) };
            eq_i32(&format!("e04 f2(NULL, {ta}, NULL, {tb})"), c, r);
            assert_eq!(c, 0);
        }
    }
}

/// E31 — full cross-product of out-of-range enum values across the FFI boundary
#[test]
fn e31_f2_enum_cross_product() {
    let p = both();
    let mut rng = Rng::new(0xE31);
    let mut types: Vec<i32> = (-4..=5).collect();
    types.extend_from_slice(BAD_TYPES);
    for _ in 0..40 {
        types.push(rng.next_i32());
    }
    // Buffer large enough to be reinterpreted as either shape.
    let buf: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
    let ptr = buf.as_ptr() as *const c_void;
    for &ta in &types {
        for &tb in &types {
            let c = unsafe { (p.c.f2)(ptr, ta, ptr, tb) };
            let r = unsafe { (p.r.f2)(ptr, ta, ptr, tb) };
            eq_i32(&format!("e31 f2(_, {ta}, _, {tb})"), c, r);
            if !(0..=1).contains(&ta) || !(0..=1).contains(&tb) {
                assert_eq!(c, 0, "e31: C must return 0 for ({ta},{tb}), got {c}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E05–E11 — `f3` guards and overflow-avoidance arms
// ---------------------------------------------------------------------------

fn f3_eq(p: &Pair, v1: i32, v2: i32, ctx: &str) -> i32 {
    let c = unsafe { (p.c.f3)(v1, v2) };
    let r = unsafe { (p.r.f3)(v1, v2) };
    eq_i32(&format!("{ctx} f3({v1}, {v2})"), c, r);
    c
}

/// E05 — `v2 == 0` ⇒ `return 0`
#[test]
fn e05_f3_zero_divisor() {
    let p = both();
    let mut rng = Rng::new(0xE05);
    for v1 in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        let got = f3_eq(&p, v1, 0, "e05");
        assert_eq!(got, 0, "e05: f3({v1}, 0) must be 0, got {got}");
    }
    for i in 0..100_000 {
        let v1 = rng.next_i32();
        let got = f3_eq(&p, v1, 0, &format!("e05[{i}]"));
        assert_eq!(got, 0, "e05: f3({v1}, 0) must be 0");
    }
}

/// E06 — `v1 >= 0`, `v2 == INT_MIN` ⇒ `q = 0, r = v1` ⇒ `0`
#[test]
fn e06_f3_v2_intmin() {
    let p = both();
    let mut rng = Rng::new(0xE06);
    for v1 in [0i32, 1, 2, 12345, i32::MAX - 1, i32::MAX] {
        let got = f3_eq(&p, v1, i32::MIN, "e06");
        assert_eq!(got, 0, "e06: f3({v1}, INT_MIN) must be 0, got {got}");
    }
    for i in 0..50_000 {
        let v1 = (rng.next_u32() >> 1) as i32;
        let got = f3_eq(&p, v1, i32::MIN, &format!("e06[{i}]"));
        assert_eq!(got, 0);
    }
}

/// E07 — `v1 < 0 (!= INT_MIN)`, `v2 == INT_MIN` ⇒ `q = 1, r = v1 - v2` ⇒ `1`
#[test]
fn e07_f3_v1neg_v2_intmin() {
    let p = both();
    let mut rng = Rng::new(0xE07);
    for v1 in [-1i32, -2, -12345, i32::MIN + 1] {
        let got = f3_eq(&p, v1, i32::MIN, "e07");
        assert_eq!(got, 1, "e07: f3({v1}, INT_MIN) must be 1, got {got}");
    }
    for i in 0..50_000 {
        let v1 = -(((rng.next_u32() >> 1) as i32).max(1));
        let got = f3_eq(&p, v1, i32::MIN, &format!("e07[{i}]"));
        assert_eq!(got, 1);
    }
}

/// E08 — `v1 == INT_MIN`, `v2 >= 1` ⇒ the `-(v1+v2)` arm
#[test]
fn e08_f3_v1_intmin_v2_pos() {
    let p = both();
    let mut rng = Rng::new(0xE08);
    for v2 in [1i32, 2, 3, 7, 12345, i32::MAX - 1, i32::MAX] {
        f3_eq(&p, i32::MIN, v2, "e08");
    }
    for i in 0..100_000 {
        let v2 = ((rng.next_u32() >> 1) as i32).max(1);
        f3_eq(&p, i32::MIN, v2, &format!("e08[{i}]"));
    }
    for v2 in 1i32..=512 {
        f3_eq(&p, i32::MIN, v2, "e08 dense");
    }
}

/// E09 — `v1 == INT_MIN`, `v2 < 0 (!= INT_MIN)` ⇒ the `-(v1-v2)` arm
#[test]
fn e09_f3_v1_intmin_v2_neg() {
    let p = both();
    let mut rng = Rng::new(0xE09);
    for v2 in [-1i32, -2, -3, -7, -12345, i32::MIN + 1, i32::MIN + 2] {
        f3_eq(&p, i32::MIN, v2, "e09");
    }
    for i in 0..100_000 {
        let v2 = -(((rng.next_u32() >> 1) as i32).max(1));
        f3_eq(&p, i32::MIN, v2, &format!("e09[{i}]"));
    }
    for v2 in -512i32..=-1 {
        f3_eq(&p, i32::MIN, v2, "e09 dense");
    }
}

/// E10 — `v1 == INT_MIN && v2 == INT_MIN` ⇒ `q = 1, r = 0` ⇒ `1`
#[test]
fn e10_f3_both_intmin() {
    let p = both();
    let got = f3_eq(&p, i32::MIN, i32::MIN, "e10");
    assert_eq!(got, 1, "e10: f3(INT_MIN, INT_MIN) must be 1, got {got}");
}

/// E11 — arms leaving `r < 0` ⇒ the `q + (v2 > 0 ? -1 : 1)` floor correction
#[test]
fn e11_f3_negative_remainder() {
    let p = both();
    // Mixed signs with a non-zero remainder always take the correction.
    let mut corrected = 0usize;
    for v1 in -50i32..=50 {
        for v2 in -50i32..=50 {
            if v2 == 0 {
                continue;
            }
            let got = f3_eq(&p, v1, v2, "e11");
            // Model the C *exactly* as written (including the fact that, for
            // two negative operands with a remainder, the C's correction
            // over-shoots the mathematical floor: e.g. f3(-50,-49) == 2, not 1).
            let trunc = (v1 as i64) / (v2 as i64);
            let rem = (v1 as i64) % (v2 as i64);
            let c_model = if v1 >= 0 && v2 > 0 {
                trunc
            } else {
                // `r` as the C computes it in each arm.
                let r = if v1 >= 0 {
                    // v2 < 0: r = v1 % (-v2)  (>= 0)
                    (v1 as i64) % (-(v2 as i64))
                } else if v2 >= 0 {
                    // r = -((-v1) % v2)  (<= 0)
                    -((-(v1 as i64)) % (v2 as i64))
                } else {
                    // r = -((-v1) % (-v2))  (<= 0)
                    -((-(v1 as i64)) % (-(v2 as i64)))
                };
                let q = if v1 >= 0 {
                    -((v1 as i64) / (-(v2 as i64)))
                } else if v2 >= 0 {
                    -((-(v1 as i64)) / (v2 as i64))
                } else {
                    (-(v1 as i64)) / (-(v2 as i64))
                };
                if r >= 0 {
                    q
                } else {
                    q + if v2 > 0 { -1 } else { 1 }
                }
            };
            assert_eq!(
                got as i64, c_model,
                "e11: f3({v1},{v2}) = {got}, hand-model = {c_model} (trunc {trunc}, rem {rem})"
            );
            if v1 % v2 != 0 && ((v1 < 0) != (v2 < 0)) {
                corrected += 1;
            }
        }
    }
    assert!(corrected > 1000, "e11: correction path barely exercised");
    // Large magnitudes with a remainder.
    let mut rng = Rng::new(0xE11);
    for i in 0..100_000 {
        let v1 = rng.next_i32();
        let v2 = rng.next_i32();
        if v2 == 0 {
            continue;
        }
        f3_eq(&p, v1, v2, &format!("e11 rand[{i}]"));
    }
}

// ---------------------------------------------------------------------------
// E12 — `f4` degenerate RNG state
// ---------------------------------------------------------------------------

/// E12 — `{0, 0}` is a fixed point of the generator
#[test]
fn e12_f4_zero_state() {
    let p = both();
    let mut sc = CnRnd { state: [0, 0] };
    let mut sr = CnRnd { state: [0, 0] };
    for step in 0..32 {
        let c = unsafe { (p.c.f4)(&mut sc) };
        let r = unsafe { (p.r.f4)(&mut sr) };
        eq_f64(&format!("e12 step {step}"), c, r);
        assert_eq!(
            c.to_bits(),
            0.0f64.to_bits(),
            "e12: C must return +0.0 for the zero state, got {c:?}"
        );
        assert_eq!(sc.state, [0, 0], "e12: C state must stay {{0,0}}");
        assert_eq!(sr.state, sc.state, "e12: Rust state must match C");
    }
}

// ---------------------------------------------------------------------------
// E13–E14 — `f7` has no validation at all
// ---------------------------------------------------------------------------

fn f7_eq(p: &Pair, bs: u32, ch: u32, bd: u32, ctx: &str) -> u32 {
    let c = unsafe { (p.c.f7)(bs, ch, bd) };
    let r = unsafe { (p.r.f7)(bs, ch, bd) };
    eq_u32(&format!("{ctx} f7({bs},{ch},{bd})"), c, r);
    c
}

/// E13 — numerator overflow wraps modulo 2^32 (no range check)
#[test]
fn e13_f7_overflow() {
    let p = both();
    let mut rng = Rng::new(0xE13);
    let big = [
        0xFFFF_FFFFu32,
        0xFFFF_FFFE,
        0x8000_0000,
        0x7FFF_FFFF,
        0x1_0000 - 1,
        0x1_0000,
        0x10_0000,
        0x100_0000,
    ];
    for &bs in &big {
        for &ch in &big {
            for &bd in &big {
                f7_eq(&p, bs, ch, bd, "e13");
            }
        }
    }
    for i in 0..100_000 {
        f7_eq(
            &p,
            rng.next_u32() | 0x8000_0000,
            rng.next_u32() | 0x8000_0000,
            rng.next_u32() | 0x8000_0000,
            &format!("e13[{i}]"),
        );
    }
}

/// E14 — zero arguments (no validation ⇒ `18 + channels`)
#[test]
fn e14_f7_zero_args() {
    let p = both();
    for &bs in &[0u32, 1, 4096] {
        for &ch in &[0u32, 1, 2, 3] {
            for &bd in &[0u32, 1, 16, 32] {
                let got = f7_eq(&p, bs, ch, bd, "e14");
                if bs == 0 || (bd == 0 && ch != 2) {
                    // numerator is 0 ⇒ (0+7)/8 == 0
                    if bs == 0 {
                        assert_eq!(got, 18u32.wrapping_add(ch), "e14: f7(0,{ch},{bd})");
                    }
                }
            }
        }
    }
    assert_eq!(f7_eq(&p, 0, 0, 0, "e14 all-zero"), 18);
}

// ---------------------------------------------------------------------------
// E15–E16 — `f9` degenerate denominators
// ---------------------------------------------------------------------------

/// E15 — degenerate triangle ⇒ `invDenom = 1.0f/0.0f`
#[test]
fn e15_f9_degenerate() {
    let p = both();
    let mut rng = Rng::new(0xE15);
    for i in 0..50_000 {
        // Colinear vertices ⇒ dot00*dot11 - dot01*dot01 == 0 (exactly, for
        // small integral coordinates).
        let d = LmVec2 {
            x: (rng.below(9) as i32 - 4) as f32,
            y: (rng.below(9) as i32 - 4) as f32,
        };
        let p1 = LmVec2 {
            x: (rng.below(9) as i32 - 4) as f32,
            y: (rng.below(9) as i32 - 4) as f32,
        };
        let t2 = (rng.below(9) as i32 - 4) as f32;
        let t3 = (rng.below(9) as i32 - 4) as f32;
        let p2 = LmVec2 {
            x: p1.x + t2 * d.x,
            y: p1.y + t2 * d.y,
        };
        let p3 = LmVec2 {
            x: p1.x + t3 * d.x,
            y: p1.y + t3 * d.y,
        };
        let pt = LmVec2 {
            x: (rng.below(9) as i32 - 4) as f32,
            y: (rng.below(9) as i32 - 4) as f32,
        };
        let c = unsafe { (p.c.f9)(p1, p2, p3, pt) };
        let r = unsafe { (p.r.f9)(p1, p2, p3, pt) };
        eq_lm(&format!("e15[{i}] colinear {p1:?} {p2:?} {p3:?} {pt:?}"), c, r);
    }
}

/// E16 — all three vertices identical ⇒ `invDenom = +inf`, `u = v = NaN`
#[test]
fn e16_f9_all_points_equal() {
    let p = both();
    let mut rng = Rng::new(0xE16);
    for i in 0..20_000 {
        let v = LmVec2 {
            x: rng.finite(),
            y: rng.finite(),
        };
        let pt = rng.wild_lm();
        let c = unsafe { (p.c.f9)(v, v, v, pt) };
        let r = unsafe { (p.r.f9)(v, v, v, pt) };
        eq_lm(&format!("e16[{i}] all-equal v={v:?} p={pt:?}"), c, r);
    }
    let z = LmVec2 { x: 0.0, y: 0.0 };
    let c = unsafe { (p.c.f9)(z, z, z, z) };
    let r = unsafe { (p.r.f9)(z, z, z, z) };
    eq_lm("e16 origin", c, r);
    assert!(
        c.x.is_nan() && c.y.is_nan(),
        "e16: C must produce NaN for the fully-degenerate case, got {c:?}"
    );
}

// ---------------------------------------------------------------------------
// E17–E18 — `f10` table-index surface
// ---------------------------------------------------------------------------

/// E17 — the inf/NaN exponent rows (`h >> 10` == 31 and 63)
#[test]
fn e17_f10_inf_nan_rows() {
    let p = both();
    for row in [31u32, 63] {
        for low in 0u32..=0x3ff {
            let h = ((row << 10) | low) as u16;
            let c = unsafe { (p.c.f10)(h) };
            let r = unsafe { (p.r.f10)(h) };
            eq_f32(&format!("e17 f10({h:#06x}) row {row}"), c, r);
        }
    }
    // Row 0 and row 32 (zero / subnormal rows, `m__offset == 0`).
    for row in [0u32, 32] {
        for low in 0u32..=0x3ff {
            let h = ((row << 10) | low) as u16;
            let c = unsafe { (p.c.f10)(h) };
            let r = unsafe { (p.r.f10)(h) };
            eq_f32(&format!("e17 f10({h:#06x}) row {row}"), c, r);
        }
    }
}

/// E18 — exhaustive: the index is always in bounds, for all 65 536 inputs
#[test]
fn e18_f10_exhaustive() {
    let p = both();
    for h in 0u32..=0xFFFF {
        let h = h as u16;
        let c = unsafe { (p.c.f10)(h) };
        let r = unsafe { (p.r.f10)(h) };
        eq_f32(&format!("e18 f10({h:#06x})"), c, r);
    }
}

// ---------------------------------------------------------------------------
// E19–E22 — `f11` early returns / unreachable-band behaviour
// ---------------------------------------------------------------------------

/// E19 — `s == 0` ⇒ `dest = {l, l, l}` (returns before touching `h`)
#[test]
fn e19_f11_s_zero() {
    let p = both();
    let mut rng = Rng::new(0xE19);
    for i in 0..50_000 {
        let l = rng.wild_f32();
        for &s in &[0.0f32, -0.0] {
            let src = [rng.wild_f32(), s, l];
            let c = p.c.color(Which::F11, src);
            let r = p.r.color(Which::F11, src);
            eq_arr3(&format!("e19[{i}] f11({src:?})"), c, r);
            assert_eq!(
                [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()],
                [l.to_bits(); 3],
                "e19: C must return {{l,l,l}} for s == 0"
            );
        }
    }
}

/// E20 — the `else` arm (`h >= 360`, `NaN`, `+inf`) ⇒ `dest = {m, m, m}`
#[test]
fn e20_f11_else_band() {
    let p = both();
    let mut rng = Rng::new(0xE20);
    let hs: Vec<f32> = vec![
        360.0,
        360.000_03,
        400.0,
        720.0,
        1.0e6,
        1.0e30,
        f32::MAX,
        f32::INFINITY,
        f32::NAN,
        NAN_A,
        NAN_B,
        NAN_C,
        SNAN,
    ];
    for &h in &hs {
        for i in 0..500 {
            let s = rng.range(0.0001, 2.0);
            let l = rng.range(-2.0, 2.0);
            let src = [h, s, l];
            let c = p.c.color(Which::F11, src);
            let r = p.r.color(Which::F11, src);
            eq_arr3(&format!("e20[{i}] f11({src:?})"), c, r);
            // All three outputs equal `m` ⇒ identical bit patterns.
            assert_eq!(
                [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()],
                [c[0].to_bits(); 3],
                "e20: C else-arm must write m,m,m for h={h:?}"
            );
        }
    }
}

/// E21 — negative `h` hits the *band 3* arm (the `h < 120 && h < 180` typo),
/// **not** the `else` arm.
#[test]
fn e21_f11_negative_h_hits_band3() {
    let p = both();
    let mut rng = Rng::new(0xE21);
    for i in 0..50_000 {
        let h = -rng.range(1.0e-6, 1.0e6);
        let s = rng.range(0.0001, 2.0);
        let l = rng.range(-2.0, 2.0);
        let src = [h, s, l];
        let c = p.c.color(Which::F11, src);
        let r = p.r.color(Which::F11, src);
        eq_arr3(&format!("e21[{i}] f11({src:?})"), c, r);
    }
    // Pin the exact C behaviour with a hand-computed case:
    // h = -60, s = 1, l = 0.5  =>  c = 1, m = 0, x = c*(1-|fmod(-1,2)-1|)
    //   fmodf(-1, 2) = -1 ; |-1 - 1| = 2 ; 1 - 2 = -1 ; x = -1
    //   band-3 assignment: dest = {m, c+m, x+m} = {0, 1, -1}
    let src = [-60.0f32, 1.0, 0.5];
    let c = p.c.color(Which::F11, src);
    let r = p.r.color(Which::F11, src);
    eq_arr3("e21 exact", c, r);
    assert_eq!(
        [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()],
        [0.0f32.to_bits(), 1.0f32.to_bits(), (-1.0f32).to_bits()],
        "e21: unexpected C band-3 output {c:?}"
    );
}

/// E22 — non-finite `h` ⇒ `fmodf(±inf, 2) == NaN` ⇒ `x == NaN`
#[test]
fn e22_f11_nonfinite_h() {
    let p = both();
    let mut rng = Rng::new(0xE22);
    for &h in &[
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        1.0e38,
        -1.0e38,
        NAN_A,
        NAN_B,
        NAN_C,
        SNAN,
    ] {
        for i in 0..500 {
            let s = rng.range(0.0001, 2.0);
            let l = rng.range(-2.0, 2.0);
            let src = [h, s, l];
            let c = p.c.color(Which::F11, src);
            let r = p.r.color(Which::F11, src);
            eq_arr3(&format!("e22[{i}] f11({src:?})"), c, r);
        }
    }
    // -inf takes the band-3 arm (h < 120 && h < 180) and therefore writes
    // x + m into dest[2], where x is NaN.
    let src = [f32::NEG_INFINITY, 1.0, 0.5];
    let c = p.c.color(Which::F11, src);
    let r = p.r.color(Which::F11, src);
    eq_arr3("e22 -inf", c, r);
    assert!(c[2].is_nan(), "e22: expected NaN in dest[2], got {c:?}");
}

// ---------------------------------------------------------------------------
// E23–E25 — `f12` early return, default sector, integer-indefinite conversion
// ---------------------------------------------------------------------------

/// E23 — `s == 0` ⇒ `dest = {v, v, v}`
#[test]
fn e23_f12_s_zero() {
    let p = both();
    let mut rng = Rng::new(0xE23);
    for i in 0..50_000 {
        let v = rng.wild_f32();
        for &s in &[0.0f32, -0.0] {
            let src = [rng.wild_f32(), s, v];
            let c = p.c.color(Which::F12, src);
            let r = p.r.color(Which::F12, src);
            eq_arr3(&format!("e23[{i}] f12({src:?})"), c, r);
            assert_eq!(
                [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()],
                [v.to_bits(); 3],
                "e23: C must return {{v,v,v}} for s == 0"
            );
        }
    }
}

/// E24 — sector `i` outside `0..=4` ⇒ `switch default:` (`r=v, g=p, b=q`)
#[test]
fn e24_f12_default_sector() {
    let p = both();
    let mut rng = Rng::new(0xE24);
    // i == 5, i > 5, and i < 0 (gcc's unsigned `ja` also sends negatives here).
    let ranges: [(f32, f32); 4] = [
        (300.0, 360.0),
        (360.0, 100_000.0),
        (-100_000.0, -1.0e-6),
        (-1.0e30, -1.0e20),
    ];
    for (lo, hi) in ranges {
        for i in 0..10_000 {
            let h = rng.range(lo, hi);
            let s = rng.range(0.0001, 2.0);
            let v = rng.range(-10.0, 10.0);
            let src = [h, s, v];
            let c = p.c.color(Which::F12, src);
            let r = p.r.color(Which::F12, src);
            eq_arr3(&format!("e24[{i}] f12({src:?})"), c, r);
        }
    }
    // Hand-computed: h = 330, s = 1, v = 1 => h/60 = 5.5, i = 5 (default)
    //   f = 0.5, p = 0, q = 1*(1 - 1*0.5) = 0.5, t = 1*(1 - 1*0.5) = 0.5
    //   default: r = v = 1, g = p = 0, b = q = 0.5
    let src = [330.0f32, 1.0, 1.0];
    let c = p.c.color(Which::F12, src);
    let r = p.r.color(Which::F12, src);
    eq_arr3("e24 exact", c, r);
    assert_eq!(
        [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()],
        [1.0f32.to_bits(), 0.0f32.to_bits(), 0.5f32.to_bits()],
        "e24: unexpected C default-sector output {c:?}"
    );
}

/// E25 — `(int)floorf(h/60)` UB ⇒ x86-64 `cvttss2si` integer-indefinite
#[test]
fn e25_f12_int_conversion_indefinite() {
    let p = both();
    let mut rng = Rng::new(0xE25);
    let hs = [
        f32::NAN,
        NAN_A,
        NAN_B,
        NAN_C,
        SNAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        // exactly 2^31 * 60 and one step either side
        1.288_490_2e11,
        -1.288_490_2e11,
        1.288_490_0e11,
        -1.288_490_0e11,
        // 2^31 - 1 and 2^31 after the divide
        2_147_483_520.0 * 60.0,
        2_147_483_648.0 * 60.0,
        -2_147_483_648.0 * 60.0,
        -2_147_483_904.0 * 60.0,
    ];
    for &h in &hs {
        for &s in &[0.25f32, 1.0, 2.0, -1.0, f32::INFINITY, NAN_A, NAN_B] {
            for &v in &[0.0f32, -0.0, 0.5, 1.0, -1.0, f32::INFINITY, NAN_C] {
                let src = [h, s, v];
                let c = p.c.color(Which::F12, src);
                let r = p.r.color(Which::F12, src);
                eq_arr3(&format!("e25 f12({src:?})"), c, r);
            }
        }
    }
    // Sweep the whole large-exponent space so both the in-range and the
    // indefinite side of the `cvttss2si` boundary are hit.
    for i in 0..100_000 {
        let h = f32::from_bits((rng.next_u32() & 0x8fff_ffff) | 0x4e00_0000);
        let src = [h, rng.range(0.0001, 1.0), rng.range(-1.0, 1.0)];
        let c = p.c.color(Which::F12, src);
        let r = p.r.color(Which::F12, src);
        eq_arr3(&format!("e25 sweep[{i}] f12({src:?})"), c, r);
    }
}

// ---------------------------------------------------------------------------
// E26–E29 — `f13` early returns and the hue wrap
// ---------------------------------------------------------------------------

/// E26 — `delta == 0` ⇒ `dest = {0, 0, max}`
#[test]
fn e26_f13_delta_zero() {
    let p = both();
    let mut rng = Rng::new(0xE26);
    for i in 0..50_000 {
        let v = rng.wild_f32();
        let src = [v, v, v];
        let c = p.c.color(Which::F13, src);
        let r = p.r.color(Which::F13, src);
        eq_arr3(&format!("e26[{i}] f13({src:?})"), c, r);
    }
    for &v in &[1.0f32, -1.0, 0.5, 1.0e30, f32::MAX, f32::MIN_POSITIVE] {
        let src = [v, v, v];
        let c = p.c.color(Which::F13, src);
        let r = p.r.color(Which::F13, src);
        eq_arr3("e26 exact", c, r);
        assert_eq!(
            [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()],
            [0.0f32.to_bits(), 0.0f32.to_bits(), v.to_bits()],
            "e26: C must return {{0,0,max}} for delta == 0, got {c:?}"
        );
    }
    // `±inf` on all three channels: max - min = inf - inf = NaN, so this is
    // NOT the delta == 0 path — assert the two libraries still agree.
    for &v in &[f32::INFINITY, f32::NEG_INFINITY] {
        let src = [v, v, v];
        let c = p.c.color(Which::F13, src);
        let r = p.r.color(Which::F13, src);
        eq_arr3("e26 inf", c, r);
    }
}

/// E27 — `max == 0` ⇒ `dest = {0, 0, max}` (incl. `-0.0` and all-negative)
#[test]
fn e27_f13_max_zero() {
    let p = both();
    let mut rng = Rng::new(0xE27);
    // All-negative input: max is the largest (least negative) value, so
    // `max == 0` is false; but a `-0.0`/`0.0` max triggers the guard.
    for i in 0..50_000 {
        let n = -rng.range(0.001, 100.0);
        let m = -rng.range(0.001, 100.0);
        for src in [
            [0.0f32, n, m],
            [-0.0f32, n, m],
            [n, 0.0, m],
            [n, -0.0, m],
            [n, m, 0.0],
            [n, m, -0.0],
        ] {
            let c = p.c.color(Which::F13, src);
            let r = p.r.color(Which::F13, src);
            eq_arr3(&format!("e27[{i}] f13({src:?})"), c, r);
            assert_eq!(
                [c[0].to_bits(), c[1].to_bits()],
                [0.0f32.to_bits(), 0.0f32.to_bits()],
                "e27: C must return h=s=0 when max == 0, got {c:?}"
            );
        }
    }
    let src = [0.0f32, 0.0, 0.0];
    let c = p.c.color(Which::F13, src);
    let r = p.r.color(Which::F13, src);
    eq_arr3("e27 zeros", c, r);
    // All-negative (max != 0, delta != 0) — the *non*-guard path, kept here so
    // the guard boundary is checked from both sides.
    for i in 0..20_000 {
        let src = [
            -rng.range(0.001, 100.0),
            -rng.range(0.001, 100.0),
            -rng.range(0.001, 100.0),
        ];
        let c = p.c.color(Which::F13, src);
        let r = p.r.color(Which::F13, src);
        eq_arr3(&format!("e27 all-neg[{i}] f13({src:?})"), c, r);
    }
}

/// E28 — `NaN` in `src`: every `comiss` is false so `min`/`max` collapse to the
/// last operand, and both `r == max` / `g == max` fail ⇒ the final `else` arm.
#[test]
fn e28_f13_nan_input() {
    let p = both();
    let mut rng = Rng::new(0xE28);
    for lane in 0..3 {
        for &nan in &[NAN_A, NAN_B, NAN_C, SNAN, f32::NAN] {
            for i in 0..2_000 {
                let mut src = [rng.finite(), rng.finite(), rng.finite()];
                src[lane] = nan;
                let c = p.c.color(Which::F13, src);
                let r = p.r.color(Which::F13, src);
                eq_arr3(&format!("e28 lane{lane}[{i}] f13({src:?})"), c, r);
            }
        }
    }
    // 2 and 3 NaN lanes with distinct payloads.
    for i in 0..50_000 {
        let src = [rng.nan_payload(), rng.nan_payload(), rng.nan_payload()];
        let c = p.c.color(Which::F13, src);
        let r = p.r.color(Which::F13, src);
        eq_arr3(&format!("e28 all-nan[{i}] f13({src:?})"), c, r);
    }
    for i in 0..50_000 {
        let mut src = [rng.nan_payload(), rng.nan_payload(), rng.finite()];
        if i % 3 == 1 {
            src.swap(0, 2);
        } else if i % 3 == 2 {
            src.swap(1, 2);
        }
        let c = p.c.color(Which::F13, src);
        let r = p.r.color(Which::F13, src);
        eq_arr3(&format!("e28 two-nan[{i}] f13({src:?})"), c, r);
    }
}

/// E29 — `h < 0` after `h *= 60` ⇒ `h += 360`
#[test]
fn e29_f13_negative_hue_wrap() {
    let p = both();
    let mut rng = Rng::new(0xE29);
    let mut wrapped = 0usize;
    for i in 0..50_000 {
        // r is max and g < b ⇒ (g-b)/delta < 0 ⇒ h < 0 ⇒ wrap.
        let r0 = rng.range(0.001, 10.0);
        let b = rng.range(-10.0, r0);
        let g = rng.range(-10.0, b);
        let src = [r0, g, b];
        let c = p.c.color(Which::F13, src);
        let rr = p.r.color(Which::F13, src);
        eq_arr3(&format!("e29[{i}] f13({src:?})"), c, rr);
        if c[0] > 180.0 {
            wrapped += 1;
        }
    }
    assert!(wrapped > 100, "e29: wrap path barely exercised ({wrapped})");
    // Hand-computed: r=1, g=0, b=0.5 => delta=1, h=(0-0.5)/1=-0.5, *60=-30,
    // +360 = 330.
    let src = [1.0f32, 0.0, 0.5];
    let c = p.c.color(Which::F13, src);
    let r = p.r.color(Which::F13, src);
    eq_arr3("e29 exact", c, r);
    assert_eq!(
        c[0].to_bits(),
        330.0f32.to_bits(),
        "e29: expected h == 330, got {c:?}"
    );
}

// ---------------------------------------------------------------------------
// E30 — `agglom`'s twelve `!isnan(...)` guards
// ---------------------------------------------------------------------------

/// E30 — `NaN` component results are skipped, not accumulated
#[test]
fn e30_agglom_nan_terms_skipped() {
    let p = both();
    let mut rng = Rng::new(0xE30);
    // Force NaN out of f9 (degenerate triangle), f11/f12 (NaN hue) and f13.
    for i in 0..50_000 {
        let mut a = rng.sane_agglom();
        // degenerate triangle ⇒ f9 returns NaN
        a.f9_1 = 1.0;
        a.f9_2 = 1.0;
        a.f9_4 = 1.0;
        a.f9_5 = 1.0;
        a.f9_7 = 1.0;
        a.f9_8 = 1.0;
        // NaN hue into f11 / f12
        a.f11_2 = f32::NAN;
        a.f12_2 = NAN_A;
        // NaN into f13
        a.f13_2 = NAN_B;
        let c = p.c.call_agglom(&a);
        let r = p.r.call_agglom(&a);
        eq_f64(&format!("e30[{i}] {a:?}"), c, r);
        assert!(
            !c.is_nan(),
            "e30: C must never return NaN thanks to the isnan guards, got {c:?} for {a:?}"
        );
    }
    // Half-float NaN rows into f10 (rows 31 and 63 with a non-zero mantissa).
    for h in [0x7c01u16, 0x7fff, 0xfc01, 0xffff, 0x7c00, 0xfc00] {
        for i in 0..200 {
            let mut a = rng.sane_agglom();
            a.f10_1 = h;
            let c = p.c.call_agglom(&a);
            let r = p.r.call_agglom(&a);
            eq_f64(&format!("e30 f10={h:#06x}[{i}]"), c, r);
        }
    }
    // Every parameter a NaN at once.
    for i in 0..20_000 {
        let mut a = rng.wild_agglom();
        for f in [
            &mut a.f2_1, &mut a.f2_2, &mut a.f2_3, &mut a.f2_7, &mut a.f2_8, &mut a.f2_9,
            &mut a.f2_10, &mut a.f9_1, &mut a.f9_2, &mut a.f9_4, &mut a.f9_5, &mut a.f9_7,
            &mut a.f9_8, &mut a.f9_10, &mut a.f9_11, &mut a.f11_2, &mut a.f11_3, &mut a.f11_4,
            &mut a.f12_2, &mut a.f12_3, &mut a.f12_4, &mut a.f13_2, &mut a.f13_3, &mut a.f13_4,
        ] {
            *f = rng.nan_payload();
        }
        let c = p.c.call_agglom(&a);
        let r = p.r.call_agglom(&a);
        eq_f64(&format!("e30 all-nan[{i}] {a:?}"), c, r);
    }
}

// ---------------------------------------------------------------------------
// E33–E34 — generic boundary behaviour
// ---------------------------------------------------------------------------

/// E33 — `f5` silently discards the high 16 bits
#[test]
fn e33_f5_high_bits_dropped() {
    let p = both();
    let mut rng = Rng::new(0xE33);
    for i in 0..200_000 {
        let low = rng.next_u32() & 0xFFFF;
        let high = rng.next_u32() & 0xFFFF_0000;
        let a = low | high;
        let c_full = unsafe { (p.c.f5)(a) };
        let r_full = unsafe { (p.r.f5)(a) };
        let c_low = unsafe { (p.c.f5)(low) };
        eq_u32(&format!("e33[{i}] f5({a:#x})"), c_full, r_full);
        assert_eq!(
            c_full, c_low,
            "e33: C f5({a:#x}) = {c_full:#x} but f5({low:#x}) = {c_low:#x}"
        );
    }
}

/// E34 — one step past documented ranges for every numeric entry point
#[test]
fn e34_boundary_values() {
    let p = both();

    for h in [0u16, 1, 0x03ff, 0x0400, 0x7bff, 0x7c00, 0x7fff, 0x8000, 0xfbff, 0xfc00, 0xffff] {
        let c = unsafe { (p.c.f10)(h) };
        let r = unsafe { (p.r.f10)(h) };
        eq_f32(&format!("e34 f10({h:#06x})"), c, r);
    }

    for (v1, v2) in [
        (i32::MIN, 1),
        (i32::MIN, -1),
        (i32::MIN, 0),
        (i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MAX, 1),
        (i32::MAX, -1),
        (i32::MAX, 0),
        (i32::MAX, i32::MIN),
        (i32::MAX, i32::MAX),
        (0, i32::MIN),
        (0, i32::MAX),
        (0, 0),
        (-1, i32::MIN),
        (1, i32::MIN),
    ] {
        let c = unsafe { (p.c.f3)(v1, v2) };
        let r = unsafe { (p.r.f3)(v1, v2) };
        eq_i32(&format!("e34 f3({v1},{v2})"), c, r);
    }

    for (bs, ch, bd) in [
        (u32::MAX, u32::MAX, u32::MAX),
        (0, 0, 0),
        (u32::MAX, 2, 32),
        (u32::MAX, 2, 31),
        (u32::MAX, 1, 32),
        (1, u32::MAX, u32::MAX),
        (u32::MAX, u32::MAX, 1),
    ] {
        let c = unsafe { (p.c.f7)(bs, ch, bd) };
        let r = unsafe { (p.r.f7)(bs, ch, bd) };
        eq_u32(&format!("e34 f7({bs},{ch},{bd})"), c, r);
    }

    for a in [0u32, 1, 0xFFFF, 0x1_0000, 0xFFFF_FFFF, 0x8000_0000] {
        let c = unsafe { (p.c.f5)(a) };
        let r = unsafe { (p.r.f5)(a) };
        eq_u32(&format!("e34 f5({a:#x})"), c, r);
    }

    for st in [[0u64, 0], [u64::MAX, u64::MAX], [1, 0], [0, 1]] {
        let mut sc = CnRnd { state: st };
        let mut sr = CnRnd { state: st };
        let c = unsafe { (p.c.f4)(&mut sc) };
        let r = unsafe { (p.r.f4)(&mut sr) };
        eq_f64(&format!("e34 f4({st:?})"), c, r);
        assert_eq!(sc.state, sr.state);
    }
}

// ---------------------------------------------------------------------------
// E32 — NULL pointers (must fault identically); run in child processes
// ---------------------------------------------------------------------------

const NULL_CASES: &[&str] = &[
    "f4",
    "f11_src",
    "f11_dest",
    "f11_both",
    "f12_src",
    "f12_dest",
    "f12_both",
    "f13_src",
    "f13_dest",
    "f13_both",
    "f2_circle_circle",
    "f2_circle_aabb",
    "f2_aabb_circle",
    "f2_aabb_aabb",
];

/// E32 — both libraries must terminate with the same signal on `NULL` input.
#[test]
fn e32_null_pointer_parity() {
    // The child re-invokes this same test binary with `--ignored --exact
    // e32_null_child` and the case selected via the environment.
    let exe = std::env::current_exe().unwrap();
    for case in NULL_CASES {
        let mut status = Vec::new();
        for target in ["c", "rust"] {
            let out = std::process::Command::new(&exe)
                .args(["--exact", "e32_null_child", "--ignored", "--nocapture"])
                .env("E32_TARGET", target)
                .env("E32_CASE", case)
                .output()
                .expect("spawning the child test process failed");
            // Record (exit code, signal) — on Unix a fatal signal shows up in
            // `signal()`, a clean run in `code()`.
            #[cfg(unix)]
            let sig = {
                use std::os::unix::process::ExitStatusExt;
                out.status.signal()
            };
            #[cfg(not(unix))]
            let sig: Option<i32> = None;
            status.push((out.status.code(), sig));
        }
        assert_eq!(
            status[0], status[1],
            "e32 case `{case}`: C child exited {:?}, Rust child exited {:?}",
            status[0], status[1]
        );
        println!("e32 case `{case}`: both children -> {:?}", status[0]);
    }
}

/// Helper executed only in the child process spawned by `e32_null_pointer_parity`.
#[test]
#[ignore = "child process helper for e32_null_pointer_parity"]
fn e32_null_child() {
    let target = std::env::var("E32_TARGET").expect("E32_TARGET");
    let case = std::env::var("E32_CASE").expect("E32_CASE");
    let lib = match target.as_str() {
        "c" => Lib::open("C", &c_so_path()),
        "rust" => Lib::open("Rust", &rust_so_path()),
        other => panic!("bad E32_TARGET {other}"),
    };
    let mut dest = [0.0f32; 3];
    let src = [1.0f32, 0.5, 0.25];
    let null_f = std::ptr::null_mut::<f32>();
    let null_cf = std::ptr::null::<f32>();
    let nullv = std::ptr::null::<c_void>();
    let circ = C2Circle::default();
    let aabb = C2Aabb::default();
    unsafe {
        match case.as_str() {
            "f4" => {
                let v = (lib.f4)(std::ptr::null_mut());
                println!("f4 returned {v}");
            }
            "f11_src" => (lib.f11)(dest.as_mut_ptr(), null_cf),
            "f11_dest" => (lib.f11)(null_f, src.as_ptr()),
            "f11_both" => (lib.f11)(null_f, null_cf),
            "f12_src" => (lib.f12)(dest.as_mut_ptr(), null_cf),
            "f12_dest" => (lib.f12)(null_f, src.as_ptr()),
            "f12_both" => (lib.f12)(null_f, null_cf),
            "f13_src" => (lib.f13)(dest.as_mut_ptr(), null_cf),
            "f13_dest" => (lib.f13)(null_f, src.as_ptr()),
            "f13_both" => (lib.f13)(null_f, null_cf),
            "f2_circle_circle" => {
                let v = (lib.f2)(nullv, C2_TYPE_CIRCLE, nullv, C2_TYPE_CIRCLE);
                println!("f2 returned {v}");
            }
            "f2_circle_aabb" => {
                let b = &aabb as *const C2Aabb as *const c_void;
                let v = (lib.f2)(nullv, C2_TYPE_CIRCLE, b, C2_TYPE_AABB);
                println!("f2 returned {v}");
            }
            "f2_aabb_circle" => {
                let a = &aabb as *const C2Aabb as *const c_void;
                let _ = &circ;
                let v = (lib.f2)(a, C2_TYPE_AABB, nullv, C2_TYPE_CIRCLE);
                println!("f2 returned {v}");
            }
            "f2_aabb_aabb" => {
                let v = (lib.f2)(nullv, C2_TYPE_AABB, nullv, C2_TYPE_AABB);
                println!("f2 returned {v}");
            }
            other => panic!("bad E32_CASE {other}"),
        }
    }
    println!("child `{case}` on `{target}` survived: dest = {dest:?}");
}
