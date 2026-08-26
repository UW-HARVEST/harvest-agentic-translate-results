//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input / degenerate condition the row
//! describes, calls **both** `.so`s, and asserts they produce the same rejection: the
//! same sentinel return value (`0` / `1` from the clip predicates, the distance from
//! `c2GJK`) or the same `c2Manifold` bytes (`count == 0` plus whichever fields the C
//! code leaves untouched).
//!
//! Row numbers in the test names refer to `ERRORS.md`.
#![allow(non_snake_case)]
#![allow(clippy::unnecessary_cast, clippy::needless_range_loop, clippy::let_and_return)]
#![allow(clippy::field_reassign_with_default)]

mod common;
use common::*;
use std::ffi::c_void;

// ===========================================================================
// Rows 1-2: ptr_from_parts with POLY / out-of-range type
// ===========================================================================

/// The C `switch` has no `C2_TYPE_POLY` case and no `default`, so control falls off
/// the end of a non-`void` function and the return value is an indeterminate
/// register. That is not a function of the inputs, so the only thing that can be
/// asserted is that neither library crashes and that the pointer is never used --
/// which `c2Collide` guarantees, because it has no POLY case either (rows 5-9).
#[test]
fn row01_02_ptr_from_parts_unhandled_type() {
    let l = libs();
    let (cf, rf) = l.get::<FnPtrFromParts>("ptr_from_parts");
    let mut tys: Vec<C2_TYPE> = vec![C2_TYPE_POLY];
    tys.extend_from_slice(&BAD_TYPES);
    for ty in tys {
        let cp = unsafe { cf(ty, 1.0, 2.0, 3.0, 4.0, 5.0) };
        let rp = unsafe { rf(ty, 1.0, 2.0, 3.0, 4.0, 5.0) };
        // Documented divergence: C returns an indeterminate register value (control
        // falls off the end of a non-`void` function), Rust returns NULL. Never
        // dereferenced by any caller -- rows 5-9 verify that byte-for-byte.
        println!("ptr_from_parts(ty={ty}) -> C {cp:?}, Rust {rp:?}");
        assert!(rp.is_null(), "Rust must return NULL for the unhandled type {ty}");
    }
    // Rust must be deterministic and must not crash.
    let a = unsafe { rf(C2_TYPE_POLY, 1.0, 2.0, 3.0, 4.0, 5.0) };
    let b = unsafe { rf(C2_TYPE_POLY, 1.0, 2.0, 3.0, 4.0, 5.0) };
    assert_eq!(a, b, "Rust ptr_from_parts(POLY) must be deterministic");
}

// ===========================================================================
// Rows 3-4: c2MakeProxy writes nothing for POLY / out-of-range
// ===========================================================================

#[test]
fn row03_04_make_proxy_unhandled_type_writes_nothing() {
    let l = libs();
    let (cf, rf) = l.get::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(304);
    let mut tys: Vec<C2_TYPE> = vec![C2_TYPE_POLY];
    tys.extend_from_slice(&BAD_TYPES);
    for ty in tys {
        for seed in [0u8, 7, 91, 255] {
            for _ in 0..64 {
                // any shape bytes -- they must be ignored entirely
                let cap = c2Capsule { a: rng.vec_norm(9.0), b: rng.vec_norm(9.0), r: rng.f_norm(4.0) };
                let mut cp = poison_proxy(seed);
                let mut rp = poison_proxy(seed);
                let before = poison_proxy(seed);
                unsafe {
                    cf(&cap as *const c2Capsule as *const c_void, ty, &mut cp);
                    rf(&cap as *const c2Capsule as *const c_void, ty, &mut rp);
                }
                let ctx = format!("ty={ty} seed={seed}");
                eq("c2MakeProxy unhandled type", &ctx, &cp, &rp);
                // and it really wrote nothing at all
                eq("c2MakeProxy must not touch the proxy", &ctx, &before, &cp);
            }
        }
    }
}

// ===========================================================================
// Rows 5-9: c2Collide / omni_manifold with POLY or out-of-range types
// ===========================================================================

#[repr(C, align(8))]
#[derive(Clone, Copy)]
struct Blob([u8; 24]);

fn blob(rng: &mut Rng) -> Blob {
    let mut b = Blob([0; 24]);
    for x in b.0.iter_mut() {
        *x = (rng.next_u32() & 0xff) as u8;
    }
    b
}

#[test]
fn row05_08_collide_unhandled_types() {
    let l = libs();
    let (cf, rf) = l.get::<FnCollide>("c2Collide");
    let mut rng = Rng::new(58);
    let mut all: Vec<C2_TYPE> = ALL_TYPES.to_vec();
    all.extend_from_slice(&BAD_TYPES);
    for &ta in all.iter() {
        for &tb in all.iter() {
            let handled = VALID_TYPES.contains(&ta) && VALID_TYPES.contains(&tb);
            if handled {
                continue; // covered by Phase B rows 73-74
            }
            for seed in [0u8, 37, 211] {
                for _ in 0..24 {
                    let (a, b) = (blob(&mut rng), blob(&mut rng));
                    let mut cm = poison_manifold(seed);
                    let mut rm = poison_manifold(seed);
                    let expect = {
                        // C: `m->count = 0;` and nothing else
                        let mut e = poison_manifold(seed);
                        e.count = 0;
                        e
                    };
                    zero_stack();
                    unsafe {
                        cf(&a as *const Blob as *const c_void, ta, &b as *const Blob as *const c_void, tb, &mut cm);
                    }
                    zero_stack();
                    unsafe {
                        rf(&a as *const Blob as *const c_void, ta, &b as *const Blob as *const c_void, tb, &mut rm);
                    }
                    let ctx = format!("ta={ta} tb={tb} seed={seed}");
                    eq("c2Collide unhandled pair", &ctx, &cm, &rm);
                    eq("c2Collide unhandled pair leaves everything but count", &ctx, &expect, &cm);
                }
            }
        }
    }
}

#[test]
fn row09_omni_manifold_unhandled_types() {
    let l = libs();
    let (cf, rf) = l.get::<FnOmni>("omni_manifold");
    let mut rng = Rng::new(9);
    // Warm up so lazy PLT resolution does not dirty the stack during the measured
    // calls. `zero_stack()` must be the last statement before each FFI call: the
    // (AABB, CAPSULE) pair reaches `c2GJK` with `C2_TYPE_POLY`, and without a zeroed
    // stack the C library's garbage `pB.count` can make `c2Support` read unmapped
    // memory and SIGSEGV.
    warmup(|| {
        let mut m = c2Manifold::default();
        zero_stack();
        unsafe {
            cf(&mut m, C2_TYPE_AABB, -1.0, -1.0, 1.0, 1.0, 0.0, C2_TYPE_CAPSULE, -2.0, 0.0, 2.0, 0.0, 0.5);
        }
        zero_stack();
        unsafe {
            rf(&mut m, C2_TYPE_AABB, -1.0, -1.0, 1.0, 1.0, 0.0, C2_TYPE_CAPSULE, -2.0, 0.0, 2.0, 0.0, 0.5);
        }
    });
    let mut all: Vec<C2_TYPE> = ALL_TYPES.to_vec();
    all.extend_from_slice(&BAD_TYPES);
    for &ta in all.iter() {
        for &tb in all.iter() {
            if VALID_TYPES.contains(&ta) && VALID_TYPES.contains(&tb) {
                continue;
            }
            for seed in [0u8, 91, 255] {
                for _ in 0..16 {
                    let q: [f32; 5] = [rng.f_norm(5.0), rng.f_norm(5.0), rng.f_norm(5.0), rng.f_norm(5.0), rng.f_pos(2.0)];
                    let p: [f32; 5] = [rng.f_norm(5.0), rng.f_norm(5.0), rng.f_norm(5.0), rng.f_norm(5.0), rng.f_pos(2.0)];
                    let mut cm = poison_manifold(seed);
                    let mut rm = poison_manifold(seed);
                    let mut expect = poison_manifold(seed);
                    expect.count = 0;
                    zero_stack();
                    unsafe { cf(&mut cm, ta, q[0], q[1], q[2], q[3], q[4], tb, p[0], p[1], p[2], p[3], p[4]) };
                    zero_stack();
                    unsafe { rf(&mut rm, ta, q[0], q[1], q[2], q[3], q[4], tb, p[0], p[1], p[2], p[3], p[4]) };
                    let ctx = format!("ta={ta} tb={tb} seed={seed}");
                    eq("omni_manifold unhandled pair", &ctx, &cm, &rm);
                    eq("omni_manifold unhandled pair sets only count", &ctx, &expect, &cm);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 10-15: c2GJK null-pointer arguments
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn gjk_raw(
    f: &libloading::Symbol<'_, FnGJK>,
    a: *const c_void,
    ta: C2_TYPE,
    ax: *const c2x,
    b: *const c_void,
    tb: C2_TYPE,
    bx: *const c2x,
    outa: *mut c2v,
    outb: *mut c2v,
    ur: i32,
    it: *mut i32,
    cache: *mut c2GJKCache,
) -> f32 {
    zero_stack();
    unsafe { f(a, ta, ax, b, tb, bx, outa, outb, ur, it, cache) }
}

#[test]
fn row10_11_null_transforms_use_identity() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(1011);
    let ident = x_identity();
    for _ in 0..3_000 {
        let ca = c2Circle { p: rng.vec_norm(20.0), r: rng.f_pos(3.0) };
        let cb = c2Capsule { a: rng.vec_norm(20.0), b: rng.vec_norm(20.0), r: rng.f_pos(3.0) };
        let (pa, pb) = (
            &ca as *const c2Circle as *const c_void,
            &cb as *const c2Capsule as *const c_void,
        );
        // NULL transform must behave exactly like an explicit identity
        for (label, ax, bx) in [
            ("both NULL", std::ptr::null(), std::ptr::null()),
            ("ax NULL", std::ptr::null(), &ident as *const c2x),
            ("bx NULL", &ident as *const c2x, std::ptr::null()),
            ("neither NULL", &ident as *const c2x, &ident as *const c2x),
        ] {
            let (mut c_a, mut c_b, mut c_i) = (poison_v(1), poison_v(2), -1i32);
            let (mut r_a, mut r_b, mut r_i) = (poison_v(1), poison_v(2), -1i32);
            let cd = gjk_raw(&cf, pa, C2_TYPE_CIRCLE, ax, pb, C2_TYPE_CAPSULE, bx, &mut c_a, &mut c_b, 1, &mut c_i, std::ptr::null_mut());
            let rd = gjk_raw(&rf, pa, C2_TYPE_CIRCLE, ax, pb, C2_TYPE_CAPSULE, bx, &mut r_a, &mut r_b, 1, &mut r_i, std::ptr::null_mut());
            let ctx = format!("{label} A={ca:?} B={cb:?}");
            eq_f32("c2GJK dist", &ctx, cd, rd);
            eq("c2GJK outA", &ctx, &c_a, &r_a);
            eq("c2GJK outB", &ctx, &c_b, &r_b);
            eq_i32("c2GJK iters", &ctx, c_i, r_i);
        }
    }
}

#[test]
fn row12_13_14_15_null_out_params_and_cache() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(1215);
    for _ in 0..2_000 {
        let ca = c2AABB { min: rng.vec_norm(10.0), max: rng.vec_norm(10.0) };
        let cb = c2Capsule { a: rng.vec_norm(10.0), b: rng.vec_norm(10.0), r: rng.f_pos(3.0) };
        let (pa, pb) = (
            &ca as *const c2AABB as *const c_void,
            &cb as *const c2Capsule as *const c_void,
        );
        for mask in 0..16u32 {
            let (mut c_a, mut c_b, mut c_i) = (poison_v(3), poison_v(4), -7i32);
            let (mut r_a, mut r_b, mut r_i) = (poison_v(3), poison_v(4), -7i32);
            let mut c_cache = c2GJKCache { metric: 1.0, count: 0, iA: [1, 2, 3], iB: [3, 2, 1], div: 9.0 };
            let mut r_cache = c_cache;
            let cd = gjk_raw(
                &cf, pa, C2_TYPE_AABB, std::ptr::null(), pb, C2_TYPE_CAPSULE, std::ptr::null(),
                if mask & 1 != 0 { &mut c_a } else { std::ptr::null_mut() },
                if mask & 2 != 0 { &mut c_b } else { std::ptr::null_mut() },
                1,
                if mask & 4 != 0 { &mut c_i } else { std::ptr::null_mut() },
                if mask & 8 != 0 { &mut c_cache } else { std::ptr::null_mut() },
            );
            let rd = gjk_raw(
                &rf, pa, C2_TYPE_AABB, std::ptr::null(), pb, C2_TYPE_CAPSULE, std::ptr::null(),
                if mask & 1 != 0 { &mut r_a } else { std::ptr::null_mut() },
                if mask & 2 != 0 { &mut r_b } else { std::ptr::null_mut() },
                1,
                if mask & 4 != 0 { &mut r_i } else { std::ptr::null_mut() },
                if mask & 8 != 0 { &mut r_cache } else { std::ptr::null_mut() },
            );
            let ctx = format!("mask={mask} A={ca:?} B={cb:?}");
            eq_f32("c2GJK dist", &ctx, cd, rd);
            eq("c2GJK outA", &ctx, &c_a, &r_a);
            eq("c2GJK outB", &ctx, &c_b, &r_b);
            eq_i32("c2GJK iters", &ctx, c_i, r_i);
            eq("c2GJK cache", &ctx, &c_cache, &r_cache);
            // NULL out-params really are left alone
            if mask & 1 == 0 {
                eq("outA==NULL leaves the local alone", &ctx, &poison_v(3), &c_a);
            }
            if mask & 4 == 0 {
                eq_i32("iterations==NULL leaves the local alone", &ctx, -7, c_i);
            }
        }
    }
}

// ===========================================================================
// Rows 16-18, 28: cache count 0 / negative / poisoned div
// ===========================================================================

#[test]
fn row16_17_18_28_cache_counts() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(161828);
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for &count in [0i32, -1, -3, i32::MIN, 1, 2, 3].iter() {
                for i in 0..300 {
                    let sa = c2Circle { p: rng.vec_norm(10.0), r: rng.f_pos(2.0) };
                    let sb = c2Capsule { a: rng.vec_norm(10.0), b: rng.vec_norm(10.0), r: rng.f_pos(2.0) };
                    let bb = c2AABB { min: rng.vec_norm(10.0), max: rng.vec_norm(10.0) };
                    let (pa, na) = match ta {
                        C2_TYPE_CIRCLE => (&sa as *const _ as *const c_void, 1u32),
                        C2_TYPE_AABB => (&bb as *const _ as *const c_void, 4),
                        _ => (&sb as *const _ as *const c_void, 2),
                    };
                    let (pb, nb) = match tb {
                        C2_TYPE_CIRCLE => (&sa as *const _ as *const c_void, 1u32),
                        C2_TYPE_AABB => (&bb as *const _ as *const c_void, 4),
                        _ => (&sb as *const _ as *const c_void, 2),
                    };
                    let cache = c2GJKCache {
                        metric: match i % 4 { 0 => 0.0, 1 => -2.0e8, 2 => f32::NAN, _ => rng.f_norm(50.0) },
                        count,
                        // in range for the proxies, so no uninitialised proxy slot is read
                        iA: [rng.below(na) as i32, rng.below(na) as i32, rng.below(na) as i32],
                        iB: [rng.below(nb) as i32, rng.below(nb) as i32, rng.below(nb) as i32],
                        // row 28: div == 0 makes `den = 1.0f/0.0f == +inf`
                        div: match i % 5 { 0 => 0.0, 1 => -0.0, 2 => f32::NAN, 3 => 1.0, _ => rng.f_norm(10.0) },
                    };
                    let (mut c_a, mut c_b, mut c_i) = (poison_v(5), poison_v(6), 0i32);
                    let (mut r_a, mut r_b, mut r_i) = (poison_v(5), poison_v(6), 0i32);
                    let mut cc = cache;
                    let mut rc = cache;
                    let cd = gjk_raw(&cf, pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut c_a, &mut c_b, 0, &mut c_i, &mut cc);
                    let rd = gjk_raw(&rf, pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut r_a, &mut r_b, 0, &mut r_i, &mut rc);
                    let ctx = format!("ta={ta} tb={tb} count={count} i={i} cache={cache:?}");
                    eq_f32("c2GJK dist", &ctx, cd, rd);
                    eq("c2GJK outA", &ctx, &c_a, &r_a);
                    eq("c2GJK outB", &ctx, &c_b, &r_b);
                    eq_i32("c2GJK iters", &ctx, c_i, r_i);
                    eq("c2GJK cache", &ctx, &cc, &rc);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 19-20: c2GJK with POLY / out-of-range type (uninitialised proxy)
// ===========================================================================

/// With [`zero_stack`] applied to both sides the C library's uninitialised `c2Proxy`
/// reads back all-zero, which is exactly the model `src/gjk.rs` implements, so the
/// two agree byte-for-byte. Without it the C side returns a value derived from a
/// stack address (see `tests/probe_uninit.rs`).
#[test]
fn row19_20_gjk_unhandled_type_uses_zero_proxy() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(1920);
    let mut tys: Vec<C2_TYPE> = vec![C2_TYPE_POLY];
    tys.extend_from_slice(&BAD_TYPES);
    // warm up
    let cap0 = c2Capsule { a: v(0.0, 0.0), b: v(1.0, 0.0), r: 1.0 };
    let poly0 = c2Poly::default();
    warmup(|| {
        let (mut a, mut b) = (c2v::default(), c2v::default());
        gjk_raw(&cf, &cap0 as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &poly0 as *const _ as *const c_void, C2_TYPE_POLY, std::ptr::null(),
                &mut a, &mut b, 0, std::ptr::null_mut(), std::ptr::null_mut());
        gjk_raw(&rf, &cap0 as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &poly0 as *const _ as *const c_void, C2_TYPE_POLY, std::ptr::null(),
                &mut a, &mut b, 0, std::ptr::null_mut(), std::ptr::null_mut());
    });
    for &bad in tys.iter() {
        for side in 0..3u32 {
            for i in 0..400 {
                let cap = c2Capsule { a: rng.vec_norm(15.0), b: rng.vec_norm(15.0), r: rng.f_pos(3.0) };
                let ctr = rng.vec_norm(5.0);
                let mut poly = convex_poly(&mut rng, 3 + (i % 6) as i32, 3.0, ctr);
                fill_norms(&mut poly);
                let (pa, ta, pb, tb) = match side {
                    0 => (&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &poly as *const _ as *const c_void, bad),
                    1 => (&poly as *const _ as *const c_void, bad, &cap as *const _ as *const c_void, C2_TYPE_CAPSULE),
                    _ => (&poly as *const _ as *const c_void, bad, &poly as *const _ as *const c_void, bad),
                };
                for &ur in [0i32, 1].iter() {
                    let (mut c_a, mut c_b, mut c_i) = (poison_v(7), poison_v(8), 0i32);
                    let (mut r_a, mut r_b, mut r_i) = (poison_v(7), poison_v(8), 0i32);
                    let cd = gjk_raw(&cf, pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut c_a, &mut c_b, ur, &mut c_i, std::ptr::null_mut());
                    let rd = gjk_raw(&rf, pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut r_a, &mut r_b, ur, &mut r_i, std::ptr::null_mut());
                    let ctx = format!("bad={bad} side={side} ur={ur} i={i} cap={cap:?}");
                    eq_f32("c2GJK dist", &ctx, cd, rd);
                    eq("c2GJK outA", &ctx, &c_a, &r_a);
                    eq("c2GJK outB", &ctx, &c_b, &r_b);
                    eq_i32("c2GJK iters", &ctx, c_i, r_i);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 21-27: the c2GJK termination / radius paths
// ===========================================================================

#[test]
fn row21_22_23_24_25_26_27_gjk_termination_paths() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(2127);
    let mut hit_zero = 0u32;      // row 23: `hit` -> dist exactly 0 and a == b
    let mut radius_shrink = 0u32; // row 21/22: use_radius took the shrink path
    let mut midpoint = 0u32;      // row 21: use_radius took the midpoint fallback
    let mut max_iter = 0i32;      // row 27
    for i in 0..40_000 {
        // A mixture that reliably covers separated / touching / overlapping.
        let (sa, sb) = match i % 4 {
            0 => (
                c2Circle { p: v(0.0, 0.0), r: rng.f_pos(2.0) },
                c2Circle { p: v(rng.f_pos(6.0), 0.0), r: rng.f_pos(2.0) },
            ),
            1 => (
                c2Circle { p: rng.vec_lattice(3), r: rng.below(3) as f32 },
                c2Circle { p: rng.vec_lattice(3), r: rng.below(3) as f32 },
            ),
            2 => {
                let p = rng.vec_norm(4.0);
                (c2Circle { p, r: rng.f_pos(3.0) }, c2Circle { p, r: rng.f_pos(3.0) })
            }
            _ => (
                c2Circle { p: rng.vec_special(), r: rng.f_special() },
                c2Circle { p: rng.vec_special(), r: rng.f_special() },
            ),
        };
        let (pa, pb) = (&sa as *const _ as *const c_void, &sb as *const _ as *const c_void);
        for &ur in [0i32, 1].iter() {
            let (mut c_a, mut c_b, mut c_i) = (poison_v(9), poison_v(10), 0i32);
            let (mut r_a, mut r_b, mut r_i) = (poison_v(9), poison_v(10), 0i32);
            let cd = gjk_raw(&cf, pa, C2_TYPE_CIRCLE, std::ptr::null(), pb, C2_TYPE_CIRCLE, std::ptr::null(), &mut c_a, &mut c_b, ur, &mut c_i, std::ptr::null_mut());
            let rd = gjk_raw(&rf, pa, C2_TYPE_CIRCLE, std::ptr::null(), pb, C2_TYPE_CIRCLE, std::ptr::null(), &mut r_a, &mut r_b, ur, &mut r_i, std::ptr::null_mut());
            let ctx = format!("i={i} ur={ur} A={sa:?} B={sb:?}");
            eq_f32("c2GJK dist", &ctx, cd, rd);
            eq("c2GJK outA", &ctx, &c_a, &r_a);
            eq("c2GJK outB", &ctx, &c_b, &r_b);
            eq_i32("c2GJK iters", &ctx, c_i, r_i);
            max_iter = max_iter.max(c_i);
            if cd == 0.0 && c_a == c_b {
                if ur == 0 { hit_zero += 1 } else { midpoint += 1 }
            } else if ur == 1 && cd > 0.0 {
                radius_shrink += 1;
            }
        }
    }
    println!("rows21-27: hit_zero={hit_zero} radius_shrink={radius_shrink} midpoint={midpoint} max_iter={max_iter}");
    assert!(hit_zero > 0, "row 23 (`hit`) never fired");
    assert!(radius_shrink > 0, "rows 21/22 (radius shrink) never fired");
    assert!(midpoint > 0, "row 21 (midpoint fallback) never fired");
}

/// Rows 24/25/26/27 specifically: the four ways the iteration loop can stop. The
/// observable is `*iterations`, which is compared for every input.
#[test]
fn row24_25_26_27_iteration_counts() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(2427);
    let mut seen = std::collections::BTreeSet::new();
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for i in 0..4_000 {
                let bb1 = c2AABB { min: rng.vec_norm(8.0), max: rng.vec_norm(8.0) };
                let bb2 = c2AABB { min: rng.vec_norm(8.0), max: rng.vec_norm(8.0) };
                let cap1 = c2Capsule { a: rng.vec_norm(8.0), b: rng.vec_norm(8.0), r: rng.f_pos(2.0) };
                let cap2 = c2Capsule { a: rng.vec_norm(8.0), b: rng.vec_norm(8.0), r: rng.f_pos(2.0) };
                let ci1 = c2Circle { p: rng.vec_norm(8.0), r: rng.f_pos(2.0) };
                let ci2 = c2Circle { p: rng.vec_norm(8.0), r: rng.f_pos(2.0) };
                let pa = match ta {
                    C2_TYPE_CIRCLE => &ci1 as *const _ as *const c_void,
                    C2_TYPE_AABB => &bb1 as *const _ as *const c_void,
                    _ => &cap1 as *const _ as *const c_void,
                };
                let pb = match tb {
                    C2_TYPE_CIRCLE => &ci2 as *const _ as *const c_void,
                    C2_TYPE_AABB => &bb2 as *const _ as *const c_void,
                    _ => &cap2 as *const _ as *const c_void,
                };
                let (mut c_a, mut c_b, mut c_i) = (c2v::default(), c2v::default(), 0i32);
                let (mut r_a, mut r_b, mut r_i) = (c2v::default(), c2v::default(), 0i32);
                let cd = gjk_raw(&cf, pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut c_a, &mut c_b, 0, &mut c_i, std::ptr::null_mut());
                let rd = gjk_raw(&rf, pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut r_a, &mut r_b, 0, &mut r_i, std::ptr::null_mut());
                let ctx = format!("ta={ta} tb={tb} i={i}");
                eq_f32("c2GJK dist", &ctx, cd, rd);
                eq("c2GJK outA", &ctx, &c_a, &r_a);
                eq("c2GJK outB", &ctx, &c_b, &r_b);
                eq_i32("c2GJK iters", &ctx, c_i, r_i);
                seen.insert(c_i);
            }
        }
    }
    println!("row24-27 iteration counts observed: {seen:?}");
    assert!(seen.len() > 1, "only one iteration count was ever observed");
}

// ===========================================================================
// Row 29: c2Support with count <= 0 still reads verts[0] and returns 0
// ===========================================================================

#[test]
fn row29_support_nonpositive_count() {
    let l = libs();
    let (cf, rf) = l.get::<FnSupport>("c2Support");
    let mut rng = Rng::new(29);
    for &count in [0i32, -1, -8, i32::MIN].iter() {
        for _ in 0..2_000 {
            let mut verts = [c2v::default(); 8];
            for k in 0..8 {
                verts[k] = rng.vec_mixed(50.0);
            }
            let d = rng.vec_mixed(10.0);
            let (c, r) = unsafe { (cf(verts.as_ptr(), count, d), rf(verts.as_ptr(), count, d)) };
            eq_i32("c2Support count<=0", &format!("count={count} d={d:?}"), c, r);
            assert_eq!(c, 0, "C must return 0 for count={count}");
        }
    }
}

// ===========================================================================
// Rows 30-31: c2Norms with count <= 0 / count == 1
// ===========================================================================

#[test]
fn row30_31_norms_degenerate_counts() {
    let l = libs();
    let (cf, rf) = l.get::<FnNorms>("c2Norms");
    let mut rng = Rng::new(3031);
    for &count in [0i32, -1, -9, i32::MIN, 1].iter() {
        for seed in [0u8, 61, 255] {
            for _ in 0..1_000 {
                let mut verts = [c2v::default(); 8];
                for k in 0..8 {
                    verts[k] = rng.vec_mixed(20.0);
                }
                let mut cv = verts;
                let mut rv = verts;
                let mut cn = [poison_v(seed); 8];
                let mut rn = [poison_v(seed); 8];
                let before = [poison_v(seed); 8];
                unsafe {
                    cf(cv.as_mut_ptr(), cn.as_mut_ptr(), count);
                    rf(rv.as_mut_ptr(), rn.as_mut_ptr(), count);
                }
                let ctx = format!("count={count} seed={seed}");
                eq("c2Norms", &ctx, &cn, &rn);
                eq("c2Norms input", &ctx, &cv, &rv);
                if count <= 0 {
                    eq("c2Norms count<=0 must write nothing", &ctx, &before, &cn);
                } else {
                    // count == 1: norms[0] must be (NaN, NaN)
                    assert!(cn[0].x.is_nan() && cn[0].y.is_nan(), "count==1 should give a NaN normal, got {:?}", cn[0]);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 32-34: divide-by-zero degeneracies in c2Norm / c2Div / c2Intersect
// ===========================================================================

#[test]
fn row32_norm_of_zero_vector() {
    let l = libs();
    let (cf, rf) = l.get::<FnV_v>("c2Norm");
    for a in [v(0.0, 0.0), v(-0.0, 0.0), v(0.0, -0.0), v(-0.0, -0.0)] {
        let (c, r) = unsafe { (cf(a), rf(a)) };
        eq("c2Norm zero vector", &format!("a={a:?}"), &c, &r);
        assert!(c.x.is_nan() && c.y.is_nan(), "expected (NaN, NaN), got {c:?}");
    }
}

#[test]
fn row33_div_by_zero() {
    let l = libs();
    let (cf, rf) = l.get::<FnV_vf>("c2Div");
    let mut rng = Rng::new(33);
    for b in [0.0f32, -0.0] {
        for _ in 0..2_000 {
            let a = rng.vec_mixed(100.0);
            let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
            eq("c2Div by zero", &format!("a={a:?} b=0x{:08x}", b.to_bits()), &c, &r);
        }
    }
    // and the exact sentinel for a well-behaved numerator
    let (c, _r) = unsafe { (cf(v(1.0, -2.0), 0.0), rf(v(1.0, -2.0), 0.0)) };
    assert_eq!((c.x, c.y), (f32::INFINITY, f32::NEG_INFINITY));
}

#[test]
fn row34_intersect_parallel() {
    let l = libs();
    let (cf, rf) = l.get::<FnV_vvff>("c2Intersect");
    let mut rng = Rng::new(34);
    for _ in 0..4_000 {
        let (a, b) = (rng.vec_mixed(20.0), rng.vec_mixed(20.0));
        // da == db  -> x/0
        let d = rng.f_mixed(10.0);
        let (c, r) = unsafe { (cf(a, b, d, d), rf(a, b, d, d)) };
        eq("c2Intersect da==db", &format!("a={a:?} b={b:?} d=0x{:08x}", d.to_bits()), &c, &r);
    }
    // 0/0 -> NaN
    for (da, db) in [(0.0f32, 0.0f32), (0.0, -0.0), (-0.0, 0.0), (-0.0, -0.0)] {
        let (c, r) = unsafe { (cf(v(1.0, 2.0), v(3.0, 4.0), da, db), rf(v(1.0, 2.0), v(3.0, 4.0), da, db)) };
        eq("c2Intersect 0/0", &format!("da={da} db={db}"), &c, &r);
    }
}

// ===========================================================================
// Rows 35-42, 59-66: the static clip helpers, reached through
// c2CapsuletoPolyManifold (their only callers)
// ===========================================================================

fn call_cp(
    f: &libloading::Symbol<'_, FnCapsulePoly>,
    cap: c2Capsule,
    poly: &c2Poly,
    bx: Option<&c2x>,
    seed: u8,
) -> c2Manifold {
    let mut m = poison_manifold(seed);
    zero_stack();
    unsafe { f(cap, poly, bx.map_or(std::ptr::null(), |x| x as *const c2x), &mut m) };
    m
}

/// Rows 35-42 (`c2Clip` / `c2SidePlanes` rejections) and 59-61 (the early `return`s
/// they cause in `c2CapsuletoPolyManifold`), 66 (`c2KeepDeep` writing `m->n` while
/// leaving `count` at 0), plus 42/64 (`ra == rb` making every distance NaN).
#[test]
fn row35_42_59_66_clip_rejections() {
    let l = libs();
    let (cf, rf) = l.get::<FnCapsulePoly>("c2CapsuletoPolyManifold");
    let mut warmp = c2Poly::default();
    warmp.count = 4;
    warmp.verts[0] = v(-1.0, -1.0);
    warmp.verts[1] = v(1.0, -1.0);
    warmp.verts[2] = v(1.0, 1.0);
    warmp.verts[3] = v(-1.0, 1.0);
    fill_norms(&mut warmp);
    warmup(|| {
        let _ = call_cp(&cf, c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 }, &warmp, None, 1);
        let _ = call_cp(&rf, c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 }, &warmp, None, 1);
    });

    let mut rng = Rng::new(3542);
    // n_written[k] counts outcomes: 0 = count 0 and n untouched (early return),
    // 1 = count 0 but n written (c2KeepDeep kept nothing), 2 = count > 0
    let mut outcomes = [0u32; 3];
    for i in 0..30_000 {
        let count = 3 + (i % 6) as i32;
        let (rad, ctr) = (0.25 + rng.f_pos(4.0), rng.vec_norm(3.0));
        let mut poly = convex_poly(&mut rng, count, rad, ctr);
        fill_norms(&mut poly);
        let cap = match i % 6 {
            // degenerate capsule: ra == rb inside c2SidePlanes -> all NaN -> reject
            0 => {
                let p = rng.vec_norm(3.0);
                c2Capsule { a: p, b: p, r: rng.f_pos(2.0) }
            }
            // capsule endpoints on the lattice: exact zero clip distances
            1 => c2Capsule { a: rng.vec_lattice(3), b: rng.vec_lattice(3), r: rng.below(3) as f32 },
            // tiny capsule far to one side of the reference edge -> sp < 2
            2 => {
                let d = rng.vec_norm(0.01);
                c2Capsule { a: d, b: v(d.x + 1.0e-30, d.y + 1.0e-30), r: rng.f_pos(2.0) }
            }
            // both clip distances tiny and negative -> d0*d1 underflows to +0
            3 => {
                let e = 1.0e-20;
                c2Capsule { a: v(-e, 0.0), b: v(e, 0.0), r: rng.f_pos(2.0) }
            }
            4 => {
                let d = rng.vec_norm(5.0);
                c2Capsule { a: d, b: v(-d.x, -d.y), r: rng.f_pos(2.0) }
            }
            _ => c2Capsule { a: rng.vec_norm(5.0), b: rng.vec_norm(5.0), r: rng.f_pos(2.0) },
        };
        for seed in [17u8, 200] {
            let cm = call_cp(&cf, cap, &poly, None, seed);
            let rm = call_cp(&rf, cap, &poly, None, seed);
            eq(
                "c2CapsuletoPolyManifold clip rejection",
                &format!("i={i} seed={seed} count={count} cap={cap:?} poly={poly:?}"),
                &cm,
                &rm,
            );
            let untouched = poison_manifold(seed);
            if cm.count > 0 {
                outcomes[2] += 1;
            } else if raw(&cm.n) == raw(&untouched.n) {
                outcomes[0] += 1;
            } else {
                outcomes[1] += 1;
            }
        }
    }
    println!("rows35-42/59-66 outcomes: early-return(n untouched)={} keepdeep-empty(n written)={} contact={}",
        outcomes[0], outcomes[1], outcomes[2]);
    assert!(outcomes[0] > 0, "no early-return-with-n-untouched case (rows 59-61)");
    assert!(outcomes[1] > 0, "no c2KeepDeep-kept-nothing case (row 66)");
    assert!(outcomes[2] > 0, "no contact case");
}

/// Rows 62/63: `B->count <= 0` and all-NaN face distances, which leave `index` at
/// `~0 == -1` so the C code reads `verts[-1]` -- four bytes before the caller's
/// `c2Poly`. Both libraries read the *same* address here (the test's own stack), so
/// the out-of-bounds read is well-defined for the purposes of the comparison.
#[test]
fn row62_63_empty_polygon_and_all_nan_faces() {
    let l = libs();
    let (cf, rf) = l.get::<FnCapsulePoly>("c2CapsuletoPolyManifold");
    let mut warmp = c2Poly::default();
    warmp.count = 4;
    fill_norms(&mut warmp);
    warmup(|| {
        let _ = call_cp(&cf, c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 }, &warmp, None, 1);
        let _ = call_cp(&rf, c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 }, &warmp, None, 1);
    });

    // A padded wrapper so the four bytes before `poly` are a *known*, identical value
    // for both libraries.
    #[repr(C)]
    struct Padded {
        pad: [f32; 2],
        poly: c2Poly,
    }

    let mut rng = Rng::new(6263);
    for &pcount in [0i32, -1, -5, 1, 2].iter() {
        for &pad in [0.0f32, 1.0, -3.5, f32::NAN, f32::INFINITY].iter() {
            for i in 0..400 {
                let mut w = Padded { pad: [pad, pad], poly: c2Poly::default() };
                w.poly.count = pcount;
                for k in 0..8 {
                    w.poly.verts[k] = rng.vec_norm(4.0);
                    w.poly.norms[k] = if i % 3 == 0 { v(f32::NAN, f32::NAN) } else { rng.vec_norm(1.0) };
                }
                let cap = match i % 3 {
                    0 => {
                        let d = rng.vec_norm(4.0);
                        c2Capsule { a: d, b: v(-d.x, -d.y), r: rng.f_pos(2.0) }
                    }
                    1 => {
                        let p = rng.vec_norm(4.0);
                        c2Capsule { a: p, b: p, r: rng.f_pos(2.0) } // NaN `ab`
                    }
                    _ => c2Capsule { a: rng.vec_lattice(3), b: rng.vec_lattice(3), r: rng.below(3) as f32 },
                };
                for seed in [17u8, 200] {
                    let cm = call_cp(&cf, cap, &w.poly, None, seed);
                    let rm = call_cp(&rf, cap, &w.poly, None, seed);
                    eq(
                        "c2CapsuletoPolyManifold verts[-1]",
                        &format!("pcount={pcount} pad={pad} i={i} seed={seed} cap={cap:?}"),
                        &cm,
                        &rm,
                    );
                }
            }
        }
    }
}

/// Row 65: `bx_ptr == NULL` must behave exactly like an explicit identity transform.
#[test]
fn row65_null_bx_equals_identity() {
    let l = libs();
    let (cf, rf) = l.get::<FnCapsulePoly>("c2CapsuletoPolyManifold");
    let ident = x_identity();
    let mut warmp = c2Poly::default();
    warmp.count = 4;
    fill_norms(&mut warmp);
    warmup(|| {
        let _ = call_cp(&cf, c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 }, &warmp, None, 1);
        let _ = call_cp(&rf, c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 }, &warmp, Some(&ident), 1);
    });
    let mut rng = Rng::new(65);
    for i in 0..4_000 {
        let (rad, ctr) = (0.5 + rng.f_pos(4.0), rng.vec_norm(3.0));
        let mut poly = convex_poly(&mut rng, 3 + (i % 6) as i32, rad, ctr);
        fill_norms(&mut poly);
        let d = rng.vec_norm(5.0);
        let cap = c2Capsule { a: d, b: v(-d.x, -d.y), r: rng.f_pos(2.0) };
        let c_null = call_cp(&cf, cap, &poly, None, 33);
        let r_null = call_cp(&rf, cap, &poly, None, 33);
        let c_id = call_cp(&cf, cap, &poly, Some(&ident), 33);
        let r_id = call_cp(&rf, cap, &poly, Some(&ident), 33);
        let ctx = format!("i={i} cap={cap:?}");
        eq("bx=NULL", &ctx, &c_null, &r_null);
        eq("bx=identity", &ctx, &c_id, &r_id);
        eq("C: NULL == identity", &ctx, &c_null, &c_id);
    }
}

// ===========================================================================
// Rows 43-46: c2AABBtoAABBManifold rejections
// ===========================================================================

#[test]
fn row43_46_aabb_aabb_rejections() {
    let l = libs();
    let (cf, rf) = l.get::<FnAABBAABB>("c2AABBtoAABBManifold");
    let unit = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
    let mut rng = Rng::new(4346);
    let mut sep_x = 0u32;
    let mut sep_y = 0u32;
    let mut nan_case = 0u32;
    for seed in [0u8, 91, 255] {
        let untouched = poison_manifold(seed);
        for i in 0..4_000 {
            let (A, B) = match i % 4 {
                // row 43: separated on x
                0 => (unit, c2AABB { min: v(5.0 + rng.f_pos(50.0), 0.0), max: v(60.0, 1.0) }),
                // row 44: separated on y only (x overlaps)
                1 => (unit, c2AABB { min: v(-0.5, 5.0 + rng.f_pos(50.0)), max: v(0.5, 60.0) }),
                // row 45: NaN coordinate
                2 => {
                    let mut b = c2AABB { min: rng.vec_norm(3.0), max: rng.vec_norm(3.0) };
                    match rng.below(4) {
                        0 => b.min.x = f32::NAN,
                        1 => b.min.y = f32::NAN,
                        2 => b.max.x = f32::NAN,
                        _ => b.max.y = f32::NAN,
                    }
                    (unit, b)
                }
                // row 46: inverted AABB (still accepted, c2Absv fixes the extent)
                _ => {
                    let max = rng.vec_norm(2.0);
                    (
                        c2AABB { min: v(max.x + rng.f_pos(3.0), max.y + rng.f_pos(3.0)), max },
                        unit,
                    )
                }
            };
            let mut cm = poison_manifold(seed);
            let mut rm = poison_manifold(seed);
            unsafe {
                cf(A, B, &mut cm);
                rf(A, B, &mut rm);
            }
            let ctx = format!("i={i} seed={seed} A={A:?} B={B:?}");
            eq("c2AABBtoAABBManifold", &ctx, &cm, &rm);
            if cm.count == 0 {
                // the early `return`s write only `count`
                let mut e = untouched;
                e.count = 0;
                eq("early return writes only count", &ctx, &e, &cm);
                if i % 4 == 0 { sep_x += 1 } else if i % 4 == 1 { sep_y += 1 }
            } else if i % 4 == 2 {
                // row 45: NaN -> both `< 0` tests false -> y branch, NaN depth
                nan_case += 1;
                assert_eq!((cm.n.x, cm.n.y.abs()), (0.0, 1.0), "row 45 should pick the y axis: {cm:?}");
            }
        }
    }
    println!("rows43-46: sep_x={sep_x} sep_y={sep_y} nan_y_branch={nan_case}");
    assert!(sep_x > 0 && sep_y > 0 && nan_case > 0, "rows 43/44/45 not all covered");
}

// ===========================================================================
// Rows 47-56: the remaining manifold no-contact / degenerate paths
// ===========================================================================

#[test]
fn row47_49_circle_circle_rejections() {
    let l = libs();
    let (cf, rf) = l.get::<FnCircleCircle>("c2CircletoCircleManifold");
    let mut rng = Rng::new(4749);
    for seed in [0u8, 91, 255] {
        let untouched = poison_manifold(seed);
        // row 47: exactly touching must NOT report contact (`d2 < r*r` is strict)
        for k in 0..4 {
            let (ra, rb) = (k as f32, (4 - k) as f32);
            let A = c2Circle { p: v(0.0, 0.0), r: ra };
            let B = c2Circle { p: v(ra + rb, 0.0), r: rb };
            let mut cm = poison_manifold(seed);
            let mut rm = poison_manifold(seed);
            unsafe {
                cf(A, B, &mut cm);
                rf(A, B, &mut rm);
            }
            let ctx = format!("exact touch ra={ra} rb={rb} seed={seed}");
            eq("c2CircletoCircleManifold", &ctx, &cm, &rm);
            assert_eq!(cm.count, 0, "exact touch must not report contact");
            let mut e = untouched;
            e.count = 0;
            eq("no-contact writes only count", &ctx, &e, &cm);
        }
        // row 48: concentric -> fallback normal (0, 1)
        for _ in 0..500 {
            let p = rng.vec_norm(10.0);
            let A = c2Circle { p, r: 1.0 + rng.f_pos(3.0) };
            let B = c2Circle { p, r: 1.0 + rng.f_pos(3.0) };
            let mut cm = poison_manifold(seed);
            let mut rm = poison_manifold(seed);
            unsafe {
                cf(A, B, &mut cm);
                rf(A, B, &mut rm);
            }
            eq("concentric", &format!("A={A:?} B={B:?}"), &cm, &rm);
            assert_eq!((cm.count, cm.n.x, cm.n.y), (1, 0.0, 1.0));
        }
        // row 49: negative radii can still report contact because r is squared
        let mut neg_contacts = 0u32;
        for _ in 0..2_000 {
            let A = c2Circle { p: rng.vec_norm(3.0), r: -(1.0 + rng.f_pos(4.0)) };
            let B = c2Circle { p: rng.vec_norm(3.0), r: -(1.0 + rng.f_pos(4.0)) };
            let mut cm = poison_manifold(seed);
            let mut rm = poison_manifold(seed);
            unsafe {
                cf(A, B, &mut cm);
                rf(A, B, &mut rm);
            }
            eq("negative radii", &format!("A={A:?} B={B:?}"), &cm, &rm);
            if cm.count > 0 {
                neg_contacts += 1;
            }
        }
        assert!(neg_contacts > 0, "row 49: negative radii never reported contact");
    }
}

#[test]
fn row50_52_circle_aabb_rejections() {
    let l = libs();
    let (cf, rf) = l.get::<FnCircleAABB>("c2CircletoAABBManifold");
    let bb = c2AABB { min: v(-2.0, -2.0), max: v(2.0, 2.0) };
    for seed in [0u8, 91, 255] {
        let untouched = poison_manifold(seed);
        // row 50: circle exactly touching a face -> `d2 < r2` false -> no contact
        for &r in [1.0f32, 2.0, 0.5].iter() {
            for p in [v(2.0 + r, 0.0), v(0.0, 2.0 + r), v(-2.0 - r, 0.0), v(0.0, -2.0 - r)] {
                let A = c2Circle { p, r };
                let mut cm = poison_manifold(seed);
                let mut rm = poison_manifold(seed);
                unsafe {
                    cf(A, bb, &mut cm);
                    rf(A, bb, &mut rm);
                }
                let ctx = format!("touch p={p:?} r={r} seed={seed}");
                eq("c2CircletoAABBManifold", &ctx, &cm, &rm);
                assert_eq!(cm.count, 0, "exact touch must not report contact");
                let mut e = untouched;
                e.count = 0;
                eq("no-contact writes only count", &ctx, &e, &cm);
            }
        }
        // rows 51/52: centre inside; tie between x and y overlap picks the y axis
        for t in [-1.0f32, 0.0, 1.0] {
            let A = c2Circle { p: v(t, t), r: 1.0 };
            let mut cm = poison_manifold(seed);
            let mut rm = poison_manifold(seed);
            unsafe {
                cf(A, bb, &mut cm);
                rf(A, bb, &mut rm);
            }
            eq("centre inside, tie", &format!("t={t}"), &cm, &rm);
            assert_eq!(cm.count, 1);
            assert_eq!((cm.n.x, cm.n.y.abs()), (0.0, 1.0), "tie must choose the y axis: {cm:?}");
        }
    }
}

#[test]
fn row53_56_capsule_rejections() {
    let l = libs();
    let (ccc, rcc) = l.get::<FnCircleCapsule>("c2CircletoCapsuleManifold");
    let (ccap, rcap) = l.get::<FnCapsuleCapsule>("c2CapsuletoCapsuleManifold");
    let mut rng = Rng::new(5356);
    for seed in [0u8, 91, 255] {
        let untouched = poison_manifold(seed);
        // rows 53/55: far apart -> count 0, everything else untouched
        for _ in 0..500 {
            let A = c2Circle { p: v(1000.0, 1000.0), r: rng.f_pos(2.0) };
            let B = c2Capsule { a: v(-1.0, 0.0), b: v(1.0, 0.0), r: rng.f_pos(2.0) };
            let mut cm = poison_manifold(seed);
            let mut rm = poison_manifold(seed);
            unsafe {
                ccc(A, B, &mut cm);
                rcc(A, B, &mut rm);
            }
            let ctx = format!("circle/capsule far seed={seed}");
            eq("c2CircletoCapsuleManifold", &ctx, &cm, &rm);
            let mut e = untouched;
            e.count = 0;
            eq("no-contact writes only count", &ctx, &e, &cm);

            let A2 = c2Capsule { a: v(1000.0, 1000.0), b: v(1001.0, 1000.0), r: rng.f_pos(2.0) };
            let mut cm2 = poison_manifold(seed);
            let mut rm2 = poison_manifold(seed);
            unsafe {
                ccap(A2, B, &mut cm2);
                rcap(A2, B, &mut rm2);
            }
            eq("c2CapsuletoCapsuleManifold", &ctx, &cm2, &rm2);
            eq("no-contact writes only count", &ctx, &e, &cm2);
        }
        // rows 54/56: `d == 0` with a degenerate capsule -> NaN normal
        for _ in 0..500 {
            let p = rng.vec_norm(3.0);
            let A = c2Circle { p, r: 1.0 + rng.f_pos(2.0) };
            let B = c2Capsule { a: p, b: p, r: 1.0 + rng.f_pos(2.0) };
            let mut cm = poison_manifold(seed);
            let mut rm = poison_manifold(seed);
            unsafe {
                ccc(A, B, &mut cm);
                rcc(A, B, &mut rm);
            }
            eq("circle/degenerate capsule", &format!("A={A:?} B={B:?}"), &cm, &rm);
            assert!(cm.n.x.is_nan() && cm.n.y.is_nan(), "row 54 expects a NaN normal, got {:?}", cm.n);

            let A2 = c2Capsule { a: p, b: p, r: 1.0 + rng.f_pos(2.0) };
            let mut cm2 = poison_manifold(seed);
            let mut rm2 = poison_manifold(seed);
            unsafe {
                ccap(A2, B, &mut cm2);
                rcap(A2, B, &mut rm2);
            }
            eq("degenerate/degenerate capsule", &format!("A={A2:?} B={B:?}"), &cm2, &rm2);
            assert!(cm2.n.x.is_nan() && cm2.n.y.is_nan(), "row 56 expects a NaN normal, got {:?}", cm2.n);
        }
    }
}

/// Rows 57/58: `c2CapsuletoPolyManifold` with `1e-6 <= d` -- the shallow branch and
/// the no-branch-at-all case.
#[test]
fn row57_58_capsule_poly_distance_bands() {
    let l = libs();
    let (cf, rf) = l.get::<FnCapsulePoly>("c2CapsuletoPolyManifold");
    let mut warmp = c2Poly::default();
    warmp.count = 4;
    fill_norms(&mut warmp);
    warmup(|| {
        let _ = call_cp(&cf, c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 }, &warmp, None, 1);
        let _ = call_cp(&rf, c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 }, &warmp, None, 1);
    });
    let mut rng = Rng::new(5758);
    let mut shallow = 0u32;
    let mut nothing = 0u32;
    for seed in [17u8, 200] {
        let untouched = poison_manifold(seed);
        for i in 0..4_000 {
            let (rad, ctr) = (0.5 + rng.f_pos(3.0), rng.vec_norm(3.0));
            let mut poly = convex_poly(&mut rng, 3 + (i % 6) as i32, rad, ctr);
            fill_norms(&mut poly);
            // Under the zero proxy `d` is the distance from the capsule segment to the
            // origin, so we can place the segment at a chosen distance.
            let want = 1.0 + rng.f_pos(20.0);
            let dir = {
                let t = rng.f_pos(std::f32::consts::TAU);
                v(t.cos(), t.sin())
            };
            let base = v(dir.x * want, dir.y * want);
            let cap = c2Capsule {
                a: base,
                b: v(base.x + dir.y, base.y - dir.x), // perpendicular, same distance-ish
                // A.r either above or below `d`, to hit both branches
                r: if i % 2 == 0 { want * 2.0 } else { want * 0.25 },
            };
            let cm = call_cp(&cf, cap, &poly, None, seed);
            let rm = call_cp(&rf, cap, &poly, None, seed);
            let ctx = format!("i={i} seed={seed} want={want} cap={cap:?}");
            eq("c2CapsuletoPolyManifold band", &ctx, &cm, &rm);
            if cm.count == 0 {
                let mut e = untouched;
                e.count = 0;
                eq("row 57 writes only count", &ctx, &e, &cm);
                nothing += 1;
            } else {
                shallow += 1;
            }
        }
    }
    println!("rows57/58: no-branch={nothing} shallow={shallow}");
    assert!(nothing > 0, "row 57 (neither branch) never fired");
    assert!(shallow > 0, "row 58 (shallow branch) never fired");
}

/// Rows 67/68: `c2AABBtoCapsuleManifold` negates `m->n` even when
/// `c2CapsuletoPolyManifold` bailed out, and a degenerate AABB gives NaN normals.
#[test]
fn row67_68_aabb_capsule_trailing_negate() {
    let l = libs();
    let (cf, rf) = l.get::<FnAABBCapsule>("c2AABBtoCapsuleManifold");
    let unit = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
    warmup(|| {
        let mut m = poison_manifold(1);
        zero_stack();
        unsafe { cf(unit, c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 }, &mut m) };
        let mut m = poison_manifold(1);
        zero_stack();
        unsafe { rf(unit, c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 }, &mut m) };
    });
    let mut rng = Rng::new(6768);
    let mut negated_poison = 0u32;
    for seed in [0u8, 17, 91, 200, 255] {
        let untouched = poison_manifold(seed);
        for i in 0..2_000 {
            let (A, B) = match i % 3 {
                // far away -> c2CapsuletoPolyManifold writes nothing but count
                0 => (unit, c2Capsule { a: v(900.0, 900.0), b: v(901.0, 901.0), r: 0.5 }),
                // row 68: degenerate AABB -> NaN normals from c2Norms
                1 => {
                    let p = rng.vec_norm(2.0);
                    (c2AABB { min: p, max: p }, c2Capsule { a: rng.vec_norm(3.0), b: rng.vec_norm(3.0), r: rng.f_pos(1.0) })
                }
                _ => (unit, c2Capsule { a: rng.vec_norm(3.0), b: rng.vec_norm(3.0), r: rng.f_pos(1.0) }),
            };
            let mut cm = poison_manifold(seed);
            let mut rm = poison_manifold(seed);
            zero_stack();
            unsafe { cf(A, B, &mut cm) };
            zero_stack();
            unsafe { rf(A, B, &mut rm) };
            let ctx = format!("i={i} seed={seed} A={A:?} B={B:?}");
            eq("c2AABBtoCapsuleManifold", &ctx, &cm, &rm);
            if i % 3 == 0 {
                // n must be exactly the negation of the caller's poison
                assert_eq!(cm.count, 0);
                assert_eq!(cm.n.x.to_bits(), untouched.n.x.to_bits() ^ 0x8000_0000, "n.x must be the negated poison");
                assert_eq!(cm.n.y.to_bits(), untouched.n.y.to_bits() ^ 0x8000_0000, "n.y must be the negated poison");
                negated_poison += 1;
            }
        }
    }
    println!("row67: negated-poison cases = {negated_poison}");
    assert!(negated_poison > 0);
}

// ===========================================================================
// Row 69: c2PlaneAt with an out-of-range index
// ===========================================================================

/// The C code indexes `p->norms[i]` / `p->verts[i]` with no bounds check. To make the
/// out-of-bounds read well-defined *for the comparison*, the `c2Poly` is embedded in a
/// larger struct with known padding on both sides, so both libraries read identical
/// bytes at the same address.
#[test]
fn row69_plane_at_out_of_range_index() {
    let l = libs();
    let (cf, rf) = l.get::<FnH_polyi>("c2PlaneAt");

    #[repr(C)]
    struct Padded {
        before: [c2v; 4],
        poly: c2Poly,
        after: [c2v; 4],
    }

    let mut rng = Rng::new(69);
    for trial in 0..2_000 {
        let mut w = Padded {
            before: [c2v::default(); 4],
            poly: c2Poly::default(),
            after: [c2v::default(); 4],
        };
        for k in 0..4 {
            w.before[k] = rng.vec_mixed(30.0);
            w.after[k] = rng.vec_mixed(30.0);
        }
        w.poly.count = 1 + rng.below(8) as i32;
        for k in 0..8 {
            w.poly.verts[k] = rng.vec_mixed(30.0);
            w.poly.norms[k] = rng.vec_mixed(1.0);
        }
        // in range, one past the end, and negative
        for i in [-4i32, -3, -2, -1, 0, 7, 8, 9, 10, 11] {
            let (c, r) = unsafe { (cf(&w.poly, i), rf(&w.poly, i)) };
            eq("c2PlaneAt OOB index", &format!("trial={trial} i={i}"), &c, &r);
        }
    }
}

// ===========================================================================
// Rows 70-79: c22 / c23 branch selection with degenerate inputs
// ===========================================================================

#[test]
fn row70_72_c22_branches() {
    let l = libs();
    let (cf, rf) = l.get::<FnSimplexVoid>("c22");
    let mut rng = Rng::new(7072);
    let mut branches = [0u32; 3];
    for i in 0..40_000 {
        let mut s = c2Simplex::default();
        s.count = 2;
        s.a.sA = rng.vec_norm(10.0);
        s.a.sB = rng.vec_norm(10.0);
        s.b.sA = rng.vec_norm(10.0);
        s.b.sB = rng.vec_norm(10.0);
        s.a.iA = rng.below(4) as i32;
        s.b.iB = rng.below(4) as i32;
        s.div = rng.f_norm(5.0);
        match i % 5 {
            // row 70: v <= 0
            0 => {
                let d = rng.vec_norm(5.0);
                s.a.p = d;
                s.b.p = v(d.x * 3.0, d.y * 3.0);
            }
            // row 71: u <= 0
            1 => {
                let d = rng.vec_norm(5.0);
                s.a.p = v(d.x * 3.0, d.y * 3.0);
                s.b.p = d;
            }
            // row 72: u > 0 && v > 0
            2 => {
                let d = rng.vec_norm(5.0);
                s.a.p = d;
                s.b.p = v(-d.x, -d.y);
            }
            // a == b: u == v == 0 -> row 70
            3 => {
                s.a.p = rng.vec_norm(5.0);
                s.b.p = s.a.p;
            }
            // underflowing div
            _ => {
                s.a.p = v(1.0e-30, 0.0);
                s.b.p = v(-1.0e-30, 0.0);
            }
        }
        let (mut cs, mut rs) = (s, s);
        unsafe {
            cf(&mut cs);
            rf(&mut rs);
        }
        eq("c22 branch", &format!("i={i} in={s:?}"), &cs, &rs);
        // Classify by which vertex survived. Both collapse branches overwrite
        // `a.u` with 1.0, so compare the *witness* field instead: the vertex-A
        // branch leaves `a.sA` alone, the vertex-B branch copies `b` over `a`.
        match (cs.count, raw(&cs.a.sA) == raw(&s.a.sA)) {
            (1, true) => branches[0] += 1,
            (1, false) => branches[1] += 1,
            _ => branches[2] += 1,
        }
    }
    println!("rows70-72 c22 branches: A={} B={} edge={}", branches[0], branches[1], branches[2]);
    assert!(branches.iter().all(|&x| x > 0), "not all c22 branches covered: {branches:?}");
}

#[test]
fn row73_79_c23_branches() {
    let l = libs();
    let (cf, rf) = l.get::<FnSimplexVoid>("c23");
    let mut rng = Rng::new(7379);
    let mut all_nan = 0u32;
    for i in 0..40_000 {
        let mut s = c2Simplex::default();
        s.count = 3;
        for sv in [&mut s.a, &mut s.b, &mut s.c] {
            sv.sA = rng.vec_norm(10.0);
            sv.sB = rng.vec_norm(10.0);
        }
        s.div = rng.f_norm(5.0);
        if i % 7 == 0 {
            // row 79: all NaN -> every comparison false -> interior fallthrough
            s.a.p = v(f32::NAN, f32::NAN);
            s.b.p = v(f32::NAN, f32::NAN);
            s.c.p = v(f32::NAN, f32::NAN);
        } else {
            s.a.p = rng.vec_lattice(4);
            s.b.p = rng.vec_lattice(4);
            s.c.p = rng.vec_lattice(4);
        }
        let (mut cs, mut rs) = (s, s);
        unsafe {
            cf(&mut cs);
            rf(&mut rs);
        }
        eq("c23 branch", &format!("i={i} in={s:?}"), &cs, &rs);
        if i % 7 == 0 {
            all_nan += 1;
            assert_eq!(cs.count, 3, "row 79: all-NaN must fall through to the interior branch");
            assert!(cs.div.is_nan(), "row 79: div should be NaN");
        }
    }
    println!("row79 all-NaN c23 cases: {all_nan}");
    assert!(all_nan > 0);
}

// ===========================================================================
// Rows 80-86: switch defaults in c2GJKSimplexMetric / c2D / c2Witness / c2L
// ===========================================================================

#[test]
fn row80_86_simplex_switch_defaults() {
    let l = libs();
    let (cmetric, rmetric) = l.get::<FnSimplexF>("c2GJKSimplexMetric");
    let (cd, rd) = l.get::<FnSimplexV>("c2D");
    let (cw, rw) = l.get::<FnWitness>("c2Witness");
    let (cll, rll) = l.get::<FnSimplexV>("c2L");
    let mut rng = Rng::new(8086);
    let counts = [i32::MIN, -100, -3, -1, 0, 1, 2, 3, 4, 5, 99, i32::MAX];
    for &count in counts.iter() {
        for i in 0..2_000 {
            let mut s = c2Simplex::default();
            s.count = count;
            for sv in [&mut s.a, &mut s.b, &mut s.c, &mut s.d] {
                sv.sA = rng.vec_mixed(20.0);
                sv.sB = rng.vec_mixed(20.0);
                sv.p = rng.vec_mixed(20.0);
                sv.u = rng.f_mixed(10.0);
                sv.iA = rng.below(8) as i32;
                sv.iB = rng.below(8) as i32;
            }
            // rows 84/86: div == 0 -> den == +inf
            s.div = match i % 6 { 0 => 0.0, 1 => -0.0, 2 => f32::NAN, 3 => f32::INFINITY, 4 => 1.0, _ => rng.f_norm(10.0) };
            let ctx = format!("count={count} div=0x{:08x} i={i}", s.div.to_bits());

            let (mut a, mut b) = (s, s);
            let (x, y) = unsafe { (cmetric(&mut a), rmetric(&mut b)) };
            eq_f32("c2GJKSimplexMetric", &ctx, x, y);
            if !(2..=3).contains(&count) {
                assert_eq!(x, 0.0, "row 80: default must return 0 for count={count}");
            }

            let (mut a, mut b) = (s, s);
            let (x, y) = unsafe { (cd(&mut a), rd(&mut b)) };
            eq("c2D", &ctx, &x, &y);
            if count != 1 && count != 2 {
                assert_eq!((x.x, x.y), (0.0, 0.0), "row 81: default must return (0,0)");
            }

            let (mut a, mut b) = (s, s);
            let (mut ca, mut cb) = (poison_v(1), poison_v(2));
            let (mut ra, mut rb) = (poison_v(1), poison_v(2));
            unsafe {
                cw(&mut a, &mut ca, &mut cb);
                rw(&mut b, &mut ra, &mut rb);
            }
            eq("c2Witness outA", &ctx, &ca, &ra);
            eq("c2Witness outB", &ctx, &cb, &rb);
            if !(1..=3).contains(&count) {
                assert_eq!((ca.x, ca.y, cb.x, cb.y), (0.0, 0.0, 0.0, 0.0), "row 83: default must zero both");
            }

            let (mut a, mut b) = (s, s);
            let (x, y) = unsafe { (cll(&mut a), rll(&mut b)) };
            eq("c2L", &ctx, &x, &y);
            if count != 1 && count != 2 {
                // row 85: note c2L's default covers count == 3 too, unlike c2Witness
                assert_eq!((x.x, x.y), (0.0, 0.0), "row 85: default must return (0,0)");
            }
        }
    }
}

// ===========================================================================
// Rows 87-95: NaN / signed-zero semantics of the comparison primitives
// ===========================================================================

#[test]
fn row87_maxv_minv_nan_returns_second_operand() {
    let l = libs();
    let (cmax, rmax) = l.get::<FnV_vv>("c2Maxv");
    let (cmin, rmin) = l.get::<FnV_vv>("c2Minv");
    let nans = [0x7fc0_0000u32, 0xffc0_0000, 0x7fc0_1234, 0xffc0_5678, 0x7f80_0001, 0xff80_0001];
    for &nb in nans.iter() {
        let n = f32::from_bits(nb);
        for &ob in [0x3f80_0000u32, 0xbf80_0000, 0x0000_0000, 0x8000_0000, 0x7f80_0000, 0xff80_0000].iter() {
            let o = f32::from_bits(ob);
            for (a, b) in [(v(n, n), v(o, o)), (v(o, o), v(n, n)), (v(n, o), v(o, n)), (v(n, n), v(n, n))] {
                let ctx = format!("a={a:?} b={b:?} nan=0x{nb:08x} other=0x{ob:08x}");
                let (c, r) = unsafe { (cmax(a, b), rmax(a, b)) };
                eq("c2Maxv NaN", &ctx, &c, &r);
                // C's ternary is false when either operand is NaN -> returns b
                if a.x.is_nan() || b.x.is_nan() {
                    assert_eq!(c.x.to_bits(), b.x.to_bits(), "c2Maxv must return b.x: {ctx}");
                }
                let (c, r) = unsafe { (cmin(a, b), rmin(a, b)) };
                eq("c2Minv NaN", &ctx, &c, &r);
                if a.x.is_nan() || b.x.is_nan() {
                    assert_eq!(c.x.to_bits(), b.x.to_bits(), "c2Minv must return b.x: {ctx}");
                }
            }
        }
    }
}

#[test]
fn row88_89_absv_signed_zero_and_nan() {
    let l = libs();
    let (cf, rf) = l.get::<FnV_v>("c2Absv");
    for &bits in [0x8000_0000u32, 0x0000_0000, 0xffc0_5678, 0x7fc0_1234, 0xff80_0001, 0x7f80_0001, 0x8000_0001].iter() {
        let x = f32::from_bits(bits);
        let a = v(x, x);
        let (c, r) = unsafe { (cf(a), rf(a)) };
        eq("c2Absv", &format!("0x{bits:08x}"), &c, &r);
        if bits == 0x8000_0000 {
            assert_eq!(c.x.to_bits(), 0x8000_0000, "row 88: -0.0 must be returned unchanged");
        }
        if x.is_nan() {
            assert_eq!(c.x.to_bits(), bits, "row 89: NaN must pass through with its sign intact");
        }
    }
}

#[test]
fn row90_91_clampv_inverted_and_nan() {
    let l = libs();
    let (cf, rf) = l.get::<FnV_vvv>("c2Clampv");
    let mut rng = Rng::new(9091);
    // row 90: lo > hi
    for _ in 0..4_000 {
        let hi = rng.vec_norm(10.0);
        let lo = v(hi.x + rng.f_pos(10.0) + 0.1, hi.y + rng.f_pos(10.0) + 0.1);
        let a = rng.vec_norm(20.0);
        let (c, r) = unsafe { (cf(a, lo, hi), rf(a, lo, hi)) };
        eq("c2Clampv inverted", &format!("a={a:?} lo={lo:?} hi={hi:?}"), &c, &r);
        assert_eq!((c.x, c.y), (lo.x, lo.y), "row 90: inverted range must yield lo");
    }
    // row 91: NaN in each of a / lo / hi
    let nan = f32::from_bits(0x7fc0_1234);
    for pos in 0..3 {
        for xy in 0..2 {
            let mut a = v(1.0, 1.0);
            let mut lo = v(-2.0, -2.0);
            let mut hi = v(2.0, 2.0);
            let t = match pos { 0 => &mut a, 1 => &mut lo, _ => &mut hi };
            if xy == 0 { t.x = nan } else { t.y = nan }
            let (c, r) = unsafe { (cf(a, lo, hi), rf(a, lo, hi)) };
            eq("c2Clampv NaN", &format!("pos={pos} xy={xy}"), &c, &r);
        }
    }
}

#[test]
fn row92_93_dot_dist_len_inf_times_zero() {
    let l = libs();
    let (cdot, rdot) = l.get::<FnF_vv>("c2Dot");
    let (cdist, rdist) = l.get::<FnF_hv>("c2Dist");
    let (clen, rlen) = l.get::<FnF_v>("c2Len");
    let inf = f32::INFINITY;
    let cases = [
        (v(inf, 0.0), v(0.0, inf)),
        (v(0.0, inf), v(inf, 0.0)),
        (v(inf, inf), v(0.0, 0.0)),
        (v(inf, -inf), v(1.0, 1.0)),
        (v(FLT_MAX, FLT_MAX), v(FLT_MAX, FLT_MAX)),
    ];
    for (a, b) in cases {
        let ctx = format!("a={a:?} b={b:?}");
        let (c, r) = unsafe { (cdot(a, b), rdot(a, b)) };
        eq_f32("c2Dot inf*0", &ctx, c, r);
        let h = c2h { n: a, d: b.x };
        let (c, r) = unsafe { (cdist(h, b), rdist(h, b)) };
        eq_f32("c2Dist inf*0", &ctx, c, r);
        let (c, r) = unsafe { (clen(a), rlen(a)) };
        eq_f32("c2Len", &ctx, c, r);
    }
    // row 93: sqrtf of inf / NaN
    for a in [v(inf, 0.0), v(f32::NAN, 0.0), v(FLT_MAX, FLT_MAX), v(f32::from_bits(0x7f80_0001), 0.0)] {
        let (c, r) = unsafe { (clen(a), rlen(a)) };
        eq_f32("c2Len special", &format!("a={a:?}"), c, r);
    }
}

/// Row 94: a signalling NaN argument must be quieted identically by every entry point
/// that does arithmetic on it.
#[test]
fn row94_signalling_nan_quieting() {
    let l = libs();
    let snans = [0x7f80_0001u32, 0xff80_0001, 0x7fbf_ffff, 0xffbf_ffff, 0x7f80_0000 | 0x1234];
    let others = [0x3f80_0000u32, 0x0000_0000, 0x8000_0000, 0x7f80_0000, 0x7fc0_0000];

    let unary = ["c2Neg", "c2CCW90", "c2Skew", "c2Absv", "c2Norm"];
    for name in unary {
        let (cf, rf) = l.get::<FnV_v>(name);
        for &s in snans.iter() {
            for &o in others.iter() {
                let a = v(f32::from_bits(s), f32::from_bits(o));
                let (c, r) = unsafe { (cf(a), rf(a)) };
                eq(name, &format!("sNaN 0x{s:08x} other 0x{o:08x}"), &c, &r);
            }
        }
    }
    let binary = ["c2Add", "c2Sub", "c2Maxv", "c2Minv"];
    for name in binary {
        let (cf, rf) = l.get::<FnV_vv>(name);
        for &s in snans.iter() {
            for &o in others.iter() {
                for (a, b) in [
                    (v(f32::from_bits(s), f32::from_bits(o)), v(f32::from_bits(o), f32::from_bits(s))),
                    (v(f32::from_bits(o), f32::from_bits(s)), v(f32::from_bits(s), f32::from_bits(o))),
                    (v(f32::from_bits(s), f32::from_bits(s)), v(f32::from_bits(s), f32::from_bits(s))),
                ] {
                    let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
                    eq(name, &format!("sNaN a={a:?} b={b:?}"), &c, &r);
                }
            }
        }
    }
    let scalar = ["c2Dot", "c2Det2"];
    for name in scalar {
        let (cf, rf) = l.get::<FnF_vv>(name);
        for &s in snans.iter() {
            for &o in others.iter() {
                let a = v(f32::from_bits(s), f32::from_bits(o));
                let b = v(f32::from_bits(o), f32::from_bits(s));
                let (c, r) = unsafe { (cf(a, b), rf(a, b)) };
                eq_f32(name, &format!("sNaN a={a:?} b={b:?}"), c, r);
            }
        }
    }
    // and through c2Mulvs / c2Div, where the scalar is the sNaN
    for name in ["c2Mulvs", "c2Div"] {
        let (cf, rf) = l.get::<FnV_vf>(name);
        for &s in snans.iter() {
            for &o in others.iter() {
                let a = v(f32::from_bits(o), f32::from_bits(o));
                let sc = f32::from_bits(s);
                let (c, r) = unsafe { (cf(a, sc), rf(a, sc)) };
                eq(name, &format!("sNaN scalar 0x{s:08x} a={a:?}"), &c, &r);
            }
        }
    }
}

#[test]
fn row95_bbverts_inverted_aabb() {
    let l = libs();
    let (cf, rf) = l.get::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(95);
    for _ in 0..4_000 {
        let max = rng.vec_norm(50.0);
        let min = v(max.x + rng.f_pos(50.0) + 0.1, max.y + rng.f_pos(50.0) + 0.1);
        let bb = c2AABB { min, max };
        let (mut cbb, mut rbb) = (bb, bb);
        let mut cout = [poison_v(3); 8];
        let mut rout = [poison_v(3); 8];
        unsafe {
            cf(cout.as_mut_ptr(), &mut cbb);
            rf(rout.as_mut_ptr(), &mut rbb);
        }
        let ctx = format!("inverted bb={bb:?}");
        eq("c2BBVerts inverted", &ctx, &cout, &rout);
        // no validation: the corners come out in the declared order
        assert_eq!((cout[0].x, cout[0].y), (min.x, min.y));
        assert_eq!((cout[2].x, cout[2].y), (max.x, max.y));
        // and slots 4..8 must be untouched
        eq("c2BBVerts must write only 4 slots", &ctx, &[poison_v(3); 4], &[cout[4], cout[5], cout[6], cout[7]]);
    }
}
