//! Phase B, CONFIGS.md rows 57-72: the per-shape-pair manifold generators.
//!
//! The output `c2Manifold` is pre-filled with a poison pattern and compared as all
//! 36 raw bytes, so "the C code leaves `depths` / `contact_points` / `n` untouched on
//! the no-contact path" is itself part of what is verified.
//!
//! Rows 63-72 reach `c2GJK` with `C2_TYPE_POLY`, where the C library reads an
//! uninitialised `c2Proxy`. [`zero_stack`] is called immediately before *both* the C
//! and the Rust invocation, which forces that region to all-zero and makes the C side
//! a deterministic function of its inputs again (see `tests/probe_uninit.rs`).
//! Those rows are the only way to reach the five `static` helpers `c2Clip`,
//! `c2SidePlanes`, `c2SidePlanesFromPoly`, `c2KeepDeep` and `c2Incident`.
#![allow(non_snake_case)]
#![allow(clippy::unnecessary_cast, clippy::needless_range_loop, clippy::let_and_return)]
#![allow(clippy::field_reassign_with_default)]

mod common;
use common::*;

const N: usize = 4_000;

// ---------------------------------------------------------------------------
// Row 57: c2CircletoCircleManifold
// ---------------------------------------------------------------------------

#[test]
fn row57_circle_circle() {
    let l = libs();
    let (cf, rf) = l.get::<FnCircleCircle>("c2CircletoCircleManifold");
    let mut rng = Rng::new(57);
    let mut contacts = 0u32;
    for regime in 0..8u32 {
        for i in 0..N {
            let (A, B) = match regime {
                0 => (
                    // far apart
                    c2Circle { p: v(0.0, 0.0), r: rng.f_pos(1.0) },
                    c2Circle { p: rng.vec_norm(500.0), r: rng.f_pos(1.0) },
                ),
                1 => {
                    // exactly touching: |d| == rA + rB on a lattice
                    let (ra, rb) = (rng.below(4) as f32, rng.below(4) as f32);
                    (
                        c2Circle { p: v(0.0, 0.0), r: ra },
                        c2Circle { p: v(ra + rb, 0.0), r: rb },
                    )
                }
                2 => (
                    // shallow / deep overlap
                    c2Circle { p: rng.vec_norm(5.0), r: rng.f_pos(5.0) },
                    c2Circle { p: rng.vec_norm(5.0), r: rng.f_pos(5.0) },
                ),
                3 => {
                    // concentric -> l == 0 -> fallback normal (0, 1)
                    let p = rng.vec_norm(10.0);
                    (
                        c2Circle { p, r: rng.f_pos(5.0) },
                        c2Circle { p, r: rng.f_pos(5.0) },
                    )
                }
                4 => (
                    // zero radii
                    c2Circle { p: rng.vec_lattice(3), r: 0.0 },
                    c2Circle { p: rng.vec_lattice(3), r: 0.0 },
                ),
                5 => (
                    // negative radii: `d2 < r*r` compares the SQUARE, so contact can
                    // still be reported
                    c2Circle { p: rng.vec_norm(5.0), r: -rng.f_pos(5.0) },
                    c2Circle { p: rng.vec_norm(5.0), r: -rng.f_pos(5.0) },
                ),
                6 => (
                    c2Circle { p: rng.vec_lattice(4), r: rng.f_half_lattice(2) },
                    c2Circle { p: rng.vec_lattice(4), r: rng.f_half_lattice(2) },
                ),
                _ => (
                    c2Circle { p: rng.vec_special(), r: rng.f_special() },
                    c2Circle { p: rng.vec_special(), r: rng.f_special() },
                ),
            };
            let mut cm = poison_manifold(41);
            let mut rm = poison_manifold(41);
            unsafe {
                cf(A, B, &mut cm);
                rf(A, B, &mut rm);
            }
            eq("c2CircletoCircleManifold", &format!("regime={regime} i={i} A={A:?} B={B:?}"), &cm, &rm);
            if cm.count > 0 {
                contacts += 1;
            }
        }
    }
    println!("row57 contacts reported: {contacts}");
    assert!(contacts > 0 && contacts < (N as u32 * 8), "row 57 did not cover both outcomes");
}

// ---------------------------------------------------------------------------
// Rows 58-59: c2CircletoAABBManifold
// ---------------------------------------------------------------------------

#[test]
fn row58_59_circle_aabb() {
    let l = libs();
    let (cf, rf) = l.get::<FnCircleAABB>("c2CircletoAABBManifold");
    let mut rng = Rng::new(58);
    let mut deep = 0u32;
    let mut shallow = 0u32;
    let mut none = 0u32;
    for regime in 0..9u32 {
        for i in 0..N {
            let (A, B) = match regime {
                0 => (
                    // far outside
                    c2Circle { p: rng.vec_norm(500.0), r: rng.f_pos(1.0) },
                    c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) },
                ),
                1 => {
                    // centre strictly inside -> d2 == 0 -> deep branch
                    let bb = c2AABB { min: v(-4.0, -3.0), max: v(4.0, 3.0) };
                    (c2Circle { p: v(rng.f_norm(3.5), rng.f_norm(2.5)), r: rng.f_pos(2.0) }, bb)
                }
                2 => {
                    // centre inside with x_overlap == y_overlap (tie -> y axis)
                    let bb = c2AABB { min: v(-2.0, -2.0), max: v(2.0, 2.0) };
                    let t = rng.f_lattice(1);
                    (c2Circle { p: v(t, t), r: rng.f_pos(2.0) }, bb)
                }
                3 => {
                    // centre exactly on an edge / corner
                    let bb = c2AABB { min: v(-2.0, -2.0), max: v(2.0, 2.0) };
                    let p = match rng.below(4) {
                        0 => v(2.0, 0.0),
                        1 => v(0.0, 2.0),
                        2 => v(2.0, 2.0),
                        _ => v(-2.0, -2.0),
                    };
                    (c2Circle { p, r: rng.f_half_lattice(2) }, bb)
                }
                4 => (
                    // overlapping an edge or corner
                    c2Circle { p: rng.vec_norm(4.0), r: rng.f_pos(2.0) },
                    c2AABB { min: v(-2.0, -1.0), max: v(2.0, 1.0) },
                ),
                5 => {
                    // inverted AABB
                    let max = rng.vec_norm(3.0);
                    (
                        c2Circle { p: rng.vec_norm(3.0), r: rng.f_pos(3.0) },
                        c2AABB { min: v(max.x + rng.f_pos(4.0), max.y + rng.f_pos(4.0)), max },
                    )
                }
                6 => {
                    // zero-extent AABB
                    let p = rng.vec_norm(3.0);
                    (c2Circle { p: rng.vec_norm(3.0), r: rng.f_pos(3.0) }, c2AABB { min: p, max: p })
                }
                7 => (
                    c2Circle { p: rng.vec_lattice(4), r: rng.f_half_lattice(2) },
                    c2AABB { min: rng.vec_lattice(3), max: rng.vec_lattice(3) },
                ),
                _ => (
                    c2Circle { p: rng.vec_special(), r: rng.f_special() },
                    c2AABB { min: rng.vec_special(), max: rng.vec_special() },
                ),
            };
            let mut cm = poison_manifold(42);
            let mut rm = poison_manifold(42);
            unsafe {
                cf(A, B, &mut cm);
                rf(A, B, &mut rm);
            }
            eq("c2CircletoAABBManifold", &format!("regime={regime} i={i} A={A:?} B={B:?}"), &cm, &rm);
            // Classify which of the three C paths ran, from the *output* only
            // (NaN-safe: no comparison chains on the inputs).
            //   count == 0                       -> no contact
            //   count == 1 and |n| is axis-aligned unit and depth includes A.r
            //                                    -> the d2 == 0 "deep" branch
            //   otherwise                        -> the ordinary d2 != 0 branch
            if cm.count == 0 {
                none += 1;
            } else if (cm.n.x == 1.0 || cm.n.x == -1.0 || cm.n.y == 1.0 || cm.n.y == -1.0)
                && (cm.n.x == 0.0 || cm.n.y == 0.0)
            {
                deep += 1;
            } else {
                shallow += 1;
            }
        }
    }
    println!("row58/59 paths: none={none} shallow={shallow} deep={deep}");
    assert!(none > 0 && shallow > 0 && deep > 0, "row 58/59 did not cover all three paths");
}

// ---------------------------------------------------------------------------
// Row 60: c2CircletoCapsuleManifold
// ---------------------------------------------------------------------------

#[test]
fn row60_circle_capsule() {
    let l = libs();
    let (cf, rf) = l.get::<FnCircleCapsule>("c2CircletoCapsuleManifold");
    let mut rng = Rng::new(60);
    let mut contacts = 0u32;
    for regime in 0..7u32 {
        for i in 0..N {
            let (A, B) = match regime {
                0 => (
                    c2Circle { p: rng.vec_norm(500.0), r: rng.f_pos(1.0) },
                    c2Capsule { a: v(-1.0, 0.0), b: v(1.0, 0.0), r: rng.f_pos(1.0) },
                ),
                1 => {
                    // circle centre exactly on the capsule axis -> d == 0
                    let (a, b) = (rng.vec_norm(5.0), rng.vec_norm(5.0));
                    let t = rng.f_pos(1.0);
                    (
                        c2Circle { p: v(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t), r: rng.f_pos(2.0) },
                        c2Capsule { a, b, r: rng.f_pos(2.0) },
                    )
                }
                2 => {
                    // degenerate capsule (a == b) -> c2Norm(c2Skew((0,0))) -> NaN normal
                    let p = rng.vec_norm(5.0);
                    (c2Circle { p: rng.vec_norm(5.0), r: rng.f_pos(3.0) }, c2Capsule { a: p, b: p, r: rng.f_pos(2.0) })
                }
                3 => (
                    c2Circle { p: rng.vec_norm(5.0), r: rng.f_pos(3.0) },
                    c2Capsule { a: rng.vec_norm(5.0), b: rng.vec_norm(5.0), r: rng.f_pos(3.0) },
                ),
                4 => (
                    // exact touch candidates on a lattice
                    c2Circle { p: rng.vec_lattice(4), r: rng.below(3) as f32 },
                    c2Capsule { a: rng.vec_lattice(4), b: rng.vec_lattice(4), r: rng.below(3) as f32 },
                ),
                5 => (
                    // zero / negative radii
                    c2Circle { p: rng.vec_norm(5.0), r: if rng.bool() { 0.0 } else { -rng.f_pos(2.0) } },
                    c2Capsule { a: rng.vec_norm(5.0), b: rng.vec_norm(5.0), r: if rng.bool() { 0.0 } else { -rng.f_pos(2.0) } },
                ),
                _ => (
                    c2Circle { p: rng.vec_special(), r: rng.f_special() },
                    c2Capsule { a: rng.vec_special(), b: rng.vec_special(), r: rng.f_special() },
                ),
            };
            let mut cm = poison_manifold(43);
            let mut rm = poison_manifold(43);
            unsafe {
                cf(A, B, &mut cm);
                rf(A, B, &mut rm);
            }
            eq("c2CircletoCapsuleManifold", &format!("regime={regime} i={i} A={A:?} B={B:?}"), &cm, &rm);
            if cm.count > 0 {
                contacts += 1;
            }
        }
    }
    println!("row60 contacts reported: {contacts}");
    assert!(contacts > 0, "row 60 never produced a contact");
}

// ---------------------------------------------------------------------------
// Row 61: c2AABBtoAABBManifold
// ---------------------------------------------------------------------------

#[test]
fn row61_aabb_aabb() {
    let l = libs();
    let (cf, rf) = l.get::<FnAABBAABB>("c2AABBtoAABBManifold");
    let mut rng = Rng::new(61);
    let mut none = 0u32;
    let mut x_minor = 0u32;
    let mut y_minor = 0u32;
    for regime in 0..9u32 {
        for i in 0..N {
            let unit = c2AABB { min: v(-2.0, -2.0), max: v(2.0, 2.0) };
            let (A, B) = match regime {
                0 => (unit, c2AABB { min: v(100.0, 0.0), max: v(102.0, 2.0) }), // separated x
                1 => (unit, c2AABB { min: v(0.0, 100.0), max: v(2.0, 102.0) }), // separated y
                2 => (unit, c2AABB { min: v(1.0, -0.5), max: v(5.0, 0.5) }),    // x-minor overlap
                3 => (unit, c2AABB { min: v(-0.5, 1.0), max: v(0.5, 5.0) }),    // y-minor overlap
                4 => (unit, unit),                                              // identical -> dx == dy tie
                5 => (unit, c2AABB { min: v(-0.5, -0.5), max: v(0.5, 0.5) }),    // one inside the other
                6 => {
                    // inverted
                    let max = rng.vec_norm(3.0);
                    (
                        c2AABB { min: v(max.x + rng.f_pos(3.0), max.y + rng.f_pos(3.0)), max },
                        c2AABB { min: rng.vec_norm(3.0), max: rng.vec_norm(3.0) },
                    )
                }
                7 => {
                    // half-integer lattice: exact touches and ties are frequent
                    let amin = v(rng.f_half_lattice(3), rng.f_half_lattice(3));
                    let bmin = v(rng.f_half_lattice(3), rng.f_half_lattice(3));
                    (
                        c2AABB { min: amin, max: v(amin.x + rng.below(5) as f32 * 0.5, amin.y + rng.below(5) as f32 * 0.5) },
                        c2AABB { min: bmin, max: v(bmin.x + rng.below(5) as f32 * 0.5, bmin.y + rng.below(5) as f32 * 0.5) },
                    )
                }
                _ => (
                    c2AABB { min: rng.vec_special(), max: rng.vec_special() },
                    c2AABB { min: rng.vec_special(), max: rng.vec_special() },
                ),
            };
            let mut cm = poison_manifold(44);
            let mut rm = poison_manifold(44);
            unsafe {
                cf(A, B, &mut cm);
                rf(A, B, &mut rm);
            }
            eq("c2AABBtoAABBManifold", &format!("regime={regime} i={i} A={A:?} B={B:?}"), &cm, &rm);
            if cm.count == 0 {
                none += 1;
            } else if cm.n.x != 0.0 {
                x_minor += 1;
            } else {
                y_minor += 1;
            }
        }
    }
    println!("row61 paths: none={none} x_axis={x_minor} y_axis={y_minor}");
    assert!(none > 0 && x_minor > 0 && y_minor > 0, "row 61 did not cover all paths");
}

// ---------------------------------------------------------------------------
// Row 62: c2CapsuletoCapsuleManifold
// ---------------------------------------------------------------------------

#[test]
fn row62_capsule_capsule() {
    let l = libs();
    let (cf, rf) = l.get::<FnCapsuleCapsule>("c2CapsuletoCapsuleManifold");
    let mut rng = Rng::new(62);
    let mut contacts = 0u32;
    for regime in 0..8u32 {
        for i in 0..N {
            let o = rng.vec_norm(4.0);
            let d = rng.vec_norm(4.0);
            let (A, B) = match regime {
                0 => (
                    c2Capsule { a: v(-1.0, 0.0), b: v(1.0, 0.0), r: 0.5 },
                    c2Capsule { a: v(100.0, 0.0), b: v(102.0, 0.0), r: 0.5 },
                ),
                1 => (
                    // parallel
                    c2Capsule { a: o, b: v(o.x + d.x, o.y + d.y), r: rng.f_pos(1.0) },
                    c2Capsule { a: v(o.x - d.y, o.y + d.x), b: v(o.x + d.x - d.y, o.y + d.y + d.x), r: rng.f_pos(1.0) },
                ),
                2 => (
                    // crossing
                    c2Capsule { a: v(o.x - d.x, o.y - d.y), b: v(o.x + d.x, o.y + d.y), r: rng.f_pos(1.0) },
                    c2Capsule { a: v(o.x - d.y, o.y + d.x), b: v(o.x + d.y, o.y - d.x), r: rng.f_pos(1.0) },
                ),
                3 => (
                    // collinear
                    c2Capsule { a: o, b: v(o.x + d.x, o.y + d.y), r: rng.f_pos(1.0) },
                    c2Capsule { a: v(o.x + 2.0 * d.x, o.y + 2.0 * d.y), b: v(o.x + 3.0 * d.x, o.y + 3.0 * d.y), r: rng.f_pos(1.0) },
                ),
                4 => {
                    // both degenerate
                    let (p, q) = (rng.vec_norm(4.0), rng.vec_norm(4.0));
                    (c2Capsule { a: p, b: p, r: rng.f_pos(2.0) }, c2Capsule { a: q, b: q, r: rng.f_pos(2.0) })
                }
                5 => {
                    // A degenerate only -> `d == 0` branch takes c2Norm(c2Skew(A.b-A.a))
                    let p = rng.vec_norm(4.0);
                    (
                        c2Capsule { a: p, b: p, r: rng.f_pos(2.0) },
                        c2Capsule { a: rng.vec_norm(4.0), b: rng.vec_norm(4.0), r: rng.f_pos(2.0) },
                    )
                }
                6 => (
                    c2Capsule { a: rng.vec_lattice(4), b: rng.vec_lattice(4), r: rng.below(3) as f32 },
                    c2Capsule { a: rng.vec_lattice(4), b: rng.vec_lattice(4), r: rng.below(3) as f32 },
                ),
                _ => (
                    c2Capsule { a: rng.vec_special(), b: rng.vec_special(), r: rng.f_special() },
                    c2Capsule { a: rng.vec_special(), b: rng.vec_special(), r: rng.f_special() },
                ),
            };
            let mut cm = poison_manifold(45);
            let mut rm = poison_manifold(45);
            unsafe {
                cf(A, B, &mut cm);
                rf(A, B, &mut rm);
            }
            eq("c2CapsuletoCapsuleManifold", &format!("regime={regime} i={i} A={A:?} B={B:?}"), &cm, &rm);
            if cm.count > 0 {
                contacts += 1;
            }
        }
    }
    println!("row62 contacts reported: {contacts}");
    assert!(contacts > 0, "row 62 never produced a contact");
}

// ---------------------------------------------------------------------------
// Rows 63-70: c2CapsuletoPolyManifold  (POLY path -- needs zero_stack)
// ---------------------------------------------------------------------------

fn call_capsule_poly(
    f: &libloading::Symbol<'_, FnCapsulePoly>,
    cap: c2Capsule,
    poly: &c2Poly,
    bx: Option<&c2x>,
    seed: u8,
) -> c2Manifold {
    let mut m = poison_manifold(seed);
    zero_stack();
    unsafe {
        f(cap, poly, bx.map_or(std::ptr::null(), |x| x as *const c2x), &mut m);
    }
    m
}

fn warm_capsule_poly(cf: &libloading::Symbol<'_, FnCapsulePoly>, rf: &libloading::Symbol<'_, FnCapsulePoly>) {
    let mut p = c2Poly::default();
    p.count = 4;
    p.verts[0] = v(-1.0, -1.0);
    p.verts[1] = v(1.0, -1.0);
    p.verts[2] = v(1.0, 1.0);
    p.verts[3] = v(-1.0, 1.0);
    fill_norms(&mut p);
    let cap = c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 };
    let x = x_identity();
    warmup(|| {
        let _ = call_capsule_poly(cf, cap, &p, None, 1);
        let _ = call_capsule_poly(rf, cap, &p, None, 1);
        let _ = call_capsule_poly(cf, cap, &p, Some(&x), 1);
        let _ = call_capsule_poly(rf, cap, &p, Some(&x), 1);
    });
}

#[test]
fn row63_to_70_capsule_poly() {
    let l = libs();
    let (cf, rf) = l.get::<FnCapsulePoly>("c2CapsuletoPolyManifold");
    warm_capsule_poly(&cf, &rf);
    let mut rng = Rng::new(63);

    // Under the zero proxy the GJK distance is |capsule segment -> origin|, so
    // placing the capsule across the origin gives d == 0 (the `d < 1e-6` face /
    // side-plane branch), and placing it away from the origin gives the shallow
    // or the no-contact branch. Both are exercised.
    let mut by_count = [0u32; 3];
    let mut contact_kinds = std::collections::BTreeSet::new();

    for count in 3i32..=8 {
        for regime in 0..7u32 {
            for i in 0..N / 2 {
                let (rad, ctr) = (0.5 + rng.f_pos(6.0), rng.vec_norm(4.0));
                let mut poly = if regime == 6 {
                    concave_wound_poly(&mut rng, count, rad, ctr) // row 69: CW winding
                } else {
                    convex_poly(&mut rng, count, rad, ctr)
                };
                fill_norms(&mut poly);

                let cap = match regime {
                    // straddles the origin -> d == 0 -> deep branch
                    0 => {
                        let d = rng.vec_norm(6.0);
                        c2Capsule { a: d, b: v(-d.x, -d.y), r: rng.f_pos(2.0) }
                    }
                    // shallow band: 1e-6 <= d < A.r
                    1 => {
                        let d = rng.vec_norm(1.0);
                        let off = 0.5 + rng.f_pos(1.0);
                        c2Capsule {
                            a: v(d.x + off, d.y + off),
                            b: v(d.x + off + 1.0, d.y + off + 1.0),
                            r: off * 4.0,
                        }
                    }
                    // far away -> no contact
                    2 => c2Capsule {
                        a: v(500.0 + rng.f_norm(5.0), 500.0),
                        b: v(505.0, 505.0),
                        r: rng.f_pos(2.0),
                    },
                    // lattice: exact zeros in the clip distances
                    3 => c2Capsule { a: rng.vec_lattice(4), b: rng.vec_lattice(4), r: rng.below(4) as f32 },
                    // degenerate capsule -> ab == NaN -> index stays -1
                    4 => {
                        let p = rng.vec_norm(3.0);
                        c2Capsule { a: p, b: p, r: rng.f_pos(2.0) }
                    }
                    // random
                    5 => c2Capsule { a: rng.vec_norm(6.0), b: rng.vec_norm(6.0), r: rng.f_pos(3.0) },
                    _ => {
                        let d = rng.vec_norm(6.0);
                        c2Capsule { a: d, b: v(-d.x, -d.y), r: rng.f_pos(2.0) }
                    }
                };

                // rows 63/65/68: bx_ptr NULL and non-NULL
                let bx = rng.xform(5.0);
                for use_bx in [false, true] {
                    let o = if use_bx { Some(&bx) } else { None };
                    let cm = call_capsule_poly(&cf, cap, &poly, o, 51);
                    let rm = call_capsule_poly(&rf, cap, &poly, o, 51);
                    eq(
                        "c2CapsuletoPolyManifold",
                        &format!("count={count} regime={regime} i={i} use_bx={use_bx} cap={cap:?} poly={poly:?} bx={bx:?}"),
                        &cm,
                        &rm,
                    );
                    by_count[cm.count.clamp(0, 2) as usize] += 1;
                    contact_kinds.insert(cm.count);
                }
            }
        }
    }
    println!("row63-70 manifold counts: 0={} 1={} 2={} (kinds seen {contact_kinds:?})",
        by_count[0], by_count[1], by_count[2]);
    assert!(by_count[0] > 0, "never hit the no-contact path");
    assert!(by_count[1] > 0, "never hit a 1-point manifold");
    assert!(by_count[2] > 0, "never hit a 2-point manifold (c2KeepDeep with both points)");
}

/// Row 70 explicitly: `m` pre-poisoned with several distinct patterns, so any
/// difference in *which* fields the early-return paths leave alone is caught.
#[test]
fn row70_capsule_poly_poison_patterns() {
    let l = libs();
    let (cf, rf) = l.get::<FnCapsulePoly>("c2CapsuletoPolyManifold");
    warm_capsule_poly(&cf, &rf);
    let mut rng = Rng::new(70);
    for seed in [0u8, 1, 17, 63, 127, 200, 255] {
        for i in 0..N {
            let (rad, ctr) = (0.5 + rng.f_pos(5.0), rng.vec_norm(4.0));
            let mut poly = convex_poly(&mut rng, 3 + (i % 6) as i32, rad, ctr);
            fill_norms(&mut poly);
            let cap = match i % 3 {
                0 => c2Capsule { a: v(1000.0, 1000.0), b: v(1001.0, 1001.0), r: 0.5 }, // no contact
                1 => {
                    let d = rng.vec_norm(5.0);
                    c2Capsule { a: d, b: v(-d.x, -d.y), r: rng.f_pos(2.0) }
                }
                _ => c2Capsule { a: rng.vec_norm(6.0), b: rng.vec_norm(6.0), r: rng.f_pos(3.0) },
            };
            let cm = call_capsule_poly(&cf, cap, &poly, None, seed);
            let rm = call_capsule_poly(&rf, cap, &poly, None, seed);
            eq("c2CapsuletoPolyManifold poison", &format!("seed={seed} i={i} cap={cap:?}"), &cm, &rm);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 71-72: c2AABBtoCapsuleManifold  (POLY path -- needs zero_stack)
// ---------------------------------------------------------------------------

fn call_aabb_capsule(
    f: &libloading::Symbol<'_, FnAABBCapsule>,
    A: c2AABB,
    B: c2Capsule,
    seed: u8,
) -> c2Manifold {
    let mut m = poison_manifold(seed);
    zero_stack();
    unsafe { f(A, B, &mut m) };
    m
}

#[test]
fn row71_72_aabb_capsule() {
    let l = libs();
    let (cf, rf) = l.get::<FnAABBCapsule>("c2AABBtoCapsuleManifold");
    let unit = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
    let cap0 = c2Capsule { a: v(-2.0, 0.0), b: v(2.0, 0.0), r: 0.5 };
    warmup(|| {
        let _ = call_aabb_capsule(&cf, unit, cap0, 1);
        let _ = call_aabb_capsule(&rf, unit, cap0, 1);
    });

    let mut rng = Rng::new(71);
    let mut by_count = [0u32; 3];
    for regime in 0..9u32 {
        for i in 0..N {
            let (A, B) = match regime {
                // separated
                0 => (unit, c2Capsule { a: v(100.0, 0.0), b: v(102.0, 0.0), r: 0.5 }),
                // capsule crossing the box through the middle
                1 => (unit, c2Capsule { a: v(-3.0, rng.f_norm(0.9)), b: v(3.0, rng.f_norm(0.9)), r: rng.f_pos(1.0) }),
                // axis parallel to an edge
                2 => {
                    let y = rng.f_half_lattice(2);
                    (unit, c2Capsule { a: v(-3.0, y), b: v(3.0, y), r: rng.f_pos(1.0) })
                }
                // axis through a corner
                3 => (unit, c2Capsule { a: v(-3.0, -3.0), b: v(3.0, 3.0), r: rng.f_pos(1.0) }),
                // capsule entirely inside
                4 => (
                    c2AABB { min: v(-5.0, -5.0), max: v(5.0, 5.0) },
                    c2Capsule { a: rng.vec_norm(2.0), b: rng.vec_norm(2.0), r: rng.f_pos(0.5) },
                ),
                // degenerate capsule
                5 => {
                    let p = rng.vec_norm(2.0);
                    (unit, c2Capsule { a: p, b: p, r: rng.f_pos(1.0) })
                }
                // degenerate AABB -> c2Norms yields 4 NaN normals
                6 => {
                    let p = rng.vec_norm(2.0);
                    (c2AABB { min: p, max: p }, c2Capsule { a: rng.vec_norm(3.0), b: rng.vec_norm(3.0), r: rng.f_pos(1.0) })
                }
                // lattice
                7 => {
                    let min = v(rng.f_half_lattice(3), rng.f_half_lattice(3));
                    (
                        c2AABB { min, max: v(min.x + rng.below(5) as f32 * 0.5, min.y + rng.below(5) as f32 * 0.5) },
                        c2Capsule { a: rng.vec_lattice(3), b: rng.vec_lattice(3), r: rng.below(3) as f32 },
                    )
                }
                // pathological
                _ => (
                    c2AABB { min: rng.vec_special(), max: rng.vec_special() },
                    c2Capsule { a: rng.vec_special(), b: rng.vec_special(), r: rng.f_special() },
                ),
            };
            for seed in [17u8, 200] {
                let cm = call_aabb_capsule(&cf, A, B, seed);
                let rm = call_aabb_capsule(&rf, A, B, seed);
                eq(
                    "c2AABBtoCapsuleManifold",
                    &format!("regime={regime} i={i} seed={seed} A={A:?} B={B:?}"),
                    &cm,
                    &rm,
                );
                by_count[cm.count.clamp(0, 2) as usize] += 1;
            }
        }
    }
    println!("row71/72 manifold counts: 0={} 1={} 2={}", by_count[0], by_count[1], by_count[2]);
    assert!(by_count[0] > 0 && by_count[2] > 0, "row 71/72 did not cover both extremes");
}
