//! Phase B — Group 6: the manifold producers.
//! CONFIGS.md rows 79..108.
#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 2048;

// -------------------------------------------------------------------------
// rows 79, 80
// -------------------------------------------------------------------------
#[test]
fn cfg_circle_circle() {
    let mut acc = DiffAccum::new("cfg_circle_circle");
    let mut rng = Rng::new(0xaeed_0001);
    // random (overlap-biased)
    for i in 0..(N * 4) {
        let A = rng.circle();
        let B = rng.circle();
        acc.check(format!("rand #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
    }
    // guaranteed overlap
    for i in 0..N {
        let A = c2Circle {
            p: c2v {
                x: rng.sym(1.0),
                y: rng.sym(1.0),
            },
            r: 1.0 + rng.unit(),
        };
        let B = c2Circle {
            p: c2v {
                x: A.p.x + rng.sym(0.5),
                y: A.p.y + rng.sym(0.5),
            },
            r: 1.0 + rng.unit(),
        };
        acc.check(format!("overlap #{i}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
    }
    // exact touch: |d| == rA + rB
    for k in 0..256 {
        let rA = 0.5 + k as f32 * 0.125;
        let rB = 1.0;
        let A = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: rA,
        };
        let B = c2Circle {
            p: c2v { x: rA + rB, y: 0.0 },
            r: rB,
        };
        acc.check(format!("touch k={k}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
    }
    // coincident centres (l == 0 ⇒ fallback normal (0,1))
    for i in 0..N {
        let p = rng.vec();
        let A = c2Circle { p, r: rng.radius() };
        let B = c2Circle { p, r: rng.radius() };
        acc.check(format!("coincident #{i}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
    }
    // zero / negative radii
    for i in 0..N {
        let A = c2Circle {
            p: rng.vec(),
            r: [0.0f32, -0.0, -1.0, -5.0][rng.below(4) as usize],
        };
        let B = c2Circle {
            p: rng.vec(),
            r: [0.0f32, -0.0, -1.0, -5.0][rng.below(4) as usize],
        };
        acc.check(format!("badradius #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
    }
    // non-finite
    for i in 0..N {
        let A = c2Circle {
            p: rng.special_vec(),
            r: rng.special(),
        };
        let B = c2Circle {
            p: rng.special_vec(),
            r: rng.special(),
        };
        acc.check(format!("special #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
    }
    acc.finish();
}

// -------------------------------------------------------------------------
// rows 81..84
// -------------------------------------------------------------------------
#[test]
fn cfg_circle_aabb() {
    let mut acc = DiffAccum::new("cfg_circle_aabb");
    let mut rng = Rng::new(0xaeed_0002);
    let mut outside = 0usize;
    let mut inside_x = 0usize;
    let mut inside_y = 0usize;
    for i in 0..(N * 4) {
        let A = rng.circle();
        let B = rng.aabb();
        let m = acc_ret(&mut acc, format!("rand #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
        let _ = m;
    }
    // centre strictly outside but overlapping (d2 != 0)
    for i in 0..N {
        let e = 1.0 + rng.unit();
        let B = c2AABB {
            min: c2v { x: -e, y: -e },
            max: c2v { x: e, y: e },
        };
        let ang = rng.unit() * std::f32::consts::TAU;
        let dist = e + 0.05 + rng.unit() * 0.9;
        let A = c2Circle {
            p: c2v {
                x: dist * ang.cos(),
                y: dist * ang.sin(),
            },
            r: 1.5 + rng.unit(),
        };
        outside += 1;
        acc.check(format!("outside #{i}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
    }
    // centre inside the box (d2 == 0) with x_overlap < y_overlap and the reverse
    for i in 0..N {
        let ex = 1.0 + rng.unit() * 3.0;
        let ey = 1.0 + rng.unit() * 3.0;
        let B = c2AABB {
            min: c2v { x: -ex, y: -ey },
            max: c2v { x: ex, y: ey },
        };
        // push the centre near one of the faces
        let near_x = rng.bool();
        let A = c2Circle {
            p: if near_x {
                c2v {
                    x: (ex - 0.05) * if rng.bool() { 1.0 } else { -1.0 },
                    y: rng.sym(ey * 0.2),
                }
            } else {
                c2v {
                    x: rng.sym(ex * 0.2),
                    y: (ey - 0.05) * if rng.bool() { 1.0 } else { -1.0 },
                }
            },
            r: 0.25 + rng.unit(),
        };
        if near_x {
            inside_x += 1;
        } else {
            inside_y += 1;
        }
        acc.check(format!("inside near_x={near_x} #{i}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
    }
    // exactly on a corner / on an edge
    for k in 0..128 {
        let e = 1.0 + k as f32 * 0.25;
        let B = c2AABB {
            min: c2v { x: -e, y: -e },
            max: c2v { x: e, y: e },
        };
        for &(x, y) in &[
            (e, e),
            (-e, e),
            (e, -e),
            (-e, -e),
            (e, 0.0),
            (0.0, e),
            (0.0, 0.0),
        ] {
            let A = c2Circle {
                p: c2v { x, y },
                r: 1.0,
            };
            acc.check(format!("corner k={k} ({x},{y})"), |s| {
                with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
            });
        }
    }
    // degenerate / inverted boxes
    for i in 0..N {
        let p = rng.vec();
        let B = match rng.below(3) {
            0 => c2AABB { min: p, max: p },
            1 => c2AABB {
                min: c2v { x: 1.0, y: 1.0 },
                max: c2v { x: -1.0, y: -1.0 },
            },
            _ => c2AABB {
                min: rng.vec(),
                max: rng.vec(),
            },
        };
        let A = rng.circle();
        acc.check(format!("degenbox #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
    }
    // non-finite
    for i in 0..N {
        let A = c2Circle {
            p: rng.special_vec(),
            r: rng.special(),
        };
        let B = c2AABB {
            min: rng.special_vec(),
            max: rng.special_vec(),
        };
        acc.check(format!("special #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
    }
    acc.finish();
    eprintln!("cfg_circle_aabb: outside={outside} inside_x={inside_x} inside_y={inside_y}");
    assert!(outside > 0 && inside_x > 0 && inside_y > 0);
}

fn acc_ret<R: BitEq + Copy, F: FnMut(Side) -> R>(
    acc: &mut DiffAccum,
    label: String,
    mut f: F,
) -> R {
    let c = f(Side::C);
    acc.check(label, |s| f(s));
    c
}

// -------------------------------------------------------------------------
// rows 85..87
// -------------------------------------------------------------------------
#[test]
fn cfg_circle_capsule() {
    let mut acc = DiffAccum::new("cfg_circle_capsule");
    let mut rng = Rng::new(0xaeed_0003);
    for i in 0..(N * 4) {
        let A = rng.circle();
        let B = rng.capsule();
        acc.check(format!("rand #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A, B, m))
        });
    }
    // overlapping, d != 0
    for i in 0..N {
        let B = c2Capsule {
            a: c2v { x: -2.0, y: 0.0 },
            b: c2v { x: 2.0, y: 0.0 },
            r: 0.5 + rng.unit(),
        };
        let A = c2Circle {
            p: c2v {
                x: rng.sym(3.0),
                y: (0.2 + rng.unit()) * if rng.bool() { 1.0 } else { -1.0 },
            },
            r: 0.5 + rng.unit(),
        };
        acc.check(format!("overlap #{i}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A, B, m))
        });
    }
    // centre exactly on the spine ⇒ d == 0
    for i in 0..N {
        let B = c2Capsule {
            a: c2v { x: -2.0, y: 0.0 },
            b: c2v { x: 2.0, y: 0.0 },
            r: 0.5 + rng.unit(),
        };
        let A = c2Circle {
            p: c2v {
                x: rng.sym(1.5),
                y: 0.0,
            },
            r: 0.5 + rng.unit(),
        };
        acc.check(format!("on-spine #{i}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A, B, m))
        });
    }
    // degenerate capsule (a == b) with the circle on top ⇒ NaN normal
    for i in 0..N {
        let p = rng.vec();
        let B = c2Capsule {
            a: p,
            b: p,
            r: 0.5 + rng.unit(),
        };
        let A = c2Circle {
            p: if rng.bool() { p } else { rng.vec() },
            r: 0.5 + rng.unit(),
        };
        acc.check(format!("degen-capsule #{i}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A, B, m))
        });
    }
    // separated
    for i in 0..N {
        let B = c2Capsule {
            a: c2v { x: -1.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: 0.5,
        };
        let A = c2Circle {
            p: c2v {
                x: rng.sym(2.0),
                y: 10.0 + rng.unit(),
            },
            r: 0.5,
        };
        acc.check(format!("separated #{i}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A, B, m))
        });
    }
    // zero radii
    for i in 0..N {
        let A = c2Circle {
            p: rng.vec(),
            r: 0.0,
        };
        let B = c2Capsule {
            a: rng.vec(),
            b: rng.vec(),
            r: 0.0,
        };
        acc.check(format!("r0 #{i}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A, B, m))
        });
    }
    // non-finite
    for i in 0..N {
        let A = c2Circle {
            p: rng.special_vec(),
            r: rng.special(),
        };
        let B = c2Capsule {
            a: rng.special_vec(),
            b: rng.special_vec(),
            r: rng.special(),
        };
        acc.check(format!("special #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A, B, m))
        });
    }
    acc.finish();
}

// -------------------------------------------------------------------------
// rows 88..92
// -------------------------------------------------------------------------
#[test]
fn cfg_aabb_aabb() {
    let mut acc = DiffAccum::new("cfg_aabb_aabb");
    let mut rng = Rng::new(0xaeed_0004);
    let mut quad = [0usize; 4];
    for i in 0..(N * 8) {
        let A = rng.aabb();
        let B = rng.aabb();
        acc.check(format!("rand #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2AABBtoAABBManifold(s, A, B, m))
        });
    }
    // targeted: control which axis and which sign wins
    for i in 0..(N * 2) {
        let ex = 1.0 + rng.unit() * 2.0;
        let ey = 1.0 + rng.unit() * 2.0;
        let A = c2AABB {
            min: c2v { x: -ex, y: -ey },
            max: c2v { x: ex, y: ey },
        };
        // pick a small overlap along one axis
        let want_x = rng.bool();
        let neg = rng.bool();
        let sx = if want_x {
            (2.0 * ex - (0.05 + rng.unit() * 0.5)) * if neg { -1.0 } else { 1.0 }
        } else {
            rng.sym(0.5)
        };
        let sy = if want_x {
            rng.sym(0.5)
        } else {
            (2.0 * ey - (0.05 + rng.unit() * 0.5)) * if neg { -1.0 } else { 1.0 }
        };
        let B = c2AABB {
            min: c2v {
                x: sx - ex,
                y: sy - ey,
            },
            max: c2v {
                x: sx + ex,
                y: sy + ey,
            },
        };
        quad[(want_x as usize) * 2 + neg as usize] += 1;
        acc.check(format!("axis want_x={want_x} neg={neg} #{i}"), |s| {
            with_sentinel(|m| c2AABBtoAABBManifold(s, A, B, m))
        });
    }
    // identical boxes / shared edges / inverted
    for k in 0..128 {
        let e = 1.0 + k as f32 * 0.25;
        let A = c2AABB {
            min: c2v { x: -e, y: -e },
            max: c2v { x: e, y: e },
        };
        for &(dx, dy) in &[
            (0.0f32, 0.0f32),
            (2.0 * e, 0.0),
            (-2.0 * e, 0.0),
            (0.0, 2.0 * e),
            (0.0, -2.0 * e),
            (2.0 * e, 2.0 * e),
        ] {
            let B = c2AABB {
                min: c2v {
                    x: -e + dx,
                    y: -e + dy,
                },
                max: c2v { x: e + dx, y: e + dy },
            };
            acc.check(format!("edge k={k} ({dx},{dy})"), |s| {
                with_sentinel(|m| c2AABBtoAABBManifold(s, A, B, m))
            });
        }
        let inv = c2AABB {
            min: c2v { x: e, y: e },
            max: c2v { x: -e, y: -e },
        };
        acc.check(format!("inv k={k}"), |s| {
            with_sentinel(|m| c2AABBtoAABBManifold(s, A, inv, m))
        });
        acc.check(format!("inv2 k={k}"), |s| {
            with_sentinel(|m| c2AABBtoAABBManifold(s, inv, A, m))
        });
    }
    // non-finite
    for i in 0..(N * 2) {
        let A = c2AABB {
            min: rng.special_vec(),
            max: rng.special_vec(),
        };
        let B = c2AABB {
            min: rng.special_vec(),
            max: rng.special_vec(),
        };
        acc.check(format!("special #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2AABBtoAABBManifold(s, A, B, m))
        });
    }
    acc.finish();
    eprintln!("cfg_aabb_aabb: axis/sign quadrants = {quad:?}");
    assert!(quad.iter().all(|&n| n > 0));
}

// -------------------------------------------------------------------------
// rows 93..95
// -------------------------------------------------------------------------
#[test]
fn cfg_capsule_capsule() {
    let mut acc = DiffAccum::new("cfg_capsule_capsule");
    let mut rng = Rng::new(0xaeed_0005);
    for i in 0..(N * 4) {
        let A = rng.capsule();
        let B = rng.capsule();
        acc.check(format!("rand #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, B, m))
        });
    }
    // crossing (deep, d == 0)
    for i in 0..N {
        let A = c2Capsule {
            a: c2v { x: -2.0, y: 0.0 },
            b: c2v { x: 2.0, y: 0.0 },
            r: 0.25 + rng.unit(),
        };
        let B = c2Capsule {
            a: c2v {
                x: rng.sym(1.0),
                y: -2.0,
            },
            b: c2v {
                x: rng.sym(1.0),
                y: 2.0,
            },
            r: 0.25 + rng.unit(),
        };
        acc.check(format!("cross #{i}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, B, m))
        });
    }
    // parallel, overlapping laterally
    for i in 0..N {
        let r = 0.25 + rng.unit();
        let A = c2Capsule {
            a: c2v { x: -2.0, y: 0.0 },
            b: c2v { x: 2.0, y: 0.0 },
            r,
        };
        let B = c2Capsule {
            a: c2v {
                x: -2.0 + rng.sym(1.0),
                y: r * (0.5 + rng.unit()),
            },
            b: c2v {
                x: 2.0 + rng.sym(1.0),
                y: r * (0.5 + rng.unit()),
            },
            r,
        };
        acc.check(format!("parallel #{i}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, B, m))
        });
    }
    // end to end
    for k in 0..256 {
        let r = 0.25 + k as f32 * 0.05;
        let A = c2Capsule {
            a: c2v { x: -2.0, y: 0.0 },
            b: c2v { x: 0.0, y: 0.0 },
            r,
        };
        let B = c2Capsule {
            a: c2v { x: 2.0 * r, y: 0.0 },
            b: c2v { x: 2.0 * r + 2.0, y: 0.0 },
            r,
        };
        acc.check(format!("end2end k={k}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, B, m))
        });
    }
    // degenerate on one / both, r == 0
    for i in 0..N {
        let pa = rng.vec();
        let pb = rng.vec();
        let A = c2Capsule {
            a: pa,
            b: if rng.below(3) == 0 { pa } else { rng.vec() },
            r: if rng.below(4) == 0 { 0.0 } else { rng.unit() * 2.0 },
        };
        let B = c2Capsule {
            a: pb,
            b: if rng.below(3) == 0 { pb } else { rng.vec() },
            r: if rng.below(4) == 0 { 0.0 } else { rng.unit() * 2.0 },
        };
        acc.check(format!("degen #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, B, m))
        });
    }
    // identical (deep, degenerate normal path)
    for i in 0..N / 4 {
        let A = rng.capsule();
        acc.check(format!("identical #{i}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, A, m))
        });
    }
    // non-finite
    for i in 0..N {
        let A = c2Capsule {
            a: rng.special_vec(),
            b: rng.special_vec(),
            r: rng.special(),
        };
        let B = c2Capsule {
            a: rng.special_vec(),
            b: rng.special_vec(),
            r: rng.special(),
        };
        acc.check(format!("special #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, B, m))
        });
    }
    acc.finish();
}

// -------------------------------------------------------------------------
// rows 96..103
// -------------------------------------------------------------------------
#[test]
fn cfg_capsule_poly() {
    let mut acc = DiffAccum::new("cfg_capsule_poly");
    let mut rng = Rng::new(0xaeed_0006);
    let mut counts = [0usize; 3]; // contact-point counts observed
    for count in 3..=8i32 {
        for &has_bx in &[false, true] {
            // random
            for i in 0..N {
                let verts = rng.convex_poly_verts(count as usize);
                let poly = make_poly(&verts, count);
                let A = rng.capsule();
                let bx = if has_bx { Some(rng.xform()) } else { None };
                let m = acc_ret(
                    &mut acc,
                    format!("rand count={count} bx={has_bx} #{i}"),
                    |s| {
                        let bxp = match &bx {
                            Some(x) => x as *const c2x,
                            None => std::ptr::null(),
                        };
                        with_sentinel(|m| c2CapsuletoPolyManifold(s, A, &poly, bxp, m))
                    },
                );
                if (0..=2).contains(&m.count) {
                    counts[m.count as usize] += 1;
                }
            }
            // capsule slicing through the poly (deep ⇒ code 0/1/2 reachable)
            for i in 0..N {
                let verts = rng.convex_poly_verts(count as usize);
                let poly = make_poly(&verts, count);
                // centre of the poly
                let mut cx = 0.0f32;
                let mut cy = 0.0f32;
                for k in 0..count as usize {
                    cx += verts[k].x;
                    cy += verts[k].y;
                }
                cx /= count as f32;
                cy /= count as f32;
                let ang = rng.unit() * std::f32::consts::TAU;
                let len = 1.0 + rng.unit() * 4.0;
                let A = c2Capsule {
                    a: c2v {
                        x: cx - len * ang.cos(),
                        y: cy - len * ang.sin(),
                    },
                    b: c2v {
                        x: cx + len * ang.cos(),
                        y: cy + len * ang.sin(),
                    },
                    r: 0.1 + rng.unit(),
                };
                let bx = if has_bx { Some(rng.xform()) } else { None };
                let m = acc_ret(
                    &mut acc,
                    format!("deep count={count} bx={has_bx} #{i}"),
                    |s| {
                        let bxp = match &bx {
                            Some(x) => x as *const c2x,
                            None => std::ptr::null(),
                        };
                        with_sentinel(|m| c2CapsuletoPolyManifold(s, A, &poly, bxp, m))
                    },
                );
                if (0..=2).contains(&m.count) {
                    counts[m.count as usize] += 1;
                }
            }
            // shallow: just outside the poly but within A.r (row 100)
            for i in 0..N {
                let verts = rng.convex_poly_verts(count as usize);
                let poly = make_poly(&verts, count);
                let mut cx = 0.0f32;
                let mut cy = 0.0f32;
                for k in 0..count as usize {
                    cx += verts[k].x;
                    cy += verts[k].y;
                }
                cx /= count as f32;
                cy /= count as f32;
                let ang = rng.unit() * std::f32::consts::TAU;
                let d = 4.0 + rng.unit() * 2.0;
                let A = c2Capsule {
                    a: c2v {
                        x: cx + d * ang.cos(),
                        y: cy + d * ang.sin(),
                    },
                    b: c2v {
                        x: cx + (d + 1.0) * ang.cos(),
                        y: cy + (d + 1.0) * ang.sin(),
                    },
                    r: 0.5 + rng.unit() * 6.0,
                };
                let bx = if has_bx { Some(rng.xform()) } else { None };
                acc.check(format!("shallow count={count} bx={has_bx} #{i}"), |s| {
                    let bxp = match &bx {
                        Some(x) => x as *const c2x,
                        None => std::ptr::null(),
                    };
                    with_sentinel(|m| c2CapsuletoPolyManifold(s, A, bxp_poly(&poly), bxp, m))
                });
            }
        }
    }
    acc.finish();
    eprintln!("cfg_capsule_poly: contact-count histogram = {counts:?}");
    assert!(counts.iter().all(|&n| n > 0), "cp counts 0/1/2 not all hit: {counts:?}");
}

fn bxp_poly(p: &c2Poly) -> *const c2Poly {
    p as *const c2Poly
}

// -------------------------------------------------------------------------
// rows 101, 102
// -------------------------------------------------------------------------
#[test]
fn cfg_capsule_poly_counts() {
    let mut acc = DiffAccum::new("cfg_capsule_poly_counts");
    let mut rng = Rng::new(0xaeed_0007);
    for count in 1..=8i32 {
        for i in 0..(N * 2) {
            let verts = rng.convex_poly_verts(count.max(1) as usize);
            let poly = make_poly(&verts, count);
            let A = rng.capsule();
            let bx = if rng.bool() { Some(rng.xform()) } else { None };
            acc.check(format!("count={count} #{i}"), |s| {
                let bxp = match &bx {
                    Some(x) => x as *const c2x,
                    None => std::ptr::null(),
                };
                with_sentinel(|m| c2CapsuletoPolyManifold(s, A, &poly, bxp, m))
            });
        }
    }
    acc.finish();
}

// -------------------------------------------------------------------------
// rows 104..108
// -------------------------------------------------------------------------
#[test]
fn cfg_aabb_capsule() {
    let mut acc = DiffAccum::new("cfg_aabb_capsule");
    let mut rng = Rng::new(0xaeed_0008);
    // A wide random sweep is what reaches the rare `index == ~0` path inside
    // `c2CapsuletoPolyManifold`, where the C indexes `p.verts[-1]` and therefore
    // reads the two 4-byte words that gcc places immediately below the `c2Poly`
    // local, i.e. `{A.max.y, p.count}`.  The Rust reproduces that with the
    // `AabbCapsuleFrame` struct; reordering its fields makes ~222 of 80 000
    // cases diverge, so this row is what pins the frame layout down.
    for i in 0..(N * 40) {
        let A = rng.aabb();
        let B = rng.capsule();
        acc.check(format!("rand #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, A, B, m))
        });
        // and through the two dispatchers, so the same path is covered from the
        // public API as well
        acc.check(format!("collide #{i}"), |s| {
            with_sentinel(|m| {
                c2Collide(
                    s,
                    &A as *const c2AABB as *const std::os::raw::c_void,
                    C2_TYPE_AABB,
                    &B as *const c2Capsule as *const std::os::raw::c_void,
                    C2_TYPE_CAPSULE,
                    m,
                )
            })
        });
        acc.check(format!("omni #{i}"), |s| {
            with_sentinel(|m| {
                omni_manifold(
                    s, m, C2_TYPE_AABB, A.min.x, A.min.y, A.max.x, A.max.y, 0.0,
                    C2_TYPE_CAPSULE, B.a.x, B.a.y, B.b.x, B.b.y, B.r,
                )
            })
        });
        acc.check(format!("omni-rev #{i}"), |s| {
            with_sentinel(|m| {
                omni_manifold(
                    s, m, C2_TYPE_CAPSULE, B.a.x, B.a.y, B.b.x, B.b.y, B.r,
                    C2_TYPE_AABB, A.min.x, A.min.y, A.max.x, A.max.y, 0.0,
                )
            })
        });
    }
    // capsule crossing the box
    for i in 0..(N * 2) {
        let e = 1.0 + rng.unit() * 2.0;
        let A = c2AABB {
            min: c2v { x: -e, y: -e },
            max: c2v { x: e, y: e },
        };
        let ang = rng.unit() * std::f32::consts::TAU;
        let len = e + 1.0 + rng.unit() * 3.0;
        let B = c2Capsule {
            a: c2v {
                x: -len * ang.cos(),
                y: -len * ang.sin(),
            },
            b: c2v {
                x: len * ang.cos(),
                y: len * ang.sin(),
            },
            r: 0.1 + rng.unit(),
        };
        acc.check(format!("cross #{i}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, A, B, m))
        });
    }
    // just outside, shallow radius branch
    for i in 0..(N * 2) {
        let e = 1.0 + rng.unit() * 2.0;
        let A = c2AABB {
            min: c2v { x: -e, y: -e },
            max: c2v { x: e, y: e },
        };
        let d = e + 1.0 + rng.unit() * 2.0;
        let B = c2Capsule {
            a: c2v { x: -1.0, y: d },
            b: c2v { x: 1.0, y: d },
            r: 0.5 + rng.unit() * 4.0,
        };
        acc.check(format!("shallow #{i}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, A, B, m))
        });
    }
    // separated (reject, but m->n is still negated)
    for i in 0..N {
        let A = c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        let B = c2Capsule {
            a: c2v { x: -1.0, y: 50.0 },
            b: c2v { x: 1.0, y: 50.0 },
            r: 0.5,
        };
        acc.check(format!("separated #{i}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, A, B, m))
        });
    }
    acc.finish();
}

#[test]
fn cfg_aabb_capsule_degenerate() {
    let mut acc = DiffAccum::new("cfg_aabb_capsule_degenerate");
    let mut rng = Rng::new(0xaeed_0009);
    // degenerate box (min == max) ⇒ NaN poly normals ⇒ index stays ~0 ⇒ verts[-1]
    for i in 0..(N * 2) {
        let p = rng.vec();
        let A = c2AABB { min: p, max: p };
        let B = rng.capsule();
        acc.check(format!("degenbox #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, A, B, m))
        });
    }
    // partially degenerate box (zero width or height)
    for i in 0..(N * 2) {
        let p = rng.vec();
        let A = if rng.bool() {
            c2AABB {
                min: p,
                max: c2v { x: p.x, y: p.y + 2.0 },
            }
        } else {
            c2AABB {
                min: p,
                max: c2v { x: p.x + 2.0, y: p.y },
            }
        };
        let B = rng.capsule();
        acc.check(format!("flatbox #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, A, B, m))
        });
    }
    // degenerate capsule (a == b)
    for i in 0..(N * 2) {
        let p = rng.vec();
        let B = c2Capsule {
            a: p,
            b: p,
            r: rng.radius(),
        };
        let A = rng.aabb();
        acc.check(format!("degencap #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, A, B, m))
        });
    }
    // inverted box
    for i in 0..N {
        let a = rng.vec();
        let b = rng.vec();
        let A = c2AABB {
            min: c2v {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
            max: c2v {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
        };
        let B = rng.capsule();
        acc.check(format!("invbox #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, A, B, m))
        });
    }
    // non-finite
    for i in 0..(N * 2) {
        let A = c2AABB {
            min: rng.special_vec(),
            max: rng.special_vec(),
        };
        let B = c2Capsule {
            a: rng.special_vec(),
            b: rng.special_vec(),
            r: rng.special(),
        };
        acc.check(format!("special #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, A, B, m))
        });
    }
    acc.finish();
}

// c2Norms is used by c2AABBtoCapsuleManifold internally; also exercise the
// exported version on box verts with count 8 (row 42 of ERRORS.md boundary).
#[test]
fn err_norms_count_boundary() {
    let mut acc = DiffAccum::new("err_norms_count_boundary");
    let mut rng = Rng::new(0xaeed_000a);
    for &count in &[0i32, -1, -100, 1, 8] {
        for i in 0..N {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.vec();
            }
            acc.check(format!("count={count} #{i}"), |s| {
                let mut v = verts;
                let mut n = [c2v { x: 3.5, y: -3.5 }; 8];
                c2Norms(s, v.as_mut_ptr(), n.as_mut_ptr(), count);
                (v.to_vec(), n.to_vec())
            });
        }
    }
    acc.finish();
}

#[test]
fn err_norms_nonpositive() {
    let mut acc = DiffAccum::new("err_norms_nonpositive");
    let mut rng = Rng::new(0xaeed_000b);
    for &count in &[0i32, -1, i32::MIN, -12345] {
        for i in 0..64 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.vec();
            }
            acc.check(format!("count={count} #{i}"), |s| {
                let mut v = verts;
                let mut n = [c2v { x: 1.25, y: -8.75 }; 8];
                c2Norms(s, v.as_mut_ptr(), n.as_mut_ptr(), count);
                (v.to_vec(), n.to_vec())
            });
        }
    }
    acc.finish();
}

#[test]
fn err_norms_degenerate() {
    let mut acc = DiffAccum::new("err_norms_degenerate");
    let mut rng = Rng::new(0xaeed_000c);
    for count in 1..=8i32 {
        for i in 0..N {
            let mut verts = [c2v::default(); 8];
            let p = rng.vec();
            for v in verts.iter_mut() {
                *v = p; // all identical ⇒ every edge zero ⇒ NaN normals
            }
            acc.check(format!("all-same count={count} #{i}"), |s| {
                let mut v = verts;
                let mut n = [c2v { x: 1.0, y: 2.0 }; 8];
                c2Norms(s, v.as_mut_ptr(), n.as_mut_ptr(), count);
                (v.to_vec(), n.to_vec())
            });
        }
    }
    acc.finish();
}

// c2BBVerts on degenerate input (ERRORS.md row 88)
#[test]
fn err_bbverts_degenerate() {
    let mut acc = DiffAccum::new("err_bbverts_degenerate");
    let mut rng = Rng::new(0xaeed_000d);
    for i in 0..(N * 2) {
        let p = rng.vec();
        let cases = [
            c2AABB { min: p, max: p },
            c2AABB {
                min: c2v { x: p.x, y: p.y },
                max: c2v { x: p.x - 1.0, y: p.y - 1.0 },
            },
            c2AABB {
                min: rng.special_vec(),
                max: rng.special_vec(),
            },
        ];
        for (k, bb) in cases.iter().enumerate() {
            let bb = *bb;
            acc.check(format!("#{i} case={k} {bb:?}"), |s| {
                let mut bb = bb;
                let mut out = [c2v { x: -1.5, y: 2.5 }; 6];
                c2BBVerts(s, out.as_mut_ptr(), &mut bb);
                (out.to_vec(), bb)
            });
        }
    }
    acc.finish();
}
