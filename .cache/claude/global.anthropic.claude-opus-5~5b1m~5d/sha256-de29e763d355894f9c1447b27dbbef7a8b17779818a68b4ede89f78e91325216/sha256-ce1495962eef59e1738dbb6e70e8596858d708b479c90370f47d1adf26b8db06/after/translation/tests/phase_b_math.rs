//! Phase B — valid-path differential tests, rows C1..C47 of CONFIGS.md
//! (vector primitives, proxies, support function, simplex machinery).
//!
//! Every call goes through the dynamically loaded `.so` exports of BOTH the C
//! and the Rust library; results are compared bit-for-bit.

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 4000;

// ---------------------------------------------------------------------------
// C1/C2 — c2V
// ---------------------------------------------------------------------------
#[test]
fn c1_c2_c2v() {
    let l = libs();
    let mut rng = Rng::new(0xC1);
    unsafe {
        for _ in 0..N {
            let (x, y) = (rng.coord(), rng.coord());
            eq_v("C1 c2V", (l.c.c2V)(x, y), (l.r.c2V)(x, y));
        }
        for _ in 0..N {
            let (x, y) = (rng.wild(), rng.wild());
            eq_v("C2 c2V wild", (l.c.c2V)(x, y), (l.r.c2V)(x, y));
        }
    }
}

// ---------------------------------------------------------------------------
// C3/C4 — c2Mulvs
// ---------------------------------------------------------------------------
#[test]
fn c3_c4_mulvs() {
    let l = libs();
    let mut rng = Rng::new(0xC3);
    unsafe {
        for _ in 0..N {
            let (a, b) = (rng.v(), rng.coord());
            eq_v("C3 c2Mulvs", (l.c.c2Mulvs)(a, b), (l.r.c2Mulvs)(a, b));
        }
        for _ in 0..N {
            let (a, b) = (rng.wild_v(), rng.wild());
            eq_v("C4 c2Mulvs wild", (l.c.c2Mulvs)(a, b), (l.r.c2Mulvs)(a, b));
        }
        for b in [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            for a in [
                c2v { x: 0.0, y: -0.0 },
                c2v {
                    x: f32::INFINITY,
                    y: f32::NEG_INFINITY,
                },
                c2v {
                    x: f32::NAN,
                    y: 1.0,
                },
                c2v {
                    x: f32::MAX,
                    y: f32::MIN_POSITIVE,
                },
            ] {
                eq_v("C4 c2Mulvs grid", (l.c.c2Mulvs)(a, b), (l.r.c2Mulvs)(a, b));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C5/C6 — c2Add / c2Sub
// ---------------------------------------------------------------------------
#[test]
fn c5_c6_add_sub() {
    let l = libs();
    let mut rng = Rng::new(0xC5);
    unsafe {
        for _ in 0..N {
            let (a, b) = (rng.v(), rng.v());
            eq_v("C5 c2Add", (l.c.c2Add)(a, b), (l.r.c2Add)(a, b));
            eq_v("C5 c2Sub", (l.c.c2Sub)(a, b), (l.r.c2Sub)(a, b));
            // cancellation to +/-0 and self-subtraction
            eq_v("C5 c2Sub self", (l.c.c2Sub)(a, a), (l.r.c2Sub)(a, a));
        }
        for _ in 0..N {
            let (a, b) = (rng.wild_v(), rng.wild_v());
            eq_v("C6 c2Add wild", (l.c.c2Add)(a, b), (l.r.c2Add)(a, b));
            eq_v("C6 c2Sub wild", (l.c.c2Sub)(a, b), (l.r.c2Sub)(a, b));
        }
        let big = c2v {
            x: f32::MAX,
            y: -f32::MAX,
        };
        eq_v("C5 overflow", (l.c.c2Add)(big, big), (l.r.c2Add)(big, big));
        let inf = c2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        };
        eq_v("C6 inf-inf", (l.c.c2Sub)(inf, inf), (l.r.c2Sub)(inf, inf));
        eq_v("C6 inf+(-inf)", (l.c.c2Add)(inf, (l.c.c2Neg)(inf)), (l.r.c2Add)(inf, (l.r.c2Neg)(inf)));
    }
}

// ---------------------------------------------------------------------------
// C7 — c2Maxv / c2Minv
// ---------------------------------------------------------------------------
#[test]
fn c7_maxv_minv() {
    let l = libs();
    let mut rng = Rng::new(0xC7);
    unsafe {
        for _ in 0..N {
            let (a, b) = (rng.v(), rng.v());
            eq_v("C7 c2Maxv", (l.c.c2Maxv)(a, b), (l.r.c2Maxv)(a, b));
            eq_v("C7 c2Minv", (l.c.c2Minv)(a, b), (l.r.c2Minv)(a, b));
            eq_v("C7 c2Maxv eq", (l.c.c2Maxv)(a, a), (l.r.c2Maxv)(a, a));
            eq_v("C7 c2Minv eq", (l.c.c2Minv)(a, a), (l.r.c2Minv)(a, a));
        }
        for _ in 0..N {
            let (a, b) = (rng.wild_v(), rng.wild_v());
            eq_v("C7 c2Maxv wild", (l.c.c2Maxv)(a, b), (l.r.c2Maxv)(a, b));
            eq_v("C7 c2Minv wild", (l.c.c2Minv)(a, b), (l.r.c2Minv)(a, b));
        }
        // +0 vs -0 (comparison is false, so `b` is selected in both branches)
        let z = c2v { x: 0.0, y: 0.0 };
        let nz = c2v { x: -0.0, y: -0.0 };
        eq_v("C7 max 0/-0", (l.c.c2Maxv)(z, nz), (l.r.c2Maxv)(z, nz));
        eq_v("C7 max -0/0", (l.c.c2Maxv)(nz, z), (l.r.c2Maxv)(nz, z));
        eq_v("C7 min 0/-0", (l.c.c2Minv)(z, nz), (l.r.c2Minv)(z, nz));
        eq_v("C7 min -0/0", (l.c.c2Minv)(nz, z), (l.r.c2Minv)(nz, z));
    }
}

// ---------------------------------------------------------------------------
// C8 — c2Clampv
// ---------------------------------------------------------------------------
#[test]
fn c8_clampv() {
    let l = libs();
    let mut rng = Rng::new(0xC8);
    unsafe {
        for _ in 0..N {
            let a = rng.v();
            let p = rng.v();
            let q = rng.v();
            let lo = c2v {
                x: p.x.min(q.x),
                y: p.y.min(q.y),
            };
            let hi = c2v {
                x: p.x.max(q.x),
                y: p.y.max(q.y),
            };
            eq_v("C8 in-range", (l.c.c2Clampv)(a, lo, hi), (l.r.c2Clampv)(a, lo, hi));
            // inverted lo/hi
            eq_v("C8 inverted", (l.c.c2Clampv)(a, hi, lo), (l.r.c2Clampv)(a, hi, lo));
            // exactly on the bounds
            eq_v("C8 on-lo", (l.c.c2Clampv)(lo, lo, hi), (l.r.c2Clampv)(lo, lo, hi));
            eq_v("C8 on-hi", (l.c.c2Clampv)(hi, lo, hi), (l.r.c2Clampv)(hi, lo, hi));
        }
        for _ in 0..N {
            let (a, lo, hi) = (rng.wild_v(), rng.wild_v(), rng.wild_v());
            eq_v("C8 wild", (l.c.c2Clampv)(a, lo, hi), (l.r.c2Clampv)(a, lo, hi));
        }
    }
}

// ---------------------------------------------------------------------------
// C9/C10 — c2Dot / c2Det2
// ---------------------------------------------------------------------------
#[test]
fn c9_c10_dot_det2() {
    let l = libs();
    let mut rng = Rng::new(0xC9);
    unsafe {
        for _ in 0..N {
            let (a, b) = (rng.v(), rng.v());
            eq_f32("C9 c2Dot", (l.c.c2Dot)(a, b), (l.r.c2Dot)(a, b));
            eq_f32("C9 c2Dot self", (l.c.c2Dot)(a, a), (l.r.c2Dot)(a, a));
            eq_f32("C10 c2Det2", (l.c.c2Det2)(a, b), (l.r.c2Det2)(a, b));
            // collinear -> +/-0
            let k = rng.coord();
            let ka = (l.c.c2Mulvs)(a, k);
            eq_f32("C10 collinear", (l.c.c2Det2)(a, ka), (l.r.c2Det2)(a, ka));
        }
        for _ in 0..N {
            let (a, b) = (rng.wild_v(), rng.wild_v());
            eq_f32("C9 c2Dot wild", (l.c.c2Dot)(a, b), (l.r.c2Dot)(a, b));
            eq_f32("C10 c2Det2 wild", (l.c.c2Det2)(a, b), (l.r.c2Det2)(a, b));
        }
        // overflow to +inf and 0*inf -> NaN
        let huge = c2v { x: 1e30, y: 1e30 };
        eq_f32("C9 overflow", (l.c.c2Dot)(huge, huge), (l.r.c2Dot)(huge, huge));
        let zi = c2v {
            x: 0.0,
            y: f32::INFINITY,
        };
        let iz = c2v {
            x: f32::INFINITY,
            y: 0.0,
        };
        eq_f32("C9 0*inf", (l.c.c2Dot)(zi, iz), (l.r.c2Dot)(zi, iz));
        eq_f32("C10 0*inf", (l.c.c2Det2)(zi, iz), (l.r.c2Det2)(zi, iz));
    }
}

// ---------------------------------------------------------------------------
// C11 — c2Len
// ---------------------------------------------------------------------------
#[test]
fn c11_len() {
    let l = libs();
    let mut rng = Rng::new(0x11);
    unsafe {
        for _ in 0..N {
            let a = rng.v();
            eq_f32("C11 c2Len", (l.c.c2Len)(a), (l.r.c2Len)(a));
        }
        for _ in 0..N {
            let a = rng.wild_v();
            eq_f32("C11 c2Len wild", (l.c.c2Len)(a), (l.r.c2Len)(a));
        }
        for a in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: 3.0, y: 4.0 },
            c2v { x: 1e30, y: 1e30 },
            c2v { x: f32::MAX, y: f32::MAX },
            c2v { x: f32::NAN, y: 0.0 },
            c2v { x: f32::INFINITY, y: f32::NAN },
        ] {
            eq_f32("C11 c2Len fixed", (l.c.c2Len)(a), (l.r.c2Len)(a));
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — c2Div / c2Norm
// ---------------------------------------------------------------------------
#[test]
fn c12_div_norm() {
    let l = libs();
    let mut rng = Rng::new(0x12);
    unsafe {
        for _ in 0..N {
            let a = rng.v();
            let b = rng.coord();
            eq_v("C12 c2Div", (l.c.c2Div)(a, b), (l.r.c2Div)(a, b));
            eq_v("C12 c2Norm", (l.c.c2Norm)(a), (l.r.c2Norm)(a));
        }
        for _ in 0..N {
            let a = rng.wild_v();
            let b = rng.wild();
            eq_v("C12 c2Div wild", (l.c.c2Div)(a, b), (l.r.c2Div)(a, b));
            eq_v("C12 c2Norm wild", (l.c.c2Norm)(a), (l.r.c2Norm)(a));
        }
        for a in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 3.0, y: 4.0 },
            c2v { x: 1e30, y: 1e30 },
            c2v { x: f32::MIN_POSITIVE / 5.0, y: 0.0 },
        ] {
            eq_v("C12 c2Norm fixed", (l.c.c2Norm)(a), (l.r.c2Norm)(a));
            for b in [0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NAN, f32::MAX] {
                eq_v("C12 c2Div fixed", (l.c.c2Div)(a, b), (l.r.c2Div)(a, b));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C13 — c2Neg / c2Skew / c2CCW90
// ---------------------------------------------------------------------------
#[test]
fn c13_neg_skew_ccw90() {
    let l = libs();
    let mut rng = Rng::new(0x13);
    unsafe {
        for _ in 0..N {
            let a = rng.v();
            eq_v("C13 c2Neg", (l.c.c2Neg)(a), (l.r.c2Neg)(a));
            eq_v("C13 c2Skew", (l.c.c2Skew)(a), (l.r.c2Skew)(a));
            eq_v("C13 c2CCW90", (l.c.c2CCW90)(a), (l.r.c2CCW90)(a));
        }
        for _ in 0..N {
            let a = rng.wild_v();
            eq_v("C13 c2Neg wild", (l.c.c2Neg)(a), (l.r.c2Neg)(a));
            eq_v("C13 c2Skew wild", (l.c.c2Skew)(a), (l.r.c2Skew)(a));
            eq_v("C13 c2CCW90 wild", (l.c.c2CCW90)(a), (l.r.c2CCW90)(a));
        }
    }
}

// ---------------------------------------------------------------------------
// C14 — identities
// ---------------------------------------------------------------------------
#[test]
fn c14_identities() {
    let l = libs();
    unsafe {
        for _ in 0..8 {
            eq_r("C14 c2RotIdentity", (l.c.c2RotIdentity)(), (l.r.c2RotIdentity)());
            eq_x("C14 c2xIdentity", (l.c.c2xIdentity)(), (l.r.c2xIdentity)());
        }
    }
}

// ---------------------------------------------------------------------------
// C15/C16 — c2Mulrv / c2MulrvT / c2Mulxv
// ---------------------------------------------------------------------------
#[test]
fn c15_c16_rotations() {
    let l = libs();
    let mut rng = Rng::new(0x15);
    unsafe {
        for _ in 0..N {
            let r = rng.rot();
            let v = rng.v();
            eq_v("C15 c2Mulrv", (l.c.c2Mulrv)(r, v), (l.r.c2Mulrv)(r, v));
            eq_v("C15 c2MulrvT", (l.c.c2MulrvT)(r, v), (l.r.c2MulrvT)(r, v));
            let x = rng.x();
            eq_v("C16 c2Mulxv", (l.c.c2Mulxv)(x, v), (l.r.c2Mulxv)(x, v));
        }
        for _ in 0..N {
            let r = c2r {
                c: rng.wild(),
                s: rng.wild(),
            };
            let v = rng.wild_v();
            eq_v("C15 c2Mulrv wild", (l.c.c2Mulrv)(r, v), (l.r.c2Mulrv)(r, v));
            eq_v("C15 c2MulrvT wild", (l.c.c2MulrvT)(r, v), (l.r.c2MulrvT)(r, v));
            let x = c2x {
                p: rng.wild_v(),
                r,
            };
            eq_v("C16 c2Mulxv wild", (l.c.c2Mulxv)(x, v), (l.r.c2Mulxv)(x, v));
        }
        // identity / pure translation / pure rotation
        let idr = (l.c.c2RotIdentity)();
        for _ in 0..200 {
            let v = rng.v();
            eq_v("C15 identity rot", (l.c.c2Mulrv)(idr, v), (l.r.c2Mulrv)(idr, v));
            eq_v("C15 identity rotT", (l.c.c2MulrvT)(idr, v), (l.r.c2MulrvT)(idr, v));
            let trans = c2x {
                p: rng.v(),
                r: idr,
            };
            eq_v("C16 translation only", (l.c.c2Mulxv)(trans, v), (l.r.c2Mulxv)(trans, v));
            let rotonly = c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: rng.rot(),
            };
            eq_v("C16 rotation only", (l.c.c2Mulxv)(rotonly, v), (l.r.c2Mulxv)(rotonly, v));
        }
    }
}

// ---------------------------------------------------------------------------
// C17 — c2BBVerts
// ---------------------------------------------------------------------------
#[test]
fn c17_bbverts() {
    let l = libs();
    let mut rng = Rng::new(0x17);
    unsafe {
        for i in 0..N {
            let mut bb = if i % 4 == 0 {
                c2AABB {
                    min: rng.wild_v(),
                    max: rng.wild_v(),
                }
            } else {
                rng.aabb()
            };
            let mut oc = [c2v { x: 7.5, y: -7.5 }; 4];
            let mut or_ = oc;
            (l.c.c2BBVerts)(oc.as_mut_ptr(), &mut bb);
            (l.r.c2BBVerts)(or_.as_mut_ptr(), &mut bb);
            for k in 0..4 {
                eq_v(&format!("C17 c2BBVerts[{k}]"), oc[k], or_[k]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C18/C19/C20 — c2MakeProxy for each valid type
// ---------------------------------------------------------------------------
#[test]
fn c18_c19_c20_makeproxy() {
    let l = libs();
    let mut rng = Rng::new(0x18);
    unsafe {
        // sentinel-filled destination proves exactly which fields are written
        let sentinel = c2Proxy {
            radius: -1234.5,
            count: -99,
            verts: [c2v { x: 9.25, y: -9.25 }; 8],
        };
        for _ in 0..N {
            let c = rng.circle();
            let mut pc = sentinel;
            let mut pr = sentinel;
            (l.c.c2MakeProxy)(&c as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pc);
            (l.r.c2MakeProxy)(&c as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pr);
            eq_proxy("C18 circle", &pc, &pr);

            let bb = rng.aabb();
            let mut pc = sentinel;
            let mut pr = sentinel;
            (l.c.c2MakeProxy)(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut pc);
            (l.r.c2MakeProxy)(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut pr);
            eq_proxy("C19 aabb", &pc, &pr);

            let cap = rng.capsule();
            let mut pc = sentinel;
            let mut pr = sentinel;
            (l.c.c2MakeProxy)(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pc);
            (l.r.c2MakeProxy)(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pr);
            eq_proxy("C20 capsule", &pc, &pr);
        }
        // degenerate shapes
        let zc = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        };
        let mut pc = sentinel;
        let mut pr = sentinel;
        (l.c.c2MakeProxy)(&zc as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pc);
        (l.r.c2MakeProxy)(&zc as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pr);
        eq_proxy("C18 zero circle", &pc, &pr);

        let inv = c2AABB {
            min: c2v { x: 5.0, y: 5.0 },
            max: c2v { x: -5.0, y: -5.0 },
        };
        let mut pc = sentinel;
        let mut pr = sentinel;
        (l.c.c2MakeProxy)(&inv as *const _ as *const c_void, C2_TYPE_AABB, &mut pc);
        (l.r.c2MakeProxy)(&inv as *const _ as *const c_void, C2_TYPE_AABB, &mut pr);
        eq_proxy("C19 inverted aabb", &pc, &pr);

        let deg = c2Capsule {
            a: c2v { x: 1.0, y: 2.0 },
            b: c2v { x: 1.0, y: 2.0 },
            r: 0.0,
        };
        let mut pc = sentinel;
        let mut pr = sentinel;
        (l.c.c2MakeProxy)(&deg as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pc);
        (l.r.c2MakeProxy)(&deg as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pr);
        eq_proxy("C20 zero-length capsule", &pc, &pr);
    }
}

// ---------------------------------------------------------------------------
// C21..C25 — c2Support at every proxy vertex count
// ---------------------------------------------------------------------------
#[test]
fn c21_to_c25_support() {
    let l = libs();
    let mut rng = Rng::new(0x21);
    unsafe {
        for &count in &[1i32, 2, 4, 8] {
            for _ in 0..N {
                let mut verts = [c2v::default(); 8];
                for i in 0..count as usize {
                    verts[i] = rng.v();
                }
                let d = rng.v();
                eq_i(
                    &format!("C2x c2Support count={count}"),
                    (l.c.c2Support)(verts.as_ptr(), count, d),
                    (l.r.c2Support)(verts.as_ptr(), count, d),
                );
                // axis-aligned directions produce ties -> first max must win
                for d in [
                    c2v { x: 1.0, y: 0.0 },
                    c2v { x: -1.0, y: 0.0 },
                    c2v { x: 0.0, y: 1.0 },
                    c2v { x: 0.0, y: -1.0 },
                    c2v { x: 0.0, y: 0.0 },
                ] {
                    eq_i(
                        &format!("C25 c2Support axis count={count}"),
                        (l.c.c2Support)(verts.as_ptr(), count, d),
                        (l.r.c2Support)(verts.as_ptr(), count, d),
                    );
                }
            }
            // all-equal vertices (total tie) and NaN direction
            for _ in 0..200 {
                let v0 = rng.v();
                let verts = [v0; 8];
                let d = rng.v();
                eq_i(
                    &format!("C25 c2Support tie count={count}"),
                    (l.c.c2Support)(verts.as_ptr(), count, d),
                    (l.r.c2Support)(verts.as_ptr(), count, d),
                );
                let dn = c2v {
                    x: f32::NAN,
                    y: rng.coord(),
                };
                eq_i(
                    &format!("C25 c2Support NaN dir count={count}"),
                    (l.c.c2Support)(verts.as_ptr(), count, dn),
                    (l.r.c2Support)(verts.as_ptr(), count, dn),
                );
            }
        }
        // wild vertices
        for &count in &[1i32, 2, 4, 8] {
            for _ in 0..N {
                let mut verts = [c2v::default(); 8];
                for i in 0..count as usize {
                    verts[i] = rng.wild_v();
                }
                let d = rng.wild_v();
                eq_i(
                    &format!("C2x c2Support wild count={count}"),
                    (l.c.c2Support)(verts.as_ptr(), count, d),
                    (l.r.c2Support)(verts.as_ptr(), count, d),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C26/C27/C28 — c2GJKSimplexMetric
// ---------------------------------------------------------------------------
#[test]
fn c26_c27_c28_simplex_metric() {
    let l = libs();
    let mut rng = Rng::new(0x26);
    unsafe {
        for &count in &[1i32, 2, 3] {
            for _ in 0..N {
                let div = rng.coord();
                let mut sc = rand_simplex(&mut rng, count, div);
                let mut sr = sc;
                let a = (l.c.c2GJKSimplexMetric)(&mut sc);
                let b = (l.r.c2GJKSimplexMetric)(&mut sr);
                eq_f32(&format!("C2x metric count={count}"), a, b);
                eq_simplex(&format!("C2x metric simplex count={count}"), &sc, &sr);
            }
        }
        // collinear / coincident triangles (metric == +/-0)
        for _ in 0..N {
            let p = rng.v();
            let k = rng.coord();
            let mut sc = rand_simplex(&mut rng, 3, 1.0);
            sc.a.p = p;
            sc.b.p = (l.c.c2Mulvs)(p, k);
            sc.c.p = (l.c.c2Mulvs)(p, k + 1.0);
            let mut sr = sc;
            eq_f32(
                "C28 metric collinear",
                (l.c.c2GJKSimplexMetric)(&mut sc),
                (l.r.c2GJKSimplexMetric)(&mut sr),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C29..C32 — c22 (all three Voronoi regions)
// ---------------------------------------------------------------------------
#[test]
fn c29_to_c32_c22() {
    let l = libs();
    let mut rng = Rng::new(0x29);
    unsafe {
        // fully random: statistically covers all three branches
        for _ in 0..(N * 4) {
            let mut sc = { let __d = rng.coord(); rand_simplex(&mut rng, 2, __d) };
            let mut sr = sc;
            (l.c.c22)(&mut sc);
            (l.r.c22)(&mut sr);
            eq_simplex("C32 c22 random", &sc, &sr);
        }
        // targeted regions: a/b on a line through the origin at parameter t
        for i in 0..N {
            let dir = rng.v();
            let t = (i as f32 / N as f32) * 4.0 - 2.0;
            let a = (l.c.c2Mulvs)(dir, t);
            let b = (l.c.c2Mulvs)(dir, t + 1.0);
            let mut sc = { let __d = rng.coord(); rand_simplex(&mut rng, 2, __d) };
            sc.a.p = a;
            sc.b.p = b;
            let mut sr = sc;
            (l.c.c22)(&mut sc);
            (l.r.c22)(&mut sr);
            eq_simplex("C29..C31 c22 targeted", &sc, &sr);
        }
        // degenerate: a == b, a == origin, b == origin, wild values
        for _ in 0..N {
            let p = rng.v();
            for (a, b) in [
                (p, p),
                (c2v { x: 0.0, y: 0.0 }, p),
                (p, c2v { x: 0.0, y: 0.0 }),
                (c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }),
                (rng.wild_v(), rng.wild_v()),
            ] {
                let mut sc = { let __d = rng.coord(); rand_simplex(&mut rng, 2, __d) };
                sc.a.p = a;
                sc.b.p = b;
                let mut sr = sc;
                (l.c.c22)(&mut sc);
                (l.r.c22)(&mut sr);
                eq_simplex("C32 c22 degenerate", &sc, &sr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C33..C40 — c23 (all seven regions)
// ---------------------------------------------------------------------------
#[test]
fn c33_to_c40_c23() {
    let l = libs();
    let mut rng = Rng::new(0x33);
    unsafe {
        for _ in 0..(N * 6) {
            let mut sc = { let __d = rng.coord(); rand_simplex(&mut rng, 3, __d) };
            let mut sr = sc;
            (l.c.c23)(&mut sc);
            (l.r.c23)(&mut sr);
            eq_simplex("C40 c23 random", &sc, &sr);
        }
        // triangles that contain the origin (interior region) and triangles
        // shifted so the origin falls in each vertex/edge region
        for i in 0..(N * 2) {
            let base = [
                c2v { x: -1.0, y: -1.0 },
                c2v { x: 1.0, y: -1.0 },
                c2v { x: 0.0, y: 1.5 },
            ];
            let sh = c2v {
                x: rng.range(4.0),
                y: rng.range(4.0),
            };
            let scale = 0.25 + (i % 7) as f32;
            let mut sc = { let __d = rng.coord(); rand_simplex(&mut rng, 3, __d) };
            sc.a.p = (l.c.c2Add)((l.c.c2Mulvs)(base[0], scale), sh);
            sc.b.p = (l.c.c2Add)((l.c.c2Mulvs)(base[1], scale), sh);
            sc.c.p = (l.c.c2Add)((l.c.c2Mulvs)(base[2], scale), sh);
            if i % 2 == 0 {
                // reverse the winding
                std::mem::swap(&mut sc.b.p, &mut sc.c.p);
            }
            let mut sr = sc;
            (l.c.c23)(&mut sc);
            (l.r.c23)(&mut sr);
            eq_simplex("C33..C39 c23 targeted", &sc, &sr);
        }
        // degenerate: zero-area, duplicate points, origin as a vertex, wild
        for _ in 0..N {
            let p = rng.v();
            let q = rng.v();
            let z = c2v { x: 0.0, y: 0.0 };
            for (a, b, c) in [
                (p, p, p),
                (p, q, p),
                (z, p, q),
                (p, z, q),
                (p, q, z),
                (p, (l.c.c2Mulvs)(p, 2.0), (l.c.c2Mulvs)(p, 3.0)),
                (rng.wild_v(), rng.wild_v(), rng.wild_v()),
            ] {
                let mut sc = { let __d = rng.coord(); rand_simplex(&mut rng, 3, __d) };
                sc.a.p = a;
                sc.b.p = b;
                sc.c.p = c;
                let mut sr = sc;
                (l.c.c23)(&mut sc);
                (l.r.c23)(&mut sr);
                eq_simplex("C40 c23 degenerate", &sc, &sr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C41/C42/C43 — c2D
// ---------------------------------------------------------------------------
#[test]
fn c41_to_c43_c2d() {
    let l = libs();
    let mut rng = Rng::new(0x41);
    unsafe {
        for &count in &[1i32, 2] {
            for _ in 0..(N * 2) {
                let mut sc = { let __d = rng.coord(); rand_simplex(&mut rng, count, __d) };
                let mut sr = sc;
                let a = (l.c.c2D)(&mut sc);
                let b = (l.r.c2D)(&mut sr);
                eq_v(&format!("C4x c2D count={count}"), a, b);
                eq_simplex(&format!("C4x c2D simplex count={count}"), &sc, &sr);
            }
        }
        // force det > 0 and det <= 0 explicitly
        for _ in 0..N {
            let a = rng.v();
            let ab = rng.v();
            let mut sc = rand_simplex(&mut rng, 2, 1.0);
            sc.a.p = a;
            sc.b.p = (l.c.c2Add)(a, ab);
            let mut sr = sc;
            eq_v("C42/C43 c2D det", (l.c.c2D)(&mut sc), (l.r.c2D)(&mut sr));
            // exactly collinear with the origin -> det == 0 -> CCW90
            let mut sc = rand_simplex(&mut rng, 2, 1.0);
            sc.a.p = a;
            sc.b.p = (l.c.c2Mulvs)(a, 2.0);
            let mut sr = sc;
            eq_v("C43 c2D collinear", (l.c.c2D)(&mut sc), (l.r.c2D)(&mut sr));
        }
        for _ in 0..N {
            let mut sc = { let __d = rng.coord(); rand_simplex(&mut rng, 2, __d) };
            sc.a.p = rng.wild_v();
            sc.b.p = rng.wild_v();
            let mut sr = sc;
            eq_v("C4x c2D wild", (l.c.c2D)(&mut sc), (l.r.c2D)(&mut sr));
        }
    }
}

// ---------------------------------------------------------------------------
// C44 — c2L
// ---------------------------------------------------------------------------
#[test]
fn c44_c2l() {
    let l = libs();
    let mut rng = Rng::new(0x44);
    unsafe {
        for &count in &[1i32, 2] {
            for _ in 0..(N * 2) {
                let div = match rng.below(5) {
                    0 => 0.0,
                    1 => 1.0,
                    _ => rng.coord(),
                };
                let mut sc = rand_simplex(&mut rng, count, div);
                let mut sr = sc;
                eq_v(
                    &format!("C44 c2L count={count}"),
                    (l.c.c2L)(&mut sc),
                    (l.r.c2L)(&mut sr),
                );
                eq_simplex(&format!("C44 c2L simplex count={count}"), &sc, &sr);
            }
        }
        // consistent div = u_a + u_b (the way c22 leaves it)
        for _ in 0..N {
            let mut sc = rand_simplex(&mut rng, 2, 1.0);
            let ua = rng.unit() * 10.0;
            let ub = rng.unit() * 10.0;
            sc.a.u = ua;
            sc.b.u = ub;
            sc.div = ua + ub;
            let mut sr = sc;
            eq_v("C44 c2L consistent", (l.c.c2L)(&mut sc), (l.r.c2L)(&mut sr));
        }
        // wild
        for _ in 0..N {
            let mut sc = { let __d = rng.wild(); rand_simplex(&mut rng, 2, __d) };
            sc.a.p = rng.wild_v();
            sc.b.p = rng.wild_v();
            sc.a.u = rng.wild();
            sc.b.u = rng.wild();
            let mut sr = sc;
            eq_v("C44 c2L wild", (l.c.c2L)(&mut sc), (l.r.c2L)(&mut sr));
        }
    }
}

// ---------------------------------------------------------------------------
// C45/C46/C47 — c2Witness
// ---------------------------------------------------------------------------
#[test]
fn c45_to_c47_witness() {
    let l = libs();
    let mut rng = Rng::new(0x45);
    unsafe {
        for &count in &[1i32, 2, 3] {
            for _ in 0..(N * 2) {
                let div = match rng.below(5) {
                    0 => 0.0,
                    1 => 1.0,
                    _ => rng.coord(),
                };
                let mut sc = rand_simplex(&mut rng, count, div);
                let mut sr = sc;
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let mut ar = c2v::default();
                let mut br = c2v::default();
                (l.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (l.r.c2Witness)(&mut sr, &mut ar, &mut br);
                eq_v(&format!("C4x witness a count={count}"), ac, ar);
                eq_v(&format!("C4x witness b count={count}"), bc, br);
                eq_simplex(&format!("C4x witness simplex count={count}"), &sc, &sr);
            }
        }
        // barycentric-consistent weights
        for &count in &[2i32, 3] {
            for _ in 0..N {
                let mut sc = rand_simplex(&mut rng, count, 1.0);
                let u = [rng.unit(), rng.unit(), rng.unit()];
                sc.a.u = u[0];
                sc.b.u = u[1];
                sc.c.u = u[2];
                sc.div = if count == 2 {
                    u[0] + u[1]
                } else {
                    u[0] + u[1] + u[2]
                };
                let mut sr = sc;
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let mut ar = c2v::default();
                let mut br = c2v::default();
                (l.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (l.r.c2Witness)(&mut sr, &mut ar, &mut br);
                eq_v("C46/C47 witness a", ac, ar);
                eq_v("C46/C47 witness b", bc, br);
            }
        }
        // wild
        for &count in &[1i32, 2, 3] {
            for _ in 0..N {
                let mut sc = { let __d = rng.wild(); rand_simplex(&mut rng, count, __d) };
                for sv in [&mut sc.a, &mut sc.b, &mut sc.c] {
                    sv.sA = c2v {
                        x: rng.wild(),
                        y: rng.wild(),
                    };
                    sv.sB = c2v {
                        x: rng.wild(),
                        y: rng.wild(),
                    };
                    sv.u = rng.wild();
                }
                let mut sr = sc;
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let mut ar = c2v::default();
                let mut br = c2v::default();
                (l.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (l.r.c2Witness)(&mut sr, &mut ar, &mut br);
                eq_v("C4x witness wild a", ac, ar);
                eq_v("C4x witness wild b", bc, br);
            }
        }
    }
}

// keep c_int / c_void imports used even if a future edit drops a call site
const _: Option<c_int> = None;
const _: Option<*const c_void> = None;
