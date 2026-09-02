//! Phase B — CONFIGS.md rows 15..23: `c2BBVerts`, `c2MakeProxy`, `c2Support`,
//! `c2GJKSimplexMetric`.
//!
//! Out-params are pre-filled with a poison pattern so that a write the other
//! implementation does not make (or makes and shouldn't) is detected.

mod common;
use common::*;
use std::ffi::c_void;

const N: usize = 8_000;

type FnBBVerts = unsafe extern "C" fn(*mut C2v, *mut C2AABB);
type FnMakeProxy = unsafe extern "C" fn(*const c_void, i32, *mut C2Proxy);
type FnSupport = unsafe extern "C" fn(*const C2v, i32, C2v) -> i32;
type FnMetric = unsafe extern "C" fn(*mut C2Simplex) -> f32;

const POISON: f32 = f32::from_bits(0x5eed_face);

fn poison_verts() -> [C2v; 8] {
    let mut v = [C2v { x: POISON, y: POISON }; 8];
    for (i, s) in v.iter_mut().enumerate() {
        s.x = f32::from_bits(0x5eed_0000 | i as u32);
        s.y = f32::from_bits(0x5eed_1000 | i as u32);
    }
    v
}

fn poison_proxy() -> C2Proxy {
    C2Proxy {
        radius: POISON,
        count: -0x5eed,
        verts: poison_verts(),
    }
}

/// row 15 — c2BBVerts: valid, zero-extent, inverted, infinite bounds
#[test]
fn row15_bbverts() {
    let l = libs();
    let (c, r) = l.pair::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(0x5B_0015);

    let mut cases: Vec<C2AABB> = degenerate_aabbs();
    for _ in 0..N {
        cases.push(rng.aabb());
    }
    for _ in 0..N / 4 {
        // fully arbitrary (incl. inverted / NaN / inf)
        cases.push(C2AABB { min: rng.v_mixed(), max: rng.v_mixed() });
    }
    for _ in 0..N / 8 {
        cases.push(C2AABB { min: rng.v_any(), max: rng.v_any() });
    }

    for bb in cases {
        // 8 slots so an over-write past vert 3 would be caught.
        let mut cv = poison_verts();
        let mut rv = poison_verts();
        let mut cb = bb;
        let mut rb = bb;
        unsafe {
            c(cv.as_mut_ptr(), &mut cb);
            r(rv.as_mut_ptr(), &mut rb);
        }
        same("c2BBVerts out", &(bb.min, bb.max), &cv, &rv);
        // The C never modifies the AABB it reads.
        same("c2BBVerts input aliasing", &(bb.min, bb.max), &cb, &rb);
    }
}

/// rows 16,17,18 — c2MakeProxy for each of the three valid C2_TYPEs
#[test]
fn row16_17_18_makeproxy() {
    let l = libs();
    let (c, r) = l.pair::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(0x5B_0016);

    let mut shapes: Vec<(i32, ShapeBlob)> = Vec::new();
    for k in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        for i in 0..6 {
            shapes.push((k, ShapeBlob::degenerate(k, i)));
        }
        for _ in 0..N {
            shapes.push((k, ShapeBlob::random(&mut rng, k)));
        }
    }
    // radius sign / magnitude extremes for the radius-bearing shapes
    for &rad in SPECIAL {
        shapes.push((
            C2_TYPE_CIRCLE,
            ShapeBlob::circle(C2Circle { p: C2v { x: 1.0, y: 2.0 }, r: rad }),
        ));
        shapes.push((
            C2_TYPE_CAPSULE,
            ShapeBlob::capsule(C2Capsule {
                a: C2v { x: 1.0, y: 2.0 },
                b: C2v { x: 3.0, y: 4.0 },
                r: rad,
            }),
        ));
    }

    for (kind, blob) in shapes {
        let mut cp = poison_proxy();
        let mut rp = poison_proxy();
        unsafe {
            c(blob.ptr(), kind, &mut cp);
            r(blob.ptr(), kind, &mut rp);
        }
        same("c2MakeProxy", &(type_name(kind), blob), &cp, &rp);
    }
}

/// rows 20,21,22,23 — c2Support at counts 1, 2, 4, 8
#[test]
fn row20_23_support() {
    let l = libs();
    let (c, r) = l.pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(0x5B_0020);

    for &count in &[1i32, 2, 4, 8] {
        for i in 0..N {
            let mut verts = [C2v::default(); 8];
            match i % 5 {
                // Ties everywhere: `dot > dmax` is false, first index must win.
                0 => {
                    let v = rng.v_tame();
                    verts = [v; 8];
                }
                // Axis-aligned box verts -> axis-aligned directions produce ties.
                1 => {
                    let bb = rng.aabb();
                    verts[0] = bb.min;
                    verts[1] = C2v { x: bb.max.x, y: bb.min.y };
                    verts[2] = bb.max;
                    verts[3] = C2v { x: bb.min.x, y: bb.max.y };
                    for k in 4..8 {
                        verts[k] = rng.v_tame();
                    }
                }
                2 => {
                    for v in verts.iter_mut() {
                        *v = rng.v_any();
                    }
                }
                3 => {
                    for v in verts.iter_mut() {
                        *v = rng.v_mixed();
                    }
                }
                _ => {
                    for v in verts.iter_mut() {
                        *v = rng.v_tame();
                    }
                }
            }
            let d = match i % 7 {
                0 => C2v { x: 1.0, y: 0.0 },
                1 => C2v { x: 0.0, y: 1.0 },
                2 => C2v { x: -1.0, y: 0.0 },
                3 => C2v { x: 0.0, y: 0.0 },
                4 => rng.v_any(),
                5 => rng.v_mixed(),
                _ => rng.v_tame(),
            };
            let (ci, ri) = unsafe {
                (
                    c(verts.as_ptr(), count, d),
                    r(verts.as_ptr(), count, d),
                )
            };
            same_i32("c2Support", &(count, verts, d), ci, ri);
        }
    }
}

/// row 19 — c2GJKSimplexMetric at counts 1, 2, 3 (+ degenerate geometry)
#[test]
fn row19_simplex_metric() {
    let l = libs();
    let (c, r) = l.pair::<FnMetric>("c2GJKSimplexMetric");
    let mut rng = Rng::new(0x5B_0019);

    for &count in &[1i32, 2, 3] {
        for i in 0..N {
            let mut s = C2Simplex::default();
            s.count = count;
            s.div = rng.f32_mixed();
            match i % 6 {
                // collinear -> det == 0
                0 => {
                    let a = rng.v_tame();
                    let d = rng.v_tame();
                    s.v[0].p = a;
                    s.v[1].p = C2v { x: a.x + d.x, y: a.y + d.y };
                    s.v[2].p = C2v { x: a.x + 2.0 * d.x, y: a.y + 2.0 * d.y };
                }
                // all coincident
                1 => {
                    let a = rng.v_tame();
                    for k in 0..4 {
                        s.v[k].p = a;
                    }
                }
                2 => {
                    for k in 0..4 {
                        s.v[k].p = rng.v_any();
                    }
                }
                3 => {
                    for k in 0..4 {
                        s.v[k].p = rng.v_mixed();
                    }
                }
                _ => {
                    for k in 0..4 {
                        s.v[k].p = rng.v_tame();
                    }
                }
            }
            let mut cs = s;
            let mut rs = s;
            let (cm, rm) = unsafe { (c(&mut cs), r(&mut rs)) };
            same_f32("c2GJKSimplexMetric", &(count, s.v[0].p, s.v[1].p, s.v[2].p), cm, rm);
            // The function must not mutate the simplex.
            same("c2GJKSimplexMetric no-mutate", &count, &cs, &rs);
        }
    }
}
