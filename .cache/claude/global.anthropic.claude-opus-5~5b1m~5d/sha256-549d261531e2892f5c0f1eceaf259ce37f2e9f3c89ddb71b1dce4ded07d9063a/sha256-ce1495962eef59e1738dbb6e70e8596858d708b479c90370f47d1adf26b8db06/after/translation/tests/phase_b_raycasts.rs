//! Phase B — valid-path differential tests for the per-shape raycasts.
//! Covers `CONFIGS.md` rows 24–33.
//!
//! These are the *low-level* entry points (`c2RaytoCircle`, `c2RaytoAABB`,
//! `c2RaytoCapsule`), driven directly rather than through `c2CastRay`, with a
//! poisoned 32-byte out-buffer so that "the C left `*out` alone" is verified
//! rather than assumed, and so an over-long write would show up.
//!
//! Every assertion compares the return code AND all 32 out-buffer bytes.

#![allow(non_snake_case)]

mod common;

use common::*;

const N: usize = 4096;

macro_rules! cmp_circle {
    ($l:expr, $ctx:expr, $a:expr, $b:expr) => {{
        let a = $a;
        let b = $b;
        let cr = run_circle(&$l.c, a, b);
        let rr = run_circle(&$l.rs, a, b);
        assert!(
            cr == rr,
            "DIVERGENCE [{}]\n  ray  = p={} d={} t={}\n  circ = p={} r={}\n  C    = {:?}\n  RUST = {:?}",
            $ctx, showv(a.p), showv(a.d), show(a.t), showv(b.p), show(b.r), cr, rr
        );
    }};
}

macro_rules! cmp_aabb {
    ($l:expr, $ctx:expr, $a:expr, $b:expr) => {{
        let a = $a;
        let b = $b;
        let cr = run_aabb(&$l.c, a, b);
        let rr = run_aabb(&$l.rs, a, b);
        assert!(
            cr == rr,
            "DIVERGENCE [{}]\n  ray = p={} d={} t={}\n  box = min={} max={}\n  C    = {:?}\n  RUST = {:?}",
            $ctx, showv(a.p), showv(a.d), show(a.t), showv(b.min), showv(b.max), cr, rr
        );
    }};
}

macro_rules! cmp_capsule {
    ($l:expr, $ctx:expr, $a:expr, $b:expr) => {{
        let a = $a;
        let b = $b;
        let cr = run_capsule(&$l.c, a, b);
        let rr = run_capsule(&$l.rs, a, b);
        assert!(
            cr == rr,
            "DIVERGENCE [{}]\n  ray = p={} d={} t={}\n  cap = a={} b={} r={}\n  C    = {:?}\n  RUST = {:?}",
            $ctx, showv(a.p), showv(a.d), show(a.t), showv(b.a), showv(b.b), show(b.r), cr, rr
        );
    }};
}

// ===========================================================================
// c2RaytoCircle — rows 24–27
// ===========================================================================

#[test]
fn row24_c2RaytoCircle_hit_path() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 24);
    let mut hits = 0usize;
    for i in 0..(N * 4) {
        // Construct a ray that is guaranteed to hit reasonably often: aim from
        // outside the circle roughly at its centre.
        let c = c2Circle {
            p: rng.vec_sym(10.0),
            r: 0.25 + rng.unit() * 5.0,
        };
        let ang = rng.unit() * std::f32::consts::TAU;
        let dist = c.r + 0.1 + rng.unit() * 20.0;
        let origin = c2v {
            x: c.p.x + dist * ang.cos(),
            y: c.p.y + dist * ang.sin(),
        };
        // Direction towards the centre, jittered so some rays miss.
        let jitter = rng.sym(0.35);
        let da = ang + std::f32::consts::PI + jitter;
        let a = c2Ray {
            p: origin,
            d: c2v {
                x: da.cos(),
                y: da.sin(),
            },
            t: dist + rng.unit() * 10.0,
        };
        let cr = run_circle(&l.c, a, c);
        if cr.ret != 0 {
            hits += 1;
        }
        cmp_circle!(l, format!("row24 aimed #{i}"), a, c);
    }
    assert!(
        hits > N,
        "row24 only produced {hits} hits — the generator is not exercising the hit path"
    );
}

#[test]
fn row25_c2RaytoCircle_inside_and_tangent() {
    let l = libs();
    // Ray origin exactly ON the rim: t == 0 → hit with a radial normal.
    for &(cx, cy, r, ox, oy, dx, dy) in &[
        (0.0f32, 0.0, 5.0, 3.0, 4.0, -0.6, -0.8),
        (0.0f32, 0.0, 5.0, 5.0, 0.0, -1.0, 0.0),
        (0.0f32, 0.0, 1.0, 0.0, 1.0, 0.0, -1.0),
        (1.0f32, 2.0, 13.0, 6.0, 14.0, -1.0, 0.0),
    ] {
        let a = c2Ray {
            p: c2v { x: ox, y: oy },
            d: c2v { x: dx, y: dy },
            t: 100.0,
        };
        let b = c2Circle {
            p: c2v { x: cx, y: cy },
            r,
        };
        cmp_circle!(l, "row25 on-rim", a, b);
    }

    let mut rng = Rng::new(SEED ^ 25);
    // Origin strictly INSIDE → nearest root is behind → t < 0 → miss.
    let mut inside_misses = 0usize;
    for i in 0..(N * 2) {
        let c = c2Circle {
            p: rng.vec_sym(10.0),
            r: 1.0 + rng.unit() * 6.0,
        };
        let ang = rng.unit() * std::f32::consts::TAU;
        let rad = rng.unit() * c.r * 0.9;
        let a = c2Ray {
            p: c2v {
                x: c.p.x + rad * ang.cos(),
                y: c.p.y + rad * ang.sin(),
            },
            d: rng.dir(),
            t: 50.0,
        };
        if run_circle(&l.c, a, c).ret == 0 {
            inside_misses += 1;
        }
        cmp_circle!(l, format!("row25 inside #{i}"), a, c);
    }
    assert!(inside_misses > 0, "row25 never exercised the inside/miss path");

    // Near-tangent rays: sweep the perpendicular offset across the radius so
    // `disc` passes through zero.
    for i in 0..2048 {
        let c = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 2.0,
        };
        let off = -3.0 + 6.0 * (i as f32) / 2048.0;
        let a = c2Ray {
            p: c2v { x: -10.0, y: off },
            d: c2v { x: 1.0, y: 0.0 },
            t: 40.0,
        };
        cmp_circle!(l, format!("row25 tangent #{i} off={}", show(off)), a, c);
    }
    // Exactly tangent (disc == 0 exactly).
    for &r in &[1.0f32, 2.0, 4.0, 0.5] {
        let c = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r,
        };
        let a = c2Ray {
            p: c2v { x: -8.0, y: r },
            d: c2v { x: 1.0, y: 0.0 },
            t: 40.0,
        };
        cmp_circle!(l, format!("row25 exact tangent r={}", show(r)), a, c);
    }
}

#[test]
fn row26_c2RaytoCircle_t_and_direction_shapes() {
    let l = libs();
    let c = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 2.0,
    };
    let origins = [
        c2v { x: -5.0, y: 0.0 },
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 2.0, y: 0.0 },
        c2v { x: 100.0, y: 100.0 },
    ];
    let dirs = [
        c2v { x: 1.0, y: 0.0 },
        c2v { x: -1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: 0.0, y: -1.0 },
        c2v { x: 0.0, y: 0.0 }, // zero direction
        c2v { x: 3.0, y: 4.0 }, // non-unit
        c2v { x: -0.0, y: -0.0 },
        c2v {
            x: f32::INFINITY,
            y: 0.0,
        },
    ];
    let ts = [
        0.0f32,
        -0.0,
        1.0,
        3.0,
        2.9999,
        3.0001,
        -1.0,
        -100.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN_POSITIVE,
    ];
    for (oi, &p) in origins.iter().enumerate() {
        for (di, &d) in dirs.iter().enumerate() {
            for (ti, &t) in ts.iter().enumerate() {
                cmp_circle!(l, format!("row26 o{oi} d{di} t{ti}"), c2Ray { p, d, t }, c);
            }
        }
    }
    // `A.t` swept finely across the exact hit distance (3.0) to catch the
    // `t <= A.t` boundary.
    for i in 0..1024 {
        let t = 2.5 + (i as f32) / 1024.0;
        cmp_circle!(
            l,
            format!("row26 t-sweep {}", show(t)),
            c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t
            },
            c
        );
    }
}

#[test]
fn row27_c2RaytoCircle_degenerate_and_nonfinite() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 27);
    for i in 0..(N * 8) {
        let a = any_ray(&mut rng);
        let b = any_circle(&mut rng);
        cmp_circle!(l, format!("row27 wild #{i}"), a, b);
    }
    // r == 0 with the origin exactly at the centre → t == 0, n = norm((0,0)).
    for &p in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 5.0, y: -7.0 },
    ] {
        for &r in &[0.0f32, -0.0, -3.0, 3.0] {
            for &d in &[
                c2v { x: 1.0, y: 0.0 },
                c2v { x: 0.0, y: 0.0 },
                c2v { x: -1.0, y: -1.0 },
            ] {
                cmp_circle!(
                    l,
                    format!("row27 r={} p={}", show(r), showv(p)),
                    c2Ray { p, d, t: 4.0 },
                    c2Circle { p, r }
                );
            }
        }
    }
    // Special value in every one of the 8 input slots, one at a time.
    let sp = special_wide();
    for &v in &sp {
        for slot in 0..8 {
            let mut a = c2Ray {
                p: c2v { x: -5.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            };
            let mut b = c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 2.0,
            };
            match slot {
                0 => a.p.x = v,
                1 => a.p.y = v,
                2 => a.d.x = v,
                3 => a.d.y = v,
                4 => a.t = v,
                5 => b.p.x = v,
                6 => b.p.y = v,
                _ => b.r = v,
            }
            cmp_circle!(l, format!("row27 slot{slot} {}", show(v)), a, b);
        }
    }
}

// ===========================================================================
// c2RaytoAABB — rows 28–31
// ===========================================================================

#[test]
fn row28_c2RaytoAABB_hit_all_four_normals() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 28);
    // Track which of the four `out->n` selections we actually reached.
    let mut seen = [0usize; 4];
    let mut hits = 0usize;
    for i in 0..(N * 8) {
        let hx = 0.25 + rng.unit() * 5.0;
        let hy = 0.25 + rng.unit() * 5.0;
        let c = rng.vec_sym(6.0);
        let b = c2AABB {
            min: c2v {
                x: c.x - hx,
                y: c.y - hy,
            },
            max: c2v {
                x: c.x + hx,
                y: c.y + hy,
            },
        };
        // Start outside on a random side and aim across the box.
        let ang = rng.unit() * std::f32::consts::TAU;
        let dist = hx.max(hy) + 1.0 + rng.unit() * 15.0;
        let p = c2v {
            x: c.x + dist * ang.cos(),
            y: c.y + dist * ang.sin(),
        };
        let da = ang + std::f32::consts::PI + rng.sym(0.5);
        let a = c2Ray {
            p,
            d: c2v {
                x: da.cos(),
                y: da.sin(),
            },
            t: dist + rng.unit() * 10.0,
        };
        let cr = run_aabb(&l.c, a, b);
        if cr.ret != 0 {
            hits += 1;
            let n = unsafe { (cr.out.as_ptr() as *const c2Raycast).read_unaligned() }.n;
            let idx = if n.x == -1.0 {
                0
            } else if n.x == 1.0 {
                1
            } else if n.y == -1.0 {
                2
            } else {
                3
            };
            seen[idx] += 1;
        }
        cmp_aabb!(l, format!("row28 aimed #{i}"), a, b);
    }
    assert!(hits > N, "row28 only produced {hits} hits");
    assert!(
        seen.iter().all(|&c| c > 0),
        "row28 did not reach all four out->n branches: {seen:?}"
    );
}

#[test]
fn row29_c2RaytoAABB_axis_aligned_rays() {
    let l = libs();
    let b = c2AABB {
        min: c2v { x: -1.0, y: -2.0 },
        max: c2v { x: 3.0, y: 4.0 },
    };
    let dirs = [
        c2v { x: 1.0, y: 0.0 },
        c2v { x: -1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: 0.0, y: -1.0 },
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
    ];
    // Sweep the perpendicular offset over each face, including exact corners
    // and exact face coordinates, and sweep A.t past the entry point.
    for (di, &d) in dirs.iter().enumerate() {
        for i in 0..256 {
            let s = -6.0 + 12.0 * (i as f32) / 256.0;
            for &t in &[0.0f32, 1.0, 4.0, 20.0, -3.0, f32::INFINITY] {
                cmp_aabb!(
                    l,
                    format!("row29 d{di} s={} t={}", show(s), show(t)),
                    c2Ray {
                        p: c2v { x: -8.0, y: s },
                        d,
                        t
                    },
                    b
                );
                cmp_aabb!(
                    l,
                    format!("row29 d{di} vert s={} t={}", show(s), show(t)),
                    c2Ray {
                        p: c2v { x: s, y: -8.0 },
                        d,
                        t
                    },
                    b
                );
            }
        }
    }
    // Exact grazes at the four corners.
    for &corner in &[
        c2v { x: -1.0, y: -2.0 },
        c2v { x: 3.0, y: -2.0 },
        c2v { x: -1.0, y: 4.0 },
        c2v { x: 3.0, y: 4.0 },
    ] {
        for &d in &dirs {
            for &t in &[0.0f32, 1.0, 10.0] {
                cmp_aabb!(l, "row29 corner", c2Ray { p: corner, d, t }, b);
            }
        }
    }
}

#[test]
fn row30_c2RaytoAABB_inside_before_after() {
    let l = libs();
    let b = c2AABB {
        min: c2v { x: -1.0, y: -2.0 },
        max: c2v { x: 3.0, y: 4.0 },
    };
    let mut rng = Rng::new(SEED ^ 30);
    // Origin strictly inside.
    for i in 0..(N * 2) {
        let p = c2v {
            x: -1.0 + rng.unit() * 4.0,
            y: -2.0 + rng.unit() * 6.0,
        };
        let a = c2Ray {
            p,
            d: rng.dir(),
            t: rng.unit() * 20.0,
        };
        cmp_aabb!(l, format!("row30 inside #{i}"), a, b);
    }
    // Segment entirely before / entirely after the box along its own line.
    for i in 0..(N * 2) {
        let short = rng.unit() * 0.5;
        let a = c2Ray {
            p: c2v { x: -20.0, y: rng.sym(6.0) },
            d: c2v { x: 1.0, y: 0.0 },
            t: short,
        };
        cmp_aabb!(l, format!("row30 before #{i}"), a, b);
        let a2 = c2Ray {
            p: c2v { x: 20.0, y: rng.sym(6.0) },
            d: c2v { x: 1.0, y: 0.0 },
            t: 5.0 + rng.unit() * 5.0,
        };
        cmp_aabb!(l, format!("row30 after #{i}"), a2, b);
    }
    // Fully random sane rays vs proper boxes.
    for i in 0..(N * 4) {
        let a = sane_ray(&mut rng);
        let min = rng.vec_grid(6);
        let bb = c2AABB {
            min,
            max: c2v {
                x: min.x + rng.gridded(5).abs(),
                y: min.y + rng.gridded(5).abs(),
            },
        };
        cmp_aabb!(l, format!("row30 sane #{i}"), a, bb);
    }
}

#[test]
fn row31_c2RaytoAABB_degenerate_and_nonfinite() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 31);
    for i in 0..(N * 8) {
        let a = any_ray(&mut rng);
        let b = any_aabb(&mut rng);
        cmp_aabb!(l, format!("row31 wild #{i}"), a, b);
    }
    // Degenerate / inverted / zero-width boxes with structured rays.
    let boxes = [
        c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        },
        c2AABB {
            min: c2v { x: 3.0, y: 4.0 },
            max: c2v { x: -1.0, y: -2.0 },
        },
        c2AABB {
            min: c2v { x: 0.0, y: -2.0 },
            max: c2v { x: 0.0, y: 4.0 },
        },
        c2AABB {
            min: c2v { x: -1.0, y: 0.0 },
            max: c2v { x: 3.0, y: 0.0 },
        },
        c2AABB {
            min: c2v { x: -0.0, y: -0.0 },
            max: c2v { x: 0.0, y: 0.0 },
        },
    ];
    for (bi, &b) in boxes.iter().enumerate() {
        for i in 0..512 {
            let s = -4.0 + 8.0 * (i as f32) / 512.0;
            for &d in &[
                c2v { x: 1.0, y: 0.0 },
                c2v { x: 0.0, y: 1.0 },
                c2v { x: 1.0, y: 1.0 },
                c2v { x: 0.0, y: 0.0 },
            ] {
                for &t in &[0.0f32, 4.0, -4.0, f32::INFINITY] {
                    cmp_aabb!(
                        l,
                        format!("row31 box{bi} s={} t={}", show(s), show(t)),
                        c2Ray {
                            p: c2v { x: s, y: s },
                            d,
                            t
                        },
                        b
                    );
                }
            }
        }
    }
    // Special value in every one of the 9 input slots.
    let sp = special_wide();
    for &v in &sp {
        for slot in 0..9 {
            let mut a = c2Ray {
                p: c2v { x: -5.0, y: 0.5 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            };
            let mut b = c2AABB {
                min: c2v { x: -1.0, y: -2.0 },
                max: c2v { x: 3.0, y: 4.0 },
            };
            match slot {
                0 => a.p.x = v,
                1 => a.p.y = v,
                2 => a.d.x = v,
                3 => a.d.y = v,
                4 => a.t = v,
                5 => b.min.x = v,
                6 => b.min.y = v,
                7 => b.max.x = v,
                _ => b.max.y = v,
            }
            cmp_aabb!(l, format!("row31 slot{slot} {}", show(v)), a, b);
        }
    }
}

// ===========================================================================
// c2RaytoCapsule — rows 32–33
// ===========================================================================

/// Classify which branch of `c2RaytoCapsule` a result came from, using only
/// externally-visible information, so the test can *prove* it covered the
/// branch tree rather than hammering one path.
///
/// `M.y = norm(b-a)`, `M.x = c2CCW90(M.y)`, and `c2Skew(M.y) == -M.x`, so the
/// two side-plane normals are distinguishable from a radial (circle) normal and
/// from the `norm(b-a)` value that is pre-written into `*out` on entry.
const BK_EARLY_HIT: usize = 0; // `return 1` from the bb / end-circle checks
const BK_REJECT: usize = 1; // `return 0` (fall-through, or a delegated circle miss)
const BK_SIDE_POS: usize = 2; // side-plane hit with `c > 0`  → n = M.x
const BK_SIDE_NEG: usize = 3; // side-plane hit with `c <= 0` → n = c2Skew(M.y)
const BK_CIRCLE_HIT: usize = 4; // delegated c2RaytoCircle reported a hit
const NBUCKETS: usize = 5;

fn capsule_bucket(l: &Pair, b: c2Capsule, r: &RayResult) -> usize {
    let cap_n = (l.rs.c2Sub)(b.b, b.a);
    let my = (l.rs.c2Norm)(cap_n);
    let mx = (l.rs.c2CCW90)(my);
    let skew_my = (l.rs.c2Skew)(my);
    let prewritten_n = my; // c2Norm(cap_n)
    let got = unsafe { (r.out.as_ptr() as *const c2Raycast).read_unaligned() };

    if r.ret == 0 {
        return BK_REJECT;
    }
    if vb(got.n) == vb(mx) && vb(mx) != vb(prewritten_n) {
        return BK_SIDE_POS;
    }
    if vb(got.n) == vb(skew_my) && vb(skew_my) != vb(prewritten_n) {
        return BK_SIDE_NEG;
    }
    if vb(got.n) == vb(prewritten_n) && fb(got.t) == fb(0.0) {
        return BK_EARLY_HIT;
    }
    BK_CIRCLE_HIT
}

#[test]
fn row32_c2RaytoCapsule_branch_coverage() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 32);
    let mut buckets = [0usize; NBUCKETS];

    for i in 0..(N * 12) {
        // Capsules with a definite axis and a positive radius, and rays aimed
        // near them, so all of the interesting branches get reached.
        let a0 = rng.vec_sym(8.0);
        let ang = rng.unit() * std::f32::consts::TAU;
        let len = 0.25 + rng.unit() * 8.0;
        let b = c2Capsule {
            a: a0,
            b: c2v {
                x: a0.x + len * ang.cos(),
                y: a0.y + len * ang.sin(),
            },
            r: 0.1 + rng.unit() * 3.0,
        };
        // Ray origin: sometimes inside the capsule, sometimes just outside,
        // sometimes far away.
        let p = match rng.below(4) {
            0 => {
                // near the axis mid-point (likely inside)
                let m = rng.unit();
                c2v {
                    x: b.a.x + (b.b.x - b.a.x) * m + rng.sym(b.r * 0.8),
                    y: b.a.y + (b.b.y - b.a.y) * m + rng.sym(b.r * 0.8),
                }
            }
            1 => {
                // just beyond one cap
                let e = if rng.below(2) == 0 { b.a } else { b.b };
                c2v {
                    x: e.x + rng.sym(b.r * 2.0),
                    y: e.y + rng.sym(b.r * 2.0),
                }
            }
            2 => rng.vec_sym(15.0),
            _ => {
                // off to one side, aiming across
                let n = c2v {
                    x: -(b.b.y - b.a.y),
                    y: b.b.x - b.a.x,
                };
                let nl = (n.x * n.x + n.y * n.y).sqrt().max(1e-6);
                let k = (b.r + 0.05 + rng.unit() * 6.0) * if rng.below(2) == 0 { 1.0 } else { -1.0 };
                let m = rng.unit();
                c2v {
                    x: b.a.x + (b.b.x - b.a.x) * m + n.x / nl * k,
                    y: b.a.y + (b.b.y - b.a.y) * m + n.y / nl * k,
                }
            }
        };
        let a = c2Ray {
            p,
            d: rng.dir(),
            t: rng.unit() * 25.0,
        };
        let cr = run_capsule(&l.c, a, b);
        buckets[capsule_bucket(l, b, &cr)] += 1;
        cmp_capsule!(l, format!("row32 aimed #{i}"), a, b);
    }

    // Targeted: sweep a ray across a vertical capsule so it crosses the side
    // plane, both caps, and the middle, hitting `c > 0` and `c <= 0`.
    let b = c2Capsule {
        a: c2v { x: 0.0, y: -3.0 },
        b: c2v { x: 0.0, y: 3.0 },
        r: 1.0,
    };
    for i in 0..2048 {
        let y = -6.0 + 12.0 * (i as f32) / 2048.0;
        for &(px, dx) in &[(-8.0f32, 1.0f32), (8.0, -1.0)] {
            for &t in &[0.0f32, 4.0, 20.0, -4.0, f32::INFINITY] {
                cmp_capsule!(
                    l,
                    format!("row32 sweep y={} t={}", show(y), show(t)),
                    c2Ray {
                        p: c2v { x: px, y },
                        d: c2v { x: dx, y: 0.0 },
                        t
                    },
                    b
                );
            }
        }
    }
    // Exactly on the side planes / cap boundaries.
    for &x in &[-1.0f32, 1.0, -0.0, 0.0] {
        for &y in &[-3.0f32, 3.0, 0.0, -4.0, 4.0] {
            for &d in &[
                c2v { x: 1.0, y: 0.0 },
                c2v { x: 0.0, y: 1.0 },
                c2v { x: 0.0, y: -1.0 },
                c2v { x: 1.0, y: 1.0 },
            ] {
                cmp_capsule!(
                    l,
                    format!("row32 exact x={} y={}", show(x), show(y)),
                    c2Ray {
                        p: c2v { x, y },
                        d,
                        t: 8.0
                    },
                    b
                );
            }
        }
    }

    // Fold the targeted vertical sweep into the coverage tally: it is the case
    // that reliably reaches both side-plane normals.
    for i in 0..4096 {
        let y = -6.0 + 12.0 * (i as f32) / 4096.0;
        for &(px, dx) in &[(-8.0f32, 1.0f32), (8.0, -1.0)] {
            let a = c2Ray {
                p: c2v { x: px, y },
                d: c2v { x: dx, y: 0.0 },
                t: 20.0,
            };
            buckets[capsule_bucket(l, b, &run_capsule(&l.c, a, b))] += 1;
            cmp_capsule!(l, format!("row32 tally y={}", show(y)), a, b);
        }
    }
    // Rays travelling along the axis direction reach the caps, and rays from
    // inside reach the early return; between them every bucket is populated.
    for k in 0..NBUCKETS {
        assert!(
            buckets[k] > 0,
            "row32 never reached capsule branch bucket {k}; buckets = {buckets:?}"
        );
    }
}

#[test]
fn row33_c2RaytoCapsule_orientations_and_degenerate() {
    let l = libs();
    // Horizontal, vertical, reversed-vertical, oblique, and degenerate axes,
    // crossed with r = 0 / negative / positive.
    let axes = [
        (c2v { x: -2.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }),
        (c2v { x: 0.0, y: -2.0 }, c2v { x: 0.0, y: 2.0 }),
        (c2v { x: 0.0, y: 2.0 }, c2v { x: 0.0, y: -2.0 }), // b below a
        (c2v { x: -1.0, y: -1.0 }, c2v { x: 2.0, y: 3.0 }),
        (c2v { x: 1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }), // degenerate a == b
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }),
        (c2v { x: -0.0, y: -0.0 }, c2v { x: 0.0, y: 0.0 }),
    ];
    let radii = [0.0f32, -0.0, 1.0, -1.0, 0.5, -3.0, f32::INFINITY];
    let dirs = [
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 0.0, y: 1.0 },
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 1.0, y: 1.0 },
        c2v { x: -1.0, y: 2.0 },
    ];
    let ts = [0.0f32, 1.0, 5.0, -5.0, f32::INFINITY, f32::NEG_INFINITY];
    let mut rng = Rng::new(SEED ^ 33);
    for (ai, &(a0, b0)) in axes.iter().enumerate() {
        for (ri, &r) in radii.iter().enumerate() {
            let b = c2Capsule { a: a0, b: b0, r };
            for (di, &d) in dirs.iter().enumerate() {
                for (ti, &t) in ts.iter().enumerate() {
                    for k in 0..8 {
                        let p = if k == 0 {
                            a0
                        } else if k == 1 {
                            b0
                        } else {
                            rng.vec_grid(5)
                        };
                        cmp_capsule!(
                            l,
                            format!("row33 a{ai} r{ri} d{di} t{ti} k{k}"),
                            c2Ray { p, d, t },
                            b
                        );
                    }
                }
            }
        }
    }
    // Fully random / non-finite.
    for i in 0..(N * 8) {
        let a = any_ray(&mut rng);
        let b = any_capsule(&mut rng);
        cmp_capsule!(l, format!("row33 wild #{i}"), a, b);
    }
    // Special value in every one of the 10 input slots.
    let sp = special_wide();
    for &v in &sp {
        for slot in 0..10 {
            let mut a = c2Ray {
                p: c2v { x: -5.0, y: 0.5 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            };
            let mut b = c2Capsule {
                a: c2v { x: 0.0, y: -2.0 },
                b: c2v { x: 0.0, y: 2.0 },
                r: 1.0,
            };
            match slot {
                0 => a.p.x = v,
                1 => a.p.y = v,
                2 => a.d.x = v,
                3 => a.d.y = v,
                4 => a.t = v,
                5 => b.a.x = v,
                6 => b.a.y = v,
                7 => b.b.x = v,
                8 => b.b.y = v,
                _ => b.r = v,
            }
            cmp_capsule!(l, format!("row33 slot{slot} {}", show(v)), a, b);
        }
    }
}
