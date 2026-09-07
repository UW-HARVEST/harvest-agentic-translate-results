//! Phase B — valid-path differential tests for `c2MakeProxy` and `c2GJK`.
//!
//! CONFIGS.md rows 10–12 and 35–56.
//! `c2GJK` is the lowest-level composed entry point in the library; every one of
//! its option axes (type pair, transform nullness/content, `use_radius`, cache
//! state, out-pointer nullness) is driven directly here rather than only through
//! the `c2*to*` convenience wrappers.

mod common;

use common::*;
use std::ffi::c_void;

type MakeProxyFn = unsafe extern "C" fn(*const c_void, i32, *mut c2Proxy) -> ();

#[allow(clippy::type_complexity)]
type GjkFn = unsafe extern "C" fn(
    *const c_void, // A
    i32,           // typeA
    *const c2x,    // ax
    *const c_void, // B
    i32,           // typeB
    *const c2x,    // bx
    *mut c2v,      // outA
    *mut c2v,      // outB
    i32,           // use_radius
    *mut i32,      // iterations
    *mut c2GJKCache,
) -> f32;

/// A shape blob big enough for any of the three shape types, plus its type tag.
#[derive(Copy, Clone, Debug)]
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn ty(&self) -> i32 {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(c) => c as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
    /// Highest valid proxy vertex index for this shape type.
    fn max_vert_index(&self) -> i32 {
        match self {
            Shape::Circle(_) => 0,
            Shape::Aabb(_) => 3,
            Shape::Capsule(_) => 1,
        }
    }
    fn rand(rng: &mut Rng, ty: i32, scale: f32) -> Shape {
        match ty {
            C2_TYPE_CIRCLE => Shape::Circle(rng.circle(scale)),
            C2_TYPE_AABB => Shape::Aabb(rng.aabb(scale)),
            _ => Shape::Capsule(rng.capsule(scale)),
        }
    }
    /// Translate the shape by `d` (used for the "cache survives motion" row).
    fn translated(&self, d: c2v) -> Shape {
        let t = |v: c2v| c2v {
            x: v.x + d.x,
            y: v.y + d.y,
        };
        match *self {
            Shape::Circle(c) => Shape::Circle(c2Circle { p: t(c.p), r: c.r }),
            Shape::Aabb(c) => Shape::Aabb(c2AABB {
                min: t(c.min),
                max: t(c.max),
            }),
            Shape::Capsule(c) => Shape::Capsule(c2Capsule {
                a: t(c.a),
                b: t(c.b),
                r: c.r,
            }),
        }
    }
}

/// One differential `c2GJK` invocation. Compares the return value, both witness
/// points, the iteration count and the written-back cache, all bit-exactly.
#[allow(clippy::too_many_arguments)]
struct GjkCall {
    a: Shape,
    b: Shape,
    ax: Option<c2x>,
    bx: Option<c2x>,
    use_radius: i32,
    /// `None` = pass NULL; `Some(seed)` = pass a cache initialised to `seed`.
    cache: Option<c2GJKCache>,
    want_out_a: bool,
    want_out_b: bool,
    want_iters: bool,
}

impl GjkCall {
    fn simple(a: Shape, b: Shape, use_radius: i32) -> Self {
        GjkCall {
            a,
            b,
            ax: None,
            bx: None,
            use_radius,
            cache: None,
            want_out_a: true,
            want_out_b: true,
            want_iters: true,
        }
    }

    /// Runs the call against one library. Returns
    /// `(dist, outA, outB, iters, cache_after)`.
    unsafe fn run(&self, f: &GjkFn) -> (f32, c2v, c2v, i32, Option<c2GJKCache>) {
        // sentinels so "not written" is distinguishable from "written to 0"
        let sent_v = c2v {
            x: -98765.4,
            y: 13579.25,
        };
        let sent_i: i32 = -424242;

        let mut oa = sent_v;
        let mut ob = sent_v;
        let mut it = sent_i;
        let mut cache = self.cache;

        let ax_ptr = self.ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
        let bx_ptr = self.bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);

        let d = unsafe {
            f(
                self.a.ptr(),
                self.a.ty(),
                ax_ptr,
                self.b.ptr(),
                self.b.ty(),
                bx_ptr,
                if self.want_out_a {
                    &mut oa
                } else {
                    std::ptr::null_mut()
                },
                if self.want_out_b {
                    &mut ob
                } else {
                    std::ptr::null_mut()
                },
                self.use_radius,
                if self.want_iters {
                    &mut it
                } else {
                    std::ptr::null_mut()
                },
                cache
                    .as_mut()
                    .map_or(std::ptr::null_mut(), |c| c as *mut c2GJKCache),
            )
        };
        (d, oa, ob, it, cache)
    }

    fn check(&self, c: &GjkFn, r: &GjkFn, tag: &str) {
        unsafe {
            let (cd, ca, cb, ci, cc) = self.run(c);
            let (rd, ra, rb, ri, rc) = self.run(r);
            assert_f32_bits!(cd, rd, "{tag} dist :: {self:?}");
            assert_v_bits!(ca, ra, "{tag} outA :: {self:?}");
            assert_v_bits!(cb, rb, "{tag} outB :: {self:?}");
            assert_eq!(ci, ri, "{tag} iterations :: {self:?}");
            match (cc, rc) {
                (Some(x), Some(y)) => assert!(
                    cache_bits_eq(&x, &y),
                    "{tag} cache writeback :: {self:?}\n  C={x:?}\n Rs={y:?}"
                ),
                (None, None) => {}
                _ => panic!("{tag} cache presence mismatch"),
            }
        }
    }
}

impl std::fmt::Debug for GjkCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A={:?} B={:?} ax={:?} bx={:?} use_radius={} cache={:?} outA={} outB={} iters={}",
            self.a,
            self.b,
            self.ax,
            self.bx,
            self.use_radius,
            self.cache,
            self.want_out_a,
            self.want_out_b,
            self.want_iters
        )
    }
}

fn gjk_pair() -> (GjkFn, GjkFn) {
    let (c, r) = pair::<GjkFn>("c2GJK");
    (*c, *r)
}

// ---------------------------------------------------------------------------
// Rows 10, 11, 12 — c2MakeProxy for each shape type
// ---------------------------------------------------------------------------

#[test]
fn row10_row11_row12_make_proxy() {
    let (c_f, r_f) = pair::<MakeProxyFn>("c2MakeProxy");
    let mut rng = Rng::new(0x0010);

    // Pre-fill the proxy with a recognisable pattern so we also verify the
    // untouched tail slots (verts[count..8]) are left exactly as they were.
    let seed_proxy = |k: usize| c2Proxy {
        radius: -777.5,
        count: -31337,
        verts: std::array::from_fn(|i| c2v {
            x: (k * 100 + i) as f32,
            y: -((k * 100 + i) as f32),
        }),
    };

    for i in 0..6000 {
        let ty = ALL_TYPES[i % 3];
        let shape = Shape::rand(&mut rng, ty, 100.0);
        unsafe {
            let mut cp = seed_proxy(i);
            let mut rp = seed_proxy(i);
            c_f(shape.ptr(), ty, &mut cp);
            r_f(shape.ptr(), ty, &mut rp);
            assert!(
                proxy_bits_eq(&cp, &rp),
                "c2MakeProxy ty={ty} #{i} {shape:?}\n  C={cp:?}\n Rs={rp:?}"
            );
        }
    }

    // Edge-value shapes.
    for &a in EDGE_F32 {
        for &b in EDGE_F32 {
            let shapes = [
                Shape::Circle(c2Circle {
                    p: c2v { x: a, y: b },
                    r: b,
                }),
                Shape::Aabb(c2AABB {
                    min: c2v { x: a, y: b },
                    max: c2v { x: b, y: a },
                }),
                Shape::Capsule(c2Capsule {
                    a: c2v { x: a, y: b },
                    b: c2v { x: b, y: a },
                    r: a,
                }),
            ];
            for s in shapes {
                unsafe {
                    let mut cp = seed_proxy(1);
                    let mut rp = seed_proxy(1);
                    c_f(s.ptr(), s.ty(), &mut cp);
                    r_f(s.ptr(), s.ty(), &mut rp);
                    assert!(proxy_bits_eq(&cp, &rp), "c2MakeProxy edge {s:?}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 35–44 — all 9 type pairs, NULL transforms, use_radius 0 and 1
// ---------------------------------------------------------------------------

#[test]
fn row35_to_row44_all_type_pairs_null_transforms() {
    let (c_f, r_f) = gjk_pair();
    let mut rng = Rng::new(0x0035);

    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for use_radius in [0i32, 1] {
                for i in 0..1500 {
                    // vary the coordinate scale so separated / overlapping /
                    // nested arrangements all occur
                    let scale = match i % 4 {
                        0 => 2.0,
                        1 => 20.0,
                        2 => 200.0,
                        _ => 1.0e6,
                    };
                    let call = GjkCall::simple(
                        Shape::rand(&mut rng, ta, scale),
                        Shape::rand(&mut rng, tb, scale),
                        use_radius,
                    );
                    call.check(&c_f, &r_f, &format!("gjk({ta},{tb}) ur={use_radius} #{i}"));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 45, 46, 47 — transform pointer nullness and content
// ---------------------------------------------------------------------------

#[test]
fn row45_row46_row47_transforms() {
    let (c_f, r_f) = gjk_pair();
    let mut rng = Rng::new(0x0045);
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };

    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for i in 0..1200 {
                let a = Shape::rand(&mut rng, ta, 30.0);
                let b = Shape::rand(&mut rng, tb, 30.0);
                let use_radius = (i % 2) as i32;

                // Row 45: explicit identity transforms.
                let mut call = GjkCall::simple(a, b, use_radius);
                call.ax = Some(ident);
                call.bx = Some(ident);
                call.check(&c_f, &r_f, &format!("gjk ident-xform ({ta},{tb}) #{i}"));

                // Row 46: randomized rotation + translation on both.
                let mut call = GjkCall::simple(a, b, use_radius);
                call.ax = Some(rng.xform(30.0));
                call.bx = Some(rng.xform(30.0));
                call.check(&c_f, &r_f, &format!("gjk rot-xform ({ta},{tb}) #{i}"));

                // Row 47: mixed nullness, both ways round.
                let mut call = GjkCall::simple(a, b, use_radius);
                call.ax = Some(rng.xform(30.0));
                call.bx = None;
                call.check(&c_f, &r_f, &format!("gjk ax-only ({ta},{tb}) #{i}"));

                let mut call = GjkCall::simple(a, b, use_radius);
                call.ax = None;
                call.bx = Some(rng.xform(30.0));
                call.check(&c_f, &r_f, &format!("gjk bx-only ({ta},{tb}) #{i}"));

                // translation-only and rotation-only
                let mut call = GjkCall::simple(a, b, use_radius);
                call.ax = Some(c2x {
                    p: rng.vec(30.0),
                    r: c2r { c: 1.0, s: 0.0 },
                });
                call.bx = Some(c2x {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: rng.rot(),
                });
                call.check(&c_f, &r_f, &format!("gjk split-xform ({ta},{tb}) #{i}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 48 — cold cache (count == 0, garbage elsewhere); compare writeback
// ---------------------------------------------------------------------------

#[test]
fn row48_cold_cache() {
    let (c_f, r_f) = gjk_pair();
    let mut rng = Rng::new(0x0048);

    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for i in 0..1200 {
                // count == 0 makes cache_was_good false; the rest is garbage
                // that the C must therefore ignore on read but overwrite on exit.
                let seed = c2GJKCache {
                    metric: rng.coord(1e4),
                    count: 0,
                    iA: [rng.below(100) as i32 - 50; 3],
                    iB: [rng.below(100) as i32 - 50; 3],
                    div: rng.coord(1e4),
                };
                let mut call = GjkCall::simple(
                    Shape::rand(&mut rng, ta, 30.0),
                    Shape::rand(&mut rng, tb, 30.0),
                    (i % 2) as i32,
                );
                call.cache = Some(seed);
                call.check(&c_f, &r_f, &format!("gjk cold-cache ({ta},{tb}) #{i}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 49, 50 — warm cache reuse, including across a moved shape
// ---------------------------------------------------------------------------

#[test]
fn row49_row50_warm_cache() {
    let (c_f, r_f) = gjk_pair();
    let mut rng = Rng::new(0x0049);

    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for i in 0..1000 {
                let a0 = Shape::rand(&mut rng, ta, 30.0);
                let b0 = Shape::rand(&mut rng, tb, 30.0);
                let use_radius = (i % 2) as i32;
                let ax = if i % 3 == 0 { Some(rng.xform(20.0)) } else { None };

                // Pass 1: cold, both libraries, keep each library's own writeback.
                let mk = |shape_a: Shape, shape_b: Shape, cache: Option<c2GJKCache>| GjkCall {
                    a: shape_a,
                    b: shape_b,
                    ax,
                    bx: None,
                    use_radius,
                    cache,
                    want_out_a: true,
                    want_out_b: true,
                    want_iters: true,
                };

                let cold = c2GJKCache::default();
                let call1 = mk(a0, b0, Some(cold));
                call1.check(&c_f, &r_f, &format!("gjk warm p1 ({ta},{tb}) #{i}"));

                let (_, _, _, _, c_after1) = unsafe { call1.run(&c_f) };
                let (_, _, _, _, r_after1) = unsafe { call1.run(&r_f) };
                let c1 = c_after1.unwrap();
                let r1 = r_after1.unwrap();
                assert!(cache_bits_eq(&c1, &r1), "warm p1 cache diverged");

                // The written-back indices must be inside the proxy's vertex
                // range; anything else would be a C-side out-of-bounds read and
                // is excluded per ERRORS.md.
                let ok = (0..c1.count.clamp(0, 3) as usize).all(|k| {
                    c1.iA[k] >= 0
                        && c1.iA[k] <= a0.max_vert_index()
                        && c1.iB[k] >= 0
                        && c1.iB[k] <= b0.max_vert_index()
                });
                if !ok || c1.count < 0 || c1.count > 3 {
                    continue;
                }

                // Row 49: pass 2 reuses the warm cache on the same geometry.
                let call2 = mk(a0, b0, Some(c1));
                call2.check(&c_f, &r_f, &format!("gjk warm p2-same ({ta},{tb}) #{i}"));

                // Row 50: pass 2 reuses the warm cache after moving both shapes.
                let d = rng.vec(15.0);
                let call3 = mk(a0.translated(d), b0.translated(rng.vec(15.0)), Some(c1));
                call3.check(&c_f, &r_f, &format!("gjk warm p2-moved ({ta},{tb}) #{i} d={d:?}"));

                // Row 49 continued: a third chained pass (cache from pass 2).
                let (_, _, _, _, c_after2) = unsafe { call2.run(&c_f) };
                let c2c = c_after2.unwrap();
                if c2c.count >= 0
                    && c2c.count <= 3
                    && (0..c2c.count as usize).all(|k| {
                        c2c.iA[k] >= 0
                            && c2c.iA[k] <= a0.max_vert_index()
                            && c2c.iB[k] >= 0
                            && c2c.iB[k] <= b0.max_vert_index()
                    })
                {
                    let call4 = mk(a0, b0, Some(c2c));
                    call4.check(&c_f, &r_f, &format!("gjk warm p3 ({ta},{tb}) #{i}"));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 51 — out-pointer nullness combinations
// ---------------------------------------------------------------------------

#[test]
fn row51_out_pointer_nullness() {
    let (c_f, r_f) = gjk_pair();
    let mut rng = Rng::new(0x0051);

    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for i in 0..800 {
                let a = Shape::rand(&mut rng, ta, 30.0);
                let b = Shape::rand(&mut rng, tb, 30.0);
                for mask in 0u8..8 {
                    let mut call = GjkCall::simple(a, b, (i % 2) as i32);
                    call.want_out_a = mask & 1 != 0;
                    call.want_out_b = mask & 2 != 0;
                    call.want_iters = mask & 4 != 0;
                    call.check(
                        &c_f,
                        &r_f,
                        &format!("gjk outmask={mask} ({ta},{tb}) #{i}"),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 52, 53, 54, 56 — hit path, touching path, far-separated, iteration count
// ---------------------------------------------------------------------------

#[test]
fn row52_row53_row54_row56_geometric_regimes() {
    let (c_f, r_f) = gjk_pair();
    let mut rng = Rng::new(0x0052);
    let mut hit_seen = 0usize;
    let mut multi_iter_seen = 0usize;
    let mut far_seen = 0usize;

    for &ta in ALL_TYPES {
        for &tb in ALL_TYPES {
            for i in 0..1500 {
                // Row 52: force heavy overlap by placing both shapes near the
                // origin with large extents.
                let a = Shape::rand(&mut rng, ta, 5.0);
                let b = Shape::rand(&mut rng, tb, 5.0);
                let call = GjkCall::simple(a, b, (i % 2) as i32);
                call.check(&c_f, &r_f, &format!("gjk overlap ({ta},{tb}) #{i}"));
                unsafe {
                    let (d, _, _, it, _) = call.run(&c_f);
                    if d == 0.0 {
                        hit_seen += 1;
                    }
                    if it > 1 {
                        multi_iter_seen += 1;
                    }
                }

                // Row 54: far separated, large magnitudes.
                let off = 1.0e6;
                let a2 = Shape::rand(&mut rng, ta, 10.0);
                let b2 = Shape::rand(&mut rng, tb, 10.0).translated(c2v { x: off, y: -off });
                let call = GjkCall::simple(a2, b2, (i % 2) as i32);
                call.check(&c_f, &r_f, &format!("gjk far ({ta},{tb}) #{i}"));
                unsafe {
                    let (d, _, _, _, _) = call.run(&c_f);
                    if d > 1.0e5 {
                        far_seen += 1;
                    }
                }

                // Row 53: nearly-touching — offset by (rA + rB) plus a tiny
                // epsilon so `dist` lands within FLT_EPSILON of the radius sum.
                let a3 = Shape::rand(&mut rng, ta, 3.0);
                let b3 = Shape::rand(&mut rng, tb, 3.0);
                for eps in [
                    0.0f32,
                    FLT_EPSILON,
                    -FLT_EPSILON,
                    FLT_EPSILON * 0.5,
                    1.0e-30,
                    -1.0e-30,
                ] {
                    let b3t = b3.translated(c2v { x: eps, y: eps });
                    for ur in [0i32, 1] {
                        GjkCall::simple(a3, b3t, ur).check(
                            &c_f,
                            &r_f,
                            &format!("gjk touch eps={eps:e} ur={ur} ({ta},{tb}) #{i}"),
                        );
                    }
                }
            }
        }
    }
    assert!(hit_seen > 0, "row52: never reached the hit/zero-distance path");
    assert!(multi_iter_seen > 0, "row56: never exceeded 1 GJK iteration");
    assert!(far_seen > 0, "row54: never produced a large separation");
}

// ---------------------------------------------------------------------------
// Row 55 — degenerate shapes
// ---------------------------------------------------------------------------

#[test]
fn row55_degenerate_shapes() {
    let (c_f, r_f) = gjk_pair();
    let mut rng = Rng::new(0x0055);

    let zero_circle = |p: c2v| Shape::Circle(c2Circle { p, r: 0.0 });
    let point_aabb = |p: c2v| Shape::Aabb(c2AABB { min: p, max: p });
    let point_capsule = |p: c2v, r: f32| Shape::Capsule(c2Capsule { a: p, b: p, r });
    let flat_aabb = |p: c2v, w: f32| {
        Shape::Aabb(c2AABB {
            min: p,
            max: c2v { x: p.x + w, y: p.y },
        })
    };

    for i in 0..4000 {
        let p = rng.vec(20.0);
        let q = rng.vec(20.0);
        let r = rng.radius(10.0);
        let w = rng.coord(10.0);
        let degen: Vec<Shape> = vec![
            zero_circle(p),
            point_aabb(p),
            point_capsule(p, 0.0),
            point_capsule(p, r),
            flat_aabb(p, w),
            Shape::Capsule(c2Capsule { a: p, b: q, r: 0.0 }),
            Shape::Circle(c2Circle { p, r }),
            Shape::Aabb(c2AABB { min: p, max: p }),
        ];
        for a in &degen {
            for b in &degen {
                for ur in [0i32, 1] {
                    GjkCall::simple(*a, *b, ur).check(
                        &c_f,
                        &r_f,
                        &format!("gjk degenerate ur={ur} #{i}"),
                    );
                }
            }
        }
        if i > 200 {
            break; // 8*8*2 = 128 calls per iteration; 200 iterations is plenty
        }
    }
}
