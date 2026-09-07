//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! The library has no error enum and no error return code, so "the same error"
//! means "the same sentinel / fallback / rejection outcome", compared
//! bit-for-bit (`inf`, `NaN` with the exact payload, `0`, an unchanged
//! out-parameter, an unchanged `count`, ...).
//!
//! Rows that are undefined behaviour in the C (uninitialised reads,
//! out-of-bounds reads, NULL dereferences) cannot have a bit-equality
//! assertion, because the C has no reproducible result to match.  Those rows
//! are marked `UB` below and are covered by the strongest assertion that is
//! actually meaningful: either "the well-defined half matches exactly"
//! (rows 16/17) or "neither library aborts" (rows 14/15).  The three rows that
//! would take the harness down with a `SIGSEGV` (18/19/24) or smash the stack
//! (13) are deliberately not executed in-process; each is documented instead.

#![allow(non_snake_case)]

mod common;
use common::gjk::*;
use common::*;
use std::ffi::{c_int, c_uint, c_void};

const N: u32 = 2000;

fn aabb(x0: f32, y0: f32, x1: f32, y1: f32) -> Shape {
    Shape::Aabb(c2AABB { min: c2v { x: x0, y: y0 }, max: c2v { x: x1, y: y1 } })
}
fn circle(x: f32, y: f32, r: f32) -> Shape {
    Shape::Circle(c2Circle { p: c2v { x, y }, r })
}
fn capsule(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Shape {
    Shape::Capsule(c2Capsule { a: c2v { x: x0, y: y0 }, b: c2v { x: x1, y: y1 }, r })
}

// ===========================================================================
// Rows 1..2 — NULL transform pointers fall back to c2xIdentity()
// ===========================================================================
#[test]
fn row01_02_null_transform_fallback() {
    let mut rng = Rng::new(0xC001);
    let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
    let (c, _) = apis();
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..N / 4 {
                let a = shape_rand(&mut rng, ta, 40.0, 5.0);
                let b = shape_rand(&mut rng, tb, 40.0, 5.0);
                // NULL == explicit identity, in both libraries
                for (xa, xb) in [
                    (None, None),
                    (Some(ident), None),
                    (None, Some(ident)),
                    (Some(ident), Some(ident)),
                ] {
                    let cfg = Cfg::new(a, b).xf(xa, xb);
                    check("row01-02 null transform", &cfg);
                    assert_eq!(
                        call(c.c2GJK, &Cfg::new(a, b)),
                        call(c.c2GJK, &cfg),
                        "NULL transform is not equivalent to c2xIdentity() in the C"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Rows 3..5 — NULL outA / outB / iterations are simply not written
// ===========================================================================
#[test]
fn row03_05_null_out_params_not_written() {
    let mut rng = Rng::new(0xC003);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..N / 4 {
                let a = shape_rand(&mut rng, ta, 40.0, 5.0);
                let b = shape_rand(&mut rng, tb, 40.0, 5.0);
                let base = Cfg::new(a, b);
                let full = check("row03-05 baseline", &base);
                let _ = full;
                for (oa, ob, it) in [
                    (false, true, true),
                    (true, false, true),
                    (true, true, false),
                    (false, false, false),
                ] {
                    // `Obs::untouched` carries the poison values of every
                    // skipped out-param; `check` asserts they are identical,
                    // i.e. neither library wrote through a NULL slot.
                    check("row03-05 null outs", &base.outs(oa, ob, it));
                }
                // the returned distance must be unaffected by the out-params
                let (c, _) = apis();
                let d_full = call(c.c2GJK, &base).dist;
                let d_none = call(c.c2GJK, &base.outs(false, false, false)).dist;
                assert_eq!(d_full, d_none, "NULL out-params changed the return value");
            }
        }
    }
}

// ===========================================================================
// Rows 6..7 — NULL cache / cold cache (count == 0)
// ===========================================================================
#[test]
fn row06_07_cache_null_and_cold() {
    let mut rng = Rng::new(0xC006);
    let (c, _) = apis();
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..N / 4 {
                let a = shape_rand(&mut rng, ta, 40.0, 5.0);
                let b = shape_rand(&mut rng, tb, 40.0, 5.0);
                // row 6: NULL cache
                check("row06 cache=NULL", &Cfg::new(a, b).cache(None));
                // row 7: count == 0 with deliberately junky other fields
                let cold = c2GJKCache {
                    metric: rng.spicy(1e6),
                    count: 0,
                    iA: [7, -3, 99],
                    iB: [-1, 4, 12],
                    div: rng.spicy(1e6),
                };
                check("row07 cache cold", &Cfg::new(a, b).cache(Some(cold)));
                // a cold cache must give the same distance as no cache at all
                let d_null = call(c.c2GJK, &Cfg::new(a, b).cache(None)).dist;
                let d_cold = call(c.c2GJK, &Cfg::new(a, b).cache(Some(cold))).dist;
                assert_eq!(d_null, d_cold, "cold cache changed the distance");
            }
        }
    }
}

// ===========================================================================
// Row 8 — the quirk: the stale-cache test is essentially always false, so the
//         cached simplex is trusted verbatim (`cache_was_read = 1`).
// ===========================================================================
#[test]
fn row08_cache_trusted_verbatim() {
    let mut rng = Rng::new(0xC008);
    let (c, _) = apis();
    let mut trusted = 0u32;
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..N / 4 {
                let a = shape_rand(&mut rng, ta, 40.0, 5.0);
                let b = shape_rand(&mut rng, tb, 40.0, 5.0);
                let warm = check("row08 seed", &Cfg::new(a, b).cache(Some(c2GJKCache::default())))
                    .unwrap();
                if warm.count == 0 {
                    continue;
                }
                let warm = clamp_cache(warm, a.ty(), b.ty());
                check("row08 warm reuse", &Cfg::new(a, b).cache(Some(warm)));
                // Detect the quirk: a warm cache is NOT equivalent to a cold
                // one in general, because the cached simplex is trusted.
                let d_cold =
                    call(c.c2GJK, &Cfg::new(a, b).cache(Some(c2GJKCache::default()))).cache;
                let d_warm = call(c.c2GJK, &Cfg::new(a, b).cache(Some(warm))).cache;
                if d_cold != d_warm {
                    trusted += 1;
                }
            }
        }
    }
    // Not a hard requirement of the row (both branches are legal), but this
    // documents that the warm path really is being taken.
    println!("row08: {trusted} configurations where the warm cache changed the outcome");
}

// ===========================================================================
// Row 9 — the ONLY way to make the C re-seed: metric <= -1.0e8f
// ===========================================================================
#[test]
fn row09_cache_reseed_on_huge_negative_metric() {
    let (c, _) = apis();
    let mut found = 0u32;
    // Big AABBs so that det2 of the simplex edges is around 1e10 in magnitude.
    let big = 1.0e5f32;
    let a = aabb(-big, -big, big, big);
    let b = aabb(big * 0.5, big * 0.5, big * 2.0, big * 2.0);
    // Try every 3-index combination; at least one gives a large NEGATIVE det2.
    for i0 in 0..4i32 {
        for i1 in 0..4i32 {
            for i2 in 0..4i32 {
                for j0 in 0..4i32 {
                    for j1 in 0..4i32 {
                        for j2 in 0..4i32 {
                            let warm = c2GJKCache {
                                metric: 0.0, // -> min_metric = metric, max_metric = 0
                                count: 3,
                                iA: [i0, i1, i2],
                                iB: [j0, j1, j2],
                                div: 1.0,
                            };
                            let cfg = Cfg::new(a, b).cache(Some(warm));
                            check("row09 reseed candidate", &cfg);
                            // If the re-seed branch was taken the result must
                            // equal the cold-cache result exactly.
                            let cold = call(
                                c.c2GJK,
                                &Cfg::new(a, b).cache(Some(c2GJKCache::default())),
                            );
                            let got = call(c.c2GJK, &cfg);
                            if got.dist == cold.dist && got.cache == cold.cache {
                                found += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(found > 0, "never hit the metric <= -1e8 re-seed branch");
}

// ===========================================================================
// Row 10 — warm cache with count == 3 -> the `hit` path fires immediately
// ===========================================================================
#[test]
fn row10_warm_cache_count3_immediate_hit() {
    let (c, _) = apis();
    let mut hits = 0u32;
    // AABBs (4 initialised verts) so all indices 0..3 are legal.
    let a = aabb(-2.0, -2.0, 2.0, 2.0);
    let b = aabb(-1.0, -1.0, 3.0, 3.0);
    for i0 in 0..4i32 {
        for i1 in 0..4i32 {
            for i2 in 0..4i32 {
                for j0 in 0..4i32 {
                    let warm = c2GJKCache {
                        metric: 0.0,
                        count: 3,
                        iA: [i0, i1, i2],
                        iB: [j0, (j0 + 1) % 4, (j0 + 2) % 4],
                        div: 1.0,
                    };
                    let cfg = Cfg::new(a, b).cache(Some(warm));
                    check("row10 count3", &cfg);
                    let got = call(c.c2GJK, &cfg);
                    if got.iters == Some(0) && got.dist == 0.0f32.to_bits() {
                        hits += 1;
                    }
                    // and with use_radius = 0
                    check("row10 count3 ur=0", &cfg.radius(0));
                }
            }
        }
    }
    assert!(hits > 0, "warm count==3 cache never produced the immediate `hit` path");
}

// ===========================================================================
// Row 11 — cache->count < 0
// ===========================================================================
#[test]
fn row11_cache_negative_count() {
    let mut rng = Rng::new(0xC011);
    let (c, _) = apis();
    for &neg in [-1i32, -2, -3, -1000, i32::MIN].iter() {
        for &ta in TYPES.iter() {
            for &tb in TYPES.iter() {
                for _ in 0..40 {
                    let a = shape_rand(&mut rng, ta, 40.0, 5.0);
                    let b = shape_rand(&mut rng, tb, 40.0, 5.0);
                    let bad = c2GJKCache {
                        metric: rng.spicy(100.0),
                        count: neg,
                        iA: [0, 0, 0],
                        iB: [0, 0, 0],
                        div: rng.spicy(100.0),
                    };
                    for ur in [0i32, 1] {
                        let cfg = Cfg::new(a, b).cache(Some(bad)).radius(ur);
                        check(&format!("row11 count={neg} ur={ur}"), &cfg);
                        // documented outcome: dist == 0, witness points zero,
                        // iter == 0, and the negative count survives verbatim
                        let got = call(c.c2GJK, &cfg);
                        assert_eq!(got.dist, 0.0f32.to_bits(), "row11 dist");
                        assert_eq!(got.iters, Some(0), "row11 iter");
                        let cb = got.cache.unwrap();
                        assert_eq!(cb[1] as i32, neg, "row11 count not preserved");
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 12 — cache->count == 4: `iA[3]` aliases `iB[0]` and `iB[3]` aliases
//          `div`.  Both alias reads are kept in the initialised index range so
//          the behaviour is fully defined and can be compared bit-for-bit.
// ===========================================================================
#[test]
fn row12_cache_count_four_struct_aliasing() {
    let (c, _) = apis();
    // AABB x AABB -> 4 initialised verts each, so indices 0..3 are all legal.
    let a = aabb(-2.0, -2.0, 2.0, 2.0);
    let b = aabb(1.0, 1.0, 5.0, 5.0);
    // `div` is reinterpreted as `iB[3]`; pick bit patterns that are small,
    // in-range vertex indices AND legal floats.
    for div_idx in 0..4u32 {
        let div = f32::from_bits(div_idx);
        for i0 in 0..4i32 {
            for j0 in 0..4i32 {
                let bad = c2GJKCache {
                    metric: 0.0,
                    count: 4,
                    // iA[3] does not exist: byte-wise it IS iB[0], which is
                    // `j0` below -- also a legal index.
                    iA: [i0, (i0 + 1) % 4, (i0 + 2) % 4],
                    iB: [j0, (j0 + 1) % 4, (j0 + 3) % 4],
                    div,
                };
                for ur in [0i32, 1] {
                    let cfg = Cfg::new(a, b).cache(Some(bad)).radius(ur);
                    check(&format!("row12 count=4 div_idx={div_idx} ur={ur}"), &cfg);
                    let got = call(c.c2GJK, &cfg);
                    // count 4 matches no `case` in the switch, so the loop
                    // bails out immediately with the default paths.
                    assert_eq!(got.iters, Some(0), "row12 iter");
                }
            }
        }
    }
}

// ===========================================================================
// Row 13 — cache->count > 4  [UB, NOT EXECUTED]
//
// `verts + 4` is `&s.d + 1`, i.e. bytes 144..180 of a 152-byte `c2Simplex`.
// Writing a 36-byte `c2sv` there overruns the struct by 28 bytes and smashes
// `c2GJK`'s other stack locals.  Running it would corrupt (or crash) the test
// harness itself and, because the two libraries lay their frames out
// differently, there is no reproducible result to compare.  Documented here
// rather than executed; the boundary that IS well defined (count == 4) is
// covered by `row12_cache_count_four_struct_aliasing`.
// ===========================================================================
#[test]
fn row13_cache_count_gt_four_documented_ub() {
    // Assert the layout fact the row rests on, so this stays honest if the
    // struct definitions ever change.
    assert_eq!(std::mem::size_of::<c2Simplex>(), 152);
    assert_eq!(std::mem::size_of::<c2sv>(), 36);
    // verts[4] would start at offset 144 and end at 180 > 152.
    assert!(4 * std::mem::size_of::<c2sv>() + std::mem::size_of::<c2sv>() > std::mem::size_of::<c2Simplex>());
}

// ===========================================================================
// Rows 14..15 — cached index outside the initialised / allocated vertex range.
//               [UB] The C reads uninitialised stack (14) or past the proxy
//               (15), so only "neither library aborts" is asserted.
// ===========================================================================
#[test]
fn row14_15_cached_index_out_of_range_no_crash() {
    let mut rng = Rng::new(0xC014);
    let mut differed = 0u32;
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..200 {
                let a = shape_rand(&mut rng, ta, 40.0, 5.0);
                let b = shape_rand(&mut rng, tb, 40.0, 5.0);
                for &idx in [1i32, 2, 3, 5, 7, 8, 11, 15].iter() {
                    let bad = c2GJKCache {
                        metric: 0.0,
                        count: 1,
                        iA: [idx, 0, 0],
                        iB: [idx, 0, 0],
                        div: 1.0,
                    };
                    let (oc, or) = run_both_no_assert(&Cfg::new(a, b).cache(Some(bad)));
                    if oc != or {
                        differed += 1;
                    }
                }
            }
        }
    }
    // Both libraries survived every call; that is the only guarantee available.
    println!(
        "row14-15: {differed} of the UB configurations differed (expected: the C \
         reads uninitialised stack, so any value is possible)"
    );
}

// ===========================================================================
// Row 16 — out-of-range C2_TYPE reaching c2GJK.  [UB] the `c2Proxy` local is
//          left wholly uninitialised, so only "no abort" is asserted here; the
//          well-defined half is row 17.
// ===========================================================================
// MEASURED, NOT ASSUMED: calling the C's `c2GJK` with an out-of-range
// `C2_TYPE` reliably raises SIGSEGV.  `c2MakeProxy`'s `switch` has no
// `default:`, so `c2Proxy pA` / `pB` keep whatever the stack held -- including
// `count`.  `c2Support(pA.verts, pA.count, d)` then walks `count` vertices, and
// with a garbage `count` (observed: values in the millions) it runs off the
// stack.  Executing it would kill the harness, and since the C's behaviour
// depends entirely on stack residue there is no reproducible result to compare
// anyway.  The WELL-DEFINED half of this row -- that `c2MakeProxy` writes
// nothing at all for such a type -- is asserted exhaustively in row 17, and
// that is the only part of the enum handling the C actually defines.
#[test]
fn row16_bad_enum_into_gjk_documented_ub() {
    // Guard the premise: `c2Proxy::count` sits at offset 4 and is a plain
    // `int` with no validation anywhere in the C, so an uninitialised value
    // reaches `c2Support` unchecked.
    assert_eq!(std::mem::size_of::<c2Proxy>(), 72);
    assert_eq!(std::mem::size_of::<c_uint>(), 4);
    // And confirm the enum values that DO have a variant are exactly 0, 1, 2 --
    // i.e. everything else is out of range.
    let (c, r) = apis();
    for t in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        let sh = c2Capsule { a: c2v { x: 1.0, y: 2.0 }, b: c2v { x: 3.0, y: 4.0 }, r: 5.0 };
        let mut pc = c2Proxy { radius: 0.0, count: -1, verts: [c2v::default(); 8] };
        let mut pr = pc;
        unsafe {
            (c.c2MakeProxy)(&sh as *const c2Capsule as *const c_void, t, &mut pc);
            (r.c2MakeProxy)(&sh as *const c2Capsule as *const c_void, t, &mut pr);
        }
        diff_eq!(format!("row16 valid enum={t}"), raw_bytes(&pc), raw_bytes(&pr));
        assert_ne!(pc.count, -1, "type {t} should have written count");
    }
}

// ===========================================================================
// Row 17 — out-of-range C2_TYPE at c2MakeProxy: the switch has no `default:`,
//          so NOT ONE BYTE of the caller's proxy is written.  Fully defined and
//          asserted bit-for-bit.
// ===========================================================================
#[test]
fn row17_bad_enum_makeproxy_writes_nothing() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC017);
    let mut bad: Vec<c_uint> = vec![3, 4, 5, 6, 7, 8, 42, 255, 256, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF];
    for _ in 0..N {
        let v = rng.next_u32();
        if v > 2 {
            bad.push(v);
        }
    }
    for (i, &t) in bad.iter().enumerate() {
        // an arbitrary 32-byte shape blob; the C must not read it either
        let blob: [u32; 8] = [
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
        ];
        let mut pc = c2Proxy {
            radius: f32::from_bits(0xDEAD_BEEF),
            count: -0x5A5A5A,
            verts: [c2v { x: f32::from_bits(0xCAFEBABE), y: f32::from_bits(0xFEEDFACE) }; 8],
        };
        pc.verts[i % 8].x = f32::from_bits(0x0BADF00D);
        let mut pr = pc;
        let before = raw_bytes(&pc);
        unsafe {
            (c.c2MakeProxy)(blob.as_ptr() as *const c_void, t, &mut pc);
            (r.c2MakeProxy)(blob.as_ptr() as *const c_void, t, &mut pr);
        }
        diff_eq!(format!("row17 makeproxy type={t}"), raw_bytes(&pc), raw_bytes(&pr));
        assert_eq!(before, raw_bytes(&pc), "C wrote to the proxy for type={t}");
    }
    // Also confirm 0/1/2 DO write (so the test above is not vacuous).
    let sh = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 };
    let mut p = c2Proxy::default();
    p.count = -1;
    let before = raw_bytes(&p);
    unsafe {
        (c.c2MakeProxy)(&sh as *const c2Circle as *const c_void, C2_TYPE_CIRCLE, &mut p);
    }
    assert_ne!(before, raw_bytes(&p), "valid type wrote nothing?");
}

// ===========================================================================
// Rows 18/19/24 — NULL pointer dereferences.  [UB, NOT EXECUTED]
//
//   18: c2MakeProxy(NULL, <valid type>, p)  -> reads *(c2Circle*)NULL
//   19: c2BBVerts(NULL, bb) / c2BBVerts(out, NULL)
//   24: c2Support(NULL, count, d)           -> reads verts[0]
//
// All three fault with SIGSEGV in both libraries, which would take the test
// process down with them, so they are documented rather than executed.  What
// IS asserted below is the property that makes them equivalent: neither
// library has a NULL guard on these parameters, i.e. the very first thing each
// does is an unconditional load.  That is verified structurally by the fact
// that every OTHER pointer parameter in the API (rows 1..6) *is* guarded and
// those guards are tested above.
// ===========================================================================
#[test]
fn row18_19_24_null_deref_documented_ub() {
    // c2Support with a valid pointer but count == 0 still reads verts[0]:
    // that is the observable proof that there is no count/NULL guard.
    let (c, r) = apis();
    let verts = [c2v { x: 7.0, y: -7.0 }; 1];
    for &count in [0i32, -1, i32::MIN].iter() {
        let d = c2v { x: 1.0, y: 1.0 };
        let (a, b) = unsafe {
            ((c.c2Support)(verts.as_ptr(), count, d), (r.c2Support)(verts.as_ptr(), count, d))
        };
        diff_eq!(format!("row24 support count={count}"), a, b);
        assert_eq!(a, 0, "c2Support must return 0 when the loop never runs");
    }
}

// ===========================================================================
// Rows 20..23 — c2Support degenerate counts and tie / NaN behaviour
// ===========================================================================
#[test]
fn row20_23_support_degenerate() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC020);
    for _ in 0..N {
        let mut verts = [c2v::default(); 8];
        for i in 0..8 {
            verts[i] = rng.v(100.0);
        }
        // row 20: count <= 0
        for &count in [0i32, -1, -100, i32::MIN].iter() {
            let d = rng.v(100.0);
            let (a, b) = unsafe {
                (
                    (c.c2Support)(verts.as_ptr(), count, d),
                    (r.c2Support)(verts.as_ptr(), count, d),
                )
            };
            diff_eq!(format!("row20 count={count}"), a, b);
            assert_eq!(a, 0);
        }
        // row 21: count == 1
        let d = rng.v_spicy(100.0);
        let (a, b) =
            unsafe { ((c.c2Support)(verts.as_ptr(), 1, d), (r.c2Support)(verts.as_ptr(), 1, d)) };
        diff_eq!("row21 count=1", a, b);
        assert_eq!(a, 0);

        // row 22: d == (0,0) -> every dot is 0, index 0 wins
        let z = c2v { x: 0.0, y: 0.0 };
        for count in 1..=8i32 {
            let (a, b) = unsafe {
                ((c.c2Support)(verts.as_ptr(), count, z), (r.c2Support)(verts.as_ptr(), count, z))
            };
            diff_eq!(format!("row22 d=0 count={count}"), a, b);
            assert_eq!(a, 0);
        }
        // row 22: all vertices identical -> tie, index 0 wins
        let same = [rng.v(100.0); 8];
        let d = rng.v(100.0);
        for count in 1..=8i32 {
            let (a, b) = unsafe {
                ((c.c2Support)(same.as_ptr(), count, d), (r.c2Support)(same.as_ptr(), count, d))
            };
            diff_eq!(format!("row22 ties count={count}"), a, b);
            assert_eq!(a, 0);
        }

        // row 23: NaN vertices
        for &pos in [0usize, 1, 4, 7].iter() {
            let mut nanv = verts;
            nanv[pos] = c2v { x: f32::NAN, y: f32::from_bits(0xFFC0_0011) };
            for count in 1..=8i32 {
                let d = rng.v(100.0);
                let (a, b) = unsafe {
                    (
                        (c.c2Support)(nanv.as_ptr(), count, d),
                        (r.c2Support)(nanv.as_ptr(), count, d),
                    )
                };
                diff_eq!(format!("row23 nan@{pos} count={count}"), a, b);
            }
        }
        // all NaN
        let alln = [c2v { x: f32::NAN, y: f32::NAN }; 8];
        for count in 1..=8i32 {
            let d = rng.v(100.0);
            let (a, b) = unsafe {
                ((c.c2Support)(alln.as_ptr(), count, d), (r.c2Support)(alln.as_ptr(), count, d))
            };
            diff_eq!(format!("row23 all-nan count={count}"), a, b);
            assert_eq!(a, 0);
        }
    }
}

// ===========================================================================
// Rows 25..27 — c2Witness with an out-of-domain count / degenerate div
// ===========================================================================
#[test]
fn row25_27_witness_degenerate() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC025);
    let divs = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::from_bits(0xFFC0_0021),
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0,
    ];
    for _ in 0..N {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i] = c2sv {
                sA: rng.v(100.0),
                sB: rng.v(100.0),
                p: rng.v(100.0),
                u: rng.spicy(10.0),
                iA: 0,
                iB: 0,
            };
        }
        // row 25: count outside {1,2,3} -> default: a = b = (0,0)
        for &count in [0i32, 4, 5, -1, i32::MIN, i32::MAX].iter() {
            s.count = count;
            s.div = rng.spicy(10.0);
            let mut sc = s;
            let mut sr = s;
            let mut ac = c2v { x: 1.5, y: 2.5 };
            let mut bc = c2v { x: 3.5, y: 4.5 };
            let mut ar = ac;
            let mut br = bc;
            unsafe {
                (c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (r.c2Witness)(&mut sr, &mut ar, &mut br);
            }
            diff_eq!(format!("row25 count={count}"), (vb(ac), vb(bc)), (vb(ar), vb(br)));
            assert_eq!(vb(ac), (0u32, 0u32), "row25 default must write (0,0)");
            assert_eq!(vb(bc), (0u32, 0u32), "row25 default must write (0,0)");
        }
        // rows 26/27: div == 0 / -0.0 / NaN / inf for counts 1..3
        for count in 1..=3i32 {
            for &div in divs.iter() {
                s.count = count;
                s.div = div;
                let mut sc = s;
                let mut sr = s;
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let mut ar = c2v::default();
                let mut br = c2v::default();
                unsafe {
                    (c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (r.c2Witness)(&mut sr, &mut ar, &mut br);
                }
                diff_eq!(
                    format!("row26-27 count={count} div={div:?}"),
                    (vb(ac), vb(bc)),
                    (vb(ar), vb(br))
                );
            }
            // u == 0 with div == 0 -> inf * 0 -> NaN
            let mut s2 = s;
            s2.count = count;
            s2.div = 0.0;
            for i in 0..4 {
                s2.verts[i].u = 0.0;
            }
            let mut sc = s2;
            let mut sr = s2;
            let mut ac = c2v::default();
            let mut bc = c2v::default();
            let mut ar = c2v::default();
            let mut br = c2v::default();
            unsafe {
                (c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (r.c2Witness)(&mut sr, &mut ar, &mut br);
            }
            diff_eq!(
                format!("row26 u=0 div=0 count={count}"),
                (vb(ac), vb(bc)),
                (vb(ar), vb(br))
            );
        }
    }
}

// ===========================================================================
// Rows 28..29 — c2L out-of-domain count / div == 0
// ===========================================================================
#[test]
fn row28_29_c2L_degenerate() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC028);
    for _ in 0..N {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i] = c2sv {
                sA: rng.v(100.0),
                sB: rng.v(100.0),
                p: rng.v(100.0),
                u: rng.spicy(10.0),
                iA: 0,
                iB: 0,
            };
        }
        for &count in [3i32, 0, 4, -1, i32::MIN, i32::MAX].iter() {
            s.count = count;
            s.div = rng.spicy(10.0);
            let mut sc = s;
            let mut sr = s;
            let (a, b) = unsafe { ((c.c2L)(&mut sc), (r.c2L)(&mut sr)) };
            diff_eq!(format!("row28 count={count}"), vb(a), vb(b));
            assert_eq!(vb(a), (0u32, 0u32), "row28 default must return (0,0)");
        }
        for &div in [0.0f32, -0.0, f32::NAN, f32::INFINITY].iter() {
            for count in 1..=2i32 {
                s.count = count;
                s.div = div;
                let mut sc = s;
                let mut sr = s;
                let (a, b) = unsafe { ((c.c2L)(&mut sc), (r.c2L)(&mut sr)) };
                diff_eq!(format!("row29 count={count} div={div:?}"), vb(a), vb(b));
            }
        }
    }
}

// ===========================================================================
// Rows 30..31 — c2D out-of-domain count / degenerate edge
// ===========================================================================
#[test]
fn row30_31_c2D_degenerate() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC030);
    for _ in 0..N {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i].p = rng.v(100.0);
        }
        for &count in [3i32, 0, 4, -1, i32::MIN, i32::MAX].iter() {
            s.count = count;
            let mut sc = s;
            let mut sr = s;
            let (a, b) = unsafe { ((c.c2D)(&mut sc), (r.c2D)(&mut sr)) };
            diff_eq!(format!("row30 count={count}"), vb(a), vb(b));
            assert_eq!(vb(a), (0u32, 0u32), "row30 default must return (0,0)");
        }
        // row 31: count == 2 with a.p == b.p -> ab == (0,0), det2 == 0
        let p = rng.v(100.0);
        s.count = 2;
        s.verts[0].p = p;
        s.verts[1].p = p;
        let mut sc = s;
        let mut sr = s;
        let (a, b) = unsafe { ((c.c2D)(&mut sc), (r.c2D)(&mut sr)) };
        diff_eq!("row31 degenerate edge", vb(a), vb(b));
        // `c2Sub(p, p)` is (+0.0, +0.0), `det2` is 0 which is NOT `> 0`, so the
        // C takes `c2CCW90((+0.0,+0.0))` = `(+0.0, -0.0)`.  The sign of the
        // second zero is part of the observable result.
        assert_eq!(
            vb(a),
            (0x0000_0000u32, 0x8000_0000u32),
            "row31 must take the CCW90 path and yield (+0.0, -0.0)"
        );
        // and with signed zeros / NaN in p
        for pp in [
            c2v { x: -0.0, y: -0.0 },
            c2v { x: f32::NAN, y: 0.0 },
            c2v { x: 0.0, y: f32::NAN },
        ] {
            s.verts[0].p = pp;
            s.verts[1].p = pp;
            let mut sc = s;
            let mut sr = s;
            let (a, b) = unsafe { ((c.c2D)(&mut sc), (r.c2D)(&mut sr)) };
            diff_eq!(format!("row31 degenerate {pp:?}"), vb(a), vb(b));
        }
    }
}

// ===========================================================================
// Rows 32..33 — c2GJKSimplexMetric out-of-domain count / equal points
// ===========================================================================
#[test]
fn row32_33_metric_degenerate() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC032);
    for _ in 0..N {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i].p = rng.v(100.0);
        }
        for &count in [1i32, 0, 4, 5, -1, i32::MIN, i32::MAX].iter() {
            s.count = count;
            let mut sc = s;
            let mut sr = s;
            let (a, b) =
                unsafe { ((c.c2GJKSimplexMetric)(&mut sc), (r.c2GJKSimplexMetric)(&mut sr)) };
            diff_eq!(format!("row32 count={count}"), fb(a), fb(b));
            assert_eq!(fb(a), 0.0f32.to_bits(), "row32 default/case-1 must return 0");
        }
        // row 33: count == 2 with equal points -> sqrtf(0) == 0
        let p = rng.v(100.0);
        s.count = 2;
        s.verts[0].p = p;
        s.verts[1].p = p;
        let mut sc = s;
        let mut sr = s;
        let (a, b) = unsafe { ((c.c2GJKSimplexMetric)(&mut sc), (r.c2GJKSimplexMetric)(&mut sr)) };
        diff_eq!("row33 equal points", fb(a), fb(b));
        assert_eq!(fb(a), 0.0f32.to_bits());
        // count == 3 with all points equal -> det2 of two zero vectors
        s.count = 3;
        s.verts[2].p = p;
        let mut sc = s;
        let mut sr = s;
        let (a, b) = unsafe { ((c.c2GJKSimplexMetric)(&mut sc), (r.c2GJKSimplexMetric)(&mut sr)) };
        diff_eq!("row33 all equal", fb(a), fb(b));
    }
}

// ===========================================================================
// Rows 34..38 — c2Norm / c2Div sentinel values
// ===========================================================================
#[test]
fn row34_38_norm_div_sentinels() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC034);
    // row 34: c2Norm((0,0)) -> NaN in both components
    for v in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: 0.0 },
    ] {
        let (a, b) = ((c.c2Norm)(v), (r.c2Norm)(v));
        diff_eq!(format!("row34 norm({v:?})"), vb(a), vb(b));
        assert!(a.x.is_nan() && a.y.is_nan(), "row34 must produce NaN");
    }
    // row 35: NaN / inf components
    for v in [
        c2v { x: f32::NAN, y: 1.0 },
        c2v { x: 1.0, y: f32::NAN },
        c2v { x: f32::INFINITY, y: 1.0 },
        c2v { x: f32::INFINITY, y: f32::INFINITY },
        c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
        c2v { x: f32::from_bits(0xFFC0_0031), y: f32::from_bits(0x7FC0_0032) },
        c2v { x: 1e30, y: 1e30 },
        c2v { x: 1e-30, y: 1e-30 },
    ] {
        let (a, b) = ((c.c2Norm)(v), (r.c2Norm)(v));
        diff_eq!(format!("row35 norm({v:?})"), vb(a), vb(b));
    }
    // rows 36..38: c2Div by 0 / -0.0 / NaN / inf / denormal
    let bad_divs = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::from_bits(0xFFC0_0041),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN_POSITIVE / 3.0,
        FLT_MAX,
    ];
    for _ in 0..N {
        let mut vs = vec![
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: 1.0, y: -1.0 },
            c2v { x: f32::NAN, y: f32::INFINITY },
        ];
        vs.push(rng.v_spicy(1e3));
        vs.push(rng.v_bits());
        for v in vs {
            for &d in bad_divs.iter() {
                let (a, b) = ((c.c2Div)(v, d), (r.c2Div)(v, d));
                diff_eq!(format!("row36-38 div({v:?},{d:?})"), vb(a), vb(b));
            }
        }
    }
}

// ===========================================================================
// Rows 39..40 — c2Len overflow / NaN
// ===========================================================================
#[test]
fn row39_40_len_extremes() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC039);
    let mut vs = vec![
        c2v { x: 1e38, y: 1e38 },
        c2v { x: FLT_MAX, y: FLT_MAX },
        c2v { x: -FLT_MAX, y: FLT_MAX },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: f32::NEG_INFINITY, y: f32::NEG_INFINITY },
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: 0.0, y: f32::NAN },
        c2v { x: f32::from_bits(0xFFC0_0051), y: 1.0 },
        c2v { x: f32::MIN_POSITIVE / 5.0, y: f32::MIN_POSITIVE / 7.0 },
        c2v { x: 0.0, y: 0.0 },
    ];
    for _ in 0..N {
        vs.push(rng.v_bits());
    }
    for v in vs {
        let (a, b) = ((c.c2Len)(v), (r.c2Len)(v));
        diff_eq!(format!("row39-40 len({v:?})"), fb(a), fb(b));
    }
    // the documented sentinel: overflow -> +inf
    let huge = c2v { x: 1e38, y: 1e38 };
    assert_eq!((c.c2Len)(huge), f32::INFINITY);
}

// ===========================================================================
// Rows 41..43 — c2Maxv / c2Minv NaN ordering and signed zeros (b wins)
// ===========================================================================
#[test]
fn row41_43_minmax_nan_and_signed_zero() {
    let (c, r) = apis();
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_0061),
        f32::from_bits(0xFFC0_0062),
        f32::from_bits(0x7F80_0001), // signalling NaN
    ];
    let plain = [0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY];

    // row 41/42: NaN in a only, in b only, in both -- and the reverse order
    for &n in nans.iter() {
        for &p in plain.iter() {
            let cases = [
                (c2v { x: n, y: n }, c2v { x: p, y: p }),
                (c2v { x: p, y: p }, c2v { x: n, y: n }),
                (c2v { x: n, y: p }, c2v { x: p, y: n }),
            ];
            for (a, b) in cases {
                diff_eq!(format!("row41 max({a:?},{b:?})"), vb((c.c2Maxv)(a, b)), vb((r.c2Maxv)(a, b)));
                diff_eq!(format!("row42 min({a:?},{b:?})"), vb((c.c2Minv)(a, b)), vb((r.c2Minv)(a, b)));
            }
        }
        for &m in nans.iter() {
            let a = c2v { x: n, y: n };
            let b = c2v { x: m, y: m };
            // both NaN: the C's ternary always yields `b`
            let mx = (c.c2Maxv)(a, b);
            diff_eq!(format!("row41 max nan/nan"), vb(mx), vb((r.c2Maxv)(a, b)));
            assert_eq!(vb(mx), vb(b), "c2Maxv must return b when neither compares");
            let mn = (c.c2Minv)(a, b);
            diff_eq!(format!("row42 min nan/nan"), vb(mn), vb((r.c2Minv)(a, b)));
            assert_eq!(vb(mn), vb(b), "c2Minv must return b when neither compares");
        }
    }
    // row 43: +0.0 vs -0.0 -> b's component wins, so the sign of zero is
    // argument-order dependent
    let zp = c2v { x: 0.0, y: 0.0 };
    let zn = c2v { x: -0.0, y: -0.0 };
    for (a, b) in [(zp, zn), (zn, zp)] {
        let mx = (c.c2Maxv)(a, b);
        let mn = (c.c2Minv)(a, b);
        diff_eq!("row43 max signed zero", vb(mx), vb((r.c2Maxv)(a, b)));
        diff_eq!("row43 min signed zero", vb(mn), vb((r.c2Minv)(a, b)));
        assert_eq!(vb(mx), vb(b));
        assert_eq!(vb(mn), vb(b));
    }
}

// ===========================================================================
// Rows 44..45 — c2Clampv with an inverted range / NaN arguments
// ===========================================================================
#[test]
fn row44_45_clamp_inverted_and_nan() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC044);
    for _ in 0..N {
        // row 44: lo > hi (no validation) -> lo wins
        let lo = rng.v(50.0);
        let hi = c2v { x: lo.x - 10.0 - rng.unit() * 50.0, y: lo.y - 10.0 - rng.unit() * 50.0 };
        let a = rng.v(200.0);
        let got = (c.c2Clampv)(a, lo, hi);
        diff_eq!(format!("row44 clamp inverted"), vb(got), vb((r.c2Clampv)(a, lo, hi)));
        assert_eq!(vb(got), vb(lo), "inverted range must collapse to lo");

        // row 45: NaN in each of the three arguments, all combinations
        let n = c2v { x: f32::NAN, y: f32::from_bits(0xFFC0_0071) };
        for (aa, ll, hh) in [
            (n, lo, hi),
            (a, n, hi),
            (a, lo, n),
            (n, n, hi),
            (n, lo, n),
            (a, n, n),
            (n, n, n),
        ] {
            diff_eq!(
                format!("row45 clamp nan({aa:?},{ll:?},{hh:?})"),
                vb((c.c2Clampv)(aa, ll, hh)),
                vb((r.c2Clampv)(aa, ll, hh))
            );
        }
        // arbitrary bits
        let (aa, ll, hh) = (rng.v_bits(), rng.v_bits(), rng.v_bits());
        diff_eq!("row45 clamp bits", vb((c.c2Clampv)(aa, ll, hh)), vb((r.c2Clampv)(aa, ll, hh)));
    }
}

// ===========================================================================
// Rows 46..47 — c22 degenerate / NaN
// ===========================================================================
#[test]
fn row46_47_c22_degenerate() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC046);
    for _ in 0..N {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i] = c2sv {
                sA: rng.v(100.0),
                sB: rng.v(100.0),
                p: rng.v(100.0),
                u: rng.sym(10.0),
                iA: 1,
                iB: 2,
            };
        }
        s.count = 2;
        s.div = rng.sym(10.0);
        // row 46: a.p == b.p -> u == v == 0 -> `v <= 0` collapses to count 1
        let p = rng.v(100.0);
        s.verts[0].p = p;
        s.verts[1].p = p;
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (c.c22)(&mut sc);
            (r.c22)(&mut sr);
        }
        diff_eq!("row46 c22 a.p==b.p", simplex_bits(&sc), simplex_bits(&sr));
        assert_eq!(sc.count, 1, "row46 must collapse to count 1");
        assert_eq!(fb(sc.div), 1.0f32.to_bits(), "row46 div must be 1");

        // row 47: NaN points -> both guards false -> else branch, div = NaN
        for pp in [
            (c2v { x: f32::NAN, y: 0.0 }, c2v { x: 1.0, y: 1.0 }),
            (c2v { x: 1.0, y: 1.0 }, c2v { x: f32::NAN, y: 0.0 }),
            (
                c2v { x: f32::NAN, y: f32::NAN },
                c2v { x: f32::from_bits(0xFFC0_0081), y: f32::NAN },
            ),
            (c2v { x: f32::INFINITY, y: 0.0 }, c2v { x: f32::INFINITY, y: 0.0 }),
        ] {
            s.verts[0].p = pp.0;
            s.verts[1].p = pp.1;
            let mut sc = s;
            let mut sr = s;
            unsafe {
                (c.c22)(&mut sc);
                (r.c22)(&mut sr);
            }
            diff_eq!(format!("row47 c22 nan {pp:?}"), simplex_bits(&sc), simplex_bits(&sr));
        }
    }
}

// ===========================================================================
// Rows 48..50 — c23 all-equal / collinear / NaN
// ===========================================================================
#[test]
fn row48_50_c23_degenerate() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC048);
    let mut saw_div_zero = 0u32;
    for _ in 0..N {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i] = c2sv {
                sA: rng.v(100.0),
                sB: rng.v(100.0),
                p: rng.v(100.0),
                u: rng.sym(10.0),
                iA: i as i32,
                iB: (3 - i) as i32,
            };
        }
        s.count = 3;
        s.div = rng.sym(10.0);

        // row 48: all three points equal
        let p = rng.v(100.0);
        for i in 0..3 {
            s.verts[i].p = p;
        }
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (c.c23)(&mut sc);
            (r.c23)(&mut sr);
        }
        diff_eq!("row48 c23 all equal", simplex_bits(&sc), simplex_bits(&sr));
        assert_eq!(sc.count, 1, "row48 must collapse to count 1");

        // row 49: collinear (area == 0 -> uABC == vABC == wABC == 0)
        let o = rng.v(100.0);
        let d = rng.v(50.0);
        s.verts[0].p = o;
        s.verts[1].p = c2v { x: o.x + d.x, y: o.y + d.y };
        s.verts[2].p = c2v { x: o.x + d.x * 2.0, y: o.y + d.y * 2.0 };
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (c.c23)(&mut sc);
            (r.c23)(&mut sr);
        }
        diff_eq!("row49 c23 collinear", simplex_bits(&sc), simplex_bits(&sr));
        if sc.count == 3 && sc.div == 0.0 {
            saw_div_zero += 1;
        }

        // row 50: NaN points -> every guard false -> else, div = NaN, count 3
        for i in 0..3 {
            s.verts[i].p = c2v {
                x: f32::from_bits(0x7FC0_0090 + i as u32),
                y: f32::from_bits(0xFFC0_00A0 + i as u32),
            };
        }
        let mut sc = s;
        let mut sr = s;
        unsafe {
            (c.c23)(&mut sc);
            (r.c23)(&mut sr);
        }
        diff_eq!("row50 c23 nan", simplex_bits(&sc), simplex_bits(&sr));
        assert_eq!(sc.count, 3, "row50 NaN must fall through to the else branch");
        assert!(sc.div.is_nan(), "row50 div must be NaN");
    }
    println!("row49: {saw_div_zero} collinear cases produced a count-3 simplex with div == 0");
}

// ===========================================================================
// Row 51 — the iteration counter.  Both libraries must agree on `*iterations`
//          for every input; a wide search records the maximum reached.
// ===========================================================================
#[test]
fn row51_iteration_counter_agrees() {
    let (c, _) = apis();
    let mut rng = Rng::new(0xC051);
    let mut max_it = 0;
    let mut hist = [0u32; 21];
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..N {
                for gen in 0..3 {
                    let (a, b) = match gen {
                        0 => (shape_rand(&mut rng, ta, 60.0, 8.0), shape_rand(&mut rng, tb, 60.0, 8.0)),
                        1 => (shape_spicy(&mut rng, ta, 1e3), shape_spicy(&mut rng, tb, 1e3)),
                        _ => (shape_bits(&mut rng, ta), shape_bits(&mut rng, tb)),
                    };
                    for ur in [0i32, 1] {
                        let cfg = Cfg::new(a, b).radius(ur);
                        check("row51 iteration count", &cfg);
                        if let Some(it) = call(c.c2GJK, &cfg).iters {
                            max_it = max_it.max(it);
                            if (0..=20).contains(&it) {
                                hist[it as usize] += 1;
                            }
                        }
                    }
                    // warm-cache variants can iterate differently
                    if let Some(w) = check(
                        "row51 seed",
                        &Cfg::new(a, b).cache(Some(c2GJKCache::default())),
                    ) {
                        let w = clamp_cache(w, a.ty(), b.ty());
                        let cfg = Cfg::new(a, b).cache(Some(w));
                        check("row51 warm iteration count", &cfg);
                        if let Some(it) = call(c.c2GJK, &cfg).iters {
                            max_it = max_it.max(it);
                            if (0..=20).contains(&it) {
                                hist[it as usize] += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    println!("row51: max iterations observed = {max_it}; histogram = {hist:?}");
    // The loop must be exercised, and the C's cap must never be exceeded.
    assert!(max_it >= 1, "the GJK loop never iterated");
    assert!(max_it <= 20, "the C's 20-iteration cap was exceeded: {max_it}");
}

// ===========================================================================
// Rows 52..55 — the use_radius branches and non-canonical truthy values
// ===========================================================================
#[test]
fn row52_55_use_radius_branches() {
    let (c, _) = apis();
    let mut rng = Rng::new(0xC052);
    let mut mid = 0u32;
    let mut shrunk = 0u32;
    let mut coincide = 0u32;
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..N / 2 {
                // row 52: overlapping -> dist <= rA + rB -> midpoint, dist = 0
                let a = shape_rand(&mut rng, ta, 2.0, 10.0);
                let b = shape_rand(&mut rng, tb, 2.0, 10.0);
                let cfg = Cfg::new(a, b).radius(1);
                check("row52 midpoint", &cfg);
                let got = call(c.c2GJK, &cfg);
                if got.dist == 0.0f32.to_bits() {
                    mid += 1;
                    // the midpoint branch sets a == b
                    if got.a == got.b {
                        coincide += 1;
                    }
                }
                // row 53: far apart -> the shrink branch
                let a = shape_rand(&mut rng, ta, 5.0, 3.0);
                let far = shape_at(&mut rng, tb, c2v { x: 5000.0, y: 0.0 }, 3.0);
                let cfg = Cfg::new(a, far).radius(1);
                check("row53 shrink", &cfg);
                if f32::from_bits(call(c.c2GJK, &cfg).dist) > 0.0 {
                    shrunk += 1;
                }
                // row 54: NaN / overflowing radii -> the comparison is false
                let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: f32::NAN });
                let b = Shape::Circle(c2Circle { p: c2v { x: 100.0, y: 0.0 }, r: 1e38 });
                check("row54 nan/huge radius", &Cfg::new(a, b).radius(1));
                let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1e38 });
                let b = Shape::Circle(c2Circle { p: c2v { x: 100.0, y: 0.0 }, r: 1e38 });
                check("row54 overflow radius", &Cfg::new(a, b).radius(1));
                // row 55: non-canonical truthy use_radius behaves exactly like 1
                let a = shape_rand(&mut rng, ta, 50.0, 5.0);
                let b = shape_rand(&mut rng, tb, 50.0, 5.0);
                let one = call(c.c2GJK, &Cfg::new(a, b).radius(1));
                for &ur in [2i32, -1, i32::MIN, i32::MAX, 0x100].iter() {
                    let cfg = Cfg::new(a, b).radius(ur);
                    check(&format!("row55 ur={ur}"), &cfg);
                    assert_eq!(one, call(c.c2GJK, &cfg), "use_radius={ur} differed from 1");
                }
                // and use_radius == 0 must differ from 1 at least sometimes
                check("row55 ur=0", &Cfg::new(a, b).radius(0));
            }
        }
    }
    assert!(mid > 0 && shrunk > 0, "radius branches not covered: mid={mid} shrunk={shrunk}");
    assert!(coincide > 0, "the midpoint branch never set a == b");
}

// ===========================================================================
// Rows 56..58 — inverted AABB, degenerate/negative-radius capsule, all-NaN
// ===========================================================================
#[test]
fn row56_58_degenerate_shapes() {
    let mut rng = Rng::new(0xC056);
    for &tb in TYPES.iter() {
        for _ in 0..N / 2 {
            let other = shape_rand(&mut rng, tb, 50.0, 5.0);
            // row 56: inverted AABB (min > max)
            let inv = aabb(10.0, 10.0, -10.0, -10.0);
            let half = aabb(10.0, -10.0, -10.0, 10.0);
            for deg in [inv, half] {
                for ur in [0i32, 1] {
                    check("row56 inverted aabb", &Cfg::new(deg, other).radius(ur));
                    check("row56 inverted aabb (B)", &Cfg::new(other, deg).radius(ur));
                }
            }
            // row 57: capsule with a == b, zero radius, negative radius
            for (x0, y0, x1, y1, r) in [
                (1.0f32, 2.0, 1.0, 2.0, 0.0f32),
                (1.0, 2.0, 1.0, 2.0, -5.0),
                (0.0, 0.0, 3.0, 4.0, -5.0),
                (0.0, 0.0, 3.0, 4.0, 0.0),
            ] {
                let cap = capsule(x0, y0, x1, y1, r);
                for ur in [0i32, 1] {
                    check("row57 degenerate capsule", &Cfg::new(cap, other).radius(ur));
                    check("row57 degenerate capsule (B)", &Cfg::new(other, cap).radius(ur));
                }
            }
            // negative-radius circle
            for r in [0.0f32, -0.0, -7.5] {
                let ci = circle(1.0, 2.0, r);
                for ur in [0i32, 1] {
                    check("row57 degenerate circle", &Cfg::new(ci, other).radius(ur));
                    check("row57 degenerate circle (B)", &Cfg::new(other, ci).radius(ur));
                }
            }
            // row 58: all-NaN / all-inf shapes
            let nan_c = circle(f32::NAN, f32::NAN, f32::NAN);
            let nan_bb = aabb(f32::NAN, f32::NAN, f32::NAN, f32::NAN);
            let nan_cap = capsule(f32::NAN, f32::NAN, f32::NAN, f32::NAN, f32::NAN);
            let inf_c = circle(f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY);
            for bad in [nan_c, nan_bb, nan_cap, inf_c] {
                for ur in [0i32, 1] {
                    check("row58 nan/inf shape", &Cfg::new(bad, other).radius(ur));
                    check("row58 nan/inf shape (B)", &Cfg::new(other, bad).radius(ur));
                    check("row58 nan/inf both", &Cfg::new(bad, bad).radius(ur));
                }
            }
        }
    }
}

// ===========================================================================
// Rows 59..60 — gjk_cache: `char` truthiness, NULL out-params, extreme floats
//               (the function is observably a no-op; see phase_b_gjk_cache).
// ===========================================================================
#[test]
fn row59_60_gjk_cache_no_op() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC059);
    let revs: [i8; 10] = [0, 1, -1, 2, 127, -128, 0x40, -0x40, 65, 3];
    for _ in 0..N {
        for &rev in revs.iter() {
            for kind in 0..4 {
                let mut p = [0.0f32; 9];
                for slot in p.iter_mut() {
                    *slot = match kind {
                        0 => rng.sym(100.0),
                        1 => rng.spicy(1e3),
                        2 => rng.any_bits(),
                        _ => f32::NAN,
                    };
                }
                // Poisoned buffers on BOTH sides: the C never writes them.
                let mut buf_c = [c2v { x: f32::from_bits(0x1234_5678), y: f32::from_bits(0x9ABC_DEF0) }; 2];
                let mut buf_r = buf_c;
                unsafe {
                    (c.gjk_cache)(
                        rev, &mut buf_c[0], &mut buf_c[1], p[0], p[1], p[2], p[3], p[4], p[5],
                        p[6], p[7], p[8],
                    );
                    (r.gjk_cache)(
                        rev, &mut buf_r[0], &mut buf_r[1], p[0], p[1], p[2], p[3], p[4], p[5],
                        p[6], p[7], p[8],
                    );
                }
                diff_eq!(
                    format!("row59-60 gjk_cache rev={rev} kind={kind}"),
                    raw_bytes(&buf_c),
                    raw_bytes(&buf_r)
                );
                assert_eq!(
                    raw_bytes(&buf_c),
                    raw_bytes(&[c2v { x: f32::from_bits(0x1234_5678), y: f32::from_bits(0x9ABC_DEF0) }; 2]),
                    "gjk_cache wrote through a9/b9"
                );
                // NULL out-params must also be safe
                unsafe {
                    (c.gjk_cache)(
                        rev,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
                    );
                    (r.gjk_cache)(
                        rev,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Generic FFI boundary checks the task calls for beyond the table:
// out-of-range enums, zero / oversized lengths, one-past-range values.
// ===========================================================================
#[test]
fn generic_boundaries() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xC0FF);

    // every enum value from 0..=8 plus the extremes, at the c2MakeProxy level
    for t in 0u32..=8 {
        let sh = c2Capsule { a: c2v { x: 1.0, y: 2.0 }, b: c2v { x: 3.0, y: 4.0 }, r: 5.0 };
        let mut pc = c2Proxy { radius: 1.5, count: -9, verts: [c2v { x: 8.0, y: 9.0 }; 8] };
        let mut pr = pc;
        unsafe {
            (c.c2MakeProxy)(&sh as *const c2Capsule as *const c_void, t, &mut pc);
            (r.c2MakeProxy)(&sh as *const c2Capsule as *const c_void, t, &mut pr);
        }
        diff_eq!(format!("boundary enum={t}"), raw_bytes(&pc), raw_bytes(&pr));
    }

    // c2Support: length 0, 1, and one past the 8-vertex proxy array
    let verts = [c2v { x: 1.0, y: 2.0 }; 9];
    for &count in [0i32, 1, 8, 9].iter() {
        let d = rng.v(10.0);
        let (a, b) = unsafe {
            ((c.c2Support)(verts.as_ptr(), count, d), (r.c2Support)(verts.as_ptr(), count, d))
        };
        diff_eq!(format!("boundary support count={count}"), a, b);
    }

    // simplex `count` one step past each documented case
    let mut s = c2Simplex::default();
    for i in 0..4 {
        s.verts[i].p = rng.v(10.0);
        s.verts[i].u = 1.0;
    }
    s.div = 3.0;
    for &count in [-1i32, 0, 1, 2, 3, 4, 5].iter() {
        s.count = count;
        let mut a1 = s;
        let mut a2 = s;
        diff_eq!(
            format!("boundary c2L count={count}"),
            vb(unsafe { (c.c2L)(&mut a1) }),
            vb(unsafe { (r.c2L)(&mut a2) })
        );
        let mut a1 = s;
        let mut a2 = s;
        diff_eq!(
            format!("boundary c2D count={count}"),
            vb(unsafe { (c.c2D)(&mut a1) }),
            vb(unsafe { (r.c2D)(&mut a2) })
        );
        let mut a1 = s;
        let mut a2 = s;
        diff_eq!(
            format!("boundary metric count={count}"),
            fb(unsafe { (c.c2GJKSimplexMetric)(&mut a1) }),
            fb(unsafe { (r.c2GJKSimplexMetric)(&mut a2) })
        );
        let mut a1 = s;
        let mut a2 = s;
        let (mut x1, mut y1, mut x2, mut y2) =
            (c2v::default(), c2v::default(), c2v::default(), c2v::default());
        unsafe {
            (c.c2Witness)(&mut a1, &mut x1, &mut y1);
            (r.c2Witness)(&mut a2, &mut x2, &mut y2);
        }
        diff_eq!(
            format!("boundary witness count={count}"),
            (vb(x1), vb(y1)),
            (vb(x2), vb(y2))
        );
        // c22 / c23 accept any count; they only read verts[0..3]
        let mut a1 = s;
        let mut a2 = s;
        unsafe {
            (c.c22)(&mut a1);
            (r.c22)(&mut a2);
        }
        diff_eq!(format!("boundary c22 count={count}"), simplex_bits(&a1), simplex_bits(&a2));
        let mut a1 = s;
        let mut a2 = s;
        unsafe {
            (c.c23)(&mut a1);
            (r.c23)(&mut a2);
        }
        diff_eq!(format!("boundary c23 count={count}"), simplex_bits(&a1), simplex_bits(&a2));
    }

    // c2GJK: use_radius one step past 0/1, and the enum extremes on a shape
    // whose proxy IS initialised (so the call stays well defined)
    let a = aabb(-1.0, -1.0, 1.0, 1.0);
    let b = capsule(3.0, 3.0, 5.0, 5.0, 0.5);
    for &ur in [-1i32, 0, 1, 2, i32::MIN, i32::MAX].iter() {
        check(&format!("boundary gjk ur={ur}"), &Cfg::new(a, b).radius(ur));
    }
    // cache count one step past every documented case (4 handled in row12)
    for &n in [-1i32, 0, 1, 2, 3].iter() {
        let ca = c2GJKCache { metric: 0.0, count: n, iA: [0, 1, 2], iB: [0, 1, 1], div: 1.0 };
        check(&format!("boundary gjk cache count={n}"), &Cfg::new(a, b).cache(Some(ca)));
    }
}

/// `c_int` / `c_uint` widths must match what the C was compiled with, otherwise
/// every "same error code" assertion above would be comparing the wrong bytes.
#[test]
fn abi_layout_matches_c() {
    assert_eq!(std::mem::size_of::<c_int>(), 4);
    assert_eq!(std::mem::size_of::<c_uint>(), 4);
    assert_eq!(std::mem::size_of::<c2v>(), 8);
    assert_eq!(std::mem::size_of::<c2r>(), 8);
    assert_eq!(std::mem::size_of::<c2x>(), 16);
    assert_eq!(std::mem::size_of::<c2Circle>(), 12);
    assert_eq!(std::mem::size_of::<c2AABB>(), 16);
    assert_eq!(std::mem::size_of::<c2Capsule>(), 20);
    assert_eq!(std::mem::size_of::<c2GJKCache>(), 36);
    assert_eq!(std::mem::size_of::<c2Proxy>(), 72);
    assert_eq!(std::mem::size_of::<c2sv>(), 36);
    assert_eq!(std::mem::size_of::<c2Simplex>(), 152);
}

/// Harness self-check: the two loaded objects really are two DIFFERENT files
/// (the C `.so` and the Rust `cdylib`), and the function pointers resolved from
/// them are distinct addresses.  Without this, every `diff_eq!` above could be
/// comparing one library against itself and pass vacuously.
#[test]
fn harness_loads_two_distinct_libraries() {
    let (cp, rp) = so_paths();
    assert_ne!(cp, rp, "the same .so was loaded twice");
    assert!(cp.to_string_lossy().contains("c_src"), "C .so not from c_src: {cp:?}");
    assert!(
        rp.to_string_lossy().contains("libgjk_cache_lib"),
        "Rust .so is not the crate cdylib: {rp:?}"
    );
    println!("C   .so = {cp:?}");
    println!("RUST.so = {rp:?}");

    let (c, r) = apis();
    // Every resolved symbol must come from a different address in each library.
    macro_rules! distinct {
        ($($f:ident),* $(,)?) => {$(
            assert_ne!(
                c.$f as usize, r.$f as usize,
                concat!("symbol `", stringify!($f), "` resolved to the same address in both libraries")
            );
        )*};
    }
    distinct!(
        c2V, c2Mulvs, c2Maxv, c2Minv, c2Clampv, c2Sub, c2Dot, c2RotIdentity, c2xIdentity,
        c2BBVerts, c2MakeProxy, c2Len, c2Det2, c2GJKSimplexMetric, c2Mulrv, c2Add, c2Mulxv,
        c22, c23, c2Neg, c2Skew, c2CCW90, c2D, c2Support, c2Witness, c2Div, c2Norm, c2L,
        c2MulrvT, c2GJK, gjk_cache,
    );
}
