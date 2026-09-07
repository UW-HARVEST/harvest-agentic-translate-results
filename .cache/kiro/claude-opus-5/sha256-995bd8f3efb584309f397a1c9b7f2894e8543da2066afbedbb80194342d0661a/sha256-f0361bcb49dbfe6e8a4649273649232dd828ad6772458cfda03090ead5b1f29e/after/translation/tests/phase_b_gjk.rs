//! Phase B — CONFIGS.md rows 22..39: the `c2GJK` low-level entry point.
//!
//! `c2GJK` is the lowest-level composed operation in the library and takes six
//! independent option knobs (`ax_ptr`, `bx_ptr`, `outA`, `outB`, `use_radius`,
//! `iterations`, `cache`). It is driven here directly rather than only through
//! the manifold convenience wrappers.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// One shape, kept in a form both libraries can be handed a pointer to.
#[derive(Clone, Copy)]
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
    /// C2_TYPE_POLY — `c2MakeProxy` has no case for it (CONFIGS row 39).
    Poly(c2Poly),
}

impl Shape {
    fn ty(&self) -> c_int {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
            Shape::Poly(_) => C2_TYPE_POLY,
        }
    }
    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const _ as *const c_void,
            Shape::Aabb(c) => c as *const _ as *const c_void,
            Shape::Capsule(c) => c as *const _ as *const c_void,
            Shape::Poly(c) => c as *const _ as *const c_void,
        }
    }
}

/// `scale` controls the separation regime: small scale ⇒ deep overlap,
/// large scale ⇒ separated (CONFIGS row 38).
fn rand_shape(rng: &mut Rng, kind: u32, scale: f32) -> Shape {
    match kind {
        0 => Shape::Circle(c2Circle {
            p: rng.vec(scale),
            r: rng.fpos(3.0),
        }),
        1 => {
            let c = rng.vec(scale);
            let e = v(rng.fpos(3.0), rng.fpos(3.0));
            Shape::Aabb(c2AABB {
                min: v(c.x - e.x, c.y - e.y),
                max: v(c.x + e.x, c.y + e.y),
            })
        }
        2 => {
            let c = rng.vec(scale);
            let d = rng.vec(3.0);
            Shape::Capsule(c2Capsule {
                a: v(c.x - d.x, c.y - d.y),
                b: v(c.x + d.x, c.y + d.y),
                r: rng.fpos(2.0),
            })
        }
        _ => {
            let mut p = c2Poly::default();
            p.count = (3 + rng.below(6)) as c_int;
            for k in 0..8 {
                p.verts[k] = rng.vec(scale);
            }
            Shape::Poly(p)
        }
    }
}

struct GjkOut {
    dist: f32,
    a: c2v,
    b: c2v,
    iters: c_int,
    cache: c2GJKCache,
}

#[allow(clippy::too_many_arguments)]
unsafe fn call(
    f: &FnGJK,
    sa: &Shape,
    ax: Option<&c2x>,
    sb: &Shape,
    bx: Option<&c2x>,
    use_radius: c_int,
    want_a: bool,
    want_b: bool,
    want_iters: bool,
    cache_in: Option<c2GJKCache>,
) -> GjkOut {
    let mut a = v(-777.0, -777.0);
    let mut b = v(-888.0, -888.0);
    let mut iters: c_int = -999;
    let mut cache = cache_in.unwrap_or_default();
    // Canonicalize the uninitialized-`pB` precondition (see `scrub_stack`).
    scrub_stack();
    let dist = unsafe {
        f(
            sa.ptr(),
            sa.ty(),
            ax.map_or(std::ptr::null(), |x| x as *const c2x),
            sb.ptr(),
            sb.ty(),
            bx.map_or(std::ptr::null(), |x| x as *const c2x),
            if want_a { &mut a } else { std::ptr::null_mut() },
            if want_b { &mut b } else { std::ptr::null_mut() },
            use_radius,
            if want_iters {
                &mut iters
            } else {
                std::ptr::null_mut()
            },
            if cache_in.is_some() {
                &mut cache
            } else {
                std::ptr::null_mut()
            },
        )
    };
    GjkOut {
        dist,
        a,
        b,
        iters,
        cache,
    }
}

fn cache_eq(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    feq(a.metric, b.metric) && a.count == b.count && a.iA == b.iA && a.iB == b.iB
        && feq(a.div, b.div)
}

fn cache_dbg(c: &c2GJKCache) -> String {
    format!(
        "metric={} count={} iA={:?} iB={:?} div={}",
        fs(c.metric),
        c.count,
        c.iA,
        c.iB,
        fs(c.div)
    )
}

fn assert_same(tag: &str, c: &GjkOut, r: &GjkOut) {
    assert!(
        feq(c.dist, r.dist),
        "{tag}: dist C {} vs R {}",
        fs(c.dist),
        fs(r.dist)
    );
    assert!(
        veq(c.a, r.a),
        "{tag}: outA C {} vs R {}",
        vs(c.a),
        vs(r.a)
    );
    assert!(
        veq(c.b, r.b),
        "{tag}: outB C {} vs R {}",
        vs(c.b),
        vs(r.b)
    );
    assert_eq!(c.iters, r.iters, "{tag}: iterations");
    assert!(
        cache_eq(&c.cache, &r.cache),
        "{tag}: cache C [{}] vs R [{}]",
        cache_dbg(&c.cache),
        cache_dbg(&r.cache)
    );
}

/// Drives rows 22..38: every ordered pair of {circle, aabb, capsule} against
/// every option combination, over four separation regimes.
#[test]
fn rows22_38_gjk_pairs_options_and_regimes() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0xC0FF_EE00_1234_5678);

    // (kindA, kindB) — rows 22..28 (all 9 ordered pairs over the 3 proxy shapes)
    let pairs: [(u32, u32); 9] = [
        (0, 0),
        (0, 1),
        (0, 2),
        (1, 0),
        (1, 1),
        (1, 2),
        (2, 0),
        (2, 1),
        (2, 2),
    ];
    // rows 38: deep / touching / shallow / separated
    let scales: [f32; 4] = [0.5, 2.0, 5.0, 60.0];

    let mut seen_hit = 0usize;
    let mut seen_miss = 0usize;

    for &(ka, kb) in &pairs {
        for &scale in &scales {
            for use_radius in [0, 1] {
                for iteration in 0..300usize {
                    let sa = rand_shape(&mut rng, ka, scale);
                    let sb = rand_shape(&mut rng, kb, scale);

                    // rows 29..32: transform variants
                    let (ax, bx) = match iteration % 6 {
                        0 => (None, None),                       // row 22 base
                        1 => (Some(c2x::default()), None),       // zero c2r (row 32)
                        2 => (
                            Some(c2x {
                                p: v(0.0, 0.0),
                                r: c2r { c: 1.0, s: 0.0 },
                            }),
                            None,
                        ), // row 29 identity
                        3 => (
                            Some(c2x {
                                p: rng.vec(5.0),
                                r: c2r { c: 1.0, s: 0.0 },
                            }),
                            None,
                        ), // row 30 pure translation
                        4 => (Some(rng.xform(5.0)), Some(rng.xform(5.0))), // row 31
                        _ => (
                            Some(c2x {
                                p: rng.vec(5.0),
                                r: c2r {
                                    c: rng.f(3.0),
                                    s: rng.f(3.0),
                                },
                            }),
                            Some(rng.xform(5.0)),
                        ), // row 32 non-unit c2r
                    };

                    // row 33 / 34: NULL out pointers, iterations pointer
                    let want_a = iteration % 7 != 3;
                    let want_b = iteration % 7 != 5;
                    let want_iters = iteration % 4 != 2;

                    // rows 35..37: cache variants
                    let cache_in = match iteration % 5 {
                        0 | 1 => None,                     // row 13 of ERRORS: NULL
                        2 => Some(c2GJKCache::default()),  // row 35: cold, count == 0
                        _ => None,
                    };

                    let tag = format!(
                        "c2GJK ka={ka} kb={kb} scale={scale} ur={use_radius} it={iteration}"
                    );
                    let co = unsafe {
                        call(
                            &cf, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, want_a,
                            want_b, want_iters, cache_in,
                        )
                    };
                    let ro = unsafe {
                        call(
                            &rf, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, want_a,
                            want_b, want_iters, cache_in,
                        )
                    };
                    if co.dist == 0.0 {
                        seen_hit += 1;
                    } else {
                        seen_miss += 1;
                    }
                    assert_same(&tag, &co, &ro);

                    // rows 36/37: feed the produced cache straight back in,
                    // once with the same shapes and once with moved shapes.
                    if iteration % 5 == 2 {
                        let warm = co.cache;
                        let co2 = unsafe {
                            call(
                                &cf, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, true,
                                true, true, Some(warm),
                            )
                        };
                        let ro2 = unsafe {
                            call(
                                &rf, &sa, ax.as_ref(), &sb, bx.as_ref(), use_radius, true,
                                true, true, Some(warm),
                            )
                        };
                        assert_same(&format!("{tag} [warm cache]"), &co2, &ro2);

                        // row 37: stale cache — same cache, different shapes
                        let sa2 = rand_shape(&mut rng, ka, scale);
                        let sb2 = rand_shape(&mut rng, kb, scale);
                        let co3 = unsafe {
                            call(
                                &cf, &sa2, ax.as_ref(), &sb2, bx.as_ref(), use_radius,
                                true, true, true, Some(warm),
                            )
                        };
                        let ro3 = unsafe {
                            call(
                                &rf, &sa2, ax.as_ref(), &sb2, bx.as_ref(), use_radius,
                                true, true, true, Some(warm),
                            )
                        };
                        assert_same(&format!("{tag} [stale cache]"), &co3, &ro3);
                    }
                }
            }
        }
    }
    assert!(
        seen_hit > 100 && seen_miss > 100,
        "regime coverage: hit={seen_hit} miss={seen_miss}"
    );
}

/// Row 39 — `C2_TYPE_POLY` reaches `c2GJK` but `c2MakeProxy` has no poly case,
/// so the proxy keeps whatever the (uninitialized) stack held. Both libraries
/// must agree.
#[test]
fn row39_gjk_with_poly_type() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15);

    for i in 0..1500usize {
        let poly = rand_shape(&mut rng, 3, 10.0);
        let other = rand_shape(&mut rng, i as u32 % 3, 10.0);
        let use_radius = (i % 2) as c_int;
        let (bx, ax) = (rng.xform(4.0), rng.xform(4.0));
        let (axo, bxo) = match i % 3 {
            0 => (None, None),
            1 => (Some(&ax), None),
            _ => (Some(&ax), Some(&bx)),
        };

        // poly as A
        let tag = format!("c2GJK poly-as-A i={i}");
        let co = unsafe { call(&cf, &poly, axo, &other, bxo, use_radius, true, true, true, None) };
        let ro = unsafe { call(&rf, &poly, axo, &other, bxo, use_radius, true, true, true, None) };
        assert_same(&tag, &co, &ro);

        // poly as B
        let tag = format!("c2GJK poly-as-B i={i}");
        let co = unsafe { call(&cf, &other, axo, &poly, bxo, use_radius, true, true, true, None) };
        let ro = unsafe { call(&rf, &other, axo, &poly, bxo, use_radius, true, true, true, None) };
        assert_same(&tag, &co, &ro);

        // both poly
        let poly2 = rand_shape(&mut rng, 3, 10.0);
        let tag = format!("c2GJK poly-both i={i}");
        let co = unsafe { call(&cf, &poly, axo, &poly2, bxo, use_radius, true, true, true, None) };
        let ro = unsafe { call(&rf, &poly, axo, &poly2, bxo, use_radius, true, true, true, None) };
        assert_same(&tag, &co, &ro);
    }
}

/// Row 34 — the iteration counter must match exactly, including the
/// non-convergence path that exits on `iter == 20` (ERRORS.md row 17).
#[test]
fn row34_iteration_counts_and_non_convergence() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x1357_9BDF_0246_8ACE);
    let mut hist = [0usize; 21];

    for i in 0..8000usize {
        // Long thin shapes at grazing angles maximize the iteration count.
        let a = Shape::Capsule(c2Capsule {
            a: v(rng.grid(0.25, 40), rng.grid(0.25, 40)),
            b: v(rng.grid(0.25, 40), rng.grid(0.25, 40)),
            r: rng.grid(0.25, 8).abs(),
        });
        let b = Shape::Aabb(c2AABB {
            min: v(rng.grid(0.25, 40), rng.grid(0.25, 40)),
            max: v(rng.grid(0.25, 40), rng.grid(0.25, 40)),
        });
        let ur = (i % 2) as c_int;
        let co = unsafe { call(&cf, &a, None, &b, None, ur, true, true, true, None) };
        let ro = unsafe { call(&rf, &a, None, &b, None, ur, true, true, true, None) };
        assert_same(&format!("c2GJK iters i={i}"), &co, &ro);
        if (0..=20).contains(&co.iters) {
            hist[co.iters as usize] += 1;
        }
    }
    assert!(hist.iter().sum::<usize>() > 7000, "iteration histogram {hist:?}");
}

/// Extra coverage for the `use_radius` collapse branches (ERRORS.md rows 21/22)
/// using grid-quantized coordinates so `dist == rA + rB` exactly occurs.
#[test]
fn gjk_use_radius_boundaries() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x2468_ACE0_1357_9BDF);
    for i in 0..8000usize {
        let ra = rng.grid(0.5, 6).abs();
        let rb = rng.grid(0.5, 6).abs();
        let a = Shape::Circle(c2Circle {
            p: v(rng.grid(0.5, 20), rng.grid(0.5, 20)),
            r: ra,
        });
        let b = Shape::Circle(c2Circle {
            p: v(rng.grid(0.5, 20), rng.grid(0.5, 20)),
            r: rb,
        });
        let co = unsafe { call(&cf, &a, None, &b, None, 1, true, true, true, None) };
        let ro = unsafe { call(&rf, &a, None, &b, None, 1, true, true, true, None) };
        assert_same(&format!("c2GJK radius boundary i={i}"), &co, &ro);
    }
}
