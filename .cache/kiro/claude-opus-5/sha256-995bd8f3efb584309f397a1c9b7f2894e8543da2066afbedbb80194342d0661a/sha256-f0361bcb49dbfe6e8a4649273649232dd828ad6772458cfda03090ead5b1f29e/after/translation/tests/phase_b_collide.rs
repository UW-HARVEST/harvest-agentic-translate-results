//! Phase B — CONFIGS.md rows 56..71: `c2Collide`, `ptr_from_parts` and the
//! public `omni_manifold` entry point declared in `include/lib.h`.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// The three types `c2Collide` / `ptr_from_parts` actually handle.
const HANDLED: [c_int; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];

fn type_name(t: c_int) -> &'static str {
    match t {
        C2_TYPE_CAPSULE => "CAPSULE",
        C2_TYPE_CIRCLE => "CIRCLE",
        C2_TYPE_AABB => "AABB",
        C2_TYPE_POLY => "POLY",
        _ => "OUT_OF_RANGE",
    }
}

/// Five floats for `ptr_from_parts`, shaped for the given type.
fn parts(rng: &mut Rng, t: c_int, spread: f32, mode: u32) -> [f32; 5] {
    match mode {
        // grid-quantized: exact ties, tangency and touching are common
        1 => [
            rng.grid(0.5, 10),
            rng.grid(0.5, 10),
            rng.grid(0.5, 10),
            rng.grid(0.5, 10),
            rng.grid(0.5, 6),
        ],
        // special float classes (row 70), NaN excluded — see `nan_payloads`
        2 => [
            rng.wild(),
            rng.wild(),
            rng.wild(),
            rng.wild(),
            rng.wild(),
        ],
        _ => match t {
            C2_TYPE_CIRCLE => [rng.f(spread), rng.f(spread), rng.fpos(4.0), 0.0, 0.0],
            C2_TYPE_AABB => {
                let c = rng.vec(spread);
                let e = v(rng.fpos(4.0), rng.fpos(4.0));
                [c.x - e.x, c.y - e.y, c.x + e.x, c.y + e.y, 0.0]
            }
            _ => {
                let c = rng.vec(spread);
                let d = rng.vec(4.0);
                [c.x - d.x, c.y - d.y, c.x + d.x, c.y + d.y, rng.fpos(3.0)]
            }
        },
    }
}

// ------------------------------------------------------------------ rows 65..67
#[test]
fn rows65_67_ptr_from_parts() {
    let l = libs();
    let (cf, rf) = l.pair::<FnPtrFromParts>("ptr_from_parts");
    let mut rng = Rng::new(0x1BAD_C0DE_5EED_0001);

    for i in 0..20_000usize {
        let t = HANDLED[i % 3];
        let p = parts(&mut rng, t, 20.0, (i % 3) as u32);
        let (cp, rp) = unsafe {
            (
                cf(t, p[0], p[1], p[2], p[3], p[4]),
                rf(t, p[0], p[1], p[2], p[3], p[4]),
            )
        };
        assert!(!cp.is_null(), "C ptr_from_parts returned NULL for {}", type_name(t));
        assert!(!rp.is_null(), "Rust ptr_from_parts returned NULL for {}", type_name(t));
        unsafe {
            match t {
                // row 65: (a, b) -> p, c -> r
                C2_TYPE_CIRCLE => {
                    let a = *(cp as *const c2Circle);
                    let b = *(rp as *const c2Circle);
                    assert!(
                        veq(a.p, b.p) && feq(a.r, b.r),
                        "ptr_from_parts CIRCLE {i}: C {a:?} vs R {b:?}"
                    );
                }
                // row 66: (a, b) -> min, (c, d) -> max
                C2_TYPE_AABB => {
                    let a = *(cp as *const c2AABB);
                    let b = *(rp as *const c2AABB);
                    assert!(
                        veq(a.min, b.min) && veq(a.max, b.max),
                        "ptr_from_parts AABB {i}: C {a:?} vs R {b:?}"
                    );
                }
                // row 67: (a, b) -> a, (c, d) -> b, e -> r
                _ => {
                    let a = *(cp as *const c2Capsule);
                    let b = *(rp as *const c2Capsule);
                    assert!(
                        veq(a.a, b.a) && veq(a.b, b.b) && feq(a.r, b.r),
                        "ptr_from_parts CAPSULE {i}: C {a:?} vs R {b:?}"
                    );
                }
            }
        }
        // Both libraries leak these, exactly as the C does; free them here so
        // the test process does not grow unboundedly.
        unsafe {
            libc_free(cp);
            libc_free(rp);
        }
    }
}

unsafe extern "C" {
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

// ------------------------------------------------------------------ rows 56..64
#[test]
fn rows56_64_collide_all_pairs() {
    let l = libs();
    let (c_collide, r_collide) = l.pair::<FnCollide>("c2Collide");
    let (c_pfp, _) = l.pair::<FnPtrFromParts>("ptr_from_parts");
    let mut rng = Rng::new(0x2BAD_C0DE_5EED_0002);

    let mut per_pair_hits = [[0usize; 3]; 3];
    for (ia, &ta) in HANDLED.iter().enumerate() {
        for (ib, &tb) in HANDLED.iter().enumerate() {
            for i in 0..4000usize {
                let mode = (i % 3) as u32;
                let spread = if i % 4 == 0 { 40.0 } else { 5.0 };
                let pa = parts(&mut rng, ta, spread, mode);
                let pb = parts(&mut rng, tb, spread, mode);

                // Build the shapes once (with the C allocator) and hand the
                // *same* pointers to both libraries, so only the collide logic
                // differs.
                let (a_ptr, b_ptr) = unsafe {
                    (
                        c_pfp(ta, pa[0], pa[1], pa[2], pa[3], pa[4]),
                        c_pfp(tb, pb[0], pb[1], pb[2], pb[3], pb[4]),
                    )
                };

                let mut cm = seeded_manifold(-4242.0);
                let mut rm = cm;
                unsafe {
                    scrub_stack();
                    c_collide(a_ptr, ta, b_ptr, tb, &mut cm);
                    scrub_stack();
                    r_collide(a_ptr, ta, b_ptr, tb, &mut rm);
                }
                if cm.count > 0 {
                    per_pair_hits[ia][ib] += 1;
                }
                assert!(
                    meq(&cm, &rm),
                    "c2Collide {}x{} i={i} mode={mode}\n  A={pa:?}\n  B={pb:?}\n  C {}\n  R {}",
                    type_name(ta),
                    type_name(tb),
                    ms(&cm),
                    ms(&rm)
                );
                unsafe {
                    libc_free(a_ptr);
                    libc_free(b_ptr);
                }
            }
        }
    }
    for (ia, row) in per_pair_hits.iter().enumerate() {
        for (ib, &n) in row.iter().enumerate() {
            assert!(
                n > 50,
                "pair {}x{} only produced {n} contacts",
                type_name(HANDLED[ia]),
                type_name(HANDLED[ib])
            );
        }
    }
}

// ------------------------------------------------------------------ rows 68..70
#[test]
fn rows68_70_omni_manifold_all_pairs() {
    let l = libs();
    let (cf, rf) = l.pair::<FnOmni>("omni_manifold");
    let mut rng = Rng::new(0x3BAD_C0DE_5EED_0003);

    let mut hits = 0usize;
    let mut misses = 0usize;
    for (ia, &ta) in HANDLED.iter().enumerate() {
        let _ = ia;
        for &tb in HANDLED.iter() {
            for i in 0..5000usize {
                // row 68 random, row 69 grid, row 70 special float classes
                let mode = (i % 3) as u32;
                let spread = if i % 5 == 0 { 40.0 } else { 5.0 };
                let a = parts(&mut rng, ta, spread, mode);
                let b = parts(&mut rng, tb, spread, mode);

                let mut cm = seeded_manifold(-4242.0);
                let mut rm = cm;
                unsafe {
                    scrub_stack();
                    cf(&mut cm, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]);
                    scrub_stack();
                    rf(&mut rm, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]);
                }
                if cm.count > 0 { hits += 1 } else { misses += 1 }
                assert!(
                    meq(&cm, &rm),
                    "omni_manifold {}x{} i={i} mode={mode}\n  A={a:?}\n  B={b:?}\n  C {}\n  R {}",
                    type_name(ta),
                    type_name(tb),
                    ms(&cm),
                    ms(&rm)
                );
            }
        }
    }
    assert!(hits > 1000 && misses > 1000, "coverage hits={hits} misses={misses}");
}

// ----------------------------------------------------------------------- row 71
/// All 16 ordered pairs over `{CAPSULE, CIRCLE, AABB, POLY}` plus out-of-range
/// enum values crossing the FFI boundary.
#[test]
fn row71_omni_manifold_all_type_values() {
    let l = libs();
    let (cf, rf) = l.pair::<FnOmni>("omni_manifold");
    let mut rng = Rng::new(0x4BAD_C0DE_5EED_0004);

    let types: [c_int; 9] = [0, 1, 2, 3, 4, 7, -1, i32::MIN, i32::MAX];
    for &ta in &types {
        for &tb in &types {
            for i in 0..400usize {
                let a = parts(&mut rng, ta, 6.0, (i % 2) as u32);
                let b = parts(&mut rng, tb, 6.0, (i % 2) as u32);
                let mut cm = seeded_manifold(-4242.0);
                let mut rm = cm;
                unsafe {
                    scrub_stack();
                    cf(&mut cm, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]);
                    scrub_stack();
                    rf(&mut rm, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]);
                }
                assert!(
                    meq(&cm, &rm),
                    "omni_manifold type_a={ta} type_b={tb} i={i}\n  C {}\n  R {}",
                    ms(&cm),
                    ms(&rm)
                );
            }
        }
    }
}
