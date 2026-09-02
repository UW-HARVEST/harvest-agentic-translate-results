//! Phase B — rows 52..66 of CONFIGS.md: every public manifold generator,
//! driven directly with all the geometric relationships and degeneracies the C
//! branches on. The output `c2Manifold` is pre-poisoned so that fields the C
//! leaves untouched are compared too.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_int;

const N: usize = 6000;

fn minv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x < b.x { a.x } else { b.x },
        y: if a.y < b.y { a.y } else { b.y },
    }
}
fn maxv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x > b.x { a.x } else { b.x },
        y: if a.y > b.y { a.y } else { b.y },
    }
}

/// The value families used for every manifold row.
/// 0 tame, 1 grid-snapped (exact ties / touching), 2 large, 3 tiny,
/// 4 pathological (inf / NaN / ±0 / FLT_MAX).
fn coord(rng: &mut Rng, family: usize) -> f32 {
    match family {
        0 => rng.sym(5.0),
        1 => rng.grid(0.5, 8),
        2 => rng.sym(1e18),
        3 => rng.sym(1e-20),
        _ => rng.spicy(),
    }
}
fn cvec(rng: &mut Rng, family: usize) -> c2v {
    c2v {
        x: coord(rng, family),
        y: coord(rng, family),
    }
}
/// Radii: includes 0, negative, grid values and pathological ones.
fn radius(rng: &mut Rng, family: usize, i: usize) -> f32 {
    match i % 6 {
        0 => 0.0,
        1 => -rng.unit() * 3.0, // NEGATIVE radius
        2 => rng.grid(0.5, 6),
        _ => coord(rng, family).abs(),
    }
}

fn circle(rng: &mut Rng, family: usize, i: usize) -> c2Circle {
    c2Circle {
        p: cvec(rng, family),
        r: radius(rng, family, i),
    }
}

/// AABBs: proper, degenerate (`min == max`) and INVERTED (`min > max`).
fn aabb(rng: &mut Rng, family: usize, i: usize) -> c2AABB {
    let a = cvec(rng, family);
    let b = cvec(rng, family);
    match i % 4 {
        0 => c2AABB {
            min: minv(a, b),
            max: maxv(a, b),
        },
        1 => c2AABB { min: a, max: a }, // degenerate
        2 => c2AABB {
            min: maxv(a, b),
            max: minv(a, b),
        }, // inverted
        _ => c2AABB { min: a, max: b }, // arbitrary (may be inverted on one axis)
    }
}

/// Capsules: general, degenerate (`a == b`), axis-aligned, zero/negative radius.
fn capsule(rng: &mut Rng, family: usize, i: usize) -> c2Capsule {
    let a = cvec(rng, family);
    let b = match i % 5 {
        0 => a,                                    // degenerate segment
        1 => c2v { x: a.x, y: a.y + 3.0 },         // vertical
        2 => c2v { x: a.x + 4.0, y: a.y },         // horizontal
        _ => cvec(rng, family),
    };
    c2Capsule {
        a,
        b,
        r: radius(rng, family, i),
    }
}

/// Random convex polygon with `count` verts and normals from the library's own
/// `c2Norms`, plus deliberately inconsistent-normal and degenerate variants.
fn poly(rng: &mut Rng, norms: &FnNorms, count: c_int, mode: usize) -> c2Poly {
    let mut p = c2Poly::default();
    p.count = count;
    let n = count.clamp(0, 8) as usize;
    match mode % 4 {
        // convex CCW
        0 | 1 => {
            let mut angs: Vec<f32> = (0..n).map(|_| rng.unit() * std::f32::consts::TAU).collect();
            angs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for k in 0..n {
                let r = 1.0 + rng.unit() * 3.0;
                p.verts[k] = c2v {
                    x: r * angs[k].cos(),
                    y: r * angs[k].sin(),
                };
            }
            unsafe { norms(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), n.max(1) as c_int) };
        }
        // duplicate consecutive verts -> NaN normals
        2 => {
            for k in 0..n {
                p.verts[k] = rng.vec_sym(3.0);
            }
            if n >= 2 {
                p.verts[1] = p.verts[0];
            }
            unsafe { norms(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), n.max(1) as c_int) };
        }
        // normals deliberately INCONSISTENT with verts (the C never validates)
        _ => {
            for k in 0..8 {
                p.verts[k] = rng.vec_sym(3.0);
                p.norms[k] = rng.vec_sym(1.0);
            }
        }
    }
    p
}

// ===========================================================================
// Row 52 — c2CircletoCircleManifold
// ===========================================================================

#[test]
fn row52_circle_circle() {
    let p = pair();
    let (cf, rf) = p.get::<FnCC>(b"c2CircletoCircleManifold");
    let mut rng = Rng::new(0x5200);
    for i in 0..N * 4 {
        let family = i % 5;
        let A = circle(&mut rng, family, i);
        // Place B at a controlled separation so all branches are hit.
        let sep = match i % 6 {
            0 => 0.0,                       // coincident centres
            1 => A.r * 2.0,                 // exactly touching
            2 => A.r * 0.5,                 // deep overlap
            3 => A.r * 1.9,                 // shallow overlap
            4 => A.r * 20.0 + 5.0,          // far
            _ => rng.sym(10.0),
        };
        let ang = rng.unit() * std::f32::consts::TAU;
        let mut B = circle(&mut rng, family, i + 1);
        if i % 7 != 0 {
            B.p = c2v {
                x: A.p.x + sep * ang.cos(),
                y: A.p.y + sep * ang.sin(),
            };
        }
        let mut cm = poison_manifold(i as u8);
        let mut rm = cm;
        unsafe {
            cf(A, B, &mut cm);
            rf(A, B, &mut rm);
        }
        same("c2CircletoCircleManifold", &cm, &rm);
    }
}

// ===========================================================================
// Row 53 — c2CircletoAABBManifold (face / corner / inside / tie / degenerate)
// ===========================================================================

#[test]
fn row53_circle_aabb() {
    let p = pair();
    let (cf, rf) = p.get::<FnCA>(b"c2CircletoAABBManifold");
    let mut rng = Rng::new(0x5300);
    for i in 0..N * 4 {
        let family = i % 5;
        let B = aabb(&mut rng, family, i);
        let mid = c2v {
            x: (B.min.x + B.max.x) * 0.5,
            y: (B.min.y + B.max.y) * 0.5,
        };
        let mut A = circle(&mut rng, family, i);
        match i % 7 {
            0 => A.p = mid,                                          // centre inside -> d2 == 0
            1 => A.p = c2v { x: mid.x, y: B.min.y - A.r * 0.5 },      // straddling a face
            2 => A.p = c2v { x: B.min.x - A.r * 0.5, y: B.min.y - A.r * 0.5 }, // corner
            3 => A.p = B.min,                                        // exactly on a corner
            4 => {
                // symmetric box + centred circle => x_overlap == y_overlap tie
                A.p = mid;
            }
            _ => {}
        }
        // Row 53 tie sub-case: a square box with the circle at its centre.
        let B = if i % 7 == 4 {
            c2AABB {
                min: c2v { x: -2.0, y: -2.0 },
                max: c2v { x: 2.0, y: 2.0 },
            }
        } else {
            B
        };
        let mut cm = poison_manifold(i as u8);
        let mut rm = cm;
        unsafe {
            cf(A, B, &mut cm);
            rf(A, B, &mut rm);
        }
        same("c2CircletoAABBManifold", &cm, &rm);
    }
}

// ===========================================================================
// Row 54 — c2CircletoCapsuleManifold
// ===========================================================================

#[test]
fn row54_circle_capsule() {
    let p = pair();
    let (cf, rf) = p.get::<FnCCap>(b"c2CircletoCapsuleManifold");
    let mut rng = Rng::new(0x5400);
    for i in 0..N * 4 {
        let family = i % 5;
        let B = capsule(&mut rng, family, i);
        let mut A = circle(&mut rng, family, i);
        match i % 5 {
            0 => A.p = B.a,                                     // on an endpoint -> d == 0
            1 => {
                A.p = c2v {
                    x: (B.a.x + B.b.x) * 0.5,
                    y: (B.a.y + B.b.y) * 0.5,
                }
            } // on the axis
            2 => {
                A.p = c2v {
                    x: B.a.x + 1e4,
                    y: B.a.y + 1e4,
                }
            } // far
            _ => {}
        }
        let mut cm = poison_manifold(i as u8);
        let mut rm = cm;
        scrub_stack();
        unsafe { cf(A, B, &mut cm) };
        scrub_stack();
        unsafe { rf(A, B, &mut rm) };
        same("c2CircletoCapsuleManifold", &cm, &rm);
    }
}

// ===========================================================================
// Row 55 — c2AABBtoAABBManifold (all four axis/sign sub-branches + the tie)
// ===========================================================================

#[test]
fn row55_aabb_aabb() {
    let p = pair();
    let (cf, rf) = p.get::<FnAA>(b"c2AABBtoAABBManifold");
    let mut rng = Rng::new(0x5500);
    for i in 0..N * 4 {
        let family = i % 5;
        let A = aabb(&mut rng, family, i);
        let mut B = aabb(&mut rng, family, i + 1);
        match i % 8 {
            0 => B = A,                                       // identical
            1 => {
                // separated on X only
                B = c2AABB {
                    min: c2v { x: A.max.x + 1.0, y: A.min.y },
                    max: c2v { x: A.max.x + 3.0, y: A.max.y },
                }
            }
            2 => {
                // separated on Y only
                B = c2AABB {
                    min: c2v { x: A.min.x, y: A.max.y + 1.0 },
                    max: c2v { x: A.max.x, y: A.max.y + 3.0 },
                }
            }
            3 => {
                // exactly touching on X (dx == 0)
                B = c2AABB {
                    min: c2v { x: A.max.x, y: A.min.y },
                    max: c2v { x: A.max.x + 2.0, y: A.max.y },
                }
            }
            4 => {
                // dx == dy tie: equal-size boxes offset diagonally by the same amount
                let e = 2.0f32;
                B = c2AABB {
                    min: c2v { x: A.min.x + e, y: A.min.y + e },
                    max: c2v { x: A.max.x + e, y: A.max.y + e },
                }
            }
            5 => {
                // B fully inside A
                let mid = c2v {
                    x: (A.min.x + A.max.x) * 0.5,
                    y: (A.min.y + A.max.y) * 0.5,
                };
                B = c2AABB {
                    min: c2v { x: mid.x - 0.01, y: mid.y - 0.01 },
                    max: c2v { x: mid.x + 0.01, y: mid.y + 0.01 },
                }
            }
            _ => {}
        }
        let mut cm = poison_manifold(i as u8);
        let mut rm = cm;
        unsafe {
            cf(A, B, &mut cm);
            rf(A, B, &mut rm);
        }
        same("c2AABBtoAABBManifold", &cm, &rm);
    }
}

// ===========================================================================
// Row 56 — c2CapsuletoCapsuleManifold
// ===========================================================================

#[test]
fn row56_capsule_capsule() {
    let p = pair();
    let (cf, rf) = p.get::<FnCapCap>(b"c2CapsuletoCapsuleManifold");
    let mut rng = Rng::new(0x5600);
    for i in 0..N * 4 {
        let family = i % 5;
        let A = capsule(&mut rng, family, i);
        let mut B = capsule(&mut rng, family, i + 2);
        match i % 6 {
            0 => B = A,                                                    // identical
            1 => {
                // parallel, offset
                B = c2Capsule {
                    a: c2v { x: A.a.x + 1.0, y: A.a.y + 1.0 },
                    b: c2v { x: A.b.x + 1.0, y: A.b.y + 1.0 },
                    r: A.r,
                }
            }
            2 => {
                // crossing
                B = c2Capsule {
                    a: c2v { x: A.a.x, y: A.b.y },
                    b: c2v { x: A.b.x, y: A.a.y },
                    r: A.r,
                }
            }
            3 => {
                // collinear extension
                B = c2Capsule {
                    a: A.b,
                    b: c2v {
                        x: A.b.x + (A.b.x - A.a.x),
                        y: A.b.y + (A.b.y - A.a.y),
                    },
                    r: A.r,
                }
            }
            _ => {}
        }
        let mut cm = poison_manifold(i as u8);
        let mut rm = cm;
        scrub_stack();
        unsafe { cf(A, B, &mut cm) };
        scrub_stack();
        unsafe { rf(A, B, &mut rm) };
        same("c2CapsuletoCapsuleManifold", &cm, &rm);
    }
}

// ===========================================================================
// Rows 57..65 — c2CapsuletoPolyManifold: poly vertex counts 0..8, all three
// separating-axis `code` paths, transforms, degenerate capsules, bad normals.
// ===========================================================================

#[test]
fn rows57_65_capsule_poly() {
    let p = pair();
    let (cf, rf) = p.get::<FnCapPoly>(b"c2CapsuletoPolyManifold");
    let (cNorms, _) = p.get::<FnNorms>(b"c2Norms");
    let mut rng = Rng::new(0x5700);
    for i in 0..N * 3 {
        // Row 63: counts 0,1,2 included alongside the useful 3..8.
        let count = (i % 9) as c_int;
        let pl = poly(&mut rng, &cNorms, count, i / 9);
        let family = i % 5;
        // Place the capsule to sweep far / shallow band / overlapping, which is
        // what selects between the `d >= A.r`, `1e-6 <= d < A.r` and `d < 1e-6`
        // branches and among code 0/1/2.
        let mut A = capsule(&mut rng, family, i);
        match i % 6 {
            0 => {
                A.a = c2v { x: 500.0, y: 500.0 };
                A.b = c2v { x: 505.0, y: 503.0 };
            } // far
            1 => {
                A.a = c2v { x: 4.2, y: 0.0 };
                A.b = c2v { x: 4.2, y: 2.0 };
                A.r = 1.0;
            } // shallow band
            2 => {
                A.a = c2v { x: -0.5, y: -0.5 };
                A.b = c2v { x: 0.5, y: 0.5 };
                A.r = 0.3;
            } // overlapping, poly face likely
            3 => {
                A.a = c2v { x: -4.0, y: 0.0 };
                A.b = c2v { x: 4.0, y: 0.0 };
                A.r = 0.2;
            } // long axis through the poly -> capsule axis wins
            4 => {
                A.a = c2v { x: 0.0, y: -4.0 };
                A.b = c2v { x: 0.0, y: 4.0 };
                A.r = 0.2;
            } // the other capsule axis
            _ => {}
        }
        // Row 64: degenerate capsule (a == b) => `ab` is NaN.
        if i % 11 == 0 {
            A.b = A.a;
        }
        // Row 62: bx_ptr NULL / identity / translation / rotation / non-normalized.
        let bx = match i % 5 {
            0 => None,
            1 => Some(c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: c2r { c: 1.0, s: 0.0 },
            }),
            2 => Some(c2x {
                p: rng.vec_sym(3.0),
                r: c2r { c: 1.0, s: 0.0 },
            }),
            3 => Some(rng.xform(3.0, true)),
            _ => Some(rng.xform(3.0, false)),
        };
        let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);

        let mut cm = poison_manifold(i as u8);
        let mut rm = cm;
        scrub_stack();
        unsafe { cf(A, &pl, bxp, &mut cm) };
        scrub_stack();
        unsafe { rf(A, &pl, bxp, &mut rm) };
        same(
            &format!("c2CapsuletoPolyManifold i={i} count={count}"),
            &cm,
            &rm,
        );
    }
}

// ===========================================================================
// Row 66 — c2AABBtoCapsuleManifold, including the unconditional `m->n` negation
// on the no-manifold path (hence the poisoned `m`).
// ===========================================================================

#[test]
fn row66_aabb_capsule() {
    let p = pair();
    let (cf, rf) = p.get::<FnACap>(b"c2AABBtoCapsuleManifold");
    let mut rng = Rng::new(0x6600);
    for i in 0..N * 4 {
        let family = i % 5;
        let A = aabb(&mut rng, family, i);
        let mid = c2v {
            x: (A.min.x + A.max.x) * 0.5,
            y: (A.min.y + A.max.y) * 0.5,
        };
        let mut B = capsule(&mut rng, family, i);
        match i % 6 {
            0 => {
                B.a = mid;
                B.b = c2v { x: mid.x + 0.1, y: mid.y + 0.1 };
            } // deep inside
            1 => {
                B.a = c2v { x: A.min.x - 10.0, y: A.min.y - 10.0 };
                B.b = c2v { x: A.min.x - 9.0, y: A.min.y - 11.0 };
            } // far
            2 => {
                B.a = A.min;
                B.b = A.max;
            } // diagonal
            3 => {
                B.a = c2v { x: mid.x, y: A.min.y - B.r * 0.5 };
                B.b = c2v { x: mid.x, y: A.min.y - B.r * 2.0 };
            } // grazing a face
            _ => {}
        }
        let mut cm = poison_manifold(i as u8);
        let mut rm = cm;
        scrub_stack();
        unsafe { cf(A, B, &mut cm) };
        scrub_stack();
        unsafe { rf(A, B, &mut rm) };
        same(&format!("c2AABBtoCapsuleManifold i={i}"), &cm, &rm);
    }
}
