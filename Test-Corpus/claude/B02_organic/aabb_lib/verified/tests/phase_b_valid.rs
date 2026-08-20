//! PHASE B — valid-path differential tests.
//!
//! One test (or one clearly-labelled block inside a test) per row of
//! `CONFIGS.md`.  Every case calls the C `.so` and the Rust `.so` through
//! `libloading` and compares the results **bit for bit** (`f32::to_bits`), so
//! `+0.0` vs `-0.0` and NaN sign/payload differences are caught too.
//!
//! Inputs are property-style randomized with a fixed seed per row.

#![allow(non_snake_case)]

mod common;
use common::*;

// ===========================================================================
// Group 1 — leaf value helpers (rows 01..14)
// ===========================================================================

/// row 01 — `c2V`
#[test]
fn b01_c2V() {
    let (c, r) = apis();
    let mut g = Rng::new(0x01);
    for _ in 0..20_000 {
        let x = g.f32_mixed(200.0);
        let y = g.f32_mixed(200.0);
        unsafe { assert_same("c2V", &(x, y), (c.c2V)(x, y), (r.c2V)(x, y)) };
    }
}

/// row 02 — `c2Mulvs`
#[test]
fn b02_c2Mulvs() {
    let (c, r) = apis();
    let mut g = Rng::new(0x02);
    for _ in 0..20_000 {
        let a = g.v_mixed(200.0);
        let b = g.f32_mixed(200.0);
        unsafe { assert_same("c2Mulvs", &(a, b), (c.c2Mulvs)(a, b), (r.c2Mulvs)(a, b)) };
    }
}

/// row 03 — `c2Maxv` / `c2Minv`
#[test]
fn b03_c2Maxv_c2Minv() {
    let (c, r) = apis();
    let mut g = Rng::new(0x03);
    for _ in 0..20_000 {
        let a = g.v_mixed(50.0);
        let b = g.v_mixed(50.0);
        unsafe {
            assert_same("c2Maxv", &(a, b), (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
            assert_same("c2Minv", &(a, b), (c.c2Minv)(a, b), (r.c2Minv)(a, b));
        }
    }
    // explicit +0 / -0 ties (the ternary must pick `b`)
    let zeros = [0.0f32, -0.0f32];
    for &ax in &zeros {
        for &ay in &zeros {
            for &bx in &zeros {
                for &by in &zeros {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    unsafe {
                        assert_same("c2Maxv/0", &(a, b), (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
                        assert_same("c2Minv/0", &(a, b), (c.c2Minv)(a, b), (r.c2Minv)(a, b));
                    }
                }
            }
        }
    }
}

/// row 04 — `c2Clampv`
#[test]
fn b04_c2Clampv() {
    let (c, r) = apis();
    let mut g = Rng::new(0x04);
    for _ in 0..20_000 {
        let a = g.v_mixed(50.0);
        let lo = g.v_mixed(50.0);
        let hi = g.v_mixed(50.0);
        unsafe {
            assert_same(
                "c2Clampv",
                &(a, lo, hi),
                (c.c2Clampv)(a, lo, hi),
                (r.c2Clampv)(a, lo, hi),
            )
        };
    }
    // inverted and collapsed ranges
    for _ in 0..20_000 {
        let a = g.v_geom(20.0);
        let lo = g.v_geom(20.0);
        let hi = if g.below(3) == 0 { lo } else { g.v_geom(20.0) };
        unsafe {
            assert_same(
                "c2Clampv/inv",
                &(a, lo, hi),
                (c.c2Clampv)(a, lo, hi),
                (r.c2Clampv)(a, lo, hi),
            )
        };
    }
}

/// row 05 — `c2Sub` / `c2Add`
#[test]
fn b05_c2Sub_c2Add() {
    let (c, r) = apis();
    let mut g = Rng::new(0x05);
    for _ in 0..20_000 {
        let a = g.v_mixed(200.0);
        let b = g.v_mixed(200.0);
        unsafe {
            assert_same("c2Sub", &(a, b), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
            assert_same("c2Add", &(a, b), (c.c2Add)(a, b), (r.c2Add)(a, b));
        }
    }
    // inf - inf, ±0 ± ±0
    let sp = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::MAX,
        f32::MIN,
    ];
    for &x in &sp {
        for &y in &sp {
            let a = c2v { x, y };
            for &u in &sp {
                for &v in &sp {
                    let b = c2v { x: u, y: v };
                    unsafe {
                        assert_same("c2Sub/sp", &(a, b), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
                        assert_same("c2Add/sp", &(a, b), (c.c2Add)(a, b), (r.c2Add)(a, b));
                    }
                }
            }
        }
    }
}

/// row 06 — `c2Dot`
#[test]
fn b06_c2Dot() {
    let (c, r) = apis();
    let mut g = Rng::new(0x06);
    for _ in 0..20_000 {
        let a = g.v_mixed(200.0);
        let b = g.v_mixed(200.0);
        unsafe {
            assert_same("c2Dot", &(a, b), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
            // a·a — the `c2Len` feeder
            assert_same("c2Dot/aa", &a, (c.c2Dot)(a, a), (r.c2Dot)(a, a));
        }
    }
    // exact cancellation: a.x*b.x == -(a.y*b.y)
    for _ in 0..20_000 {
        let k = g.f32_grid(6);
        let a = c2v {
            x: k,
            y: g.f32_grid(6),
        };
        let b = c2v {
            x: g.f32_grid(6),
            y: -k,
        };
        unsafe { assert_same("c2Dot/cancel", &(a, b), (c.c2Dot)(a, b), (r.c2Dot)(a, b)) };
    }
}

/// row 07 — `c2Det2`
#[test]
fn b07_c2Det2() {
    let (c, r) = apis();
    let mut g = Rng::new(0x07);
    for _ in 0..20_000 {
        let a = g.v_mixed(200.0);
        let b = g.v_mixed(200.0);
        unsafe { assert_same("c2Det2", &(a, b), (c.c2Det2)(a, b), (r.c2Det2)(a, b)) };
    }
    // collinear (det == ±0) and anti-collinear
    for _ in 0..20_000 {
        let a = g.v_geom(30.0);
        let k = g.f32_grid(4);
        let b = c2v {
            x: a.x * k,
            y: a.y * k,
        };
        unsafe {
            assert_same("c2Det2/col", &(a, b), (c.c2Det2)(a, b), (r.c2Det2)(a, b));
            assert_same("c2Det2/col2", &(b, a), (c.c2Det2)(b, a), (r.c2Det2)(b, a));
        }
    }
}

/// row 08 — `c2Len`
#[test]
fn b08_c2Len() {
    let (c, r) = apis();
    let mut g = Rng::new(0x08);
    for _ in 0..20_000 {
        let a = g.v_mixed(200.0);
        unsafe { assert_same("c2Len", &a, (c.c2Len)(a), (r.c2Len)(a)) };
    }
    // overflow / underflow / zero
    let sp = [
        0.0f32,
        -0.0,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        1e30,
        1e-30,
        3.0,
        4.0,
    ];
    for &x in &sp {
        for &y in &sp {
            let a = c2v { x, y };
            unsafe { assert_same("c2Len/sp", &a, (c.c2Len)(a), (r.c2Len)(a)) };
        }
    }
}

/// row 09 — `c2Neg` / `c2Skew` / `c2CCW90`
#[test]
fn b09_c2Neg_Skew_CCW90() {
    let (c, r) = apis();
    let mut g = Rng::new(0x09);
    for _ in 0..20_000 {
        let a = g.v_mixed(200.0);
        unsafe {
            assert_same("c2Neg", &a, (c.c2Neg)(a), (r.c2Neg)(a));
            assert_same("c2Skew", &a, (c.c2Skew)(a), (r.c2Skew)(a));
            assert_same("c2CCW90", &a, (c.c2CCW90)(a), (r.c2CCW90)(a));
        }
    }
    for &x in &[0.0f32, -0.0, f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for &y in &[0.0f32, -0.0, f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let a = c2v { x, y };
            unsafe {
                assert_same("c2Neg/0", &a, (c.c2Neg)(a), (r.c2Neg)(a));
                assert_same("c2Skew/0", &a, (c.c2Skew)(a), (r.c2Skew)(a));
                assert_same("c2CCW90/0", &a, (c.c2CCW90)(a), (r.c2CCW90)(a));
            }
        }
    }
}

/// row 10 — `c2Div`
#[test]
fn b10_c2Div() {
    let (c, r) = apis();
    let mut g = Rng::new(0x0A);
    for _ in 0..20_000 {
        let a = g.v_mixed(200.0);
        let b = g.f32_mixed(200.0);
        unsafe { assert_same("c2Div", &(a, b), (c.c2Div)(a, b), (r.c2Div)(a, b)) };
    }
    let sp = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE,
        f32::MAX,
    ];
    for &b in &sp {
        for &x in &sp {
            for &y in &sp {
                let a = c2v { x, y };
                unsafe { assert_same("c2Div/sp", &(a, b), (c.c2Div)(a, b), (r.c2Div)(a, b)) };
            }
        }
    }
}

/// row 11 — `c2Norm`
#[test]
fn b11_c2Norm() {
    let (c, r) = apis();
    let mut g = Rng::new(0x0B);
    for _ in 0..20_000 {
        let a = g.v_mixed(200.0);
        unsafe { assert_same("c2Norm", &a, (c.c2Norm)(a), (r.c2Norm)(a)) };
    }
    let sp = [
        0.0f32,
        -0.0,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        3.0,
        -4.0,
        1e30,
        1e-30,
    ];
    for &x in &sp {
        for &y in &sp {
            let a = c2v { x, y };
            unsafe { assert_same("c2Norm/sp", &a, (c.c2Norm)(a), (r.c2Norm)(a)) };
        }
    }
}

/// row 12 — `c2RotIdentity` / `c2xIdentity`
#[test]
fn b12_identities() {
    let (c, r) = apis();
    unsafe {
        assert_same("c2RotIdentity", &(), (c.c2RotIdentity)(), (r.c2RotIdentity)());
        assert_same("c2xIdentity", &(), (c.c2xIdentity)(), (r.c2xIdentity)());
    }
}

/// row 13 — `c2Mulrv` / `c2MulrvT`
#[test]
fn b13_c2Mulrv_c2MulrvT() {
    let (c, r) = apis();
    let mut g = Rng::new(0x0D);
    for _ in 0..20_000 {
        let rot = g.r_geom();
        let v = g.v_mixed(100.0);
        unsafe {
            assert_same("c2Mulrv", &(rot, v), (c.c2Mulrv)(rot, v), (r.c2Mulrv)(rot, v));
            assert_same(
                "c2MulrvT",
                &(rot, v),
                (c.c2MulrvT)(rot, v),
                (r.c2MulrvT)(rot, v),
            );
        }
    }
    // extreme rotation components
    for _ in 0..20_000 {
        let rot = c2r {
            c: g.f32_mixed(4.0),
            s: g.f32_mixed(4.0),
        };
        let v = g.v_mixed(100.0);
        unsafe {
            assert_same("c2Mulrv/x", &(rot, v), (c.c2Mulrv)(rot, v), (r.c2Mulrv)(rot, v));
            assert_same(
                "c2MulrvT/x",
                &(rot, v),
                (c.c2MulrvT)(rot, v),
                (r.c2MulrvT)(rot, v),
            );
        }
    }
}

/// row 14 — `c2Mulxv`
#[test]
fn b14_c2Mulxv() {
    let (c, r) = apis();
    let mut g = Rng::new(0x0E);
    for _ in 0..20_000 {
        let x = g.x_geom();
        let v = g.v_mixed(100.0);
        unsafe { assert_same("c2Mulxv", &(x, v), (c.c2Mulxv)(x, v), (r.c2Mulxv)(x, v)) };
    }
    for _ in 0..10_000 {
        let x = c2x {
            p: g.v_mixed(100.0),
            r: c2r {
                c: g.f32_mixed(2.0),
                s: g.f32_mixed(2.0),
            },
        };
        let v = g.v_mixed(100.0);
        unsafe { assert_same("c2Mulxv/x", &(x, v), (c.c2Mulxv)(x, v), (r.c2Mulxv)(x, v)) };
    }
}

// ===========================================================================
// Group 2 — proxy construction (rows 15..18)
// ===========================================================================

const POISON_V: c2v = c2v {
    x: 1.234_567_8e-11,
    y: -9.876_543e12,
};

fn poisoned_proxy() -> c2Proxy {
    c2Proxy {
        radius: -4.242e7,
        count: -777,
        verts: [POISON_V; 8],
    }
}

/// row 15 — `c2BBVerts`
#[test]
fn b15_c2BBVerts() {
    let (c, r) = apis();
    let mut g = Rng::new(0x0F);
    for i in 0..4000 {
        let mut bbc = match i % 4 {
            0 => g.aabb(60.0),
            1 => {
                let p = g.v_geom(60.0);
                c2AABB { min: p, max: p } // degenerate point
            }
            2 => {
                let p = g.v_geom(60.0);
                c2AABB {
                    min: p,
                    max: c2v { x: p.x, y: p.y + 3.0 }, // zero-thickness slab
                }
            }
            _ => c2AABB {
                min: g.v_mixed(60.0),
                max: g.v_mixed(60.0),
            },
        };
        let mut bbr = bbc;
        let mut oc = [POISON_V; 6];
        let mut or_ = [POISON_V; 6];
        unsafe {
            (c.c2BBVerts)(oc.as_mut_ptr(), &mut bbc);
            (r.c2BBVerts)(or_.as_mut_ptr(), &mut bbr);
        }
        assert_same("c2BBVerts", &bbc, oc.to_vec(), or_.to_vec());
        // the input struct must not be modified either
        assert_same("c2BBVerts/in", &bbc, bbc, bbr);
    }
}

fn diff_make_proxy(what: &str, shape: &Shape, ty: C2_TYPE) {
    let (c, r) = apis();
    let mut pc = poisoned_proxy();
    let mut pr = poisoned_proxy();
    unsafe {
        (c.c2MakeProxy)(shape.ptr(), ty, &mut pc);
        (r.c2MakeProxy)(shape.ptr(), ty, &mut pr);
    }
    assert_same(what, &(shape, ty), pc, pr);
}

/// row 16 — `c2MakeProxy(CIRCLE)`
#[test]
fn b16_makeproxy_circle() {
    let mut g = Rng::new(0x10);
    for i in 0..2000 {
        let mut cc = g.circle(60.0);
        match i % 5 {
            0 => cc.r = 0.0,
            1 => cc.r = f32::INFINITY,
            2 => cc.r = f32::NAN,
            3 => cc.p = c2v { x: -0.0, y: 0.0 },
            _ => {}
        }
        diff_make_proxy("c2MakeProxy/circle", &Shape::Circle(cc), C2_TYPE_CIRCLE);
    }
}

/// row 17 — `c2MakeProxy(AABB)`
#[test]
fn b17_makeproxy_aabb() {
    let mut g = Rng::new(0x11);
    for i in 0..2000 {
        let bb = match i % 3 {
            0 => g.aabb(60.0),
            1 => {
                let p = g.v_geom(60.0);
                c2AABB { min: p, max: p }
            }
            _ => c2AABB {
                min: g.v_mixed(60.0),
                max: g.v_mixed(60.0),
            },
        };
        diff_make_proxy("c2MakeProxy/aabb", &Shape::Aabb(bb), C2_TYPE_AABB);
    }
}

/// row 18 — `c2MakeProxy(CAPSULE)`
#[test]
fn b18_makeproxy_capsule() {
    let mut g = Rng::new(0x12);
    for i in 0..2000 {
        let mut cp = g.capsule(60.0);
        match i % 4 {
            0 => cp.r = 0.0,
            1 => cp.b = cp.a,
            2 => cp.r = f32::NAN,
            _ => {}
        }
        diff_make_proxy("c2MakeProxy/capsule", &Shape::Capsule(cp), C2_TYPE_CAPSULE);
    }
}

// ===========================================================================
// Group 3 — simplex machinery (rows 19..34)
// ===========================================================================

/// `div` values the code distinguishes (row 27 / 33 / 34).
const DIVS: [f32; 8] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    3.5,
    f32::INFINITY,
    f32::NAN,
    f32::MIN_POSITIVE,
];

/// rows 19,20,21 — `c2GJKSimplexMetric` for count 1 / 2 / 3
#[test]
fn b19_simplexmetric_by_count() {
    let (c, r) = apis();
    let mut g = Rng::new(0x13);
    for count in [1i32, 2, 3] {
        for i in 0..8000 {
            let mut s = g.simplex(count, 40.0);
            // regularly force degenerate configurations
            match i % 6 {
                0 => s.verts[1].p = s.verts[0].p,          // zero-length edge
                1 => s.verts[2].p = s.verts[0].p,          // duplicated vertex
                2 => {
                    // collinear triple => det2 == 0
                    let d = c2v {
                        x: s.verts[1].p.x - s.verts[0].p.x,
                        y: s.verts[1].p.y - s.verts[0].p.y,
                    };
                    s.verts[2].p = c2v {
                        x: s.verts[0].p.x + 2.0 * d.x,
                        y: s.verts[0].p.y + 2.0 * d.y,
                    };
                }
                3 => s.verts[0].p = c2v { x: f32::NAN, y: 0.0 },
                4 => s.verts[1].p = c2v {
                    x: f32::INFINITY,
                    y: f32::NEG_INFINITY,
                },
                _ => {}
            }
            let (mut sc, mut sr) = (s, s);
            let (vc, vr) = unsafe { ((c.c2GJKSimplexMetric)(&mut sc), (r.c2GJKSimplexMetric)(&mut sr)) };
            assert_same("c2GJKSimplexMetric", &s, (vc, sc), (vr, sr));
        }
    }
}

fn diff_c22(what: &str, s: c2Simplex) {
    let (c, r) = apis();
    let (mut sc, mut sr) = (s, s);
    unsafe {
        (c.c22)(&mut sc);
        (r.c22)(&mut sr);
    }
    assert_same(what, &s, sc, sr);
}

/// row 22 — `c22` on the integer grid (exercises `u == 0`, `v == 0`, `u == v`)
#[test]
fn b22_c22_grid() {
    let mut g = Rng::new(0x16);
    for i in 0..20_000 {
        let mut s = c2Simplex {
            verts: [g.sv(4.0), g.sv(4.0), g.sv(4.0), g.sv(4.0)],
            div: 1.0,
            count: 2,
        };
        s.verts[0].p = c2v {
            x: g.f32_grid(3),
            y: g.f32_grid(3),
        };
        s.verts[1].p = if i % 7 == 0 {
            s.verts[0].p // a == b => u == v == 0
        } else {
            c2v {
                x: g.f32_grid(3),
                y: g.f32_grid(3),
            }
        };
        diff_c22("c22/grid", s);
    }
}

/// row 23 — `c22` with continuous / extreme coordinates
#[test]
fn b23_c22_wide() {
    let mut g = Rng::new(0x17);
    for _ in 0..20_000 {
        let mut s = g.simplex(2, 100.0);
        s.verts[0].p = g.v_mixed(1e3);
        s.verts[1].p = g.v_mixed(1e3);
        diff_c22("c22/wide", s);
    }
    // pure specials
    let sp = [0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN, f32::MAX];
    let mut s = g.simplex(2, 10.0);
    for &ax in &sp {
        for &ay in &sp {
            for &bx in &sp {
                for &by in &sp {
                    s.verts[0].p = c2v { x: ax, y: ay };
                    s.verts[1].p = c2v { x: bx, y: by };
                    diff_c22("c22/sp", s);
                }
            }
        }
    }
}

fn diff_c23(what: &str, s: c2Simplex) {
    let (c, r) = apis();
    let (mut sc, mut sr) = (s, s);
    unsafe {
        (c.c23)(&mut sc);
        (r.c23)(&mut sr);
    }
    assert_same(what, &s, sc, sr);
}

/// row 24 — `c23` on the integer grid (all 7 branches + `area == 0`)
#[test]
fn b24_c23_grid() {
    let mut g = Rng::new(0x18);
    for i in 0..20_000 {
        let mut s = c2Simplex {
            verts: [g.sv(4.0), g.sv(4.0), g.sv(4.0), g.sv(4.0)],
            div: 1.0,
            count: 3,
        };
        s.verts[0].p = c2v { x: g.f32_grid(3), y: g.f32_grid(3) };
        s.verts[1].p = c2v { x: g.f32_grid(3), y: g.f32_grid(3) };
        s.verts[2].p = match i % 9 {
            0 => s.verts[0].p,           // duplicate a
            1 => s.verts[1].p,           // duplicate b
            2 => {
                // collinear => area == 0
                let d = c2v {
                    x: s.verts[1].p.x - s.verts[0].p.x,
                    y: s.verts[1].p.y - s.verts[0].p.y,
                };
                c2v {
                    x: s.verts[0].p.x + 3.0 * d.x,
                    y: s.verts[0].p.y + 3.0 * d.y,
                }
            }
            _ => c2v { x: g.f32_grid(3), y: g.f32_grid(3) },
        };
        diff_c23("c23/grid", s);
    }
}

/// row 25 — `c23` with continuous / extreme coordinates
#[test]
fn b25_c23_wide() {
    let mut g = Rng::new(0x19);
    for _ in 0..20_000 {
        let mut s = g.simplex(3, 100.0);
        s.verts[0].p = g.v_mixed(1e3);
        s.verts[1].p = g.v_mixed(1e3);
        s.verts[2].p = g.v_mixed(1e3);
        diff_c23("c23/wide", s);
    }
    // triangles containing the origin, and triangles far from it
    for _ in 0..20_000 {
        let mut s = g.simplex(3, 100.0);
        let k = if g.next_u32() & 1 == 0 { 1.0 } else { 40.0 };
        s.verts[0].p = c2v { x: -k + g.f32_range(0.5), y: -k + g.f32_range(0.5) };
        s.verts[1].p = c2v { x: k + g.f32_range(0.5), y: -k + g.f32_range(0.5) };
        s.verts[2].p = c2v { x: g.f32_range(0.5), y: k + g.f32_range(0.5) };
        diff_c23("c23/origin", s);
    }
}

/// row 26 — `c2D` for count 1 / 2 (both det sub-branches) / 3
#[test]
fn b26_c2D_by_count() {
    let (c, r) = apis();
    let mut g = Rng::new(0x1A);
    for count in [1i32, 2, 3] {
        for i in 0..8000 {
            let mut s = g.simplex(count, 40.0);
            if i % 5 == 0 {
                // det == 0 exactly: origin on the AB line
                s.verts[0].p = c2v { x: g.f32_grid(3), y: g.f32_grid(3) };
                s.verts[1].p = c2v {
                    x: s.verts[0].p.x * 2.0,
                    y: s.verts[0].p.y * 2.0,
                };
            } else if i % 5 == 1 {
                s.verts[0].p = g.v_mixed(1e3);
                s.verts[1].p = g.v_mixed(1e3);
            }
            let (mut sc, mut sr) = (s, s);
            let (vc, vr) = unsafe { ((c.c2D)(&mut sc), (r.c2D)(&mut sr)) };
            assert_same("c2D", &s, (vc, sc), (vr, sr));
        }
    }
}

/// row 27 — `c2L` for count 1 / 2 across the whole `div` sweep
#[test]
fn b27_c2L_by_count_and_div() {
    let (c, r) = apis();
    let mut g = Rng::new(0x1B);
    for count in [1i32, 2] {
        for &div in DIVS.iter() {
            for _ in 0..2000 {
                let mut s = g.simplex(count, 40.0);
                s.div = div;
                if g.below(4) == 0 {
                    s.verts[0].u = 0.0;
                }
                if g.below(4) == 0 {
                    s.verts[1].u = -0.0;
                }
                let (mut sc, mut sr) = (s, s);
                let (vc, vr) = unsafe { ((c.c2L)(&mut sc), (r.c2L)(&mut sr)) };
                assert_same("c2L", &s, (vc, sc), (vr, sr));
            }
        }
    }
}

/// rows 28..31 — `c2Support` with count 1 / 2 / 4 / 8 (+ ties and NaN dots)
#[test]
fn b28_support_by_count() {
    let (c, r) = apis();
    let mut g = Rng::new(0x1C);
    for count in [1i32, 2, 4, 8] {
        for i in 0..6000 {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = match i % 5 {
                    0 => g.v_geom(30.0),
                    1 => c2v { x: g.f32_grid(2), y: g.f32_grid(2) }, // ties are frequent
                    2 => g.v_mixed(30.0),
                    3 => c2v { x: 0.0, y: 0.0 },                     // all dots identical
                    _ => g.v_geom(1e3),
                };
            }
            let d = match i % 4 {
                0 => g.v_geom(10.0),
                1 => c2v { x: 0.0, y: 0.0 },                          // every dot == 0
                2 => c2v { x: f32::NAN, y: g.f32_geom(3.0) },          // every dot NaN
                _ => g.v_mixed(10.0),
            };
            let (vc, vr) = unsafe {
                (
                    (c.c2Support)(verts.as_ptr(), count, d),
                    (r.c2Support)(verts.as_ptr(), count, d),
                )
            };
            assert_same("c2Support", &(verts.to_vec(), count as i32, d), vc, vr);
        }
    }
}

/// rows 32,33,34 — `c2Witness` for count 1 / 2 / 3 across the `div` sweep
#[test]
fn b32_witness_by_count() {
    let (c, r) = apis();
    let mut g = Rng::new(0x1D);
    for count in [1i32, 2, 3] {
        for &div in DIVS.iter() {
            for i in 0..1500 {
                let mut s = g.simplex(count, 40.0);
                s.div = div;
                if i % 4 == 0 {
                    for v in s.verts.iter_mut() {
                        v.u = 0.0;
                    }
                } else if i % 4 == 1 {
                    for v in s.verts.iter_mut() {
                        v.u = g.f32_mixed(5.0);
                    }
                }
                let (mut sc, mut sr) = (s, s);
                let mut ac = POISON_V;
                let mut bc = POISON_V;
                let mut ar = POISON_V;
                let mut br = POISON_V;
                unsafe {
                    (c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (r.c2Witness)(&mut sr, &mut ar, &mut br);
                }
                assert_same("c2Witness", &s, (ac, bc, sc), (ar, br, sr));
            }
        }
    }
}

// ===========================================================================
// Group 4 — `c2GJK`, the low-level composed pipeline (rows 35..53)
// ===========================================================================

fn gen_shape(g: &mut Rng, ty: C2_TYPE, centre: c2v, mag: f32) -> Shape {
    let off = |g: &mut Rng| c2v {
        x: centre.x + g.f32_geom(mag),
        y: centre.y + g.f32_geom(mag),
    };
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
            p: centre,
            r: g.f32_geom(mag).abs(),
        }),
        C2_TYPE_AABB => {
            let a = off(g);
            let b = off(g);
            Shape::Aabb(c2AABB {
                min: c2v { x: a.x.min(b.x), y: a.y.min(b.y) },
                max: c2v { x: a.x.max(b.x), y: a.y.max(b.y) },
            })
        }
        _ => {
            let a = off(g);
            let b = if g.below(8) == 0 { a } else { off(g) };
            Shape::Capsule(c2Capsule {
                a,
                b,
                r: g.f32_geom(mag).abs(),
            })
        }
    }
}

/// `gen_shape` with a randomly drawn centre (avoids a double `&mut g` borrow).
fn gen_shape_rand(g: &mut Rng, ty: C2_TYPE, centre_mag: f32, mag: f32) -> Shape {
    let centre = g.v_geom(centre_mag);
    gen_shape(g, ty, centre, mag)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Xf {
    Null,
    Identity,
    Translation,
    Rotation,
    RotTrans,
    NonNormalised,
}

fn make_xf(g: &mut Rng, k: Xf) -> Option<c2x> {
    let id = c2r { c: 1.0, s: 0.0 };
    match k {
        Xf::Null => None,
        Xf::Identity => Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: id }),
        Xf::Translation => Some(c2x { p: g.v_geom(40.0), r: id }),
        Xf::Rotation => {
            let a = g.f32_range(3.2);
            Some(c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: c2r { c: a.cos(), s: a.sin() },
            })
        }
        Xf::RotTrans => {
            let a = g.f32_range(3.2);
            Some(c2x {
                p: g.v_geom(40.0),
                r: c2r { c: a.cos(), s: a.sin() },
            })
        }
        Xf::NonNormalised => Some(c2x {
            p: g.v_geom(40.0),
            r: c2r { c: g.f32_range(3.0), s: g.f32_range(3.0) },
        }),
    }
}

/// Drive one `c2GJK` configuration across every `typeA × typeB` pair with
/// randomized shapes and placements.
fn gjk_sweep(seed: u64, iters: usize, xa: Xf, xb: Xf, use_radius: i32, label: &str) {
    let (c, r) = apis();
    let mut g = Rng::new(seed);
    let mut max_iter = -1i32;
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..iters {
                // placement: sometimes coincident, sometimes near, sometimes far
                let (ca, cb) = match i % 5 {
                    0 => (c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }),
                    1 => (g.v_geom(10.0), g.v_geom(10.0)),
                    2 => (g.v_geom(60.0), g.v_geom(60.0)),
                    3 => (g.v_geom(200.0), g.v_geom(200.0)),
                    _ => {
                        let p = g.v_geom(30.0);
                        (p, p)
                    }
                };
                let mag = if i % 3 == 0 { 4.0 } else { 20.0 };
                let sa = gen_shape(&mut g, ta, ca, mag);
                let sb = gen_shape(&mut g, tb, cb, mag);
                let ax = make_xf(&mut g, xa);
                let bx = make_xf(&mut g, xb);
                let oc = call_gjk(c, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, OutSel::ALL, None);
                let or_ = call_gjk(r, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, OutSel::ALL, None);
                max_iter = max_iter.max(oc.iterations.unwrap());
                assert_same(label, &(sa, sb, ax, bx, use_radius), oc, or_);
            }
        }
    }
    // The C reads `verts[count-1].u` uninitialised only if the loop leaves via
    // the `iter < 20` condition; assert that never happened in this sweep.
    assert!(
        max_iter < 20,
        "{label}: reached the 20-iteration cap (max_iter={max_iter}) — the C \
         then reads an uninitialised `u`, so the comparison would be meaningless"
    );
}

/// row 35 — all 9 type pairs, no transforms, `use_radius = 1`
#[test]
fn b35_gjk_all_type_pairs_no_xform() {
    gjk_sweep(0x35, 3000, Xf::Null, Xf::Null, 1, "gjk/plain");
}

/// row 36 — all 9 type pairs, `use_radius = 0`
#[test]
fn b36_gjk_all_type_pairs_no_radius() {
    gjk_sweep(0x36, 3000, Xf::Null, Xf::Null, 0, "gjk/no-radius");
}

/// row 37 — `ax` non-NULL (identity content), `bx` NULL
#[test]
fn b37_gjk_ax_only() {
    gjk_sweep(0x37, 1200, Xf::Identity, Xf::Null, 1, "gjk/ax-id");
}

/// row 38 — `ax` NULL, `bx` non-NULL (identity content)
#[test]
fn b38_gjk_bx_only() {
    gjk_sweep(0x38, 1200, Xf::Null, Xf::Identity, 1, "gjk/bx-id");
}

/// row 39 — both transforms, pure translation
#[test]
fn b39_gjk_translation() {
    gjk_sweep(0x39, 1200, Xf::Translation, Xf::Translation, 1, "gjk/trans");
    gjk_sweep(0x3901, 600, Xf::Translation, Xf::Translation, 0, "gjk/trans/nr");
}

/// row 40 — both transforms, pure rotation
#[test]
fn b40_gjk_rotation() {
    gjk_sweep(0x40, 1200, Xf::Rotation, Xf::Rotation, 1, "gjk/rot");
    gjk_sweep(0x4001, 600, Xf::Rotation, Xf::Rotation, 0, "gjk/rot/nr");
}

/// row 41 — both transforms, rotation + translation
#[test]
fn b41_gjk_rot_trans() {
    gjk_sweep(0x41, 1200, Xf::RotTrans, Xf::RotTrans, 1, "gjk/rt");
    gjk_sweep(0x4101, 600, Xf::RotTrans, Xf::RotTrans, 0, "gjk/rt/nr");
}

/// row 42 — non-normalised `c2r` (scale / shear)
#[test]
fn b42_gjk_non_normalised_rot() {
    gjk_sweep(0x42, 1200, Xf::NonNormalised, Xf::NonNormalised, 1, "gjk/nn");
    gjk_sweep(0x4201, 600, Xf::NonNormalised, Xf::Rotation, 0, "gjk/nn/mixed");
}

/// row 43 — cache non-NULL but zeroed (cold start + write-back)
#[test]
fn b43_gjk_cache_cold() {
    let (c, r) = apis();
    let mut g = Rng::new(0x43);
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..2000 {
                let sa = gen_shape_rand(&mut g, ta, 40.0, 15.0);
                let sb = gen_shape_rand(&mut g, tb, 40.0, 15.0);
                let ur = (i % 2) as i32;
                let cold = c2GJKCache::default();
                let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, Some(cold));
                let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, Some(cold));
                assert_same("gjk/cache-cold", &(sa, sb, ur), oc, or_);
            }
        }
    }
}

/// Run a sequence of queries on one side, carrying the cache forward.
fn gjk_sequence(api: &Api, frames: &[(Shape, Shape)], use_radius: i32) -> Vec<GjkOut> {
    let mut cache = c2GJKCache::default();
    let mut out = Vec::with_capacity(frames.len());
    for (a, b) in frames {
        let o = call_gjk(api, a, None, b, None, use_radius, OutSel::ALL, Some(cache));
        cache = o.cache.unwrap();
        out.push(o);
    }
    out
}

/// row 44 — cache primed then re-queried with the *same* shapes
#[test]
fn b44_gjk_cache_warm_same() {
    let (c, r) = apis();
    let mut g = Rng::new(0x44);
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..800 {
                let sa = gen_shape_rand(&mut g, ta, 40.0, 15.0);
                let sb = gen_shape_rand(&mut g, tb, 40.0, 15.0);
                let ur = (i % 2) as i32;
                let frames = vec![(sa, sb), (sa, sb), (sa, sb), (sa, sb)];
                let oc = gjk_sequence(c, &frames, ur);
                let or_ = gjk_sequence(r, &frames, ur);
                assert_same("gjk/cache-warm-same", &(sa, sb, ur), oc, or_);
            }
        }
    }
}

/// row 45 — cache primed then re-queried with moved shapes (8 frames)
#[test]
fn b45_gjk_cache_warm_moved_sequence() {
    let (c, r) = apis();
    let mut g = Rng::new(0x45);
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..500 {
                let ur = (i % 2) as i32;
                // A drifts towards / past B, so the query goes
                // separated -> touching -> overlapping -> separated again.
                let mut frames = Vec::new();
                let base_a = g.v_geom(50.0);
                let base_b = g.v_geom(50.0);
                for f in 0..8 {
                    let t = f as f32 * 0.25;
                    let ca = c2v {
                        x: base_a.x + t * (base_b.x - base_a.x) * 2.0,
                        y: base_a.y + t * (base_b.y - base_a.y) * 2.0,
                    };
                    frames.push((
                        gen_shape(&mut g, ta, ca, 12.0),
                        gen_shape(&mut g, tb, base_b, 12.0),
                    ));
                }
                let oc = gjk_sequence(c, &frames, ur);
                let or_ = gjk_sequence(r, &frames, ur);
                assert_same("gjk/cache-warm-moved", &(ta, tb, ur, frames.len() as u32), oc, or_);
            }
        }
    }
}

/// row 46 — cache primed, then the shape type changes to one with **at least as
/// many** proxy vertices (so every cached index stays inside `verts[0..count)`).
/// Proxy vertex counts: circle 1, capsule 2, aabb 4.
#[test]
fn b46_gjk_cache_warm_type_switch() {
    let (c, r) = apis();
    let mut g = Rng::new(0x46);
    let vcount = |t: C2_TYPE| match t {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_CAPSULE => 2,
        _ => 4,
    };
    for &t1a in C2_TYPES.iter() {
        for &t1b in C2_TYPES.iter() {
            for &t2a in C2_TYPES.iter() {
                for &t2b in C2_TYPES.iter() {
                    if vcount(t2a) < vcount(t1a) || vcount(t2b) < vcount(t1b) {
                        continue; // would read uninitialised proxy verts in the C
                    }
                    for i in 0..120 {
                        let ur = (i % 2) as i32;
                        let frames = vec![
                            (
                                gen_shape_rand(&mut g, t1a, 40.0, 14.0),
                                gen_shape_rand(&mut g, t1b, 40.0, 14.0),
                            ),
                            (
                                gen_shape_rand(&mut g, t2a, 40.0, 14.0),
                                gen_shape_rand(&mut g, t2b, 40.0, 14.0),
                            ),
                            (
                                gen_shape_rand(&mut g, t2a, 40.0, 14.0),
                                gen_shape_rand(&mut g, t2b, 40.0, 14.0),
                            ),
                        ];
                        let oc = gjk_sequence(c, &frames, ur);
                        let or_ = gjk_sequence(r, &frames, ur);
                        assert_same(
                            "gjk/cache-type-switch",
                            &(t1a, t1b, t2a, t2b, ur),
                            oc,
                            or_,
                        );
                    }
                }
            }
        }
    }
}

/// row 47 — hand-crafted caches: `count` × `metric` × `div` × every valid index
#[test]
fn b47_gjk_cache_handcrafted() {
    let (c, r) = apis();
    let mut g = Rng::new(0x47);
    let vcount = |t: C2_TYPE| match t {
        C2_TYPE_CIRCLE => 1i32,
        C2_TYPE_CAPSULE => 2,
        _ => 4,
    };
    let metrics = [0.0f32, -0.0, 1.0, 1.0e9, -1.0e9, -1.0e30, f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    let divs = [0.0f32, 1.0, -1.0, 7.5, f32::NAN];
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            let (na, nb) = (vcount(ta), vcount(tb));
            for count in 1i32..=3 {
                for &metric in metrics.iter() {
                    for &div in divs.iter() {
                        for _ in 0..12 {
                            let sa = gen_shape_rand(&mut g, ta, 40.0, 14.0);
                            let sb = gen_shape_rand(&mut g, tb, 40.0, 14.0);
                            let mut cache = c2GJKCache {
                                metric,
                                count,
                                iA: [0; 3],
                                iB: [0; 3],
                                div,
                            };
                            for k in 0..3 {
                                cache.iA[k] = g.below(na as u32) as i32;
                                cache.iB[k] = g.below(nb as u32) as i32;
                            }
                            let ur = (g.next_u32() & 1) as i32;
                            let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, Some(cache));
                            let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, Some(cache));
                            assert_same("gjk/cache-crafted", &(sa, sb, cache, ur), oc, or_);
                        }
                    }
                }
            }
        }
    }
}

/// row 48 — all 8 out-param NULL combinations × cache NULL/non-NULL
#[test]
fn b48_gjk_out_param_matrix() {
    let (c, r) = apis();
    let mut g = Rng::new(0x48);
    for mask in 0u32..8 {
        let sel = OutSel {
            a: mask & 1 != 0,
            b: mask & 2 != 0,
            iters: mask & 4 != 0,
        };
        for with_cache in [false, true] {
            for &ta in C2_TYPES.iter() {
                for &tb in C2_TYPES.iter() {
                    for i in 0..150 {
                        let sa = gen_shape_rand(&mut g, ta, 40.0, 14.0);
                        let sb = gen_shape_rand(&mut g, tb, 40.0, 14.0);
                        let ur = (i % 2) as i32;
                        let cache = if with_cache {
                            Some(c2GJKCache::default())
                        } else {
                            None
                        };
                        let oc = call_gjk(c, &sa, None, &sb, None, ur, sel, cache);
                        let or_ = call_gjk(r, &sa, None, &sb, None, ur, sel, cache);
                        assert_same("gjk/outsel", &(sa, sb, mask, with_cache as u32, ur), oc, or_);
                    }
                }
            }
        }
    }
}

/// row 49 — A and B identical (degenerate search direction, `c2Norm((0,0))`)
#[test]
fn b49_gjk_identical_shapes() {
    let (c, r) = apis();
    let mut g = Rng::new(0x49);
    for &ta in C2_TYPES.iter() {
        for i in 0..4000 {
            let s = gen_shape_rand(&mut g, ta, 40.0, 15.0);
            let ur = (i % 2) as i32;
            let oc = call_gjk(c, &s, None, &s, None, ur, OutSel::ALL, None);
            let or_ = call_gjk(r, &s, None, &s, None, ur, OutSel::ALL, None);
            assert_same("gjk/identical", &(s, ur), oc, or_);
            // and with a cache, which makes the second query warm-start from a
            // fully-degenerate simplex
            let cold = c2GJKCache::default();
            let f = vec![(s, s), (s, s), (s, s)];
            assert_same(
                "gjk/identical-cached",
                &(s, ur),
                gjk_sequence(c, &f, ur),
                gjk_sequence(r, &f, ur),
            );
            let _ = cold;
        }
    }
}

/// row 50 — zero-sized shapes
#[test]
fn b50_gjk_zero_sized_shapes() {
    let (c, r) = apis();
    let mut g = Rng::new(0x50);
    for i in 0..8000 {
        let p = g.v_geom(40.0);
        let q = g.v_geom(40.0);
        let shapes = [
            Shape::Circle(c2Circle { p, r: 0.0 }),
            Shape::Aabb(c2AABB { min: p, max: p }),
            Shape::Capsule(c2Capsule { a: p, b: p, r: 0.0 }),
            Shape::Circle(c2Circle { p: q, r: 0.0 }),
            Shape::Aabb(c2AABB { min: q, max: q }),
            Shape::Capsule(c2Capsule { a: q, b: q, r: 0.0 }),
        ];
        let ur = (i % 2) as i32;
        for sa in shapes.iter() {
            for sb in shapes.iter() {
                let oc = call_gjk(c, sa, None, sb, None, ur, OutSel::ALL, None);
                let or_ = call_gjk(r, sa, None, sb, None, ur, OutSel::ALL, None);
                assert_same("gjk/zero-sized", &(*sa, *sb, ur), oc, or_);
            }
        }
    }
}

/// row 51 — placements that make `dist` land exactly on `rA + rB`
#[test]
fn b51_gjk_tangent_placements() {
    let (c, r) = apis();
    let mut g = Rng::new(0x51);
    for i in 0..20_000 {
        // integer radii and integer separations => exact tangency is hit often
        let ra = g.f32_grid(4).abs();
        let rb = g.f32_grid(4).abs();
        let d = ra + rb + g.f32_grid(2); // straddles the `dist > rA+rB` test
        let a = Shape::Circle(c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: ra,
        });
        let b = Shape::Circle(c2Circle {
            p: c2v { x: d, y: 0.0 },
            r: rb,
        });
        let ur = (i % 2) as i32;
        let oc = call_gjk(c, &a, None, &b, None, ur, OutSel::ALL, None);
        let or_ = call_gjk(r, &a, None, &b, None, ur, OutSel::ALL, None);
        assert_same("gjk/tangent-circles", &(a, b, ur), oc, or_);

        // capsule / aabb variants of the same idea
        let ca = Shape::Capsule(c2Capsule {
            a: c2v { x: 0.0, y: -2.0 },
            b: c2v { x: 0.0, y: 2.0 },
            r: ra,
        });
        let cb = Shape::Capsule(c2Capsule {
            a: c2v { x: d, y: -2.0 },
            b: c2v { x: d, y: 2.0 },
            r: rb,
        });
        let oc = call_gjk(c, &ca, None, &cb, None, ur, OutSel::ALL, None);
        let or_ = call_gjk(r, &ca, None, &cb, None, ur, OutSel::ALL, None);
        assert_same("gjk/tangent-capsules", &(ca, cb, ur), oc, or_);

        let ba = Shape::Aabb(c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        });
        let bb = Shape::Aabb(c2AABB {
            min: c2v { x: d, y: -1.0 },
            max: c2v { x: d + 2.0, y: 1.0 },
        });
        let oc = call_gjk(c, &ba, None, &bb, None, ur, OutSel::ALL, None);
        let or_ = call_gjk(r, &ba, None, &bb, None, ur, OutSel::ALL, None);
        assert_same("gjk/tangent-aabbs", &(ba, bb, ur), oc, or_);
    }
}

/// row 52 — huge coordinates (overflow inside `c2Dot` / `c2Len`)
#[test]
fn b52_gjk_huge_coords() {
    let (c, r) = apis();
    let mut g = Rng::new(0x52);
    let scales = [1.0e15f32, 1.0e30, 1.0e35, f32::MAX / 4.0];
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..900 {
                let s = scales[(i % scales.len()) as usize];
                let ca = c2v { x: g.f32_range(1.0) * s, y: g.f32_range(1.0) * s };
                let cb = c2v { x: g.f32_range(1.0) * s, y: g.f32_range(1.0) * s };
                let sa = gen_shape(&mut g, ta, ca, s * 0.25);
                let sb = gen_shape(&mut g, tb, cb, s * 0.25);
                let ur = (i % 2) as i32;
                let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, None);
                let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, None);
                assert_same("gjk/huge", &(sa, sb, ur), oc, or_);
            }
        }
    }
}

/// row 53 — denormal / `FLT_EPSILON`-scale coordinates (underflow, `d1 > d0`)
#[test]
fn b53_gjk_tiny_coords() {
    let (c, r) = apis();
    let mut g = Rng::new(0x53);
    let scales = [
        f32::EPSILON,
        f32::EPSILON * 0.5,
        1.0e-20f32,
        f32::MIN_POSITIVE,
        f32::from_bits(4),
    ];
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..900 {
                let s = scales[(i % scales.len()) as usize];
                let ca = c2v { x: g.f32_range(1.0) * s, y: g.f32_range(1.0) * s };
                let cb = c2v { x: g.f32_range(1.0) * s, y: g.f32_range(1.0) * s };
                let sa = gen_shape(&mut g, ta, ca, s);
                let sb = gen_shape(&mut g, tb, cb, s);
                let ur = (i % 2) as i32;
                let oc = call_gjk(c, &sa, None, &sb, None, ur, OutSel::ALL, None);
                let or_ = call_gjk(r, &sa, None, &sb, None, ur, OutSel::ALL, None);
                assert_same("gjk/tiny", &(sa, sb, ur), oc, or_);
            }
        }
    }
}

// ===========================================================================
// Group 5 — boolean convenience wrappers (rows 54..60)
// ===========================================================================

/// How many GJK iterations the C reference needs for this query.
///
/// If it ever returns 20 the C left the loop through the `iter < 20` condition,
/// in which case it goes on to read the `u` field of the vertex it appended last
/// — a field `c2GJK` never wrote (`c2Simplex s;` is uninitialised at
/// `lib.c:379`).  The C result is then not a function of its inputs, so such
/// cases must be excluded from a differential comparison rather than "fixed".
fn gjk_iters_c(sa: &Shape, sb: &Shape, use_radius: i32) -> i32 {
    let (c, _) = apis();
    call_gjk(c, sa, None, sb, None, use_radius, OutSel::ALL, None)
        .iterations
        .unwrap()
}

/// row 54 — `c2AABBtoAABB`
#[test]
fn b54_aabb_to_aabb() {
    let (c, r) = apis();
    let mut g = Rng::new(0x54);
    for i in 0..50_000 {
        let (a, b) = match i % 4 {
            0 => (g.aabb(40.0), g.aabb(40.0)),
            1 => (
                // integer grid => exact touching happens
                c2AABB { min: c2v { x: g.f32_grid(5), y: g.f32_grid(5) }, max: c2v { x: g.f32_grid(5), y: g.f32_grid(5) } },
                c2AABB { min: c2v { x: g.f32_grid(5), y: g.f32_grid(5) }, max: c2v { x: g.f32_grid(5), y: g.f32_grid(5) } },
            ),
            2 => (
                c2AABB { min: g.v_mixed(40.0), max: g.v_mixed(40.0) },
                c2AABB { min: g.v_mixed(40.0), max: g.v_mixed(40.0) },
            ),
            _ => {
                let p = g.v_geom(30.0);
                (c2AABB { min: p, max: p }, c2AABB { min: p, max: p })
            }
        };
        unsafe {
            assert_same(
                "c2AABBtoAABB",
                &(a, b),
                (c.c2AABBtoAABB)(a, b),
                (r.c2AABBtoAABB)(a, b),
            )
        };
    }
    // exact separating-axis boundaries on each of the four axes
    for axis in 0..4 {
        for d in [-1.0f32, -0.5, 0.0, 0.5, 1.0, 2.0] {
            let a = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
            let b = match axis {
                0 => c2AABB { min: c2v { x: -2.0 + d, y: 0.0 }, max: c2v { x: -1.0 + d, y: 1.0 } },
                1 => c2AABB { min: c2v { x: 1.0 + d, y: 0.0 }, max: c2v { x: 2.0 + d, y: 1.0 } },
                2 => c2AABB { min: c2v { x: 0.0, y: -2.0 + d }, max: c2v { x: 1.0, y: -1.0 + d } },
                _ => c2AABB { min: c2v { x: 0.0, y: 1.0 + d }, max: c2v { x: 1.0, y: 2.0 + d } },
            };
            unsafe {
                assert_same(
                    "c2AABBtoAABB/axis",
                    &(a, b, axis as u32),
                    (c.c2AABBtoAABB)(a, b),
                    (r.c2AABBtoAABB)(a, b),
                )
            };
        }
    }
}

/// row 55 — `c2CircletoCircle`
#[test]
fn b55_circle_to_circle() {
    let (c, r) = apis();
    let mut g = Rng::new(0x55);
    for i in 0..50_000 {
        let (a, b) = match i % 4 {
            0 => (g.circle(40.0), g.circle(40.0)),
            1 => (
                // integer grid: exact tangency (d2 == r2) really occurs
                c2Circle { p: c2v { x: g.f32_grid(5), y: 0.0 }, r: g.f32_grid(3).abs() },
                c2Circle { p: c2v { x: g.f32_grid(5), y: 0.0 }, r: g.f32_grid(3).abs() },
            ),
            2 => (
                c2Circle { p: g.v_mixed(40.0), r: g.f32_mixed(20.0) },
                c2Circle { p: g.v_mixed(40.0), r: g.f32_mixed(20.0) },
            ),
            _ => {
                let p = g.v_geom(30.0);
                (c2Circle { p, r: 0.0 }, c2Circle { p, r: 0.0 })
            }
        };
        unsafe {
            assert_same(
                "c2CircletoCircle",
                &(a, b),
                (c.c2CircletoCircle)(a, b),
                (r.c2CircletoCircle)(a, b),
            )
        };
    }
    // 3-4-5 exact tangency
    for d in [4.0f32, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0] {
        let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 3.0 };
        let b = c2Circle { p: c2v { x: d, y: 0.0 }, r: 4.0 };
        unsafe {
            assert_same(
                "c2CircletoCircle/tangent",
                &(a, b),
                (c.c2CircletoCircle)(a, b),
                (r.c2CircletoCircle)(a, b),
            )
        };
    }
}

/// row 56 — `c2CircletoAABB`
#[test]
fn b56_circle_to_aabb() {
    let (c, r) = apis();
    let mut g = Rng::new(0x56);
    for i in 0..50_000 {
        let (a, b) = match i % 5 {
            0 => (g.circle(40.0), g.aabb(40.0)),
            1 => (
                c2Circle { p: c2v { x: g.f32_grid(5), y: g.f32_grid(5) }, r: g.f32_grid(3).abs() },
                c2AABB { min: c2v { x: g.f32_grid(4), y: g.f32_grid(4) }, max: c2v { x: g.f32_grid(4), y: g.f32_grid(4) } },
            ),
            2 => (
                c2Circle { p: g.v_mixed(40.0), r: g.f32_mixed(20.0) },
                c2AABB { min: g.v_mixed(40.0), max: g.v_mixed(40.0) },
            ),
            3 => {
                // centre exactly on an edge / corner of the box
                let bb = c2AABB { min: c2v { x: -2.0, y: -2.0 }, max: c2v { x: 2.0, y: 2.0 } };
                let p = match g.below(4) {
                    0 => c2v { x: 2.0, y: 0.0 },
                    1 => c2v { x: 2.0, y: 2.0 },
                    2 => c2v { x: -2.0, y: 1.0 },
                    _ => c2v { x: 0.0, y: -2.0 },
                };
                (c2Circle { p, r: g.f32_grid(3).abs() }, bb)
            }
            _ => {
                let p = g.v_geom(30.0);
                (c2Circle { p, r: 0.0 }, c2AABB { min: p, max: p })
            }
        };
        unsafe {
            assert_same(
                "c2CircletoAABB",
                &(a, b),
                (c.c2CircletoAABB)(a, b),
                (r.c2CircletoAABB)(a, b),
            )
        };
    }
}

/// row 57 — `c2CircletoCapsule` (all three `da`/`db` branches + degenerate)
#[test]
fn b57_circle_to_capsule() {
    let (c, r) = apis();
    let mut g = Rng::new(0x57);
    for i in 0..50_000 {
        let (a, b) = match i % 6 {
            0 => (g.circle(40.0), g.capsule(40.0)),
            // before endpoint a  (da < 0)
            1 => (
                c2Circle { p: c2v { x: -5.0 + g.f32_range(2.0), y: g.f32_range(2.0) }, r: g.f32_grid(3).abs() },
                c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: g.f32_grid(3).abs() },
            ),
            // inside the segment (da >= 0, db < 0)
            2 => (
                c2Circle { p: c2v { x: 5.0 + g.f32_range(2.0), y: g.f32_range(4.0) }, r: g.f32_grid(3).abs() },
                c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: g.f32_grid(3).abs() },
            ),
            // past endpoint b (db >= 0)
            3 => (
                c2Circle { p: c2v { x: 15.0 + g.f32_range(2.0), y: g.f32_range(2.0) }, r: g.f32_grid(3).abs() },
                c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: g.f32_grid(3).abs() },
            ),
            // degenerate capsule a == b  => c2Dot(n,n) == 0 => da/0
            4 => {
                let p = g.v_geom(20.0);
                (
                    c2Circle { p: g.v_geom(20.0), r: g.f32_grid(4).abs() },
                    c2Capsule { a: p, b: p, r: g.f32_grid(4).abs() },
                )
            }
            _ => (
                c2Circle { p: g.v_mixed(40.0), r: g.f32_mixed(20.0) },
                c2Capsule { a: g.v_mixed(40.0), b: g.v_mixed(40.0), r: g.f32_mixed(20.0) },
            ),
        };
        unsafe {
            assert_same(
                "c2CircletoCapsule",
                &(a, b),
                (c.c2CircletoCapsule)(a, b),
                (r.c2CircletoCapsule)(a, b),
            )
        };
    }
}

/// row 58 — `c2AABBtoCapsule` (drives the whole GJK pipeline)
#[test]
fn b58_aabb_to_capsule() {
    let (c, r) = apis();
    let mut g = Rng::new(0x58);
    let mut skipped = 0usize;
    for i in 0..20_000 {
        let (a, b) = match i % 4 {
            0 => (g.aabb(40.0), g.capsule(40.0)),
            1 => (
                c2AABB { min: c2v { x: g.f32_grid(4), y: g.f32_grid(4) }, max: c2v { x: g.f32_grid(4), y: g.f32_grid(4) } },
                c2Capsule { a: c2v { x: g.f32_grid(4), y: g.f32_grid(4) }, b: c2v { x: g.f32_grid(4), y: g.f32_grid(4) }, r: g.f32_grid(3).abs() },
            ),
            2 => {
                let p = g.v_geom(20.0);
                (c2AABB { min: p, max: p }, c2Capsule { a: p, b: p, r: 0.0 })
            }
            _ => (
                c2AABB { min: g.v_mixed(40.0), max: g.v_mixed(40.0) },
                c2Capsule { a: g.v_mixed(40.0), b: g.v_mixed(40.0), r: g.f32_mixed(20.0) },
            ),
        };
        if gjk_iters_c(&Shape::Aabb(a), &Shape::Capsule(b), 1) >= 20 {
            skipped += 1;
            continue;
        }
        unsafe {
            assert_same(
                "c2AABBtoCapsule",
                &(a, b),
                (c.c2AABBtoCapsule)(a, b),
                (r.c2AABBtoCapsule)(a, b),
            )
        };
    }
    eprintln!("b58: {skipped} case(s) skipped (C hit the 20-iteration cap => UB)");
}

/// row 59 — `c2CapsuletoCapsule`
#[test]
fn b59_capsule_to_capsule() {
    let (c, r) = apis();
    let mut g = Rng::new(0x59);
    let mut skipped = 0usize;
    for i in 0..20_000 {
        let (a, b) = match i % 6 {
            0 => (g.capsule(40.0), g.capsule(40.0)),
            1 => (
                c2Capsule { a: c2v { x: g.f32_grid(4), y: g.f32_grid(4) }, b: c2v { x: g.f32_grid(4), y: g.f32_grid(4) }, r: g.f32_grid(3).abs() },
                c2Capsule { a: c2v { x: g.f32_grid(4), y: g.f32_grid(4) }, b: c2v { x: g.f32_grid(4), y: g.f32_grid(4) }, r: g.f32_grid(3).abs() },
            ),
            // parallel
            2 => {
                let d = g.f32_grid(4);
                (
                    c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: g.f32_grid(3).abs() },
                    c2Capsule { a: c2v { x: 0.0, y: d }, b: c2v { x: 10.0, y: d }, r: g.f32_grid(3).abs() },
                )
            }
            // crossing
            3 => (
                c2Capsule { a: c2v { x: -5.0, y: 0.0 }, b: c2v { x: 5.0, y: 0.0 }, r: g.f32_grid(2).abs() },
                c2Capsule { a: c2v { x: g.f32_grid(6), y: -5.0 }, b: c2v { x: g.f32_grid(6), y: 5.0 }, r: g.f32_grid(2).abs() },
            ),
            // collinear
            4 => {
                let s = g.f32_grid(6);
                (
                    c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 4.0, y: 0.0 }, r: g.f32_grid(2).abs() },
                    c2Capsule { a: c2v { x: s, y: 0.0 }, b: c2v { x: s + 4.0, y: 0.0 }, r: g.f32_grid(2).abs() },
                )
            }
            _ => (
                c2Capsule { a: g.v_mixed(40.0), b: g.v_mixed(40.0), r: g.f32_mixed(20.0) },
                c2Capsule { a: g.v_mixed(40.0), b: g.v_mixed(40.0), r: g.f32_mixed(20.0) },
            ),
        };
        if gjk_iters_c(&Shape::Capsule(a), &Shape::Capsule(b), 1) >= 20 {
            skipped += 1;
            continue;
        }
        unsafe {
            assert_same(
                "c2CapsuletoCapsule",
                &(a, b),
                (c.c2CapsuletoCapsule)(a, b),
                (r.c2CapsuletoCapsule)(a, b),
            )
        };
    }
    eprintln!("b59: {skipped} case(s) skipped (C hit the 20-iteration cap => UB)");
}

/// row 60 — `c2Collided` over all 9 valid type pairs (incl. the swapping cases)
#[test]
fn b60_collided_all_pairs() {
    let (c, r) = apis();
    let mut g = Rng::new(0x60);
    let mut skipped = 0usize;
    for &ta in C2_TYPES.iter() {
        for &tb in C2_TYPES.iter() {
            for i in 0..5000 {
                let (sa, sb) = match i % 4 {
                    0 => (
                        gen_shape_rand(&mut g, ta, 40.0, 15.0),
                        gen_shape_rand(&mut g, tb, 40.0, 15.0),
                    ),
                    1 => (
                        gen_shape_rand(&mut g, ta, 4.0, 3.0),
                        gen_shape_rand(&mut g, tb, 4.0, 3.0),
                    ),
                    2 => {
                        let p = g.v_geom(20.0);
                        (gen_shape(&mut g, ta, p, 8.0), gen_shape(&mut g, tb, p, 8.0))
                    }
                    _ => (
                        gen_shape_rand(&mut g, ta, 200.0, 60.0),
                        gen_shape_rand(&mut g, tb, 200.0, 60.0),
                    ),
                };
                // the GJK-backed dispatch branches must not hit the UB cap
                let needs_gjk = (ta == C2_TYPE_AABB && tb == C2_TYPE_CAPSULE)
                    || (ta == C2_TYPE_CAPSULE && tb == C2_TYPE_AABB)
                    || (ta == C2_TYPE_CAPSULE && tb == C2_TYPE_CAPSULE);
                if needs_gjk {
                    // c2Collided swaps arguments so that AABB is always A
                    let (pa, pb) = if ta == C2_TYPE_CAPSULE && tb == C2_TYPE_AABB {
                        (sb, sa)
                    } else {
                        (sa, sb)
                    };
                    if gjk_iters_c(&pa, &pb, 1) >= 20 {
                        skipped += 1;
                        continue;
                    }
                }
                let (vc, vr) = unsafe {
                    (
                        (c.c2Collided)(sa.ptr(), ta, sb.ptr(), tb),
                        (r.c2Collided)(sa.ptr(), ta, sb.ptr(), tb),
                    )
                };
                assert_same("c2Collided", &(sa, ta, sb, tb), vc, vr);
            }
        }
    }
    eprintln!("b60: {skipped} case(s) skipped (C hit the 20-iteration cap => UB)");
}

// ===========================================================================
// Group 6 — the public header entry point `aabb` (rows 61..63)
// ===========================================================================

/// The three shapes `aabb()` hard-codes, needed for the UB pre-flight check.
const AABB_CAPSULE: c2Capsule = c2Capsule {
    a: c2v { x: -40.0, y: 40.0 },
    b: c2v { x: -20.0, y: 100.0 },
    r: 10.0,
};

/// `aabb()` funnels the capsule case through `c2AABBtoCapsule(aabb_in, capsule)`
/// -> `c2GJK(&aabb_in, AABB, &capsule, CAPSULE, ..., use_radius = 1)`.
fn aabb_entry_is_defined(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> bool {
    let bb = Shape::Aabb(c2AABB {
        min: c2v { x: min_x, y: min_y },
        max: c2v { x: max_x, y: max_y },
    });
    gjk_iters_c(&bb, &Shape::Capsule(AABB_CAPSULE), 1) < 20
}

fn diff_aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> bool {
    let (c, r) = apis();
    if !aabb_entry_is_defined(min_x, min_y, max_x, max_y) {
        return false;
    }
    let (vc, vr) = unsafe {
        (
            (c.aabb)(min_x, min_y, max_x, max_y),
            (r.aabb)(min_x, min_y, max_x, max_y),
        )
    };
    assert_same("aabb", &(min_x, min_y, max_x, max_y), vc, vr);
    assert!((0..=7).contains(&vc), "aabb returned {vc}, outside 0..=7");
    true
}

/// row 61 — 200k random boxes in the geometrically interesting window
#[test]
fn b61_aabb_entry_random() {
    let mut g = Rng::new(0x61);
    let mut seen = [false; 8];
    let (c, _) = apis();
    let mut skipped = 0usize;
    for _ in 0..200_000 {
        let a = g.f32_range(150.0);
        let b = g.f32_range(150.0);
        let cc = g.f32_range(150.0);
        let d = g.f32_range(150.0);
        // half properly ordered, half raw (inverted boxes included)
        let (min_x, max_x) = if g.next_u32() & 1 == 0 { (a.min(cc), a.max(cc)) } else { (a, cc) };
        let (min_y, max_y) = if g.next_u32() & 1 == 0 { (b.min(d), b.max(d)) } else { (b, d) };
        if !diff_aabb(min_x, min_y, max_x, max_y) {
            skipped += 1;
            continue;
        }
        let v = unsafe { (c.aabb)(min_x, min_y, max_x, max_y) };
        seen[v as usize] = true;
    }
    eprintln!("b61: result bit-masks observed: {seen:?}, {skipped} skipped");
    assert!(seen.iter().filter(|x| **x).count() >= 6, "coverage too low: {seen:?}");
}

/// row 62 — integer grid around the three hard-coded shapes
#[test]
fn b62_aabb_entry_grid() {
    // circle at (-70,0) r=20 ; aabb (-40,-40)..(-15,-15) ; capsule (-40,40)-(-20,100) r=10
    let xs: Vec<f32> = (-110..=10).step_by(5).map(|v| v as f32).collect();
    let ys: Vec<f32> = (-70..=120).step_by(10).map(|v| v as f32).collect();
    let mut n = 0usize;
    let mut skipped = 0usize;
    for wx in [0.0f32, 5.0, 25.0, 60.0] {
        for wy in [0.0f32, 5.0, 25.0, 60.0] {
            for &x in xs.iter() {
                for &y in ys.iter() {
                    if diff_aabb(x, y, x + wx, y + wy) {
                        n += 1;
                    } else {
                        skipped += 1;
                    }
                }
            }
        }
    }
    eprintln!("b62: {n} grid cases, {skipped} skipped");
    assert!(n > 5000);
}

/// row 63 — full special-value sweep (`±0`, `±inf`, `NaN`, `FLT_MAX`, denormals)
#[test]
fn b63_aabb_entry_specials() {
    let sp = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::EPSILON,
        -70.0,
        -40.0,
        -15.0,
        40.0,
        100.0,
    ];
    let mut n = 0usize;
    let mut skipped = 0usize;
    for &a in sp.iter() {
        for &b in sp.iter() {
            for &c in sp.iter() {
                for &d in sp.iter() {
                    if diff_aabb(a, b, c, d) {
                        n += 1;
                    } else {
                        skipped += 1;
                    }
                }
            }
        }
    }
    eprintln!("b63: {n} special cases compared, {skipped} skipped (UB cap)");
    assert!(n > 90_000, "only {n} of {} cases were defined", sp.len().pow(4));
}
