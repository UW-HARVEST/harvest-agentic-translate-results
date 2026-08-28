//! Phase C — NaN operand-position tests.
//!
//! x86 SSE scalar ops propagate NaN by *operand position*: `op dst, src` yields
//! `quiet(dst)` if `dst` is NaN, else `quiet(src)` if `src` is NaN.  gcc's
//! register allocation therefore fixes, per expression, *which* NaN comes out —
//! e.g. `c2Witness` compiles `den * s->a.u` to
//!
//! ```text
//!     movss  0x3c(%rax),%xmm0      ; xmm0 = s->a.u        <- dst
//!     mulss  -0x14(%rbp),%xmm0     ; xmm0 *= den          <- src
//! ```
//!
//! i.e. `u` wins, not `den`.  A translation that gets the position backwards is
//! invisible unless a test feeds **two different NaN payloads** into the two
//! operands at once — random NaNs mostly collide on the default QNaN
//! `0x7fc00000` and hide the difference.
//!
//! Every test below therefore sweeps a small pool of *pairwise distinct* NaNs
//! (quiet/signalling, both signs) across the full cross-product of a function's
//! float inputs, so each operand slot is exercised against every other.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::c_int;

/// Pairwise distinct NaN payloads plus one ordinary value, so that whichever
/// operand a `mulss`/`addss`/`subss`/`divss` picks is identifiable from the
/// result bits alone.
const POOL: [f32; 5] = [
    1.5,                            // ordinary
    f32::from_bits(0x7fc0_0001), // +qNaN, payload 1
    f32::from_bits(0xffc0_0002), // -qNaN, payload 2
    f32::from_bits(0x7f80_0003), // +sNaN, payload 3
    f32::from_bits(0xff80_0004), // -sNaN, payload 4
];

const P: usize = POOL.len();

/// Iterate over the cross-product of `POOL` for `n` slots.
fn each<F: FnMut(&[f32])>(n: usize, mut f: F) {
    let total = P.pow(n as u32);
    let mut slot = vec![0.0f32; n];
    for i in 0..total {
        let mut k = i;
        for s in slot.iter_mut() {
            *s = POOL[k % P];
            k /= P;
        }
        f(&slot);
    }
}

#[test]
fn nan_order_binary_vec_ops() {
    let mut acc = DiffAccum::new("nan_order_binary_vec_ops");
    each(4, |v| {
        let a = c2v { x: v[0], y: v[1] };
        let b = c2v { x: v[2], y: v[3] };
        let tag = format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
        acc.check(format!("add {tag}"), |s| c2Add(s, a, b));
        acc.check(format!("sub {tag}"), |s| c2Sub(s, a, b));
        acc.check(format!("dot {tag}"), |s| c2Dot(s, a, b));
        acc.check(format!("det2 {tag}"), |s| c2Det2(s, a, b));
        acc.check(format!("max {tag}"), |s| c2Maxv(s, a, b));
        acc.check(format!("min {tag}"), |s| c2Minv(s, a, b));
    });
    acc.finish();
}

#[test]
fn nan_order_unary_vec_ops() {
    let mut acc = DiffAccum::new("nan_order_unary_vec_ops");
    each(2, |v| {
        let a = c2v { x: v[0], y: v[1] };
        let tag = format!("{:#010x} {:#010x}", v[0].to_bits(), v[1].to_bits());
        acc.check(format!("neg {tag}"), |s| c2Neg(s, a));
        acc.check(format!("skew {tag}"), |s| c2Skew(s, a));
        acc.check(format!("ccw90 {tag}"), |s| c2CCW90(s, a));
        acc.check(format!("absv {tag}"), |s| c2Absv(s, a));
        acc.check(format!("len {tag}"), |s| c2Len(s, a));
        acc.check(format!("norm {tag}"), |s| c2Norm(s, a));
    });
    each(3, |v| {
        let a = c2v { x: v[0], y: v[1] };
        let tag = format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
        acc.check(format!("mulvs {tag}"), |s| c2Mulvs(s, a, v[2]));
        acc.check(format!("div {tag}"), |s| c2Div(s, a, v[2]));
    });
    acc.finish();
}

#[test]
fn nan_order_clampv() {
    let mut acc = DiffAccum::new("nan_order_clampv");
    each(6, |v| {
        let a = c2v { x: v[0], y: v[1] };
        let lo = c2v { x: v[2], y: v[3] };
        let hi = c2v { x: v[4], y: v[5] };
        acc.check(
            format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>()),
            |s| c2Clampv(s, a, lo, hi),
        );
    });
    acc.finish();
}

#[test]
fn nan_order_dist() {
    let mut acc = DiffAccum::new("nan_order_dist");
    each(5, |v| {
        let h = c2h {
            n: c2v { x: v[0], y: v[1] },
            d: v[2],
        };
        let p = c2v { x: v[3], y: v[4] };
        acc.check(
            format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>()),
            |s| c2Dist(s, h, p),
        );
    });
    acc.finish();
}

#[test]
fn nan_order_rotations() {
    let mut acc = DiffAccum::new("nan_order_rotations");
    each(4, |v| {
        let r = c2r { c: v[0], s: v[1] };
        let b = c2v { x: v[2], y: v[3] };
        let tag = format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
        acc.check(format!("mulrv {tag}"), |s| c2Mulrv(s, r, b));
        acc.check(format!("mulrvT {tag}"), |s| c2MulrvT(s, r, b));
    });
    each(6, |v| {
        let x = c2x {
            p: c2v { x: v[0], y: v[1] },
            r: c2r { c: v[2], s: v[3] },
        };
        let b = c2v { x: v[4], y: v[5] };
        let tag = format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
        acc.check(format!("mulxv {tag}"), |s| c2Mulxv(s, x, b));
        acc.check(format!("mulxvT {tag}"), |s| c2MulxvT(s, x, b));
    });
    acc.finish();
}

#[test]
fn nan_order_intersect() {
    let mut acc = DiffAccum::new("nan_order_intersect");
    each(6, |v| {
        let a = c2v { x: v[0], y: v[1] };
        let b = c2v { x: v[2], y: v[3] };
        acc.check(
            format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>()),
            |s| c2Intersect(s, a, b, v[4], v[5]),
        );
    });
    acc.finish();
}

/// `c2Witness` / `c2L`: `den = 1/div` and the barycentric weights `u` meet in a
/// `mulss`.  This is the case the operand-position mutation survived before this
/// test existed.
#[test]
fn nan_order_witness_and_l() {
    let mut acc = DiffAccum::new("nan_order_witness_and_l");
    let base = c2sv {
        sA: c2v { x: 2.0, y: 3.0 },
        sB: c2v { x: 4.0, y: 5.0 },
        p: c2v { x: 6.0, y: 7.0 },
        u: 1.0,
        iA: 0,
        iB: 0,
    };
    // div × a.u × b.u × c.u over the full pool, for every simplex count
    each(4, |v| {
        for count in 0..=4i32 {
            let mut sx = c2Simplex {
                a: base,
                b: base,
                c: base,
                d: base,
                div: v[0],
                count,
            };
            sx.a.u = v[1];
            sx.b.u = v[2];
            sx.c.u = v[3];
            let tag = format!(
                "count={count} {:#010x?}",
                v.iter().map(|f| f.to_bits()).collect::<Vec<_>>()
            );
            acc.check(format!("witness {tag}"), |side| {
                let mut s = sx;
                let mut a = c2v { x: 8.0, y: 9.0 };
                let mut b = c2v { x: -8.0, y: -9.0 };
                c2Witness(side, &mut s, &mut a, &mut b);
                (a, b, s)
            });
            acc.check(format!("c2L {tag}"), |side| {
                let mut s = sx;
                let r = c2L(side, &mut s);
                (r, s)
            });
        }
    });
    // and with NaN simplex points too, so `p` and `u` NaNs meet
    each(3, |v| {
        for count in 1..=3i32 {
            let mut sx = c2Simplex {
                a: base,
                b: base,
                c: base,
                d: base,
                div: POOL[1],
                count,
            };
            sx.a.p = c2v { x: v[0], y: v[1] };
            sx.b.p = c2v { x: v[1], y: v[2] };
            sx.c.p = c2v { x: v[2], y: v[0] };
            sx.a.u = v[2];
            sx.b.u = v[0];
            sx.c.u = v[1];
            let tag = format!(
                "pts count={count} {:#010x?}",
                v.iter().map(|f| f.to_bits()).collect::<Vec<_>>()
            );
            acc.check(format!("witness {tag}"), |side| {
                let mut s = sx;
                let mut a = c2v { x: 8.0, y: 9.0 };
                let mut b = c2v { x: -8.0, y: -9.0 };
                c2Witness(side, &mut s, &mut a, &mut b);
                (a, b, s)
            });
            acc.check(format!("c2L {tag}"), |side| {
                let mut s = sx;
                let r = c2L(side, &mut s);
                (r, s)
            });
            acc.check(format!("c2D {tag}"), |side| {
                let mut s = sx;
                let r = c2D(side, &mut s);
                (r, s)
            });
            acc.check(format!("metric {tag}"), |side| {
                let mut s = sx;
                let r = c2GJKSimplexMetric(side, &mut s);
                (r, s)
            });
        }
    });
    acc.finish();
}

/// `c22` / `c23`: the barycentric arithmetic mixes many NaN operands
/// (`u*+v*`, `det2 * area`, the three-way `div` sum).
#[test]
fn nan_order_simplex_reduction() {
    let mut acc = DiffAccum::new("nan_order_simplex_reduction");
    let base = c2sv {
        sA: c2v { x: 2.0, y: 3.0 },
        sB: c2v { x: 4.0, y: 5.0 },
        p: c2v { x: 6.0, y: 7.0 },
        u: 1.0,
        iA: 1,
        iB: 2,
    };
    // all 6 point coordinates of a 3-simplex over the pool
    each(6, |v| {
        let mut sx = c2Simplex {
            a: base,
            b: base,
            c: base,
            d: base,
            div: 1.0,
            count: 3,
        };
        sx.a.p = c2v { x: v[0], y: v[1] };
        sx.b.p = c2v { x: v[2], y: v[3] };
        sx.c.p = c2v { x: v[4], y: v[5] };
        let tag = format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
        acc.check(format!("c23 {tag}"), |side| {
            let mut s = sx;
            c23(side, &mut s);
            s
        });
        let mut s2 = sx;
        s2.count = 2;
        acc.check(format!("c22 {tag}"), |side| {
            let mut s = s2;
            c22(side, &mut s);
            s
        });
    });
    acc.finish();
}

/// `c2Support` / `c2PlaneAt` / `c2Norms` with distinct NaN payloads.
#[test]
fn nan_order_poly_helpers() {
    let mut acc = DiffAccum::new("nan_order_poly_helpers");
    each(4, |v| {
        let verts = [
            c2v { x: v[0], y: v[1] },
            c2v { x: v[2], y: v[3] },
            c2v { x: v[1], y: v[2] },
            c2v { x: v[3], y: v[0] },
            c2v { x: v[0], y: v[2] },
            c2v { x: v[1], y: v[3] },
            c2v { x: v[2], y: v[0] },
            c2v { x: v[3], y: v[1] },
        ];
        let tag = format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
        for count in [1i32, 4, 8] {
            let d = c2v { x: v[3], y: v[0] };
            acc.check(format!("support count={count} {tag}"), |s| {
                c2Support(s, verts.as_ptr(), count, d)
            });
            acc.check(format!("norms count={count} {tag}"), |s| {
                let mut vv = verts;
                let mut nn = [c2v { x: 0.5, y: -0.5 }; 8];
                c2Norms(s, vv.as_mut_ptr(), nn.as_mut_ptr(), count);
                (vv.to_vec(), nn.to_vec())
            });
        }
        let poly = c2Poly {
            count: 8,
            verts,
            norms: verts,
        };
        for i in 0..8 {
            acc.check(format!("planeat i={i} {tag}"), |s| c2PlaneAt(s, &poly, i));
        }
    });
    acc.finish();
}

/// The manifold producers, with distinct NaN payloads in every shape field.
#[test]
fn nan_order_manifolds() {
    let mut acc = DiffAccum::new("nan_order_manifolds");
    each(5, |v| {
        let tag = format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
        let ci = c2Circle {
            p: c2v { x: v[0], y: v[1] },
            r: v[2],
        };
        let ci2 = c2Circle {
            p: c2v { x: v[3], y: v[4] },
            r: v[0],
        };
        let bb = c2AABB {
            min: c2v { x: v[0], y: v[1] },
            max: c2v { x: v[2], y: v[3] },
        };
        let bb2 = c2AABB {
            min: c2v { x: v[4], y: v[0] },
            max: c2v { x: v[1], y: v[2] },
        };
        let ca = c2Capsule {
            a: c2v { x: v[0], y: v[1] },
            b: c2v { x: v[2], y: v[3] },
            r: v[4],
        };
        let ca2 = c2Capsule {
            a: c2v { x: v[4], y: v[3] },
            b: c2v { x: v[2], y: v[1] },
            r: v[0],
        };
        acc.check(format!("ci/ci {tag}"), |s| {
            with_sentinel(|m| c2CircletoCircleManifold(s, ci, ci2, m))
        });
        acc.check(format!("ci/bb {tag}"), |s| {
            with_sentinel(|m| c2CircletoAABBManifold(s, ci, bb, m))
        });
        acc.check(format!("ci/ca {tag}"), |s| {
            with_sentinel(|m| c2CircletoCapsuleManifold(s, ci, ca, m))
        });
        acc.check(format!("bb/bb {tag}"), |s| {
            with_sentinel(|m| c2AABBtoAABBManifold(s, bb, bb2, m))
        });
        acc.check(format!("ca/ca {tag}"), |s| {
            with_sentinel(|m| c2CapsuletoCapsuleManifold(s, ca, ca2, m))
        });
        acc.check(format!("bb/ca {tag}"), |s| {
            with_sentinel(|m| c2AABBtoCapsuleManifold(s, bb, ca, m))
        });
    });
    acc.finish();
}

/// The whole pipeline through the public entry point.
#[test]
fn nan_order_omni() {
    let mut acc = DiffAccum::new("nan_order_omni");
    let types = [
        C2_TYPE_CAPSULE,
        C2_TYPE_CIRCLE,
        C2_TYPE_AABB,
        C2_TYPE_POLY,
    ];
    each(4, |v| {
        let tag = format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
        let p = [
            v[0], v[1], v[2], v[3], v[0], v[1], v[2], v[3], v[0], v[1],
        ];
        for &ta in &types {
            for &tb in &types {
                acc.check(format!("ta={ta} tb={tb} {tag}"), |s| {
                    with_sentinel(|m| {
                        omni_manifold(
                            s, m, ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8],
                            p[9],
                        )
                    })
                });
            }
        }
    });
    acc.finish();
}

/// `c2GJK`'s own arithmetic (`dist -= rA + rB`, the midpoint average, the metric)
/// with NaN radii and NaN transforms.
#[test]
fn nan_order_gjk() {
    let mut acc = DiffAccum::new("nan_order_gjk");
    each(4, |v| {
        let tag = format!("{:#010x?}", v.iter().map(|f| f.to_bits()).collect::<Vec<_>>());
        let ci = c2Circle {
            p: c2v { x: v[0], y: v[1] },
            r: v[2],
        };
        let ca = c2Capsule {
            a: c2v { x: v[1], y: v[2] },
            b: c2v { x: v[3], y: v[0] },
            r: v[3],
        };
        let bb = c2AABB {
            min: c2v { x: v[0], y: v[2] },
            max: c2v { x: v[1], y: v[3] },
        };
        let shapes = [Shape::Ci(ci), Shape::Bb(bb), Shape::Ca(ca)];
        let xf = c2x {
            p: c2v { x: v[3], y: v[2] },
            r: c2r { c: v[1], s: v[0] },
        };
        for (i, sa) in shapes.iter().enumerate() {
            for (j, sb) in shapes.iter().enumerate() {
                for &ur in &[0, 1] {
                    for &with_x in &[false, true] {
                        let args = GjkArgs {
                            ax: if with_x { Some(xf) } else { None },
                            bx: if with_x { Some(xf) } else { None },
                            use_radius: ur,
                            cache: Some(c2GJKCache {
                                metric: v[0],
                                count: 1 + (i as c_int % 3),
                                iA: [0, 0, 0],
                                iB: [0, 0, 0],
                                div: v[1],
                            }),
                            ..Default::default()
                        };
                        acc.check(
                            format!("{i}/{j} ur={ur} x={with_x} {tag}"),
                            |s| run_gjk(s, sa, sb, &args),
                        );
                    }
                }
            }
        }
    });
    acc.finish();
}
