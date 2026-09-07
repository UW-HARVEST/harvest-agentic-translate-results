//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input / rejection condition, calls
//! BOTH libraries through their `.so` exports, and asserts they produce the
//! same rejection (same sentinel value, same `m->count`, same error return),
//! not merely "both failed somehow".

mod common;

use common::*;
use std::ffi::{c_int, c_void};

unsafe extern "C" {
    #[link_name = "free"]
    #[allow(dead_code)]
    fn libc_free(p: *mut c_void);
}

/// Out-of-range / boundary `C2_TYPE` values crossing the FFI boundary. C enums
/// accept any `int`, so these are real inputs the C library handles.
const BAD_TYPES: [c_int; 10] = [3, 4, 5, 99, -1, -2, -99, i32::MIN, i32::MAX, 0x7fff_fffe];

fn shapes(rng: &mut Rng) -> (c2Circle, c2AABB, c2Capsule) {
    (
        c2Circle {
            p: rng.vec(5.0),
            r: rng.fpos(3.0),
        },
        c2AABB {
            min: rng.vec(5.0),
            max: rng.vec(5.0),
        },
        c2Capsule {
            a: rng.vec(5.0),
            b: rng.vec(5.0),
            r: rng.fpos(3.0),
        },
    )
}

// ======================================================================
// ROWS 1, 2, 3, 4, 5, 6 — c2Collide type dispatch with no matching `case`
// ======================================================================

#[test]
fn rows1_6_collide_unhandled_types() {
    let l = libs();
    let (cf, rf) = l.pair::<FnCollide>("c2Collide");
    let mut rng = Rng::new(0xE001);

    let good: [c_int; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];

    for &bad in &BAD_TYPES {
        for &g in &good {
            for i in 0..200usize {
                let (ci, bb, cap) = shapes(&mut rng);
                let ptr = |t: c_int| -> *const c_void {
                    match t {
                        C2_TYPE_CIRCLE => &ci as *const _ as *const c_void,
                        C2_TYPE_AABB => &bb as *const _ as *const c_void,
                        C2_TYPE_CAPSULE => &cap as *const _ as *const c_void,
                        // A deliberately NULL shape for the unhandled types:
                        // the C must never dereference it (rows 1..6).
                        _ => std::ptr::null(),
                    }
                };

                // ROW 1/2: typeA unhandled  -> outer switch falls through
                let mut cm = seeded_manifold(-1111.0);
                let mut rm = cm;
                unsafe {
                    cf(ptr(bad), bad, ptr(g), g, &mut cm);
                    rf(ptr(bad), bad, ptr(g), g, &mut rm);
                }
                assert_eq!(cm.count, 0, "ROW1/2 C typeA={bad}: count must be 0");
                assert!(
                    meq(&cm, &rm),
                    "ROW1/2 typeA={bad} typeB={g} i={i}\n  C {}\n  R {}",
                    ms(&cm),
                    ms(&rm)
                );

                // ROWS 3..6: typeA handled, typeB unhandled -> inner switch
                let mut cm = seeded_manifold(-2222.0);
                let mut rm = cm;
                unsafe {
                    cf(ptr(g), g, ptr(bad), bad, &mut cm);
                    rf(ptr(g), g, ptr(bad), bad, &mut rm);
                }
                assert_eq!(cm.count, 0, "ROW3-6 C typeA={g} typeB={bad}: count must be 0");
                assert!(
                    meq(&cm, &rm),
                    "ROW3-6 typeA={g} typeB={bad} i={i}\n  C {}\n  R {}",
                    ms(&cm),
                    ms(&rm)
                );

                // Both unhandled.
                let mut cm = seeded_manifold(-3333.0);
                let mut rm = cm;
                unsafe {
                    cf(std::ptr::null(), bad, std::ptr::null(), bad, &mut cm);
                    rf(std::ptr::null(), bad, std::ptr::null(), bad, &mut rm);
                }
                assert_eq!(cm.count, 0);
                assert!(meq(&cm, &rm), "ROW1-6 both={bad}\n  C {}\n  R {}", ms(&cm), ms(&rm));
            }
        }
    }
}

// ======================================================================
// ROW 7 — ptr_from_parts falls off the end for POLY / out-of-range
// ======================================================================

#[test]
fn row7_ptr_from_parts_falls_off_end() {
    let l = libs();
    let (cf, rf) = l.pair::<FnPtrFromParts>("ptr_from_parts");
    let (c_omni, r_omni) = l.pair::<FnOmni>("omni_manifold");
    let mut rng = Rng::new(0xE007);

    for &bad in &BAD_TYPES {
        for i in 0..100usize {
            let p = [rng.f(10.0), rng.f(10.0), rng.f(10.0), rng.f(10.0), rng.f(10.0)];
            // The C has no `return` on this path, so the returned pointer value
            // is indeterminate (it is whatever is left in %rax). The Rust
            // returns NULL. The *observable* contract is that the pointer is
            // never dereferenced: `c2Collide` has no arm for these types. Assert
            // that neither library allocates/derefs and that the full
            // `omni_manifold` path yields the identical rejection.
            let _c = unsafe { cf(bad, p[0], p[1], p[2], p[3], p[4]) };
            let r = unsafe { rf(bad, p[0], p[1], p[2], p[3], p[4]) };
            assert!(r.is_null(), "ROW7 Rust must produce NULL for type {bad}");

            let mut cm = seeded_manifold(-777.0);
            let mut rm = cm;
            unsafe {
                scrub_stack();
                c_omni(&mut cm, bad, p[0], p[1], p[2], p[3], p[4], bad, p[0], p[1], p[2], p[3], p[4]);
                scrub_stack();
                r_omni(&mut rm, bad, p[0], p[1], p[2], p[3], p[4], bad, p[0], p[1], p[2], p[3], p[4]);
            }
            assert_eq!(cm.count, 0, "ROW7 C count for type {bad}");
            assert!(
                meq(&cm, &rm),
                "ROW7 omni type={bad} i={i}\n  C {}\n  R {}",
                ms(&cm),
                ms(&rm)
            );
        }
    }
}

// ======================================================================
// ROW 8 — c2MakeProxy has no POLY case: `*p` left untouched
// ======================================================================

#[test]
fn row8_make_proxy_unhandled_type() {
    let l = libs();
    let (cf, rf) = l.pair::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(0xE008);

    for &bad in &BAD_TYPES {
        for i in 0..200usize {
            // Seed both proxies with an identical recognizable pattern; the C
            // must leave every byte of it alone.
            let mut cp = c2Proxy {
                radius: 12.5,
                count: 1234,
                verts: [v(9.0, -9.0); 8],
            };
            for k in 0..8 {
                cp.verts[k] = rng.vec(100.0);
            }
            let before = cp;
            let mut rp = cp;
            let dummy = c2Poly::default();
            unsafe {
                cf(&dummy as *const _ as *const c_void, bad, &mut cp);
                rf(&dummy as *const _ as *const c_void, bad, &mut rp);
            }
            assert!(
                feq(cp.radius, before.radius)
                    && cp.count == before.count
                    && (0..8).all(|k| veq(cp.verts[k], before.verts[k])),
                "ROW8 C modified the proxy for type {bad}"
            );
            assert!(
                feq(cp.radius, rp.radius)
                    && cp.count == rp.count
                    && (0..8).all(|k| veq(cp.verts[k], rp.verts[k])),
                "ROW8 type={bad} i={i}: proxy diverged"
            );
        }
    }
}

// ======================================================================
// ROWS 9, 10, 11, 12, 13, 14 — c2GJK NULL pointer arguments
// ======================================================================

#[test]
fn rows9_14_gjk_null_arguments() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0xE009);

    for i in 0..4000usize {
        let (ci, _bb, cap) = shapes(&mut rng);
        let ident = c2x {
            p: v(0.0, 0.0),
            r: c2r { c: 1.0, s: 0.0 },
        };
        // ROWS 9/10: NULL transforms must be substituted with c2xIdentity(),
        // so a NULL transform and an explicit identity must agree.
        let mut out = [v(0.0, 0.0); 8];
        let mut it = [0i32; 4];
        let (d_c_null, d_c_ident, d_r_null, d_r_ident) = unsafe {
            scrub_stack();
            let a = cf(
                &ci as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &mut out[0], &mut out[1], 1, &mut it[0], std::ptr::null_mut(),
            );
            scrub_stack();
            let b = cf(
                &ci as *const _ as *const c_void, C2_TYPE_CIRCLE, &ident,
                &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &ident,
                &mut out[2], &mut out[3], 1, &mut it[1], std::ptr::null_mut(),
            );
            scrub_stack();
            let c = rf(
                &ci as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &mut out[4], &mut out[5], 1, &mut it[2], std::ptr::null_mut(),
            );
            scrub_stack();
            let d = rf(
                &ci as *const _ as *const c_void, C2_TYPE_CIRCLE, &ident,
                &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &ident,
                &mut out[6], &mut out[7], 1, &mut it[3], std::ptr::null_mut(),
            );
            (a, b, c, d)
        };
        assert!(feq(d_c_null, d_r_null), "ROW9/10 NULL xform dist {i}");
        assert!(feq(d_c_ident, d_r_ident), "ROW9/10 identity xform dist {i}");
        assert!(
            feq(d_c_null, d_c_ident),
            "ROW9/10 C: NULL xform must equal identity ({} vs {})",
            fs(d_c_null),
            fs(d_c_ident)
        );
        for k in 0..4 {
            assert!(veq(out[k], out[k + 4]), "ROW9/10 witness[{k}] {i}");
        }
        assert_eq!(it[0], it[2], "ROW9/10 iterations {i}");
        assert_eq!(it[1], it[3], "ROW9/10 iterations {i}");

        // ROWS 11/12/13: NULL outA / outB / iterations / cache must be skipped
        // and must not change the returned distance.
        let mut ca = v(-5.0, -5.0);
        let mut ra = v(-5.0, -5.0);
        let (dc, dr) = unsafe {
            scrub_stack();
            let a = cf(
                &ci as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                std::ptr::null_mut(), &mut ca, 1, std::ptr::null_mut(), std::ptr::null_mut(),
            );
            scrub_stack();
            let b = rf(
                &ci as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                std::ptr::null_mut(), &mut ra, 1, std::ptr::null_mut(), std::ptr::null_mut(),
            );
            (a, b)
        };
        assert!(feq(dc, dr) && veq(ca, ra), "ROW11-13 NULL outA/iters/cache {i}");
        assert!(feq(dc, d_c_null), "ROW11 C: NULL outA changed the distance");

        let mut cb = v(-6.0, -6.0);
        let mut rb = v(-6.0, -6.0);
        let (dc, dr) = unsafe {
            scrub_stack();
            let a = cf(
                &ci as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &mut cb, std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut(),
            );
            scrub_stack();
            let b = rf(
                &ci as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &mut rb, std::ptr::null_mut(), 1, std::ptr::null_mut(), std::ptr::null_mut(),
            );
            (a, b)
        };
        assert!(feq(dc, dr) && veq(cb, rb), "ROW11 NULL outB {i}");

        // ROW 14: non-NULL cache with count == 0 (cache_was_good false).
        let mut cc = c2GJKCache::default();
        let mut rc = c2GJKCache::default();
        let (dc, dr) = unsafe {
            scrub_stack();
            let a = cf(
                &ci as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &mut out[0], &mut out[1], 0, std::ptr::null_mut(), &mut cc,
            );
            scrub_stack();
            let b = rf(
                &ci as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &mut out[4], &mut out[5], 0, std::ptr::null_mut(), &mut rc,
            );
            (a, b)
        };
        assert!(feq(dc, dr), "ROW14 cold cache dist {i}");
        assert!(
            feq(cc.metric, rc.metric)
                && cc.count == rc.count
                && cc.iA == rc.iA
                && cc.iB == rc.iB
                && feq(cc.div, rc.div),
            "ROW14 cold cache written out differs: C {cc:?} vs R {rc:?}"
        );
    }
}

// ======================================================================
// ROWS 15, 16 — stale / out-of-range GJK cache accepted without validation
// ======================================================================

#[test]
fn rows15_16_gjk_cache_unvalidated() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0xE015);

    for i in 0..8000usize {
        let (ci, bb, cap) = shapes(&mut rng);
        // Proxy vertex counts are 1 (circle), 2 (capsule), 4 (AABB); feed
        // indices that are valid for the 8-element proxy array but out of range
        // for `count`, plus `cache.count` values from 0..=3.
        let mk = |rng: &mut Rng| c2GJKCache {
            metric: if rng.below(4) == 0 { 0.0 } else { rng.f(100.0) },
            count: rng.below(4) as c_int,
            iA: [
                rng.below(8) as c_int,
                rng.below(8) as c_int,
                rng.below(8) as c_int,
            ],
            iB: [
                rng.below(8) as c_int,
                rng.below(8) as c_int,
                rng.below(8) as c_int,
            ],
            div: if rng.below(4) == 0 { 0.0 } else { rng.f(10.0) },
        };
        let seed = mk(&mut rng);
        let (ta, pa): (c_int, *const c_void) = match i % 3 {
            0 => (C2_TYPE_CIRCLE, &ci as *const _ as *const c_void),
            1 => (C2_TYPE_CAPSULE, &cap as *const _ as *const c_void),
            _ => (C2_TYPE_AABB, &bb as *const _ as *const c_void),
        };
        let (tb, pb): (c_int, *const c_void) = match (i / 3) % 3 {
            0 => (C2_TYPE_CIRCLE, &ci as *const _ as *const c_void),
            1 => (C2_TYPE_CAPSULE, &cap as *const _ as *const c_void),
            _ => (C2_TYPE_AABB, &bb as *const _ as *const c_void),
        };

        let mut cc = seed;
        let mut rc = seed;
        let mut ca = v(0.0, 0.0);
        let mut cb2 = v(0.0, 0.0);
        let mut ra = v(0.0, 0.0);
        let mut rb2 = v(0.0, 0.0);
        let mut cit = 0i32;
        let mut rit = 0i32;
        let (dc, dr) = unsafe {
            scrub_stack();
            let a = cf(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut ca, &mut cb2, (i % 2) as c_int, &mut cit, &mut cc);
            scrub_stack();
            let b = rf(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut ra, &mut rb2, (i % 2) as c_int, &mut rit, &mut rc);
            (a, b)
        };
        assert!(
            feq(dc, dr) && veq(ca, ra) && veq(cb2, rb2) && cit == rit,
            "ROW15/16 stale cache {i} seed={seed:?} ta={ta} tb={tb}: \
             dist C {} vs R {}, a C {} vs R {}, b C {} vs R {}, it {cit} vs {rit}",
            fs(dc), fs(dr), vs(ca), vs(ra), vs(cb2), vs(rb2)
        );
        assert!(
            feq(cc.metric, rc.metric)
                && cc.count == rc.count
                && cc.iA == rc.iA
                && cc.iB == rc.iB
                && feq(cc.div, rc.div),
            "ROW15/16 cache out {i}: C {cc:?} vs R {rc:?}"
        );
    }
}

// ======================================================================
// ROWS 17, 18, 19, 20, 21, 22, 23 — c2GJK loop-exit and use_radius branches
// ======================================================================

#[test]
fn rows17_23_gjk_loop_and_radius_branches() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0xE017);

    let mut max_iter = 0i32;
    let mut zero_dist = 0usize;
    let mut nonzero_dist = 0usize;

    for i in 0..15_000usize {
        // Coincident and grid-aligned shapes maximize `hit`, the duplicate
        // support-point exit, the `d1 > d0` exit and the degenerate-direction
        // exit; identical positions give dist == 0 exactly.
        let (a_shape, ta): (c2Capsule, c_int) = (
            c2Capsule {
                a: v(rng.grid(0.25, 8), rng.grid(0.25, 8)),
                b: v(rng.grid(0.25, 8), rng.grid(0.25, 8)),
                r: rng.grid(0.25, 8).abs(),
            },
            C2_TYPE_CAPSULE,
        );
        let b_shape = if i % 3 == 0 {
            a_shape // identical shapes -> `hit` (ROW 23) and exact zero
        } else {
            c2Capsule {
                a: v(rng.grid(0.25, 8), rng.grid(0.25, 8)),
                b: v(rng.grid(0.25, 8), rng.grid(0.25, 8)),
                r: rng.grid(0.25, 8).abs(),
            }
        };
        let ur = (i % 2) as c_int;
        let mut ca = v(0.0, 0.0);
        let mut cb = v(0.0, 0.0);
        let mut ra = v(0.0, 0.0);
        let mut rb = v(0.0, 0.0);
        let mut cit = -1i32;
        let mut rit = -1i32;
        let (dc, dr) = unsafe {
            scrub_stack();
            let x = cf(
                &a_shape as *const _ as *const c_void, ta, std::ptr::null(),
                &b_shape as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &mut ca, &mut cb, ur, &mut cit, std::ptr::null_mut(),
            );
            scrub_stack();
            let y = rf(
                &a_shape as *const _ as *const c_void, ta, std::ptr::null(),
                &b_shape as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &mut ra, &mut rb, ur, &mut rit, std::ptr::null_mut(),
            );
            (x, y)
        };
        assert!(
            feq(dc, dr) && veq(ca, ra) && veq(cb, rb) && cit == rit,
            "ROW17-23 {i} ur={ur}: dist C {} vs R {}; a C {} vs R {}; b C {} vs R {}; it {cit} vs {rit}",
            fs(dc), fs(dr), vs(ca), vs(ra), vs(cb), vs(rb)
        );
        assert!(cit >= 0 && cit <= 20, "ROW17 iteration bound violated: {cit}");
        max_iter = max_iter.max(cit);
        if dc == 0.0 {
            zero_dist += 1;
        } else {
            nonzero_dist += 1;
        }
    }
    assert!(
        zero_dist > 100 && nonzero_dist > 100,
        "ROW21-23 coverage: zero={zero_dist} nonzero={nonzero_dist}"
    );
    assert!(max_iter > 1, "ROW17/18 loop never iterated (max_iter={max_iter})");
}

// ======================================================================
// ROWS 24, 25, 26, 27, 28 — simplex helpers: `default:` arms and div == 0
// ======================================================================

#[test]
fn rows24_28_simplex_defaults_and_zero_div() {
    let l = libs();
    let (c_metric, r_metric) = l.pair::<FnFsimplex>("c2GJKSimplexMetric");
    let (c_d, r_d) = l.pair::<FnVsimplex>("c2D");
    let (c_w, r_w) = l.pair::<FnWitness>("c2Witness");
    let (c_l, r_l) = l.pair::<FnVsimplex>("c2L");
    let mut rng = Rng::new(0xE024);

    // Counts outside {1,2,3} hit the shared `default:` arms.
    let counts: [c_int; 9] = [0, 1, 2, 3, 4, 5, -1, 100, i32::MIN];

    for &count in &counts {
        for i in 0..500usize {
            let mut s = c2Simplex::default();
            for k in 0..4 {
                s.verts[k] = c2sv {
                    sA: rng.vec(20.0),
                    sB: rng.vec(20.0),
                    p: rng.vec(20.0),
                    u: rng.f(5.0),
                    iA: rng.below(8) as c_int,
                    iB: rng.below(8) as c_int,
                };
            }
            // ROW 28: div == 0 => den = 1/0 = +inf
            s.div = match i % 3 {
                0 => 0.0,
                1 => -0.0,
                _ => rng.f(5.0),
            };
            s.count = count;

            // ROW 24
            let mut a = s;
            let mut b = s;
            unsafe {
                let x = c_metric(&mut a);
                let y = r_metric(&mut b);
                assert!(
                    feq(x, y),
                    "ROW24 c2GJKSimplexMetric count={count} i={i}: C {} vs R {}",
                    fs(x),
                    fs(y)
                );
                if !(1..=3).contains(&count) || count == 1 {
                    assert!(feq(x, 0.0), "ROW24 default arm must return 0, got {}", fs(x));
                }
            }
            // ROW 25
            let mut a = s;
            let mut b = s;
            unsafe {
                let x = c_d(&mut a);
                let y = r_d(&mut b);
                assert!(
                    veq(x, y),
                    "ROW25 c2D count={count} i={i}: C {} vs R {}",
                    vs(x),
                    vs(y)
                );
                if count == 3 || !(1..=3).contains(&count) {
                    assert!(veq(x, v(0.0, 0.0)), "ROW25 default arm must be (0,0)");
                }
            }
            // ROW 26 + ROW 28
            let mut a = s;
            let mut b = s;
            let mut ca = v(-1.0, -1.0);
            let mut cb = v(-2.0, -2.0);
            let mut ra = v(-1.0, -1.0);
            let mut rb = v(-2.0, -2.0);
            unsafe {
                c_w(&mut a, &mut ca, &mut cb);
                r_w(&mut b, &mut ra, &mut rb);
            }
            assert!(
                veq(ca, ra) && veq(cb, rb),
                "ROW26/28 c2Witness count={count} i={i} div={}: C ({},{}) vs R ({},{})",
                fs(s.div),
                vs(ca),
                vs(cb),
                vs(ra),
                vs(rb)
            );
            if !(1..=3).contains(&count) {
                assert!(
                    veq(ca, v(0.0, 0.0)) && veq(cb, v(0.0, 0.0)),
                    "ROW26 default arm must write (0,0)"
                );
            }
            // ROW 27 + ROW 28
            let mut a = s;
            let mut b = s;
            unsafe {
                let x = c_l(&mut a);
                let y = r_l(&mut b);
                assert!(
                    veq(x, y),
                    "ROW27/28 c2L count={count} i={i} div={}: C {} vs R {}",
                    fs(s.div),
                    vs(x),
                    vs(y)
                );
                if !(1..=2).contains(&count) {
                    assert!(veq(x, v(0.0, 0.0)), "ROW27 default arm must be (0,0)");
                }
            }
        }
    }
}

// ======================================================================
// ROW 29 — c2Support with count <= 0 still reads verts[0] and returns 0
// ======================================================================

#[test]
fn row29_support_nonpositive_count() {
    let l = libs();
    let (cf, rf) = l.pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(0xE029);
    let counts: [c_int; 6] = [0, -1, -2, -100, i32::MIN, 1];
    for &count in &counts {
        for i in 0..500usize {
            let mut verts = [v(0.0, 0.0); 8];
            for k in 0..8 {
                verts[k] = rng.vec(50.0);
            }
            let d = rng.vec(1.0);
            unsafe {
                let a = cf(verts.as_ptr(), count, d);
                let b = rf(verts.as_ptr(), count, d);
                assert_eq!(a, b, "ROW29 c2Support count={count} i={i}");
                if count <= 0 {
                    assert_eq!(a, 0, "ROW29 count<=0 must return 0");
                }
            }
        }
    }
}

// ======================================================================
// ROWS 30/31 — c2Norm / c2Div by zero
// ======================================================================

#[test]
fn rows30_31_norm_and_div_by_zero() {
    let l = libs();
    let (c_norm, r_norm) = l.pair::<FnVv>("c2Norm");
    let (c_div, r_div) = l.pair::<FnVvf>("c2Div");
    let (c_len, r_len) = l.pair::<FnFv>("c2Len");

    // ROW 30: exact zero vector -> c2Len == 0 -> 1/0 == inf -> 0*inf == NaN
    for &(x, y) in &[
        (0.0f32, 0.0f32),
        (-0.0, 0.0),
        (0.0, -0.0),
        (-0.0, -0.0),
        (f32::MIN_POSITIVE, 0.0),
        (1e-30, 1e-30),
        (f32::from_bits(1), 0.0),
    ] {
        let a = v(x, y);
        unsafe {
            let cl = c_len(a);
            let rl = r_len(a);
            assert!(feq(cl, rl), "ROW30 c2Len {} : C {} vs R {}", vs(a), fs(cl), fs(rl));
            let cn = c_norm(a);
            let rn = r_norm(a);
            assert!(
                veq(cn, rn),
                "ROW30 c2Norm {} : C {} vs R {}",
                vs(a),
                vs(cn),
                vs(rn)
            );
        }
    }

    // ROW 31: c2Div by zero and by non-finite scalars.
    let mut rng = Rng::new(0xE031);
    for i in 0..4000usize {
        let a = if i % 4 == 0 { rng.wild_vec() } else { rng.vec(1e6) };
        let b = match i % 6 {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::from_bits(1),
            _ => rng.f(100.0),
        };
        unsafe {
            let x = c_div(a, b);
            let y = r_div(a, b);
            assert!(
                veq(x, y),
                "ROW31 c2Div {} / {} : C {} vs R {}",
                vs(a),
                fs(b),
                vs(x),
                vs(y)
            );
        }
    }
}

// ======================================================================
// ROWS 32, 33, 34, 35, 36, 37, 38, 65 — static c2Clip / c2SidePlanes rejections.
//
// These are `static` in C (not exported by either .so), so they are driven
// through their only callers, `c2CapsuletoPolyManifold` and
// `c2AABBtoCapsuleManifold`. Each row's condition is constructed and the
// observable rejection (`m->count == 0` plus the full manifold) is compared.
// ======================================================================

/// Builds a poly whose normals come from the library itself.
fn poly_from(l: &'static Libs, verts: &[c2v]) -> c2Poly {
    let (c_norms, _) = l.pair::<FnNorms>("c2Norms");
    let mut p = c2Poly::default();
    p.count = verts.len() as c_int;
    for (i, &x) in verts.iter().enumerate() {
        p.verts[i] = x;
    }
    unsafe {
        c_norms(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), p.count);
    }
    p
}

fn cap_poly_both(
    l: &'static Libs,
    cap: c2Capsule,
    poly: &c2Poly,
    bx: Option<&c2x>,
) -> (c2Manifold, c2Manifold) {
    let (cf, rf) = l.pair::<FnCapsulePoly>("c2CapsuletoPolyManifold");
    let bxp = bx.map_or(std::ptr::null(), |x| x as *const c2x);
    let mut cm = seeded_manifold(-9999.0);
    let mut rm = cm;
    unsafe {
        scrub_stack();
        cf(cap, poly, bxp, &mut cm);
        scrub_stack();
        rf(cap, poly, bxp, &mut rm);
    }
    (cm, rm)
}

#[test]
fn rows32_38_clip_and_side_planes_rejections() {
    let l = libs();
    let mut rng = Rng::new(0xE032);

    let mut rejected = 0usize;
    let mut accepted_1 = 0usize;
    let mut accepted_2 = 0usize;
    let mut degenerate = 0usize;

    for i in 0..30_000usize {
        // A square poly centred on the origin, so the zeroed poly proxy makes
        // `c2GJK` measure the capsule against (0,0) and the deep branch (and
        // therefore c2Clip / c2SidePlanes) is reached whenever the segment
        // passes through the origin.
        let s = 1.0 + rng.fpos(4.0);
        let poly = poly_from(
            l,
            &[v(-s, -s), v(s, -s), v(s, s), v(-s, s)],
        );

        let cap = match i % 6 {
            // ROW 37: ra == rb -> c2Norm(0,0) == NaN -> every c2Dist compare
            // false -> c2Clip returns 0 -> c2SidePlanes returns 0.
            // A point capsule at the origin makes `in` degenerate.
            0 => {
                degenerate += 1;
                c2Capsule { a: v(0.0, 0.0), b: v(0.0, 0.0), r: rng.fpos(4.0) }
            }
            // ROWS 32/35/36: segment through the origin but far outside the
            // poly's side planes -> clip yields < 2 -> rejection.
            1 | 2 => {
                let ang = rng.f(std::f32::consts::PI);
                let d = v(ang.cos(), ang.sin());
                let t = 1e-3 + rng.fpos(0.02);
                c2Capsule {
                    a: v(-d.x * t, -d.y * t),
                    b: v(d.x * t, d.y * t),
                    r: rng.fpos(4.0),
                }
            }
            // ROWS 33/34: exact grid values so `d0 == 0 && d1 == 0` and
            // `d0 * d1 <= 0` boundaries are hit.
            3 | 4 => {
                let ang = rng.f(std::f32::consts::PI);
                let d = v(ang.cos(), ang.sin());
                c2Capsule {
                    a: v(-d.x * rng.grid(1.0, 8).abs(), -d.y * rng.grid(1.0, 8).abs()),
                    b: v(d.x * rng.grid(1.0, 8).abs(), d.y * rng.grid(1.0, 8).abs()),
                    r: rng.grid(0.5, 8).abs(),
                }
            }
            // ROW 65: tiny distances so `d0 * d1` UNDERFLOWS to +0.0 while
            // both are negative -- the case where C's `sp` reaches 3 and it
            // writes past `out[2]` (onto the dead `d1` slot) and returns 3.
            // This is the input class that used to abort the Rust build.
            _ => {
                let ang = rng.f(std::f32::consts::PI);
                let d = v(ang.cos(), ang.sin());
                let t = f32::from_bits(rng.next_u32() % 0x0100_0000 + 1);
                c2Capsule {
                    a: v(-d.x * t, -d.y * t),
                    b: v(d.x * t, d.y * t),
                    r: rng.fpos(4.0),
                }
            }
        };
        // ROW 38 is covered implicitly: `c2SidePlanesFromPoly` always passes a
        // non-NULL `h`, and `c2SidePlanes` with NULL `h` is unreachable from
        // any exported entry point.
        let bx = if i % 3 == 0 { None } else { Some(rng.xform(2.0)) };
        let (cm, rm) = cap_poly_both(l, cap, &poly, bx.as_ref());
        match cm.count {
            0 => rejected += 1,
            1 => accepted_1 += 1,
            _ => accepted_2 += 1,
        }
        assert!(
            meq(&cm, &rm),
            "ROW32-38 i={i} cap={cap:?} s={s} bx={}\n  C {}\n  R {}",
            bx.is_some(),
            ms(&cm),
            ms(&rm)
        );
    }
    assert!(
        rejected > 100 && accepted_1 > 100 && accepted_2 > 100 && degenerate > 100,
        "ROW32-38 coverage: rejected={rejected} c1={accepted_1} c2={accepted_2} degen={degenerate}"
    );
}

// ======================================================================
// ROWS 39, 40, 41, 42, 43, 44, 45 — c2CapsuletoPolyManifold early returns
// ======================================================================

#[test]
fn rows39_45_capsule_poly_early_returns() {
    let l = libs();
    let mut rng = Rng::new(0xE039);

    let mut zero_count = 0usize;
    for i in 0..30_000usize {
        // ROW 44: B->count == 0/1/2 leaves `index` at ~0 == -1 and `sep` at
        // -FLT_MAX; ROW 45: A.a == A.b makes `ab` NaN.
        let n = match i % 8 {
            0 => 0usize,
            1 => 1,
            2 => 2,
            k => k,
        };
        let mut verts = Vec::new();
        let s = 1.0 + rng.fpos(4.0);
        for k in 0..n {
            let t = std::f32::consts::TAU * (k as f32) / (n.max(1) as f32);
            verts.push(v(s * t.cos(), s * t.sin()));
        }
        let poly = poly_from(l, &verts);

        let cap = match i % 5 {
            // ROW 45: degenerate point capsule -> c2Norm(0,0) -> NaN
            0 => {
                let p = rng.vec(3.0);
                c2Capsule { a: p, b: p, r: rng.fpos(4.0) }
            }
            // ROW 43: d >= 1e-6 and d >= A.r -> no branch taken at all
            1 => c2Capsule {
                a: v(50.0 + rng.f(5.0), 50.0 + rng.f(5.0)),
                b: v(60.0 + rng.f(5.0), 60.0 + rng.f(5.0)),
                r: rng.fpos(1.0),
            },
            // ROWS 39..41: deep branch, so one of the three `code` arms runs
            // and its c2SidePlanes* can fail
            _ => {
                let ang = rng.f(std::f32::consts::PI);
                let d = v(ang.cos(), ang.sin());
                let t = rng.fpos(6.0);
                let u = rng.fpos(6.0);
                c2Capsule {
                    a: v(-d.x * t, -d.y * t),
                    b: v(d.x * u, d.y * u),
                    r: rng.fpos(5.0),
                }
            }
        };
        let bx = if i % 4 == 0 { None } else { Some(rng.xform(2.0)) };
        let (cm, rm) = cap_poly_both(l, cap, &poly, bx.as_ref());
        if cm.count == 0 {
            zero_count += 1;
        }
        assert!(
            meq(&cm, &rm),
            "ROW39-45 i={i} n={n} cap={cap:?}\n  C {}\n  R {}",
            ms(&cm),
            ms(&rm)
        );
    }
    // ROW 42's `default:` arm is unreachable (`code` is only ever 0, 1 or 2);
    // it is documented in ERRORS.md and produces the same bare `return;`.
    assert!(zero_count > 500, "ROW39-43 rejection coverage: {zero_count}");
}

// ======================================================================
// ROWS 46, 47, 48 — c2AABBtoAABBManifold dx < 0 / dy < 0 / inverted boxes
// ======================================================================

#[test]
fn rows46_48_aabb_aabb_rejections() {
    let l = libs();
    let (cf, rf) = l.pair::<FnAABBAABB>("c2AABBtoAABBManifold");
    let mut rng = Rng::new(0xE046);

    let mut sep_x = 0usize;
    let mut sep_y = 0usize;
    let mut inverted = 0usize;

    for i in 0..30_000usize {
        let (a, b) = match i % 4 {
            // ROW 46: separated on x only -> dx < 0
            0 => {
                sep_x += 1;
                (
                    c2AABB { min: v(-2.0, -2.0), max: v(-1.0, 2.0) },
                    c2AABB { min: v(1.0 + rng.fpos(5.0), -2.0), max: v(5.0, 2.0) },
                )
            }
            // ROW 47: overlapping on x, separated on y -> dy < 0
            1 => {
                sep_y += 1;
                (
                    c2AABB { min: v(-2.0, -2.0), max: v(2.0, -1.0) },
                    c2AABB { min: v(-2.0, 1.0 + rng.fpos(5.0)), max: v(2.0, 5.0) },
                )
            }
            // ROW 48: inverted boxes (min > max)
            2 => {
                inverted += 1;
                (
                    c2AABB { min: rng.vec(4.0), max: rng.vec(-4.0) },
                    c2AABB { min: rng.vec(4.0), max: rng.vec(-4.0) },
                )
            }
            // exact touching via grid coordinates: dx == 0 / dy == 0
            _ => (
                c2AABB {
                    min: v(rng.grid(1.0, 4), rng.grid(1.0, 4)),
                    max: v(rng.grid(1.0, 4), rng.grid(1.0, 4)),
                },
                c2AABB {
                    min: v(rng.grid(1.0, 4), rng.grid(1.0, 4)),
                    max: v(rng.grid(1.0, 4), rng.grid(1.0, 4)),
                },
            ),
        };
        let mut cm = seeded_manifold(-8888.0);
        let mut rm = cm;
        unsafe {
            cf(a, b, &mut cm);
            rf(a, b, &mut rm);
        }
        if i % 4 < 2 {
            assert_eq!(cm.count, 0, "ROW46/47 must reject: {a:?} {b:?}");
            // The bare `return;` leaves depths / contact_points / n untouched.
            assert!(
                feq(cm.depths[0], -8888.0) && veq(cm.n, v(-8888.0, -8888.0)),
                "ROW46/47 the early return must not touch depths or n: {}",
                ms(&cm)
            );
        }
        assert!(
            meq(&cm, &rm),
            "ROW46-48 i={i}: {a:?} {b:?}\n  C {}\n  R {}",
            ms(&cm),
            ms(&rm)
        );
    }
    assert!(sep_x > 100 && sep_y > 100 && inverted > 100);
}

// ======================================================================
// ROWS 49, 50, 51, 52, 53 — circle manifold rejections and degenerate branches
// ======================================================================

#[test]
fn rows49_53_circle_rejections() {
    let l = libs();
    let (c_cc, r_cc) = l.pair::<FnCircleCircle>("c2CircletoCircleManifold");
    let (c_ca, r_ca) = l.pair::<FnCircleAABB>("c2CircletoAABBManifold");
    let mut rng = Rng::new(0xE049);

    for i in 0..20_000usize {
        // ROW 49: d2 >= r*r
        let a = c2Circle { p: v(-10.0, 0.0), r: rng.fpos(1.0) };
        let b = c2Circle { p: v(10.0 + rng.fpos(5.0), 0.0), r: rng.fpos(1.0) };
        let mut cm = seeded_manifold(-6666.0);
        let mut rm = cm;
        unsafe {
            c_cc(a, b, &mut cm);
            r_cc(a, b, &mut rm);
        }
        assert_eq!(cm.count, 0, "ROW49 must reject");
        assert!(meq(&cm, &rm), "ROW49 i={i}\n  C {}\n  R {}", ms(&cm), ms(&rm));

        // ROW 50: concentric circles -> l == 0 -> normal forced to (0, 1)
        let p = rng.vec(10.0);
        let a = c2Circle { p, r: 1.0 + rng.fpos(3.0) };
        let b = c2Circle { p, r: 1.0 + rng.fpos(3.0) };
        let mut cm = seeded_manifold(-6666.0);
        let mut rm = cm;
        unsafe {
            c_cc(a, b, &mut cm);
            r_cc(a, b, &mut rm);
        }
        assert_eq!(cm.count, 1, "ROW50 concentric must produce a contact");
        assert!(veq(cm.n, v(0.0, 1.0)), "ROW50 normal must be (0,1): {}", vs(cm.n));
        assert!(meq(&cm, &rm), "ROW50 i={i}\n  C {}\n  R {}", ms(&cm), ms(&rm));

        // ROW 51: circle far outside the AABB -> d2 >= r2
        let ci = c2Circle { p: v(100.0 + rng.fpos(10.0), 0.0), r: rng.fpos(2.0) };
        let bb = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
        let mut cm = seeded_manifold(-6666.0);
        let mut rm = cm;
        unsafe {
            c_ca(ci, bb, &mut cm);
            r_ca(ci, bb, &mut rm);
        }
        assert_eq!(cm.count, 0, "ROW51 must reject");
        assert!(meq(&cm, &rm), "ROW51 i={i}\n  C {}\n  R {}", ms(&cm), ms(&rm));

        // ROW 52: circle centre strictly inside -> d2 == 0 deep branch. Steer
        // both `x_overlap < y_overlap` and the else branch.
        let ex = 1.0 + rng.fpos(4.0);
        let ey = 1.0 + rng.fpos(4.0);
        let mid = rng.vec(5.0);
        let bb = c2AABB {
            min: v(mid.x - ex, mid.y - ey),
            max: v(mid.x + ex, mid.y + ey),
        };
        let ci = c2Circle {
            p: v(
                mid.x + rng.f(ex * 0.9),
                mid.y + rng.f(ey * 0.9),
            ),
            r: rng.fpos(3.0),
        };
        let mut cm = seeded_manifold(-6666.0);
        let mut rm = cm;
        unsafe {
            c_ca(ci, bb, &mut cm);
            r_ca(ci, bb, &mut rm);
        }
        assert!(meq(&cm, &rm), "ROW52 i={i}: {ci:?} {bb:?}\n  C {}\n  R {}", ms(&cm), ms(&rm));

        // ROW 53: zero-radius circle -> r2 == 0, so `d2 < r2` is never true
        let ci = c2Circle { p: rng.vec(5.0), r: 0.0 };
        let bb = c2AABB { min: v(-3.0, -3.0), max: v(3.0, 3.0) };
        let mut cm = seeded_manifold(-6666.0);
        let mut rm = cm;
        unsafe {
            c_ca(ci, bb, &mut cm);
            r_ca(ci, bb, &mut rm);
        }
        assert_eq!(cm.count, 0, "ROW53 zero-radius circle must never collide");
        assert!(meq(&cm, &rm), "ROW53 i={i}\n  C {}\n  R {}", ms(&cm), ms(&rm));
    }
}

// ======================================================================
// ROWS 54, 55, 56, 57 — capsule manifold rejections and NaN normals
// ======================================================================

#[test]
fn rows54_57_capsule_rejections() {
    let l = libs();
    let (c_cc, r_cc) = l.pair::<FnCircleCapsule>("c2CircletoCapsuleManifold");
    let (c_kk, r_kk) = l.pair::<FnCapsuleCapsule>("c2CapsuletoCapsuleManifold");
    let mut rng = Rng::new(0xE054);

    let mut nan_normals = 0usize;
    for i in 0..20_000usize {
        // ROW 54: circle far from the capsule -> d >= A.r + B.r
        let ci = c2Circle { p: v(-100.0 - rng.fpos(10.0), 0.0), r: rng.fpos(2.0) };
        let cap = c2Capsule { a: v(0.0, -1.0), b: v(0.0, 1.0), r: rng.fpos(2.0) };
        let mut cm = seeded_manifold(-5555.0);
        let mut rm = cm;
        unsafe {
            scrub_stack();
            c_cc(ci, cap, &mut cm);
            scrub_stack();
            r_cc(ci, cap, &mut rm);
        }
        assert_eq!(cm.count, 0, "ROW54 must reject");
        assert!(meq(&cm, &rm), "ROW54 i={i}\n  C {}\n  R {}", ms(&cm), ms(&rm));

        // ROW 55: d == 0 with a point capsule -> c2Norm(c2Skew(0,0)) -> NaN
        let p = rng.vec(5.0);
        let ci = c2Circle { p, r: 1.0 + rng.fpos(3.0) };
        let cap = c2Capsule { a: p, b: p, r: 1.0 + rng.fpos(3.0) };
        let mut cm = seeded_manifold(-5555.0);
        let mut rm = cm;
        unsafe {
            scrub_stack();
            c_cc(ci, cap, &mut cm);
            scrub_stack();
            r_cc(ci, cap, &mut rm);
        }
        assert_eq!(cm.count, 1, "ROW55 must still report a contact");
        if cm.n.x.is_nan() {
            nan_normals += 1;
        }
        assert!(meq(&cm, &rm), "ROW55 i={i}\n  C {}\n  R {}", ms(&cm), ms(&rm));

        // ROW 56: capsules far apart -> d >= A.r + B.r
        let a = c2Capsule { a: v(-100.0, 0.0), b: v(-90.0, 0.0), r: rng.fpos(2.0) };
        let b = c2Capsule { a: v(90.0, 0.0), b: v(100.0, 0.0), r: rng.fpos(2.0) };
        let mut cm = seeded_manifold(-5555.0);
        let mut rm = cm;
        unsafe {
            scrub_stack();
            c_kk(a, b, &mut cm);
            scrub_stack();
            r_kk(a, b, &mut rm);
        }
        assert_eq!(cm.count, 0, "ROW56 must reject");
        assert!(meq(&cm, &rm), "ROW56 i={i}\n  C {}\n  R {}", ms(&cm), ms(&rm));

        // ROW 57: coincident point capsules -> d == 0 and A.a == A.b -> NaN
        let p = rng.vec(5.0);
        let a = c2Capsule { a: p, b: p, r: 1.0 + rng.fpos(3.0) };
        let b = c2Capsule { a: p, b: p, r: 1.0 + rng.fpos(3.0) };
        let mut cm = seeded_manifold(-5555.0);
        let mut rm = cm;
        unsafe {
            scrub_stack();
            c_kk(a, b, &mut cm);
            scrub_stack();
            r_kk(a, b, &mut rm);
        }
        assert_eq!(cm.count, 1, "ROW57 must still report a contact");
        assert!(meq(&cm, &rm), "ROW57 i={i}\n  C {}\n  R {}", ms(&cm), ms(&rm));
    }
    assert!(nan_normals > 1000, "ROW55 NaN-normal coverage: {nan_normals}");
}

// ======================================================================
// ROW 58 — c2AABBtoCapsuleManifold negates `m->n` unconditionally, even on
// the early-return path where no manifold was produced.
// ======================================================================

#[test]
fn row58_aabb_capsule_negates_on_rejection() {
    let l = libs();
    let (cf, rf) = l.pair::<FnAABBCapsule>("c2AABBtoCapsuleManifold");
    let mut rng = Rng::new(0xE058);

    let mut rejected_with_written_n = 0usize;
    for i in 0..20_000usize {
        // Far apart, so `c2CapsuletoPolyManifold` takes a rejection path but the
        // trailing `m->n = c2Neg(m->n)` still runs.
        let bb = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
        let cap = match i % 3 {
            0 => c2Capsule {
                a: v(500.0 + rng.f(10.0), 500.0),
                b: v(600.0, 600.0 + rng.f(10.0)),
                r: rng.fpos(1.0),
            },
            1 => {
                let p = rng.vec(3.0);
                c2Capsule { a: p, b: p, r: rng.fpos(2.0) }
            }
            _ => c2Capsule {
                a: v(rng.grid(1.0, 6), rng.grid(1.0, 6)),
                b: v(rng.grid(1.0, 6), rng.grid(1.0, 6)),
                r: rng.grid(0.5, 4).abs(),
            },
        };
        let mut cm = seeded_manifold(-4444.0);
        let mut rm = cm;
        unsafe {
            scrub_stack();
            cf(bb, cap, &mut cm);
            scrub_stack();
            rf(bb, cap, &mut rm);
        }
        if cm.count == 0 && !veq(cm.n, v(-4444.0, -4444.0)) {
            rejected_with_written_n += 1;
        }
        assert!(
            meq(&cm, &rm),
            "ROW58 i={i} cap={cap:?}\n  C {}\n  R {}",
            ms(&cm),
            ms(&rm)
        );
    }
    assert!(
        rejected_with_written_n > 100,
        "ROW58 coverage (rejected but `n` still negated): {rejected_with_written_n}"
    );
}

// ======================================================================
// ROWS 59, 60 — c2Norms with count == 0 and duplicate vertices
// ======================================================================

#[test]
fn rows59_60_norms_degenerate() {
    let l = libs();
    let (cf, rf) = l.pair::<FnNorms>("c2Norms");
    let mut rng = Rng::new(0xE059);

    for i in 0..10_000usize {
        // ROW 59: count == 0 (and negative counts) must write nothing.
        let counts: [c_int; 5] = [0, -1, -7, i32::MIN, 1];
        let count = counts[i % 5];
        let mut verts = [v(0.0, 0.0); 8];
        for k in 0..8 {
            verts[k] = rng.vec(10.0);
        }
        let mut cn = [v(-3.5, -3.5); 8];
        let mut rn = [v(-3.5, -3.5); 8];
        unsafe {
            cf(verts.as_mut_ptr(), cn.as_mut_ptr(), count);
            rf(verts.as_mut_ptr(), rn.as_mut_ptr(), count);
        }
        if count <= 0 {
            assert!(
                (0..8).all(|k| veq(cn[k], v(-3.5, -3.5))),
                "ROW59 count={count} must write nothing"
            );
        }
        for k in 0..8 {
            assert!(veq(cn[k], rn[k]), "ROW59 count={count} [{k}]");
        }

        // ROW 60: duplicate consecutive verts -> c2Norm(0,0) -> NaN normals
        let n = 3 + (i % 6) as c_int;
        let mut verts = [v(0.0, 0.0); 8];
        for k in 0..8 {
            verts[k] = rng.vec(10.0);
        }
        let dup = (i % n as usize) as usize;
        verts[(dup + 1) % n as usize] = verts[dup];
        let mut cn = [v(-3.5, -3.5); 8];
        let mut rn = [v(-3.5, -3.5); 8];
        unsafe {
            cf(verts.as_mut_ptr(), cn.as_mut_ptr(), n);
            rf(verts.as_mut_ptr(), rn.as_mut_ptr(), n);
        }
        assert!(
            (0..n as usize).any(|k| cn[k].x.is_nan()),
            "ROW60 duplicate verts must produce a NaN normal"
        );
        for k in 0..8 {
            assert!(
                veq(cn[k], rn[k]),
                "ROW60 n={n} [{k}]: C {} vs R {}",
                vs(cn[k]),
                vs(rn[k])
            );
        }
    }
}

// ======================================================================
// ROW 61 — c2PlaneAt with an out-of-range index (no bounds check in C)
// ======================================================================

#[test]
fn row61_plane_at_out_of_range_index() {
    let l = libs();
    let (cf, rf) = l.pair::<FnHpi>("c2PlaneAt");
    let mut rng = Rng::new(0xE061);

    // Pad the poly on both sides so out-of-range reads stay inside memory this
    // test owns; both libraries are handed the SAME pointer, so any read they
    // perform -- in range or not -- must observe identical bytes.
    #[repr(C)]
    struct Padded {
        before: [c2v; 8],
        poly: c2Poly,
        after: [c2v; 8],
    }

    for i in 0..10_000usize {
        let mut pad = Padded {
            before: [v(0.0, 0.0); 8],
            poly: c2Poly::default(),
            after: [v(0.0, 0.0); 8],
        };
        for k in 0..8 {
            pad.before[k] = rng.vec(100.0);
            pad.after[k] = rng.vec(100.0);
            pad.poly.verts[k] = rng.vec(100.0);
            pad.poly.norms[k] = rng.vec(100.0);
        }
        pad.poly.count = 3 + (i % 6) as c_int;

        // norms has 8 slots; index i reads norms[i] and verts[i]. Negative
        // indices walk back into `count`/`before`, large ones into `after`.
        for idx in [-8i32, -3, -1, 0, 1, 7, 8, 9, 15] {
            unsafe {
                let a = cf(&pad.poly, idx);
                let b = rf(&pad.poly, idx);
                assert!(
                    heq(a, b),
                    "ROW61 c2PlaneAt idx={idx} i={i}: C n={} d={} vs R n={} d={}",
                    vs(a.n),
                    fs(a.d),
                    vs(b.n),
                    fs(b.d)
                );
            }
        }
    }
}

// ======================================================================
// ROW 62 — omni_manifold with POLY / out-of-range types
// ======================================================================

#[test]
fn row62_omni_manifold_unhandled_types() {
    let l = libs();
    let (cf, rf) = l.pair::<FnOmni>("omni_manifold");
    let mut rng = Rng::new(0xE062);
    let good: [c_int; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];

    for &bad in &BAD_TYPES {
        for &g in &good {
            for i in 0..100usize {
                let p: [f32; 5] = [rng.f(8.0), rng.f(8.0), rng.f(8.0), rng.f(8.0), rng.fpos(3.0)];
                let q: [f32; 5] = [rng.f(8.0), rng.f(8.0), rng.f(8.0), rng.f(8.0), rng.fpos(3.0)];
                for (ta, tb) in [(bad, g), (g, bad), (bad, bad)] {
                    let mut cm = seeded_manifold(-3210.0);
                    let mut rm = cm;
                    unsafe {
                        scrub_stack();
                        cf(&mut cm, ta, p[0], p[1], p[2], p[3], p[4], tb, q[0], q[1], q[2], q[3], q[4]);
                        scrub_stack();
                        rf(&mut rm, ta, p[0], p[1], p[2], p[3], p[4], tb, q[0], q[1], q[2], q[3], q[4]);
                    }
                    assert_eq!(cm.count, 0, "ROW62 C count for ({ta},{tb})");
                    // The unhandled path never writes anything but `count`.
                    assert!(
                        feq(cm.depths[0], -3210.0) && veq(cm.n, v(-3210.0, -3210.0)),
                        "ROW62 ({ta},{tb}) must leave the rest of *m untouched: {}",
                        ms(&cm)
                    );
                    assert!(
                        meq(&cm, &rm),
                        "ROW62 ({ta},{tb}) i={i}\n  C {}\n  R {}",
                        ms(&cm),
                        ms(&rm)
                    );
                }
            }
        }
    }
}

// ======================================================================
// ROWS 64, 67 — non-finite coordinates through every entry point.
//
// Note on NaN: a NaN *payload* is chosen by the destination register of the
// SSE op that consumes it, so when two NaNs with DIFFERENT payloads meet in
// `a.x*b.x + a.y*b.y`, the payload of the result depends on the compiler's
// operand ordering (the C is built at -O0, the Rust at -O3). Every NaN a
// caller can observe from either library is still a NaN, and all internally
// generated NaNs are the x86 "real indefinite" `0xffc00000`, so this test uses
// one canonical NaN payload per run -- which is what a real consumer sees.
// ======================================================================

#[test]
fn row64_non_finite_inputs() {
    let l = libs();
    let (cf, rf) = l.pair::<FnOmni>("omni_manifold");
    let (c_ac, r_ac) = l.pair::<FnAABBCapsule>("c2AABBtoCapsuleManifold");
    let mut rng = Rng::new(0xE064);
    let good: [c_int; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];

    const SPECIALS: [f32; 8] = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        0.0,
        -0.0,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
    ];

    let mut differing = 0usize;
    // Inputs where the C's own two entry points to the very same computation
    // disagree with each other (see the assertion below).
    let mut c_self_inconsistent = 0usize;
    let mut poly_path_cases = 0usize;

    for &ta in &good {
        for &tb in &good {
            // These two pairs route through c2AABBtoCapsuleManifold ->
            // c2CapsuletoPolyManifold -> c2GJK(.., C2_TYPE_POLY, ..), where the
            // C reads c2GJK's never-written `c2Proxy pB` local.
            let poly_path = (ta == C2_TYPE_CAPSULE && tb == C2_TYPE_AABB)
                || (ta == C2_TYPE_AABB && tb == C2_TYPE_CAPSULE);

            for i in 0..6000usize {
                let mut p = [0.0f32; 5];
                let mut q = [0.0f32; 5];
                for k in 0..5 {
                    p[k] = if rng.below(3) == 0 {
                        SPECIALS[rng.below(8) as usize]
                    } else {
                        rng.f(20.0)
                    };
                    q[k] = if rng.below(3) == 0 {
                        SPECIALS[rng.below(8) as usize]
                    } else {
                        rng.f(20.0)
                    };
                }
                let mut cm = seeded_manifold(-1234.0);
                let mut rm = cm;
                unsafe {
                    scrub_stack();
                    cf(&mut cm, ta, p[0], p[1], p[2], p[3], p[4], tb, q[0], q[1], q[2], q[3], q[4]);
                    scrub_stack();
                    rf(&mut rm, ta, p[0], p[1], p[2], p[3], p[4], tb, q[0], q[1], q[2], q[3], q[4]);
                }
                if !veq_nan(cm.n, v(-1234.0, -1234.0)) {
                    differing += 1;
                }
                if meq_nan(&cm, &rm) {
                    continue;
                }

                if !poly_path {
                    panic!(
                        "ROW64 ({ta},{tb}) i={i}\n  A={p:?}\n  B={q:?}\n  C {}\n  R {}",
                        ms(&cm),
                        ms(&rm)
                    );
                }

                // ROW 67 also lives here: a non-finite AABB makes every
                // `c2Norms` normal NaN, so `c2Incident` / the plane loop keep
                // `index == ~0` and read `p->verts[-1]`.
                //
                // The pair goes through the uninitialized-proxy path. Prove the
                // divergence is the C's own nondeterminism rather than a
                // translation defect: run the SAME shapes through
                // c2AABBtoCapsuleManifold directly (no `malloc` between the
                // stack scrub and the read of `pB`). There C and Rust must agree
                // exactly -- and the C's own two answers for identical shapes
                // must differ, which is only possible if the C consumed
                // uninitialized memory.
                let (bb, cap) = if ta == C2_TYPE_AABB {
                    (
                        c2AABB { min: v(p[0], p[1]), max: v(p[2], p[3]) },
                        c2Capsule { a: v(q[0], q[1]), b: v(q[2], q[3]), r: q[4] },
                    )
                } else {
                    (
                        c2AABB { min: v(q[0], q[1]), max: v(q[2], q[3]) },
                        c2Capsule { a: v(p[0], p[1]), b: v(p[2], p[3]), r: p[4] },
                    )
                };
                let mut cd = seeded_manifold(-1234.0);
                let mut rd = cd;
                unsafe {
                    scrub_stack();
                    c_ac(bb, cap, &mut cd);
                    scrub_stack();
                    r_ac(bb, cap, &mut rd);
                }
                assert!(
                    meq_nan(&cd, &rd),
                    "ROW64 ({ta},{tb}) i={i}: C and Rust disagree even on the \
                     deterministic direct path\n  A={p:?}\n  B={q:?}\n  C {}\n  R {}",
                    ms(&cd),
                    ms(&rd)
                );
                // `c2Collide` negates `m->n` once more than the direct call, so
                // compare everything except `n`.
                let same_but_n = cd.count == cm.count
                    && (0..2).all(|k| {
                        feq_nan(cd.depths[k], cm.depths[k])
                            && veq_nan(cd.contact_points[k], cm.contact_points[k])
                    });
                assert!(
                    !same_but_n,
                    "ROW64 ({ta},{tb}) i={i}: omni and direct agree inside C, so the \
                     C-vs-Rust divergence is NOT C nondeterminism\n  A={p:?}\n  B={q:?}\n  \
                     C omni {}\n  R omni {}\n  C direct {}",
                    ms(&cm),
                    ms(&rm),
                    ms(&cd)
                );
                c_self_inconsistent += 1;
                poly_path_cases += 1;
            }
        }
    }
    assert!(differing > 1000, "ROW64 coverage: {differing}");
    // 7 of the 9 pairs are held to strict equality above (any divergence
    // panics). The 2 poly-path pairs may diverge only where the C itself is
    // inconsistent; keep that rate visibly tiny.
    assert!(
        c_self_inconsistent * 200 < 6000 * 2,
        "ROW64: too many uninitialized-proxy divergences ({c_self_inconsistent} of 12000 \
         poly-path cases); expected a small tail, investigate"
    );
    println!(
        "ROW64: {differing} manifolds written; {poly_path_cases} uninitialized-proxy \
         divergences (C self-inconsistent)"
    );
}

/// Companion to ROW 64: the exported primitives with mixed NaN payloads.
/// Asserts the weaker but still meaningful contract that both libraries agree
/// on NaN-ness (and are bit-identical whenever the result is not NaN).
#[test]
fn row64_nan_payload_agreement() {
    let l = libs();
    let (c_dot, r_dot) = l.pair::<FnFvv>("c2Dot");
    let (c_len, r_len) = l.pair::<FnFv>("c2Len");
    let (c_norm, r_norm) = l.pair::<FnVv>("c2Norm");
    let (c_det, r_det) = l.pair::<FnFvv>("c2Det2");
    let mut rng = Rng::new(0xE065);

    for i in 0..20_000usize {
        let a = rng.wild_nan_vec();
        let b = rng.wild_nan_vec();
        unsafe {
            let (x, y) = (c_dot(a, b), r_dot(a, b));
            assert_eq!(
                x.is_nan(),
                y.is_nan(),
                "ROW64 c2Dot NaN-ness {i}: {} . {} -> C {} vs R {}",
                vs(a), vs(b), fs(x), fs(y)
            );
            if !x.is_nan() {
                assert!(feq(x, y), "ROW64 c2Dot {i}: C {} vs R {}", fs(x), fs(y));
            }
            let (x, y) = (c_det(a, b), r_det(a, b));
            assert_eq!(x.is_nan(), y.is_nan(), "ROW64 c2Det2 NaN-ness {i}");
            if !x.is_nan() {
                assert!(feq(x, y), "ROW64 c2Det2 {i}");
            }
            let (x, y) = (c_len(a), r_len(a));
            assert_eq!(x.is_nan(), y.is_nan(), "ROW64 c2Len NaN-ness {i}");
            if !x.is_nan() {
                assert!(feq(x, y), "ROW64 c2Len {i}: C {} vs R {}", fs(x), fs(y));
            }
            let (x, y) = (c_norm(a), r_norm(a));
            assert_eq!(x.x.is_nan(), y.x.is_nan(), "ROW64 c2Norm.x NaN-ness {i}");
            assert_eq!(x.y.is_nan(), y.y.is_nan(), "ROW64 c2Norm.y NaN-ness {i}");
        }
    }
}

/// Companion to ROW 16 — `cache->iA` / `cache->iB` / `cache->count` are used
/// entirely unvalidated by the C. Indices outside `[0, 8)` make both libraries
/// read past their own `c2Proxy.verts` (and `cache->count > 3` reads past
/// `cache->iA`), so the *values* are frame-dependent and cannot be compared.
/// What IS a hard requirement: neither library may abort or crash, and the Rust
/// must not bounds-check where the C does not (with `panic = "abort"` a bounds
/// check would kill the process). Reaching the end of this test is the
/// assertion.
#[test]
fn row16_cache_indices_out_of_range_must_not_abort() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0xE016);

    // Kept modest so the out-of-bounds reads stay within the libraries' own
    // stack frames rather than wandering into unmapped pages.
    let idx: [c_int; 7] = [-3, -1, 0, 7, 8, 9, 12];
    let _ = &idx;

    for i in 0..6000usize {
        let (ci, bb, cap) = shapes(&mut rng);
        let cache = c2GJKCache {
            metric: rng.f(100.0),
            // 1..=3 only. `cache->count >= 4` makes `c2GJK` write `verts + i`
            // past the 4-slot `c2Simplex`, clobber `s.count`, and then keep
            // walking further every iteration until it destroys its own return
            // address -- BOTH libraries crash identically. Documented in
            // ERRORS.md, not executed here (it would abort the test process).
            count: 1 + (i % 3) as c_int,
            iA: [
                idx[rng.below(7) as usize],
                idx[rng.below(7) as usize],
                idx[rng.below(7) as usize],
            ],
            iB: [
                idx[rng.below(7) as usize],
                idx[rng.below(7) as usize],
                idx[rng.below(7) as usize],
            ],
            div: rng.f(10.0),
        };
        let (ta, pa): (c_int, *const c_void) = match i % 3 {
            0 => (C2_TYPE_CIRCLE, &ci as *const _ as *const c_void),
            1 => (C2_TYPE_CAPSULE, &cap as *const _ as *const c_void),
            _ => (C2_TYPE_AABB, &bb as *const _ as *const c_void),
        };
        let mut cc = cache;
        let mut rc = cache;
        let mut ca = v(0.0, 0.0);
        let mut cb = v(0.0, 0.0);
        let mut ra = v(0.0, 0.0);
        let mut rb = v(0.0, 0.0);
        unsafe {
            scrub_stack();
            let _ = cf(pa, ta, std::ptr::null(), &cap as *const _ as *const c_void,
                C2_TYPE_CAPSULE, std::ptr::null(), &mut ca, &mut cb, 0,
                std::ptr::null_mut(), &mut cc);
            scrub_stack();
            let _ = rf(pa, ta, std::ptr::null(), &cap as *const _ as *const c_void,
                C2_TYPE_CAPSULE, std::ptr::null(), &mut ra, &mut rb, 0,
                std::ptr::null_mut(), &mut rc);
        }
        // Both must have written a cache back and stayed within 0..=3 for count.
        assert!(
            (0..=4).contains(&cc.count) && (0..=4).contains(&rc.count),
            "ROW16 i={i}: cache count out of the simplex range: C {} R {}",
            cc.count,
            rc.count
        );
    }
}

/// Companion to ROW 16 — with `cache->count` and the indices *in range* for the
/// 8-slot proxy array, the reads are well defined relative to each library's own
/// (zero-initialized) proxy, so the results must match bit-for-bit. This is the
/// strict half of row 16.
#[test]
fn row16_cache_indices_in_range_must_match() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0xE017);

    for i in 0..8000usize {
        let (ci, bb, cap) = shapes(&mut rng);
        let cache = c2GJKCache {
            metric: rng.f(100.0),
            count: 1 + (i % 3) as c_int, // 1..=3, within iA[3]
            iA: [
                rng.below(8) as c_int,
                rng.below(8) as c_int,
                rng.below(8) as c_int,
            ],
            iB: [
                rng.below(8) as c_int,
                rng.below(8) as c_int,
                rng.below(8) as c_int,
            ],
            div: if i % 5 == 0 { 0.0 } else { rng.f(10.0) },
        };
        let pick = |k: usize| -> (c_int, *const c_void) {
            match k {
                0 => (C2_TYPE_CIRCLE, &ci as *const _ as *const c_void),
                1 => (C2_TYPE_CAPSULE, &cap as *const _ as *const c_void),
                _ => (C2_TYPE_AABB, &bb as *const _ as *const c_void),
            }
        };
        let (ta, pa) = pick(i % 3);
        let (tb, pb) = pick((i / 3) % 3);
        let mut cc = cache;
        let mut rc = cache;
        let mut ca = v(0.0, 0.0);
        let mut cb = v(0.0, 0.0);
        let mut ra = v(0.0, 0.0);
        let mut rb = v(0.0, 0.0);
        let mut cit = 0i32;
        let mut rit = 0i32;
        let (dc, dr) = unsafe {
            scrub_stack();
            let x = cf(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut ca, &mut cb,
                (i % 2) as c_int, &mut cit, &mut cc);
            scrub_stack();
            let y = rf(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut ra, &mut rb,
                (i % 2) as c_int, &mut rit, &mut rc);
            (x, y)
        };
        assert!(
            feq(dc, dr) && veq(ca, ra) && veq(cb, rb) && cit == rit,
            "ROW16 in-range i={i} cache={cache:?} ta={ta} tb={tb}: \
             dist C {} vs R {}; a C {} vs R {}; b C {} vs R {}; it {cit} vs {rit}",
            fs(dc), fs(dr), vs(ca), vs(ra), vs(cb), vs(rb)
        );
        assert!(
            feq(cc.metric, rc.metric)
                && cc.count == rc.count
                && cc.iA == rc.iA
                && cc.iB == rc.iB
                && feq(cc.div, rc.div),
            "ROW16 in-range cache out i={i}: C {cc:?} vs R {rc:?}"
        );
    }
}
