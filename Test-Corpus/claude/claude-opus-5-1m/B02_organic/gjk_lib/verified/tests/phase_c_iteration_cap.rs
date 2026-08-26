//! ERRORS.md row 25 — the `while (iter < 20)` iteration cap.
//!
//! This is the one C path that a plain happy-path sweep never reaches, and it is
//! special: it is the ONLY loop exit that leaves the simplex *un-reduced*. Every
//! other exit (`d1 > d0`, degenerate direction, `dup`) happens either before a
//! vertex is appended or after `c22`/`c23` ran, so `count` and the `u` weights
//! are consistent. When the loop instead runs out of iterations right after
//! `++s.count`, the freshly appended `c2sv` has had `iA/sA/iB/sB/p` written but
//! NOT `u` (see `c_src/src/lib.c` L453-L458) — and `c2Witness` then reads that
//! `u`.
//!
//! So this file (a) SEARCHES for inputs that drive the iteration count up, and
//! (b) reports the maximum reached, so the cap's reachability is a measured fact
//! rather than an assumption.

#![allow(non_snake_case)]

#[macro_use]
mod common;

use common::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicI32, Ordering};

static MAX_ITERS: AtomicI32 = AtomicI32::new(-1);

fn note(iters: i32) {
    MAX_ITERS.fetch_max(iters, Ordering::Relaxed);
}

#[allow(clippy::too_many_arguments)]
unsafe fn call(
    f: &FnGJK,
    pa: *const c_void,
    ta: i32,
    pb: *const c_void,
    tb: i32,
    ur: i32,
    cache: Option<&mut GJKCache>,
) -> (f32, V, V, i32) {
    let poison = f32::from_bits(0xA5A5_A5A5);
    let mut a = V::new(poison, poison);
    let mut b = a;
    let mut it: i32 = -1;
    let cp = match cache {
        Some(c) => c as *mut GJKCache,
        None => std::ptr::null_mut(),
    };
    let d = unsafe {
        f(pa, ta, std::ptr::null(), pb, tb, std::ptr::null(), &mut a, &mut b, ur, &mut it, cp)
    };
    (d, a, b, it)
}

/// Broad search over shape classes and magnitudes, tracking both libraries'
/// agreement AND the highest iteration count either one reports.
#[test]
fn iteration_cap_search() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let mut g = Rng::new(0xCA01);

    let mut hist = [0usize; 22];

    for round in 0..200_000 {
        // Deliberately nasty geometry: tiny, degenerate, mixed magnitude,
        // grid-snapped and NaN/Inf-laced shapes are the ones that make the
        // simplex oscillate instead of converging.
        let scale = match round % 5 {
            0 => 1.0f32,
            1 => 1e-30,
            2 => 1e30,
            3 => 1e-6,
            _ => 1e6,
        };
        let mk = |g: &mut Rng, kind: u32| -> (Circle, AABB, Capsule) {
            let _ = kind;
            let c = Circle { p: V::new(g.grid() * scale, g.grid() * scale), r: g.grid().abs() * scale };
            let bb = AABB {
                min: V::new(g.grid() * scale, g.grid() * scale),
                max: V::new(g.grid() * scale, g.grid() * scale),
            };
            let cap = Capsule {
                a: V::new(g.grid() * scale, g.grid() * scale),
                b: V::new(g.grid() * scale, g.grid() * scale),
                r: g.grid().abs() * scale,
            };
            (c, bb, cap)
        };
        let (ci, bb, cap) = mk(&mut g, 0);
        let (ci2, bb2, cap2) = mk(&mut g, 1);

        let sa: [(*const c_void, i32); 3] = [
            (&ci as *const Circle as *const c_void, C2_TYPE_CIRCLE),
            (&bb as *const AABB as *const c_void, C2_TYPE_AABB),
            (&cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE),
        ];
        let sb: [(*const c_void, i32); 3] = [
            (&ci2 as *const Circle as *const c_void, C2_TYPE_CIRCLE),
            (&bb2 as *const AABB as *const c_void, C2_TYPE_AABB),
            (&cap2 as *const Capsule as *const c_void, C2_TYPE_CAPSULE),
        ];

        for &(pa, ta) in sa.iter() {
            for &(pb, tb) in sb.iter() {
                for &ur in &[0i32, 1] {
                    let mut cc = GJKCache::default();
                    let mut rc = GJKCache::default();
                    let (cd, ca, cb, cit) = unsafe { call(&c, pa, ta, pb, tb, ur, Some(&mut cc)) };
                    let (rd, ra, rb, rit) = unsafe { call(&r, pa, ta, pb, tb, ur, Some(&mut rc)) };
                    ck_f32!("cap dist", cd, rd, "round={round} ta={ta} tb={tb} ur={ur}");
                    ck_v!("cap outA", ca, ra, "round={round} ta={ta} tb={tb} ur={ur}");
                    ck_v!("cap outB", cb, rb, "round={round} ta={ta} tb={tb} ur={ur}");
                    ck_i32!("cap iters", cit, rit, "round={round} ta={ta} tb={tb} ur={ur}");
                    ck_bytes!("cap cache", cc, rc, "round={round} ta={ta} tb={tb} ur={ur}");
                    note(cit);
                    if (0..22).contains(&cit) {
                        hist[cit as usize] += 1;
                    }
                }
            }
        }
    }

    eprintln!("iteration-count histogram (index = iterations): {hist:?}");
    eprintln!("max iterations observed = {}", MAX_ITERS.load(Ordering::Relaxed));
}

/// Warm-start caches let the loop begin from a 2- or 3-vertex simplex, which is
/// the most direct way to push the iteration count up. All indices are kept in
/// range so the C stays inside initialised proxy verts.
#[test]
fn iteration_cap_via_warm_cache() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let mut g = Rng::new(0xCA02);
    let mut maxit = -1i32;

    for round in 0..100_000 {
        let bb = AABB { min: V::new(g.grid(), g.grid()), max: V::new(g.grid(), g.grid()) };
        let bb2 = AABB { min: V::new(g.grid(), g.grid()), max: V::new(g.grid(), g.grid()) };
        let pa = &bb as *const AABB as *const c_void;
        let pb = &bb2 as *const AABB as *const c_void;

        for count in 1..=3i32 {
            let mut cc = GJKCache {
                metric: g.range(-2e9, 2e9),
                count,
                iA: [0; 3],
                iB: [0; 3],
                div: g.range(-5.0, 5.0),
            };
            for k in 0..3 {
                cc.iA[k] = (g.next_u32() % 4) as i32; // AABB -> 4 verts
                cc.iB[k] = (g.next_u32() % 4) as i32;
            }
            let mut rc = cc;
            for &ur in &[0i32, 1] {
                let (cd, ca, cb, cit) = unsafe { call(&c, pa, C2_TYPE_AABB, pb, C2_TYPE_AABB, ur, Some(&mut cc)) };
                let (rd, ra, rb, rit) = unsafe { call(&r, pa, C2_TYPE_AABB, pb, C2_TYPE_AABB, ur, Some(&mut rc)) };
                ck_f32!("warmcap dist", cd, rd, "round={round} count={count} ur={ur}");
                ck_v!("warmcap outA", ca, ra, "round={round} count={count} ur={ur}");
                ck_v!("warmcap outB", cb, rb, "round={round} count={count} ur={ur}");
                ck_i32!("warmcap iters", cit, rit, "round={round} count={count} ur={ur}");
                ck_bytes!("warmcap cache", cc, rc, "round={round} count={count} ur={ur}");
                maxit = maxit.max(cit);
                note(cit);
            }
        }
    }
    eprintln!("warm-cache max iterations observed = {maxit}");
}

/// Exhaustive small-integer lattice: every AABB-vs-AABB pair on a 3x3 lattice,
/// which is where support-function ties (and therefore simplex oscillation) are
/// densest.
#[test]
fn iteration_cap_exhaustive_lattice() {
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let mut maxit = -1i32;
    let mut checked = 0usize;

    for a0 in 0..3i32 {
        for a1 in 0..3i32 {
            for b0 in -1..3i32 {
                for b1 in -1..3i32 {
                    for cx in -2..3i32 {
                        for cy in -2..3i32 {
                            let bb = AABB {
                                min: V::new(a0 as f32, a1 as f32),
                                max: V::new(a0 as f32 + 1.0, a1 as f32 + 1.0),
                            };
                            let cap = Capsule {
                                a: V::new(b0 as f32, b1 as f32),
                                b: V::new(cx as f32, cy as f32),
                                r: 0.0,
                            };
                            let circ = Circle { p: V::new(cx as f32, cy as f32), r: 1.0 };
                            let pairs: [(*const c_void, i32, *const c_void, i32); 3] = [
                                (
                                    &bb as *const AABB as *const c_void, C2_TYPE_AABB,
                                    &cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE,
                                ),
                                (
                                    &cap as *const Capsule as *const c_void, C2_TYPE_CAPSULE,
                                    &bb as *const AABB as *const c_void, C2_TYPE_AABB,
                                ),
                                (
                                    &bb as *const AABB as *const c_void, C2_TYPE_AABB,
                                    &circ as *const Circle as *const c_void, C2_TYPE_CIRCLE,
                                ),
                            ];
                            for &(pa, ta, pb, tb) in pairs.iter() {
                                for &ur in &[0i32, 1] {
                                    let mut cc = GJKCache::default();
                                    let mut rc = GJKCache::default();
                                    let (cd, ca, cb, cit) =
                                        unsafe { call(&c, pa, ta, pb, tb, ur, Some(&mut cc)) };
                                    let (rd, ra, rb, rit) =
                                        unsafe { call(&r, pa, ta, pb, tb, ur, Some(&mut rc)) };
                                    ck_f32!("lattice dist", cd, rd, "ta={ta} tb={tb} ur={ur}");
                                    ck_v!("lattice outA", ca, ra, "ta={ta} tb={tb} ur={ur}");
                                    ck_v!("lattice outB", cb, rb, "ta={ta} tb={tb} ur={ur}");
                                    ck_i32!("lattice iters", cit, rit, "ta={ta} tb={tb} ur={ur}");
                                    ck_bytes!("lattice cache", cc, rc, "ta={ta} tb={tb} ur={ur}");
                                    maxit = maxit.max(cit);
                                    note(cit);
                                    checked += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("lattice: {checked} calls, max iterations = {maxit}");
}

/// Records the global maximum so the cap's reachability is documented.
/// The C caps `iter` at 20, so any observed value must lie in `0..=20`.
#[test]
fn zz_report_max_iterations() {
    // depends on the other tests in this binary having run; they share the process
    let l = libs();
    let (c, r) = l.get::<FnGJK>("c2GJK");
    let mut g = Rng::new(0xCA03);
    for _ in 0..50_000 {
        let bb = AABB { min: g.v_grid(), max: g.v_grid() };
        let cap = Capsule { a: g.v_grid(), b: g.v_grid(), r: g.grid().abs() };
        let pa = &bb as *const AABB as *const c_void;
        let pb = &cap as *const Capsule as *const c_void;
        let (_, _, _, cit) = unsafe { call(&c, pa, C2_TYPE_AABB, pb, C2_TYPE_CAPSULE, 1, None) };
        let (_, _, _, rit) = unsafe { call(&r, pa, C2_TYPE_AABB, pb, C2_TYPE_CAPSULE, 1, None) };
        ck_i32!("iters", cit, rit, "bb={bb:?} cap={cap:?}");
        note(cit);
        assert!(
            (0..=20).contains(&cit),
            "C reported {cit} iterations, outside the 0..=20 the cap allows"
        );
    }
    eprintln!(
        "GLOBAL max iterations observed across this binary = {}",
        MAX_ITERS.load(Ordering::Relaxed)
    );
}
