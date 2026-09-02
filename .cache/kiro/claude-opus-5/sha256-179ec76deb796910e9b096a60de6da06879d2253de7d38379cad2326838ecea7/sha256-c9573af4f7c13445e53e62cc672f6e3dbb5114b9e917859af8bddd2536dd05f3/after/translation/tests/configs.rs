//! Phase B — valid-path differential tests. One test (or one clearly labelled
//! section) per row of `CONFIGS.md`. Every comparison is bit-exact and every row
//! is driven with many randomized inputs from a fixed-seed PRNG.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

const N: usize = 60000;

// ===========================================================================
// Rows 1-13 — vector / rotation primitives
// ===========================================================================

#[test]
fn row01_c2V() {
    let b = both();
    let mut rng = Rng::new(1);
    for i in 0..N {
        let (x, y) = if i < EDGE_F32.len() * EDGE_F32.len() {
            (
                EDGE_F32[i / EDGE_F32.len()],
                EDGE_F32[i % EDGE_F32.len()],
            )
        } else {
            (rng.wild(), rng.wild())
        };
        unsafe { same("c2V", &(b.c.c2V)(x, y), &(b.rs.c2V)(x, y)) };
    }
}

#[test]
fn row02_c2Mulvs() {
    let b = both();
    let mut rng = Rng::new(2);
    for _ in 0..N {
        let v = rng.vec_wild();
        let s = rng.wild();
        unsafe { same("c2Mulvs", &(b.c.c2Mulvs)(v, s), &(b.rs.c2Mulvs)(v, s)) };
    }
    for &s in EDGE_F32 {
        for &x in EDGE_F32 {
            let v = c2v { x, y: -x };
            unsafe { same("c2Mulvs/edge", &(b.c.c2Mulvs)(v, s), &(b.rs.c2Mulvs)(v, s)) };
        }
    }
}

#[test]
fn row03_c2Maxv_c2Minv() {
    let b = both();
    let mut rng = Rng::new(3);
    for _ in 0..N {
        let (p, q) = (rng.vec_wild(), rng.vec_wild());
        unsafe {
            same("c2Maxv", &(b.c.c2Maxv)(p, q), &(b.rs.c2Maxv)(p, q));
            same("c2Minv", &(b.c.c2Minv)(p, q), &(b.rs.c2Minv)(p, q));
        }
    }
    // NaN asymmetry: the C ternary yields the *second* operand for NaN.
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let (p, q) = (c2v { x, y }, c2v { x: y, y: x });
            unsafe {
                same("c2Maxv/edge", &(b.c.c2Maxv)(p, q), &(b.rs.c2Maxv)(p, q));
                same("c2Minv/edge", &(b.c.c2Minv)(p, q), &(b.rs.c2Minv)(p, q));
            }
        }
    }
}

#[test]
fn row04_c2Clampv() {
    let b = both();
    let mut rng = Rng::new(4);
    for _ in 0..N {
        let (a, lo, hi) = (rng.vec_wild(), rng.vec_wild(), rng.vec_wild());
        unsafe {
            same(
                "c2Clampv",
                &(b.c.c2Clampv)(a, lo, hi),
                &(b.rs.c2Clampv)(a, lo, hi),
            )
        };
    }
    // lo > hi, lo == hi, NaN bounds
    for &l in EDGE_F32 {
        for &h in EDGE_F32 {
            let a = c2v { x: 0.25, y: -0.25 };
            let lo = c2v { x: l, y: h };
            let hi = c2v { x: h, y: l };
            unsafe {
                same(
                    "c2Clampv/edge",
                    &(b.c.c2Clampv)(a, lo, hi),
                    &(b.rs.c2Clampv)(a, lo, hi),
                )
            };
        }
    }
}

#[test]
fn row05_c2Sub_c2Add() {
    let b = both();
    let mut rng = Rng::new(5);
    for _ in 0..N {
        let (p, q) = (rng.vec_wild(), rng.vec_wild());
        unsafe {
            same("c2Sub", &(b.c.c2Sub)(p, q), &(b.rs.c2Sub)(p, q));
            same("c2Add", &(b.c.c2Add)(p, q), &(b.rs.c2Add)(p, q));
        }
    }
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let (p, q) = (c2v { x, y }, c2v { x: y, y: x });
            unsafe {
                same("c2Sub/edge", &(b.c.c2Sub)(p, q), &(b.rs.c2Sub)(p, q));
                same("c2Add/edge", &(b.c.c2Add)(p, q), &(b.rs.c2Add)(p, q));
            }
        }
    }
}

#[test]
fn row06_c2Dot() {
    let b = both();
    let mut rng = Rng::new(6);
    for _ in 0..N {
        let (p, q) = (rng.vec_wild(), rng.vec_wild());
        unsafe { same("c2Dot", &(b.c.c2Dot)(p, q), &(b.rs.c2Dot)(p, q)) };
    }
    for scale in [1.0e-30f32, 1.0e-8, 1.0, 1.0e8, 1.0e30] {
        for _ in 0..N {
            let p = rng.vec_coord(scale);
            let q = rng.vec_coord(scale);
            unsafe { same("c2Dot/scaled", &(b.c.c2Dot)(p, q), &(b.rs.c2Dot)(p, q)) };
        }
    }
}

#[test]
fn row07_c2Det2() {
    let b = both();
    let mut rng = Rng::new(7);
    for _ in 0..N {
        let (p, q) = (rng.vec_wild(), rng.vec_wild());
        unsafe { same("c2Det2", &(b.c.c2Det2)(p, q), &(b.rs.c2Det2)(p, q)) };
    }
    // collinear -> exact zero / negative zero
    for _ in 0..N {
        let p = rng.vec_coord(10.0);
        let k = rng.range(4.0);
        let q = c2v {
            x: p.x * k,
            y: p.y * k,
        };
        unsafe { same("c2Det2/collinear", &(b.c.c2Det2)(p, q), &(b.rs.c2Det2)(p, q)) };
    }
}

#[test]
fn row08_c2Len() {
    let b = both();
    let mut rng = Rng::new(8);
    for _ in 0..N {
        let v = rng.vec_wild();
        unsafe { same("c2Len", &(b.c.c2Len)(v), &(b.rs.c2Len)(v)) };
    }
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let v = c2v { x, y };
            unsafe { same("c2Len/edge", &(b.c.c2Len)(v), &(b.rs.c2Len)(v)) };
        }
    }
}

#[test]
fn row09_c2Div_c2Norm() {
    let b = both();
    let mut rng = Rng::new(9);
    for _ in 0..N {
        let v = rng.vec_wild();
        let s = rng.wild();
        unsafe {
            same("c2Div", &(b.c.c2Div)(v, s), &(b.rs.c2Div)(v, s));
            same("c2Norm", &(b.c.c2Norm)(v), &(b.rs.c2Norm)(v));
        }
    }
    for &x in EDGE_F32 {
        for &s in EDGE_F32 {
            let v = c2v { x, y: -x };
            unsafe {
                same("c2Div/edge", &(b.c.c2Div)(v, s), &(b.rs.c2Div)(v, s));
                same("c2Norm/edge", &(b.c.c2Norm)(v), &(b.rs.c2Norm)(v));
            }
        }
    }
}

#[test]
fn row10_c2Neg_c2Skew_c2CCW90() {
    let b = both();
    let mut rng = Rng::new(10);
    for _ in 0..N {
        let v = rng.vec_wild();
        unsafe {
            same("c2Neg", &(b.c.c2Neg)(v), &(b.rs.c2Neg)(v));
            same("c2Skew", &(b.c.c2Skew)(v), &(b.rs.c2Skew)(v));
            same("c2CCW90", &(b.c.c2CCW90)(v), &(b.rs.c2CCW90)(v));
        }
    }
    for &x in EDGE_F32 {
        let v = c2v { x, y: 0.0 };
        unsafe {
            same("c2Neg/edge", &(b.c.c2Neg)(v), &(b.rs.c2Neg)(v));
            same("c2Skew/edge", &(b.c.c2Skew)(v), &(b.rs.c2Skew)(v));
            same("c2CCW90/edge", &(b.c.c2CCW90)(v), &(b.rs.c2CCW90)(v));
        }
    }
}

#[test]
fn row11_identities() {
    let b = both();
    unsafe {
        same("c2RotIdentity", &(b.c.c2RotIdentity)(), &(b.rs.c2RotIdentity)());
        same("c2xIdentity", &(b.c.c2xIdentity)(), &(b.rs.c2xIdentity)());
    }
}

#[test]
fn row12_c2Mulrv_c2MulrvT() {
    let b = both();
    let mut rng = Rng::new(12);
    for _ in 0..N {
        // unit rotations
        let r = rng.rot();
        let v = rng.vec_coord(100.0);
        unsafe {
            same("c2Mulrv/unit", &(b.c.c2Mulrv)(r, v), &(b.rs.c2Mulrv)(r, v));
            same("c2MulrvT/unit", &(b.c.c2MulrvT)(r, v), &(b.rs.c2MulrvT)(r, v));
        }
        // arbitrary / non-unit / wild rotations
        let r2 = c2r {
            c: rng.wild(),
            s: rng.wild(),
        };
        let v2 = rng.vec_wild();
        unsafe {
            same("c2Mulrv/wild", &(b.c.c2Mulrv)(r2, v2), &(b.rs.c2Mulrv)(r2, v2));
            same(
                "c2MulrvT/wild",
                &(b.c.c2MulrvT)(r2, v2),
                &(b.rs.c2MulrvT)(r2, v2),
            );
        }
    }
    let id = c2r { c: 1.0, s: 0.0 };
    for &x in EDGE_F32 {
        let v = c2v { x, y: -x };
        unsafe {
            same("c2Mulrv/id", &(b.c.c2Mulrv)(id, v), &(b.rs.c2Mulrv)(id, v));
            same("c2MulrvT/id", &(b.c.c2MulrvT)(id, v), &(b.rs.c2MulrvT)(id, v));
        }
    }
}

#[test]
fn row13_c2Mulxv() {
    let b = both();
    let mut rng = Rng::new(13);
    let id = unsafe { (b.c.c2xIdentity)() };
    for _ in 0..N {
        let v = rng.vec_coord(100.0);
        let translation = c2x {
            p: rng.vec_coord(100.0),
            r: c2r { c: 1.0, s: 0.0 },
        };
        let rotation = c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: rng.rot(),
        };
        let full = rng.xform(100.0);
        let wild = c2x {
            p: rng.vec_wild(),
            r: c2r {
                c: rng.wild(),
                s: rng.wild(),
            },
        };
        for (label, x) in [
            ("identity", id),
            ("translation", translation),
            ("rotation", rotation),
            ("full", full),
            ("wild", wild),
        ] {
            unsafe {
                same(
                    &format!("c2Mulxv/{label}"),
                    &(b.c.c2Mulxv)(x, v),
                    &(b.rs.c2Mulxv)(x, v),
                )
            };
        }
    }
}

// ===========================================================================
// Rows 14-21 — proxies and support
// ===========================================================================

fn aabb_shapes(rng: &mut Rng) -> Vec<(&'static str, c2AABB)> {
    let n = rng.aabb(50.0);
    let z = {
        let p = rng.vec_coord(50.0);
        c2AABB { min: p, max: p }
    };
    let inv = {
        let a = rng.aabb(50.0);
        c2AABB { min: a.max, max: a.min }
    };
    let huge = c2AABB {
        min: c2v { x: -3.0e38, y: -3.0e38 },
        max: c2v { x: 3.0e38, y: 3.0e38 },
    };
    let wild = c2AABB {
        min: rng.vec_wild(),
        max: rng.vec_wild(),
    };
    vec![
        ("normal", n),
        ("zero-area", z),
        ("inverted", inv),
        ("huge", huge),
        ("wild", wild),
    ]
}

#[test]
fn row14_c2BBVerts() {
    let b = both();
    let mut rng = Rng::new(14);
    for _ in 0..N {
        for (label, bb) in aabb_shapes(&mut rng) {
            let mut cbb = bb;
            let mut rbb = bb;
            let mut co = [c2v::default(); 4];
            let mut ro = [c2v::default(); 4];
            unsafe {
                (b.c.c2BBVerts)(co.as_mut_ptr(), &mut cbb);
                (b.rs.c2BBVerts)(ro.as_mut_ptr(), &mut rbb);
            }
            same(&format!("c2BBVerts/{label}"), &co.to_vec(), &ro.to_vec());
            same(&format!("c2BBVerts/{label}/in"), &cbb.min, &rbb.min);
        }
    }
}

/// Rows 15-17: `c2MakeProxy` for each of the three valid types. The output
/// buffer is pre-filled with a known pattern so untouched bytes are visible.
#[test]
fn row15_16_17_c2MakeProxy() {
    let b = both();
    let mut rng = Rng::new(15);
    for _ in 0..N {
        for &t in TYPES.iter() {
            let parts = shape_parts(&mut rng, t, 50.0);
            let mut cp = filled_proxy();
            let mut rp = filled_proxy();
            unsafe {
                (b.c.c2MakeProxy)(parts.as_ptr() as *const c_void, t, &mut cp);
                (b.rs.c2MakeProxy)(parts.as_ptr() as *const c_void, t, &mut rp);
            }
            // Only the fields the C writes are compared for the AABB/capsule
            // cases; the untouched tail must also agree because both start from
            // the same pattern.
            same(&format!("c2MakeProxy/{}", type_name(t)), &cp, &rp);
        }
        // wild float payloads
        for &t in TYPES.iter() {
            let parts = [
                rng.wild(),
                rng.wild(),
                rng.wild(),
                rng.wild(),
                rng.wild(),
            ];
            let mut cp = filled_proxy();
            let mut rp = filled_proxy();
            unsafe {
                (b.c.c2MakeProxy)(parts.as_ptr() as *const c_void, t, &mut cp);
                (b.rs.c2MakeProxy)(parts.as_ptr() as *const c_void, t, &mut rp);
            }
            same(&format!("c2MakeProxy/wild/{}", type_name(t)), &cp, &rp);
        }
    }
}

pub fn filled_proxy() -> c2Proxy {
    let mut p = c2Proxy {
        radius: f32::from_bits(0xDEAD_BEEF),
        count: -559038737,
        verts: [c2v {
            x: f32::from_bits(0xCAFE_BABE),
            y: f32::from_bits(0xF00D_F00D),
        }; 8],
    };
    for (i, v) in p.verts.iter_mut().enumerate() {
        v.x = f32::from_bits(0xAAAA_0000 | i as u32);
    }
    p
}

#[test]
fn rows18_21_c2Support() {
    let b = both();
    let mut rng = Rng::new(18);
    for count in [1i32, 2, 4, 8] {
        for _ in 0..N {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.vec_coord(20.0);
            }
            let d = rng.vec_coord(20.0);
            unsafe {
                same(
                    &format!("c2Support/count={count}"),
                    &(b.c.c2Support)(verts.as_ptr(), count, d),
                    &(b.rs.c2Support)(verts.as_ptr(), count, d),
                )
            };
        }
        // tie-heavy: axis-aligned box verts + axis-aligned directions
        let bb = c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        let mut verts = [c2v::default(); 8];
        verts[0] = bb.min;
        verts[1] = c2v { x: bb.max.x, y: bb.min.y };
        verts[2] = bb.max;
        verts[3] = c2v { x: bb.min.x, y: bb.max.y };
        for d in [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: -1.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: f32::NAN, y: 0.0 },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        ] {
            unsafe {
                same(
                    &format!("c2Support/ties/count={count}"),
                    &(b.c.c2Support)(verts.as_ptr(), count, d),
                    &(b.rs.c2Support)(verts.as_ptr(), count, d),
                )
            };
        }
        // wild vertices
        for _ in 0..N {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.vec_wild();
            }
            let d = rng.vec_wild();
            unsafe {
                same(
                    &format!("c2Support/wild/count={count}"),
                    &(b.c.c2Support)(verts.as_ptr(), count, d),
                    &(b.rs.c2Support)(verts.as_ptr(), count, d),
                )
            };
        }
    }
}

// ===========================================================================
// Rows 22-36 — simplex machinery
// ===========================================================================

/// Random simplex whose four vertices carry distinguishable `iA`/`iB` markers,
/// so the vertex-shuffling performed by `c22`/`c23` is observable.
fn rand_simplex(rng: &mut Rng, count: i32, scale: f32, wild: bool) -> c2Simplex {
    let mut s = c2Simplex {
        verts: [c2sv::default(); 4],
        div: if wild { rng.wild() } else { rng.unit() * 4.0 + 0.25 },
        count,
    };
    for (i, v) in s.verts.iter_mut().enumerate() {
        let (p, sA, sB) = if wild {
            (rng.vec_wild(), rng.vec_wild(), rng.vec_wild())
        } else {
            (
                rng.vec_coord(scale),
                rng.vec_coord(scale),
                rng.vec_coord(scale),
            )
        };
        v.p = p;
        v.sA = sA;
        v.sB = sB;
        v.u = if wild { rng.wild() } else { rng.range(3.0) };
        v.iA = 10 * (i as i32 + 1);
        v.iB = 10 * (i as i32 + 1) + 1;
    }
    s
}

/// Coarse classification of the outcome of `c22`/`c23`, used only to prove that
/// every branch of `CONFIGS.md` rows 23-33 was actually reached.
fn classify(s: &c2Simplex) -> String {
    let mut k = format!("count{}", s.count);
    for i in 0..s.count.clamp(0, 4) {
        k.push_str(&format!(":{}", s.verts[i as usize].iA));
    }
    k
}

#[test]
fn rows23_25_c22() {
    let b = both();
    let mut rng = Rng::new(22);
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    for i in 0..N * 4 {
        let s0 = rand_simplex(&mut rng, 2, 10.0, i % 8 == 7);
        let mut cs = s0;
        let mut rs = s0;
        unsafe {
            (b.c.c22)(&mut cs);
            (b.rs.c22)(&mut rs);
        }
        same("c22", &cs, &rs);
        seen.insert(classify(&cs));
    }
    // v<=0 keeps a, u<=0 promotes b, otherwise the 2-vertex case survives.
    for want in ["count1:10", "count1:20", "count2:10:20"] {
        assert!(seen.contains(want), "c22 branch {want} never hit: {seen:?}");
    }
}

#[test]
fn rows26_33_c23() {
    let b = both();
    let mut rng = Rng::new(23);
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    for i in 0..N * 8 {
        let mut s0 = rand_simplex(&mut rng, 3, 10.0, i % 16 == 15);
        // Every 5th case: force a degenerate (zero-area) triangle.
        if i % 5 == 0 {
            let a = s0.verts[0].p;
            let d = c2v { x: rng.range(5.0), y: rng.range(5.0) };
            let k1 = rng.range(3.0);
            let k2 = rng.range(3.0);
            s0.verts[1].p = c2v { x: a.x + d.x * k1, y: a.y + d.y * k1 };
            s0.verts[2].p = c2v { x: a.x + d.x * k2, y: a.y + d.y * k2 };
        }
        let mut cs = s0;
        let mut rs = s0;
        unsafe {
            (b.c.c23)(&mut cs);
            (b.rs.c23)(&mut rs);
        }
        same("c23", &cs, &rs);
        seen.insert(classify(&cs));
    }
    for want in [
        "count1:10",       // vertex region A
        "count1:20",       // vertex region B
        "count1:30",       // vertex region C
        "count2:10:20",    // edge AB
        "count2:20:30",    // edge BC
        "count2:30:10",    // edge CA
        "count3:10:20:30", // interior
    ] {
        assert!(seen.contains(want), "c23 branch {want} never hit: {seen:?}");
    }
}

#[test]
fn row22_c2GJKSimplexMetric() {
    let b = both();
    let mut rng = Rng::new(24);
    for count in [1i32, 2, 3] {
        for i in 0..N {
            let s0 = rand_simplex(&mut rng, count, 10.0, i % 4 == 3);
            let mut cs = s0;
            let mut rs = s0;
            unsafe {
                same(
                    &format!("c2GJKSimplexMetric/count={count}"),
                    &(b.c.c2GJKSimplexMetric)(&mut cs),
                    &(b.rs.c2GJKSimplexMetric)(&mut rs),
                )
            };
            same("c2GJKSimplexMetric/no-mutate", &cs, &rs);
        }
    }
}

#[test]
fn row34_c2D() {
    let b = both();
    let mut rng = Rng::new(25);
    for count in [1i32, 2, 3] {
        for i in 0..N {
            let mut s0 = rand_simplex(&mut rng, count, 10.0, i % 4 == 3);
            // every 3rd 2-simplex: make ab collinear with the origin so the
            // strict `> 0` test fails and c2CCW90 is taken
            if count == 2 && i % 3 == 0 {
                let k = rng.range(3.0);
                let p = rng.vec_coord(10.0);
                s0.verts[0].p = p;
                s0.verts[1].p = c2v { x: p.x * k, y: p.y * k };
            }
            let mut cs = s0;
            let mut rs = s0;
            unsafe {
                same(
                    &format!("c2D/count={count}"),
                    &(b.c.c2D)(&mut cs),
                    &(b.rs.c2D)(&mut rs),
                )
            };
        }
    }
}

#[test]
fn row35_c2L() {
    let b = both();
    let mut rng = Rng::new(26);
    for count in [1i32, 2] {
        for i in 0..N {
            let mut s0 = rand_simplex(&mut rng, count, 10.0, i % 4 == 3);
            if i % 7 == 0 {
                s0.div = 0.0;
            }
            let mut cs = s0;
            let mut rs = s0;
            unsafe {
                same(
                    &format!("c2L/count={count}"),
                    &(b.c.c2L)(&mut cs),
                    &(b.rs.c2L)(&mut rs),
                )
            };
        }
    }
}

#[test]
fn row36_c2Witness() {
    let b = both();
    let mut rng = Rng::new(27);
    for count in [1i32, 2, 3] {
        for i in 0..N {
            let mut s0 = rand_simplex(&mut rng, count, 10.0, i % 4 == 3);
            if i % 7 == 0 {
                s0.div = 0.0;
            }
            let mut cs = s0;
            let mut rs = s0;
            let mut ca = c2v::default();
            let mut cb = c2v::default();
            let mut ra = c2v::default();
            let mut rb = c2v::default();
            unsafe {
                (b.c.c2Witness)(&mut cs, &mut ca, &mut cb);
                (b.rs.c2Witness)(&mut rs, &mut ra, &mut rb);
            }
            same(&format!("c2Witness/count={count}"), &(ca, cb), &(ra, rb));
        }
    }
}

// ===========================================================================
// Rows 37-46 — boolean collision routines
// ===========================================================================

fn interesting_aabbs(rng: &mut Rng) -> Vec<c2AABB> {
    let a = rng.aabb(20.0);
    let p = rng.vec_coord(20.0);
    vec![
        a,
        c2AABB { min: p, max: p },                             // zero area
        c2AABB { min: a.max, max: a.min },                     // inverted
        c2AABB { min: c2v { x: -1e30, y: -1e30 }, max: c2v { x: 1e30, y: 1e30 } },
        c2AABB { min: rng.vec_wild(), max: rng.vec_wild() },   // NaN / inf
    ]
}

fn interesting_capsules(rng: &mut Rng) -> Vec<c2Capsule> {
    let c = rng.capsule(20.0);
    let p = rng.vec_coord(20.0);
    vec![
        c,
        c2Capsule { a: p, b: p, r: rng.unit() * 5.0 },        // degenerate
        c2Capsule { a: p, b: c2v { x: p.x + 50.0, y: p.y }, r: 0.0 }, // axis aligned, r=0
        c2Capsule { a: c2v { x: -1e18, y: 0.0 }, b: c2v { x: 1e18, y: 0.0 }, r: 1.0 },
        c2Capsule { a: rng.vec_wild(), b: rng.vec_wild(), r: rng.wild() },
    ]
}

fn interesting_circles(rng: &mut Rng) -> Vec<c2Circle> {
    let c = rng.circle(20.0);
    vec![
        c,
        c2Circle { p: c.p, r: 0.0 },
        c2Circle { p: c.p, r: -c.r },
        c2Circle { p: c.p, r: 1.0e30 },
        c2Circle { p: rng.vec_wild(), r: rng.wild() },
    ]
}

#[test]
fn rows37_38_c2AABBtoAABB() {
    let b = both();
    let mut rng = Rng::new(37);
    for _ in 0..N {
        for x in interesting_aabbs(&mut rng) {
            for y in interesting_aabbs(&mut rng) {
                unsafe {
                    same(
                        "c2AABBtoAABB",
                        &(b.c.c2AABBtoAABB)(x, y),
                        &(b.rs.c2AABBtoAABB)(x, y),
                    )
                };
            }
        }
    }
    // exact-touch sweep along each axis
    for k in 0..64 {
        let d = k as f32 - 32.0;
        let a = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 10.0, y: 10.0 } };
        for bb in [
            c2AABB { min: c2v { x: d, y: 0.0 }, max: c2v { x: d + 10.0, y: 10.0 } },
            c2AABB { min: c2v { x: 0.0, y: d }, max: c2v { x: 10.0, y: d + 10.0 } },
        ] {
            unsafe {
                same(
                    "c2AABBtoAABB/touch",
                    &(b.c.c2AABBtoAABB)(a, bb),
                    &(b.rs.c2AABBtoAABB)(a, bb),
                )
            };
        }
    }
}

#[test]
fn row39_c2CircletoCircle() {
    let b = both();
    let mut rng = Rng::new(39);
    for _ in 0..N {
        for x in interesting_circles(&mut rng) {
            for y in interesting_circles(&mut rng) {
                unsafe {
                    same(
                        "c2CircletoCircle",
                        &(b.c.c2CircletoCircle)(x, y),
                        &(b.rs.c2CircletoCircle)(x, y),
                    )
                };
            }
        }
    }
    // exact-touch sweep
    for k in 0..200 {
        let d = k as f32 * 0.05;
        let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 2.0 };
        let c = c2Circle { p: c2v { x: d, y: 0.0 }, r: 3.0 };
        unsafe {
            same(
                "c2CircletoCircle/touch",
                &(b.c.c2CircletoCircle)(a, c),
                &(b.rs.c2CircletoCircle)(a, c),
            )
        };
    }
}

#[test]
fn row40_51_c2CircletoAABB() {
    let b = both();
    let mut rng = Rng::new(40);
    for _ in 0..N {
        for c in interesting_circles(&mut rng) {
            for bb in interesting_aabbs(&mut rng) {
                unsafe {
                    same(
                        "c2CircletoAABB",
                        &(b.c.c2CircletoAABB)(c, bb),
                        &(b.rs.c2CircletoAABB)(c, bb),
                    )
                };
            }
        }
    }
    // grid sweep of the circle centre over/around the box, incl. faces+corners
    let bb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
    for i in -12i32..=12 {
        for j in -12i32..=12 {
            for r in [0.0f32, 0.25, 1.0, 2.0] {
                let c = c2Circle { p: c2v { x: i as f32 * 0.25, y: j as f32 * 0.25 }, r };
                unsafe {
                    same(
                        "c2CircletoAABB/grid",
                        &(b.c.c2CircletoAABB)(c, bb),
                        &(b.rs.c2CircletoAABB)(c, bb),
                    )
                };
            }
        }
    }
}

#[test]
fn rows41_44_c2CircletoCapsule() {
    let b = both();
    let mut rng = Rng::new(41);
    for _ in 0..N {
        for c in interesting_circles(&mut rng) {
            for cap in interesting_capsules(&mut rng) {
                unsafe {
                    same(
                        "c2CircletoCapsule",
                        &(b.c.c2CircletoCapsule)(c, cap),
                        &(b.rs.c2CircletoCapsule)(c, cap),
                    )
                };
            }
        }
    }
    // sweep the circle along and past the capsule axis: hits all three
    // (da<0), (da>=0,db<0), (da>=0,db>=0) branches
    let cap = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 4.0, y: 0.0 }, r: 1.0 };
    for i in -20i32..=40 {
        for j in -6i32..=6 {
            let c = c2Circle { p: c2v { x: i as f32 * 0.25, y: j as f32 * 0.25 }, r: 0.5 };
            unsafe {
                same(
                    "c2CircletoCapsule/sweep",
                    &(b.c.c2CircletoCapsule)(c, cap),
                    &(b.rs.c2CircletoCapsule)(c, cap),
                )
            };
        }
    }
    // degenerate capsule (n == 0) -> da/c2Dot(n,n) is 0/0
    let deg = c2Capsule { a: c2v { x: 1.0, y: 1.0 }, b: c2v { x: 1.0, y: 1.0 }, r: 0.5 };
    for i in -10i32..=10 {
        let c = c2Circle { p: c2v { x: i as f32 * 0.3, y: 1.0 }, r: 0.5 };
        unsafe {
            same(
                "c2CircletoCapsule/degenerate",
                &(b.c.c2CircletoCapsule)(c, deg),
                &(b.rs.c2CircletoCapsule)(c, deg),
            )
        };
    }
}

#[test]
fn row45_c2AABBtoCapsule() {
    let b = both();
    let mut rng = Rng::new(45);
    for _ in 0..N {
        for bb in interesting_aabbs(&mut rng) {
            for cap in interesting_capsules(&mut rng) {
                unsafe {
                    same(
                        "c2AABBtoCapsule",
                        &(b.c.c2AABBtoCapsule)(bb, cap),
                        &(b.rs.c2AABBtoCapsule)(bb, cap),
                    )
                };
            }
        }
    }
    let bb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
    for i in -16i32..=16 {
        for j in -16i32..=16 {
            let cap = c2Capsule {
                a: c2v { x: i as f32 * 0.25, y: j as f32 * 0.25 },
                b: c2v { x: i as f32 * 0.25 + 1.5, y: j as f32 * 0.25 + 0.5 },
                r: 0.4,
            };
            unsafe {
                same(
                    "c2AABBtoCapsule/grid",
                    &(b.c.c2AABBtoCapsule)(bb, cap),
                    &(b.rs.c2AABBtoCapsule)(bb, cap),
                )
            };
        }
    }
}

#[test]
fn row46_c2CapsuletoCapsule() {
    let b = both();
    let mut rng = Rng::new(46);
    for _ in 0..N {
        for x in interesting_capsules(&mut rng) {
            for y in interesting_capsules(&mut rng) {
                unsafe {
                    same(
                        "c2CapsuletoCapsule",
                        &(b.c.c2CapsuletoCapsule)(x, y),
                        &(b.rs.c2CapsuletoCapsule)(x, y),
                    )
                };
            }
        }
    }
    // crossing / parallel / collinear / identical
    let base = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 4.0, y: 0.0 }, r: 0.5 };
    for i in -20i32..=20 {
        let t = i as f32 * 0.25;
        for other in [
            c2Capsule { a: c2v { x: t, y: -2.0 }, b: c2v { x: t, y: 2.0 }, r: 0.5 }, // crossing
            c2Capsule { a: c2v { x: 0.0, y: t }, b: c2v { x: 4.0, y: t }, r: 0.5 },   // parallel
            c2Capsule { a: c2v { x: t, y: 0.0 }, b: c2v { x: t + 4.0, y: 0.0 }, r: 0.5 }, // collinear
            base,                                                                     // identical
        ] {
            unsafe {
                same(
                    "c2CapsuletoCapsule/sweep",
                    &(b.c.c2CapsuletoCapsule)(base, other),
                    &(b.rs.c2CapsuletoCapsule)(base, other),
                )
            };
        }
    }
}

// ===========================================================================
// Rows 47-73 — c2GJK, the lowest-level entry point, driven directly
// ===========================================================================

pub struct GjkResult {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: i32,
    pub cache: c2GJKCache,
}

impl Bits for GjkResult {
    fn bits(&self) -> Vec<u32> {
        let mut v = vec![self.dist.to_bits()];
        v.extend(self.a.bits());
        v.extend(self.b.bits());
        v.push(self.iters as u32);
        v.extend(self.cache.bits());
        v
    }
}

impl std::fmt::Debug for GjkResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dist={:?} a={:?} b={:?} iters={} cache={:?}",
            self.dist, self.a, self.b, self.iters, self.cache
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn call_gjk(
    api: &Api,
    pa: &[f32; 5],
    ta: C2_TYPE,
    ax: Option<c2x>,
    pb: &[f32; 5],
    tb: C2_TYPE,
    bx: Option<c2x>,
    use_radius: i32,
    cache_in: Option<c2GJKCache>,
) -> GjkResult {
    let mut outa = c2v { x: 1.25, y: -7.5 };
    let mut outb = c2v { x: -3.5, y: 11.0 };
    let mut iters: i32 = -12345;
    let mut cache = cache_in.unwrap_or_default();
    let axp = ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let cp = if cache_in.is_some() {
        &mut cache as *mut c2GJKCache
    } else {
        std::ptr::null_mut()
    };
    let dist = unsafe {
        (api.c2GJK)(
            pa.as_ptr() as *const c_void,
            ta,
            axp,
            pb.as_ptr() as *const c_void,
            tb,
            bxp,
            &mut outa,
            &mut outb,
            use_radius,
            &mut iters,
            cp,
        )
    };
    GjkResult {
        dist,
        a: outa,
        b: outb,
        iters,
        cache,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn cmp_gjk(
    b: &Both,
    label: &str,
    pa: &[f32; 5],
    ta: C2_TYPE,
    ax: Option<c2x>,
    pb: &[f32; 5],
    tb: C2_TYPE,
    bx: Option<c2x>,
    use_radius: i32,
    cache_in: Option<c2GJKCache>,
) -> (GjkResult, GjkResult) {
    let rc = call_gjk(&b.c, pa, ta, ax, pb, tb, bx, use_radius, cache_in);
    let rr = call_gjk(&b.rs, pa, ta, ax, pb, tb, bx, use_radius, cache_in);
    same(
        &format!(
            "c2GJK/{label} {}x{} ur={use_radius} ax={} bx={} cache={}",
            type_name(ta),
            type_name(tb),
            ax.is_some(),
            bx.is_some(),
            cache_in.is_some()
        ),
        &rc,
        &rr,
    );
    (rc, rr)
}

const GJK_N: usize = 5000;

/// Rows 47-56: every (typeA,typeB) pair × `use_radius` ∈ {0,1}, no transforms,
/// no cache, all output pointers live.
#[test]
fn rows47_56_gjk_type_matrix() {
    let b = both();
    let mut rng = Rng::new(47);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for ur in [0i32, 1] {
                for _ in 0..GJK_N {
                    let pa = shape_parts(&mut rng, ta, 20.0);
                    let pb = shape_parts(&mut rng, tb, 20.0);
                    cmp_gjk(b, "matrix", &pa, ta, None, &pb, tb, None, ur, None);
                }
            }
        }
    }
}

/// Rows 57-60: transforms — translation only, rotation only, both, non-unit.
#[test]
fn rows57_60_gjk_transforms() {
    let b = both();
    let mut rng = Rng::new(57);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..GJK_N {
                let pa = shape_parts(&mut rng, ta, 20.0);
                let pb = shape_parts(&mut rng, tb, 20.0);
                let trans = c2x { p: rng.vec_coord(30.0), r: c2r { c: 1.0, s: 0.0 } };
                let rot = c2x { p: c2v { x: 0.0, y: 0.0 }, r: rng.rot() };
                let full = rng.xform(30.0);
                let nonunit = c2x {
                    p: rng.vec_coord(30.0),
                    r: c2r { c: rng.range(3.0), s: rng.range(3.0) },
                };
                for ur in [0i32, 1] {
                    // row 57: A transformed by translation
                    cmp_gjk(b, "ax-translation", &pa, ta, Some(trans), &pb, tb, None, ur, None);
                    // row 58: B transformed by rotation
                    cmp_gjk(b, "bx-rotation", &pa, ta, None, &pb, tb, Some(rot), ur, None);
                    // row 59: both full transforms
                    cmp_gjk(b, "both-full", &pa, ta, Some(full), &pb, tb, Some(rot), ur, None);
                    // row 60: non-unit rotation
                    cmp_gjk(b, "non-unit", &pa, ta, Some(nonunit), &pb, tb, Some(nonunit), ur, None);
                }
            }
        }
    }
}

/// Rows 61-63: cold cache, warm cache reused on identical shapes, warm cache
/// reused after moving B (the real broadphase usage pattern).
#[test]
fn rows61_63_gjk_cache() {
    let b = both();
    let mut rng = Rng::new(61);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..GJK_N {
                let pa = shape_parts(&mut rng, ta, 20.0);
                let mut pb = shape_parts(&mut rng, tb, 20.0);
                let ur = if rng.bool() { 1 } else { 0 };

                // row 61 — cold cache (count == 0), written back on exit
                let cold = c2GJKCache::default();
                let (rc, _) = cmp_gjk(b, "cache-cold", &pa, ta, None, &pb, tb, None, ur, Some(cold));

                // row 62 — warm cache, identical shapes
                let warm = rc.cache;
                let (rc2, _) =
                    cmp_gjk(b, "cache-warm-same", &pa, ta, None, &pb, tb, None, ur, Some(warm));

                // row 63 — warm cache, shape B moved
                let dx = rng.range(4.0);
                let dy = rng.range(4.0);
                match tb {
                    C2_TYPE_CIRCLE => {
                        pb[0] += dx;
                        pb[1] += dy;
                    }
                    C2_TYPE_AABB => {
                        pb[0] += dx;
                        pb[1] += dy;
                        pb[2] += dx;
                        pb[3] += dy;
                    }
                    _ => {
                        pb[0] += dx;
                        pb[1] += dy;
                        pb[2] += dx;
                        pb[3] += dy;
                    }
                }
                let mut acc = rc2.cache;
                for step in 0..4 {
                    let (r, _) = cmp_gjk(
                        b,
                        &format!("cache-warm-moved-{step}"),
                        &pa,
                        ta,
                        None,
                        &pb,
                        tb,
                        None,
                        ur,
                        Some(acc),
                    );
                    acc = r.cache;
                }
            }
        }
    }
}

/// Rows 64-67: deeply overlapping / exactly touching / identical / far apart.
#[test]
fn rows64_67_gjk_separations() {
    let b = both();
    let mut rng = Rng::new(64);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..GJK_N {
                let pa = shape_parts(&mut rng, ta, 5.0);
                for ur in [0i32, 1] {
                    // row 64: deeply overlapping -> hit path (count == 3)
                    let mut pb = shape_parts(&mut rng, tb, 1.0);
                    cmp_gjk(b, "overlap", &pa, ta, None, &pb, tb, None, ur, None);

                    // row 66: identical shapes (only meaningful when ta == tb)
                    if ta == tb {
                        cmp_gjk(b, "identical", &pa, ta, None, &pa, tb, None, ur, None);
                    }

                    // row 67: far apart
                    let far = c2x { p: c2v { x: 1.0e6, y: -2.5e6 }, r: c2r { c: 1.0, s: 0.0 } };
                    cmp_gjk(b, "far", &pa, ta, None, &pb, tb, Some(far), ur, None);

                    // row 65: exact touch — shift B so it just meets A on x
                    for k in 0..8 {
                        let d = k as f32 * 0.5;
                        let sh = c2x { p: c2v { x: d, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
                        cmp_gjk(b, "touch", &pa, ta, None, &pb, tb, Some(sh), ur, None);
                    }
                    // ...and again with B shrunk to a point
                    pb[4] = 0.0;
                    cmp_gjk(b, "touch-point", &pa, ta, None, &pb, tb, None, ur, None);
                }
            }
        }
    }
}

/// Rows 68-71: extreme magnitudes and radii.
#[test]
fn rows68_71_gjk_magnitudes() {
    let b = both();
    let mut rng = Rng::new(68);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for ur in [0i32, 1] {
                for _ in 0..GJK_N {
                    // row 68: huge coordinates
                    let mut pa = shape_parts(&mut rng, ta, 1.0e18);
                    let mut pb = shape_parts(&mut rng, tb, 1.0e18);
                    cmp_gjk(b, "huge", &pa, ta, None, &pb, tb, None, ur, None);

                    // row 69: tiny coordinates
                    let pa2 = shape_parts(&mut rng, ta, 1.0e-30);
                    let pb2 = shape_parts(&mut rng, tb, 1.0e-30);
                    cmp_gjk(b, "tiny", &pa2, ta, None, &pb2, tb, None, ur, None);

                    // row 70: zero radii
                    let mut pa3 = shape_parts(&mut rng, ta, 10.0);
                    let mut pb3 = shape_parts(&mut rng, tb, 10.0);
                    zero_radius(&mut pa3, ta);
                    zero_radius(&mut pb3, tb);
                    cmp_gjk(b, "zero-radius", &pa3, ta, None, &pb3, tb, None, ur, None);

                    // row 71: radii larger than the separation
                    set_radius(&mut pa, ta, 1.0e20);
                    set_radius(&mut pb, tb, 1.0e20);
                    let pa4 = shape_parts(&mut rng, ta, 10.0);
                    let mut pa4 = pa4;
                    let mut pb4 = shape_parts(&mut rng, tb, 10.0);
                    set_radius(&mut pa4, ta, 1000.0);
                    set_radius(&mut pb4, tb, 1000.0);
                    cmp_gjk(b, "big-radius", &pa4, ta, None, &pb4, tb, None, ur, None);
                }
            }
        }
    }
}

fn radius_slot(t: C2_TYPE) -> Option<usize> {
    match t {
        C2_TYPE_CIRCLE => Some(2),
        C2_TYPE_CAPSULE => Some(4),
        _ => None,
    }
}
fn zero_radius(p: &mut [f32; 5], t: C2_TYPE) {
    if let Some(i) = radius_slot(t) {
        p[i] = 0.0;
    }
}
fn set_radius(p: &mut [f32; 5], t: C2_TYPE, r: f32) {
    if let Some(i) = radius_slot(t) {
        p[i] = r;
    }
}

/// Rows 72-73: degenerate proxies (zero-area AABB, `a == b` capsule) and
/// inverted AABBs.
#[test]
fn rows72_73_gjk_degenerate() {
    let b = both();
    let mut rng = Rng::new(72);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for ur in [0i32, 1] {
                for _ in 0..GJK_N {
                    let mut pa = shape_parts(&mut rng, ta, 10.0);
                    let mut pb = shape_parts(&mut rng, tb, 10.0);
                    degenerate(&mut pa, ta);
                    degenerate(&mut pb, tb);
                    cmp_gjk(b, "degenerate", &pa, ta, None, &pb, tb, None, ur, None);

                    let mut pa2 = shape_parts(&mut rng, ta, 10.0);
                    let mut pb2 = shape_parts(&mut rng, tb, 10.0);
                    invert(&mut pa2, ta);
                    invert(&mut pb2, tb);
                    cmp_gjk(b, "inverted", &pa2, ta, None, &pb2, tb, None, ur, None);
                }
            }
        }
    }
}

fn degenerate(p: &mut [f32; 5], t: C2_TYPE) {
    match t {
        C2_TYPE_AABB => {
            p[2] = p[0];
            p[3] = p[1];
        }
        C2_TYPE_CAPSULE => {
            p[2] = p[0];
            p[3] = p[1];
        }
        _ => p[2] = 0.0,
    }
}
fn invert(p: &mut [f32; 5], t: C2_TYPE) {
    if t == C2_TYPE_AABB {
        p.swap(0, 2);
        p.swap(1, 3);
        // force min > max
        if p[0] < p[2] {
            p.swap(0, 2);
        }
        if p[1] < p[3] {
            p.swap(1, 3);
        }
    }
}

// ===========================================================================
// Rows 74-80 — top-level entry points and cross-library pipelines
// ===========================================================================

#[test]
fn row74_c2Collided() {
    let b = both();
    let mut rng = Rng::new(74);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..N {
                let pa = shape_parts(&mut rng, ta, 15.0);
                let pb = shape_parts(&mut rng, tb, 15.0);
                let (ca, cb) = unsafe {
                    (
                        (b.c.c2Collided)(
                            pa.as_ptr() as *const c_void,
                            ta,
                            pb.as_ptr() as *const c_void,
                            tb,
                        ),
                        (b.rs.c2Collided)(
                            pa.as_ptr() as *const c_void,
                            ta,
                            pb.as_ptr() as *const c_void,
                            tb,
                        ),
                    )
                };
                same(
                    &format!("c2Collided/{}x{}", type_name(ta), type_name(tb)),
                    &ca,
                    &cb,
                );
            }
        }
    }
}

fn cmp_omni(b: &Both, ta: C2_TYPE, a: &[f32; 5], tb: C2_TYPE, bb: &[f32; 5], label: &str) {
    let rc = unsafe { (b.c.omni_collide)(ta, a[0], a[1], a[2], a[3], a[4], tb, bb[0], bb[1], bb[2], bb[3], bb[4]) };
    let rr = unsafe { (b.rs.omni_collide)(ta, a[0], a[1], a[2], a[3], a[4], tb, bb[0], bb[1], bb[2], bb[3], bb[4]) };
    if rc != rr {
        panic!(
            "MISMATCH omni_collide/{label} {}x{}\n  A = {a:?}\n  B = {bb:?}\n  C = {rc}, Rust = {rr}",
            type_name(ta),
            type_name(tb)
        );
    }
}

#[test]
fn row75_omni_collide_random() {
    let b = both();
    let mut rng = Rng::new(75);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for scale in [0.001f32, 1.0, 15.0, 1.0e6, 1.0e18] {
                for _ in 0..N {
                    let a = shape_parts(&mut rng, ta, scale);
                    let bb = shape_parts(&mut rng, tb, scale);
                    cmp_omni(b, ta, &a, tb, &bb, "random");
                }
            }
        }
    }
}

#[test]
fn row76_omni_collide_grid() {
    let b = both();
    let mut rng = Rng::new(76);
    let g: Vec<f32> = (-6i32..=6).map(|i| i as f32 * 0.5).collect();
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for &x in g.iter() {
                for &y in g.iter() {
                    let a = [0.0, 0.0, 1.0, 1.0, 0.5];
                    let bb = [x, y, x + 1.0, y + 1.0, 0.5];
                    cmp_omni(b, ta, &a, tb, &bb, "grid");
                    let a2 = [x, y, x + 0.25, y - 0.25, 0.0];
                    cmp_omni(b, ta, &a2, tb, &bb, "grid2");
                    let a3 = shape_parts(&mut rng, ta, 3.0);
                    cmp_omni(b, ta, &a3, tb, &bb, "grid3");
                }
            }
        }
    }
}

#[test]
fn row77_omni_collide_float_zoo() {
    let b = both();
    let mut rng = Rng::new(77);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..N {
                let a = [rng.wild(), rng.wild(), rng.wild(), rng.wild(), rng.wild()];
                let bb = [rng.wild(), rng.wild(), rng.wild(), rng.wild(), rng.wild()];
                cmp_omni(b, ta, &a, tb, &bb, "zoo");
            }
            // one wild component at a time, everything else sane
            for &w in EDGE_F32 {
                for slot in 0..5usize {
                    let mut a = shape_parts(&mut rng, ta, 5.0);
                    let mut bb = shape_parts(&mut rng, tb, 5.0);
                    a[slot] = w;
                    cmp_omni(b, ta, &a, tb, &bb, "zoo-a");
                    bb[slot] = w;
                    cmp_omni(b, ta, &a, tb, &bb, "zoo-b");
                }
            }
        }
    }
}

#[test]
fn row78_ptr_from_parts_valid() {
    let b = both();
    let mut rng = Rng::new(78);
    for &t in TYPES.iter() {
        let nwords = match t {
            C2_TYPE_CIRCLE => 3usize,
            C2_TYPE_AABB => 4,
            _ => 5,
        };
        for _ in 0..N {
            let p = shape_parts(&mut rng, t, 20.0);
            unsafe {
                let cp = (b.c.ptr_from_parts)(t, p[0], p[1], p[2], p[3], p[4]);
                let rp = (b.rs.ptr_from_parts)(t, p[0], p[1], p[2], p[3], p[4]);
                assert!(!cp.is_null(), "C ptr_from_parts returned NULL for valid type");
                assert!(!rp.is_null(), "Rust ptr_from_parts returned NULL for valid type");
                let cs = std::slice::from_raw_parts(cp as *const f32, nwords).to_vec();
                let rs = std::slice::from_raw_parts(rp as *const f32, nwords).to_vec();
                same(
                    &format!("ptr_from_parts/{}", type_name(t)),
                    &cs.iter().map(|x| c2v { x: *x, y: 0.0 }).collect::<Vec<_>>(),
                    &rs.iter().map(|x| c2v { x: *x, y: 0.0 }).collect::<Vec<_>>(),
                );
                libc_free(cp);
                libc_free(rp);
            }
        }
    }
}

unsafe extern "C" {
    fn free(p: *mut c_void);
}
fn libc_free(p: *mut c_void) {
    unsafe { free(p) }
}

/// Row 79: cross-library composition — a pointer produced by one library's
/// `ptr_from_parts` fed into the other library's `c2Collided`. All four
/// combinations must agree.
#[test]
fn row79_cross_library_pipeline() {
    let b = both();
    let mut rng = Rng::new(79);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..N {
                let pa = shape_parts(&mut rng, ta, 15.0);
                let pb = shape_parts(&mut rng, tb, 15.0);
                unsafe {
                    let ca = (b.c.ptr_from_parts)(ta, pa[0], pa[1], pa[2], pa[3], pa[4]);
                    let cb = (b.c.ptr_from_parts)(tb, pb[0], pb[1], pb[2], pb[3], pb[4]);
                    let ra = (b.rs.ptr_from_parts)(ta, pa[0], pa[1], pa[2], pa[3], pa[4]);
                    let rb = (b.rs.ptr_from_parts)(tb, pb[0], pb[1], pb[2], pb[3], pb[4]);
                    let v = [
                        (b.c.c2Collided)(ca, ta, cb, tb),
                        (b.c.c2Collided)(ra, ta, rb, tb),
                        (b.rs.c2Collided)(ca, ta, cb, tb),
                        (b.rs.c2Collided)(ra, ta, rb, tb),
                    ];
                    assert!(
                        v.iter().all(|x| *x == v[0]),
                        "cross-library pipeline disagreement {}x{}: {v:?}",
                        type_name(ta),
                        type_name(tb)
                    );
                    libc_free(ca);
                    libc_free(cb);
                    libc_free(ra);
                    libc_free(rb);
                }
            }
        }
    }
}

/// Row 80: the GJK inner loop driven by hand, one primitive at a time, so a
/// divergence is attributed to a specific step rather than to the whole
/// pipeline. Mirrors the body of `c2GJK` for a couple of iterations.
#[test]
fn row80_manual_pipeline() {
    let b = both();
    let mut rng = Rng::new(80);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for _ in 0..4000 {
                let pa = shape_parts(&mut rng, ta, 15.0);
                let pb = shape_parts(&mut rng, tb, 15.0);
                let ax = if rng.bool() { rng.xform(10.0) } else { unsafe { (b.c.c2xIdentity)() } };
                let bx = if rng.bool() { rng.xform(10.0) } else { unsafe { (b.c.c2xIdentity)() } };

                let mut cpa = filled_proxy();
                let mut cpb = filled_proxy();
                let mut rpa = filled_proxy();
                let mut rpb = filled_proxy();
                unsafe {
                    (b.c.c2MakeProxy)(pa.as_ptr() as *const c_void, ta, &mut cpa);
                    (b.c.c2MakeProxy)(pb.as_ptr() as *const c_void, tb, &mut cpb);
                    (b.rs.c2MakeProxy)(pa.as_ptr() as *const c_void, ta, &mut rpa);
                    (b.rs.c2MakeProxy)(pb.as_ptr() as *const c_void, tb, &mut rpb);
                }
                same("pipeline/proxyA", &cpa, &rpa);
                same("pipeline/proxyB", &cpb, &rpb);

                // seed simplex vertex 0 exactly like c2GJK does
                let mut cs = c2Simplex::default();
                let mut rs = c2Simplex::default();
                unsafe {
                    for (api, s, ppa, ppb) in [
                        (&b.c, &mut cs, &cpa, &cpb),
                        (&b.rs, &mut rs, &rpa, &rpb),
                    ] {
                        s.verts[0].iA = 0;
                        s.verts[0].iB = 0;
                        s.verts[0].sA = (api.c2Mulxv)(ax, ppa.verts[0]);
                        s.verts[0].sB = (api.c2Mulxv)(bx, ppb.verts[0]);
                        s.verts[0].p = (api.c2Sub)(s.verts[0].sB, s.verts[0].sA);
                        s.verts[0].u = 1.0;
                        s.div = 1.0;
                        s.count = 1;
                    }
                }
                same("pipeline/seed", &cs, &rs);

                for _ in 0..3 {
                    match cs.count {
                        2 => unsafe {
                            (b.c.c22)(&mut cs);
                            (b.rs.c22)(&mut rs);
                        },
                        3 => unsafe {
                            (b.c.c23)(&mut cs);
                            (b.rs.c23)(&mut rs);
                        },
                        _ => {}
                    }
                    same("pipeline/subdistance", &cs, &rs);
                    if cs.count == 3 {
                        break;
                    }
                    let (cl, rl) = unsafe { ((b.c.c2L)(&mut cs), (b.rs.c2L)(&mut rs)) };
                    same("pipeline/c2L", &cl, &rl);
                    let (cd, rd) = unsafe { ((b.c.c2D)(&mut cs), (b.rs.c2D)(&mut rs)) };
                    same("pipeline/c2D", &cd, &rd);
                    let cia = unsafe {
                        (b.c.c2Support)(
                            cpa.verts.as_ptr(),
                            cpa.count,
                            (b.c.c2MulrvT)(ax.r, (b.c.c2Neg)(cd)),
                        )
                    };
                    let ria = unsafe {
                        (b.rs.c2Support)(
                            rpa.verts.as_ptr(),
                            rpa.count,
                            (b.rs.c2MulrvT)(ax.r, (b.rs.c2Neg)(rd)),
                        )
                    };
                    same("pipeline/supportA", &cia, &ria);
                    let cib = unsafe {
                        (b.c.c2Support)(cpb.verts.as_ptr(), cpb.count, (b.c.c2MulrvT)(bx.r, cd))
                    };
                    let rib = unsafe {
                        (b.rs.c2Support)(rpb.verts.as_ptr(), rpb.count, (b.rs.c2MulrvT)(bx.r, rd))
                    };
                    same("pipeline/supportB", &cib, &rib);

                    let n = cs.count as usize;
                    if n >= 4 {
                        break;
                    }
                    unsafe {
                        for (api, s, ppa, ppb, ia, ib) in [
                            (&b.c, &mut cs, &cpa, &cpb, cia, cib),
                            (&b.rs, &mut rs, &rpa, &rpb, ria, rib),
                        ] {
                            s.verts[n].iA = ia;
                            s.verts[n].iB = ib;
                            s.verts[n].sA = (api.c2Mulxv)(ax, ppa.verts[ia as usize]);
                            s.verts[n].sB = (api.c2Mulxv)(bx, ppb.verts[ib as usize]);
                            s.verts[n].p = (api.c2Sub)(s.verts[n].sB, s.verts[n].sA);
                            s.count += 1;
                        }
                    }
                    same("pipeline/extend", &cs, &rs);
                }

                let mut ca = c2v::default();
                let mut cb = c2v::default();
                let mut ra = c2v::default();
                let mut rb = c2v::default();
                unsafe {
                    (b.c.c2Witness)(&mut cs, &mut ca, &mut cb);
                    (b.rs.c2Witness)(&mut rs, &mut ra, &mut rb);
                }
                same("pipeline/witness", &(ca, cb), &(ra, rb));
                let cd = unsafe { (b.c.c2Len)((b.c.c2Sub)(ca, cb)) };
                let rd = unsafe { (b.rs.c2Len)((b.rs.c2Sub)(ra, rb)) };
                same("pipeline/dist", &cd, &rd);
                let cm = unsafe { (b.c.c2GJKSimplexMetric)(&mut cs) };
                let rm = unsafe { (b.rs.c2GJKSimplexMetric)(&mut rs) };
                same("pipeline/metric", &cm, &rm);
            }
        }
    }
}
