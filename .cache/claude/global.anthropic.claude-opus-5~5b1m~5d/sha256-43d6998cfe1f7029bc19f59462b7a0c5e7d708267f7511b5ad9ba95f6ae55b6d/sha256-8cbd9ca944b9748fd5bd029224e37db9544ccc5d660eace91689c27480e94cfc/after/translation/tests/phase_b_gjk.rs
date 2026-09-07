//! Phase B — `c2GJK` differential tests (CONFIGS.md rows 25..37).
//!
//! `c2GJK` is the lowest-level composed entry point: it is driven directly here
//! (not only through the manifold wrappers) with every combination of
//! `use_radius`, transform, out-pointer and cache state.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_int, c_void};

type GjkFn = unsafe extern "C" fn(
    *const c_void, // A
    c_int,         // typeA
    *const c2x,    // ax
    *const c_void, // B
    c_int,         // typeB
    *const c2x,    // bx
    *mut c2v,      // outA
    *mut c2v,      // outB
    c_int,         // use_radius
    *mut c_int,    // iterations
    *mut c2GJKCache,
) -> f32;

fn gjk_fns() -> (GjkFn, GjkFn) {
    let p = pair();
    (p.c.get::<GjkFn>("c2GJK"), p.r.get::<GjkFn>("c2GJK"))
}

/// An owned shape blob + its type tag.
#[derive(Copy, Clone, Debug)]
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn ty(&self) -> c_int {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const _ as *const c_void,
            Shape::Aabb(c) => c as *const _ as *const c_void,
            Shape::Capsule(c) => c as *const _ as *const c_void,
        }
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
    c2AABB {
        min: c2v { x: cx - ex, y: cy - ey },
        max: c2v { x: cx + ex, y: cy + ey },
    }
}

fn rand_capsule(rng: &mut Rng, spread: f32) -> c2Capsule {
    c2Capsule {
        a: c2v { x: rng.range(-spread, spread), y: rng.range(-spread, spread) },
        b: c2v { x: rng.range(-spread, spread), y: rng.range(-spread, spread) },
        r: rng.range(0.0, 3.0),
    }
}

fn rand_shape(rng: &mut Rng, kind: u32, spread: f32) -> Shape {
    match kind {
        0 => Shape::Circle(rand_circle(rng, spread)),
        1 => Shape::Aabb(rand_aabb(rng, spread)),
        _ => Shape::Capsule(rand_capsule(rng, spread)),
    }
}

/// Runs one `c2GJK` configuration on both libraries and asserts bit-equality of
/// the return value, `outA`, `outB`, `iterations` and the mutated cache.
#[allow(clippy::too_many_arguments)]
fn diff_gjk(
    label: &str,
    a: &Shape,
    ax: Option<c2x>,
    b: &Shape,
    bx: Option<c2x>,
    use_radius: c_int,
    want_out: bool,
    cache_in: Option<c2GJKCache>,
) -> (f32, Option<c2GJKCache>) {
    let (f_c, f_r) = gjk_fns();
    let axp = ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);

    let mut oac = c2v { x: 111.0, y: 222.0 };
    let mut obc = c2v { x: 333.0, y: 444.0 };
    let mut oar = oac;
    let mut obr = obc;
    let mut itc: c_int = -99;
    let mut itr: c_int = -99;
    let mut cc = cache_in;
    let mut cr = cache_in;

    scrub_stack();

    let (dc, dr) = unsafe {
        let (pac, pbc, pic) = if want_out {
            (&mut oac as *mut c2v, &mut obc as *mut c2v, &mut itc as *mut c_int)
        } else {
            (std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
        };
        let (par, pbr, pir) = if want_out {
            (&mut oar as *mut c2v, &mut obr as *mut c2v, &mut itr as *mut c_int)
        } else {
            (std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
        };
        let pcc = cc.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut c2GJKCache);
        let pcr = cr.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut c2GJKCache);
        (
            f_c(a.ptr(), a.ty(), axp, b.ptr(), b.ty(), bxp, pac, pbc, use_radius, pic, pcc),
            f_r(a.ptr(), a.ty(), axp, b.ptr(), b.ty(), bxp, par, pbr, use_radius, pir, pcr),
        )
    };

    assert!(
        bits_eq_f32(dc, dr),
        "{}: dist C={} R={} A={:?} B={:?} ax={:?} bx={:?} ur={} cache={:?}",
        label, fmt_f32(dc), fmt_f32(dr), a, b, ax, bx, use_radius, cache_in
    );
    assert!(v_eq(oac, oar), "{}: outA C={} R={} A={:?} B={:?}", label, fmt_v(oac), fmt_v(oar), a, b);
    assert!(v_eq(obc, obr), "{}: outB C={} R={} A={:?} B={:?}", label, fmt_v(obc), fmt_v(obr), a, b);
    assert_eq!(itc, itr, "{}: iterations A={:?} B={:?}", label, a, b);
    match (cc, cr) {
        (Some(x), Some(y)) => assert!(
            cache_eq(&x, &y),
            "{}: cache C={:?} R={:?} A={:?} B={:?}",
            label, x, y, a, b
        ),
        (None, None) => {}
        _ => unreachable!(),
    }
    (dc, cc)
}

// --- rows 25 / 26 : circle vs circle --------------------------------------

#[test]
fn row25_row26_circle_circle() {
    fresh(|| {
        let mut rng = Rng::new(0x6A_0025);
        for _ in 0..1500 {
            // vary the spread so separated / touching / overlapping all occur
            let spread = [0.5f32, 2.0, 6.0][rng.below(3) as usize];
            let a = Shape::Circle(rand_circle(&mut rng, spread));
            let b = Shape::Circle(rand_circle(&mut rng, spread));
            for ur in [0, 1] {
                diff_gjk("cc", &a, None, &b, None, ur, true, None);
            }
        }
        // concentric (hits the midpoint-collapse branch under use_radius)
        let a = Shape::Circle(c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 1.0 });
        let b = Shape::Circle(c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 });
        for ur in [0, 1] {
            diff_gjk("cc-concentric", &a, None, &b, None, ur, true, None);
        }
        // zero radii
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 });
        let b = Shape::Circle(c2Circle { p: c2v { x: 3.0, y: 4.0 }, r: 0.0 });
        for ur in [0, 1] {
            diff_gjk("cc-zero-r", &a, None, &b, None, ur, true, None);
        }
    });
}

// --- rows 27 / 28 / 29 / 30 : all other type pairs ------------------------

#[test]
fn row27_row30_all_type_pairs() {
    fresh(|| {
        let mut rng = Rng::new(0x6A_0027);
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                for _ in 0..700 {
                    let spread = [0.5f32, 2.0, 6.0][rng.below(3) as usize];
                    let a = rand_shape(&mut rng, ka, spread);
                    let b = rand_shape(&mut rng, kb, spread);
                    for ur in [0, 1] {
                        diff_gjk("pairs", &a, None, &b, None, ur, true, None);
                    }
                }
            }
        }
    });
}

#[test]
fn row28_capsule_special_layouts() {
    fresh(|| {
        let mut rng = Rng::new(0x6A_0028);
        for _ in 0..600 {
            let t = rng.range(-3.0, 3.0);
            let d = c2v { x: t.cos(), y: t.sin() };
            let o = c2v { x: rng.range(-3.0, 3.0), y: rng.range(-3.0, 3.0) };
            let l = rng.range(0.0, 4.0);
            let a = Shape::Capsule(c2Capsule {
                a: o,
                b: c2v { x: o.x + d.x * l, y: o.y + d.y * l },
                r: rng.range(0.0, 2.0),
            });
            // parallel
            let off = rng.range(-2.0, 2.0);
            let b_par = Shape::Capsule(c2Capsule {
                a: c2v { x: o.x - d.y * off, y: o.y + d.x * off },
                b: c2v { x: o.x + d.x * l - d.y * off, y: o.y + d.y * l + d.x * off },
                r: rng.range(0.0, 2.0),
            });
            // perpendicular / crossing
            let mid = c2v { x: o.x + d.x * l * 0.5, y: o.y + d.y * l * 0.5 };
            let b_cross = Shape::Capsule(c2Capsule {
                a: c2v { x: mid.x + d.y * 2.0, y: mid.y - d.x * 2.0 },
                b: c2v { x: mid.x - d.y * 2.0, y: mid.y + d.x * 2.0 },
                r: rng.range(0.0, 2.0),
            });
            // collinear
            let b_col = Shape::Capsule(c2Capsule {
                a: c2v { x: o.x + d.x * l, y: o.y + d.y * l },
                b: c2v { x: o.x + d.x * l * 2.0, y: o.y + d.y * l * 2.0 },
                r: rng.range(0.0, 2.0),
            });
            for b in [b_par, b_cross, b_col] {
                for ur in [0, 1] {
                    diff_gjk("cap-layout", &a, None, &b, None, ur, true, None);
                }
            }
        }
    });
}

// --- rows 31 / 32 : transforms -------------------------------------------

#[test]
fn row31_row32_transforms() {
    fresh(|| {
        let mut rng = Rng::new(0x6A_0031);
        let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                for _ in 0..300 {
                    let a = rand_shape(&mut rng, ka, 3.0);
                    let b = rand_shape(&mut rng, kb, 3.0);
                    let rot_a = rng.xform();
                    let rot_b = rng.xform();
                    let cases: [(Option<c2x>, Option<c2x>); 6] = [
                        (None, None),
                        (Some(ident), Some(ident)),
                        (Some(ident), None),
                        (None, Some(ident)),
                        (Some(rot_a), None),
                        (Some(rot_a), Some(rot_b)),
                    ];
                    for (ax, bx) in cases {
                        for ur in [0, 1] {
                            diff_gjk("xform", &a, ax, &b, bx, ur, true, None);
                        }
                    }
                }
            }
        }
    });
}

// --- row 33 : NULL out pointers ------------------------------------------

#[test]
fn row33_null_out_pointers() {
    fresh(|| {
        let mut rng = Rng::new(0x6A_0033);
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                for _ in 0..250 {
                    let a = rand_shape(&mut rng, ka, 3.0);
                    let b = rand_shape(&mut rng, kb, 3.0);
                    for ur in [0, 1] {
                        let (d_out, _) = diff_gjk("null-out-ref", &a, None, &b, None, ur, true, None);
                        let (d_null, _) = diff_gjk("null-out", &a, None, &b, None, ur, false, None);
                        assert!(
                            bits_eq_f32(d_out, d_null),
                            "NULL out changed the return value: {} vs {}",
                            fmt_f32(d_out),
                            fmt_f32(d_null)
                        );
                    }
                }
            }
        }
    });
}

// --- rows 34 / 35 / 36 / 37 : cache ---------------------------------------

#[test]
fn row34_cache_cold() {
    fresh(|| {
        let mut rng = Rng::new(0x6A_0034);
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                for _ in 0..300 {
                    let a = rand_shape(&mut rng, ka, 3.0);
                    let b = rand_shape(&mut rng, kb, 3.0);
                    let cold = c2GJKCache {
                        metric: rng.range(-5.0, 5.0),
                        count: 0,
                        iA: [rng.below(4) as c_int, rng.below(4) as c_int, rng.below(4) as c_int],
                        iB: [rng.below(4) as c_int, rng.below(4) as c_int, rng.below(4) as c_int],
                        div: rng.range(-2.0, 4.0),
                    };
                    for ur in [0, 1] {
                        diff_gjk("cache-cold", &a, None, &b, None, ur, true, Some(cold));
                    }
                }
            }
        }
    });
}

#[test]
fn row35_row36_cache_roundtrip() {
    fresh(|| {
        let mut rng = Rng::new(0x6A_0035);
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                for _ in 0..250 {
                    let a = rand_shape(&mut rng, ka, 3.0);
                    let b = rand_shape(&mut rng, kb, 3.0);
                    let cold = c2GJKCache::default();
                    // pass 1 — cold, produces a warm cache
                    let (_, warm) = diff_gjk("cache-rt1", &a, None, &b, None, 0, true, Some(cold));
                    let warm = warm.unwrap();
                    // row 35: reuse against the SAME shapes
                    let (_, warm2) = diff_gjk("cache-rt2", &a, None, &b, None, 0, true, Some(warm));
                    // row 36: reuse after MOVING the shapes (metric mismatch path)
                    let b2 = rand_shape(&mut rng, kb, 3.0);
                    diff_gjk("cache-moved", &a, None, &b2, None, 0, true, Some(warm2.unwrap()));
                    diff_gjk("cache-moved-ur", &a, None, &b2, None, 1, true, Some(warm));
                }
            }
        }
    });
}

#[test]
fn row37_cache_handbuilt() {
    fresh(|| {
        let mut rng = Rng::new(0x6A_0037);
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                for count in [1i32, 2, 3] {
                    for _ in 0..200 {
                        let a = rand_shape(&mut rng, ka, 3.0);
                        let b = rand_shape(&mut rng, kb, 3.0);
                        // indices must stay inside the proxy's vertex count for the
                        // C code to stay in-bounds: circle=1, capsule=2, aabb=4.
                        let na = match ka { 0 => 1u32, 1 => 4, _ => 2 };
                        let nb = match kb { 0 => 1u32, 1 => 4, _ => 2 };
                        let cache = c2GJKCache {
                            metric: match rng.below(5) {
                                0 => 0.0,
                                1 => -1e9,
                                2 => 1e9,
                                3 => rng.range(-1.0, 1.0),
                                _ => rng.range(-100.0, 100.0),
                            },
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
                            div: match rng.below(4) {
                                0 => 0.0,
                                1 => 1.0,
                                _ => rng.range(-3.0, 5.0),
                            },
                        };
                        for ur in [0, 1] {
                            diff_gjk("cache-hand", &a, None, &b, None, ur, true, Some(cache));
                        }
                    }
                }
            }
        }
    });
}
