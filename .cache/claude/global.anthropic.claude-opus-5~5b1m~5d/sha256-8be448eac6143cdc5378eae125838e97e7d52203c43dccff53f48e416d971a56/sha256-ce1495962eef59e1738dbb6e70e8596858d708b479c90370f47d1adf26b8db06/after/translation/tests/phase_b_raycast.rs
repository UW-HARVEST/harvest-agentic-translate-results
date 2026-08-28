//! Phase B — CONFIGS.md rows 27..45: the three low-level raycasters
//! `c2RaytoCircle`, `c2RaytoAABB`, `c2RaytoCapsule`, driven directly (NOT via
//! the `c2CastRay` / `gen_ray` convenience wrappers).
//!
//! Every call pre-fills the `c2Raycast` out-parameter with a sentinel bit
//! pattern (CONFIGS.md row 56) so that "was `*out` written on the reject path?"
//! is part of the compared observable state.

mod common;

use common::*;
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Call helpers: run both libraries with a sentinel-filled out-param.
// ---------------------------------------------------------------------------

fn circle_pair(p: &Pair, a: c2Ray, b: c2Circle) -> (i32, c2Raycast, i32, c2Raycast) {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { (p.c.c2RaytoCircle)(a, b, &mut co) };
    let rr = unsafe { (p.r.c2RaytoCircle)(a, b, &mut ro) };
    (cr, co, rr, ro)
}

fn aabb_pair(p: &Pair, a: c2Ray, b: c2AABB) -> (i32, c2Raycast, i32, c2Raycast) {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { (p.c.c2RaytoAABB)(a, b, &mut co) };
    let rr = unsafe { (p.r.c2RaytoAABB)(a, b, &mut ro) };
    (cr, co, rr, ro)
}

fn capsule_pair(p: &Pair, a: c2Ray, b: c2Capsule) -> (i32, c2Raycast, i32, c2Raycast) {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { (p.c.c2RaytoCapsule)(a, b, &mut co) };
    let rr = unsafe { (p.r.c2RaytoCapsule)(a, b, &mut ro) };
    (cr, co, rr, ro)
}

macro_rules! chk {
    ($d:expr, $p:expr, $f:ident, $a:expr, $b:expr) => {{
        let a = $a;
        let b = $b;
        let (cr, co, rr, ro) = $f($p, a, b);
        $d.eq_cast(
            || format!("{}({:?}, {:?})", stringify!($f), a, b),
            cr,
            &co,
            rr,
            &ro,
        );
        (cr, co)
    }};
}

// ---------------------------------------------------------------------------
// Input generators
// ---------------------------------------------------------------------------

fn rand_ray(rng: &mut Rng, scale: f32) -> c2Ray {
    c2Ray {
        p: rng.vec_uniform(scale),
        d: rng.vec_uniform(scale),
        t: rng.uniform(scale),
    }
}

/// The shape `gen_ray` produces: `d` normalised, `t` = distance to a target.
fn normalized_ray(rng: &mut Rng, scale: f32) -> c2Ray {
    let p = rng.vec_uniform(scale);
    let target = rng.vec_uniform(scale);
    let dx = target.x - p.x;
    let dy = target.y - p.y;
    let l = (dx * dx + dy * dy).sqrt();
    let d = v(dx / l, dy / l);
    c2Ray {
        p,
        d,
        t: (target.x * d.x + target.y * d.y) - (p.x * d.x + p.y * d.y),
    }
}

fn rand_box(rng: &mut Rng, scale: f32) -> c2AABB {
    let a = rng.vec_uniform(scale);
    let b = rng.vec_uniform(scale);
    c2AABB {
        min: v(a.x.min(b.x), a.y.min(b.y)),
        max: v(a.x.max(b.x), a.y.max(b.y)),
    }
}

fn rand_circle(rng: &mut Rng, scale: f32) -> c2Circle {
    c2Circle {
        p: rng.vec_uniform(scale),
        r: rng.positive(scale),
    }
}

fn rand_capsule(rng: &mut Rng, scale: f32) -> c2Capsule {
    c2Capsule {
        a: rng.vec_uniform(scale),
        b: rng.vec_uniform(scale),
        r: rng.positive(scale * 0.5),
    }
}

const T_SPECIALS: &[f32] = &[
    0.0,
    -0.0,
    -1.0,
    1.0,
    1e-30,
    1e30,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    f32::MAX,
];

// ===========================================================================
// c2RaytoCircle — rows 27, 28, 29, 30
// ===========================================================================

#[test]
fn cfg_27_circle_raw_random() {
    let p = load();
    let mut d = Diff::new("cfg_27_circle_raw_random");
    let mut rng = Rng::new(0x2727);
    let mut hits = 0u32;
    for scale in [1e-3f32, 1.0, 1e3, 1e15] {
        for _ in 0..40_000 {
            let a = rand_ray(&mut rng, scale);
            let b = rand_circle(&mut rng, scale);
            let (cr, _) = chk!(d, &p, circle_pair, a, b);
            hits += (cr != 0) as u32;
        }
    }
    assert!(hits > 100, "population never hit the circle ({hits})");
    d.finish();
}

#[test]
fn cfg_28_circle_normalized_ray() {
    let p = load();
    let mut d = Diff::new("cfg_28_circle_normalized_ray");
    let mut rng = Rng::new(0x2828);
    let mut hits = 0u32;
    for scale in [1e-2f32, 1.0, 1e2, 1e6] {
        for _ in 0..40_000 {
            let a = normalized_ray(&mut rng, scale);
            let b = rand_circle(&mut rng, scale);
            let (cr, _) = chk!(d, &p, circle_pair, a, b);
            hits += (cr != 0) as u32;
        }
    }
    assert!(hits > 1_000, "normalized population barely hit ({hits})");
    d.finish();
}

#[test]
fn cfg_29_circle_geometry_branches() {
    let p = load();
    let mut d = Diff::new("cfg_29_circle_geometry_branches");
    let mut rng = Rng::new(0x2929);
    let mut inside = 0u32;
    let mut behind = 0u32;
    let mut past = 0u32;
    let mut tangent = 0u32;
    for _ in 0..40_000 {
        let b = rand_circle(&mut rng, 10.0);
        // (a) origin exactly at the centre → t = -sqrt(r*r) < 0 → reject
        let a0 = c2Ray { p: b.p, d: v(1.0, 0.0), t: 100.0 };
        let (r0, _) = chk!(d, &p, circle_pair, a0, b);
        inside += (r0 == 0) as u32;

        // (b) origin inside the circle
        let ang = rng.uniform(3.14159265);
        let ins = v(
            b.p.x + b.r * 0.5 * ang.cos(),
            b.p.y + b.r * 0.5 * ang.sin(),
        );
        let a1 = c2Ray { p: ins, d: v(ang.cos(), ang.sin()), t: 100.0 };
        chk!(d, &p, circle_pair, a1, b);

        // (c) circle behind the ray origin
        let dir = v(ang.cos(), ang.sin());
        let behind_p = v(b.p.x - dir.x * (b.r + 5.0), b.p.y - dir.y * (b.r + 5.0));
        let a2 = c2Ray { p: behind_p, d: v(-dir.x, -dir.y), t: 100.0 };
        let (r2, _) = chk!(d, &p, circle_pair, a2, b);
        behind += (r2 == 0) as u32;

        // (d) exact hit but A.t too short
        let a3 = c2Ray { p: behind_p, d: dir, t: 1.0 };
        let (r3, _) = chk!(d, &p, circle_pair, a3, b);
        past += (r3 == 0) as u32;

        // (e) tangent: offset the origin perpendicular by exactly r
        let perp = v(-dir.y, dir.x);
        let tan_p = v(
            b.p.x - dir.x * (b.r + 5.0) + perp.x * b.r,
            b.p.y - dir.y * (b.r + 5.0) + perp.y * b.r,
        );
        let a4 = c2Ray { p: tan_p, d: dir, t: 100.0 };
        let (r4, _) = chk!(d, &p, circle_pair, a4, b);
        tangent += (r4 != 0) as u32;

        // (f) A.t sweep on a known-hit configuration
        for &t in T_SPECIALS {
            let a5 = c2Ray { p: behind_p, d: dir, t };
            chk!(d, &p, circle_pair, a5, b);
        }
    }
    assert!(inside > 0 && behind > 0 && past > 0, "{inside} {behind} {past}");
    assert!(tangent > 0, "no tangent hit reached");
    d.finish();
}

#[test]
fn cfg_30_circle_specials() {
    let p = load();
    let mut d = Diff::new("cfg_30_circle_specials");
    let mut rng = Rng::new(0x3030);
    let sp = specials();
    // Sweep every one of the 8 float slots (5 ray + 3 circle) against the pool.
    let base = [-5.0f32, 0.0, 1.0, 0.0, 100.0, 0.0, 0.0, 2.0];
    for slot in 0..8usize {
        for &s in &sp {
            for &s2 in &sp {
                let mut f = base;
                f[slot] = s;
                f[(slot + 3) % 8] = s2;
                let a = c2Ray { p: v(f[0], f[1]), d: v(f[2], f[3]), t: f[4] };
                let b = c2Circle { p: v(f[5], f[6]), r: f[7] };
                chk!(d, &p, circle_pair, a, b);
            }
        }
    }
    // A.d == (0,0), r ∈ {0, -r, inf, NaN}, centre at ±inf
    for &r in &[0.0f32, -0.0, -2.0, f32::INFINITY, f32::NAN, f32::MAX] {
        for _ in 0..3_000 {
            let a = c2Ray { p: rng.vec_uniform(10.0), d: v(0.0, 0.0), t: rng.uniform(10.0) };
            let b = c2Circle { p: rng.vec_uniform(10.0), r };
            chk!(d, &p, circle_pair, a, b);
            let a2 = rand_ray(&mut rng, 10.0);
            let b2 = c2Circle { p: v(f32::INFINITY, rng.uniform(10.0)), r };
            chk!(d, &p, circle_pair, a2, b2);
        }
    }
    // Full bit-pattern fuzz (row 58 for this entry point).
    for _ in 0..150_000 {
        let a = c2Ray { p: rng.vec_bits(), d: rng.vec_bits(), t: rng.any_bits() };
        let b = c2Circle { p: rng.vec_bits(), r: rng.any_bits() };
        chk!(d, &p, circle_pair, a, b);
    }
    for _ in 0..150_000 {
        let a = c2Ray { p: rng.vec_spicy(10.0), d: rng.vec_spicy(10.0), t: rng.spicy(10.0) };
        let b = c2Circle { p: rng.vec_spicy(10.0), r: rng.spicy(10.0) };
        chk!(d, &p, circle_pair, a, b);
    }
    d.finish();
}

// ===========================================================================
// c2RaytoAABB — rows 31..36
// ===========================================================================

#[test]
fn cfg_31_aabb_raw_random() {
    let p = load();
    let mut d = Diff::new("cfg_31_aabb_raw_random");
    let mut rng = Rng::new(0x3131);
    let mut hits = 0u32;
    for scale in [1e-3f32, 1.0, 1e3, 1e15] {
        for _ in 0..40_000 {
            let a = rand_ray(&mut rng, scale);
            let b = rand_box(&mut rng, scale);
            let (cr, _) = chk!(d, &p, aabb_pair, a, b);
            hits += (cr != 0) as u32;
        }
    }
    assert!(hits > 100, "population never hit the box ({hits})");
    d.finish();
}

#[test]
fn cfg_32_aabb_normalized_ray() {
    let p = load();
    let mut d = Diff::new("cfg_32_aabb_normalized_ray");
    let mut rng = Rng::new(0x3232);
    let mut hits = 0u32;
    for scale in [1e-2f32, 1.0, 1e2, 1e6] {
        for _ in 0..40_000 {
            let a = normalized_ray(&mut rng, scale);
            let b = rand_box(&mut rng, scale);
            let (cr, _) = chk!(d, &p, aabb_pair, a, b);
            hits += (cr != 0) as u32;
        }
    }
    assert!(hits > 1_000, "normalized population barely hit ({hits})");
    d.finish();
}

/// Row 33 — force every one of the four `out->n` branches plus the tie-breaks.
#[test]
fn cfg_33_aabb_all_normal_branches() {
    let p = load();
    let mut d = Diff::new("cfg_33_aabb_all_normal_branches");
    let mut rng = Rng::new(0x3333);
    let mut normals: BTreeSet<(u32, u32)> = BTreeSet::new();

    let b = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
    // Approach from each of the 4 sides plus the 4 corners (exact 45° ties).
    let dirs = [
        v(1.0, 0.0), v(-1.0, 0.0), v(0.0, 1.0), v(0.0, -1.0),
        v(1.0, 1.0), v(-1.0, 1.0), v(1.0, -1.0), v(-1.0, -1.0),
        v(0.70710678, 0.70710678), v(-0.70710678, 0.70710678),
        v(0.70710678, -0.70710678), v(-0.70710678, -0.70710678),
    ];
    for dir in dirs {
        for k in 0..600 {
            let s = 1.0 + (k as f32) * 0.01;
            let a = c2Ray {
                p: v(-dir.x * 3.0 * s, -dir.y * 3.0 * s),
                d: dir,
                t: 10.0 * s,
            };
            let (cr, co) = chk!(d, &p, aabb_pair, a, b);
            if cr != 0 {
                normals.insert(vbits(co.n));
            }
            // Same but with an offset perpendicular start, so the winning slab
            // varies across the sweep.
            let perp = v(-dir.y, dir.x);
            let off = (k as f32 - 300.0) * 0.005;
            let a2 = c2Ray {
                p: v(-dir.x * 3.0 + perp.x * off, -dir.y * 3.0 + perp.y * off),
                d: dir,
                t: 10.0,
            };
            let (cr2, co2) = chk!(d, &p, aabb_pair, a2, b);
            if cr2 != 0 {
                normals.insert(vbits(co2.n));
            }
        }
    }
    // Random rays through many random boxes, harvesting whatever normals appear.
    for _ in 0..100_000 {
        let bx = rand_box(&mut rng, 10.0);
        let a = normalized_ray(&mut rng, 10.0);
        let (cr, co) = chk!(d, &p, aabb_pair, a, bx);
        if cr != 0 {
            normals.insert(vbits(co.n));
        }
    }
    let want: BTreeSet<(u32, u32)> = [
        vbits(v(-1.0, 0.0)),
        vbits(v(1.0, 0.0)),
        vbits(v(0.0, -1.0)),
        vbits(v(0.0, 1.0)),
    ]
    .into_iter()
    .collect();
    assert!(
        want.is_subset(&normals),
        "not all four out->n branches were reached: got {:?}",
        normals
    );
    d.finish();
}

/// Row 34 — axis-aligned rays exercise `da*db == 0` and `da - db == 0`.
#[test]
fn cfg_34_aabb_axis_aligned() {
    let p = load();
    let mut d = Diff::new("cfg_34_aabb_axis_aligned");
    let mut rng = Rng::new(0x3434);
    let dirs = [v(1.0, 0.0), v(-1.0, 0.0), v(0.0, 1.0), v(0.0, -1.0), v(0.0, 0.0)];
    for dir in dirs {
        for _ in 0..20_000 {
            let bx = rand_box(&mut rng, 10.0);
            // Origins on the box planes, inside, and far outside.
            let origins = [
                v(bx.min.x, bx.min.y),
                v(bx.max.x, bx.max.y),
                v(bx.min.x, bx.max.y),
                v((bx.min.x + bx.max.x) * 0.5, (bx.min.y + bx.max.y) * 0.5),
                v(bx.min.x - 5.0, bx.min.y),
                v(bx.min.x, bx.min.y - 5.0),
                rng.vec_uniform(10.0),
            ];
            for o in origins {
                for &t in &[0.0f32, 1.0, 10.0, -1.0] {
                    let a = c2Ray { p: o, d: dir, t };
                    chk!(d, &p, aabb_pair, a, bx);
                }
            }
        }
    }
    d.finish();
}

/// Row 35 — `A.t` extremes.
#[test]
fn cfg_35_aabb_ray_length_extremes() {
    let p = load();
    let mut d = Diff::new("cfg_35_aabb_ray_length_extremes");
    let mut rng = Rng::new(0x3535);
    for &t in T_SPECIALS {
        for _ in 0..20_000 {
            let bx = rand_box(&mut rng, 10.0);
            let mut a = normalized_ray(&mut rng, 10.0);
            a.t = t;
            chk!(d, &p, aabb_pair, a, bx);
            let mut a2 = rand_ray(&mut rng, 10.0);
            a2.t = t;
            chk!(d, &p, aabb_pair, a2, bx);
        }
    }
    d.finish();
}

/// Row 36 — degenerate/inverted/NaN boxes and origin-inside cases.
#[test]
fn cfg_36_aabb_degenerate_boxes() {
    let p = load();
    let mut d = Diff::new("cfg_36_aabb_degenerate_boxes");
    let mut rng = Rng::new(0x3636);
    let sp = specials();
    let base = [-3.0f32, 0.0, 1.0, 0.0, 10.0, -1.0, -1.0, 1.0, 1.0];
    for slot in 0..9usize {
        for &s in &sp {
            for &s2 in &sp {
                let mut f = base;
                f[slot] = s;
                f[(slot + 4) % 9] = s2;
                let a = c2Ray { p: v(f[0], f[1]), d: v(f[2], f[3]), t: f[4] };
                let bx = c2AABB { min: v(f[5], f[6]), max: v(f[7], f[8]) };
                chk!(d, &p, aabb_pair, a, bx);
            }
        }
    }
    // Inverted / zero-area boxes and origin-inside rays.
    for _ in 0..40_000 {
        let c0 = rng.vec_uniform(10.0);
        let c1 = rng.vec_uniform(10.0);
        let variants = [
            c2AABB { min: c0, max: c1 },                        // may be inverted
            c2AABB { min: c0, max: c0 },                        // point
            c2AABB { min: v(c0.x, c0.y), max: v(c0.x, c1.y) },  // zero width
            c2AABB { min: v(c0.x, c0.y), max: v(c1.x, c0.y) },  // zero height
            c2AABB { min: v(-0.0, -0.0), max: v(0.0, 0.0) },
        ];
        for bx in variants {
            let inside = c2Ray {
                p: v((bx.min.x + bx.max.x) * 0.5, (bx.min.y + bx.max.y) * 0.5),
                d: rng.vec_uniform(1.0),
                t: rng.uniform(10.0),
            };
            chk!(d, &p, aabb_pair, inside, bx);
            let outside = normalized_ray(&mut rng, 10.0);
            chk!(d, &p, aabb_pair, outside, bx);
        }
    }
    // Full bit-pattern fuzz (row 58).
    for _ in 0..150_000 {
        let a = c2Ray { p: rng.vec_bits(), d: rng.vec_bits(), t: rng.any_bits() };
        let bx = c2AABB { min: rng.vec_bits(), max: rng.vec_bits() };
        chk!(d, &p, aabb_pair, a, bx);
    }
    for _ in 0..150_000 {
        let a = c2Ray { p: rng.vec_spicy(10.0), d: rng.vec_spicy(10.0), t: rng.spicy(10.0) };
        let bx = c2AABB { min: rng.vec_spicy(10.0), max: rng.vec_spicy(10.0) };
        chk!(d, &p, aabb_pair, a, bx);
    }
    d.finish();
}

// ===========================================================================
// c2RaytoCapsule — rows 37..45
// ===========================================================================

#[test]
fn cfg_37_capsule_raw_random() {
    let p = load();
    let mut d = Diff::new("cfg_37_capsule_raw_random");
    let mut rng = Rng::new(0x3737);
    let mut hits = 0u32;
    for scale in [1e-3f32, 1.0, 1e3, 1e15] {
        for _ in 0..40_000 {
            let a = rand_ray(&mut rng, scale);
            let b = rand_capsule(&mut rng, scale);
            let (cr, _) = chk!(d, &p, capsule_pair, a, b);
            hits += (cr != 0) as u32;
        }
    }
    assert!(hits > 100, "population never hit the capsule ({hits})");
    d.finish();
}

#[test]
fn cfg_38_capsule_normalized_ray() {
    let p = load();
    let mut d = Diff::new("cfg_38_capsule_normalized_ray");
    let mut rng = Rng::new(0x3838);
    let mut hits = 0u32;
    for scale in [1e-2f32, 1.0, 1e2, 1e6] {
        for _ in 0..40_000 {
            let a = normalized_ray(&mut rng, scale);
            let b = rand_capsule(&mut rng, scale);
            let (cr, _) = chk!(d, &p, capsule_pair, a, b);
            hits += (cr != 0) as u32;
        }
    }
    assert!(hits > 1_000, "normalized population barely hit ({hits})");
    d.finish();
}

/// Rows 39, 40, 41, 42, 43 — every branch of `c2RaytoCapsule` reached on
/// purpose, driven from the capsule's own local frame.
#[test]
fn cfg_39_43_capsule_all_branches() {
    let p = load();
    let mut d = Diff::new("cfg_39_43_capsule_all_branches");
    let mut rng = Rng::new(0x3943);
    let mut slab_accept = 0u32;
    let mut cap_a_accept = 0u32;
    let mut cap_b_accept = 0u32;
    let mut sidewall_pos = 0u32;
    let mut sidewall_neg = 0u32;
    let mut reject = 0u32;

    for _ in 0..30_000 {
        let ca = rng.vec_uniform(10.0);
        let ang = rng.uniform(3.14159265);
        let len = rng.positive(10.0) + 0.5;
        let axis = v(ang.cos(), ang.sin());
        let cb = v(ca.x + axis.x * len, ca.y + axis.y * len);
        let r = rng.positive(2.0) + 0.01;
        let b = c2Capsule { a: ca, b: cb, r };
        // Local frame: M.y = axis, M.x = ccw90(axis) = (axis.y, -axis.x)
        let mx = v(axis.y, -axis.x);
        let to_world = |lx: f32, ly: f32| {
            v(
                ca.x + mx.x * lx + axis.x * ly,
                ca.y + mx.y * lx + axis.y * ly,
            )
        };

        // (row 39) origin strictly inside the slab [-r, r] x [0, len]
        let o_slab = to_world(rng.uniform(r * 0.9), rng.positive(len * 0.9));
        let a1 = c2Ray { p: o_slab, d: rng.vec_uniform(1.0), t: rng.uniform(10.0) };
        let (r1, _) = chk!(d, &p, capsule_pair, a1, b);
        slab_accept += (r1 == 1) as u32;

        // (row 40) origin inside end-cap A (below y=0 but within r of `a`)
        let o_capa = to_world(rng.uniform(r * 0.5), -rng.positive(r * 0.4));
        let a2 = c2Ray { p: o_capa, d: rng.vec_uniform(1.0), t: rng.uniform(10.0) };
        let (r2, _) = chk!(d, &p, capsule_pair, a2, b);
        cap_a_accept += (r2 == 1) as u32;

        // (row 40) origin inside end-cap B (above y=len but within r of `b`)
        let o_capb = to_world(rng.uniform(r * 0.5), len + rng.positive(r * 0.4));
        let a3 = c2Ray { p: o_capb, d: rng.vec_uniform(1.0), t: rng.uniform(10.0) };
        let (r3, _) = chk!(d, &p, capsule_pair, a3, b);
        cap_b_accept += (r3 == 1) as u32;

        // (row 41) |yAp.x| < r but outside the caps → delegates to Ca or Cb
        //   yAp.y < 0 → Ca, yAp.y >= 0 → Cb
        for sign in [-1.0f32, 1.0] {
            let ly = if sign < 0.0 { -(r + 3.0) } else { len + r + 3.0 };
            let o = to_world(rng.uniform(r * 0.9), ly);
            let dir = v(
                to_world(0.0, len * 0.5).x - o.x,
                to_world(0.0, len * 0.5).y - o.y,
            );
            let dl = (dir.x * dir.x + dir.y * dir.y).sqrt();
            let a4 = c2Ray { p: o, d: v(dir.x / dl, dir.y / dl), t: dl + 1.0 };
            chk!(d, &p, capsule_pair, a4, b);
            // Also a short ray that cannot reach.
            let a5 = c2Ray { p: o, d: v(dir.x / dl, dir.y / dl), t: 0.001 };
            chk!(d, &p, capsule_pair, a5, b);
        }

        // (rows 42, 43) slab-crossing: start at local x = ±(r + k), aim across.
        for side in [-1.0f32, 1.0] {
            for ly_frac in [-2.0f32, -0.2, 0.0, 0.25, 0.5, 0.75, 1.0, 1.3, 3.0] {
                let ox = side * (r + 2.0);
                let oy = len * ly_frac;
                let o = to_world(ox, oy);
                // aim at the opposite side, same local height → crosses the slab
                let tgt = to_world(-ox, oy);
                let dx = tgt.x - o.x;
                let dy = tgt.y - o.y;
                let dl = (dx * dx + dy * dy).sqrt();
                let a6 = c2Ray { p: o, d: v(dx / dl, dy / dl), t: dl };
                let (r6, o6) = chk!(d, &p, capsule_pair, a6, b);
                if r6 == 1 {
                    // side-wall hit sets n to M.x (c > 0) or skew(M.y) (c <= 0)
                    let nb = vbits(o6.n);
                    if nb == vbits(mx) {
                        sidewall_pos += 1;
                    } else if nb == vbits(v(-axis.y, axis.x)) {
                        sidewall_neg += 1;
                    }
                } else {
                    reject += 1;
                }
                // Diagonal crossings so `y` lands above/below the segment.
                let tgt2 = to_world(-ox, oy + len * 2.0);
                let dx2 = tgt2.x - o.x;
                let dy2 = tgt2.y - o.y;
                let dl2 = (dx2 * dx2 + dy2 * dy2).sqrt();
                let a7 = c2Ray { p: o, d: v(dx2 / dl2, dy2 / dl2), t: dl2 };
                chk!(d, &p, capsule_pair, a7, b);
                let tgt3 = to_world(-ox, oy - len * 2.0);
                let dx3 = tgt3.x - o.x;
                let dy3 = tgt3.y - o.y;
                let dl3 = (dx3 * dx3 + dy3 * dy3).sqrt();
                let a8 = c2Ray { p: o, d: v(dx3 / dl3, dy3 / dl3), t: dl3 };
                chk!(d, &p, capsule_pair, a8, b);
                // A ray that stays on one side → final `return 0`
                let a9 = c2Ray { p: o, d: v(axis.x, axis.y), t: len };
                let (r9, _) = chk!(d, &p, capsule_pair, a9, b);
                reject += (r9 == 0) as u32;
            }
        }
    }
    assert!(slab_accept > 0, "slab accept branch never reached");
    assert!(cap_a_accept > 0, "end-cap A accept never reached");
    assert!(cap_b_accept > 0, "end-cap B accept never reached");
    assert!(sidewall_pos > 0, "side-wall hit with c > 0 (M.x) never reached");
    assert!(sidewall_neg > 0, "side-wall hit with c <= 0 (skew(M.y)) never reached");
    assert!(reject > 0, "final reject never reached");
    d.finish();
}

/// Rows 44, 45 — degenerate axis, extreme radii, zero denominator, fuzz.
#[test]
fn cfg_44_45_capsule_degenerate() {
    let p = load();
    let mut d = Diff::new("cfg_44_45_capsule_degenerate");
    let mut rng = Rng::new(0x4445);
    let sp = specials();
    let base = [-3.0f32, 0.0, 1.0, 0.0, 10.0, 0.0, 0.0, 0.0, 4.0, 1.0];
    for slot in 0..10usize {
        for &s in &sp {
            for &s2 in &sp {
                let mut f = base;
                f[slot] = s;
                f[(slot + 5) % 10] = s2;
                let a = c2Ray { p: v(f[0], f[1]), d: v(f[2], f[3]), t: f[4] };
                let b = c2Capsule { a: v(f[5], f[6]), b: v(f[7], f[8]), r: f[9] };
                chk!(d, &p, capsule_pair, a, b);
            }
        }
    }
    // Degenerate axis (a == b) → norm((0,0)) = NaN
    for _ in 0..20_000 {
        let q = rng.vec_uniform(10.0);
        for &r in &[0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NAN, f32::MAX] {
            let b = c2Capsule { a: q, b: q, r };
            chk!(d, &p, capsule_pair, rand_ray(&mut rng, 10.0), b);
            chk!(d, &p, capsule_pair, normalized_ray(&mut rng, 10.0), b);
        }
    }
    // Axis-aligned capsules (vertical / horizontal / 45°) with extreme radii.
    for _ in 0..20_000 {
        let q = rng.vec_uniform(10.0);
        let axes = [
            v(q.x + 5.0, q.y),
            v(q.x, q.y + 5.0),
            v(q.x + 5.0, q.y + 5.0),
            v(q.x - 5.0, q.y),
            v(q.x, q.y - 5.0),
        ];
        for e in axes {
            for &r in &[0.0f32, -0.0, -3.0, 1e-30, 1e30, f32::MIN_POSITIVE] {
                let b = c2Capsule { a: q, b: e, r };
                chk!(d, &p, capsule_pair, rand_ray(&mut rng, 10.0), b);
            }
        }
    }
    // Row 45: `d = yAe.x - yAp.x == 0` → the *unguarded* division. Achieved
    // with a ray parallel to the capsule axis, or A.t == 0.
    for _ in 0..40_000 {
        let ca = rng.vec_uniform(10.0);
        let ang = rng.uniform(3.14159265);
        let axis = v(ang.cos(), ang.sin());
        let len = rng.positive(10.0) + 0.5;
        let cb = v(ca.x + axis.x * len, ca.y + axis.y * len);
        let r = rng.positive(1.0) + 0.01;
        let b = c2Capsule { a: ca, b: cb, r };
        let mx = v(axis.y, -axis.x);
        // origin far to one side, direction exactly along the axis → yAd.x = 0
        let o = v(ca.x + mx.x * (r + 3.0), ca.y + mx.y * (r + 3.0));
        for &t in &[0.0f32, -0.0, 1.0, 10.0, f32::INFINITY, f32::NAN, -1.0] {
            chk!(d, &p, capsule_pair, c2Ray { p: o, d: axis, t }, b);
            chk!(d, &p, capsule_pair, c2Ray { p: o, d: v(-axis.x, -axis.y), t }, b);
            chk!(d, &p, capsule_pair, c2Ray { p: o, d: v(0.0, 0.0), t }, b);
        }
    }
    // Full bit-pattern fuzz (row 58).
    for _ in 0..150_000 {
        let a = c2Ray { p: rng.vec_bits(), d: rng.vec_bits(), t: rng.any_bits() };
        let b = c2Capsule { a: rng.vec_bits(), b: rng.vec_bits(), r: rng.any_bits() };
        chk!(d, &p, capsule_pair, a, b);
    }
    for _ in 0..150_000 {
        let a = c2Ray { p: rng.vec_spicy(10.0), d: rng.vec_spicy(10.0), t: rng.spicy(10.0) };
        let b = c2Capsule {
            a: rng.vec_spicy(10.0),
            b: rng.vec_spicy(10.0),
            r: rng.spicy(10.0),
        };
        chk!(d, &p, capsule_pair, a, b);
    }
    d.finish();
}
