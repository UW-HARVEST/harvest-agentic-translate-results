//! Level 3: `c2GJK`, the boolean shape-vs-shape predicates, the `c2Collided`
//! dispatcher and the public `reverse_collide` entry point.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;
use std::ffi::c_void;

const ITERS: u32 = 6_000;

// ---------------------------------------------------------------------------
// Shape plumbing
// ---------------------------------------------------------------------------

/// A shape plus its type tag, kept in a byte buffer so both libraries receive
/// the exact same bit pattern behind a `const void *`.
#[derive(Clone, Copy)]
struct Shape {
    ty: c_int,
    buf: [u8; 32],
    /// Number of vertices `c2MakeProxy` will produce - used to keep crafted
    /// GJK caches in range.
    verts: c_int,
}

impl Shape {
    fn as_ptr(&self) -> *const c_void {
        self.buf.as_ptr() as *const c_void
    }
}

fn write_shape<T: Copy>(ty: c_int, verts: c_int, v: &T) -> Shape {
    let mut buf = [0u8; 32];
    assert!(std::mem::size_of::<T>() <= 32);
    unsafe {
        std::ptr::copy_nonoverlapping(v as *const T as *const u8, buf.as_mut_ptr(), size_of::<T>());
    }
    Shape { ty, buf, verts }
}

fn gen_shape(g: &mut Rng) -> Shape {
    match g.below(3) {
        0 => write_shape(C2_TYPE_CIRCLE, 1, &g.circle()),
        1 => write_shape(C2_TYPE_AABB, 4, &g.aabb_any()),
        _ => write_shape(C2_TYPE_CAPSULE, 2, &g.capsule_any()),
    }
}

/// Shapes clustered near the origin so that the overlapping / touching /
/// just-separated regimes are all hit frequently.
fn gen_close_shape(g: &mut Rng) -> Shape {
    let s = 8.0;
    match g.below(3) {
        0 => write_shape(
            C2_TYPE_CIRCLE,
            1,
            &c2Circle {
                p: g.v(s),
                r: g.f32_range(s).abs(),
            },
        ),
        1 => {
            let a = g.v(s);
            let b = g.v(s);
            write_shape(
                C2_TYPE_AABB,
                4,
                &c2AABB {
                    min: c2v {
                        x: a.x.min(b.x),
                        y: a.y.min(b.y),
                    },
                    max: c2v {
                        x: a.x.max(b.x),
                        y: a.y.max(b.y),
                    },
                },
            )
        }
        _ => write_shape(
            C2_TYPE_CAPSULE,
            2,
            &c2Capsule {
                a: g.v(s),
                b: g.v(s),
                r: g.f32_range(s).abs(),
            },
        ),
    }
}

// ---------------------------------------------------------------------------
// c2GJK
// ---------------------------------------------------------------------------

struct GjkOutcome {
    dist: f32,
    outA: c2v,
    outB: c2v,
    iters: c_int,
    cache: c2GJKCache,
}

#[allow(clippy::too_many_arguments)]
fn call_gjk(
    api: &Api,
    a: &Shape,
    axf: Option<&c2x>,
    b: &Shape,
    bxf: Option<&c2x>,
    use_radius: c_int,
    cache_in: Option<c2GJKCache>,
    want_out: bool,
    want_iters: bool,
) -> GjkOutcome {
    // Sentinels so that "did not write" is distinguishable from "wrote 0".
    let mut outA = c2v { x: 1234.5, y: -876.25 };
    let mut outB = c2v { x: -4321.5, y: 55.125 };
    let mut iters: c_int = -12345;
    let mut cache = cache_in.unwrap_or_default();

    let dist = unsafe {
        (api.c2GJK)(
            a.as_ptr(),
            a.ty,
            axf.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            b.as_ptr(),
            b.ty,
            bxf.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            if want_out {
                &mut outA as *mut c2v
            } else {
                std::ptr::null_mut()
            },
            if want_out {
                &mut outB as *mut c2v
            } else {
                std::ptr::null_mut()
            },
            use_radius,
            if want_iters {
                &mut iters as *mut c_int
            } else {
                std::ptr::null_mut()
            },
            if cache_in.is_some() {
                &mut cache as *mut c2GJKCache
            } else {
                std::ptr::null_mut()
            },
        )
    };
    GjkOutcome {
        dist,
        outA,
        outB,
        iters,
        cache,
    }
}

#[track_caller]
fn cmp_gjk(x: &GjkOutcome, y: &GjkOutcome, ctx: &str) {
    eq_f32(x.dist, y.dist, &format!("{ctx} dist"));
    eq_v(x.outA, y.outA, &format!("{ctx} outA"));
    eq_v(x.outB, y.outB, &format!("{ctx} outB"));
    eq_i(x.iters, y.iters, &format!("{ctx} iterations"));
    eq_cache(&x.cache, &y.cache, &format!("{ctx} cache"));
}

fn gjk_sweep(seed: u64, iters: u32, close: bool, ctx: &str) {
    let (c, r) = apis();
    let mut g = Rng::new(seed);
    for n in 0..iters {
        let sa = if close {
            gen_close_shape(&mut g)
        } else {
            gen_shape(&mut g)
        };
        let sb = if close {
            gen_close_shape(&mut g)
        } else {
            gen_shape(&mut g)
        };
        // Transform variants: NULL (identity fast path), an explicit identity
        // and a general rotation+translation.
        let axf = match g.below(3) {
            0 => None,
            1 => Some(c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: c2r { c: 1.0, s: 0.0 },
            }),
            _ => Some(g.xform()),
        };
        let bxf = match g.below(3) {
            0 => None,
            1 => Some(c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: c2r { c: 1.0, s: 0.0 },
            }),
            _ => Some(g.xform()),
        };
        let use_radius = (g.below(2)) as c_int;
        let want_out = g.below(4) != 0;
        let want_iters = g.below(4) != 0;

        let cache_in = if g.below(3) == 0 {
            // A crafted cache with in-range indices and 1..=3 entries.
            let count = 1 + g.below(3) as c_int;
            let mut ca = c2GJKCache {
                metric: g.f32_range(50.0),
                count,
                iA: [0; 3],
                iB: [0; 3],
                div: g.f32_range(4.0),
            };
            for i in 0..3 {
                ca.iA[i] = g.below(sa.verts as u32) as c_int;
                ca.iB[i] = g.below(sb.verts as u32) as c_int;
            }
            Some(ca)
        } else if g.below(2) == 0 {
            Some(c2GJKCache::default()) // count == 0 -> "cache not good"
        } else {
            None
        };

        let ctx = format!("{ctx}#{n} (tA={} tB={} ur={use_radius})", sa.ty, sb.ty);
        let rc = call_gjk(
            c,
            &sa,
            axf.as_ref(),
            &sb,
            bxf.as_ref(),
            use_radius,
            cache_in,
            want_out,
            want_iters,
        );
        let rr = call_gjk(
            r,
            &sa,
            axf.as_ref(),
            &sb,
            bxf.as_ref(),
            use_radius,
            cache_in,
            want_out,
            want_iters,
        );
        cmp_gjk(&rc, &rr, &ctx);

        // Feed the produced cache straight back in - the same shapes are used,
        // so the stored indices stay valid, which is exactly how a caller would
        // reuse a cache across frames.
        if cache_in.is_some() {
            let rc2 = call_gjk(
                c,
                &sa,
                axf.as_ref(),
                &sb,
                bxf.as_ref(),
                use_radius,
                Some(rc.cache),
                true,
                true,
            );
            let rr2 = call_gjk(
                r,
                &sa,
                axf.as_ref(),
                &sb,
                bxf.as_ref(),
                use_radius,
                Some(rr.cache),
                true,
                true,
            );
            cmp_gjk(&rc2, &rr2, &format!("{ctx}/warm"));
        }
    }
}

#[test]
fn c2GJK_random_shapes() {
    gjk_sweep(201, scaled(ITERS), false, "c2GJK");
}

#[test]
fn c2GJK_close_shapes() {
    gjk_sweep(202, scaled(ITERS), true, "c2GJK/close");
}

#[test]
fn c2GJK_identical_and_touching() {
    let (c, r) = apis();
    let mut g = Rng::new(203);
    for n in 0..scaled(ITERS) {
        // Identical shapes (distance 0, deep overlap) and exactly touching
        // configurations drive the `hit` and the `dist <= rA + rB` branches.
        let sa = gen_close_shape(&mut g);
        let sb = if g.below(2) == 0 {
            sa
        } else {
            gen_close_shape(&mut g)
        };
        for &use_radius in &[0, 1] {
            let rc = call_gjk(c, &sa, None, &sb, None, use_radius, None, true, true);
            let rr = call_gjk(r, &sa, None, &sb, None, use_radius, None, true, true);
            cmp_gjk(&rc, &rr, &format!("c2GJK/ident#{n} ur={use_radius}"));
        }
    }
}

#[test]
fn c2GJK_degenerate_shapes() {
    let (c, r) = apis();
    let mut g = Rng::new(204);
    for n in 0..scaled(ITERS) {
        // Zero-radius circles, zero-area boxes, zero-length capsules and huge
        // coordinates: these are where the `c2Norm` division and the
        // `dist > FLT_EPSILON` guard become interesting.
        let mk = |g: &mut Rng| -> Shape {
            match g.below(6) {
                0 => write_shape(
                    C2_TYPE_CIRCLE,
                    1,
                    &c2Circle {
                        p: g.v(5.0),
                        r: 0.0,
                    },
                ),
                1 => {
                    let p = g.v(5.0);
                    write_shape(C2_TYPE_AABB, 4, &c2AABB { min: p, max: p })
                }
                2 => {
                    let a = g.v(5.0);
                    write_shape(C2_TYPE_CAPSULE, 2, &c2Capsule { a, b: a, r: 0.0 })
                }
                3 => write_shape(
                    C2_TYPE_CIRCLE,
                    1,
                    &c2Circle {
                        p: g.v(1.0e7),
                        r: g.f32_range(1.0e6).abs(),
                    },
                ),
                4 => write_shape(
                    C2_TYPE_CAPSULE,
                    2,
                    &c2Capsule {
                        a: g.v(1.0e-4),
                        b: g.v(1.0e-4),
                        r: g.f32_range(1.0e-4).abs(),
                    },
                ),
                _ => gen_close_shape(g),
            }
        };
        let sa = mk(&mut g);
        let sb = mk(&mut g);
        for &use_radius in &[0, 1] {
            let rc = call_gjk(c, &sa, None, &sb, None, use_radius, None, true, true);
            let rr = call_gjk(r, &sa, None, &sb, None, use_radius, None, true, true);
            cmp_gjk(&rc, &rr, &format!("c2GJK/degen#{n} ur={use_radius}"));
        }
    }
}

#[test]
fn c2GJK_nonfinite_shapes() {
    let (c, r) = apis();
    let mut g = Rng::new(205);
    // `c2GJK` returns a float and writes floats through `outA`/`outB` and the
    // cache, so NaN/infinity handling (and NaN payload propagation) is directly
    // observable here.
    for n in 0..scaled(ITERS) {
        let mk = |g: &mut Rng| -> Shape {
            match g.below(3) {
                0 => write_shape(
                    C2_TYPE_CIRCLE,
                    1,
                    &c2Circle {
                        p: g.v_nasty(),
                        r: g.f32_nasty(),
                    },
                ),
                1 => write_shape(
                    C2_TYPE_AABB,
                    4,
                    &c2AABB {
                        min: g.v_nasty(),
                        max: g.v_nasty(),
                    },
                ),
                _ => write_shape(
                    C2_TYPE_CAPSULE,
                    2,
                    &c2Capsule {
                        a: g.v_nasty(),
                        b: g.v_nasty(),
                        r: g.f32_nasty(),
                    },
                ),
            }
        };
        let sa = mk(&mut g);
        let sb = mk(&mut g);
        let axf = if g.below(2) == 0 {
            None
        } else {
            Some(c2x {
                p: g.v_nasty(),
                r: c2r {
                    c: g.f32_nasty(),
                    s: g.f32_nasty(),
                },
            })
        };
        for &use_radius in &[0, 1] {
            let rc = call_gjk(c, &sa, axf.as_ref(), &sb, None, use_radius, None, true, true);
            let rr = call_gjk(r, &sa, axf.as_ref(), &sb, None, use_radius, None, true, true);
            cmp_gjk(&rc, &rr, &format!("c2GJK/nonfinite#{n} ur={use_radius}"));
        }
        // Same again with a cache so `metric` / `div` are compared too.
        let rc = call_gjk(
            c,
            &sa,
            None,
            &sb,
            None,
            1,
            Some(c2GJKCache::default()),
            true,
            true,
        );
        let rr = call_gjk(
            r,
            &sa,
            None,
            &sb,
            None,
            1,
            Some(c2GJKCache::default()),
            true,
            true,
        );
        cmp_gjk(&rc, &rr, &format!("c2GJK/nonfinite-cache#{n}"));
    }
}

// ---------------------------------------------------------------------------
// Boolean predicates
// ---------------------------------------------------------------------------

#[test]
fn c2AABBtoAABB_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(210);
    for _ in 0..scaled(40_000) {
        let (A, B) = if g.below(4) == 0 {
            (
                c2AABB {
                    min: g.v_nasty(),
                    max: g.v_nasty(),
                },
                c2AABB {
                    min: g.v_nasty(),
                    max: g.v_nasty(),
                },
            )
        } else {
            (g.aabb_any(), g.aabb_any())
        };
        unsafe {
            eq_i(
                (c.c2AABBtoAABB)(A, B),
                (r.c2AABBtoAABB)(A, B),
                "c2AABBtoAABB",
            );
        }
    }
}

#[test]
fn c2CircletoCircle_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(211);
    for _ in 0..scaled(40_000) {
        let (A, B) = if g.below(4) == 0 {
            (
                c2Circle {
                    p: g.v_nasty(),
                    r: g.f32_nasty(),
                },
                c2Circle {
                    p: g.v_nasty(),
                    r: g.f32_nasty(),
                },
            )
        } else {
            (g.circle(), g.circle())
        };
        unsafe {
            eq_i(
                (c.c2CircletoCircle)(A, B),
                (r.c2CircletoCircle)(A, B),
                "c2CircletoCircle",
            );
        }
    }
}

#[test]
fn c2CircletoAABB_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(212);
    for _ in 0..scaled(40_000) {
        let (A, B) = if g.below(4) == 0 {
            (
                c2Circle {
                    p: g.v_nasty(),
                    r: g.f32_nasty(),
                },
                c2AABB {
                    min: g.v_nasty(),
                    max: g.v_nasty(),
                },
            )
        } else {
            (g.circle(), g.aabb_any())
        };
        unsafe {
            eq_i(
                (c.c2CircletoAABB)(A, B),
                (r.c2CircletoAABB)(A, B),
                "c2CircletoAABB",
            );
        }
    }
}

#[test]
fn c2CircletoCapsule_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(213);
    for _ in 0..scaled(40_000) {
        let (A, B) = if g.below(4) == 0 {
            (
                c2Circle {
                    p: g.v_nasty(),
                    r: g.f32_nasty(),
                },
                c2Capsule {
                    a: g.v_nasty(),
                    b: g.v_nasty(),
                    r: g.f32_nasty(),
                },
            )
        } else {
            (g.circle(), g.capsule_any())
        };
        unsafe {
            eq_i(
                (c.c2CircletoCapsule)(A, B),
                (r.c2CircletoCapsule)(A, B),
                "c2CircletoCapsule",
            );
        }
    }
}

#[test]
fn c2AABBtoCapsule_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(214);
    for _ in 0..scaled(ITERS * 2) {
        let (A, B) = if g.below(3) == 0 {
            let s = 8.0;
            let a = g.v(s);
            let b = g.v(s);
            (
                c2AABB {
                    min: c2v {
                        x: a.x.min(b.x),
                        y: a.y.min(b.y),
                    },
                    max: c2v {
                        x: a.x.max(b.x),
                        y: a.y.max(b.y),
                    },
                },
                c2Capsule {
                    a: g.v(s),
                    b: g.v(s),
                    r: g.f32_range(s).abs(),
                },
            )
        } else {
            (g.aabb_any(), g.capsule_any())
        };
        unsafe {
            eq_i(
                (c.c2AABBtoCapsule)(A, B),
                (r.c2AABBtoCapsule)(A, B),
                "c2AABBtoCapsule",
            );
        }
    }
}

#[test]
fn c2CapsuletoCapsule_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(215);
    for _ in 0..scaled(ITERS * 2) {
        let (A, B) = if g.below(3) == 0 {
            let s = 8.0;
            (
                c2Capsule {
                    a: g.v(s),
                    b: g.v(s),
                    r: g.f32_range(s).abs(),
                },
                c2Capsule {
                    a: g.v(s),
                    b: g.v(s),
                    r: g.f32_range(s).abs(),
                },
            )
        } else {
            (g.capsule_any(), g.capsule_any())
        };
        unsafe {
            eq_i(
                (c.c2CapsuletoCapsule)(A, B),
                (r.c2CapsuletoCapsule)(A, B),
                "c2CapsuletoCapsule",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// c2Collided dispatcher
// ---------------------------------------------------------------------------

#[test]
fn c2Collided_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(220);
    for n in 0..scaled(ITERS * 3) {
        let sa = if g.below(2) == 0 {
            gen_close_shape(&mut g)
        } else {
            gen_shape(&mut g)
        };
        let sb = if g.below(2) == 0 {
            gen_close_shape(&mut g)
        } else {
            gen_shape(&mut g)
        };
        unsafe {
            eq_i(
                (c.c2Collided)(sa.as_ptr(), sa.ty, sb.as_ptr(), sb.ty),
                (r.c2Collided)(sa.as_ptr(), sa.ty, sb.as_ptr(), sb.ty),
                &format!("c2Collided#{n} ({},{})", sa.ty, sb.ty),
            );
        }
    }
}

#[test]
fn c2Collided_all_type_pairs_including_invalid() {
    let (c, r) = apis();
    let mut g = Rng::new(221);
    // Every ordered pair over the valid tags plus out-of-range tags, which
    // select the `default:` arms and must return 0.
    let tags: [c_int; 6] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE, 3, -1, 77];
    for _ in 0..scaled(400) {
        let sa = gen_close_shape(&mut g);
        let sb = gen_close_shape(&mut g);
        for &ta in tags.iter() {
            for &tb in tags.iter() {
                // Only exercise combinations where at least one tag is invalid
                // with a real payload; valid/valid is covered above.  An
                // invalid *typeA* returns 0 without dereferencing anything.
                if (0..=2).contains(&ta) && (0..=2).contains(&tb) {
                    continue;
                }
                let (mut a, mut b) = (sa, sb);
                a.ty = ta;
                b.ty = tb;
                unsafe {
                    eq_i(
                        (c.c2Collided)(a.as_ptr(), ta, b.as_ptr(), tb),
                        (r.c2Collided)(a.as_ptr(), ta, b.as_ptr(), tb),
                        &format!("c2Collided({ta},{tb})"),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// reverse_collide - the only function declared in include/lib.h
// ---------------------------------------------------------------------------

#[test]
fn reverse_collide_matches_random() {
    let (c, r) = apis();
    let mut g = Rng::new(230);
    for _ in 0..scaled(200_000) {
        let (x, y, rad) = match g.below(8) {
            0 => (g.f32_nasty(), g.f32_nasty(), g.f32_nasty()),
            1 => (g.f32_range(200.0), g.f32_range(200.0), g.f32_range(200.0)),
            2 => (g.f32_range(1.0e20), g.f32_range(1.0e20), g.f32_range(1.0e20)),
            // Cluster around the three hard-coded shapes in `reverse_collide`.
            3 => (-70.0 + g.f32_range(30.0), g.f32_range(30.0), g.f32_range(30.0)),
            4 => (
                -27.5 + g.f32_range(30.0),
                -27.5 + g.f32_range(30.0),
                g.f32_range(30.0),
            ),
            5 => (
                -30.0 + g.f32_range(40.0),
                70.0 + g.f32_range(50.0),
                g.f32_range(30.0),
            ),
            _ => (g.f32_range(120.0), g.f32_range(120.0), g.f32_range(40.0)),
        };
        unsafe {
            eq_i(
                (c.reverse_collide)(x, y, rad),
                (r.reverse_collide)(x, y, rad),
                &format!("reverse_collide({x:e}, {y:e}, {rad:e})"),
            );
        }
    }
}

#[test]
fn reverse_collide_matches_grid() {
    let (c, r) = apis();
    // Dense deterministic grid across the region occupied by the three
    // hard-coded shapes, with several radii, so every one of the eight possible
    // result bit patterns is reachable.
    let mut checked = 0u64;
    let mut seen = [false; 8];
    for xi in -130..=20 {
        for yi in -60..=120 {
            for &rad in &[0.0f32, 0.5, 1.0, 5.0, 10.0, 20.0, 40.0] {
                let x = xi as f32 * 1.0;
                let y = yi as f32 * 1.0;
                let (vc, vr) = unsafe {
                    (
                        (c.reverse_collide)(x, y, rad),
                        (r.reverse_collide)(x, y, rad),
                    )
                };
                eq_i(vc, vr, &format!("reverse_collide({x}, {y}, {rad})"));
                if (0..8).contains(&vc) {
                    seen[vc as usize] = true;
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 100_000, "grid too small: {checked}");
    // Sanity check on coverage: the interesting non-zero results must occur.
    for bit in [0usize, 1, 2, 4] {
        assert!(seen[bit], "result {bit} never produced - coverage gap");
    }
}

#[test]
fn reverse_collide_special_values() {
    let (c, r) = apis();
    let specials = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::MIN,
        f32::EPSILON,
        1.192_092_895_507_812_5e-7,
        -70.0,
        -40.0,
        -20.0,
        -15.0,
        20.0,
        10.0,
        100.0,
    ];
    for &x in specials.iter() {
        for &y in specials.iter() {
            for &rad in specials.iter() {
                unsafe {
                    eq_i(
                        (c.reverse_collide)(x, y, rad),
                        (r.reverse_collide)(x, y, rad),
                        &format!("reverse_collide({x:?}, {y:?}, {rad:?})"),
                    );
                }
            }
        }
    }
}
