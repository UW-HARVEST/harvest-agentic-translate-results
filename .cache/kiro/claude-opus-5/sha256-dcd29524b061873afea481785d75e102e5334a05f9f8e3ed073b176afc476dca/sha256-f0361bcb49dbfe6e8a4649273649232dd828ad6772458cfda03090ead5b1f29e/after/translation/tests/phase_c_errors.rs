//! Phase C — error / rejection differential tests, one per ERRORS.md row.
//! Every test constructs the exact invalid input or degenerate condition the C
//! source checks for, calls BOTH `.so`s, and asserts they reject/degrade
//! identically (same sentinel value, same collapse, same out-pointer write
//! pattern) — not merely "both failed somehow".

mod common;
use common::*;
use std::ffi::{c_int, c_void};

fn tri(rng: &mut Rng) -> (ShapeBuf, ShapeBuf) {
    (
        ShapeBuf::aabb(rng.aabb()),
        ShapeBuf::capsule(rng.capsule()),
    )
}

// ===========================================================================
// Rows 1-2: NULL transform pointers substitute c2xIdentity()
// ===========================================================================

#[test]
fn err01_err02_null_transform_pointers() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 101);
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    for i in 0..2000 {
        let (a, b) = tri(&mut rng);
        for ur in [1i32, 0] {
            // NULL ax must behave exactly like an explicit identity ax, in both libs
            for (tag, ax, bx) in [
                ("both-null", None, None),
                ("ax-null", None, Some(&ident)),
                ("bx-null", Some(&ident), None),
                ("neither-null", Some(&ident), Some(&ident)),
            ] {
                let co = call_gjk(c, &a, ax, &b, bx, ur, true, true, None);
                let ro = call_gjk(r, &a, ax, &b, bx, ur, true, true, None);
                assert_gjk_eq(&format!("err01/02 {tag} ur={ur} #{i}"), &co, &ro);
            }
            // and the NULL / explicit-identity results must agree with each other
            let n = call_gjk(c, &a, None, &b, None, ur, true, true, None);
            let e = call_gjk(c, &a, Some(&ident), &b, Some(&ident), ur, true, true, None);
            assert_feq("err01/02 C null==identity", n.dist, e.dist);
            let n = call_gjk(r, &a, None, &b, None, ur, true, true, None);
            let e = call_gjk(r, &a, Some(&ident), &b, Some(&ident), ur, true, true, None);
            assert_feq("err01/02 Rust null==identity", n.dist, e.dist);
        }
    }
}

// ===========================================================================
// Rows 3-7: NULL cache / outA / outB / iterations
// ===========================================================================

#[test]
fn err03_to_err07_null_out_pointers() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 103);
    for i in 0..1500 {
        let (a, b) = tri(&mut rng);
        for ur in [1i32, 0] {
            for want_out in [false, true] {
                for want_iters in [false, true] {
                    for cache in [None, Some(c2GJKCache::default())] {
                        let co = call_gjk(c, &a, None, &b, None, ur, want_out, want_iters, cache);
                        let ro = call_gjk(r, &a, None, &b, None, ur, want_out, want_iters, cache);
                        assert_gjk_eq(
                            &format!(
                                "err03-07 out={want_out} it={want_iters} cache={} ur={ur} #{i}",
                                cache.is_some()
                            ),
                            &co,
                            &ro,
                        );
                        // the NULL guards must not have written anything
                        assert_eq!(co.a_written, want_out, "err04 C outA write");
                        assert_eq!(ro.a_written, want_out, "err04 Rust outA write");
                        assert_eq!(co.iters_written, want_iters, "err06 C iterations write");
                        assert_eq!(ro.iters_written, want_iters, "err06 Rust iterations write");
                    }
                }
            }
            // row 7: everything NULL, only the float return exists
            let cd = unsafe {
                (c.c2GJK)(
                    a.ptr(),
                    a.ty,
                    std::ptr::null(),
                    b.ptr(),
                    b.ty,
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
                    a.ptr(),
                    a.ty,
                    std::ptr::null(),
                    b.ptr(),
                    b.ty,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    ur,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            assert_feq(&format!("err07 all-null ur={ur} #{i}"), cd, rd);
        }
    }
}

/// Independently exercise *one* out-pointer NULL while the other is live,
/// which the packed helper cannot express.
#[test]
fn err04_err05_asymmetric_out_pointers() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 104);
    let poison = c2v { x: -777.5, y: 333.25 };
    for i in 0..2000 {
        let (a, b) = tri(&mut rng);
        for (tag, want_a, want_b) in [("a-only", true, false), ("b-only", false, true)] {
            let mut ca = poison;
            let mut cb = poison;
            let mut ra = poison;
            let mut rb = poison;
            let cd = unsafe {
                (c.c2GJK)(
                    a.ptr(), a.ty, std::ptr::null(), b.ptr(), b.ty, std::ptr::null(),
                    if want_a { &mut ca } else { std::ptr::null_mut() },
                    if want_b { &mut cb } else { std::ptr::null_mut() },
                    1, std::ptr::null_mut(), std::ptr::null_mut(),
                )
            };
            let rd = unsafe {
                (r.c2GJK)(
                    a.ptr(), a.ty, std::ptr::null(), b.ptr(), b.ty, std::ptr::null(),
                    if want_a { &mut ra } else { std::ptr::null_mut() },
                    if want_b { &mut rb } else { std::ptr::null_mut() },
                    1, std::ptr::null_mut(), std::ptr::null_mut(),
                )
            };
            assert_feq(&format!("err04/05 {tag} dist #{i}"), cd, rd);
            assert_veq(&format!("err04/05 {tag} outA #{i}"), ca, ra);
            assert_veq(&format!("err04/05 {tag} outB #{i}"), cb, rb);
            if !want_a {
                assert!(veq(ca, poison) && veq(ra, poison), "err04 {tag}: outA was written");
            }
            if !want_b {
                assert!(veq(cb, poison) && veq(rb, poison), "err05 {tag}: outB was written");
            }
        }
    }
}

// ===========================================================================
// Rows 8-10: cache validity / the inverted metric test
// ===========================================================================

#[test]
fn err08_cache_count_zero_short_circuit() {
    let mut rng = Rng::new(SEED ^ 108);
    let (c, r) = (&libs().c, &libs().r);
    for i in 0..2000 {
        let (a, b) = tri(&mut rng);
        // count == 0 but every other field poisoned: must be ignored, and the
        // cache must still be *written* on exit.
        let dirty = c2GJKCache {
            metric: rng.wild(),
            count: 0,
            iA: [rng.below(4) as c_int, 2, 1],
            iB: [1, 0, 1],
            div: rng.wild(),
        };
        let co = call_gjk(c, &a, None, &b, None, 1, true, true, Some(dirty));
        let ro = call_gjk(r, &a, None, &b, None, 1, true, true, Some(dirty));
        assert_gjk_eq(&format!("err08 dirty-cold #{i}"), &co, &ro);
        // the exit write always sets count to the final simplex count
        assert_eq!(co.cache.count, ro.cache.count, "err08 cache count #{i}");
    }
}

#[test]
fn err09_negative_cache_count() {
    let mut rng = Rng::new(SEED ^ 109);
    let (c, r) = (&libs().c, &libs().r);
    for count in [-1i32, -2, -3, -100, i32::MIN] {
        for i in 0..400 {
            let (a, b) = tri(&mut rng);
            let forged = c2GJKCache {
                metric: if i % 3 == 0 { -1.0e9 } else { rng.coord() },
                count,
                iA: [0, 1, 2],
                iB: [0, 1, 0],
                div: if i % 4 == 0 { 0.0 } else { rng.coord() },
            };
            for ur in [1i32, 0] {
                let co = call_gjk(c, &a, None, &b, None, ur, true, true, Some(forged));
                let ro = call_gjk(r, &a, None, &b, None, ur, true, true, Some(forged));
                assert_gjk_eq(
                    &format!("err09 negative cache count={count} ur={ur} #{i}"),
                    &co,
                    &ro,
                );
                // The C's degenerate path returns exactly 0.0 with both witnesses
                // at the origin; assert that sentinel explicitly on both sides.
                assert_eq!(co.dist.to_bits(), 0u32, "err09 C sentinel dist");
                assert_eq!(ro.dist.to_bits(), 0u32, "err09 Rust sentinel dist");
                assert_eq!(co.cache.count, count, "err09 C rewrote count");
                assert_eq!(ro.cache.count, count, "err09 Rust rewrote count");
            }
        }
    }
}

/// Row 10: the C's cache-validity test is
/// `!(min_metric < max_metric * 2.0f && metric < -1.0e8f)`.
/// The `metric < -1.0e8f` conjunct makes acceptance near-unconditional; the
/// only way to make it *false* is a metric below -1e8, which for a 2-vertex
/// simplex (`c2Len`, always >= 0) is impossible and for a 3-vertex simplex
/// (`c2Det2`) needs a huge negative determinant. Both regimes are forced here.
#[test]
fn err10_inverted_cache_validity_test() {
    let mut rng = Rng::new(SEED ^ 110);
    let (c, r) = (&libs().c, &libs().r);
    let metrics = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        -9.9e7,      // just above the -1e8 threshold
        -1.0e8,      // exactly at it (not <, so still accepted)
        -1.000_001e8, // just below it
        -1.0e9,
        -f32::MAX,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            let (na, nb) = (
                match ta { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 },
                match tb { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 },
            );
            for count in [1i32, 2, 3] {
                for (mi, &metric) in metrics.iter().enumerate() {
                    for i in 0..12 {
                        let a = rand_shape(&mut rng, ta);
                        let b = rand_shape(&mut rng, tb);
                        let forged = c2GJKCache {
                            metric,
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
                            div: [1.0f32, 0.0, -1.0, 1e30, f32::NAN][i % 5],
                        };
                        for ur in [1i32, 0] {
                            let co = call_gjk(c, &a, None, &b, None, ur, true, true, Some(forged));
                            let ro = call_gjk(r, &a, None, &b, None, ur, true, true, Some(forged));
                            assert_gjk_eq(
                                &format!(
                                    "err10 count={count} metric#{mi}={metric:?} ur={ur} #{i}"
                                ),
                                &co,
                                &ro,
                            );
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Rows 14-22: use_radius, the hit path and the loop early-outs
// ===========================================================================

#[test]
fn err14_use_radius_disabled() {
    let mut rng = Rng::new(SEED ^ 114);
    let (c, r) = (&libs().c, &libs().r);
    // Every out-of-range int is truthy except 0, so use_radius must behave as a
    // plain boolean across the FFI boundary.
    for ur in [0i32, 1, 2, -1, i32::MIN, i32::MAX, 0x1000] {
        for i in 0..600 {
            let (a, b) = tri(&mut rng);
            let co = call_gjk(c, &a, None, &b, None, ur, true, true, None);
            let ro = call_gjk(r, &a, None, &b, None, ur, true, true, None);
            assert_gjk_eq(&format!("err14 use_radius={ur} #{i}"), &co, &ro);
            if ur != 0 {
                let c1 = call_gjk(c, &a, None, &b, None, 1, true, true, None);
                assert_feq("err14 truthy == 1", co.dist, c1.dist);
                let r1 = call_gjk(r, &a, None, &b, None, 1, true, true, None);
                assert_feq("err14 truthy == 1 (rust)", ro.dist, r1.dist);
            }
        }
    }
}

#[test]
fn err15_to_err18_radius_collapse_and_hit() {
    let mut rng = Rng::new(SEED ^ 115);
    let (c, r) = (&libs().c, &libs().r);
    let mut collapsed = 0usize;
    let mut hits = 0usize;
    // Circle-circle lets us dial `dist` against `rA + rB` precisely.
    for i in 0..8000 {
        let r_a = (i % 5) as f32 * 0.5;
        let r_b = ((i / 5) % 5) as f32 * 0.5;
        // separations straddling rA+rB, plus the exact boundary
        let sep = match i % 9 {
            0 => 0.0,
            1 => r_a + r_b,
            2 => (r_a + r_b) * 0.999_999,
            3 => (r_a + r_b) * 1.000_001,
            4 => 1.192_092_9e-7,
            5 => 1.192_092_8e-7,
            6 => f32::MIN_POSITIVE,
            7 => rng.unit() * (r_a + r_b + 0.001),
            _ => rng.unit() * 10.0,
        };
        let a = ShapeBuf::circle(c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: r_a,
        });
        let b = ShapeBuf::circle(c2Circle {
            p: c2v { x: sep, y: 0.0 },
            r: r_b,
        });
        for ur in [1i32, 0] {
            let co = call_gjk(c, &a, None, &b, None, ur, true, true, None);
            let ro = call_gjk(r, &a, None, &b, None, ur, true, true, None);
            assert_gjk_eq(
                &format!("err15-18 circles rA={r_a} rB={r_b} sep={sep:?} ur={ur} #{i}"),
                &co,
                &ro,
            );
            if ur == 1 && co.dist == 0.0 {
                collapsed += 1;
            }
            if ur == 0 && co.dist == 0.0 && veq(co.a, co.b) {
                hits += 1;
            }
        }
    }
    assert!(collapsed > 0, "err15/16/17 radius-collapse never observed");
    // The `hit` path (row 18) needs real penetration; drive it with boxes.
    for i in 0..3000 {
        let a = ShapeBuf::aabb(c2AABB {
            min: c2v { x: -2.0, y: -2.0 },
            max: c2v { x: 2.0, y: 2.0 },
        });
        let d = rng.sym(1.5);
        let b = ShapeBuf::aabb(c2AABB {
            min: c2v { x: d - 1.0, y: d - 1.0 },
            max: c2v { x: d + 1.0, y: d + 1.0 },
        });
        for ur in [1i32, 0] {
            let co = call_gjk(c, &a, None, &b, None, ur, true, true, None);
            let ro = call_gjk(r, &a, None, &b, None, ur, true, true, None);
            assert_gjk_eq(&format!("err18 penetrating boxes ur={ur} #{i}"), &co, &ro);
            if co.dist == 0.0 && veq(co.a, co.b) {
                hits += 1;
                // row 18: on `hit` the C sets `a = b`, so both witnesses match
                assert!(veq(co.a, co.b) && veq(ro.a, ro.b), "err18 a != b on hit");
            }
        }
    }
    assert!(hits > 0, "err18 hit path never observed");
}

/// Rows 19-22: the loop early-outs. `iter` is only incremented at the very end
/// of a successful iteration, so any `break` leaves it at the pre-iteration
/// value — that quirk is asserted by comparing the reported `iterations`.
#[test]
fn err19_to_err22_loop_early_outs() {
    let mut rng = Rng::new(SEED ^ 119);
    let (c, r) = (&libs().c, &libs().r);
    let mut seen = [0usize; 4];
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for i in 0..2000 {
                let a = rand_shape(&mut rng, ta);
                let b = rand_shape(&mut rng, tb);
                for ur in [1i32, 0] {
                    let co = call_gjk(c, &a, None, &b, None, ur, true, true, Some(c2GJKCache::default()));
                    let ro = call_gjk(r, &a, None, &b, None, ur, true, true, Some(c2GJKCache::default()));
                    assert_gjk_eq(&format!("err19-22 early-out #{i}"), &co, &ro);
                    if co.iters >= 0 && co.iters < 4 {
                        seen[co.iters as usize] += 1;
                    }
                    // row 22: the cap is never exceeded
                    assert!(co.iters <= 20 && ro.iters <= 20, "err22 iteration cap");
                }
            }
        }
    }
    // Coincident shapes drive the degenerate-search-direction break (row 20).
    for i in 0..2000 {
        let s = rand_shape(&mut rng, ALL_TYPES[i % 3]);
        for ur in [1i32, 0] {
            let co = call_gjk(c, &s, None, &s, None, ur, true, true, Some(c2GJKCache::default()));
            let ro = call_gjk(r, &s, None, &s, None, ur, true, true, Some(c2GJKCache::default()));
            assert_gjk_eq(&format!("err20 coincident #{i}"), &co, &ro);
        }
    }
    assert!(seen[0] > 0, "err19-21: no zero-iteration break observed");
    eprintln!("err19-22 iteration histogram (0..3): {seen:?}");
}

// ===========================================================================
// Row 13 / 23: out-of-range C2_TYPE across the FFI boundary
// ===========================================================================

/// Row 23 — `c2MakeProxy`'s `switch` has no `default:`, so an out-of-range enum
/// value must leave the caller's `c2Proxy` byte-for-byte untouched. This is
/// fully defined behaviour and is asserted strictly.
#[test]
fn err23_c2MakeProxy_out_of_range_enum() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 123);
    let bad: Vec<c_int> = vec![
        -1, 3, 4, 5, 100, -100, i32::MIN, i32::MAX, 0x8000_0000u32 as c_int, 255, 256, 1 << 16,
    ];
    for &ty in &bad {
        for i in 0..300 {
            // a poisoned proxy so "untouched" is observable
            let mut base = c2Proxy {
                radius: f32::from_bits(0xDEAD_BEEF),
                count: -0x0BAD_F00D,
                verts: [c2v::default(); 8],
            };
            for k in 0..8 {
                base.verts[k] = c2v {
                    x: f32::from_bits(0xFEED_0000 | k as u32),
                    y: rng.coord(),
                };
            }
            let shape = rand_shape(&mut rng, ALL_TYPES[i % 3]);
            let mut cp = base;
            let mut rp = base;
            unsafe {
                (c.c2MakeProxy)(shape.ptr(), ty, &mut cp);
                (r.c2MakeProxy)(shape.ptr(), ty, &mut rp);
            }
            assert!(
                proxyeq(&cp, &rp),
                "err23 type={ty}: C={cp:?} != Rust={rp:?}"
            );
            // and it really is untouched
            assert!(
                proxyeq(&cp, &base),
                "err23 type={ty}: C modified the proxy: {cp:?} vs {base:?}"
            );
            assert!(
                proxyeq(&rp, &base),
                "err23 type={ty}: Rust modified the proxy: {rp:?} vs {base:?}"
            );
        }
    }
    // Every *valid* value must of course still be handled.
    for &ty in &ALL_TYPES {
        let shape = rand_shape(&mut rng, ty);
        let mut cp = c2Proxy::default();
        let mut rp = c2Proxy::default();
        unsafe {
            (c.c2MakeProxy)(shape.ptr(), ty, &mut cp);
            (r.c2MakeProxy)(shape.ptr(), ty, &mut rp);
        }
        assert!(proxyeq(&cp, &rp), "err23 valid type={ty}");
        assert!(cp.count > 0, "err23 valid type={ty} produced no verts");
    }
}

/// Row 13 — `c2GJK` with an out-of-range `C2_TYPE`. The C leaves its
/// `c2Proxy pA;` / `pB;` **uninitialised on its own stack**, so the result is
/// indeterminate by C's own rules and a value comparison would be meaningless.
/// What *is* checkable, and what this test asserts, is that neither library
/// aborts or traps: the call is isolated in a child process so a segfault in
/// either one is observed rather than killing the suite.
#[test]
fn err13_c2GJK_out_of_range_enum_isolated() {
    // child mode
    if std::env::var("GJK_ERR13_CHILD").is_ok() {
        let which = std::env::var("GJK_ERR13_CHILD").unwrap();
        let api = if which == "c" { &libs().c } else { &libs().r };
        let mut rng = Rng::new(SEED ^ 113);
        for &ty in &[-1i32, 3, 4, 99, i32::MIN, i32::MAX] {
            for i in 0..50 {
                let good = rand_shape(&mut rng, ALL_TYPES[i % 3]);
                let mut oa = c2v::default();
                let mut ob = c2v::default();
                let mut it: c_int = 0;
                for (ta, tb) in [(ty, good.ty), (good.ty, ty), (ty, ty)] {
                    let d = unsafe {
                        (api.c2GJK)(
                            good.ptr(),
                            ta,
                            std::ptr::null(),
                            good.ptr(),
                            tb,
                            std::ptr::null(),
                            &mut oa,
                            &mut ob,
                            1,
                            &mut it,
                            std::ptr::null_mut(),
                        )
                    };
                    // Consume the value so it cannot be optimised away.
                    std::hint::black_box(d);
                }
            }
        }
        println!("SURVIVED");
        return;
    }

    let exe = std::env::current_exe().expect("current_exe");
    let mut results = Vec::new();
    for which in ["c", "r"] {
        let out = std::process::Command::new(&exe)
            .arg("err13_c2GJK_out_of_range_enum_isolated")
            .arg("--exact")
            .arg("--nocapture")
            .env("GJK_ERR13_CHILD", which)
            .output()
            .expect("spawn child");
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        results.push((which, out.status.success(), stdout.contains("SURVIVED")));
    }
    // Both sides must agree on whether this UB input is survivable; if one
    // crashes and the other does not, that is a genuine behavioural difference.
    assert_eq!(
        (results[0].1, results[0].2),
        (results[1].1, results[1].2),
        "err13: C and Rust disagree on surviving an out-of-range C2_TYPE: {results:?}"
    );
    eprintln!(
        "err13 (documented UB — value not compared): C survived={} Rust survived={}",
        results[0].2, results[1].2
    );
}

// ===========================================================================
// Rows 24-33: simplex-layer degenerate handling
// ===========================================================================

#[test]
fn err24_err25_metric_default_label() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 124);
    // The C is `default: case 1: return 0;` — every count that is not 2 or 3
    // must return exactly +0.0 with the same bit pattern.
    for count in [1i32, 0, -1, -2, 4, 5, 6, 7, 100, i32::MIN, i32::MAX] {
        for i in 0..200 {
            let d = rng.wild();
            let mut cs = rand_simplex(&mut rng, count, d, true);
            let mut rs = cs;
            let cv = unsafe { (c.c2GJKSimplexMetric)(&mut cs) };
            let rv = unsafe { (r.c2GJKSimplexMetric)(&mut rs) };
            assert_feq(&format!("err24/25 count={count} #{i}"), cv, rv);
            assert_eq!(
                cv.to_bits(),
                0u32,
                "err24/25 count={count}: C did not return +0.0"
            );
            assert_eq!(
                rv.to_bits(),
                0u32,
                "err24/25 count={count}: Rust did not return +0.0"
            );
            assert!(simplexeq(&cs, &rs), "err24/25 simplex mutated");
        }
    }
}

#[test]
fn err26_err27_c2D_default_label() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 126);
    for count in [3i32, 0, -1, 4, 5, 99, i32::MIN, i32::MAX] {
        for i in 0..200 {
            let d = rng.wild();
            let mut cs = rand_simplex(&mut rng, count, d, true);
            let mut rs = cs;
            let cv = unsafe { (c.c2D)(&mut cs) };
            let rv = unsafe { (r.c2D)(&mut rs) };
            assert_veq(&format!("err26/27 count={count} #{i}"), cv, rv);
            assert_eq!(
                (cv.x.to_bits(), cv.y.to_bits()),
                (0, 0),
                "err26/27 count={count}: C did not return (+0,+0)"
            );
            assert_eq!(
                (rv.x.to_bits(), rv.y.to_bits()),
                (0, 0),
                "err26/27 count={count}: Rust did not return (+0,+0)"
            );
        }
    }
}

#[test]
fn err28_c2D_det_exactly_zero_picks_ccw90() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 128);
    let mut zero_det = 0usize;
    for i in 0..3000 {
        // a, b and the origin collinear => det2(ab, -a) == ±0 => `> 0` false
        let d = rng.coord_v();
        let (t0, t1) = (rng.sym(4.0), rng.sym(4.0));
        let mut s = rand_simplex(&mut rng, 2, 1.0, false);
        s.verts[0].p = c2v { x: d.x * t0, y: d.y * t0 };
        s.verts[1].p = c2v { x: d.x * t1, y: d.y * t1 };
        let mut cs = s;
        let mut rs = s;
        let cv = unsafe { (c.c2D)(&mut cs) };
        let rv = unsafe { (r.c2D)(&mut rs) };
        assert_veq(&format!("err28 collinear #{i}"), cv, rv);
        // Decide the expected branch from the determinant the C actually
        // computes (float rounding means `d*t` is not exactly collinear).
        let ab = c2v {
            x: s.verts[1].p.x - s.verts[0].p.x,
            y: s.verts[1].p.y - s.verts[0].p.y,
        };
        let det = (c.c2Det2)(ab, (c.c2Neg)(s.verts[0].p));
        let expected = if det > 0.0 {
            (c.c2Skew)(ab)
        } else {
            (c.c2CCW90)(ab)
        };
        assert_veq(&format!("err28 branch (det={det:?}) #{i}"), cv, expected);
        if det == 0.0 {
            zero_det += 1;
            assert_veq(
                &format!("err28 det==0 must take CCW90 #{i}"),
                cv,
                (c.c2CCW90)(ab),
            );
        }
    }
    assert!(
        zero_det > 0,
        "err28: an exactly-zero determinant was never produced"
    );
    eprintln!("err28: exactly-zero determinants exercised: {zero_det}");
}

#[test]
fn err29_to_err31_c2Witness_default_and_div_zero() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 129);
    let poison = c2v { x: 4242.0, y: -2424.0 };
    // row 29: default label => both outputs (+0,+0)
    for count in [0i32, -1, 4, 5, 77, i32::MIN, i32::MAX] {
        for i in 0..200 {
            let d = rng.wild();
            let mut cs = rand_simplex(&mut rng, count, d, true);
            let mut rs = cs;
            let (mut ca, mut cb, mut ra, mut rb) = (poison, poison, poison, poison);
            unsafe {
                (c.c2Witness)(&mut cs, &mut ca, &mut cb);
                (r.c2Witness)(&mut rs, &mut ra, &mut rb);
            }
            assert_veq(&format!("err29 count={count} outA #{i}"), ca, ra);
            assert_veq(&format!("err29 count={count} outB #{i}"), cb, rb);
            assert_eq!(
                (ca.x.to_bits(), ca.y.to_bits(), cb.x.to_bits(), cb.y.to_bits()),
                (0, 0, 0, 0),
                "err29 count={count}: C default label did not zero the outputs"
            );
            assert_eq!(
                (ra.x.to_bits(), ra.y.to_bits(), rb.x.to_bits(), rb.y.to_bits()),
                (0, 0, 0, 0),
                "err29 count={count}: Rust default label did not zero the outputs"
            );
        }
    }
    // rows 30/31: div == 0
    for &div in &[0.0f32, -0.0] {
        for count in [1i32, 2, 3] {
            for i in 0..500 {
                let mut cs = rand_simplex(&mut rng, count, div, false);
                let mut rs = cs;
                let (mut ca, mut cb, mut ra, mut rb) = (poison, poison, poison, poison);
                unsafe {
                    (c.c2Witness)(&mut cs, &mut ca, &mut cb);
                    (r.c2Witness)(&mut rs, &mut ra, &mut rb);
                }
                assert_veq(&format!("err30/31 div={div:?} count={count} outA #{i}"), ca, ra);
                assert_veq(&format!("err30/31 div={div:?} count={count} outB #{i}"), cb, rb);
                if count == 1 {
                    // row 31: den is unused, so this is an exact copy
                    assert_veq("err31 count=1 exact copy A", ca, cs.verts[0].sA);
                    assert_veq("err31 count=1 exact copy B", cb, cs.verts[0].sB);
                }
            }
        }
    }
}

#[test]
fn err32_err33_c2L_default_and_div_zero() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 132);
    for count in [3i32, 0, -1, 4, 66, i32::MIN, i32::MAX] {
        for i in 0..200 {
            let d = rng.wild();
            let mut cs = rand_simplex(&mut rng, count, d, true);
            let mut rs = cs;
            let cv = unsafe { (c.c2L)(&mut cs) };
            let rv = unsafe { (r.c2L)(&mut rs) };
            assert_veq(&format!("err32 count={count} #{i}"), cv, rv);
            assert_eq!(
                (cv.x.to_bits(), cv.y.to_bits()),
                (0, 0),
                "err32 count={count}: C default label did not return (+0,+0)"
            );
        }
    }
    for &div in &[0.0f32, -0.0] {
        for i in 0..1000 {
            let mut cs = rand_simplex(&mut rng, 2, div, false);
            let mut rs = cs;
            let cv = unsafe { (c.c2L)(&mut cs) };
            let rv = unsafe { (r.c2L)(&mut rs) };
            assert_veq(&format!("err33 div={div:?} #{i}"), cv, rv);
        }
    }
}

// ===========================================================================
// Rows 34-42: leaf-function degenerate handling
// ===========================================================================

#[test]
fn err34_c2Support_nonpositive_count() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 134);
    for count in [0i32, -1, -2, -1000, i32::MIN] {
        for i in 0..500 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.wild_v();
            }
            let d = rng.wild_v();
            let ci = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
            let ri = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
            assert_eq!(ci, ri, "err34 count={count} #{i}");
            assert_eq!(ci, 0, "err34 count={count}: C did not return the 0 sentinel");
            assert_eq!(ri, 0, "err34 count={count}: Rust did not return the 0 sentinel");
        }
    }
}

#[test]
fn err35_err36_c2Support_ties_and_nan() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 135);
    // row 35: strict `>` keeps the lowest index on a tie
    for count in [1i32, 2, 3, 4, 8] {
        for i in 0..500 {
            let v = rng.coord_v();
            let verts = [v; 8];
            let d = rng.coord_v();
            let ci = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
            let ri = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
            assert_eq!(ci, ri, "err35 ties count={count} #{i}");
            if !(v.x.is_nan() || v.y.is_nan() || d.x.is_nan() || d.y.is_nan()) {
                assert_eq!(ci, 0, "err35: tie did not keep the lowest index");
            }
        }
    }
    // row 36: NaN in `d` => every comparison false => index 0
    for count in [1i32, 2, 4, 8] {
        for i in 0..200 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.coord_v();
            }
            for d in [
                c2v { x: f32::NAN, y: 0.0 },
                c2v { x: 0.0, y: f32::NAN },
                c2v { x: f32::NAN, y: f32::NAN },
                c2v { x: -f32::NAN, y: -f32::NAN },
            ] {
                let ci = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
                let ri = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
                assert_eq!(ci, ri, "err36 nan-d count={count} #{i}");
                assert_eq!(ci, 0, "err36: NaN direction did not return 0");
            }
        }
    }
}

#[test]
fn err37_err38_c2Div_by_zero() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 137);
    for &b in &[0.0f32, -0.0] {
        for i in 0..2000 {
            let a = rng.wild_v();
            let cv = (c.c2Div)(a, b);
            let rv = (r.c2Div)(a, b);
            assert_veq(&format!("err37/38 c2Div a={a:?} b={b:?} #{i}"), cv, rv);
        }
        // explicit sign cross-product
        for &x in &[1.0f32, -1.0, 0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY] {
            for &y in &[1.0f32, -1.0, 0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY] {
                let a = c2v { x, y };
                let cv = (c.c2Div)(a, b);
                let rv = (r.c2Div)(a, b);
                assert_veq(&format!("err37/38 c2Div explicit a={a:?} b={b:?}"), cv, rv);
            }
        }
    }
}

#[test]
fn err39_err40_c2Norm_zero_and_c2Len_extremes() {
    let (c, r) = (&libs().c, &libs().r);
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ] {
        let cv = (c.c2Norm)(a);
        let rv = (r.c2Norm)(a);
        assert_veq(&format!("err39 c2Norm zero {a:?}"), cv, rv);
        assert!(
            cv.x.is_nan() && cv.y.is_nan(),
            "err39: C c2Norm((0,0)) was not NaN: {cv:?}"
        );
        assert!(
            rv.x.is_nan() && rv.y.is_nan(),
            "err39: Rust c2Norm((0,0)) was not NaN: {rv:?}"
        );
    }
    for a in [
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MAX, y: 0.0 },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: f32::MIN_POSITIVE, y: f32::MIN_POSITIVE },
        c2v { x: f32::from_bits(1), y: f32::from_bits(1) },
    ] {
        assert_feq(&format!("err40 c2Len {a:?}"), (c.c2Len)(a), (r.c2Len)(a));
        assert_veq(&format!("err40 c2Norm {a:?}"), (c.c2Norm)(a), (r.c2Norm)(a));
    }
}

#[test]
fn err41_c2Maxv_c2Minv_nan_returns_b() {
    let (c, r) = (&libs().c, &libs().r);
    let nans = [f32::NAN, -f32::NAN, f32::from_bits(0x7FC0_1234)];
    for &n in &nans {
        for &other in &[0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY] {
            // NaN in a
            let a = c2v { x: n, y: n };
            let b = c2v { x: other, y: other };
            let cmax = (c.c2Maxv)(a, b);
            let rmax = (r.c2Maxv)(a, b);
            let cmin = (c.c2Minv)(a, b);
            let rmin = (r.c2Minv)(a, b);
            assert_veq("err41 c2Maxv nan-in-a", cmax, rmax);
            assert_veq("err41 c2Minv nan-in-a", cmin, rmin);
            assert_veq("err41 c2Maxv nan-in-a picks b", cmax, b);
            assert_veq("err41 c2Minv nan-in-a picks b", cmin, b);
            // NaN in b
            let cmax = (c.c2Maxv)(b, a);
            let rmax = (r.c2Maxv)(b, a);
            let cmin = (c.c2Minv)(b, a);
            let rmin = (r.c2Minv)(b, a);
            assert_veq("err41 c2Maxv nan-in-b", cmax, rmax);
            assert_veq("err41 c2Minv nan-in-b", cmin, rmin);
            assert!(cmax.x.is_nan() && cmin.x.is_nan(), "err41: NaN in b must be returned");
        }
    }
}

#[test]
fn err42_c2Clampv_inverted_range() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 142);
    for i in 0..3000 {
        let a = rng.wild_v();
        let p = rng.coord_v();
        let q = rng.coord_v();
        // deliberately inverted: lo > hi
        let lo = c2v { x: p.x.max(q.x), y: p.y.max(q.y) };
        let hi = c2v { x: p.x.min(q.x), y: p.y.min(q.y) };
        let cv = (c.c2Clampv)(a, lo, hi);
        let rv = (r.c2Clampv)(a, lo, hi);
        assert_veq(&format!("err42 inverted range #{i}"), cv, rv);
        if !(a.x.is_nan() || a.y.is_nan() || lo.x.is_nan() || lo.y.is_nan()) && lo.x > hi.x {
            assert!(feq(cv.x, lo.x), "err42: inverted clamp did not return lo.x");
        }
    }
}

// ===========================================================================
// Rows 43-53: c22 / c23 collapse branches, hit explicitly
// ===========================================================================

fn c22_diff(ctx: &str, a: c2v, b: c2v, rng: &mut Rng) -> c2Simplex {
    let (c, r) = (&libs().c, &libs().r);
    let d = rng.coord();
    let mut s = rand_simplex(rng, 2, d, false);
    s.verts[0].p = a;
    s.verts[1].p = b;
    let mut cs = s;
    let mut rs = s;
    unsafe {
        (c.c22)(&mut cs);
        (r.c22)(&mut rs);
    }
    assert!(simplexeq(&cs, &rs), "{ctx}:\n in={s:?}\n C={cs:?}\n R={rs:?}");
    cs
}

#[test]
fn err43_to_err45_c22_collapse_branches() {
    let mut rng = Rng::new(SEED ^ 143);
    let mut branch = [0usize; 3];
    for i in 0..4000 {
        // v <= 0  =>  origin is "beyond" a  =>  collapse to a
        let dir = rng.coord_v();
        let t = 1.0 + rng.unit() * 3.0;
        let a = dir;
        let b = c2v { x: dir.x * t, y: dir.y * t };
        let out = c22_diff(&format!("err43 v<=0 #{i}"), a, b, &mut rng);
        if out.count == 1 {
            branch[0] += 1;
        }
        // u <= 0  =>  collapse to b (with s->a = s->b)
        let out = c22_diff(&format!("err44 u<=0 #{i}"), b, a, &mut rng);
        if out.count == 1 {
            branch[1] += 1;
        }
        // interior
        let a = c2v { x: -dir.x, y: -dir.y };
        let out = c22_diff(&format!("err43-45 interior #{i}"), a, dir, &mut rng);
        if out.count == 2 {
            branch[2] += 1;
        }
        // row 45: duplicate points => u == v == 0 => the `v <= 0` branch wins
        let p = rng.coord_v();
        let out = c22_diff(&format!("err45 duplicate #{i}"), p, p, &mut rng);
        assert_eq!(out.count, 1, "err45: duplicate points did not collapse");
        assert_eq!(out.div.to_bits(), 1.0f32.to_bits(), "err45: div != 1.0");
        // both points at the origin
        let z = c2v { x: 0.0, y: 0.0 };
        c22_diff(&format!("err45 both-origin #{i}"), z, z, &mut rng);
    }
    assert!(branch.iter().all(|&b| b > 0), "err43-45 branches: {branch:?}");
}

fn c23_diff(ctx: &str, ps: [c2v; 3], rng: &mut Rng) -> c2Simplex {
    let (c, r) = (&libs().c, &libs().r);
    let d = rng.coord();
    let mut s = rand_simplex(rng, 3, d, false);
    for k in 0..3 {
        s.verts[k].p = ps[k];
    }
    let mut cs = s;
    let mut rs = s;
    unsafe {
        (c.c23)(&mut cs);
        (r.c23)(&mut rs);
    }
    assert!(simplexeq(&cs, &rs), "{ctx}:\n in={s:?}\n C={cs:?}\n R={rs:?}");
    cs
}

#[test]
fn err46_to_err53_c23_collapse_branches() {
    let c = &libs().c;
    let mut rng = Rng::new(SEED ^ 146);
    let mut counts = [0usize; 4]; // index = resulting simplex count
    let mut zero_div = 0usize;
    let mut exact_zero_area = 0usize;
    for i in 0..6000 {
        // random triangles reach the vertex/edge/interior collapses
        let mut ps = [c2v::default(); 3];
        for p in ps.iter_mut() {
            *p = rng.coord_v();
        }
        let out = c23_diff(&format!("err46-51 random #{i}"), ps, &mut rng);
        if (0..4).contains(&out.count) {
            counts[out.count as usize] += 1;
        }
        if out.count == 3 && out.div == 0.0 {
            zero_div += 1;
        }

        // rows 52/53: exactly-degenerate (collinear) triangles => area == 0 =>
        // uABC = vABC = wABC = 0. Measured invariant: with a bit-exactly zero
        // area the ladder ALWAYS matches an earlier vertex/edge branch, so the
        // interior branch is never taken (verified over 400k exact-zero-area
        // triples). Asserted on both libraries.
        let o = rng.coord_v();
        let d = rng.coord_v();
        let mk = |t: f32| c2v { x: o.x + d.x * t, y: o.y + d.y * t };
        let (t0, t1, t2) = (rng.sym(4.0), rng.sym(4.0), rng.sym(4.0));
        let out = c23_diff(&format!("err52 collinear #{i}"), [mk(t0), mk(t1), mk(t2)], &mut rng);
        let (pa, pb, pc) = (mk(t0), mk(t1), mk(t2));
        let area = (c.c2Det2)((c.c2Sub)(pb, pa), (c.c2Sub)(pc, pa));
        if area == 0.0 {
            exact_zero_area += 1;
            assert!(
                out.count == 1 || out.count == 2,
                "err52: area==0 reached the interior branch (count={}, div={:?})",
                out.count,
                out.div
            );
        }
        // Exactly-zero-area triples built on an axis (multiplication-free, so
        // the determinant cancels bit-exactly).
        for axis in 0..3u32 {
            let f = |r: &mut Rng| {
                let t = r.below(21) as f32 - 10.0;
                match axis {
                    0 => c2v { x: t, y: 0.0 },
                    1 => c2v { x: 0.0, y: t },
                    _ => c2v { x: t, y: t },
                }
            };
            let (qa, qb, qc) = (f(&mut rng), f(&mut rng), f(&mut rng));
            let ar = (c.c2Det2)((c.c2Sub)(qb, qa), (c.c2Sub)(qc, qa));
            assert_eq!(ar, 0.0, "err52 axis construction was not exactly degenerate");
            let out = c23_diff(&format!("err52 axis={axis} #{i}"), [qa, qb, qc], &mut rng);
            exact_zero_area += 1;
            assert!(
                out.count == 1 || out.count == 2,
                "err52: exact-zero-area axis triple reached the interior branch (count={})",
                out.count
            );
        }
        // all three points identical
        let p = rng.coord_v();
        c23_diff(&format!("err52 all-same #{i}"), [p, p, p], &mut rng);
        // a point exactly at the origin, in each slot
        let z = c2v { x: 0.0, y: 0.0 };
        c23_diff(&format!("err52 origin0 #{i}"), [z, ps[1], ps[2]], &mut rng);
        c23_diff(&format!("err52 origin1 #{i}"), [ps[0], z, ps[2]], &mut rng);
        c23_diff(&format!("err52 origin2 #{i}"), [ps[0], ps[1], z], &mut rng);
    }
    assert!(counts[1] > 0, "err46-48: no vertex collapse observed");
    assert!(counts[2] > 0, "err49-51: no edge collapse observed");
    assert!(counts[3] > 0, "err53: no interior result observed");
    assert!(
        exact_zero_area > 0,
        "err52: no bit-exactly-degenerate triangle was constructed"
    );

    // Row 53: the interior branch with `div == 0`. Reachable only through
    // subnormal/overflow underflow, where uABC, vABC and wABC all flush to
    // zero; these three inputs were found by exhaustive search over 2e6 random
    // triples and are pinned here so the row stays covered.
    let underflow: [[c2v; 3]; 3] = [
        [
            c2v { x: 1.1754944e-38, y: 1.1920929e-7 },
            c2v { x: 1.1754944e-38, y: -1e-45 },
            c2v { x: 1.1920929e-7, y: 3.4028235e38 },
        ],
        [
            c2v { x: 1e-45, y: -6.5475816e-16 },
            c2v { x: 1e-45, y: 1.1754944e-38 },
            c2v { x: 1.1920929e-7, y: -3.4028235e38 },
        ],
        [
            c2v { x: -1.1754944e-38, y: 1.2124317e-19 },
            c2v { x: -1.1754944e-38, y: -1e-45 },
            c2v { x: -0.07970071, y: 3.4028235e38 },
        ],
    ];
    for (k, ps) in underflow.iter().enumerate() {
        for _ in 0..50 {
            let out = c23_diff(&format!("err53 interior div==0 #{k}"), *ps, &mut rng);
            assert_eq!(out.count, 3, "err53 #{k}: expected the interior branch");
            assert_eq!(out.div, 0.0, "err53 #{k}: expected div == 0, got {:?}", out.div);
            zero_div += 1;
        }
    }
    assert!(zero_div > 0, "err53: interior branch with div == 0 never produced");
    eprintln!(
        "err46-53 result-count histogram: {counts:?}, exact-zero-area cases: {exact_zero_area}, interior div==0 cases: {zero_div}"
    );
}

// ===========================================================================
// Row 54: c2BBVerts with no validation
// ===========================================================================

#[test]
fn err54_c2BBVerts_inverted_box() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 154);
    for i in 0..3000 {
        let p = rng.coord_v();
        let q = rng.coord_v();
        // force min > max on both axes
        let mut bb = c2AABB {
            min: c2v { x: p.x.max(q.x), y: p.y.max(q.y) },
            max: c2v { x: p.x.min(q.x), y: p.y.min(q.y) },
        };
        let mut bb2 = bb;
        let mut co = [c2v { x: 9.0, y: 9.0 }; 4];
        let mut ro = co;
        unsafe {
            (c.c2BBVerts)(co.as_mut_ptr(), &mut bb);
            (r.c2BBVerts)(ro.as_mut_ptr(), &mut bb2);
        }
        for k in 0..4 {
            assert_veq(&format!("err54 inverted out[{k}] #{i}"), co[k], ro[k]);
        }
        // and via the proxy path
        let s = ShapeBuf::aabb(bb);
        let mut cp = c2Proxy::default();
        let mut rp = c2Proxy::default();
        unsafe {
            (c.c2MakeProxy)(s.ptr(), C2_TYPE_AABB, &mut cp);
            (r.c2MakeProxy)(s.ptr(), C2_TYPE_AABB, &mut rp);
        }
        assert!(proxyeq(&cp, &rp), "err54 proxy #{i}");
    }
}

// ===========================================================================
// Rows 55-62: the public `gjk` entry point
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn gjk_diff(ctx: &str, reverse: i8, p: [f32; 9]) {
    let (c, r) = (&libs().c, &libs().r);
    let poison = c2v { x: -98765.4, y: 12345.6 };
    let (mut ca, mut cb) = (poison, poison);
    let (mut ra, mut rb) = (poison, poison);
    unsafe {
        (c.gjk)(reverse, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
        (r.gjk)(reverse, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
    }
    assert_veq(&format!("{ctx} a (rev={reverse} p={p:?})"), ca, ra);
    assert_veq(&format!("{ctx} b (rev={reverse} p={p:?})"), cb, rb);
}

#[test]
fn err55_err56_gjk_reverse_truthiness() {
    let mut rng = Rng::new(SEED ^ 155);
    let reverses: [i8; 10] = [0, 1, 2, -1, 127, -128, 3, -2, 42, -42];
    for i in 0..3000 {
        let mut p = [0f32; 9];
        for v in p.iter_mut() {
            *v = rng.coord();
        }
        p[8] = rng.radius();
        for &rev in &reverses {
            gjk_diff(&format!("err55/56 rev #{i}"), rev, p);
        }
        // every non-zero `char` must behave identically to 1
        let (c, r) = (&libs().c, &libs().r);
        let mut base_a = c2v::default();
        let mut base_b = c2v::default();
        unsafe {
            (c.gjk)(1, &mut base_a, &mut base_b, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
        }
        for &rev in &reverses {
            if rev == 0 {
                continue;
            }
            let (mut a, mut b) = (c2v::default(), c2v::default());
            unsafe {
                (c.gjk)(rev, &mut a, &mut b, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            }
            assert_veq(&format!("err56 C rev={rev} == rev=1"), a, base_a);
            let (mut a, mut b2) = (c2v::default(), c2v::default());
            unsafe {
                (r.gjk)(rev, &mut a, &mut b2, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            }
            assert_veq(&format!("err56 Rust rev={rev} == rev=1"), a, base_a);
            let _ = b;
        }
    }
}

#[test]
fn err57_gjk_null_out_pointers() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 157);
    let poison = c2v { x: 5.5, y: -5.5 };
    for i in 0..3000 {
        let mut p = [0f32; 9];
        for v in p.iter_mut() {
            *v = rng.coord();
        }
        p[8] = rng.radius();
        for rev in [0i8, 1] {
            for (want_a, want_b) in [(false, false), (true, false), (false, true)] {
                let (mut ca, mut cb, mut ra, mut rb) = (poison, poison, poison, poison);
                unsafe {
                    (c.gjk)(
                        rev,
                        if want_a { &mut ca } else { std::ptr::null_mut() },
                        if want_b { &mut cb } else { std::ptr::null_mut() },
                        p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
                    );
                    (r.gjk)(
                        rev,
                        if want_a { &mut ra } else { std::ptr::null_mut() },
                        if want_b { &mut rb } else { std::ptr::null_mut() },
                        p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
                    );
                }
                assert_veq(&format!("err57 rev={rev} a want={want_a} #{i}"), ca, ra);
                assert_veq(&format!("err57 rev={rev} b want={want_b} #{i}"), cb, rb);
                if !want_a {
                    assert!(veq(ca, poison) && veq(ra, poison), "err57: NULL a was written");
                }
                if !want_b {
                    assert!(veq(cb, poison) && veq(rb, poison), "err57: NULL b was written");
                }
            }
        }
    }
}

#[test]
fn err58_gjk_aliased_out_pointers() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 158);
    for i in 0..3000 {
        let mut p = [0f32; 9];
        for v in p.iter_mut() {
            *v = rng.coord();
        }
        p[8] = rng.radius();
        for rev in [0i8, 1] {
            let mut cv = c2v { x: 1.0, y: 2.0 };
            let mut rv = cv;
            unsafe {
                let cp: *mut c2v = &mut cv;
                (c.gjk)(rev, cp, cp, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
                let rp: *mut c2v = &mut rv;
                (r.gjk)(rev, rp, rp, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            }
            assert_veq(&format!("err58 aliased rev={rev} #{i}"), cv, rv);
            // *outA is written first, then *outB, so the final value is `b`
            let (mut a2, mut b2) = (c2v::default(), c2v::default());
            unsafe {
                (c.gjk)(rev, &mut a2, &mut b2, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            }
            assert_veq(&format!("err58 aliased == outB rev={rev} #{i}"), cv, b2);
        }
    }
}

#[test]
fn err59_to_err61_gjk_degenerate_shapes() {
    let mut rng = Rng::new(SEED ^ 159);
    for i in 0..2000 {
        let x = rng.coord();
        let y = rng.coord();
        let bx = rng.coord();
        let by = rng.coord();
        let radii = [0.0f32, -0.0, -1.0, -1e6, 1e-30, 1e30, f32::MAX, f32::MIN_POSITIVE];
        for &rad in &radii {
            // row 59: zero-extent AABB
            gjk_diff(
                &format!("err59 zero-aabb #{i}"),
                (i % 2) as i8,
                [x, y, x, y, bx, by, bx + 1.0, by + 1.0, rad],
            );
            // row 60: zero-length capsule
            gjk_diff(
                &format!("err60 zero-capsule #{i}"),
                (i % 2) as i8,
                [x, y, x + 1.0, y + 1.0, bx, by, bx, by, rad],
            );
            // both degenerate at once
            gjk_diff(
                &format!("err59+60 both #{i}"),
                (i % 2) as i8,
                [x, y, x, y, bx, by, bx, by, rad],
            );
            // row 61: negative radius with a normal pair
            gjk_diff(
                &format!("err61 radius={rad:?} #{i}"),
                (i % 2) as i8,
                [x, y, x + 2.0, y + 2.0, bx, by, bx + 1.0, by, rad],
            );
            // inverted AABB
            gjk_diff(
                &format!("err54/61 inverted-aabb #{i}"),
                (i % 2) as i8,
                [x + 2.0, y + 2.0, x, y, bx, by, bx + 1.0, by, rad],
            );
        }
    }
}

#[test]
fn err62_gjk_nan_and_inf_in_every_parameter() {
    let mut rng = Rng::new(SEED ^ 162);
    let bad = [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        -0.0,
    ];
    for i in 0..600 {
        let mut base = [0f32; 9];
        for v in base.iter_mut() {
            *v = rng.coord();
        }
        base[8] = rng.radius();
        for slot in 0..9usize {
            for &v in &bad {
                let mut p = base;
                p[slot] = v;
                for rev in [0i8, 1] {
                    gjk_diff(&format!("err62 slot={slot} v={v:?} #{i}"), rev, p);
                }
            }
        }
        // all nine poisoned at once
        for &v in &bad {
            let p = [v; 9];
            for rev in [0i8, 1] {
                gjk_diff(&format!("err62 all-slots v={v:?} #{i}"), rev, p);
            }
        }
    }
}

// ===========================================================================
// Generic FFI boundary checks that every C API has
// ===========================================================================

/// A NULL `shape` pointer with a *valid* type is a straight null dereference in
/// the C (`c->r`), so it is not comparable; a NULL shape with an *out-of-range*
/// type, however, is never dereferenced and must be handled identically.
#[test]
fn generic_null_shape_with_invalid_type() {
    let (c, r) = (&libs().c, &libs().r);
    for ty in [-1i32, 3, 4, 99, i32::MIN, i32::MAX] {
        let mut cp = c2Proxy {
            radius: 1.5,
            count: 7,
            verts: [c2v { x: 3.0, y: 4.0 }; 8],
        };
        let mut rp = cp;
        unsafe {
            (c.c2MakeProxy)(std::ptr::null::<c_void>(), ty, &mut cp);
            (r.c2MakeProxy)(std::ptr::null::<c_void>(), ty, &mut rp);
        }
        assert!(
            proxyeq(&cp, &rp),
            "generic null-shape invalid-type {ty}: C={cp:?} != Rust={rp:?}"
        );
        assert_eq!(cp.count, 7, "generic: C touched the proxy for type {ty}");
        assert_eq!(rp.count, 7, "generic: Rust touched the proxy for type {ty}");
    }
}

/// Out-of-range `int` values for the boolean-ish `use_radius` parameter and the
/// `char` `reverse` parameter, i.e. values with no "valid variant".
#[test]
fn generic_out_of_range_scalar_flags() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 200);
    let urs = [
        0i32,
        1,
        -1,
        2,
        0x7FFF_FFFF,
        i32::MIN,
        0x0000_0100,
        0x1_0000i32,
    ];
    for i in 0..400 {
        let (a, b) = tri(&mut rng);
        for &ur in &urs {
            let co = call_gjk(c, &a, None, &b, None, ur, true, true, Some(c2GJKCache::default()));
            let ro = call_gjk(r, &a, None, &b, None, ur, true, true, Some(c2GJKCache::default()));
            assert_gjk_eq(&format!("generic use_radius={ur} #{i}"), &co, &ro);
        }
        // every possible `char` value for `reverse`
        let mut p = [0f32; 9];
        for v in p.iter_mut() {
            *v = rng.coord();
        }
        p[8] = rng.radius();
        for rev in [i8::MIN, -100, -1, 0, 1, 100, i8::MAX] {
            gjk_diff(&format!("generic reverse={rev} #{i}"), rev, p);
        }
    }
}

/// Zero and oversized lengths for the only length-taking function.
#[test]
fn generic_zero_and_oversized_lengths() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 201);
    // A 64-vertex buffer lets us pass counts far past the 8 the library ever
    // uses internally, without going out of bounds.
    let mut verts = vec![c2v::default(); 64];
    for v in verts.iter_mut() {
        *v = rng.coord_v();
    }
    for count in [0i32, 1, 2, 3, 4, 7, 8, 9, 16, 63, 64, -1, i32::MIN] {
        for i in 0..200 {
            let d = if i % 4 == 0 { rng.wild_v() } else { rng.coord_v() };
            let ci = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
            let ri = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
            assert_eq!(ci, ri, "generic c2Support count={count} d={d:?} #{i}");
        }
    }
}

/// One step past every documented simplex `count` range, for every function
/// that switches on it.
#[test]
fn generic_one_past_simplex_count_ranges() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 202);
    for count in [-1i32, 0, 1, 2, 3, 4, 5] {
        for i in 0..400 {
            let d = if i % 3 == 0 { 0.0 } else { rng.coord() };
            let s = rand_simplex(&mut rng, count, d, i % 5 == 0);

            let (mut cs, mut rs) = (s, s);
            assert_feq(
                &format!("generic metric count={count} #{i}"),
                unsafe { (c.c2GJKSimplexMetric)(&mut cs) },
                unsafe { (r.c2GJKSimplexMetric)(&mut rs) },
            );
            assert!(simplexeq(&cs, &rs));

            let (mut cs, mut rs) = (s, s);
            assert_veq(
                &format!("generic c2D count={count} #{i}"),
                unsafe { (c.c2D)(&mut cs) },
                unsafe { (r.c2D)(&mut rs) },
            );

            let (mut cs, mut rs) = (s, s);
            assert_veq(
                &format!("generic c2L count={count} #{i}"),
                unsafe { (c.c2L)(&mut cs) },
                unsafe { (r.c2L)(&mut rs) },
            );

            let (mut cs, mut rs) = (s, s);
            let (mut ca, mut cb) = (c2v::default(), c2v::default());
            let (mut ra, mut rb) = (c2v::default(), c2v::default());
            unsafe {
                (c.c2Witness)(&mut cs, &mut ca, &mut cb);
                (r.c2Witness)(&mut rs, &mut ra, &mut rb);
            }
            assert_veq(&format!("generic c2Witness A count={count} #{i}"), ca, ra);
            assert_veq(&format!("generic c2Witness B count={count} #{i}"), cb, rb);
            assert!(simplexeq(&cs, &rs));

            // c22 / c23 do not switch on count but must still agree
            let (mut cs, mut rs) = (s, s);
            unsafe {
                (c.c22)(&mut cs);
                (r.c22)(&mut rs);
            }
            assert!(simplexeq(&cs, &rs), "generic c22 count={count} #{i}");
            let (mut cs, mut rs) = (s, s);
            unsafe {
                (c.c23)(&mut cs);
                (r.c23)(&mut rs);
            }
            assert!(simplexeq(&cs, &rs), "generic c23 count={count} #{i}");
        }
    }
}
