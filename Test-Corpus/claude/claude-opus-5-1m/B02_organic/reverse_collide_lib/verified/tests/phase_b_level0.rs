//! Phase B, level 0 — CONFIGS.md rows B01 … B10.
//!
//! The scalar / vector leaf functions, driven through both `.so` exports.
//! Everything is compared bit-exactly (`f32::to_bits`), so NaN payloads and
//! signed zeros are part of the contract.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::os::raw::c_int;

const N: usize = 4096;

// ---------------------------------------------------------------------------
// B01 — c2V, c2Sub, c2Add, c2Neg, c2Skew, c2CCW90
// ---------------------------------------------------------------------------

#[test]
fn b01_v_sub_add_neg_skew_ccw90() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB01);
    unsafe {
        // Exhaustive boundary grid.
        for &ax in GRID.iter() {
            for &ay in GRID.iter() {
                for &bx in GRID.iter() {
                    for &by in GRID.iter() {
                        let a = c2v { x: ax, y: ay };
                        let b = c2v { x: bx, y: by };
                        let ctx = format!("grid a={a:?} b={b:?}");
                        eq_v(&format!("c2V {ctx}"), (c.c2V)(ax, ay), (r.c2V)(ax, ay));
                        eq_v(&format!("c2Sub {ctx}"), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
                        eq_v(&format!("c2Add {ctx}"), (c.c2Add)(a, b), (r.c2Add)(a, b));
                    }
                    let a = c2v { x: ax, y: ay };
                    eq_v("c2Neg", (c.c2Neg)(a), (r.c2Neg)(a));
                    eq_v("c2Skew", (c.c2Skew)(a), (r.c2Skew)(a));
                    eq_v("c2CCW90", (c.c2CCW90)(a), (r.c2CCW90)(a));
                    let _ = bx;
                }
            }
        }
        // Randomized.
        for i in 0..N {
            let a = rng.wild_v();
            let b = rng.wild_v();
            let ctx = format!("rand#{i} a={a:?} b={b:?}");
            eq_v(&format!("c2V {ctx}"), (c.c2V)(a.x, a.y), (r.c2V)(a.x, a.y));
            eq_v(&format!("c2Sub {ctx}"), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
            eq_v(&format!("c2Add {ctx}"), (c.c2Add)(a, b), (r.c2Add)(a, b));
            eq_v(&format!("c2Neg {ctx}"), (c.c2Neg)(a), (r.c2Neg)(a));
            eq_v(&format!("c2Skew {ctx}"), (c.c2Skew)(a), (r.c2Skew)(a));
            eq_v(&format!("c2CCW90 {ctx}"), (c.c2CCW90)(a), (r.c2CCW90)(a));
        }
    }
}

// ---------------------------------------------------------------------------
// B02 — c2Mulvs, c2Div
// ---------------------------------------------------------------------------

#[test]
fn b02_mulvs_div() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB02);
    unsafe {
        for &ax in GRID.iter() {
            for &ay in GRID.iter() {
                for &s in GRID.iter() {
                    let a = c2v { x: ax, y: ay };
                    let ctx = format!("a={a:?} s={s:?}");
                    eq_v(
                        &format!("c2Mulvs {ctx}"),
                        (c.c2Mulvs)(a, s),
                        (r.c2Mulvs)(a, s),
                    );
                    eq_v(&format!("c2Div {ctx}"), (c.c2Div)(a, s), (r.c2Div)(a, s));
                }
            }
        }
        for i in 0..N {
            let a = rng.wild_v();
            let s = rng.wild_f32();
            let ctx = format!("rand#{i} a={a:?} s={s:?}");
            eq_v(
                &format!("c2Mulvs {ctx}"),
                (c.c2Mulvs)(a, s),
                (r.c2Mulvs)(a, s),
            );
            eq_v(&format!("c2Div {ctx}"), (c.c2Div)(a, s), (r.c2Div)(a, s));
        }
    }
}

// ---------------------------------------------------------------------------
// B03 — c2Dot, c2Det2 over magnitudes that overflow/underflow
// ---------------------------------------------------------------------------

#[test]
fn b03_dot_det2() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB03);
    let scales = [
        1.0e-38f32, 1.0e-20, 1.0e-7, 1.0, 1.0e7, 1.0e20, 1.0e30, 1.0e38,
    ];
    unsafe {
        for i in 0..N * 4 {
            let sa = scales[rng.below(scales.len() as u32) as usize];
            let sb = scales[rng.below(scales.len() as u32) as usize];
            let a = c2v {
                x: rng.uniform(-1.0, 1.0) * sa,
                y: rng.uniform(-1.0, 1.0) * sa,
            };
            let b = c2v {
                x: rng.uniform(-1.0, 1.0) * sb,
                y: rng.uniform(-1.0, 1.0) * sb,
            };
            let ctx = format!("rand#{i} a={a:?} b={b:?}");
            eq_f32(&format!("c2Dot {ctx}"), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
            eq_f32(&format!("c2Det2 {ctx}"), (c.c2Det2)(a, b), (r.c2Det2)(a, b));
        }
        for &ax in GRID.iter() {
            for &ay in GRID.iter() {
                for &bx in GRID.iter() {
                    for &by in GRID.iter() {
                        let a = c2v { x: ax, y: ay };
                        let b = c2v { x: bx, y: by };
                        eq_f32("c2Dot grid", (c.c2Dot)(a, b), (r.c2Dot)(a, b));
                        eq_f32("c2Det2 grid", (c.c2Det2)(a, b), (r.c2Det2)(a, b));
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B04 — c2Maxv, c2Minv, c2Clampv (incl. inverted boxes and NaN in every slot)
// ---------------------------------------------------------------------------

#[test]
fn b04_maxv_minv_clampv() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB04);
    unsafe {
        // NaN in each of the 6 slots, over the boundary grid.
        for slot in 0..6 {
            for &g in GRID.iter() {
                let mut vals = [g; 6];
                vals[slot] = f32::NAN;
                let a = c2v {
                    x: vals[0],
                    y: vals[1],
                };
                let lo = c2v {
                    x: vals[2],
                    y: vals[3],
                };
                let hi = c2v {
                    x: vals[4],
                    y: vals[5],
                };
                eq_v("c2Maxv nan-slot", (c.c2Maxv)(a, lo), (r.c2Maxv)(a, lo));
                eq_v("c2Minv nan-slot", (c.c2Minv)(a, lo), (r.c2Minv)(a, lo));
                eq_v(
                    "c2Clampv nan-slot",
                    (c.c2Clampv)(a, lo, hi),
                    (r.c2Clampv)(a, lo, hi),
                );
            }
        }
        for i in 0..N {
            let a = rng.wild_v();
            let p = rng.wild_v();
            let q = rng.wild_v();
            // ordered, equal and inverted boxes
            let boxes = [
                (
                    c2v {
                        x: p.x.min(q.x),
                        y: p.y.min(q.y),
                    },
                    c2v {
                        x: p.x.max(q.x),
                        y: p.y.max(q.y),
                    },
                ),
                (p, p),
                (q, p),
                (p, q),
            ];
            for (lo, hi) in boxes {
                let ctx = format!("rand#{i} a={a:?} lo={lo:?} hi={hi:?}");
                eq_v(&format!("c2Maxv {ctx}"), (c.c2Maxv)(a, lo), (r.c2Maxv)(a, lo));
                eq_v(&format!("c2Minv {ctx}"), (c.c2Minv)(a, hi), (r.c2Minv)(a, hi));
                eq_v(
                    &format!("c2Clampv {ctx}"),
                    (c.c2Clampv)(a, lo, hi),
                    (r.c2Clampv)(a, lo, hi),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B05 — c2Len, c2Norm
// ---------------------------------------------------------------------------

#[test]
fn b05_len_norm() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB05);
    unsafe {
        let special = [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: 0.0, y: -0.0 },
            c2v {
                x: FLT_MIN_POS,
                y: FLT_MIN_POS,
            },
            c2v { x: 1.0e-45, y: 0.0 },
            c2v { x: 1.0e30, y: 1.0e30 },
            c2v { x: FLT_MAX, y: FLT_MAX },
            c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
            c2v {
                x: f32::NAN,
                y: 1.0,
            },
            c2v {
                x: 3.0,
                y: 4.0,
            },
        ];
        for a in special {
            eq_f32(&format!("c2Len {a:?}"), (c.c2Len)(a), (r.c2Len)(a));
            eq_v(&format!("c2Norm {a:?}"), (c.c2Norm)(a), (r.c2Norm)(a));
        }
        for &ax in GRID.iter() {
            for &ay in GRID.iter() {
                let a = c2v { x: ax, y: ay };
                eq_f32("c2Len grid", (c.c2Len)(a), (r.c2Len)(a));
                eq_v("c2Norm grid", (c.c2Norm)(a), (r.c2Norm)(a));
            }
        }
        for i in 0..N * 4 {
            let a = if rng.bool() {
                rng.wild_v()
            } else {
                rng.v(1000.0)
            };
            eq_f32(&format!("c2Len rand#{i} {a:?}"), (c.c2Len)(a), (r.c2Len)(a));
            eq_v(&format!("c2Norm rand#{i} {a:?}"), (c.c2Norm)(a), (r.c2Norm)(a));
        }
    }
}

// ---------------------------------------------------------------------------
// B06 — c2RotIdentity, c2xIdentity
// ---------------------------------------------------------------------------

#[test]
fn b06_identities() {
    let (c, r) = libs();
    unsafe {
        eq_r("c2RotIdentity", (c.c2RotIdentity)(), (r.c2RotIdentity)());
        eq_x("c2xIdentity", (c.c2xIdentity)(), (r.c2xIdentity)());
        // and bit-exact against the documented constants
        let id = (r.c2RotIdentity)();
        assert_eq!(id.c.to_bits(), 1.0f32.to_bits());
        assert_eq!(id.s.to_bits(), 0.0f32.to_bits());
    }
}

// ---------------------------------------------------------------------------
// B07 — c2Mulrv, c2MulrvT
// ---------------------------------------------------------------------------

#[test]
fn b07_mulrv_mulrvT() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB07);
    unsafe {
        for &rc in GRID.iter() {
            for &rs in GRID.iter() {
                for &bx in GRID.iter() {
                    for &by in GRID.iter() {
                        let rot = c2r { c: rc, s: rs };
                        let b = c2v { x: bx, y: by };
                        eq_v("c2Mulrv grid", (c.c2Mulrv)(rot, b), (r.c2Mulrv)(rot, b));
                        eq_v("c2MulrvT grid", (c.c2MulrvT)(rot, b), (r.c2MulrvT)(rot, b));
                    }
                }
            }
        }
        for i in 0..N * 4 {
            let rot = rng.rot();
            let b = if rng.bool() {
                rng.wild_v()
            } else {
                rng.v(1000.0)
            };
            let ctx = format!("rand#{i} rot=({:?},{:?}) b={b:?}", rot.c, rot.s);
            eq_v(
                &format!("c2Mulrv {ctx}"),
                (c.c2Mulrv)(rot, b),
                (r.c2Mulrv)(rot, b),
            );
            eq_v(
                &format!("c2MulrvT {ctx}"),
                (c.c2MulrvT)(rot, b),
                (r.c2MulrvT)(rot, b),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B08 — c2Mulxv
// ---------------------------------------------------------------------------

#[test]
fn b08_mulxv() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB08);
    unsafe {
        for i in 0..N * 4 {
            let x = match rng.below(4) {
                0 => c2x {
                    p: rng.v(1000.0),
                    r: c2r { c: 1.0, s: 0.0 },
                }, // pure translation
                1 => c2x {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: rng.rot(),
                }, // pure rotation
                2 => c2x {
                    p: rng.wild_v(),
                    r: c2r {
                        c: rng.wild_f32(),
                        s: rng.wild_f32(),
                    },
                },
                _ => rng.xform(1000.0),
            };
            let b = if rng.bool() {
                rng.wild_v()
            } else {
                rng.v(1000.0)
            };
            let ctx = format!("rand#{i}");
            eq_v(&format!("c2Mulxv {ctx}"), (c.c2Mulxv)(x, b), (r.c2Mulxv)(x, b));
        }
    }
}

// ---------------------------------------------------------------------------
// B09 — c2Support: count 1/2/4/8, ties, wild directions
// ---------------------------------------------------------------------------

#[test]
fn b09_support() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB09);
    unsafe {
        for i in 0..N * 4 {
            let count = [1i32, 2, 3, 4, 5, 8][rng.below(6) as usize];
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = if rng.below(4) == 0 {
                    rng.wild_v()
                } else {
                    rng.v(100.0)
                };
            }
            // Deliberate ties: duplicate a vertex so two projections are equal.
            if rng.bool() && count >= 2 {
                let src = rng.below(count as u32) as usize;
                let dst = rng.below(count as u32) as usize;
                verts[dst] = verts[src];
            }
            let d = if rng.below(4) == 0 {
                rng.wild_v()
            } else {
                rng.v(10.0)
            };
            let ctx = format!("rand#{i} count={count} d={d:?}");
            eq_int(
                &format!("c2Support {ctx}"),
                (c.c2Support)(verts.as_ptr(), count, d),
                (r.c2Support)(verts.as_ptr(), count, d),
            );
        }
        // Explicit tie test: all vertices identical, every count.
        for count in 1..=8i32 {
            let verts = [c2v { x: 2.0, y: 3.0 }; 8];
            for d in [
                c2v { x: 1.0, y: 0.0 },
                c2v { x: 0.0, y: 0.0 },
                c2v { x: -1.0, y: -1.0 },
            ] {
                eq_int(
                    "c2Support all-equal",
                    (c.c2Support)(verts.as_ptr(), count, d),
                    (r.c2Support)(verts.as_ptr(), count, d),
                );
            }
        }
        // Monotone ramp so the maximum is at a known, varying index.
        for count in 1..=8i32 {
            let mut verts = [c2v::default(); 8];
            for (k, v) in verts.iter_mut().enumerate() {
                v.x = k as f32;
                v.y = -(k as f32);
            }
            for d in [
                c2v { x: 1.0, y: 0.0 },
                c2v { x: -1.0, y: 0.0 },
                c2v { x: 0.0, y: 1.0 },
                c2v { x: 0.0, y: -1.0 },
            ] {
                eq_int(
                    "c2Support ramp",
                    (c.c2Support)(verts.as_ptr(), count, d),
                    (r.c2Support)(verts.as_ptr(), count, d),
                );
            }
        }
        let _: c_int = 0;
    }
}

// ---------------------------------------------------------------------------
// B10 — c2BBVerts
// ---------------------------------------------------------------------------

#[test]
fn b10_bbverts() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB10);
    unsafe {
        for i in 0..N * 4 {
            let mut bb_c = if rng.below(4) == 0 {
                c2AABB {
                    min: rng.wild_v(),
                    max: rng.wild_v(),
                }
            } else {
                rng.aabb(500.0)
            };
            let mut bb_r = bb_c;
            // 8 slots so an accidental overrun is visible.
            let mut out_c = [c2v {
                x: 12.5,
                y: -12.5,
            }; 8];
            let mut out_r = out_c;
            (c.c2BBVerts)(out_c.as_mut_ptr(), &mut bb_c);
            (r.c2BBVerts)(out_r.as_mut_ptr(), &mut bb_r);
            eq_bytes(&format!("c2BBVerts out rand#{i}"), &out_c, &out_r);
            eq_bytes(&format!("c2BBVerts bb rand#{i}"), &bb_c, &bb_r);
        }
        for &lo in GRID.iter() {
            for &hi in GRID.iter() {
                let mut bb_c = c2AABB {
                    min: c2v { x: lo, y: hi },
                    max: c2v { x: hi, y: lo },
                };
                let mut bb_r = bb_c;
                let mut out_c = [c2v::default(); 8];
                let mut out_r = out_c;
                (c.c2BBVerts)(out_c.as_mut_ptr(), &mut bb_c);
                (r.c2BBVerts)(out_r.as_mut_ptr(), &mut bb_r);
                eq_bytes("c2BBVerts grid", &out_c, &out_r);
            }
        }
    }
}
