//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! This library has no error enum and no negative sentinel: every rejection is
//! either an explicit `default: return 0`, a predicate returning `0`, a
//! null-pointer guard that substitutes a default, or a loop bail-out. Each test
//! constructs the exact triggering condition and asserts BOTH builds produce the
//! SAME specific result (the same `0`, the same sentinel vector, the same
//! `iterations` value) — not merely "both rejected somehow".
//!
//! Rows 41 and 25 are undefined behaviour in the C (a missing `return` and an
//! uninitialised read); they are tested for the property that IS defined, and
//! the UB boundary is measured explicitly rather than assumed.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

// ===========================================================================
// Row 1 — c2MakeProxy with a type matching no case (no `default:`)
// ===========================================================================

#[test]
fn err_row01_makeproxy_no_default_writes_nothing() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 101);
    for &bad in &BAD_TYPES {
        let buf = ShapeBuf::from_capsule(g.capsule());
        // Sentinel-filled destination: if either build wrote anything, it shows.
        let sentinel = c2Proxy {
            radius: -98.75,
            count: 1234,
            verts: [c2v { x: 5.5, y: -6.5 }; 8],
        };
        let mut cp = sentinel;
        let mut rp = sentinel;
        unsafe { (c.c2MakeProxy)(buf.as_ptr(), bad, &mut cp) };
        unsafe { (r.c2MakeProxy)(buf.as_ptr(), bad, &mut rp) };
        eq_proxy(&format!("row01 type={bad}"), &cp, &rp);
        eq_bytes(&format!("row01 type={bad} identical bytes"), &cp, &rp);
        eq_bytes(&format!("row01 type={bad} untouched"), &cp, &sentinel);
    }
}

// ===========================================================================
// Row 2 — c2GJKSimplexMetric `default:` falls through to `case 1:` => 0
// ===========================================================================

#[test]
fn err_row02_simplex_metric_default_is_zero() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 102);
    for &count in &[0i32, 1, 4, 5, 6, -1, -7, 999, i32::MAX, i32::MIN] {
        for _ in 0..64 {
            let mut s = c2Simplex {
                verts: [
                    g.simplex_vert(),
                    g.simplex_vert(),
                    g.simplex_vert(),
                    g.simplex_vert(),
                ],
                div: g.coord(),
                count,
            };
            let mut s2 = s;
            let cv = unsafe { (c.c2GJKSimplexMetric)(&mut s) };
            let rv = unsafe { (r.c2GJKSimplexMetric)(&mut s2) };
            eq_f32_bits(&format!("row02 count={count}"), cv, rv);
            eq_f32_bits(&format!("row02 count={count} is +0.0"), cv, 0.0);
        }
    }
}

// ===========================================================================
// Row 3 — c2D: `case 3:` and `default:` share the `c2V(0,0)` body
// ===========================================================================

#[test]
fn err_row03_c2D_default_is_zero_vector() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 103);
    for &count in &[3i32, 0, 4, 5, -1, 999, i32::MAX, i32::MIN] {
        for _ in 0..64 {
            let mut s = c2Simplex {
                verts: [
                    g.simplex_vert(),
                    g.simplex_vert(),
                    g.simplex_vert(),
                    g.simplex_vert(),
                ],
                div: g.coord(),
                count,
            };
            let mut s2 = s;
            let cv = unsafe { (c.c2D)(&mut s) };
            let rv = unsafe { (r.c2D)(&mut s2) };
            eq_v(&format!("row03 count={count}"), cv, rv);
            eq_f32_bits(&format!("row03 count={count} x is +0.0"), cv.x, 0.0);
            eq_f32_bits(&format!("row03 count={count} y is +0.0"), cv.y, 0.0);
        }
    }
}

// ===========================================================================
// Row 4 — c2Witness `default:` writes (0,0) to both outputs
// ===========================================================================

#[test]
fn err_row04_witness_default_writes_zero_vectors() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 104);
    for &count in &[0i32, 4, 5, -1, 999, i32::MAX, i32::MIN] {
        for _ in 0..64 {
            let mut s = c2Simplex {
                verts: [
                    g.simplex_vert(),
                    g.simplex_vert(),
                    g.simplex_vert(),
                    g.simplex_vert(),
                ],
                div: g.coord(),
                count,
            };
            let mut s2 = s;
            let mut ca = c2v { x: 9.5, y: -9.5 };
            let mut cb = c2v { x: -8.25, y: 8.25 };
            let mut ra = ca;
            let mut rb = cb;
            unsafe { (c.c2Witness)(&mut s, &mut ca, &mut cb) };
            unsafe { (r.c2Witness)(&mut s2, &mut ra, &mut rb) };
            eq_v(&format!("row04 count={count} outA"), ca, ra);
            eq_v(&format!("row04 count={count} outB"), cb, rb);
            eq_f32_bits(&format!("row04 count={count} outA.x is +0.0"), ca.x, 0.0);
            eq_f32_bits(&format!("row04 count={count} outB.y is +0.0"), cb.y, 0.0);
        }
    }
}

// ===========================================================================
// Row 5 — c2L `default:` => (0,0)
// ===========================================================================

#[test]
fn err_row05_c2L_default_is_zero_vector() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 105);
    for &count in &[0i32, 3, 4, 5, -1, 999, i32::MAX, i32::MIN] {
        for _ in 0..64 {
            let mut s = c2Simplex {
                verts: [
                    g.simplex_vert(),
                    g.simplex_vert(),
                    g.simplex_vert(),
                    g.simplex_vert(),
                ],
                div: g.coord(),
                count,
            };
            let mut s2 = s;
            let cv = unsafe { (c.c2L)(&mut s) };
            let rv = unsafe { (r.c2L)(&mut s2) };
            eq_v(&format!("row05 count={count}"), cv, rv);
            eq_f32_bits(&format!("row05 count={count} x is +0.0"), cv.x, 0.0);
            eq_f32_bits(&format!("row05 count={count} y is +0.0"), cv.y, 0.0);
        }
    }
}

// ===========================================================================
// Rows 6-8 — c2Support: count <= 0, exact ties, NaN direction => index 0
// ===========================================================================

#[test]
fn err_rows06_08_support_returns_zero() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 106);
    let mut verts = [c2v::default(); 8];
    for v in verts.iter_mut() {
        *v = g.v();
    }

    // Row 6: count <= 0. verts[0] is read before the loop guard, so the array
    // must still be valid, but the loop never runs.
    for &count in &[0i32, -1, -2, -1000, i32::MIN] {
        let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, c2v { x: 1.0, y: 1.0 }) };
        let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, c2v { x: 1.0, y: 1.0 }) };
        eq_int(&format!("row06 count={count}"), cv, rv);
        eq_int(&format!("row06 count={count} is 0"), cv, 0);
    }

    // Row 7: exact ties (all verts identical) — strict `>` never fires.
    let same = [c2v { x: 3.0, y: -4.0 }; 8];
    for &count in &[1i32, 2, 3, 4, 8] {
        for _ in 0..64 {
            let d = g.v();
            let cv = unsafe { (c.c2Support)(same.as_ptr(), count, d) };
            let rv = unsafe { (r.c2Support)(same.as_ptr(), count, d) };
            eq_int(&format!("row07 ties count={count}"), cv, rv);
            eq_int(&format!("row07 ties count={count} is 0"), cv, 0);
        }
    }
    // Zero direction: every dot is 0, so no strict improvement.
    for &count in &[1i32, 2, 4, 8] {
        let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, c2v { x: 0.0, y: 0.0 }) };
        let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, c2v { x: 0.0, y: 0.0 }) };
        eq_int(&format!("row07 zero-dir count={count}"), cv, rv);
        eq_int(&format!("row07 zero-dir count={count} is 0"), cv, 0);
    }

    // Row 8: NaN direction — every `dot > dmax` is false.
    for &d in &[
        c2v {
            x: f32::NAN,
            y: f32::NAN,
        },
        c2v {
            x: f32::NAN,
            y: 0.0,
        },
        c2v {
            x: 0.0,
            y: f32::NAN,
        },
    ] {
        for &count in &[1i32, 2, 4, 8] {
            let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
            let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
            eq_int(&format!("row08 nan-dir count={count}"), cv, rv);
            eq_int(&format!("row08 nan-dir count={count} is 0"), cv, 0);
        }
    }
}

// ===========================================================================
// Rows 9-14 — every null-pointer guard in c2GJK, individually
// ===========================================================================

struct GjkArgs {
    ax: Option<c2x>,
    bx: Option<c2x>,
    outA: bool,
    outB: bool,
    iters: bool,
    cache: Option<c2GJKCache>,
    use_radius: c_int,
}

fn gjk(
    api: &Api,
    a: &ShapeBuf,
    ta: C2_TYPE,
    b: &ShapeBuf,
    tb: C2_TYPE,
    args: &GjkArgs,
) -> (f32, c2v, c2v, c_int, Option<c2GJKCache>) {
    let mut oa = c2v { x: 77.0, y: -77.0 };
    let mut ob = c2v { x: -88.0, y: 88.0 };
    let mut it: c_int = -999;
    let mut cache = args.cache;
    let ax = args.ax;
    let bx = args.bx;
    let dist = unsafe {
        (api.c2GJK)(
            a.as_ptr(),
            ta,
            ax.as_ref().map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            b.as_ptr(),
            tb,
            bx.as_ref().map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            if args.outA { &mut oa } else { std::ptr::null_mut() },
            if args.outB { &mut ob } else { std::ptr::null_mut() },
            args.use_radius,
            if args.iters { &mut it } else { std::ptr::null_mut() },
            cache
                .as_mut()
                .map(|c| c as *mut c2GJKCache)
                .unwrap_or(std::ptr::null_mut()),
        )
    };
    (dist, oa, ob, it, cache)
}

/// Rows 9-10 — a NULL transform must behave EXACTLY like an explicitly passed
/// `c2xIdentity()`, on both builds. That is the specific documented substitution,
/// so asserting "both agree" alone would be too weak.
#[test]
fn err_rows09_10_null_transforms_equal_identity() {
    let (c, r) = apis();
    let ident = (c.c2xIdentity)();
    let mut g = Rng::new(SEED ^ 109);
    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            for ur in [0, 1] {
                for i in 0..200 {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let combos = [
                        (None, None),
                        (Some(ident), None),
                        (None, Some(ident)),
                        (Some(ident), Some(ident)),
                    ];
                    let mut results = Vec::new();
                    for api in [c, r] {
                        for (ax, bx) in combos {
                            let args = GjkArgs {
                                ax,
                                bx,
                                outA: true,
                                outB: true,
                                iters: true,
                                cache: None,
                                use_radius: ur,
                            };
                            results.push(gjk(api, &a, ta, &b, tb, &args));
                        }
                    }
                    // All 4 C variants identical, all 4 Rust variants identical,
                    // and C == Rust.
                    for k in 0..8 {
                        let ctx = format!(
                            "row09/10 #{i} {}/{} ur={ur} variant{k}",
                            type_name(ta),
                            type_name(tb)
                        );
                        eq_f32(&format!("{ctx} dist"), results[0].0, results[k].0);
                        eq_v(&format!("{ctx} outA"), results[0].1, results[k].1);
                        eq_v(&format!("{ctx} outB"), results[0].2, results[k].2);
                        eq_int(&format!("{ctx} iters"), results[0].3, results[k].3);
                    }
                }
            }
        }
    }
}

/// Rows 11-13 — a NULL `outA` / `outB` / `iterations` must skip the store and
/// leave the return value unchanged.
#[test]
fn err_rows11_13_null_out_params_skip_stores() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 111);
    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            for ur in [0, 1] {
                for i in 0..200 {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let full = GjkArgs {
                        ax: None,
                        bx: None,
                        outA: true,
                        outB: true,
                        iters: true,
                        cache: None,
                        use_radius: ur,
                    };
                    let (cd, _, _, _, _) = gjk(c, &a, ta, &b, tb, &full);
                    let (rd, _, _, _, _) = gjk(r, &a, ta, &b, tb, &full);
                    let ctx = format!("row11-13 #{i} {}/{} ur={ur}", type_name(ta), type_name(tb));
                    eq_f32(&format!("{ctx} full dist"), cd, rd);

                    for (oa, ob, oi) in [
                        (false, true, true),
                        (true, false, true),
                        (true, true, false),
                        (false, false, false),
                    ] {
                        let args = GjkArgs {
                            ax: None,
                            bx: None,
                            outA: oa,
                            outB: ob,
                            iters: oi,
                            cache: None,
                            use_radius: ur,
                        };
                        let (cd2, coa, cob, cit, _) = gjk(c, &a, ta, &b, tb, &args);
                        let (rd2, roa, rob, rit, _) = gjk(r, &a, ta, &b, tb, &args);
                        eq_f32(&format!("{ctx} partial dist"), cd2, rd2);
                        // The return value must not depend on out-param presence.
                        eq_f32(&format!("{ctx} dist unaffected by nulls"), cd, cd2);
                        eq_f32(&format!("{ctx} rust dist unaffected"), rd, rd2);
                        // Skipped stores leave the sentinel in place, identically.
                        eq_v(&format!("{ctx} outA"), coa, roa);
                        eq_v(&format!("{ctx} outB"), cob, rob);
                        eq_int(&format!("{ctx} iters"), cit, rit);
                        if !oa {
                            eq_f32_bits(&format!("{ctx} outA sentinel kept"), coa.x, 77.0);
                            eq_f32_bits(&format!("{ctx} rust outA sentinel kept"), roa.x, 77.0);
                        }
                        if !ob {
                            eq_f32_bits(&format!("{ctx} outB sentinel kept"), cob.x, -88.0);
                            eq_f32_bits(&format!("{ctx} rust outB sentinel kept"), rob.x, -88.0);
                        }
                        if !oi {
                            eq_int(&format!("{ctx} iters sentinel kept"), cit, -999);
                            eq_int(&format!("{ctx} rust iters sentinel kept"), rit, -999);
                        }
                    }
                }
            }
        }
    }
}

/// Row 14 — a NULL cache skips BOTH the warm-start read and the write-back, and
/// must give the same answer as a cold (`count == 0`) cache.
#[test]
fn err_row14_null_cache_matches_cold_cache() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 114);
    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            for ur in [0, 1] {
                for i in 0..200 {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let ctx = format!("row14 #{i} {}/{} ur={ur}", type_name(ta), type_name(tb));
                    let no_cache = GjkArgs {
                        ax: None,
                        bx: None,
                        outA: true,
                        outB: true,
                        iters: true,
                        cache: None,
                        use_radius: ur,
                    };
                    let cold = GjkArgs {
                        cache: Some(c2GJKCache::default()),
                        ..no_cache
                    };
                    let (cd0, ca0, cb0, ci0, cch0) = gjk(c, &a, ta, &b, tb, &no_cache);
                    let (rd0, ra0, rb0, ri0, rch0) = gjk(r, &a, ta, &b, tb, &no_cache);
                    assert!(cch0.is_none() && rch0.is_none());
                    eq_f32(&format!("{ctx} null-cache dist"), cd0, rd0);
                    eq_v(&format!("{ctx} null-cache outA"), ca0, ra0);
                    eq_v(&format!("{ctx} null-cache outB"), cb0, rb0);
                    eq_int(&format!("{ctx} null-cache iters"), ci0, ri0);

                    let (cd1, _, _, ci1, cch1) = gjk(c, &a, ta, &b, tb, &cold);
                    let (rd1, _, _, ri1, rch1) = gjk(r, &a, ta, &b, tb, &cold);
                    eq_f32(&format!("{ctx} cold-cache dist"), cd1, rd1);
                    eq_int(&format!("{ctx} cold-cache iters"), ci1, ri1);
                    eq_cache(
                        &format!("{ctx} cold-cache writeback"),
                        &cch1.unwrap(),
                        &rch1.unwrap(),
                    );
                    // A cold cache must not change the answer vs no cache.
                    eq_f32(&format!("{ctx} cold == null"), cd0, cd1);
                    eq_f32(&format!("{ctx} rust cold == null"), rd0, rd1);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 15-16 — the cache-acceptance guard
// ===========================================================================

/// Row 15 — `cache->count == 0` makes `cache_was_good` false, forcing the cold
/// start (`count=1`, `div=1`) regardless of the rest of the struct's contents.
#[test]
fn err_row15_zero_count_cache_forces_cold_start() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 115);
    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            for i in 0..200 {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                // Garbage everywhere EXCEPT count, which is 0.
                let junk = c2GJKCache {
                    metric: g.wild(),
                    count: 0,
                    iA: [7, 6, 5],
                    iB: [5, 6, 7],
                    div: g.wild(),
                };
                let args = GjkArgs {
                    ax: None,
                    bx: None,
                    outA: true,
                    outB: true,
                    iters: true,
                    cache: Some(junk),
                    use_radius: 1,
                };
                let clean = GjkArgs {
                    cache: Some(c2GJKCache::default()),
                    ..args
                };
                let ctx = format!("row15 #{i} {}/{}", type_name(ta), type_name(tb));
                let (cd, _, _, ci, cch) = gjk(c, &a, ta, &b, tb, &args);
                let (rd, _, _, ri, rch) = gjk(r, &a, ta, &b, tb, &args);
                eq_f32(&format!("{ctx} dist"), cd, rd);
                eq_int(&format!("{ctx} iters"), ci, ri);
                eq_cache(&format!("{ctx} cache"), &cch.unwrap(), &rch.unwrap());
                // The junk fields must have had no effect: same as a zeroed cache.
                let (cd2, _, _, _, _) = gjk(c, &a, ta, &b, tb, &clean);
                eq_f32(&format!("{ctx} junk fields ignored (C)"), cd, cd2);
                let (rd2, _, _, _, _) = gjk(r, &a, ta, &b, tb, &clean);
                eq_f32(&format!("{ctx} junk fields ignored (Rust)"), rd, rd2);
            }
        }
    }
}

/// Row 16 — the acceptance guard `!(min_metric < max_metric*2 && metric < -1e8)`.
/// Because of the `metric < -1.0e8f` conjunct, an ordinary metric ALWAYS makes
/// the condition false, so `cache_was_read` is set and the cached simplex is
/// kept. Driven with metrics on both sides of `-1e8` (including `NaN`, where
/// every comparison is false) so both outcomes of the guard are reached.
#[test]
fn err_row16_cache_metric_guard_both_outcomes() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 116);
    let metrics = [
        0.0f32,
        1.0,
        -1.0,
        -1e7,
        -9.999e7,
        -1.0e8,
        -1.000_1e8,
        -1e9,
        -1e30,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::NAN,
    ];
    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            let na = match ta {
                C2_TYPE_CIRCLE => 1u32,
                C2_TYPE_CAPSULE => 2,
                _ => 4,
            };
            let nb = match tb {
                C2_TYPE_CIRCLE => 1u32,
                C2_TYPE_CAPSULE => 2,
                _ => 4,
            };
            for &metric in &metrics {
                for count in 1..=3i32 {
                    for &div in &[1.0f32, 0.0, -2.5, 1e9] {
                        for _ in 0..8 {
                            let cache = c2GJKCache {
                                metric,
                                count,
                                iA: [
                                    g.below(na) as c_int,
                                    g.below(na) as c_int,
                                    g.below(na) as c_int,
                                ],
                                iB: [
                                    g.below(nb) as c_int,
                                    g.below(nb) as c_int,
                                    g.below(nb) as c_int,
                                ],
                                div,
                            };
                            let args = GjkArgs {
                                ax: None,
                                bx: None,
                                outA: true,
                                outB: true,
                                iters: true,
                                cache: Some(cache),
                                use_radius: 1,
                            };
                            let a = rand_shape(&mut g, ta);
                            let b = rand_shape(&mut g, tb);
                            let ctx = format!(
                                "row16 {}/{} metric={metric:?} count={count} div={div}",
                                type_name(ta),
                                type_name(tb)
                            );
                            let (cd, coa, cob, ci, cch) = gjk(c, &a, ta, &b, tb, &args);
                            let (rd, roa, rob, ri, rch) = gjk(r, &a, ta, &b, tb, &args);
                            eq_f32(&format!("{ctx} dist"), cd, rd);
                            eq_v(&format!("{ctx} outA"), coa, roa);
                            eq_v(&format!("{ctx} outB"), cob, rob);
                            eq_int(&format!("{ctx} iters"), ci, ri);
                            eq_cache(&format!("{ctx} cache"), &cch.unwrap(), &rch.unwrap());
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Rows 17-21 — the GJK loop bail-outs
// ===========================================================================

/// Rows 17-21 — every loop exit: the `iter == 20` budget, the `d1 > d0`
/// non-decrease, the `dot(d,d) < eps^2` degenerate direction, the duplicate
/// support point, and the `s.count == 3` hit. `iterations` is the externally
/// visible witness of which exit fired, so this test asserts C and Rust agree on
/// it over a wide corpus AND reports the distribution so no exit is silently
/// unexercised.
#[test]
fn err_rows17_21_gjk_loop_bailouts() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 117);
    let mut iter_hist = std::collections::BTreeMap::<c_int, usize>::new();
    let mut hits = 0usize;
    let mut cases = 0usize;

    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            for ur in [0, 1] {
                for _ in 0..2000 {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let args = GjkArgs {
                        ax: None,
                        bx: None,
                        outA: true,
                        outB: true,
                        iters: true,
                        cache: None,
                        use_radius: ur,
                    };
                    let (cd, coa, cob, ci, _) = gjk(c, &a, ta, &b, tb, &args);
                    let (rd, roa, rob, ri, _) = gjk(r, &a, ta, &b, tb, &args);
                    eq_f32("row17-21 dist", cd, rd);
                    eq_v("row17-21 outA", coa, roa);
                    eq_v("row17-21 outB", cob, rob);
                    eq_int("row17-21 iterations", ci, ri);
                    *iter_hist.entry(ci).or_insert(0) += 1;
                    // Row 21: the `hit` path forces dist == 0 and a == b.
                    if cd == 0.0
                        && coa.x.to_bits() == cob.x.to_bits()
                        && coa.y.to_bits() == cob.y.to_bits()
                    {
                        hits += 1;
                    }
                    cases += 1;
                }
            }
        }
    }
    println!("row17-21 iteration histogram: {iter_hist:?}");
    println!("row17-21 hit/collapse cases: {hits}/{cases}");
    // The loop must be exercised over a range of iteration counts, and the
    // count is never allowed past the C's cap of 20.
    let max_iter = *iter_hist.keys().max().unwrap();
    assert!(
        iter_hist.len() >= 3,
        "row17-21: too few distinct iteration counts observed: {iter_hist:?}"
    );
    assert!(
        max_iter <= 20,
        "row17: iterations exceeded the C's cap of 20 ({max_iter})"
    );
    assert!(hits > 0, "row21: the hit path was never reached");

    // Row 17 specifically: drive toward the iteration cap with shapes that make
    // the simplex churn — many-vertex proxies under wild transforms.
    let mut max_seen = 0;
    for _ in 0..40_000 {
        let a = rand_shape(&mut g, C2_TYPE_AABB);
        let b = rand_shape(&mut g, C2_TYPE_AABB);
        let ax = g.xform();
        let bx = g.xform();
        let args = GjkArgs {
            ax: Some(ax),
            bx: Some(bx),
            outA: true,
            outB: true,
            iters: true,
            cache: None,
            use_radius: 1,
        };
        let (cd, coa, cob, ci, _) = gjk(c, &a, C2_TYPE_AABB, &b, C2_TYPE_AABB, &args);
        let (rd, roa, rob, ri, _) = gjk(r, &a, C2_TYPE_AABB, &b, C2_TYPE_AABB, &args);
        eq_f32("row17 dist", cd, rd);
        eq_v("row17 outA", coa, roa);
        eq_v("row17 outB", cob, rob);
        eq_int("row17 iterations", ci, ri);
        max_seen = max_seen.max(ci);
    }
    println!("row17 max iterations observed under wild transforms: {max_seen}");
}

// ===========================================================================
// Rows 22-24 — the use_radius block
// ===========================================================================

/// Row 22 — `use_radius != 0` and NOT (`dist > rA+rB && dist > eps`): both
/// witnesses collapse to the midpoint and `dist` is forced to exactly `+0.0`.
/// Row 23 — the shrink branch where `a == b` afterwards also forces `dist = 0`.
/// Row 24 — `use_radius == 0` ignores radii entirely.
#[test]
fn err_rows22_24_use_radius_block() {
    let (c, r) = apis();
    let mut collapsed = 0usize;
    let mut shrunk = 0usize;

    // Circles make the radius arithmetic exact and easy to straddle.
    for k in 0..400i32 {
        let ra = 1.0f32;
        let rb = 0.5f32;
        // Sweep the centre distance across rA + rB.
        let d = 1.0 + k as f32 * 0.01;
        let a = ShapeBuf::from_circle(c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: ra,
        });
        let b = ShapeBuf::from_circle(c2Circle {
            p: c2v { x: d, y: 0.0 },
            r: rb,
        });
        for ur in [0, 1] {
            let args = GjkArgs {
                ax: None,
                bx: None,
                outA: true,
                outB: true,
                iters: true,
                cache: None,
                use_radius: ur,
            };
            let (cd, coa, cob, ci, _) = gjk(c, &a, C2_TYPE_CIRCLE, &b, C2_TYPE_CIRCLE, &args);
            let (rd, roa, rob, ri, _) = gjk(r, &a, C2_TYPE_CIRCLE, &b, C2_TYPE_CIRCLE, &args);
            let ctx = format!("row22-24 d={d} ur={ur}");
            eq_f32(&format!("{ctx} dist"), cd, rd);
            eq_v(&format!("{ctx} outA"), coa, roa);
            eq_v(&format!("{ctx} outB"), cob, rob);
            eq_int(&format!("{ctx} iters"), ci, ri);
            if ur == 1 {
                if cd == 0.0 {
                    collapsed += 1;
                    // Row 22: the midpoint collapse makes a == b exactly.
                    eq_f32_bits(&format!("{ctx} collapse a.x==b.x"), coa.x, cob.x);
                    eq_f32_bits(&format!("{ctx} collapse a.y==b.y"), coa.y, cob.y);
                } else {
                    shrunk += 1;
                }
            } else {
                // Row 24: with use_radius == 0 the distance is the raw core
                // distance, so radii cannot have been subtracted.
                assert!(
                    cd >= 0.0,
                    "{ctx}: core distance must be non-negative, got {cd}"
                );
            }
        }
    }
    println!("row22-24: collapsed={collapsed} shrunk={shrunk}");
    assert!(collapsed > 0, "row22: the midpoint-collapse branch was never taken");
    assert!(shrunk > 0, "row23: the radius-shrink branch was never taken");

    // Row 24 explicitly: with use_radius == 0 the answer must not change when
    // only the radii change (an AABB proxy already has radius 0; circles and
    // capsules must be ignored too).
    let mut g = Rng::new(SEED ^ 124);
    for &t in &[C2_TYPE_CIRCLE, C2_TYPE_CAPSULE] {
        for _ in 0..2000 {
            let p1 = g.v();
            let p2 = g.v();
            let mk = |rad: f32| -> ShapeBuf {
                if t == C2_TYPE_CIRCLE {
                    ShapeBuf::from_circle(c2Circle { p: p1, r: rad })
                } else {
                    ShapeBuf::from_capsule(c2Capsule {
                        a: p1,
                        b: p2,
                        r: rad,
                    })
                }
            };
            let other = ShapeBuf::from_aabb(g.aabb());
            let args = GjkArgs {
                ax: None,
                bx: None,
                outA: true,
                outB: true,
                iters: true,
                cache: None,
                use_radius: 0,
            };
            let s0 = mk(0.0);
            let s1 = mk(5.0);
            let (d0c, _, _, _, _) = gjk(c, &s0, t, &other, C2_TYPE_AABB, &args);
            let (d1c, _, _, _, _) = gjk(c, &s1, t, &other, C2_TYPE_AABB, &args);
            let (d0r, _, _, _, _) = gjk(r, &s0, t, &other, C2_TYPE_AABB, &args);
            let (d1r, _, _, _, _) = gjk(r, &s1, t, &other, C2_TYPE_AABB, &args);
            eq_f32("row24 C ignores radius", d0c, d1c);
            eq_f32("row24 Rust ignores radius", d0r, d1r);
            eq_f32("row24 C==Rust r=0", d0c, d0r);
            eq_f32("row24 C==Rust r=5", d1c, d1r);
        }
    }
}

// ===========================================================================
// Row 25 — unchecked cache indices (see also level3_gjk.rs)
// ===========================================================================

/// Row 25 — `cache->count` is never checked against `iA[3]`/`iB[3]`, and the
/// cached indices are never checked against the proxy's vertex count.
///
/// The DEFINED part of this row — `count` in `1..=3` with indices inside the
/// initialised vertex range — is asserted here bit-for-bit (and exhaustively in
/// `level3_gjk.rs` rows 64). Two sub-cases are genuinely undefined in the C and
/// are therefore measured in isolation rather than asserted:
///
/// * an index at or beyond the proxy's vertex count reads uninitialised
///   `c2Proxy.verts[]` stack memory → `level3_gjk.rs::row64_out_of_count_cache_index_is_ub`;
/// * `count > 3` makes the C write past `int saveA[3]` **and** past the 4-slot
///   `c2sv a,b,c,d` simplex, corrupting its own stack frame → measured in
///   `probe_oversized_cache.rs`, which runs each library in a forked child and
///   finds both survive `count == 4` and both die with SIGSEGV from `count == 5`
///   on. That test is where the `count > 3` behaviour is characterised; asserting
///   it in-process would simply kill the test binary.
#[test]
fn err_row25_unchecked_cache_indices() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 125);
    for count in 1..=3i32 {
        for &ta in &VALID_TYPES {
            for &tb in &VALID_TYPES {
                let na = match ta {
                    C2_TYPE_CIRCLE => 1u32,
                    C2_TYPE_CAPSULE => 2,
                    _ => 4,
                };
                let nb = match tb {
                    C2_TYPE_CIRCLE => 1u32,
                    C2_TYPE_CAPSULE => 2,
                    _ => 4,
                };
                for _ in 0..256 {
                    // Indices are unconstrained by the C beyond the caller's own
                    // array; keep them inside the *initialised* vertex range so
                    // both builds read the same defined bytes.
                    let cache = c2GJKCache {
                        metric: g.coord(),
                        count,
                        iA: [
                            g.below(na) as c_int,
                            g.below(na) as c_int,
                            g.below(na) as c_int,
                        ],
                        iB: [
                            g.below(nb) as c_int,
                            g.below(nb) as c_int,
                            g.below(nb) as c_int,
                        ],
                        div: g.coord(),
                    };
                    let args = GjkArgs {
                        ax: None,
                        bx: None,
                        outA: true,
                        outB: true,
                        iters: true,
                        cache: Some(cache),
                        use_radius: 1,
                    };
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let ctx = format!(
                        "row25 count={count} {}/{}",
                        type_name(ta),
                        type_name(tb)
                    );
                    let (cd, coa, cob, ci, cch) = gjk(c, &a, ta, &b, tb, &args);
                    let (rd, roa, rob, ri, rch) = gjk(r, &a, ta, &b, tb, &args);
                    eq_f32(&format!("{ctx} dist"), cd, rd);
                    eq_v(&format!("{ctx} outA"), coa, roa);
                    eq_v(&format!("{ctx} outB"), cob, rob);
                    eq_int(&format!("{ctx} iters"), ci, ri);
                    eq_cache(&format!("{ctx} cache"), &cch.unwrap(), &rch.unwrap());
                    // The write-back never reports more than 3 vertices, so a
                    // round-tripped cache can never itself become oversized.
                    let back = cch.unwrap();
                    assert!(
                        (1..=3).contains(&back.count),
                        "{ctx}: written-back count {} outside 1..=3",
                        back.count
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Rows 26-27 — the two GJK-backed predicates reject via `return 0`
// ===========================================================================

#[test]
fn err_rows26_27_gjk_backed_predicates_return_zero() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 126);
    let mut zeros = (0usize, 0usize);
    let mut ones = (0usize, 0usize);

    // Row 26: c2AABBtoCapsule returns 0 exactly when c2GJK != 0.
    for _ in 0..20_000 {
        let bb = g.aabb();
        let cap = g.capsule();
        let cv = (c.c2AABBtoCapsule)(bb, cap);
        let rv = (r.c2AABBtoCapsule)(bb, cap);
        eq_int("row26 c2AABBtoCapsule", cv, rv);
        assert!(cv == 0 || cv == 1, "row26: result must be 0 or 1, got {cv}");
        if cv == 0 {
            zeros.0 += 1
        } else {
            ones.0 += 1
        }
        // Cross-check against c2GJK directly: `if (dist) return 0; return 1;`
        let a = ShapeBuf::from_aabb(bb);
        let b = ShapeBuf::from_capsule(cap);
        let args = GjkArgs {
            ax: None,
            bx: None,
            outA: false,
            outB: false,
            iters: false,
            cache: None,
            use_radius: 1,
        };
        let (cdist, _, _, _, _) = gjk(c, &a, C2_TYPE_AABB, &b, C2_TYPE_CAPSULE, &args);
        let expect = if cdist != 0.0 { 0 } else { 1 };
        eq_int("row26 matches c2GJK sentinel", cv, expect);
    }

    // Row 27: same for c2CapsuletoCapsule.
    for _ in 0..20_000 {
        let ca = g.capsule();
        let cb = g.capsule();
        let cv = (c.c2CapsuletoCapsule)(ca, cb);
        let rv = (r.c2CapsuletoCapsule)(ca, cb);
        eq_int("row27 c2CapsuletoCapsule", cv, rv);
        assert!(cv == 0 || cv == 1, "row27: result must be 0 or 1, got {cv}");
        if cv == 0 {
            zeros.1 += 1
        } else {
            ones.1 += 1
        }
        let a = ShapeBuf::from_capsule(ca);
        let b = ShapeBuf::from_capsule(cb);
        let args = GjkArgs {
            ax: None,
            bx: None,
            outA: false,
            outB: false,
            iters: false,
            cache: None,
            use_radius: 1,
        };
        let (cdist, _, _, _, _) = gjk(c, &a, C2_TYPE_CAPSULE, &b, C2_TYPE_CAPSULE, &args);
        let expect = if cdist != 0.0 { 0 } else { 1 };
        eq_int("row27 matches c2GJK sentinel", cv, expect);
    }
    println!("row26/27 rejections: {zeros:?} acceptances: {ones:?}");
    assert!(zeros.0 > 0 && ones.0 > 0, "row26 needs both outcomes");
    assert!(zeros.1 > 0 && ones.1 > 0, "row27 needs both outcomes");
}

// ===========================================================================
// Rows 28-32 — the closed-form predicates' strict `<` rejections
// ===========================================================================

/// Rows 28-29 — `c2CircletoCircle` and `c2CircletoAABB` use a strict `<`, so
/// the exactly-touching case is a REJECTION. Driven with exactly representable
/// values so `d2 == r2` bit-exactly.
#[test]
fn err_rows28_29_strict_less_than_rejects_exact_touch() {
    let (c, r) = apis();
    // Powers of two make d2 and r2 exactly representable.
    for e in 0..12u32 {
        let ra = (1u32 << e) as f32;
        let rb = (1u32 << e) as f32;
        let touch = ra + rb;
        for &(d, expect) in &[
            (touch, 0),          // exactly touching => rejected (strict <)
            (touch * 0.5, 1),    // overlapping
            (touch * 2.0, 0),    // apart
        ] {
            let a = c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: ra,
            };
            let b = c2Circle {
                p: c2v { x: d, y: 0.0 },
                r: rb,
            };
            let cv = (c.c2CircletoCircle)(a, b);
            let rv = (r.c2CircletoCircle)(a, b);
            eq_int(&format!("row28 e={e} d={d}"), cv, rv);
            eq_int(&format!("row28 e={e} d={d} expected {expect}"), cv, expect);
        }
        // Row 29: circle centre exactly `r` away from the AABB surface.
        let bb = c2AABB {
            min: c2v { x: -ra, y: -ra },
            max: c2v { x: ra, y: ra },
        };
        for &(cx, rad, expect) in &[
            (ra * 2.0, ra, 0),       // exactly touching the +x face => rejected
            (ra * 2.0, ra * 2.0, 1), // clearly overlapping
            (ra * 4.0, ra, 0),       // apart
        ] {
            let circ = c2Circle {
                p: c2v { x: cx, y: 0.0 },
                r: rad,
            };
            let cv = (c.c2CircletoAABB)(circ, bb);
            let rv = (r.c2CircletoAABB)(circ, bb);
            eq_int(&format!("row29 e={e} cx={cx} r={rad}"), cv, rv);
            eq_int(
                &format!("row29 e={e} cx={cx} r={rad} expected {expect}"),
                cv,
                expect,
            );
        }
    }
    // Zero-radius circle can never collide with anything (d2 < 0 is impossible).
    let mut g = Rng::new(SEED ^ 128);
    for _ in 0..5000 {
        let a = c2Circle { p: g.v(), r: 0.0 };
        let b = c2Circle { p: a.p, r: 0.0 };
        eq_int(
            "row28 zero radius coincident",
            (c.c2CircletoCircle)(a, b),
            (r.c2CircletoCircle)(a, b),
        );
        eq_int(
            "row28 zero radius is a miss",
            (c.c2CircletoCircle)(a, b),
            0,
        );
        let bb = c2AABB {
            min: a.p,
            max: a.p,
        };
        eq_int(
            "row29 zero radius vs degenerate box",
            (c.c2CircletoAABB)(a, bb),
            (r.c2CircletoAABB)(a, bb),
        );
        eq_int("row29 zero radius is a miss", (c.c2CircletoAABB)(a, bb), 0);
    }
}

/// Rows 30-32 — `c2CircletoCapsule` rejects on each of its three branches, and
/// the middle branch divides by `c2Dot(n, n)` which is `0` for a degenerate
/// capsule (row 31).
#[test]
fn err_rows30_32_circle_to_capsule_rejections() {
    let (c, r) = apis();
    let cap = c2Capsule {
        a: c2v { x: 0.0, y: 0.0 },
        b: c2v { x: 4.0, y: 0.0 },
        r: 0.5,
    };
    // Row 30: da < 0 (before endpoint a) and far away => reject.
    for k in 1..100i32 {
        let circ = c2Circle {
            p: c2v {
                x: -(k as f32),
                y: 0.0,
            },
            r: 0.25,
        };
        let cv = (c.c2CircletoCapsule)(circ, cap);
        let rv = (r.c2CircletoCapsule)(circ, cap);
        eq_int(&format!("row30 k={k}"), cv, rv);
        eq_int(&format!("row30 k={k} rejected"), cv, 0);
    }
    // Row 32: da >= 0, db >= 0 (past endpoint b) and far away => reject.
    for k in 5..100i32 {
        let circ = c2Circle {
            p: c2v {
                x: 4.0 + k as f32,
                y: 0.0,
            },
            r: 0.25,
        };
        let cv = (c.c2CircletoCapsule)(circ, cap);
        let rv = (r.c2CircletoCapsule)(circ, cap);
        eq_int(&format!("row32 k={k}"), cv, rv);
        eq_int(&format!("row32 k={k} rejected"), cv, 0);
    }
    // Row 31: mid-segment branch, rejected because it is too far perpendicular.
    for k in 2..100i32 {
        let circ = c2Circle {
            p: c2v {
                x: 2.0,
                y: k as f32,
            },
            r: 0.25,
        };
        let cv = (c.c2CircletoCapsule)(circ, cap);
        let rv = (r.c2CircletoCapsule)(circ, cap);
        eq_int(&format!("row31 k={k}"), cv, rv);
        eq_int(&format!("row31 k={k} rejected"), cv, 0);
    }
    // Row 31 degenerate: a == b makes n == (0,0), so `da / c2Dot(n,n)` would be
    // 0/0 -> NaN. The C reaches the mid branch only if da >= 0 && db < 0, which
    // with n == 0 means 0 >= 0 && 0 < 0 -> false, so it takes the third branch.
    // Assert both builds agree and that the result is the documented rejection.
    let mut g = Rng::new(SEED ^ 131);
    for _ in 0..5000 {
        let p = g.v();
        let deg = c2Capsule {
            a: p,
            b: p,
            r: 0.5,
        };
        let far = c2Circle {
            p: c2v {
                x: p.x + 100.0,
                y: p.y + 100.0,
            },
            r: 0.25,
        };
        let cv = (c.c2CircletoCapsule)(far, deg);
        let rv = (r.c2CircletoCapsule)(far, deg);
        eq_int("row31 degenerate far", cv, rv);
        eq_int("row31 degenerate far rejected", cv, 0);
    }
    // And a NaN-producing configuration: NaN `<` is always false => reject.
    for &nan_field in &[0usize, 1, 2] {
        let mut bad = cap;
        match nan_field {
            0 => bad.a.x = f32::NAN,
            1 => bad.b.y = f32::NAN,
            _ => bad.r = f32::NAN,
        }
        let circ = c2Circle {
            p: c2v { x: 2.0, y: 0.0 },
            r: 0.5,
        };
        let cv = (c.c2CircletoCapsule)(circ, bad);
        let rv = (r.c2CircletoCapsule)(circ, bad);
        eq_int(&format!("row31 nan field {nan_field}"), cv, rv);
        eq_int(&format!("row31 nan field {nan_field} rejected"), cv, 0);
    }
}

// ===========================================================================
// Rows 33-36 — the four separating axes of c2AABBtoAABB
// ===========================================================================

#[test]
fn err_rows33_36_aabb_separating_axes() {
    let (c, r) = apis();
    let unit = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    // One row per axis, each isolated so exactly one of d0..d3 is true.
    let cases: [(&str, c2AABB); 4] = [
        (
            "row33 B.max.x < A.min.x",
            c2AABB {
                min: c2v { x: -5.0, y: -1.0 },
                max: c2v { x: -2.0, y: 1.0 },
            },
        ),
        (
            "row34 A.max.x < B.min.x",
            c2AABB {
                min: c2v { x: 2.0, y: -1.0 },
                max: c2v { x: 5.0, y: 1.0 },
            },
        ),
        (
            "row35 B.max.y < A.min.y",
            c2AABB {
                min: c2v { x: -1.0, y: -5.0 },
                max: c2v { x: 1.0, y: -2.0 },
            },
        ),
        (
            "row36 A.max.y < B.min.y",
            c2AABB {
                min: c2v { x: -1.0, y: 2.0 },
                max: c2v { x: 1.0, y: 5.0 },
            },
        ),
    ];
    for (name, b) in cases {
        let cv = (c.c2AABBtoAABB)(unit, b);
        let rv = (r.c2AABBtoAABB)(unit, b);
        eq_int(name, cv, rv);
        eq_int(&format!("{name} rejected"), cv, 0);
    }
    // The boundary: exactly touching is NOT a rejection, because the tests are
    // strict `<` on the opposite corner.
    for &(dx, dy) in &[(2.0f32, 0.0f32), (-2.0, 0.0), (0.0, 2.0), (0.0, -2.0)] {
        let b = c2AABB {
            min: c2v {
                x: -1.0 + dx,
                y: -1.0 + dy,
            },
            max: c2v {
                x: 1.0 + dx,
                y: 1.0 + dy,
            },
        };
        let cv = (c.c2AABBtoAABB)(unit, b);
        let rv = (r.c2AABBtoAABB)(unit, b);
        eq_int(&format!("row33-36 touch dx={dx} dy={dy}"), cv, rv);
        eq_int(&format!("row33-36 touch dx={dx} dy={dy} accepted"), cv, 1);
    }
    // NaN in any coordinate makes every `<` false, so `!(0|0|0|0)` == 1.
    for field in 0..8usize {
        let mut b = c2AABB {
            min: c2v { x: 10.0, y: 10.0 },
            max: c2v { x: 20.0, y: 20.0 },
        };
        let mut a = unit;
        match field {
            0 => a.min.x = f32::NAN,
            1 => a.min.y = f32::NAN,
            2 => a.max.x = f32::NAN,
            3 => a.max.y = f32::NAN,
            4 => b.min.x = f32::NAN,
            5 => b.min.y = f32::NAN,
            6 => b.max.x = f32::NAN,
            _ => b.max.y = f32::NAN,
        }
        eq_int(
            &format!("row33-36 nan field {field}"),
            (c.c2AABBtoAABB)(a, b),
            (r.c2AABBtoAABB)(a, b),
        );
    }
}

// ===========================================================================
// Rows 37-40 — the four `default: return 0` arms of c2Collided
// ===========================================================================

#[test]
fn err_rows37_40_collided_default_arms() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 137);
    // Rows 37-39: valid typeA, invalid typeB — one row per typeA arm.
    for (row, &ta) in [37, 38, 39].iter().zip(
        [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE].iter(),
    ) {
        for &bad in &BAD_TYPES {
            let a = rand_shape(&mut g, ta);
            let b = rand_shape(&mut g, ta);
            let cv = unsafe { (c.c2Collided)(a.as_ptr(), ta, b.as_ptr(), bad) };
            let rv = unsafe { (r.c2Collided)(a.as_ptr(), ta, b.as_ptr(), bad) };
            eq_int(&format!("row{row} typeA={} typeB={bad}", type_name(ta)), cv, rv);
            eq_int(&format!("row{row} typeA={} typeB={bad} is 0", type_name(ta)), cv, 0);
            // B is never dereferenced on this path, so NULL is a valid input.
            let cn = unsafe { (c.c2Collided)(a.as_ptr(), ta, std::ptr::null(), bad) };
            let rn = unsafe { (r.c2Collided)(a.as_ptr(), ta, std::ptr::null(), bad) };
            eq_int(&format!("row{row} null B typeB={bad}"), cn, rn);
            eq_int(&format!("row{row} null B typeB={bad} is 0"), cn, 0);
        }
    }
    // Row 40: invalid typeA — the outer `default:`. Neither A nor B is read.
    for &bad in &BAD_TYPES {
        for &tb in &VALID_TYPES {
            let cv = unsafe { (c.c2Collided)(std::ptr::null(), bad, std::ptr::null(), tb) };
            let rv = unsafe { (r.c2Collided)(std::ptr::null(), bad, std::ptr::null(), tb) };
            eq_int(&format!("row40 typeA={bad} typeB={}", type_name(tb)), cv, rv);
            eq_int(&format!("row40 typeA={bad} is 0"), cv, 0);
        }
        for &bad2 in &BAD_TYPES {
            let cv = unsafe { (c.c2Collided)(std::ptr::null(), bad, std::ptr::null(), bad2) };
            let rv = unsafe { (r.c2Collided)(std::ptr::null(), bad, std::ptr::null(), bad2) };
            eq_int(&format!("row40 {bad}/{bad2}"), cv, rv);
            eq_int(&format!("row40 {bad}/{bad2} is 0"), cv, 0);
        }
    }
}

// ===========================================================================
// Rows 41-42 — ptr_from_parts UB and its observable consequence
// ===========================================================================

/// Row 41 — the C falls off the end of `ptr_from_parts` for an unknown type
/// (undefined behaviour: no `return`). Only the well-defined consequence is
/// asserted; the pointer value is not comparable. Row 42 — `omni_collide` must
/// nevertheless return a deterministic `0`, because `c2Collided` rejects the
/// unknown tag before dereferencing.
#[test]
fn err_rows41_42_ptr_from_parts_ub_and_omni_collide() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 141);
    for &bad in &BAD_TYPES {
        for _ in 0..128 {
            let p: [f32; 10] = [
                g.wild(),
                g.wild(),
                g.wild(),
                g.wild(),
                g.wild(),
                g.wild(),
                g.wild(),
                g.wild(),
                g.wild(),
                g.wild(),
            ];
            // Row 41: must not crash. Not dereferenced, not freed.
            let _cp = unsafe { (c.ptr_from_parts)(bad, p[0], p[1], p[2], p[3], p[4]) };
            let rp = unsafe { (r.ptr_from_parts)(bad, p[0], p[1], p[2], p[3], p[4]) };
            assert!(rp.is_null(), "row41: Rust must return NULL for type {bad}");

            // Row 42: the observable public behaviour is a deterministic 0.
            for &good in &VALID_TYPES {
                for (ta, tb) in [(bad, good), (good, bad), (bad, bad)] {
                    let cv = unsafe {
                        (c.omni_collide)(
                            ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                        )
                    };
                    let rv = unsafe {
                        (r.omni_collide)(
                            ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                        )
                    };
                    eq_int(&format!("row42 {ta}/{tb}"), cv, rv);
                    eq_int(&format!("row42 {ta}/{tb} is 0"), cv, 0);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 43-45 — division by zero
// ===========================================================================

/// Row 43 — `c2Div` / `c2Norm` divide with no guard: `1/0 = inf`, and `0*inf`
/// yields NaN.
#[test]
fn err_row43_div_by_zero_unguarded() {
    let (c, r) = apis();
    for &b in &[0.0f32, -0.0] {
        for &a in &[
            c2v { x: 1.0, y: 1.0 },
            c2v { x: -1.0, y: -1.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v {
                x: f32::INFINITY,
                y: 0.0,
            },
        ] {
            let cv = (c.c2Div)(a, b);
            let rv = (r.c2Div)(a, b);
            eq_v(&format!("row43 c2Div {a:?}/{b}"), cv, rv);
        }
    }
    // c2Norm of the zero vector: len == 0 -> 1/0 == inf -> 0*inf == NaN.
    for &a in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 0.0, y: -0.0 },
    ] {
        let cv = (c.c2Norm)(a);
        let rv = (r.c2Norm)(a);
        eq_v(&format!("row43 c2Norm {a:?}"), cv, rv);
        assert!(
            cv.x.is_nan() && rv.x.is_nan(),
            "row43: normalising the zero vector must yield NaN, got C={cv:?} Rust={rv:?}"
        );
    }
    // And the overflow direction: an infinite length divides to zero.
    let huge = c2v {
        x: f32::MAX,
        y: f32::MAX,
    };
    eq_v("row43 c2Norm huge", (c.c2Norm)(huge), (r.c2Norm)(huge));
}

/// Rows 44-45 — `s->div == 0` makes `den = 1/0 = inf` inside `c2Witness` and
/// `c2L`, with no guard.
#[test]
fn err_rows44_45_zero_div_in_witness_and_L() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 144);
    for &div in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for &count in &[1i32, 2, 3, 0, 4] {
            for _ in 0..64 {
                let mut s = c2Simplex {
                    verts: [
                        g.simplex_vert(),
                        g.simplex_vert(),
                        g.simplex_vert(),
                        g.simplex_vert(),
                    ],
                    div,
                    count,
                };
                let mut s2 = s;
                // Row 44
                let mut ca = c2v::default();
                let mut cb = c2v::default();
                let mut ra = c2v::default();
                let mut rb = c2v::default();
                unsafe { (c.c2Witness)(&mut s, &mut ca, &mut cb) };
                unsafe { (r.c2Witness)(&mut s2, &mut ra, &mut rb) };
                eq_v(&format!("row44 div={div:?} count={count} outA"), ca, ra);
                eq_v(&format!("row44 div={div:?} count={count} outB"), cb, rb);
                // Row 45
                let mut s3 = s;
                let mut s4 = s;
                eq_v(
                    &format!("row45 div={div:?} count={count}"),
                    unsafe { (c.c2L)(&mut s3) },
                    unsafe { (r.c2L)(&mut s4) },
                );
            }
        }
    }
}

// ===========================================================================
// Row 46 — c2BBVerts does not validate the box
// ===========================================================================

#[test]
fn err_row46_bbverts_no_validation() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 146);
    let boxes = [
        c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: -1.0, y: -1.0 },
        },
        c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 0.0, y: 0.0 },
        },
        c2AABB {
            min: c2v {
                x: f32::NAN,
                y: 0.0,
            },
            max: c2v { x: 1.0, y: 1.0 },
        },
        c2AABB {
            min: c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
            max: c2v {
                x: f32::NEG_INFINITY,
                y: f32::INFINITY,
            },
        },
    ];
    for (i, bb) in boxes.iter().enumerate() {
        let mut cbb = *bb;
        let mut rbb = *bb;
        let mut co = [c2v { x: -1.0, y: -1.0 }; 8];
        let mut ro = co;
        unsafe { (c.c2BBVerts)(co.as_mut_ptr(), &mut cbb) };
        unsafe { (r.c2BBVerts)(ro.as_mut_ptr(), &mut rbb) };
        for k in 0..8 {
            eq_v(&format!("row46 box{i} out[{k}]"), co[k], ro[k]);
        }
        // Only the first four slots are written; the rest keep the sentinel.
        for k in 4..8 {
            eq_f32_bits(&format!("row46 box{i} out[{k}] untouched"), co[k].x, -1.0);
        }
        eq_bytes(&format!("row46 box{i} input untouched"), &cbb, &rbb);
    }
    for i in 0..5000 {
        let bb = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        let mut cbb = bb;
        let mut rbb = bb;
        let mut co = [c2v::default(); 8];
        let mut ro = [c2v::default(); 8];
        unsafe { (c.c2BBVerts)(co.as_mut_ptr(), &mut cbb) };
        unsafe { (r.c2BBVerts)(ro.as_mut_ptr(), &mut rbb) };
        for k in 0..8 {
            eq_v(&format!("row46 wild #{i} out[{k}]"), co[k], ro[k]);
        }
    }
}

// ===========================================================================
// Row 47 — no input validation anywhere: every entry point on every float class
// ===========================================================================

#[test]
fn err_row47_no_input_validation_anywhere() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 147);
    // Every exported predicate and the public API, hammered with the full float
    // spread. None of them may reject via anything other than returning 0.
    for i in 0..20_000 {
        let ci = c2Circle {
            p: g.wild_v(),
            r: g.wild(),
        };
        let cj = c2Circle {
            p: g.wild_v(),
            r: g.wild(),
        };
        let bb = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        let bc = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        let ca = c2Capsule {
            a: g.wild_v(),
            b: g.wild_v(),
            r: g.wild(),
        };
        let cb = c2Capsule {
            a: g.wild_v(),
            b: g.wild_v(),
            r: g.wild(),
        };
        for (name, cv, rv) in [
            (
                "c2AABBtoAABB",
                (c.c2AABBtoAABB)(bb, bc),
                (r.c2AABBtoAABB)(bb, bc),
            ),
            (
                "c2CircletoCircle",
                (c.c2CircletoCircle)(ci, cj),
                (r.c2CircletoCircle)(ci, cj),
            ),
            (
                "c2CircletoAABB",
                (c.c2CircletoAABB)(ci, bb),
                (r.c2CircletoAABB)(ci, bb),
            ),
            (
                "c2CircletoCapsule",
                (c.c2CircletoCapsule)(ci, ca),
                (r.c2CircletoCapsule)(ci, ca),
            ),
            (
                "c2AABBtoCapsule",
                (c.c2AABBtoCapsule)(bb, ca),
                (r.c2AABBtoCapsule)(bb, ca),
            ),
            (
                "c2CapsuletoCapsule",
                (c.c2CapsuletoCapsule)(ca, cb),
                (r.c2CapsuletoCapsule)(ca, cb),
            ),
        ] {
            eq_int(&format!("row47 {name} #{i}"), cv, rv);
            assert!(
                cv == 0 || cv == 1,
                "row47 {name} #{i}: result must be 0 or 1, got {cv}"
            );
        }
    }
}

// ===========================================================================
// Generic FFI boundary checks required by Phase C beyond the table
// ===========================================================================

/// Null shape pointers where the C provably does not dereference them.
#[test]
fn generic_null_pointer_boundary() {
    let (c, r) = apis();
    // c2Collided with an unknown typeA never touches A or B.
    for &bad in &BAD_TYPES {
        for &tb in &VALID_TYPES {
            eq_int(
                &format!("null-ptr c2Collided {bad}/{}", type_name(tb)),
                unsafe { (c.c2Collided)(std::ptr::null(), bad, std::ptr::null(), tb) },
                unsafe { (r.c2Collided)(std::ptr::null(), bad, std::ptr::null(), tb) },
            );
        }
    }
    // c2MakeProxy with an unknown type never touches the shape pointer.
    for &bad in &BAD_TYPES {
        let mut cp = c2Proxy::default();
        let mut rp = c2Proxy::default();
        unsafe { (c.c2MakeProxy)(std::ptr::null(), bad, &mut cp) };
        unsafe { (r.c2MakeProxy)(std::ptr::null(), bad, &mut rp) };
        eq_proxy(&format!("null-ptr c2MakeProxy {bad}"), &cp, &rp);
    }
    // All three optional c2GJK outputs null at once.
    let mut g = Rng::new(SEED ^ 200);
    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            for ur in [0, 1] {
                for _ in 0..200 {
                    let a = rand_shape(&mut g, ta);
                    let b = rand_shape(&mut g, tb);
                    let cd = unsafe {
                        (c.c2GJK)(
                            a.as_ptr(),
                            ta,
                            std::ptr::null(),
                            b.as_ptr(),
                            tb,
                            std::ptr::null(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            ur,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    };
                    let rd = unsafe {
                        (r.c2GJK)(
                            a.as_ptr(),
                            ta,
                            std::ptr::null(),
                            b.as_ptr(),
                            tb,
                            std::ptr::null(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            ur,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    };
                    eq_f32("null-ptr c2GJK all-null outputs", cd, rd);
                }
            }
        }
    }
}

/// Out-of-range enum values one step past each end of the valid range, plus the
/// extremes — the class of input a C enum accepts but no variant covers.
#[test]
fn generic_out_of_range_enum_boundary() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 201);
    // Valid range is 0..=2. One step past each end, plus the int extremes.
    let probes: Vec<C2_TYPE> = vec![-1, 0, 1, 2, 3, 4, i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX];
    for &ta in &probes {
        for &tb in &probes {
            for _ in 0..16 {
                let p: [f32; 10] = [
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.radius(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.radius(),
                ];
                let cv = unsafe {
                    (c.omni_collide)(
                        ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                    )
                };
                let rv = unsafe {
                    (r.omni_collide)(
                        ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                    )
                };
                eq_int(&format!("enum boundary omni_collide {ta}/{tb}"), cv, rv);
                // Anything outside 0..=2 must be a deterministic 0.
                if !(0..=2).contains(&ta) || !(0..=2).contains(&tb) {
                    eq_int(&format!("enum boundary {ta}/{tb} is 0"), cv, 0);
                }
            }
            // And the dispatcher directly.
            let a = rand_shape(&mut g, if (0..=2).contains(&ta) { ta } else { C2_TYPE_CIRCLE });
            let b = rand_shape(&mut g, if (0..=2).contains(&tb) { tb } else { C2_TYPE_CIRCLE });
            eq_int(
                &format!("enum boundary c2Collided {ta}/{tb}"),
                unsafe { (c.c2Collided)(a.as_ptr(), ta, b.as_ptr(), tb) },
                unsafe { (r.c2Collided)(a.as_ptr(), ta, b.as_ptr(), tb) },
            );
        }
    }
}

/// Zero and oversized lengths / counts across every count-taking export.
#[test]
fn generic_zero_and_oversized_counts() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 202);
    let mut verts = [c2v::default(); 8];
    for v in verts.iter_mut() {
        *v = g.v();
    }
    // c2Support: 0, 1, exactly the buffer size, and negative.
    for &count in &[0i32, 1, 8, -1, i32::MIN] {
        for _ in 0..64 {
            let d = g.v();
            eq_int(
                &format!("oversized c2Support count={count}"),
                unsafe { (c.c2Support)(verts.as_ptr(), count, d) },
                unsafe { (r.c2Support)(verts.as_ptr(), count, d) },
            );
        }
    }
    // Simplex `count` at and past the 4-slot array bound, for every reader.
    for &count in &[0i32, 1, 2, 3, 4, 5, -1, i32::MAX, i32::MIN] {
        for _ in 0..64 {
            let s = c2Simplex {
                verts: [
                    g.simplex_vert(),
                    g.simplex_vert(),
                    g.simplex_vert(),
                    g.simplex_vert(),
                ],
                div: g.coord(),
                count,
            };
            let (mut a, mut b) = (s, s);
            eq_f32(
                &format!("oversized metric count={count}"),
                unsafe { (c.c2GJKSimplexMetric)(&mut a) },
                unsafe { (r.c2GJKSimplexMetric)(&mut b) },
            );
            let (mut a, mut b) = (s, s);
            eq_v(
                &format!("oversized c2D count={count}"),
                unsafe { (c.c2D)(&mut a) },
                unsafe { (r.c2D)(&mut b) },
            );
            let (mut a, mut b) = (s, s);
            eq_v(
                &format!("oversized c2L count={count}"),
                unsafe { (c.c2L)(&mut a) },
                unsafe { (r.c2L)(&mut b) },
            );
            let (mut a, mut b) = (s, s);
            let mut ca = c2v::default();
            let mut cb = c2v::default();
            let mut ra = c2v::default();
            let mut rb = c2v::default();
            unsafe { (c.c2Witness)(&mut a, &mut ca, &mut cb) };
            unsafe { (r.c2Witness)(&mut b, &mut ra, &mut rb) };
            eq_v(&format!("oversized witness A count={count}"), ca, ra);
            eq_v(&format!("oversized witness B count={count}"), cb, rb);
        }
    }
    let _: *const c_void = std::ptr::null();
}
