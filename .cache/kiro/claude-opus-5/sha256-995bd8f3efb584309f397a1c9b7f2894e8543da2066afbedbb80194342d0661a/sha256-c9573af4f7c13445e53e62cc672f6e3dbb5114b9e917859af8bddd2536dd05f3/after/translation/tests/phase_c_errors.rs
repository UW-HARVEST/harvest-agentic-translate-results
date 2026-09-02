//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact rejecting condition, calls BOTH libraries
//! through their `.so` exports, and asserts the rejection is byte-identical
//! (same sentinel / same untouched output bytes), not merely "both failed".
//!
//! The output `c2Manifold` is always pre-poisoned with a recognisable pattern,
//! because most of this library's rejections are *early returns that leave part
//! of `*m` unwritten* — comparing against a zeroed struct would hide that.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Harness self-test: prove `same()` actually detects a divergence, so that a
// green Phase C cannot be an artefact of a no-op comparison.
// ---------------------------------------------------------------------------

#[test]
fn phase_c_harness_detects_divergence() {
    let p = pair();
    let (cNeg, _) = p.get::<FnVV>(b"c2Neg");
    let (_, rSkew) = p.get::<FnVV>(b"c2Skew");
    let a = c2v { x: 1.0, y: 2.0 };
    let (x, y) = unsafe { (cNeg(a), rSkew(a)) };
    let caught = std::panic::catch_unwind(|| same("deliberate mismatch", &x, &y)).is_err();
    assert!(caught, "same() failed to report a real difference");
    // ...and it must NOT fire on equal values.
    same("deliberate match", &x, &x);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A `c2Poly` with explicitly-controlled bytes on both sides, so the C's
/// out-of-range `verts[-1]` / `norms[-1]` / `verts[8]` reads land on memory we
/// own and both libraries observe the same values (ERRORS.md #15..#17, #19, #65).
#[repr(C)]
struct PaddedPoly {
    lead: [u32; 4],
    poly: c2Poly,
    trail: [u32; 8],
}

impl PaddedPoly {
    fn new(poly: c2Poly) -> Self {
        PaddedPoly {
            lead: [0xAAAA_AAAA; 4],
            poly,
            trail: [0xBBBB_BBBB; 8],
        }
    }
    fn ptr(&self) -> *const c2Poly {
        &self.poly
    }
}

fn convex(rng: &mut Rng, norms: &FnNorms, count: c_int) -> c2Poly {
    let mut p = c2Poly::default();
    p.count = count;
    let n = count.clamp(0, 8) as usize;
    let mut angs: Vec<f32> = (0..n.max(1))
        .map(|_| rng.unit() * std::f32::consts::TAU)
        .collect();
    angs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for k in 0..n {
        let r = 1.0 + rng.unit() * 2.0;
        p.verts[k] = c2v {
            x: r * angs[k].cos(),
            y: r * angs[k].sin(),
        };
    }
    unsafe { norms(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), n.max(1) as c_int) };
    p
}

/// Run `c2CapsuletoPolyManifold` on both and compare.
fn diff_cap_poly(p: &Pair, A: c2Capsule, poly: &PaddedPoly, bx: *const c2x, label: &str) {
    let (cf, rf) = p.get::<FnCapPoly>(b"c2CapsuletoPolyManifold");
    let mut cm = poison_manifold(0x5A);
    let mut rm = cm;
    scrub_stack();
    unsafe { cf(A, poly.ptr(), bx, &mut cm) };
    scrub_stack();
    unsafe { rf(A, poly.ptr(), bx, &mut rm) };
    same(label, &cm, &rm);
}

// ===========================================================================
// Rows 1..6, 9..12 — the private clipping helpers' rejection paths, reached
// through c2CapsuletoPolyManifold.
// ===========================================================================

#[test]
fn rows01_12_clip_and_sideplane_rejections() {
    let p = pair();
    let (cNorms, _) = p.get::<FnNorms>(b"c2Norms");
    let mut rng = Rng::new(0xC001);

    // Row 5: degenerate reference edge (`ra == rb`) => c2Norm(0) => NaN planes.
    {
        let mut pl = c2Poly::default();
        pl.count = 4;
        // two identical consecutive verts => the reference edge collapses
        pl.verts[0] = c2v { x: 0.0, y: 0.0 };
        pl.verts[1] = c2v { x: 0.0, y: 0.0 };
        pl.verts[2] = c2v { x: 1.0, y: 0.0 };
        pl.verts[3] = c2v { x: 1.0, y: 1.0 };
        unsafe { cNorms(pl.verts.as_mut_ptr(), pl.norms.as_mut_ptr(), 4) };
        let padded = PaddedPoly::new(pl);
        for k in 0..200 {
            let A = c2Capsule {
                a: c2v { x: rng.sym(2.0), y: rng.sym(2.0) },
                b: c2v { x: rng.sym(2.0), y: rng.sym(2.0) },
                r: rng.unit() * 2.0,
            };
            diff_cap_poly(&p, A, &padded, std::ptr::null(), &format!("row5 k={k}"));
        }
    }

    // Rows 1..4, 9..12: sweep many capsule placements against convex polys of
    // every vertex count so that both side-plane rejections and every `code`
    // branch's early return are hit.
    for count in 0..=8 {
        let padded = PaddedPoly::new(convex(&mut rng, &cNorms, count));
        for k in 0..600 {
            // Deliberately include placements that are almost tangential, which
            // is what makes the clip produce fewer than two points.
            let scale = match k % 4 {
                0 => 1.0,
                1 => 3.0,
                2 => 0.05,
                _ => 30.0,
            };
            let a = c2v {
                x: rng.sym(scale),
                y: rng.sym(scale),
            };
            let A = c2Capsule {
                a,
                b: match k % 5 {
                    0 => a,
                    1 => c2v { x: a.x, y: a.y + scale },
                    2 => c2v { x: a.x + scale, y: a.y },
                    _ => c2v {
                        x: rng.sym(scale),
                        y: rng.sym(scale),
                    },
                },
                r: match k % 4 {
                    0 => 0.0,
                    1 => rng.unit() * 3.0,
                    2 => rng.grid(0.5, 4),
                    _ => 1.0e-7, // just under the 1e-6 GJK threshold
                },
            };
            diff_cap_poly(
                &p,
                A,
                &padded,
                std::ptr::null(),
                &format!("rows1-12 count={count} k={k}"),
            );
        }
    }

    // Row 2 specific: endpoint exactly ON the plane (`d1 == 0`) — grid-snapped
    // geometry so the distance lands on exact zero.
    {
        let mut pl = c2Poly::default();
        pl.count = 4;
        pl.verts[0] = c2v { x: -1.0, y: -1.0 };
        pl.verts[1] = c2v { x: 1.0, y: -1.0 };
        pl.verts[2] = c2v { x: 1.0, y: 1.0 };
        pl.verts[3] = c2v { x: -1.0, y: 1.0 };
        unsafe { cNorms(pl.verts.as_mut_ptr(), pl.norms.as_mut_ptr(), 4) };
        let padded = PaddedPoly::new(pl);
        for &(ax, ay, bx2, by, r) in &[
            (-1.0f32, 1.0f32, 1.0f32, 1.0f32, 0.5f32), // both endpoints on the edge
            (-1.0, 1.0, 0.0, 1.0, 0.5),
            (1.0, 1.0, 2.0, 1.0, 0.5),
            (-2.0, 1.0, 2.0, 1.0, 0.5),
            (0.0, 1.0, 0.0, 2.0, 0.5),
            (-1.0, -1.0, 1.0, 1.0, 0.0),
            (0.0, 0.0, 0.0, 0.0, 0.0),
        ] {
            let A = c2Capsule {
                a: c2v { x: ax, y: ay },
                b: c2v { x: bx2, y: by },
                r,
            };
            diff_cap_poly(&p, A, &padded, std::ptr::null(), "row2 exact-plane");
        }
    }

    // Row 1 specific: `d0 < 0 && d1 < 0` whose product UNDERFLOWS to +0, which
    // is what pushes a third element into the C's 2-element `out[]`.
    {
        let mut pl = c2Poly::default();
        pl.count = 4;
        let e = 1.0e-24f32;
        pl.verts[0] = c2v { x: -1.0, y: -1.0 };
        pl.verts[1] = c2v { x: 1.0, y: -1.0 };
        pl.verts[2] = c2v { x: 1.0, y: 1.0 };
        pl.verts[3] = c2v { x: -1.0, y: 1.0 };
        unsafe { cNorms(pl.verts.as_mut_ptr(), pl.norms.as_mut_ptr(), 4) };
        let padded = PaddedPoly::new(pl);
        for k in 0..400 {
            let d = e * (1.0 + k as f32);
            let A = c2Capsule {
                a: c2v { x: -1.0 + d, y: 0.0 },
                b: c2v { x: 1.0 - d, y: 0.0 },
                r: 0.5,
            };
            diff_cap_poly(&p, A, &padded, std::ptr::null(), "row1 underflow product");
        }
    }
}

// ===========================================================================
// Rows 7, 8 — c2AABBtoAABBManifold separated on X / on Y (early `return`,
// leaving depths / contact_points / n untouched).
// ===========================================================================

#[test]
fn rows07_08_aabb_separated() {
    let p = pair();
    let (cf, rf) = p.get::<FnAA>(b"c2AABBtoAABBManifold");
    let A = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let cases: [(c2AABB, &str); 6] = [
        (
            c2AABB {
                min: c2v { x: 2.0, y: -1.0 },
                max: c2v { x: 3.0, y: 1.0 },
            },
            "row7 separated +X",
        ),
        (
            c2AABB {
                min: c2v { x: -3.0, y: -1.0 },
                max: c2v { x: -2.0, y: 1.0 },
            },
            "row7 separated -X",
        ),
        (
            c2AABB {
                min: c2v { x: -1.0, y: 2.0 },
                max: c2v { x: 1.0, y: 3.0 },
            },
            "row8 separated +Y",
        ),
        (
            c2AABB {
                min: c2v { x: -1.0, y: -3.0 },
                max: c2v { x: 1.0, y: -2.0 },
            },
            "row8 separated -Y",
        ),
        (
            c2AABB {
                min: c2v { x: 1.0, y: -1.0 },
                max: c2v { x: 2.0, y: 1.0 },
            },
            "boundary dx == 0",
        ),
        (
            c2AABB {
                min: c2v { x: f32::NAN, y: f32::NAN },
                max: c2v { x: f32::NAN, y: f32::NAN },
            },
            "NaN box (dx < 0 is false)",
        ),
    ];
    for (B, label) in cases {
        let mut cm = poison_manifold(7);
        let mut rm = cm;
        unsafe {
            cf(A, B, &mut cm);
            rf(A, B, &mut rm);
        }
        same(label, &cm, &rm);
        // Both must also agree with the documented C behaviour: count == 0 and
        // the rest of `*m` byte-identical to the poison for the separated cases.
        if label.starts_with("row7") || label.starts_with("row8") {
            assert_eq!(cm.count, 0, "{label}: expected rejection");
            let poison = poison_manifold(7);
            assert_eq!(
                raw(&cm)[4..],
                raw(&poison)[4..],
                "{label}: C wrote beyond count"
            );
        }
    }
}

// ===========================================================================
// Rows 13..19 — c2CapsuletoPolyManifold: too-far rejection, degenerate capsule,
// count == 0 / < 0 / > 8, NULL bx, c2Incident's `index == ~0`.
// ===========================================================================

#[test]
fn rows13_19_capsule_poly_edge_counts() {
    let p = pair();
    let (cNorms, _) = p.get::<FnNorms>(b"c2Norms");
    let mut rng = Rng::new(0xC013);

    // Row 13: far apart -> `d >= 1e-6 && d >= A.r`, neither branch taken.
    {
        let padded = PaddedPoly::new(convex(&mut rng, &cNorms, 4));
        let A = c2Capsule {
            a: c2v { x: 1000.0, y: 1000.0 },
            b: c2v { x: 1001.0, y: 1002.0 },
            r: 1.0,
        };
        let mut cm = poison_manifold(13);
        let mut rm = cm;
        let (cf, rf) = p.get::<FnCapPoly>(b"c2CapsuletoPolyManifold");
        scrub_stack();
        unsafe { cf(A, padded.ptr(), std::ptr::null(), &mut cm) };
        scrub_stack();
        unsafe { rf(A, padded.ptr(), std::ptr::null(), &mut rm) };
        same("row13 far", &cm, &rm);
        assert_eq!(cm.count, 0);
    }

    // Rows 15, 16, 17: count == 0, negative, and > 8.
    for count in [0i32, -1, -7, 9, 12, 100] {
        let mut pl = convex(&mut rng, &cNorms, 4);
        pl.count = count;
        // Fill every slot so an over-run read is at least well-defined memory
        // that both libraries see identically.
        for k in 0..8 {
            pl.verts[k] = rng.vec_sym(2.0);
            pl.norms[k] = rng.vec_sym(1.0);
        }
        let padded = PaddedPoly::new(pl);
        for k in 0..200 {
            let A = c2Capsule {
                a: rng.vec_sym(2.0),
                b: rng.vec_sym(2.0),
                r: rng.unit() * 2.0,
            };
            diff_cap_poly(
                &p,
                A,
                &padded,
                std::ptr::null(),
                &format!("rows15-17 count={count} k={k}"),
            );
        }
    }

    // Rows 14, 19: degenerate capsule and NaN normals, which is what drives
    // `c2Incident`'s index to stay `~0`.
    {
        let mut pl = c2Poly::default();
        pl.count = 4;
        for k in 0..4 {
            pl.verts[k] = c2v { x: 0.5, y: -0.5 }; // all identical -> NaN norms
        }
        unsafe { cNorms(pl.verts.as_mut_ptr(), pl.norms.as_mut_ptr(), 4) };
        let padded = PaddedPoly::new(pl);
        for k in 0..300 {
            let a = rng.vec_sym(2.0);
            let A = c2Capsule {
                a,
                b: if k % 2 == 0 { a } else { rng.vec_sym(2.0) },
                r: rng.unit() * 3.0,
            };
            diff_cap_poly(&p, A, &padded, std::ptr::null(), &format!("rows14/19 k={k}"));
        }
    }

    // Row 18: bx_ptr == NULL vs an explicit identity must agree with each other
    // in BOTH libraries (the C substitutes c2xIdentity()).
    {
        let padded = PaddedPoly::new(convex(&mut rng, &cNorms, 5));
        let ident = c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 1.0, s: 0.0 },
        };
        let (cf, rf) = p.get::<FnCapPoly>(b"c2CapsuletoPolyManifold");
        for k in 0..300 {
            let A = c2Capsule {
                a: rng.vec_sym(3.0),
                b: rng.vec_sym(3.0),
                r: rng.unit() * 2.0,
            };
            let mut m_null_c = poison_manifold(18);
            let mut m_id_c = m_null_c;
            let mut m_null_r = m_null_c;
            let mut m_id_r = m_null_c;
            scrub_stack();
            unsafe { cf(A, padded.ptr(), std::ptr::null(), &mut m_null_c) };
            scrub_stack();
            unsafe { cf(A, padded.ptr(), &ident, &mut m_id_c) };
            scrub_stack();
            unsafe { rf(A, padded.ptr(), std::ptr::null(), &mut m_null_r) };
            scrub_stack();
            unsafe { rf(A, padded.ptr(), &ident, &mut m_id_r) };
            same(&format!("row18 NULL bx k={k}"), &m_null_c, &m_null_r);
            same(&format!("row18 identity bx k={k}"), &m_id_c, &m_id_r);
            same(&format!("row18 NULL==identity (C) k={k}"), &m_null_c, &m_id_c);
        }
    }
}

// ===========================================================================
// Rows 20..23 — c2CircletoCircleManifold rejections.
// ===========================================================================

#[test]
fn rows20_23_circle_circle_rejections() {
    let p = pair();
    let (cf, rf) = p.get::<FnCC>(b"c2CircletoCircleManifold");
    let cases: [(c2Circle, c2Circle, &str); 8] = [
        // Row 20: disjoint
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2Circle { p: c2v { x: 10.0, y: 0.0 }, r: 1.0 },
            "row20 disjoint",
        ),
        // exactly touching (d2 == r*r -> NOT less than -> rejected)
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2Circle { p: c2v { x: 2.0, y: 0.0 }, r: 1.0 },
            "row20 exactly touching",
        ),
        // Row 21: coincident centres, positive radii -> the l == 0 fallback
        (
            c2Circle { p: c2v { x: 3.0, y: -4.0 }, r: 1.0 },
            c2Circle { p: c2v { x: 3.0, y: -4.0 }, r: 2.0 },
            "row21 coincident",
        ),
        // Row 22: both radii zero
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
            "row22 zero radii",
        ),
        // Row 23: negative radii summing negative, but r*r positive
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: -1.0 },
            c2Circle { p: c2v { x: 0.5, y: 0.0 }, r: -1.0 },
            "row23 negative radii",
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: -3.0 },
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            "row23 mixed signs coincident",
        ),
        // NaN radius / position
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: f32::NAN },
            c2Circle { p: c2v { x: 1.0, y: 0.0 }, r: 1.0 },
            "NaN radius",
        ),
        (
            c2Circle { p: c2v { x: f32::INFINITY, y: 0.0 }, r: 1.0 },
            c2Circle { p: c2v { x: f32::INFINITY, y: 0.0 }, r: 1.0 },
            "inf positions",
        ),
    ];
    for (A, B, label) in cases {
        let mut cm = poison_manifold(20);
        let mut rm = cm;
        unsafe {
            cf(A, B, &mut cm);
            rf(A, B, &mut rm);
        }
        same(label, &cm, &rm);
    }
}

// ===========================================================================
// Rows 24..28 — c2CircletoAABBManifold rejections and the deep branch.
// ===========================================================================

#[test]
fn rows24_28_circle_aabb_rejections() {
    let p = pair();
    let (cf, rf) = p.get::<FnCA>(b"c2CircletoAABBManifold");
    let unit = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let cases: [(c2Circle, c2AABB, &str); 9] = [
        (
            c2Circle { p: c2v { x: 10.0, y: 0.0 }, r: 1.0 },
            unit,
            "row24 no overlap",
        ),
        (
            c2Circle { p: c2v { x: 2.0, y: 0.0 }, r: 1.0 },
            unit,
            "row24 exactly touching",
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.5 },
            unit,
            "row25 centre inside (d2 == 0)",
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.5 },
            c2AABB {
                min: c2v { x: -2.0, y: -2.0 },
                max: c2v { x: 2.0, y: 2.0 },
            },
            "row26 x_overlap == y_overlap tie",
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2AABB {
                min: c2v { x: 1.0, y: 1.0 },
                max: c2v { x: -1.0, y: -1.0 },
            },
            "row27 inverted AABB",
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: 0.0, y: 0.0 },
            },
            "row27 degenerate AABB",
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.5 }, r: -1.0 },
            unit,
            "row28 negative radius",
        ),
        (
            c2Circle { p: c2v { x: f32::NAN, y: 0.0 }, r: 1.0 },
            unit,
            "NaN centre",
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: f32::INFINITY },
            unit,
            "inf radius",
        ),
    ];
    for (A, B, label) in cases {
        let mut cm = poison_manifold(24);
        let mut rm = cm;
        unsafe {
            cf(A, B, &mut cm);
            rf(A, B, &mut rm);
        }
        same(label, &cm, &rm);
    }
}

// ===========================================================================
// Rows 29..34 — capsule rejections and NaN normals; row 33's unconditional
// `m->n` negation on the no-manifold path.
// ===========================================================================

#[test]
fn rows29_34_capsule_rejections() {
    let p = pair();
    let (cCCap, rCCap) = p.get::<FnCCap>(b"c2CircletoCapsuleManifold");
    let (cCapCap, rCapCap) = p.get::<FnCapCap>(b"c2CapsuletoCapsuleManifold");
    let (cACap, rACap) = p.get::<FnACap>(b"c2AABBtoCapsuleManifold");

    // Rows 29, 30
    let circ_cases: [(c2Circle, c2Capsule, &str); 5] = [
        (
            c2Circle { p: c2v { x: 100.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 1.0 },
            "row29 far apart",
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            "row30 d == 0 with degenerate capsule (NaN normal)",
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
            "row29 all-zero radii, coincident",
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 2.0 },
            c2Capsule { a: c2v { x: 2.0, y: 0.0 }, b: c2v { x: 3.0, y: 0.0 }, r: 0.0 },
            "row29 exactly touching",
        ),
        (
            c2Circle { p: c2v { x: 0.5, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 1.0 },
            "d == 0 with a real axis",
        ),
    ];
    for (A, B, label) in circ_cases {
        let mut cm = poison_manifold(29);
        let mut rm = cm;
        scrub_stack();
        unsafe { cCCap(A, B, &mut cm) };
        scrub_stack();
        unsafe { rCCap(A, B, &mut rm) };
        same(label, &cm, &rm);
    }

    // Rows 31, 32
    let cap_cases: [(c2Capsule, c2Capsule, &str); 4] = [
        (
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 50.0, y: 0.0 }, b: c2v { x: 51.0, y: 0.0 }, r: 1.0 },
            "row32 far apart",
        ),
        (
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 1.0 },
            "row31 A degenerate, d == 0 -> NaN normal",
        ),
        (
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
            "row32 both degenerate, zero radii",
        ),
        (
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 2.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 0.0, y: 2.0 }, b: c2v { x: 2.0, y: 2.0 }, r: 1.0 },
            "exactly touching parallel",
        ),
    ];
    for (A, B, label) in cap_cases {
        let mut cm = poison_manifold(31);
        let mut rm = cm;
        scrub_stack();
        unsafe { cCapCap(A, B, &mut cm) };
        scrub_stack();
        unsafe { rCapCap(A, B, &mut rm) };
        same(label, &cm, &rm);
    }

    // Rows 33, 34
    let ac_cases: [(c2AABB, c2Capsule, &str); 4] = [
        (
            c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } },
            c2Capsule { a: c2v { x: 100.0, y: 100.0 }, b: c2v { x: 101.0, y: 102.0 }, r: 1.0 },
            "row33 far apart (n still negated)",
        ),
        (
            c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } },
            c2Capsule { a: c2v { x: 1.0, y: 1.0 }, b: c2v { x: 2.0, y: 2.0 }, r: 1.0 },
            "row34 degenerate AABB -> NaN norms",
        ),
        (
            c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: -1.0, y: -1.0 } },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.5, y: 0.5 }, r: 0.5 },
            "inverted AABB",
        ),
        (
            c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
            "degenerate capsule inside",
        ),
    ];
    for (A, B, label) in ac_cases {
        let mut cm = poison_manifold(33);
        let mut rm = cm;
        scrub_stack();
        unsafe { cACap(A, B, &mut cm) };
        scrub_stack();
        unsafe { rACap(A, B, &mut rm) };
        same(label, &cm, &rm);
    }
}

// ===========================================================================
// Rows 35, 36 — c2Norms with count <= 0 and duplicate consecutive vertices.
// ===========================================================================

#[test]
fn rows35_36_norms_edge_cases() {
    let p = pair();
    let (cf, rf) = p.get::<FnNorms>(b"c2Norms");
    let mut rng = Rng::new(0xC035);
    for count in [0i32, -1, -8, 1, 2, 8] {
        for k in 0..300 {
            let mut verts = [c2v::default(); 8];
            for j in 0..8 {
                verts[j] = rng.vec_sym(3.0);
            }
            // Row 36: duplicate a consecutive pair.
            if k % 3 == 0 {
                verts[1] = verts[0];
            }
            if k % 5 == 0 {
                for j in 0..8 {
                    verts[j] = c2v { x: 1.0, y: 2.0 };
                }
            }
            // Poison the output so an untouched slot is detected (row 35).
            let mut cn = [c2v { x: 42.5, y: -17.25 }; 8];
            let mut rn = cn;
            let mut cv = verts;
            let mut rv = verts;
            unsafe {
                cf(cv.as_mut_ptr(), cn.as_mut_ptr(), count);
                rf(rv.as_mut_ptr(), rn.as_mut_ptr(), count);
            }
            same(&format!("row35/36 count={count} k={k} norms"), &cn, &rn);
            same(&format!("row35/36 count={count} k={k} verts"), &cv, &rv);
            if count <= 0 {
                assert_eq!(
                    raw(&cn),
                    raw(&[c2v { x: 42.5, y: -17.25 }; 8]),
                    "count <= 0 must write nothing"
                );
            }
        }
    }
}

// ===========================================================================
// Rows 37, 38 — c2MakeProxy with C2_TYPE_POLY and out-of-range enum values.
// ===========================================================================

#[test]
fn rows37_38_make_proxy_unhandled_types() {
    let p = pair();
    let (cf, rf) = p.get::<FnMakeProxy>(b"c2MakeProxy");
    let mut rng = Rng::new(0xC037);
    let mut tys: Vec<c_int> = vec![C2_TYPE_POLY];
    tys.extend_from_slice(&BAD_TYPES);
    for k in 0..2000 {
        let shape = c2Circle {
            p: rng.vec_sym(10.0),
            r: rng.sym(3.0),
        };
        for &ty in &tys {
            // Fully-poisoned proxy: any write at all shows up.
            let mut orig = c2Proxy::default();
            let q = &mut orig as *mut c2Proxy as *mut u8;
            unsafe {
                for i in 0..std::mem::size_of::<c2Proxy>() {
                    *q.add(i) = (k as u8).wrapping_mul(13).wrapping_add(i as u8) | 1;
                }
            }
            let mut cp = orig;
            let mut rp = orig;
            unsafe {
                cf(&shape as *const _ as *const c_void, ty, &mut cp);
                rf(&shape as *const _ as *const c_void, ty, &mut rp);
            }
            same(&format!("row37/38 type={ty}"), &cp, &rp);
            assert_eq!(raw(&cp), raw(&orig), "type={ty} must not write *p");
        }
    }
}

// ===========================================================================
// Rows 39..50 — c2GJK's optional-pointer handling, cache validity test,
// iteration cap, loop-exit conditions and the use_radius fallbacks.
// ===========================================================================

#[test]
fn rows39_50_gjk_error_and_boundary_paths() {
    let p = pair();
    let (cf, rf) = p.get::<FnGJK>(b"c2GJK");
    let mut rng = Rng::new(0xC039);

    let mk_circle = |r: &mut Rng, c: c2v| c2Circle { p: c, r: r.unit() * 2.0 };

    for k in 0..4000 {
        let c0 = rng.vec_sym(3.0);
        let A = mk_circle(&mut rng, c0);
        let far = k % 3 == 0;
        let B = c2Capsule {
            a: if far {
                c2v { x: 1e6, y: 1e6 }
            } else {
                rng.vec_sym(3.0)
            },
            b: if far {
                c2v { x: 1e6 + 1.0, y: 1e6 }
            } else {
                rng.vec_sym(3.0)
            },
            r: rng.unit() * 2.0,
        };
        let ap = &A as *const _ as *const c_void;
        let bp = &B as *const _ as *const c_void;

        // Rows 39, 40: every combination of NULL optional pointers.
        for mask in 0..32u32 {
            let ax = rng.xform(3.0, true);
            let bx = rng.xform(3.0, true);
            let axp = if mask & 1 != 0 {
                &ax as *const c2x
            } else {
                std::ptr::null()
            };
            let bxp = if mask & 2 != 0 {
                &bx as *const c2x
            } else {
                std::ptr::null()
            };
            let want_a = mask & 4 != 0;
            let want_b = mask & 8 != 0;
            let want_it = mask & 16 != 0;
            let poison = c2v { x: 777.5, y: -888.25 };
            let (mut ca, mut cb, mut ra, mut rb) = (poison, poison, poison, poison);
            let (mut cit, mut rit) = (-999i32, -999i32);
            let use_radius = (k % 2) as c_int;
            let cd = unsafe {
                scrub_stack();
                cf(
                    ap,
                    C2_TYPE_CIRCLE,
                    axp,
                    bp,
                    C2_TYPE_CAPSULE,
                    bxp,
                    if want_a { &mut ca } else { std::ptr::null_mut() },
                    if want_b { &mut cb } else { std::ptr::null_mut() },
                    use_radius,
                    if want_it { &mut cit } else { std::ptr::null_mut() },
                    std::ptr::null_mut(),
                )
            };
            let rd = unsafe {
                scrub_stack();
                rf(
                    ap,
                    C2_TYPE_CIRCLE,
                    axp,
                    bp,
                    C2_TYPE_CAPSULE,
                    bxp,
                    if want_a { &mut ra } else { std::ptr::null_mut() },
                    if want_b { &mut rb } else { std::ptr::null_mut() },
                    use_radius,
                    if want_it { &mut rit } else { std::ptr::null_mut() },
                    std::ptr::null_mut(),
                )
            };
            same(&format!("rows39/40 mask={mask} dist"), &cd, &rd);
            same(&format!("rows39/40 mask={mask} outA"), &ca, &ra);
            same(&format!("rows39/40 mask={mask} outB"), &cb, &rb);
            same(&format!("rows39/40 mask={mask} iters"), &cit, &rit);
            // Row 44: the iteration count is hard-capped at 20.
            if want_it {
                assert!(cit >= 0 && cit <= 20, "iterations out of range: {cit}");
            }
        }
    }
}

#[test]
fn rows41_42_43_gjk_bad_types_and_cache() {
    let p = pair();
    let (cf, rf) = p.get::<FnGJK>(b"c2GJK");
    let mut rng = Rng::new(0xC041);
    let mut tys: Vec<c_int> = ALL_TYPES.to_vec();
    tys.extend_from_slice(&BAD_TYPES);

    for k in 0..1200 {
        let A = c2Circle {
            p: rng.vec_sym(3.0),
            r: rng.unit() * 2.0,
        };
        let B = c2Capsule {
            a: rng.vec_sym(3.0),
            b: rng.vec_sym(3.0),
            r: rng.unit() * 2.0,
        };
        let ap = &A as *const _ as *const c_void;
        let bp = &B as *const _ as *const c_void;

        // Row 41: POLY and out-of-range types (the proxy is never filled).
        for &ta in &tys {
            for &tb in &tys {
                let poison = c2v { x: 5.5, y: -6.25 };
                let (mut ca, mut cb, mut ra, mut rb) = (poison, poison, poison, poison);
                let (mut cit, mut rit) = (-3i32, -3i32);
                let cd = unsafe {
                    scrub_stack();
                    cf(
                        ap, ta, std::ptr::null(), bp, tb, std::ptr::null(),
                        &mut ca, &mut cb, 1, &mut cit, std::ptr::null_mut(),
                    )
                };
                let rd = unsafe {
                    scrub_stack();
                    rf(
                        ap, ta, std::ptr::null(), bp, tb, std::ptr::null(),
                        &mut ra, &mut rb, 1, &mut rit, std::ptr::null_mut(),
                    )
                };
                same(&format!("row41 ta={ta} tb={tb} dist"), &cd, &rd);
                same(&format!("row41 ta={ta} tb={tb} outA"), &ca, &ra);
                same(&format!("row41 ta={ta} tb={tb} outB"), &cb, &rb);
                same(&format!("row41 ta={ta} tb={tb} iters"), &cit, &rit);
            }
        }

        // Rows 42, 43: caches with in-range indices, boundary metrics and
        // div == 0, including the `metric < -1e8` half of the validity test.
        //
        // `count` is swept over 0..=3, the full range the library itself ever
        // writes into a cache. A forged `count >= 4` makes the C write
        // `saveA[3]` past the end of its `int saveA[3]`, clobbering its own loop
        // state; the C then segfaults. That is out-of-contract UB in the C, not
        // a comparable behaviour, so it is documented (ERRORS.md #42) instead of
        // asserted. `src/lib.rs` keeps `saveA`/`saveB` adjacent in one
        // `#[repr(C)]` struct so the Rust write stays inside its own 24 bytes
        // rather than aborting on a bounds check.
        for count in 0..=3i32 {
            for &metric in &[0.0f32, -1.0e9, 1.0e9, f32::MAX, -f32::MAX, f32::NAN] {
                let mut cache = c2GJKCache::default();
                cache.count = count;
                cache.metric = metric;
                cache.div = if count == 0 { 0.0 } else { 1.0 };
                for j in 0..3 {
                    // Indices stay inside the 8-vertex proxy array; out-of-range
                    // indices are an unbounded read of c2GJK's own frame and are
                    // documented (ERRORS.md #42) rather than asserted.
                    cache.iA[j] = rng.below(8) as c_int;
                    cache.iB[j] = rng.below(8) as c_int;
                }
                let mut cc = cache;
                let mut rc = cache;
                let (mut ca, mut cb, mut ra, mut rb) =
                    (c2v::default(), c2v::default(), c2v::default(), c2v::default());
                let cd = unsafe {
                    scrub_stack();
                    cf(
                        ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CAPSULE,
                        std::ptr::null(), &mut ca, &mut cb, 0,
                        std::ptr::null_mut(), &mut cc,
                    )
                };
                let rd = unsafe {
                    scrub_stack();
                    rf(
                        ap, C2_TYPE_CIRCLE, std::ptr::null(), bp, C2_TYPE_CAPSULE,
                        std::ptr::null(), &mut ra, &mut rb, 0,
                        std::ptr::null_mut(), &mut rc,
                    )
                };
                same(&format!("row42/43 k={k} count={count} dist"), &cd, &rd);
                same(&format!("row42/43 k={k} count={count} cache"), &cc, &rc);
                same(&format!("row42/43 k={k} count={count} outA"), &ca, &ra);
                same(&format!("row42/43 k={k} count={count} outB"), &cb, &rb);
            }
        }
    }
}

// ===========================================================================
// Rows 51..56 — the simplex accessors with out-of-contract `count` / `div`.
// ===========================================================================

#[test]
fn rows51_56_simplex_out_of_range_state() {
    let p = pair();
    let (cw, rw) = p.get::<FnWitness>(b"c2Witness");
    let (cl, rl) = p.get::<FnSimplexV>(b"c2L");
    let (cd, rd) = p.get::<FnSimplexV>(b"c2D");
    let (cm, rm) = p.get::<FnSimplexF>(b"c2GJKSimplexMetric");
    let mut rng = Rng::new(0xC051);

    // Rows 51, 53, 55, 56 sweep `count`; rows 52, 54 sweep `div` (incl. 0).
    let counts: [c_int; 10] = [0, 1, 2, 3, 4, 5, 7, -1, -100, i32::MAX];
    let divs: [f32; 6] = [1.0, 0.0, -0.0, -1.0, f32::NAN, f32::INFINITY];
    for k in 0..600 {
        for &count in &counts {
            for &div in &divs {
                let mut s = c2Simplex::default();
                s.count = count;
                s.div = div;
                for j in 0..4 {
                    s.verts[j].sA = rng.vec_sym(4.0);
                    s.verts[j].sB = rng.vec_sym(4.0);
                    s.verts[j].p = if k % 4 == 0 {
                        rng.vec_grid(1.0, 3)
                    } else {
                        rng.vec_sym(4.0)
                    };
                    s.verts[j].u = if k % 5 == 0 { 0.0 } else { rng.sym(4.0) };
                    s.verts[j].iA = rng.below(8) as c_int;
                    s.verts[j].iB = rng.below(8) as c_int;
                }
                let tag = format!("rows51-56 count={count} div={div:?} k={k}");

                let poison = c2v { x: 1.5e9, y: -2.5e9 };
                let (mut a1, mut b1, mut a2, mut b2) = (poison, poison, poison, poison);
                let mut s1 = s;
                let mut s2 = s;
                unsafe {
                    cw(&mut s1, &mut a1, &mut b1);
                    rw(&mut s2, &mut a2, &mut b2);
                }
                same(&format!("{tag} witness a"), &a1, &a2);
                same(&format!("{tag} witness b"), &b1, &b2);
                same(&format!("{tag} witness simplex"), &s1, &s2);

                let mut s1 = s;
                let mut s2 = s;
                unsafe { same(&format!("{tag} c2L"), &cl(&mut s1), &rl(&mut s2)) };
                let mut s1 = s;
                let mut s2 = s;
                unsafe { same(&format!("{tag} c2D"), &cd(&mut s1), &rd(&mut s2)) };
                let mut s1 = s;
                let mut s2 = s;
                unsafe { same(&format!("{tag} metric"), &cm(&mut s1), &rm(&mut s2)) };
            }
        }
    }
}

// ===========================================================================
// Rows 57, 58 — c2Support with count <= 0 and with all-equal / all-NaN dots.
// ===========================================================================

#[test]
fn rows57_58_support_degenerate() {
    let p = pair();
    let (cf, rf) = p.get::<FnSupport>(b"c2Support");
    let mut rng = Rng::new(0xC057);
    for k in 0..4000 {
        let mut verts = [c2v::default(); 8];
        for j in 0..8 {
            verts[j] = match k % 4 {
                0 => rng.vec_sym(3.0),
                1 => c2v { x: 1.0, y: 1.0 }, // all identical -> ties
                2 => rng.vec_grid(1.0, 2),
                _ => rng.vec_spicy(),
            };
        }
        let dirs: [c2v; 6] = [
            rng.vec_sym(1.0),
            c2v { x: 0.0, y: 0.0 },
            c2v { x: f32::NAN, y: f32::NAN },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
            c2v { x: -0.0, y: -0.0 },
            rng.vec_spicy(),
        ];
        for d in dirs {
            for count in [0i32, -1, -9, 1, 8] {
                unsafe {
                    same(
                        &format!("rows57/58 count={count}"),
                        &cf(verts.as_ptr(), count, d),
                        &rf(verts.as_ptr(), count, d),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Rows 59, 60, 63 — c2Collide / omni_manifold with unhandled and out-of-range
// C2_TYPE values. `*m` must come back with count == 0 and nothing else changed.
// ===========================================================================

#[test]
fn rows59_60_63_dispatch_unhandled_types() {
    let p = pair();
    let (cCol, rCol) = p.get::<FnCollide>(b"c2Collide");
    let (cO, rO) = p.get::<FnOmni>(b"omni_manifold");
    let mut rng = Rng::new(0xC059);

    let mut tys: Vec<c_int> = ALL_TYPES.to_vec();
    tys.extend_from_slice(&BAD_TYPES);

    for k in 0..1500 {
        let circle = c2Circle {
            p: rng.vec_sym(4.0),
            r: rng.unit() * 2.0,
        };
        let capsule = c2Capsule {
            a: rng.vec_sym(4.0),
            b: rng.vec_sym(4.0),
            r: rng.unit() * 2.0,
        };
        for &ta in &tys {
            for &tb in &tys {
                // c2Collide: both operands are our own buffers, so even a
                // mis-typed read is identical in the two libraries.
                let mut cm = poison_manifold(59);
                let mut rm = cm;
                let ap = &capsule as *const _ as *const c_void;
                let bp = &circle as *const _ as *const c_void;
                scrub_stack();
                unsafe { cCol(ap, ta, bp, tb, &mut cm) };
                scrub_stack();
                unsafe { rCol(ap, ta, bp, tb, &mut rm) };
                same(&format!("rows59/60 c2Collide ta={ta} tb={tb}"), &cm, &rm);

                let unhandled = !(0..=2).contains(&ta) || !(0..=2).contains(&tb);
                if unhandled {
                    assert_eq!(cm.count, 0, "ta={ta} tb={tb}: expected count 0");
                    let poison = poison_manifold(59);
                    assert_eq!(
                        raw(&cm)[4..],
                        raw(&poison)[4..],
                        "ta={ta} tb={tb}: wrote past count"
                    );
                }

                // Row 63: same through the public entry point.
                let mut cm = poison_manifold(63);
                let mut rm = cm;
                let v: [f32; 10] = std::array::from_fn(|_| rng.sym(4.0));
                scrub_stack();
                unsafe {
                    cO(&mut cm, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9])
                };
                scrub_stack();
                unsafe {
                    rO(&mut rm, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9])
                };
                same(&format!("row63 omni ta={ta} tb={tb} k={k}"), &cm, &rm);
                if unhandled {
                    assert_eq!(cm.count, 0);
                }
            }
        }
    }
}

// ===========================================================================
// Row 64 — omni_manifold with NaN / inf coordinates in every position.
// ===========================================================================

#[test]
fn row64_omni_nonfinite_inputs() {
    let p = pair();
    let (cf, rf) = p.get::<FnOmni>(b"omni_manifold");
    let mut rng = Rng::new(0xC064);
    const NASTY: [f32; 10] = [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ];
    for k in 0..40_000 {
        let ta = ALL_TYPES[k % 4];
        let tb = ALL_TYPES[(k / 4) % 4];
        // Put exactly one nasty value in a rotating slot, keeping the rest tame,
        // then also test all-nasty.
        let slot = k % 10;
        let nasty = NASTY[(k / 10) % NASTY.len()];
        let v: [f32; 10] = std::array::from_fn(|j| {
            if k % 3 == 0 {
                NASTY[rng.below(NASTY.len())]
            } else if j == slot {
                nasty
            } else {
                rng.sym(4.0)
            }
        });
        let mut cm = poison_manifold(64);
        let mut rm = cm;
        scrub_stack();
        unsafe {
            cf(&mut cm, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9])
        };
        scrub_stack();
        unsafe {
            rf(&mut rm, ta, v[0], v[1], v[2], v[3], v[4], tb, v[5], v[6], v[7], v[8], v[9])
        };
        same(&format!("row64 k={k} ta={ta} tb={tb} slot={slot}"), &cm, &rm);
    }
}

// ===========================================================================
// Row 65 — c2PlaneAt with out-of-range indices (reads adjacent bytes of OUR
// buffer, so both libraries must agree exactly).
// ===========================================================================

#[test]
fn row65_planeat_out_of_range_index() {
    let p = pair();
    let (cf, rf) = p.get::<FnPolyIH>(b"c2PlaneAt");
    let mut rng = Rng::new(0xC065);
    for k in 0..3000 {
        let mut pl = c2Poly::default();
        pl.count = rng.below(9) as c_int;
        for j in 0..8 {
            pl.verts[j] = rng.vec_sym(3.0);
            pl.norms[j] = rng.vec_sym(1.0);
        }
        let padded = PaddedPoly::new(pl);
        for i in [-4i32, -2, -1, 0, 1, 7, 8, 9, 11] {
            unsafe {
                same(
                    &format!("row65 i={i} k={k}"),
                    &cf(padded.ptr(), i),
                    &rf(padded.ptr(), i),
                );
            }
        }
    }
}

// ===========================================================================
// Rows 66, 67 — c2Div / c2Norm division by zero and c2Intersect with da == db.
// ===========================================================================

#[test]
fn rows66_67_division_degeneracies() {
    let p = pair();
    let (cDiv, rDiv) = p.get::<FnVFV>(b"c2Div");
    let (cNorm, rNorm) = p.get::<FnVV>(b"c2Norm");
    let (cInt, rInt) = p.get::<FnIntersect>(b"c2Intersect");
    let mut rng = Rng::new(0xC066);

    let zeros: [f32; 4] = [0.0, -0.0, f32::MIN_POSITIVE, -f32::MIN_POSITIVE];
    let vecs: [c2v; 7] = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: f32::INFINITY, y: 1.0 },
        c2v { x: f32::NAN, y: 1.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: 1e-30, y: 1e-30 },
    ];
    for v in vecs {
        for z in zeros {
            unsafe {
                same("row66 c2Div", &cDiv(v, z), &rDiv(v, z));
            }
        }
        unsafe {
            same("row66 c2Norm", &cNorm(v), &rNorm(v));
        }
    }
    for k in 0..4000 {
        let a = if k % 3 == 0 { rng.vec_spicy() } else { rng.vec_sym(5.0) };
        let b = if k % 3 == 1 { rng.vec_spicy() } else { rng.vec_sym(5.0) };
        // Row 67: da == db exactly (division by zero inside c2Intersect).
        let da = rng.sym(5.0);
        for db in [da, -da, 0.0, -0.0, f32::NAN, f32::INFINITY, rng.sym(5.0)] {
            unsafe {
                same(
                    &format!("row67 c2Intersect k={k}"),
                    &cInt(a, b, da, db),
                    &rInt(a, b, da, db),
                );
            }
        }
        // da == db == 0 -> 0/0
        unsafe {
            same("row67 0/0", &cInt(a, b, 0.0, 0.0), &rInt(a, b, 0.0, 0.0));
            same("row67 -0/-0", &cInt(a, b, -0.0, -0.0), &rInt(a, b, -0.0, -0.0));
        }
    }
}
