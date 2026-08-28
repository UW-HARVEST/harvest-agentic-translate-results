//! Exhaustive NaN-payload / signed-zero / infinity matrix.
//!
//! On x86 SSE, `addss`/`mulss` return the **destination** operand when both
//! operands are NaN, so a C expression's result depends on which operand the
//! compiler placed in the destination register.  Any Rust translation that
//! commutes an `fadd`/`fmul` differently from gcc diverges here and nowhere
//! else.  These tests drive the full cross product of distinctly-payloaded
//! NaNs (plus `±inf`, `±0`, and finite values that make an operation invalid,
//! e.g. `0 * inf` and `inf - inf`) through every exported arithmetic helper.
//!
//! This is a Phase B / Phase C hardening test: it backs CONFIGS.md rows
//! C02..C16 and ERRORS.md rows E27..E36, E78..E83.

mod common;
use common::*;

/// Quiet NaNs with pairwise-distinct payloads and both sign bits, one signalling
/// NaN, both infinities, both zeros, and a couple of plain finite values.
const VALS: [u32; 12] = [
    0x7FC0_0001, // +qNaN payload 1
    0xFFC0_0002, // -qNaN payload 2
    0x7FC0_1234, // +qNaN payload 3
    0xFFCA_BCDE, // -qNaN payload 4
    0x7F80_0001, // +sNaN
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x0000_0000, // +0
    0x8000_0000, // -0
    0x3F80_0000, // 1.0
    0xBF80_0000, // -1.0
    0x7F7F_FFFF, // FLT_MAX
];

fn vals() -> Vec<f32> {
    VALS.iter().map(|&b| f32::from_bits(b)).collect()
}

fn quad<F: FnMut(f32, f32, f32, f32)>(mut f: F) {
    let v = vals();
    for &a in &v {
        for &b in &v {
            for &c in &v {
                for &d in &v {
                    f(a, b, c, d);
                }
            }
        }
    }
}

#[allow(dead_code)]
fn pair<F: FnMut(f32, f32)>(mut f: F) {
    let v = vals();
    for &a in &v {
        for &b in &v {
            f(a, b);
        }
    }
}

// ---------------------------------------------------------------------------
// Two-vector arithmetic: c2Dot, c2Det2, c2Add, c2Sub
// ---------------------------------------------------------------------------

#[test]
fn nan_c2dot() {
    let (c, r): (FnFvv, FnFvv) = sym(b"c2Dot");
    quad(|ax, ay, bx, by| {
        let a = C2v { x: ax, y: ay };
        let b = C2v { x: bx, y: by };
        assert_f32(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            &format!("c2Dot {} . {}", fmt_v(a), fmt_v(b)),
        );
    });
}

#[test]
fn nan_c2det2() {
    let (c, r): (FnFvv, FnFvv) = sym(b"c2Det2");
    quad(|ax, ay, bx, by| {
        let a = C2v { x: ax, y: ay };
        let b = C2v { x: bx, y: by };
        assert_f32(
            unsafe { c(a, b) },
            unsafe { r(a, b) },
            &format!("c2Det2 {} x {}", fmt_v(a), fmt_v(b)),
        );
    });
}

#[test]
fn nan_c2add_c2sub() {
    let (ca, ra): (FnVvv, FnVvv) = sym(b"c2Add");
    let (cs, rs): (FnVvv, FnVvv) = sym(b"c2Sub");
    quad(|ax, ay, bx, by| {
        let a = C2v { x: ax, y: ay };
        let b = C2v { x: bx, y: by };
        assert_v(
            unsafe { ca(a, b) },
            unsafe { ra(a, b) },
            &format!("c2Add {} + {}", fmt_v(a), fmt_v(b)),
        );
        assert_v(
            unsafe { cs(a, b) },
            unsafe { rs(a, b) },
            &format!("c2Sub {} - {}", fmt_v(a), fmt_v(b)),
        );
    });
}

// ---------------------------------------------------------------------------
// Rotations: c2Mulrv, c2MulrvT (the sign-of-NaN trap)
// ---------------------------------------------------------------------------

#[test]
fn nan_c2mulrv() {
    let (c, r): (FnMulrv, FnMulrv) = sym(b"c2Mulrv");
    quad(|rc, rs, bx, by| {
        let m = C2r { c: rc, s: rs };
        let b = C2v { x: bx, y: by };
        assert_v(
            unsafe { c(m, b) },
            unsafe { r(m, b) },
            &format!(
                "c2Mulrv rot=({},{}) v={}",
                fmt_f32(rc),
                fmt_f32(rs),
                fmt_v(b)
            ),
        );
    });
}

#[test]
fn nan_c2mulrvt() {
    let (c, r): (FnMulrv, FnMulrv) = sym(b"c2MulrvT");
    quad(|rc, rs, bx, by| {
        let m = C2r { c: rc, s: rs };
        let b = C2v { x: bx, y: by };
        assert_v(
            unsafe { c(m, b) },
            unsafe { r(m, b) },
            &format!(
                "c2MulrvT rot=({},{}) v={}",
                fmt_f32(rc),
                fmt_f32(rs),
                fmt_v(b)
            ),
        );
    });
}

#[test]
fn nan_c2mulxv() {
    let (c, r): (FnMulxv, FnMulxv) = sym(b"c2Mulxv");
    // c2Mulxv takes 6 floats; sweep the rotation+vector cross product with a
    // rotating selection of translations so the total stays bounded.
    let v = vals();
    let mut k = 0usize;
    quad(|rc, rs, bx, by| {
        let t = C2v {
            x: v[k % v.len()],
            y: v[(k / v.len()) % v.len()],
        };
        k += 1;
        let x = C2x {
            p: t,
            r: C2r { c: rc, s: rs },
        };
        let b = C2v { x: bx, y: by };
        assert_v(
            unsafe { c(x, b) },
            unsafe { r(x, b) },
            &format!(
                "c2Mulxv rot=({},{}) t={} v={}",
                fmt_f32(rc),
                fmt_f32(rs),
                fmt_v(t),
                fmt_v(b)
            ),
        );
    });
}

// ---------------------------------------------------------------------------
// Scalar-vector ops: c2Mulvs, c2Div, c2Len, c2Norm and the unary sign helpers
// ---------------------------------------------------------------------------

#[test]
fn nan_scalar_vector_ops() {
    let (cmv, rmv): (FnVvf, FnVvf) = sym(b"c2Mulvs");
    let (cdv, rdv): (FnVvf, FnVvf) = sym(b"c2Div");
    let (cln, rln): (FnFv, FnFv) = sym(b"c2Len");
    let (cnm, rnm): (FnVv, FnVv) = sym(b"c2Norm");
    let (cng, rng_): (FnVv, FnVv) = sym(b"c2Neg");
    let (csk, rsk): (FnVv, FnVv) = sym(b"c2Skew");
    let (ccw, rcw): (FnVv, FnVv) = sym(b"c2CCW90");
    let (cv, rv): (FnV2, FnV2) = sym(b"c2V");
    let v = vals();
    for &ax in &v {
        for &ay in &v {
            let a = C2v { x: ax, y: ay };
            for &b in &v {
                assert_v(
                    unsafe { cmv(a, b) },
                    unsafe { rmv(a, b) },
                    &format!("c2Mulvs {} * {}", fmt_v(a), fmt_f32(b)),
                );
                assert_v(
                    unsafe { cdv(a, b) },
                    unsafe { rdv(a, b) },
                    &format!("c2Div {} / {}", fmt_v(a), fmt_f32(b)),
                );
            }
            assert_f32(
                unsafe { cln(a) },
                unsafe { rln(a) },
                &format!("c2Len {}", fmt_v(a)),
            );
            assert_v(
                unsafe { cnm(a) },
                unsafe { rnm(a) },
                &format!("c2Norm {}", fmt_v(a)),
            );
            assert_v(unsafe { cng(a) }, unsafe { rng_(a) }, "c2Neg");
            assert_v(unsafe { csk(a) }, unsafe { rsk(a) }, "c2Skew");
            assert_v(unsafe { ccw(a) }, unsafe { rcw(a) }, "c2CCW90");
            assert_v(unsafe { cv(ax, ay) }, unsafe { rv(ax, ay) }, "c2V");
        }
    }
}

// ---------------------------------------------------------------------------
// min / max / clamp NaN ordering (ERRORS.md E71..E77)
// ---------------------------------------------------------------------------

#[test]
fn nan_minmax_clamp() {
    let (cx, rx): (FnVvv, FnVvv) = sym(b"c2Maxv");
    let (cm, rm): (FnVvv, FnVvv) = sym(b"c2Minv");
    let (cc, rc): (FnVvvv, FnVvvv) = sym(b"c2Clampv");
    quad(|ax, ay, bx, by| {
        let a = C2v { x: ax, y: ay };
        let b = C2v { x: bx, y: by };
        assert_v(
            unsafe { cx(a, b) },
            unsafe { rx(a, b) },
            &format!("c2Maxv {} {}", fmt_v(a), fmt_v(b)),
        );
        assert_v(
            unsafe { cm(a, b) },
            unsafe { rm(a, b) },
            &format!("c2Minv {} {}", fmt_v(a), fmt_v(b)),
        );
    });
    // clamp has three vector arguments; sweep lo/hi fully with a rotating `a`.
    let v = vals();
    let mut k = 0usize;
    quad(|lx, ly, hx, hy| {
        let a = C2v {
            x: v[k % v.len()],
            y: v[(k / v.len()) % v.len()],
        };
        k += 1;
        let lo = C2v { x: lx, y: ly };
        let hi = C2v { x: hx, y: hy };
        assert_v(
            unsafe { cc(a, lo, hi) },
            unsafe { rc(a, lo, hi) },
            &format!("c2Clampv a={} lo={} hi={}", fmt_v(a), fmt_v(lo), fmt_v(hi)),
        );
    });
}

// ---------------------------------------------------------------------------
// Simplex-level NaN propagation: c22 / c23 / c2D / c2L / c2Witness / metric
// ---------------------------------------------------------------------------

fn simplex_with(pts: [C2v; 3], count: i32, div: f32, us: [f32; 3]) -> C2Simplex {
    let mut s = C2Simplex::default();
    for i in 0..4 {
        let p = if i < 3 { pts[i] } else { C2v::default() };
        s.verts[i] = C2sv {
            sA: C2v {
                x: p.x,
                y: f32::from_bits(0x7FC0_0007),
            },
            sB: C2v {
                x: f32::from_bits(0xFFC0_0008),
                y: p.y,
            },
            p,
            u: if i < 3 { us[i] } else { 0.0 },
            iA: i as i32,
            iB: 3 - i as i32,
        };
    }
    s.div = div;
    s.count = count;
    s
}

#[test]
fn nan_simplex_ops() {
    let (c22c, c22r): (FnSimplex, FnSimplex) = sym(b"c22");
    let (c23c, c23r): (FnSimplex, FnSimplex) = sym(b"c23");
    let (cdc, cdr): (FnVSimplex, FnVSimplex) = sym(b"c2D");
    let (clc, clr): (FnVSimplex, FnVSimplex) = sym(b"c2L");
    let (cmc, cmr): (FnFSimplex, FnFSimplex) = sym(b"c2GJKSimplexMetric");
    let (cwc, cwr): (FnWitness, FnWitness) = sym(b"c2Witness");

    let v = vals();
    // Sweep the three simplex points over the special table (a rotating window
    // keeps the count sane) plus every `div` and `u` special.
    let mut k = 0usize;
    for &p0x in &v {
        for &p0y in &v {
            for &p1x in &v {
                for &p1y in &v {
                    let p2 = C2v {
                        x: v[k % v.len()],
                        y: v[(k / v.len()) % v.len()],
                    };
                    let div = v[(k / (v.len() * v.len())) % v.len()];
                    let us = [
                        v[(k + 1) % v.len()],
                        v[(k + 5) % v.len()],
                        v[(k + 7) % v.len()],
                    ];
                    k += 1;
                    let pts = [C2v { x: p0x, y: p0y }, C2v { x: p1x, y: p1y }, p2];

                    for count in [1i32, 2, 3, 4, 0, -1] {
                        let base = simplex_with(pts, count, div, us);

                        let mut sc = base;
                        let mut sr = base;
                        unsafe { c22c(&mut sc) };
                        unsafe { c22r(&mut sr) };
                        assert_raw(&sc, &sr, "c22 NaN sweep");

                        let mut sc = base;
                        let mut sr = base;
                        unsafe { c23c(&mut sc) };
                        unsafe { c23r(&mut sr) };
                        assert_raw(&sc, &sr, "c23 NaN sweep");

                        let mut sc = base;
                        let mut sr = base;
                        let dc = unsafe { cdc(&mut sc) };
                        let dr = unsafe { cdr(&mut sr) };
                        assert_v(dc, dr, "c2D NaN sweep");
                        assert_raw(&sc, &sr, "c2D must not modify the simplex");

                        let mut sc = base;
                        let mut sr = base;
                        let lc = unsafe { clc(&mut sc) };
                        let lr = unsafe { clr(&mut sr) };
                        assert_v(lc, lr, "c2L NaN sweep");

                        let mut sc = base;
                        let mut sr = base;
                        let mc = unsafe { cmc(&mut sc) };
                        let mr = unsafe { cmr(&mut sr) };
                        assert_f32(mc, mr, "c2GJKSimplexMetric NaN sweep");

                        let mut sc = base;
                        let mut sr = base;
                        let (mut ac, mut bc) = (C2v::default(), C2v::default());
                        let (mut ar, mut br) = (C2v::default(), C2v::default());
                        unsafe { cwc(&mut sc, &mut ac, &mut bc) };
                        unsafe { cwr(&mut sr, &mut ar, &mut br) };
                        assert_v(ac, ar, "c2Witness a NaN sweep");
                        assert_v(bc, br, "c2Witness b NaN sweep");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// c2Support with NaN vertices (ERRORS.md E64..E66)
// ---------------------------------------------------------------------------

#[test]
fn nan_c2support() {
    let (c, r): (FnSupport, FnSupport) = sym(b"c2Support");
    let v = vals();
    for &vx in &v {
        for &vy in &v {
            for &dx in &v {
                for &dy in &v {
                    // 4 verts: one all-special, the rest deterministic.
                    let verts = [
                        C2v { x: vx, y: vy },
                        C2v { x: 1.0, y: 2.0 },
                        C2v { x: vy, y: vx },
                        C2v { x: -3.0, y: 0.5 },
                    ];
                    let d = C2v { x: dx, y: dy };
                    for count in [0i32, 1, 2, 3, 4, -1] {
                        let cc = unsafe { c(verts.as_ptr(), count, d) };
                        let rr = unsafe { r(verts.as_ptr(), count, d) };
                        assert_eq!(
                            cc, rr,
                            "c2Support count={count} verts[0]={} d={}",
                            fmt_v(verts[0]),
                            fmt_v(d)
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// c2BBVerts must copy NaN payloads verbatim (ERRORS.md E70)
// ---------------------------------------------------------------------------

#[test]
fn nan_c2bbverts() {
    let (c, r): (FnBBVerts, FnBBVerts) = sym(b"c2BBVerts");
    quad(|ax, ay, bx, by| {
        let mut bb = C2AABB {
            min: C2v { x: ax, y: ay },
            max: C2v { x: bx, y: by },
        };
        let mut oc = [C2v { x: 9.0, y: 9.0 }; 4];
        let mut or_ = oc;
        unsafe { c(oc.as_mut_ptr(), &mut bb) };
        unsafe { r(or_.as_mut_ptr(), &mut bb) };
        assert_raw(&oc, &or_, "c2BBVerts NaN sweep");
    });
}

// ---------------------------------------------------------------------------
// c2GJK's own inline arithmetic:
//   * `dist > rA + rB`      -> `addss rA, rB`
//   * `dist -= rA + rB`     -> `subss dist, (rA+rB)`
//   * `max_metric * 2.0f`   -> gcc emits `addss m, m`
// All three take operands that can be NaN with independent payloads (the shape
// radii and the cached metric come straight from the caller), so the
// destination-register choice is observable here too.
// ---------------------------------------------------------------------------

#[test]
fn nan_gjk_radius_and_metric_arithmetic() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let v = vals();
    let mut checked = 0u32;
    for &ra in &v {
        for &rb in &v {
            // Circles: the proxy radius is exactly the caller's `r`, so `rA + rB`
            // is computed on these two bit patterns directly.
            for &(px, py) in &[(0.0f32, 0.0f32), (100.0, 0.0), (1e-7, 0.0), (-3.5, 7.25)] {
                let a = ShapeBlob::circle(C2Circle {
                    p: C2v { x: 0.0, y: 0.0 },
                    r: ra,
                });
                let b = ShapeBlob::circle(C2Circle {
                    p: C2v { x: px, y: py },
                    r: rb,
                });
                for ur in [0i32, 1] {
                    for metric in &v {
                        // A warm 1-vertex cache exercises the metric comparison
                        // (`min_metric < max_metric * 2.0f && metric < -1e8f`).
                        let base = C2GJKCache {
                            metric: *metric,
                            count: 1,
                            iA: [0, 0, 0],
                            iB: [0, 0, 0],
                            div: 1.0,
                        };
                        let mut cc = base;
                        let mut rc = base;
                        let (mut ca, mut cb, mut ci) = (C2v::default(), C2v::default(), -1i32);
                        let (mut ra_, mut rb_, mut ri) = (C2v::default(), C2v::default(), -1i32);
                        let dc = unsafe {
                            c(
                                a.as_ptr(),
                                C2_TYPE_CIRCLE,
                                std::ptr::null(),
                                b.as_ptr(),
                                C2_TYPE_CIRCLE,
                                std::ptr::null(),
                                &mut ca,
                                &mut cb,
                                ur,
                                &mut ci,
                                &mut cc,
                            )
                        };
                        let dr = unsafe {
                            r(
                                a.as_ptr(),
                                C2_TYPE_CIRCLE,
                                std::ptr::null(),
                                b.as_ptr(),
                                C2_TYPE_CIRCLE,
                                std::ptr::null(),
                                &mut ra_,
                                &mut rb_,
                                ur,
                                &mut ri,
                                &mut rc,
                            )
                        };
                        let ctx = format!(
                            "c2GJK rA={} rB={} metric={} ur={ur} centre=({px},{py})",
                            fmt_f32(ra),
                            fmt_f32(rb),
                            fmt_f32(*metric)
                        );
                        assert_f32(dc, dr, &ctx);
                        assert_v(ca, ra_, &ctx);
                        assert_v(cb, rb_, &ctx);
                        assert_eq!(ci, ri, "iterations [{ctx}]");
                        assert_raw(&cc, &rc, &ctx);
                        checked += 1;
                    }
                }
            }
        }
    }
    println!("nan_gjk: {checked} configurations compared bit-for-bit");
}

// ---------------------------------------------------------------------------
// gjk_cache with NaN-payload parameters: the only observable contract is "does
// not write a9/b9 and does not trap", checked here for the special table.
// ---------------------------------------------------------------------------

#[test]
fn nan_gjk_cache_entry() {
    let (c, r): (FnGjkCache, FnGjkCache) = sym(b"gjk_cache");
    let v = vals();
    for (i, &p0) in v.iter().enumerate() {
        for &p1 in &v {
            let mut p = [0.0f32; 9];
            for (k, slot) in p.iter_mut().enumerate() {
                *slot = if k % 2 == 0 { p0 } else { p1 };
            }
            for rev in [0i8, 1, -1, 2] {
                let s = C2v {
                    x: f32::from_bits(0x1BAD_C0DE),
                    y: f32::from_bits(0xFACE_B00C),
                };
                let (mut ca, mut cb) = (s, s);
                let (mut ra, mut rb) = (s, s);
                unsafe {
                    c(
                        rev as core::ffi::c_char,
                        &mut ca,
                        &mut cb,
                        p[0],
                        p[1],
                        p[2],
                        p[3],
                        p[4],
                        p[5],
                        p[6],
                        p[7],
                        p[8],
                    )
                };
                unsafe {
                    r(
                        rev as core::ffi::c_char,
                        &mut ra,
                        &mut rb,
                        p[0],
                        p[1],
                        p[2],
                        p[3],
                        p[4],
                        p[5],
                        p[6],
                        p[7],
                        p[8],
                    )
                };
                assert!(v_same(ca, s) && v_same(cb, s), "C wrote a9/b9 (#{i})");
                assert!(v_same(ra, s) && v_same(rb, s), "Rust wrote a9/b9 (#{i})");
            }
        }
    }
}
