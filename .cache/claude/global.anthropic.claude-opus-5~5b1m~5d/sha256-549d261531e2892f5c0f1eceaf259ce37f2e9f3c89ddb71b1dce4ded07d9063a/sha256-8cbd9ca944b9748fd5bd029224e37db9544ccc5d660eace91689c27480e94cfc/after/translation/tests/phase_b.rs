//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares results **bit-for-bit** (raw
//! `u32` patterns, so `-0.0` vs `+0.0` and NaN payloads are not glossed over).
//! Inputs are property-style: many randomized values from a fixed-seed PRNG plus
//! the hand-picked boundary values named in the row.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

const N: usize = 4096;
const N_BIG: usize = 16384;

// ===========================================================================
// Rows 1-7: leaf vector math
// ===========================================================================

/// Row 1 — `c2V`, `c2Skew`, `c2CCW90`, `c2Absv`.
#[test]
fn cfg_01_leaf_unary() {
    let mut d = Diff::new("cfg_01_leaf_unary");
    let (v_c, v_r) = sym!("c2V", FnVff);
    let (sk_c, sk_r) = sym!("c2Skew", FnVv);
    let (cw_c, cw_r) = sym!("c2CCW90", FnVv);
    let (ab_c, ab_r) = sym!("c2Absv", FnVv);

    // Exhaustive over the special-value cross product.
    for &x in SPECIALS.iter() {
        for &y in SPECIALS.iter() {
            let v = c2v { x, y };
            d.eq(("c2V", fb(x), fb(y)), vb(v_c(x, y)), vb(v_r(x, y)));
            d.eq(("c2Skew", vb(v)), vb(sk_c(v)), vb(sk_r(v)));
            d.eq(("c2CCW90", vb(v)), vb(cw_c(v)), vb(cw_r(v)));
            d.eq(("c2Absv", vb(v)), vb(ab_c(v)), vb(ab_r(v)));
        }
    }
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..N {
        let v = rng.any_v();
        d.eq(("c2V", vb(v)), vb(v_c(v.x, v.y)), vb(v_r(v.x, v.y)));
        d.eq(("c2Skew", vb(v)), vb(sk_c(v)), vb(sk_r(v)));
        d.eq(("c2CCW90", vb(v)), vb(cw_c(v)), vb(cw_r(v)));
        d.eq(("c2Absv", vb(v)), vb(ab_c(v)), vb(ab_r(v)));
    }
    d.finish();
}

/// Row 2 — `c2Add`, `c2Sub`, `c2Mulvs`.
#[test]
fn cfg_02_leaf_binary() {
    let mut d = Diff::new("cfg_02_leaf_binary");
    let (add_c, add_r) = sym!("c2Add", FnVvv);
    let (sub_c, sub_r) = sym!("c2Sub", FnVvv);
    let (mul_c, mul_r) = sym!("c2Mulvs", FnVvf);

    // inf-inf, 0*inf, +-0 sign combinations, and every special pair.
    for &ax in SPECIALS.iter() {
        for &bx in SPECIALS.iter() {
            let a = c2v { x: ax, y: bx };
            let b = c2v { x: bx, y: ax };
            d.eq(("c2Add", vb(a), vb(b)), vb(add_c(a, b)), vb(add_r(a, b)));
            d.eq(("c2Sub", vb(a), vb(b)), vb(sub_c(a, b)), vb(sub_r(a, b)));
            d.eq(
                ("c2Mulvs", vb(a), fb(bx)),
                vb(mul_c(a, bx)),
                vb(mul_r(a, bx)),
            );
        }
    }
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        let a = rng.any_v();
        let b = rng.any_v();
        let s = rng.any_f32();
        d.eq(("c2Add", vb(a), vb(b)), vb(add_c(a, b)), vb(add_r(a, b)));
        d.eq(("c2Sub", vb(a), vb(b)), vb(sub_c(a, b)), vb(sub_r(a, b)));
        d.eq(("c2Mulvs", vb(a), fb(s)), vb(mul_c(a, s)), vb(mul_r(a, s)));
    }
    d.finish();
}

/// Row 3 — `c2Dot`, `c2Len` (incl. overflow-to-inf and underflow-to-zero).
#[test]
fn cfg_03_dot_len() {
    let mut d = Diff::new("cfg_03_dot_len");
    let (dot_c, dot_r) = sym!("c2Dot", FnFvv);
    let (len_c, len_r) = sym!("c2Len", FnFv);

    for &ax in SPECIALS.iter() {
        for &ay in SPECIALS.iter() {
            let a = c2v { x: ax, y: ay };
            for &bx in SPECIALS.iter() {
                let b = c2v { x: bx, y: ax };
                d.eq(("c2Dot", vb(a), vb(b)), fb(dot_c(a, b)), fb(dot_r(a, b)));
            }
            d.eq(("c2Len", vb(a)), fb(len_c(a)), fb(len_r(a)));
        }
    }
    // Deliberate overflow / underflow of the squared length.
    for &m in &[1e19f32, 1e20, 1e30, f32::MAX, 1e-20, 1e-25, f32::MIN_POSITIVE] {
        for v in [
            c2v { x: m, y: m },
            c2v { x: m, y: 0.0 },
            c2v { x: -m, y: m },
        ] {
            d.eq(("c2Len ovf", vb(v)), fb(len_c(v)), fb(len_r(v)));
            d.eq(("c2Dot ovf", vb(v)), fb(dot_c(v, v)), fb(dot_r(v, v)));
        }
    }
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        let a = rng.any_v();
        let b = rng.any_v();
        d.eq(("c2Dot", vb(a), vb(b)), fb(dot_c(a, b)), fb(dot_r(a, b)));
        d.eq(("c2Len", vb(a)), fb(len_c(a)), fb(len_r(a)));
    }
    d.finish();
}

/// Row 4 — `c2Div`, `c2Norm`. The C multiplies by `1.0f/b`, which is *not*
/// `a/b`; and `c2Norm` of the zero vector is `0 * inf == NaN`.
#[test]
fn cfg_04_div_norm() {
    let mut d = Diff::new("cfg_04_div_norm");
    let (div_c, div_r) = sym!("c2Div", FnVvf);
    let (nrm_c, nrm_r) = sym!("c2Norm", FnVv);

    for &ax in SPECIALS.iter() {
        for &b in SPECIALS.iter() {
            let a = c2v { x: ax, y: -ax };
            d.eq(("c2Div", vb(a), fb(b)), vb(div_c(a, b)), vb(div_r(a, b)));
        }
        for &ay in SPECIALS.iter() {
            let a = c2v { x: ax, y: ay };
            d.eq(("c2Norm", vb(a)), vb(nrm_c(a)), vb(nrm_r(a)));
        }
    }
    // The reciprocal-vs-division distinction shows up for divisors that are not
    // powers of two; sweep many of them.
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        let a = rng.any_v();
        let b = rng.any_f32();
        d.eq(("c2Div", vb(a), fb(b)), vb(div_c(a, b)), vb(div_r(a, b)));
        d.eq(("c2Norm", vb(a)), vb(nrm_c(a)), vb(nrm_r(a)));
        // exact-integer divisors 1..=64 (non-power-of-two included)
        let k = (1 + rng.below(64)) as f32;
        d.eq(("c2Div int", vb(a), fb(k)), vb(div_c(a, k)), vb(div_r(a, k)));
    }
    d.finish();
}

/// Row 5 — `c2Minv`, `c2Maxv`. C uses ternaries, so NaN and `-0.0` behave
/// differently from `fminf`/`fmaxf` and from Rust's `f32::min`/`max`.
#[test]
fn cfg_05_minv_maxv() {
    let mut d = Diff::new("cfg_05_minv_maxv");
    let (min_c, min_r) = sym!("c2Minv", FnVvv);
    let (max_c, max_r) = sym!("c2Maxv", FnVvv);

    // All 4 NaN placements x all +-0 combinations, exhaustively over specials.
    for &ax in SPECIALS.iter() {
        for &ay in SPECIALS.iter() {
            for &bx in SPECIALS.iter() {
                for &by in SPECIALS.iter() {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    d.eq(("c2Minv", vb(a), vb(b)), vb(min_c(a, b)), vb(min_r(a, b)));
                    d.eq(("c2Maxv", vb(a), vb(b)), vb(max_c(a, b)), vb(max_r(a, b)));
                }
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..N {
        let a = rng.any_v();
        let b = rng.any_v();
        d.eq(("c2Minv", vb(a), vb(b)), vb(min_c(a, b)), vb(min_r(a, b)));
        d.eq(("c2Maxv", vb(a), vb(b)), vb(max_c(a, b)), vb(max_r(a, b)));
    }
    d.finish();
}

/// Row 6 — `c2MulmvT`, `c2Mulrv`, `c2MulrvT`, `c2MulxvT`.
#[test]
fn cfg_06_matrix_ops() {
    let mut d = Diff::new("cfg_06_matrix_ops");
    let (mv_c, mv_r) = sym!("c2MulmvT", FnVmv);
    let (rv_c, rv_r) = sym!("c2Mulrv", FnVrv);
    let (rvt_c, rvt_r) = sym!("c2MulrvT", FnVrv);
    let (xvt_c, xvt_r) = sym!("c2MulxvT", FnVxv);

    // Canonical rotors: identity, 90/180/270 degrees, zero, non-unit, specials.
    let rotors: Vec<c2r> = {
        let mut v = vec![
            c2r { c: 1.0, s: 0.0 },
            c2r { c: 0.0, s: 1.0 },
            c2r { c: -1.0, s: 0.0 },
            c2r { c: 0.0, s: -1.0 },
            c2r { c: 0.0, s: 0.0 },
            c2r { c: 3.0, s: 4.0 },
            c2r { c: -0.0, s: -0.0 },
        ];
        for &a in SPECIALS.iter() {
            v.push(c2r { c: a, s: 1.0 });
            v.push(c2r { c: 1.0, s: a });
            v.push(c2r { c: a, s: a });
        }
        for i in 0..64 {
            v.push(rot_x(std::f32::consts::TAU * (i as f32) / 64.0));
        }
        v
    };
    for r in &rotors {
        for &vx in SPECIALS.iter() {
            for &vy in SPECIALS.iter() {
                let b = c2v { x: vx, y: vy };
                d.eq(("c2Mulrv", rb(*r), vb(b)), vb(rv_c(*r, b)), vb(rv_r(*r, b)));
                d.eq(
                    ("c2MulrvT", rb(*r), vb(b)),
                    vb(rvt_c(*r, b)),
                    vb(rvt_r(*r, b)),
                );
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..N {
        let m = c2m {
            x: rng.any_v(),
            y: rng.any_v(),
        };
        let b = rng.any_v();
        let r = rng.rot();
        let x = c2x {
            p: rng.any_v(),
            r,
        };
        d.eq(
            ("c2MulmvT", vb(m.x), vb(m.y), vb(b)),
            vb(mv_c(m, b)),
            vb(mv_r(m, b)),
        );
        d.eq(("c2Mulrv", rb(r), vb(b)), vb(rv_c(r, b)), vb(rv_r(r, b)));
        d.eq(("c2MulrvT", rb(r), vb(b)), vb(rvt_c(r, b)), vb(rvt_r(r, b)));
        d.eq(
            ("c2MulxvT", xb(x), vb(b)),
            vb(xvt_c(x, b)),
            vb(xvt_r(x, b)),
        );
    }
    d.finish();
}

/// Row 7 — nullary constructors `c2RotIdentity`, `c2xIdentity`.
#[test]
fn cfg_07_identities() {
    let mut d = Diff::new("cfg_07_identities");
    let (rot_c, rot_r) = sym!("c2RotIdentity", FnR);
    let (id_c, id_r) = sym!("c2xIdentity", FnX);
    for i in 0..8 {
        d.eq(("c2RotIdentity", i), rb(rot_c()), rb(rot_r()));
        d.eq(("c2xIdentity", i), xb(id_c()), xb(id_r()));
    }
    // And against the literal expected bit patterns.
    d.eq("rot bits", rb(rot_c()), [1.0f32.to_bits(), 0.0f32.to_bits()]);
    d.eq(
        "x bits",
        xb(id_c()),
        [
            0.0f32.to_bits(),
            0.0f32.to_bits(),
            1.0f32.to_bits(),
            0.0f32.to_bits(),
        ],
    );
    d.finish();
}

// ===========================================================================
// Rows 8-10: boolean predicates
// ===========================================================================

/// Row 8 — `c2AABBtoAABB`: overlap / touch / disjoint on each axis / nesting /
/// identity / inverted / NaN.
#[test]
fn cfg_08_aabb_aabb() {
    let mut d = Diff::new("cfg_08_aabb_aabb");
    let (f_c, f_r) = sym!("c2AABBtoAABB", FnIbb);
    let mk = |a: f32, b: f32, c: f32, e: f32| c2AABB {
        min: c2v { x: a, y: b },
        max: c2v { x: c, y: e },
    };
    let base = mk(-1.0, -1.0, 1.0, 1.0);
    let cases: Vec<(c2AABB, c2AABB)> = vec![
        (base, base),                            // identical
        (base, mk(-0.5, -0.5, 0.5, 0.5)),        // nested
        (mk(-0.5, -0.5, 0.5, 0.5), base),        // nested other way
        (base, mk(1.0, -1.0, 3.0, 1.0)),         // touching +x edge
        (base, mk(-3.0, -1.0, -1.0, 1.0)),       // touching -x edge
        (base, mk(-1.0, 1.0, 1.0, 3.0)),         // touching +y edge
        (base, mk(-1.0, -3.0, 1.0, -1.0)),       // touching -y edge
        (base, mk(1.0, 1.0, 3.0, 3.0)),          // touching corner
        (base, mk(1.001, -1.0, 3.0, 1.0)),       // disjoint +x
        (base, mk(-3.0, -1.0, -1.001, 1.0)),     // disjoint -x
        (base, mk(-1.0, 1.001, 1.0, 3.0)),       // disjoint +y
        (base, mk(-1.0, -3.0, 1.0, -1.001)),     // disjoint -y
        (mk(1.0, 1.0, -1.0, -1.0), base),        // inverted A
        (base, mk(1.0, 1.0, -1.0, -1.0)),        // inverted B
        (mk(0.0, 0.0, 0.0, 0.0), base),          // degenerate point box
        (base, mk(f32::NAN, 0.0, 1.0, 1.0)),     // NaN min.x
        (base, mk(0.0, 0.0, f32::NAN, 1.0)),     // NaN max.x
        (mk(f32::NAN, f32::NAN, f32::NAN, f32::NAN), base),
        (base, mk(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY)),
        (mk(-0.0, -0.0, 0.0, 0.0), mk(0.0, 0.0, -0.0, -0.0)),
    ];
    for (a, b) in cases {
        d.eq(("hand", xb_box(a), xb_box(b)), f_c(a, b), f_r(a, b));
    }
    let mut rng = Rng::new(SEED ^ 8);
    let mut hits = 0u32;
    for _ in 0..N_BIG {
        let a = rng.aabb();
        let b = rng.aabb();
        let cr = f_c(a, b);
        hits += (cr != 0) as u32;
        d.eq(("rand", xb_box(a), xb_box(b)), cr, f_r(a, b));
    }
    assert!(
        hits > 100 && hits < N_BIG as u32 - 100,
        "poor outcome coverage: {hits} overlaps out of {N_BIG}"
    );
    d.finish();
}

fn xb_box(b: c2AABB) -> [u32; 4] {
    [
        b.min.x.to_bits(),
        b.min.y.to_bits(),
        b.max.x.to_bits(),
        b.max.y.to_bits(),
    ]
}

/// Row 9 — `c2AABBtoPoint`: inside / on each edge / on each corner / outside
/// each side / inverted box / NaN.
#[test]
fn cfg_09_aabb_point() {
    let mut d = Diff::new("cfg_09_aabb_point");
    let (f_c, f_r) = sym!("c2AABBtoPoint", FnIbv);
    let base = c2AABB {
        min: c2v { x: -1.0, y: -2.0 },
        max: c2v { x: 3.0, y: 4.0 },
    };
    let mut pts: Vec<c2v> = vec![
        c2v { x: 0.0, y: 0.0 },   // inside
        c2v { x: -1.0, y: 0.0 },  // on min.x edge
        c2v { x: 3.0, y: 0.0 },   // on max.x edge
        c2v { x: 0.0, y: -2.0 },  // on min.y edge
        c2v { x: 0.0, y: 4.0 },   // on max.y edge
        c2v { x: -1.0, y: -2.0 }, // corner
        c2v { x: 3.0, y: -2.0 },
        c2v { x: -1.0, y: 4.0 },
        c2v { x: 3.0, y: 4.0 },
        c2v { x: -1.001, y: 0.0 }, // just outside each side
        c2v { x: 3.001, y: 0.0 },
        c2v { x: 0.0, y: -2.001 },
        c2v { x: 0.0, y: 4.001 },
    ];
    for &s in SPECIALS.iter() {
        pts.push(c2v { x: s, y: 0.0 });
        pts.push(c2v { x: 0.0, y: s });
        pts.push(c2v { x: s, y: s });
    }
    let inverted = c2AABB {
        min: base.max,
        max: base.min,
    };
    let point_box = c2AABB {
        min: c2v { x: 1.0, y: 1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let nan_box = c2AABB {
        min: c2v {
            x: f32::NAN,
            y: 0.0,
        },
        max: c2v {
            x: 1.0,
            y: f32::NAN,
        },
    };
    for b in [base, inverted, point_box, nan_box] {
        for &p in &pts {
            d.eq(("hand", xb_box(b), vb(p)), f_c(b, p), f_r(b, p));
        }
    }
    let mut rng = Rng::new(SEED ^ 9);
    let mut hits = 0u32;
    for _ in 0..N_BIG {
        let b = rng.aabb();
        let p = rng.v();
        let cr = f_c(b, p);
        hits += (cr != 0) as u32;
        d.eq(("rand", xb_box(b), vb(p)), cr, f_r(b, p));
    }
    assert!(hits > 50, "poor coverage: only {hits} inside-hits");
    d.finish();
}

/// Row 10 — `c2CircleToPoint`: inside / exactly on the boundary (strict `<`, so
/// a boundary point is a MISS) / outside / r == 0 / r < 0 / NaN.
#[test]
fn cfg_10_circle_point() {
    let mut d = Diff::new("cfg_10_circle_point");
    let (f_c, f_r) = sym!("c2CircleToPoint", FnIcv);
    let center = c2v { x: 1.0, y: -2.0 };
    for &r in &[0.0f32, -0.0, 1.0, -1.0, 3.0, -3.0, 1e-20, 1e20, f32::NAN, f32::INFINITY] {
        let a = c2Circle { p: center, r };
        let pts = [
            center,                                   // dead centre
            c2v { x: center.x + r, y: center.y },     // exactly on boundary
            c2v { x: center.x - r, y: center.y },
            c2v { x: center.x, y: center.y + r },
            c2v {
                x: center.x + r * 0.999,
                y: center.y,
            }, // just inside
            c2v {
                x: center.x + r * 1.001,
                y: center.y,
            }, // just outside
            c2v { x: 1e30, y: 1e30 },                 // far away
            c2v {
                x: f32::NAN,
                y: 0.0,
            },
            c2v {
                x: f32::INFINITY,
                y: 0.0,
            },
        ];
        for &p in &pts {
            d.eq(("hand", fb(r), vb(p)), f_c(a, p), f_r(a, p));
        }
    }
    // 3-4-5 triangle: the exact-boundary case with no rounding slop.
    let a5 = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 5.0,
    };
    for p in [
        c2v { x: 3.0, y: 4.0 },
        c2v { x: -3.0, y: -4.0 },
        c2v { x: 5.0, y: 0.0 },
        c2v { x: 0.0, y: 5.0 },
    ] {
        d.eq(("345", vb(p)), f_c(a5, p), f_r(a5, p));
    }
    let mut rng = Rng::new(SEED ^ 10);
    let mut hits = 0u32;
    for _ in 0..N_BIG {
        let a = rng.circle();
        let p = rng.v();
        let cr = f_c(a, p);
        hits += (cr != 0) as u32;
        d.eq(("rand", vb(a.p), fb(a.r), vb(p)), cr, f_r(a, p));
    }
    assert!(hits > 50, "poor coverage: only {hits} inside-hits");
    d.finish();
}

// ===========================================================================
// Rows 11-13: c2RaytoCircle
// ===========================================================================

/// Row 11 — `c2RaytoCircle` over many random rays x circles, plus targeted hits.
#[test]
fn cfg_11_ray_circle_random() {
    let mut d = Diff::new("cfg_11_ray_circle_random");
    let mut rng = Rng::new(SEED ^ 11);
    let (f_c, _) = sym!("c2RaytoCircle", FnRayCircle);
    let mut hits = 0u32;
    for i in 0..N_BIG {
        let a = rng.ray();
        let b = rng.circle();
        let mut probe = dirty();
        hits += (unsafe { f_c(a, b, &mut probe) } != 0) as u32;
        diff_ray_circle(&mut d, ("rand", i), a, b);
    }
    // Targeted geometric hits: aim straight at, tangentially to, and through
    // the centre of a circle from many angles.
    for i in 0..512 {
        let theta = std::f32::consts::TAU * (i as f32) / 512.0;
        let r = 1.0 + (i % 7) as f32;
        let dist = r + 1.0 + (i % 5) as f32;
        let c = c2Circle {
            p: c2v { x: 2.0, y: -1.0 },
            r,
        };
        let origin = c2v {
            x: c.p.x - dist * theta.cos(),
            y: c.p.y - dist * theta.sin(),
        };
        for offset in [0.0f32, r, -r, r * 0.5, r * 1.5] {
            // perpendicular offset -> through-centre, tangent, and miss
            let perp = c2v {
                x: -theta.sin(),
                y: theta.cos(),
            };
            let p = c2v {
                x: origin.x + perp.x * offset,
                y: origin.y + perp.y * offset,
            };
            let a = c2Ray {
                p,
                d: c2v {
                    x: theta.cos(),
                    y: theta.sin(),
                },
                t: 100.0,
            };
            let mut probe = dirty();
            hits += (unsafe { f_c(a, c, &mut probe) } != 0) as u32;
            diff_ray_circle(&mut d, ("aim", i, fb(offset)), a, c);
        }
    }
    assert!(hits > 200, "poor hit coverage: {hits}");
    d.finish();
}

/// Row 12 — `c2RaytoCircle` `A.t` boundary sweep around the exact hit distance.
#[test]
fn cfg_12_ray_circle_t_boundary() {
    let mut d = Diff::new("cfg_12_ray_circle_t_boundary");
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..512 {
        let r = 0.5 + rng.unit() * 4.0;
        let dist = r + 0.5 + rng.unit() * 6.0;
        let c = c2Circle {
            p: rng.v(),
            r,
        };
        let theta = rng.unit() * std::f32::consts::TAU;
        let dir = c2v {
            x: theta.cos(),
            y: theta.sin(),
        };
        let p = c2v {
            x: c.p.x - dir.x * dist,
            y: c.p.y - dir.y * dist,
        };
        let t_hit = dist - r; // approximately
        for t in [
            0.0f32,
            -0.0,
            t_hit,
            next_down(t_hit),
            next_up(t_hit),
            t_hit * 0.999,
            t_hit * 1.001,
            1e6,
            f32::INFINITY,
            f32::NAN,
            -1.0,
        ] {
            diff_ray_circle(&mut d, ("t", i, fb(t)), c2Ray { p, d: dir, t }, c);
        }
    }
    d.finish();
}

/// Row 13 — `c2RaytoCircle` with the ray ORIGIN INSIDE the circle: `c < 0` so
/// `disc > 0` but `t < 0`, exercising the `t >= 0` gate rather than `disc < 0`.
#[test]
fn cfg_13_ray_circle_inside() {
    let mut d = Diff::new("cfg_13_ray_circle_inside");
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..2048 {
        let r = 0.5 + rng.unit() * 6.0;
        let c = c2Circle { p: rng.v(), r };
        // origin strictly inside
        let rho = rng.unit() * r * 0.98;
        let phi = rng.unit() * std::f32::consts::TAU;
        let p = c2v {
            x: c.p.x + rho * phi.cos(),
            y: c.p.y + rho * phi.sin(),
        };
        let theta = rng.unit() * std::f32::consts::TAU;
        let dir = c2v {
            x: theta.cos(),
            y: theta.sin(),
        };
        for t in [0.0f32, 1.0, r, 1e3, f32::INFINITY] {
            diff_ray_circle(&mut d, ("inside", i, fb(t)), c2Ray { p, d: dir, t }, c);
        }
        // exactly at the centre
        diff_ray_circle(
            &mut d,
            ("centre", i),
            c2Ray {
                p: c.p,
                d: dir,
                t: 10.0,
            },
            c,
        );
    }
    d.finish();
}

fn next_up(x: f32) -> f32 {
    if x.is_nan() || x == f32::INFINITY {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    if x > 0.0 {
        f32::from_bits(x.to_bits() + 1)
    } else {
        f32::from_bits(x.to_bits() - 1)
    }
}

fn next_down(x: f32) -> f32 {
    if x.is_nan() || x == f32::NEG_INFINITY {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(0x8000_0001);
    }
    if x > 0.0 {
        f32::from_bits(x.to_bits() - 1)
    } else {
        f32::from_bits(x.to_bits() + 1)
    }
}

// ===========================================================================
// Rows 14-17: c2RaytoAABB
// ===========================================================================

/// Row 14 — axis-aligned rays entering each of the 4 faces, so each of the four
/// `t0>=..` / `t1>=..` / `t2>=..` / else branches (and each output normal) runs.
#[test]
fn cfg_14_ray_aabb_four_faces() {
    let mut d = Diff::new("cfg_14_ray_aabb_four_faces");
    let (f_c, _) = sym!("c2RaytoAABB", FnRayAABB);
    let mut rng = Rng::new(SEED ^ 14);
    let mut normals = std::collections::BTreeSet::new();
    for i in 0..1024 {
        let hw = 0.25 + rng.unit() * 5.0;
        let hh = 0.25 + rng.unit() * 5.0;
        let cx = rng.range(10.0);
        let cy = rng.range(10.0);
        let b = c2AABB {
            min: c2v {
                x: cx - hw,
                y: cy - hh,
            },
            max: c2v {
                x: cx + hw,
                y: cy + hh,
            },
        };
        let dist = hw.max(hh) + 1.0 + rng.unit() * 5.0;
        let dirs = [
            (c2v { x: 1.0, y: 0.0 }, c2v { x: cx - dist, y: cy }),
            (c2v { x: -1.0, y: 0.0 }, c2v { x: cx + dist, y: cy }),
            (c2v { x: 0.0, y: 1.0 }, c2v { x: cx, y: cy - dist }),
            (c2v { x: 0.0, y: -1.0 }, c2v { x: cx, y: cy + dist }),
        ];
        for (k, (dir, origin)) in dirs.iter().enumerate() {
            // jitter the perpendicular coordinate so the entry point moves
            let mut o = *origin;
            if dir.x != 0.0 {
                o.y = cy + rng.range(hh * 0.9);
            } else {
                o.x = cx + rng.range(hw * 0.9);
            }
            let a = c2Ray {
                p: o,
                d: *dir,
                t: dist * 2.0,
            };
            let mut probe = dirty();
            if unsafe { f_c(a, b, &mut probe) } != 0 {
                normals.insert((probe.n.x.to_bits(), probe.n.y.to_bits()));
            }
            diff_ray_aabb(&mut d, ("face", i, k), a, b);
        }
    }
    assert_eq!(
        normals.len(),
        4,
        "expected all 4 output normals to be produced, got {normals:?}"
    );
    d.finish();
}

/// Row 15 — oblique rays, corner hits, and exact ties in the `t0>=t1&&...`
/// comparison chain (equal values make the FIRST branch win).
#[test]
fn cfg_15_ray_aabb_oblique_ties() {
    let mut d = Diff::new("cfg_15_ray_aabb_oblique_ties");
    let mut rng = Rng::new(SEED ^ 15);
    // Symmetric box + diagonal rays through the corners => t values tie exactly.
    let sym = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    for &(dx, dy) in &[
        (1.0f32, 1.0f32),
        (-1.0, 1.0),
        (1.0, -1.0),
        (-1.0, -1.0),
        (1.0, 0.0),
        (0.0, 1.0),
    ] {
        for dist in [2.0f32, 4.0, 8.0, 1.0, 0.5] {
            let a = c2Ray {
                p: c2v {
                    x: -dx * dist,
                    y: -dy * dist,
                },
                d: c2v { x: dx, y: dy },
                t: dist * 3.0,
            };
            diff_ray_aabb(&mut d, ("tie", fb(dx), fb(dy), fb(dist)), a, sym);
            // aim exactly at each corner
            for &(cxs, cys) in &[(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                let target = c2v { x: cxs, y: cys };
                let o = c2v {
                    x: target.x - dx * dist,
                    y: target.y - dy * dist,
                };
                let a = c2Ray {
                    p: o,
                    d: c2v { x: dx, y: dy },
                    t: dist * 2.0,
                };
                diff_ray_aabb(&mut d, ("corner", fb(cxs), fb(cys), fb(dist)), a, sym);
            }
        }
    }
    // Random oblique rays aimed at a random point inside a random box.
    for i in 0..4096 {
        let b = rng.aabb();
        let inside = c2v {
            x: b.min.x + (b.max.x - b.min.x) * rng.unit(),
            y: b.min.y + (b.max.y - b.min.y) * rng.unit(),
        };
        let theta = rng.unit() * std::f32::consts::TAU;
        let dir = c2v {
            x: theta.cos(),
            y: theta.sin(),
        };
        let dist = 1.0 + rng.unit() * 20.0;
        let a = c2Ray {
            p: c2v {
                x: inside.x - dir.x * dist,
                y: inside.y - dir.y * dist,
            },
            d: dir,
            t: dist * (0.5 + rng.unit() * 2.0),
        };
        diff_ray_aabb(&mut d, ("aim", i), a, b);
    }
    d.finish();
}

/// Row 16 — degenerate shapes: origin inside the box, ray entirely inside,
/// zero-extent box, zero-direction ray.
#[test]
fn cfg_16_ray_aabb_degenerate() {
    let mut d = Diff::new("cfg_16_ray_aabb_degenerate");
    let mut rng = Rng::new(SEED ^ 16);
    let b = c2AABB {
        min: c2v { x: -2.0, y: -3.0 },
        max: c2v { x: 4.0, y: 5.0 },
    };
    let zero_box = c2AABB {
        min: c2v { x: 1.0, y: 1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let flat_x = c2AABB {
        min: c2v { x: 1.0, y: -2.0 },
        max: c2v { x: 1.0, y: 2.0 },
    };
    let flat_y = c2AABB {
        min: c2v { x: -2.0, y: 1.0 },
        max: c2v { x: 2.0, y: 1.0 },
    };
    let inverted = c2AABB {
        min: b.max,
        max: b.min,
    };
    for bb in [b, zero_box, flat_x, flat_y, inverted] {
        for i in 0..256 {
            // origin inside / ray fully inside
            let p = c2v {
                x: rng.range(4.0),
                y: rng.range(5.0),
            };
            let theta = rng.unit() * std::f32::consts::TAU;
            for (dir, t) in [
                (
                    c2v {
                        x: theta.cos(),
                        y: theta.sin(),
                    },
                    0.1f32,
                ),
                (
                    c2v {
                        x: theta.cos(),
                        y: theta.sin(),
                    },
                    100.0,
                ),
                (c2v { x: 0.0, y: 0.0 }, 5.0),   // zero direction
                (c2v { x: 0.0, y: 0.0 }, 0.0),   // zero direction, zero length
                (c2v { x: 1.0, y: 0.0 }, 0.0),   // zero length
                (c2v { x: 1.0, y: 0.0 }, -5.0),  // negative length
            ] {
                diff_ray_aabb(&mut d, ("deg", i, fb(t), vb(dir)), c2Ray { p, d: dir, t }, bb);
            }
        }
    }
    d.finish();
}

/// Row 17 — fully random rays x boxes, exercising the bb pre-reject, the SAT
/// `d > 0` reject, and both `hit` outcomes.
#[test]
fn cfg_17_ray_aabb_random() {
    let mut d = Diff::new("cfg_17_ray_aabb_random");
    let (f_c, _) = sym!("c2RaytoAABB", FnRayAABB);
    let mut rng = Rng::new(SEED ^ 17);
    let mut hits = 0u32;
    for i in 0..N_BIG {
        let a = rng.ray();
        let b = rng.aabb();
        let mut probe = dirty();
        hits += (unsafe { f_c(a, b, &mut probe) } != 0) as u32;
        diff_ray_aabb(&mut d, ("rand", i), a, b);
    }
    assert!(
        hits > 100 && hits < N_BIG as u32 - 100,
        "poor outcome coverage: {hits} hits of {N_BIG}"
    );
    d.finish();
}

// ===========================================================================
// Rows 18-28: c2RaytoCapsule
//
// `c2RaytoCapsule` has NINE distinct outcome paths. Rather than guess which
// inputs reach which, the branch is classified with an ORACLE built out of the
// C library's own exported helpers (`c2Norm`, `c2CCW90`, `c2MulmvT`,
// `c2AABBtoPoint`, `c2CircleToPoint`), so the classification cannot drift from
// the C's actual control flow. Each row then filters a large shared input pool
// for its branch and asserts the branch was really reached.
// ===========================================================================

/// Branch identifiers, matching the order of the `return`s in `c2RaytoCapsule`.
const CB_CORE: u8 = 0; // c2AABBtoPoint(capsule_bb, yAp)  -> return 1
const CB_CAP_A: u8 = 1; // c2CircleToPoint(capsule_a, A.p) -> return 1
const CB_CAP_B: u8 = 2; // c2CircleToPoint(capsule_b, A.p) -> return 1
const CB_NEAR_LO: u8 = 3; // |yAp.x| < r, yAp.y <  0 -> c2RaytoCircle(Ca)
const CB_NEAR_HI: u8 = 4; // |yAp.x| < r, yAp.y >= 0 -> c2RaytoCircle(Cb)
const CB_SIDE_LO: u8 = 5; // else, y <= 0       -> c2RaytoCircle(Ca)
const CB_SIDE_HI: u8 = 6; // else, y >= yBb.y   -> c2RaytoCircle(Cb)
const CB_SIDE_POS: u8 = 7; // else, side hit, c > 0 -> n = M.x
const CB_SIDE_NEG: u8 = 8; // else, side hit, c <= 0 -> n = c2Skew(M.y)
const CB_MISS: u8 = 9; // final `return 0`

fn capsule_branch(a: c2Ray, b: c2Capsule) -> u8 {
    let (norm, _) = sym!("c2Norm", FnVv);
    let (ccw, _) = sym!("c2CCW90", FnVv);
    let (sub, _) = sym!("c2Sub", FnVvv);
    let (add, _) = sym!("c2Add", FnVvv);
    let (mulvs, _) = sym!("c2Mulvs", FnVvf);
    let (mulmvt, _) = sym!("c2MulmvT", FnVmv);
    let (v, _) = sym!("c2V", FnVff);
    let (bpt, _) = sym!("c2AABBtoPoint", FnIbv);
    let (cpt, _) = sym!("c2CircleToPoint", FnIcv);

    let my = norm(sub(b.b, b.a));
    let m = c2m {
        x: ccw(my),
        y: my,
    };
    let cap_n = sub(b.b, b.a);
    let ybb = mulmvt(m, cap_n);
    let yap = mulmvt(m, sub(a.p, b.a));
    let yad = mulmvt(m, a.d);
    let yae = add(yap, mulvs(yad, a.t));
    let bb = c2AABB {
        min: v(-b.r, 0.0),
        max: v(b.r, ybb.y),
    };
    if bpt(bb, yap) != 0 {
        return CB_CORE;
    }
    if cpt(c2Circle { p: b.a, r: b.r }, a.p) != 0 {
        return CB_CAP_A;
    }
    if cpt(c2Circle { p: b.b, r: b.r }, a.p) != 0 {
        return CB_CAP_B;
    }
    let cabs = |x: f32| if x < 0.0 { -x } else { x };
    let cmin = |x: f32, y: f32| if x < y { x } else { y };
    if yae.x * yap.x < 0.0 || cmin(cabs(yae.x), cabs(yap.x)) < b.r {
        if cabs(yap.x) < b.r {
            return if yap.y < 0.0 { CB_NEAR_LO } else { CB_NEAR_HI };
        }
        let c = if yap.x > 0.0 { b.r } else { -b.r };
        let dd = yae.x - yap.x;
        let t = (c - yap.x) / dd;
        let y = yap.y + (yae.y - yap.y) * t;
        if y <= 0.0 {
            return CB_SIDE_LO;
        }
        if y >= ybb.y {
            return CB_SIDE_HI;
        }
        return if c > 0.0 { CB_SIDE_POS } else { CB_SIDE_NEG };
    }
    CB_MISS
}

/// A large mixed pool of (ray, capsule) pairs designed so that every one of the
/// nine branches is reached many times. Deterministic for a given seed.
fn capsule_pool(seed: u64, n: usize) -> Vec<(c2Ray, c2Capsule)> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n * 4);
    for _ in 0..n {
        // Canonical capsule, then a random rigid transform applied to BOTH the
        // capsule and the ray so local-space reasoning still holds.
        let len = 0.5 + rng.unit() * 8.0;
        let r = 0.25 + rng.unit() * 3.0;
        let theta = rng.unit() * std::f32::consts::TAU;
        let (ct, st) = (theta.cos(), theta.sin());
        let off = c2v {
            x: rng.range(8.0),
            y: rng.range(8.0),
        };
        let xf = |p: c2v| c2v {
            x: ct * p.x - st * p.y + off.x,
            y: st * p.x + ct * p.y + off.y,
        };
        let rot = |p: c2v| c2v {
            x: ct * p.x - st * p.y,
            y: st * p.x + ct * p.y,
        };
        let cap = c2Capsule {
            a: xf(c2v { x: 0.0, y: 0.0 }),
            b: xf(c2v { x: 0.0, y: len }),
            r,
        };
        // Local-space origins chosen to land in each region.
        let origins_local = [
            c2v { x: r * 0.4, y: len * 0.5 },   // inside core
            c2v { x: 0.0, y: -r * 0.5 },        // inside cap a
            c2v { x: 0.0, y: len + r * 0.5 },   // inside cap b
            c2v { x: r * 0.5, y: -3.0 * r - 2.0 }, // |x|<r, below
            c2v { x: -r * 0.5, y: len + 3.0 * r + 2.0 }, // |x|<r, above
            c2v { x: r + 1.0 + rng.unit() * 5.0, y: rng.range(len + 6.0) }, // right side
            c2v { x: -(r + 1.0 + rng.unit() * 5.0), y: rng.range(len + 6.0) }, // left side
            c2v { x: r + 0.001, y: len * 0.5 }, // exactly just outside the slab
            c2v { x: -r, y: len * 0.5 },        // exactly on the slab boundary
            c2v { x: rng.range(15.0), y: rng.range(15.0) }, // anywhere
        ];
        for ol in origins_local {
            // Directions: toward the axis, away, along, and random.
            let dirs_local = [
                c2v { x: -ol.x, y: 0.0 },
                c2v { x: -ol.x, y: len - ol.y },
                c2v { x: -ol.x, y: -ol.y },
                c2v { x: -ol.x, y: len * 0.5 - ol.y },
                c2v { x: ol.x, y: 0.0 },
                c2v { x: 0.0, y: 1.0 },
                c2v { x: 1.0, y: 0.0 },
                {
                    let a = rng.unit() * std::f32::consts::TAU;
                    c2v {
                        x: a.cos(),
                        y: a.sin(),
                    }
                },
            ];
            for dl in dirs_local {
                let l = (dl.x * dl.x + dl.y * dl.y).sqrt();
                let dn = if l > 0.0 {
                    c2v {
                        x: dl.x / l,
                        y: dl.y / l,
                    }
                } else {
                    dl
                };
                for t in [0.0f32, 0.5, 1.0, 5.0, 50.0] {
                    out.push((
                        c2Ray {
                            p: xf(ol),
                            d: rot(dn),
                            t,
                        },
                        cap,
                    ));
                }
            }
        }
    }
    out
}

fn run_capsule_branch(label: &'static str, branch: u8, seed: u64) {
    let mut d = Diff::new(label);
    let mut seen = 0u32;
    for (i, (a, b)) in capsule_pool(seed, 24).into_iter().enumerate() {
        if capsule_branch(a, b) != branch {
            continue;
        }
        seen += 1;
        diff_ray_capsule(&mut d, (label, i), a, b);
    }
    assert!(
        seen > 0,
        "[{label}] branch {branch} was never reached by the input pool"
    );
    eprintln!("[{label}] branch {branch} reached {seen} times");
    d.finish();
}

/// Row 18 — origin inside the capsule's rectangular core.
#[test]
fn cfg_18_capsule_inside_core() {
    run_capsule_branch("cfg_18_capsule_inside_core", CB_CORE, SEED ^ 18);
}

/// Row 19 — origin inside the `a` end-cap only.
#[test]
fn cfg_19_capsule_inside_cap_a() {
    run_capsule_branch("cfg_19_capsule_inside_cap_a", CB_CAP_A, SEED ^ 19);
}

/// Row 20 — origin inside the `b` end-cap only.
#[test]
fn cfg_20_capsule_inside_cap_b() {
    run_capsule_branch("cfg_20_capsule_inside_cap_b", CB_CAP_B, SEED ^ 20);
}

/// Row 21 — `|yAp.x| < r`, `yAp.y < 0` -> delegates to `c2RaytoCircle(A, Ca)`.
#[test]
fn cfg_21_capsule_delegate_ca() {
    run_capsule_branch("cfg_21_capsule_delegate_ca", CB_NEAR_LO, SEED ^ 21);
}

/// Row 22 — `|yAp.x| < r`, `yAp.y >= 0` -> delegates to `c2RaytoCircle(A, Cb)`.
#[test]
fn cfg_22_capsule_delegate_cb() {
    run_capsule_branch("cfg_22_capsule_delegate_cb", CB_NEAR_HI, SEED ^ 22);
}

/// Row 23 — side-plane crossing below the capsule (`y <= 0`).
#[test]
fn cfg_23_capsule_side_below() {
    run_capsule_branch("cfg_23_capsule_side_below", CB_SIDE_LO, SEED ^ 23);
}

/// Row 24 — side-plane crossing above the capsule (`y >= yBb.y`).
#[test]
fn cfg_24_capsule_side_above() {
    run_capsule_branch("cfg_24_capsule_side_above", CB_SIDE_HI, SEED ^ 24);
}

/// Row 25 — genuine side hit with `c > 0` -> `out->n = M.x`.
#[test]
fn cfg_25_capsule_side_hit_pos() {
    run_capsule_branch("cfg_25_capsule_side_hit_pos", CB_SIDE_POS, SEED ^ 25);
}

/// Row 26 — genuine side hit with `c <= 0` -> `out->n = c2Skew(M.y)`.
#[test]
fn cfg_26_capsule_side_hit_neg() {
    run_capsule_branch("cfg_26_capsule_side_hit_neg", CB_SIDE_NEG, SEED ^ 26);
}

/// Row 27 — orientation sweep: vertical, horizontal, 45 degrees, 64 random
/// angles, each with several radii and lengths. Also asserts that the pool as a
/// whole reaches ALL NINE branches.
#[test]
fn cfg_27_capsule_orientations() {
    let mut d = Diff::new("cfg_27_capsule_orientations");
    let mut hist = [0u32; 10];
    let mut rng = Rng::new(SEED ^ 27);
    let mut angles: Vec<f32> = vec![
        0.0,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        std::f32::consts::FRAC_PI_4,
        -std::f32::consts::FRAC_PI_4,
    ];
    for i in 0..64 {
        angles.push(std::f32::consts::TAU * (i as f32) / 64.0);
    }
    for (ai, &th) in angles.iter().enumerate() {
        for &len in &[0.25f32, 1.0, 5.0, 20.0] {
            for &r in &[0.1f32, 1.0, 4.0] {
                let cap = c2Capsule {
                    a: c2v { x: 1.0, y: -2.0 },
                    b: c2v {
                        x: 1.0 + len * th.cos(),
                        y: -2.0 + len * th.sin(),
                    },
                    r,
                };
                for k in 0..12 {
                    let a = rng.ray();
                    hist[capsule_branch(a, cap) as usize] += 1;
                    diff_ray_capsule(&mut d, ("orient", ai, k), a, cap);
                }
                // rays aimed straight at the capsule midpoint
                let mid = c2v {
                    x: (cap.a.x + cap.b.x) * 0.5,
                    y: (cap.a.y + cap.b.y) * 0.5,
                };
                for k in 0..12 {
                    let phi = std::f32::consts::TAU * (k as f32) / 12.0;
                    let dist = r + len + 1.0;
                    let p = c2v {
                        x: mid.x - dist * phi.cos(),
                        y: mid.y - dist * phi.sin(),
                    };
                    let a = c2Ray {
                        p,
                        d: c2v {
                            x: phi.cos(),
                            y: phi.sin(),
                        },
                        t: dist * 2.0,
                    };
                    hist[capsule_branch(a, cap) as usize] += 1;
                    diff_ray_capsule(&mut d, ("aim", ai, k), a, cap);
                }
            }
        }
    }
    eprintln!("[cfg_27] capsule branch histogram: {hist:?}");
    for b in 0..10 {
        assert!(hist[b] > 0, "capsule branch {b} never reached: {hist:?}");
    }
    d.finish();
}

/// Row 28 — fully random rays x capsules (incl. degenerate `a == b`, `r <= 0`).
#[test]
fn cfg_28_capsule_random() {
    let mut d = Diff::new("cfg_28_capsule_random");
    let mut rng = Rng::new(SEED ^ 28);
    for i in 0..N_BIG {
        let a = rng.ray();
        let b = rng.capsule();
        diff_ray_capsule(&mut d, ("rand", i), a, b);
    }
    d.finish();
}

// ===========================================================================
// Rows 29-44: c2RaytoPoly
//
// Driven DIRECTLY (not via `c2CastRay`/`poly_ray`), across the full `bx`
// transform state space and every `count` the fixed `verts[8]` array allows.
// ===========================================================================

/// Rays aimed at a polygon from `k` directions, at several distances and `A.t`s.
fn poly_probe_rays(rng: &mut Rng, center: c2v, span: f32, k: usize) -> Vec<c2Ray> {
    let mut out = Vec::new();
    for i in 0..k {
        let th = std::f32::consts::TAU * (i as f32) / (k as f32);
        let dir = c2v {
            x: th.cos(),
            y: th.sin(),
        };
        for &dist in &[span * 0.5, span + 1.0, span * 3.0] {
            // aim at the centre, and at an offset target so faces other than the
            // "front" one become the entering face
            for off in [0.0f32, span * 0.4, -span * 0.4] {
                let perp = c2v {
                    x: -dir.y,
                    y: dir.x,
                };
                let target = c2v {
                    x: center.x + perp.x * off,
                    y: center.y + perp.y * off,
                };
                let p = c2v {
                    x: target.x - dir.x * dist,
                    y: target.y - dir.y * dist,
                };
                for &t in &[0.0f32, dist * 0.5, dist, dist * 2.0, 1e5, f32::INFINITY] {
                    out.push(c2Ray { p, d: dir, t });
                }
            }
        }
    }
    for _ in 0..64 {
        out.push(rng.ray());
    }
    out
}

/// Row 29 — `bx = NULL`, 4-face box polygon, rays entering each face.
#[test]
fn cfg_29_poly_null_bx_box() {
    let mut d = Diff::new("cfg_29_poly_null_bx_box");
    let (f_c, _) = sym!("c2RaytoPoly", FnRayPoly);
    let mut rng = Rng::new(SEED ^ 29);
    let mut normals = std::collections::BTreeSet::new();
    for &(hw, hh) in &[(1.0f32, 1.0f32), (0.875, 11.5), (5.0, 0.5), (0.1, 0.1)] {
        let p = box_poly(hw, hh);
        for (i, a) in poly_probe_rays(&mut rng, c2v { x: 0.0, y: 0.0 }, hw.max(hh), 16)
            .into_iter()
            .enumerate()
        {
            let mut probe = dirty();
            if unsafe { f_c(a, &p, std::ptr::null(), &mut probe) } != 0 {
                normals.insert((probe.n.x.to_bits(), probe.n.y.to_bits()));
            }
            diff_ray_poly(&mut d, ("null_bx", fb(hw), i), a, &p, None);
        }
    }
    assert_eq!(
        normals.len(),
        4,
        "expected all 4 box faces to be reported as the entering face, got {normals:?}"
    );
    d.finish();
}

/// Row 30 — an EXPLICIT identity `bx` must be bit-identical to `bx = NULL`.
#[test]
fn cfg_30_poly_explicit_identity() {
    let mut d = Diff::new("cfg_30_poly_explicit_identity");
    let (f_c, f_r) = sym!("c2RaytoPoly", FnRayPoly);
    let mut rng = Rng::new(SEED ^ 30);
    let id = identity_x();
    for n in 1..=8usize {
        let p = ngon(n, 3.0, 0.3, c2v { x: 0.0, y: 0.0 });
        for (i, a) in poly_probe_rays(&mut rng, c2v { x: 0.0, y: 0.0 }, 3.0, 8)
            .into_iter()
            .enumerate()
        {
            // C(NULL) vs Rust(NULL), C(&id) vs Rust(&id), and NULL vs &id.
            diff_ray_poly(&mut d, ("id", n, i), a, &p, Some(&id));
            diff_ray_poly(&mut d, ("null", n, i), a, &p, None);
            let (mut o1, mut o2) = (dirty(), dirty());
            let r1 = unsafe { f_c(a, &p, std::ptr::null(), &mut o1) };
            let r2 = unsafe { f_c(a, &p, &id, &mut o2) };
            d.eq(("C null==id", n, i), (r1, cb(o1)), (r2, cb(o2)));
            let (mut o3, mut o4) = (dirty(), dirty());
            let r3 = unsafe { f_r(a, &p, std::ptr::null(), &mut o3) };
            let r4 = unsafe { f_r(a, &p, &id, &mut o4) };
            d.eq(("RS null==id", n, i), (r3, cb(o3)), (r4, cb(o4)));
        }
    }
    d.finish();
}

/// Row 31 — `bx` = pure translation.
#[test]
fn cfg_31_poly_translation() {
    let mut d = Diff::new("cfg_31_poly_translation");
    let mut rng = Rng::new(SEED ^ 31);
    for n in [1usize, 3, 4, 5, 8] {
        let p = ngon(n, 2.5, 0.0, c2v { x: 0.0, y: 0.0 });
        for k in 0..32 {
            let bx = c2x {
                p: c2v {
                    x: rng.range(15.0),
                    y: rng.range(15.0),
                },
                r: c2r { c: 1.0, s: 0.0 },
            };
            for (i, a) in poly_probe_rays(&mut rng, bx.p, 3.5, 6).into_iter().enumerate() {
                diff_ray_poly(&mut d, ("xlat", n, k, i), a, &p, Some(&bx));
            }
        }
    }
    d.finish();
}

/// Row 32 — `bx` = pure rotation, 64 angles.
#[test]
fn cfg_32_poly_rotation() {
    let mut d = Diff::new("cfg_32_poly_rotation");
    let mut rng = Rng::new(SEED ^ 32);
    for n in [1usize, 3, 4, 6, 8] {
        let p = ngon(n, 2.5, 0.0, c2v { x: 0.0, y: 0.0 });
        for k in 0..64 {
            let bx = c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: rot_x(std::f32::consts::TAU * (k as f32) / 64.0),
            };
            for (i, a) in poly_probe_rays(&mut rng, c2v { x: 0.0, y: 0.0 }, 3.5, 4)
                .into_iter()
                .enumerate()
            {
                diff_ray_poly(&mut d, ("rot", n, k, i), a, &p, Some(&bx));
            }
        }
    }
    d.finish();
}

/// Row 33 — `bx` = rotation AND translation together.
#[test]
fn cfg_33_poly_rot_translate() {
    let mut d = Diff::new("cfg_33_poly_rot_translate");
    let mut rng = Rng::new(SEED ^ 33);
    for n in [1usize, 2, 4, 7, 8] {
        let p = ngon(n, 2.0, 0.7, c2v { x: 0.5, y: -0.5 });
        for k in 0..64 {
            let bx = c2x {
                p: c2v {
                    x: rng.range(12.0),
                    y: rng.range(12.0),
                },
                r: rot_x(rng.unit() * std::f32::consts::TAU),
            };
            for (i, a) in poly_probe_rays(&mut rng, bx.p, 3.0, 4).into_iter().enumerate() {
                diff_ray_poly(&mut d, ("rt", n, k, i), a, &p, Some(&bx));
            }
        }
    }
    d.finish();
}

/// Row 34 — NON-UNIT `bx.r` (including the zero rotor and huge rotors). The C
/// never normalises, so the transform scales/collapses space.
#[test]
fn cfg_34_poly_nonunit_rotor() {
    let mut d = Diff::new("cfg_34_poly_nonunit_rotor");
    let mut rng = Rng::new(SEED ^ 34);
    let rotors = [
        c2r { c: 0.0, s: 0.0 },
        c2r { c: 2.0, s: 0.0 },
        c2r { c: 0.0, s: 3.0 },
        c2r { c: 3.0, s: 4.0 },
        c2r { c: -5.0, s: 12.0 },
        c2r { c: 1e-6, s: 1e-6 },
        c2r { c: 1e6, s: -1e6 },
        c2r { c: 0.5, s: 0.5 },
        c2r {
            c: f32::INFINITY,
            s: 0.0,
        },
        c2r {
            c: f32::NAN,
            s: 1.0,
        },
        c2r {
            c: 1.0,
            s: f32::NAN,
        },
        c2r {
            c: -0.0,
            s: -0.0,
        },
    ];
    for n in [1usize, 4, 8] {
        let p = ngon(n, 2.0, 0.0, c2v { x: 0.0, y: 0.0 });
        for (k, &r) in rotors.iter().enumerate() {
            for tp in [
                c2v { x: 0.0, y: 0.0 },
                c2v { x: 3.0, y: -4.0 },
                c2v {
                    x: f32::NAN,
                    y: 0.0,
                },
            ] {
                let bx = c2x { p: tp, r };
                for (i, a) in poly_probe_rays(&mut rng, c2v { x: 0.0, y: 0.0 }, 3.0, 4)
                    .into_iter()
                    .enumerate()
                {
                    diff_ray_poly(&mut d, ("nonunit", n, k, i), a, &p, Some(&bx));
                }
            }
        }
    }
    d.finish();
}

fn run_poly_count(label: &'static str, counts: &[usize], seed: u64) {
    let mut d = Diff::new(label);
    let mut rng = Rng::new(seed);
    let bxs = [
        None,
        Some(identity_x()),
        Some(c2x {
            p: c2v { x: 2.0, y: -3.0 },
            r: rot_x(0.9),
        }),
    ];
    for &n in counts {
        for phase in 0..8 {
            let p = ngon(
                n,
                1.0 + (phase as f32),
                std::f32::consts::TAU * (phase as f32) / 8.0,
                c2v {
                    x: rng.range(3.0),
                    y: rng.range(3.0),
                },
            );
            for (bi, bx) in bxs.iter().enumerate() {
                for (i, a) in poly_probe_rays(&mut rng, c2v { x: 0.0, y: 0.0 }, 6.0, 6)
                    .into_iter()
                    .enumerate()
                {
                    diff_ray_poly(&mut d, (label, n, phase, bi, i), a, &p, bx.as_ref());
                }
            }
            // Also the reversed-winding variant: flip every normal, which turns
            // entering faces into exiting ones and vice versa.
            let mut q = p;
            for j in 0..n {
                q.norms[j] = c2v {
                    x: -p.norms[j].x,
                    y: -p.norms[j].y,
                };
            }
            for (i, a) in poly_probe_rays(&mut rng, c2v { x: 0.0, y: 0.0 }, 6.0, 4)
                .into_iter()
                .enumerate()
            {
                diff_ray_poly(&mut d, (label, "flipped", n, phase, i), a, &q, None);
            }
        }
    }
    d.finish();
}

/// Row 35 — `count == 1` (a single half-plane).
#[test]
fn cfg_35_poly_count_1() {
    run_poly_count("cfg_35_poly_count_1", &[1], SEED ^ 35);
}

/// Row 36 — `count == 2` (a wedge / slab).
#[test]
fn cfg_36_poly_count_2() {
    run_poly_count("cfg_36_poly_count_2", &[2], SEED ^ 36);
}

/// Row 37 — `count == 3` (a triangle), both windings.
#[test]
fn cfg_37_poly_count_3() {
    run_poly_count("cfg_37_poly_count_3", &[3], SEED ^ 37);
}

/// Row 38 — `count == 5,6,7,8`, up to the fixed `verts[8]` capacity.
#[test]
fn cfg_38_poly_count_5_to_8() {
    run_poly_count("cfg_38_poly_count_5_to_8", &[5, 6, 7, 8], SEED ^ 38);
}

/// Row 39 — ray origin strictly INSIDE the polygon: no face has `den < 0` with
/// `num < lo*den`, so `index` stays `~0` and the C returns 0 without writing.
#[test]
fn cfg_39_poly_origin_inside() {
    let mut d = Diff::new("cfg_39_poly_origin_inside");
    let (f_c, _) = sym!("c2RaytoPoly", FnRayPoly);
    let mut rng = Rng::new(SEED ^ 39);
    let mut zero_rets = 0u32;
    for n in 3..=8usize {
        let p = ngon(n, 4.0, 0.11, c2v { x: 0.0, y: 0.0 });
        for i in 0..512 {
            // Strictly inside the inscribed circle of a radius-4 n-gon.
            let rho = rng.unit() * 4.0 * (std::f32::consts::PI / n as f32).cos() * 0.9;
            let phi = rng.unit() * std::f32::consts::TAU;
            let a = c2Ray {
                p: c2v {
                    x: rho * phi.cos(),
                    y: rho * phi.sin(),
                },
                d: {
                    let th = rng.unit() * std::f32::consts::TAU;
                    c2v {
                        x: th.cos(),
                        y: th.sin(),
                    }
                },
                t: rng.unit() * 20.0,
            };
            let mut probe = dirty();
            if unsafe { f_c(a, &p, std::ptr::null(), &mut probe) } == 0 {
                zero_rets += 1;
            }
            diff_ray_poly(&mut d, ("inside", n, i), a, &p, None);
        }
    }
    assert!(
        zero_rets > 1000,
        "expected mostly `index == ~0` rejections from inside, got {zero_rets}"
    );
    d.finish();
}

/// Row 40 — ray exactly PARALLEL to a face (`den == 0`), on both sides of that
/// face's plane: `num > 0` continues the loop, `num < 0` returns 0 immediately.
#[test]
fn cfg_40_poly_parallel() {
    let mut d = Diff::new("cfg_40_poly_parallel");
    let (f_c, _) = sym!("c2RaytoPoly", FnRayPoly);
    let mut rng = Rng::new(SEED ^ 40);
    let mut saw_reject = 0u32;
    let mut saw_continue = 0u32;
    for &(hw, hh) in &[(1.0f32, 1.0f32), (0.875, 11.5), (3.0, 0.25)] {
        let p = box_poly(hw, hh);
        // norms are exactly +-x / +-y, so axis-aligned rays give exact den == 0.
        for &dir in &[
            c2v { x: 1.0, y: 0.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: -1.0 },
        ] {
            for k in -40i32..=40 {
                let s = (k as f32) * 0.25 * hw.max(hh);
                for base in [
                    c2v { x: -50.0, y: s }, // horizontal sweep
                    c2v { x: s, y: -50.0 }, // vertical sweep
                    c2v { x: hw, y: s },    // exactly on a face plane
                    c2v { x: s, y: hh },
                ] {
                    for t in [0.0f32, 1.0, 100.0, 1e6, f32::INFINITY] {
                        let a = c2Ray { p: base, d: dir, t };
                        let mut probe = dirty();
                        if unsafe { f_c(a, &p, std::ptr::null(), &mut probe) } == 0 {
                            saw_reject += 1;
                        } else {
                            saw_continue += 1;
                        }
                        diff_ray_poly(&mut d, ("par", fb(hw), vb(dir), k, fb(t)), a, &p, None);
                    }
                }
            }
        }
    }
    let _ = &mut rng;
    assert!(saw_reject > 0 && saw_continue > 0, "need both outcomes: {saw_reject}/{saw_continue}");
    d.finish();
}

/// Row 41 — ray origin exactly ON a face plane (`num == 0`).
#[test]
fn cfg_41_poly_origin_on_face() {
    let mut d = Diff::new("cfg_41_poly_origin_on_face");
    let mut rng = Rng::new(SEED ^ 41);
    for n in [1usize, 3, 4, 5, 8] {
        let p = ngon(n, 3.0, 0.0, c2v { x: 0.0, y: 0.0 });
        for j in 0..n {
            // Any point on the segment verts[j]..verts[j+1] lies on plane j.
            let k = (j + 1) % n;
            for step in 0..9 {
                let u = (step as f32) / 8.0;
                let o = c2v {
                    x: p.verts[j].x + (p.verts[k].x - p.verts[j].x) * u,
                    y: p.verts[j].y + (p.verts[k].y - p.verts[j].y) * u,
                };
                for m in 0..12 {
                    let th = std::f32::consts::TAU * (m as f32) / 12.0;
                    for t in [0.0f32, 1.0, 10.0, f32::INFINITY] {
                        let a = c2Ray {
                            p: o,
                            d: c2v {
                                x: th.cos(),
                                y: th.sin(),
                            },
                            t,
                        };
                        diff_ray_poly(&mut d, ("onface", n, j, step, m, fb(t)), a, &p, None);
                    }
                }
            }
        }
        // and the vertices themselves (on TWO planes at once)
        for j in 0..n {
            for m in 0..12 {
                let th = std::f32::consts::TAU * (m as f32) / 12.0;
                let a = c2Ray {
                    p: p.verts[j],
                    d: c2v {
                        x: th.cos(),
                        y: th.sin(),
                    },
                    t: 5.0,
                };
                diff_ray_poly(&mut d, ("vert", n, j, m), a, &p, None);
            }
        }
    }
    let _ = &mut rng;
    d.finish();
}

/// Row 42 — `A.t` sweep around the exact hit distance (`hi` is initialised to
/// `A.t`, so this is the boundary of the `num < hi*den` narrowing).
#[test]
fn cfg_42_poly_t_boundary() {
    let mut d = Diff::new("cfg_42_poly_t_boundary");
    let (f_c, _) = sym!("c2RaytoPoly", FnRayPoly);
    let mut rng = Rng::new(SEED ^ 42);
    for n in [1usize, 3, 4, 6, 8] {
        let p = ngon(n, 2.0, 0.25, c2v { x: 0.0, y: 0.0 });
        for i in 0..128 {
            let th = rng.unit() * std::f32::consts::TAU;
            let dir = c2v {
                x: th.cos(),
                y: th.sin(),
            };
            let dist = 4.0 + rng.unit() * 6.0;
            let a0 = c2Ray {
                p: c2v {
                    x: -dir.x * dist,
                    y: -dir.y * dist,
                },
                d: dir,
                t: 1e9,
            };
            // Find the actual hit distance with the C, then probe around it.
            let mut probe = dirty();
            let hit = unsafe { f_c(a0, &p, std::ptr::null(), &mut probe) };
            let th_hit = if hit != 0 { probe.t } else { dist };
            for t in [
                0.0f32,
                -0.0,
                next_down(th_hit),
                th_hit,
                next_up(th_hit),
                th_hit * 0.5,
                th_hit * 2.0,
                1e9,
                f32::INFINITY,
                f32::NAN,
                -1.0,
                -1e9,
            ] {
                diff_ray_poly(
                    &mut d,
                    ("t", n, i, fb(t)),
                    c2Ray { p: a0.p, d: dir, t },
                    &p,
                    None,
                );
            }
        }
    }
    d.finish();
}

/// Row 43 — unnormalised and zero ray directions (the C never normalises `A.d`).
#[test]
fn cfg_43_poly_unnormalized_d() {
    let mut d = Diff::new("cfg_43_poly_unnormalized_d");
    let mut rng = Rng::new(SEED ^ 43);
    for n in [1usize, 4, 8] {
        let p = ngon(n, 2.0, 0.0, c2v { x: 0.0, y: 0.0 });
        for i in 0..256 {
            let th = rng.unit() * std::f32::consts::TAU;
            let unit = c2v {
                x: th.cos(),
                y: th.sin(),
            };
            for &scale in &[
                0.0f32, 1e-8, 1e-3, 1.0, 1e3, 1e8, 1e20, -1.0, -1e6, f32::INFINITY,
            ] {
                let dir = c2v {
                    x: unit.x * scale,
                    y: unit.y * scale,
                };
                for t in [0.0f32, 1.0, 10.0, 1e6, f32::INFINITY] {
                    let a = c2Ray {
                        p: c2v {
                            x: -unit.x * 6.0,
                            y: -unit.y * 6.0,
                        },
                        d: dir,
                        t,
                    };
                    diff_ray_poly(&mut d, ("scale", n, i, fb(scale), fb(t)), a, &p, None);
                }
            }
            // exactly-zero direction
            for t in [0.0f32, 1.0, f32::INFINITY] {
                let a = c2Ray {
                    p: rng.v(),
                    d: c2v { x: 0.0, y: 0.0 },
                    t,
                };
                diff_ray_poly(&mut d, ("zerodir", n, i, fb(t)), a, &p, None);
            }
        }
    }
    d.finish();
}

/// Row 44 — fully random polys (random `count`, arbitrary verts AND norms, so
/// deliberately non-convex / inconsistent) x random rays x random `bx`.
#[test]
fn cfg_44_poly_random() {
    let mut d = Diff::new("cfg_44_poly_random");
    let (f_c, _) = sym!("c2RaytoPoly", FnRayPoly);
    let mut rng = Rng::new(SEED ^ 44);
    let mut hits = 0u32;
    for i in 0..N_BIG {
        let p = rng.poly();
        let a = rng.ray();
        let bx = rng.x();
        let use_bx = rng.below(3) != 0;
        let bxr = if use_bx { Some(&bx) } else { None };
        let mut probe = dirty();
        let bxp: *const c2x = if use_bx { &bx } else { std::ptr::null() };
        hits += (unsafe { f_c(a, &p, bxp, &mut probe) } != 0) as u32;
        diff_ray_poly(&mut d, ("rand", i), a, &p, bxr);
    }
    assert!(
        hits > 100 && hits < N_BIG as u32 - 100,
        "poor outcome coverage: {hits} hits of {N_BIG}"
    );
    d.finish();
}

// ===========================================================================
// Rows 45-48: c2CastRay (the enum dispatcher)
// ===========================================================================

/// Row 45 — `typeB = C2_TYPE_CIRCLE`, with `bx` both NULL and non-NULL (the C
/// ignores `bx` for every type but POLY).
#[test]
fn cfg_45_castray_circle() {
    let mut d = Diff::new("cfg_45_castray_circle");
    let mut rng = Rng::new(SEED ^ 45);
    for i in 0..N {
        let a = rng.ray();
        let s = rng.circle();
        let bx = rng.x();
        diff_cast_ray(
            &mut d,
            ("null", i),
            a,
            &s as *const c2Circle as *const c_void,
            std::ptr::null(),
            C2_TYPE_CIRCLE,
        );
        diff_cast_ray(
            &mut d,
            ("bx", i),
            a,
            &s as *const c2Circle as *const c_void,
            &bx,
            C2_TYPE_CIRCLE,
        );
    }
    d.finish();
}

/// Row 46 — `typeB = C2_TYPE_AABB`, both `bx` states.
#[test]
fn cfg_46_castray_aabb() {
    let mut d = Diff::new("cfg_46_castray_aabb");
    let mut rng = Rng::new(SEED ^ 46);
    for i in 0..N {
        let a = rng.ray();
        let s = rng.aabb();
        let bx = rng.x();
        diff_cast_ray(
            &mut d,
            ("null", i),
            a,
            &s as *const c2AABB as *const c_void,
            std::ptr::null(),
            C2_TYPE_AABB,
        );
        diff_cast_ray(
            &mut d,
            ("bx", i),
            a,
            &s as *const c2AABB as *const c_void,
            &bx,
            C2_TYPE_AABB,
        );
    }
    d.finish();
}

/// Row 47 — `typeB = C2_TYPE_CAPSULE`, both `bx` states.
#[test]
fn cfg_47_castray_capsule() {
    let mut d = Diff::new("cfg_47_castray_capsule");
    let mut rng = Rng::new(SEED ^ 47);
    for i in 0..N {
        let a = rng.ray();
        let s = rng.capsule();
        let bx = rng.x();
        diff_cast_ray(
            &mut d,
            ("null", i),
            a,
            &s as *const c2Capsule as *const c_void,
            std::ptr::null(),
            C2_TYPE_CAPSULE,
        );
        diff_cast_ray(
            &mut d,
            ("bx", i),
            a,
            &s as *const c2Capsule as *const c_void,
            &bx,
            C2_TYPE_CAPSULE,
        );
    }
    d.finish();
}

/// Row 48 — `typeB = C2_TYPE_POLY` across the whole `bx` state space and every
/// `count` from 0..=8, dispatched through `c2CastRay` (not `c2RaytoPoly`).
#[test]
fn cfg_48_castray_poly_bx_matrix() {
    let mut d = Diff::new("cfg_48_castray_poly_bx_matrix");
    let mut rng = Rng::new(SEED ^ 48);
    for i in 0..N {
        let count = rng.below(9) as usize; // includes 0
        let mut p = if count == 0 {
            zero_poly()
        } else {
            ngon(count, 0.5 + rng.unit() * 5.0, rng.unit() * 6.28, rng.v())
        };
        p.count = count as c_int;
        let a = rng.ray();
        let bx_variants: [Option<c2x>; 6] = [
            None,
            Some(identity_x()),
            Some(c2x {
                p: rng.v(),
                r: c2r { c: 1.0, s: 0.0 },
            }),
            Some(c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: rot_x(rng.unit() * std::f32::consts::TAU),
            }),
            Some(c2x {
                p: rng.v(),
                r: rot_x(rng.unit() * std::f32::consts::TAU),
            }),
            Some(c2x {
                p: rng.v(),
                r: c2r {
                    c: rng.range(4.0),
                    s: rng.range(4.0),
                },
            }),
        ];
        for (k, bv) in bx_variants.iter().enumerate() {
            let bxp: *const c2x = match bv {
                Some(v) => v,
                None => std::ptr::null(),
            };
            diff_cast_ray(
                &mut d,
                ("poly", i, count, k),
                a,
                &p as *const c2Poly as *const c_void,
                bxp,
                C2_TYPE_POLY,
            );
        }
    }
    d.finish();
}

// ===========================================================================
// Rows 49-50: poly_ray (the public one-shot entry point)
// ===========================================================================

/// Row 49 — `poly_ray`: return value and both out-params.
#[test]
fn cfg_49_poly_ray() {
    let mut d = Diff::new("cfg_49_poly_ray");
    let (cf, rf) = sym!("poly_ray", FnPolyRay);
    for i in 0..64 {
        let (mut c1, mut c2) = (dirty(), dirty());
        let (mut r1, mut r2) = (dirty(), dirty());
        let cret = unsafe { cf(&mut c1, &mut c2) };
        let rret = unsafe { rf(&mut r1, &mut r2) };
        d.eq(("ret", i), cret, rret);
        d.eq(("cast1", i), cb(c1), cb(r1));
        d.eq(("cast2", i), cb(c2), cb(r2));
    }
    d.finish();
}

/// Row 50 — `poly_ray` with pre-dirtied out-params, including aliasing both
/// arguments to the SAME `c2Raycast`, to catch any difference in WHICH fields
/// get written (the C leaves them untouched when a cast misses).
#[test]
fn cfg_50_poly_ray_dirty_out() {
    let mut d = Diff::new("cfg_50_poly_ray_dirty_out");
    let (cf, rf) = sym!("poly_ray", FnPolyRay);
    let mut rng = Rng::new(SEED ^ 50);
    for i in 0..256 {
        // distinct random garbage in every field
        let mut garbage = || c2Raycast {
            t: f32::from_bits(rng.next_u32()),
            n: c2v {
                x: f32::from_bits(rng.next_u32()),
                y: f32::from_bits(rng.next_u32()),
            },
        };
        let g1 = garbage();
        let g2 = garbage();
        let (mut c1, mut c2) = (g1, g2);
        let (mut r1, mut r2) = (g1, g2);
        let cret = unsafe { cf(&mut c1, &mut c2) };
        let rret = unsafe { rf(&mut r1, &mut r2) };
        d.eq(("ret", i), cret, rret);
        d.eq(("cast1", i, cb(g1)), cb(c1), cb(r1));
        d.eq(("cast2", i, cb(g2)), cb(c2), cb(r2));

        // both out-params aliased to one object
        let g3 = garbage();
        let mut ca = g3;
        let mut ra = g3;
        let cr = unsafe { cf(&mut ca, &mut ca) };
        let rr = unsafe { rf(&mut ra, &mut ra) };
        d.eq(("alias ret", i), cr, rr);
        d.eq(("alias out", i, cb(g3)), cb(ca), cb(ra));
    }
    d.finish();
}
