//! Phase B — CONFIGS.md rows 22–28: `c2RaytoAABB` as a low-level entry point.
//!
//! Branch coverage is measured by replaying the C's own control flow through
//! the C library's exported primitives, so every rejection path and each of the
//! four winning-plane branches is proven to have been reached.

#![allow(non_snake_case)]

mod common;
use common::*;

const SEED: u64 = 0x5EED_C2A1;
const N: usize = 15_000;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
enum AabbBranch {
    RejectBBox,
    RejectSat,
    NoPlaneHit,
    PlaneMinX,
    PlaneMaxX,
    PlaneMinY,
    PlaneMaxY,
}

fn sel_lt(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}
fn sel_gt(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}
fn sel_abs(a: f32) -> f32 {
    if a < 0.0 { -a } else { a }
}

fn plane_dist(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

fn ray_to_plane_1d(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0
    } else {
        let d = da - db;
        if d != 0.0 { da / d } else { 0.0 }
    }
}

/// Faithful replay of `c2RaytoAABB`'s branch selection, using the C `.so` for
/// the vector primitives.
fn classify(A: c2Ray, B: c2AABB) -> AabbBranch {
    let l = libs();
    unsafe {
        let p0 = A.p;
        let p1 = (l.c.c2Add)(A.p, (l.c.c2Mulvs)(A.d, A.t));
        let a_box = c2AABB {
            min: (l.c.c2Minv)(p0, p1),
            max: (l.c.c2Maxv)(p0, p1),
        };
        if (l.c.c2AABBtoAABB)(a_box, B) == 0 {
            return AabbBranch::RejectBBox;
        }
        let ab = (l.c.c2Sub)(p1, p0);
        let n = (l.c.c2Skew)(ab);
        let abs_n = (l.c.c2Absv)(n);
        let half = (l.c.c2Mulvs)((l.c.c2Sub)(B.max, B.min), 0.5);
        let centre = (l.c.c2Mulvs)((l.c.c2Add)(B.min, B.max), 0.5);
        let dot = (l.c.c2Dot)(n, (l.c.c2Sub)(p0, centre));
        let d = sel_abs(dot) - (l.c.c2Dot)(abs_n, half);
        if d > 0.0 {
            return AabbBranch::RejectSat;
        }
        let t0 = ray_to_plane_1d(plane_dist(p0.x, -1.0, B.min.x), plane_dist(p1.x, -1.0, B.min.x));
        let t1 = ray_to_plane_1d(plane_dist(p0.x, 1.0, B.max.x), plane_dist(p1.x, 1.0, B.max.x));
        let t2 = ray_to_plane_1d(plane_dist(p0.y, -1.0, B.min.y), plane_dist(p1.y, -1.0, B.min.y));
        let t3 = ray_to_plane_1d(plane_dist(p0.y, 1.0, B.max.y), plane_dist(p1.y, 1.0, B.max.y));
        let (h0, h1, h2, h3) = (
            (t0 <= 1.0) as i32,
            (t1 <= 1.0) as i32,
            (t2 <= 1.0) as i32,
            (t3 <= 1.0) as i32,
        );
        if h0 | h1 | h2 | h3 == 0 {
            return AabbBranch::NoPlaneHit;
        }
        let t0 = h0 as f32 * t0;
        let t1 = h1 as f32 * t1;
        let t2 = h2 as f32 * t2;
        let t3 = h3 as f32 * t3;
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            AabbBranch::PlaneMinX
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            AabbBranch::PlaneMaxX
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            AabbBranch::PlaneMinY
        } else {
            AabbBranch::PlaneMaxY
        }
    }
}

fn call(ray: c2Ray, b: c2AABB) -> (i32, c2Raycast, i32, c2Raycast) {
    both_ray(|lib, r, s, o| unsafe { (lib.c2RaytoAABB)(r, s, o) }, ray, b)
}

fn fmt_case(ray: c2Ray, b: c2AABB) -> String {
    format!(
        "ray{{p:{} d:{} t:{}}} box{{min:{} max:{}}}",
        fmt_v(ray.p),
        fmt_v(ray.d),
        fmt_f(ray.t),
        fmt_v(b.min),
        fmt_v(b.max)
    )
}

fn run_batch(label: &str, cases: Vec<(c2Ray, c2AABB)>, required: &[AabbBranch]) {
    let mut d = Diff::new(label.to_string());
    let mut seen: std::collections::HashMap<AabbBranch, usize> = Default::default();
    for (ray, b) in cases {
        *seen.entry(classify(ray, b)).or_default() += 1;
        let (cr, co, rr, ro) = call(ray, b);
        d.check_ray(cr, co, rr, ro, || fmt_case(ray, b));
    }
    for br in required {
        assert!(
            seen.get(br).copied().unwrap_or(0) > 0,
            "{label}: branch {br:?} never exercised (coverage: {seen:?})"
        );
    }
    eprintln!("    coverage {label}: {seen:?}");
    d.finish();
}

fn norm(from: c2v, to: c2v) -> c2v {
    let l = libs();
    unsafe { (l.c.c2Norm)((l.c.c2Sub)(to, from)) }
}

fn rand_box(rng: &mut Rng) -> c2AABB {
    let cx = rng.coord();
    let cy = rng.coord();
    let hx = rng.range(0.01, 30.0);
    let hy = rng.range(0.01, 30.0);
    c2AABB {
        min: c2v { x: cx - hx, y: cy - hy },
        max: c2v { x: cx + hx, y: cy + hy },
    }
}

/// Row 22 — random rays × random boxes.
#[test]
fn cfg_22_raytoaabb_random() {
    let mut rng = Rng::new(SEED ^ 22);
    let mut cases = Vec::new();
    for _ in 0..N {
        let b = rand_box(&mut rng);
        let origin = rng.vec_coord();
        let target = c2v {
            x: rng.range(b.min.x - 10.0, b.max.x + 10.0),
            y: rng.range(b.min.y - 10.0, b.max.y + 10.0),
        };
        cases.push((
            c2Ray {
                p: origin,
                d: norm(origin, target),
                t: rng.range(0.0, 400.0),
            },
            b,
        ));
    }
    run_batch(
        "row22 c2RaytoAABB random",
        cases,
        &[
            AabbBranch::RejectBBox,
            AabbBranch::RejectSat,
            AabbBranch::PlaneMinX,
            AabbBranch::PlaneMaxX,
            AabbBranch::PlaneMinY,
            AabbBranch::PlaneMaxY,
        ],
    );
}

/// Row 23 — each winning-plane branch forced by approaching from each side.
#[test]
fn cfg_23_raytoaabb_each_plane() {
    let mut rng = Rng::new(SEED ^ 23);
    let mut cases = Vec::new();
    for _ in 0..(N / 8 + 1) {
        let b = rand_box(&mut rng);
        let cx = (b.min.x + b.max.x) * 0.5;
        let cy = (b.min.y + b.max.y) * 0.5;
        let w = b.max.x - b.min.x;
        let h = b.max.y - b.min.y;
        // Eight approach directions: 4 faces + 4 diagonals.
        let offs: [(f32, f32); 8] = [
            (-(w + 20.0), 0.0),
            (w + 20.0, 0.0),
            (0.0, -(h + 20.0)),
            (0.0, h + 20.0),
            (-(w + 20.0), -(h + 20.0)),
            (w + 20.0, -(h + 20.0)),
            (-(w + 20.0), h + 20.0),
            (w + 20.0, h + 20.0),
        ];
        for (ox, oy) in offs {
            let origin = c2v { x: cx + ox, y: cy + oy };
            let target = c2v {
                x: cx + rng.range(-w * 0.4, w * 0.4),
                y: cy + rng.range(-h * 0.4, h * 0.4),
            };
            let dx = target.x - origin.x;
            let dy = target.y - origin.y;
            let len = (dx * dx + dy * dy).sqrt();
            cases.push((
                c2Ray {
                    p: origin,
                    d: norm(origin, target),
                    t: len * rng.range(1.0, 2.0),
                },
                b,
            ));
        }
    }
    run_batch(
        "row23 c2RaytoAABB each plane",
        cases,
        &[
            AabbBranch::PlaneMinX,
            AabbBranch::PlaneMaxX,
            AabbBranch::PlaneMinY,
            AabbBranch::PlaneMaxY,
        ],
    );
}

/// Row 24 — exact-corner rays, which tie several `tN` and expose the `>=` chain
/// evaluation order.
#[test]
fn cfg_24_raytoaabb_corner_ties() {
    let mut rng = Rng::new(SEED ^ 24);
    let mut cases = Vec::new();
    for _ in 0..(N / 4 + 1) {
        let b = rand_box(&mut rng);
        let corners = [
            c2v { x: b.min.x, y: b.min.y },
            c2v { x: b.max.x, y: b.min.y },
            c2v { x: b.min.x, y: b.max.y },
            c2v { x: b.max.x, y: b.max.y },
        ];
        for corner in corners {
            let cx = (b.min.x + b.max.x) * 0.5;
            let cy = (b.min.y + b.max.y) * 0.5;
            // Fire straight through the corner from outside, along the diagonal.
            let dx = corner.x - cx;
            let dy = corner.y - cy;
            let origin = c2v {
                x: corner.x + dx,
                y: corner.y + dy,
            };
            let len = ((corner.x - origin.x).powi(2) + (corner.y - origin.y).powi(2)).sqrt();
            for tmul in [0.5f32, 1.0, 1.5, 3.0] {
                cases.push((
                    c2Ray {
                        p: origin,
                        d: norm(origin, corner),
                        t: len * tmul,
                    },
                    b,
                ));
            }
            // Exactly symmetric box + 45-degree ray => exact ties.
            let sq = c2AABB {
                min: c2v { x: -1.0, y: -1.0 },
                max: c2v { x: 1.0, y: 1.0 },
            };
            let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
            for &(sx, sy) in &[(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                cases.push((
                    c2Ray {
                        p: c2v { x: -3.0 * sx, y: -3.0 * sy },
                        d: c2v { x: inv_sqrt2 * sx, y: inv_sqrt2 * sy },
                        t: 10.0,
                    },
                    sq,
                ));
            }
        }
    }
    run_batch(
        "row24 c2RaytoAABB corner ties",
        cases,
        &[AabbBranch::PlaneMinX, AabbBranch::PlaneMaxX],
    );
}

/// Row 25 — axis-aligned rays: `da == db` on two planes ⇒ the `d != 0` guard in
/// `c2RayToPlane_OneDimensional`.
#[test]
fn cfg_25_raytoaabb_axis_aligned() {
    let mut rng = Rng::new(SEED ^ 25);
    let mut cases = Vec::new();
    let dirs = [
        c2v { x: 1.0, y: 0.0 },
        c2v { x: -1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: 0.0, y: -1.0 },
        c2v { x: 1.0, y: -0.0 },
        c2v { x: -0.0, y: 1.0 },
    ];
    for _ in 0..(N / dirs.len() + 1) {
        let b = rand_box(&mut rng);
        let cx = (b.min.x + b.max.x) * 0.5;
        let cy = (b.min.y + b.max.y) * 0.5;
        for d in dirs {
            // Origin on the exact centre line, and also exactly on each edge
            // line, on BOTH sides of the box so the `+x`/`+y` planes can win
            // too (the `>=` chain favours earlier planes on a tie).
            for (ox, oy) in [
                (cx - 50.0, cy),
                (cx + 50.0, cy),
                (cx, cy - 50.0),
                (cx, cy + 50.0),
                (cx - 50.0, b.min.y),
                (cx + 50.0, b.max.y),
                (b.min.x, cy - 50.0),
                (b.max.x, cy + 50.0),
            ] {
                cases.push((
                    c2Ray {
                        p: c2v { x: ox, y: oy },
                        d,
                        t: rng.range(0.0, 150.0),
                    },
                    b,
                ));
            }
        }
    }
    run_batch(
        "row25 c2RaytoAABB axis-aligned",
        cases,
        &[
            AabbBranch::PlaneMinX,
            AabbBranch::PlaneMaxX,
            AabbBranch::PlaneMinY,
            AabbBranch::PlaneMaxY,
            AabbBranch::RejectBBox,
        ],
    );
}

/// Row 26 — the SAT rejection: ray bbox overlaps the box but the ray line misses.
#[test]
fn cfg_26_raytoaabb_sat_reject() {
    let mut rng = Rng::new(SEED ^ 26);
    let mut cases = Vec::new();
    let mut tries = 0;
    while cases.len() < N && tries < N * 60 {
        tries += 1;
        let b = rand_box(&mut rng);
        // A ray from one corner region of the box's bbox to the diagonally
        // opposite one, offset so the line passes outside a corner.
        let origin = c2v {
            x: rng.range(b.min.x - 20.0, b.max.x + 20.0),
            y: rng.range(b.min.y - 20.0, b.max.y + 20.0),
        };
        let target = c2v {
            x: rng.range(b.min.x - 20.0, b.max.x + 20.0),
            y: rng.range(b.min.y - 20.0, b.max.y + 20.0),
        };
        let len = ((target.x - origin.x).powi(2) + (target.y - origin.y).powi(2)).sqrt();
        let ray = c2Ray {
            p: origin,
            d: norm(origin, target),
            t: len,
        };
        if classify(ray, b) == AabbBranch::RejectSat {
            cases.push((ray, b));
        }
    }
    assert!(cases.len() > 1000, "only found {} SAT-reject cases", cases.len());
    run_batch("row26 c2RaytoAABB SAT reject", cases, &[AabbBranch::RejectSat]);
}

/// Row 27 — zero-length ray, degenerate box (`min == max`), inverted box.
#[test]
fn cfg_27_raytoaabb_degenerate() {
    let mut rng = Rng::new(SEED ^ 27);
    let mut cases = Vec::new();
    for _ in 0..(N / 6 + 1) {
        let b = rand_box(&mut rng);
        let cx = (b.min.x + b.max.x) * 0.5;
        let cy = (b.min.y + b.max.y) * 0.5;

        // Zero-length ray, inside and outside.
        for p in [
            c2v { x: cx, y: cy },
            c2v { x: b.min.x, y: b.min.y },
            c2v { x: b.max.x + 5.0, y: cy },
        ] {
            cases.push((
                c2Ray {
                    p,
                    d: rng.vec_coord(),
                    t: 0.0,
                },
                b,
            ));
            cases.push((
                c2Ray {
                    p,
                    d: rng.vec_coord(),
                    t: -0.0,
                },
                b,
            ));
        }
        // Degenerate box: a point, a horizontal segment, a vertical segment.
        let pt = c2AABB {
            min: c2v { x: cx, y: cy },
            max: c2v { x: cx, y: cy },
        };
        let hseg = c2AABB {
            min: c2v { x: b.min.x, y: cy },
            max: c2v { x: b.max.x, y: cy },
        };
        let vseg = c2AABB {
            min: c2v { x: cx, y: b.min.y },
            max: c2v { x: cx, y: b.max.y },
        };
        // Inverted box.
        let inv = c2AABB { min: b.max, max: b.min };
        let origin = c2v { x: cx - 60.0, y: cy + rng.range(-2.0, 2.0) };
        let target = c2v { x: cx, y: cy };
        let ray = c2Ray {
            p: origin,
            d: norm(origin, target),
            t: 120.0,
        };
        for bb in [pt, hseg, vseg, inv] {
            cases.push((ray, bb));
        }
    }
    run_batch(
        "row27 c2RaytoAABB degenerate",
        cases,
        &[AabbBranch::RejectBBox],
    );
}

/// Row 28 — `A.t` sweep and NaN in `A.d`, plus a full special-class field sweep
/// and arbitrary bit patterns in all 9 fields.
#[test]
fn cfg_28_raytoaabb_t_sweep_and_specials() {
    let mut rng = Rng::new(SEED ^ 28);
    let mut d = Diff::new("row28 c2RaytoAABB A.t sweep + specials");

    let ts: &[f32] = &[
        0.0, -0.0, 1e-6, 1e-45, 1.0, 1e6, f32::MAX, f32::INFINITY, -1.0,
        f32::NEG_INFINITY, f32::NAN,
    ];
    for _ in 0..(N / ts.len() + 1) {
        let b = rand_box(&mut rng);
        let cx = (b.min.x + b.max.x) * 0.5;
        let cy = (b.min.y + b.max.y) * 0.5;
        let origin = rng.vec_coord();
        let dir = norm(origin, c2v { x: cx, y: cy });
        for &t in ts {
            let ray = c2Ray { p: origin, d: dir, t };
            let (cr, co, rr, ro) = call(ray, b);
            d.check_ray(cr, co, rr, ro, || fmt_case(ray, b));
        }
        // NaN direction: `c2Norm` of a zero vector, the real source in gen_ray.
        let nan_dir = unsafe { (libs().c.c2Norm)(c2v { x: 0.0, y: 0.0 }) };
        let ray = c2Ray { p: origin, d: nan_dir, t: 10.0 };
        let (cr, co, rr, ro) = call(ray, b);
        d.check_ray(cr, co, rr, ro, || fmt_case(ray, b));
    }

    // One special class into each of the 9 fields of a hitting configuration.
    let base_ray = c2Ray {
        p: c2v { x: -10.0, y: 0.5 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 100.0,
    };
    let base_box = c2AABB {
        min: c2v { x: -2.0, y: -3.0 },
        max: c2v { x: 4.0, y: 5.0 },
    };
    let mut inject: Vec<f32> = SPECIALS.to_vec();
    inject.extend(NAN_BITS.iter().map(|&b| f32::from_bits(b)));
    for s in inject {
        for field in 0..9 {
            let mut ray = base_ray;
            let mut b = base_box;
            match field {
                0 => ray.p.x = s,
                1 => ray.p.y = s,
                2 => ray.d.x = s,
                3 => ray.d.y = s,
                4 => ray.t = s,
                5 => b.min.x = s,
                6 => b.min.y = s,
                7 => b.max.x = s,
                _ => b.max.y = s,
            }
            let (cr, co, rr, ro) = call(ray, b);
            d.check_ray(cr, co, rr, ro, || fmt_case(ray, b));
        }
    }
    // Arbitrary bit patterns everywhere.
    for _ in 0..N {
        let ray = c2Ray {
            p: rng.vec_spicy(),
            d: rng.vec_spicy(),
            t: rng.spicy(),
        };
        let b = c2AABB {
            min: rng.vec_spicy(),
            max: rng.vec_spicy(),
        };
        let (cr, co, rr, ro) = call(ray, b);
        d.check_ray(cr, co, rr, ro, || fmt_case(ray, b));
    }
    d.finish();
}
