//! Phase C — error/rejection-path differential tests.
//! One test (or one clearly labelled block) per row of ERRORS.md.
//! Every row asserts the SAME sentinel/error value from both `.so`s, not just
//! "both failed somehow".

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// Out-of-range `C2_TYPE` values crossing the FFI boundary. C enums accept any
/// `int`, so these are all real inputs the library must handle.
const BAD_TYPES: &[c_int] = &[
    3,
    4,
    -1,
    -2,
    7,
    99,
    255,
    256,
    65536,
    c_int::MAX,
    c_int::MIN,
    c_int::MAX - 1,
    c_int::MIN + 1,
];

/// Out-of-range simplex `count` values.
const BAD_COUNTS: &[c_int] = &[0, 4, 5, 8, -1, -2, -3, 100, c_int::MAX, c_int::MIN];

// ===========================================================================
// E1 — c2MakeProxy with a type that has no `case` label: writes nothing
// ===========================================================================
#[test]
fn e1_makeproxy_invalid_type_writes_nothing() {
    let l = libs();
    let mut rng = Rng::new(0xE1);
    let sentinel = c2Proxy {
        radius: -1234.5,
        count: -99,
        verts: [c2v { x: 9.25, y: -9.25 }; 8],
    };
    unsafe {
        for &t in BAD_TYPES {
            for _ in 0..64 {
                // any shape bytes; they must not be read at all
                let cap = rng.capsule();
                let mut pc = sentinel;
                let mut pr = sentinel;
                (l.c.c2MakeProxy)(&cap as *const _ as *const c_void, t, &mut pc);
                (l.r.c2MakeProxy)(&cap as *const _ as *const c_void, t, &mut pr);
                eq_proxy(&format!("E1 c2MakeProxy type={t}"), &pc, &pr);
                assert_eq!(
                    pc, sentinel,
                    "E1: C wrote to the proxy for invalid type {t}"
                );
                assert_eq!(
                    pr, sentinel,
                    "E1: Rust wrote to the proxy for invalid type {t}"
                );
            }
        }
        // A NULL shape pointer is safe here because it is never dereferenced.
        let mut pc = sentinel;
        let mut pr = sentinel;
        (l.c.c2MakeProxy)(std::ptr::null(), 42, &mut pc);
        (l.r.c2MakeProxy)(std::ptr::null(), 42, &mut pr);
        eq_proxy("E1 c2MakeProxy NULL shape + bad type", &pc, &pr);
        assert_eq!(pc, sentinel);
        assert_eq!(pr, sentinel);
    }
}

// ===========================================================================
// E2/E3/E4 — c2GJKSimplexMetric `default:` falls through to `case 1:` -> 0
// ===========================================================================
#[test]
fn e2_e3_e4_simplex_metric_bad_count() {
    let l = libs();
    let mut rng = Rng::new(0xE2);
    unsafe {
        let mut counts: Vec<c_int> = BAD_COUNTS.to_vec();
        counts.push(1); // E3
        for &n in &counts {
            for _ in 0..128 {
                let d = rng.coord();
                let mut sc = rand_simplex(&mut rng, n, d);
                let mut sr = sc;
                let a = (l.c.c2GJKSimplexMetric)(&mut sc);
                let b = (l.r.c2GJKSimplexMetric)(&mut sr);
                eq_f32(&format!("E2-E4 metric count={n}"), a, b);
                eq_f32(&format!("E2-E4 metric count={n} sentinel"), 0.0, a);
                eq_simplex(&format!("E2-E4 metric count={n} unmodified"), &sc, &sr);
            }
        }
    }
}

// ===========================================================================
// E5/E6 — c2D returns (0,0) for count 3 and every out-of-range count
// ===========================================================================
#[test]
fn e5_e6_c2d_bad_count() {
    let l = libs();
    let mut rng = Rng::new(0xE5);
    unsafe {
        let mut counts: Vec<c_int> = BAD_COUNTS.to_vec();
        counts.push(3); // E5
        for &n in &counts {
            for _ in 0..128 {
                let d = rng.coord();
                let mut sc = rand_simplex(&mut rng, n, d);
                let mut sr = sc;
                let a = (l.c.c2D)(&mut sc);
                let b = (l.r.c2D)(&mut sr);
                eq_v(&format!("E5-E6 c2D count={n}"), a, b);
                eq_v(
                    &format!("E5-E6 c2D count={n} sentinel"),
                    c2v { x: 0.0, y: 0.0 },
                    a,
                );
            }
        }
    }
}

// ===========================================================================
// E7/E9/E10 — c2L default: -> (0,0); div == +/-0 -> +/-inf `den`
// ===========================================================================
#[test]
fn e7_e9_e10_c2l_bad_count_and_zero_div() {
    let l = libs();
    let mut rng = Rng::new(0xE7);
    unsafe {
        let mut counts: Vec<c_int> = BAD_COUNTS.to_vec();
        counts.push(3);
        for &n in &counts {
            for &div in &[0.0f32, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY] {
                for _ in 0..64 {
                    let mut sc = rand_simplex(&mut rng, n, div);
                    let mut sr = sc;
                    let a = (l.c.c2L)(&mut sc);
                    let b = (l.r.c2L)(&mut sr);
                    eq_v(&format!("E7 c2L count={n} div={div}"), a, b);
                    eq_v(
                        &format!("E7 c2L count={n} div={div} sentinel"),
                        c2v { x: 0.0, y: 0.0 },
                        a,
                    );
                }
            }
        }
        // valid counts with a zero / negative-zero / NaN div: no trap, IEEE result
        for &n in &[1i32, 2] {
            for &div in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                for _ in 0..256 {
                    let mut sc = rand_simplex(&mut rng, n, div);
                    let mut sr = sc;
                    eq_v(
                        &format!("E9/E10 c2L count={n} div={div}"),
                        (l.c.c2L)(&mut sc),
                        (l.r.c2L)(&mut sr),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// E8/E9/E10 — c2Witness default: -> (0,0),(0,0); zero div
// ===========================================================================
#[test]
fn e8_e9_e10_witness_bad_count_and_zero_div() {
    let l = libs();
    let mut rng = Rng::new(0xE8);
    unsafe {
        for &n in BAD_COUNTS {
            for &div in &[0.0f32, -0.0, 1.0, f32::NAN] {
                for _ in 0..64 {
                    let mut sc = rand_simplex(&mut rng, n, div);
                    let mut sr = sc;
                    let (mut ac, mut bc) = (c2v { x: 1.0, y: 2.0 }, c2v { x: 3.0, y: 4.0 });
                    let (mut ar, mut br) = (ac, bc);
                    (l.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (l.r.c2Witness)(&mut sr, &mut ar, &mut br);
                    eq_v(&format!("E8 witness a count={n}"), ac, ar);
                    eq_v(&format!("E8 witness b count={n}"), bc, br);
                    eq_v(
                        &format!("E8 witness a count={n} sentinel"),
                        c2v { x: 0.0, y: 0.0 },
                        ac,
                    );
                    eq_v(
                        &format!("E8 witness b count={n} sentinel"),
                        c2v { x: 0.0, y: 0.0 },
                        bc,
                    );
                }
            }
        }
        for &n in &[1i32, 2, 3] {
            for &div in &[0.0f32, -0.0, f32::NAN, f32::INFINITY] {
                for _ in 0..256 {
                    let mut sc = rand_simplex(&mut rng, n, div);
                    let mut sr = sc;
                    let (mut ac, mut bc) = (c2v::default(), c2v::default());
                    let (mut ar, mut br) = (c2v::default(), c2v::default());
                    (l.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (l.r.c2Witness)(&mut sr, &mut ar, &mut br);
                    eq_v(&format!("E9/E10 witness a count={n} div={div}"), ac, ar);
                    eq_v(&format!("E9/E10 witness b count={n} div={div}"), bc, br);
                }
            }
        }
    }
}

// ===========================================================================
// E11/E12/E13/E14/E15 — division / sqrt edge cases
// ===========================================================================
#[test]
fn e11_to_e15_div_norm_len_edges() {
    let l = libs();
    let mut rng = Rng::new(0xEB);
    unsafe {
        // E11/E12: c2Div by 0 / -0 / NaN / inf
        for &b in &[0.0f32, -0.0, f32::NAN, f32::from_bits(0xFFC0_0000), f32::INFINITY, f32::NEG_INFINITY] {
            for _ in 0..256 {
                let a = rng.wild_v();
                eq_v(&format!("E11/E12 c2Div b={b}"), (l.c.c2Div)(a, b), (l.r.c2Div)(a, b));
            }
            for a in [
                c2v { x: 0.0, y: 0.0 },
                c2v { x: -0.0, y: 0.0 },
                c2v { x: 1.0, y: -1.0 },
                c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
                c2v { x: f32::NAN, y: 1.0 },
            ] {
                eq_v(&format!("E11/E12 c2Div fixed b={b}"), (l.c.c2Div)(a, b), (l.r.c2Div)(a, b));
            }
        }
        // E13: c2Norm of the zero vector -> NaN components
        for a in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: 0.0, y: -0.0 },
        ] {
            let rc = (l.c.c2Norm)(a);
            let rr = (l.r.c2Norm)(a);
            eq_v("E13 c2Norm zero vector", rc, rr);
            assert!(rc.x.is_nan() && rc.y.is_nan(), "E13: expected NaN, got {rc:?}");
        }
        // E14: c2Len overflow -> +inf
        for a in [
            c2v { x: 1e30, y: 1e30 },
            c2v { x: f32::MAX, y: f32::MAX },
            c2v { x: f32::INFINITY, y: 0.0 },
            c2v { x: 0.0, y: f32::NEG_INFINITY },
        ] {
            let rc = (l.c.c2Len)(a);
            eq_f32("E14 c2Len overflow", rc, (l.r.c2Len)(a));
            assert_eq!(rc, f32::INFINITY, "E14: expected +inf, got {rc}");
        }
        // E15: c2Len of a NaN vector -> NaN
        for a in [
            c2v { x: f32::NAN, y: 0.0 },
            c2v { x: 0.0, y: f32::NAN },
            c2v { x: f32::from_bits(0xFFC0_0000), y: 1.0 },
            c2v { x: f32::from_bits(0x7F80_0001), y: 1.0 }, // signalling NaN
        ] {
            let rc = (l.c.c2Len)(a);
            eq_f32("E15 c2Len NaN", rc, (l.r.c2Len)(a));
            assert!(rc.is_nan(), "E15: expected NaN, got {rc}");
        }
        // E13 again through c2Norm on huge/NaN inputs
        for _ in 0..1024 {
            let a = rng.wild_v();
            eq_v("E13 c2Norm wild", (l.c.c2Norm)(a), (l.r.c2Norm)(a));
        }
    }
}

// ===========================================================================
// E16/E17/E18 — c2Support with a non-positive or unit count / NaN dots
// ===========================================================================
#[test]
fn e16_e17_e18_support_bad_count() {
    let l = libs();
    let mut rng = Rng::new(0x16);
    unsafe {
        for &count in &[0i32, -1, -2, -100, c_int::MIN, 1] {
            for _ in 0..512 {
                let mut verts = [c2v::default(); 8];
                for v in verts.iter_mut() {
                    *v = rng.v();
                }
                let d = rng.v();
                let rc = (l.c.c2Support)(verts.as_ptr(), count, d);
                let rr = (l.r.c2Support)(verts.as_ptr(), count, d);
                eq_i(&format!("E16/E17 c2Support count={count}"), rc, rr);
                eq_i(
                    &format!("E16/E17 c2Support count={count} sentinel"),
                    0,
                    rc,
                );
            }
        }
        // E18: every dot is NaN -> `dot > dmax` never true -> index 0
        for &count in &[1i32, 2, 4, 8] {
            for _ in 0..256 {
                let mut verts = [c2v::default(); 8];
                for v in verts.iter_mut() {
                    *v = c2v {
                        x: f32::NAN,
                        y: rng.coord(),
                    };
                }
                let d = rng.v();
                let rc = (l.c.c2Support)(verts.as_ptr(), count, d);
                eq_i(&format!("E18 c2Support NaN count={count}"), rc, (l.r.c2Support)(verts.as_ptr(), count, d));
                eq_i(&format!("E18 c2Support NaN count={count} sentinel"), 0, rc);
            }
        }
    }
}

// ===========================================================================
// E19..E25 — c2GJK NULL-pointer guards and the cold-cache path
// ===========================================================================
#[test]
fn e19_to_e25_gjk_null_guards() {
    let l = libs();
    let mut rng = Rng::new(0x19);
    let id = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    unsafe {
        for _ in 0..600 {
            let a = rng.capsule();
            let b = rng.circle();
            let ap = &a as *const _ as *const c_void;
            let bp = &b as *const _ as *const c_void;
            // E19/E20: NULL transform == explicit identity transform
            for (ax, bx) in [
                (std::ptr::null(), std::ptr::null()),
                (&id as *const c2x, std::ptr::null()),
                (std::ptr::null(), &id as *const c2x),
                (&id as *const c2x, &id as *const c2x),
            ] {
                let mut oac = c2v::default();
                let mut obc = c2v::default();
                let mut itc: c_int = -1;
                let mut oar = c2v::default();
                let mut obr = c2v::default();
                let mut itr: c_int = -1;
                let dc = (l.c.c2GJK)(ap, C2_TYPE_CAPSULE, ax, bp, C2_TYPE_CIRCLE, bx, &mut oac, &mut obc, 1, &mut itc, std::ptr::null_mut());
                let dr = (l.r.c2GJK)(ap, C2_TYPE_CAPSULE, ax, bp, C2_TYPE_CIRCLE, bx, &mut oar, &mut obr, 1, &mut itr, std::ptr::null_mut());
                eq_f32("E19/E20 dist", dc, dr);
                eq_v("E19/E20 outA", oac, oar);
                eq_v("E19/E20 outB", obc, obr);
                eq_i("E19/E20 iters", itc, itr);
            }
            // E21/E22/E23/E24: every combination of NULL out-params, NULL cache
            for mask in 0..8u32 {
                let mut oac = c2v { x: -1.0, y: -2.0 };
                let mut obc = c2v { x: -3.0, y: -4.0 };
                let mut itc: c_int = -55;
                let mut oar = oac;
                let mut obr = obc;
                let mut itr: c_int = -55;
                let pa = if mask & 1 != 0 { &mut oac as *mut c2v } else { std::ptr::null_mut() };
                let pb = if mask & 2 != 0 { &mut obc as *mut c2v } else { std::ptr::null_mut() };
                let pi = if mask & 4 != 0 { &mut itc as *mut c_int } else { std::ptr::null_mut() };
                let qa = if mask & 1 != 0 { &mut oar as *mut c2v } else { std::ptr::null_mut() };
                let qb = if mask & 2 != 0 { &mut obr as *mut c2v } else { std::ptr::null_mut() };
                let qi = if mask & 4 != 0 { &mut itr as *mut c_int } else { std::ptr::null_mut() };
                let dc = (l.c.c2GJK)(ap, C2_TYPE_CAPSULE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), pa, pb, 1, pi, std::ptr::null_mut());
                let dr = (l.r.c2GJK)(ap, C2_TYPE_CAPSULE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), qa, qb, 1, qi, std::ptr::null_mut());
                eq_f32("E21-E24 dist", dc, dr);
                eq_v("E21-E24 outA", oac, oar);
                eq_v("E21-E24 outB", obc, obr);
                eq_i("E21-E24 iters", itc, itr);
                if mask & 1 == 0 {
                    assert_eq!(oac, c2v { x: -1.0, y: -2.0 }, "E21: NULL outA was written");
                    assert_eq!(oar, c2v { x: -1.0, y: -2.0 }, "E21: NULL outA was written (Rust)");
                }
                if mask & 4 == 0 {
                    assert_eq!(itc, -55, "E23: NULL iterations was written");
                    assert_eq!(itr, -55, "E23: NULL iterations was written (Rust)");
                }
            }
            // E25: cache present but count == 0 (cold) -> not read, still written
            let mut cc = c2GJKCache { metric: 1234.5, count: 0, iA: [7, 7, 7], iB: [7, 7, 7], div: -9.0 };
            let mut cr = cc;
            let dc = (l.c.c2GJK)(ap, C2_TYPE_CAPSULE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), &mut cc);
            let dr = (l.r.c2GJK)(ap, C2_TYPE_CAPSULE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), &mut cr);
            eq_f32("E25 dist", dc, dr);
            eq_cache("E25 cache write-back", &cc, &cr);
            assert!(cc.count > 0, "E25: cache was not written back");
        }
    }
}

// ===========================================================================
// E26/E27 — a stale cache is accepted verbatim (the inverted validity test)
// ===========================================================================
#[test]
fn e26_e27_stale_cache_is_accepted() {
    let l = libs();
    let mut rng = Rng::new(0x26);
    let mut saw_hit_iter0 = 0usize;
    unsafe {
        for _ in 0..3000 {
            // deliberately nonsense cache metadata for a *different* geometry
            let a = c2Capsule { a: rng.v(), b: rng.v(), r: rng.radius() };
            let b = c2AABB { min: rng.v(), max: rng.v() };
            let ap = &a as *const _ as *const c_void;
            let bp = &b as *const _ as *const c_void;
            for count in 1..=3i32 {
                let mut cc = c2GJKCache {
                    metric: match rng.below(4) {
                        0 => 0.0,
                        1 => -1.0e9,   // the only value that could fail the test
                        2 => 1.0e30,
                        _ => rng.coord(),
                    },
                    count,
                    iA: [rng.below(2) as c_int, rng.below(2) as c_int, rng.below(2) as c_int],
                    iB: [rng.below(4) as c_int, rng.below(4) as c_int, rng.below(4) as c_int],
                    div: rng.coord(),
                };
                let mut cr = cc;
                let mut itc: c_int = -1;
                let mut itr: c_int = -1;
                let mut oac = c2v::default();
                let mut obc = c2v::default();
                let mut oar = c2v::default();
                let mut obr = c2v::default();
                let dc = (l.c.c2GJK)(ap, C2_TYPE_CAPSULE, std::ptr::null(), bp, C2_TYPE_AABB, std::ptr::null(), &mut oac, &mut obc, 1, &mut itc, &mut cc);
                let dr = (l.r.c2GJK)(ap, C2_TYPE_CAPSULE, std::ptr::null(), bp, C2_TYPE_AABB, std::ptr::null(), &mut oar, &mut obr, 1, &mut itr, &mut cr);
                eq_f32(&format!("E26 dist count={count}"), dc, dr);
                eq_v(&format!("E26 outA count={count}"), oac, oar);
                eq_v(&format!("E26 outB count={count}"), obc, obr);
                eq_i(&format!("E26 iters count={count}"), itc, itr);
                eq_cache(&format!("E26 cache count={count}"), &cc, &cr);
                if count == 3 && itc == 0 && dc == 0.0 {
                    saw_hit_iter0 += 1;
                }
            }
        }
    }
    assert!(
        saw_hit_iter0 > 0,
        "E27: never observed the immediate-hit (iter==0, dist==0) cache path"
    );
}

// ===========================================================================
// E28..E31 — the four loop-termination conditions
// ===========================================================================
#[test]
fn e28_to_e31_loop_termination() {
    let l = libs();
    let mut rng = Rng::new(0x28);
    let mut iter_hist = [0usize; 21];
    let mut other_iters = 0usize;
    unsafe {
        for _ in 0..20000 {
            let ka = rng.below(3);
            let kb = rng.below(3);
            let mk = |rng: &mut Rng, k: u32| -> (Vec<u8>, c_int) {
                match k {
                    0 => {
                        let c = rng.circle();
                        (
                            std::slice::from_raw_parts(&c as *const _ as *const u8, std::mem::size_of::<c2Circle>()).to_vec(),
                            C2_TYPE_CIRCLE,
                        )
                    }
                    1 => {
                        let c = rng.aabb();
                        (
                            std::slice::from_raw_parts(&c as *const _ as *const u8, std::mem::size_of::<c2AABB>()).to_vec(),
                            C2_TYPE_AABB,
                        )
                    }
                    _ => {
                        let c = rng.capsule();
                        (
                            std::slice::from_raw_parts(&c as *const _ as *const u8, std::mem::size_of::<c2Capsule>()).to_vec(),
                            C2_TYPE_CAPSULE,
                        )
                    }
                }
            };
            let (ab, at) = mk(&mut rng, ka);
            let (bb, bt) = mk(&mut rng, kb);
            let mut itc: c_int = -1;
            let mut itr: c_int = -1;
            let mut oac = c2v::default();
            let mut obc = c2v::default();
            let mut oar = c2v::default();
            let mut obr = c2v::default();
            let ur = (rng.below(2)) as c_int;
            let dc = (l.c.c2GJK)(ab.as_ptr() as *const c_void, at, std::ptr::null(), bb.as_ptr() as *const c_void, bt, std::ptr::null(), &mut oac, &mut obc, ur, &mut itc, std::ptr::null_mut());
            let dr = (l.r.c2GJK)(ab.as_ptr() as *const c_void, at, std::ptr::null(), bb.as_ptr() as *const c_void, bt, std::ptr::null(), &mut oar, &mut obr, ur, &mut itr, std::ptr::null_mut());
            eq_f32("E28-E31 dist", dc, dr);
            eq_v("E28-E31 outA", oac, oar);
            eq_v("E28-E31 outB", obc, obr);
            eq_i("E28-E31 iters", itc, itr);
            if (0..=20).contains(&itc) {
                iter_hist[itc as usize] += 1;
            } else {
                other_iters += 1;
            }
        }
    }
    // The loop must terminate through several different routes, not just one.
    let distinct = iter_hist.iter().filter(|c| **c > 0).count();
    assert!(
        distinct >= 3,
        "E28-E31: only {distinct} distinct iteration counts observed: {iter_hist:?}"
    );
    assert_eq!(other_iters, 0, "iterations outside 0..=20 observed");
    eprintln!("E28-E31 iteration histogram: {iter_hist:?}");
}

// ===========================================================================
// E29 — degenerate search direction (dot(d,d) < FLT_EPSILON^2)
// ===========================================================================
#[test]
fn e29_degenerate_direction() {
    let l = libs();
    let mut rng = Rng::new(0x29);
    unsafe {
        // Coincident / near-coincident shapes drive `d` to (0,0).
        for _ in 0..4000 {
            let p = rng.v();
            let eps = f32::EPSILON * 0.25;
            let pairs: [(c2Circle, c2Circle); 3] = [
                (c2Circle { p, r: 0.0 }, c2Circle { p, r: 0.0 }),
                (
                    c2Circle { p, r: rng.radius() },
                    c2Circle { p: c2v { x: p.x + eps, y: p.y }, r: rng.radius() },
                ),
                (
                    c2Circle { p, r: rng.radius() },
                    c2Circle { p, r: rng.radius() },
                ),
            ];
            for (i, (a, b)) in pairs.iter().enumerate() {
                for &ur in &[0i32, 1] {
                    let mut itc: c_int = -1;
                    let mut itr: c_int = -1;
                    let mut oac = c2v::default();
                    let mut obc = c2v::default();
                    let mut oar = c2v::default();
                    let mut obr = c2v::default();
                    let dc = (l.c.c2GJK)(a as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(), b as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(), &mut oac, &mut obc, ur, &mut itc, std::ptr::null_mut());
                    let dr = (l.r.c2GJK)(a as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(), b as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(), &mut oar, &mut obr, ur, &mut itr, std::ptr::null_mut());
                    eq_f32(&format!("E29 dist set{i} ur={ur}"), dc, dr);
                    eq_v(&format!("E29 outA set{i} ur={ur}"), oac, oar);
                    eq_v(&format!("E29 outB set{i} ur={ur}"), obc, obr);
                    eq_i(&format!("E29 iters set{i} ur={ur}"), itc, itr);
                }
            }
        }
    }
}

// ===========================================================================
// E32/E33/E34/E36/E37 — the radius post-processing branches
// ===========================================================================
#[test]
fn e32_to_e37_radius_branches() {
    let l = libs();
    let mut rng = Rng::new(0x32);
    let mut saw_shrink = 0usize; // dist > rA+rB  -> real shrink
    let mut saw_midpoint = 0usize; // dist <= rA+rB -> midpoint, dist = 0
    let mut saw_hit = 0usize; // hit == 1
    unsafe {
        for _ in 0..8000 {
            let ra = rng.radius();
            let rb = rng.radius();
            let p = rng.v();
            let sep = rng.unit() * 4.0 * (ra.abs() + rb.abs() + 1.0);
            let a = c2Circle { p, r: ra };
            let b = c2Circle { p: c2v { x: p.x + sep, y: p.y }, r: rb };
            let ap = &a as *const _ as *const c_void;
            let bp = &b as *const _ as *const c_void;

            // E32: use_radius == 0 -> raw witness/dist
            let mut oac0 = c2v::default();
            let mut obc0 = c2v::default();
            let mut oar0 = c2v::default();
            let mut obr0 = c2v::default();
            let raw_c = (l.c.c2GJK)(ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), &mut oac0, &mut obc0, 0, std::ptr::null_mut(), std::ptr::null_mut());
            let raw_r = (l.r.c2GJK)(ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), &mut oar0, &mut obr0, 0, std::ptr::null_mut(), std::ptr::null_mut());
            eq_f32("E32 raw dist", raw_c, raw_r);
            eq_v("E32 raw outA", oac0, oar0);
            eq_v("E32 raw outB", obc0, obr0);

            // E33..E37: use_radius == 1
            let mut oac = c2v::default();
            let mut obc = c2v::default();
            let mut oar = c2v::default();
            let mut obr = c2v::default();
            let dc = (l.c.c2GJK)(ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), &mut oac, &mut obc, 1, std::ptr::null_mut(), std::ptr::null_mut());
            let dr = (l.r.c2GJK)(ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), &mut oar, &mut obr, 1, std::ptr::null_mut(), std::ptr::null_mut());
            eq_f32("E33-E37 dist", dc, dr);
            eq_v("E33-E37 outA", oac, oar);
            eq_v("E33-E37 outB", obc, obr);

            if raw_c > ra + rb && raw_c > f32::EPSILON {
                saw_shrink += 1;
            } else if raw_c == 0.0 && oac == obc {
                saw_hit += 1;
            } else {
                saw_midpoint += 1;
            }
        }
        // E36: guaranteed penetration -> hit == 1 via AABBs that overlap
        for _ in 0..4000 {
            let p = rng.v();
            let w = 1.0 + rng.unit() * 10.0;
            let a = c2AABB { min: p, max: c2v { x: p.x + w, y: p.y + w } };
            let b = c2AABB {
                min: c2v { x: p.x + w * 0.25, y: p.y + w * 0.25 },
                max: c2v { x: p.x + w * 0.75, y: p.y + w * 0.75 },
            };
            for &ur in &[0i32, 1] {
                let mut oac = c2v::default();
                let mut obc = c2v::default();
                let mut oar = c2v::default();
                let mut obr = c2v::default();
                let mut itc: c_int = -1;
                let mut itr: c_int = -1;
                let dc = (l.c.c2GJK)(&a as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(), &b as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(), &mut oac, &mut obc, ur, &mut itc, std::ptr::null_mut());
                let dr = (l.r.c2GJK)(&a as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(), &b as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(), &mut oar, &mut obr, ur, &mut itr, std::ptr::null_mut());
                eq_f32("E36 dist", dc, dr);
                eq_v("E36 outA", oac, oar);
                eq_v("E36 outB", obc, obr);
                eq_i("E36 iters", itc, itr);
                // `hit` is observable as "dist == 0 with coincident witness
                // points"; the C sometimes bails out of the loop earlier and
                // reports a tiny non-zero distance instead, which is fine --
                // we only require that BOTH libraries agree (asserted above).
                if dc == 0.0 && oac.x.to_bits() == obc.x.to_bits() && oac.y.to_bits() == obc.y.to_bits() {
                    saw_hit += 1;
                }
            }
        }
        // E37: both radii negative
        for _ in 0..4000 {
            let p = rng.v();
            let ra = -(0.5 + rng.unit() * 20.0);
            let rb = -(0.5 + rng.unit() * 20.0);
            let a = c2Circle { p, r: ra };
            let b = c2Circle { p: c2v { x: p.x + 1.0 + rng.unit() * 50.0, y: p.y }, r: rb };
            let mut oac = c2v::default();
            let mut obc = c2v::default();
            let mut oar = c2v::default();
            let mut obr = c2v::default();
            let dc = (l.c.c2GJK)(&a as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(), &b as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(), &mut oac, &mut obc, 1, std::ptr::null_mut(), std::ptr::null_mut());
            let dr = (l.r.c2GJK)(&a as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(), &b as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(), &mut oar, &mut obr, 1, std::ptr::null_mut(), std::ptr::null_mut());
            eq_f32("E37 negative-radius dist", dc, dr);
            eq_v("E37 negative-radius outA", oac, oar);
            eq_v("E37 negative-radius outB", obc, obr);
        }
    }
    assert!(saw_shrink > 0, "E32/E35: never took the radius-shrink branch");
    assert!(saw_midpoint > 0, "E33/E34: never took the midpoint branch");
    assert!(saw_hit > 0, "E36: never took the `hit` branch");
    eprintln!("E32-E37 branch counts: shrink={saw_shrink} midpoint={saw_midpoint} hit={saw_hit}");
}

// ===========================================================================
// E35 — after shrinking, a == b exactly -> dist forced back to 0
// ===========================================================================
#[test]
fn e35_shrink_collapses_to_zero() {
    let l = libs();
    let mut rng = Rng::new(0x35);
    let mut saw = 0usize;
    unsafe {
        // Huge coordinates make `a + n*rA` and `b - n*rB` round to the same
        // representable value even though dist > rA + rB.
        for base_exp in [18i32, 20, 24, 28, 30, 32, 34, 36] {
            let base = 2.0f32.powi(base_exp);
            for k in 1..200 {
                let ulp = base * f32::EPSILON;
                let sep = ulp * (k as f32) * 0.5;
                let ra = sep * (0.5 - 1.0 / (k as f32 + 2.0));
                let rb = sep * (0.5 - 1.0 / (k as f32 + 2.0));
                let a = c2Circle { p: c2v { x: base, y: 0.0 }, r: ra };
                let b = c2Circle { p: c2v { x: base + sep, y: 0.0 }, r: rb };
                let ap = &a as *const _ as *const c_void;
                let bp = &b as *const _ as *const c_void;
                let mut oac = c2v::default();
                let mut obc = c2v::default();
                let mut oar = c2v::default();
                let mut obr = c2v::default();
                let dc = (l.c.c2GJK)(ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), &mut oac, &mut obc, 1, std::ptr::null_mut(), std::ptr::null_mut());
                let dr = (l.r.c2GJK)(ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), &mut oar, &mut obr, 1, std::ptr::null_mut(), std::ptr::null_mut());
                eq_f32("E35 dist", dc, dr);
                eq_v("E35 outA", oac, oar);
                eq_v("E35 outB", obc, obr);
                // raw distance without the radius adjustment
                let raw = (l.c.c2GJK)(ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut());
                if raw > ra + rb && raw > f32::EPSILON && dc == 0.0 && oac == obc {
                    saw += 1;
                }
            }
        }
        // plus a randomized sweep at huge magnitudes
        for _ in 0..8000 {
            let base = 2.0f32.powi(16 + (rng.below(20) as i32));
            let ulp = base * f32::EPSILON;
            let sep = ulp * (1.0 + rng.unit() * 8.0);
            let f = rng.unit();
            let a = c2Circle { p: c2v { x: base, y: 0.0 }, r: sep * f * 0.999 };
            let b = c2Circle { p: c2v { x: base + sep, y: 0.0 }, r: sep * (1.0 - f) * 0.999 };
            let ap = &a as *const _ as *const c_void;
            let bp = &b as *const _ as *const c_void;
            let mut oac = c2v::default();
            let mut obc = c2v::default();
            let mut oar = c2v::default();
            let mut obr = c2v::default();
            let dc = (l.c.c2GJK)(ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), &mut oac, &mut obc, 1, std::ptr::null_mut(), std::ptr::null_mut());
            let dr = (l.r.c2GJK)(ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), &mut oar, &mut obr, 1, std::ptr::null_mut(), std::ptr::null_mut());
            eq_f32("E35 rand dist", dc, dr);
            eq_v("E35 rand outA", oac, oar);
            eq_v("E35 rand outB", obc, obr);
            let raw = (l.c.c2GJK)(ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CIRCLE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut());
            if raw > a.r + b.r && raw > f32::EPSILON && dc == 0.0 && oac == obc {
                saw += 1;
            }
        }
    }
    assert!(
        saw > 0,
        "E35: never observed the `a == b after shrink -> dist = 0` collapse"
    );
    eprintln!("E35 collapse observations: {saw}");
}

// ===========================================================================
// E38..E41 — c2Collided with out-of-range C2_TYPE values (incl. NULL shapes)
// ===========================================================================
#[test]
fn e38_to_e41_collided_bad_enums() {
    let l = libs();
    let mut rng = Rng::new(0x38);
    unsafe {
        for &ta in BAD_TYPES {
            for &tb in BAD_TYPES {
                // E38: outer default -> 0, pointers never dereferenced
                let rc = (l.c.c2Collided)(std::ptr::null(), ta, std::ptr::null(), tb);
                let rr = (l.r.c2Collided)(std::ptr::null(), ta, std::ptr::null(), tb);
                eq_i(&format!("E38 collided({ta},{tb}) NULL"), rc, rr);
                eq_i(&format!("E38 collided({ta},{tb}) sentinel"), 0, rc);
            }
            // ... and with real shape bytes present
            for _ in 0..16 {
                let a = rng.capsule();
                let b = rng.circle();
                let ap = &a as *const _ as *const c_void;
                let bp = &b as *const _ as *const c_void;
                let rc = (l.c.c2Collided)(ap, ta, bp, C2_TYPE_CIRCLE);
                eq_i(&format!("E38 collided badA={ta}"), rc, (l.r.c2Collided)(ap, ta, bp, C2_TYPE_CIRCLE));
                eq_i(&format!("E38 collided badA={ta} sentinel"), 0, rc);
                // E39/E40/E41: valid typeA, invalid typeB (B is never read)
                for &va in &[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
                    let sa: Box<[u8]> = match va {
                        C2_TYPE_CIRCLE => {
                            let c = rng.circle();
                            std::slice::from_raw_parts(&c as *const _ as *const u8, 12).to_vec().into()
                        }
                        C2_TYPE_AABB => {
                            let c = rng.aabb();
                            std::slice::from_raw_parts(&c as *const _ as *const u8, 16).to_vec().into()
                        }
                        _ => {
                            let c = rng.capsule();
                            std::slice::from_raw_parts(&c as *const _ as *const u8, 20).to_vec().into()
                        }
                    };
                    let sap = sa.as_ptr() as *const c_void;
                    let rc = (l.c.c2Collided)(sap, va, std::ptr::null(), ta);
                    let rr = (l.r.c2Collided)(sap, va, std::ptr::null(), ta);
                    eq_i(&format!("E39-E41 collided(valid {va}, bad {ta})"), rc, rr);
                    eq_i(&format!("E39-E41 collided(valid {va}, bad {ta}) sentinel"), 0, rc);
                }
            }
        }
    }
}

// ===========================================================================
// E42/E45 — negative radius sums in the squared comparisons
// ===========================================================================
#[test]
fn e42_e45_negative_radii() {
    let l = libs();
    let mut rng = Rng::new(0x42);
    unsafe {
        for _ in 0..8000 {
            let p = rng.v();
            let q = rng.v();
            let ra = -(rng.unit() * 30.0);
            let rb = -(rng.unit() * 30.0);
            // E42
            let a = c2Circle { p, r: ra };
            let b = c2Circle { p: q, r: rb };
            eq_i("E42 c2CircletoCircle neg", (l.c.c2CircletoCircle)(a, b), (l.r.c2CircletoCircle)(a, b));
            // mixed sign so that rA + rB can be negative while |rA| is large
            let a2 = c2Circle { p, r: -50.0 };
            let b2 = c2Circle { p: q, r: 10.0 };
            eq_i("E42 c2CircletoCircle mixed", (l.c.c2CircletoCircle)(a2, b2), (l.r.c2CircletoCircle)(a2, b2));
            // E45
            let cap = c2Capsule { a: p, b: q, r: rb };
            eq_i("E45 c2CircletoCapsule neg", (l.c.c2CircletoCapsule)(a, cap), (l.r.c2CircletoCapsule)(a, cap));
            let cap2 = c2Capsule { a: p, b: q, r: 5.0 };
            eq_i("E45 c2CircletoCapsule mixed", (l.c.c2CircletoCapsule)(a2, cap2), (l.r.c2CircletoCapsule)(a2, cap2));
            // negative radius through c2CircletoAABB (r*r positive again)
            let bb = rng.aabb();
            eq_i("E42 c2CircletoAABB neg", (l.c.c2CircletoAABB)(a, bb), (l.r.c2CircletoAABB)(a, bb));
        }
    }
}

// ===========================================================================
// E43 — inverted AABB through c2Clampv / c2CircletoAABB
// ===========================================================================
#[test]
fn e43_inverted_aabb() {
    let l = libs();
    let mut rng = Rng::new(0x43);
    unsafe {
        for _ in 0..8000 {
            let p = rng.v();
            let q = rng.v();
            let lo = c2v { x: p.x.max(q.x), y: p.y.max(q.y) };
            let hi = c2v { x: p.x.min(q.x), y: p.y.min(q.y) };
            let inv = c2AABB { min: lo, max: hi };
            let c = rng.circle();
            eq_i("E43 c2CircletoAABB inverted", (l.c.c2CircletoAABB)(c, inv), (l.r.c2CircletoAABB)(c, inv));
            eq_v("E43 c2Clampv inverted", (l.c.c2Clampv)(c.p, lo, hi), (l.r.c2Clampv)(c.p, lo, hi));
            let bb = c2AABB { min: lo, max: hi };
            eq_i("E43 c2AABBtoAABB inverted", (l.c.c2AABBtoAABB)(bb, bb), (l.r.c2AABBtoAABB)(bb, bb));
            let cap = rng.capsule();
            eq_i("E43 c2AABBtoCapsule inverted", (l.c.c2AABBtoCapsule)(bb, cap), (l.r.c2AABBtoCapsule)(bb, cap));
            // fully inverted AABB through the GJK proxy path
            let mut pc = c2Proxy::default();
            let mut pr = c2Proxy::default();
            (l.c.c2MakeProxy)(&inv as *const _ as *const c_void, C2_TYPE_AABB, &mut pc);
            (l.r.c2MakeProxy)(&inv as *const _ as *const c_void, C2_TYPE_AABB, &mut pr);
            eq_proxy("E43 proxy inverted", &pc, &pr);
        }
    }
}

// ===========================================================================
// E44 — zero-length capsule: the `da/dot(n,n)` division is never reached
// ===========================================================================
#[test]
fn e44_zero_length_capsule() {
    let l = libs();
    let mut rng = Rng::new(0x44);
    unsafe {
        for _ in 0..8000 {
            let p = rng.v();
            let cap = c2Capsule { a: p, b: p, r: rng.radius() };
            for c in [
                c2Circle { p, r: 0.0 },
                c2Circle { p, r: rng.radius() },
                rng.circle(),
                c2Circle { p: c2v { x: p.x + 1.0, y: p.y }, r: 0.0 },
            ] {
                let rc = (l.c.c2CircletoCapsule)(c, cap);
                let rr = (l.r.c2CircletoCapsule)(c, cap);
                eq_i("E44 c2CircletoCapsule zero-length", rc, rr);
            }
            // and through the GJK paths
            let cap2 = c2Capsule { a: p, b: p, r: rng.radius() };
            eq_i("E44 c2CapsuletoCapsule zero-length", (l.c.c2CapsuletoCapsule)(cap, cap2), (l.r.c2CapsuletoCapsule)(cap, cap2));
            let bb = rng.aabb();
            eq_i("E44 c2AABBtoCapsule zero-length", (l.c.c2AABBtoCapsule)(bb, cap), (l.r.c2AABBtoCapsule)(bb, cap));
        }
        // exact NaN-free zero-vector `n` with a NaN circle centre too
        let cap = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 1.0 };
        for c in [
            c2Circle { p: c2v { x: f32::NAN, y: 0.0 }, r: 1.0 },
            c2Circle { p: c2v { x: f32::INFINITY, y: 0.0 }, r: 1.0 },
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: f32::NAN },
        ] {
            eq_i("E44 c2CircletoCapsule zero-length special", (l.c.c2CircletoCapsule)(c, cap), (l.r.c2CircletoCapsule)(c, cap));
        }
    }
}

// ===========================================================================
// E46 — `if (c2GJK(...))` truthiness: nonzero -> 0, zero/NaN -> 1
// ===========================================================================
#[test]
fn e46_gjk_truthiness_rejection() {
    let l = libs();
    let mut rng = Rng::new(0x46);
    let mut nan_ur0 = 0usize;
    let mut nan_ur1 = 0usize;
    let mut zero_ur1 = 0usize;
    let mut nonzero_ur1 = 0usize;
    unsafe {
        for _ in 0..40000 {
            let bb = c2AABB { min: rng.wild_v(), max: rng.wild_v() };
            let cap = c2Capsule { a: rng.wild_v(), b: rng.wild_v(), r: rng.wild() };
            let bp = &bb as *const _ as *const c_void;
            let cp = &cap as *const _ as *const c_void;

            // raw (use_radius = 0) distance: this is where NaN survives
            let raw_c = (l.c.c2GJK)(bp, C2_TYPE_AABB, std::ptr::null(), cp, C2_TYPE_CAPSULE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut());
            let raw_r = (l.r.c2GJK)(bp, C2_TYPE_AABB, std::ptr::null(), cp, C2_TYPE_CAPSULE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut());
            eq_f32("E46 raw dist", raw_c, raw_r);
            if raw_c.is_nan() {
                nan_ur0 += 1;
            }

            // the distance the wrapper actually tests (use_radius = 1)
            let d_c = (l.c.c2GJK)(bp, C2_TYPE_AABB, std::ptr::null(), cp, C2_TYPE_CAPSULE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut());
            let d_r = (l.r.c2GJK)(bp, C2_TYPE_AABB, std::ptr::null(), cp, C2_TYPE_CAPSULE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut());
            eq_f32("E46 use_radius dist", d_c, d_r);

            let rc = (l.c.c2AABBtoCapsule)(bb, cap);
            let rr = (l.r.c2AABBtoCapsule)(bb, cap);
            eq_i("E46 c2AABBtoCapsule", rc, rr);
            // The C rejection rule, replicated: `if (dist) return 0; return 1;`
            // C truthiness of a float: nonzero AND non-NaN-that-compares-false.
            let expect = if d_c != 0.0 { 0 } else { 1 };
            eq_i("E46 c2AABBtoCapsule truthiness", expect, rc);
            if d_c.is_nan() {
                nan_ur1 += 1;
                assert_eq!(rc, 0, "E46: a NaN distance is nonzero-comparing -> 0");
            } else if d_c == 0.0 {
                zero_ur1 += 1;
            } else {
                nonzero_ur1 += 1;
            }

            let ca = c2Capsule { a: rng.wild_v(), b: rng.wild_v(), r: rng.wild() };
            let cb = c2Capsule { a: rng.wild_v(), b: rng.wild_v(), r: rng.wild() };
            let d2_c = (l.c.c2GJK)(&ca as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(), &cb as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut());
            let rc2 = (l.c.c2CapsuletoCapsule)(ca, cb);
            eq_i("E46 c2CapsuletoCapsule", rc2, (l.r.c2CapsuletoCapsule)(ca, cb));
            eq_i(
                "E46 c2CapsuletoCapsule truthiness",
                if d2_c != 0.0 { 0 } else { 1 },
                rc2,
            );
        }
    }
    // Both rejection outcomes must actually have been produced.
    assert!(zero_ur1 > 0, "E46: never observed a `return 1` (dist == 0) rejection");
    assert!(nonzero_ur1 > 0, "E46: never observed a `return 0` (dist != 0) rejection");
    assert!(nan_ur0 > 0, "E46: never produced a raw NaN GJK distance (use_radius=0)");
    // Documented consequence: with use_radius = 1 the midpoint branch always
    // clamps a NaN distance to 0, so the wrappers can never see NaN.
    assert_eq!(
        nan_ur1, 0,
        "E46: use_radius=1 produced a NaN distance ({nan_ur1}x) -- update ERRORS.md"
    );
    eprintln!(
        "E46 counts: raw NaN={nan_ur0}, use_radius NaN={nan_ur1}, dist==0 -> 1: {zero_ur1}, dist!=0 -> 0: {nonzero_ur1}"
    );
}

// ===========================================================================
// E47 — `capsule` performs no validation at all
// ===========================================================================
#[test]
fn e47_capsule_entry_no_validation() {
    let l = libs();
    let specials = [
        f32::NAN,
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001), // signalling NaN
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        f32::MIN_POSITIVE / 7.0,
        1.0e30,
        -1.0e30,
        -70.0,
        -40.0,
        -15.0,
        20.0,
    ];
    unsafe {
        for &a in &specials {
            for &b in &specials {
                for &r in &specials {
                    eq_i(
                        &format!("E47 capsule({a},{b},{a},{b},{r})"),
                        (l.c.capsule)(a, b, a, b, r),
                        (l.r.capsule)(a, b, a, b, r),
                    );
                    eq_i(
                        &format!("E47 capsule({a},{b},{b},{a},{r})"),
                        (l.c.capsule)(a, b, b, a, r),
                        (l.r.capsule)(a, b, b, a, r),
                    );
                    eq_i(
                        &format!("E47 capsule({r},{r},{a},{b},{a})"),
                        (l.c.capsule)(r, r, a, b, a),
                        (l.r.capsule)(r, r, a, b, a),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// B1 — c2BBVerts has no validation (inverted / special AABBs)
// ===========================================================================
#[test]
fn b1_bbverts_no_validation() {
    let l = libs();
    let mut rng = Rng::new(0xB1);
    unsafe {
        for _ in 0..8000 {
            let mut bb = c2AABB {
                min: rng.wild_v(),
                max: rng.wild_v(),
            };
            let mut oc = [c2v { x: -1.0, y: -2.0 }; 4];
            let mut or_ = oc;
            (l.c.c2BBVerts)(oc.as_mut_ptr(), &mut bb);
            (l.r.c2BBVerts)(or_.as_mut_ptr(), &mut bb);
            for k in 0..4 {
                eq_v(&format!("B1 c2BBVerts[{k}]"), oc[k], or_[k]);
            }
            // the input struct must be left untouched (bitwise, NaN-aware)
            let mut bb2 = bb;
            let mut o2 = [c2v::default(); 4];
            (l.r.c2BBVerts)(o2.as_mut_ptr(), &mut bb2);
            eq_v("B1 input min preserved", bb.min, bb2.min);
            eq_v("B1 input max preserved", bb.max, bb2.max);
        }
    }
}

// ===========================================================================
// B4 — c2GJK with a negative cache count
// ===========================================================================
#[test]
fn b4_negative_cache_count() {
    let l = libs();
    let mut rng = Rng::new(0xB4);
    unsafe {
        for &n in &[-1i32, -2, -3, -100, c_int::MIN] {
            for _ in 0..400 {
                let a = rng.capsule();
                let b = rng.aabb();
                let ap = &a as *const _ as *const c_void;
                let bp = &b as *const _ as *const c_void;
                let mut cc = c2GJKCache {
                    metric: rng.coord(),
                    count: n,
                    iA: [0, 0, 0],
                    iB: [0, 0, 0],
                    div: rng.coord(),
                };
                let mut cr = cc;
                let mut oac = c2v::default();
                let mut obc = c2v::default();
                let mut oar = c2v::default();
                let mut obr = c2v::default();
                let mut itc: c_int = -1;
                let mut itr: c_int = -1;
                let dc = (l.c.c2GJK)(ap, C2_TYPE_CAPSULE, std::ptr::null(), bp, C2_TYPE_AABB, std::ptr::null(), &mut oac, &mut obc, 1, &mut itc, &mut cc);
                let dr = (l.r.c2GJK)(ap, C2_TYPE_CAPSULE, std::ptr::null(), bp, C2_TYPE_AABB, std::ptr::null(), &mut oar, &mut obr, 1, &mut itr, &mut cr);
                eq_f32(&format!("B4 dist count={n}"), dc, dr);
                eq_v(&format!("B4 outA count={n}"), oac, oar);
                eq_v(&format!("B4 outB count={n}"), obc, obr);
                eq_i(&format!("B4 iters count={n}"), itc, itr);
                eq_cache(&format!("B4 cache count={n}"), &cc, &cr);
            }
        }
    }
}
