//! Phase C — ERRORS.md rows 32-67: the primitive-level rejection surface
//! (`c2Support`, the simplex `switch` default arms, division by zero, and the
//! public `gjk` wrapper's edge cases).

#![allow(non_snake_case)]

#[macro_use]
mod common;

use common::*;
use std::ffi::c_char;

/// Every out-of-range `count` a caller can hand a simplex/support function.
const BAD_COUNTS: &[i32] = &[
    0,
    4,
    5,
    -1,
    -2,
    7,
    100,
    i32::MAX,
    i32::MIN,
    i32::MIN + 1,
];

// ===========================================================================
// Rows 32-36 — c2Support
// ===========================================================================

#[test]
fn err_support_count_zero() {
    let l = libs();
    let (c, r) = l.get::<FnSupport>("c2Support");
    let mut g = Rng::new(0x32);
    for i in 0..20_000 {
        let verts = [g.v_mixed(), g.v_mixed(), g.v_mixed(), g.v_mixed()];
        let d = g.v_mixed();
        // C reads verts[0] unconditionally (L294) then never enters the loop.
        let cv = unsafe { c(verts.as_ptr(), 0, d) };
        let rv = unsafe { r(verts.as_ptr(), 0, d) };
        ck_i32!("row32 c2Support count==0", cv, rv, "i={i} d={d:?}");
        assert_eq!(cv, 0, "count==0 must still return index 0");
    }
}

#[test]
fn err_support_count_negative() {
    let l = libs();
    let (c, r) = l.get::<FnSupport>("c2Support");
    let mut g = Rng::new(0x33);
    for &count in &[-1i32, -2, -100, i32::MIN, i32::MIN + 1] {
        for i in 0..2_000 {
            let verts = [g.v_mixed(), g.v_mixed(), g.v_mixed(), g.v_mixed()];
            let d = g.v_mixed();
            let cv = unsafe { c(verts.as_ptr(), count, d) };
            let rv = unsafe { r(verts.as_ptr(), count, d) };
            ck_i32!("row33 c2Support negative count", cv, rv, "count={count} i={i}");
            assert_eq!(cv, 0, "negative count must return 0");
        }
    }
}

#[test]
fn err_support_ties() {
    let l = libs();
    let (c, r) = l.get::<FnSupport>("c2Support");
    // Strict `>` (L298) means the LOWEST index wins a tie.
    let mut g = Rng::new(0x34);
    for i in 0..20_000 {
        let v = g.v_grid();
        let verts = [v; 8];
        for &count in &[1i32, 2, 3, 4, 8] {
            let d = g.v_grid();
            let cv = unsafe { c(verts.as_ptr(), count, d) };
            let rv = unsafe { r(verts.as_ptr(), count, d) };
            ck_i32!("row34 c2Support exact tie", cv, rv, "i={i} count={count} d={d:?}");
            assert_eq!(cv, 0, "an all-equal tie must resolve to index 0");
        }
        // partial ties: verts[2] == verts[0]
        let mut verts2 = [g.v_grid(), g.v_grid(), V::default(), g.v_grid()];
        verts2[2] = verts2[0];
        for &count in &[3i32, 4] {
            let d = g.v_grid();
            let cv = unsafe { c(verts2.as_ptr(), count, d) };
            let rv = unsafe { r(verts2.as_ptr(), count, d) };
            ck_i32!("row34 c2Support partial tie", cv, rv, "i={i} count={count} d={d:?}");
        }
    }
}

#[test]
fn err_support_zero_dir() {
    let l = libs();
    let (c, r) = l.get::<FnSupport>("c2Support");
    let mut g = Rng::new(0x35);
    for i in 0..20_000 {
        let verts = [g.v_mixed(), g.v_mixed(), g.v_mixed(), g.v_mixed()];
        for d in [V::new(0.0, 0.0), V::new(-0.0, -0.0), V::new(0.0, -0.0)] {
            for &count in &[1i32, 2, 4] {
                let cv = unsafe { c(verts.as_ptr(), count, d) };
                let rv = unsafe { r(verts.as_ptr(), count, d) };
                ck_i32!("row35 c2Support zero direction", cv, rv, "i={i} count={count} d={d:?}");
            }
        }
    }
}

#[test]
fn err_support_nan() {
    let l = libs();
    let (c, r) = l.get::<FnSupport>("c2Support");
    let nanp: &[f32] = &[
        f32::NAN,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0xffc0_0abc),
        f32::from_bits(0x7f80_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &nv in nanp {
        // NaN in the direction
        for slot in 0..2usize {
            let mut dd = [1.0f32, 1.0];
            dd[slot] = nv;
            let d = V::new(dd[0], dd[1]);
            let verts = [
                V::new(1.0, 2.0),
                V::new(-3.0, 4.0),
                V::new(5.0, -6.0),
                V::new(0.0, 0.0),
            ];
            for &count in &[1i32, 2, 4] {
                let cv = unsafe { c(verts.as_ptr(), count, d) };
                let rv = unsafe { r(verts.as_ptr(), count, d) };
                ck_i32!("row36 c2Support NaN dir", cv, rv, "nan={:#010x} count={count}", nv.to_bits());
            }
        }
        // NaN in the verts, at each index
        for slot in 0..4usize {
            let mut verts = [
                V::new(1.0, 2.0),
                V::new(-3.0, 4.0),
                V::new(5.0, -6.0),
                V::new(7.0, 8.0),
            ];
            verts[slot] = V::new(nv, nv);
            for &count in &[1i32, 2, 3, 4] {
                let d = V::new(1.0, 1.0);
                let cv = unsafe { c(verts.as_ptr(), count, d) };
                let rv = unsafe { r(verts.as_ptr(), count, d) };
                ck_i32!("row36 c2Support NaN vert", cv, rv,
                        "nan={:#010x} slot={slot} count={count}", nv.to_bits());
            }
        }
    }
}

// ===========================================================================
// Rows 37-44 — the simplex `switch` default arms
// ===========================================================================

fn rand_simplex(g: &mut Rng, count: i32) -> Simplex {
    let mut s = Simplex::default();
    s.count = count;
    s.div = g.mixed();
    for k in 0..4 {
        s.verts[k].sA = g.v_mixed();
        s.verts[k].sB = g.v_mixed();
        s.verts[k].p = g.v_mixed();
        s.verts[k].u = g.mixed();
        s.verts[k].iA = g.below(8) as i32;
        s.verts[k].iB = g.below(8) as i32;
    }
    s
}

#[test]
fn err_metric_bad_count() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexF>("c2GJKSimplexMetric");
    let mut g = Rng::new(0x37);
    for &count in BAD_COUNTS {
        for i in 0..2_000 {
            let mut cs = rand_simplex(&mut g, count);
            let mut rs = cs;
            let cv = unsafe { c(&mut cs) };
            let rv = unsafe { r(&mut rs) };
            ck_f32!("row37 metric bad count", cv, rv, "count={count} i={i}");
            // `default:` falls through to `case 1:` -> exactly 0.0f
            if count != 2 && count != 3 {
                assert_eq!(cv.to_bits(), 0.0f32.to_bits(), "count={count} must give +0.0");
            }
            ck_bytes!("row37 simplex untouched", cs, rs, "count={count} i={i}");
        }
    }
}

#[test]
fn err_witness_bad_count() {
    let l = libs();
    let (c, r) = l.get::<FnWitness>("c2Witness");
    let mut g = Rng::new(0x38);
    for &count in BAD_COUNTS {
        for i in 0..2_000 {
            let mut cs = rand_simplex(&mut g, count);
            let mut rs = cs;
            let mut ca = V::new(POISON_F32, POISON_F32);
            let mut cb = ca;
            let mut ra = ca;
            let mut rb = ca;
            unsafe {
                c(&mut cs, &mut ca, &mut cb);
                r(&mut rs, &mut ra, &mut rb);
            }
            ck_v!("row38 witness bad count a", ca, ra, "count={count} i={i}");
            ck_v!("row38 witness bad count b", cb, rb, "count={count} i={i}");
            if !(1..=3).contains(&count) {
                assert_eq!((ca.x.to_bits(), ca.y.to_bits()), (0, 0), "count={count}: a must be (+0,+0)");
                assert_eq!((cb.x.to_bits(), cb.y.to_bits()), (0, 0), "count={count}: b must be (+0,+0)");
            }
        }
    }
}

#[test]
fn err_witness_zero_div() {
    let l = libs();
    let (c, r) = l.get::<FnWitness>("c2Witness");
    let mut g = Rng::new(0x39);
    for &div in &[0.0f32, -0.0] {
        for count in 1..=3i32 {
            for i in 0..5_000 {
                let mut cs = rand_simplex(&mut g, count);
                cs.div = div;
                let mut rs = cs;
                let mut ca = V::default();
                let mut cb = V::default();
                let mut ra = V::default();
                let mut rb = V::default();
                unsafe {
                    c(&mut cs, &mut ca, &mut cb);
                    r(&mut rs, &mut ra, &mut rb);
                }
                ck_v!("row39 witness div==0 a", ca, ra, "div={div:?} count={count} i={i}");
                ck_v!("row39 witness div==0 b", cb, rb, "div={div:?} count={count} i={i}");
            }
        }
    }
}

#[test]
fn err_witness_nan_div() {
    let l = libs();
    let (c, r) = l.get::<FnWitness>("c2Witness");
    let mut g = Rng::new(0x40);
    let divs: &[f32] = &[
        f32::NAN,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e-45,
        -1e-45,
        f32::MIN_POSITIVE,
        f32::MAX,
    ];
    for &div in divs {
        for count in 1..=3i32 {
            for i in 0..2_000 {
                let mut cs = rand_simplex(&mut g, count);
                cs.div = div;
                let mut rs = cs;
                let mut ca = V::default();
                let mut cb = V::default();
                let mut ra = V::default();
                let mut rb = V::default();
                unsafe {
                    c(&mut cs, &mut ca, &mut cb);
                    r(&mut rs, &mut ra, &mut rb);
                }
                ck_v!("row40 witness NaN/Inf div a", ca, ra, "div={div:?} count={count} i={i}");
                ck_v!("row40 witness NaN/Inf div b", cb, rb, "div={div:?} count={count} i={i}");
            }
        }
    }
}

#[test]
fn err_l_bad_count() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexV>("c2L");
    let mut g = Rng::new(0x41);
    // c2L has cases 1 and 2 only; 3 and everything else hit `default:`
    for &count in [BAD_COUNTS, &[3i32]].concat().iter() {
        for i in 0..2_000 {
            let mut cs = rand_simplex(&mut g, count);
            let mut rs = cs;
            let cv = unsafe { c(&mut cs) };
            let rv = unsafe { r(&mut rs) };
            ck_v!("row41 c2L bad count", cv, rv, "count={count} i={i}");
            if count != 1 && count != 2 {
                assert_eq!((cv.x.to_bits(), cv.y.to_bits()), (0, 0), "count={count} must be (+0,+0)");
            }
            ck_bytes!("row41 simplex untouched", cs, rs, "count={count} i={i}");
        }
    }
}

#[test]
fn err_l_zero_div() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexV>("c2L");
    let mut g = Rng::new(0x42);
    for &div in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for count in 1..=2i32 {
            for i in 0..5_000 {
                let mut cs = rand_simplex(&mut g, count);
                cs.div = div;
                let mut rs = cs;
                let cv = unsafe { c(&mut cs) };
                let rv = unsafe { r(&mut rs) };
                ck_v!("row42 c2L div==0", cv, rv, "div={div:?} count={count} i={i}");
            }
        }
    }
}

#[test]
fn err_d_bad_count() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexV>("c2D");
    let mut g = Rng::new(0x43);
    for &count in [BAD_COUNTS, &[3i32]].concat().iter() {
        for i in 0..2_000 {
            let mut cs = rand_simplex(&mut g, count);
            let mut rs = cs;
            let cv = unsafe { c(&mut cs) };
            let rv = unsafe { r(&mut rs) };
            ck_v!("row43 c2D bad count", cv, rv, "count={count} i={i}");
            if count != 1 && count != 2 {
                assert_eq!((cv.x.to_bits(), cv.y.to_bits()), (0, 0), "count={count} must be (+0,+0)");
            }
            ck_bytes!("row43 simplex untouched", cs, rs, "count={count} i={i}");
        }
    }
}

#[test]
fn err_d_det_zero() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexV>("c2D");
    let mut g = Rng::new(0x44);
    // count == 2 with the origin exactly collinear with a and b makes
    // c2Det2(ab, -a) == 0, which is NOT > 0, so the c2CCW90 branch is taken.
    for i in 0..20_000 {
        let mut cs = rand_simplex(&mut g, 2);
        let a = V::new(g.grid(), g.grid());
        let k = g.grid();
        cs.verts[0].p = a;
        cs.verts[1].p = V::new(a.x * k, a.y * k); // collinear through the origin
        let mut rs = cs;
        let cv = unsafe { c(&mut cs) };
        let rv = unsafe { r(&mut rs) };
        ck_v!("row44 c2D det==0", cv, rv, "i={i} a={a:?} k={k:?}");

        // also the exact a == b case (ab == 0)
        cs.verts[1].p = cs.verts[0].p;
        let mut rs2 = cs;
        let cv2 = unsafe { c(&mut cs) };
        let rv2 = unsafe { r(&mut rs2) };
        ck_v!("row44 c2D ab==0", cv2, rv2, "i={i}");
    }
}

// ===========================================================================
// Rows 45-49 — division by zero / sqrt edge cases
// ===========================================================================

#[test]
fn err_norm_zero() {
    let l = libs();
    let (c, r) = l.get::<FnVV>("c2Norm");
    // c2Len == 0 -> 1/0 == +Inf -> 0 * Inf == NaN
    for a in [
        V::new(0.0, 0.0),
        V::new(-0.0, -0.0),
        V::new(0.0, -0.0),
        V::new(-0.0, 0.0),
    ] {
        let cv = unsafe { c(a) };
        let rv = unsafe { r(a) };
        ck_v!("row45 c2Norm zero vector", cv, rv, "a={a:?}");
        assert!(cv.x.is_nan() && cv.y.is_nan(), "expected NaN components, got {cv:?}");
    }
}

#[test]
fn err_norm_nan_inf() {
    let l = libs();
    let (c, r) = l.get::<FnVV>("c2Norm");
    let (cl, rl) = l.get::<FnVf>("c2Len");
    let vals: &[f32] = &[
        f32::NAN,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0xffc0_9999),
        f32::from_bits(0x7f80_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        1e-45,
        -1e-45,
    ];
    for &x in vals {
        for &y in vals {
            let a = V::new(x, y);
            ck_v!("row46 c2Norm NaN/Inf", unsafe { c(a) }, unsafe { r(a) }, "a={a:?}");
            ck_f32!("row46 c2Len NaN/Inf", unsafe { cl(a) }, unsafe { rl(a) }, "a={a:?}");
        }
        // mixed with a finite partner
        for &y in &[0.0f32, 1.0, -1.0] {
            let a = V::new(x, y);
            ck_v!("row46 c2Norm mixed", unsafe { c(a) }, unsafe { r(a) }, "a={a:?}");
            let b = V::new(y, x);
            ck_v!("row46 c2Norm mixed2", unsafe { c(b) }, unsafe { r(b) }, "b={b:?}");
        }
    }
}

#[test]
fn err_div_zero() {
    let l = libs();
    let (c, r) = l.get::<FnVsV>("c2Div");
    let mut g = Rng::new(0x47);
    for &b in &[0.0f32, -0.0] {
        for i in 0..20_000 {
            let a = g.v_mixed();
            ck_v!("row47 c2Div by zero", unsafe { c(a, b) }, unsafe { r(a, b) }, "i={i} a={a:?} b={b:?}");
        }
        for a in [
            V::new(0.0, 0.0),
            V::new(-0.0, 0.0),
            V::new(1.0, -1.0),
            V::new(f32::INFINITY, f32::NEG_INFINITY),
            V::new(f32::NAN, 1.0),
        ] {
            ck_v!("row47 c2Div by zero (edge)", unsafe { c(a, b) }, unsafe { r(a, b) }, "a={a:?} b={b:?}");
        }
    }
}

#[test]
fn err_div_nan() {
    let l = libs();
    let (c, r) = l.get::<FnVsV>("c2Div");
    let bs: &[f32] = &[
        f32::NAN,
        f32::from_bits(0x7fc0_0555),
        f32::from_bits(0xffc0_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e-45,
        f32::MIN_POSITIVE,
        f32::MAX,
    ];
    for &b in bs {
        for a in [
            V::new(0.0, 0.0),
            V::new(-0.0, -0.0),
            V::new(1.0, -1.0),
            V::new(f32::NAN, 2.0),
            V::new(f32::INFINITY, 0.0),
            V::new(1e30, -1e30),
        ] {
            ck_v!("row48 c2Div NaN divisor", unsafe { c(a, b) }, unsafe { r(a, b) }, "a={a:?} b={b:?}");
        }
    }
}

#[test]
fn err_len_edge() {
    let l = libs();
    let (c, r) = l.get::<FnVf>("c2Len");
    // sqrtf of a negative-zero dot product must give -0.0, not +0.0.
    for a in [
        V::new(0.0, 0.0),
        V::new(-0.0, -0.0),
        V::new(-0.0, 0.0),
        V::new(0.0, -0.0),
        V::new(f32::MAX, f32::MAX),   // dot overflows to +Inf
        V::new(f32::MIN, f32::MIN),
        V::new(1e-45, 1e-45),         // dot underflows to 0
        V::new(f32::NAN, 0.0),
        V::new(f32::INFINITY, f32::NEG_INFINITY),
    ] {
        let cv = unsafe { c(a) };
        let rv = unsafe { r(a) };
        ck_f32!("row49 c2Len edge", cv, rv, "a={a:?}");
    }
    // exhaustive over the shared edge list
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let a = V::new(x, y);
            ck_f32!("row49 c2Len edge grid", unsafe { c(a) }, unsafe { r(a) }, "a={a:?}");
        }
    }
}

// ===========================================================================
// Rows 50-59 — c22 / c23 arm selection under degenerate input
// ===========================================================================

#[test]
fn err_c22_v_le_zero() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c22");
    let mut g = Rng::new(0x50);
    for i in 0..10_000 {
        // v = dot(a, a-b) <= 0  : a is "behind" b along the segment
        let t = g.range(0.1, 10.0);
        let mut cs = rand_simplex(&mut g, 2);
        cs.verts[0].p = V::new(t, 0.0);
        cs.verts[1].p = V::new(t * g.range(1.1, 5.0), 0.0);
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row50 c22 v<=0", cs, rs, "i={i} t={t:?}");
        assert_eq!(cs.count, 1, "expected the vertex-A arm");
        assert_eq!(cs.div.to_bits(), 1.0f32.to_bits());
    }
}

#[test]
fn err_c22_u_le_zero() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c22");
    let mut g = Rng::new(0x51);
    for i in 0..10_000 {
        let t = g.range(0.1, 10.0);
        let mut cs = rand_simplex(&mut g, 2);
        cs.verts[0].p = V::new(t * g.range(1.1, 5.0), 0.0);
        cs.verts[1].p = V::new(t, 0.0);
        let expect_b = cs.verts[1];
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row51 c22 u<=0", cs, rs, "i={i} t={t:?}");
        assert_eq!(cs.count, 1, "expected the vertex-B arm");
        // s->a = s->b happened BEFORE s->a.u = 1.0f
        assert_eq!(cs.verts[0].p.bits(), expect_b.p.bits(), "a must have been replaced by b");
        assert_eq!(cs.verts[0].sA.bits(), expect_b.sA.bits());
        assert_eq!(cs.verts[0].sB.bits(), expect_b.sB.bits());
        assert_eq!(cs.verts[0].iA, expect_b.iA);
        assert_eq!(cs.verts[0].iB, expect_b.iB);
        assert_eq!(cs.verts[0].u.to_bits(), 1.0f32.to_bits());
    }
}

#[test]
fn err_c22_nan() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c22");
    let mut g = Rng::new(0x52);
    let nanp: &[f32] = &[
        f32::NAN,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0xffc0_1234),
        f32::from_bits(0x7f80_0001),
    ];
    for &nv in nanp {
        for slot in 0..4usize {
            for i in 0..500 {
                let mut cs = rand_simplex(&mut g, 2);
                let mut pv = [1.0f32, 2.0, 3.0, 4.0];
                pv[slot] = nv;
                cs.verts[0].p = V::new(pv[0], pv[1]);
                cs.verts[1].p = V::new(pv[2], pv[3]);
                let mut rs = cs;
                unsafe {
                    c(&mut cs);
                    r(&mut rs);
                }
                ck_bytes!("row52 c22 NaN", cs, rs, "nan={:#010x} slot={slot} i={i}", nv.to_bits());
            }
        }
        // both vertices fully NaN -> all `<=` false -> final else arm
        let mut cs = rand_simplex(&mut g, 2);
        cs.verts[0].p = V::new(nv, nv);
        cs.verts[1].p = V::new(nv, nv);
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row52 c22 all-NaN", cs, rs, "nan={:#010x}", nv.to_bits());
        assert_eq!(cs.count, 2, "all-NaN must fall to the two-vertex else arm");
        assert!(cs.div.is_nan(), "div must be NaN");
    }
}

#[test]
fn err_c22_duplicate() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c22");
    let mut g = Rng::new(0x53);
    for i in 0..20_000 {
        let mut cs = rand_simplex(&mut g, 2);
        let p = g.v_mixed();
        cs.verts[0].p = p;
        cs.verts[1].p = p; // u == v == 0 -> `v <= 0` first arm wins
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row53 c22 duplicate vertex", cs, rs, "i={i} p={p:?}");
    }
    // and the exact zero case
    for i in 0..100 {
        let mut cs = rand_simplex(&mut g, 2);
        cs.verts[0].p = V::new(0.0, 0.0);
        cs.verts[1].p = V::new(0.0, 0.0);
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row53 c22 both at origin", cs, rs, "i={i}");
        assert_eq!(cs.count, 1);
    }
}

#[test]
fn err_c23_region_a() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c23");
    let mut g = Rng::new(0x54);
    for i in 0..10_000 {
        let mut cs = rand_simplex(&mut g, 3);
        let s = g.range(1.0, 10.0);
        cs.verts[0].p = V::new(s, 0.0);
        cs.verts[1].p = V::new(s * 3.0, s);
        cs.verts[2].p = V::new(s * 3.0, -s);
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row54 c23 vertex-A region", cs, rs, "i={i} s={s:?}");
        assert_eq!(cs.count, 1, "expected the vertex-A arm");
    }
}

#[test]
fn err_c23_region_b() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c23");
    let mut g = Rng::new(0x55);
    for i in 0..10_000 {
        let mut cs = rand_simplex(&mut g, 3);
        let s = g.range(1.0, 10.0);
        cs.verts[0].p = V::new(s * 3.0, s);
        cs.verts[1].p = V::new(s, 0.0);
        cs.verts[2].p = V::new(s * 3.0, -s);
        let expect_b = cs.verts[1];
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row55 c23 vertex-B region", cs, rs, "i={i} s={s:?}");
        assert_eq!(cs.count, 1);
        assert_eq!(cs.verts[0].p.bits(), expect_b.p.bits(), "a must become b");
    }
}

#[test]
fn err_c23_region_c() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c23");
    let mut g = Rng::new(0x56);
    for i in 0..10_000 {
        let mut cs = rand_simplex(&mut g, 3);
        let s = g.range(1.0, 10.0);
        cs.verts[0].p = V::new(s * 3.0, s);
        cs.verts[1].p = V::new(s * 3.0, -s);
        cs.verts[2].p = V::new(s, 0.0);
        let expect_c = cs.verts[2];
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row56 c23 vertex-C region", cs, rs, "i={i} s={s:?}");
        assert_eq!(cs.count, 1);
        assert_eq!(cs.verts[0].p.bits(), expect_c.p.bits(), "a must become c");
    }
}

#[test]
fn err_c23_all_same() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c23");
    let mut g = Rng::new(0x57);
    for i in 0..20_000 {
        let mut cs = rand_simplex(&mut g, 3);
        let p = g.v_mixed();
        cs.verts[0].p = p;
        cs.verts[1].p = p;
        cs.verts[2].p = p;
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row57 c23 all identical", cs, rs, "i={i} p={p:?}");
    }
}

#[test]
fn err_c23_collinear() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c23");
    let mut g = Rng::new(0x58);
    for i in 0..20_000 {
        let mut cs = rand_simplex(&mut g, 3);
        let a = V::new(g.grid(), g.grid());
        let b = V::new(g.grid(), g.grid());
        // c = a + k*(b-a) : exactly collinear -> area == 0
        let k = g.grid();
        cs.verts[0].p = a;
        cs.verts[1].p = b;
        cs.verts[2].p = V::new(a.x + k * (b.x - a.x), a.y + k * (b.y - a.y));
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row58 c23 collinear", cs, rs, "i={i} a={a:?} b={b:?} k={k:?}");
    }
}

#[test]
fn err_c23_nan() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c23");
    let mut g = Rng::new(0x59);
    let nanp: &[f32] = &[
        f32::NAN,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0xffc0_4321),
        f32::from_bits(0x7f80_0001),
    ];
    for &nv in nanp {
        for slot in 0..6usize {
            for i in 0..300 {
                let mut cs = rand_simplex(&mut g, 3);
                let mut pv = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
                pv[slot] = nv;
                cs.verts[0].p = V::new(pv[0], pv[1]);
                cs.verts[1].p = V::new(pv[2], pv[3]);
                cs.verts[2].p = V::new(pv[4], pv[5]);
                let mut rs = cs;
                unsafe {
                    c(&mut cs);
                    r(&mut rs);
                }
                ck_bytes!("row59 c23 NaN", cs, rs, "nan={:#010x} slot={slot} i={i}", nv.to_bits());
            }
        }
        // all NaN -> every comparison false -> final else, count = 3
        let mut cs = rand_simplex(&mut g, 3);
        for k in 0..3 {
            cs.verts[k].p = V::new(nv, nv);
        }
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("row59 c23 all-NaN", cs, rs, "nan={:#010x}", nv.to_bits());
        assert_eq!(cs.count, 3, "all-NaN must fall to the interior arm");
        assert!(cs.div.is_nan());
    }
}

// ===========================================================================
// Row 60 — c2BBVerts
// ===========================================================================

#[test]
fn err_bbverts_edge() {
    let l = libs();
    let (c, r) = l.get::<FnBBVerts>("c2BBVerts");
    let vals: &[f32] = &[
        0.0, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY,
        f32::MAX, f32::MIN, 1e-45,
    ];
    for &a in vals {
        for &b in vals {
            // inverted, degenerate and non-finite AABBs
            for (mn, mx) in [
                (V::new(a, b), V::new(b, a)),
                (V::new(a, a), V::new(a, a)),
                (V::new(b, b), V::new(a, a)),
            ] {
                let mut cbb = AABB { min: mn, max: mx };
                let mut rbb = cbb;
                let mut co = poisoned_verts(8);
                let mut ro = poisoned_verts(8);
                unsafe {
                    c(co.as_mut_ptr(), &mut cbb);
                    r(ro.as_mut_ptr(), &mut rbb);
                }
                for k in 0..8 {
                    ck_v!("row60 c2BBVerts", co[k], ro[k], "k={k} min={mn:?} max={mx:?}");
                }
                // only 4 verts may be written
                for k in 4..8 {
                    assert_eq!(co[k].x.to_bits(), 0xA5A5_A5A5, "slot {k} must stay poisoned");
                }
                ck_bytes!("row60 input AABB untouched", cbb, rbb, "min={mn:?} max={mx:?}");
            }
        }
    }
}

// ===========================================================================
// Rows 61-65 — the public `gjk` wrapper
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn wrap_check(l: &Pair, ctx: &str, rev: c_char, p: [f32; 9]) {
    let (c, r) = l.get::<FnGjkWrapper>("gjk");
    let (mut ca, mut cb) = (V::new(POISON_F32, POISON_F32), V::new(POISON_F32, POISON_F32));
    let (mut ra, mut rb) = (ca, cb);
    unsafe {
        c(rev, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
        r(rev, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
    }
    ck_v!("gjk outA", ca, ra, "{ctx} rev={rev} p={p:?}");
    ck_v!("gjk outB", cb, rb, "{ctx} rev={rev} p={p:?}");
}

#[test]
fn err_gjk_wrapper_forward() {
    let l = libs();
    let mut g = Rng::new(0x61);
    for i in 0..20_000 {
        let p = [
            g.grid(), g.grid(), g.grid(), g.grid(),
            g.grid(), g.grid(), g.grid(), g.grid(),
            g.grid().abs(),
        ];
        wrap_check(l, &format!("row61 forward i={i}"), 0, p);
    }
}

#[test]
fn err_gjk_wrapper_reverse_truthy() {
    let l = libs();
    let mut g = Rng::new(0x62);
    // C tests `if (reverse)` on a `char`, so every nonzero byte is "reverse".
    for &rev in &[1i8, 2, -1, 0x7f, -0x80, 42, -42] {
        for i in 0..3_000 {
            let p = [
                g.grid(), g.grid(), g.grid(), g.grid(),
                g.grid(), g.grid(), g.grid(), g.grid(),
                g.grid().abs(),
            ];
            wrap_check(l, &format!("row62 reverse={rev} i={i}"), rev, p);
        }
    }
}

#[test]
fn err_gjk_wrapper_char_truncation() {
    let l = libs();
    let (c, r) = l.get::<FnGjkWrapper>("gjk");
    // A `char` parameter only carries 8 bits. Verify that reverse=0 and
    // reverse=1 really do differ (so the flag is meaningful) and that the two
    // libraries agree for every one of the 256 byte values.
    let p = [0.0f32, 0.0, 2.0, 2.0, 1.0, -3.0, 1.0, 3.0, 0.5];
    let mut differ = false;
    let (mut a0, mut b0) = (V::default(), V::default());
    for rev in i8::MIN..=i8::MAX {
        let (mut ca, mut cb) = (V::new(POISON_F32, POISON_F32), V::new(POISON_F32, POISON_F32));
        let (mut ra, mut rb) = (ca, cb);
        unsafe {
            c(rev, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            r(rev, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
        }
        ck_v!("row63 gjk char outA", ca, ra, "rev={rev}");
        ck_v!("row63 gjk char outB", cb, rb, "rev={rev}");
        if rev == 0 {
            a0 = ca;
            b0 = cb;
        } else if ca.bits() != a0.bits() || cb.bits() != b0.bits() {
            differ = true;
        }
    }
    assert!(differ, "reverse must actually change the result for this input");
}

#[test]
fn err_gjk_wrapper_null_out() {
    let l = libs();
    let (c, r) = l.get::<FnGjkWrapper>("gjk");
    let mut g = Rng::new(0x64);
    for i in 0..10_000 {
        let p = [
            g.grid(), g.grid(), g.grid(), g.grid(),
            g.grid(), g.grid(), g.grid(), g.grid(),
            g.grid().abs(),
        ];
        for rev in [0i8, 1] {
            for (na, nb) in [(true, true), (true, false), (false, true)] {
                let (mut ca, mut cb) = (V::new(POISON_F32, POISON_F32), V::new(POISON_F32, POISON_F32));
                let (mut ra, mut rb) = (ca, cb);
                let cap = if na { std::ptr::null_mut() } else { &mut ca as *mut V };
                let cbp = if nb { std::ptr::null_mut() } else { &mut cb as *mut V };
                let rap = if na { std::ptr::null_mut() } else { &mut ra as *mut V };
                let rbp = if nb { std::ptr::null_mut() } else { &mut rb as *mut V };
                unsafe {
                    c(rev, cap, cbp, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
                    r(rev, rap, rbp, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
                }
                ck_v!("row64 gjk NULL out a", ca, ra, "i={i} rev={rev} na={na} nb={nb}");
                ck_v!("row64 gjk NULL out b", cb, rb, "i={i} rev={rev} na={na} nb={nb}");
                if na {
                    assert_eq!(ca.x.to_bits(), POISON_F32.to_bits());
                    assert_eq!(ra.x.to_bits(), POISON_F32.to_bits());
                }
                if nb {
                    assert_eq!(cb.x.to_bits(), POISON_F32.to_bits());
                    assert_eq!(rb.x.to_bits(), POISON_F32.to_bits());
                }
            }
        }
    }
}

#[test]
fn err_gjk_wrapper_degenerate() {
    let l = libs();
    let specials: &[f32] = &[
        0.0, -0.0, f32::NAN, f32::from_bits(0x7fc0_0777), f32::INFINITY,
        f32::NEG_INFINITY, f32::MAX, f32::MIN, 1e-45, -1e-45, -1.0, 1e30,
    ];
    // one special value in each of the nine slots, everything else well-formed
    for slot in 0..9usize {
        for &sv in specials {
            let mut p = [-2.0f32, -2.0, 3.0, 4.0, 1.0, 1.0, 5.0, -2.0, 0.75];
            p[slot] = sv;
            for rev in [0i8, 1] {
                wrap_check(l, &format!("row65 slot={slot} val={sv:?}"), rev, p);
            }
        }
    }
    // two special values at once
    for slot in 0..9usize {
        for slot2 in 0..9usize {
            let mut p = [-2.0f32, -2.0, 3.0, 4.0, 1.0, 1.0, 5.0, -2.0, 0.75];
            p[slot] = f32::NAN;
            p[slot2] = f32::INFINITY;
            for rev in [0i8, 1] {
                wrap_check(l, &format!("row65 pair {slot}/{slot2}"), rev, p);
            }
        }
    }
}

// ===========================================================================
// Rows 66-67 — c2Maxv / c2Minv / c2Clampv
// ===========================================================================

#[test]
fn err_minmax_nan() {
    let l = libs();
    let (cx, rx) = l.get::<FnVVV>("c2Maxv");
    let (cn, rn) = l.get::<FnVVV>("c2Minv");
    let (cc, rc) = l.get::<FnVVVV>("c2Clampv");
    // The C ternary `a > b ? a : b` returns b whenever either side is NaN, so
    // the operation is ASYMMETRIC in NaN — the exact payload returned depends on
    // which argument is which.
    let nanp: &[f32] = &[
        f32::NAN,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0xffc0_1111),
        f32::from_bits(0x7f80_0001),
    ];
    let fin: &[f32] = &[0.0, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY];
    for &nv in nanp {
        for &fv in fin {
            // NaN on the left, then on the right
            for (a, b) in [
                (V::new(nv, fv), V::new(fv, nv)),
                (V::new(fv, nv), V::new(nv, fv)),
                (V::new(nv, nv), V::new(fv, fv)),
                (V::new(fv, fv), V::new(nv, nv)),
            ] {
                ck_v!("row66 c2Maxv NaN", unsafe { cx(a, b) }, unsafe { rx(a, b) }, "a={a:?} b={b:?}");
                ck_v!("row66 c2Minv NaN", unsafe { cn(a, b) }, unsafe { rn(a, b) }, "a={a:?} b={b:?}");
                ck_v!("row66 c2Clampv NaN", unsafe { cc(a, b, a) }, unsafe { rc(a, b, a) }, "a={a:?} b={b:?}");
                ck_v!("row66 c2Clampv NaN2", unsafe { cc(b, a, b) }, unsafe { rc(b, a, b) }, "a={a:?} b={b:?}");
            }
        }
    }
}

#[test]
fn err_clampv_inverted() {
    let l = libs();
    let (c, r) = l.get::<FnVVVV>("c2Clampv");
    let mut g = Rng::new(0x67);
    for i in 0..20_000 {
        let a = g.v_mixed();
        let p = g.v_coord();
        let q = g.v_coord();
        // deliberately inverted: lo > hi
        let lo = V::new(p.x.max(q.x), p.y.max(q.y));
        let hi = V::new(p.x.min(q.x), p.y.min(q.y));
        let cv = unsafe { c(a, lo, hi) };
        let rv = unsafe { r(a, lo, hi) };
        ck_v!("row67 c2Clampv inverted range", cv, rv, "i={i} a={a:?} lo={lo:?} hi={hi:?}");
        // with lo > hi the C's Maxv(lo, Minv(a,hi)) always yields lo
        if a.x.is_finite() && lo.x > hi.x {
            assert_eq!(cv.x.to_bits(), lo.x.to_bits(), "inverted range must clamp to lo");
        }
    }
    // lo == hi
    for i in 0..5_000 {
        let a = g.v_mixed();
        let p = g.v_coord();
        let cv = unsafe { c(a, p, p) };
        let rv = unsafe { r(a, p, p) };
        ck_v!("row67 c2Clampv lo==hi", cv, rv, "i={i} a={a:?} p={p:?}");
    }
}
