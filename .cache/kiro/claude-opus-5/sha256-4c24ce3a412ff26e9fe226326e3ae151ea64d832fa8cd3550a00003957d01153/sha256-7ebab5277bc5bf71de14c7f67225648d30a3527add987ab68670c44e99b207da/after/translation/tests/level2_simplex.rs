//! Level 2: proxy construction and simplex helpers.
//!
//! Everything here is exercised through the dynamic libraries with identical
//! input buffers; the *whole* output struct is compared field by field so that
//! untouched padding-adjacent fields are covered too.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;
use std::ffi::c_void;

const ITERS: u32 = 20_000;

// ---------------------------------------------------------------------------
// c2BBVerts / c2MakeProxy
// ---------------------------------------------------------------------------

#[test]
fn c2BBVerts_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(101);
    for _ in 0..scaled(ITERS) {
        let mut bb_c = if g.below(3) == 0 {
            c2AABB {
                min: g.v_nasty(),
                max: g.v_nasty(),
            }
        } else {
            g.aabb_any()
        };
        let mut bb_r = bb_c;
        // Sentinel fill so that any missed store shows up as a difference.
        let mut out_c = [c2v { x: -7.5, y: 3.25 }; 8];
        let mut out_r = out_c;
        unsafe {
            (c.c2BBVerts)(out_c.as_mut_ptr(), &mut bb_c);
            (r.c2BBVerts)(out_r.as_mut_ptr(), &mut bb_r);
        }
        for i in 0..8 {
            eq_v(out_c[i], out_r[i], &format!("c2BBVerts out[{i}]"));
        }
        // The C version takes a non-const pointer but must not modify the box.
        eq_v(bb_c.min, bb_r.min, "c2BBVerts in.min");
        eq_v(bb_c.max, bb_r.max, "c2BBVerts in.max");
    }
}

#[test]
fn c2MakeProxy_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(102);
    for _ in 0..scaled(ITERS) {
        // Sentinel-filled proxies: `c2MakeProxy` leaves most of `verts`
        // untouched, and the `default:` case leaves everything untouched.
        let sentinel = c2Proxy {
            radius: -13.5,
            count: -99,
            verts: [c2v { x: 1.5, y: -2.5 }; 8],
        };
        let mut p_c = sentinel;
        let mut p_r = sentinel;

        // Include out-of-range type tags to cover the (implicit) default arm.
        let ty = match g.below(6) {
            0 => C2_TYPE_CIRCLE,
            1 => C2_TYPE_AABB,
            2 => C2_TYPE_CAPSULE,
            3 => 3,
            4 => -1,
            _ => 1000,
        };
        // Back the shape with a buffer big enough for the largest shape so that
        // even a mis-tagged read stays in bounds.
        let mut buf = [0u8; 32];
        let cap = g.capsule_any();
        unsafe {
            std::ptr::copy_nonoverlapping(
                &cap as *const c2Capsule as *const u8,
                buf.as_mut_ptr(),
                std::mem::size_of::<c2Capsule>(),
            );
        }
        unsafe {
            (c.c2MakeProxy)(buf.as_ptr() as *const c_void, ty, &mut p_c);
            (r.c2MakeProxy)(buf.as_ptr() as *const c_void, ty, &mut p_r);
        }
        eq_proxy(&p_c, &p_r, &format!("c2MakeProxy(type={ty})"));
    }
}

// ---------------------------------------------------------------------------
// c2Support
// ---------------------------------------------------------------------------

#[test]
fn c2Support_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(103);
    for _ in 0..scaled(ITERS) {
        let mut verts = [c2v::default(); 8];
        let nasty = g.below(4) == 0;
        for v in verts.iter_mut() {
            *v = if nasty { g.v_nasty() } else { g.v(60.0) };
        }
        // Duplicate vertices exercise the strict `>` tie-break.
        if g.below(3) == 0 {
            verts[3] = verts[1];
            verts[5] = verts[1];
        }
        let count = g.below(9) as c_int; // 0..8, including the degenerate 0
        let d = if nasty { g.v_nasty() } else { g.v(10.0) };
        unsafe {
            eq_i(
                (c.c2Support)(verts.as_ptr(), count, d),
                (r.c2Support)(verts.as_ptr(), count, d),
                &format!("c2Support(count={count})"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Simplex generators
// ---------------------------------------------------------------------------

/// Random simplex.  `count` covers the whole `int` range that the switch
/// statements can see, including values outside 1..=3 that hit `default:`.
fn gen_simplex(g: &mut Rng) -> c2Simplex {
    let mag = match g.below(4) {
        0 => 1.0e-6,
        1 => 1.0e6,
        _ => 60.0,
    };
    let mut s = c2Simplex {
        a: g.sv(mag),
        b: g.sv(mag),
        c: g.sv(mag),
        d: g.sv(mag),
        div: g.f32_range(mag),
        count: match g.below(8) {
            0 => 0,
            1 => 4,
            2 => -1,
            3 => 7,
            n => (n - 3) as c_int, // 1, 2, 3, 4
        },
    };
    if g.below(6) == 0 {
        s.a.p = g.v_nasty();
        s.b.p = g.v_nasty();
        s.c.p = g.v_nasty();
        s.div = g.f32_nasty();
    }
    // Frequently force a "real" count so the interesting branches dominate.
    if g.below(2) == 0 {
        s.count = 1 + g.below(3) as c_int;
    }
    s
}

/// A simplex whose points are coherent Minkowski-difference points, so the
/// barycentric branch selection in `c22` / `c23` is realistically exercised.
fn gen_coherent_simplex(g: &mut Rng, count: c_int) -> c2Simplex {
    let mut s = c2Simplex::default();
    let mag = match g.below(3) {
        0 => 1.0,
        1 => 1000.0,
        _ => 30.0,
    };
    for i in 0..3 {
        let sA = g.v(mag);
        let sB = g.v(mag);
        let v = c2sv {
            sA,
            sB,
            p: c2v {
                x: sB.x - sA.x,
                y: sB.y - sA.y,
            },
            u: g.f32_range(1.0).abs(),
            iA: g.below(4) as c_int,
            iB: g.below(4) as c_int,
        };
        match i {
            0 => s.a = v,
            1 => s.b = v,
            _ => s.c = v,
        }
    }
    s.div = s.a.u + s.b.u + s.c.u;
    s.count = count;
    s
}

// ---------------------------------------------------------------------------
// c2GJKSimplexMetric / c2D / c2L / c22 / c23 / c2Witness
// ---------------------------------------------------------------------------

#[test]
fn c2GJKSimplexMetric_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(104);
    for _ in 0..scaled(ITERS) {
        let mut s_c = gen_simplex(&mut g);
        let mut s_r = s_c;
        let (vc, vr) = unsafe {
            (
                (c.c2GJKSimplexMetric)(&mut s_c),
                (r.c2GJKSimplexMetric)(&mut s_r),
            )
        };
        eq_f32(vc, vr, "c2GJKSimplexMetric");
        eq_simplex(&s_c, &s_r, "c2GJKSimplexMetric/simplex");
    }
}

#[test]
fn c2D_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(105);
    for _ in 0..scaled(ITERS) {
        let mut s_c = gen_simplex(&mut g);
        let mut s_r = s_c;
        let (vc, vr) = unsafe { ((c.c2D)(&mut s_c), (r.c2D)(&mut s_r)) };
        eq_v(vc, vr, "c2D");
        eq_simplex(&s_c, &s_r, "c2D/simplex");
    }
}

#[test]
fn c2L_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(106);
    for _ in 0..scaled(ITERS) {
        let mut s_c = gen_simplex(&mut g);
        let mut s_r = s_c;
        let (vc, vr) = unsafe { ((c.c2L)(&mut s_c), (r.c2L)(&mut s_r)) };
        eq_v(vc, vr, "c2L");
        eq_simplex(&s_c, &s_r, "c2L/simplex");
    }
}

#[test]
fn c22_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(107);
    for _ in 0..scaled(ITERS) {
        let mut s_c = gen_simplex(&mut g);
        let mut s_r = s_c;
        unsafe {
            (c.c22)(&mut s_c);
            (r.c22)(&mut s_r);
        }
        eq_simplex(&s_c, &s_r, "c22");
    }
    let mut g = Rng::new(1077);
    for _ in 0..scaled(ITERS) {
        let mut s_c = gen_coherent_simplex(&mut g, 2);
        let mut s_r = s_c;
        unsafe {
            (c.c22)(&mut s_c);
            (r.c22)(&mut s_r);
        }
        eq_simplex(&s_c, &s_r, "c22/coherent");
    }
}

#[test]
fn c23_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(108);
    for _ in 0..scaled(ITERS) {
        let mut s_c = gen_simplex(&mut g);
        let mut s_r = s_c;
        unsafe {
            (c.c23)(&mut s_c);
            (r.c23)(&mut s_r);
        }
        eq_simplex(&s_c, &s_r, "c23");
    }
    let mut g = Rng::new(1088);
    for _ in 0..scaled(ITERS) {
        let mut s_c = gen_coherent_simplex(&mut g, 3);
        let mut s_r = s_c;
        unsafe {
            (c.c23)(&mut s_c);
            (r.c23)(&mut s_r);
        }
        eq_simplex(&s_c, &s_r, "c23/coherent");
    }
    // Degenerate triangles (collinear / duplicated points) drive `area == 0`
    // and therefore all of uABC/vABC/wABC to zero, selecting the final else.
    let mut g = Rng::new(10888);
    for _ in 0..scaled(ITERS) {
        let mut s = gen_coherent_simplex(&mut g, 3);
        match g.below(3) {
            0 => s.b.p = s.a.p,
            1 => s.c.p = s.a.p,
            _ => {
                s.c.p = c2v {
                    x: s.a.p.x + (s.b.p.x - s.a.p.x) * 2.0,
                    y: s.a.p.y + (s.b.p.y - s.a.p.y) * 2.0,
                }
            }
        }
        let mut s_c = s;
        let mut s_r = s;
        unsafe {
            (c.c23)(&mut s_c);
            (r.c23)(&mut s_r);
        }
        eq_simplex(&s_c, &s_r, "c23/degenerate");
    }
}

#[test]
fn c2Witness_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(109);
    for _ in 0..scaled(ITERS) {
        let mut s_c = gen_simplex(&mut g);
        let mut s_r = s_c;
        let mut a_c = c2v { x: 99.0, y: -99.0 };
        let mut b_c = c2v { x: -1.0, y: 1.0 };
        let mut a_r = a_c;
        let mut b_r = b_c;
        unsafe {
            (c.c2Witness)(&mut s_c, &mut a_c, &mut b_c);
            (r.c2Witness)(&mut s_r, &mut a_r, &mut b_r);
        }
        eq_v(a_c, a_r, "c2Witness a");
        eq_v(b_c, b_r, "c2Witness b");
        eq_simplex(&s_c, &s_r, "c2Witness/simplex");
    }
}
