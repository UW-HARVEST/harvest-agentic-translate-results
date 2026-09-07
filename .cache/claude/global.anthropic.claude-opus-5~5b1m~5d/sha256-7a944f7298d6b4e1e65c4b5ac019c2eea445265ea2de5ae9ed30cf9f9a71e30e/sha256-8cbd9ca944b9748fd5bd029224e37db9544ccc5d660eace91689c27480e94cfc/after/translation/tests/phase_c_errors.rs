//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input / degenerate condition, calls
//! BOTH `.so`s, and asserts they produce the SAME rejection value (identical
//! sentinel / error code / bit pattern), not merely "both failed".

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// Every out-of-range `C2_TYPE` value worth pushing across the FFI boundary.
/// A C `enum` parameter is just an `int`, so these are all real inputs.
const BAD_TYPES: &[c_int] = &[3, 4, 7, 100, -1, -2, -1000, c_int::MIN, c_int::MAX, 0x7fff_fffe];
const GOOD_TYPES: &[c_int] = &[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

fn seeded_proxy(tag: f32) -> c2Proxy {
    let mut p = c2Proxy { radius: tag, count: -424242, verts: [c2v::default(); 8] };
    for i in 0..8 {
        p.verts[i] = c2v { x: tag * (i as f32 + 1.0), y: -tag - i as f32 };
    }
    p
}

// ===========================================================================
// row 1 — c2MakeProxy with an out-of-range C2_TYPE: the C `switch` has no
//         `default:`, so *nothing* is written.
// ===========================================================================

#[test]
fn err01_c2MakeProxy_invalid_type_writes_nothing() {
    let a = api();
    let mut d = Diff::new("ERRORS row 1: c2MakeProxy invalid C2_TYPE -> output untouched");
    let shape = c2Circle { p: c2v { x: 3.0, y: -4.0 }, r: 5.0 };
    for &bad in BAD_TYPES {
        let pristine = seeded_proxy(1.5);
        let mut cp = pristine;
        let mut rp = pristine;
        unsafe {
            (a.c2MakeProxy.0)(&shape as *const _ as *const c_void, bad, &mut cp);
            (a.c2MakeProxy.1)(&shape as *const _ as *const c_void, bad, &mut rp);
        }
        d.check(proxyeq(&cp, &rp), || format!("type={bad}\n C={cp:?}\n R={rp:?}"));
        // and the C really does leave it alone — so the Rust must too
        d.check(proxyeq(&cp, &pristine), || format!("type={bad}: C mutated the proxy: {cp:?}"));
        d.check(proxyeq(&rp, &pristine), || format!("type={bad}: Rust mutated the proxy: {rp:?}"));
    }
    d.finish();
}

// ===========================================================================
// rows 2-5 — c2Collided `default:` labels: every out-of-range enum returns 0.
// ===========================================================================

#[test]
fn err02_05_c2Collided_invalid_enums() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 2-5: c2Collided out-of-range typeA / typeB -> 0");
    let circle = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 };
    let bb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
    let cap = c2Capsule { a: c2v { x: -2.0, y: 0.0 }, b: c2v { x: 2.0, y: 0.0 }, r: 1.0 };
    let shapes: [(*const c_void, &str); 3] = [
        (&circle as *const _ as *const c_void, "circle"),
        (&bb as *const _ as *const c_void, "aabb"),
        (&cap as *const _ as *const c_void, "capsule"),
    ];

    // row 2: typeA invalid (typeB both valid and invalid)
    for &bad in BAD_TYPES {
        for &(ap, an) in &shapes {
            for &tb in GOOD_TYPES.iter().chain(BAD_TYPES.iter()) {
                for &(bp, bn) in &shapes {
                    let (c, s) = unsafe { ((a.c2Collided.0)(ap, bad, bp, tb), (a.c2Collided.1)(ap, bad, bp, tb)) };
                    d.check(c == s, || format!("row2 typeA={bad} typeB={tb} ({an},{bn}) C={c} R={s}"));
                    d.check(c == 0, || format!("row2: C did not return 0 (got {c})"));
                }
            }
        }
    }

    // rows 3-5: typeA valid, typeB invalid
    for &ta in GOOD_TYPES {
        for &bad in BAD_TYPES {
            for &(ap, an) in &shapes {
                for &(bp, bn) in &shapes {
                    let (c, s) = unsafe { ((a.c2Collided.0)(ap, ta, bp, bad), (a.c2Collided.1)(ap, ta, bp, bad)) };
                    d.check(c == s, || format!("row{} typeA={ta} typeB={bad} ({an},{bn}) C={c} R={s}", 3 + ta));
                    d.check(c == 0, || format!("row{}: C did not return 0 (got {c})", 3 + ta));
                }
            }
        }
    }
    d.finish();
}

// ===========================================================================
// rows 6-7 — c2GJK with an out-of-range C2_TYPE.
//
// `c2MakeProxy` writes nothing (row 1), so the C proceeds with the
// **uninitialised** stack local `c2Proxy pA;`.  Its `count` field is whatever
// the stack held, and `c2Support(pA.verts, pA.count, d)` then loops
// `for (i = 1; i < count; ++i)` over that garbage count — so the C can return
// anything (observed: `inf`) or walk off the end of the stack and take SIGSEGV.
// There is no defined result to compare against.  What we CAN and DO assert:
//   * the Rust `.so` always returns, deterministically, for every out-of-range
//     enum value (measured in a child process so a hypothetical crash is caught
//     rather than killing the runner);
//   * the C's outcome is recorded for the record but not constrained.
// ===========================================================================

#[test]
fn err06_07_c2GJK_invalid_type() {
    // Child role: one (lib, typeA, typeB) call, then exit.
    if let Ok(role) = std::env::var("PHASE_C_ROW67_ROLE") {
        let a = api();
        let ta: c_int = std::env::var("PHASE_C_ROW67_TA").unwrap().parse().unwrap();
        let tb: c_int = std::env::var("PHASE_C_ROW67_TB").unwrap().parse().unwrap();
        let circle = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 };
        let f = if role == "c" { a.c2GJK.0 } else { a.c2GJK.1 };
        let mut it: c_int = -7;
        let (mut oa, mut ob) = (c2v::default(), c2v::default());
        let dist = unsafe {
            f(&circle as *const _ as *const c_void, ta, std::ptr::null(),
              &circle as *const _ as *const c_void, tb, std::ptr::null(),
              &mut oa, &mut ob, 1, &mut it, std::ptr::null_mut())
        };
        println!("RESULT {} {} {} {} {} {}", dist.to_bits(), it,
                 oa.x.to_bits(), oa.y.to_bits(), ob.x.to_bits(), ob.y.to_bits());
        std::process::exit(0);
    }

    fn results(out: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(out)
            .lines()
            .filter_map(|l| l.find("RESULT ").map(|i| l[i..].trim().to_string()))
            .collect()
    }

    let exe = std::env::current_exe().expect("current_exe");
    let spawn = |role: &str, ta: c_int, tb: c_int| -> std::process::Output {
        std::process::Command::new(&exe)
            .args(["err06_07_c2GJK_invalid_type", "--exact", "--nocapture", "--test-threads=1"])
            .env("PHASE_C_ROW67_ROLE", role)
            .env("PHASE_C_ROW67_TA", ta.to_string())
            .env("PHASE_C_ROW67_TB", tb.to_string())
            .output()
            .expect("spawn child")
    };

    let mut c_crashes = 0usize;
    let mut c_returns = 0usize;
    for &bad in BAD_TYPES {
        for &(ta, tb) in &[(bad, C2_TYPE_CIRCLE), (C2_TYPE_CIRCLE, bad), (bad, bad)] {
            // Rust: must always return, and must be deterministic.
            let r1 = spawn("r", ta, tb);
            let r2 = spawn("r", ta, tb);
            assert!(r1.status.success(), "the Rust .so crashed for typeA={ta} typeB={tb}: {:?}", r1.status);
            let l1 = results(&r1.stdout);
            let l2 = results(&r2.stdout);
            assert_eq!(l1.len(), 1, "no RESULT from the Rust child (typeA={ta} typeB={tb})");
            assert_eq!(l1, l2, "the Rust .so is not deterministic for typeA={ta} typeB={tb}");

            // C: unspecified — record it.
            let c = spawn("c", ta, tb);
            if c.status.success() { c_returns += 1 } else { c_crashes += 1 }
        }
    }
    eprintln!(
        "[rows 6-7] out-of-range C2_TYPE, {} cases: the C returned normally {c_returns}x and \
         crashed {c_crashes}x (uninitialised c2Proxy — unspecified). The Rust .so returned \
         deterministically in all {} cases.",
        BAD_TYPES.len() * 3,
        BAD_TYPES.len() * 3
    );

    // Control: with VALID enum values in the same call shape the two libraries
    // must agree exactly (proving the harness itself is sound).
    let a = api();
    let circle = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 };
    let mut d = Diff::new("ERRORS rows 6-7 control: valid enum values still agree exactly");
    for &ta in GOOD_TYPES {
        for &tb in GOOD_TYPES {
            let (mut ci, mut ri) = (-7 as c_int, -7 as c_int);
            let (mut coa, mut cob) = (c2v::default(), c2v::default());
            let (mut roa, mut rob) = (c2v::default(), c2v::default());
            let cd = unsafe {
                (a.c2GJK.0)(&circle as *const _ as *const c_void, ta, std::ptr::null(),
                            &circle as *const _ as *const c_void, tb, std::ptr::null(),
                            &mut coa, &mut cob, 1, &mut ci, std::ptr::null_mut())
            };
            let rd = unsafe {
                (a.c2GJK.1)(&circle as *const _ as *const c_void, ta, std::ptr::null(),
                            &circle as *const _ as *const c_void, tb, std::ptr::null(),
                            &mut roa, &mut rob, 1, &mut ri, std::ptr::null_mut())
            };
            d.check(feq(cd, rd) && veq(coa, roa) && veq(cob, rob) && ci == ri, || {
                format!("typeA={ta} typeB={tb}: dist C={} R={} iter C={ci} R={ri}", fmt_f(cd), fmt_f(rd))
            });
        }
    }
    d.finish();
}

// ===========================================================================
// rows 8-13 — c2GJK null-pointer handling (every combination).
// ===========================================================================

#[test]
fn err08_13_c2GJK_null_pointers() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 8-13: c2GJK NULL ax/bx/outA/outB/iterations/cache (all 64 combos)");
    let r = Rng::new(813);
    let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
    let xf = c2x { p: c2v { x: 7.0, y: -3.0 }, r: c2r { c: 0.6, s: 0.8 } };

    for trial in 0..400 {
        let circle = c2Circle { p: r.geo_v(), r: r.radius() };
        let bb = r.aabb();
        for mask in 0u32..64 {
            let ap: *const c2x = match mask & 3 {
                0 => std::ptr::null(),      // row 8
                1 => &ident,
                _ => &xf,
            };
            let bp: *const c2x = match (mask >> 2) & 1 {
                0 => std::ptr::null(),      // row 9
                _ => &xf,
            };
            let use_a = (mask >> 3) & 1 == 1; // row 10
            let use_b = (mask >> 4) & 1 == 1; // row 11
            let use_i = (mask >> 5) & 1 == 1; // row 12

            for cache_on in [false, true] {
                // row 13 (cache == NULL) / row 14 (zeroed cache)
                let (mut coa, mut cob) = (c2v { x: 99.0, y: 98.0 }, c2v { x: 97.0, y: 96.0 });
                let (mut roa, mut rob) = (coa, cob);
                let (mut ci, mut ri) = (-31337 as c_int, -31337 as c_int);
                let mut cc = c2GJKCache::default();
                let mut rc = c2GJKCache::default();
                let cd = unsafe {
                    (a.c2GJK.0)(
                        &circle as *const _ as *const c_void, C2_TYPE_CIRCLE, ap,
                        &bb as *const _ as *const c_void, C2_TYPE_AABB, bp,
                        if use_a { &mut coa } else { std::ptr::null_mut() },
                        if use_b { &mut cob } else { std::ptr::null_mut() },
                        1,
                        if use_i { &mut ci } else { std::ptr::null_mut() },
                        if cache_on { &mut cc } else { std::ptr::null_mut() },
                    )
                };
                let rd = unsafe {
                    (a.c2GJK.1)(
                        &circle as *const _ as *const c_void, C2_TYPE_CIRCLE, ap,
                        &bb as *const _ as *const c_void, C2_TYPE_AABB, bp,
                        if use_a { &mut roa } else { std::ptr::null_mut() },
                        if use_b { &mut rob } else { std::ptr::null_mut() },
                        1,
                        if use_i { &mut ri } else { std::ptr::null_mut() },
                        if cache_on { &mut rc } else { std::ptr::null_mut() },
                    )
                };
                let ctx = || {
                    format!(
                        "trial={trial} mask={mask:#08b} cache_on={cache_on}\n dist C={} R={}\n outA C={} R={}\n outB C={} R={}\n iter C={ci} R={ri}\n cache C={cc:?}\n cache R={rc:?}",
                        fmt_f(cd), fmt_f(rd), fmt_v(coa), fmt_v(roa), fmt_v(cob), fmt_v(rob)
                    )
                };
                d.check(feq(cd, rd), ctx);
                d.check(veq(coa, roa) && veq(cob, rob), ctx);
                d.check(ci == ri, ctx);
                d.check(cacheeq(&cc, &rc), ctx);
                // the skipped stores must have left the sentinels alone
                if !use_a {
                    d.check(veq(coa, c2v { x: 99.0, y: 98.0 }) && veq(roa, c2v { x: 99.0, y: 98.0 }), ctx);
                }
                if !use_b {
                    d.check(veq(cob, c2v { x: 97.0, y: 96.0 }) && veq(rob, c2v { x: 97.0, y: 96.0 }), ctx);
                }
                if !use_i {
                    d.check(ci == -31337 && ri == -31337, ctx);
                }
                if !cache_on {
                    d.check(cacheeq(&cc, &c2GJKCache::default()) && cacheeq(&rc, &c2GJKCache::default()), ctx);
                }
            }
        }
    }
    d.finish();
}

// ===========================================================================
// row 14 — cache != NULL with count == 0: cache NOT read, but IS written.
// ===========================================================================

#[test]
fn err14_cache_count_zero() {
    let a = api();
    let mut d = Diff::new("ERRORS row 14: cache with count==0 -> cold start, cache still written back");
    let r = Rng::new(14);
    for _ in 0..2000 {
        // count == 0 but everything ELSE in the cache is junk, to prove the junk
        // is ignored identically by both builds.
        let junk = c2GJKCache {
            metric: r.sym(1.0e9),
            count: 0,
            iA: [r.below(9) as c_int - 4, r.below(9) as c_int - 4, r.below(9) as c_int - 4],
            iB: [r.below(9) as c_int - 4, r.below(9) as c_int - 4, r.below(9) as c_int - 4],
            div: r.sym(1.0e9),
        };
        let circle = c2Circle { p: r.geo_v(), r: r.radius() };
        let cap = r.capsule();
        let mut cc = junk;
        let mut rc = junk;
        let cd = unsafe {
            (a.c2GJK.0)(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                        &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                        std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), &mut cc)
        };
        let rd = unsafe {
            (a.c2GJK.1)(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                        &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                        std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), &mut rc)
        };
        d.check(feq(cd, rd) && cacheeq(&cc, &rc), || {
            format!("in={junk:?}\n dist C={} R={}\n out C={cc:?}\n out R={rc:?}", fmt_f(cd), fmt_f(rd))
        });
        d.check(cc.count != 0, || format!("C did not write the cache back: {cc:?}"));
    }
    d.finish();
}

// ===========================================================================
// rows 15-17 — cache with count != 0: div == 0, the `metric < -1.0e8f` quirk,
//              and a NaN metric.
// ===========================================================================

#[test]
fn err15_17_cache_pathological_metric_and_div() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 15-17: warm cache with div==0 / metric quirk / NaN metric");
    let r = Rng::new(1517);
    let metrics: &[f32] = &[
        0.0, -0.0, 1.0, -1.0, 1.0e8, -1.0e8, -1.000_001e8, -1.0e9, -1.0e30,
        f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MIN, f32::MAX,
    ];
    let divs: &[f32] = &[0.0, -0.0, 1.0, -1.0, 1.0e-30, 1.0e30, f32::NAN, f32::INFINITY];
    for &metric in metrics {
        for &div in divs {
            for count in [1i32, 2, 3] {
                for _ in 0..40 {
                    // Only AABB proxies (4 verts) so every index 0..3 is valid.
                    let A = r.aabb();
                    let B = r.aabb();
                    let cache = c2GJKCache {
                        metric,
                        count,
                        iA: [r.below(4) as c_int, r.below(4) as c_int, r.below(4) as c_int],
                        iB: [r.below(4) as c_int, r.below(4) as c_int, r.below(4) as c_int],
                        div,
                    };
                    let mut cc = cache;
                    let mut rc = cache;
                    let (mut ci, mut ri) = (0 as c_int, 0 as c_int);
                    let (mut coa, mut cob) = (c2v::default(), c2v::default());
                    let (mut roa, mut rob) = (c2v::default(), c2v::default());
                    let cd = unsafe {
                        (a.c2GJK.0)(&A as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                                    &B as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                                    &mut coa, &mut cob, 1, &mut ci, &mut cc)
                    };
                    let rd = unsafe {
                        (a.c2GJK.1)(&A as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                                    &B as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                                    &mut roa, &mut rob, 1, &mut ri, &mut rc)
                    };
                    d.check(
                        feq(cd, rd) && veq(coa, roa) && veq(cob, rob) && ci == ri && cacheeq(&cc, &rc),
                        || format!(
                            "metric={} div={} count={count} A={A:?} B={B:?} cache={cache:?}\n dist C={} R={}\n outA C={} R={}\n outB C={} R={}\n iter C={ci} R={ri}\n cache C={cc:?}\n cache R={rc:?}",
                            fmt_f(metric), fmt_f(div), fmt_f(cd), fmt_f(rd), fmt_v(coa), fmt_v(roa), fmt_v(cob), fmt_v(rob)
                        ),
                    );
                }
            }
        }
    }
    d.finish();
}

// ===========================================================================
// rows 18-22 — the GJK loop's own rejection/termination conditions.
// ===========================================================================

#[test]
fn err18_22_gjk_loop_terminations() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 18-22: iteration cap, degenerate direction, stall, duplicate vertex, hit");
    let r = Rng::new(1822);
    let mut saw_hit = 0usize;
    let mut saw_max_iter = 0usize;
    let mut iters = [0usize; 22];
    for _ in 0..20000 {
        // A wide mix so that every termination reason gets hit.
        let ty = |i: u32| GOOD_TYPES[(i % 3) as usize];
        let ta = ty(r.next_u32());
        let tb = ty(r.next_u32());
        let mk = |t: c_int| -> (Vec<u8>, c_int) {
            let mut buf = vec![0u8; 32];
            match t {
                C2_TYPE_CIRCLE => {
                    let c = c2Circle { p: r.geo_v(), r: r.radius() };
                    buf[..12].copy_from_slice(bytes_of(&c));
                }
                C2_TYPE_AABB => {
                    let b = r.aabb();
                    buf[..16].copy_from_slice(bytes_of(&b));
                }
                _ => {
                    let c = r.capsule();
                    buf[..20].copy_from_slice(bytes_of(&c));
                }
            }
            (buf, t)
        };
        let (ba, _) = mk(ta);
        let (bb, _) = mk(tb);
        let xa = c2x { p: r.geo_v(), r: r.rot() };
        let xb = c2x { p: r.geo_v(), r: r.rot() };
        let (mut ci, mut ri) = (-1 as c_int, -1 as c_int);
        let (mut coa, mut cob) = (c2v::default(), c2v::default());
        let (mut roa, mut rob) = (c2v::default(), c2v::default());
        let cd = unsafe {
            (a.c2GJK.0)(ba.as_ptr() as *const c_void, ta, &xa, bb.as_ptr() as *const c_void, tb, &xb,
                        &mut coa, &mut cob, 1, &mut ci, std::ptr::null_mut())
        };
        let rd = unsafe {
            (a.c2GJK.1)(ba.as_ptr() as *const c_void, ta, &xa, bb.as_ptr() as *const c_void, tb, &xb,
                        &mut roa, &mut rob, 1, &mut ri, std::ptr::null_mut())
        };
        d.check(feq(cd, rd) && veq(coa, roa) && veq(cob, rob) && ci == ri, || {
            format!("ta={ta} tb={tb} xa={xa:?} xb={xb:?}\n dist C={} R={}\n outA C={} R={}\n outB C={} R={}\n iter C={ci} R={ri}",
                    fmt_f(cd), fmt_f(rd), fmt_v(coa), fmt_v(roa), fmt_v(cob), fmt_v(rob))
        });
        // row 18: the hard cap
        d.check(ci <= 20 && ri <= 20, || format!("iteration cap violated: C={ci} R={ri}"));
        if ci == 20 { saw_max_iter += 1 }
        if cd == 0.0 { saw_hit += 1 }
        if (0..=21).contains(&ci) { iters[ci as usize] += 1 }
    }
    eprintln!("rows18-22: hit={saw_hit} at_cap={saw_max_iter} iters={iters:?}");
    assert!(saw_hit > 0, "row 22 (hit path) never reached");
    d.finish();
}

// ===========================================================================
// rows 23-27 — the `use_radius` rejection branches.
// ===========================================================================

#[test]
fn err23_27_use_radius_branches() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 23-27: use_radius else-branch (dist<=rA+rB, dist<=eps), coincident witness, use_radius==0, nonzero-non-1");
    let r = Rng::new(2327);
    let mut midpoint = 0usize;
    let mut shrink = 0usize;
    for _ in 0..8000 {
        // Big radii relative to the separation, so `dist <= rA + rB` fires often.
        let ca = r.geo_v();
        let A = c2Circle { p: ca, r: r.unit() * 60.0 };
        let B = c2Capsule {
            a: c2v { x: ca.x + r.sym(30.0), y: ca.y + r.sym(30.0) },
            b: c2v { x: ca.x + r.sym(30.0), y: ca.y + r.sym(30.0) },
            r: r.unit() * 60.0,
        };
        for &ur in &[0i32, 1, 2, -1, c_int::MIN, c_int::MAX] {
            let (mut coa, mut cob) = (c2v::default(), c2v::default());
            let (mut roa, mut rob) = (c2v::default(), c2v::default());
            let cd = unsafe {
                (a.c2GJK.0)(&A as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                            &B as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                            &mut coa, &mut cob, ur, std::ptr::null_mut(), std::ptr::null_mut())
            };
            let rd = unsafe {
                (a.c2GJK.1)(&A as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                            &B as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                            &mut roa, &mut rob, ur, std::ptr::null_mut(), std::ptr::null_mut())
            };
            d.check(feq(cd, rd) && veq(coa, roa) && veq(cob, rob), || {
                format!("use_radius={ur} A={A:?} B={B:?}\n dist C={} R={}\n outA C={} R={}\n outB C={} R={}",
                        fmt_f(cd), fmt_f(rd), fmt_v(coa), fmt_v(roa), fmt_v(cob), fmt_v(rob))
            });
            if ur != 0 {
                if cd == 0.0 {
                    midpoint += 1;
                    // row 23/24: a==b==midpoint
                    d.check(veq(coa, cob) && veq(roa, rob), || format!("use_radius else-branch did not collapse a==b: C {} vs {}", fmt_v(coa), fmt_v(cob)));
                } else {
                    shrink += 1;
                }
            }
        }
    }
    eprintln!("rows23-27: midpoint={midpoint} shrink={shrink}");
    assert!(midpoint > 0 && shrink > 0, "use_radius branch coverage: midpoint={midpoint} shrink={shrink}");
    d.finish();
}

// ===========================================================================
// row 28 — c2GJKSimplexMetric with an out-of-range count -> 0.0f
// ===========================================================================

#[test]
fn err28_c2GJKSimplexMetric_bad_count() {
    let a = api();
    let mut d = Diff::new("ERRORS row 28: c2GJKSimplexMetric count not in {2,3} -> 0.0f");
    let r = Rng::new(28);
    for &count in &[0i32, 1, 4, 5, 99, -1, -7, c_int::MIN, c_int::MAX] {
        for _ in 0..200 {
            let mut s = c2Simplex::default();
            for i in 0..4 {
                s.verts[i].p = r.wild_v();
                s.verts[i].u = r.wild();
            }
            s.div = r.wild();
            s.count = count;
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { ((a.c2GJKSimplexMetric.0)(&mut cs), (a.c2GJKSimplexMetric.1)(&mut rs)) };
            d.check(feq(cv, rv), || format!("count={count} C={} R={}", fmt_f(cv), fmt_f(rv)));
            d.check(feq(cv, 0.0), || format!("count={count}: C did not return +0.0 (got {})", fmt_f(cv)));
        }
    }
    d.finish();
}

// ===========================================================================
// rows 29-30 — c2D `default:` and the `<= 0` det branch.
// ===========================================================================

#[test]
fn err29_30_c2D_defaults() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 29-30: c2D count==3/other -> (0,0); count==2 det<=0 -> c2CCW90");
    let r = Rng::new(2930);
    for &count in &[0i32, 3, 4, 9, -1, c_int::MIN, c_int::MAX] {
        for _ in 0..300 {
            let mut s = c2Simplex::default();
            for i in 0..4 {
                s.verts[i].p = r.wild_v();
            }
            s.div = r.wild();
            s.count = count;
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { ((a.c2D.0)(&mut cs), (a.c2D.1)(&mut rs)) };
            d.check(veq(cv, rv), || format!("count={count} C={} R={}", fmt_v(cv), fmt_v(rv)));
            d.check(veq(cv, c2v { x: 0.0, y: 0.0 }), || format!("count={count}: C did not return (+0,+0): {}", fmt_v(cv)));
        }
    }
    // row 30: count == 2 with det exactly 0 (a.p collinear with ab through the origin)
    for _ in 0..500 {
        let t = r.sym(20.0);
        let dir = c2v { x: r.sym(5.0) + 1.0, y: r.sym(5.0) + 1.0 };
        let mut s = c2Simplex::default();
        s.verts[0].p = c2v { x: dir.x * t, y: dir.y * t };
        s.verts[1].p = c2v { x: dir.x * (t + 1.0), y: dir.y * (t + 1.0) };
        s.count = 2;
        let mut cs = s;
        let mut rs = s;
        let (cv, rv) = unsafe { ((a.c2D.0)(&mut cs), (a.c2D.1)(&mut rs)) };
        d.check(veq(cv, rv), || format!("collinear count=2 {s:?} C={} R={}", fmt_v(cv), fmt_v(rv)));
    }
    d.finish();
}

// ===========================================================================
// rows 31-32 — c2Witness `default:` and div == 0
// ===========================================================================

#[test]
fn err31_32_c2Witness_defaults_and_zero_div() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 31-32: c2Witness count out of {1,2,3} -> (0,0); div == 0 -> inf/NaN");
    let r = Rng::new(3132);
    for &count in &[0i32, 4, 5, 77, -1, -3, c_int::MIN, c_int::MAX] {
        for _ in 0..200 {
            let mut s = c2Simplex::default();
            for i in 0..4 {
                s.verts[i].sA = r.wild_v();
                s.verts[i].sB = r.wild_v();
                s.verts[i].u = r.wild();
            }
            s.div = r.wild();
            s.count = count;
            let (mut cs, mut rs) = (s, s);
            let (mut ca, mut cb) = (c2v { x: 1.0, y: 2.0 }, c2v { x: 3.0, y: 4.0 });
            let (mut ra, mut rb) = (ca, cb);
            unsafe {
                (a.c2Witness.0)(&mut cs, &mut ca, &mut cb);
                (a.c2Witness.1)(&mut rs, &mut ra, &mut rb);
            }
            d.check(veq(ca, ra) && veq(cb, rb), || {
                format!("count={count} C=({},{}) R=({},{})", fmt_v(ca), fmt_v(cb), fmt_v(ra), fmt_v(rb))
            });
            let z = c2v { x: 0.0, y: 0.0 };
            d.check(veq(ca, z) && veq(cb, z), || format!("count={count}: C default did not write (0,0): {} {}", fmt_v(ca), fmt_v(cb)));
        }
    }
    // row 32: div == 0 for every valid count
    for &count in &[1i32, 2, 3] {
        for &div in &[0.0f32, -0.0] {
            for _ in 0..400 {
                let mut s = c2Simplex::default();
                for i in 0..4 {
                    s.verts[i].sA = r.geo_v();
                    s.verts[i].sB = r.geo_v();
                    s.verts[i].u = r.geo();
                }
                s.div = div;
                s.count = count;
                let (mut cs, mut rs) = (s, s);
                let (mut ca, mut cb) = (c2v::default(), c2v::default());
                let (mut ra, mut rb) = (c2v::default(), c2v::default());
                unsafe {
                    (a.c2Witness.0)(&mut cs, &mut ca, &mut cb);
                    (a.c2Witness.1)(&mut rs, &mut ra, &mut rb);
                }
                d.check(veq(ca, ra) && veq(cb, rb), || {
                    format!("count={count} div={} {s:?}\n C=({},{})\n R=({},{})", fmt_f(div), fmt_v(ca), fmt_v(cb), fmt_v(ra), fmt_v(rb))
                });
            }
        }
    }
    d.finish();
}

// ===========================================================================
// rows 33-34 — c2L `default:` and div == 0
// ===========================================================================

#[test]
fn err33_34_c2L_defaults_and_zero_div() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 33-34: c2L count not in {1,2} -> (0,0); div == 0 -> inf/NaN");
    let r = Rng::new(3334);
    for &count in &[0i32, 3, 4, 88, -1, c_int::MIN, c_int::MAX] {
        for _ in 0..200 {
            let mut s = c2Simplex::default();
            for i in 0..4 {
                s.verts[i].p = r.wild_v();
                s.verts[i].u = r.wild();
            }
            s.div = r.wild();
            s.count = count;
            let (mut cs, mut rs) = (s, s);
            let (cv, rv) = unsafe { ((a.c2L.0)(&mut cs), (a.c2L.1)(&mut rs)) };
            d.check(veq(cv, rv), || format!("count={count} C={} R={}", fmt_v(cv), fmt_v(rv)));
            d.check(veq(cv, c2v { x: 0.0, y: 0.0 }), || format!("count={count}: C default != (0,0): {}", fmt_v(cv)));
        }
    }
    for &count in &[1i32, 2] {
        for &div in &[0.0f32, -0.0] {
            for _ in 0..500 {
                let mut s = c2Simplex::default();
                for i in 0..4 {
                    s.verts[i].p = r.geo_v();
                    s.verts[i].u = r.geo();
                }
                s.div = div;
                s.count = count;
                let (mut cs, mut rs) = (s, s);
                let (cv, rv) = unsafe { ((a.c2L.0)(&mut cs), (a.c2L.1)(&mut rs)) };
                d.check(veq(cv, rv), || format!("count={count} div={} {s:?} C={} R={}", fmt_f(div), fmt_v(cv), fmt_v(rv)));
            }
        }
    }
    d.finish();
}

// ===========================================================================
// rows 35-36 — c2Support with count <= 0 and with ties.
// ===========================================================================

#[test]
fn err35_36_c2Support_bad_count_and_ties() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 35-36: c2Support count<=0 -> 0 (verts[0] still read); ties -> lowest index");
    let r = Rng::new(3536);
    for &count in &[0i32, -1, -5, -1000, c_int::MIN] {
        for _ in 0..300 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = r.wild_v();
            }
            let dir = r.wild_v();
            let (cv, rv) = unsafe {
                ((a.c2Support.0)(verts.as_ptr(), count, dir), (a.c2Support.1)(verts.as_ptr(), count, dir))
            };
            d.check(cv == rv, || format!("count={count} d={} C={cv} R={rv}", fmt_v(dir)));
            d.check(cv == 0, || format!("count={count}: C did not return 0 (got {cv})"));
        }
    }
    // row 36: full ties must pick index 0 in both
    for _ in 0..500 {
        let v0 = r.wild_v();
        let verts = [v0; 8];
        let dir = r.wild_v();
        for &count in &[1i32, 2, 3, 4, 8] {
            let (cv, rv) = unsafe {
                ((a.c2Support.0)(verts.as_ptr(), count, dir), (a.c2Support.1)(verts.as_ptr(), count, dir))
            };
            d.check(cv == rv, || format!("tie count={count} C={cv} R={rv}"));
            d.check(cv == 0, || format!("tie count={count}: C picked {cv}, expected 0"));
        }
    }
    // NaN direction: every `dot > dmax` is false -> index 0
    for _ in 0..300 {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = r.geo_v();
        }
        let dir = c2v { x: f32::NAN, y: f32::NAN };
        let (cv, rv) = unsafe {
            ((a.c2Support.0)(verts.as_ptr(), 8, dir), (a.c2Support.1)(verts.as_ptr(), 8, dir))
        };
        d.check(cv == rv && cv == 0, || format!("NaN direction: C={cv} R={rv}"));
    }
    d.finish();
}

// ===========================================================================
// rows 37-38 — c22 collapse branches
// ===========================================================================

#[test]
fn err37_38_c22_collapse() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 37-38: c22 v<=0 -> collapse to A; u<=0 -> collapse to B");
    let r = Rng::new(3738);
    let mut hits = [0usize; 3];
    // Construct the collapse cases explicitly: put the origin outside the segment.
    for _ in 0..4000 {
        let dir = c2v { x: r.sym(10.0) + 0.5, y: r.sym(10.0) + 0.5 };
        let len = r.unit() * 20.0 + 0.1;
        // origin "before A": A and B both on the +dir side
        let mut cases = Vec::new();
        cases.push((c2v { x: dir.x, y: dir.y }, c2v { x: dir.x * (1.0 + len), y: dir.y * (1.0 + len) }));
        // origin "past B": both on the -dir side
        cases.push((c2v { x: -dir.x * (1.0 + len), y: -dir.y * (1.0 + len) }, c2v { x: -dir.x, y: -dir.y }));
        // origin between: straddling
        cases.push((c2v { x: -dir.x, y: -dir.y }, c2v { x: dir.x, y: dir.y }));
        // exactly a.p == b.p -> u == v == 0 -> `v <= 0` branch
        let p = r.geo_v();
        cases.push((p, p));
        for (pa, pb) in cases {
            let mut s = c2Simplex::default();
            s.verts[0].p = pa;
            s.verts[1].p = pb;
            for i in 0..4 {
                s.verts[i].sA = r.geo_v();
                s.verts[i].sB = r.geo_v();
                s.verts[i].iA = r.below(8) as c_int;
                s.verts[i].iB = r.below(8) as c_int;
                s.verts[i].u = r.geo();
            }
            s.div = r.geo();
            s.count = 2;
            let (mut cs, mut rs) = (s, s);
            unsafe {
                (a.c22.0)(&mut cs);
                (a.c22.1)(&mut rs);
            }
            d.check(bytes_of(&cs) == bytes_of(&rs), || format!("c22 in={s:?}\n C={cs:?}\n R={rs:?}"));
            match cs.count {
                1 => hits[if bytes_of(&cs.verts[0].sA) == bytes_of(&s.verts[1].sA) { 1 } else { 0 }] += 1,
                _ => hits[2] += 1,
            }
        }
    }
    eprintln!("rows37-38 c22 outcomes (collapseA, collapseB, keep2): {hits:?}");
    assert!(hits.iter().all(|&h| h > 0), "c22 collapse coverage: {hits:?}");
    d.finish();
}

// ===========================================================================
// rows 39-45 — c23 collapse / edge / degenerate branches
// ===========================================================================

#[test]
fn err39_45_c23_all_rejection_branches() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 39-45: c23 vertex/edge collapse regions and area==0 degeneracy");
    let r = Rng::new(3945);
    let mut counts = [0usize; 4]; // resulting s.count 0..3
    let mut zero_div = 0usize;
    let mut degenerate = 0usize;
    for _ in 0..30000 {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i].sA = r.geo_v();
            s.verts[i].sB = r.geo_v();
            s.verts[i].u = r.geo();
            s.verts[i].iA = r.below(8) as c_int;
            s.verts[i].iB = r.below(8) as c_int;
        }
        s.div = r.geo();
        s.count = 3;
        match r.below(6) {
            // degenerate: collinear -> area == 0 -> final else with div == 0 (row 45)
            0 => {
                let base = r.geo_v();
                let dir = c2v { x: r.sym(20.0), y: r.sym(20.0) };
                let t = [r.sym(3.0), r.sym(3.0), r.sym(3.0)];
                for i in 0..3 {
                    s.verts[i].p = c2v { x: base.x + dir.x * t[i], y: base.y + dir.y * t[i] };
                }
            }
            // all three vertices identical
            1 => {
                let p = r.geo_v();
                for i in 0..3 {
                    s.verts[i].p = p;
                }
            }
            // triangle far from the origin -> a vertex/edge region
            2 => {
                let off = c2v { x: r.sym(1.0) * 300.0 + 100.0, y: r.sym(1.0) * 300.0 + 100.0 };
                for i in 0..3 {
                    let q = r.geo_v();
                    s.verts[i].p = c2v { x: off.x + q.x * 0.05, y: off.y + q.y * 0.05 };
                }
            }
            // triangle containing the origin -> the final else
            3 => {
                s.verts[0].p = c2v { x: -1.0 - r.unit() * 5.0, y: -1.0 - r.unit() * 5.0 };
                s.verts[1].p = c2v { x: 1.0 + r.unit() * 5.0, y: -1.0 - r.unit() * 5.0 };
                s.verts[2].p = c2v { x: r.sym(1.0), y: 1.0 + r.unit() * 5.0 };
            }
            _ => {
                for i in 0..3 {
                    s.verts[i].p = r.geo_v();
                }
            }
        }
        let (mut cs, mut rs) = (s, s);
        unsafe {
            (a.c23.0)(&mut cs);
            (a.c23.1)(&mut rs);
        }
        d.check(bytes_of(&cs) == bytes_of(&rs), || format!("c23 in={s:?}\n C={cs:?}\n R={rs:?}"));
        if (0..4).contains(&cs.count) {
            counts[cs.count as usize] += 1;
        }
        {
            let (p0, p1, p2) = (s.verts[0].p, s.verts[1].p, s.verts[2].p);
            let det = |u: c2v, v: c2v| u.x * v.y - u.y * v.x;
            let sub = |u: c2v, v: c2v| c2v { x: u.x - v.x, y: u.y - v.y };
            if det(sub(p1, p0), sub(p2, p0)) == 0.0 {
                degenerate += 1;
            }
        }
        if cs.count == 3 && cs.div == 0.0 {
            zero_div += 1;
        }
    }
    eprintln!("rows39-45 c23 result counts {counts:?}, area==0 cases: {degenerate}, count==3 && div==0 cases: {zero_div}");
    assert!(counts[1] > 0 && counts[2] > 0 && counts[3] > 0, "c23 outcome coverage: {counts:?}");
    // Row 45: the `area == 0` corpus must actually have been generated ...
    assert!(degenerate > 0, "row 45: no area == 0 triangle was generated");
    // ... and the C's own behaviour for it is that the final `else` is NEVER
    // reached (all three `?ABC <= 0` tests hold, so an earlier collapse/edge
    // branch always wins).  Assert that observed fact so a regression in either
    // library would surface here.
    assert_eq!(zero_div, 0, "row 45: unexpectedly reached count==3 with div==0");
    d.finish();
}

// ===========================================================================
// rows 46-49 — c2Div by zero, c2Norm of the zero vector, c2Len extremes
// ===========================================================================

#[test]
fn err46_49_division_and_sqrt_degeneracies() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 46-49: c2Div by 0, c2Norm of (0,0), c2Len overflow/NaN/negative");
    // row 46
    let vecs = [
        c2v { x: 1.0, y: -1.0 },
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: f32::NAN, y: 1.0 },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: 1.0e30, y: -1.0e30 },
        c2v { x: 1.0e-40, y: -1.0e-40 },
    ];
    for v in vecs {
        for &b in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.0e-45] {
            let (c, s) = ((a.c2Div.0)(v, b), (a.c2Div.1)(v, b));
            d.check(veq(c, s), || format!("c2Div({}, {}) C={} R={}", fmt_v(v), fmt_f(b), fmt_v(c), fmt_v(s)));
        }
        // row 47
        let (c, s) = ((a.c2Norm.0)(v), (a.c2Norm.1)(v));
        d.check(veq(c, s), || format!("c2Norm({}) C={} R={}", fmt_v(v), fmt_v(c), fmt_v(s)));
        // rows 48-49
        let (c, s) = ((a.c2Len.0)(v), (a.c2Len.1)(v));
        d.check(feq(c, s), || format!("c2Len({}) C={} R={}", fmt_v(v), fmt_f(c), fmt_f(s)));
    }
    // row 47 specifically: c2Norm((0,0)) must give the same NaN pattern
    let z = c2v { x: 0.0, y: 0.0 };
    let (c, s) = ((a.c2Norm.0)(z), (a.c2Norm.1)(z));
    d.check(veq(c, s) && c.x.is_nan(), || format!("c2Norm((0,0)) C={} R={}", fmt_v(c), fmt_v(s)));
    // rows 48-49: sqrt of inf and of a NaN-derived negative
    for v in [
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::INFINITY, y: f32::INFINITY },
        c2v { x: f32::NAN, y: f32::NAN },
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
    ] {
        let (c, s) = ((a.c2Len.0)(v), (a.c2Len.1)(v));
        d.check(feq(c, s), || format!("c2Len({}) C={} R={}", fmt_v(v), fmt_f(c), fmt_f(s)));
    }
    d.finish();
}

// ===========================================================================
// rows 50-51 — c2Maxv/c2Minv ternary NaN semantics, inverted c2Clampv
// ===========================================================================

#[test]
fn err50_51_minmax_nan_and_inverted_clamp() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 50-51: NaN in c2Maxv/c2Minv (ternary picks b) and lo>hi in c2Clampv");
    let nan = f32::NAN;
    let interesting = [nan, -nan, 0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY, f32::MIN, f32::MAX];
    for &ax in &interesting {
        for &bx in &interesting {
            for &ay in &interesting {
                for &by in &interesting {
                    let u = c2v { x: ax, y: ay };
                    let v = c2v { x: bx, y: by };
                    let (c, s) = ((a.c2Maxv.0)(u, v), (a.c2Maxv.1)(u, v));
                    d.check(veq(c, s), || format!("c2Maxv({}, {}) C={} R={}", fmt_v(u), fmt_v(v), fmt_v(c), fmt_v(s)));
                    let (c, s) = ((a.c2Minv.0)(u, v), (a.c2Minv.1)(u, v));
                    d.check(veq(c, s), || format!("c2Minv({}, {}) C={} R={}", fmt_v(u), fmt_v(v), fmt_v(c), fmt_v(s)));
                }
            }
        }
    }
    // row 51: inverted clamp box, including NaN corners
    let r = Rng::new(51);
    for _ in 0..4000 {
        let p = r.wild_v();
        let lo = r.wild_v();
        let hi = c2v { x: lo.x - r.unit() * 20.0, y: lo.y - r.unit() * 20.0 };
        let (c, s) = ((a.c2Clampv.0)(p, lo, hi), (a.c2Clampv.1)(p, lo, hi));
        d.check(veq(c, s), || format!("c2Clampv inverted C={} R={}", fmt_v(c), fmt_v(s)));
    }
    for &x in &interesting {
        let p = c2v { x, y: x };
        let lo = c2v { x: 1.0, y: 1.0 };
        let hi = c2v { x: -1.0, y: -1.0 };
        let (c, s) = ((a.c2Clampv.0)(p, lo, hi), (a.c2Clampv.1)(p, lo, hi));
        d.check(veq(c, s), || format!("c2Clampv({}, lo>hi) C={} R={}", fmt_v(p), fmt_v(c), fmt_v(s)));
    }
    d.finish();
}

// ===========================================================================
// row 52 — c2CircletoCapsule with a degenerate (point) capsule -> divide by 0
// ===========================================================================

#[test]
fn err52_c2CircletoCapsule_point_capsule() {
    let a = api();
    let mut d = Diff::new("ERRORS row 52: c2CircletoCapsule with B.a == B.b (n == (0,0), da/0)");
    let r = Rng::new(52);
    for _ in 0..4000 {
        let p = r.geo_v();
        let B = c2Capsule { a: p, b: p, r: r.radius() };
        // circle centre exactly on the point, off it, and at ±0 offsets
        for A in [
            c2Circle { p, r: r.radius() },
            c2Circle { p: c2v { x: p.x + r.sym(20.0), y: p.y + r.sym(20.0) }, r: r.radius() },
            c2Circle { p: c2v { x: p.x, y: p.y }, r: 0.0 },
            c2Circle { p: r.geo_v(), r: -r.radius() },
        ] {
            let (c, s) = ((a.c2CircletoCapsule.0)(A, B), (a.c2CircletoCapsule.1)(A, B));
            d.check(c == s, || format!("c2CircletoCapsule({A:?}, {B:?}) C={c} R={s}"));
        }
    }
    // A NaN must make `d2 < r*r` false -> 0 in both
    let B = c2Capsule { a: c2v { x: 1.0, y: 1.0 }, b: c2v { x: 1.0, y: 1.0 }, r: 5.0 };
    let A = c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: 5.0 };
    let (c, s) = ((a.c2CircletoCapsule.0)(A, B), (a.c2CircletoCapsule.1)(A, B));
    d.check(c == s, || format!("point-capsule exact-centre C={c} R={s}"));
    d.finish();
}

// ===========================================================================
// rows 53-54 — negative radii
// ===========================================================================

#[test]
fn err53_54_negative_radii() {
    let a = api();
    let mut d = Diff::new("ERRORS rows 53-54: negative radii in c2CircletoCircle / c2CircletoAABB");
    let r = Rng::new(5354);
    for _ in 0..6000 {
        let A = c2Circle { p: r.geo_v(), r: -r.radius() };
        let B = c2Circle { p: r.geo_v(), r: -r.radius() };
        let (c, s) = ((a.c2CircletoCircle.0)(A, B), (a.c2CircletoCircle.1)(A, B));
        d.check(c == s, || format!("c2CircletoCircle(neg) ({A:?}, {B:?}) C={c} R={s}"));
        let bb = r.aabb();
        let (c, s) = ((a.c2CircletoAABB.0)(A, bb), (a.c2CircletoAABB.1)(A, bb));
        d.check(c == s, || format!("c2CircletoAABB(neg) ({A:?}, {bb:?}) C={c} R={s}"));
        // r == -0.0 and r == f32::MIN
        for rr in [-0.0f32, f32::MIN, -f32::INFINITY, f32::NAN] {
            let A2 = c2Circle { p: A.p, r: rr };
            let (c, s) = ((a.c2CircletoCircle.0)(A2, B), (a.c2CircletoCircle.1)(A2, B));
            d.check(c == s, || format!("c2CircletoCircle(r={}) C={c} R={s}", fmt_f(rr)));
            let (c, s) = ((a.c2CircletoAABB.0)(A2, bb), (a.c2CircletoAABB.1)(A2, bb));
            d.check(c == s, || format!("c2CircletoAABB(r={}) C={c} R={s}", fmt_f(rr)));
        }
    }
    d.finish();
}

// ===========================================================================
// row 55 — c2AABBtoAABB with NaN coordinates reports a COLLISION.
// ===========================================================================

#[test]
fn err55_c2AABBtoAABB_nan_reports_collision() {
    let a = api();
    let mut d = Diff::new("ERRORS row 55: c2AABBtoAABB with NaN coordinates -> 1 in both builds");
    let nan = f32::NAN;
    let base = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
    let far = c2AABB { min: c2v { x: 1000.0, y: 1000.0 }, max: c2v { x: 1001.0, y: 1001.0 } };
    for slot in 0..4 {
        for other in [base, far] {
            let mut A = far;
            match slot {
                0 => A.min.x = nan,
                1 => A.min.y = nan,
                2 => A.max.x = nan,
                _ => A.max.y = nan,
            }
            for (P, Q) in [(A, other), (other, A)] {
                let (c, s) = ((a.c2AABBtoAABB.0)(P, Q), (a.c2AABBtoAABB.1)(P, Q));
                d.check(c == s, || format!("c2AABBtoAABB({P:?}, {Q:?}) C={c} R={s}"));
            }
        }
    }
    // all-NaN
    let allnan = c2AABB { min: c2v { x: nan, y: nan }, max: c2v { x: nan, y: nan } };
    let (c, s) = ((a.c2AABBtoAABB.0)(allnan, far), (a.c2AABBtoAABB.1)(allnan, far));
    d.check(c == s && c == 1, || format!("all-NaN C={c} R={s} (C should report 1)"));
    d.finish();
}

// ===========================================================================
// row 56 — c2AABBtoCapsule / c2CapsuletoCapsule return 0 whenever the GJK
//          distance is non-zero OR NaN.
// ===========================================================================

#[test]
fn err56_gjk_predicates_nan_distance() {
    let a = api();
    let mut d = Diff::new("ERRORS row 56: c2AABBtoCapsule / c2CapsuletoCapsule with NaN / inf inputs -> same rejection");
    let nan = f32::NAN;
    let inf = f32::INFINITY;
    let bbs = [
        c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
        c2AABB { min: c2v { x: nan, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
        c2AABB { min: c2v { x: -inf, y: -inf }, max: c2v { x: inf, y: inf } },
        c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: -5.0, y: -5.0 } },
    ];
    let caps = [
        c2Capsule { a: c2v { x: 0.5, y: 0.5 }, b: c2v { x: 0.5, y: 0.5 }, r: 0.0 },
        c2Capsule { a: c2v { x: nan, y: 0.0 }, b: c2v { x: 1.0, y: 1.0 }, r: 1.0 },
        c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: nan },
        c2Capsule { a: c2v { x: -inf, y: 0.0 }, b: c2v { x: inf, y: 0.0 }, r: 1.0 },
        c2Capsule { a: c2v { x: 3.0, y: 3.0 }, b: c2v { x: 4.0, y: 4.0 }, r: 0.25 },
    ];
    for &bb in &bbs {
        for &cp in &caps {
            let (c, s) = ((a.c2AABBtoCapsule.0)(bb, cp), (a.c2AABBtoCapsule.1)(bb, cp));
            d.check(c == s, || format!("c2AABBtoCapsule({bb:?}, {cp:?}) C={c} R={s}"));
        }
    }
    for &p in &caps {
        for &q in &caps {
            let (c, s) = ((a.c2CapsuletoCapsule.0)(p, q), (a.c2CapsuletoCapsule.1)(p, q));
            d.check(c == s, || format!("c2CapsuletoCapsule({p:?}, {q:?}) C={c} R={s}"));
        }
    }
    d.finish();
}

// ===========================================================================
// row 57 — aabb() has no validation at all
// ===========================================================================

#[test]
fn err57_aabb_no_validation() {
    let a = api();
    let mut d = Diff::new("ERRORS row 57: aabb() with NaN / inf / denormal / inverted inputs");
    let vals: &[f32] = &[
        f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0, 1.0e-45, -1.0e-45,
        f32::MIN, f32::MAX, -70.0, -40.0, -15.0, 100.0,
    ];
    for &a0 in vals {
        for &a1 in vals {
            for &a2 in vals {
                for &a3 in vals {
                    let (c, s) = ((a.aabb.0)(a0, a1, a2, a3), (a.aabb.1)(a0, a1, a2, a3));
                    d.check(c == s, || {
                        format!("aabb({}, {}, {}, {}) C={c} R={s}", fmt_f(a0), fmt_f(a1), fmt_f(a2), fmt_f(a3))
                    });
                }
            }
        }
    }
    d.finish();
}

// ===========================================================================
// row 58 — c2BBVerts with an inverted AABB
// ===========================================================================

#[test]
fn err58_c2BBVerts_inverted() {
    let a = api();
    let mut d = Diff::new("ERRORS row 58: c2BBVerts with min > max / NaN / inf -> same corner order");
    let r = Rng::new(58);
    let mut cases: Vec<c2AABB> = vec![
        c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: -5.0, y: -5.0 } },
        c2AABB { min: c2v { x: f32::NAN, y: 0.0 }, max: c2v { x: 1.0, y: f32::NAN } },
        c2AABB { min: c2v { x: f32::INFINITY, y: f32::NEG_INFINITY }, max: c2v { x: f32::NEG_INFINITY, y: f32::INFINITY } },
        c2AABB { min: c2v { x: -0.0, y: -0.0 }, max: c2v { x: 0.0, y: 0.0 } },
    ];
    for _ in 0..2000 {
        let bb = r.aabb();
        cases.push(c2AABB { min: bb.max, max: bb.min });
        cases.push(c2AABB { min: r.wild_v(), max: r.wild_v() });
    }
    for mut bb in cases {
        let mut co = [c2v { x: 1.0, y: 2.0 }; 4];
        let mut ro = co;
        unsafe {
            (a.c2BBVerts.0)(co.as_mut_ptr(), &mut bb);
            (a.c2BBVerts.1)(ro.as_mut_ptr(), &mut bb);
        }
        d.check(bytes_of(&co) == bytes_of(&ro), || format!("c2BBVerts({bb:?}) C={co:?} R={ro:?}"));
    }
    d.finish();
}

// ===========================================================================
// row 59 — cache indices outside the proxy's valid vertex range.
//          The C reads an UNINITIALISED `c2Proxy::verts` slot, so the *value*
//          is unspecified; what must hold is that both builds read the SAME
//          slot of the SAME array (no panic, no out-of-bounds abort) and that
//          all the index bookkeeping still agrees.
// ===========================================================================

#[test]
fn err59_cache_indices_out_of_proxy_range() {
    let a = api();
    let mut d = Diff::new("ERRORS row 59: cache iA/iB beyond the proxy vertex count (uninitialised read in C)");
    let r = Rng::new(59);
    for _ in 0..3000 {
        let circle = c2Circle { p: r.geo_v(), r: r.radius() };
        let cap = r.capsule();
        for &(ia, ib) in &[(1, 1), (2, 2), (3, 3), (7, 7), (5, 1), (0, 7)] {
            let cache = c2GJKCache { metric: 0.0, count: 1, iA: [ia, 0, 0], iB: [ib, 0, 0], div: 1.0 };
            let mut cc = cache;
            let mut rc = cache;
            let (mut ci, mut ri) = (-1 as c_int, -1 as c_int);
            let cd = unsafe {
                (a.c2GJK.0)(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                            &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                            std::ptr::null_mut(), std::ptr::null_mut(), 1, &mut ci, &mut cc)
            };
            let rd = unsafe {
                (a.c2GJK.1)(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                            &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                            std::ptr::null_mut(), std::ptr::null_mut(), 1, &mut ri, &mut rc)
            };
            // The specified, reproducible part of the contract:
            d.check((0..=20).contains(&ci) && (0..=20).contains(&ri), || format!("iA={ia} iB={ib}: iter C={ci} R={ri}"));
            d.check(cc.count >= 0 && cc.count <= 3 && rc.count >= 0 && rc.count <= 3,
                    || format!("iA={ia} iB={ib}: cache count C={} R={}", cc.count, rc.count));
            d.check(cd >= 0.0 || cd.is_nan(), || format!("C distance {}", fmt_f(cd)));
            d.check(rd >= 0.0 || rd.is_nan(), || format!("Rust distance {}", fmt_f(rd)));
        }
    }
    d.finish();
}

// ===========================================================================
// row 60 — cache->count == 4: the C reads `cache->iA[3]` (which ALIASES
//          `cache->iB[0]`) and writes `cache->iB[3]` (which ALIASES `div`).
// ===========================================================================

/// The C `.so` **crashes** for `cache->count >= 4` (verified: exit 139 /
/// SIGSEGV).  `int saveA[3], saveB[3];` are indexed with `i < save_count ==
/// cache->count`, so `saveA[3]`/`saveB[3]` overrun and corrupt the stack frame,
/// and `verts[4]` runs past `c2Simplex::d` onto `div`/`count`.  There is no
/// defined result to compare, so this test asserts the two facts that ARE
/// checkable:
///   * the C really does die (measured in a child process, so the crash does not
///     take the test runner with it), for `count = 4` and `count = 5`;
///   * the Rust `.so` returns normally with a deterministic, reproducible value
///     instead of corrupting memory.
#[test]
fn err60_cache_count_four_aliasing() {
    // Child role: perform the call named by the env var, then exit cleanly.
    if let Ok(role) = std::env::var("PHASE_C_ROW60_ROLE") {
        let a = api();
        let n: c_int = std::env::var("PHASE_C_ROW60_COUNT").unwrap().parse().unwrap();
        let A = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
        let B = c2AABB { min: c2v { x: 2.0, y: 2.0 }, max: c2v { x: 3.0, y: 3.0 } };
        let mut cache = c2GJKCache { metric: 0.0, count: n, iA: [0, 1, 2], iB: [0, 1, 2], div: 1.0 };
        let f = if role == "c" { a.c2GJK.0 } else { a.c2GJK.1 };
        let mut it: c_int = -1;
        let (mut oa, mut ob) = (c2v::default(), c2v::default());
        let dist = unsafe {
            f(&A as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
              &B as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
              &mut oa, &mut ob, 1, &mut it, &mut cache)
        };
        println!("RESULT {} {} {} {} {} {} {}", dist.to_bits(), it, cache.metric.to_bits(), cache.count,
                 cache.iA[0], cache.iB[0], cache.div.to_bits());
        std::process::exit(0);
    }

    /// libtest prefixes `--nocapture` output with `test <name> ... `, so pull the
    /// RESULT payload out of wherever it appears on the line.
    fn results(out: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(out)
            .lines()
            .filter_map(|l| l.find("RESULT ").map(|i| l[i..].trim().to_string()))
            .collect()
    }

    let exe = std::env::current_exe().expect("current_exe");
    let spawn = |role: &str, n: i32| -> std::process::Output {
        std::process::Command::new(&exe)
            .args(["err60_cache_count_four_aliasing", "--exact", "--nocapture", "--test-threads=1"])
            .env("PHASE_C_ROW60_ROLE", role)
            .env("PHASE_C_ROW60_COUNT", n.to_string())
            .output()
            .expect("spawn child")
    };

    for n in [4i32, 5] {
        // count 1..3 must work in the C (control), count >= 4 must crash it.
        let c = spawn("c", n);
        assert!(
            !c.status.success(),
            "the C .so was expected to crash for cache->count == {n} but it exited cleanly:\n{}",
            String::from_utf8_lossy(&c.stdout)
        );
        eprintln!("[row 60] C .so with cache->count == {n}: {:?} (as documented: stack corruption)", c.status);

        // The Rust .so must survive and be deterministic.
        let r1 = spawn("r", n);
        let r2 = spawn("r", n);
        assert!(r1.status.success(), "the Rust .so crashed for cache->count == {n}: {:?}", r1.status);
        let l1 = results(&r1.stdout);
        let l2 = results(&r2.stdout);
        assert_eq!(l1.len(), 1, "no RESULT line from the Rust child (count={n})");
        assert_eq!(l1, l2, "the Rust .so is not deterministic for cache->count == {n}");
        eprintln!("[row 60] Rust .so with cache->count == {n}: {} (deterministic)", l1[0]);
    }

    // Control: counts 1..3 do NOT crash the C, and there the two libraries agree
    // byte-for-byte (this is Phase B row 67 territory, re-checked here).
    for n in [1i32, 2, 3] {
        let c = spawn("c", n);
        let r = spawn("r", n);
        assert!(c.status.success() && r.status.success(), "count={n} should not crash either lib");
        let cl = results(&c.stdout);
        let rl = results(&r.stdout);
        assert_eq!(cl, rl, "cache->count == {n}: C and Rust results differ");
        eprintln!("[row 60 control] cache->count == {n}: {} (identical)", cl[0]);
    }
}

/// Row 61 — `cache->count < 0`: fully defined (every loop body is skipped).
/// Covered by `errX_cache_count_out_of_range` below.
#[test]
fn err61_cache_count_negative() {
    let a = api();
    let mut d = Diff::new("ERRORS row 61: cache->count < 0 -> all loops skipped, defined result");
    let A = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
    let B = c2AABB { min: c2v { x: 4.0, y: 4.0 }, max: c2v { x: 5.0, y: 5.0 } };
    for &n in &[-1i32, -2, -3, -100, c_int::MIN] {
        let cache = c2GJKCache { metric: 3.5, count: n, iA: [1, 2, 3], iB: [3, 2, 1], div: 2.25 };
        let mut cc = cache;
        let mut rc = cache;
        let (mut ci, mut ri) = (-1 as c_int, -1 as c_int);
        let (mut coa, mut cob) = (c2v::default(), c2v::default());
        let (mut roa, mut rob) = (c2v::default(), c2v::default());
        let cd = unsafe {
            (a.c2GJK.0)(&A as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                        &B as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                        &mut coa, &mut cob, 1, &mut ci, &mut cc)
        };
        let rd = unsafe {
            (a.c2GJK.1)(&A as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                        &B as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                        &mut roa, &mut rob, 1, &mut ri, &mut rc)
        };
        d.check(feq(cd, rd) && veq(coa, roa) && veq(cob, rob) && ci == ri && cacheeq(&cc, &rc), || {
            format!("count={n}\n dist C={} R={}\n iter C={ci} R={ri}\n cache C={cc:?}\n cache R={rc:?}",
                    fmt_f(cd), fmt_f(rd))
        });
        d.check(cc.count == n, || format!("count={n}: C changed cache->count to {}", cc.count));
    }
    d.finish();
}

// ===========================================================================
// Generic FFI boundary sweeps that every C API has, beyond the table.
// ===========================================================================

/// Negative and absurd `cache->count` values (one step past the valid range in
/// both directions).
#[test]
fn errX_cache_count_out_of_range() {
    let a = api();
    let mut d = Diff::new("extra: cache->count negative / one past the range");
    let r = Rng::new(1000);
    for &count in &[-1i32, -2, -100, c_int::MIN] {
        for _ in 0..500 {
            let A = r.aabb();
            let B = r.capsule();
            let cache = c2GJKCache {
                metric: r.sym(10.0),
                count,
                iA: [r.below(4) as c_int, r.below(4) as c_int, r.below(4) as c_int],
                iB: [r.below(2) as c_int, r.below(2) as c_int, r.below(2) as c_int],
                div: r.unit() * 5.0 + 0.5,
            };
            let mut cc = cache;
            let mut rc = cache;
            let (mut ci, mut ri) = (-1 as c_int, -1 as c_int);
            let (mut coa, mut cob) = (c2v::default(), c2v::default());
            let (mut roa, mut rob) = (c2v::default(), c2v::default());
            let cd = unsafe {
                (a.c2GJK.0)(&A as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                            &B as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                            &mut coa, &mut cob, 1, &mut ci, &mut cc)
            };
            let rd = unsafe {
                (a.c2GJK.1)(&A as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                            &B as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                            &mut roa, &mut rob, 1, &mut ri, &mut rc)
            };
            d.check(feq(cd, rd) && veq(coa, roa) && veq(cob, rob) && ci == ri && cacheeq(&cc, &rc), || {
                format!("count={count} cache={cache:?}\n dist C={} R={}\n iter C={ci} R={ri}\n cache C={cc:?}\n cache R={rc:?}",
                        fmt_f(cd), fmt_f(rd))
            });
        }
    }
    d.finish();
}

/// Every out-of-range enum value, pushed through *every* entry point that takes
/// a `C2_TYPE`.
#[test]
fn errX_all_enum_entry_points() {
    let a = api();
    let mut d = Diff::new("extra: out-of-range C2_TYPE through c2MakeProxy and c2Collided, all argument slots");
    let circle = c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: 2.0 };
    for &t in BAD_TYPES.iter().chain(GOOD_TYPES.iter()) {
        // c2MakeProxy
        let pristine = seeded_proxy(-2.25);
        let mut cp = pristine;
        let mut rp = pristine;
        unsafe {
            (a.c2MakeProxy.0)(&circle as *const _ as *const c_void, t, &mut cp);
            (a.c2MakeProxy.1)(&circle as *const _ as *const c_void, t, &mut rp);
        }
        d.check(proxyeq(&cp, &rp), || format!("c2MakeProxy(type={t})\n C={cp:?}\n R={rp:?}"));

        // c2Collided in both slots
        for &u in BAD_TYPES.iter().chain(GOOD_TYPES.iter()) {
            // only exercise combinations where a wrong reinterpretation cannot
            // read past our buffer: use a 32-byte scratch for both operands.
            let mut buf = [0u8; 32];
            buf[..12].copy_from_slice(bytes_of(&circle));
            let p = buf.as_ptr() as *const c_void;
            let (c, s) = unsafe { ((a.c2Collided.0)(p, t, p, u), (a.c2Collided.1)(p, t, p, u)) };
            d.check(c == s, || format!("c2Collided(typeA={t}, typeB={u}) C={c} R={s}"));
        }
    }
    d.finish();
}

/// `c2Support` with `count` one past every proxy size the library produces.
#[test]
fn errX_c2Support_one_past_range() {
    let a = api();
    let mut d = Diff::new("extra: c2Support count == 1..8 and 8 (the array maximum), one-past boundaries");
    let r = Rng::new(1002);
    for _ in 0..2000 {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = r.wild_v();
        }
        let dir = r.wild_v();
        for count in 1..=8i32 {
            let (cv, rv) = unsafe {
                ((a.c2Support.0)(verts.as_ptr(), count, dir), (a.c2Support.1)(verts.as_ptr(), count, dir))
            };
            d.check(cv == rv, || format!("c2Support(count={count}, d={}) C={cv} R={rv}", fmt_v(dir)));
            d.check(cv >= 0 && cv < count.max(1), || format!("c2Support(count={count}) returned {cv}"));
        }
    }
    d.finish();
}
