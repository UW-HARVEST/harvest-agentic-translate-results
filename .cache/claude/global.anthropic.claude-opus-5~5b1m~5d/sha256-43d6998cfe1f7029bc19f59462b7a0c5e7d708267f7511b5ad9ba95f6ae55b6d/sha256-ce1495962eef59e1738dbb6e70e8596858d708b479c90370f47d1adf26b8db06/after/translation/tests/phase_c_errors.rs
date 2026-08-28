//! Phase C — error / rejection path differential tests.
//! One test (or one clearly-labelled block) per row of `ERRORS.md`.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_int, c_void};

const N: usize = 1024;
const KINDS: [u32; 3] = [0, 1, 2];

/// Every "not a valid variant" int a caller can push through a `C2_TYPE`
/// parameter.  C enums accept any `int`, so these are real inputs.
const BAD_TYPES: [c_int; 10] = [
    C2_TYPE_POLY, // valid variant, but unhandled by c2MakeProxy/c2Collide
    -1,
    4,
    5,
    99,
    255,
    256,
    -12345,
    c_int::MIN,
    c_int::MAX,
];

const BAD_COUNTS: [c_int; 9] = [0, -1, 4, 5, 6, 100, -100, c_int::MIN, c_int::MAX];

fn sentinel_proxy() -> c2Proxy {
    let mut p = c2Proxy {
        radius: -98765.5,
        count: -4242,
        verts: [c2v::default(); 8],
    };
    for (i, v) in p.verts.iter_mut().enumerate() {
        *v = c2v {
            x: 100.0 + i as f32,
            y: -100.0 - i as f32,
        };
    }
    p
}

// =========================================================================
// rows 1, 2 — c2MakeProxy with an unhandled / out-of-range type
// =========================================================================
#[test]
fn err_makeproxy_unhandled_type() {
    let mut acc = DiffAccum::new("err_makeproxy_unhandled_type");
    let mut rng = Rng::new(0xceed_0001);
    for &t in &BAD_TYPES {
        for i in 0..N {
            // give it a real shape so a *wrong* implementation would fill it in
            let cap = rng.capsule();
            acc.check(format!("t={t} #{i}"), |s| {
                let mut p = sentinel_proxy();
                let cap = cap;
                c2MakeProxy(s, &cap as *const c2Capsule as *const c_void, t, &mut p);
                p
            });
        }
    }
    // and the sentinel must come back completely unchanged
    let mut p = sentinel_proxy();
    let cap = rng.capsule();
    c2MakeProxy(
        Side::C,
        &cap as *const c2Capsule as *const c_void,
        C2_TYPE_POLY,
        &mut p,
    );
    assert!(
        p.bit_eq(&sentinel_proxy()),
        "C wrote to the proxy for C2_TYPE_POLY: {}",
        p.show()
    );
    acc.finish();
}

// =========================================================================
// rows 3, 4, 5 — c2GJKSimplexMetric default/case-1
// rows 6, 7, 9 — c2D / c2L / c2Witness default
// =========================================================================
fn simplex_with_count(rng: &mut Rng, count: c_int) -> c2Simplex {
    let mk = |rng: &mut Rng| c2sv {
        sA: rng.vec(),
        sB: rng.vec(),
        p: rng.vec(),
        u: rng.coord(),
        iA: rng.below(8) as c_int,
        iB: rng.below(8) as c_int,
    };
    c2Simplex {
        a: mk(rng),
        b: mk(rng),
        c: mk(rng),
        d: mk(rng),
        div: rng.coord(),
        count,
    }
}

#[test]
fn err_simplex_metric_bad_count() {
    let mut acc = DiffAccum::new("err_simplex_metric_bad_count");
    let mut rng = Rng::new(0xceed_0002);
    for &count in &[0i32, 1, 4, 5, 6, -1, -100, i32::MIN, i32::MAX] {
        for i in 0..N {
            let s0 = simplex_with_count(&mut rng, count);
            let out = acc_ret(&mut acc, format!("count={count} #{i}"), |side| {
                let mut s = s0;
                let r = c2GJKSimplexMetric(side, &mut s);
                (r, s)
            });
            assert_eq!(
                out.0.to_bits(),
                0.0f32.to_bits(),
                "C must return +0.0 for count={count}, got {}",
                out.0.show()
            );
        }
    }
    acc.finish();
}

fn acc_ret<R: BitEq + Copy, F: FnMut(Side) -> R>(
    acc: &mut DiffAccum,
    label: String,
    mut f: F,
) -> R {
    let c = f(Side::C);
    acc.check(label, |s| f(s));
    c
}

#[test]
fn err_c2d_bad_count() {
    let mut acc = DiffAccum::new("err_c2d_bad_count");
    let mut rng = Rng::new(0xceed_0003);
    for &count in &BAD_COUNTS {
        for i in 0..N {
            let s0 = simplex_with_count(&mut rng, count);
            let out = acc_ret(&mut acc, format!("count={count} #{i}"), |side| {
                let mut s = s0;
                let r = c2D(side, &mut s);
                (r, s)
            });
            assert!(
                out.0.x.to_bits() == 0 && out.0.y.to_bits() == 0,
                "C must return (0,0) for count={count}, got {}",
                out.0.show()
            );
        }
    }
    // count == 3 explicitly (shares the `default:` label in the C switch)
    for i in 0..N {
        let s0 = simplex_with_count(&mut rng, 3);
        acc.check(format!("count=3 #{i}"), |side| {
            let mut s = s0;
            let r = c2D(side, &mut s);
            (r, s)
        });
    }
    acc.finish();
}

#[test]
fn err_c2l_bad_count() {
    let mut acc = DiffAccum::new("err_c2l_bad_count");
    let mut rng = Rng::new(0xceed_0004);
    for &count in &[0i32, 3, 4, 5, -1, -100, i32::MIN, i32::MAX] {
        for i in 0..N {
            let s0 = simplex_with_count(&mut rng, count);
            let out = acc_ret(&mut acc, format!("count={count} #{i}"), |side| {
                let mut s = s0;
                let r = c2L(side, &mut s);
                (r, s)
            });
            assert!(out.0.x.to_bits() == 0 && out.0.y.to_bits() == 0);
        }
    }
    acc.finish();
}

#[test]
fn err_c2l_div_zero() {
    let mut acc = DiffAccum::new("err_c2l_div_zero");
    let mut rng = Rng::new(0xceed_0005);
    for &div in &[0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for &count in &[1i32, 2] {
            for i in 0..N {
                let mut s0 = simplex_with_count(&mut rng, count);
                s0.div = div;
                acc.check(format!("div={div:?} count={count} #{i}"), |side| {
                    let mut s = s0;
                    let r = c2L(side, &mut s);
                    (r, s)
                });
                // u == 0 as well ⇒ 0 * inf
                let mut s1 = s0;
                s1.a.u = 0.0;
                s1.b.u = 0.0;
                acc.check(format!("div={div:?} u=0 count={count} #{i}"), |side| {
                    let mut s = s1;
                    let r = c2L(side, &mut s);
                    (r, s)
                });
            }
        }
    }
    acc.finish();
}

#[test]
fn err_witness_bad_count() {
    let mut acc = DiffAccum::new("err_witness_bad_count");
    let mut rng = Rng::new(0xceed_0006);
    for &count in &[0i32, 4, 5, 6, -1, -100, i32::MIN, i32::MAX] {
        for i in 0..N {
            let s0 = simplex_with_count(&mut rng, count);
            let out = acc_ret(&mut acc, format!("count={count} #{i}"), |side| {
                let mut s = s0;
                let mut a = c2v { x: 6.0, y: -6.0 };
                let mut b = c2v { x: -6.0, y: 6.0 };
                c2Witness(side, &mut s, &mut a, &mut b);
                (a, b, s)
            });
            assert!(
                out.0.x.to_bits() == 0
                    && out.0.y.to_bits() == 0
                    && out.1.x.to_bits() == 0
                    && out.1.y.to_bits() == 0,
                "C must zero both witnesses for count={count}"
            );
        }
    }
    acc.finish();
}

#[test]
fn err_witness_div_zero() {
    let mut acc = DiffAccum::new("err_witness_div_zero");
    let mut rng = Rng::new(0xceed_0007);
    for &div in &[0.0f32, -0.0, f32::NAN, f32::INFINITY] {
        for &count in &[1i32, 2, 3] {
            for i in 0..N / 2 {
                let mut s0 = simplex_with_count(&mut rng, count);
                s0.div = div;
                acc.check(format!("div={div:?} count={count} #{i}"), |side| {
                    let mut s = s0;
                    let mut a = c2v { x: 6.0, y: -6.0 };
                    let mut b = c2v { x: -6.0, y: 6.0 };
                    c2Witness(side, &mut s, &mut a, &mut b);
                    (a, b, s)
                });
                let mut s1 = s0;
                s1.a.u = 0.0;
                s1.b.u = 0.0;
                s1.c.u = 0.0;
                acc.check(format!("div={div:?} u=0 count={count} #{i}"), |side| {
                    let mut s = s1;
                    let mut a = c2v { x: 6.0, y: -6.0 };
                    let mut b = c2v { x: -6.0, y: 6.0 };
                    c2Witness(side, &mut s, &mut a, &mut b);
                    (a, b, s)
                });
            }
        }
    }
    acc.finish();
}

// =========================================================================
// rows 11, 12 — c2Support
// =========================================================================
#[test]
fn err_support_nonpositive_count() {
    let mut acc = DiffAccum::new("err_support_nonpositive_count");
    let mut rng = Rng::new(0xceed_0008);
    for &count in &[0i32, -1, -2, -100, i32::MIN] {
        for i in 0..N {
            let mut verts = [c2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.vec();
            }
            let d = rng.vec();
            let out = acc_ret(&mut acc, format!("count={count} #{i}"), |s| {
                c2Support(s, verts.as_ptr(), count, d)
            });
            assert_eq!(out, 0, "C must return 0 for count={count}");
        }
    }
    acc.finish();
}

#[test]
fn err_support_nan_dir() {
    let mut acc = DiffAccum::new("err_support_nan_dir");
    let mut rng = Rng::new(0xceed_0009);
    let nan_dirs = [
        c2v {
            x: f32::NAN,
            y: f32::NAN,
        },
        c2v {
            x: f32::NAN,
            y: 0.0,
        },
        c2v {
            x: 0.0,
            y: f32::NAN,
        },
        c2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        },
    ];
    for (k, d) in nan_dirs.iter().enumerate() {
        for count in 1..=8i32 {
            for i in 0..N / 4 {
                let mut verts = [c2v::default(); 8];
                for v in verts.iter_mut() {
                    *v = rng.vec();
                }
                let d = *d;
                acc.check(format!("dir={k} count={count} #{i}"), |s| {
                    c2Support(s, verts.as_ptr(), count, d)
                });
            }
        }
    }
    // all-NaN verts too
    for count in 1..=8i32 {
        for i in 0..N / 4 {
            let verts = [c2v {
                x: f32::NAN,
                y: f32::NAN,
            }; 8];
            let d = rng.vec();
            acc.check(format!("nanverts count={count} #{i}"), |s| {
                c2Support(s, verts.as_ptr(), count, d)
            });
        }
    }
    acc.finish();
}

// =========================================================================
// rows 13..20 — float-domain errors in the leaf helpers
// =========================================================================
#[test]
fn err_div_zero() {
    let mut acc = DiffAccum::new("err_div_zero");
    let mut rng = Rng::new(0xceed_000a);
    for &b in &[
        0.0f32,
        -0.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ] {
        for i in 0..N {
            let v = if i % 3 == 0 {
                rng.special_vec()
            } else {
                rng.vec()
            };
            acc.check(format!("b={b:?} #{i} {v:?}"), |s| c2Div(s, v, b));
        }
        // exact zero / signed zero numerators (0 * inf ⇒ NaN)
        for &(x, y) in &[
            (0.0f32, 0.0f32),
            (-0.0, 0.0),
            (0.0, -0.0),
            (-0.0, -0.0),
        ] {
            let v = c2v { x, y };
            acc.check(format!("b={b:?} zeronum {x:?},{y:?}"), |s| c2Div(s, v, b));
        }
    }
    acc.finish();
}

#[test]
fn err_norm_zero_vector() {
    let mut acc = DiffAccum::new("err_norm_zero_vector");
    let mut rng = Rng::new(0xceed_000b);
    let zeros = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ];
    for (k, v) in zeros.iter().enumerate() {
        let v = *v;
        let out = acc_ret(&mut acc, format!("zero={k}"), |s| c2Norm(s, v));
        assert!(
            out.x.is_nan() && out.y.is_nan(),
            "C must produce NaN for c2Norm(0): {}",
            out.show()
        );
    }
    // NaN / inf components
    for i in 0..(N * 4) {
        let v = if i % 2 == 0 {
            rng.special_vec()
        } else {
            rng.any_vec()
        };
        acc.check(format!("special #{i} {v:?}"), |s| c2Norm(s, v));
    }
    // subnormal vectors — c2Len underflows to 0 ⇒ 1/0 = inf ⇒ inf * subnormal
    for i in 0..N {
        let e = 1 + (i as u32 % 8);
        let v = c2v {
            x: f32::from_bits(e),
            y: f32::from_bits(e + 1),
        };
        acc.check(format!("subnormal #{i} {v:?}"), |s| c2Norm(s, v));
    }
    acc.finish();
}

#[test]
fn err_len_overflow() {
    let mut acc = DiffAccum::new("err_len_overflow");
    let big = [
        f32::MAX,
        -f32::MAX,
        1.0e30,
        -1.0e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &x in &big {
        for &y in &big {
            let v = c2v { x, y };
            acc.check(format!("({x:?},{y:?})"), |s| c2Len(s, v));
        }
    }
    // exactly at the overflow boundary
    for k in 0..64u32 {
        let x = f32::from_bits(0x7f7f_ffff - k);
        let v = c2v { x, y: x };
        acc.check(format!("boundary k={k}"), |s| c2Len(s, v));
    }
    acc.finish();
}

#[test]
fn err_len_nan() {
    let mut acc = DiffAccum::new("err_len_nan");
    let mut rng = Rng::new(0xceed_000c);
    // glibc `sqrtf` (C) vs the `sqrtss` instruction (Rust) must agree, incl. on
    // signalling NaNs and on the inf−inf ⇒ NaN case inside c2Dot.
    for i in 0..(N * 4) {
        let v = match i % 4 {
            0 => c2v {
                x: f32::NAN,
                y: rng.coord(),
            },
            1 => c2v {
                x: rng.coord(),
                y: f32::NAN,
            },
            2 => c2v {
                // signalling NaN
                x: f32::from_bits(0x7f80_0001 | (rng.next_u32() & 0x003f_ffff)),
                y: rng.coord(),
            },
            _ => c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
        };
        acc.check(format!("#{i} {v:?}"), |s| c2Len(s, v));
    }
    // every NaN payload/sign combination in a small sweep
    for k in 0..512u32 {
        let bits = 0x7f80_0000 | (k << 8) | 1;
        for &sign in &[0u32, 0x8000_0000] {
            let v = c2v {
                x: f32::from_bits(bits | sign),
                y: 0.0,
            };
            acc.check(format!("payload k={k} sign={sign:#x}"), |s| c2Len(s, v));
        }
    }
    acc.finish();
}

#[test]
fn err_intersect_degenerate() {
    let mut acc = DiffAccum::new("err_intersect_degenerate");
    let mut rng = Rng::new(0xceed_000d);
    // da == db ⇒ da/(da-db) divides by zero
    for i in 0..(N * 2) {
        let (a, b) = (rng.vec(), rng.vec());
        let da = rng.coord();
        acc.check(format!("equal #{i} da={da:?}"), |s| {
            c2Intersect(s, a, b, da, da)
        });
    }
    // da == db == 0 ⇒ 0/0
    for i in 0..N {
        let (a, b) = (rng.vec(), rng.vec());
        for &(da, db) in &[(0.0f32, 0.0f32), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
            acc.check(format!("zero #{i} {da:?}/{db:?}"), |s| {
                c2Intersect(s, a, b, da, db)
            });
        }
    }
    // inf / NaN
    for i in 0..(N * 2) {
        let (a, b) = (rng.special_vec(), rng.special_vec());
        let (da, db) = (rng.special(), rng.special());
        acc.check(format!("special #{i}"), |s| c2Intersect(s, a, b, da, db));
    }
    acc.finish();
}

// =========================================================================
// row 21 — c2PlaneAt with an out-of-range index
// =========================================================================
/// A `c2Poly` embedded in a bigger `#[repr(C)]` block so that a negative index
/// reads *our* memory (deterministic for both sides) instead of the C's frame.
#[repr(C)]
#[derive(Copy, Clone)]
struct PolyBox {
    pre: [u32; 8],
    poly: c2Poly,
    post: [u32; 8],
}

#[test]
fn err_planeat_oob_index() {
    let mut acc = DiffAccum::new("err_planeat_oob_index");
    let mut rng = Rng::new(0xceed_000e);
    for count in 1..=8i32 {
        for i in 0..N / 2 {
            let verts = rng.convex_poly_verts(count as usize);
            let poly = make_poly(&verts, count);
            let mut bx = PolyBox {
                pre: [0xDEAD_BEEF; 8],
                poly,
                post: [0xFEED_FACE; 8],
            };
            for k in 0..8 {
                bx.pre[k] = rng.next_u32();
                bx.post[k] = rng.next_u32();
            }
            let bx = bx;
            // in-range, one past `count`, the array end, and one before the start
            for idx in -1..=8i32 {
                acc.check(format!("count={count} #{i} idx={idx}"), |s| {
                    c2PlaneAt(s, &bx.poly as *const c2Poly, idx)
                });
            }
        }
    }
    acc.finish();
}

// =========================================================================
// rows 22, 23, 24 — c2AABBtoAABBManifold rejections
// =========================================================================
#[test]
fn err_aabb_aabb_no_x_overlap() {
    let mut acc = DiffAccum::new("err_aabb_aabb_no_x_overlap");
    let mut rng = Rng::new(0xceed_000f);
    for i in 0..(N * 4) {
        let ex = 0.5 + rng.unit() * 2.0;
        let ey = 0.5 + rng.unit() * 2.0;
        let A = c2AABB {
            min: c2v { x: -ex, y: -ey },
            max: c2v { x: ex, y: ey },
        };
        // separate strictly along x, keep y overlapping
        let gap = 0.01 + rng.unit() * 5.0;
        let sx = (2.0 * ex + gap) * if rng.bool() { 1.0 } else { -1.0 };
        let B = c2AABB {
            min: c2v { x: sx - ex, y: -ey },
            max: c2v { x: sx + ex, y: ey },
        };
        let m = acc_ret(&mut acc, format!("#{i}"), |s| {
            with_sentinel(|m| c2AABBtoAABBManifold(s, A, B, m))
        });
        assert_eq!(m.count, 0, "expected rejection");
        // everything except `count` must be the untouched sentinel
        assert!(
            m.depths[0].to_bits() == SENTINEL_MANIFOLD.depths[0].to_bits()
                && m.n.bit_eq(&SENTINEL_MANIFOLD.n),
            "C wrote fields on the dx<0 rejection path: {}",
            m.show()
        );
    }
    acc.finish();
}

#[test]
fn err_aabb_aabb_no_y_overlap() {
    let mut acc = DiffAccum::new("err_aabb_aabb_no_y_overlap");
    let mut rng = Rng::new(0xceed_0010);
    for i in 0..(N * 4) {
        let ex = 0.5 + rng.unit() * 2.0;
        let ey = 0.5 + rng.unit() * 2.0;
        let A = c2AABB {
            min: c2v { x: -ex, y: -ey },
            max: c2v { x: ex, y: ey },
        };
        // x overlaps, y does not ⇒ passes the dx test, fails the dy test
        let gap = 0.01 + rng.unit() * 5.0;
        let sy = (2.0 * ey + gap) * if rng.bool() { 1.0 } else { -1.0 };
        let B = c2AABB {
            min: c2v { x: -ex, y: sy - ey },
            max: c2v { x: ex, y: sy + ey },
        };
        let m = acc_ret(&mut acc, format!("#{i}"), |s| {
            with_sentinel(|m| c2AABBtoAABBManifold(s, A, B, m))
        });
        assert_eq!(m.count, 0, "expected rejection");
        assert!(m.n.bit_eq(&SENTINEL_MANIFOLD.n));
    }
    // exact-touch boundary (dx == 0 / dy == 0 ⇒ NOT rejected)
    for k in 0..128 {
        let e = 0.5 + k as f32 * 0.25;
        let A = c2AABB {
            min: c2v { x: -e, y: -e },
            max: c2v { x: e, y: e },
        };
        let B = c2AABB {
            min: c2v { x: e, y: -e },
            max: c2v { x: 3.0 * e, y: e },
        };
        acc.check(format!("touch k={k}"), |s| {
            with_sentinel(|m| c2AABBtoAABBManifold(s, A, B, m))
        });
    }
    acc.finish();
}

#[test]
fn err_aabb_aabb_nan() {
    let mut acc = DiffAccum::new("err_aabb_aabb_nan");
    let mut rng = Rng::new(0xceed_0011);
    for i in 0..(N * 4) {
        let A = c2AABB {
            min: rng.special_vec(),
            max: rng.special_vec(),
        };
        let B = c2AABB {
            min: rng.special_vec(),
            max: rng.special_vec(),
        };
        acc.check(format!("#{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2AABBtoAABBManifold(s, A, B, m))
        });
    }
    // one NaN coordinate at a time
    for slot in 0..8 {
        for i in 0..N / 4 {
            let mut vals: [f32; 8] = std::array::from_fn(|_| rng.coord());
            vals[slot] = f32::NAN;
            let A = c2AABB {
                min: c2v {
                    x: vals[0],
                    y: vals[1],
                },
                max: c2v {
                    x: vals[2],
                    y: vals[3],
                },
            };
            let B = c2AABB {
                min: c2v {
                    x: vals[4],
                    y: vals[5],
                },
                max: c2v {
                    x: vals[6],
                    y: vals[7],
                },
            };
            acc.check(format!("slot={slot} #{i}"), |s| {
                with_sentinel(|m| c2AABBtoAABBManifold(s, A, B, m))
            });
        }
    }
    acc.finish();
}

// =========================================================================
// rows 25, 26, 27 — c2CircletoCircleManifold
// =========================================================================
#[test]
fn err_circle_circle_reject() {
    let mut acc = DiffAccum::new("err_circle_circle_reject");
    let mut rng = Rng::new(0xceed_0012);
    for i in 0..(N * 4) {
        let rA = rng.unit() * 2.0;
        let rB = rng.unit() * 2.0;
        let A = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: rA,
        };
        let gap = rng.unit() * 5.0;
        let B = c2Circle {
            p: c2v {
                x: rA + rB + gap,
                y: 0.0,
            },
            r: rB,
        };
        let m = acc_ret(&mut acc, format!("#{i}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
        assert_eq!(m.count, 0);
        assert!(m.n.bit_eq(&SENTINEL_MANIFOLD.n) && m.depths[0].to_bits() == SENTINEL_MANIFOLD.depths[0].to_bits());
    }
    // exact touch: d2 == r*r ⇒ `<` is false ⇒ rejected
    for k in 0..256 {
        let rA = 0.25 + k as f32 * 0.125;
        let rB = 1.0;
        let A = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: rA,
        };
        let B = c2Circle {
            p: c2v { x: rA + rB, y: 0.0 },
            r: rB,
        };
        acc.check(format!("touch k={k}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
    }
    acc.finish();
}

#[test]
fn err_circle_circle_negative_radius() {
    let mut acc = DiffAccum::new("err_circle_circle_negative_radius");
    let mut rng = Rng::new(0xceed_0013);
    for i in 0..(N * 2) {
        let A = c2Circle {
            p: rng.vec(),
            r: -(0.1 + rng.unit() * 3.0),
        };
        let B = c2Circle {
            p: rng.vec(),
            r: -(0.1 + rng.unit() * 3.0),
        };
        acc.check(format!("both-neg #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
        let C = c2Circle {
            p: rng.vec(),
            r: 0.1 + rng.unit() * 3.0,
        };
        acc.check(format!("one-neg #{i}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, C, m))
        });
        acc.check(format!("one-neg-rev #{i}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, C, A, m))
        });
    }
    acc.finish();
}

#[test]
fn err_circle_circle_coincident() {
    let mut acc = DiffAccum::new("err_circle_circle_coincident");
    let mut rng = Rng::new(0xceed_0014);
    for i in 0..(N * 2) {
        let p = rng.vec();
        let A = c2Circle { p, r: rng.radius() };
        let B = c2Circle { p, r: rng.radius() };
        let m = acc_ret(&mut acc, format!("#{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
        if m.count == 1 {
            assert_eq!(m.n.x.to_bits(), 0.0f32.to_bits());
            assert_eq!(m.n.y.to_bits(), 1.0f32.to_bits());
        }
    }
    // signed-zero difference: B.p == A.p but reached via -0.0
    for i in 0..N {
        let x = rng.coord();
        let y = rng.coord();
        let A = c2Circle {
            p: c2v { x, y },
            r: 1.0,
        };
        let B = c2Circle {
            p: c2v { x, y },
            r: 1.0,
        };
        acc.check(format!("signedzero #{i}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, A, B, m))
        });
    }
    acc.finish();
}

// =========================================================================
// rows 28, 29, 30 — c2CircletoAABBManifold
// =========================================================================
#[test]
fn err_circle_aabb_reject() {
    let mut acc = DiffAccum::new("err_circle_aabb_reject");
    let mut rng = Rng::new(0xceed_0015);
    for i in 0..(N * 4) {
        let e = 0.5 + rng.unit() * 2.0;
        let B = c2AABB {
            min: c2v { x: -e, y: -e },
            max: c2v { x: e, y: e },
        };
        let r = rng.unit();
        let d = e + r + 0.01 + rng.unit() * 4.0;
        let ang = rng.unit() * std::f32::consts::TAU;
        let A = c2Circle {
            p: c2v {
                x: d * ang.cos() * 2.0,
                y: d * ang.sin() * 2.0,
            },
            r,
        };
        let m = acc_ret(&mut acc, format!("#{i}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
        if m.count == 0 {
            assert!(m.n.bit_eq(&SENTINEL_MANIFOLD.n));
        }
    }
    // exact touch on a face: d2 == r2 ⇒ rejected
    for k in 0..256 {
        let e = 1.0 + k as f32 * 0.125;
        let r = 1.0;
        let B = c2AABB {
            min: c2v { x: -e, y: -e },
            max: c2v { x: e, y: e },
        };
        let A = c2Circle {
            p: c2v { x: e + r, y: 0.0 },
            r,
        };
        let m = acc_ret(&mut acc, format!("touch k={k}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
        assert_eq!(m.count, 0, "exact touch must be rejected (k={k})");
    }
    // r == 0 ⇒ r2 == 0 ⇒ `d2 < 0` never true ⇒ always rejected
    for i in 0..N {
        let A = c2Circle {
            p: rng.vec(),
            r: 0.0,
        };
        let B = rng.aabb();
        let m = acc_ret(&mut acc, format!("r0 #{i}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
        assert_eq!(m.count, 0);
    }
    acc.finish();
}

#[test]
fn err_circle_aabb_center_inside() {
    let mut acc = DiffAccum::new("err_circle_aabb_center_inside");
    let mut rng = Rng::new(0xceed_0016);
    let mut xwins = 0;
    let mut ywins = 0;
    for i in 0..(N * 4) {
        let ex = 0.5 + rng.unit() * 3.0;
        let ey = 0.5 + rng.unit() * 3.0;
        let cx = rng.sym(5.0);
        let cy = rng.sym(5.0);
        let B = c2AABB {
            min: c2v {
                x: cx - ex,
                y: cy - ey,
            },
            max: c2v {
                x: cx + ex,
                y: cy + ey,
            },
        };
        // strictly inside ⇒ clamp is a no-op ⇒ d2 == 0
        let A = c2Circle {
            p: c2v {
                x: cx + rng.sym(ex * 0.99),
                y: cy + rng.sym(ey * 0.99),
            },
            r: 0.1 + rng.unit() * 2.0,
        };
        let m = acc_ret(&mut acc, format!("#{i}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
        assert_eq!(m.count, 1, "centre inside must always produce a contact");
        if m.n.x != 0.0 {
            xwins += 1;
        } else {
            ywins += 1;
        }
    }
    // exactly on the box centre (x_overlap == y_overlap possible)
    for k in 0..256 {
        let e = 0.5 + k as f32 * 0.125;
        let B = c2AABB {
            min: c2v { x: -e, y: -e },
            max: c2v { x: e, y: e },
        };
        let A = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        acc.check(format!("centre k={k}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
    }
    acc.finish();
    eprintln!("err_circle_aabb_center_inside: x-axis={xwins} y-axis={ywins}");
    assert!(xwins > 0 && ywins > 0);
}

#[test]
fn err_circle_aabb_inverted_box() {
    let mut acc = DiffAccum::new("err_circle_aabb_inverted_box");
    let mut rng = Rng::new(0xceed_0017);
    for i in 0..(N * 4) {
        let a = rng.vec();
        let b = rng.vec();
        let B = c2AABB {
            min: c2v {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
            max: c2v {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
        };
        let A = rng.circle();
        acc.check(format!("inverted #{i} {A:?} {B:?}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B, m))
        });
        // half-inverted
        let B2 = c2AABB {
            min: c2v { x: a.x, y: b.y },
            max: c2v { x: b.x, y: a.y },
        };
        acc.check(format!("half #{i}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B2, m))
        });
        // fully degenerate
        let B3 = c2AABB { min: a, max: a };
        acc.check(format!("degen #{i}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, A, B3, m))
        });
    }
    acc.finish();
}

// =========================================================================
// rows 31, 32 — c2CircletoCapsuleManifold
// =========================================================================
#[test]
fn err_circle_capsule_reject() {
    let mut acc = DiffAccum::new("err_circle_capsule_reject");
    let mut rng = Rng::new(0xceed_0018);
    for i in 0..(N * 4) {
        let rB = rng.unit();
        let B = c2Capsule {
            a: c2v { x: -1.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: rB,
        };
        let rA = rng.unit();
        let A = c2Circle {
            p: c2v {
                x: rng.sym(2.0),
                y: rA + rB + 0.01 + rng.unit() * 4.0,
            },
            r: rA,
        };
        let m = acc_ret(&mut acc, format!("#{i}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A, B, m))
        });
        if m.count == 0 {
            assert!(m.n.bit_eq(&SENTINEL_MANIFOLD.n));
        }
    }
    // exact touch (d == rA + rB ⇒ `<` false ⇒ rejected)
    for k in 0..256 {
        let rA = 0.25 + k as f32 * 0.0625;
        let rB = 0.5;
        let B = c2Capsule {
            a: c2v { x: -1.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: rB,
        };
        let A = c2Circle {
            p: c2v {
                x: 0.0,
                y: rA + rB,
            },
            r: rA,
        };
        acc.check(format!("touch k={k}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A, B, m))
        });
    }
    acc.finish();
}

#[test]
fn err_circle_capsule_degenerate_axis() {
    let mut acc = DiffAccum::new("err_circle_capsule_degenerate_axis");
    let mut rng = Rng::new(0xceed_0019);
    for i in 0..(N * 4) {
        let p = rng.vec();
        let B = c2Capsule {
            a: p,
            b: p,
            r: 0.5 + rng.unit(),
        };
        // circle centre exactly on the (degenerate) spine ⇒ d == 0 ⇒
        // c2Norm(c2Skew(0,0)) ⇒ (NaN, NaN)
        let A = c2Circle {
            p,
            r: 0.5 + rng.unit(),
        };
        let m = acc_ret(&mut acc, format!("coincident #{i}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A, B, m))
        });
        assert_eq!(m.count, 1);
        assert!(
            m.n.x.is_nan() && m.n.y.is_nan(),
            "expected NaN normal, got {}",
            m.n.show()
        );
        // and slightly off the spine
        let A2 = c2Circle {
            p: c2v {
                x: p.x + rng.sym(0.5),
                y: p.y + rng.sym(0.5),
            },
            r: 0.5 + rng.unit(),
        };
        acc.check(format!("offspine #{i}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, A2, B, m))
        });
    }
    acc.finish();
}

// =========================================================================
// rows 33, 34 — c2CapsuletoCapsuleManifold
// =========================================================================
#[test]
fn err_capsule_capsule_reject() {
    let mut acc = DiffAccum::new("err_capsule_capsule_reject");
    let mut rng = Rng::new(0xceed_001a);
    for i in 0..(N * 4) {
        let rA = rng.unit();
        let rB = rng.unit();
        let A = c2Capsule {
            a: c2v { x: -1.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: rA,
        };
        let dy = rA + rB + 0.01 + rng.unit() * 4.0;
        let B = c2Capsule {
            a: c2v { x: -1.0, y: dy },
            b: c2v { x: 1.0, y: dy },
            r: rB,
        };
        let m = acc_ret(&mut acc, format!("#{i}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, B, m))
        });
        assert_eq!(m.count, 0);
        assert!(m.n.bit_eq(&SENTINEL_MANIFOLD.n));
    }
    // exact touch
    for k in 0..256 {
        let r = 0.25 + k as f32 * 0.0625;
        let A = c2Capsule {
            a: c2v { x: -1.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r,
        };
        let B = c2Capsule {
            a: c2v { x: -1.0, y: 2.0 * r },
            b: c2v { x: 1.0, y: 2.0 * r },
            r,
        };
        acc.check(format!("touch k={k}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, B, m))
        });
    }
    acc.finish();
}

#[test]
fn err_capsule_capsule_degenerate_axis() {
    let mut acc = DiffAccum::new("err_capsule_capsule_degenerate_axis");
    let mut rng = Rng::new(0xceed_001b);
    for i in 0..(N * 4) {
        let p = rng.vec();
        let A = c2Capsule {
            a: p,
            b: p,
            r: 0.5 + rng.unit(),
        };
        let B = c2Capsule {
            a: p,
            b: p,
            r: 0.5 + rng.unit(),
        };
        let m = acc_ret(&mut acc, format!("both #{i}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, B, m))
        });
        assert_eq!(m.count, 1);
        assert!(m.n.x.is_nan() && m.n.y.is_nan(), "got {}", m.n.show());
        // only A degenerate, overlapping ⇒ d may still be 0
        let B2 = c2Capsule {
            a: c2v { x: p.x - 1.0, y: p.y },
            b: c2v { x: p.x + 1.0, y: p.y },
            r: 0.5 + rng.unit(),
        };
        acc.check(format!("A-degen #{i}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, A, B2, m))
        });
        acc.check(format!("B-degen #{i}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, B2, A, m))
        });
    }
    acc.finish();
}

// =========================================================================
// rows 35..44, 46, 48 — c2CapsuletoPolyManifold rejections
// =========================================================================
#[test]
fn err_capsule_poly_reject() {
    let mut acc = DiffAccum::new("err_capsule_poly_reject");
    let mut rng = Rng::new(0xceed_001c);
    for count in 3..=8i32 {
        for i in 0..N {
            let verts = rng.convex_poly_verts(count as usize);
            let poly = make_poly(&verts, count);
            let mut cx = 0.0f32;
            let mut cy = 0.0f32;
            for k in 0..count as usize {
                cx += verts[k].x;
                cy += verts[k].y;
            }
            cx /= count as f32;
            cy /= count as f32;
            // far away and with a small radius ⇒ d >= 1e-6 and d >= A.r
            let A = c2Capsule {
                a: c2v {
                    x: cx + 100.0,
                    y: cy + 100.0,
                },
                b: c2v {
                    x: cx + 102.0,
                    y: cy + 100.0,
                },
                r: rng.unit(),
            };
            let m = acc_ret(&mut acc, format!("count={count} #{i}"), |s| {
                with_sentinel(|m| {
                    c2CapsuletoPolyManifold(s, A, &poly, std::ptr::null(), m)
                })
            });
            assert_eq!(m.count, 0, "expected rejection");
            assert!(
                m.n.bit_eq(&SENTINEL_MANIFOLD.n),
                "C touched n on the reject path"
            );
        }
    }
    acc.finish();
}

#[test]
fn err_capsule_poly_sideplanes_reject() {
    let mut acc = DiffAccum::new("err_capsule_poly_sideplanes_reject");
    let mut rng = Rng::new(0xceed_001d);
    // Deep overlaps with wildly varying capsule lengths/orientations drive
    // c2Clip to every sp ∈ {0,1,2}, hence c2SidePlanes to both 0 and 1, for all
    // three `code` branches.
    let mut rejects = 0usize;
    for count in 3..=8i32 {
        for i in 0..(N * 4) {
            let verts = rng.convex_poly_verts(count as usize);
            let poly = make_poly(&verts, count);
            let mut cx = 0.0f32;
            let mut cy = 0.0f32;
            for k in 0..count as usize {
                cx += verts[k].x;
                cy += verts[k].y;
            }
            cx /= count as f32;
            cy /= count as f32;
            let ang = rng.unit() * std::f32::consts::TAU;
            // very short capsules deep inside ⇒ the side planes clip everything
            let half = 0.001 + rng.unit() * 0.05;
            let A = c2Capsule {
                a: c2v {
                    x: cx - half * ang.cos(),
                    y: cy - half * ang.sin(),
                },
                b: c2v {
                    x: cx + half * ang.cos(),
                    y: cy + half * ang.sin(),
                },
                r: 0.05 + rng.unit() * 0.5,
            };
            let bx = if rng.bool() { Some(rng.xform()) } else { None };
            let m = acc_ret(
                &mut acc,
                format!("count={count} #{i} {A:?} bx={bx:?}"),
                |s| {
                    let bxp = match &bx {
                        Some(x) => x as *const c2x,
                        None => std::ptr::null(),
                    };
                    with_sentinel(|m| c2CapsuletoPolyManifold(s, A, &poly, bxp, m))
                },
            );
            if m.count == 0 && m.n.bit_eq(&SENTINEL_MANIFOLD.n) {
                rejects += 1;
            }
        }
    }
    acc.finish();
    eprintln!("err_capsule_poly_sideplanes_reject: {rejects} untouched-sentinel rejections");
    assert!(rejects > 0, "no c2SidePlanes rejection observed");
}

#[test]
fn err_capsule_poly_degenerate_axis() {
    let mut acc = DiffAccum::new("err_capsule_poly_degenerate_axis");
    let mut rng = Rng::new(0xceed_001e);
    // A.a == A.b ⇒ ab = c2Norm(0,0) = NaN ⇒ every separation is NaN ⇒
    // index stays ~0 == -1 and code stays 0 ⇒ the C reads verts[-1].
    for count in 1..=8i32 {
        for i in 0..(N * 2) {
            let verts = rng.convex_poly_verts(count.max(1) as usize);
            let poly = make_poly(&verts, count);
            let mut cx = 0.0f32;
            let mut cy = 0.0f32;
            for k in 0..count.max(1) as usize {
                cx += verts[k].x;
                cy += verts[k].y;
            }
            cx /= count.max(1) as f32;
            cy /= count.max(1) as f32;
            let p = c2v {
                x: cx + rng.sym(0.5),
                y: cy + rng.sym(0.5),
            };
            let A = c2Capsule {
                a: p,
                b: p,
                r: 0.25 + rng.unit(),
            };
            let bx = if rng.bool() { Some(rng.xform()) } else { None };
            acc.check(format!("count={count} #{i}"), |s| {
                let bxp = match &bx {
                    Some(x) => x as *const c2x,
                    None => std::ptr::null(),
                };
                with_sentinel(|m| c2CapsuletoPolyManifold(s, A, &poly, bxp, m))
            });
        }
    }
    // degenerate poly (all verts identical ⇒ NaN normals) with a real capsule
    for count in 1..=8i32 {
        for i in 0..N {
            let p = rng.vec();
            let verts = [p; 8];
            let poly = make_poly(&verts, count);
            let A = rng.capsule();
            acc.check(format!("degenpoly count={count} #{i}"), |s| {
                with_sentinel(|m| c2CapsuletoPolyManifold(s, A, &poly, std::ptr::null(), m))
            });
        }
    }
    acc.finish();
}

#[test]
fn err_capsule_poly_zero_count() {
    let mut acc = DiffAccum::new("err_capsule_poly_zero_count");
    let mut rng = Rng::new(0xceed_001f);
    for i in 0..(N * 4) {
        let verts = rng.convex_poly_verts(4);
        let mut poly = make_poly(&verts, 4);
        poly.count = 0;
        let A = rng.capsule();
        let bx = if rng.bool() { Some(rng.xform()) } else { None };
        acc.check(format!("#{i} {A:?}"), |s| {
            let bxp = match &bx {
                Some(x) => x as *const c2x,
                None => std::ptr::null(),
            };
            with_sentinel(|m| c2CapsuletoPolyManifold(s, A, &poly, bxp, m))
        });
    }
    acc.finish();
}

#[test]
fn err_capsule_poly_negative_count() {
    let mut acc = DiffAccum::new("err_capsule_poly_negative_count");
    let mut rng = Rng::new(0xceed_0020);
    for &count in &[-1i32, -2, -8, -1000] {
        for i in 0..N {
            let verts = rng.convex_poly_verts(4);
            let mut poly = make_poly(&verts, 4);
            poly.count = count;
            let A = rng.capsule();
            acc.check(format!("count={count} #{i}"), |s| {
                with_sentinel(|m| c2CapsuletoPolyManifold(s, A, &poly, std::ptr::null(), m))
            });
        }
    }
    acc.finish();
}

#[test]
fn err_capsule_poly_count_boundary() {
    let mut acc = DiffAccum::new("err_capsule_poly_count_boundary");
    let mut rng = Rng::new(0xceed_0021);
    // count == 8 is the maximum the `verts[8]` array allows; exercise the
    // `index + 1 == count ? 0` wrap for the *last* index specifically.
    for i in 0..(N * 4) {
        let verts = rng.convex_poly_verts(8);
        let poly = make_poly(&verts, 8);
        let A = rng.capsule();
        let bx = if rng.bool() { Some(rng.xform()) } else { None };
        acc.check(format!("count=8 #{i} {A:?}"), |s| {
            let bxp = match &bx {
                Some(x) => x as *const c2x,
                None => std::ptr::null(),
            };
            with_sentinel(|m| c2CapsuletoPolyManifold(s, A, &poly, bxp, m))
        });
    }
    // also drive c2SidePlanesFromPoly's wrap directly via the exported
    // c2PlaneAt / c2Support pair on a count-8 poly
    for i in 0..N {
        let verts = rng.convex_poly_verts(8);
        let poly = make_poly(&verts, 8);
        for idx in 0..8 {
            acc.check(format!("planeat #{i} idx={idx}"), |s| {
                c2PlaneAt(s, &poly, idx)
            });
        }
    }
    acc.finish();
}

// =========================================================================
// row 45 — c2Clip with both endpoints exactly on the plane
// =========================================================================
#[test]
fn err_clip_both_on_plane() {
    let mut acc = DiffAccum::new("err_clip_both_on_plane");
    // `c2Clip` is `static`, so drive it through `c2CapsuletoPolyManifold`:
    // an axis-aligned capsule lying exactly along a box edge makes both
    // clipped endpoints land exactly on a side plane (d0 == d1 == 0).
    for k in 0..512 {
        let e = 1.0 + k as f32 * 0.25;
        let verts = [
            c2v { x: -e, y: -e },
            c2v { x: e, y: -e },
            c2v { x: e, y: e },
            c2v { x: -e, y: e },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
        ];
        let poly = make_poly(&verts, 4);
        // capsule endpoints exactly at the box corners ⇒ both on both side planes
        for &(ax, ay, bx_, by) in &[
            (-e, -e, e, -e),
            (e, -e, e, e),
            (e, e, -e, e),
            (-e, e, -e, -e),
            (-e, 0.0, e, 0.0),
            (0.0, -e, 0.0, e),
        ] {
            let A = c2Capsule {
                a: c2v { x: ax, y: ay },
                b: c2v { x: bx_, y: by },
                r: 0.5,
            };
            acc.check(format!("k={k} ({ax},{ay})-({bx_},{by})"), |s| {
                with_sentinel(|m| c2CapsuletoPolyManifold(s, A, &poly, std::ptr::null(), m))
            });
        }
    }
    acc.finish();
}

// =========================================================================
// rows 57, 58, 59 — c2GJK cache abuse
// =========================================================================
#[test]
fn err_gjk_cache_negative_count() {
    let mut acc = DiffAccum::new("err_gjk_cache_negative_count");
    let mut rng = Rng::new(0xceed_0022);
    for &count in &[-1i32, -2, -100, i32::MIN] {
        for ka in KINDS {
            for kb in KINDS {
                for i in 0..64 {
                    let sa = rng.nice_shape(ka);
                    let sb = rng.nice_shape(kb);
                    let cache = c2GJKCache {
                        metric: rng.coord(),
                        count,
                        iA: [0, 1, 2],
                        iB: [0, 1, 2],
                        div: rng.coord(),
                    };
                    let args = GjkArgs {
                        cache: Some(cache),
                        use_radius: rng.below(2) as c_int,
                        ..Default::default()
                    };
                    acc.check(format!("count={count} ka={ka} kb={kb} #{i}"), |s| {
                        run_gjk(s, &sa, &sb, &args)
                    });
                }
            }
        }
    }
    acc.finish();
}

#[test]
fn err_gjk_cache_bad_indices() {
    let mut acc = DiffAccum::new("err_gjk_cache_bad_indices");
    let mut rng = Rng::new(0xceed_0023);
    // Indices in [-1, 7]: `pA.verts[i]` then still resolves inside (or exactly
    // one `c2v` before) the `c2Proxy` object, which both sides lay out
    // identically — so this is well-defined UB that must agree.  Indices at or
    // beyond 8 leave the object entirely and are therefore *unbounded* UB
    // (a different local in each build); they are documented in ERRORS.md
    // rather than asserted.
    for &idx in &[-1i32, 0, 1, 2, 3, 4, 5, 6, 7] {
        for ka in KINDS {
            for kb in KINDS {
                for count in 1..=3i32 {
                    for i in 0..24 {
                        let sa = rng.nice_shape(ka);
                        let sb = rng.nice_shape(kb);
                        let cache = c2GJKCache {
                            metric: [0.0f32, -1.0e9, 1.0][rng.below(3) as usize],
                            count,
                            iA: [idx, idx, idx],
                            iB: [idx, idx, idx],
                            div: 1.0,
                        };
                        let args = GjkArgs {
                            cache: Some(cache),
                            use_radius: rng.below(2) as c_int,
                            ..Default::default()
                        };
                        acc.check(
                            format!("idx={idx} ka={ka} kb={kb} cnt={count} #{i}"),
                            |s| run_gjk(s, &sa, &sb, &args),
                        );
                    }
                }
            }
        }
    }
    acc.finish();
}

#[test]
fn err_gjk_cache_count_overflow() {
    // `cache->count > 3` makes the C's warm-start loop write `verts[count-1]`
    // past the end of the 136-byte `c2Simplex` object, i.e. straight into
    // `c2GJK`'s other locals / spill slots.
    //
    // **The C itself crashes for `count == 4`.**  Verified with a standalone C
    // program linked against the very same `.so` (no Rust involved):
    //
    //     count=1 ... d=40400000 it=0 cache.count=1
    //     count=2 ... d=40400000 it=0 cache.count=1
    //     count=3 ... d=40400000 it=0 cache.count=1
    //     count=4 ... <killed by SIGSEGV, exit 135>
    //
    // So the ground truth for `count >= 4` is "the process dies"; there is no
    // byte pattern to match, and executing it would kill this test binary.
    // ERRORS.md row 57 records that.  What *is* well defined — and asserted
    // here — is the whole in-bounds range `count ∈ {1,2,3}`, i.e. the largest
    // value that still writes inside the object, over every type pair, metric
    // and `div`.
    let mut acc = DiffAccum::new("err_gjk_cache_count_overflow");
    let mut rng = Rng::new(0xceed_0024);
    for count in 1..=3i32 {
        for ka in KINDS {
            for kb in KINDS {
                for &metric in &[0.0f32, -1.0e9, 1.0e9, f32::NAN, -1.0] {
                    for i in 0..16 {
                        let sa = rng.nice_shape(ka);
                        let sb = rng.nice_shape(kb);
                        let cache = c2GJKCache {
                            metric,
                            count,
                            iA: [0, 0, 0],
                            iB: [0, 0, 0],
                            div: [1.0f32, 0.0, -1.0, 2.0][rng.below(4) as usize],
                        };
                        let args = GjkArgs {
                            cache: Some(cache),
                            use_radius: rng.below(2) as c_int,
                            ..Default::default()
                        };
                        acc.check(
                            format!("count={count} ka={ka} kb={kb} metric={metric:?} #{i}"),
                            |s| run_gjk(s, &sa, &sb, &args),
                        );
                    }
                }
            }
        }
    }
    acc.finish();
}

// =========================================================================
// rows 60, 61 — c2GJK with POLY / out-of-range types
// =========================================================================
#[test]
fn err_gjk_poly_type() {
    let mut acc = DiffAccum::new("err_gjk_poly_type");
    let mut rng = Rng::new(0xceed_0025);
    for count in 1..=8i32 {
        for ka in KINDS {
            for i in 0..64 {
                let verts = rng.convex_poly_verts(count as usize);
                let poly = make_poly(&verts, count);
                let sa = rng.nice_shape(ka);
                for &ur in &[0, 1] {
                    let args = GjkArgs {
                        use_radius: ur,
                        ..Default::default()
                    };
                    acc.check(format!("B-poly count={count} ka={ka} ur={ur} #{i}"), |s| {
                        run_gjk_raw(
                            s,
                            sa.as_ptr(),
                            sa.ty(),
                            &poly as *const c2Poly as *const c_void,
                            C2_TYPE_POLY,
                            &args,
                        )
                    });
                    acc.check(format!("A-poly count={count} kb={ka} ur={ur} #{i}"), |s| {
                        run_gjk_raw(
                            s,
                            &poly as *const c2Poly as *const c_void,
                            C2_TYPE_POLY,
                            sa.as_ptr(),
                            sa.ty(),
                            &args,
                        )
                    });
                }
            }
        }
    }
    acc.finish();
}

#[test]
fn err_gjk_bad_type() {
    let mut acc = DiffAccum::new("err_gjk_bad_type");
    let mut rng = Rng::new(0xceed_0026);
    for &t in &BAD_TYPES {
        for ka in KINDS {
            for i in 0..64 {
                let sa = rng.nice_shape(ka);
                let sb = rng.nice_shape(ka);
                for &ur in &[0, 1] {
                    let args = GjkArgs {
                        use_radius: ur,
                        ..Default::default()
                    };
                    // bad type for B
                    acc.check(format!("B t={t} ka={ka} ur={ur} #{i}"), |s| {
                        run_gjk_raw(s, sa.as_ptr(), sa.ty(), sb.as_ptr(), t, &args)
                    });
                    // bad type for A
                    acc.check(format!("A t={t} kb={ka} ur={ur} #{i}"), |s| {
                        run_gjk_raw(s, sa.as_ptr(), t, sb.as_ptr(), sb.ty(), &args)
                    });
                    // bad type for both
                    acc.check(format!("AB t={t} ur={ur} #{i}"), |s| {
                        run_gjk_raw(s, sa.as_ptr(), t, sb.as_ptr(), t, &args)
                    });
                }
            }
        }
    }
    acc.finish();
}

// =========================================================================
// rows 70..74 — c22 / c23 branch and NaN behaviour
// =========================================================================
#[test]
fn err_c22_branches() {
    let mut acc = DiffAccum::new("err_c22_branches");
    let mut rng = Rng::new(0xceed_0027);
    // v == 0 exactly (a at the origin)
    for i in 0..N {
        let mut s0 = simplex_with_count(&mut rng, 2);
        s0.a.p = c2v { x: 0.0, y: 0.0 };
        let out = acc_ret(&mut acc, format!("v=0 #{i}"), |side| {
            let mut s = s0;
            c22(side, &mut s);
            s
        });
        assert_eq!(out.count, 1);
        assert_eq!(out.div.to_bits(), 1.0f32.to_bits());
    }
    // u == 0 exactly (b at the origin, and a != 0 so v > 0)
    for i in 0..N {
        let mut s0 = simplex_with_count(&mut rng, 2);
        s0.b.p = c2v { x: 0.0, y: 0.0 };
        s0.a.p = c2v {
            x: 1.0 + rng.unit(),
            y: 1.0 + rng.unit(),
        };
        let out = acc_ret(&mut acc, format!("u=0 #{i}"), |side| {
            let mut s = s0;
            c22(side, &mut s);
            s
        });
        assert_eq!(out.count, 1);
    }
    // a == b ⇒ u == v == 0 ⇒ first branch
    for i in 0..N {
        let mut s0 = simplex_with_count(&mut rng, 2);
        s0.b.p = s0.a.p;
        acc.check(format!("a==b #{i}"), |side| {
            let mut s = s0;
            c22(side, &mut s);
            s
        });
    }
    acc.finish();
}

#[test]
fn err_c22_nan() {
    let mut acc = DiffAccum::new("err_c22_nan");
    let mut rng = Rng::new(0xceed_0028);
    for i in 0..(N * 4) {
        let mut s0 = simplex_with_count(&mut rng, 2);
        s0.a.p = rng.special_vec();
        s0.b.p = rng.special_vec();
        acc.check(format!("#{i} {:?} {:?}", s0.a.p, s0.b.p), |side| {
            let mut s = s0;
            c22(side, &mut s);
            s
        });
    }
    // guaranteed NaN u and v
    for i in 0..N {
        let mut s0 = simplex_with_count(&mut rng, 2);
        s0.a.p = c2v {
            x: f32::NAN,
            y: f32::NAN,
        };
        s0.b.p = c2v {
            x: f32::NAN,
            y: f32::NAN,
        };
        let out = acc_ret(&mut acc, format!("allnan #{i}"), |side| {
            let mut s = s0;
            c22(side, &mut s);
            s
        });
        assert_eq!(out.count, 2, "NaN must fall through to the interior branch");
        assert!(out.div.is_nan());
    }
    acc.finish();
}

#[test]
fn err_c23_branches() {
    let mut acc = DiffAccum::new("err_c23_branches");
    let mut rng = Rng::new(0xceed_0029);
    // all three vertices at the origin ⇒ every barycentric is 0
    for i in 0..N {
        let mut s0 = simplex_with_count(&mut rng, 3);
        let z = c2v { x: 0.0, y: 0.0 };
        s0.a.p = z;
        s0.b.p = z;
        s0.c.p = z;
        let out = acc_ret(&mut acc, format!("all-origin #{i}"), |side| {
            let mut s = s0;
            c23(side, &mut s);
            s
        });
        assert_eq!(out.count, 1);
    }
    // degenerate (collinear) triangles ⇒ area == 0 ⇒ uABC == vABC == wABC == 0
    for i in 0..(N * 2) {
        let mut s0 = simplex_with_count(&mut rng, 3);
        let d = c2v {
            x: 1.0 + rng.unit(),
            y: rng.sym(1.0),
        };
        let o = c2v {
            x: rng.sym(1.0),
            y: rng.sym(1.0),
        };
        let t = [rng.sym(2.0), rng.sym(2.0), rng.sym(2.0)];
        s0.a.p = c2v {
            x: o.x + d.x * t[0],
            y: o.y + d.y * t[0],
        };
        s0.b.p = c2v {
            x: o.x + d.x * t[1],
            y: o.y + d.y * t[1],
        };
        s0.c.p = c2v {
            x: o.x + d.x * t[2],
            y: o.y + d.y * t[2],
        };
        acc.check(format!("collinear #{i}"), |side| {
            let mut s = s0;
            c23(side, &mut s);
            s
        });
    }
    // duplicated vertices
    for slot in 0..3 {
        for i in 0..N {
            let mut s0 = simplex_with_count(&mut rng, 3);
            match slot {
                0 => s0.b.p = s0.a.p,
                1 => s0.c.p = s0.b.p,
                _ => s0.a.p = s0.c.p,
            }
            acc.check(format!("dup slot={slot} #{i}"), |side| {
                let mut s = s0;
                c23(side, &mut s);
                s
            });
        }
    }
    acc.finish();
}

#[test]
fn err_c23_nan() {
    let mut acc = DiffAccum::new("err_c23_nan");
    let mut rng = Rng::new(0xceed_002a);
    for i in 0..(N * 4) {
        let mut s0 = simplex_with_count(&mut rng, 3);
        s0.a.p = rng.special_vec();
        s0.b.p = rng.special_vec();
        s0.c.p = rng.special_vec();
        acc.check(format!("#{i}"), |side| {
            let mut s = s0;
            c23(side, &mut s);
            s
        });
    }
    for i in 0..N {
        let mut s0 = simplex_with_count(&mut rng, 3);
        let n = c2v {
            x: f32::NAN,
            y: f32::NAN,
        };
        s0.a.p = n;
        s0.b.p = n;
        s0.c.p = n;
        let out = acc_ret(&mut acc, format!("allnan #{i}"), |side| {
            let mut s = s0;
            c23(side, &mut s);
            s
        });
        assert_eq!(out.count, 3);
        assert!(out.div.is_nan());
    }
    acc.finish();
}

// =========================================================================
// rows 78..81 — c2Collide dispatch failures
// =========================================================================
#[test]
fn err_collide_unhandled_type() {
    let mut acc = DiffAccum::new("err_collide_unhandled_type");
    let mut rng = Rng::new(0xceed_002b);
    for &t in &BAD_TYPES {
        for ka in KINDS {
            for i in 0..64 {
                let sa = rng.nice_shape(ka);
                let sb = rng.nice_shape(ka);
                // bad typeA ⇒ the outer switch matches nothing
                let m = acc_ret(&mut acc, format!("A t={t} ka={ka} #{i}"), |s| {
                    with_sentinel(|m| c2Collide(s, sa.as_ptr(), t, sb.as_ptr(), sb.ty(), m))
                });
                assert_eq!(m.count, 0);
                assert!(
                    m.n.bit_eq(&SENTINEL_MANIFOLD.n),
                    "n must stay untouched for typeA={t}"
                );
                // bad typeB with a valid typeA ⇒ the inner switch matches nothing
                let m = acc_ret(&mut acc, format!("B t={t} ka={ka} #{i}"), |s| {
                    with_sentinel(|m| c2Collide(s, sa.as_ptr(), sa.ty(), sb.as_ptr(), t, m))
                });
                assert_eq!(m.count, 0);
                assert!(
                    m.n.bit_eq(&SENTINEL_MANIFOLD.n),
                    "n must stay untouched for typeB={t}"
                );
                // both bad
                acc.check(format!("AB t={t} #{i}"), |s| {
                    with_sentinel(|m| c2Collide(s, sa.as_ptr(), t, sb.as_ptr(), t, m))
                });
            }
        }
    }
    acc.finish();
}

#[test]
fn err_collide_negate_stale_n() {
    let mut acc = DiffAccum::new("err_collide_negate_stale_n");
    let mut rng = Rng::new(0xceed_002c);
    // For AABB/CIRCLE, CAPSULE/CIRCLE and CAPSULE/AABB the C negates m->n even
    // when the sub-manifold rejected and never wrote it, so the caller's stale
    // value comes back sign-flipped.  Verify with far-apart shapes.
    //
    // NOTE the double negation for CAPSULE/AABB: `c2AABBtoCapsuleManifold`
    // already flips `n` itself, and `c2Collide` flips it again, so for that pair
    // a rejected manifold comes back with the *original* sentinel.  Both
    // outcomes are asserted below, keyed by pair.
    let pairs: [(u32, u32); 3] = [(1, 0), (2, 0), (2, 1)];
    let mut rejects = [0usize; 3];
    for (pi, (ka, kb)) in pairs.into_iter().enumerate() {
        for i in 0..(N * 2) {
            // Both shapes are pushed far from the origin as well as from each
            // other: the CAPSULE/AABB path measures the capsule against the
            // (all-zero) poly proxy, so being far from the origin is what makes
            // that pair reject.
            let sa = rng.nice_shape(ka).translate(300.0, 300.0);
            let sb = rng.nice_shape(kb).translate(900.0, 900.0);
            let m = acc_ret(&mut acc, format!("ka={ka} kb={kb} #{i}"), |s| {
                with_sentinel(|m| c2Collide(s, sa.as_ptr(), sa.ty(), sb.as_ptr(), sb.ty(), m))
            });
            if m.count != 0 {
                continue;
            }
            rejects[pi] += 1;
            // one flip for AABB/CIRCLE and CAPSULE/CIRCLE, two for CAPSULE/AABB
            let flips = if (ka, kb) == (2, 1) { 2 } else { 1 };
            let mask = if flips % 2 == 0 { 0 } else { 0x8000_0000 };
            assert_eq!(
                m.n.x.to_bits(),
                SENTINEL_MANIFOLD.n.x.to_bits() ^ mask,
                "ka={ka} kb={kb}: stale n.x sign-flip mismatch ({} flips)",
                flips
            );
            assert_eq!(
                m.n.y.to_bits(),
                SENTINEL_MANIFOLD.n.y.to_bits() ^ mask,
                "ka={ka} kb={kb}: stale n.y sign-flip mismatch"
            );
        }
    }
    eprintln!("err_collide_negate_stale_n: rejections per pair = {rejects:?}");
    assert!(
        rejects.iter().all(|&n| n > 0),
        "no rejection observed for some pair: {rejects:?}"
    );
    // and with a sentinel n that is NaN / ±0 so the sign-bit semantics show
    for (ka, kb) in pairs {
        for &(nx, ny) in &[
            (0.0f32, -0.0f32),
            (-0.0, 0.0),
            (f32::NAN, -f32::NAN),
            (f32::INFINITY, f32::NEG_INFINITY),
        ] {
            let sa = rng.nice_shape(ka);
            let sb = rng.nice_shape(kb).translate(500.0, 500.0);
            acc.check(format!("ka={ka} kb={kb} n=({nx:?},{ny:?})"), |s| {
                let mut m = SENTINEL_MANIFOLD;
                m.n = c2v { x: nx, y: ny };
                c2Collide(s, sa.as_ptr(), sa.ty(), sb.as_ptr(), sb.ty(), &mut m);
                m
            });
        }
    }
    acc.finish();
}

// =========================================================================
// row 82 — ptr_from_parts with an unhandled type
// =========================================================================
#[test]
fn err_ptr_from_parts_unhandled() {
    // The C function has no `default:` and no trailing `return`, so it falls off
    // the end and its return value is indeterminate (gcc -O0 leaves whatever is
    // in %rax).  It must never be dereferenced, and `omni_manifold` never does
    // for those types — that is what is asserted here.  The pointer *value*
    // itself is not comparable.
    let mut rng = Rng::new(0xceed_002d);
    for &t in &BAD_TYPES {
        for _ in 0..64 {
            let p = rng.special();
            let c = ptr_from_parts(Side::C, t, p, p, p, p, p);
            let r = ptr_from_parts(Side::Rust, t, p, p, p, p, p);
            // both must simply return without crashing
            let _ = (c, r);
        }
    }
    // the observable consequence: identical manifolds through omni_manifold
    let mut acc = DiffAccum::new("err_ptr_from_parts_unhandled");
    for &ta in &BAD_TYPES {
        for &tb in &BAD_TYPES {
            for i in 0..8 {
                let p: [f32; 10] = std::array::from_fn(|_| rng.coord());
                acc.check(format!("ta={ta} tb={tb} #{i}"), |s| {
                    with_sentinel(|m| {
                        omni_manifold(
                            s, m, ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8],
                            p[9],
                        )
                    })
                });
            }
        }
    }
    acc.finish();
}

// =========================================================================
// rows 83..87 — omni_manifold
// =========================================================================
#[test]
fn err_omni_unhandled_type() {
    let mut acc = DiffAccum::new("err_omni_unhandled_type");
    let mut rng = Rng::new(0xceed_002e);
    let good = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
    for &t in &good {
        for i in 0..(N * 2) {
            let p: [f32; 10] = std::array::from_fn(|_| rng.coord());
            // POLY for A
            let m = acc_ret(&mut acc, format!("A-poly t={t} #{i}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s,
                        m,
                        C2_TYPE_POLY,
                        p[0],
                        p[1],
                        p[2],
                        p[3],
                        p[4],
                        t,
                        p[5],
                        p[6],
                        p[7],
                        p[8],
                        p[9],
                    )
                })
            });
            assert_eq!(m.count, 0);
            assert!(m.n.bit_eq(&SENTINEL_MANIFOLD.n));
            // POLY for B
            let m = acc_ret(&mut acc, format!("B-poly t={t} #{i}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s,
                        m,
                        t,
                        p[0],
                        p[1],
                        p[2],
                        p[3],
                        p[4],
                        C2_TYPE_POLY,
                        p[5],
                        p[6],
                        p[7],
                        p[8],
                        p[9],
                    )
                })
            });
            assert_eq!(m.count, 0);
            assert!(m.n.bit_eq(&SENTINEL_MANIFOLD.n));
        }
    }
    // POLY / POLY
    for i in 0..N {
        let p: [f32; 10] = std::array::from_fn(|_| rng.coord());
        acc.check(format!("poly/poly #{i}"), |s| {
            with_sentinel(|m| {
                omni_manifold(
                    s,
                    m,
                    C2_TYPE_POLY,
                    p[0],
                    p[1],
                    p[2],
                    p[3],
                    p[4],
                    C2_TYPE_POLY,
                    p[5],
                    p[6],
                    p[7],
                    p[8],
                    p[9],
                )
            })
        });
    }
    acc.finish();
}

#[test]
fn err_omni_out_of_range_enum() {
    let mut acc = DiffAccum::new("err_omni_out_of_range_enum");
    let mut rng = Rng::new(0xceed_002f);
    let bad: [c_int; 9] = [-1, 4, 5, 99, 255, 256, -12345, c_int::MIN, c_int::MAX];
    let good = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
    for &t in &bad {
        for &g in &good {
            for i in 0..64 {
                let p: [f32; 10] = std::array::from_fn(|_| rng.coord());
                let m = acc_ret(&mut acc, format!("A t={t} g={g} #{i}"), |s| {
                    with_sentinel(|m| {
                        omni_manifold(
                            s, m, t, p[0], p[1], p[2], p[3], p[4], g, p[5], p[6], p[7], p[8], p[9],
                        )
                    })
                });
                assert_eq!(m.count, 0);
                assert!(m.n.bit_eq(&SENTINEL_MANIFOLD.n));
                let m = acc_ret(&mut acc, format!("B t={t} g={g} #{i}"), |s| {
                    with_sentinel(|m| {
                        omni_manifold(
                            s, m, g, p[0], p[1], p[2], p[3], p[4], t, p[5], p[6], p[7], p[8], p[9],
                        )
                    })
                });
                assert_eq!(m.count, 0);
                assert!(m.n.bit_eq(&SENTINEL_MANIFOLD.n));
            }
        }
        for i in 0..64 {
            let p: [f32; 10] = std::array::from_fn(|_| rng.coord());
            acc.check(format!("both t={t} #{i}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s, m, t, p[0], p[1], p[2], p[3], p[4], t, p[5], p[6], p[7], p[8], p[9],
                    )
                })
            });
        }
    }
    acc.finish();
}

#[test]
fn err_omni_nonfinite_params() {
    let mut acc = DiffAccum::new("err_omni_nonfinite_params");
    let mut rng = Rng::new(0xceed_0030);
    let types = [
        C2_TYPE_CIRCLE,
        C2_TYPE_AABB,
        C2_TYPE_CAPSULE,
        C2_TYPE_POLY,
    ];
    // one non-finite slot at a time, the rest ordinary
    for &ta in &types {
        for &tb in &types {
            for slot in 0..10 {
                for &bad in &[
                    f32::NAN,
                    -f32::NAN,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    -0.0f32,
                    f32::MIN_POSITIVE,
                    f32::MAX,
                    -f32::MAX,
                ] {
                    let mut p: [f32; 10] = std::array::from_fn(|_| rng.coord());
                    p[slot] = bad;
                    acc.check(format!("ta={ta} tb={tb} slot={slot} bad={bad:?}"), |s| {
                        with_sentinel(|m| {
                            omni_manifold(
                                s, m, ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7],
                                p[8], p[9],
                            )
                        })
                    });
                }
            }
        }
    }
    // signalling NaNs (payload + sign must survive identically)
    for &ta in &types {
        for &tb in &types {
            for k in 0..64u32 {
                let snan = f32::from_bits(0x7f80_0000 | (k << 6) | 1);
                let p: [f32; 10] = std::array::from_fn(|i| {
                    if i % 3 == 0 {
                        snan
                    } else {
                        rng.coord()
                    }
                });
                acc.check(format!("snan ta={ta} tb={tb} k={k}"), |s| {
                    with_sentinel(|m| {
                        omni_manifold(
                            s, m, ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8],
                            p[9],
                        )
                    })
                });
            }
        }
    }
    acc.finish();
}

#[test]
fn err_omni_bad_radius() {
    let mut acc = DiffAccum::new("err_omni_bad_radius");
    let mut rng = Rng::new(0xceed_0031);
    let bad_r = [0.0f32, -0.0, -1.0, -1.0e-7, -1.0e7, f32::NAN, f32::INFINITY];
    for &r in &bad_r {
        for i in 0..(N / 2) {
            let (x1, y1) = (rng.coord(), rng.coord());
            let (x2, y2) = (rng.coord(), rng.coord());
            // circle/circle
            acc.check(format!("cc r={r:?} #{i}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s,
                        m,
                        C2_TYPE_CIRCLE,
                        x1,
                        y1,
                        r,
                        0.0,
                        0.0,
                        C2_TYPE_CIRCLE,
                        x2,
                        y2,
                        r,
                        0.0,
                        0.0,
                    )
                })
            });
            // capsule/capsule
            acc.check(format!("caca r={r:?} #{i}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s,
                        m,
                        C2_TYPE_CAPSULE,
                        x1,
                        y1,
                        x2,
                        y2,
                        r,
                        C2_TYPE_CAPSULE,
                        x2,
                        y2,
                        x1,
                        y1,
                        r,
                    )
                })
            });
            // circle/capsule and back
            acc.check(format!("cica r={r:?} #{i}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s,
                        m,
                        C2_TYPE_CIRCLE,
                        x1,
                        y1,
                        r,
                        0.0,
                        0.0,
                        C2_TYPE_CAPSULE,
                        x2,
                        y2,
                        x1,
                        y1,
                        r,
                    )
                })
            });
            // aabb/capsule (goes through the POLY proxy path)
            acc.check(format!("bbca r={r:?} #{i}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s,
                        m,
                        C2_TYPE_AABB,
                        x1,
                        y1,
                        x2,
                        y2,
                        0.0,
                        C2_TYPE_CAPSULE,
                        x2,
                        y2,
                        x1,
                        y1,
                        r,
                    )
                })
            });
        }
    }
    acc.finish();
}

#[test]
fn err_omni_inverted_aabb() {
    let mut acc = DiffAccum::new("err_omni_inverted_aabb");
    let mut rng = Rng::new(0xceed_0032);
    for i in 0..(N * 2) {
        let a = rng.coord();
        let b = rng.coord();
        let lo = a.min(b);
        let hi = a.max(b);
        // inverted: min > max
        for &(mn0, mn1, mx0, mx1) in &[
            (hi, hi, lo, lo),
            (hi, lo, lo, hi),
            (lo, hi, hi, lo),
            (lo, lo, lo, lo), // degenerate
        ] {
            acc.check(format!("bb/bb #{i} {mn0},{mn1},{mx0},{mx1}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s,
                        m,
                        C2_TYPE_AABB,
                        mn0,
                        mn1,
                        mx0,
                        mx1,
                        0.0,
                        C2_TYPE_AABB,
                        lo,
                        lo,
                        hi,
                        hi,
                        0.0,
                    )
                })
            });
            acc.check(format!("bb/ci #{i} {mn0},{mn1},{mx0},{mx1}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s,
                        m,
                        C2_TYPE_AABB,
                        mn0,
                        mn1,
                        mx0,
                        mx1,
                        0.0,
                        C2_TYPE_CIRCLE,
                        lo,
                        hi,
                        1.0,
                        0.0,
                        0.0,
                    )
                })
            });
            acc.check(format!("bb/ca #{i} {mn0},{mn1},{mx0},{mx1}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s,
                        m,
                        C2_TYPE_AABB,
                        mn0,
                        mn1,
                        mx0,
                        mx1,
                        0.0,
                        C2_TYPE_CAPSULE,
                        lo,
                        lo,
                        hi,
                        hi,
                        0.5,
                    )
                })
            });
            acc.check(format!("ca/bb #{i} {mn0},{mn1},{mx0},{mx1}"), |s| {
                with_sentinel(|m| {
                    omni_manifold(
                        s,
                        m,
                        C2_TYPE_CAPSULE,
                        lo,
                        lo,
                        hi,
                        hi,
                        0.5,
                        C2_TYPE_AABB,
                        mn0,
                        mn1,
                        mx0,
                        mx1,
                        0.0,
                    )
                })
            });
        }
    }
    acc.finish();
}

// =========================================================================
// rows 89..96 — leaf helpers on the boundary values
// =========================================================================
#[test]
fn err_minmax_nan() {
    let mut acc = DiffAccum::new("err_minmax_nan");
    let specials = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7f80_0001), // signalling NaN
    ];
    for &ax in &specials {
        for &ay in &specials {
            for &bx in &specials {
                for &by in &specials {
                    let u = c2v { x: ax, y: ay };
                    let v = c2v { x: bx, y: by };
                    acc.check(format!("max {ax:?},{ay:?} / {bx:?},{by:?}"), |s| {
                        c2Maxv(s, u, v)
                    });
                    acc.check(format!("min {ax:?},{ay:?} / {bx:?},{by:?}"), |s| {
                        c2Minv(s, u, v)
                    });
                    acc.check(format!("clamp {ax:?},{ay:?} / {bx:?},{by:?}"), |s| {
                        c2Clampv(s, u, v, u)
                    });
                    acc.check(format!("clamp2 {ax:?},{ay:?} / {bx:?},{by:?}"), |s| {
                        c2Clampv(s, u, u, v)
                    });
                }
            }
        }
    }
    acc.finish();
}

#[test]
fn err_absv_negative_zero() {
    let mut acc = DiffAccum::new("err_absv_negative_zero");
    // C's `(a.x) < 0 ? -(a.x) : (a.x)` does NOT normalise -0.0 (because
    // `-0.0 < 0` is false), so the sign bit survives.
    for &(x, y) in &[
        (-0.0f32, -0.0f32),
        (-0.0, 0.0),
        (0.0, -0.0),
        (0.0, 0.0),
    ] {
        let v = c2v { x, y };
        let out = diff(&format!("absv {x:?},{y:?}"), |s| c2Absv(s, v));
        assert_eq!(
            out.x.to_bits(),
            x.to_bits(),
            "C must preserve the sign of zero"
        );
        assert_eq!(out.y.to_bits(), y.to_bits());
        acc.check(format!("{x:?},{y:?}"), |s| c2Absv(s, v));
    }
    acc.finish();
}

#[test]
fn err_absv_nan() {
    let mut acc = DiffAccum::new("err_absv_nan");
    for k in 0..1024u32 {
        let bits = 0x7f80_0000 | (k << 6) | 1;
        for &sign in &[0u32, 0x8000_0000] {
            let x = f32::from_bits(bits | sign);
            let v = c2v { x, y: -x };
            let out = diff(&format!("absv nan {:#x}", bits | sign), |s| c2Absv(s, v));
            assert_eq!(
                out.x.to_bits(),
                x.to_bits(),
                "C must return the NaN unchanged"
            );
            acc.check(format!("k={k} sign={sign:#x}"), |s| c2Absv(s, v));
        }
    }
    acc.finish();
}

#[test]
fn err_unary_neg_signbit() {
    let mut acc = DiffAccum::new("err_unary_neg_signbit");
    let mut rng = Rng::new(0xceed_0033);
    // c2Neg / c2Skew / c2CCW90 all use unary `-`, i.e. a pure sign-bit flip
    // (gcc emits xorps), which must also apply to NaN and to ±0.
    for k in 0..2048u32 {
        let x = match k % 4 {
            0 => f32::from_bits(0x7f80_0000 | (k << 5) | 1), // sNaN family
            1 => f32::from_bits(0xff80_0000 | (k << 5) | 1),
            2 => [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY][(k / 4 % 4) as usize],
            _ => rng.any_f32(),
        };
        let y = rng.any_f32();
        let v = c2v { x, y };
        let n = diff(&format!("neg k={k}"), |s| c2Neg(s, v));
        assert_eq!(n.x.to_bits(), x.to_bits() ^ 0x8000_0000);
        assert_eq!(n.y.to_bits(), y.to_bits() ^ 0x8000_0000);
        let sk = diff(&format!("skew k={k}"), |s| c2Skew(s, v));
        assert_eq!(sk.x.to_bits(), y.to_bits() ^ 0x8000_0000);
        assert_eq!(sk.y.to_bits(), x.to_bits());
        let cw = diff(&format!("ccw90 k={k}"), |s| c2CCW90(s, v));
        assert_eq!(cw.x.to_bits(), y.to_bits());
        assert_eq!(cw.y.to_bits(), x.to_bits() ^ 0x8000_0000);
        acc.check(format!("k={k}"), |s| {
            (c2Neg(s, v), c2Skew(s, v), c2CCW90(s, v))
        });
    }
    acc.finish();
}

#[test]
fn err_dist_nonfinite() {
    let mut acc = DiffAccum::new("err_dist_nonfinite");
    let mut rng = Rng::new(0xceed_0034);
    for i in 0..(N * 8) {
        let h = c2h {
            n: rng.special_vec(),
            d: rng.special(),
        };
        let p = rng.special_vec();
        acc.check(format!("#{i} {h:?} {p:?}"), |s| c2Dist(s, h, p));
    }
    // inf - inf
    for &(nx, ny, d) in &[
        (f32::INFINITY, 0.0f32, f32::INFINITY),
        (f32::NEG_INFINITY, 0.0, f32::NEG_INFINITY),
        (f32::INFINITY, f32::NEG_INFINITY, 0.0),
        (0.0, 0.0, f32::NAN),
    ] {
        let h = c2h {
            n: c2v { x: nx, y: ny },
            d,
        };
        for i in 0..64 {
            let p = if i % 2 == 0 {
                c2v { x: 1.0, y: 1.0 }
            } else {
                c2v { x: 0.0, y: 0.0 }
            };
            acc.check(format!("infinf {nx:?},{ny:?},{d:?} #{i}"), |s| {
                c2Dist(s, h, p)
            });
        }
    }
    acc.finish();
}

#[test]
fn err_dot_nonfinite() {
    let mut acc = DiffAccum::new("err_dot_nonfinite");
    let mut rng = Rng::new(0xceed_0035);
    // 0 * inf and inf + (-inf) both produce the default QNaN
    let vals = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xffc0_1234),
        1.0,
        f32::MAX,
    ];
    for &ax in &vals {
        for &ay in &vals {
            for &bx in &vals {
                for &by in &vals {
                    let u = c2v { x: ax, y: ay };
                    let v = c2v { x: bx, y: by };
                    acc.check(format!("dot {ax:?},{ay:?}/{bx:?},{by:?}"), |s| {
                        c2Dot(s, u, v)
                    });
                }
            }
        }
    }
    for i in 0..(N * 4) {
        let (u, v) = (rng.any_vec(), rng.any_vec());
        acc.check(format!("rand #{i}"), |s| c2Dot(s, u, v));
    }
    acc.finish();
}

#[test]
fn err_det2_nonfinite() {
    let mut acc = DiffAccum::new("err_det2_nonfinite");
    let mut rng = Rng::new(0xceed_0036);
    let vals = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x7f80_0001),
        1.0,
        f32::MAX,
        -f32::MAX,
    ];
    for &ax in &vals {
        for &ay in &vals {
            for &bx in &vals {
                for &by in &vals {
                    let u = c2v { x: ax, y: ay };
                    let v = c2v { x: bx, y: by };
                    acc.check(format!("det2 {ax:?},{ay:?}/{bx:?},{by:?}"), |s| {
                        c2Det2(s, u, v)
                    });
                }
            }
        }
    }
    for i in 0..(N * 4) {
        let (u, v) = (rng.any_vec(), rng.any_vec());
        acc.check(format!("rand #{i}"), |s| c2Det2(s, u, v));
    }
    acc.finish();
}

#[test]
fn err_xform_nonfinite() {
    let mut acc = DiffAccum::new("err_xform_nonfinite");
    let mut rng = Rng::new(0xceed_0037);
    for i in 0..(N * 8) {
        let r = c2r {
            c: rng.special(),
            s: rng.special(),
        };
        let x = c2x {
            p: rng.special_vec(),
            r,
        };
        let v = rng.special_vec();
        acc.check(format!("#{i} {x:?} {v:?}"), |s| {
            (
                c2Mulrv(s, r, v),
                c2MulrvT(s, r, v),
                c2Mulxv(s, x, v),
                c2MulxvT(s, x, v),
            )
        });
    }
    for i in 0..(N * 4) {
        let r = c2r {
            c: rng.any_f32(),
            s: rng.any_f32(),
        };
        let x = c2x {
            p: rng.any_vec(),
            r,
        };
        let v = rng.any_vec();
        acc.check(format!("any #{i}"), |s| {
            (
                c2Mulrv(s, r, v),
                c2MulrvT(s, r, v),
                c2Mulxv(s, x, v),
                c2MulxvT(s, x, v),
            )
        });
    }
    acc.finish();
}

// =========================================================================
// row 97 — NULL pointer arguments (documented, deliberately not called)
// =========================================================================
#[test]
fn err_null_pointers_documented() {
    // `c2GJK(A = NULL, …)`, `c2Collide(m = NULL, …)`, `c2PlaneAt(NULL, …)` and
    // friends all dereference unconditionally in the C, so both libraries
    // SIGSEGV identically.  Invoking them would kill the test process, so the
    // behaviour is documented in ERRORS.md row 97 instead of executed.
    //
    // What *is* checkable is that every pointer the C explicitly guards with
    // `if (p)` behaves the same — covered by `err_gjk_null_outputs` and
    // `err_gjk_null_transforms` in `phase_b_gjk.rs`.
    let l = libs();
    assert!(l.c_path.exists() && l.r_path.exists());
}
