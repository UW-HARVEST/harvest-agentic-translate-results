//! Phase B — rows 30..51 of CONFIGS.md: `c2GJK` driven directly, across the
//! full cross-product of its runtime options (transforms, `use_radius`, cache
//! warm/cold, optional out-params) and the shape-type / geometry axes.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// Every shape kind, kept alive so raw pointers stay valid.
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
    /// `c2MakeProxy` has no poly case; the pointer is never dereferenced there.
    Poly(c2Poly),
}

impl Shape {
    fn ty(&self) -> c_int {
        match self {
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Poly(_) => C2_TYPE_POLY,
        }
    }
    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const _ as *const c_void,
            Shape::Aabb(a) => a as *const _ as *const c_void,
            Shape::Capsule(c) => c as *const _ as *const c_void,
            Shape::Poly(p) => p as *const _ as *const c_void,
        }
    }
}

/// Geometry family — chosen so the GJK loop takes every one of its exit paths.
#[derive(Copy, Clone, Debug)]
enum Geo {
    /// far apart: `d1 > d0` / small-direction exit
    Far,
    /// close but disjoint
    Near,
    /// overlapping (simplex reaches count 3 -> `hit`)
    Overlap,
    /// coincident / identical shapes: duplicate support point
    Same,
    /// degenerate: zero radius, `a == b`, `min == max`
    Degenerate,
    /// grid-snapped values, to force exact ties in the `<= 0` tests
    Grid,
}

const GEOS: [Geo; 6] = [
    Geo::Far,
    Geo::Near,
    Geo::Overlap,
    Geo::Same,
    Geo::Degenerate,
    Geo::Grid,
];

fn make_shape(rng: &mut Rng, ty: c_int, geo: Geo, center: c2v) -> Shape {
    let (sz, deg, grid) = match geo {
        Geo::Degenerate => (0.0f32, true, false),
        Geo::Grid => (1.0, false, true),
        _ => (1.0 + rng.unit() * 2.0, false, false),
    };
    let jit = |r: &mut Rng| -> c2v {
        if grid {
            c2v {
                x: center.x + r.grid(0.5, 4),
                y: center.y + r.grid(0.5, 4),
            }
        } else {
            c2v {
                x: center.x + r.sym(sz),
                y: center.y + r.sym(sz),
            }
        }
    };
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
            p: center,
            r: if deg {
                0.0
            } else if grid {
                rng.grid(0.5, 3)
            } else {
                rng.unit() * sz
            },
        }),
        C2_TYPE_AABB => {
            let a = jit(rng);
            let b = if deg { a } else { jit(rng) };
            Shape::Aabb(c2AABB {
                min: c2Minv_local(a, b),
                max: c2Maxv_local(a, b),
            })
        }
        C2_TYPE_CAPSULE => {
            let a = jit(rng);
            let b = if deg { a } else { jit(rng) };
            Shape::Capsule(c2Capsule {
                a,
                b,
                r: if deg {
                    0.0
                } else if grid {
                    rng.grid(0.5, 3)
                } else {
                    rng.unit() * sz
                },
            })
        }
        _ => {
            // Poly: fill it with plausible data even though the C never reads it
            // through the proxy, so any accidental read is still comparable.
            let mut p = c2Poly::default();
            p.count = 4;
            for k in 0..4 {
                p.verts[k] = jit(rng);
            }
            Shape::Poly(p)
        }
    }
}

fn c2Minv_local(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x < b.x { a.x } else { b.x },
        y: if a.y < b.y { a.y } else { b.y },
    }
}
fn c2Maxv_local(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x > b.x { a.x } else { b.x },
        y: if a.y > b.y { a.y } else { b.y },
    }
}

fn centers(rng: &mut Rng, geo: Geo) -> (c2v, c2v) {
    let a = rng.vec_sym(3.0);
    let b = match geo {
        Geo::Far => c2v {
            x: a.x + 200.0 + rng.unit() * 800.0,
            y: a.y + 200.0 + rng.unit() * 800.0,
        },
        Geo::Near => c2v {
            x: a.x + 3.0 + rng.unit(),
            y: a.y + 3.0 + rng.unit(),
        },
        Geo::Overlap => c2v {
            x: a.x + rng.sym(0.4),
            y: a.y + rng.sym(0.4),
        },
        Geo::Same => a,
        Geo::Degenerate => c2v {
            x: a.x + rng.sym(2.0),
            y: a.y + rng.sym(2.0),
        },
        Geo::Grid => c2v {
            x: a.x + rng.grid(1.0, 3),
            y: a.y + rng.grid(1.0, 3),
        },
    };
    (a, b)
}

/// One differential `c2GJK` call. `null_out` selects which optional out-params
/// are omitted; `cache_mode` selects cold/warm/absent cache.
#[allow(clippy::too_many_arguments)]
fn diff_gjk(
    p: &Pair,
    A: &Shape,
    ax: Option<c2x>,
    B: &Shape,
    bx: Option<c2x>,
    use_radius: c_int,
    null_out: u8,
    cache_seed: Option<(c2GJKCache, c2GJKCache)>,
    label: &str,
) -> (Option<c2GJKCache>, Option<c2GJKCache>) {
    let (cf, rf) = p.get::<FnGJK>(b"c2GJK");
    let axp = ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);

    let poison = c2v { x: -98765.5, y: 4321.25 };
    let (mut ca, mut cb) = (poison, poison);
    let (mut ra, mut rb) = (poison, poison);
    let mut cit: c_int = -12345;
    let mut rit: c_int = -12345;
    let want_a = null_out & 1 == 0;
    let want_b = null_out & 2 == 0;
    let want_it = null_out & 4 == 0;

    let (mut cc, mut rc) = match cache_seed {
        Some((a, b)) => (Some(a), Some(b)),
        None => (None, None),
    };

    let cdist = unsafe {
        scrub_stack();
        cf(
            A.ptr(),
            A.ty(),
            axp,
            B.ptr(),
            B.ty(),
            bxp,
            if want_a { &mut ca } else { std::ptr::null_mut() },
            if want_b { &mut cb } else { std::ptr::null_mut() },
            use_radius,
            if want_it {
                &mut cit
            } else {
                std::ptr::null_mut()
            },
            cc.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut _),
        )
    };
    let rdist = unsafe {
        scrub_stack();
        rf(
            A.ptr(),
            A.ty(),
            axp,
            B.ptr(),
            B.ty(),
            bxp,
            if want_a { &mut ra } else { std::ptr::null_mut() },
            if want_b { &mut rb } else { std::ptr::null_mut() },
            use_radius,
            if want_it {
                &mut rit
            } else {
                std::ptr::null_mut()
            },
            rc.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut _),
        )
    };

    same(&format!("{label}: c2GJK return"), &cdist, &rdist);
    same(&format!("{label}: outA"), &ca, &ra);
    same(&format!("{label}: outB"), &cb, &rb);
    same(&format!("{label}: iterations"), &cit, &rit);
    if let (Some(a), Some(b)) = (cc.as_ref(), rc.as_ref()) {
        same(&format!("{label}: cache"), a, b);
    }
    (cc, rc)
}

const N: usize = 260;

// ===========================================================================
// Rows 30..42, 46..51 — type pairs x geometry x use_radius x transforms x
// optional out-params.
// ===========================================================================

#[test]
fn rows30_42_gjk_typepairs_transforms_outparams() {
    let p = pair();
    let mut rng = Rng::new(0x3000);
    for i in 0..N {
        for &ta in &ALL_TYPES {
            for &tb in &ALL_TYPES {
                for &geo in &GEOS {
                    let (ca, cb) = centers(&mut rng, geo);
                    let A = make_shape(&mut rng, ta, geo, ca);
                    let B = make_shape(&mut rng, tb, geo, cb);
                    for use_radius in [0, 1] {
                        // Row 37/38/39/40: transform variants.
                        let xf = match i % 5 {
                            0 => (None, None),                       // both NULL
                            1 => (Some(c2xIdentity_local()), None),  // explicit identity
                            2 => (
                                Some(c2x {
                                    p: rng.vec_sym(5.0),
                                    r: c2r { c: 1.0, s: 0.0 },
                                }),
                                None,
                            ), // translation only
                            3 => (
                                Some(rng.xform(5.0, true)),
                                Some(rng.xform(5.0, true)),
                            ), // unit rotations + translations
                            _ => (
                                Some(rng.xform(5.0, false)),
                                Some(rng.xform(5.0, false)),
                            ), // NON-normalized rotations
                        };
                        // Rows 41/42: drop some out-params.
                        let null_out = (i % 8) as u8;
                        diff_gjk(
                            &p,
                            &A,
                            xf.0,
                            &B,
                            xf.1,
                            use_radius,
                            null_out,
                            None,
                            &format!("row30-42 i={i} ta={ta} tb={tb} {geo:?} ur={use_radius}"),
                        );
                    }
                }
            }
        }
    }
}

fn c2xIdentity_local() -> c2x {
    c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    }
}

// ===========================================================================
// Rows 43, 44, 45 — the cache: cold (count == 0), warm-started from a previous
// run, and warm-started then invalidated by moving the shapes.
// ===========================================================================

#[test]
fn rows43_45_gjk_cache() {
    let p = pair();
    let mut rng = Rng::new(0x4300);
    for i in 0..N * 3 {
        for &ta in &ALL_TYPES {
            for &tb in &ALL_TYPES {
                let geo = GEOS[i % GEOS.len()];
                let (ca, cb) = centers(&mut rng, geo);
                let A = make_shape(&mut rng, ta, geo, ca);
                let B = make_shape(&mut rng, tb, geo, cb);
                let use_radius = (i % 2) as c_int;

                // Row 43: cold cache (count == 0) - it is rejected, then written.
                let cold = c2GJKCache::default();
                let (cc, rc) = diff_gjk(
                    &p,
                    &A,
                    None,
                    &B,
                    None,
                    use_radius,
                    0,
                    Some((cold, cold)),
                    &format!("row43 i={i} ta={ta} tb={tb}"),
                );
                let (cc, rc) = (cc.unwrap(), rc.unwrap());
                assert_eq!(raw(&cc), raw(&rc));

                // Row 44: feed it straight back (warm start, cache IS read).
                let (cc2, rc2) = diff_gjk(
                    &p,
                    &A,
                    None,
                    &B,
                    None,
                    use_radius,
                    0,
                    Some((cc, rc)),
                    &format!("row44 i={i} ta={ta} tb={tb}"),
                );

                // Row 45: same warm cache but the shapes have MOVED, so the
                // metric check at lib.c:464 sees a mismatch.
                let moved = c2v {
                    x: cb.x + rng.sym(6.0),
                    y: cb.y + rng.sym(6.0),
                };
                let B2 = make_shape(&mut rng, tb, geo, moved);
                diff_gjk(
                    &p,
                    &A,
                    None,
                    &B2,
                    None,
                    use_radius,
                    0,
                    Some((cc2.unwrap(), rc2.unwrap())),
                    &format!("row45 i={i} ta={ta} tb={tb}"),
                );
            }
        }
    }
}

// ===========================================================================
// Row 44 variant — a hand-primed cache with counts 1..3 (not produced by a
// previous run), which drives the warm-start loop with arbitrary indices.
// ===========================================================================

#[test]
fn row44_gjk_hand_primed_cache() {
    let p = pair();
    let mut rng = Rng::new(0x4400);
    for i in 0..N * 4 {
        for &ta in &ALL_TYPES {
            for &tb in &ALL_TYPES {
                let geo = GEOS[i % GEOS.len()];
                let (ca, cb) = centers(&mut rng, geo);
                let A = make_shape(&mut rng, ta, geo, ca);
                let B = make_shape(&mut rng, tb, geo, cb);
                let mut cache = c2GJKCache::default();
                cache.count = 1 + (i % 3) as c_int;
                cache.div = if i % 4 == 0 { 0.0 } else { 1.0 + rng.unit() };
                cache.metric = match i % 5 {
                    0 => 0.0,
                    1 => -1.0e9, // trips the `metric < -1e8` half of the test
                    2 => rng.sym(10.0),
                    3 => f32::MAX,
                    _ => rng.sym(1e-6),
                };
                // Indices are only guaranteed in-bounds for the 8-vertex array.
                for k in 0..3 {
                    cache.iA[k] = rng.below(8) as c_int;
                    cache.iB[k] = rng.below(8) as c_int;
                }
                diff_gjk(
                    &p,
                    &A,
                    None,
                    &B,
                    None,
                    (i % 2) as c_int,
                    0,
                    Some((cache, cache)),
                    &format!("row44-primed i={i} ta={ta} tb={tb} count={}", cache.count),
                );
            }
        }
    }
}
