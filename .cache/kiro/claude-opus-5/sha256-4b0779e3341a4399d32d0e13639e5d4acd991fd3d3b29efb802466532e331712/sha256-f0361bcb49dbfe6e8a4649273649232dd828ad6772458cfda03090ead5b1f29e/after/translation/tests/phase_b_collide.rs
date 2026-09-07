//! Phase B — valid-path differential tests for the boolean collision wrappers,
//! `c2Collided`, and the public `reverse_collide` entry point.
//!
//! CONFIGS.md rows 57–66.

mod common;

use common::*;
use std::ffi::c_void;

const N: usize = 20000;

// ---------------------------------------------------------------------------
// Row 57 — c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn row57_aabb_to_aabb() {
    type F = unsafe extern "C" fn(c2AABB, c2AABB) -> i32;
    let (c_f, r_f) = pair::<F>("c2AABBtoAABB");
    let mut rng = Rng::new(0x0057);

    for i in 0..N {
        let a = rng.aabb(50.0);
        let b = match i % 5 {
            // separated on x only
            0 => c2AABB {
                min: c2v {
                    x: a.max.x + 1.0,
                    y: a.min.y,
                },
                max: c2v {
                    x: a.max.x + 3.0,
                    y: a.max.y,
                },
            },
            // separated on y only
            1 => c2AABB {
                min: c2v {
                    x: a.min.x,
                    y: a.max.y + 1.0,
                },
                max: c2v {
                    x: a.max.x,
                    y: a.max.y + 3.0,
                },
            },
            // exactly edge-touching
            2 => c2AABB {
                min: c2v {
                    x: a.max.x,
                    y: a.min.y,
                },
                max: c2v {
                    x: a.max.x + 2.0,
                    y: a.max.y,
                },
            },
            // nested
            3 => c2AABB {
                min: a.min,
                max: c2v {
                    x: (a.min.x + a.max.x) * 0.5,
                    y: (a.min.y + a.max.y) * 0.5,
                },
            },
            // possibly inverted
            _ => rng.aabb_raw(50.0),
        };
        unsafe {
            assert_eq!(c_f(a, b), r_f(a, b), "c2AABBtoAABB #{i} {a:?} {b:?}");
            assert_eq!(c_f(b, a), r_f(b, a), "c2AABBtoAABB rev #{i} {b:?} {a:?}");
        }
    }

    // Edge-value exhaustive sweep (NaN, inf, ±0, FLT_MAX ...).
    for &p in EDGE_F32 {
        for &q in EDGE_F32 {
            let a = c2AABB {
                min: c2v { x: p, y: q },
                max: c2v { x: q, y: p },
            };
            for &s in EDGE_F32 {
                let b = c2AABB {
                    min: c2v { x: s, y: s },
                    max: c2v { x: p, y: q },
                };
                unsafe {
                    assert_eq!(c_f(a, b), r_f(a, b), "c2AABBtoAABB edge {a:?} {b:?}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 60 — c2CircletoCircle
// ---------------------------------------------------------------------------

#[test]
fn row60_circle_to_circle() {
    type F = unsafe extern "C" fn(c2Circle, c2Circle) -> i32;
    let (c_f, r_f) = pair::<F>("c2CircletoCircle");
    let mut rng = Rng::new(0x0060);

    for i in 0..N {
        let a = rng.circle(50.0);
        let b = match i % 4 {
            // exactly tangent
            0 => c2Circle {
                p: c2v {
                    x: a.p.x + a.r + 2.0,
                    y: a.p.y,
                },
                r: 2.0,
            },
            // nested
            1 => c2Circle { p: a.p, r: a.r * 0.5 },
            // negative radii (C squares the sum, so sign is erased)
            2 => c2Circle {
                p: rng.vec(50.0),
                r: -rng.radius(50.0),
            },
            _ => rng.circle(50.0),
        };
        unsafe {
            assert_eq!(c_f(a, b), r_f(a, b), "c2CircletoCircle #{i} {a:?} {b:?}");
            assert_eq!(c_f(b, a), r_f(b, a), "c2CircletoCircle rev #{i}");
        }
    }
    for &x in EDGE_F32 {
        for &r in EDGE_F32 {
            let a = c2Circle {
                p: c2v { x, y: r },
                r,
            };
            for &r2 in EDGE_F32 {
                let b = c2Circle {
                    p: c2v { x: r2, y: x },
                    r: r2,
                };
                unsafe {
                    assert_eq!(c_f(a, b), r_f(a, b), "c2CircletoCircle edge {a:?} {b:?}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 61 — c2CircletoAABB
// ---------------------------------------------------------------------------

#[test]
fn row61_circle_to_aabb() {
    type F = unsafe extern "C" fn(c2Circle, c2AABB) -> i32;
    let (c_f, r_f) = pair::<F>("c2CircletoAABB");
    let mut rng = Rng::new(0x0061);

    for i in 0..N {
        let bb = rng.aabb(50.0);
        let c = match i % 6 {
            // center inside
            0 => c2Circle {
                p: c2v {
                    x: (bb.min.x + bb.max.x) * 0.5,
                    y: (bb.min.y + bb.max.y) * 0.5,
                },
                r: rng.radius(10.0),
            },
            // outside a face
            1 => c2Circle {
                p: c2v {
                    x: bb.max.x + 1.0,
                    y: (bb.min.y + bb.max.y) * 0.5,
                },
                r: rng.radius(4.0),
            },
            // outside a corner
            2 => c2Circle {
                p: c2v {
                    x: bb.max.x + 1.0,
                    y: bb.max.y + 1.0,
                },
                r: rng.radius(4.0),
            },
            // exactly on the boundary
            3 => c2Circle {
                p: bb.max,
                r: rng.radius(4.0),
            },
            // degenerate AABB handled below via `bb2`
            4 => c2Circle {
                p: rng.vec(50.0),
                r: 0.0,
            },
            _ => rng.circle(50.0),
        };
        let bb2 = if i % 6 == 4 {
            c2AABB {
                min: bb.min,
                max: bb.min,
            }
        } else if i % 11 == 0 {
            rng.aabb_raw(50.0) // inverted
        } else {
            bb
        };
        unsafe {
            assert_eq!(
                c_f(c, bb2),
                r_f(c, bb2),
                "c2CircletoAABB #{i} {c:?} {bb2:?}"
            );
        }
    }
    for &p in EDGE_F32 {
        for &q in EDGE_F32 {
            let c = c2Circle {
                p: c2v { x: p, y: q },
                r: q,
            };
            let bb = c2AABB {
                min: c2v { x: q, y: p },
                max: c2v { x: p, y: q },
            };
            unsafe {
                assert_eq!(c_f(c, bb), r_f(c, bb), "c2CircletoAABB edge {c:?} {bb:?}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 62 — c2CircletoCapsule (all three da/db branches)
// ---------------------------------------------------------------------------

#[test]
fn row62_circle_to_capsule() {
    type F = unsafe extern "C" fn(c2Circle, c2Capsule) -> i32;
    let (c_f, r_f) = pair::<F>("c2CircletoCapsule");
    let mut rng = Rng::new(0x0062);

    // Track which of the three branches was taken, computed independently so
    // the coverage assertion does not depend on either implementation.
    let mut branch = [0usize; 3];
    for i in 0..N {
        let cap = match i % 4 {
            // degenerate a == b -> forces the `bp` branch (da==0, db==0)
            0 => {
                let p = rng.vec(30.0);
                c2Capsule {
                    a: p,
                    b: p,
                    r: rng.radius(10.0),
                }
            }
            _ => rng.capsule(30.0),
        };
        let circ = match i % 3 {
            // beyond endpoint a  -> da < 0
            0 => c2Circle {
                p: c2v {
                    x: cap.a.x - (cap.b.x - cap.a.x),
                    y: cap.a.y - (cap.b.y - cap.a.y),
                },
                r: rng.radius(8.0),
            },
            // near the midpoint  -> da >= 0, db < 0
            1 => c2Circle {
                p: c2v {
                    x: (cap.a.x + cap.b.x) * 0.5 + rng.coord(1.0),
                    y: (cap.a.y + cap.b.y) * 0.5 + rng.coord(1.0),
                },
                r: rng.radius(8.0),
            },
            // beyond endpoint b  -> db >= 0
            _ => c2Circle {
                p: c2v {
                    x: cap.b.x + (cap.b.x - cap.a.x),
                    y: cap.b.y + (cap.b.y - cap.a.y),
                },
                r: rng.radius(8.0),
            },
        };
        // independent branch classification
        let n = c2v {
            x: cap.b.x - cap.a.x,
            y: cap.b.y - cap.a.y,
        };
        let ap = c2v {
            x: circ.p.x - cap.a.x,
            y: circ.p.y - cap.a.y,
        };
        let da = ap.x * n.x + ap.y * n.y;
        if da < 0.0 {
            branch[0] += 1;
        } else {
            let bpv = c2v {
                x: circ.p.x - cap.b.x,
                y: circ.p.y - cap.b.y,
            };
            if bpv.x * n.x + bpv.y * n.y < 0.0 {
                branch[1] += 1;
            } else {
                branch[2] += 1;
            }
        }
        unsafe {
            assert_eq!(
                c_f(circ, cap),
                r_f(circ, cap),
                "c2CircletoCapsule #{i} {circ:?} {cap:?}"
            );
        }
    }
    assert!(
        branch.iter().all(|&x| x > 0),
        "c2CircletoCapsule branch coverage incomplete: {branch:?}"
    );

    for &p in EDGE_F32 {
        for &q in EDGE_F32 {
            let circ = c2Circle {
                p: c2v { x: p, y: q },
                r: q,
            };
            let cap = c2Capsule {
                a: c2v { x: q, y: p },
                b: c2v { x: p, y: q },
                r: p,
            };
            let cap_degen = c2Capsule {
                a: c2v { x: q, y: p },
                b: c2v { x: q, y: p },
                r: p,
            };
            unsafe {
                assert_eq!(
                    c_f(circ, cap),
                    r_f(circ, cap),
                    "c2CircletoCapsule edge {circ:?} {cap:?}"
                );
                assert_eq!(
                    c_f(circ, cap_degen),
                    r_f(circ, cap_degen),
                    "c2CircletoCapsule edge-degen {circ:?} {cap_degen:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 58, 59 — the two GJK-backed wrappers
// ---------------------------------------------------------------------------

#[test]
fn row58_aabb_to_capsule() {
    type F = unsafe extern "C" fn(c2AABB, c2Capsule) -> i32;
    let (c_f, r_f) = pair::<F>("c2AABBtoCapsule");
    let mut rng = Rng::new(0x0058);

    for i in 0..N {
        let bb = rng.aabb(40.0);
        let cap = match i % 5 {
            // capsule fully inside the AABB
            0 => c2Capsule {
                a: c2v {
                    x: (bb.min.x + bb.max.x) * 0.5,
                    y: (bb.min.y + bb.max.y) * 0.5,
                },
                b: c2v {
                    x: (bb.min.x + bb.max.x) * 0.5 + 0.1,
                    y: (bb.min.y + bb.max.y) * 0.5,
                },
                r: 0.05,
            },
            // degenerate capsule
            1 => {
                let p = rng.vec(40.0);
                c2Capsule {
                    a: p,
                    b: p,
                    r: rng.radius(5.0),
                }
            }
            // just touching the right face
            2 => c2Capsule {
                a: c2v {
                    x: bb.max.x + 1.0,
                    y: bb.min.y,
                },
                b: c2v {
                    x: bb.max.x + 1.0,
                    y: bb.max.y,
                },
                r: 1.0,
            },
            _ => rng.capsule(40.0),
        };
        unsafe {
            assert_eq!(
                c_f(bb, cap),
                r_f(bb, cap),
                "c2AABBtoCapsule #{i} {bb:?} {cap:?}"
            );
        }
    }
    for &p in EDGE_F32 {
        for &q in EDGE_F32 {
            let bb = c2AABB {
                min: c2v { x: p, y: q },
                max: c2v { x: q, y: p },
            };
            let cap = c2Capsule {
                a: c2v { x: q, y: p },
                b: c2v { x: p, y: q },
                r: p,
            };
            unsafe {
                assert_eq!(
                    c_f(bb, cap),
                    r_f(bb, cap),
                    "c2AABBtoCapsule edge {bb:?} {cap:?}"
                );
            }
        }
    }
}

#[test]
fn row59_capsule_to_capsule() {
    type F = unsafe extern "C" fn(c2Capsule, c2Capsule) -> i32;
    let (c_f, r_f) = pair::<F>("c2CapsuletoCapsule");
    let mut rng = Rng::new(0x0059);

    for i in 0..N {
        let a = rng.capsule(40.0);
        let b = match i % 6 {
            // parallel, offset
            0 => c2Capsule {
                a: c2v {
                    x: a.a.x,
                    y: a.a.y + a.r + 1.0,
                },
                b: c2v {
                    x: a.b.x,
                    y: a.b.y + a.r + 1.0,
                },
                r: 1.0,
            },
            // crossing
            1 => c2Capsule {
                a: c2v { x: a.a.x, y: a.b.y },
                b: c2v { x: a.b.x, y: a.a.y },
                r: rng.radius(3.0),
            },
            // collinear extension
            2 => c2Capsule {
                a: a.b,
                b: c2v {
                    x: a.b.x + (a.b.x - a.a.x),
                    y: a.b.y + (a.b.y - a.a.y),
                },
                r: a.r,
            },
            // coincident
            3 => a,
            // both degenerate
            4 => {
                let p = rng.vec(40.0);
                c2Capsule {
                    a: p,
                    b: p,
                    r: rng.radius(5.0),
                }
            }
            _ => rng.capsule(40.0),
        };
        unsafe {
            assert_eq!(
                c_f(a, b),
                r_f(a, b),
                "c2CapsuletoCapsule #{i} {a:?} {b:?}"
            );
            assert_eq!(c_f(b, a), r_f(b, a), "c2CapsuletoCapsule rev #{i}");
        }
    }
    for &p in EDGE_F32 {
        for &q in EDGE_F32 {
            let a = c2Capsule {
                a: c2v { x: p, y: q },
                b: c2v { x: q, y: p },
                r: p,
            };
            let b = c2Capsule {
                a: c2v { x: q, y: q },
                b: c2v { x: p, y: p },
                r: q,
            };
            unsafe {
                assert_eq!(
                    c_f(a, b),
                    r_f(a, b),
                    "c2CapsuletoCapsule edge {a:?} {b:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 63 — c2Collided, all 9 valid ordered type pairs
// ---------------------------------------------------------------------------

#[test]
fn row63_collided_all_pairs() {
    type F = unsafe extern "C" fn(*const c_void, i32, *const c_void, i32) -> i32;
    let (c_f, r_f) = pair::<F>("c2Collided");
    let mut rng = Rng::new(0x0063);

    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for i in 0..6000 {
                let scale = if i % 3 == 0 { 4.0 } else { 40.0 };
                // Keep each shape variant alive in its own binding so the raw
                // pointers stay valid for the duration of both calls.
                let ca = rng.circle(scale);
                let aa = rng.aabb(scale);
                let pa = rng.capsule(scale);
                let cb = rng.circle(scale);
                let ab = rng.aabb(scale);
                let pb = rng.capsule(scale);
                let pick = |t: i32,
                            c: &c2Circle,
                            a: &c2AABB,
                            p: &c2Capsule|
                 -> *const c_void {
                    match t {
                        C2_TYPE_CIRCLE => c as *const c2Circle as *const c_void,
                        C2_TYPE_AABB => a as *const c2AABB as *const c_void,
                        _ => p as *const c2Capsule as *const c_void,
                    }
                };
                let pa_ptr = pick(ta, &ca, &aa, &pa);
                let pb_ptr = pick(tb, &cb, &ab, &pb);
                unsafe {
                    assert_eq!(
                        c_f(pa_ptr, ta, pb_ptr, tb),
                        r_f(pa_ptr, ta, pb_ptr, tb),
                        "c2Collided ({ta},{tb}) #{i} \
                         circA={ca:?} aabbA={aa:?} capA={pa:?} \
                         circB={cb:?} aabbB={ab:?} capB={pb:?}"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 64, 65, 66 — reverse_collide (the one header-declared entry point)
// ---------------------------------------------------------------------------

#[test]
fn row64_reverse_collide_random() {
    type F = unsafe extern "C" fn(f32, f32, f32) -> i32;
    let (c_f, r_f) = pair::<F>("reverse_collide");
    let mut rng = Rng::new(0x0064);

    let mut masks = std::collections::BTreeSet::new();
    for i in 0..300000 {
        // Bias into the region occupied by the three fixed shapes so all eight
        // result bit-masks are produced.
        let (x, y, r) = if i % 4 == 0 {
            (rng.range(-130.0, 30.0), rng.range(-70.0, 130.0), rng.range(0.0, 30.0))
        } else if i % 4 == 1 {
            (rng.range(-60.0, 0.0), rng.range(-60.0, 60.0), rng.range(0.0, 90.0))
        } else if i % 4 == 2 {
            (rng.coord(200.0), rng.coord(200.0), rng.radius(200.0))
        } else {
            (rng.coord(1e5), rng.coord(1e5), rng.coord(1e5))
        };
        unsafe {
            let (cv, rv) = (c_f(x, y, r), r_f(x, y, r));
            assert_eq!(cv, rv, "reverse_collide #{i} ({x}, {y}, {r})");
            masks.insert(cv);
        }
    }
    // The three collisions are independent bits; a correct sweep must reach all 8.
    assert_eq!(
        masks.len(),
        8,
        "reverse_collide did not reach all 8 result masks, got {masks:?}"
    );
}

#[test]
fn row65_reverse_collide_grid_sweep() {
    type F = unsafe extern "C" fn(f32, f32, f32) -> i32;
    let (c_f, r_f) = pair::<F>("reverse_collide");

    // Deterministic dense sweep over the region containing all three shapes.
    let steps = 160;
    for &r in &[0.0f32, 0.5, 5.0, 20.0, 100.0] {
        for ix in 0..steps {
            let x = -130.0 + (ix as f32) * (170.0 / steps as f32);
            for iy in 0..steps {
                let y = -80.0 + (iy as f32) * (200.0 / steps as f32);
                unsafe {
                    assert_eq!(
                        c_f(x, y, r),
                        r_f(x, y, r),
                        "reverse_collide grid ({x}, {y}, {r})"
                    );
                }
            }
        }
    }
}

#[test]
fn row66_reverse_collide_extremes() {
    type F = unsafe extern "C" fn(f32, f32, f32) -> i32;
    let (c_f, r_f) = pair::<F>("reverse_collide");

    // Full cross product of the interesting scalar edge values, plus values
    // sitting exactly on the hard-coded shape boundaries in the C source.
    let boundary: &[f32] = &[
        -70.0, -50.0, -90.0, -40.0, -15.0, -20.0, 40.0, 100.0, 20.0, 10.0, 0.0, -0.0,
        -70.000_01, -69.999_99, -40.000_01, -39.999_99, -15.000_01, -14.999_99,
    ];
    let vals: Vec<f32> = EDGE_F32.iter().chain(boundary.iter()).copied().collect();

    for &x in &vals {
        for &y in &vals {
            for &r in &vals {
                unsafe {
                    assert_eq!(
                        c_f(x, y, r),
                        r_f(x, y, r),
                        "reverse_collide extreme ({x}, {y}, {r})"
                    );
                }
            }
        }
    }
}
