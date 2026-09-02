//! Phase C — error / rejection-path differential tests.
//! One test (or one clearly-labelled block) per `ERRORS.md` row.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const N: usize = 3000;

/// `count` values that hit a `default:` / out-of-range arm.
const BAD_COUNTS: [c_int; 9] = [0, 4, 5, -1, -7, 100, i32::MIN, i32::MAX, -2147483647];

/// `C2_TYPE` values with no valid variant (C enums accept any `int`).
const BAD_TYPES: [c_int; 10] = [3, 4, 5, 7, 99, -1, -2, -100, i32::MIN, i32::MAX];

fn poisoned_proxy(tag: f32) -> c2Proxy {
    let mut px = c2Proxy {
        radius: -321.75 + tag,
        count: -4242,
        verts: [c2v { x: 0.0, y: 0.0 }; 8],
    };
    for (k, v) in px.verts.iter_mut().enumerate() {
        v.x = 200.0 + k as f32 + tag;
        v.y = -200.0 - k as f32 - tag;
    }
    px
}

// ===========================================================================
// Rows 1, 22 — c2MakeProxy with an out-of-range C2_TYPE
// ===========================================================================

#[test]
fn row01_row22_makeproxy_out_of_range_enum() {
    let p = load_pair();
    let mut rng = Rng::new(0xC001);
    unsafe {
        // fixed out-of-range enum values
        for (n, &ty) in BAD_TYPES.iter().enumerate() {
            for i in 0..200 {
                let tag = i as f32;
                let mut pc = poisoned_proxy(tag);
                let mut pr = poisoned_proxy(tag);
                let before = pc;
                // any shape bytes; the C must not read them
                let cap = c2Capsule { a: rng.v(), b: rng.v(), r: rng.finite() };
                let sp = &cap as *const c2Capsule as *const c_void;
                (p.c.c2MakeProxy)(sp, ty, &mut pc);
                (p.r.c2MakeProxy)(sp, ty, &mut pr);
                eq_proxy(&format!("row01 ty={ty} #{n}/{i}"), &pc, &pr);
                // and the C really left it untouched
                eq_proxy(&format!("row01 ty={ty} #{n}/{i} untouched"), &before, &pc);
            }
        }
        // random out-of-range enum values
        for i in 0..N {
            let mut ty = rng.next_u32() as c_int;
            if (0..=2).contains(&ty) {
                ty = ty.wrapping_add(3);
            }
            let mut pc = poisoned_proxy(i as f32);
            let mut pr = poisoned_proxy(i as f32);
            let bb = c2AABB { min: rng.v(), max: rng.v() };
            let sp = &bb as *const c2AABB as *const c_void;
            (p.c.c2MakeProxy)(sp, ty, &mut pc);
            (p.r.c2MakeProxy)(sp, ty, &mut pr);
            eq_proxy(&format!("row22 rand ty={ty} #{i}"), &pc, &pr);
        }
    }
}

// ===========================================================================
// Rows 2, 3 — c2GJKSimplexMetric sentinel (`case 1` and `default`)
// ===========================================================================

#[test]
fn row02_row03_simplex_metric_sentinel() {
    let p = load_pair();
    let mut rng = Rng::new(0xC002);
    unsafe {
        let mut counts: Vec<c_int> = vec![1];
        counts.extend_from_slice(&BAD_COUNTS);
        for &count in &counts {
            for i in 0..300 {
                let mut sc = c2Simplex::default();
                for k in 0..4 {
                    sc.verts[k] = c2sv {
                        sA: rng.v(),
                        sB: rng.v(),
                        p: rng.v(),
                        u: rng.finite(),
                        iA: rng.next_u32() as c_int,
                        iB: rng.next_u32() as c_int,
                    };
                }
                sc.div = rng.finite();
                sc.count = count;
                let mut sr = sc;
                let rc = (p.c.c2GJKSimplexMetric)(&mut sc);
                let rr = (p.r.c2GJKSimplexMetric)(&mut sr);
                eq_f32(&format!("row02/03 count={count} #{i}"), rc, rr);
                eq_f32(&format!("row02/03 count={count} #{i} is-zero"), 0.0, rc);
                eq_simplex(&format!("row02/03 count={count} #{i} struct"), &sc, &sr);
            }
        }
    }
}

// ===========================================================================
// Rows 4, 5 — c2D sentinel (`case 3` and `default`)
// ===========================================================================

#[test]
fn row04_row05_c2d_sentinel() {
    let p = load_pair();
    let mut rng = Rng::new(0xC004);
    unsafe {
        let mut counts: Vec<c_int> = vec![3];
        counts.extend_from_slice(&BAD_COUNTS);
        for &count in &counts {
            for i in 0..300 {
                let mut sc = c2Simplex::default();
                for k in 0..4 {
                    sc.verts[k].p = rng.v();
                    sc.verts[k].u = rng.finite();
                }
                sc.div = rng.finite();
                sc.count = count;
                let mut sr = sc;
                let rc = (p.c.c2D)(&mut sc);
                let rr = (p.r.c2D)(&mut sr);
                eq_v(&format!("row04/05 count={count} #{i}"), rc, rr);
                eq_v(
                    &format!("row04/05 count={count} #{i} is-zero"),
                    c2v { x: 0.0, y: 0.0 },
                    rc,
                );
                eq_simplex(&format!("row04/05 count={count} #{i} struct"), &sc, &sr);
            }
        }
    }
}

// ===========================================================================
// Rows 6, 7 — c2L default arm and div == 0
// ===========================================================================

#[test]
fn row06_c2l_default_arm() {
    let p = load_pair();
    let mut rng = Rng::new(0xC006);
    unsafe {
        for &count in &BAD_COUNTS[..] {
            if count == 1 || count == 2 {
                continue;
            }
            for i in 0..300 {
                let mut sc = c2Simplex::default();
                for k in 0..4 {
                    sc.verts[k].p = rng.v();
                    sc.verts[k].u = rng.finite();
                }
                sc.div = if i % 4 == 0 { 0.0 } else { rng.finite() };
                sc.count = count;
                let mut sr = sc;
                let rc = (p.c.c2L)(&mut sc);
                let rr = (p.r.c2L)(&mut sr);
                eq_v(&format!("row06 count={count} #{i}"), rc, rr);
                eq_v(
                    &format!("row06 count={count} #{i} is-zero"),
                    c2v { x: 0.0, y: 0.0 },
                    rc,
                );
            }
        }
        // count == 3 is also a default-arm case for c2L
        for i in 0..300 {
            let mut sc = c2Simplex::default();
            for k in 0..4 {
                sc.verts[k].p = rng.v();
                sc.verts[k].u = rng.finite();
            }
            sc.div = rng.finite();
            sc.count = 3;
            let mut sr = sc;
            eq_v(
                &format!("row06 count=3 #{i}"),
                (p.c.c2L)(&mut sc),
                (p.r.c2L)(&mut sr),
            );
        }
    }
}

#[test]
fn row07_c2l_div_zero() {
    let p = load_pair();
    let mut rng = Rng::new(0xC007);
    unsafe {
        for i in 0..N {
            let mut sc = c2Simplex::default();
            for k in 0..4 {
                sc.verts[k].p = match i % 3 {
                    0 => c2v { x: 0.0, y: 0.0 },
                    1 => rng.v_coord(),
                    _ => rng.v(),
                };
                sc.verts[k].u = if i % 5 == 0 { 0.0 } else { rng.finite() };
            }
            sc.div = if i % 2 == 0 { 0.0 } else { -0.0 };
            sc.count = if i % 4 < 2 { 2 } else { 1 };
            let mut sr = sc;
            eq_v(
                &format!("row07 div={:e} count={} #{i}", sc.div, sc.count),
                (p.c.c2L)(&mut sc),
                (p.r.c2L)(&mut sr),
            );
        }
    }
}

// ===========================================================================
// Rows 8, 9, 10 — c2Witness default arm, div == 0, div == -0.0
// ===========================================================================

#[test]
fn row08_row09_row10_c2witness() {
    let p = load_pair();
    let mut rng = Rng::new(0xC008);
    unsafe {
        let divs = [0.0f32, -0.0, f32::MIN_POSITIVE, 1.0];
        let mut counts: Vec<c_int> = BAD_COUNTS.to_vec();
        counts.extend_from_slice(&[1, 2, 3]);
        for &count in &counts {
            for i in 0..120 {
                let mut sc = c2Simplex::default();
                for k in 0..4 {
                    sc.verts[k] = c2sv {
                        sA: rng.v(),
                        sB: rng.v(),
                        p: rng.v(),
                        u: if i % 6 == 0 { 0.0 } else { rng.finite() },
                        iA: 0,
                        iB: 0,
                    };
                }
                sc.div = divs[i % divs.len()];
                sc.count = count;
                let mut sr = sc;
                let mut ac = c2v { x: 5.5, y: -5.5 };
                let mut bc = c2v { x: -1.25, y: 1.25 };
                let mut ar = ac;
                let mut br = bc;
                (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
                let ctx = format!("row08/09/10 count={count} div={:e} #{i}", sc.div);
                eq_v(&format!("{ctx} a"), ac, ar);
                eq_v(&format!("{ctx} b"), bc, br);
                if !(1..=3).contains(&count) {
                    eq_v(&format!("{ctx} default-a"), c2v { x: 0.0, y: 0.0 }, ac);
                    eq_v(&format!("{ctx} default-b"), c2v { x: 0.0, y: 0.0 }, bc);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 11, 12, 13 — c2Support with count <= 0 and tie/NaN directions
// ===========================================================================

#[test]
fn row11_row12_support_nonpositive_count() {
    let p = load_pair();
    let mut rng = Rng::new(0xC011);
    unsafe {
        let counts: [c_int; 7] = [0, -1, -2, -100, i32::MIN, -2147483647, 1];
        for &count in &counts {
            for i in 0..400 {
                let mut verts = [c2v { x: 0.0, y: 0.0 }; 8];
                for k in 0..8 {
                    verts[k] = rng.v();
                }
                let d = rng.v();
                let rc = (p.c.c2Support)(verts.as_ptr(), count, d);
                let rr = (p.r.c2Support)(verts.as_ptr(), count, d);
                eq_i(&format!("row11/12 count={count} #{i}"), rc, rr);
                eq_i(&format!("row11/12 count={count} #{i} is-zero"), 0, rc);
            }
        }
    }
}

#[test]
fn row13_support_ties_and_nan() {
    let p = load_pair();
    let mut rng = Rng::new(0xC013);
    let nasty = [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
    ];
    unsafe {
        for i in 0..N {
            let mut verts = [c2v { x: 0.0, y: 0.0 }; 8];
            match i % 4 {
                0 => {
                    // all identical -> every dot equal, first index must win
                    let v = rng.v_coord();
                    verts = [v; 8];
                }
                1 => {
                    for k in 0..8 {
                        verts[k] = c2v {
                            x: nasty[(i + k) % nasty.len()],
                            y: nasty[(i + k + 1) % nasty.len()],
                        };
                    }
                }
                2 => {
                    for k in 0..8 {
                        verts[k] = rng.v_coord();
                    }
                    verts[3] = verts[0];
                    verts[5] = verts[0];
                }
                _ => {
                    for k in 0..8 {
                        verts[k] = rng.v();
                    }
                }
            }
            let d = match i % 5 {
                0 => c2v { x: 0.0, y: 0.0 },
                1 => c2v { x: f32::NAN, y: 1.0 },
                2 => c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
                3 => c2v { x: 1.0, y: 1.0 },
                _ => rng.v(),
            };
            for &count in &[1i32, 2, 4, 8] {
                eq_i(
                    &format!("row13 #{i} count={count}"),
                    (p.c.c2Support)(verts.as_ptr(), count, d),
                    (p.r.c2Support)(verts.as_ptr(), count, d),
                );
            }
        }
    }
}

// ===========================================================================
// Rows 14, 15, 16, 17, 18, 19 — division by zero, NaN/inf in the maths layer
// ===========================================================================

#[test]
fn row14_row15_c2div_by_zero() {
    let p = load_pair();
    let mut rng = Rng::new(0xC014);
    let bad = [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    unsafe {
        for i in 0..N {
            let a = match i % 4 {
                0 => c2v { x: 0.0, y: 0.0 },
                1 => c2v { x: -0.0, y: 0.0 },
                2 => c2v { x: f32::INFINITY, y: f32::NAN },
                _ => rng.v(),
            };
            for &b in &bad {
                eq_v(
                    &format!("row14/15 #{i} b={b}"),
                    (p.c.c2Div)(a, b),
                    (p.r.c2Div)(a, b),
                );
                eq_v(
                    &format!("row14/15 mulvs #{i} b={b}"),
                    (p.c.c2Mulvs)(a, b),
                    (p.r.c2Mulvs)(a, b),
                );
            }
        }
    }
}

#[test]
fn row16_row17_row18_row19_len_norm_nonfinite() {
    let p = load_pair();
    let mut rng = Rng::new(0xC016);
    let nasty = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        1.0e30,
        -1.0e30,
        f32::MIN_POSITIVE,
    ];
    unsafe {
        for (i, &x) in nasty.iter().enumerate() {
            for (j, &y) in nasty.iter().enumerate() {
                let v = c2v { x, y };
                eq_f32(&format!("row18/19 c2Len [{i}][{j}]"), (p.c.c2Len)(v), (p.r.c2Len)(v));
                eq_v(&format!("row16/17 c2Norm [{i}][{j}]"), (p.c.c2Norm)(v), (p.r.c2Norm)(v));
            }
        }
        for i in 0..N {
            let v = match i % 3 {
                0 => c2v { x: 0.0, y: 0.0 },
                1 => c2v { x: rng.scaled(1.0e30), y: rng.scaled(1.0e30) },
                _ => c2v { x: rng.finite(), y: rng.finite() },
            };
            eq_f32(&format!("row18/19 rand c2Len #{i}"), (p.c.c2Len)(v), (p.r.c2Len)(v));
            eq_v(&format!("row16/17 rand c2Norm #{i}"), (p.c.c2Norm)(v), (p.r.c2Norm)(v));
        }
    }
}

// ===========================================================================
// Rows 20, 21 — out-of-range C2_TYPE reaching c2GJK
// ===========================================================================

/// `c2MakeProxy` writes nothing for an out-of-range type, so `c2GJK`'s
/// `c2Proxy pA; c2Proxy pB;` locals stay *uninitialised* and the following
/// reads are genuine C undefined behaviour (stack garbage from the caller's
/// frame history). No translation can reproduce that, so this test asserts the
/// only deterministic, well-defined part of the contract instead:
///
///  * `c2MakeProxy` itself performs **no write** for every out-of-range type
///    (already covered exhaustively by `row01_row22_makeproxy_out_of_range_enum`);
///  * `c2GJK` is called with all nine *valid* enum pairings and matches.
///
/// The divergence is documented in `ERRORS.md` rather than silently skipped.
#[test]
fn row20_row21_out_of_range_type_is_c_ub() {
    let p = load_pair();
    let mut rng = Rng::new(0xC020);
    unsafe {
        // deterministic half: no write for any bad type, for both C and Rust
        for &ty in BAD_TYPES.iter() {
            let mut pc = poisoned_proxy(1.0);
            let mut pr = poisoned_proxy(1.0);
            let before = pc;
            let ci = c2Circle { p: rng.v(), r: rng.finite() };
            let sp = &ci as *const c2Circle as *const c_void;
            (p.c.c2MakeProxy)(sp, ty, &mut pc);
            (p.r.c2MakeProxy)(sp, ty, &mut pr);
            eq_proxy(&format!("row20/21 no-write C ty={ty}"), &before, &pc);
            eq_proxy(&format!("row20/21 no-write Rust ty={ty}"), &before, &pr);
        }
        // valid pairings through c2GJK still agree
        for i in 0..500 {
            for &ta in ALL_TYPES.iter() {
                for &tb in ALL_TYPES.iter() {
                    let ca = rng.v_coord();
                    let sa = gen_shape(&mut rng, ta, ca, 3.0);
                    let cb = rng.v_coord();
                    let sb = gen_shape(&mut rng, tb, cb, 3.0);
                    let oc = call_gjk(&p.c, &sa, None, &sb, None, 1, true, true, true, None);
                    let or = call_gjk(&p.r, &sa, None, &sb, None, 1, true, true, true, None);
                    eq_gjk_out(&format!("row20/21 valid #{i} {ta}x{tb}"), &oc, &or);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 23-28 — c2GJK null-pointer guards
// ===========================================================================

#[test]
fn row23_row24_null_transforms() {
    let p = load_pair();
    let mut rng = Rng::new(0xC023);
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let ca = rng.v_coord();
            let sa = gen_shape(&mut rng, ta, ca, 4.0);
            let cb = rng.v_coord();
            let sb = gen_shape(&mut rng, tb, cb, 4.0);
            let t = rng.x_transform();
            let ur = (i % 2) as c_int;
            let id = (p.c.c2xIdentity)();
            // (ax, bx) in {(null,null), (null,set), (set,null)}
            for combo in 0..3 {
                let (ax, bx) = match combo {
                    0 => (None, None),
                    1 => (None, Some(&t)),
                    _ => (Some(&t), None),
                };
                let oc = call_gjk(&p.c, &sa, ax, &sb, bx, ur, true, true, true, None);
                let or = call_gjk(&p.r, &sa, ax, &sb, bx, ur, true, true, true, None);
                eq_gjk_out(&format!("row23/24 #{i} combo={combo}"), &oc, &or);
            }
            // NULL must behave exactly like an explicit identity, in both
            let n_c = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, None);
            let i_c = call_gjk(&p.c, &sa, Some(&id), &sb, Some(&id), ur, true, true, true, None);
            eq_f32(&format!("row23/24 #{i} C null==id dist"), n_c.dist, i_c.dist);
            let n_r = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, None);
            let i_r = call_gjk(&p.r, &sa, Some(&id), &sb, Some(&id), ur, true, true, true, None);
            eq_f32(&format!("row23/24 #{i} R null==id dist"), n_r.dist, i_r.dist);
        }
    }
}

#[test]
fn row25_row26_row27_row28_null_outputs_and_cache() {
    let p = load_pair();
    let mut rng = Rng::new(0xC025);
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let ca = rng.v_coord();
            let sa = gen_shape(&mut rng, ta, ca, 4.0);
            let cb = rng.v_coord();
            let sb = gen_shape(&mut rng, tb, cb, 4.0);
            let ur = (i % 2) as c_int;
            for combo in 0..8 {
                let want_a = combo & 1 != 0;
                let want_b = combo & 2 != 0;
                let want_it = combo & 4 != 0;
                // cache == NULL (row 28)
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, want_a, want_b, want_it, None);
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, want_a, want_b, want_it, None);
                eq_gjk_out(&format!("row25-28 #{i} combo={combo}"), &oc, &or);
                // untouched sentinels must survive identically
                if !want_a {
                    eq_v(
                        &format!("row25 #{i} outA untouched"),
                        c2v { x: 12.5, y: -7.25 },
                        oc.a,
                    );
                }
                if !want_b {
                    eq_v(
                        &format!("row26 #{i} outB untouched"),
                        c2v { x: -3.125, y: 9.5 },
                        oc.b,
                    );
                }
                if !want_it {
                    eq_i(&format!("row27 #{i} iterations untouched"), -12345, oc.iters);
                    eq_i(&format!("row27 #{i} iterations untouched R"), -12345, or.iters);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 29, 30, 34 — cache_was_good / the inverted cache-validity test
// ===========================================================================

#[test]
fn row29_zero_count_cache_not_read() {
    let p = load_pair();
    let mut rng = Rng::new(0xC029);
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let ca = rng.v_coord();
            let sa = gen_shape(&mut rng, ta, ca, 4.0);
            let cb = rng.v_coord();
            let sb = gen_shape(&mut rng, tb, cb, 4.0);
            let ur = (i % 2) as c_int;
            // count == 0 but garbage elsewhere: must be ignored, then written back
            let base = c2GJKCache {
                metric: rng.finite(),
                count: 0,
                iA: [rng.next_u32() as c_int, 7, -3],
                iB: [rng.next_u32() as c_int, 5, -9],
                div: rng.finite(),
            };
            let mut cc = base;
            let mut cr = base;
            let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
            let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
            eq_gjk_out(&format!("row29 #{i}"), &oc, &or);
            eq_cache(&format!("row29 #{i} writeback"), &cc, &cr);
            // and it must equal the cache==NULL result
            let nc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, None);
            eq_f32(&format!("row29 #{i} == null-cache"), nc.dist, oc.dist);
        }
    }
}

/// The C validity test is `!(min_metric < max_metric * 2.0f && metric < -1.0e8f)`.
/// The `metric < -1.0e8f` conjunct makes `cache_was_read = 1` for virtually every
/// cache, but it *is* reachable for `count == 3` (where the metric is a signed
/// `c2Det2`) with large coordinates. Both sides of the predicate are covered here
/// and the coverage is asserted.
#[test]
fn row30_row34_cache_validity_predicate() {
    let p = load_pair();
    let mut rng = Rng::new(0xC030);
    let mut hit_read = 0usize;
    let mut hit_reject = 0usize;
    unsafe {
        for i in 0..N {
            // large AABB vs large AABB so a count-3 seed can produce a metric
            // far below -1e8
            let s = 3.0e4f32;
            let ca = c2v { x: rng.scaled(s), y: rng.scaled(s) };
            let cb = c2v { x: rng.scaled(s), y: rng.scaled(s) };
            let sa = Shape::Aabb(c2AABB {
                min: c2v { x: ca.x - s, y: ca.y - s },
                max: c2v { x: ca.x + s, y: ca.y + s },
            });
            let sb = Shape::Aabb(c2AABB {
                min: c2v { x: cb.x - s, y: cb.y - s },
                max: c2v { x: cb.x + s, y: cb.y + s },
            });
            let metric_old = match i % 5 {
                0 => f32::NAN,
                1 => 5.0,
                2 => -1.0e9,
                3 => rng.scaled(1.0e10),
                _ => rng.finite(),
            };
            let base = c2GJKCache {
                metric: metric_old,
                count: 3,
                iA: [
                    rng.below(4) as c_int,
                    rng.below(4) as c_int,
                    rng.below(4) as c_int,
                ],
                iB: [
                    rng.below(4) as c_int,
                    rng.below(4) as c_int,
                    rng.below(4) as c_int,
                ],
                div: if i % 7 == 0 { 0.0 } else { rng.finite() },
            };
            let ur = (i % 2) as c_int;
            let mut cc = base;
            let mut cr = base;
            let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
            let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
            eq_gjk_out(&format!("row30/34 #{i}"), &oc, &or);
            eq_cache(&format!("row30/34 #{i} writeback"), &cc, &cr);

            // Independently recompute the C predicate to prove coverage.
            let mut pa = c2Proxy::default();
            let mut pb = c2Proxy::default();
            (p.c.c2MakeProxy)(sa.as_ptr(), sa.ty(), &mut pa);
            (p.c.c2MakeProxy)(sb.as_ptr(), sb.ty(), &mut pb);
            let mut seed = c2Simplex::default();
            for k in 0..3 {
                let va = pa.verts[base.iA[k] as usize];
                let vb = pb.verts[base.iB[k] as usize];
                seed.verts[k].p = (p.c.c2Sub)(vb, va);
            }
            seed.count = 3;
            let metric = (p.c.c2GJKSimplexMetric)(&mut seed);
            let min_m = if metric < metric_old { metric } else { metric_old };
            let max_m = if metric > metric_old { metric } else { metric_old };
            if min_m < max_m * 2.0 && metric < -1.0e8f32 {
                hit_reject += 1;
            } else {
                hit_read += 1;
            }
        }
    }
    assert!(hit_read > 0, "cache_was_read=1 branch never taken");
    assert!(
        hit_reject > 0,
        "cache_was_read=0 branch (metric < -1e8) never taken; predicate coverage incomplete"
    );
}

// ===========================================================================
// Rows 31, 32, 33 — out-of-range / negative / zero-div cache counts
// ===========================================================================

/// `cache->count == 4` makes the C read `iA[3]`/`iB[3]`, one past the declared
/// 3-element arrays. Those bytes are `iB[0]` and `div` of the same struct, so
/// the read is deterministic (same layout in both implementations). The values
/// are chosen so the aliased bytes are valid vertex indices.
#[test]
fn row31_cache_count_four() {
    let p = load_pair();
    let mut rng = Rng::new(0xC031);
    unsafe {
        for i in 0..500 {
            let ca = rng.v_coord();
            let sa = Shape::Aabb(c2AABB {
                min: c2v { x: ca.x - 2.0, y: ca.y - 2.0 },
                max: c2v { x: ca.x + 2.0, y: ca.y + 2.0 },
            });
            let cb = rng.v_coord();
            let sb = Shape::Aabb(c2AABB {
                min: c2v { x: cb.x - 3.0, y: cb.y - 3.0 },
                max: c2v { x: cb.x + 3.0, y: cb.y + 3.0 },
            });
            // iA[3] aliases iB[0]; iB[3] aliases div. Keep both in [0,3].
            let alias_idx = (i % 4) as c_int;
            let base = c2GJKCache {
                metric: rng.finite(),
                count: 4,
                iA: [
                    rng.below(4) as c_int,
                    rng.below(4) as c_int,
                    rng.below(4) as c_int,
                ],
                iB: [alias_idx, rng.below(4) as c_int, rng.below(4) as c_int],
                div: f32::from_bits(((i % 4) as u32) & 0x7),
            };
            let ur = (i % 2) as c_int;
            let mut cc = base;
            let mut cr = base;
            let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
            let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
            eq_gjk_out(&format!("row31 #{i}"), &oc, &or);
            eq_cache(&format!("row31 #{i} writeback"), &cc, &cr);
        }
    }
}

#[test]
fn row32_negative_cache_count() {
    let p = load_pair();
    let mut rng = Rng::new(0xC032);
    unsafe {
        for &count in &[-1i32, -2, -100, i32::MIN, -2147483647] {
            for i in 0..200 {
                let ta = ALL_TYPES[i % 3];
                let tb = ALL_TYPES[(i / 3) % 3];
                let ca = rng.v_coord();
                let sa = gen_shape(&mut rng, ta, ca, 4.0);
                let cb = rng.v_coord();
                let sb = gen_shape(&mut rng, tb, cb, 4.0);
                let base = c2GJKCache {
                    metric: rng.finite(),
                    count,
                    iA: [0, 0, 0],
                    iB: [0, 0, 0],
                    div: rng.finite(),
                };
                let ur = (i % 2) as c_int;
                let mut cc = base;
                let mut cr = base;
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row32 count={count} #{i}"), &oc, &or);
                eq_cache(&format!("row32 count={count} #{i} writeback"), &cc, &cr);
            }
        }
    }
}

#[test]
fn row33_cache_div_zero() {
    let p = load_pair();
    let mut rng = Rng::new(0xC033);
    unsafe {
        for &count in &[1i32, 2, 3] {
            for &div in &[0.0f32, -0.0] {
                for i in 0..200 {
                    let ta = ALL_TYPES[i % 3];
                    let tb = ALL_TYPES[(i / 3) % 3];
                    let ca = rng.v_coord();
                    let sa = gen_shape(&mut rng, ta, ca, 4.0);
                    let cb = rng.v_coord();
                    let sb = gen_shape(&mut rng, tb, cb, 4.0);
                    let na = match ta { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
                    let nb = match tb { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
                    let base = c2GJKCache {
                        metric: rng.finite(),
                        count,
                        iA: [
                            rng.below(na) as c_int,
                            rng.below(na) as c_int,
                            rng.below(na) as c_int,
                        ],
                        iB: [
                            rng.below(nb) as c_int,
                            rng.below(nb) as c_int,
                            rng.below(nb) as c_int,
                        ],
                        div,
                    };
                    let ur = (i % 2) as c_int;
                    let mut cc = base;
                    let mut cr = base;
                    let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
                    let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
                    eq_gjk_out(&format!("row33 count={count} div={div:e} #{i}"), &oc, &or);
                    eq_cache(&format!("row33 count={count} div={div:e} #{i} wb"), &cc, &cr);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 35-39 — use_radius branches and negative radii
// ===========================================================================

#[test]
fn row35_row36_row37_use_radius_values() {
    let p = load_pair();
    let mut rng = Rng::new(0xC035);
    // any non-zero int enables the shrink; check the awkward ones too
    let urs: [c_int; 8] = [0, 1, 2, -1, 0x100, i32::MIN, i32::MAX, -2147483647];
    unsafe {
        for i in 0..800 {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            // near-touching so dist straddles rA+rB and FLT_EPSILON
            let ca = rng.v_coord();
            let sa = gen_shape(&mut rng, ta, ca, 2.0);
            let cb = c2v { x: ca.x + rng.scaled(4.0), y: ca.y + rng.scaled(4.0) };
            let sb = gen_shape(&mut rng, tb, cb, 2.0);
            for &ur in &urs {
                let mut cc = c2GJKCache::default();
                let mut cr = c2GJKCache::default();
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row35-37 #{i} ur={ur}"), &oc, &or);
                eq_cache(&format!("row35-37 #{i} ur={ur} cache"), &cc, &cr);
            }
        }
    }
}

#[test]
fn row38_shrink_collapses_to_equal_points() {
    // Craft configurations where the post-shrink witness points coincide.
    let p = load_pair();
    let mut rng = Rng::new(0xC038);
    unsafe {
        for i in 0..N {
            let ca = rng.v_coord();
            let gap = 2.0f32 + rng.unit().abs() * 4.0;
            let r = gap * 0.5; // rA + rB == gap exactly-ish -> shrink lands on itself
            let sa = Shape::Circle(c2Circle { p: ca, r });
            let sb = Shape::Circle(c2Circle {
                p: c2v { x: ca.x + gap, y: ca.y },
                r,
            });
            for &ur in &[0i32, 1] {
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, None);
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, None);
                eq_gjk_out(&format!("row38 #{i} ur={ur}"), &oc, &or);
            }
        }
    }
}

#[test]
fn row39_negative_radii() {
    let p = load_pair();
    let mut rng = Rng::new(0xC039);
    unsafe {
        for i in 0..N {
            let ca = rng.v_coord();
            let cb = c2v { x: ca.x + rng.scaled(20.0), y: ca.y + rng.scaled(20.0) };
            let ra = -(rng.unit().abs() * 10.0);
            let rb = if i % 2 == 0 { -(rng.unit().abs() * 10.0) } else { rng.unit().abs() * 10.0 };
            let sa = if i % 3 == 0 {
                Shape::Circle(c2Circle { p: ca, r: ra })
            } else {
                Shape::Capsule(c2Capsule {
                    a: ca,
                    b: c2v { x: ca.x + rng.scaled(3.0), y: ca.y + rng.scaled(3.0) },
                    r: ra,
                })
            };
            let sb = Shape::Capsule(c2Capsule {
                a: cb,
                b: c2v { x: cb.x + rng.scaled(3.0), y: cb.y + rng.scaled(3.0) },
                r: rb,
            });
            for &ur in &[0i32, 1] {
                let mut cc = c2GJKCache::default();
                let mut cr = c2GJKCache::default();
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row39 #{i} ur={ur} ra={ra} rb={rb}"), &oc, &or);
                eq_cache(&format!("row39 #{i} cache"), &cc, &cr);
            }
        }
    }
}

// ===========================================================================
// Rows 40-44 — loop-exit conditions (hit / no-progress / collapsed d / dup / cap)
// ===========================================================================

#[test]
fn row40_row41_row42_row43_row44_loop_exits() {
    let p = load_pair();
    let mut rng = Rng::new(0xC040);
    let mut iter_hist_c = [0usize; 32];
    let mut iter_hist_r = [0usize; 32];
    let mut hits = 0usize;
    unsafe {
        for i in 0..(N * 3) {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            // mix of geometries so every break path is reached
            let ca = rng.v_coord();
            let mode = i % 6;
            let (sa, sb) = match mode {
                0 => (
                    gen_shape(&mut rng, ta, ca, 5.0),
                    gen_shape(&mut rng, tb, ca, 5.0),
                ), // overlapping -> hit
                1 => {
                    let sa = gen_shape(&mut rng, ta, ca, 2.0);
                    let cb = c2v { x: ca.x + 80.0, y: ca.y + 80.0 };
                    (sa, gen_shape(&mut rng, tb, cb, 2.0))
                } // far -> dup/no-progress
                2 => (
                    Shape::Aabb(c2AABB { min: ca, max: ca }),
                    gen_shape(&mut rng, tb, ca, 1.0),
                ), // zero extent -> collapsed d
                3 => (
                    Shape::Capsule(c2Capsule { a: ca, b: ca, r: 0.0 }),
                    Shape::Capsule(c2Capsule { a: ca, b: ca, r: 0.0 }),
                ), // identical degenerate
                4 => (
                    gen_shape(&mut rng, ta, ca, 1.0e-30),
                    gen_shape(&mut rng, tb, ca, 1.0e-30),
                ), // denormal scale
                _ => {
                    let sa = gen_shape(&mut rng, ta, ca, 1.0e18);
                    let cb = c2v { x: ca.x * 2.0, y: ca.y * 3.0 };
                    (sa, gen_shape(&mut rng, tb, cb, 1.0e18))
                } // huge scale
            };
            for &ur in &[0i32, 1] {
                let mut cc = c2GJKCache::default();
                let mut cr = c2GJKCache::default();
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row40-44 #{i} mode={mode} ur={ur}"), &oc, &or);
                eq_cache(&format!("row40-44 #{i} cache"), &cc, &cr);
                if (0..32).contains(&oc.iters) {
                    iter_hist_c[oc.iters as usize] += 1;
                    iter_hist_r[or.iters as usize] += 1;
                }
                if oc.dist == 0.0 && ur == 0 {
                    hits += 1;
                }
            }
        }
    }
    assert_eq!(
        iter_hist_c, iter_hist_r,
        "iteration-count distributions diverge: C={iter_hist_c:?} Rust={iter_hist_r:?}"
    );
    assert!(hits > 0, "row40: the hit/count==3 path was never reached");
    // several distinct iteration counts must have occurred, i.e. the different
    // break paths really were exercised
    let distinct = iter_hist_c.iter().filter(|&&n| n > 0).count();
    assert!(distinct >= 3, "only {distinct} distinct iteration counts: {iter_hist_c:?}");
}

// ===========================================================================
// Rows 45, 46 — degenerate / inverted AABBs, zero-length capsules
// ===========================================================================

#[test]
fn row45_row46_inverted_and_zero_extent_shapes() {
    let p = load_pair();
    let mut rng = Rng::new(0xC045);
    unsafe {
        for i in 0..N {
            let ca = rng.v_coord();
            let hx = rng.unit().abs() * 6.0;
            let hy = rng.unit().abs() * 6.0;
            let sa = match i % 4 {
                0 => Shape::Aabb(c2AABB {
                    min: c2v { x: ca.x + hx, y: ca.y + hy },
                    max: c2v { x: ca.x - hx, y: ca.y - hy },
                }), // fully inverted
                1 => Shape::Aabb(c2AABB {
                    min: c2v { x: ca.x + hx, y: ca.y - hy },
                    max: c2v { x: ca.x - hx, y: ca.y + hy },
                }), // one axis inverted
                2 => Shape::Aabb(c2AABB { min: ca, max: ca }),
                _ => Shape::Capsule(c2Capsule { a: ca, b: ca, r: 0.0 }),
            };
            let cb = c2v { x: ca.x + rng.scaled(12.0), y: ca.y + rng.scaled(12.0) };
            let sb = match (i / 4) % 3 {
                0 => Shape::Aabb(c2AABB {
                    min: c2v { x: cb.x + 2.0, y: cb.y + 2.0 },
                    max: c2v { x: cb.x - 2.0, y: cb.y - 2.0 },
                }),
                1 => Shape::Capsule(c2Capsule { a: cb, b: cb, r: 0.0 }),
                _ => Shape::Circle(c2Circle { p: cb, r: 0.0 }),
            };
            for &ur in &[0i32, 1] {
                let mut cc = c2GJKCache::default();
                let mut cr = c2GJKCache::default();
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row45/46 #{i} ur={ur}"), &oc, &or);
                eq_cache(&format!("row45/46 #{i} cache"), &cc, &cr);
            }
        }
    }
}

// ===========================================================================
// Rows 47, 48 — NaN/inf shape coordinates, non-unit rotations
// ===========================================================================

#[test]
fn row47_nan_inf_shape_coordinates() {
    let p = load_pair();
    let mut rng = Rng::new(0xC047);
    let nasty = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0, 1.0];
    unsafe {
        for i in 0..N {
            let pick = |k: usize| nasty[(i + k) % nasty.len()];
            let sa = match i % 3 {
                0 => Shape::Circle(c2Circle {
                    p: c2v { x: pick(0), y: pick(1) },
                    r: pick(2),
                }),
                1 => Shape::Aabb(c2AABB {
                    min: c2v { x: pick(0), y: pick(1) },
                    max: c2v { x: pick(2), y: pick(3) },
                }),
                _ => Shape::Capsule(c2Capsule {
                    a: c2v { x: pick(0), y: pick(1) },
                    b: c2v { x: pick(2), y: pick(3) },
                    r: pick(4),
                }),
            };
            let sb = match (i / 3) % 3 {
                0 => Shape::Circle(c2Circle { p: rng.v_coord(), r: pick(1) }),
                1 => Shape::Aabb(c2AABB {
                    min: c2v { x: pick(3), y: rng.coord() },
                    max: c2v { x: rng.coord(), y: pick(4) },
                }),
                _ => Shape::Capsule(c2Capsule {
                    a: rng.v_coord(),
                    b: c2v { x: pick(0), y: pick(5) },
                    r: rng.unit().abs(),
                }),
            };
            for &ur in &[0i32, 1] {
                let mut cc = c2GJKCache::default();
                let mut cr = c2GJKCache::default();
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row47 #{i} ur={ur}"), &oc, &or);
                eq_cache(&format!("row47 #{i} cache"), &cc, &cr);
            }
        }
    }
}

#[test]
fn row48_pathological_rotations() {
    let p = load_pair();
    let mut rng = Rng::new(0xC048);
    let rots = [
        c2r { c: 0.0, s: 0.0 },
        c2r { c: 1.0e18, s: -1.0e18 },
        c2r { c: f32::NAN, s: 1.0 },
        c2r { c: f32::INFINITY, s: 0.0 },
        c2r { c: -0.0, s: -0.0 },
        c2r { c: 3.0, s: 4.0 },
    ];
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let ca = rng.v_coord();
            let sa = gen_shape(&mut rng, ta, ca, 3.0);
            let cb = rng.v_coord();
            let sb = gen_shape(&mut rng, tb, cb, 3.0);
            let ax = c2x { p: rng.v_coord(), r: rots[i % rots.len()] };
            let bx = c2x { p: rng.v_coord(), r: rots[(i / rots.len()) % rots.len()] };
            for &ur in &[0i32, 1] {
                let mut cc = c2GJKCache::default();
                let mut cr = c2GJKCache::default();
                let oc = call_gjk(&p.c, &sa, Some(&ax), &sb, Some(&bx), ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, &sa, Some(&ax), &sb, Some(&bx), ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row48 #{i} ur={ur}"), &oc, &or);
                eq_cache(&format!("row48 #{i} cache"), &cc, &cr);
            }
        }
    }
}

// ===========================================================================
// Rows 49-59 — c22 / c23 degenerate branches
// ===========================================================================

#[test]
fn row49_row50_row51_c22_degenerate() {
    let p = load_pair();
    let mut rng = Rng::new(0xC049);
    let mut same_point = 0usize;
    let mut collapse_a = 0usize;
    let mut collapse_b = 0usize;
    let mut interior = 0usize;
    unsafe {
        for i in 0..(N * 2) {
            let mut sc = c2Simplex::default();
            for k in 0..4 {
                sc.verts[k] = c2sv {
                    sA: rng.v_coord(),
                    sB: rng.v_coord(),
                    p: rng.v_coord(),
                    u: rng.finite(),
                    iA: rng.below(4) as c_int,
                    iB: rng.below(4) as c_int,
                };
            }
            match i % 6 {
                0 => {
                    sc.verts[1].p = sc.verts[0].p; // a == b -> u == v == 0
                    same_point += 1;
                }
                1 => {
                    sc.verts[0].p = c2v { x: 0.0, y: 0.0 };
                    sc.verts[1].p = c2v { x: 0.0, y: 0.0 };
                }
                2 => {
                    // origin beyond a
                    let d = rng.v_coord();
                    sc.verts[0].p = d;
                    sc.verts[1].p = c2v { x: d.x * 2.0, y: d.y * 2.0 };
                }
                3 => {
                    // origin beyond b
                    let d = rng.v_coord();
                    sc.verts[0].p = c2v { x: d.x * 2.0, y: d.y * 2.0 };
                    sc.verts[1].p = d;
                }
                4 => {
                    // origin strictly between -> interior branch
                    let d = rng.v_coord();
                    sc.verts[0].p = d;
                    sc.verts[1].p = c2v { x: -d.x, y: -d.y };
                }
                _ => {
                    sc.verts[0].p = c2v { x: f32::NAN, y: rng.coord() };
                }
            }
            sc.div = rng.finite();
            sc.count = 2;
            let mut sr = sc;
            let a = sc.verts[0].p;
            let b = sc.verts[1].p;
            let u = (p.c.c2Dot)(b, (p.c.c2Sub)(b, a));
            let v = (p.c.c2Dot)(a, (p.c.c2Sub)(a, b));
            (p.c.c22)(&mut sc);
            (p.r.c22)(&mut sr);
            eq_simplex(&format!("row49-51 #{i}"), &sc, &sr);
            if v <= 0.0 {
                collapse_a += 1;
            } else if u <= 0.0 {
                collapse_b += 1;
            } else {
                interior += 1;
            }
        }
    }
    assert!(same_point > 0);
    assert!(collapse_a > 0 && collapse_b > 0 && interior > 0,
        "c22 branch coverage: a={collapse_a} b={collapse_b} interior={interior}");
}

#[test]
fn row52_to_row59_c23_all_branches_and_degenerate() {
    let p = load_pair();
    let mut rng = Rng::new(0xC052);
    let mut branch = [0usize; 7];
    unsafe {
        for i in 0..(N * 3) {
            let mut sc = c2Simplex::default();
            for k in 0..4 {
                sc.verts[k] = c2sv {
                    sA: rng.v_coord(),
                    sB: rng.v_coord(),
                    p: rng.v_coord(),
                    u: rng.finite(),
                    iA: rng.below(4) as c_int,
                    iB: rng.below(4) as c_int,
                };
            }
            match i % 9 {
                0 => {
                    // duplicate vertices -> area == 0 -> u/v/wABC all 0
                    sc.verts[1].p = sc.verts[0].p;
                    sc.verts[2].p = sc.verts[0].p;
                }
                1 => sc.verts[1].p = sc.verts[0].p,
                2 => sc.verts[2].p = sc.verts[1].p,
                3 => {
                    // exactly collinear
                    let a = sc.verts[0].p;
                    let d = c2v { x: rng.coord(), y: rng.coord() };
                    sc.verts[1].p = c2v { x: a.x + d.x, y: a.y + d.y };
                    sc.verts[2].p = c2v { x: a.x + 2.0 * d.x, y: a.y + 2.0 * d.y };
                }
                4 => {
                    // triangle containing the origin
                    sc.verts[0].p = c2v { x: -3.0 - rng.unit().abs(), y: -2.0 };
                    sc.verts[1].p = c2v { x: 4.0 + rng.unit().abs(), y: -1.5 };
                    sc.verts[2].p = c2v { x: 0.5, y: 5.0 + rng.unit().abs() };
                }
                5 => {
                    sc.verts[0].p = c2v { x: f32::NAN, y: 1.0 };
                }
                6 => {
                    sc.verts[2].p = c2v { x: f32::INFINITY, y: f32::NEG_INFINITY };
                }
                7 => {
                    sc.verts[0].p = c2v { x: 0.0, y: 0.0 };
                    sc.verts[1].p = c2v { x: 0.0, y: 0.0 };
                    sc.verts[2].p = c2v { x: 0.0, y: 0.0 };
                }
                _ => {}
            }
            sc.div = rng.finite();
            sc.count = 3;
            let mut sr = sc;

            // classify against the C's own predicates
            let a = sc.verts[0].p;
            let b = sc.verts[1].p;
            let c = sc.verts[2].p;
            let dot = p.c.c2Dot;
            let sub = p.c.c2Sub;
            let det = p.c.c2Det2;
            let u_ab = dot(b, sub(b, a));
            let v_ab = dot(a, sub(a, b));
            let u_bc = dot(c, sub(c, b));
            let v_bc = dot(b, sub(b, c));
            let u_ca = dot(a, sub(a, c));
            let v_ca = dot(c, sub(c, a));
            let area = det(sub(b, a), sub(c, a));
            let u_abc = det(b, c) * area;
            let v_abc = det(c, a) * area;
            let w_abc = det(a, b) * area;
            let idx = if v_ab <= 0.0 && u_ca <= 0.0 {
                0
            } else if u_ab <= 0.0 && v_bc <= 0.0 {
                1
            } else if u_bc <= 0.0 && v_ca <= 0.0 {
                2
            } else if u_ab > 0.0 && v_ab > 0.0 && w_abc <= 0.0 {
                3
            } else if u_bc > 0.0 && v_bc > 0.0 && u_abc <= 0.0 {
                4
            } else if u_ca > 0.0 && v_ca > 0.0 && v_abc <= 0.0 {
                5
            } else {
                6
            };
            branch[idx] += 1;

            (p.c.c23)(&mut sc);
            (p.r.c23)(&mut sr);
            eq_simplex(&format!("row52-59 #{i} branch={idx}"), &sc, &sr);
        }
    }
    for (k, &n) in branch.iter().enumerate() {
        assert!(n > 0, "c23 branch {k} never exercised: {branch:?}");
    }
}

// ===========================================================================
// Rows 60, 61, 62 — c2Maxv/c2Minv/c2Clampv with NaN and inverted ranges
// ===========================================================================

#[test]
fn row60_row61_row62_minmax_clamp_nan() {
    let p = load_pair();
    let mut rng = Rng::new(0xC060);
    let nasty = [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1.0,
        -1.0,
    ];
    unsafe {
        for (i, &ax) in nasty.iter().enumerate() {
            for (j, &by) in nasty.iter().enumerate() {
                for (k, &cz) in nasty.iter().enumerate() {
                    let a = c2v { x: ax, y: by };
                    let b = c2v { x: by, y: cz };
                    let lo = c2v { x: cz, y: ax };
                    let hi = c2v { x: ax, y: by };
                    let t = format!("[{i}][{j}][{k}]");
                    eq_v(&format!("row60 max {t}"), (p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b));
                    eq_v(&format!("row60 min {t}"), (p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b));
                    eq_v(
                        &format!("row62 clamp {t}"),
                        (p.c.c2Clampv)(a, lo, hi),
                        (p.r.c2Clampv)(a, lo, hi),
                    );
                }
            }
        }
        // inverted ranges, random
        for i in 0..N {
            let lo = rng.v_coord();
            let hi = c2v { x: lo.x - rng.unit().abs() * 10.0, y: lo.y - rng.unit().abs() * 10.0 };
            let a = rng.v();
            eq_v(
                &format!("row61 inverted #{i}"),
                (p.c.c2Clampv)(a, lo, hi),
                (p.r.c2Clampv)(a, lo, hi),
            );
        }
    }
}

// ===========================================================================
// Rows 63, 64, 65, 66 — the `gjk` wrapper's edges
// ===========================================================================

#[test]
fn row63_gjk_null_out_pointers() {
    let p = load_pair();
    let mut rng = Rng::new(0xC063);
    unsafe {
        for i in 0..N {
            let ctr = rng.v_coord();
            let a1 = ctr.x - 2.0;
            let a2 = ctr.y - 2.0;
            let a3 = ctr.x + 2.0;
            let a4 = ctr.y + 2.0;
            let b1 = ctr.x + rng.scaled(10.0);
            let b2 = ctr.y + rng.scaled(10.0);
            let b3 = b1 + rng.scaled(4.0);
            let b4 = b2 + rng.scaled(4.0);
            let b5 = rng.unit().abs() * 3.0;
            let rev = (i % 2) as c_char;
            for combo in 0..4 {
                let mut ac = c2v { x: 99.0, y: -99.0 };
                let mut bc = c2v { x: -55.0, y: 55.0 };
                let mut ar = ac;
                let mut br = bc;
                let pac = if combo & 1 != 0 { &mut ac as *mut c2v } else { std::ptr::null_mut() };
                let pbc = if combo & 2 != 0 { &mut bc as *mut c2v } else { std::ptr::null_mut() };
                let par = if combo & 1 != 0 { &mut ar as *mut c2v } else { std::ptr::null_mut() };
                let pbr = if combo & 2 != 0 { &mut br as *mut c2v } else { std::ptr::null_mut() };
                (p.c.gjk)(rev, pac, pbc, a1, a2, a3, a4, b1, b2, b3, b4, b5);
                (p.r.gjk)(rev, par, pbr, a1, a2, a3, a4, b1, b2, b3, b4, b5);
                eq_v(&format!("row63 #{i} combo={combo} a"), ac, ar);
                eq_v(&format!("row63 #{i} combo={combo} b"), bc, br);
                if combo & 1 == 0 {
                    eq_v(&format!("row63 #{i} a untouched"), c2v { x: 99.0, y: -99.0 }, ac);
                }
                if combo & 2 == 0 {
                    eq_v(&format!("row63 #{i} b untouched"), c2v { x: -55.0, y: 55.0 }, bc);
                }
            }
        }
    }
}

#[test]
fn row64_row65_reverse_char_truncation() {
    let p = load_pair();
    let mut rng = Rng::new(0xC064);
    // every distinct i8 bit pattern of interest, incl. the ones that truncate to 0
    let revs: Vec<c_char> = (0..=255u16)
        .map(|v| v as u8 as i8)
        .collect();
    unsafe {
        for i in 0..300 {
            let ctr = rng.v_coord();
            let a1 = ctr.x - 3.0;
            let a2 = ctr.y - 3.0;
            let a3 = ctr.x + 3.0;
            let a4 = ctr.y + 3.0;
            let b1 = ctr.x + rng.scaled(12.0);
            let b2 = ctr.y + rng.scaled(12.0);
            let b3 = b1 + rng.scaled(5.0);
            let b4 = b2 + rng.scaled(5.0);
            let b5 = rng.unit().abs() * 4.0;
            for &rev in &revs {
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let mut ar = c2v::default();
                let mut br = c2v::default();
                (p.c.gjk)(rev, &mut ac, &mut bc, a1, a2, a3, a4, b1, b2, b3, b4, b5);
                (p.r.gjk)(rev, &mut ar, &mut br, a1, a2, a3, a4, b1, b2, b3, b4, b5);
                eq_v(&format!("row64/65 #{i} rev={rev} a"), ac, ar);
                eq_v(&format!("row64/65 #{i} rev={rev} b"), bc, br);
            }
        }
    }
}

#[test]
fn row66_gjk_pathological_floats() {
    let p = load_pair();
    let nasty = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -1.0,
        1.0,
    ];
    let mut rng = Rng::new(0xC066);
    unsafe {
        for i in 0..(N * 2) {
            let mut f = [0.0f32; 9];
            for (k, slot) in f.iter_mut().enumerate() {
                *slot = if (i + k) % 3 == 0 {
                    nasty[(i + k) % nasty.len()]
                } else {
                    rng.finite()
                };
            }
            for &rev in &[0i8, 1] {
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let mut ar = c2v::default();
                let mut br = c2v::default();
                (p.c.gjk)(rev, &mut ac, &mut bc, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8]);
                (p.r.gjk)(rev, &mut ar, &mut br, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8]);
                eq_v(&format!("row66 #{i} rev={rev} a"), ac, ar);
                eq_v(&format!("row66 #{i} rev={rev} b"), bc, br);
            }
        }
    }
}

// ===========================================================================
// Rows 67, 68 — overflow in c2Dot/c2Det2; c2BBVerts with an inverted box
// ===========================================================================

#[test]
fn row67_dot_det_overflow() {
    let p = load_pair();
    let mut rng = Rng::new(0xC067);
    let huge = [
        f32::MAX,
        f32::MIN,
        1.0e30f32,
        -1.0e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    unsafe {
        for (i, &x) in huge.iter().enumerate() {
            for (j, &y) in huge.iter().enumerate() {
                for (k, &z) in huge.iter().enumerate() {
                    let a = c2v { x, y };
                    let b = c2v { x: y, y: z };
                    let t = format!("[{i}][{j}][{k}]");
                    eq_f32(&format!("row67 dot {t}"), (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
                    eq_f32(&format!("row67 det {t}"), (p.c.c2Det2)(a, b), (p.r.c2Det2)(a, b));
                    eq_v(&format!("row67 add {t}"), (p.c.c2Add)(a, b), (p.r.c2Add)(a, b));
                    eq_v(&format!("row67 sub {t}"), (p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b));
                    eq_v(&format!("row67 mulrv {t}"), (p.c.c2Mulrv)(c2r { c: x, s: y }, b), (p.r.c2Mulrv)(c2r { c: x, s: y }, b));
                    eq_v(&format!("row67 mulrvT {t}"), (p.c.c2MulrvT)(c2r { c: x, s: y }, b), (p.r.c2MulrvT)(c2r { c: x, s: y }, b));
                }
            }
        }
        for i in 0..N {
            let a = c2v { x: rng.scaled(1.0e35), y: rng.scaled(1.0e35) };
            let b = c2v { x: rng.scaled(1.0e35), y: rng.scaled(1.0e35) };
            eq_f32(&format!("row67 rand dot #{i}"), (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
            eq_f32(&format!("row67 rand det #{i}"), (p.c.c2Det2)(a, b), (p.r.c2Det2)(a, b));
        }
    }
}

#[test]
fn row68_bbverts_inverted_and_nonfinite() {
    let p = load_pair();
    let mut rng = Rng::new(0xC068);
    let nasty = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0, 1.0e30, -1.0e30];
    unsafe {
        for (i, &x) in nasty.iter().enumerate() {
            for (j, &y) in nasty.iter().enumerate() {
                let mut bb = c2AABB {
                    min: c2v { x, y },
                    max: c2v { x: y, y: x },
                };
                let mut oc = [c2v { x: 3.5, y: -3.5 }; 4];
                let mut orr = oc;
                (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bb);
                (p.r.c2BBVerts)(orr.as_mut_ptr(), &mut bb);
                for k in 0..4 {
                    eq_v(&format!("row68 [{i}][{j}][{k}]"), oc[k], orr[k]);
                }
            }
        }
        for i in 0..N {
            let m = rng.v();
            let mut bb = c2AABB {
                min: c2v { x: m.x + rng.unit().abs() * 5.0, y: m.y + rng.unit().abs() * 5.0 },
                max: m,
            };
            let mut oc = [c2v { x: 1.0, y: 2.0 }; 4];
            let mut orr = oc;
            (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bb);
            (p.r.c2BBVerts)(orr.as_mut_ptr(), &mut bb);
            for k in 0..4 {
                eq_v(&format!("row68 rand #{i}[{k}]"), oc[k], orr[k]);
            }
        }
    }
}
