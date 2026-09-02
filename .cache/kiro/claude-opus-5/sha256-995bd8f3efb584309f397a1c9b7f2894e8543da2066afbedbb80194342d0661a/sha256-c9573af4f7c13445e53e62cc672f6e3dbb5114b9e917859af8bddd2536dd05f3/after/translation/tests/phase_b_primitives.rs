//! Phase B — rows 1..29 of CONFIGS.md: vector/rotation primitives, poly &
//! AABB helpers, `c2MakeProxy` over all four enum values, and the low-level
//! simplex solvers driven directly.
//!
//! Every call goes through `libloading` into the two `.so` files.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 4000;

// ===========================================================================
// Row 1 - c2V, c2Add, c2Sub, c2Mulvs, c2Neg, c2Skew, c2CCW90
// ===========================================================================

#[test]
fn row01_vector_basics() {
    let p = pair();
    let (cV, rV) = p.get::<FnFFV>(b"c2V");
    let (cAdd, rAdd) = p.get::<FnVVV>(b"c2Add");
    let (cSub, rSub) = p.get::<FnVVV>(b"c2Sub");
    let (cMul, rMul) = p.get::<FnVFV>(b"c2Mulvs");
    let (cNeg, rNeg) = p.get::<FnVV>(b"c2Neg");
    let (cSkew, rSkew) = p.get::<FnVV>(b"c2Skew");
    let (cCCW, rCCW) = p.get::<FnVV>(b"c2CCW90");

    let mut rng = Rng::new(0x1001);
    for i in 0..N {
        // mix of tame, arbitrary-bit-pattern and pathological values
        let (a, b, s) = match i % 3 {
            0 => (rng.vec_sym(100.0), rng.vec_sym(100.0), rng.sym(10.0)),
            1 => (rng.vec_any(), rng.vec_any(), rng.any_f32()),
            _ => (rng.vec_spicy(), rng.vec_spicy(), rng.spicy()),
        };
        unsafe {
            same("c2V", &cV(a.x, a.y), &rV(a.x, a.y));
            same("c2Add", &cAdd(a, b), &rAdd(a, b));
            same("c2Sub", &cSub(a, b), &rSub(a, b));
            same("c2Mulvs", &cMul(a, s), &rMul(a, s));
            same("c2Neg", &cNeg(a), &rNeg(a));
            same("c2Skew", &cSkew(a), &rSkew(a));
            same("c2CCW90", &cCCW(a), &rCCW(a));
        }
    }
}

// ===========================================================================
// Row 2 - c2Dot, c2Det2, c2Len
// ===========================================================================

#[test]
fn row02_dot_det_len() {
    let p = pair();
    let (cDot, rDot) = p.get::<FnVVF>(b"c2Dot");
    let (cDet, rDet) = p.get::<FnVVF>(b"c2Det2");
    let (cLen, rLen) = p.get::<FnVF>(b"c2Len");

    let mut rng = Rng::new(0x1002);
    for i in 0..N {
        let (a, b) = match i % 4 {
            0 => (rng.vec_sym(1.0), rng.vec_sym(1.0)),
            1 => (rng.vec_sym(1e18), rng.vec_sym(1e18)),
            2 => (rng.vec_any(), rng.vec_any()),
            _ => (rng.vec_spicy(), rng.vec_spicy()),
        };
        unsafe {
            same("c2Dot", &cDot(a, b), &rDot(a, b));
            same("c2Det2", &cDet(a, b), &rDet(a, b));
            same("c2Len", &cLen(a), &rLen(a));
            same("c2Len", &cLen(b), &rLen(b));
        }
    }
}

// ===========================================================================
// Rows 3, 4, 5 - c2Maxv / c2Minv / c2Clampv / c2Absv
// ===========================================================================

#[test]
fn row03_04_05_minmax_clamp_abs() {
    let p = pair();
    let (cMax, rMax) = p.get::<FnVVV>(b"c2Maxv");
    let (cMin, rMin) = p.get::<FnVVV>(b"c2Minv");
    let (cClamp, rClamp) = p.get::<FnVVVV>(b"c2Clampv");
    let (cAbs, rAbs) = p.get::<FnVV>(b"c2Absv");

    // Explicit sign-of-zero and NaN pairs: the C uses `a > b ? a : b`, which is
    // NOT fmaxf, so +0/-0 and NaN ordering must be reproduced exactly.
    let zn: [c2v; 6] = [
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v {
            x: f32::NAN,
            y: 0.0,
        },
        c2v {
            x: 0.0,
            y: f32::NAN,
        },
        c2v {
            x: f32::NAN,
            y: f32::NAN,
        },
        c2v { x: -0.0, y: -0.0 },
    ];
    for a in zn {
        for b in zn {
            unsafe {
                same("c2Maxv 0/NaN", &cMax(a, b), &rMax(a, b));
                same("c2Minv 0/NaN", &cMin(a, b), &rMin(a, b));
                same("c2Absv 0/NaN", &cAbs(a), &rAbs(a));
                for c in zn {
                    same("c2Clampv 0/NaN", &cClamp(a, b, c), &rClamp(a, b, c));
                }
            }
        }
    }

    let mut rng = Rng::new(0x1003);
    for i in 0..N {
        let (a, lo, hi) = match i % 4 {
            // inside / below / above a proper range
            0 => {
                let lo = rng.vec_sym(10.0);
                let hi = c2v {
                    x: lo.x + rng.unit() * 20.0,
                    y: lo.y + rng.unit() * 20.0,
                };
                (rng.vec_sym(20.0), lo, hi)
            }
            // INVERTED range: lo > hi
            1 => {
                let hi = rng.vec_sym(10.0);
                let lo = c2v {
                    x: hi.x + rng.unit() * 20.0,
                    y: hi.y + rng.unit() * 20.0,
                };
                (rng.vec_sym(20.0), lo, hi)
            }
            2 => (rng.vec_any(), rng.vec_any(), rng.vec_any()),
            _ => (rng.vec_spicy(), rng.vec_spicy(), rng.vec_spicy()),
        };
        unsafe {
            same("c2Maxv", &cMax(a, lo), &rMax(a, lo));
            same("c2Minv", &cMin(a, hi), &rMin(a, hi));
            same("c2Clampv", &cClamp(a, lo, hi), &rClamp(a, lo, hi));
            same("c2Absv", &cAbs(a), &rAbs(a));
        }
    }
}

// ===========================================================================
// Rows 6, 7, 8 - c2Div, c2Norm, c2Intersect, c2Dist
// ===========================================================================

#[test]
fn row06_07_08_div_norm_intersect_dist() {
    let p = pair();
    let (cDiv, rDiv) = p.get::<FnVFV>(b"c2Div");
    let (cNorm, rNorm) = p.get::<FnVV>(b"c2Norm");
    let (cInt, rInt) = p.get::<FnIntersect>(b"c2Intersect");
    let (cDist, rDist) = p.get::<FnHVF>(b"c2Dist");

    // hand-picked degeneracies for rows 6 and 7
    let zero = c2v { x: 0.0, y: 0.0 };
    unsafe {
        same("c2Div by 0", &cDiv(c2v { x: 1.0, y: 2.0 }, 0.0), &rDiv(c2v { x: 1.0, y: 2.0 }, 0.0));
        same("c2Div by -0", &cDiv(c2v { x: 1.0, y: 2.0 }, -0.0), &rDiv(c2v { x: 1.0, y: 2.0 }, -0.0));
        same("c2Norm(0)", &cNorm(zero), &rNorm(zero));
        // da == db => division by zero inside c2Intersect
        same(
            "c2Intersect da==db",
            &cInt(zero, c2v { x: 1.0, y: 1.0 }, 3.0, 3.0),
            &rInt(zero, c2v { x: 1.0, y: 1.0 }, 3.0, 3.0),
        );
        same(
            "c2Intersect da==0",
            &cInt(zero, c2v { x: 1.0, y: 1.0 }, 0.0, 5.0),
            &rInt(zero, c2v { x: 1.0, y: 1.0 }, 0.0, 5.0),
        );
    }

    let mut rng = Rng::new(0x1006);
    for i in 0..N {
        let (a, b) = match i % 4 {
            0 => (rng.vec_sym(10.0), rng.vec_sym(10.0)),
            1 => (rng.vec_sym(1e-30), rng.vec_sym(1e30)),
            2 => (rng.vec_any(), rng.vec_any()),
            _ => (rng.vec_spicy(), rng.vec_spicy()),
        };
        let (da, db) = match i % 3 {
            0 => (rng.sym(5.0), rng.sym(5.0)),
            1 => {
                let v = rng.sym(5.0);
                (v, v) // exact tie
            }
            _ => (rng.spicy(), rng.spicy()),
        };
        let s = if i % 5 == 0 { 0.0 } else { rng.sym(10.0) };
        let h = c2h { n: a, d: da };
        unsafe {
            same("c2Div", &cDiv(a, s), &rDiv(a, s));
            same("c2Norm", &cNorm(a), &rNorm(a));
            same("c2Intersect", &cInt(a, b, da, db), &rInt(a, b, da, db));
            same("c2Dist", &cDist(h, b), &rDist(h, b));
        }
    }
}

// ===========================================================================
// Rows 9, 10, 11 - identities, rotations, transforms
// ===========================================================================

#[test]
fn row09_10_11_rot_and_transform() {
    let p = pair();
    let (cRotI, rRotI) = p.get::<FnR>(b"c2RotIdentity");
    let (cXI, rXI) = p.get::<FnX>(b"c2xIdentity");
    let (cMulrv, rMulrv) = p.get::<FnRVV>(b"c2Mulrv");
    let (cMulrvT, rMulrvT) = p.get::<FnRVV>(b"c2MulrvT");
    let (cMulxv, rMulxv) = p.get::<FnXVV>(b"c2Mulxv");
    let (cMulxvT, rMulxvT) = p.get::<FnXVV>(b"c2MulxvT");

    unsafe {
        same("c2RotIdentity", &cRotI(), &rRotI());
        same("c2xIdentity", &cXI(), &rXI());
    }

    let mut rng = Rng::new(0x1009);
    for i in 0..N {
        let r = match i % 5 {
            0 => c2r { c: 1.0, s: 0.0 },      // identity
            1 => rng.rot_unit(),               // proper unit rotation
            2 => rng.rot_raw(3.0),             // NOT normalized
            3 => c2r { c: 0.0, s: 0.0 },       // zero rotation
            _ => c2r {
                c: rng.spicy(),
                s: rng.spicy(),
            },
        };
        let x = match i % 4 {
            0 => c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r,
            }, // rotation only
            1 => c2x {
                p: rng.vec_sym(50.0),
                r: c2r { c: 1.0, s: 0.0 },
            }, // translation only
            2 => c2x {
                p: rng.vec_sym(50.0),
                r,
            }, // both
            _ => c2x {
                p: rng.vec_spicy(),
                r,
            },
        };
        let v = if i % 7 == 0 {
            rng.vec_spicy()
        } else {
            rng.vec_sym(100.0)
        };
        unsafe {
            same("c2Mulrv", &cMulrv(r, v), &rMulrv(r, v));
            same("c2MulrvT", &cMulrvT(r, v), &rMulrvT(r, v));
            same("c2Mulxv", &cMulxv(x, v), &rMulxv(x, v));
            same("c2MulxvT", &cMulxvT(x, v), &rMulxvT(x, v));
        }
    }
}

// ===========================================================================
// Row 12 - c2BBVerts
// ===========================================================================

#[test]
fn row12_bbverts() {
    let p = pair();
    let (cf, rf) = p.get::<FnBBVerts>(b"c2BBVerts");
    let mut rng = Rng::new(0x1012);
    for i in 0..N {
        let mut bb = match i % 4 {
            0 => {
                let min = rng.vec_sym(20.0);
                c2AABB {
                    min,
                    max: c2v {
                        x: min.x + rng.unit() * 20.0,
                        y: min.y + rng.unit() * 20.0,
                    },
                }
            }
            1 => {
                let v = rng.vec_sym(20.0);
                c2AABB { min: v, max: v } // degenerate
            }
            2 => {
                let max = rng.vec_sym(20.0);
                c2AABB {
                    min: c2v {
                        x: max.x + rng.unit() * 20.0,
                        y: max.y + rng.unit() * 20.0,
                    },
                    max,
                } // inverted
            }
            _ => c2AABB {
                min: rng.vec_spicy(),
                max: rng.vec_spicy(),
            },
        };
        let mut co = [c2v::default(); 4];
        let mut ro = [c2v::default(); 4];
        unsafe {
            cf(co.as_mut_ptr(), &mut bb);
            rf(ro.as_mut_ptr(), &mut bb);
        }
        same("c2BBVerts", &co, &ro);
    }
}

// ===========================================================================
// Rows 13, 14, 15 - c2PlaneAt, c2Norms, c2Support
// ===========================================================================

#[test]
fn row13_14_15_poly_helpers() {
    let p = pair();
    let (cPlane, rPlane) = p.get::<FnPolyIH>(b"c2PlaneAt");
    let (cNorms, rNorms) = p.get::<FnNorms>(b"c2Norms");
    let (cSup, rSup) = p.get::<FnSupport>(b"c2Support");

    let mut rng = Rng::new(0x1013);
    for i in 0..N {
        // Row 14: c2Norms over count 0..8, convex CCW / CW / duplicate verts.
        let count = (i % 9) as c_int;
        let mut verts = [c2v::default(); 8];
        let n = count.max(0) as usize;
        match i % 4 {
            0 => {
                // convex CCW
                let mut angs: Vec<f32> =
                    (0..n).map(|_| rng.unit() * std::f32::consts::TAU).collect();
                angs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                for k in 0..n {
                    let r = 1.0 + rng.unit() * 5.0;
                    verts[k] = c2v {
                        x: r * angs[k].cos(),
                        y: r * angs[k].sin(),
                    };
                }
            }
            1 => {
                // convex CW (reverse winding)
                let mut angs: Vec<f32> =
                    (0..n).map(|_| rng.unit() * std::f32::consts::TAU).collect();
                angs.sort_by(|a, b| b.partial_cmp(a).unwrap());
                for k in 0..n {
                    verts[k] = c2v {
                        x: 3.0 * angs[k].cos(),
                        y: 3.0 * angs[k].sin(),
                    };
                }
            }
            2 => {
                // duplicate consecutive vertices => NaN normal
                for k in 0..n {
                    verts[k] = rng.vec_sym(5.0);
                }
                if n >= 2 {
                    let j = rng.below(n - 1);
                    verts[j + 1] = verts[j];
                }
            }
            _ => {
                for k in 0..8 {
                    verts[k] = rng.vec_spicy();
                }
            }
        }

        let mut cn = [c2v::default(); 8];
        let mut rn = [c2v::default(); 8];
        // pre-poison the outputs so an untouched slot is detected
        for k in 0..8 {
            let v = c2v {
                x: 12.5 + k as f32,
                y: -33.25 - k as f32,
            };
            cn[k] = v;
            rn[k] = v;
        }
        let mut cv = verts;
        let mut rv = verts;
        unsafe {
            cNorms(cv.as_mut_ptr(), cn.as_mut_ptr(), count);
            rNorms(rv.as_mut_ptr(), rn.as_mut_ptr(), count);
        }
        same("c2Norms out", &cn, &rn);
        same("c2Norms verts untouched", &cv, &rv);

        // Row 13: c2PlaneAt over every in-range index of a fully-populated poly.
        let mut poly = c2Poly::default();
        poly.count = if count == 0 { 1 } else { count };
        for k in 0..8 {
            poly.verts[k] = if i % 4 == 3 {
                rng.vec_spicy()
            } else {
                rng.vec_sym(9.0)
            };
        }
        unsafe {
            cNorms(poly.verts.as_mut_ptr(), poly.norms.as_mut_ptr(), 8);
        }
        for idx in 0..8 as c_int {
            unsafe {
                same(
                    "c2PlaneAt",
                    &cPlane(&poly, idx),
                    &rPlane(&poly, idx),
                );
            }
        }

        // Row 15: c2Support over count 1..8 plus zero and NaN directions.
        let sc = if count == 0 { 1 } else { count };
        let d = match i % 5 {
            0 => rng.vec_sym(1.0),
            1 => c2v { x: 0.0, y: 0.0 },
            2 => rng.vec_spicy(),
            3 => c2v {
                x: f32::NAN,
                y: f32::NAN,
            },
            _ => rng.vec_sym(1e20),
        };
        unsafe {
            same(
                "c2Support",
                &cSup(poly.verts.as_ptr(), sc, d),
                &rSup(poly.verts.as_ptr(), sc, d),
            );
            // count == 0 (still dereferences verts[0], returns 0)
            same(
                "c2Support count=0",
                &cSup(poly.verts.as_ptr(), 0, d),
                &rSup(poly.verts.as_ptr(), 0, d),
            );
            // negative count
            same(
                "c2Support count<0",
                &cSup(poly.verts.as_ptr(), -3, d),
                &rSup(poly.verts.as_ptr(), -3, d),
            );
        }
    }
}

// ===========================================================================
// Rows 16, 17, 18, 19 - c2MakeProxy across all four C2_TYPE values.
// The poly case (19) writes nothing, so the proxy is pre-poisoned.
// ===========================================================================

#[test]
fn row16_19_make_proxy() {
    let p = pair();
    let (cf, rf) = p.get::<FnMakeProxy>(b"c2MakeProxy");
    let mut rng = Rng::new(0x1016);

    fn poison_proxy(tag: u8) -> c2Proxy {
        let mut px = c2Proxy::default();
        let q = &mut px as *mut c2Proxy as *mut u8;
        unsafe {
            for i in 0..std::mem::size_of::<c2Proxy>() {
                *q.add(i) = tag.wrapping_mul(31).wrapping_add(i as u8) | 1;
            }
        }
        px
    }

    for i in 0..N {
        let tag = (i & 0xff) as u8;

        // --- circle (row 16)
        let circle = c2Circle {
            p: if i % 3 == 0 {
                rng.vec_spicy()
            } else {
                rng.vec_sym(30.0)
            },
            r: if i % 5 == 0 { rng.spicy() } else { rng.sym(10.0) },
        };
        // --- aabb (row 17): normal / degenerate / inverted / spicy
        let aabb = match i % 4 {
            0 => {
                let min = rng.vec_sym(30.0);
                c2AABB {
                    min,
                    max: c2v {
                        x: min.x + rng.unit() * 10.0,
                        y: min.y + rng.unit() * 10.0,
                    },
                }
            }
            1 => {
                let v = rng.vec_sym(30.0);
                c2AABB { min: v, max: v }
            }
            2 => {
                let max = rng.vec_sym(30.0);
                c2AABB {
                    min: c2v {
                        x: max.x + 5.0,
                        y: max.y + 5.0,
                    },
                    max,
                }
            }
            _ => c2AABB {
                min: rng.vec_spicy(),
                max: rng.vec_spicy(),
            },
        };
        // --- capsule (row 18): incl. a == b
        let ca = rng.vec_sym(30.0);
        let capsule = c2Capsule {
            a: ca,
            b: if i % 6 == 0 { ca } else { rng.vec_sym(30.0) },
            r: if i % 5 == 1 { rng.spicy() } else { rng.sym(8.0) },
        };

        let shapes: [(c_int, *const c_void); 3] = [
            (C2_TYPE_CIRCLE, &circle as *const _ as *const c_void),
            (C2_TYPE_AABB, &aabb as *const _ as *const c_void),
            (C2_TYPE_CAPSULE, &capsule as *const _ as *const c_void),
        ];
        for (ty, ptr) in shapes {
            let mut cp = poison_proxy(tag);
            let mut rp = poison_proxy(tag);
            unsafe {
                cf(ptr, ty, &mut cp);
                rf(ptr, ty, &mut rp);
            }
            same(&format!("c2MakeProxy type={ty}"), &cp, &rp);
        }

        // --- row 19: POLY (3) plus out-of-range enum values: no case, so the
        //     proxy must come back exactly as it went in.
        let mut tys: Vec<c_int> = vec![C2_TYPE_POLY];
        tys.extend_from_slice(&BAD_TYPES);
        for ty in tys {
            let orig = poison_proxy(tag);
            let mut cp = orig;
            let mut rp = orig;
            unsafe {
                cf(&circle as *const _ as *const c_void, ty, &mut cp);
                rf(&circle as *const _ as *const c_void, ty, &mut rp);
            }
            same(&format!("c2MakeProxy bad type={ty}"), &cp, &rp);
            same(&format!("c2MakeProxy type={ty} untouched"), &orig, &cp);
        }
    }
}

// ===========================================================================
// Rows 20..29 - the simplex solvers, driven directly at the lowest level.
// ===========================================================================

/// Build a randomized simplex. `mode` selects the geometric family so that all
/// 7 branches of `c23` and all 3 of `c22` get hit.
fn rand_simplex(rng: &mut Rng, count: c_int, mode: usize, div: f32) -> c2Simplex {
    let mut s = c2Simplex::default();
    s.count = count;
    s.div = div;
    let scale = match mode % 5 {
        0 => 1.0,
        1 => 10.0,
        2 => 0.01,
        3 => 1e6,
        _ => 1.0,
    };
    for k in 0..4 {
        s.verts[k].sA = rng.vec_sym(scale);
        s.verts[k].sB = rng.vec_sym(scale);
        s.verts[k].u = rng.sym(scale);
        s.verts[k].iA = rng.below(8) as c_int;
        s.verts[k].iB = rng.below(8) as c_int;
        s.verts[k].p = rng.vec_sym(scale);
    }
    match mode % 7 {
        // origin far outside on one side
        0 => {
            let base = rng.vec_sym(1.0);
            let off = c2v { x: 30.0, y: 30.0 };
            for k in 0..4 {
                s.verts[k].p = c2v {
                    x: base.x + off.x + rng.sym(1.0),
                    y: base.y + off.y + rng.sym(1.0),
                };
            }
        }
        // triangle straddling the origin (interior case)
        1 => {
            s.verts[0].p = c2v { x: -5.0 + rng.sym(1.0), y: -5.0 + rng.sym(1.0) };
            s.verts[1].p = c2v { x: 6.0 + rng.sym(1.0), y: -4.0 + rng.sym(1.0) };
            s.verts[2].p = c2v { x: 0.0 + rng.sym(1.0), y: 7.0 + rng.sym(1.0) };
            s.verts[3].p = rng.vec_sym(5.0);
        }
        // collinear / degenerate (area == 0)
        2 => {
            let o = rng.vec_sym(3.0);
            let d = rng.vec_sym(2.0);
            for k in 0..4 {
                let t = k as f32;
                s.verts[k].p = c2v {
                    x: o.x + d.x * t,
                    y: o.y + d.y * t,
                };
            }
        }
        // all coincident
        3 => {
            let v = rng.vec_sym(3.0);
            for k in 0..4 {
                s.verts[k].p = v;
            }
        }
        // exactly on the origin
        4 => {
            s.verts[0].p = c2v { x: 0.0, y: 0.0 };
            s.verts[1].p = rng.vec_sym(3.0);
            s.verts[2].p = rng.vec_sym(3.0);
        }
        // grid-snapped: forces exact ties in the `<= 0` comparisons
        5 => {
            for k in 0..4 {
                s.verts[k].p = rng.vec_grid(1.0, 3);
            }
        }
        // pathological floats
        _ => {
            for k in 0..4 {
                s.verts[k].p = rng.vec_spicy();
            }
        }
    }
    s
}

#[test]
fn row20_21_c22() {
    let p = pair();
    let (cf, rf) = p.get::<FnSimplex>(b"c22");
    let mut rng = Rng::new(0x1020);
    for i in 0..N * 3 {
        let mut s = rand_simplex(&mut rng, 2, i, 1.0);
        // Row 21: degenerate a.p == b.p
        if i % 9 == 0 {
            s.verts[1].p = s.verts[0].p;
        }
        let mut cs = s;
        let mut rs = s;
        unsafe {
            cf(&mut cs);
            rf(&mut rs);
        }
        same("c22", &cs, &rs);
    }
}

#[test]
fn row22_23_24_c23() {
    let p = pair();
    let (cf, rf) = p.get::<FnSimplex>(b"c23");
    let mut rng = Rng::new(0x1022);
    let mut branch_counts = [0usize; 4]; // resulting count 0..3
    for i in 0..N * 6 {
        let mut s = rand_simplex(&mut rng, 3, i, 1.0);
        if i % 11 == 0 {
            // Row 24: exactly collinear (area == 0)
            let o = rng.vec_sym(3.0);
            let d = rng.vec_sym(2.0);
            for k in 0..3 {
                s.verts[k].p = c2v {
                    x: o.x + d.x * k as f32,
                    y: o.y + d.y * k as f32,
                };
            }
        }
        let mut cs = s;
        let mut rs = s;
        unsafe {
            cf(&mut cs);
            rf(&mut rs);
        }
        same("c23", &cs, &rs);
        if (0..4).contains(&cs.count) {
            branch_counts[cs.count as usize] += 1;
        }
    }
    // Row 23 coverage assertion: the interior branch (count stays 3) must be hit.
    assert!(
        branch_counts[3] > 0 && branch_counts[1] > 0 && branch_counts[2] > 0,
        "c23 branch coverage too thin: {branch_counts:?}"
    );
}

#[test]
fn row25_26_29_c2D_c2L_metric() {
    let p = pair();
    let (cD, rD) = p.get::<FnSimplexV>(b"c2D");
    let (cL, rL) = p.get::<FnSimplexV>(b"c2L");
    let (cM, rM) = p.get::<FnSimplexF>(b"c2GJKSimplexMetric");
    let mut rng = Rng::new(0x1025);
    // counts include out-of-range values (0, 4, negative, large)
    let counts: [c_int; 8] = [0, 1, 2, 3, 4, 5, -1, 99];
    let divs: [f32; 4] = [1.0, 0.0, -0.0, 7.5];
    for i in 0..N {
        for &count in &counts {
            for &div in &divs {
                let mut s = rand_simplex(&mut rng, count, i, div);
                if i % 13 == 0 {
                    s.div = rng.sym(100.0);
                }
                let mut a = s;
                let mut b = s;
                unsafe {
                    same("c2D", &cD(&mut a), &rD(&mut b));
                }
                same("c2D no-mutate", &a, &b);
                let mut a = s;
                let mut b = s;
                unsafe {
                    same("c2L", &cL(&mut a), &rL(&mut b));
                }
                same("c2L no-mutate", &a, &b);
                let mut a = s;
                let mut b = s;
                unsafe {
                    same("c2GJKSimplexMetric", &cM(&mut a), &rM(&mut b));
                }
                same("metric no-mutate", &a, &b);
            }
        }
    }
}

#[test]
fn row27_28_witness() {
    let p = pair();
    let (cf, rf) = p.get::<FnWitness>(b"c2Witness");
    let mut rng = Rng::new(0x1027);
    let counts: [c_int; 7] = [0, 1, 2, 3, 4, -1, 77];
    let divs: [f32; 5] = [1.0, 0.0, -0.0, 3.25, -2.5];
    for i in 0..N {
        for &count in &counts {
            for &div in &divs {
                let mut s = rand_simplex(&mut rng, count, i, div);
                if i % 17 == 0 {
                    for k in 0..4 {
                        s.verts[k].sA = rng.vec_spicy();
                        s.verts[k].sB = rng.vec_spicy();
                        s.verts[k].u = rng.spicy();
                    }
                }
                // pre-poison the outputs: the `default:` arm writes (0,0), but
                // the other arms must overwrite whatever was there.
                let poison = c2v { x: 1234.5, y: -678.25 };
                let (mut ca, mut cb) = (poison, poison);
                let (mut ra, mut rb) = (poison, poison);
                let mut cs = s;
                let mut rs = s;
                unsafe {
                    cf(&mut cs, &mut ca, &mut cb);
                    rf(&mut rs, &mut ra, &mut rb);
                }
                same("c2Witness a", &ca, &ra);
                same("c2Witness b", &cb, &rb);
                same("c2Witness no-mutate", &cs, &rs);
            }
        }
    }
}
