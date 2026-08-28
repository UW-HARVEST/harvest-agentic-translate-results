//! Phase B — valid-path differential tests, `CONFIGS.md` rows 1..=21.
//!
//! Group 1: lowest-level vector math. Group 2: proxy construction.
//! Every test drives BOTH `.so`s through their exported symbols and compares
//! results bit-for-bit over many randomized inputs (fixed seed).

mod common;
use common::*;

// ---------------------------------------------------------------------------
// Row 1 / 2 — c2V, c2Neg, c2Skew, c2CCW90
// ---------------------------------------------------------------------------

#[test]
fn row01_unary_ordinary() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0001);
        for i in 0..N {
            let x = rng.ordinary(1.0e3);
            let y = rng.ordinary(1.0e3);
            let cv = (c.c2V)(x, y);
            let rv = (r.c2V)(x, y);
            assert!(
                v_same(cv, rv),
                "{label} row1 c2V #{i}: x={} y={} -> C {} vs R {}",
                fmt_f32(x),
                fmt_f32(y),
                fmt_v(cv),
                fmt_v(rv)
            );
            let a = c2v { x, y };
            assert!(v_same((c.c2Neg)(a), (r.c2Neg)(a)), "{label} row1 c2Neg #{i}");
            assert!(v_same((c.c2Skew)(a), (r.c2Skew)(a)), "{label} row1 c2Skew #{i}");
            assert!(
                v_same((c.c2CCW90)(a), (r.c2CCW90)(a)),
                "{label} row1 c2CCW90 #{i}"
            );
        }
    });
}

#[test]
fn row02_unary_extreme() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0002);
        for i in 0..N {
            let x = rng.special();
            let y = rng.special();
            let cv = (c.c2V)(x, y);
            let rv = (r.c2V)(x, y);
            assert!(
                v_same(cv, rv),
                "{label} row2 c2V #{i}: x={} y={}",
                fmt_f32(x),
                fmt_f32(y)
            );
            let a = c2v { x, y };
            assert!(
                v_same((c.c2Neg)(a), (r.c2Neg)(a)),
                "{label} row2 c2Neg #{i}: {}",
                fmt_v(a)
            );
            assert!(
                v_same((c.c2Skew)(a), (r.c2Skew)(a)),
                "{label} row2 c2Skew #{i}: {}",
                fmt_v(a)
            );
            assert!(
                v_same((c.c2CCW90)(a), (r.c2CCW90)(a)),
                "{label} row2 c2CCW90 #{i}: {}",
                fmt_v(a)
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Row 3 / 4 — c2Add, c2Sub, c2Dot, c2Det2
// ---------------------------------------------------------------------------

fn binary_axis(c: &Api, r: &Api, label: &str, row: &str, seed: u64, extreme: bool) {
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let (a, b) = if extreme {
            (rng.v_special(), rng.v_special())
        } else {
            (rng.v_ordinary(1.0e3), rng.v_ordinary(1.0e3))
        };
        assert!(
            v_same((c.c2Add)(a, b), (r.c2Add)(a, b)),
            "{label} {row} c2Add #{i}: a={} b={} -> C {} vs R {}",
            fmt_v(a),
            fmt_v(b),
            fmt_v((c.c2Add)(a, b)),
            fmt_v((r.c2Add)(a, b))
        );
        assert!(
            v_same((c.c2Sub)(a, b), (r.c2Sub)(a, b)),
            "{label} {row} c2Sub #{i}: a={} b={}",
            fmt_v(a),
            fmt_v(b)
        );
        assert!(
            f32_same((c.c2Dot)(a, b), (r.c2Dot)(a, b)),
            "{label} {row} c2Dot #{i}: a={} b={} -> C {} vs R {}",
            fmt_v(a),
            fmt_v(b),
            fmt_f32((c.c2Dot)(a, b)),
            fmt_f32((r.c2Dot)(a, b))
        );
        assert!(
            f32_same((c.c2Det2)(a, b), (r.c2Det2)(a, b)),
            "{label} {row} c2Det2 #{i}: a={} b={} -> C {} vs R {}",
            fmt_v(a),
            fmt_v(b),
            fmt_f32((c.c2Det2)(a, b)),
            fmt_f32((r.c2Det2)(a, b))
        );
    }
}

#[test]
fn row03_binary_ordinary() {
    for_each_pair(|c, r, label| binary_axis(c, r, label, "row3", 0x0003, false));
}

#[test]
fn row04_binary_extreme() {
    for_each_pair(|c, r, label| binary_axis(c, r, label, "row4", 0x0004, true));
}

// ---------------------------------------------------------------------------
// Row 5 / 6 — c2Mulvs, c2Div
// ---------------------------------------------------------------------------

fn scale_axis(c: &Api, r: &Api, label: &str, row: &str, seed: u64, extreme: bool) {
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let (a, s) = if extreme {
            (rng.v_special(), rng.special())
        } else {
            (rng.v_ordinary(1.0e3), rng.ordinary(1.0e3))
        };
        assert!(
            v_same((c.c2Mulvs)(a, s), (r.c2Mulvs)(a, s)),
            "{label} {row} c2Mulvs #{i}: a={} s={} -> C {} vs R {}",
            fmt_v(a),
            fmt_f32(s),
            fmt_v((c.c2Mulvs)(a, s)),
            fmt_v((r.c2Mulvs)(a, s))
        );
        assert!(
            v_same((c.c2Div)(a, s), (r.c2Div)(a, s)),
            "{label} {row} c2Div #{i}: a={} s={} -> C {} vs R {}",
            fmt_v(a),
            fmt_f32(s),
            fmt_v((c.c2Div)(a, s)),
            fmt_v((r.c2Div)(a, s))
        );
    }
}

#[test]
fn row05_scale_ordinary() {
    for_each_pair(|c, r, label| scale_axis(c, r, label, "row5", 0x0005, false));
}

#[test]
fn row06_scale_extreme() {
    for_each_pair(|c, r, label| {
        scale_axis(c, r, label, "row6", 0x0006, true);
        // Explicitly pin the documented divide-by-zero shapes.
        for &s in &[0.0f32, -0.0f32, f32::INFINITY, f32::NEG_INFINITY] {
            for a in [
                c2v { x: 0.0, y: 0.0 },
                c2v { x: -0.0, y: 3.0 },
                c2v { x: 1.0, y: -1.0 },
                c2v { x: f32::MAX, y: f32::MIN },
            ] {
                assert!(
                    v_same((c.c2Div)(a, s), (r.c2Div)(a, s)),
                    "{label} row6 c2Div pinned: a={} s={} -> C {} vs R {}",
                    fmt_v(a),
                    fmt_f32(s),
                    fmt_v((c.c2Div)(a, s)),
                    fmt_v((r.c2Div)(a, s))
                );
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Row 7 / 8 — c2Maxv, c2Minv
// ---------------------------------------------------------------------------

#[test]
fn row07_minmax_ordinary() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0007);
        for i in 0..N {
            let a = rng.v_ordinary(10.0);
            // Force exact ties on one or both components sometimes.
            let b = match rng.below(4) {
                0 => a,
                1 => c2v { x: a.x, y: rng.ordinary(10.0) },
                2 => c2v { x: rng.ordinary(10.0), y: a.y },
                _ => rng.v_ordinary(10.0),
            };
            assert!(
                v_same((c.c2Maxv)(a, b), (r.c2Maxv)(a, b)),
                "{label} row7 c2Maxv #{i}: a={} b={}",
                fmt_v(a),
                fmt_v(b)
            );
            assert!(
                v_same((c.c2Minv)(a, b), (r.c2Minv)(a, b)),
                "{label} row7 c2Minv #{i}: a={} b={}",
                fmt_v(a),
                fmt_v(b)
            );
        }
    });
}

#[test]
fn row08_minmax_extreme() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0008);
        for i in 0..N {
            let a = rng.v_special();
            let b = rng.v_special();
            assert!(
                v_same((c.c2Maxv)(a, b), (r.c2Maxv)(a, b)),
                "{label} row8 c2Maxv #{i}: a={} b={} -> C {} vs R {}",
                fmt_v(a),
                fmt_v(b),
                fmt_v((c.c2Maxv)(a, b)),
                fmt_v((r.c2Maxv)(a, b))
            );
            assert!(
                v_same((c.c2Minv)(a, b), (r.c2Minv)(a, b)),
                "{label} row8 c2Minv #{i}: a={} b={} -> C {} vs R {}",
                fmt_v(a),
                fmt_v(b),
                fmt_v((c.c2Minv)(a, b)),
                fmt_v((r.c2Minv)(a, b))
            );
        }
        // Pinned: NaN in either operand, and the +0.0 / -0.0 tie.
        let cases = [
            (c2v { x: f32::NAN, y: 1.0 }, c2v { x: 1.0, y: f32::NAN }),
            (c2v { x: f32::NAN, y: f32::NAN }, c2v { x: f32::NAN, y: f32::NAN }),
            (c2v { x: 0.0, y: -0.0 }, c2v { x: -0.0, y: 0.0 }),
            (
                c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
                c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
            ),
        ];
        for (a, b) in cases {
            assert!(
                v_same((c.c2Maxv)(a, b), (r.c2Maxv)(a, b)),
                "{label} row8 pinned c2Maxv: a={} b={} -> C {} vs R {}",
                fmt_v(a),
                fmt_v(b),
                fmt_v((c.c2Maxv)(a, b)),
                fmt_v((r.c2Maxv)(a, b))
            );
            assert!(
                v_same((c.c2Minv)(a, b), (r.c2Minv)(a, b)),
                "{label} row8 pinned c2Minv: a={} b={} -> C {} vs R {}",
                fmt_v(a),
                fmt_v(b),
                fmt_v((c.c2Minv)(a, b)),
                fmt_v((r.c2Minv)(a, b))
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Row 9 / 10 / 11 — c2Clampv
// ---------------------------------------------------------------------------

#[test]
fn row09_clamp_wellformed() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0009);
        for i in 0..N {
            let p = rng.v_ordinary(10.0);
            let q = rng.v_ordinary(10.0);
            let lo = c2v { x: p.x.min(q.x), y: p.y.min(q.y) };
            let hi = c2v { x: p.x.max(q.x), y: p.y.max(q.y) };
            // a inside / below / above / exactly on a bound
            let a = match rng.below(5) {
                0 => lo,
                1 => hi,
                2 => c2v { x: lo.x - 1.0, y: hi.y + 1.0 },
                3 => c2v { x: (lo.x + hi.x) * 0.5, y: (lo.y + hi.y) * 0.5 },
                _ => rng.v_ordinary(12.0),
            };
            assert!(
                v_same((c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi)),
                "{label} row9 #{i}: a={} lo={} hi={}",
                fmt_v(a),
                fmt_v(lo),
                fmt_v(hi)
            );
        }
    });
}

#[test]
fn row10_clamp_inverted() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x000A);
        for i in 0..N {
            let p = rng.v_ordinary(10.0);
            let q = rng.v_ordinary(10.0);
            // deliberately lo > hi
            let lo = c2v { x: p.x.max(q.x), y: p.y.max(q.y) };
            let hi = c2v { x: p.x.min(q.x), y: p.y.min(q.y) };
            let a = rng.v_ordinary(12.0);
            assert!(
                v_same((c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi)),
                "{label} row10 #{i}: a={} lo={} hi={} -> C {} vs R {}",
                fmt_v(a),
                fmt_v(lo),
                fmt_v(hi),
                fmt_v((c.c2Clampv)(a, lo, hi)),
                fmt_v((r.c2Clampv)(a, lo, hi))
            );
        }
    });
}

#[test]
fn row11_clamp_extreme() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x000B);
        for i in 0..N {
            let a = rng.v_special();
            let lo = rng.v_special();
            let hi = rng.v_special();
            assert!(
                v_same((c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi)),
                "{label} row11 #{i}: a={} lo={} hi={} -> C {} vs R {}",
                fmt_v(a),
                fmt_v(lo),
                fmt_v(hi),
                fmt_v((c.c2Clampv)(a, lo, hi)),
                fmt_v((r.c2Clampv)(a, lo, hi))
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Row 12 / 13 — c2Len, c2Norm
// ---------------------------------------------------------------------------

#[test]
fn row12_len_norm_ordinary() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x000C);
        for i in 0..N {
            let a = rng.v_ordinary(1.0e3);
            assert!(
                f32_same((c.c2Len)(a), (r.c2Len)(a)),
                "{label} row12 c2Len #{i}: a={} -> C {} vs R {}",
                fmt_v(a),
                fmt_f32((c.c2Len)(a)),
                fmt_f32((r.c2Len)(a))
            );
            assert!(
                v_same((c.c2Norm)(a), (r.c2Norm)(a)),
                "{label} row12 c2Norm #{i}: a={} -> C {} vs R {}",
                fmt_v(a),
                fmt_v((c.c2Norm)(a)),
                fmt_v((r.c2Norm)(a))
            );
        }
    });
}

#[test]
fn row13_len_norm_extreme() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x000D);
        let pinned = [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: f32::MIN_POSITIVE, y: 0.0 },
            c2v { x: 1e-45, y: 1e-45 },
            c2v { x: 1e20, y: 1e20 },
            c2v { x: f32::MAX, y: f32::MAX },
            c2v { x: f32::INFINITY, y: 0.0 },
            c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
            c2v { x: f32::NAN, y: 1.0 },
        ];
        for a in pinned {
            assert!(
                f32_same((c.c2Len)(a), (r.c2Len)(a)),
                "{label} row13 pinned c2Len: a={} -> C {} vs R {}",
                fmt_v(a),
                fmt_f32((c.c2Len)(a)),
                fmt_f32((r.c2Len)(a))
            );
            assert!(
                v_same((c.c2Norm)(a), (r.c2Norm)(a)),
                "{label} row13 pinned c2Norm: a={} -> C {} vs R {}",
                fmt_v(a),
                fmt_v((c.c2Norm)(a)),
                fmt_v((r.c2Norm)(a))
            );
        }
        for i in 0..N {
            let a = rng.v_special();
            assert!(
                f32_same((c.c2Len)(a), (r.c2Len)(a)),
                "{label} row13 c2Len #{i}: a={}",
                fmt_v(a)
            );
            assert!(
                v_same((c.c2Norm)(a), (r.c2Norm)(a)),
                "{label} row13 c2Norm #{i}: a={} -> C {} vs R {}",
                fmt_v(a),
                fmt_v((c.c2Norm)(a)),
                fmt_v((r.c2Norm)(a))
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Row 14 — c2RotIdentity, c2xIdentity
// ---------------------------------------------------------------------------

#[test]
fn row14_identities() {
    for_each_pair(|c, r, label| {
        let cr = (c.c2RotIdentity)();
        let rr = (r.c2RotIdentity)();
        assert!(
            r_same(cr, rr),
            "{label} row14 c2RotIdentity: C ({},{}) vs R ({},{})",
            fmt_f32(cr.c),
            fmt_f32(cr.s),
            fmt_f32(rr.c),
            fmt_f32(rr.s)
        );
        let cx = (c.c2xIdentity)();
        let rx = (r.c2xIdentity)();
        assert!(x_same(cx, rx), "{label} row14 c2xIdentity");
        // pin the documented values
        assert_eq!(cr.c.to_bits(), 1.0f32.to_bits());
        assert_eq!(cr.s.to_bits(), 0.0f32.to_bits());
        assert_eq!(cx.p.x.to_bits(), 0.0f32.to_bits());
    });
}

// ---------------------------------------------------------------------------
// Row 15 / 16 — c2Mulrv, c2MulrvT, c2Mulxv
// ---------------------------------------------------------------------------

fn rotor_axis(c: &Api, r: &Api, label: &str, row: &str, seed: u64, weird: bool) {
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let x = if weird {
            rng.xform_weird(1.0e2)
        } else {
            rng.xform_rot_trans(1.0e2)
        };
        let b = if weird {
            rng.v_special_no_nan()
        } else {
            rng.v_ordinary(1.0e2)
        };
        assert!(
            v_same((c.c2Mulrv)(x.r, b), (r.c2Mulrv)(x.r, b)),
            "{label} {row} c2Mulrv #{i}: rot=({},{}) b={} -> C {} vs R {}",
            fmt_f32(x.r.c),
            fmt_f32(x.r.s),
            fmt_v(b),
            fmt_v((c.c2Mulrv)(x.r, b)),
            fmt_v((r.c2Mulrv)(x.r, b))
        );
        assert!(
            v_same((c.c2MulrvT)(x.r, b), (r.c2MulrvT)(x.r, b)),
            "{label} {row} c2MulrvT #{i}: rot=({},{}) b={} -> C {} vs R {}",
            fmt_f32(x.r.c),
            fmt_f32(x.r.s),
            fmt_v(b),
            fmt_v((c.c2MulrvT)(x.r, b)),
            fmt_v((r.c2MulrvT)(x.r, b))
        );
        assert!(
            v_same((c.c2Mulxv)(x, b), (r.c2Mulxv)(x, b)),
            "{label} {row} c2Mulxv #{i}: p={} rot=({},{}) b={} -> C {} vs R {}",
            fmt_v(x.p),
            fmt_f32(x.r.c),
            fmt_f32(x.r.s),
            fmt_v(b),
            fmt_v((c.c2Mulxv)(x, b)),
            fmt_v((r.c2Mulxv)(x, b))
        );
    }
}

#[test]
fn row15_rotor_unit() {
    for_each_pair(|c, r, label| rotor_axis(c, r, label, "row15", 0x000F, false));
}

#[test]
fn row16_rotor_weird() {
    for_each_pair(|c, r, label| rotor_axis(c, r, label, "row16", 0x0010, true));
}

// ---------------------------------------------------------------------------
// Row 17 / 18 — c2BBVerts
// ---------------------------------------------------------------------------

fn bbverts_axis(c: &Api, r: &Api, label: &str, row: &str, seed: u64, mode: u32) {
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let mut bb = match mode {
            0 => rng.aabb(1.0e2),
            1 => {
                let p = rng.v_ordinary(1.0e2);
                let q = rng.v_ordinary(1.0e2);
                match rng.below(2) {
                    // inverted
                    0 => c2AABB {
                        min: c2v { x: p.x.max(q.x), y: p.y.max(q.y) },
                        max: c2v { x: p.x.min(q.x), y: p.y.min(q.y) },
                    },
                    // degenerate
                    _ => c2AABB { min: p, max: p },
                }
            }
            _ => c2AABB {
                min: rng.v_special(),
                max: rng.v_special(),
            },
        };
        // Poison the destination so untouched slots are visible.
        let poison = [c2v { x: -7.5e-3, y: 9.125e4 }; 4];
        let mut co = poison;
        let mut ro = poison;
        let mut bb_c = bb;
        let mut bb_r = bb;
        unsafe {
            (c.c2BBVerts)(co.as_mut_ptr(), &mut bb_c);
            (r.c2BBVerts)(ro.as_mut_ptr(), &mut bb_r);
        }
        for k in 0..4 {
            assert!(
                v_same(co[k], ro[k]),
                "{label} {row} c2BBVerts #{i} out[{k}]: bb=min{} max{} -> C {} vs R {}",
                fmt_v(bb.min),
                fmt_v(bb.max),
                fmt_v(co[k]),
                fmt_v(ro[k])
            );
        }
        // input must not be modified by either
        assert!(
            v_same(bb_c.min, bb_r.min) && v_same(bb_c.max, bb_r.max),
            "{label} {row} c2BBVerts #{i}: input mutated differently"
        );
        bb.min = bb_c.min; // keep the compiler from optimising bb away
    }
}

#[test]
fn row17_bbverts_wellformed() {
    for_each_pair(|c, r, label| bbverts_axis(c, r, label, "row17", 0x0011, 0));
}

#[test]
fn row18_bbverts_degenerate_and_extreme() {
    for_each_pair(|c, r, label| {
        bbverts_axis(c, r, label, "row18a", 0x0012, 1);
        bbverts_axis(c, r, label, "row18b", 0x0013, 2);
    });
}

// ---------------------------------------------------------------------------
// Row 19 / 20 / 21 — c2MakeProxy
// ---------------------------------------------------------------------------

/// Fill a proxy with a recognisable pattern so that any slot the C code leaves
/// untouched is visible in the comparison.
fn poisoned_proxy(seed: u32) -> c2Proxy {
    let mut p = c2Proxy {
        radius: f32::from_bits(0xDEAD_BEEF),
        count: -0x5EED,
        verts: [c2v::default(); 8],
    };
    for (i, v) in p.verts.iter_mut().enumerate() {
        v.x = f32::from_bits(0xCAFE_0000 ^ (i as u32) ^ seed);
        v.y = f32::from_bits(0xBEEF_0000 ^ ((i as u32) << 8) ^ seed);
    }
    p
}

fn makeproxy_axis(c: &Api, r: &Api, label: &str, row: &str, seed: u64, ty: std::os::raw::c_int) {
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let shape = match rng.below(3) {
            0 => Shape::random(&mut rng, ty, 1.0e2),
            1 => Shape::random_degenerate(&mut rng, ty, 1.0e2),
            _ => Shape::random_extreme(&mut rng, ty),
        };
        #[repr(align(4))]
        struct Buf([u8; 20]);
        let buf = Buf(shape.bytes());
        let ptr = buf.0.as_ptr() as *const std::ffi::c_void;

        let mut cp = poisoned_proxy(i as u32);
        let mut rp = poisoned_proxy(i as u32);
        unsafe {
            (c.c2MakeProxy)(ptr, ty, &mut cp);
            (r.c2MakeProxy)(ptr, ty, &mut rp);
        }
        assert!(
            proxy_same(&cp, &rp),
            "{label} {row} c2MakeProxy #{i} ty={ty} shape={shape:?}\n  C: {}\n  R: {}",
            fmt_proxy(&cp),
            fmt_proxy(&rp)
        );
        // Sanity: the vertex count the C code documents for this type.
        let expect = match ty {
            C2_TYPE_CIRCLE => 1,
            C2_TYPE_AABB => 4,
            _ => 2,
        };
        assert_eq!(cp.count, expect, "{label} {row}: unexpected proxy count");
    }
}

#[test]
fn row19_makeproxy_circle() {
    for_each_pair(|c, r, label| makeproxy_axis(c, r, label, "row19", 0x0014, C2_TYPE_CIRCLE));
}

#[test]
fn row20_makeproxy_aabb() {
    for_each_pair(|c, r, label| makeproxy_axis(c, r, label, "row20", 0x0015, C2_TYPE_AABB));
}

#[test]
fn row21_makeproxy_capsule() {
    for_each_pair(|c, r, label| makeproxy_axis(c, r, label, "row21", 0x0016, C2_TYPE_CAPSULE));
}
