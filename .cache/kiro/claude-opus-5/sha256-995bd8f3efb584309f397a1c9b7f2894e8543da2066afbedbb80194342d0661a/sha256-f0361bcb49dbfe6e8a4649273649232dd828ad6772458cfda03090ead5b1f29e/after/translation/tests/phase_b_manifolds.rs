//! Phase B — CONFIGS.md rows 40..55: the per-pair manifold functions,
//! including the low-level `c2CapsuletoPolyManifold` with all three `code`
//! branches and every transform variant.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn rand_circle(rng: &mut Rng, spread: f32, rmax: f32) -> c2Circle {
    c2Circle {
        p: rng.vec(spread),
        r: rng.fpos(rmax),
    }
}

fn rand_aabb(rng: &mut Rng, spread: f32, emax: f32) -> c2AABB {
    let c = rng.vec(spread);
    let e = v(rng.fpos(emax), rng.fpos(emax));
    c2AABB {
        min: v(c.x - e.x, c.y - e.y),
        max: v(c.x + e.x, c.y + e.y),
    }
}

fn rand_capsule(rng: &mut Rng, spread: f32, rmax: f32) -> c2Capsule {
    let c = rng.vec(spread);
    let d = rng.vec(spread.max(1.0));
    c2Capsule {
        a: v(c.x - d.x, c.y - d.y),
        b: v(c.x + d.x, c.y + d.y),
        r: rng.fpos(rmax),
    }
}

/// Regular convex polygon with `count` vertices, CCW, normals from `c2Norms`
/// via the C library so the input is exactly what the library expects.
fn regular_poly(l: &'static Libs, count: c_int, radius: f32, center: c2v, phase: f32) -> c2Poly {
    let (c_norms, _) = l.pair::<FnNorms>("c2Norms");
    let mut p = c2Poly::default();
    p.count = count;
    for i in 0..count as usize {
        let t = phase + std::f32::consts::TAU * (i as f32) / (count as f32);
        p.verts[i] = v(center.x + radius * t.cos(), center.y + radius * t.sin());
    }
    unsafe {
        c_norms(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), count);
    }
    p
}

// --------------------------------------------------------------- rows 40 / 49
#[test]
fn row40_circle_to_circle() {
    let l = libs();
    let (cf, rf) = l.pair::<FnCircleCircle>("c2CircletoCircleManifold");
    let mut rng = Rng::new(0xAB01_CD02_EF03_1204);
    let mut hits = 0usize;
    let mut misses = 0usize;
    for i in 0..30_000usize {
        let (a, b) = match i % 6 {
            // separated
            0 => (rand_circle(&mut rng, 50.0, 2.0), rand_circle(&mut rng, 50.0, 2.0)),
            // shallow / deep overlap
            1 | 2 => (rand_circle(&mut rng, 3.0, 3.0), rand_circle(&mut rng, 3.0, 3.0)),
            // concentric: l == 0
            3 => {
                let c = rand_circle(&mut rng, 10.0, 5.0);
                (c, c2Circle { p: c.p, r: rng.fpos(5.0) })
            }
            // zero radius
            4 => (
                c2Circle { p: rng.vec(3.0), r: 0.0 },
                c2Circle { p: rng.vec(3.0), r: rng.fpos(3.0) },
            ),
            // grid-quantized so exact tangency (d2 == r*r) occurs
            _ => (
                c2Circle { p: v(rng.grid(0.5, 12), rng.grid(0.5, 12)), r: rng.grid(0.5, 6).abs() },
                c2Circle { p: v(rng.grid(0.5, 12), rng.grid(0.5, 12)), r: rng.grid(0.5, 6).abs() },
            ),
        };
        let mut cm = seeded_manifold(-4242.0);
        let mut rm = cm;
        unsafe {
            cf(a, b, &mut cm);
            rf(a, b, &mut rm);
        }
        if cm.count > 0 { hits += 1 } else { misses += 1 }
        assert!(
            meq(&cm, &rm),
            "c2CircletoCircleManifold {i}: A={a:?} B={b:?}\n  C {}\n  R {}",
            ms(&cm),
            ms(&rm)
        );
    }
    assert!(hits > 500 && misses > 500, "coverage hits={hits} misses={misses}");
}

// ----------------------------------------------------------------------- row 41
#[test]
fn row41_circle_to_aabb() {
    let l = libs();
    let (cf, rf) = l.pair::<FnCircleAABB>("c2CircletoAABBManifold");
    let mut rng = Rng::new(0x5501_6602_7703_8804);
    let mut deep = 0usize;
    let mut shallow = 0usize;
    let mut misses = 0usize;
    for i in 0..40_000usize {
        let (a, b) = match i % 7 {
            0 => (rand_circle(&mut rng, 50.0, 2.0), rand_aabb(&mut rng, 50.0, 3.0)),
            // face / corner overlaps
            1 | 2 => (rand_circle(&mut rng, 4.0, 3.0), rand_aabb(&mut rng, 4.0, 3.0)),
            // centre strictly inside the box: d2 == 0 deep branch
            3 | 4 => {
                let bb = rand_aabb(&mut rng, 5.0, 4.0);
                let mid = v((bb.min.x + bb.max.x) * 0.5, (bb.min.y + bb.max.y) * 0.5);
                (c2Circle { p: mid, r: rng.fpos(3.0) }, bb)
            }
            // zero-radius circle (ERRORS.md row 53)
            5 => (
                c2Circle { p: rng.vec(4.0), r: 0.0 },
                rand_aabb(&mut rng, 4.0, 3.0),
            ),
            // zero-extent AABB + grid coords for exact-boundary cases
            _ => {
                let p = v(rng.grid(0.5, 10), rng.grid(0.5, 10));
                (
                    c2Circle { p: v(rng.grid(0.5, 10), rng.grid(0.5, 10)), r: rng.grid(0.5, 6).abs() },
                    c2AABB { min: p, max: p },
                )
            }
        };
        let mut cm = seeded_manifold(-4242.0);
        let mut rm = cm;
        unsafe {
            cf(a, b, &mut cm);
            rf(a, b, &mut rm);
        }
        let ab = v(
            a.p.x.clamp(b.min.x.min(b.max.x), b.max.x.max(b.min.x)) - a.p.x,
            a.p.y.clamp(b.min.y.min(b.max.y), b.max.y.max(b.min.y)) - a.p.y,
        );
        if cm.count == 0 {
            misses += 1;
        } else if ab.x == 0.0 && ab.y == 0.0 {
            deep += 1;
        } else {
            shallow += 1;
        }
        assert!(
            meq(&cm, &rm),
            "c2CircletoAABBManifold {i}: A={a:?} B={b:?}\n  C {}\n  R {}",
            ms(&cm),
            ms(&rm)
        );
    }
    assert!(
        deep > 100 && shallow > 100 && misses > 100,
        "coverage deep={deep} shallow={shallow} misses={misses}"
    );
}

// ----------------------------------------------------------------------- row 42
#[test]
fn row42_circle_to_capsule() {
    let l = libs();
    let (cf, rf) = l.pair::<FnCircleCapsule>("c2CircletoCapsuleManifold");
    let mut rng = Rng::new(0x1102_2203_3304_4405);
    let mut hits = 0usize;
    let mut misses = 0usize;
    for i in 0..30_000usize {
        let (a, b) = match i % 6 {
            0 => (rand_circle(&mut rng, 60.0, 2.0), rand_capsule(&mut rng, 60.0, 2.0)),
            1 | 2 => (rand_circle(&mut rng, 4.0, 3.0), rand_capsule(&mut rng, 4.0, 3.0)),
            // circle centre exactly on the capsule segment: d == 0
            3 => {
                let cap = rand_capsule(&mut rng, 5.0, 3.0);
                let t = rng.fpos(1.0);
                let p = v(
                    cap.a.x + (cap.b.x - cap.a.x) * t,
                    cap.a.y + (cap.b.y - cap.a.y) * t,
                );
                (c2Circle { p, r: rng.fpos(3.0) }, cap)
            }
            // point capsule (a == b)
            4 => {
                let p = rng.vec(4.0);
                (
                    rand_circle(&mut rng, 4.0, 3.0),
                    c2Capsule { a: p, b: p, r: rng.fpos(2.0) },
                )
            }
            // grid coords
            _ => (
                c2Circle { p: v(rng.grid(0.5, 10), rng.grid(0.5, 10)), r: rng.grid(0.5, 5).abs() },
                c2Capsule {
                    a: v(rng.grid(0.5, 10), rng.grid(0.5, 10)),
                    b: v(rng.grid(0.5, 10), rng.grid(0.5, 10)),
                    r: rng.grid(0.5, 5).abs(),
                },
            ),
        };
        let mut cm = seeded_manifold(-4242.0);
        let mut rm = cm;
        unsafe {
            cf(a, b, &mut cm);
            rf(a, b, &mut rm);
        }
        if cm.count > 0 { hits += 1 } else { misses += 1 }
        assert!(
            meq(&cm, &rm),
            "c2CircletoCapsuleManifold {i}: A={a:?} B={b:?}\n  C {}\n  R {}",
            ms(&cm),
            ms(&rm)
        );
    }
    assert!(hits > 500 && misses > 500, "coverage hits={hits} misses={misses}");
}

// ----------------------------------------------------------------------- row 43
#[test]
fn row43_aabb_to_aabb() {
    let l = libs();
    let (cf, rf) = l.pair::<FnAABBAABB>("c2AABBtoAABBManifold");
    let mut rng = Rng::new(0x7A0B_8C0D_9E0F_A011);
    let mut hits = 0usize;
    let mut misses = 0usize;
    for i in 0..40_000usize {
        let (a, b) = match i % 7 {
            0 => (rand_aabb(&mut rng, 60.0, 3.0), rand_aabb(&mut rng, 60.0, 3.0)),
            1 | 2 | 3 => (rand_aabb(&mut rng, 4.0, 3.0), rand_aabb(&mut rng, 4.0, 3.0)),
            // identical boxes
            4 => {
                let x = rand_aabb(&mut rng, 5.0, 3.0);
                (x, x)
            }
            // zero extent
            5 => {
                let p = rng.vec(4.0);
                (c2AABB { min: p, max: p }, rand_aabb(&mut rng, 4.0, 3.0))
            }
            // inverted (min > max) + grid coords for exact touching
            _ => (
                c2AABB {
                    min: v(rng.grid(1.0, 6), rng.grid(1.0, 6)),
                    max: v(rng.grid(1.0, 6), rng.grid(1.0, 6)),
                },
                c2AABB {
                    min: v(rng.grid(1.0, 6), rng.grid(1.0, 6)),
                    max: v(rng.grid(1.0, 6), rng.grid(1.0, 6)),
                },
            ),
        };
        let mut cm = seeded_manifold(-4242.0);
        let mut rm = cm;
        unsafe {
            cf(a, b, &mut cm);
            rf(a, b, &mut rm);
        }
        if cm.count > 0 { hits += 1 } else { misses += 1 }
        assert!(
            meq(&cm, &rm),
            "c2AABBtoAABBManifold {i}: A={a:?} B={b:?}\n  C {}\n  R {}",
            ms(&cm),
            ms(&rm)
        );
    }
    assert!(hits > 500 && misses > 500, "coverage hits={hits} misses={misses}");
}

// ----------------------------------------------------------------------- row 44
#[test]
fn row44_capsule_to_capsule() {
    let l = libs();
    let (cf, rf) = l.pair::<FnCapsuleCapsule>("c2CapsuletoCapsuleManifold");
    let mut rng = Rng::new(0xB1C2_D3E4_F506_1728);
    let mut hits = 0usize;
    let mut misses = 0usize;
    for i in 0..30_000usize {
        let (a, b) = match i % 7 {
            0 => (rand_capsule(&mut rng, 60.0, 2.0), rand_capsule(&mut rng, 60.0, 2.0)),
            1 | 2 => (rand_capsule(&mut rng, 4.0, 3.0), rand_capsule(&mut rng, 4.0, 3.0)),
            // parallel
            3 => {
                let a = rand_capsule(&mut rng, 4.0, 2.0);
                let off = rng.vec(3.0);
                (
                    a,
                    c2Capsule {
                        a: v(a.a.x + off.x, a.a.y + off.y),
                        b: v(a.b.x + off.x, a.b.y + off.y),
                        r: rng.fpos(2.0),
                    },
                )
            }
            // collinear / end-to-end
            4 => {
                let a = rand_capsule(&mut rng, 4.0, 2.0);
                let d = v(a.b.x - a.a.x, a.b.y - a.a.y);
                (
                    a,
                    c2Capsule {
                        a: a.b,
                        b: v(a.b.x + d.x, a.b.y + d.y),
                        r: rng.fpos(2.0),
                    },
                )
            }
            // point capsules
            5 => {
                let p = rng.vec(4.0);
                let q = rng.vec(4.0);
                (
                    c2Capsule { a: p, b: p, r: rng.fpos(2.0) },
                    c2Capsule { a: q, b: q, r: rng.fpos(2.0) },
                )
            }
            // crossing, grid coords
            _ => (
                c2Capsule {
                    a: v(rng.grid(0.5, 8), rng.grid(0.5, 8)),
                    b: v(rng.grid(0.5, 8), rng.grid(0.5, 8)),
                    r: rng.grid(0.5, 4).abs(),
                },
                c2Capsule {
                    a: v(rng.grid(0.5, 8), rng.grid(0.5, 8)),
                    b: v(rng.grid(0.5, 8), rng.grid(0.5, 8)),
                    r: rng.grid(0.5, 4).abs(),
                },
            ),
        };
        let mut cm = seeded_manifold(-4242.0);
        let mut rm = cm;
        unsafe {
            cf(a, b, &mut cm);
            rf(a, b, &mut rm);
        }
        if cm.count > 0 { hits += 1 } else { misses += 1 }
        assert!(
            meq(&cm, &rm),
            "c2CapsuletoCapsuleManifold {i}: A={a:?} B={b:?}\n  C {}\n  R {}",
            ms(&cm),
            ms(&rm)
        );
    }
    assert!(hits > 500 && misses > 500, "coverage hits={hits} misses={misses}");
}

// ----------------------------------------------------------------------- row 45
#[test]
fn row45_aabb_to_capsule() {
    let l = libs();
    let (cf, rf) = l.pair::<FnAABBCapsule>("c2AABBtoCapsuleManifold");
    let mut rng = Rng::new(0xC2D3_E4F5_0617_2839);
    let mut hits = 0usize;
    let mut misses = 0usize;
    for i in 0..30_000usize {
        let (a, b) = match i % 7 {
            0 => (rand_aabb(&mut rng, 60.0, 3.0), rand_capsule(&mut rng, 60.0, 2.0)),
            // crossing a face / corner / fully inside
            1 | 2 | 3 => (rand_aabb(&mut rng, 4.0, 4.0), rand_capsule(&mut rng, 4.0, 2.0)),
            // axis-aligned capsule
            4 => {
                let bb = rand_aabb(&mut rng, 4.0, 3.0);
                let y = rng.f(4.0);
                (
                    bb,
                    c2Capsule { a: v(-6.0, y), b: v(6.0, y), r: rng.fpos(2.0) },
                )
            }
            // point capsule
            5 => {
                let p = rng.vec(4.0);
                (
                    rand_aabb(&mut rng, 4.0, 3.0),
                    c2Capsule { a: p, b: p, r: rng.fpos(2.0) },
                )
            }
            // grid coords
            _ => (
                c2AABB {
                    min: v(rng.grid(1.0, 5), rng.grid(1.0, 5)),
                    max: v(rng.grid(1.0, 5), rng.grid(1.0, 5)),
                },
                c2Capsule {
                    a: v(rng.grid(1.0, 5), rng.grid(1.0, 5)),
                    b: v(rng.grid(1.0, 5), rng.grid(1.0, 5)),
                    r: rng.grid(0.5, 4).abs(),
                },
            ),
        };
        let mut cm = seeded_manifold(-4242.0);
        let mut rm = cm;
        unsafe {
            scrub_stack();
            cf(a, b, &mut cm);
            scrub_stack();
            rf(a, b, &mut rm);
        }
        if cm.count > 0 { hits += 1 } else { misses += 1 }
        assert!(
            meq(&cm, &rm),
            "c2AABBtoCapsuleManifold {i}: A={a:?} B={b:?}\n  C {}\n  R {}",
            ms(&cm),
            ms(&rm)
        );
    }
    assert!(hits > 200 && misses > 200, "coverage hits={hits} misses={misses}");
}

// -------------------------------------------------------------- rows 46..55
#[test]
fn rows46_55_capsule_to_poly() {
    let l = libs();
    let (cf, rf) = l.pair::<FnCapsulePoly>("c2CapsuletoPolyManifold");
    let mut rng = Rng::new(0xD3E4_F506_1728_394A);

    let mut counts = [0usize; 3]; // manifold count 0/1/2 histogram
    for i in 0..40_000usize {
        // rows 53/54: vertex counts 3..8
        let n = 3 + (i % 6) as c_int;
        let poly = regular_poly(
            l,
            n,
            1.0 + rng.fpos(4.0),
            rng.vec(3.0),
            rng.f(std::f32::consts::PI),
        );
        // rows 46..49: the deep branch requires `d < 1e-6`. Because the C
        // `c2MakeProxy` has no poly case, `c2GJK`'s `pB` is the (zeroed) proxy,
        // i.e. a single point at the origin — so `d` is the capsule segment's
        // distance to (0,0) and the deep branch is reached exactly when the
        // segment passes through the origin. Construct that explicitly for half
        // the iterations so `code` 0/1/2 and `c2KeepDeep` counts 0/1/2 are all
        // exercised.
        let cap = match i % 4 {
            // deep branch: segment straddles the origin
            0 | 1 => {
                let ang = rng.f(std::f32::consts::PI);
                let d = v(ang.cos(), ang.sin());
                let t = 0.5 + rng.fpos(6.0);
                let s = 0.5 + rng.fpos(6.0);
                c2Capsule {
                    a: v(-d.x * t, -d.y * t),
                    b: v(d.x * s, d.y * s),
                    r: rng.fpos(6.0),
                }
            }
            // shallow branch: 1e-6 <= d < A.r
            2 => {
                let mut c = rand_capsule(&mut rng, 8.0, 6.0);
                c.r = 4.0 + rng.fpos(6.0);
                c
            }
            // grid-quantized, mostly the "no contact" path
            _ => c2Capsule {
                a: v(rng.grid(0.5, 12), rng.grid(0.5, 12)),
                b: v(rng.grid(0.5, 12), rng.grid(0.5, 12)),
                r: rng.grid(0.5, 8).abs(),
            },
        };
        // rows 50..52: transform variants
        let bx = match i % 4 {
            0 => None,
            1 => Some(c2x {
                p: v(0.0, 0.0),
                r: c2r { c: 1.0, s: 0.0 },
            }),
            2 => Some(c2x {
                p: rng.vec(3.0),
                r: c2r { c: 1.0, s: 0.0 },
            }),
            _ => Some(rng.xform(3.0)),
        };

        let mut cm = seeded_manifold(-4242.0);
        let mut rm = cm;
        let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
        unsafe {
            scrub_stack();
            cf(cap, &poly, bxp, &mut cm);
            scrub_stack();
            rf(cap, &poly, bxp, &mut rm);
        }
        if (0..3).contains(&cm.count) {
            counts[cm.count as usize] += 1;
        }
        assert!(
            meq(&cm, &rm),
            "c2CapsuletoPolyManifold {i} n={n} bx={}: cap={cap:?}\n  poly verts={:?}\n  C {}\n  R {}",
            bx.is_some(),
            &poly.verts[..n as usize],
            ms(&cm),
            ms(&rm)
        );
    }
    assert!(
        counts[0] > 100 && counts[1] > 100 && counts[2] > 100,
        "manifold count coverage: {counts:?}"
    );
}

/// Row 55 — fully randomized capsule × randomized (possibly non-convex /
/// degenerate) poly × randomized transform, plus degenerate `count`.
#[test]
fn row55_capsule_to_poly_random_sweep() {
    let l = libs();
    let (cf, rf) = l.pair::<FnCapsulePoly>("c2CapsuletoPolyManifold");
    let (c_norms, _) = l.pair::<FnNorms>("c2Norms");
    let mut rng = Rng::new(0xE4F5_0617_2839_4A5B);

    for i in 0..20_000usize {
        let mut poly = c2Poly::default();
        poly.count = match i % 10 {
            0 => 0, // ERRORS.md row 44
            1 => 1,
            2 => 2,
            k => (1 + k) as c_int,
        };
        for k in 0..8 {
            poly.verts[k] = rng.vec(6.0);
        }
        unsafe {
            c_norms(poly.verts.as_mut_ptr(), poly.norms.as_mut_ptr(), poly.count);
        }
        let cap = if i % 11 == 0 {
            // degenerate point capsule -> c2Norm(0,0) -> NaN (ERRORS.md row 45)
            let p = rng.vec(5.0);
            c2Capsule { a: p, b: p, r: rng.fpos(3.0) }
        } else {
            rand_capsule(&mut rng, 5.0, 3.0)
        };
        let bx = if i % 3 == 0 { None } else { Some(rng.xform(4.0)) };
        let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);

        let mut cm = seeded_manifold(-4242.0);
        let mut rm = cm;
        unsafe {
            scrub_stack();
            cf(cap, &poly, bxp, &mut cm);
            scrub_stack();
            rf(cap, &poly, bxp, &mut rm);
        }
        assert!(
            meq(&cm, &rm),
            "c2CapsuletoPolyManifold random {i} count={}: cap={cap:?}\n  C {}\n  R {}",
            poly.count,
            ms(&cm),
            ms(&rm)
        );
    }
}
