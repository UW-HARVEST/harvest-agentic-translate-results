//! Phase B — CONFIGS.md rows 60..68: the boolean collision wrappers, the 9-way
//! `c2Collided` dispatcher, and the header-declared `capsule` driver.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 20000;

// ------------------------------------------------------------------- row 60
#[test]
fn row60_c2AABBtoAABB() {
    let p = apis();
    let mut rng = Rng::new(0x6000);
    for _ in 0..n_cases(N) {
        let (A, B) = (rng.aabb(), rng.aabb());
        unsafe {
            eq_int(
                "c2AABBtoAABB",
                &(A, B),
                (p.c.c2AABBtoAABB)(A, B),
                (p.r.c2AABBtoAABB)(A, B),
            );
            eq_int(
                "c2AABBtoAABB/rev",
                &(B, A),
                (p.c.c2AABBtoAABB)(B, A),
                (p.r.c2AABBtoAABB)(B, A),
            );
        }
    }
    // structured cases: separated on x only / y only / touching edges / nested
    for _ in 0..n_cases(4000) {
        let x0 = (rng.below(21) as f32) - 10.0;
        let y0 = (rng.below(21) as f32) - 10.0;
        let w = (rng.below(10) as f32);
        let h = (rng.below(10) as f32);
        let A = c2AABB {
            min: c2v { x: x0, y: y0 },
            max: c2v { x: x0 + w, y: y0 + h },
        };
        for &(dx, dy) in &[
            (0.0f32, 0.0f32),
            (w, 0.0),   // touching on x
            (0.0, h),   // touching on y
            (w, h),     // corner touch
            (w + 1.0, 0.0),
            (0.0, h + 1.0),
            (w * 0.5, h * 0.5),
            (-w, -h),
        ] {
            let B = c2AABB {
                min: c2v { x: A.min.x + dx, y: A.min.y + dy },
                max: c2v { x: A.max.x + dx, y: A.max.y + dy },
            };
            unsafe {
                eq_int(
                    "c2AABBtoAABB/struct",
                    &(A, B),
                    (p.c.c2AABBtoAABB)(A, B),
                    (p.r.c2AABBtoAABB)(A, B),
                );
            }
        }
    }
    // NaN / Inf corners and inverted boxes
    for &s in SPECIAL_F32 {
        for &t in SPECIAL_F32 {
            let A = c2AABB {
                min: c2v { x: s, y: t },
                max: c2v { x: t, y: s },
            };
            let B = c2AABB {
                min: c2v { x: -s, y: -t },
                max: c2v { x: -t, y: -s },
            };
            unsafe {
                eq_int("c2AABBtoAABB/sp", &(A, B), (p.c.c2AABBtoAABB)(A, B), (p.r.c2AABBtoAABB)(A, B));
                eq_int("c2AABBtoAABB/sp2", &(A, A), (p.c.c2AABBtoAABB)(A, A), (p.r.c2AABBtoAABB)(A, A));
            }
        }
    }
}

// ------------------------------------------------------------------- row 61
#[test]
fn row61_c2CircletoCircle() {
    let p = apis();
    let mut rng = Rng::new(0x6100);
    for _ in 0..n_cases(N) {
        let (A, B) = (rng.circle(), rng.circle());
        unsafe {
            eq_int("c2CircletoCircle", &(A, B), (p.c.c2CircletoCircle)(A, B), (p.r.c2CircletoCircle)(A, B));
            eq_int("c2CircletoCircle/rev", &(B, A), (p.c.c2CircletoCircle)(B, A), (p.r.c2CircletoCircle)(B, A));
        }
    }
    // exactly touching: d2 == r2 -> strict `<` is false
    for _ in 0..n_cases(4000) {
        let rA = (rng.below(20) as f32) * 0.5;
        let rB = (rng.below(20) as f32) * 0.5;
        let sum = rA + rB;
        for &d in &[sum, sum - 0.5, sum + 0.5, 0.0] {
            let A = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA };
            let B = c2Circle { p: c2v { x: d, y: 0.0 }, r: rB };
            unsafe {
                eq_int("c2CircletoCircle/touch", &(A, B), (p.c.c2CircletoCircle)(A, B), (p.r.c2CircletoCircle)(A, B));
            }
        }
    }
    for &s in SPECIAL_F32 {
        for &r in SPECIAL_F32 {
            let A = c2Circle { p: c2v { x: s, y: -s }, r };
            let B = c2Circle { p: c2v { x: -s, y: s }, r: -r };
            unsafe {
                eq_int("c2CircletoCircle/sp", &(A, B), (p.c.c2CircletoCircle)(A, B), (p.r.c2CircletoCircle)(A, B));
            }
        }
    }
}

// ------------------------------------------------------------------- row 62
#[test]
fn row62_c2CircletoAABB() {
    let p = apis();
    let mut rng = Rng::new(0x6200);
    for _ in 0..n_cases(N) {
        let A = rng.circle();
        let B = rng.aabb();
        unsafe {
            eq_int("c2CircletoAABB", &(A, B), (p.c.c2CircletoAABB)(A, B), (p.r.c2CircletoAABB)(A, B));
        }
    }
    // walk the circle centre through all 9 regions of a fixed box
    let B = c2AABB {
        min: c2v { x: -3.0, y: -2.0 },
        max: c2v { x: 4.0, y: 6.0 },
    };
    for _ in 0..n_cases(4000) {
        let r = rng.radius();
        for &(x, y) in &[
            (-10.0f32, -10.0f32), (0.5, -10.0), (10.0, -10.0),
            (-10.0, 2.0),         (0.5, 2.0),   (10.0, 2.0),
            (-10.0, 12.0),        (0.5, 12.0),  (10.0, 12.0),
            (-3.0, -2.0), (4.0, 6.0), (-3.0, 6.0), (4.0, -2.0), // exact corners
        ] {
            let A = c2Circle { p: c2v { x, y }, r };
            unsafe {
                eq_int("c2CircletoAABB/region", &(A, B), (p.c.c2CircletoAABB)(A, B), (p.r.c2CircletoAABB)(A, B));
            }
        }
        // inverted box
        let inv = c2AABB { min: B.max, max: B.min };
        let A = c2Circle { p: rng.v(), r };
        unsafe {
            eq_int("c2CircletoAABB/inv", &(A, inv), (p.c.c2CircletoAABB)(A, inv), (p.r.c2CircletoAABB)(A, inv));
        }
    }
    for &s in SPECIAL_F32 {
        for &r in SPECIAL_F32 {
            let A = c2Circle { p: c2v { x: s, y: -s }, r };
            let Bx = c2AABB { min: c2v { x: -s, y: s }, max: c2v { x: s, y: -s } };
            unsafe {
                eq_int("c2CircletoAABB/sp", &(A, Bx), (p.c.c2CircletoAABB)(A, Bx), (p.r.c2CircletoAABB)(A, Bx));
            }
        }
    }
}

// ------------------------------------------------------------------- row 63
#[test]
fn row63_c2CircletoCapsule() {
    let p = apis();
    let mut rng = Rng::new(0x6300);
    let mut branches = [0usize; 3];
    for _ in 0..n_cases(N) {
        let A = rng.circle();
        let B = rng.capsule();
        branches[classify(&A, &B)] += 1;
        unsafe {
            eq_int("c2CircletoCapsule", &(A, B), (p.c.c2CircletoCapsule)(A, B), (p.r.c2CircletoCapsule)(A, B));
        }
    }
    assert!(branches.iter().all(|&c| c > 0), "branches: {branches:?}");

    // deliberately place the circle so each of the three branches is hit
    for _ in 0..n_cases(4000) {
        let a = rng.v();
        let n = rng.v();
        let B = c2Capsule { a, b: c2v { x: a.x + n.x, y: a.y + n.y }, r: rng.radius() };
        let r = rng.radius();
        for &t in &[-2.0f32, -0.001, 0.0, 0.5, 1.0, 1.001, 3.0] {
            let perp = rng.f32_in(-5.0, 5.0);
            let A = c2Circle {
                p: c2v {
                    x: a.x + n.x * t - n.y * perp,
                    y: a.y + n.y * t + n.x * perp,
                },
                r,
            };
            unsafe {
                eq_int("c2CircletoCapsule/t", &(A, B, t), (p.c.c2CircletoCapsule)(A, B), (p.r.c2CircletoCapsule)(A, B));
            }
        }
        // degenerate capsule (a == b): dot(n,n) == 0
        let Bd = c2Capsule { a, b: a, r: rng.radius() };
        let A = c2Circle { p: rng.v(), r };
        unsafe {
            eq_int("c2CircletoCapsule/degen", &(A, Bd), (p.c.c2CircletoCapsule)(A, Bd), (p.r.c2CircletoCapsule)(A, Bd));
            // circle centre exactly on the degenerate capsule
            let A2 = c2Circle { p: a, r };
            eq_int("c2CircletoCapsule/degen2", &(A2, Bd), (p.c.c2CircletoCapsule)(A2, Bd), (p.r.c2CircletoCapsule)(A2, Bd));
        }
    }
    for &s in SPECIAL_F32 {
        for &r in SPECIAL_F32 {
            let A = c2Circle { p: c2v { x: s, y: -s }, r };
            let B = c2Capsule { a: c2v { x: -s, y: s }, b: c2v { x: r, y: -r }, r: s };
            unsafe {
                eq_int("c2CircletoCapsule/sp", &(A, B), (p.c.c2CircletoCapsule)(A, B), (p.r.c2CircletoCapsule)(A, B));
            }
        }
    }
}

/// Mirrors the branch ladder of `c2CircletoCapsule` (lib.c:555) for coverage.
fn classify(A: &c2Circle, B: &c2Capsule) -> usize {
    let n = (B.b.x - B.a.x, B.b.y - B.a.y);
    let ap = (A.p.x - B.a.x, A.p.y - B.a.y);
    let da = ap.0 * n.0 + ap.1 * n.1;
    if da < 0.0 {
        return 0;
    }
    let bp = (A.p.x - B.b.x, A.p.y - B.b.y);
    let db = bp.0 * n.0 + bp.1 * n.1;
    if db < 0.0 {
        1
    } else {
        2
    }
}

// --------------------------------------------------------------- rows 64, 65
#[test]
fn row64_c2AABBtoCapsule() {
    let p = apis();
    let mut rng = Rng::new(0x6400);
    for _ in 0..n_cases(N / 4) {
        let A = rng.aabb();
        let B = rng.capsule();
        unsafe {
            eq_int("c2AABBtoCapsule", &(A, B), (p.c.c2AABBtoCapsule)(A, B), (p.r.c2AABBtoCapsule)(A, B));
        }
    }
    for &s in SPECIAL_F32 {
        for &r in SPECIAL_F32 {
            let A = c2AABB { min: c2v { x: s, y: r }, max: c2v { x: r, y: s } };
            let B = c2Capsule { a: c2v { x: -s, y: r }, b: c2v { x: r, y: -s }, r };
            unsafe {
                eq_int("c2AABBtoCapsule/sp", &(A, B), (p.c.c2AABBtoCapsule)(A, B), (p.r.c2AABBtoCapsule)(A, B));
            }
        }
    }
}

#[test]
fn row65_c2CapsuletoCapsule() {
    let p = apis();
    let mut rng = Rng::new(0x6500);
    for _ in 0..n_cases(N / 4) {
        let A = rng.capsule();
        let B = rng.capsule();
        unsafe {
            eq_int("c2CapsuletoCapsule", &(A, B), (p.c.c2CapsuletoCapsule)(A, B), (p.r.c2CapsuletoCapsule)(A, B));
            eq_int("c2CapsuletoCapsule/self", &A, (p.c.c2CapsuletoCapsule)(A, A), (p.r.c2CapsuletoCapsule)(A, A));
        }
    }
    // parallel / crossing / coincident / zero-length
    for _ in 0..n_cases(3000) {
        let a = rng.v();
        let d = rng.v();
        let off = rng.v();
        let A = c2Capsule { a, b: c2v { x: a.x + d.x, y: a.y + d.y }, r: rng.radius() };
        let par = c2Capsule {
            a: c2v { x: a.x + off.x, y: a.y + off.y },
            b: c2v { x: a.x + off.x + d.x, y: a.y + off.y + d.y },
            r: rng.radius(),
        };
        let cross = c2Capsule {
            a: c2v { x: a.x - d.y, y: a.y + d.x },
            b: c2v { x: a.x + d.y, y: a.y - d.x },
            r: rng.radius(),
        };
        let zero = c2Capsule { a, b: a, r: rng.radius() };
        for B in [par, cross, zero, A] {
            unsafe {
                eq_int("c2CapsuletoCapsule/struct", &(A, B), (p.c.c2CapsuletoCapsule)(A, B), (p.r.c2CapsuletoCapsule)(A, B));
            }
        }
    }
    for &s in SPECIAL_F32 {
        for &r in SPECIAL_F32 {
            let A = c2Capsule { a: c2v { x: s, y: r }, b: c2v { x: r, y: s }, r: s };
            let B = c2Capsule { a: c2v { x: -s, y: -r }, b: c2v { x: -r, y: -s }, r };
            unsafe {
                eq_int("c2CapsuletoCapsule/sp", &(A, B), (p.c.c2CapsuletoCapsule)(A, B), (p.r.c2CapsuletoCapsule)(A, B));
            }
        }
    }
}

// --------------------------------------------------------------- rows 66, 67
#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct Buf([u8; 32]);

fn buf_of<T: Copy>(v: &T) -> Buf {
    let mut b = Buf([0u8; 32]);
    unsafe {
        std::ptr::copy_nonoverlapping(
            v as *const T as *const u8,
            b.0.as_mut_ptr(),
            std::mem::size_of::<T>(),
        );
    }
    b
}

fn rand_buf(rng: &mut Rng, ty: c_int) -> (Buf, String) {
    match ty {
        C2_TYPE_CIRCLE => {
            let c = rng.circle();
            (buf_of(&c), format!("{c:?}"))
        }
        C2_TYPE_AABB => {
            let c = rng.aabb();
            (buf_of(&c), format!("{c:?}"))
        }
        _ => {
            let c = rng.capsule();
            (buf_of(&c), format!("{c:?}"))
        }
    }
}

#[test]
fn row66_row67_c2Collided_all_nine_pairs() {
    let p = apis();
    let mut rng = Rng::new(0x6600);
    let tys = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
    for &ta in &tys {
        for &tb in &tys {
            for _ in 0..n_cases(3000) {
                let (a, sa) = rand_buf(&mut rng, ta);
                let (b, sb) = rand_buf(&mut rng, tb);
                unsafe {
                    let cv = (p.c.c2Collided)(
                        a.0.as_ptr() as *const c_void,
                        ta,
                        b.0.as_ptr() as *const c_void,
                        tb,
                    );
                    let rv = (p.r.c2Collided)(
                        a.0.as_ptr() as *const c_void,
                        ta,
                        b.0.as_ptr() as *const c_void,
                        tb,
                    );
                    eq_int("c2Collided", &(ta, tb, &sa, &sb), cv, rv);
                }
            }
        }
    }
    // Row 67: the (AABB,CIRCLE) and (CAPSULE,AABB) / (CAPSULE,CIRCLE) cases
    // swap their arguments in the C; feed matching-and-mismatching shapes so a
    // swap error would change the answer.
    for _ in 0..n_cases(6000) {
        let ci = rng.circle();
        let bb = rng.aabb();
        let ca = rng.capsule();
        let (cb, ab, kb) = (buf_of(&ci), buf_of(&bb), buf_of(&ca));
        let cases: [(&Buf, c_int, &Buf, c_int); 6] = [
            (&cb, C2_TYPE_CIRCLE, &ab, C2_TYPE_AABB),
            (&ab, C2_TYPE_AABB, &cb, C2_TYPE_CIRCLE),
            (&cb, C2_TYPE_CIRCLE, &kb, C2_TYPE_CAPSULE),
            (&kb, C2_TYPE_CAPSULE, &cb, C2_TYPE_CIRCLE),
            (&ab, C2_TYPE_AABB, &kb, C2_TYPE_CAPSULE),
            (&kb, C2_TYPE_CAPSULE, &ab, C2_TYPE_AABB),
        ];
        for (x, tx, y, ty) in cases {
            unsafe {
                let cv = (p.c.c2Collided)(
                    x.0.as_ptr() as *const c_void,
                    tx,
                    y.0.as_ptr() as *const c_void,
                    ty,
                );
                let rv = (p.r.c2Collided)(
                    x.0.as_ptr() as *const c_void,
                    tx,
                    y.0.as_ptr() as *const c_void,
                    ty,
                );
                eq_int("c2Collided/swap", &(tx, ty, ci, bb, ca), cv, rv);
            }
        }
    }
}

// ------------------------------------------------------------------- row 68
#[test]
fn row68_capsule_driver() {
    let p = apis();
    let mut rng = Rng::new(0x6800);
    let mut mask_hits = [0usize; 8];
    for _ in 0..n_cases(200_000) {
        let (a, b, c, d, e) = (
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
        );
        unsafe {
            let cv = (p.c.capsule)(a, b, c, d, e);
            let rv = (p.r.capsule)(a, b, c, d, e);
            eq_int("capsule", &(a, b, c, d, e), cv, rv);
            if (0..8).contains(&cv) {
                mask_hits[cv as usize] += 1;
            }
        }
    }
    // A deterministic grid over the region the three hard-coded shapes occupy,
    // so every reachable result bit-mask is provoked.
    let mut i = 0;
    while i < 40 {
        let mut j = 0;
        while j < 40 {
            let min_x = -90.0 + (i as f32) * 4.0;
            let min_y = -60.0 + (j as f32) * 5.0;
            for &(dx, dy) in &[(0.0f32, 0.0f32), (20.0, 0.0), (0.0, 30.0), (25.0, 45.0), (-30.0, -20.0)] {
                for &r in &[0.0f32, 1.0, 5.0, 12.0, 25.0, 60.0] {
                    unsafe {
                        let cv = (p.c.capsule)(min_x, min_y, min_x + dx, min_y + dy, r);
                        let rv = (p.r.capsule)(min_x, min_y, min_x + dx, min_y + dy, r);
                        eq_int("capsule/grid", &(min_x, min_y, dx, dy, r), cv, rv);
                        if (0..8).contains(&cv) {
                            mask_hits[cv as usize] += 1;
                        }
                    }
                }
            }
            j += 1;
        }
        i += 1;
    }
    // special float arguments
    for &a in SPECIAL_F32 {
        for &b in SPECIAL_F32 {
            for &r in &[0.0f32, 10.0, f32::NAN, f32::INFINITY, -1.0] {
                unsafe {
                    let cv = (p.c.capsule)(a, b, b, a, r);
                    let rv = (p.r.capsule)(a, b, b, a, r);
                    eq_int("capsule/sp", &(a, b, r), cv, rv);
                }
            }
        }
    }
    let reached = mask_hits.iter().filter(|&&c| c > 0).count();
    assert!(
        reached >= 5,
        "only {reached} distinct capsule() masks reached: {mask_hits:?}"
    );
}
