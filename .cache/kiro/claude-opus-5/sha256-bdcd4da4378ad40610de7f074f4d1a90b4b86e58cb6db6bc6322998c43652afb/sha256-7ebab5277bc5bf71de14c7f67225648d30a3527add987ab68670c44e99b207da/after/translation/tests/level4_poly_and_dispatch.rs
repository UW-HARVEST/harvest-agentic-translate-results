//! Level 4/5: `c2RaytoPoly`, the `c2CastRay` dispatcher, and the public
//! `poly_ray` entry point.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_int, c_void};

fn poison() -> c2Raycast {
    c2Raycast {
        t: f32::from_bits(0xDEAD_BEEF),
        n: c2v {
            x: f32::from_bits(0xCAFE_BABE),
            y: f32::from_bits(0xFEED_FACE),
        },
    }
}

#[track_caller]
fn cmp(name: &str, ctx: &str, cr: (c_int, c2Raycast), rr: (c_int, c2Raycast)) {
    assert_bits(&format!("{name} ret"), ctx, &cr.0, &rr.0);
    assert_bits(&format!("{name} out"), ctx, &cr.1, &rr.1);
}

// ---------------------------------------------------------------------------
// Polygon construction helpers
// ---------------------------------------------------------------------------

/// Axis-aligned box as a 4-gon with the same winding/normal order the C demo
/// in `poly_ray` uses.
fn box_poly(hw: f32, hh: f32) -> c2Poly {
    let mut p = c2Poly::default();
    p.verts[0] = c2v { x: hw, y: -hh };
    p.verts[1] = c2v { x: hw, y: hh };
    p.verts[2] = c2v { x: -hw, y: hh };
    p.verts[3] = c2v { x: -hw, y: -hh };
    p.norms[0] = c2v { x: 1.0, y: 0.0 };
    p.norms[1] = c2v { x: 0.0, y: 1.0 };
    p.norms[2] = c2v { x: -1.0, y: 0.0 };
    p.norms[3] = c2v { x: 0.0, y: -1.0 };
    p.count = 4;
    p
}

/// Regular n-gon of radius `rad` with outward normals; n in 1..=8.
fn ngon(n: i32, rad: f32) -> c2Poly {
    let mut p = c2Poly::default();
    p.count = n;
    for i in 0..n {
        let a = (i as f32) * std::f32::consts::TAU / (n as f32);
        p.verts[i as usize] = c2v {
            x: rad * a.cos(),
            y: rad * a.sin(),
        };
        // Face normal for the edge starting at vert i.
        let mid = a + std::f32::consts::PI / (n as f32);
        p.norms[i as usize] = c2v {
            x: mid.cos(),
            y: mid.sin(),
        };
    }
    p
}

/// Fill the unused slots with garbage so that any accidental read past `count`
/// is caught (the C code must not read them either -- if it does, both sides
/// see the same bytes and the comparison still holds).
fn fill_tail(p: &mut c2Poly, rng: &mut Rng) {
    let start = p.count.clamp(0, 8) as usize;
    for i in start..8 {
        p.verts[i] = rng.vec_range(1e3);
        p.norms[i] = rng.vec_range(1e3);
    }
}

// ---------------------------------------------------------------------------
// c2RaytoPoly
// ---------------------------------------------------------------------------

fn run_poly(
    f: FnRayPoly_i,
    A: c2Ray,
    B: &c2Poly,
    bx: Option<&c2x>,
) -> (c_int, c2Raycast) {
    let mut out = poison();
    let bxp = match bx {
        Some(x) => x as *const c2x,
        None => std::ptr::null(),
    };
    let ret = unsafe { f(A, B as *const c2Poly, bxp, &mut out) };
    (ret, out)
}

#[test]
fn c2RaytoPoly_matches_handpicked() {
    let (c, r) = libs().sym::<FnRayPoly_i>("c2RaytoPoly");
    let b = box_poly(1.0, 1.0);

    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    let shifted = c2x {
        p: c2v { x: 3.0, y: -2.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    let rotated = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r {
            c: std::f32::consts::FRAC_1_SQRT_2,
            s: std::f32::consts::FRAC_1_SQRT_2,
        },
    };

    struct Case<'a>(c2Ray, &'a c2Poly, Option<&'a c2x>, &'static str);
    let hit_x = c2Ray {
        p: c2v { x: -5.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let miss = c2Ray {
        p: c2v { x: -5.0, y: 5.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let inside = c2Ray {
        p: c2v { x: 0.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let short = c2Ray {
        p: c2v { x: -5.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 1.0,
    };
    let zero_dir = c2Ray {
        p: c2v { x: -5.0, y: 0.0 },
        d: c2v { x: 0.0, y: 0.0 },
        t: 10.0,
    };
    let corner = c2Ray {
        p: c2v { x: -3.0, y: -3.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let grazing = c2Ray {
        p: c2v { x: -5.0, y: 1.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let nan_ray = c2Ray {
        p: c2v {
            x: f32::NAN,
            y: 0.0,
        },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    let inf_ray = c2Ray {
        p: c2v {
            x: f32::NEG_INFINITY,
            y: 0.0,
        },
        d: c2v { x: 1.0, y: 0.0 },
        t: f32::INFINITY,
    };

    let empty = c2Poly::default(); // count == 0 -> loop body never runs
    let one = ngon(1, 1.0);
    let tri = ngon(3, 1.0);
    let oct = ngon(8, 2.0);

    let cases = vec![
        Case(hit_x, &b, None, "hit, null bx"),
        Case(hit_x, &b, Some(&ident), "hit, identity bx"),
        Case(hit_x, &b, Some(&shifted), "hit, translated bx"),
        Case(hit_x, &b, Some(&rotated), "hit, rotated bx"),
        Case(miss, &b, None, "miss"),
        Case(inside, &b, None, "origin inside -> index stays ~0"),
        Case(short, &b, None, "too short (hi < lo)"),
        Case(zero_dir, &b, None, "zero direction -> den == 0"),
        Case(corner, &b, None, "corner-ish"),
        Case(grazing, &b, None, "grazing along a face"),
        Case(nan_ray, &b, None, "NaN origin"),
        Case(inf_ray, &b, None, "infinite origin and t"),
        Case(hit_x, &empty, None, "count == 0"),
        Case(hit_x, &one, None, "count == 1"),
        Case(hit_x, &tri, None, "triangle"),
        Case(hit_x, &oct, None, "octagon"),
        Case(inf_ray, &oct, Some(&rotated), "octagon, rotated, infinite"),
    ];

    for Case(A, poly, bx, label) in cases {
        cmp(
            "c2RaytoPoly",
            label,
            run_poly(c, A, poly, bx),
            run_poly(r, A, poly, bx),
        );
    }
}

#[test]
fn c2RaytoPoly_matches_random() {
    let (c, r) = libs().sym::<FnRayPoly_i>("c2RaytoPoly");
    let mut rng = Rng::new(0x7777);

    // Coarse inputs: hit the exact `den == 0`, `num < lo * den`, `hi < lo`
    // and `index != ~0` boundaries.
    for _ in 0..40_000 {
        let n = 1 + (rng.next_u32() % 8) as i32;
        let mut p = c2Poly::default();
        p.count = n;
        for i in 0..n as usize {
            p.verts[i] = rng.vec_coarse();
            p.norms[i] = rng.vec_coarse();
        }
        fill_tail(&mut p, &mut rng);
        let A = c2Ray {
            p: rng.vec_coarse(),
            d: rng.vec_coarse(),
            t: rng.f32_coarse(),
        };
        let bx = c2x {
            p: rng.vec_coarse(),
            r: c2r {
                c: rng.f32_coarse(),
                s: rng.f32_coarse(),
            },
        };
        let use_bx = rng.next_u32() % 3 != 0;
        let bxr = if use_bx { Some(&bx) } else { None };
        cmp(
            "c2RaytoPoly",
            "coarse random",
            run_poly(c, A, &p, bxr),
            run_poly(r, A, &p, bxr),
        );
    }

    // Well-formed convex polygons with normalised rotations.
    for _ in 0..30_000 {
        let n = 3 + (rng.next_u32() % 6) as i32;
        let mut p = ngon(n, 0.25 + rng.f32_range(4.0).abs());
        fill_tail(&mut p, &mut rng);
        let ang = rng.f32_range(4.0);
        let A = c2Ray {
            p: rng.vec_range(12.0),
            d: c2v {
                x: ang.cos(),
                y: ang.sin(),
            },
            t: rng.f32_range(25.0),
        };
        let bang = rng.f32_range(4.0);
        let bx = c2x {
            p: rng.vec_range(6.0),
            r: c2r {
                c: bang.cos(),
                s: bang.sin(),
            },
        };
        let use_bx = rng.next_u32() % 2 == 0;
        let bxr = if use_bx { Some(&bx) } else { None };
        cmp(
            "c2RaytoPoly",
            "convex random",
            run_poly(c, A, &p, bxr),
            run_poly(r, A, &p, bxr),
        );
    }

    // Out-of-range / negative `count`: the C loop is `i < B->count`, so
    // negative counts simply skip the body. Verified rather than assumed.
    for &n in &[-1_i32, -8, 0] {
        let mut p = c2Poly::default();
        p.count = n;
        fill_tail(&mut p, &mut rng);
        let A = c2Ray {
            p: c2v { x: -5.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 10.0,
        };
        cmp(
            "c2RaytoPoly",
            &format!("count = {n}"),
            run_poly(c, A, &p, None),
            run_poly(r, A, &p, None),
        );
    }
}

// ---------------------------------------------------------------------------
// c2CastRay dispatcher
// ---------------------------------------------------------------------------

fn run_cast(
    f: FnCastRay_i,
    A: c2Ray,
    shape: *const c_void,
    bx: *const c2x,
    ty: c_int,
) -> (c_int, c2Raycast) {
    let mut out = poison();
    let ret = unsafe { f(A, shape, bx, ty, &mut out) };
    (ret, out)
}

#[test]
fn c2CastRay_matches() {
    let (c, r) = libs().sym::<FnCastRay_i>("c2CastRay");
    let mut rng = Rng::new(0x8888);

    for _ in 0..20_000 {
        let ang = rng.f32_range(4.0);
        let A = c2Ray {
            p: rng.vec_range(12.0),
            d: c2v {
                x: ang.cos(),
                y: ang.sin(),
            },
            t: rng.f32_range(25.0),
        };
        let A_coarse = c2Ray {
            p: rng.vec_coarse(),
            d: rng.vec_coarse(),
            t: rng.f32_coarse(),
        };

        let circle = c2Circle {
            p: rng.vec_coarse(),
            r: rng.f32_coarse(),
        };
        let aabb = c2AABB {
            min: rng.vec_coarse(),
            max: rng.vec_coarse(),
        };
        let capsule = c2Capsule {
            a: rng.vec_coarse(),
            b: rng.vec_coarse(),
            r: rng.f32_coarse(),
        };
        let n = 1 + (rng.next_u32() % 8) as i32;
        let mut poly = ngon(n, 1.0 + rng.f32_range(3.0).abs());
        fill_tail(&mut poly, &mut rng);
        let bang = rng.f32_range(4.0);
        let bx = c2x {
            p: rng.vec_range(5.0),
            r: c2r {
                c: bang.cos(),
                s: bang.sin(),
            },
        };

        for ray in [A, A_coarse] {
            // The C switch ignores `bx` for every type except POLY; pass a
            // non-null pointer everywhere to confirm that.
            let bxp = &bx as *const c2x;
            cmp(
                "c2CastRay/CIRCLE",
                "random",
                run_cast(c, ray, &circle as *const _ as *const c_void, bxp, C2_TYPE_CIRCLE),
                run_cast(r, ray, &circle as *const _ as *const c_void, bxp, C2_TYPE_CIRCLE),
            );
            cmp(
                "c2CastRay/AABB",
                "random",
                run_cast(c, ray, &aabb as *const _ as *const c_void, bxp, C2_TYPE_AABB),
                run_cast(r, ray, &aabb as *const _ as *const c_void, bxp, C2_TYPE_AABB),
            );
            cmp(
                "c2CastRay/CAPSULE",
                "random",
                run_cast(c, ray, &capsule as *const _ as *const c_void, bxp, C2_TYPE_CAPSULE),
                run_cast(r, ray, &capsule as *const _ as *const c_void, bxp, C2_TYPE_CAPSULE),
            );
            for bp in [bxp, std::ptr::null()] {
                cmp(
                    "c2CastRay/POLY",
                    "random",
                    run_cast(c, ray, &poly as *const _ as *const c_void, bp, C2_TYPE_POLY),
                    run_cast(r, ray, &poly as *const _ as *const c_void, bp, C2_TYPE_POLY),
                );
            }
        }
    }
}

#[test]
fn c2CastRay_unknown_type_matches() {
    let (c, r) = libs().sym::<FnCastRay_i>("c2CastRay");
    let circle = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 1.0,
    };
    let A = c2Ray {
        p: c2v { x: -5.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 10.0,
    };
    // Values outside the enum fall through the switch to `return 0` and must
    // leave `out` untouched.
    for ty in [-1_i32, 4, 5, 99, i32::MIN, i32::MAX] {
        cmp(
            "c2CastRay/unknown",
            &format!("typeB = {ty}"),
            run_cast(
                c,
                A,
                &circle as *const _ as *const c_void,
                std::ptr::null(),
                ty,
            ),
            run_cast(
                r,
                A,
                &circle as *const _ as *const c_void,
                std::ptr::null(),
                ty,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// poly_ray -- the public API in include/lib.h
// ---------------------------------------------------------------------------

#[test]
fn poly_ray_matches() {
    let (c, r) = libs().sym::<FnPolyRay_i>("poly_ray");
    for _ in 0..1000 {
        let (mut c1, mut c2) = (poison(), poison());
        let (mut r1, mut r2) = (poison(), poison());
        let cret = unsafe { c(&mut c1, &mut c2) };
        let rret = unsafe { r(&mut r1, &mut r2) };
        assert_bits("poly_ray ret", "()", &cret, &rret);
        assert_bits("poly_ray cast1", "()", &c1, &r1);
        assert_bits("poly_ray cast2", "()", &c2, &r2);
    }
}
