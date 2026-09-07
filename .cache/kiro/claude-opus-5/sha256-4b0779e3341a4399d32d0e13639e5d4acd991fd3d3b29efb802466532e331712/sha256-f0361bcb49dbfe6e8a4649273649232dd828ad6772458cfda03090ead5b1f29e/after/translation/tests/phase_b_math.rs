//! Phase B — valid-path differential tests for the low-level (leaf) API.
//!
//! CONFIGS.md rows 1–9, 13–18, 19–34.
//! Every call goes through both `.so` exports; nothing is called directly.

mod common;

use common::*;

const N: usize = 4000;

// ---------------------------------------------------------------------------
// Row 1 — c2V / c2Sub / c2Add / c2Neg / c2Skew / c2CCW90 / c2Mulvs
// ---------------------------------------------------------------------------

#[test]
fn row01_vector_basics() {
    type F2 = unsafe extern "C" fn(f32, f32) -> c2v;
    type V1 = unsafe extern "C" fn(c2v) -> c2v;
    type V2 = unsafe extern "C" fn(c2v, c2v) -> c2v;
    type VS = unsafe extern "C" fn(c2v, f32) -> c2v;

    let (c_v, r_v) = pair::<F2>("c2V");
    let (c_sub, r_sub) = pair::<V2>("c2Sub");
    let (c_add, r_add) = pair::<V2>("c2Add");
    let (c_neg, r_neg) = pair::<V1>("c2Neg");
    let (c_skew, r_skew) = pair::<V1>("c2Skew");
    let (c_ccw, r_ccw) = pair::<V1>("c2CCW90");
    let (c_muls, r_muls) = pair::<VS>("c2Mulvs");

    let mut rng = Rng::new(0x0001);
    for i in 0..N {
        let a = rng.vec(100.0);
        let b = rng.vec(100.0);
        let s = rng.coord(10.0);
        unsafe {
            assert_v_bits!(c_v(a.x, a.y), r_v(a.x, a.y), "c2V #{i}");
            assert_v_bits!(c_sub(a, b), r_sub(a, b), "c2Sub #{i} {a:?} {b:?}");
            assert_v_bits!(c_add(a, b), r_add(a, b), "c2Add #{i} {a:?} {b:?}");
            assert_v_bits!(c_neg(a), r_neg(a), "c2Neg #{i} {a:?}");
            assert_v_bits!(c_skew(a), r_skew(a), "c2Skew #{i} {a:?}");
            assert_v_bits!(c_ccw(a), r_ccw(a), "c2CCW90 #{i} {a:?}");
            assert_v_bits!(c_muls(a, s), r_muls(a, s), "c2Mulvs #{i} {a:?} * {s}");
        }
    }

    // Exhaustive edge-value cross product.
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let a = c2v { x, y };
            for &sx in EDGE_F32 {
                for &sy in EDGE_F32 {
                    let b = c2v { x: sx, y: sy };
                    unsafe {
                        assert_v_bits!(c_sub(a, b), r_sub(a, b), "c2Sub edge {a:?} {b:?}");
                        assert_v_bits!(c_add(a, b), r_add(a, b), "c2Add edge {a:?} {b:?}");
                    }
                }
                unsafe {
                    assert_v_bits!(c_muls(a, sx), r_muls(a, sx), "c2Mulvs edge {a:?} {sx}");
                }
            }
            unsafe {
                assert_v_bits!(c_v(x, y), r_v(x, y), "c2V edge {x} {y}");
                assert_v_bits!(c_neg(a), r_neg(a), "c2Neg edge {a:?}");
                assert_v_bits!(c_skew(a), r_skew(a), "c2Skew edge {a:?}");
                assert_v_bits!(c_ccw(a), r_ccw(a), "c2CCW90 edge {a:?}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 2 — c2Dot / c2Det2 / c2Len
// ---------------------------------------------------------------------------

#[test]
fn row02_dot_det_len() {
    type FF2 = unsafe extern "C" fn(c2v, c2v) -> f32;
    type FF1 = unsafe extern "C" fn(c2v) -> f32;

    let (c_dot, r_dot) = pair::<FF2>("c2Dot");
    let (c_det, r_det) = pair::<FF2>("c2Det2");
    let (c_len, r_len) = pair::<FF1>("c2Len");

    let mut rng = Rng::new(0x0002);
    for i in 0..N {
        let a = rng.vec(1000.0);
        let b = rng.vec(1000.0);
        unsafe {
            assert_f32_bits!(c_dot(a, b), r_dot(a, b), "c2Dot #{i} {a:?} {b:?}");
            assert_f32_bits!(c_det(a, b), r_det(a, b), "c2Det2 #{i} {a:?} {b:?}");
            assert_f32_bits!(c_len(a), r_len(a), "c2Len #{i} {a:?}");
        }
    }

    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let a = c2v { x, y };
            unsafe {
                assert_f32_bits!(c_len(a), r_len(a), "c2Len edge {a:?}");
            }
            for &z in EDGE_F32 {
                let b = c2v { x: z, y: x };
                unsafe {
                    assert_f32_bits!(c_dot(a, b), r_dot(a, b), "c2Dot edge {a:?} {b:?}");
                    assert_f32_bits!(c_det(a, b), r_det(a, b), "c2Det2 edge {a:?} {b:?}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 3, 4 — c2Maxv / c2Minv / c2Clampv
// ---------------------------------------------------------------------------

#[test]
fn row03_row04_min_max_clamp() {
    type V2 = unsafe extern "C" fn(c2v, c2v) -> c2v;
    type V3 = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;

    let (c_max, r_max) = pair::<V2>("c2Maxv");
    let (c_min, r_min) = pair::<V2>("c2Minv");
    let (c_clamp, r_clamp) = pair::<V3>("c2Clampv");

    let mut rng = Rng::new(0x0003);
    for i in 0..N {
        let a = rng.vec(50.0);
        let b = rng.vec(50.0);
        // well-formed lo/hi
        let bb = rng.aabb(50.0);
        // possibly inverted lo/hi
        let raw = rng.aabb_raw(50.0);
        unsafe {
            assert_v_bits!(c_max(a, b), r_max(a, b), "c2Maxv #{i} {a:?} {b:?}");
            assert_v_bits!(c_min(a, b), r_min(a, b), "c2Minv #{i} {a:?} {b:?}");
            assert_v_bits!(
                c_clamp(a, bb.min, bb.max),
                r_clamp(a, bb.min, bb.max),
                "c2Clampv #{i} {a:?} in {bb:?}"
            );
            assert_v_bits!(
                c_clamp(a, raw.min, raw.max),
                r_clamp(a, raw.min, raw.max),
                "c2Clampv inverted #{i} {a:?} in {raw:?}"
            );
        }
    }

    // All 9 per-component orderings + equality, exhaustively over edge values.
    for &lo in EDGE_F32 {
        for &hi in EDGE_F32 {
            for &v in EDGE_F32 {
                let a = c2v { x: v, y: hi };
                let l = c2v { x: lo, y: lo };
                let h = c2v { x: hi, y: v };
                unsafe {
                    assert_v_bits!(c_max(a, l), r_max(a, l), "c2Maxv edge {a:?} {l:?}");
                    assert_v_bits!(c_min(a, h), r_min(a, h), "c2Minv edge {a:?} {h:?}");
                    assert_v_bits!(
                        c_clamp(a, l, h),
                        r_clamp(a, l, h),
                        "c2Clampv edge {a:?} {l:?} {h:?}"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — c2Div / c2Norm
// ---------------------------------------------------------------------------

#[test]
fn row05_div_norm() {
    type VS = unsafe extern "C" fn(c2v, f32) -> c2v;
    type V1 = unsafe extern "C" fn(c2v) -> c2v;

    let (c_div, r_div) = pair::<VS>("c2Div");
    let (c_norm, r_norm) = pair::<V1>("c2Norm");

    let mut rng = Rng::new(0x0005);
    for i in 0..N {
        let a = rng.vec(100.0);
        let d = rng.coord(10.0);
        unsafe {
            assert_v_bits!(c_div(a, d), r_div(a, d), "c2Div #{i} {a:?} / {d}");
            assert_v_bits!(c_norm(a), r_norm(a), "c2Norm #{i} {a:?}");
        }
    }
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let a = c2v { x, y };
            unsafe {
                assert_v_bits!(c_norm(a), r_norm(a), "c2Norm edge {a:?}");
            }
            for &d in EDGE_F32 {
                unsafe {
                    assert_v_bits!(c_div(a, d), r_div(a, d), "c2Div edge {a:?} / {d}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — c2RotIdentity / c2xIdentity
// ---------------------------------------------------------------------------

#[test]
fn row06_identities() {
    type R0 = unsafe extern "C" fn() -> c2r;
    type X0 = unsafe extern "C" fn() -> c2x;
    let (c_r, r_r) = pair::<R0>("c2RotIdentity");
    let (c_x, r_x) = pair::<X0>("c2xIdentity");
    unsafe {
        let (a, b) = (c_r(), r_r());
        assert_f32_bits!(a.c, b.c, "c2RotIdentity.c");
        assert_f32_bits!(a.s, b.s, "c2RotIdentity.s");
        let (a, b) = (c_x(), r_x());
        assert_v_bits!(a.p, b.p, "c2xIdentity.p");
        assert_f32_bits!(a.r.c, b.r.c, "c2xIdentity.r.c");
        assert_f32_bits!(a.r.s, b.r.s, "c2xIdentity.r.s");
    }
}

// ---------------------------------------------------------------------------
// Rows 7, 8 — c2Mulrv / c2MulrvT / c2Mulxv
// ---------------------------------------------------------------------------

#[test]
fn row07_row08_rotations() {
    type RV = unsafe extern "C" fn(c2r, c2v) -> c2v;
    type XV = unsafe extern "C" fn(c2x, c2v) -> c2v;

    let (c_mrv, r_mrv) = pair::<RV>("c2Mulrv");
    let (c_mrvt, r_mrvt) = pair::<RV>("c2MulrvT");
    let (c_mxv, r_mxv) = pair::<XV>("c2Mulxv");

    let mut rng = Rng::new(0x0007);
    for i in 0..N {
        let rot = rng.rot();
        let v = rng.vec(100.0);
        let x = rng.xform(100.0);
        unsafe {
            assert_v_bits!(c_mrv(rot, v), r_mrv(rot, v), "c2Mulrv #{i} {rot:?} {v:?}");
            assert_v_bits!(c_mrvt(rot, v), r_mrvt(rot, v), "c2MulrvT #{i} {rot:?} {v:?}");
            assert_v_bits!(c_mxv(x, v), r_mxv(x, v), "c2Mulxv #{i} {x:?} {v:?}");
        }
    }
    // translation-only, rotation-only, identity
    let cases = [
        c2x {
            p: c2v { x: 3.5, y: -7.25 },
            r: c2r { c: 1.0, s: 0.0 },
        },
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 0.0, s: 1.0 },
        },
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 1.0, s: 0.0 },
        },
    ];
    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let v = c2v { x, y };
            for xf in cases {
                unsafe {
                    assert_v_bits!(c_mxv(xf, v), r_mxv(xf, v), "c2Mulxv edge {xf:?} {v:?}");
                }
            }
            let rot = c2r { c: x, s: y };
            unsafe {
                assert_v_bits!(c_mrv(rot, v), r_mrv(rot, v), "c2Mulrv edge {rot:?} {v:?}");
                assert_v_bits!(
                    c_mrvt(rot, v),
                    r_mrvt(rot, v),
                    "c2MulrvT edge {rot:?} {v:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — c2BBVerts
// ---------------------------------------------------------------------------

#[test]
fn row09_bbverts() {
    type BB = unsafe extern "C" fn(*mut c2v, *mut c2AABB) -> ();
    let (c_f, r_f) = pair::<BB>("c2BBVerts");

    let mut rng = Rng::new(0x0009);
    let check = |bb: c2AABB, tag: &str| unsafe {
        // 8-slot buffers prefilled with a sentinel so we also verify nothing
        // outside verts[0..4] is written.
        let sentinel = c2v { x: -1.5, y: 2.5 };
        let mut co = [sentinel; 8];
        let mut ro = [sentinel; 8];
        let mut cb = bb;
        let mut rb = bb;
        c_f(co.as_mut_ptr(), &mut cb);
        r_f(ro.as_mut_ptr(), &mut rb);
        for k in 0..8 {
            assert_v_bits!(co[k], ro[k], "c2BBVerts {tag} out[{k}] bb={bb:?}");
        }
        // the input AABB must not be mutated
        assert!(
            aabb_bits_eq(cb, rb),
            "c2BBVerts {tag} mutated bb differently: C={cb:?} Rust={rb:?}"
        );
    };

    for i in 0..N {
        check(rng.aabb(100.0), &format!("rand#{i}"));
        check(rng.aabb_raw(100.0), &format!("raw#{i}"));
    }
    for &a in EDGE_F32 {
        for &b in EDGE_F32 {
            check(
                c2AABB {
                    min: c2v { x: a, y: b },
                    max: c2v { x: b, y: a },
                },
                "edge",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 13, 14, 15 — c2Support with count 1 / 2 / 4
// ---------------------------------------------------------------------------

#[test]
fn row13_row14_row15_support() {
    type SUP = unsafe extern "C" fn(*const c2v, i32, c2v) -> i32;
    let (c_f, r_f) = pair::<SUP>("c2Support");

    let mut rng = Rng::new(0x0013);
    for count in [1i32, 2, 4] {
        for i in 0..N {
            let verts: [c2v; 8] = std::array::from_fn(|k| {
                if k < count as usize {
                    rng.vec(100.0)
                } else {
                    c2v { x: 0.0, y: 0.0 }
                }
            });
            let d = rng.vec(10.0);
            unsafe {
                let (cv, rv) = (
                    c_f(verts.as_ptr(), count, d),
                    r_f(verts.as_ptr(), count, d),
                );
                assert_eq!(cv, rv, "c2Support count={count} #{i} d={d:?} {verts:?}");
            }
        }
        // Axis-aligned directions on a square: exercises the strict `>` tie rule.
        let square = [
            c2v { x: -1.0, y: -1.0 },
            c2v { x: 1.0, y: -1.0 },
            c2v { x: 1.0, y: 1.0 },
            c2v { x: -1.0, y: 1.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
        ];
        for &dx in EDGE_F32 {
            for &dy in EDGE_F32 {
                let d = c2v { x: dx, y: dy };
                unsafe {
                    let (cv, rv) = (
                        c_f(square.as_ptr(), count, d),
                        r_f(square.as_ptr(), count, d),
                    );
                    assert_eq!(cv, rv, "c2Support square count={count} d={d:?}");
                }
            }
        }
        // All-identical verts: every dot equal, so `>` never fires -> index 0.
        let same = [c2v { x: 4.0, y: -3.0 }; 8];
        for &dx in EDGE_F32 {
            let d = c2v { x: dx, y: dx };
            unsafe {
                assert_eq!(
                    c_f(same.as_ptr(), count, d),
                    r_f(same.as_ptr(), count, d),
                    "c2Support identical count={count} d={d:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 16, 17, 18 — c2GJKSimplexMetric for count 1 / 2 / 3
// ---------------------------------------------------------------------------

#[test]
fn row16_row17_row18_simplex_metric() {
    type MET = unsafe extern "C" fn(*mut c2Simplex) -> f32;
    let (c_f, r_f) = pair::<MET>("c2GJKSimplexMetric");

    let mut rng = Rng::new(0x0016);
    for count in [1i32, 2, 3] {
        for i in 0..N {
            let s = rng.simplex(100.0, count);
            unsafe {
                let mut cs = s;
                let mut rs = s;
                let (cv, rv) = (c_f(&mut cs), r_f(&mut rs));
                assert_f32_bits!(cv, rv, "c2GJKSimplexMetric count={count} #{i}");
                assert!(
                    simplex_bits_eq(&cs, &rs),
                    "c2GJKSimplexMetric count={count} #{i} mutated simplex differently"
                );
            }
        }
    }
    // Both winding orders for count==3, plus degenerate area==0.
    let p = |x: f32, y: f32| c2v { x, y };
    let mk = |a: c2v, b: c2v, c: c2v| {
        let mut s = c2Simplex::default();
        s.count = 3;
        s.div = 1.0;
        s.verts[0].p = a;
        s.verts[1].p = b;
        s.verts[2].p = c;
        s
    };
    for s in [
        mk(p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)),
        mk(p(0.0, 0.0), p(0.0, 1.0), p(1.0, 0.0)),
        mk(p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0)),
        mk(p(5.0, 5.0), p(5.0, 5.0), p(5.0, 5.0)),
    ] {
        unsafe {
            let mut cs = s;
            let mut rs = s;
            assert_f32_bits!(c_f(&mut cs), r_f(&mut rs), "metric winding {s:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 19, 20, 21 — c22 (all three branches)
// ---------------------------------------------------------------------------

#[test]
fn row19_row20_row21_c22() {
    type S1 = unsafe extern "C" fn(*mut c2Simplex) -> ();
    let (c_f, r_f) = pair::<S1>("c22");

    let mut branch_hits = [0usize; 3];
    let mut rng = Rng::new(0x0019);
    for i in 0..(N * 4) {
        let mut s = rng.simplex(20.0, 2);
        // widen coverage of the u/v sign cases by sometimes placing the
        // segment so the origin projects outside it
        if i % 3 == 0 {
            let base = rng.vec(20.0);
            let dir = rng.vec(5.0);
            s.verts[0].p = base;
            s.verts[1].p = c2v {
                x: base.x + dir.x,
                y: base.y + dir.y,
            };
        }
        unsafe {
            let mut cs = s;
            let mut rs = s;
            c_f(&mut cs);
            r_f(&mut rs);
            assert!(
                simplex_bits_eq(&cs, &rs),
                "c22 #{i} divergence\n in={s:?}\n  C={cs:?}\n Rs={rs:?}"
            );
            // classify by observable outcome for coverage accounting
            let idx = if cs.count == 2 {
                2
            } else if v_bits_eq(cs.verts[0].p, s.verts[0].p) {
                0
            } else {
                1
            };
            branch_hits[idx] += 1;
        }
    }
    assert!(
        branch_hits.iter().all(|&h| h > 0),
        "c22 branch coverage incomplete: {branch_hits:?}"
    );

    // Hand-built inputs that pin each branch deterministically.
    let mk = |a: c2v, b: c2v| {
        let mut s = c2Simplex::default();
        s.count = 2;
        s.div = 1.0;
        s.verts[0].p = a;
        s.verts[1].p = b;
        s
    };
    let p = |x: f32, y: f32| c2v { x, y };
    for s in [
        mk(p(1.0, 0.0), p(2.0, 0.0)),   // v <= 0 -> keep a
        mk(p(-2.0, 0.0), p(-1.0, 0.0)), // u <= 0 -> a = b
        mk(p(-1.0, 0.0), p(1.0, 0.0)),  // interior
        mk(p(0.0, 0.0), p(0.0, 0.0)),   // coincident
        mk(p(0.0, 0.0), p(1.0, 0.0)),   // a at origin
    ] {
        unsafe {
            let mut cs = s;
            let mut rs = s;
            c_f(&mut cs);
            r_f(&mut rs);
            assert!(simplex_bits_eq(&cs, &rs), "c22 pinned {s:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 22–29 — c23 (all seven branches + degenerate)
// ---------------------------------------------------------------------------

#[test]
fn row22_to_row29_c23() {
    type S1 = unsafe extern "C" fn(*mut c2Simplex) -> ();
    let (c_f, r_f) = pair::<S1>("c23");

    let mut rng = Rng::new(0x0022);
    let mut outcomes = std::collections::BTreeSet::new();
    for i in 0..(N * 6) {
        let mut s = rng.simplex(20.0, 3);
        // Sometimes build a genuine triangle around a random origin offset so
        // the vertex/edge/interior Voronoi regions are all reached.
        if i % 2 == 0 {
            let o = rng.vec(6.0);
            let r = rng.range(0.5, 8.0);
            let t0 = rng.range(0.0, 6.28);
            for k in 0..3 {
                let t = t0 + k as f32 * 2.094_395_1;
                s.verts[k].p = c2v {
                    x: o.x + r * t.cos(),
                    y: o.y + r * t.sin(),
                };
            }
        }
        unsafe {
            let mut cs = s;
            let mut rs = s;
            c_f(&mut cs);
            r_f(&mut rs);
            assert!(
                simplex_bits_eq(&cs, &rs),
                "c23 #{i} divergence\n in={s:?}\n  C={cs:?}\n Rs={rs:?}"
            );
            outcomes.insert((cs.count, cs.div.to_bits() == 1.0f32.to_bits()));
        }
    }
    // count 1 (div==1), count 2, count 3 must all have been produced.
    assert!(
        outcomes.iter().any(|&(c, _)| c == 1)
            && outcomes.iter().any(|&(c, _)| c == 2)
            && outcomes.iter().any(|&(c, _)| c == 3),
        "c23 outcome coverage incomplete: {outcomes:?}"
    );

    // Pinned triangles: origin in each vertex region, each edge region, inside,
    // plus fully degenerate configurations.
    let p = |x: f32, y: f32| c2v { x, y };
    let mk = |a: c2v, b: c2v, c: c2v| {
        let mut s = c2Simplex::default();
        s.count = 3;
        s.div = 1.0;
        s.verts[0].p = a;
        s.verts[1].p = b;
        s.verts[2].p = c;
        // give the sv slots distinguishable witness data so the shuffles
        // (a=b, b=c, b=a, a=c) are observable
        for (k, v) in s.verts.iter_mut().enumerate() {
            v.sA = c2v {
                x: k as f32,
                y: -(k as f32),
            };
            v.sB = c2v {
                x: 10.0 + k as f32,
                y: -10.0 - k as f32,
            };
            v.iA = k as i32;
            v.iB = (k + 4) as i32;
        }
        s
    };
    let tris = [
        mk(p(1.0, 1.0), p(3.0, 1.0), p(1.0, 3.0)),      // vertex A region
        mk(p(3.0, 1.0), p(1.0, 1.0), p(1.0, 3.0)),      // vertex B region
        mk(p(3.0, 1.0), p(1.0, 3.0), p(1.0, 1.0)),      // vertex C region
        mk(p(-1.0, 1.0), p(1.0, 1.0), p(0.0, 3.0)),     // edge AB region
        mk(p(0.0, 3.0), p(-1.0, 1.0), p(1.0, 1.0)),     // edge BC region
        mk(p(1.0, 1.0), p(0.0, 3.0), p(-1.0, 1.0)),     // edge CA region
        mk(p(-1.0, -1.0), p(2.0, 0.0), p(0.0, 2.0)),    // interior
        mk(p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0)),      // collinear, area==0
        mk(p(1.0, 1.0), p(1.0, 1.0), p(1.0, 1.0)),      // all coincident
        mk(p(0.0, 0.0), p(0.0, 0.0), p(1.0, 0.0)),      // two coincident at origin
        mk(p(-1.0, 0.0), p(1.0, 0.0), p(0.0, 0.0)),     // origin is a vertex
    ];
    for (k, s) in tris.iter().enumerate() {
        unsafe {
            let mut cs = *s;
            let mut rs = *s;
            c_f(&mut cs);
            r_f(&mut rs);
            assert!(
                simplex_bits_eq(&cs, &rs),
                "c23 pinned #{k}\n in={s:?}\n  C={cs:?}\n Rs={rs:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 30, 31, 32 — c2D
// ---------------------------------------------------------------------------

#[test]
fn row30_row31_row32_c2d() {
    type DF = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
    let (c_f, r_f) = pair::<DF>("c2D");

    let mut rng = Rng::new(0x0030);
    for count in [1i32, 2] {
        for i in 0..(N * 2) {
            let s = rng.simplex(100.0, count);
            unsafe {
                let mut cs = s;
                let mut rs = s;
                assert_v_bits!(c_f(&mut cs), r_f(&mut rs), "c2D count={count} #{i} {s:?}");
            }
        }
    }
    // count==2 with det>0 and det<=0 pinned, plus det==0
    let p = |x: f32, y: f32| c2v { x, y };
    let mk = |a: c2v, b: c2v| {
        let mut s = c2Simplex::default();
        s.count = 2;
        s.div = 1.0;
        s.verts[0].p = a;
        s.verts[1].p = b;
        s
    };
    for s in [
        mk(p(1.0, -1.0), p(1.0, 1.0)),
        mk(p(1.0, 1.0), p(1.0, -1.0)),
        mk(p(1.0, 0.0), p(2.0, 0.0)), // det == 0
        mk(p(0.0, 0.0), p(0.0, 0.0)),
    ] {
        unsafe {
            let mut cs = s;
            let mut rs = s;
            assert_v_bits!(c_f(&mut cs), r_f(&mut rs), "c2D pinned {s:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 33, 34 — c2L / c2Witness
// ---------------------------------------------------------------------------

#[test]
fn row33_row34_l_and_witness() {
    type LF = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
    type WF = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v) -> ();
    let (c_l, r_l) = pair::<LF>("c2L");
    let (c_w, r_w) = pair::<WF>("c2Witness");

    let mut rng = Rng::new(0x0033);
    for count in [1i32, 2, 3] {
        for i in 0..(N * 2) {
            let s = rng.simplex(100.0, count);
            unsafe {
                let mut cs = s;
                let mut rs = s;
                assert_v_bits!(c_l(&mut cs), r_l(&mut rs), "c2L count={count} #{i}");

                let sent = c2v { x: 1234.5, y: -678.9 };
                let (mut ca, mut cb) = (sent, sent);
                let (mut ra, mut rb) = (sent, sent);
                let mut cs = s;
                let mut rs = s;
                c_w(&mut cs, &mut ca, &mut cb);
                r_w(&mut rs, &mut ra, &mut rb);
                assert_v_bits!(ca, ra, "c2Witness a count={count} #{i} {s:?}");
                assert_v_bits!(cb, rb, "c2Witness b count={count} #{i} {s:?}");
                assert!(
                    simplex_bits_eq(&cs, &rs),
                    "c2Witness count={count} #{i} mutated simplex differently"
                );
            }
        }
    }
    // div exactly 0 / -0 / inf / nan (Row 10, 11 of ERRORS.md also covers this)
    for &div in &[0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NAN, f32::MIN_POSITIVE] {
        for count in [1i32, 2, 3] {
            let mut s = rng.simplex(10.0, count);
            s.div = div;
            unsafe {
                let mut cs = s;
                let mut rs = s;
                assert_v_bits!(c_l(&mut cs), r_l(&mut rs), "c2L div={div} count={count}");
                let (mut ca, mut cb) = (c2v::default(), c2v::default());
                let (mut ra, mut rb) = (c2v::default(), c2v::default());
                let mut cs = s;
                let mut rs = s;
                c_w(&mut cs, &mut ca, &mut cb);
                r_w(&mut rs, &mut ra, &mut rb);
                assert_v_bits!(ca, ra, "c2Witness a div={div} count={count}");
                assert_v_bits!(cb, rb, "c2Witness b div={div} count={count}");
            }
        }
    }
}
