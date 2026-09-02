//! Phase B — CONFIGS.md rows 29–41: `c2RaytoCapsule`, the branchiest function
//! in the library, driven directly as a low-level entry point.
//!
//! Note `c2RaytoCapsule` writes `out->n` and `out->t` UNCONDITIONALLY before any
//! test, so even the "no hit" cases have an observable out-parameter. The
//! harness poisons the out-param before every call and compares it in all
//! cases, hit or miss.

#![allow(non_snake_case)]

mod common;
use common::*;

const SEED: u64 = 0x5EED_C2A1;
const N: usize = 12_000;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
enum CapBranch {
    EarlySlab,
    EarlyCapA,
    EarlyCapB,
    LateralCircleA,
    LateralCircleB,
    DelegateCircleA,
    DelegateCircleB,
    SideMx,
    SideSkew,
    FallThrough,
}

fn sel_lt(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}
fn sel_abs(a: f32) -> f32 {
    if a < 0.0 { -a } else { a }
}

/// Replays `c2RaytoCapsule`'s branch selection using the C library's exported
/// primitives, so the classification uses the C's own arithmetic.
fn classify(A: c2Ray, B: c2Capsule) -> CapBranch {
    let l = libs();
    unsafe {
        let My = (l.c.c2Norm)((l.c.c2Sub)(B.b, B.a));
        let Mx = (l.c.c2CCW90)(My);
        let M = c2m { x: Mx, y: My };
        let cap_n = (l.c.c2Sub)(B.b, B.a);
        let yBb = (l.c.c2MulmvT)(M, cap_n);
        let yAp = (l.c.c2MulmvT)(M, (l.c.c2Sub)(A.p, B.a));
        let yAd = (l.c.c2MulmvT)(M, A.d);
        let yAe = (l.c.c2Add)(yAp, (l.c.c2Mulvs)(yAd, A.t));
        let bb = c2AABB {
            min: (l.c.c2V)(-B.r, 0.0),
            max: (l.c.c2V)(B.r, yBb.y),
        };
        if (l.c.c2AABBtoPoint)(bb, yAp) != 0 {
            return CapBranch::EarlySlab;
        }
        if (l.c.c2CircleToPoint)(c2Circle { p: B.a, r: B.r }, A.p) != 0 {
            return CapBranch::EarlyCapA;
        }
        if (l.c.c2CircleToPoint)(c2Circle { p: B.b, r: B.r }, A.p) != 0 {
            return CapBranch::EarlyCapB;
        }
        if yAe.x * yAp.x < 0.0 || sel_lt(sel_abs(yAe.x), sel_abs(yAp.x)) < B.r {
            if sel_abs(yAp.x) < B.r {
                if yAp.y < 0.0 {
                    CapBranch::LateralCircleA
                } else {
                    CapBranch::LateralCircleB
                }
            } else {
                let c = if yAp.x > 0.0 { B.r } else { -B.r };
                let dd = yAe.x - yAp.x;
                let t = (c - yAp.x) / dd;
                let y = yAp.y + (yAe.y - yAp.y) * t;
                if y <= 0.0 {
                    CapBranch::DelegateCircleA
                } else if y >= yBb.y {
                    CapBranch::DelegateCircleB
                } else if c > 0.0 {
                    CapBranch::SideMx
                } else {
                    CapBranch::SideSkew
                }
            }
        } else {
            CapBranch::FallThrough
        }
    }
}

fn call(ray: c2Ray, cap: c2Capsule) -> (i32, c2Raycast, i32, c2Raycast) {
    both_ray(|lib, r, s, o| unsafe { (lib.c2RaytoCapsule)(r, s, o) }, ray, cap)
}

fn fmt_case(ray: c2Ray, c: c2Capsule) -> String {
    format!(
        "ray{{p:{} d:{} t:{}}} cap{{a:{} b:{} r:{}}}",
        fmt_v(ray.p),
        fmt_v(ray.d),
        fmt_f(ray.t),
        fmt_v(c.a),
        fmt_v(c.b),
        fmt_f(c.r)
    )
}

fn run_batch(label: &str, cases: Vec<(c2Ray, c2Capsule)>, required: &[CapBranch]) {
    let mut d = Diff::new(label.to_string());
    let mut seen: std::collections::HashMap<CapBranch, usize> = Default::default();
    for (ray, c) in cases {
        *seen.entry(classify(ray, c)).or_default() += 1;
        let (cr, co, rr, ro) = call(ray, c);
        d.check_ray(cr, co, rr, ro, || fmt_case(ray, c));
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

fn rand_capsule(rng: &mut Rng) -> c2Capsule {
    let a = rng.vec_coord();
    let ang = rng.range(0.0, 6.283_185_5);
    let len = rng.range(0.5, 60.0);
    c2Capsule {
        a,
        b: c2v {
            x: a.x + len * ang.cos(),
            y: a.y + len * ang.sin(),
        },
        r: rng.radius(),
    }
}

/// Random rays around a capsule, biased to produce a spread of branches.
fn rand_case(rng: &mut Rng) -> (c2Ray, c2Capsule) {
    let cap = rand_capsule(rng);
    let mid = c2v {
        x: (cap.a.x + cap.b.x) * 0.5,
        y: (cap.a.y + cap.b.y) * 0.5,
    };
    let ang = rng.range(0.0, 6.283_185_5);
    let dist = rng.range(0.0, 120.0);
    let origin = c2v {
        x: mid.x + dist * ang.cos(),
        y: mid.y + dist * ang.sin(),
    };
    // Aim at a random point along/around the capsule axis.
    let s = rng.range(-0.4, 1.4);
    let target = c2v {
        x: cap.a.x + (cap.b.x - cap.a.x) * s + rng.range(-cap.r * 3.0, cap.r * 3.0),
        y: cap.a.y + (cap.b.y - cap.a.y) * s + rng.range(-cap.r * 3.0, cap.r * 3.0),
    };
    (
        c2Ray {
            p: origin,
            d: norm(origin, target),
            t: rng.range(0.0, 250.0),
        },
        cap,
    )
}

/// Row 29 — broad random sweep. Requires the common branches to appear.
#[test]
fn cfg_29_raytocapsule_random() {
    let mut rng = Rng::new(SEED ^ 29);
    let mut cases = Vec::new();
    for _ in 0..N * 3 {
        cases.push(rand_case(&mut rng));
    }
    run_batch(
        "row29 c2RaytoCapsule random",
        cases,
        &[
            CapBranch::EarlySlab,
            CapBranch::LateralCircleA,
            CapBranch::LateralCircleB,
            CapBranch::DelegateCircleA,
            CapBranch::DelegateCircleB,
            CapBranch::SideMx,
            CapBranch::SideSkew,
            CapBranch::FallThrough,
        ],
    );
}

/// Rows 30 & 31 — the three early `return 1` paths: origin inside the axis slab,
/// origin inside end-cap A, origin inside end-cap B.
#[test]
fn cfg_30_31_raytocapsule_early_returns() {
    let mut rng = Rng::new(SEED ^ 30);
    let mut cases = Vec::new();
    let mut guard = 0;
    let (mut n_slab, mut n_a, mut n_b) = (0, 0, 0);
    while (n_slab < N / 3 || n_a < N / 6 || n_b < N / 6) && guard < N * 200 {
        guard += 1;
        let cap = rand_capsule(&mut rng);
        // Place the origin inside the capsule volume: along the axis, laterally
        // offset by < r.
        let s = rng.range(-0.2, 1.2);
        let axis = c2v { x: cap.b.x - cap.a.x, y: cap.b.y - cap.a.y };
        let alen = (axis.x * axis.x + axis.y * axis.y).sqrt();
        let (ux, uy) = (axis.x / alen, axis.y / alen);
        let lat = rng.range(-cap.r * 0.95, cap.r * 0.95);
        let origin = c2v {
            x: cap.a.x + axis.x * s + uy * lat,
            y: cap.a.y + axis.y * s - ux * lat,
        };
        let ang = rng.range(0.0, 6.283_185_5);
        let ray = c2Ray {
            p: origin,
            d: c2v { x: ang.cos(), y: ang.sin() },
            t: rng.range(0.0, 200.0),
        };
        match classify(ray, cap) {
            CapBranch::EarlySlab if n_slab < N / 3 => {
                n_slab += 1;
                cases.push((ray, cap));
            }
            CapBranch::EarlyCapA if n_a < N / 6 => {
                n_a += 1;
                cases.push((ray, cap));
            }
            CapBranch::EarlyCapB if n_b < N / 6 => {
                n_b += 1;
                cases.push((ray, cap));
            }
            _ => {}
        }
    }
    run_batch(
        "row30/31 c2RaytoCapsule early returns",
        cases,
        &[CapBranch::EarlySlab, CapBranch::EarlyCapA, CapBranch::EarlyCapB],
    );
}

/// Rows 32 & 33 — `|yAp.x| < B.r` delegating to end-cap A (`yAp.y < 0`) or B.
#[test]
fn cfg_32_33_raytocapsule_lateral_delegate() {
    let mut rng = Rng::new(SEED ^ 32);
    let mut cases = Vec::new();
    let mut guard = 0;
    let (mut na, mut nb) = (0, 0);
    while (na < N / 2 || nb < N / 2) && guard < N * 200 {
        guard += 1;
        let cap = rand_capsule(&mut rng);
        let axis = c2v { x: cap.b.x - cap.a.x, y: cap.b.y - cap.a.y };
        let alen = (axis.x * axis.x + axis.y * axis.y).sqrt();
        let (ux, uy) = (axis.x / alen, axis.y / alen);
        // Origin on the axis line but axially OUTSIDE the slab (so the early
        // slab/cap tests fail) and laterally within r.
        let s = if rng.bool() {
            rng.range(-3.0, -0.02)
        } else {
            rng.range(1.02, 4.0)
        };
        let lat = rng.range(-cap.r * 0.99, cap.r * 0.99);
        let origin = c2v {
            x: cap.a.x + axis.x * s + uy * lat,
            y: cap.a.y + axis.y * s - ux * lat,
        };
        let ang = rng.range(0.0, 6.283_185_5);
        let ray = c2Ray {
            p: origin,
            d: c2v { x: ang.cos(), y: ang.sin() },
            t: rng.range(0.0, 200.0),
        };
        match classify(ray, cap) {
            CapBranch::LateralCircleA if na < N / 2 => {
                na += 1;
                cases.push((ray, cap));
            }
            CapBranch::LateralCircleB if nb < N / 2 => {
                nb += 1;
                cases.push((ray, cap));
            }
            _ => {}
        }
    }
    run_batch(
        "row32/33 c2RaytoCapsule lateral delegate",
        cases,
        &[CapBranch::LateralCircleA, CapBranch::LateralCircleB],
    );
}

/// Rows 34 & 35 — `|yAp.x| >= B.r`, computed `y <= 0` (cap A) or `y >= yBb.y`
/// (cap B) ⇒ delegates to `c2RaytoCircle`.
#[test]
fn cfg_34_35_raytocapsule_computed_delegate() {
    let mut rng = Rng::new(SEED ^ 34);
    let mut cases = Vec::new();
    let mut guard = 0;
    let (mut na, mut nb) = (0, 0);
    while (na < N / 2 || nb < N / 2) && guard < N * 300 {
        guard += 1;
        let (ray, cap) = rand_case(&mut rng);
        match classify(ray, cap) {
            CapBranch::DelegateCircleA if na < N / 2 => {
                na += 1;
                cases.push((ray, cap));
            }
            CapBranch::DelegateCircleB if nb < N / 2 => {
                nb += 1;
                cases.push((ray, cap));
            }
            _ => {}
        }
    }
    run_batch(
        "row34/35 c2RaytoCapsule computed delegate",
        cases,
        &[CapBranch::DelegateCircleA, CapBranch::DelegateCircleB],
    );
}

/// Rows 36 & 37 — the two side-hit branches: `out->n = M.x` (`c > 0`) vs
/// `out->n = c2Skew(M.y)` (`c <= 0`).
#[test]
fn cfg_36_37_raytocapsule_side_hits() {
    let mut rng = Rng::new(SEED ^ 36);
    let mut cases = Vec::new();
    let mut guard = 0;
    let (mut nx, mut ns) = (0, 0);
    while (nx < N / 2 || ns < N / 2) && guard < N * 300 {
        guard += 1;
        let (ray, cap) = rand_case(&mut rng);
        match classify(ray, cap) {
            CapBranch::SideMx if nx < N / 2 => {
                nx += 1;
                cases.push((ray, cap));
            }
            CapBranch::SideSkew if ns < N / 2 => {
                ns += 1;
                cases.push((ray, cap));
            }
            _ => {}
        }
    }
    run_batch(
        "row36/37 c2RaytoCapsule side hits",
        cases,
        &[CapBranch::SideMx, CapBranch::SideSkew],
    );
}

/// Row 38 — capsule axis orientation sweep: all 8 octants plus the 4
/// axis-aligned directions, so `M` covers every sign combination (and
/// `c2CCW90` / `c2Skew(M.y)` sign handling is exercised in each).
#[test]
fn cfg_38_raytocapsule_axis_orientations() {
    let mut rng = Rng::new(SEED ^ 38);
    let mut cases = Vec::new();
    let dirs: [(f32, f32); 12] = [
        (1.0, 0.0),
        (0.0, 1.0),
        (-1.0, 0.0),
        (0.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (-1.0, -1.0),
        (2.0, 1.0),
        (-1.0, 2.0),
        (1.0, -2.0),
        (-2.0, -1.0),
    ];
    for _ in 0..(N / dirs.len() + 1) {
        let a = rng.vec_coord();
        let len = rng.range(1.0, 40.0);
        let r = rng.radius();
        for (dx, dy) in dirs {
            let m = (dx * dx + dy * dy).sqrt();
            let cap = c2Capsule {
                a,
                b: c2v {
                    x: a.x + dx / m * len,
                    y: a.y + dy / m * len,
                },
                r,
            };
            // Fire from several directions around it.
            for k in 0..4 {
                let ang = rng.range(0.0, 6.283_185_5) + k as f32 * 1.570_796_3;
                let dist = rng.range(r, 100.0);
                let mid = c2v {
                    x: (cap.a.x + cap.b.x) * 0.5,
                    y: (cap.a.y + cap.b.y) * 0.5,
                };
                let origin = c2v {
                    x: mid.x + dist * ang.cos(),
                    y: mid.y + dist * ang.sin(),
                };
                let s = rng.range(-0.3, 1.3);
                let target = c2v {
                    x: cap.a.x + (cap.b.x - cap.a.x) * s + rng.range(-r * 2.0, r * 2.0),
                    y: cap.a.y + (cap.b.y - cap.a.y) * s + rng.range(-r * 2.0, r * 2.0),
                };
                cases.push((
                    c2Ray {
                        p: origin,
                        d: norm(origin, target),
                        t: rng.range(0.0, 250.0),
                    },
                    cap,
                ));
            }
        }
    }
    run_batch(
        "row38 c2RaytoCapsule axis orientations",
        cases,
        &[
            CapBranch::SideMx,
            CapBranch::SideSkew,
            CapBranch::EarlySlab,
            CapBranch::FallThrough,
        ],
    );
}

/// Row 39 — reversed capsule (`b` before `a`) and the zero-length axis
/// (`a == b`) which makes `c2Norm` divide by zero ⇒ an all-NaN basis `M`.
#[test]
fn cfg_39_raytocapsule_reversed_and_degenerate_axis() {
    let mut rng = Rng::new(SEED ^ 39);
    let mut d = Diff::new("row39 c2RaytoCapsule reversed/degenerate axis");

    for _ in 0..N {
        let cap = rand_capsule(&mut rng);
        let rev = c2Capsule { a: cap.b, b: cap.a, r: cap.r };
        let (ray, _) = rand_case(&mut rng);
        for c in [cap, rev] {
            let (cr, co, rr, ro) = call(ray, c);
            d.check_ray(cr, co, rr, ro, || fmt_case(ray, c));
        }
        // Zero-length axis in several flavours.
        let p = rng.vec_coord();
        for c in [
            c2Capsule { a: p, b: p, r: cap.r },
            c2Capsule {
                a: p,
                b: c2v { x: -p.x * 0.0 + p.x, y: p.y },
                r: cap.r,
            },
            c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: -0.0, y: -0.0 },
                r: cap.r,
            },
            c2Capsule { a: p, b: p, r: 0.0 },
        ] {
            let (cr, co, rr, ro) = call(ray, c);
            d.check_ray(cr, co, rr, ro, || fmt_case(ray, c));
        }
    }
    d.finish();
}

/// Row 40 — `B.r` and `A.t` magnitude sweep, including 0 and inf.
#[test]
fn cfg_40_raytocapsule_r_t_sweep() {
    let mut rng = Rng::new(SEED ^ 40);
    let mut d = Diff::new("row40 c2RaytoCapsule r/t sweep");

    let rs: &[f32] = &[
        0.0, -0.0, 1e-45, 1e-20, 0.001, 1.0, 50.0, 1e20, f32::MAX,
        f32::INFINITY, -3.0, f32::NAN,
    ];
    let ts: &[f32] = &[
        0.0, -0.0, 1e-20, 1.0, 1e6, f32::MAX, f32::INFINITY, -5.0, f32::NAN,
    ];
    for _ in 0..(N / (rs.len() * ts.len()) + 2) {
        let base = rand_capsule(&mut rng);
        let (ray0, _) = rand_case(&mut rng);
        for &r in rs {
            let cap = c2Capsule { a: base.a, b: base.b, r };
            for &t in ts {
                let ray = c2Ray { p: ray0.p, d: ray0.d, t };
                let (cr, co, rr, ro) = call(ray, cap);
                d.check_ray(cr, co, rr, ro, || fmt_case(ray, cap));
            }
        }
    }
    d.finish();
}

/// Row 41 — `yAe.x == yAp.x` inside the else-branch ⇒ the UNGUARDED division by
/// zero in `t = (c - yAp.x) / d`. Reached when the ray direction is parallel to
/// the capsule axis (so its lateral component is exactly zero).
#[test]
fn cfg_41_raytocapsule_parallel_div_zero() {
    let mut rng = Rng::new(SEED ^ 41);
    let mut d = Diff::new("row41 c2RaytoCapsule parallel /0");
    let mut hit_div0 = 0usize;

    for _ in 0..N {
        let cap = rand_capsule(&mut rng);
        let axis = c2v { x: cap.b.x - cap.a.x, y: cap.b.y - cap.a.y };
        let alen = (axis.x * axis.x + axis.y * axis.y).sqrt();
        let (ux, uy) = (axis.x / alen, axis.y / alen);
        // Direction exactly parallel (and anti-parallel) to the axis, origin
        // offset laterally by more than r so `|yAp.x| >= r`.
        for sign in [1.0f32, -1.0] {
            for latmul in [1.01f32, 1.5, 3.0, 10.0] {
                let lat = cap.r * latmul * if rng.bool() { 1.0 } else { -1.0 };
                let s = rng.range(-2.0, 3.0);
                let origin = c2v {
                    x: cap.a.x + axis.x * s + uy * lat,
                    y: cap.a.y + axis.y * s - ux * lat,
                };
                let ray = c2Ray {
                    p: origin,
                    d: c2v { x: ux * sign, y: uy * sign },
                    t: rng.range(0.0, 200.0),
                };
                // Detect the actual /0 using the C's own arithmetic.
                let l = libs();
                unsafe {
                    let My = (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a));
                    let M = c2m { x: (l.c.c2CCW90)(My), y: My };
                    let yAp = (l.c.c2MulmvT)(M, (l.c.c2Sub)(ray.p, cap.a));
                    let yAd = (l.c.c2MulmvT)(M, ray.d);
                    let yAe = (l.c.c2Add)(yAp, (l.c.c2Mulvs)(yAd, ray.t));
                    if yAe.x - yAp.x == 0.0 {
                        hit_div0 += 1;
                    }
                }
                let (cr, co, rr, ro) = call(ray, cap);
                d.check_ray(cr, co, rr, ro, || fmt_case(ray, cap));
            }
        }
    }
    // Also exhaustive special-class field injection (13 fields) and arbitrary
    // bit patterns, the strongest stress on this function.
    let base_ray = c2Ray {
        p: c2v { x: -20.0, y: 0.3 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 100.0,
    };
    let base_cap = c2Capsule {
        a: c2v { x: 0.0, y: -5.0 },
        b: c2v { x: 0.0, y: 5.0 },
        r: 2.0,
    };
    let mut inject: Vec<f32> = SPECIALS.to_vec();
    inject.extend(NAN_BITS.iter().map(|&b| f32::from_bits(b)));
    for s in inject {
        for field in 0..10 {
            let mut ray = base_ray;
            let mut cap = base_cap;
            match field {
                0 => ray.p.x = s,
                1 => ray.p.y = s,
                2 => ray.d.x = s,
                3 => ray.d.y = s,
                4 => ray.t = s,
                5 => cap.a.x = s,
                6 => cap.a.y = s,
                7 => cap.b.x = s,
                8 => cap.b.y = s,
                _ => cap.r = s,
            }
            let (cr, co, rr, ro) = call(ray, cap);
            d.check_ray(cr, co, rr, ro, || fmt_case(ray, cap));
        }
    }
    for _ in 0..N * 2 {
        let ray = c2Ray {
            p: rng.vec_spicy(),
            d: rng.vec_spicy(),
            t: rng.spicy(),
        };
        let cap = c2Capsule {
            a: rng.vec_spicy(),
            b: rng.vec_spicy(),
            r: rng.spicy(),
        };
        let (cr, co, rr, ro) = call(ray, cap);
        d.check_ray(cr, co, rr, ro, || fmt_case(ray, cap));
    }
    assert!(
        hit_div0 > 0,
        "the yAe.x == yAp.x divide-by-zero condition was never actually reached"
    );
    eprintln!("    row41: {hit_div0} cases with yAe.x - yAp.x == 0");
    d.finish();
}
