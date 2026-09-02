//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `dlopen`/`dlsym` on BOTH `.so`s; results are compared
//! bit-for-bit (`f32::to_bits`), never with an epsilon.

#![allow(non_snake_case)]
mod common;

use common::*;

const N: usize = 2048;

// ===========================================================================
// A. Scalar / vector primitives (rows 1-19)
// ===========================================================================

/// Row 1 — `c2V`.
#[test]
fn row001_c2V() {
    let p = pair();
    let mut rng = Rng::new(1);
    for i in 0..N {
        let (x, y) = (rng.wild(), rng.wild());
        unsafe { same(&format!("c2V #{i} ({x},{y})"), (p.c.c2V)(x, y), (p.rs.c2V)(x, y)) };
    }
}

/// Row 2 — `c2Mulvs`.
#[test]
fn row002_c2Mulvs() {
    let p = pair();
    let mut rng = Rng::new(2);
    for i in 0..N {
        let (a, b) = (rng.vec_any(), rng.wild());
        unsafe { same(&format!("c2Mulvs #{i} ({a:?},{b})"), (p.c.c2Mulvs)(a, b), (p.rs.c2Mulvs)(a, b)) };
    }
}

/// Row 3 — `c2Add` / `c2Sub`, including exact cancellation and `inf - inf`.
#[test]
fn row003_c2Add_c2Sub() {
    let p = pair();
    let mut rng = Rng::new(3);
    for i in 0..N {
        let a = rng.vec_any();
        // Half the time reuse `a` so `a - a` cancels exactly.
        let b = if rng.below(2) == 0 { a } else { rng.vec_any() };
        unsafe {
            same(&format!("c2Add #{i} ({a:?},{b:?})"), (p.c.c2Add)(a, b), (p.rs.c2Add)(a, b));
            same(&format!("c2Sub #{i} ({a:?},{b:?})"), (p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b));
        }
    }
}

/// Row 4 — `c2Neg` (sign of zero must flip).
#[test]
fn row004_c2Neg() {
    let p = pair();
    let mut rng = Rng::new(4);
    for i in 0..N {
        let a = rng.vec_any();
        unsafe { same(&format!("c2Neg #{i} ({a:?})"), (p.c.c2Neg)(a), (p.rs.c2Neg)(a)) };
    }
    // Explicit ±0 / NaN probes.
    for a in [
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: f32::NAN, y: -0.0 },
    ] {
        unsafe { same("c2Neg zero", (p.c.c2Neg)(a), (p.rs.c2Neg)(a)) };
    }
}

/// Row 5 — `c2Skew` / `c2CCW90`.
#[test]
fn row005_c2Skew_c2CCW90() {
    let p = pair();
    let mut rng = Rng::new(5);
    for i in 0..N {
        let a = rng.vec_any();
        unsafe {
            same(&format!("c2Skew #{i} ({a:?})"), (p.c.c2Skew)(a), (p.rs.c2Skew)(a));
            same(&format!("c2CCW90 #{i} ({a:?})"), (p.c.c2CCW90)(a), (p.rs.c2CCW90)(a));
        }
    }
}

/// Row 6 — `c2Dot`, incl. overflow and cancellation.
#[test]
fn row006_c2Dot() {
    let p = pair();
    let mut rng = Rng::new(6);
    for i in 0..N {
        let (a, b) = (rng.vec_any(), rng.vec_any());
        unsafe { same(&format!("c2Dot #{i} ({a:?},{b:?})"), (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b)) };
    }
    // Guaranteed overflow / exact cancellation.
    let probes = [
        (c2v { x: 1e30, y: 1e30 }, c2v { x: 1e30, y: 1e30 }),
        (c2v { x: 1e30, y: 1e30 }, c2v { x: 1e30, y: -1e30 }),
        (c2v { x: f32::MAX, y: f32::MAX }, c2v { x: 2.0, y: 2.0 }),
    ];
    for (a, b) in probes {
        unsafe { same("c2Dot overflow", (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b)) };
    }
}

/// Row 7 — `c2Det2`, incl. parallel / anti-parallel operands.
#[test]
fn row007_c2Det2() {
    let p = pair();
    let mut rng = Rng::new(7);
    for i in 0..N {
        let a = rng.vec_any();
        let b = match rng.below(3) {
            0 => a,                                  // parallel -> det == 0
            1 => c2v { x: -a.x, y: -a.y },           // anti-parallel
            _ => rng.vec_any(),
        };
        unsafe { same(&format!("c2Det2 #{i} ({a:?},{b:?})"), (p.c.c2Det2)(a, b), (p.rs.c2Det2)(a, b)) };
    }
}

/// Row 8 — `c2Len`.
#[test]
fn row008_c2Len() {
    let p = pair();
    let mut rng = Rng::new(8);
    for i in 0..N {
        let a = rng.vec_any();
        unsafe { same(&format!("c2Len #{i} ({a:?})"), (p.c.c2Len)(a), (p.rs.c2Len)(a)) };
    }
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 1e30, y: 1e30 },
        c2v { x: f32::from_bits(1), y: 0.0 },
        c2v { x: 3.0, y: 4.0 },
    ] {
        unsafe { same("c2Len probe", (p.c.c2Len)(a), (p.rs.c2Len)(a)) };
    }
}

/// Row 9 — `c2Div` with non-zero divisors.
#[test]
fn row009_c2Div() {
    let p = pair();
    let mut rng = Rng::new(9);
    let mut done = 0;
    while done < N {
        let a = rng.vec_any();
        let b = rng.wild();
        if b == 0.0 {
            continue; // zero divisor belongs to ERRORS.md row 12
        }
        unsafe { same(&format!("c2Div #{done} ({a:?},{b})"), (p.c.c2Div)(a, b), (p.rs.c2Div)(a, b)) };
        done += 1;
    }
}

/// Row 10 — `c2Norm` on non-zero vectors.
#[test]
fn row010_c2Norm() {
    let p = pair();
    let mut rng = Rng::new(10);
    let mut done = 0;
    while done < N {
        let a = rng.vec_any();
        if a.x == 0.0 && a.y == 0.0 {
            continue; // zero vector belongs to ERRORS.md row 13
        }
        unsafe { same(&format!("c2Norm #{done} ({a:?})"), (p.c.c2Norm)(a), (p.rs.c2Norm)(a)) };
        done += 1;
    }
}

/// Row 11 — `c2Maxv` / `c2Minv` (C ternaries, NaN-sensitive).
#[test]
fn row011_c2Maxv_c2Minv() {
    let p = pair();
    let mut rng = Rng::new(11);
    for i in 0..N {
        let a = rng.vec_any();
        let b = if rng.below(3) == 0 { a } else { rng.vec_any() };
        unsafe {
            same(&format!("c2Maxv #{i} ({a:?},{b:?})"), (p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b));
            same(&format!("c2Minv #{i} ({a:?},{b:?})"), (p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b));
        }
    }
    // NaN in either operand: `a > b ? a : b` returns b when a is NaN.
    let nan = f32::NAN;
    for (a, b) in [
        (c2v { x: nan, y: 1.0 }, c2v { x: 1.0, y: nan }),
        (c2v { x: 1.0, y: nan }, c2v { x: nan, y: 1.0 }),
        (c2v { x: nan, y: nan }, c2v { x: nan, y: nan }),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: -0.0 }),
        (c2v { x: -0.0, y: -0.0 }, c2v { x: 0.0, y: 0.0 }),
    ] {
        unsafe {
            same("c2Maxv nan", (p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b));
            same("c2Minv nan", (p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b));
        }
    }
}

/// Row 12 — `c2Clampv` with a well-formed box (`lo <= hi`).
#[test]
fn row012_c2Clampv_wellformed() {
    let p = pair();
    let mut rng = Rng::new(12);
    for i in 0..N {
        let c = rng.vec_grid();
        let e = c2v {
            x: rng.below(6) as f32,
            y: rng.below(6) as f32,
        };
        let lo = c2v { x: c.x - e.x, y: c.y - e.y };
        let hi = c2v { x: c.x + e.x, y: c.y + e.y };
        // `a` sometimes lands exactly on an edge.
        let a = match rng.below(4) {
            0 => lo,
            1 => hi,
            2 => rng.vec_grid(),
            _ => rng.vec_coord(),
        };
        unsafe {
            same(
                &format!("c2Clampv #{i} ({a:?},{lo:?},{hi:?})"),
                (p.c.c2Clampv)(a, lo, hi),
                (p.rs.c2Clampv)(a, lo, hi),
            )
        };
    }
}

/// Row 13 — `c2Clampv` with an inverted box (`lo > hi`).
#[test]
fn row013_c2Clampv_inverted() {
    let p = pair();
    let mut rng = Rng::new(13);
    for i in 0..N {
        let c = rng.vec_grid();
        let e = c2v {
            x: 1.0 + rng.below(6) as f32,
            y: 1.0 + rng.below(6) as f32,
        };
        let lo = c2v { x: c.x + e.x, y: c.y + e.y };
        let hi = c2v { x: c.x - e.x, y: c.y - e.y };
        let a = rng.vec_any();
        unsafe {
            same(
                &format!("c2Clampv inverted #{i} ({a:?},{lo:?},{hi:?})"),
                (p.c.c2Clampv)(a, lo, hi),
                (p.rs.c2Clampv)(a, lo, hi),
            )
        };
    }
}

/// Row 14 — `c2RotIdentity` / `c2xIdentity` exact bit patterns.
#[test]
fn row014_identities() {
    let p = pair();
    unsafe {
        same("c2RotIdentity", (p.c.c2RotIdentity)(), (p.rs.c2RotIdentity)());
        same("c2xIdentity", (p.c.c2xIdentity)(), (p.rs.c2xIdentity)());
        let r = (p.c.c2RotIdentity)();
        assert_eq!((r.c, r.s), (1.0, 0.0));
    }
}

/// Row 15 — `c2Mulrv` / `c2MulrvT` with the identity rotation.
#[test]
fn row015_mulrv_identity() {
    let p = pair();
    let mut rng = Rng::new(15);
    let r = c2r { c: 1.0, s: 0.0 };
    for i in 0..N {
        let b = rng.vec_any();
        unsafe {
            same(&format!("c2Mulrv id #{i}"), (p.c.c2Mulrv)(r, b), (p.rs.c2Mulrv)(r, b));
            same(&format!("c2MulrvT id #{i}"), (p.c.c2MulrvT)(r, b), (p.rs.c2MulrvT)(r, b));
        }
    }
}

/// Row 16 — `c2Mulrv` / `c2MulrvT` with normalised rotations.
#[test]
fn row016_mulrv_unit_rotation() {
    let p = pair();
    let mut rng = Rng::new(16);
    for i in 0..N {
        let r = rng.rot_unit();
        let b = rng.vec_any();
        unsafe {
            same(&format!("c2Mulrv unit #{i} ({r:?},{b:?})"), (p.c.c2Mulrv)(r, b), (p.rs.c2Mulrv)(r, b));
            same(&format!("c2MulrvT unit #{i} ({r:?},{b:?})"), (p.c.c2MulrvT)(r, b), (p.rs.c2MulrvT)(r, b));
        }
    }
}

/// Row 17 — un-normalised `c2r` (never validated by C).
#[test]
fn row017_mulrv_unnormalised() {
    let p = pair();
    let mut rng = Rng::new(17);
    for i in 0..N {
        let r = c2r { c: rng.wild(), s: rng.wild() };
        let b = rng.vec_any();
        unsafe {
            same(&format!("c2Mulrv wild #{i} ({r:?},{b:?})"), (p.c.c2Mulrv)(r, b), (p.rs.c2Mulrv)(r, b));
            same(&format!("c2MulrvT wild #{i} ({r:?},{b:?})"), (p.c.c2MulrvT)(r, b), (p.rs.c2MulrvT)(r, b));
        }
    }
}

/// Row 18 — `c2Mulxv` with identity rotation + translation.
#[test]
fn row018_mulxv_translation() {
    let p = pair();
    let mut rng = Rng::new(18);
    for i in 0..N {
        let a = rng.xform_translation();
        let b = rng.vec_any();
        unsafe { same(&format!("c2Mulxv T #{i} ({a:?},{b:?})"), (p.c.c2Mulxv)(a, b), (p.rs.c2Mulxv)(a, b)) };
    }
}

/// Row 19 — `c2Mulxv` with rotation + translation (and wild rotations).
#[test]
fn row019_mulxv_full() {
    let p = pair();
    let mut rng = Rng::new(19);
    for i in 0..N {
        let a = match rng.below(3) {
            0 => rng.xform_rotation(),
            1 => rng.xform_unnormalised(),
            _ => rng.xform_full(),
        };
        let b = rng.vec_any();
        unsafe { same(&format!("c2Mulxv full #{i} ({a:?},{b:?})"), (p.c.c2Mulxv)(a, b), (p.rs.c2Mulxv)(a, b)) };
    }
}

// ===========================================================================
// B. Proxy construction (rows 20-28)
// ===========================================================================

unsafe fn diff_bbverts(p: &Pair, bb: c2AABB, what: &str) {
    // Poison the output arrays so a missing write shows up.
    let poison = c2v { x: f32::from_bits(0x7F81_2345), y: f32::from_bits(0x7F81_2346) };
    let mut oc = [poison; 4];
    let mut or = [poison; 4];
    let mut bc = bb;
    let mut br = bb;
    unsafe {
        (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bc);
        (p.rs.c2BBVerts)(or.as_mut_ptr(), &mut br);
    }
    same(what, oc, or);
    same(&format!("{what} (input untouched)"), bc, br);
}

/// Row 20 — `c2BBVerts` on well-formed AABBs.
#[test]
fn row020_bbverts_wellformed() {
    let p = pair();
    let mut rng = Rng::new(20);
    for i in 0..N {
        let c = rng.vec_coord();
        let e = c2v { x: rng.range(0.0, 50.0), y: rng.range(0.0, 50.0) };
        let bb = c2AABB {
            min: c2v { x: c.x - e.x, y: c.y - e.y },
            max: c2v { x: c.x + e.x, y: c.y + e.y },
        };
        unsafe { diff_bbverts(p, bb, &format!("c2BBVerts #{i} ({bb:?})")) };
    }
}

/// Row 21 — `c2BBVerts` on degenerate / inverted / non-finite AABBs.
#[test]
fn row021_bbverts_degenerate_inverted() {
    let p = pair();
    let mut rng = Rng::new(21);
    for i in 0..N {
        let bb = match rng.below(4) {
            0 => {
                let v = rng.vec_any();
                c2AABB { min: v, max: v } // degenerate point
            }
            1 => {
                let v = rng.vec_coord();
                c2AABB { min: c2v { x: v.x + 1.0, y: v.y + 1.0 }, max: v } // inverted
            }
            2 => c2AABB { min: rng.vec_wild(), max: rng.vec_wild() },
            _ => c2AABB { min: rng.vec_grid(), max: rng.vec_grid() },
        };
        unsafe { diff_bbverts(p, bb, &format!("c2BBVerts degen #{i} ({bb:?})")) };
    }
}

/// Fill `*p` with a recognisable pattern, call `c2MakeProxy`, compare the whole
/// struct (so an untouched field is detected too).
unsafe fn diff_makeproxy(p: &Pair, shape: &Shape, ty: i32, what: &str) {
    let seed_proxy = c2Proxy {
        radius: f32::from_bits(0x1234_5678),
        count: -777,
        verts: [c2v { x: f32::from_bits(0x1111_1111), y: f32::from_bits(0x2222_2222) }; 8],
    };
    let mut pc = seed_proxy;
    let mut pr = seed_proxy;
    let mut bc = shape.bytes.clone();
    let mut br = shape.bytes.clone();
    unsafe {
        (p.c.c2MakeProxy)(bc.as_mut_ptr() as *const std::ffi::c_void, ty, &mut pc);
        (p.rs.c2MakeProxy)(br.as_mut_ptr() as *const std::ffi::c_void, ty, &mut pr);
    }
    same(what, pc, pr);
    assert_eq!(bc, br, "{what}: shape bytes mutated differently");
}

/// Row 22 — `c2MakeProxy(CIRCLE)`.
#[test]
fn row022_makeproxy_circle() {
    let p = pair();
    let mut rng = Rng::new(22);
    for i in 0..N {
        let c = c2Circle {
            p: rng.vec_any(),
            r: match rng.below(4) {
                0 => 0.0,
                1 => -rng.range(0.0, 10.0),
                2 => rng.wild(),
                _ => rng.range(0.0, 100.0),
            },
        };
        let s = Shape::circle(c);
        unsafe { diff_makeproxy(p, &s, C2_TYPE_CIRCLE, &format!("c2MakeProxy CIRCLE #{i} ({c:?})")) };
    }
}

/// Row 23 — `c2MakeProxy(AABB)`.
#[test]
fn row023_makeproxy_aabb() {
    let p = pair();
    let mut rng = Rng::new(23);
    for i in 0..N {
        let bb = match rng.below(4) {
            0 => { let v = rng.vec_any(); c2AABB { min: v, max: v } }
            1 => { let v = rng.vec_coord(); c2AABB { min: c2v { x: v.x + 2.0, y: v.y + 2.0 }, max: v } }
            2 => c2AABB { min: rng.vec_wild(), max: rng.vec_wild() },
            _ => { let c = rng.vec_coord(); c2AABB { min: c2v { x: c.x - 1.0, y: c.y - 3.0 }, max: c2v { x: c.x + 4.0, y: c.y + 2.0 } } }
        };
        let s = Shape::aabb(bb);
        unsafe { diff_makeproxy(p, &s, C2_TYPE_AABB, &format!("c2MakeProxy AABB #{i} ({bb:?})")) };
    }
}

/// Row 24 — `c2MakeProxy(CAPSULE)`.
#[test]
fn row024_makeproxy_capsule() {
    let p = pair();
    let mut rng = Rng::new(24);
    for i in 0..N {
        let a = rng.vec_any();
        let cap = c2Capsule {
            a,
            b: if rng.below(4) == 0 { a } else { rng.vec_any() },
            r: match rng.below(4) {
                0 => 0.0,
                1 => -rng.range(0.0, 10.0),
                2 => rng.wild(),
                _ => rng.range(0.0, 100.0),
            },
        };
        let s = Shape::capsule(cap);
        unsafe { diff_makeproxy(p, &s, C2_TYPE_CAPSULE, &format!("c2MakeProxy CAPSULE #{i} ({cap:?})")) };
    }
}

fn diff_support(p: &Pair, verts: &[c2v], count: i32, d: c2v, what: &str) {
    let rc = unsafe { (p.c.c2Support)(verts.as_ptr(), count, d) };
    let rr = unsafe { (p.rs.c2Support)(verts.as_ptr(), count, d) };
    same(what, rc, rr);
}

/// Rows 25-28 — `c2Support` for each proxy vertex count (1, 2, 4, 8).
#[test]
fn row025_028_support_all_counts() {
    let p = pair();
    for (row, count) in [(25, 1i32), (26, 2), (27, 4), (28, 8)] {
        let mut rng = Rng::new(1000 + row as u64);
        for i in 0..N {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = match rng.below(4) {
                    0 => rng.vec_grid(),
                    1 => rng.vec_wild(),
                    _ => rng.vec_coord(),
                };
            }
            // Sometimes duplicate vertices so ties are exercised.
            if rng.below(3) == 0 && count > 1 {
                let j = rng.below(count as u32) as usize;
                verts[j] = verts[0];
            }
            let d = match rng.below(5) {
                0 => c2v { x: 1.0, y: 0.0 },
                1 => c2v { x: 0.0, y: 1.0 },
                2 => c2v { x: 0.0, y: 0.0 },
                3 => rng.vec_wild(),
                _ => rng.vec_coord(),
            };
            diff_support(p, &verts, count, d, &format!("row{row} c2Support count={count} #{i} d={d:?} verts={verts:?}"));
        }
        // Deterministic axis-aligned sweep so every winning index is hit.
        let square = [
            c2v { x: -1.0, y: -1.0 },
            c2v { x: 1.0, y: -1.0 },
            c2v { x: 1.0, y: 1.0 },
            c2v { x: -1.0, y: 1.0 },
            c2v { x: -2.0, y: 0.0 },
            c2v { x: 2.0, y: 0.0 },
            c2v { x: 0.0, y: -2.0 },
            c2v { x: 0.0, y: 2.0 },
        ];
        for k in 0..64 {
            let t = k as f32 * std::f32::consts::TAU / 64.0;
            let d = c2v { x: t.cos(), y: t.sin() };
            diff_support(p, &square, count, d, &format!("row{row} c2Support sweep k={k}"));
        }
    }
}

// ===========================================================================
// C. Simplex reduction (rows 29-51)
// ===========================================================================

/// Build a `c2Simplex` with fully random contents and the given `count`.
fn rand_simplex(rng: &mut Rng, count: i32) -> c2Simplex {
    let mut s = c2Simplex::default();
    for v in s.v.iter_mut() {
        *v = c2sv {
            sA: rng.vec_any(),
            sB: rng.vec_any(),
            p: rng.vec_any(),
            u: rng.wild(),
            iA: rng.below(8) as i32,
            iB: rng.below(8) as i32,
        };
    }
    s.div = match rng.below(4) {
        0 => 1.0,
        1 => 0.0,
        2 => rng.coord(),
        _ => rng.range(-4.0, 4.0),
    };
    s.count = count;
    s
}

/// Simplex whose `p` values sit on a small integer grid (exact ties / zeros).
fn grid_simplex(rng: &mut Rng, count: i32) -> c2Simplex {
    let mut s = c2Simplex::default();
    for v in s.v.iter_mut() {
        *v = c2sv {
            sA: rng.vec_grid(),
            sB: rng.vec_grid(),
            p: rng.vec_grid(),
            u: rng.grid(),
            iA: rng.below(8) as i32,
            iB: rng.below(8) as i32,
        };
    }
    s.div = if rng.below(2) == 0 { 1.0 } else { rng.grid() };
    s.count = count;
    s
}

fn diff_simplex_mutator(
    p: &Pair,
    s: c2Simplex,
    what: &str,
    f: impl Fn(&Api, *mut c2Simplex),
) -> c2Simplex {
    let mut sc = s;
    let mut sr = s;
    f(p.c, &mut sc);
    f(p.rs, &mut sr);
    same(what, sc, sr);
    sc
}

/// Rows 29-31 — `c2GJKSimplexMetric` for `count` 1, 2, 3.
#[test]
fn row029_031_simplex_metric() {
    let p = pair();
    for count in [1i32, 2, 3] {
        let mut rng = Rng::new(29 + count as u64);
        for i in 0..N {
            let mut s = if i % 2 == 0 {
                rand_simplex(&mut rng, count)
            } else {
                grid_simplex(&mut rng, count)
            };
            // Sometimes make the three `p`s collinear / coincident.
            if rng.below(4) == 0 {
                s.v[2].p = s.v[1].p;
            }
            let mut sc = s;
            let mut sr = s;
            let rc = unsafe { (p.c.c2GJKSimplexMetric)(&mut sc) };
            let rr = unsafe { (p.rs.c2GJKSimplexMetric)(&mut sr) };
            same(&format!("c2GJKSimplexMetric count={count} #{i} {s:?}"), rc, rr);
            same(&format!("c2GJKSimplexMetric count={count} #{i} (simplex untouched)"), sc, sr);
        }
    }
}

/// Which `c22` branch does the C source take for this simplex?
fn c22_branch(s: &c2Simplex) -> u32 {
    let (a, b) = (s.v[0].p, s.v[1].p);
    let dot = |x: c2v, y: c2v| x.x * y.x + x.y * y.y;
    let sub = |x: c2v, y: c2v| c2v { x: x.x - y.x, y: x.y - y.y };
    let u = dot(b, sub(b, a));
    let v = dot(a, sub(a, b));
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

/// Rows 32-35 — `c22`: all three reduction branches plus fully random input.
#[test]
fn row032_035_c22_all_branches() {
    let p = pair();
    let mut rng = Rng::new(32);
    let mut hits = [0usize; 3];
    for i in 0..N * 4 {
        let mut s = match i % 3 {
            0 => rand_simplex(&mut rng, 2),
            1 => grid_simplex(&mut rng, 2),
            _ => {
                // Deliberately place the origin outside / inside the segment.
                let mut s = grid_simplex(&mut rng, 2);
                let d = rng.rot_unit();
                let t0 = rng.range(-3.0, 3.0);
                let t1 = t0 + rng.range(0.1, 4.0);
                s.v[0].p = c2v { x: d.c * t0, y: d.s * t0 };
                s.v[1].p = c2v { x: d.c * t1, y: d.s * t1 };
                s
            }
        };
        // Occasionally make a == b exactly (u == v == 0).
        if rng.below(8) == 0 {
            s.v[1].p = s.v[0].p;
        }
        hits[c22_branch(&s) as usize] += 1;
        diff_simplex_mutator(p, s, &format!("c22 #{i} {s:?}"), |api, ptr| unsafe {
            (api.c22)(ptr)
        });
    }
    assert!(
        hits.iter().all(|&h| h > 0),
        "c22 branch coverage incomplete: {hits:?}"
    );
    eprintln!("c22 branch hits: {hits:?}");
}

/// Which `c23` branch does the C source take?
fn c23_branch(s: &c2Simplex) -> u32 {
    let (a, b, c) = (s.v[0].p, s.v[1].p, s.v[2].p);
    let dot = |x: c2v, y: c2v| x.x * y.x + x.y * y.y;
    let sub = |x: c2v, y: c2v| c2v { x: x.x - y.x, y: x.y - y.y };
    let det = |x: c2v, y: c2v| x.x * y.y - x.y * y.x;
    let uab = dot(b, sub(b, a));
    let vab = dot(a, sub(a, b));
    let ubc = dot(c, sub(c, b));
    let vbc = dot(b, sub(b, c));
    let uca = dot(a, sub(a, c));
    let vca = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let uabc = det(b, c) * area;
    let vabc = det(c, a) * area;
    let wabc = det(a, b) * area;
    if vab <= 0.0 && uca <= 0.0 {
        0
    } else if uab <= 0.0 && vbc <= 0.0 {
        1
    } else if ubc <= 0.0 && vca <= 0.0 {
        2
    } else if uab > 0.0 && vab > 0.0 && wabc <= 0.0 {
        3
    } else if ubc > 0.0 && vbc > 0.0 && uabc <= 0.0 {
        4
    } else if uca > 0.0 && vca > 0.0 && vabc <= 0.0 {
        5
    } else {
        6
    }
}

/// Rows 36-43 — `c23`: all seven reduction branches plus random/degenerate input.
#[test]
fn row036_043_c23_all_branches() {
    let p = pair();
    let mut rng = Rng::new(36);
    let mut hits = [0usize; 7];
    for i in 0..N * 8 {
        let mut s = match i % 4 {
            0 => rand_simplex(&mut rng, 3),
            1 => grid_simplex(&mut rng, 3),
            2 => {
                // Triangle around a randomly offset origin: reliably reaches
                // the "origin inside" and every edge/vertex region.
                let mut s = grid_simplex(&mut rng, 3);
                let off = c2v { x: rng.range(-3.0, 3.0), y: rng.range(-3.0, 3.0) };
                let r = rng.range(0.5, 3.0);
                for k in 0..3 {
                    let t = rng.range(0.0, std::f32::consts::TAU)
                        + k as f32 * std::f32::consts::TAU / 3.0;
                    s.v[k].p = c2v { x: off.x + r * t.cos(), y: off.y + r * t.sin() };
                }
                s
            }
            _ => {
                // Degenerate: collinear or coincident vertices (area == 0).
                let mut s = grid_simplex(&mut rng, 3);
                let d = rng.rot_unit();
                for k in 0..3 {
                    let t = rng.range(-4.0, 4.0);
                    s.v[k].p = c2v { x: d.c * t, y: d.s * t };
                }
                if rng.below(3) == 0 {
                    s.v[2].p = s.v[1].p;
                }
                s
            }
        };
        if rng.below(16) == 0 {
            // Reversed winding.
            let t = s.v[1].p;
            s.v[1].p = s.v[2].p;
            s.v[2].p = t;
        }
        hits[c23_branch(&s) as usize] += 1;
        diff_simplex_mutator(p, s, &format!("c23 #{i} {s:?}"), |api, ptr| unsafe {
            (api.c23)(ptr)
        });
    }
    assert!(
        hits.iter().all(|&h| h > 0),
        "c23 branch coverage incomplete: {hits:?}"
    );
    eprintln!("c23 branch hits: {hits:?}");
}

/// Rows 44-46 — `c2D` for `count == 1` and both `count == 2` sub-branches.
#[test]
fn row044_046_c2D() {
    let p = pair();
    let mut rng = Rng::new(44);
    let mut hits = [0usize; 3]; // count1, skew, ccw90
    for i in 0..N * 4 {
        let count = if i % 3 == 0 { 1 } else { 2 };
        let mut s = if i % 2 == 0 {
            rand_simplex(&mut rng, count)
        } else {
            grid_simplex(&mut rng, count)
        };
        if rng.below(8) == 0 {
            s.v[1].p = s.v[0].p; // ab == 0
        }
        if count == 1 {
            hits[0] += 1;
        } else {
            let ab = c2v { x: s.v[1].p.x - s.v[0].p.x, y: s.v[1].p.y - s.v[0].p.y };
            let na = c2v { x: -s.v[0].p.x, y: -s.v[0].p.y };
            if ab.x * na.y - ab.y * na.x > 0.0 {
                hits[1] += 1;
            } else {
                hits[2] += 1;
            }
        }
        let mut sc = s;
        let mut sr = s;
        let rc = unsafe { (p.c.c2D)(&mut sc) };
        let rr = unsafe { (p.rs.c2D)(&mut sr) };
        same(&format!("c2D count={count} #{i} {s:?}"), rc, rr);
        same(&format!("c2D count={count} #{i} (untouched)"), sc, sr);
    }
    assert!(hits.iter().all(|&h| h > 0), "c2D branch coverage: {hits:?}");
    eprintln!("c2D branch hits (count1, skew, ccw90): {hits:?}");
}

/// Rows 47-48 — `c2L` for `count == 1` and `count == 2`.
#[test]
fn row047_048_c2L() {
    let p = pair();
    for count in [1i32, 2] {
        let mut rng = Rng::new(47 + count as u64);
        for i in 0..N {
            let s = if i % 2 == 0 {
                rand_simplex(&mut rng, count)
            } else {
                grid_simplex(&mut rng, count)
            };
            let mut sc = s;
            let mut sr = s;
            let rc = unsafe { (p.c.c2L)(&mut sc) };
            let rr = unsafe { (p.rs.c2L)(&mut sr) };
            same(&format!("c2L count={count} #{i} {s:?}"), rc, rr);
            same(&format!("c2L count={count} #{i} (untouched)"), sc, sr);
        }
    }
}

/// Rows 49-51 — `c2Witness` for `count` 1, 2, 3.
#[test]
fn row049_051_c2Witness() {
    let p = pair();
    for count in [1i32, 2, 3] {
        let mut rng = Rng::new(49 + count as u64);
        for i in 0..N {
            let s = if i % 2 == 0 {
                rand_simplex(&mut rng, count)
            } else {
                grid_simplex(&mut rng, count)
            };
            let poison = c2v { x: f32::from_bits(0x5555_5555), y: f32::from_bits(0x6666_6666) };
            let (mut ac, mut bc, mut ar, mut br) = (poison, poison, poison, poison);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (p.rs.c2Witness)(&mut sr, &mut ar, &mut br);
            }
            same(&format!("c2Witness count={count} #{i} {s:?}"), (ac, bc), (ar, br));
            same(&format!("c2Witness count={count} #{i} (untouched)"), sc, sr);
        }
    }
}

// ===========================================================================
// D. c2GJK — full option cross-product (rows 52-88)
// ===========================================================================

/// Samples per (type-pair, geometry-class) cell. Kept modest because the rows
/// multiply out to 9 type pairs x 10 classes.
const G: usize = 48;

/// Drive `c2GJK` over all 9 type pairs x all geometry classes.
fn gjk_sweep(
    seed: u64,
    label: &str,
    xf: impl Fn(&mut Rng) -> (Option<c2x>, Option<c2x>),
    use_radii: &[i32],
    sel: OutSel,
    cache: impl Fn(&mut Rng) -> Option<c2GJKCache>,
) {
    let p = pair();
    let mut rng = Rng::new(seed);
    for tya in TYPES {
        for tyb in TYPES {
            for class in ALL_CLASSES {
                for i in 0..G {
                    let sa = gen_shape(&mut rng, tya, class, false);
                    let sb = gen_shape(&mut rng, tyb, class, true);
                    let (ax, bx) = xf(&mut rng);
                    let ci = cache(&mut rng);
                    for &ur in use_radii {
                        diff_gjk(
                            p,
                            &format!(
                                "{label} {}x{} {class:?} ur={ur} #{i}",
                                type_name(tya),
                                type_name(tyb)
                            ),
                            &sa,
                            ax,
                            &sb,
                            bx,
                            ur,
                            sel,
                            ci,
                        );
                    }
                }
            }
        }
    }
}

fn no_xf(_: &mut Rng) -> (Option<c2x>, Option<c2x>) {
    (None, None)
}
fn no_cache(_: &mut Rng) -> Option<c2GJKCache> {
    None
}

/// Rows 52-61 — every type pair, `use_radius` 0 and 1, no transforms, no cache.
///
/// One test per row so a failure names the exact `CONFIGS.md` row.
macro_rules! gjk_pair_row {
    ($name:ident, $row:expr, $seed:expr, $tya:expr, $tyb:expr) => {
        #[test]
        fn $name() {
            let p = pair();
            let mut rng = Rng::new($seed);
            for class in ALL_CLASSES {
                for i in 0..(G * 4) {
                    let sa = gen_shape(&mut rng, $tya, class, false);
                    let sb = gen_shape(&mut rng, $tyb, class, true);
                    for ur in [0, 1] {
                        diff_gjk(
                            p,
                            &format!(
                                "row{} c2GJK {}x{} {class:?} ur={ur} #{i}",
                                $row,
                                type_name($tya),
                                type_name($tyb)
                            ),
                            &sa,
                            None,
                            &sb,
                            None,
                            ur,
                            OutSel::ALL,
                            None,
                        );
                    }
                }
            }
        }
    };
}

gjk_pair_row!(row052_053_gjk_circle_circle, "052/053", 52, C2_TYPE_CIRCLE, C2_TYPE_CIRCLE);
gjk_pair_row!(row054_gjk_circle_aabb, "054", 54, C2_TYPE_CIRCLE, C2_TYPE_AABB);
gjk_pair_row!(row055_gjk_circle_capsule, "055", 55, C2_TYPE_CIRCLE, C2_TYPE_CAPSULE);
gjk_pair_row!(row056_gjk_aabb_circle, "056", 56, C2_TYPE_AABB, C2_TYPE_CIRCLE);
gjk_pair_row!(row057_gjk_aabb_aabb, "057", 57, C2_TYPE_AABB, C2_TYPE_AABB);
gjk_pair_row!(row058_gjk_aabb_capsule, "058", 58, C2_TYPE_AABB, C2_TYPE_CAPSULE);
gjk_pair_row!(row059_gjk_capsule_circle, "059", 59, C2_TYPE_CAPSULE, C2_TYPE_CIRCLE);
gjk_pair_row!(row060_gjk_capsule_aabb, "060", 60, C2_TYPE_CAPSULE, C2_TYPE_AABB);
gjk_pair_row!(row061_gjk_capsule_capsule, "061", 61, C2_TYPE_CAPSULE, C2_TYPE_CAPSULE);

/// Row 62 — `ax` = translation only, `bx = NULL`.
#[test]
fn row062_gjk_ax_translation() {
    gjk_sweep(62, "row062", |r| (Some(r.xform_translation()), None), &[0, 1], OutSel::ALL, no_cache);
}

/// Row 63 — `ax = NULL`, `bx` = translation only.
#[test]
fn row063_gjk_bx_translation() {
    gjk_sweep(63, "row063", |r| (None, Some(r.xform_translation())), &[0, 1], OutSel::ALL, no_cache);
}

/// Row 64 — both transforms rotation-only.
#[test]
fn row064_gjk_both_rotation() {
    gjk_sweep(64, "row064", |r| (Some(r.xform_rotation()), Some(r.xform_rotation())), &[0, 1], OutSel::ALL, no_cache);
}

/// Row 65 — both transforms rotation + translation, `use_radius = 0`.
#[test]
fn row065_gjk_full_xform_no_radius() {
    gjk_sweep(65, "row065", |r| (Some(r.xform_full()), Some(r.xform_full())), &[0], OutSel::ALL, no_cache);
}

/// Row 66 — both transforms rotation + translation, `use_radius = 1`.
#[test]
fn row066_gjk_full_xform_radius() {
    gjk_sweep(66, "row066", |r| (Some(r.xform_full()), Some(r.xform_full())), &[1], OutSel::ALL, no_cache);
}

/// Row 67 — explicit `c2xIdentity()` structs must give exactly the `NULL` result.
#[test]
fn row067_gjk_explicit_identity_equals_null() {
    let p = pair();
    let ident = unsafe { (p.c.c2xIdentity)() };
    let ident_rs = unsafe { (p.rs.c2xIdentity)() };
    same("c2xIdentity agreement", ident, ident_rs);

    let mut rng = Rng::new(67);
    for tya in TYPES {
        for tyb in TYPES {
            for class in ALL_CLASSES {
                for i in 0..G {
                    let sa = gen_shape(&mut rng, tya, class, false);
                    let sb = gen_shape(&mut rng, tyb, class, true);
                    for ur in [0, 1] {
                        let what = format!("row067 {}x{} {class:?} ur={ur} #{i}", type_name(tya), type_name(tyb));
                        // C vs Rust with explicit identity...
                        diff_gjk(p, &what, &sa, Some(ident), &sb, Some(ident), ur, OutSel::ALL, None);
                        // ...and identity must equal the NULL-pointer path.
                        let a = run_gjk(p.c, &sa, Some(ident), &sb, Some(ident), ur, OutSel::ALL, None);
                        let b = run_gjk(p.c, &sa, None, &sb, None, ur, OutSel::ALL, None);
                        same(&format!("{what}: identity == NULL (C)"), a, b);
                        let a = run_gjk(p.rs, &sa, Some(ident), &sb, Some(ident), ur, OutSel::ALL, None);
                        let b = run_gjk(p.rs, &sa, None, &sb, None, ur, OutSel::ALL, None);
                        same(&format!("{what}: identity == NULL (Rust)"), a, b);
                    }
                }
            }
        }
    }
}

/// Row 68 — un-normalised `c2r` in the transform (never validated by C).
#[test]
fn row068_gjk_unnormalised_rotation() {
    gjk_sweep(68, "row068", |r| (Some(r.xform_unnormalised()), Some(r.xform_unnormalised())), &[0, 1], OutSel::ALL, no_cache);
}

/// Rows 69-72 — every combination of NULL/non-NULL out-params.
#[test]
fn row069_072_gjk_out_param_selection() {
    for (row, sel) in [
        ("069", OutSel { a: false, b: true, iters: true }),
        ("070", OutSel { a: true, b: false, iters: true }),
        ("071", OutSel { a: false, b: false, iters: true }),
        ("072", OutSel { a: true, b: true, iters: false }),
        ("069-072b", OutSel { a: false, b: false, iters: false }),
        ("069-072c", OutSel { a: true, b: false, iters: false }),
        ("069-072d", OutSel { a: false, b: true, iters: false }),
    ] {
        gjk_sweep(69, &format!("row{row}"), no_xf, &[0, 1], sel, no_cache);
    }
}

/// Row 73 — the iteration count must match exactly for every geometry class.
#[test]
fn row073_gjk_iteration_count_matches() {
    let p = pair();
    let mut rng = Rng::new(73);
    let mut seen = std::collections::BTreeSet::new();
    for tya in TYPES {
        for tyb in TYPES {
            for class in ALL_CLASSES {
                for i in 0..G * 2 {
                    let sa = gen_shape(&mut rng, tya, class, false);
                    let sb = gen_shape(&mut rng, tyb, class, true);
                    for ur in [0, 1] {
                        let oc = run_gjk(p.c, &sa, None, &sb, None, ur, OutSel::ALL, None);
                        let or = run_gjk(p.rs, &sa, None, &sb, None, ur, OutSel::ALL, None);
                        same(
                            &format!("row073 iters {}x{} {class:?} ur={ur} #{i}", type_name(tya), type_name(tyb)),
                            oc.iters.unwrap(),
                            or.iters.unwrap(),
                        );
                        seen.insert(oc.iters.unwrap());
                    }
                }
            }
        }
    }
    eprintln!("row073 observed iteration counts: {seen:?}");
    assert!(seen.len() >= 3, "iteration counts not varied enough: {seen:?}");
}

/// Row 74 — non-NULL, zero-initialised cache (cold start + write-back).
#[test]
fn row074_gjk_cache_cold() {
    gjk_sweep(74, "row074", no_xf, &[0, 1], OutSel::ALL, |_| Some(c2GJKCache::default()));
}

/// Run a chain of `c2GJK` calls sharing one cache; return every observable.
fn cache_chain(
    api: &Api,
    steps: &[(Shape, Option<c2x>, Shape, Option<c2x>, i32)],
) -> Vec<GjkOut> {
    let mut cache = c2GJKCache::default();
    let mut out = Vec::new();
    for (sa, ax, sb, bx, ur) in steps {
        let mut a = c2v { x: f32::from_bits(0xDEAD_BEEF), y: f32::from_bits(0xDEAD_BEEE) };
        let mut b = a;
        let mut iters = -12345i32;
        let mut abytes = sa.bytes.clone();
        let mut bbytes = sb.bytes.clone();
        let axp = ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
        let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
        let dist = unsafe {
            (api.c2GJK)(
                abytes.as_mut_ptr() as *const std::ffi::c_void,
                sa.ty,
                axp,
                bbytes.as_mut_ptr() as *const std::ffi::c_void,
                sb.ty,
                bxp,
                &mut a,
                &mut b,
                *ur,
                &mut iters,
                &mut cache,
            )
        };
        out.push(GjkOut { dist, a: Some(a), b: Some(b), iters: Some(iters), cache: Some(cache) });
    }
    out
}

/// Row 75 — cache re-used for repeated calls with identical shapes (warm start).
#[test]
fn row075_gjk_cache_warm_same_shapes() {
    let p = pair();
    let mut rng = Rng::new(75);
    for tya in TYPES {
        for tyb in TYPES {
            for class in ALL_CLASSES {
                for i in 0..G {
                    let sa = gen_shape(&mut rng, tya, class, false);
                    let sb = gen_shape(&mut rng, tyb, class, true);
                    for ur in [0, 1] {
                        let steps: Vec<_> = (0..4)
                            .map(|_| (sa.clone(), None, sb.clone(), None, ur))
                            .collect();
                        let oc = cache_chain(p.c, &steps);
                        let or = cache_chain(p.rs, &steps);
                        same(
                            &format!("row075 warm {}x{} {class:?} ur={ur} #{i}", type_name(tya), type_name(tyb)),
                            oc,
                            or,
                        );
                    }
                }
            }
        }
    }
}

/// Row 76 — cache re-used while a shape moves (transform sweep).
#[test]
fn row076_gjk_cache_moving_shape() {
    let p = pair();
    let mut rng = Rng::new(76);
    for tya in TYPES {
        for tyb in TYPES {
            for i in 0..G * 2 {
                let sa = gen_shape(&mut rng, tya, Class::Near, false);
                let sb = gen_shape(&mut rng, tyb, Class::Near, true);
                for ur in [0, 1] {
                    let steps: Vec<_> = (0..8)
                        .map(|k| {
                            let t = k as f32 * 0.75 - 3.0;
                            let bx = c2x {
                                p: c2v { x: t, y: t * 0.5 },
                                r: {
                                    let ang = t * 0.3;
                                    c2r { c: ang.cos(), s: ang.sin() }
                                },
                            };
                            (sa.clone(), None, sb.clone(), Some(bx), ur)
                        })
                        .collect();
                    let oc = cache_chain(p.c, &steps);
                    let or = cache_chain(p.rs, &steps);
                    same(
                        &format!("row076 moving {}x{} ur={ur} #{i}", type_name(tya), type_name(tyb)),
                        oc,
                        or,
                    );
                }
            }
        }
    }
}

/// Number of vertices a proxy of this type has (mirrors `c2MakeProxy`).
fn proxy_count(ty: i32) -> i32 {
    match ty {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        C2_TYPE_CAPSULE => 2,
        _ => 0,
    }
}

/// Rows 77-79 — hand-built caches with `count` 1/2/3 and in-range indices.
#[test]
fn row077_079_gjk_handbuilt_cache_valid_indices() {
    let p = pair();
    for count in [1i32, 2, 3] {
        let mut rng = Rng::new(77 + count as u64);
        for tya in TYPES {
            for tyb in TYPES {
                for class in ALL_CLASSES {
                    for i in 0..G / 2 {
                        let sa = gen_shape(&mut rng, tya, class, false);
                        let sb = gen_shape(&mut rng, tyb, class, true);
                        let (na, nb) = (proxy_count(tya), proxy_count(tyb));
                        let mut cache = c2GJKCache { count, ..Default::default() };
                        for k in 0..count as usize {
                            cache.iA[k] = rng.below(na as u32) as i32;
                            cache.iB[k] = rng.below(nb as u32) as i32;
                        }
                        cache.metric = match rng.below(4) {
                            0 => 0.0,
                            1 => 1.0,
                            2 => rng.coord(),
                            _ => -1.0e9,
                        };
                        cache.div = match rng.below(3) {
                            0 => 1.0,
                            1 => 0.0,
                            _ => rng.range(-4.0, 4.0),
                        };
                        for ur in [0, 1] {
                            diff_gjk(
                                p,
                                &format!("row{} cache count={count} {}x{} {class:?} ur={ur} #{i}", 76 + count, type_name(tya), type_name(tyb)),
                                &sa, None, &sb, None, ur, OutSel::ALL, Some(cache),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Row 80 — extreme cache `metric` values driving the metric gate.
#[test]
fn row080_gjk_cache_extreme_metric() {
    let p = pair();
    let metrics = [0.0f32, -0.0, 1.0, -1.0, -1.0e8, -1.0e9, f32::MAX, f32::MIN, FLT_EPSILON, f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    let mut rng = Rng::new(80);
    for &metric in &metrics {
        for tya in TYPES {
            for tyb in TYPES {
                for count in [0i32, 1, 2, 3] {
                    for i in 0..8 {
                        let sa = gen_shape(&mut rng, tya, Class::Near, false);
                        let sb = gen_shape(&mut rng, tyb, Class::Near, true);
                        let (na, nb) = (proxy_count(tya), proxy_count(tyb));
                        let mut cache = c2GJKCache { metric, count, div: 1.0, ..Default::default() };
                        for k in 0..count.max(0) as usize {
                            cache.iA[k] = rng.below(na as u32) as i32;
                            cache.iB[k] = rng.below(nb as u32) as i32;
                        }
                        for ur in [0, 1] {
                            diff_gjk(
                                p,
                                &format!("row080 metric={metric} count={count} {}x{} ur={ur} #{i}", type_name(tya), type_name(tyb)),
                                &sa, None, &sb, None, ur, OutSel::ALL, Some(cache),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Row 81 — cache `div` values feeding `1.0f / div` in `c2Witness`/`c2L`.
#[test]
fn row081_gjk_cache_div_values() {
    let p = pair();
    let divs = [0.0f32, -0.0, 1.0, -1.0, 0.5, 1.0e-30, 1.0e30, f32::NAN, f32::INFINITY];
    let mut rng = Rng::new(81);
    for &div in &divs {
        for tya in TYPES {
            for tyb in TYPES {
                for count in [1i32, 2, 3] {
                    for i in 0..8 {
                        let sa = gen_shape(&mut rng, tya, Class::Near, false);
                        let sb = gen_shape(&mut rng, tyb, Class::Near, true);
                        let (na, nb) = (proxy_count(tya), proxy_count(tyb));
                        let mut cache = c2GJKCache { metric: 0.0, count, div, ..Default::default() };
                        for k in 0..count as usize {
                            cache.iA[k] = rng.below(na as u32) as i32;
                            cache.iB[k] = rng.below(nb as u32) as i32;
                        }
                        for ur in [0, 1] {
                            diff_gjk(
                                p,
                                &format!("row081 div={div} count={count} {}x{} ur={ur} #{i}", type_name(tya), type_name(tyb)),
                                &sa, None, &sb, None, ur, OutSel::ALL, Some(cache),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Row 82 — fully degenerate shapes (zero radius / zero extent) for all pairs.
#[test]
fn row082_gjk_degenerate_shapes() {
    let p = pair();
    let mut rng = Rng::new(82);
    for tya in TYPES {
        for tyb in TYPES {
            for i in 0..G * 4 {
                let sa = gen_shape(&mut rng, tya, Class::Degenerate, false);
                let sb = gen_shape(&mut rng, tyb, Class::Degenerate, true);
                for ur in [0, 1] {
                    for (ax, bx) in [(None, None), (Some(rng.xform_full()), Some(rng.xform_full()))] {
                        diff_gjk(p, &format!("row082 {}x{} ur={ur} #{i}", type_name(tya), type_name(tyb)), &sa, ax, &sb, bx, ur, OutSel::ALL, Some(c2GJKCache::default()));
                    }
                }
            }
        }
    }
}

/// Row 83 — A and B identical (`dist == 0`, `hit` path).
#[test]
fn row083_gjk_coincident_shapes() {
    let p = pair();
    let mut rng = Rng::new(83);
    let mut hit_seen = 0usize;
    for ty in TYPES {
        for i in 0..G * 8 {
            let s = gen_shape(&mut rng, ty, Class::Coincident, false);
            for ur in [0, 1] {
                diff_gjk(p, &format!("row083 {} self ur={ur} #{i}", type_name(ty)), &s, None, &s, None, ur, OutSel::ALL, None);
                let o = run_gjk(p.c, &s, None, &s, None, ur, OutSel::ALL, None);
                if o.dist == 0.0 {
                    hit_seen += 1;
                }
            }
        }
        // Cross-type but concentric.
        for tyb in TYPES {
            for i in 0..G {
                let sa = gen_shape(&mut rng, ty, Class::Coincident, false);
                let sb = gen_shape(&mut rng, tyb, Class::Coincident, true);
                for ur in [0, 1] {
                    diff_gjk(p, &format!("row083 {}x{} concentric ur={ur} #{i}", type_name(ty), type_name(tyb)), &sa, None, &sb, None, ur, OutSel::ALL, None);
                }
            }
        }
    }
    assert!(hit_seen > 0, "no zero-distance/hit cases produced");
}

/// Row 84 — exact-touching shapes -> the `use_radius` midpoint-collapse branch.
#[test]
fn row084_gjk_exact_touch_radius_collapse() {
    let p = pair();
    // Circle-circle placed so the surface distance is exactly rA + rB.
    for k in 0..256u32 {
        let r_a = 1.0 + (k % 8) as f32;
        let r_b = 0.5 + (k / 8 % 8) as f32;
        let gap = r_a + r_b;
        let a = Shape::circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: r_a });
        for delta in [0.0f32, -1.0e-6, 1.0e-6, -FLT_EPSILON, FLT_EPSILON] {
            let b = Shape::circle(c2Circle { p: c2v { x: gap + delta, y: 0.0 }, r: r_b });
            diff_gjk(p, &format!("row084 circle touch k={k} d={delta}"), &a, None, &b, None, 1, OutSel::ALL, None);
            diff_gjk(p, &format!("row084 circle touch k={k} d={delta} ur=0"), &a, None, &b, None, 0, OutSel::ALL, None);
        }
    }
    // Capsule-capsule, AABB-capsule: touching along an axis.
    let mut rng = Rng::new(84);
    for tya in TYPES {
        for tyb in TYPES {
            for i in 0..G * 2 {
                let ra = rng.range(0.0, 3.0);
                let rb = rng.range(0.0, 3.0);
                let sa = match tya {
                    C2_TYPE_CIRCLE => Shape::circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra }),
                    C2_TYPE_AABB => Shape::aabb(c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } }),
                    _ => Shape::capsule(c2Capsule { a: c2v { x: 0.0, y: -1.0 }, b: c2v { x: 0.0, y: 1.0 }, r: ra }),
                };
                let base = if tya == C2_TYPE_AABB { 1.0 } else { ra };
                let off = base + rb + if tyb == C2_TYPE_AABB { 1.0 } else { 0.0 };
                let sb = match tyb {
                    C2_TYPE_CIRCLE => Shape::circle(c2Circle { p: c2v { x: off, y: 0.0 }, r: rb }),
                    C2_TYPE_AABB => Shape::aabb(c2AABB { min: c2v { x: off - 1.0, y: -1.0 }, max: c2v { x: off + 1.0, y: 1.0 } }),
                    _ => Shape::capsule(c2Capsule { a: c2v { x: off, y: -1.0 }, b: c2v { x: off, y: 1.0 }, r: rb }),
                };
                for ur in [0, 1] {
                    diff_gjk(p, &format!("row084 {}x{} touch ur={ur} #{i}", type_name(tya), type_name(tyb)), &sa, None, &sb, None, ur, OutSel::ALL, None);
                }
            }
        }
    }
}

/// Row 85 — negative radii with `use_radius = 1`.
#[test]
fn row085_gjk_negative_radius() {
    let p = pair();
    let mut rng = Rng::new(85);
    for tya in [C2_TYPE_CIRCLE, C2_TYPE_CAPSULE] {
        for tyb in TYPES {
            for i in 0..G * 4 {
                let neg = -rng.range(0.0, 8.0);
                let sa = match tya {
                    C2_TYPE_CIRCLE => Shape::circle(c2Circle { p: rng.vec_grid(), r: neg }),
                    _ => Shape::capsule(c2Capsule { a: rng.vec_grid(), b: rng.vec_grid(), r: neg }),
                };
                let sb = match tyb {
                    C2_TYPE_CIRCLE => Shape::circle(c2Circle { p: rng.vec_grid(), r: -rng.range(0.0, 8.0) }),
                    C2_TYPE_AABB => gen_shape(&mut rng, C2_TYPE_AABB, Class::Grid, true),
                    _ => Shape::capsule(c2Capsule { a: rng.vec_grid(), b: rng.vec_grid(), r: -rng.range(0.0, 8.0) }),
                };
                for ur in [0, 1] {
                    diff_gjk(p, &format!("row085 {}x{} neg-r ur={ur} #{i}", type_name(tya), type_name(tyb)), &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
                }
            }
        }
    }
}

/// Row 86 — huge and subnormal coordinates.
#[test]
fn row086_gjk_extreme_magnitudes() {
    let p = pair();
    let mut rng = Rng::new(86);
    for class in [Class::Huge, Class::Tiny] {
        for tya in TYPES {
            for tyb in TYPES {
                for i in 0..G * 2 {
                    let sa = gen_shape(&mut rng, tya, class, false);
                    let sb = gen_shape(&mut rng, tyb, class, true);
                    for ur in [0, 1] {
                        diff_gjk(p, &format!("row086 {class:?} {}x{} ur={ur} #{i}", type_name(tya), type_name(tyb)), &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
                    }
                }
            }
        }
    }
}

/// Row 87 — inverted AABBs as A and/or B.
#[test]
fn row087_gjk_inverted_aabb() {
    let p = pair();
    let mut rng = Rng::new(87);
    for i in 0..G * 16 {
        let inv = Shape::aabb({
            let c = rng.vec_grid();
            let e = c2v { x: 1.0 + rng.below(5) as f32, y: 1.0 + rng.below(5) as f32 };
            c2AABB { min: c2v { x: c.x + e.x, y: c.y + e.y }, max: c2v { x: c.x - e.x, y: c.y - e.y } }
        });
        for tyb in TYPES {
            let other = gen_shape(&mut rng, tyb, Class::Grid, true);
            for ur in [0, 1] {
                diff_gjk(p, &format!("row087 invAABBxB {} ur={ur} #{i}", type_name(tyb)), &inv, None, &other, None, ur, OutSel::ALL, None);
                diff_gjk(p, &format!("row087 Bxinv AABB {} ur={ur} #{i}", type_name(tyb)), &other, None, &inv, None, ur, OutSel::ALL, None);
            }
        }
    }
}

/// Row 88 — configurations that stall / reach the 20-iteration cap.
#[test]
fn row088_gjk_nonconvergent() {
    let p = pair();
    let mut rng = Rng::new(88);
    let mut max_iter = 0;
    // Nearly-parallel, nearly-touching thin capsules and slivers are the worst
    // case for GJK convergence.
    for i in 0..N * 2 {
        let eps = 10f32.powi(-(rng.below(30) as i32 + 1));
        let sa = Shape::capsule(c2Capsule {
            a: c2v { x: -1.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: eps,
        });
        let sb = Shape::capsule(c2Capsule {
            a: c2v { x: -1.0, y: eps },
            b: c2v { x: 1.0, y: eps * 2.0 },
            r: eps,
        });
        for ur in [0, 1] {
            diff_gjk(p, &format!("row088 sliver eps={eps} ur={ur} #{i}"), &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
            let o = run_gjk(p.c, &sa, None, &sb, None, ur, OutSel::ALL, None);
            max_iter = max_iter.max(o.iters.unwrap());
        }
        // Slivers vs degenerate AABBs.
        let flat = Shape::aabb(c2AABB { min: c2v { x: -1.0, y: 0.0 }, max: c2v { x: 1.0, y: eps } });
        for ur in [0, 1] {
            diff_gjk(p, &format!("row088 flat eps={eps} ur={ur} #{i}"), &flat, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
            let o = run_gjk(p.c, &flat, None, &sb, None, ur, OutSel::ALL, None);
            max_iter = max_iter.max(o.iters.unwrap());
        }
    }
    eprintln!("row088 max iterations observed: {max_iter}");
}

// ===========================================================================
// E. Boolean convenience wrappers (rows 89-98)
// ===========================================================================

/// Row 89 — `c2CircletoCircle`: separated, tangent, overlapping, concentric,
/// zero and negative radii.
#[test]
fn row089_circle_to_circle() {
    let p = pair();
    let mut rng = Rng::new(89);
    for i in 0..N * 8 {
        let a = c2Circle {
            p: if rng.below(2) == 0 { rng.vec_grid() } else { rng.vec_coord() },
            r: match rng.below(5) {
                0 => 0.0,
                1 => -rng.range(0.0, 8.0),
                2 => rng.below(8) as f32,
                3 => rng.wild(),
                _ => rng.range(0.0, 20.0),
            },
        };
        let b = c2Circle {
            p: if rng.below(3) == 0 { a.p } else if rng.below(2) == 0 { rng.vec_grid() } else { rng.vec_coord() },
            r: match rng.below(5) {
                0 => 0.0,
                1 => -rng.range(0.0, 8.0),
                2 => rng.below(8) as f32,
                3 => rng.wild(),
                _ => rng.range(0.0, 20.0),
            },
        };
        unsafe {
            same(&format!("c2CircletoCircle #{i} {a:?} {b:?}"), (p.c.c2CircletoCircle)(a, b), (p.rs.c2CircletoCircle)(a, b));
        }
    }
    // Exact tangency: d2 == r2 must be a MISS (`<`, not `<=`).
    for k in 1..64u32 {
        let ra = k as f32;
        let rb = (k % 7 + 1) as f32;
        for delta in [-1.0f32, -0.5, 0.0, 0.5, 1.0] {
            let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra };
            let b = c2Circle { p: c2v { x: ra + rb + delta, y: 0.0 }, r: rb };
            unsafe {
                same(&format!("c2CircletoCircle tangent k={k} d={delta}"), (p.c.c2CircletoCircle)(a, b), (p.rs.c2CircletoCircle)(a, b));
            }
        }
    }
}

/// Row 90 — `c2CircletoAABB` with well-formed boxes (all 8 Voronoi regions).
#[test]
fn row090_circle_to_aabb_wellformed() {
    let p = pair();
    let mut rng = Rng::new(90);
    for i in 0..N * 8 {
        let c = rng.vec_grid();
        let e = c2v { x: rng.below(6) as f32, y: rng.below(6) as f32 };
        let bb = c2AABB {
            min: c2v { x: c.x - e.x, y: c.y - e.y },
            max: c2v { x: c.x + e.x, y: c.y + e.y },
        };
        let circ = c2Circle {
            p: if rng.below(2) == 0 { rng.vec_grid() } else { rng.vec_coord() },
            r: match rng.below(4) {
                0 => 0.0,
                1 => -rng.range(0.0, 5.0),
                2 => rng.below(8) as f32,
                _ => rng.range(0.0, 12.0),
            },
        };
        unsafe {
            same(&format!("c2CircletoAABB #{i} {circ:?} {bb:?}"), (p.c.c2CircletoAABB)(circ, bb), (p.rs.c2CircletoAABB)(circ, bb));
        }
    }
    // Deterministic 8-region + inside sweep around a unit box.
    let bb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
    for gx in -6..=6 {
        for gy in -6..=6 {
            for r in [0.0f32, 0.5, 1.0, 1.5, 2.0, 3.0] {
                let circ = c2Circle { p: c2v { x: gx as f32 * 0.5, y: gy as f32 * 0.5 }, r };
                unsafe {
                    same(&format!("c2CircletoAABB sweep {gx},{gy},{r}"), (p.c.c2CircletoAABB)(circ, bb), (p.rs.c2CircletoAABB)(circ, bb));
                }
            }
        }
    }
}

/// Row 91 — `c2CircletoAABB` with degenerate / inverted boxes.
#[test]
fn row091_circle_to_aabb_degenerate() {
    let p = pair();
    let mut rng = Rng::new(91);
    for i in 0..N * 8 {
        let bb = match rng.below(4) {
            0 => { let v = rng.vec_grid(); c2AABB { min: v, max: v } }          // point
            1 => { let v = rng.vec_grid(); c2AABB { min: v, max: c2v { x: v.x + 4.0, y: v.y } } } // line
            2 => { let v = rng.vec_grid(); c2AABB { min: c2v { x: v.x + 2.0, y: v.y + 2.0 }, max: v } } // inverted
            _ => c2AABB { min: rng.vec_wild(), max: rng.vec_wild() },
        };
        let circ = c2Circle { p: rng.vec_any(), r: rng.wild() };
        unsafe {
            same(&format!("c2CircletoAABB degen #{i} {circ:?} {bb:?}"), (p.c.c2CircletoAABB)(circ, bb), (p.rs.c2CircletoAABB)(circ, bb));
        }
    }
}

/// Which `c2CircletoCapsule` branch does the C source take?
fn circap_branch(a: c2Circle, b: c2Capsule) -> u32 {
    let n = c2v { x: b.b.x - b.a.x, y: b.b.y - b.a.y };
    let ap = c2v { x: a.p.x - b.a.x, y: a.p.y - b.a.y };
    let da = ap.x * n.x + ap.y * n.y;
    if da < 0.0 {
        return 0;
    }
    let d = c2v { x: a.p.x - b.b.x, y: a.p.y - b.b.y };
    let db = d.x * n.x + d.y * n.y;
    if db < 0.0 { 1 } else { 2 }
}

/// Rows 92-95 — `c2CircletoCapsule`: all three branches + degenerate capsules.
#[test]
fn row092_095_circle_to_capsule() {
    let p = pair();
    let mut rng = Rng::new(92);
    let mut hits = [0usize; 3];
    for i in 0..N * 16 {
        let cap = c2Capsule {
            a: rng.vec_grid(),
            b: if rng.below(6) == 0 { rng.vec_grid() } else { rng.vec_grid() },
            r: match rng.below(5) {
                0 => 0.0,
                1 => -rng.range(0.0, 5.0),
                2 => rng.below(8) as f32,
                3 => rng.wild(),
                _ => rng.range(0.0, 12.0),
            },
        };
        let cap = if rng.below(8) == 0 { c2Capsule { b: cap.a, ..cap } } else { cap };
        let circ = c2Circle {
            p: if rng.below(2) == 0 { rng.vec_grid() } else { rng.vec_coord() },
            r: match rng.below(4) {
                0 => 0.0,
                1 => -rng.range(0.0, 5.0),
                2 => rng.below(8) as f32,
                _ => rng.range(0.0, 12.0),
            },
        };
        hits[circap_branch(circ, cap) as usize] += 1;
        unsafe {
            same(&format!("c2CircletoCapsule #{i} {circ:?} {cap:?}"), (p.c.c2CircletoCapsule)(circ, cap), (p.rs.c2CircletoCapsule)(circ, cap));
        }
    }
    assert!(hits.iter().all(|&h| h > 0), "c2CircletoCapsule branch coverage: {hits:?}");
    eprintln!("c2CircletoCapsule branch hits (da<0, db<0, else): {hits:?}");
}

/// Row 96 — `c2AABBtoAABB`: disjoint on each axis/side, touching edges and
/// corners, nested, identical, inverted.
#[test]
fn row096_aabb_to_aabb() {
    let p = pair();
    let mut rng = Rng::new(96);
    for i in 0..N * 8 {
        let mk = |rng: &mut Rng| {
            let c = rng.vec_grid();
            let e = c2v { x: rng.below(5) as f32, y: rng.below(5) as f32 };
            if rng.below(5) == 0 {
                c2AABB { min: c2v { x: c.x + e.x, y: c.y + e.y }, max: c2v { x: c.x - e.x, y: c.y - e.y } }
            } else {
                c2AABB { min: c2v { x: c.x - e.x, y: c.y - e.y }, max: c2v { x: c.x + e.x, y: c.y + e.y } }
            }
        };
        let a = mk(&mut rng);
        let b = if rng.below(4) == 0 { a } else { mk(&mut rng) };
        unsafe {
            same(&format!("c2AABBtoAABB #{i} {a:?} {b:?}"), (p.c.c2AABBtoAABB)(a, b), (p.rs.c2AABBtoAABB)(a, b));
        }
    }
    // Exhaustive small-integer sweep: all separation/touching/overlap patterns.
    for ax0 in -2..=2 {
        for ax1 in ax0..=3 {
            for bx0 in -2..=2 {
                for bx1 in bx0..=3 {
                    let a = c2AABB { min: c2v { x: ax0 as f32, y: 0.0 }, max: c2v { x: ax1 as f32, y: 1.0 } };
                    let b = c2AABB { min: c2v { x: bx0 as f32, y: 0.0 }, max: c2v { x: bx1 as f32, y: 1.0 } };
                    unsafe {
                        same(&format!("c2AABBtoAABB sweep {ax0},{ax1},{bx0},{bx1}"), (p.c.c2AABBtoAABB)(a, b), (p.rs.c2AABBtoAABB)(a, b));
                    }
                }
            }
        }
    }
}

/// Row 97 — `c2AABBtoCapsule`.
#[test]
fn row097_aabb_to_capsule() {
    let p = pair();
    let mut rng = Rng::new(97);
    for i in 0..N * 8 {
        let c = rng.vec_grid();
        let e = c2v { x: rng.below(5) as f32, y: rng.below(5) as f32 };
        let bb = if rng.below(6) == 0 {
            c2AABB { min: c2v { x: c.x + e.x, y: c.y + e.y }, max: c2v { x: c.x - e.x, y: c.y - e.y } }
        } else {
            c2AABB { min: c2v { x: c.x - e.x, y: c.y - e.y }, max: c2v { x: c.x + e.x, y: c.y + e.y } }
        };
        let a = rng.vec_grid();
        let cap = c2Capsule {
            a,
            b: if rng.below(6) == 0 { a } else { rng.vec_grid() },
            r: match rng.below(5) {
                0 => 0.0,
                1 => -rng.range(0.0, 5.0),
                2 => rng.below(6) as f32,
                3 => rng.range(0.0, 10.0),
                _ => rng.range(0.0, 2.0),
            },
        };
        unsafe {
            same(&format!("c2AABBtoCapsule #{i} {bb:?} {cap:?}"), (p.c.c2AABBtoCapsule)(bb, cap), (p.rs.c2AABBtoCapsule)(bb, cap));
        }
    }
}

/// Row 98 — `c2CapsuletoCapsule`: parallel, crossing, collinear, coincident,
/// degenerate, zero radius.
#[test]
fn row098_capsule_to_capsule() {
    let p = pair();
    let mut rng = Rng::new(98);
    for i in 0..N * 8 {
        let mk = |rng: &mut Rng| {
            let a = rng.vec_grid();
            c2Capsule {
                a,
                b: if rng.below(6) == 0 { a } else { rng.vec_grid() },
                r: match rng.below(5) {
                    0 => 0.0,
                    1 => -rng.range(0.0, 5.0),
                    2 => rng.below(6) as f32,
                    3 => rng.range(0.0, 10.0),
                    _ => rng.range(0.0, 2.0),
                },
            }
        };
        let a = mk(&mut rng);
        let b = if rng.below(5) == 0 { a } else { mk(&mut rng) };
        unsafe {
            same(&format!("c2CapsuletoCapsule #{i} {a:?} {b:?}"), (p.c.c2CapsuletoCapsule)(a, b), (p.rs.c2CapsuletoCapsule)(a, b));
        }
    }
    // Parallel / collinear / crossing families.
    for k in 0..40u32 {
        let t = k as f32 * 0.25 - 5.0;
        let cases = [
            // parallel
            (c2Capsule { a: c2v { x: -2.0, y: 0.0 }, b: c2v { x: 2.0, y: 0.0 }, r: 1.0 },
             c2Capsule { a: c2v { x: -2.0, y: t }, b: c2v { x: 2.0, y: t }, r: 1.0 }),
            // crossing
            (c2Capsule { a: c2v { x: -2.0, y: 0.0 }, b: c2v { x: 2.0, y: 0.0 }, r: 0.5 },
             c2Capsule { a: c2v { x: t, y: -2.0 }, b: c2v { x: t, y: 2.0 }, r: 0.5 }),
            // collinear
            (c2Capsule { a: c2v { x: -2.0, y: 0.0 }, b: c2v { x: 2.0, y: 0.0 }, r: 0.75 },
             c2Capsule { a: c2v { x: t, y: 0.0 }, b: c2v { x: t + 4.0, y: 0.0 }, r: 0.75 }),
        ];
        for (a, b) in cases {
            unsafe {
                same(&format!("c2CapsuletoCapsule family k={k}"), (p.c.c2CapsuletoCapsule)(a, b), (p.rs.c2CapsuletoCapsule)(a, b));
            }
        }
    }
}

// ===========================================================================
// F. c2Collided dispatcher + public entry point (rows 99-110)
// ===========================================================================

/// Rows 99-107 — every valid `(typeA, typeB)` pair through `c2Collided`.
#[test]
fn row099_107_collided_all_valid_pairs() {
    let p = pair();
    let mut rng = Rng::new(99);
    let mut results = std::collections::BTreeMap::<(i32, i32), [u64; 2]>::new();
    for tya in TYPES {
        for tyb in TYPES {
            for class in ALL_CLASSES {
                for i in 0..N {
                    let sa = gen_shape(&mut rng, tya, class, false);
                    let sb = gen_shape(&mut rng, tyb, class, true);
                    let mut ab = sa.bytes.clone();
                    let mut bb = sb.bytes.clone();
                    let mut ab2 = sa.bytes.clone();
                    let mut bb2 = sb.bytes.clone();
                    let rc = unsafe {
                        (p.c.c2Collided)(ab.as_mut_ptr() as *const _, tya, bb.as_mut_ptr() as *const _, tyb)
                    };
                    let rr = unsafe {
                        (p.rs.c2Collided)(ab2.as_mut_ptr() as *const _, tya, bb2.as_mut_ptr() as *const _, tyb)
                    };
                    same(
                        &format!("c2Collided {}x{} {class:?} #{i} A={:?} B={:?}", type_name(tya), type_name(tyb), sa.bytes, sb.bytes),
                        rc, rr,
                    );
                    let e = results.entry((tya, tyb)).or_default();
                    e[(rc != 0) as usize] += 1;
                }
            }
        }
    }
    // Every pair must have produced both a hit and a miss, otherwise the row
    // is not actually exercising the comparison.
    for (k, v) in &results {
        assert!(
            v[0] > 0 && v[1] > 0,
            "c2Collided pair ({}, {}) produced only {v:?}",
            type_name(k.0),
            type_name(k.1)
        );
    }
}

/// Row 108 — `capsule` over random arguments; all 8 result bitmasks must occur.
#[test]
fn row108_capsule_random_args() {
    let p = pair();
    let mut rng = Rng::new(108);
    let mut masks = [0u64; 8];
    for i in 0..40_000 {
        // The probe shapes live in x in [-80, -5], y in [-40, 100].
        let (min_x, min_y, max_x, max_y, r) = match rng.below(3) {
            0 => (
                rng.range(-120.0, 40.0),
                rng.range(-90.0, 140.0),
                rng.range(-120.0, 40.0),
                rng.range(-90.0, 140.0),
                rng.range(0.0, 40.0),
            ),
            1 => (
                (rng.below(17) as f32 - 12.0) * 10.0,
                (rng.below(25) as f32 - 10.0) * 10.0,
                (rng.below(17) as f32 - 12.0) * 10.0,
                (rng.below(25) as f32 - 10.0) * 10.0,
                rng.below(9) as f32 * 5.0,
            ),
            _ => (rng.wild(), rng.wild(), rng.wild(), rng.wild(), rng.wild()),
        };
        let rc = unsafe { (p.c.capsule)(min_x, min_y, max_x, max_y, r) };
        let rr = unsafe { (p.rs.capsule)(min_x, min_y, max_x, max_y, r) };
        same(&format!("capsule #{i} ({min_x},{min_y},{max_x},{max_y},{r})"), rc, rr);
        if (0..8).contains(&rc) {
            masks[rc as usize] += 1;
        }
    }
    eprintln!("capsule result-mask histogram: {masks:?}");
    assert!(
        masks.iter().all(|&m| m > 0),
        "not every capsule() result bitmask was produced: {masks:?}"
    );
}

/// Row 109 — boundary arguments for `capsule`.
#[test]
fn row109_capsule_boundary_args() {
    let p = pair();
    let vals = [
        0.0f32, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN,
        f32::MIN_POSITIVE, f32::from_bits(1), FLT_EPSILON, -70.0, -40.0, -20.0, -15.0, 20.0, 40.0,
        100.0, 1e18, -1e18, 1e-40,
    ];
    // Full cross-product over a reduced set of positions plus every value for r.
    let pos = [0.0f32, -70.0, -40.0, -20.0, -15.0, 100.0, f32::NAN, f32::INFINITY];
    for &min_x in &pos {
        for &min_y in &pos {
            for &max_x in &pos {
                for &max_y in &pos {
                    for &r in &vals {
                        let rc = unsafe { (p.c.capsule)(min_x, min_y, max_x, max_y, r) };
                        let rr = unsafe { (p.rs.capsule)(min_x, min_y, max_x, max_y, r) };
                        same(&format!("capsule boundary ({min_x},{min_y},{max_x},{max_y},{r})"), rc, rr);
                    }
                }
            }
        }
    }
    // a == b (degenerate capsule) for every r.
    for &v in &vals {
        for &r in &vals {
            let rc = unsafe { (p.c.capsule)(v, v, v, v, r) };
            let rr = unsafe { (p.rs.capsule)(v, v, v, v, r) };
            same(&format!("capsule degenerate ({v},{r})"), rc, rr);
        }
    }
}

/// Row 110 — exhaustive grid sweep over the fixed probe geometry.
#[test]
fn row110_capsule_grid_sweep() {
    let p = pair();
    // circle @(-70,0) r20 ; AABB (-40,-40)-(-15,-15) ; capsule (-40,40)-(-20,100) r10
    let xs: Vec<f32> = (0..13).map(|k| -110.0 + k as f32 * 10.0).collect();
    let ys: Vec<f32> = (0..17).map(|k| -60.0 + k as f32 * 10.0).collect();
    let rs = [0.0f32, 1.0, 5.0, 10.0, 25.0];
    let mut count = 0u64;
    for &min_x in &xs {
        for &min_y in &ys {
            for &max_x in &xs {
                for &max_y in &ys {
                    for &r in &rs {
                        let rc = unsafe { (p.c.capsule)(min_x, min_y, max_x, max_y, r) };
                        let rr = unsafe { (p.rs.capsule)(min_x, min_y, max_x, max_y, r) };
                        if rc != rr {
                            same(&format!("capsule grid ({min_x},{min_y},{max_x},{max_y},{r})"), rc, rr);
                        }
                        count += 1;
                    }
                }
            }
        }
    }
    eprintln!("row110 capsule grid sweep: {count} comparisons");
    assert!(count > 100_000);
}
