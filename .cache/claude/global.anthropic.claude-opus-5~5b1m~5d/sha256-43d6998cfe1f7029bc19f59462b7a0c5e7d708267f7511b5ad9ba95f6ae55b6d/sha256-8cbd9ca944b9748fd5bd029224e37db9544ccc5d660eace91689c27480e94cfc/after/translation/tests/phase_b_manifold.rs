//! Phase B — manifold + dispatch differential tests (CONFIGS.md rows 38..57).

#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_int, c_void};

type CircCirc = unsafe extern "C" fn(c2Circle, c2Circle, *mut c2Manifold);
type CircAabb = unsafe extern "C" fn(c2Circle, c2AABB, *mut c2Manifold);
type CircCap = unsafe extern "C" fn(c2Circle, c2Capsule, *mut c2Manifold);
type AabbAabb = unsafe extern "C" fn(c2AABB, c2AABB, *mut c2Manifold);
type AabbCap = unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold);
type CapCap = unsafe extern "C" fn(c2Capsule, c2Capsule, *mut c2Manifold);
type CapPoly = unsafe extern "C" fn(c2Capsule, *const c2Poly, *const c2x, *mut c2Manifold);
type Collide = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int, *mut c2Manifold);
type Omni = unsafe extern "C" fn(
    *mut c2Manifold,
    c_int,
    f32,
    f32,
    f32,
    f32,
    f32,
    c_int,
    f32,
    f32,
    f32,
    f32,
    f32,
);

/// Distinctive pre-fill so that "untouched" manifold fields are observable.
fn seed_manifold() -> c2Manifold {
    c2Manifold {
        count: -13,
        depths: [7.5, -8.25],
        contact_points: [c2v { x: 11.0, y: 12.0 }, c2v { x: 13.0, y: 14.0 }],
        n: c2v { x: 0.25, y: -0.75 },
    }
}

/// Lazy: the context string is only built when the manifolds differ, so the
/// happy path performs no formatting at all. (Eagerly building a `format!`
/// argument on every iteration left large amounts of debris on the stack, which
/// then fed the C library's uninitialised-`c2Proxy` read — see ERRORS.md row 53.)
#[inline(never)]
fn check(label: &str, mc: &c2Manifold, mr: &c2Manifold, ctx: impl FnOnce() -> String) {
    if !man_eq(mc, mr) {
        panic!(
            "{}: {}\n  C: {}\n  R: {}",
            label,
            ctx(),
            fmt_man(mc),
            fmt_man(mr)
        );
    }
}

fn rand_circle(rng: &mut Rng, spread: f32) -> c2Circle {
    c2Circle {
        p: c2v { x: rng.range(-spread, spread), y: rng.range(-spread, spread) },
        r: rng.range(0.0, 3.0),
    }
}
fn rand_aabb(rng: &mut Rng, spread: f32) -> c2AABB {
    let cx = rng.range(-spread, spread);
    let cy = rng.range(-spread, spread);
    let ex = rng.range(0.0, 3.0);
    let ey = rng.range(0.0, 3.0);
    c2AABB { min: c2v { x: cx - ex, y: cy - ey }, max: c2v { x: cx + ex, y: cy + ey } }
}
fn rand_capsule(rng: &mut Rng, spread: f32) -> c2Capsule {
    c2Capsule {
        a: c2v { x: rng.range(-spread, spread), y: rng.range(-spread, spread) },
        b: c2v { x: rng.range(-spread, spread), y: rng.range(-spread, spread) },
        r: rng.range(0.0, 3.0),
    }
}

/// Regular CCW n-gon, matching the winding `c2Norms` expects.
fn ngon(rng: &mut Rng, n: c_int, radius: f32, phase: f32, center: c2v) -> c2Poly {
    let p = pair();
    let norms_c = p.c.get::<unsafe extern "C" fn(*mut c2v, *mut c2v, c_int)>("c2Norms");
    let _ = rng;
    let mut poly = c2Poly::default();
    poly.count = n;
    for i in 0..n {
        // CCW ordering
        let t = phase + 2.0 * std::f32::consts::PI * (i as f32) / (n as f32);
        poly.verts[i as usize] = c2v {
            x: center.x + radius * t.cos(),
            y: center.y + radius * t.sin(),
        };
    }
    scrub_stack();
    unsafe {
        norms_c(poly.verts.as_mut_ptr(), poly.norms.as_mut_ptr(), n);
    }
    poly
}

// --- row 38 ---------------------------------------------------------------

#[test]
fn row38_circle_circle() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<CircCirc>("c2CircletoCircleManifold"), p.r.get::<CircCirc>("c2CircletoCircleManifold"));
        let mut rng = Rng::new(0x38);
        let mut cases: Vec<(c2Circle, c2Circle)> = Vec::new();
        for _ in 0..3000 {
            let spread = [0.3f32, 1.0, 3.0, 8.0][rng.below(4) as usize];
            cases.push((rand_circle(&mut rng, spread), rand_circle(&mut rng, spread)));
        }
        // concentric (l == 0) and exact-touch
        cases.push((c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: 2.0 }, c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: 3.0 }));
        cases.push((c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }, c2Circle { p: c2v { x: 3.0, y: 0.0 }, r: 2.0 }));
        cases.push((c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 }, c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 }));
        for (a, b) in cases {
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            scrub_stack();
            unsafe {
                f_c(a, b, &mut mc);
                f_r(a, b, &mut mr);
            }
            check("row38", &mc, &mr, || format!("A={:?} B={:?}", a, b));
        }
    });
}

// --- rows 39 / 40 ---------------------------------------------------------

#[test]
fn row39_row40_circle_aabb() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<CircAabb>("c2CircletoAABBManifold"), p.r.get::<CircAabb>("c2CircletoAABBManifold"));
        let mut rng = Rng::new(0x39);
        let mut cases: Vec<(c2Circle, c2AABB)> = Vec::new();
        for _ in 0..4000 {
            let spread = [0.3f32, 1.0, 3.0, 8.0][rng.below(4) as usize];
            cases.push((rand_circle(&mut rng, spread), rand_aabb(&mut rng, spread)));
        }
        // center exactly inside (d2 == 0) with all 4 quadrants, both overlap orders
        let bb = c2AABB { min: c2v { x: -2.0, y: -1.0 }, max: c2v { x: 2.0, y: 1.0 } };
        for &(x, y) in &[
            (0.5f32, 0.25f32), (-0.5, 0.25), (0.5, -0.25), (-0.5, -0.25),
            (0.0, 0.0), (1.9, 0.0), (0.0, 0.9), (-1.9, -0.9),
        ] {
            cases.push((c2Circle { p: c2v { x, y }, r: 1.0 }, bb));
        }
        // wide vs tall box -> x_overlap < y_overlap and vice versa
        cases.push((c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }, c2AABB { min: c2v { x: -10.0, y: -0.5 }, max: c2v { x: 10.0, y: 0.5 } }));
        cases.push((c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }, c2AABB { min: c2v { x: -0.5, y: -10.0 }, max: c2v { x: 0.5, y: 10.0 } }));
        // row 40: degenerate + inverted AABB
        cases.push((c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }, c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } }));
        cases.push((c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 }, c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: -1.0, y: -1.0 } }));
        cases.push((c2Circle { p: c2v { x: 0.5, y: 0.5 }, r: 2.0 }, c2AABB { min: c2v { x: 1.0, y: -1.0 }, max: c2v { x: -1.0, y: 1.0 } }));
        for (a, b) in cases {
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            scrub_stack();
            unsafe {
                f_c(a, b, &mut mc);
                f_r(a, b, &mut mr);
            }
            check("row39/40", &mc, &mr, || format!("A={:?} B={:?}", a, b));
        }
    });
}

// --- row 41 ---------------------------------------------------------------

#[test]
fn row41_circle_capsule() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<CircCap>("c2CircletoCapsuleManifold"), p.r.get::<CircCap>("c2CircletoCapsuleManifold"));
        let mut rng = Rng::new(0x41);
        let mut cases: Vec<(c2Circle, c2Capsule)> = Vec::new();
        for _ in 0..4000 {
            let spread = [0.3f32, 1.0, 3.0, 8.0][rng.below(4) as usize];
            cases.push((rand_circle(&mut rng, spread), rand_capsule(&mut rng, spread)));
        }
        // circle center exactly on the capsule segment -> d == 0
        cases.push((
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: -2.0, y: 0.0 }, b: c2v { x: 2.0, y: 0.0 }, r: 0.5 },
        ));
        // degenerate (zero-length) capsule -> NaN normal on the d == 0 branch
        cases.push((
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 0.5 },
        ));
        cases.push((
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 5.0, y: 5.0 }, b: c2v { x: 5.0, y: 5.0 }, r: 0.5 },
        ));
        for (a, b) in cases {
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            scrub_stack();
            unsafe {
                f_c(a, b, &mut mc);
                f_r(a, b, &mut mr);
            }
            check("row41", &mc, &mr, || format!("A={:?} B={:?}", a, b));
        }
    });
}

// --- row 42 ---------------------------------------------------------------

#[test]
fn row42_aabb_aabb() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<AabbAabb>("c2AABBtoAABBManifold"), p.r.get::<AabbAabb>("c2AABBtoAABBManifold"));
        let mut rng = Rng::new(0x42);
        let mut cases: Vec<(c2AABB, c2AABB)> = Vec::new();
        for _ in 0..5000 {
            let spread = [0.3f32, 1.0, 3.0, 8.0][rng.below(4) as usize];
            cases.push((rand_aabb(&mut rng, spread), rand_aabb(&mut rng, spread)));
        }
        let unit = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
        // identical, exact-touch on each side, dx==dy tie
        cases.push((unit, unit));
        for &(dx, dy) in &[(2.0f32, 0.0f32), (-2.0, 0.0), (0.0, 2.0), (0.0, -2.0), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (3.0, 0.0), (0.0, 3.0)] {
            cases.push((unit, c2AABB { min: c2v { x: -1.0 + dx, y: -1.0 + dy }, max: c2v { x: 1.0 + dx, y: 1.0 + dy } }));
        }
        // inverted boxes
        cases.push((c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: -1.0, y: -1.0 } }, unit));
        cases.push((unit, c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: -1.0, y: -1.0 } }));
        // degenerate
        cases.push((c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } }, unit));
        for (a, b) in cases {
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            scrub_stack();
            unsafe {
                f_c(a, b, &mut mc);
                f_r(a, b, &mut mr);
            }
            check("row42", &mc, &mr, || format!("A={:?} B={:?}", a, b));
        }
    });
}

// --- row 43 ---------------------------------------------------------------

#[test]
fn row43_capsule_capsule() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<CapCap>("c2CapsuletoCapsuleManifold"), p.r.get::<CapCap>("c2CapsuletoCapsuleManifold"));
        let mut rng = Rng::new(0x43);
        let mut cases: Vec<(c2Capsule, c2Capsule)> = Vec::new();
        for _ in 0..4000 {
            let spread = [0.3f32, 1.0, 3.0, 8.0][rng.below(4) as usize];
            cases.push((rand_capsule(&mut rng, spread), rand_capsule(&mut rng, spread)));
        }
        let h = c2Capsule { a: c2v { x: -2.0, y: 0.0 }, b: c2v { x: 2.0, y: 0.0 }, r: 0.5 };
        // crossing (d == 0)
        cases.push((h, c2Capsule { a: c2v { x: 0.0, y: -2.0 }, b: c2v { x: 0.0, y: 2.0 }, r: 0.5 }));
        // parallel overlap
        cases.push((h, c2Capsule { a: c2v { x: -2.0, y: 0.4 }, b: c2v { x: 2.0, y: 0.4 }, r: 0.5 }));
        // collinear
        cases.push((h, c2Capsule { a: c2v { x: 2.0, y: 0.0 }, b: c2v { x: 6.0, y: 0.0 }, r: 0.5 }));
        // zero-length A / B
        cases.push((c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 1.0 }, h));
        cases.push((h, c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 1.0 }));
        cases.push((
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
        ));
        for (a, b) in cases {
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            scrub_stack();
            unsafe {
                f_c(a, b, &mut mc);
                f_r(a, b, &mut mr);
            }
            check("row43", &mc, &mr, || format!("A={:?} B={:?}", a, b));
        }
    });
}

// --- rows 44..51 : capsule vs poly ----------------------------------------

fn diff_cap_poly(label: &str, a: c2Capsule, poly: &c2Poly, bx: Option<c2x>) {
    let p = pair();
    let f_c = p.c.get::<CapPoly>("c2CapsuletoPolyManifold");
    let f_r = p.r.get::<CapPoly>("c2CapsuletoPolyManifold");
    let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let mut mc = seed_manifold();
    let mut mr = seed_manifold();
    with_clean_stack(|| unsafe { f_c(a, poly, bxp, &mut mc) });
    with_clean_stack(|| unsafe { f_r(a, poly, bxp, &mut mr) });
    check(
        label,
        &mc,
        &mr,
        || format!("A={:?} poly.count={} verts={:?} bx={:?}", a, poly.count, &poly.verts[..poly.count.max(0) as usize], bx),
    );
}

#[test]
fn row44_row50_capsule_poly() {
    fresh(|| {
        let mut rng = Rng::new(0x44);
        for _ in 0..3000 {
            let n = 3 + rng.below(6) as c_int; // 3..8
            let radius = rng.range(0.5, 4.0);
            let phase = rng.range(-3.2, 3.2);
            let center = c2v { x: rng.range(-2.0, 2.0), y: rng.range(-2.0, 2.0) };
            let poly = ngon(&mut rng, n, radius, phase, center);
            let spread = [0.5f32, 2.0, 5.0, 10.0][rng.below(4) as usize];
            let a = rand_capsule(&mut rng, spread);
            // row 47: NULL / identity / rotated / translated bx
            let bxs: [Option<c2x>; 4] = [
                None,
                Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } }),
                Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: rng.rot() }),
                Some(rng.xform()),
            ];
            for bx in bxs {
                diff_cap_poly("row44-50", a, &poly, bx);
            }
        }
    });
}

#[test]
fn row48_poly_counts() {
    fresh(|| {
        let mut rng = Rng::new(0x48);
        for n in 3..=8i32 {
            for _ in 0..600 {
                let rad = rng.range(0.5, 3.0);
                let ph = rng.range(-3.2, 3.2);
                let poly = ngon(&mut rng, n, rad, ph, c2v { x: 0.0, y: 0.0 });
                let a = rand_capsule(&mut rng, 4.0);
                diff_cap_poly("row48", a, &poly, None);
                diff_cap_poly("row48-bx", a, &poly, Some(rng.xform()));
            }
        }
    });
}

#[test]
fn row49_row51_capsule_poly_edge_cases() {
    fresh(|| {
        let mut rng = Rng::new(0x49);
        let quad = ngon(&mut rng, 4, 1.0, 0.7853982, c2v { x: 0.0, y: 0.0 });
        // row 49: far away
        diff_cap_poly("row49-far", c2Capsule { a: c2v { x: 50.0, y: 50.0 }, b: c2v { x: 60.0, y: 50.0 }, r: 0.5 }, &quad, None);
        // row 50: shallow (1e-6 <= d < A.r)
        for t in [1.05f32, 1.2, 1.5, 1.9] {
            diff_cap_poly(
                "row50-shallow",
                c2Capsule { a: c2v { x: t, y: -1.0 }, b: c2v { x: t, y: 1.0 }, r: 2.0 },
                &quad,
                None,
            );
        }
        // row 51: degenerate (zero-length) capsule
        for pos in [c2v { x: 0.0, y: 0.0 }, c2v { x: 0.5, y: 0.5 }, c2v { x: 5.0, y: 5.0 }] {
            diff_cap_poly("row51-degen", c2Capsule { a: pos, b: pos, r: 1.0 }, &quad, None);
            diff_cap_poly("row51-degen-bx", c2Capsule { a: pos, b: pos, r: 1.0 }, &quad, Some(rng.xform()));
        }
        // deliberately deep overlap so `code == 0` with each face as reference
        for k in 0..4 {
            let t = 0.7853982 + std::f32::consts::FRAC_PI_2 * k as f32;
            let d = c2v { x: t.cos(), y: t.sin() };
            diff_cap_poly(
                "row44-code0",
                c2Capsule {
                    a: c2v { x: d.x * 0.2, y: d.y * 0.2 },
                    b: c2v { x: -d.y * 0.6, y: d.x * 0.6 },
                    r: 0.3,
                },
                &quad,
                None,
            );
        }
    });
}

// --- rows 52 / 53 : aabb vs capsule ---------------------------------------

#[test]
fn row52_row53_aabb_capsule() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<AabbCap>("c2AABBtoCapsuleManifold"), p.r.get::<AabbCap>("c2AABBtoCapsuleManifold"));
        let mut rng = Rng::new(0x52);
        let mut cases: Vec<(c2AABB, c2Capsule)> = Vec::new();
        for _ in 0..4000 {
            let spread = [0.3f32, 1.0, 3.0, 8.0][rng.below(4) as usize];
            cases.push((rand_aabb(&mut rng, spread), rand_capsule(&mut rng, spread)));
        }
        let unit = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
        cases.push((unit, c2Capsule { a: c2v { x: -3.0, y: 0.0 }, b: c2v { x: 3.0, y: 0.0 }, r: 0.5 }));
        cases.push((unit, c2Capsule { a: c2v { x: 0.0, y: -3.0 }, b: c2v { x: 0.0, y: 3.0 }, r: 0.5 }));
        cases.push((unit, c2Capsule { a: c2v { x: 10.0, y: 10.0 }, b: c2v { x: 11.0, y: 11.0 }, r: 0.5 }));
        cases.push((unit, c2Capsule { a: c2v { x: 1.05, y: -1.0 }, b: c2v { x: 1.05, y: 1.0 }, r: 2.0 }));
        // row 53: degenerate AABBs -> NaN polygon normals
        let zero = c2v { x: 0.0, y: 0.0 };
        cases.push((c2AABB { min: zero, max: zero }, c2Capsule { a: c2v { x: -1.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.5 }));
        cases.push((c2AABB { min: c2v { x: 0.0, y: -1.0 }, max: c2v { x: 0.0, y: 1.0 } }, c2Capsule { a: c2v { x: -1.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.5 }));
        cases.push((c2AABB { min: c2v { x: -1.0, y: 0.0 }, max: c2v { x: 1.0, y: 0.0 } }, c2Capsule { a: c2v { x: 0.0, y: -1.0 }, b: c2v { x: 0.0, y: 1.0 }, r: 0.5 }));
        cases.push((c2AABB { min: zero, max: zero }, c2Capsule { a: zero, b: zero, r: 1.0 }));
        for (a, b) in cases {
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            with_clean_stack(|| unsafe { f_c(a, b, &mut mc) });
            with_clean_stack(|| unsafe { f_r(a, b, &mut mr) });
            check("row52/53", &mc, &mr, || format!("A={:?} B={:?}", a, b));
        }
    });
}

// --- row 54 : c2Collide ---------------------------------------------------

#[test]
fn row54_collide_cross_product() {
    fresh(|| {
        let p = pair();
        let (f_c, f_r) = (p.c.get::<Collide>("c2Collide"), p.r.get::<Collide>("c2Collide"));
        let mut rng = Rng::new(0x54);
        for _ in 0..6000 {
            let spread = [0.3f32, 1.0, 3.0, 8.0][rng.below(4) as usize];
            let circ = rand_circle(&mut rng, spread);
            let bb = rand_aabb(&mut rng, spread);
            let cap = rand_capsule(&mut rng, spread);
            let shapes: [(c_int, *const c_void); 3] = [
                (C2_TYPE_CIRCLE, &circ as *const _ as *const c_void),
                (C2_TYPE_AABB, &bb as *const _ as *const c_void),
                (C2_TYPE_CAPSULE, &cap as *const _ as *const c_void),
            ];
            for &(ta, pa) in &shapes {
                for &(tb, pb) in &shapes {
                    let mut mc = seed_manifold();
                    let mut mr = seed_manifold();
                    with_clean_stack(|| unsafe { f_c(pa, ta, pb, tb, &mut mc) });
                    with_clean_stack(|| unsafe { f_r(pa, ta, pb, tb, &mut mr) });
                    check(
                        "row54",
                        &mc,
                        &mr,
                        || format!("ta={} tb={} circ={:?} bb={:?} cap={:?}", ta, tb, circ, bb, cap),
                    );
                }
            }
        }
    });
}

// --- rows 55 / 56 / 57 : omni_manifold ------------------------------------

fn diff_omni(label: &str, ta: c_int, a: [f32; 5], tb: c_int, b: [f32; 5]) {
    let p = pair();
    let f_c = p.c.get::<Omni>("omni_manifold");
    let f_r = p.r.get::<Omni>("omni_manifold");
    let mut mc = seed_manifold();
    let mut mr = seed_manifold();
    with_clean_stack(|| unsafe {
        f_c(&mut mc, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4])
    });
    with_clean_stack(|| unsafe {
        f_r(&mut mr, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4])
    });
    check(label, &mc, &mr, || format!("ta={} a={:?} tb={} b={:?}", ta, a, tb, b));
}

fn parts(rng: &mut Rng, ty: c_int, spread: f32) -> [f32; 5] {
    match ty {
        C2_TYPE_CIRCLE => [rng.range(-spread, spread), rng.range(-spread, spread), rng.range(0.0, 3.0), rng.nice(), rng.nice()],
        C2_TYPE_AABB => {
            let cx = rng.range(-spread, spread);
            let cy = rng.range(-spread, spread);
            let ex = rng.range(0.0, 3.0);
            let ey = rng.range(0.0, 3.0);
            [cx - ex, cy - ey, cx + ex, cy + ey, rng.nice()]
        }
        _ => [
            rng.range(-spread, spread), rng.range(-spread, spread),
            rng.range(-spread, spread), rng.range(-spread, spread),
            rng.range(0.0, 3.0),
        ],
    }
}

#[test]
fn row55_omni_cross_product() {
    fresh(|| {
        let mut rng = Rng::new(0x55);
        let tys = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
        for _ in 0..2500 {
            let spread = [0.3f32, 1.0, 3.0, 8.0][rng.below(4) as usize];
            for &ta in &tys {
                for &tb in &tys {
                    let a = parts(&mut rng, ta, spread);
                    let b = parts(&mut rng, tb, spread);
                    diff_omni("row55", ta, a, tb, b);
                }
            }
        }
    });
}

#[test]
fn row56_omni_separation_sweep() {
    fresh(|| {
        let tys = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
        let base = |ty: c_int, off: f32| -> [f32; 5] {
            match ty {
                C2_TYPE_CIRCLE => [off, 0.0, 1.0, 0.0, 0.0],
                C2_TYPE_AABB => [off - 1.0, -1.0, off + 1.0, 1.0, 0.0],
                _ => [off - 1.0, -0.5, off + 1.0, 0.5, 0.5],
            }
        };
        for &ta in &tys {
            for &tb in &tys {
                let mut k = -400i32;
                while k <= 400 {
                    let off = k as f32 * 0.02;
                    diff_omni("row56", ta, base(ta, 0.0), tb, base(tb, off));
                    // also sweep diagonally
                    let mut bp = base(tb, off);
                    match tb {
                        C2_TYPE_CIRCLE => bp[1] = off * 0.5,
                        C2_TYPE_AABB => { bp[1] += off * 0.5; bp[3] += off * 0.5; }
                        _ => { bp[1] += off * 0.5; bp[3] += off * 0.5; }
                    }
                    diff_omni("row56-diag", ta, base(ta, 0.0), tb, bp);
                    k += 1;
                }
            }
        }
    });
}

#[test]
fn row57_omni_degenerate_parts() {
    fresh(|| {
        let tys = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
        // zero radius, zero-length capsule, zero-area AABB, inverted AABB, negative radius
        let degen: &[(c_int, [f32; 5])] = &[
            (C2_TYPE_CIRCLE, [0.0, 0.0, 0.0, 0.0, 0.0]),
            (C2_TYPE_CIRCLE, [0.0, 0.0, -1.0, 0.0, 0.0]),
            (C2_TYPE_CIRCLE, [1.0, 1.0, 0.0, 0.0, 0.0]),
            (C2_TYPE_AABB, [0.0, 0.0, 0.0, 0.0, 0.0]),
            (C2_TYPE_AABB, [1.0, 1.0, -1.0, -1.0, 0.0]),
            (C2_TYPE_AABB, [0.0, -1.0, 0.0, 1.0, 0.0]),
            (C2_TYPE_AABB, [-1.0, 0.0, 1.0, 0.0, 0.0]),
            (C2_TYPE_CAPSULE, [0.0, 0.0, 0.0, 0.0, 0.0]),
            (C2_TYPE_CAPSULE, [0.0, 0.0, 0.0, 0.0, 1.0]),
            (C2_TYPE_CAPSULE, [0.0, 0.0, 1.0, 0.0, -1.0]),
            (C2_TYPE_CAPSULE, [0.5, 0.5, 0.5, 0.5, 0.5]),
        ];
        let mut rng = Rng::new(0x57);
        for &(ta, a) in degen {
            for &(tb, b) in degen {
                diff_omni("row57", ta, a, tb, b);
            }
            // and against random ordinary shapes
            for &tb in &tys {
                for _ in 0..200 {
                    let b = parts(&mut rng, tb, 2.0);
                    diff_omni("row57-mixed", ta, a, tb, b);
                    diff_omni("row57-mixed-rev", tb, b, ta, a);
                }
            }
        }
    });
}
