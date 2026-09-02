//! Phase B — CONFIGS.md rows 16–21: `c2RaytoCircle`, called directly as the
//! low-level entry point (not only through `c2CastRay` / `gen_ray`).
//!
//! Branch coverage is *measured*, not assumed: the test recomputes the C's own
//! branch predicate using the C library's exported primitives (`c2Sub`,
//! `c2Dot`, …) and asserts every branch was actually reached.

#![allow(non_snake_case)]

mod common;
use common::*;

const SEED: u64 = 0x5EED_C2A1;
const N: usize = 15_000;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
enum CircleBranch {
    MissDisc,
    RejectTNegative,
    RejectTBeyond,
    Hit,
    Nan,
}

/// Replays `c2RaytoCircle`'s control flow using the **C** library's exported
/// primitives so the classification cannot drift from the C's arithmetic.
fn classify(ray: c2Ray, circle: c2Circle) -> CircleBranch {
    let l = libs();
    unsafe {
        let m = (l.c.c2Sub)(ray.p, circle.p);
        let c = (l.c.c2Dot)(m, m) - circle.r * circle.r;
        let b = (l.c.c2Dot)(m, ray.d);
        let disc = b * b - c;
        if disc.is_nan() {
            return CircleBranch::Nan;
        }
        if disc < 0.0 {
            return CircleBranch::MissDisc;
        }
        let t = -b - disc.sqrt();
        if t < 0.0 {
            CircleBranch::RejectTNegative
        } else if !(t <= ray.t) {
            CircleBranch::RejectTBeyond
        } else {
            CircleBranch::Hit
        }
    }
}

fn call(ray: c2Ray, circle: c2Circle) -> (i32, c2Raycast, i32, c2Raycast) {
    both_ray(|lib, r, s, o| unsafe { (lib.c2RaytoCircle)(r, s, o) }, ray, circle)
}

fn fmt_case(ray: c2Ray, c: c2Circle) -> String {
    format!(
        "ray{{p:{} d:{} t:{}}} circle{{p:{} r:{}}}",
        fmt_v(ray.p),
        fmt_v(ray.d),
        fmt_f(ray.t),
        fmt_v(c.p),
        fmt_f(c.r)
    )
}

fn normalized(from: c2v, to: c2v) -> c2v {
    let l = libs();
    unsafe { (l.c.c2Norm)((l.c.c2Sub)(to, from)) }
}

/// Runs a batch, records branch coverage and diffs every case.
fn run_batch(
    label: &str,
    cases: Vec<(c2Ray, c2Circle)>,
    required: &[CircleBranch],
) {
    let mut d = Diff::new(label.to_string());
    let mut seen: std::collections::HashMap<CircleBranch, usize> = Default::default();
    for (ray, c) in cases {
        *seen.entry(classify(ray, c)).or_default() += 1;
        let (cr, co, rr, ro) = call(ray, c);
        d.check_ray(cr, co, rr, ro, || fmt_case(ray, c));
    }
    for b in required {
        let n = seen.get(b).copied().unwrap_or(0);
        assert!(
            n > 0,
            "{label}: branch {b:?} was never exercised (coverage: {seen:?})"
        );
    }
    eprintln!("    coverage {label}: {seen:?}");
    d.finish();
}

/// Row 16 — random rays × random circles, general distribution.
#[test]
fn cfg_16_raytocircle_random() {
    let mut rng = Rng::new(SEED ^ 16);
    let mut cases = Vec::new();
    for _ in 0..N {
        let c = c2Circle {
            p: rng.vec_coord(),
            r: rng.radius(),
        };
        let origin = rng.vec_coord();
        // Aim at a point near the circle so hits and misses are both common.
        let target = c2v {
            x: c.p.x + rng.range(-c.r * 2.0, c.r * 2.0),
            y: c.p.y + rng.range(-c.r * 2.0, c.r * 2.0),
        };
        let dir = normalized(origin, target);
        let ray = c2Ray {
            p: origin,
            d: dir,
            t: rng.range(0.0, 300.0),
        };
        cases.push((ray, c));
    }
    run_batch(
        "row16 c2RaytoCircle random",
        cases,
        &[
            CircleBranch::MissDisc,
            CircleBranch::RejectTNegative,
            CircleBranch::RejectTBeyond,
            CircleBranch::Hit,
        ],
    );
}

/// Row 17 — the `return 1` path, densely: `out->t` and `out->n` bits.
#[test]
fn cfg_17_raytocircle_hits() {
    let mut rng = Rng::new(SEED ^ 17);
    let mut cases = Vec::new();
    while cases.len() < N {
        let c = c2Circle {
            p: rng.vec_coord(),
            r: rng.radius(),
        };
        // Origin outside, aimed at a point inside -> guaranteed forward hit.
        let ang = rng.range(0.0, 6.283_185_5);
        let dist = c.r + rng.range(0.1, 80.0);
        let origin = c2v {
            x: c.p.x + dist * ang.cos(),
            y: c.p.y + dist * ang.sin(),
        };
        let inner_ang = rng.range(0.0, 6.283_185_5);
        let inner_r = rng.range(0.0, c.r * 0.9);
        let target = c2v {
            x: c.p.x + inner_r * inner_ang.cos(),
            y: c.p.y + inner_r * inner_ang.sin(),
        };
        let ray = c2Ray {
            p: origin,
            d: normalized(origin, target),
            t: dist + c.r * 2.0,
        };
        if classify(ray, c) == CircleBranch::Hit {
            cases.push((ray, c));
        }
    }
    run_batch("row17 c2RaytoCircle hit path", cases, &[CircleBranch::Hit]);
}

/// Row 18 — `A.d` deliberately NOT normalised (the C never requires it).
#[test]
fn cfg_18_raytocircle_unnormalized_dir() {
    let mut rng = Rng::new(SEED ^ 18);
    let mut cases = Vec::new();
    for _ in 0..N {
        let c = c2Circle {
            p: rng.vec_coord(),
            r: rng.radius(),
        };
        let origin = rng.vec_coord();
        let scale = rng.range(0.001, 50.0);
        let base = normalized(origin, c.p);
        let dir = c2v {
            x: base.x * scale,
            y: base.y * scale,
        };
        cases.push((
            c2Ray {
                p: origin,
                d: dir,
                t: rng.range(0.0, 300.0),
            },
            c,
        ));
    }
    // Also raw non-unit directions with no relation to the circle.
    for _ in 0..N {
        cases.push((
            c2Ray {
                p: rng.vec_coord(),
                d: rng.vec_coord(),
                t: rng.range(-10.0, 300.0),
            },
            c2Circle {
                p: rng.vec_coord(),
                r: rng.radius(),
            },
        ));
    }
    run_batch(
        "row18 c2RaytoCircle unnormalized dir",
        cases,
        &[
            CircleBranch::MissDisc,
            CircleBranch::RejectTNegative,
            CircleBranch::Hit,
        ],
    );
}

/// Row 19 — `A.t` sweep including 0, inf and negative.
#[test]
fn cfg_19_raytocircle_t_sweep() {
    let mut rng = Rng::new(SEED ^ 19);
    let ts: &[f32] = &[
        0.0,
        -0.0,
        1e-6,
        1e-45,
        1.0,
        1e6,
        f32::MAX,
        f32::INFINITY,
        -1.0,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let mut cases = Vec::new();
    for _ in 0..(N / ts.len() + 1) {
        let c = c2Circle {
            p: rng.vec_coord(),
            r: rng.radius(),
        };
        let origin = rng.vec_coord();
        // Aim near, not exactly at, the centre so the `disc < 0` branch is also
        // reachable while `A.t` is swept.
        let target = c2v {
            x: c.p.x + rng.range(-c.r * 2.0, c.r * 2.0),
            y: c.p.y + rng.range(-c.r * 2.0, c.r * 2.0),
        };
        let dir = normalized(origin, target);
        for &t in ts {
            cases.push((c2Ray { p: origin, d: dir, t }, c));
        }
    }
    run_batch(
        "row19 c2RaytoCircle A.t sweep",
        cases,
        &[
            CircleBranch::RejectTBeyond,
            CircleBranch::Hit,
            CircleBranch::MissDisc,
        ],
    );
}

/// Row 20 — tangent (`disc ≈ 0`) and origin-exactly-on-circle (`c == 0`).
#[test]
fn cfg_20_raytocircle_tangent_and_boundary() {
    let mut rng = Rng::new(SEED ^ 20);
    let mut cases = Vec::new();

    for _ in 0..N / 3 {
        let c = c2Circle {
            p: rng.vec_coord(),
            r: rng.radius(),
        };
        // Tangent: origin offset perpendicular by exactly r, direction parallel.
        let ang = rng.range(0.0, 6.283_185_5);
        let (ca, sa) = (ang.cos(), ang.sin());
        let dist = rng.range(1.0, 60.0);
        for scale in [
            0.999_999_9f32,
            1.0,
            1.000_000_1,
            0.99,
            1.01,
        ] {
            let origin = c2v {
                x: c.p.x + c.r * scale * ca - dist * sa,
                y: c.p.y + c.r * scale * sa + dist * ca,
            };
            let dir = c2v { x: sa, y: ca };
            cases.push((
                c2Ray {
                    p: origin,
                    d: dir,
                    t: dist * 2.0,
                },
                c,
            ));
        }
        // Origin exactly on the circle -> c == 0 -> t == -b - |b|.
        let on = c2v {
            x: c.p.x + c.r * ca,
            y: c.p.y + c.r * sa,
        };
        for &(dx, dy) in &[(ca, sa), (-ca, -sa), (-sa, ca), (sa, -ca)] {
            cases.push((
                c2Ray {
                    p: on,
                    d: c2v { x: dx, y: dy },
                    t: rng.range(0.0, 100.0),
                },
                c,
            ));
        }
        // Origin exactly at the centre.
        cases.push((
            c2Ray {
                p: c.p,
                d: c2v { x: ca, y: sa },
                t: rng.range(0.0, 100.0),
            },
            c,
        ));
    }
    run_batch(
        "row20 c2RaytoCircle tangent/boundary",
        cases,
        &[
            CircleBranch::Hit,
            CircleBranch::RejectTNegative,
            CircleBranch::MissDisc,
        ],
    );
}

/// Row 21 — degenerate output normal: `c2Norm(impact - p)` with `impact == p`
/// (zero-radius circle hit dead centre) ⇒ NaN normal must match bit-for-bit.
/// Plus special float classes injected into every field.
#[test]
fn cfg_21_raytocircle_degenerate_and_specials() {
    let mut rng = Rng::new(SEED ^ 21);
    let mut d = Diff::new("row21 c2RaytoCircle degenerate/specials");

    // Zero-radius circle, ray aimed exactly at its centre: disc == b*b - dot,
    // and on a hit `impact == p` so `c2Norm(0,0)` -> (NaN, NaN).
    for i in 0..2000 {
        let cp = c2v {
            x: rng.coord(),
            y: rng.coord(),
        };
        let origin = c2v {
            x: cp.x - (i as f32 % 17.0 + 1.0),
            y: cp.y,
        };
        let ray = c2Ray {
            p: origin,
            d: c2v { x: 1.0, y: 0.0 },
            t: 1000.0,
        };
        for r in [0.0f32, -0.0, 1e-45, -1e-45] {
            let c = c2Circle { p: cp, r };
            let (cr, co, rr, ro) = call(ray, c);
            d.check_ray(cr, co, rr, ro, || fmt_case(ray, c));
        }
    }

    // Every field, every special class, one at a time on top of a hitting case.
    let base_ray = c2Ray {
        p: c2v { x: -10.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 100.0,
    };
    let base_c = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 3.0,
    };
    for &s in SPECIALS {
        for field in 0..8 {
            let mut ray = base_ray;
            let mut c = base_c;
            match field {
                0 => ray.p.x = s,
                1 => ray.p.y = s,
                2 => ray.d.x = s,
                3 => ray.d.y = s,
                4 => ray.t = s,
                5 => c.p.x = s,
                6 => c.p.y = s,
                _ => c.r = s,
            }
            let (cr, co, rr, ro) = call(ray, c);
            d.check_ray(cr, co, rr, ro, || fmt_case(ray, c));
        }
    }
    // Distinguishable NaN payloads in each field.
    for &nb in NAN_BITS {
        let s = f32::from_bits(nb);
        for field in 0..8 {
            let mut ray = base_ray;
            let mut c = base_c;
            match field {
                0 => ray.p.x = s,
                1 => ray.p.y = s,
                2 => ray.d.x = s,
                3 => ray.d.y = s,
                4 => ray.t = s,
                5 => c.p.x = s,
                6 => c.p.y = s,
                _ => c.r = s,
            }
            let (cr, co, rr, ro) = call(ray, c);
            d.check_ray(cr, co, rr, ro, || fmt_case(ray, c));
        }
    }
    // Fully arbitrary bit patterns in all 8 fields (ABI + NaN stress).
    for _ in 0..N {
        let ray = c2Ray {
            p: rng.vec_spicy(),
            d: rng.vec_spicy(),
            t: rng.spicy(),
        };
        let c = c2Circle {
            p: rng.vec_spicy(),
            r: rng.spicy(),
        };
        let (cr, co, rr, ro) = call(ray, c);
        d.check_ray(cr, co, rr, ro, || fmt_case(ray, c));
    }
    d.finish();
}
