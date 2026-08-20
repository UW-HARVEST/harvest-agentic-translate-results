//! Phase B, level 2 — CONFIGS.md rows B38 … B64.
//!
//! `c2GJK` is the lowest-level *composed* entry point: it runs the whole
//! Minkowski-difference pipeline (proxy construction -> simplex evolution ->
//! witness points -> radius shrink -> cache write-back).  Every row here drives
//! it exactly the way a real consumer does and compares, bit-for-bit:
//!
//!   * the returned `float` distance,
//!   * `*outA`, `*outB`, `*iterations`,
//!   * the entire 36-byte `c2GJKCache` write-back,
//!   * and the caller's input shape buffers (must be left untouched).

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_void;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Shape plumbing
// ---------------------------------------------------------------------------

/// A caller-owned shape buffer.  Larger than any shape so that an over-read by
/// the callee is at least well-defined for the test process, and so that both
/// libraries observe byte-identical memory.
#[repr(C, align(8))]
#[derive(Copy, Clone, PartialEq, Debug)]
struct Buf([u8; 32]);

#[derive(Copy, Clone, Debug)]
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn ty(&self) -> c_int {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    /// Number of vertices `c2MakeProxy` produces (bounds the legal cache index).
    fn proxy_count(&self) -> i32 {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
    fn buf(&self) -> Buf {
        // Fill with a recognizable pattern first so an over-read is at least
        // identical for both libraries.
        let mut b = Buf([0xA5; 32]);
        unsafe {
            match self {
                Shape::Circle(v) => std::ptr::copy_nonoverlapping(
                    v as *const c2Circle as *const u8,
                    b.0.as_mut_ptr(),
                    std::mem::size_of::<c2Circle>(),
                ),
                Shape::Aabb(v) => std::ptr::copy_nonoverlapping(
                    v as *const c2AABB as *const u8,
                    b.0.as_mut_ptr(),
                    std::mem::size_of::<c2AABB>(),
                ),
                Shape::Capsule(v) => std::ptr::copy_nonoverlapping(
                    v as *const c2Capsule as *const u8,
                    b.0.as_mut_ptr(),
                    std::mem::size_of::<c2Capsule>(),
                ),
            }
        }
        b
    }
}

#[derive(Copy, Clone, Debug)]
struct Cfg {
    a: Shape,
    b: Shape,
    ax: Option<c2x>,
    bx: Option<c2x>,
    use_radius: c_int,
    want_outA: bool,
    want_outB: bool,
    want_iters: bool,
    cache: Option<c2GJKCache>,
}

impl Cfg {
    fn new(a: Shape, b: Shape) -> Cfg {
        Cfg {
            a,
            b,
            ax: None,
            bx: None,
            use_radius: 0,
            want_outA: true,
            want_outB: true,
            want_iters: true,
            cache: None,
        }
    }
}

#[derive(Debug)]
struct Out {
    dist: f32,
    outA: Option<c2v>,
    outB: Option<c2v>,
    iters: Option<c_int>,
    cache: Option<c2GJKCache>,
    abuf: Buf,
    bbuf: Buf,
}

fn run(api: &Api, cfg: &Cfg) -> Out {
    let abuf = cfg.a.buf();
    let bbuf = cfg.b.buf();
    let mut oa = c2v { x: -777.5, y: 777.5 };
    let mut ob = c2v { x: 555.25, y: -555.25 };
    let mut it: c_int = -12345;
    let mut cache = cfg.cache;
    let ax = cfg.ax;
    let bx = cfg.bx;
    let dist = unsafe {
        (api.c2GJK)(
            abuf.0.as_ptr() as *const c_void,
            cfg.a.ty(),
            ax.as_ref().map_or(std::ptr::null(), |v| v as *const c2x),
            bbuf.0.as_ptr() as *const c_void,
            cfg.b.ty(),
            bx.as_ref().map_or(std::ptr::null(), |v| v as *const c2x),
            if cfg.want_outA {
                &mut oa
            } else {
                std::ptr::null_mut()
            },
            if cfg.want_outB {
                &mut ob
            } else {
                std::ptr::null_mut()
            },
            cfg.use_radius,
            if cfg.want_iters {
                &mut it
            } else {
                std::ptr::null_mut()
            },
            cache
                .as_mut()
                .map_or(std::ptr::null_mut(), |v| v as *mut c2GJKCache),
        )
    };
    Out {
        dist,
        outA: cfg.want_outA.then_some(oa),
        outB: cfg.want_outB.then_some(ob),
        iters: cfg.want_iters.then_some(it),
        cache,
        abuf,
        bbuf,
    }
}

#[track_caller]
fn differential(ctx: &str, cfg: &Cfg) -> Out {
    let (c, r) = libs();
    let oc = run(c, cfg);
    let or = run(r, cfg);
    eq_f32(&format!("{ctx} :: dist"), oc.dist, or.dist);
    match (oc.outA, or.outA) {
        (Some(a), Some(b)) => eq_v(&format!("{ctx} :: outA"), a, b),
        (None, None) => {}
        _ => panic!("{ctx}: outA presence mismatch"),
    }
    match (oc.outB, or.outB) {
        (Some(a), Some(b)) => eq_v(&format!("{ctx} :: outB"), a, b),
        (None, None) => {}
        _ => panic!("{ctx}: outB presence mismatch"),
    }
    match (oc.iters, or.iters) {
        (Some(a), Some(b)) => eq_int(&format!("{ctx} :: iterations"), a, b),
        (None, None) => {}
        _ => panic!("{ctx}: iterations presence mismatch"),
    }
    match (&oc.cache, &or.cache) {
        (Some(a), Some(b)) => eq_cache(&format!("{ctx} :: cache"), a, b),
        (None, None) => {}
        _ => panic!("{ctx}: cache presence mismatch"),
    }
    assert!(
        oc.abuf == or.abuf,
        "{ctx}: shape A buffer diverged\n C  ={:02x?}\n RUST={:02x?}",
        oc.abuf,
        or.abuf
    );
    assert!(
        oc.bbuf == or.bbuf,
        "{ctx}: shape B buffer diverged\n C  ={:02x?}\n RUST={:02x?}",
        oc.bbuf,
        or.bbuf
    );
    oc
}

// ---------------------------------------------------------------------------
// Shape generators
// ---------------------------------------------------------------------------

fn gen_shape(rng: &mut Rng, ty: c_int, range: f32) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(rng.circle(range)),
        C2_TYPE_AABB => Shape::Aabb(rng.aabb(range)),
        _ => Shape::Capsule(rng.capsule(range)),
    }
}

/// A shape whose geometry is a translate of `base` by `d`.
fn translate(s: Shape, d: c2v) -> Shape {
    let t = |v: c2v| c2v {
        x: v.x + d.x,
        y: v.y + d.y,
    };
    match s {
        Shape::Circle(c) => Shape::Circle(c2Circle { p: t(c.p), r: c.r }),
        Shape::Aabb(b) => Shape::Aabb(c2AABB {
            min: t(b.min),
            max: t(b.max),
        }),
        Shape::Capsule(c) => Shape::Capsule(c2Capsule {
            a: t(c.a),
            b: t(c.b),
            r: c.r,
        }),
    }
}

const TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];
const TYPE_NAMES: [&str; 3] = ["CIRCLE", "AABB", "CAPSULE"];
const N: usize = 400;

// ---------------------------------------------------------------------------
// B38 … B46 — the nine type pairs, no options
// ---------------------------------------------------------------------------

#[test]
fn b38_to_b46_all_type_pairs_plain() {
    let mut rng = Rng::new(0xB38);
    let mut saw_hit = 0usize;
    for (ia, &ta) in TYPES.iter().enumerate() {
        for (ib, &tb) in TYPES.iter().enumerate() {
            for i in 0..N * 4 {
                let range = [2.0f32, 20.0, 200.0][(i % 3) as usize];
                let a = gen_shape(&mut rng, ta, range);
                let b = gen_shape(&mut rng, tb, range);
                let mut cfg = Cfg::new(a, b);
                // Attach a cold cache so the final simplex count is observable.
                cfg.cache = Some(c2GJKCache::default());
                let ctx = format!(
                    "B38-46 {}x{} #{i} range={range}",
                    TYPE_NAMES[ia], TYPE_NAMES[ib]
                );
                let o = differential(&ctx, &cfg);
                if o.cache.map_or(false, |c| c.count == 3) {
                    saw_hit += 1;
                }
            }
        }
    }
    assert!(saw_hit > 0, "never reached the `count == 3` (hit) path");
    println!("B38-46: reached the hit path {saw_hit} times");
}

// ---------------------------------------------------------------------------
// B47 — use_radius = 1 for all nine pairs
// ---------------------------------------------------------------------------

#[test]
fn b47_use_radius() {
    let mut rng = Rng::new(0xB47);
    for (ia, &ta) in TYPES.iter().enumerate() {
        for (ib, &tb) in TYPES.iter().enumerate() {
            let mut zero = 0usize;
            let mut pos = 0usize;
            for i in 0..N * 4 {
                let range = [2.0f32, 20.0, 200.0][(i % 3) as usize];
                let a = gen_shape(&mut rng, ta, range);
                let b = gen_shape(&mut rng, tb, range);
                let mut cfg = Cfg::new(a, b);
                cfg.use_radius = 1;
                cfg.cache = Some(c2GJKCache::default());
                let ctx = format!("B47 {}x{} #{i}", TYPE_NAMES[ia], TYPE_NAMES[ib]);
                let o = differential(&ctx, &cfg);
                if o.dist == 0.0 {
                    zero += 1
                } else {
                    pos += 1
                }
            }
            assert!(
                zero > 0 && pos > 0,
                "{}x{}: radius path coverage incomplete (zero={zero}, pos={pos})",
                TYPE_NAMES[ia],
                TYPE_NAMES[ib]
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B48 … B52 — transform combinations
// ---------------------------------------------------------------------------

fn xform_kind(rng: &mut Rng, kind: u32) -> c2x {
    match kind {
        0 => c2x {
            p: rng.v(50.0),
            r: c2r { c: 1.0, s: 0.0 },
        }, // pure translation
        1 => {
            let t = rng.uniform(-3.15, 3.15);
            c2x {
                p: c2v { x: 0.0, y: 0.0 },
                r: c2r {
                    c: t.cos(),
                    s: t.sin(),
                },
            }
        } // pure rotation
        2 => {
            let t = rng.uniform(-3.15, 3.15);
            c2x {
                p: rng.v(50.0),
                r: c2r {
                    c: t.cos(),
                    s: t.sin(),
                },
            }
        } // general
        3 => c2x {
            p: rng.v(50.0),
            r: c2r {
                c: rng.uniform(-5.0, 5.0),
                s: rng.uniform(-5.0, 5.0),
            },
        }, // unnormalized
        _ => c2x {
            p: rng.v(1.0e18),
            r: c2r {
                c: rng.uniform(-1.0e9, 1.0e9),
                s: rng.uniform(-1.0e9, 1.0e9),
            },
        }, // huge
    }
}

#[test]
fn b48_to_b52_transforms() {
    let mut rng = Rng::new(0xB48);
    // (ax present, bx present) x xform kind
    for (which, label) in [
        (0u32, "ax-only"),
        (1, "bx-only"),
        (2, "both"),
    ] {
        for kind in 0..5u32 {
            for (ia, &ta) in TYPES.iter().enumerate() {
                for (ib, &tb) in TYPES.iter().enumerate() {
                    for i in 0..N {
                        let a = gen_shape(&mut rng, ta, 20.0);
                        let b = gen_shape(&mut rng, tb, 20.0);
                        let mut cfg = Cfg::new(a, b);
                        let x1 = xform_kind(&mut rng, kind);
                        let x2 = xform_kind(&mut rng, kind);
                        match which {
                            0 => cfg.ax = Some(x1),
                            1 => cfg.bx = Some(x2),
                            _ => {
                                cfg.ax = Some(x1);
                                cfg.bx = Some(x2);
                            }
                        }
                        cfg.use_radius = (i & 1) as c_int;
                        cfg.cache = Some(c2GJKCache::default());
                        let ctx = format!(
                            "B48-52 {label} kind={kind} {}x{} #{i}",
                            TYPE_NAMES[ia], TYPE_NAMES[ib]
                        );
                        differential(&ctx, &cfg);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B53 / B54 / B55 / B56 — controlled geometric relations
// ---------------------------------------------------------------------------

#[test]
fn b53_far_apart() {
    let mut rng = Rng::new(0xB53);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..N * 2 {
                let a = gen_shape(&mut rng, ta, 5.0);
                let base = gen_shape(&mut rng, tb, 5.0);
                // push B far away along a random axis
                let d = c2v {
                    x: rng.uniform(500.0, 5000.0) * if rng.bool() { 1.0 } else { -1.0 },
                    y: rng.uniform(500.0, 5000.0) * if rng.bool() { 1.0 } else { -1.0 },
                };
                let b = translate(base, d);
                for ur in [0, 1] {
                    let mut cfg = Cfg::new(a, b);
                    cfg.use_radius = ur;
                    cfg.cache = Some(c2GJKCache::default());
                    differential(&format!("B53 far ur={ur} #{i}"), &cfg);
                }
            }
        }
    }
}

#[test]
fn b54_deep_overlap() {
    let mut rng = Rng::new(0xB54);
    let mut hits = 0usize;
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..N * 2 {
                let a = gen_shape(&mut rng, ta, 50.0);
                let base = gen_shape(&mut rng, tb, 50.0);
                // tiny offset => almost always deeply overlapping
                let d = c2v {
                    x: rng.uniform(-1.0, 1.0),
                    y: rng.uniform(-1.0, 1.0),
                };
                let b = translate(base, d);
                for ur in [0, 1] {
                    let mut cfg = Cfg::new(a, b);
                    cfg.use_radius = ur;
                    cfg.cache = Some(c2GJKCache::default());
                    let o = differential(&format!("B54 overlap ur={ur} #{i}"), &cfg);
                    if o.cache.map_or(false, |c| c.count == 3) {
                        hits += 1;
                    }
                }
            }
        }
    }
    assert!(hits > 0, "deep-overlap set never produced a `hit`");
    println!("B54: {hits} hit-path cases");
}

#[test]
fn b55_exactly_touching() {
    let mut rng = Rng::new(0xB55);
    // Exact tangency, built from integers so the arithmetic is exact in f32.
    for i in 0..N * 4 {
        let r1 = (rng.below(10) + 1) as f32;
        let r2 = (rng.below(10) + 1) as f32;
        let cx = (rng.below(41) as f32) - 20.0;
        let cy = (rng.below(41) as f32) - 20.0;
        let a = Shape::Circle(c2Circle {
            p: c2v { x: cx, y: cy },
            r: r1,
        });
        // exactly r1+r2 apart along x, and along y
        for (dx, dy) in [(r1 + r2, 0.0f32), (0.0, r1 + r2), (-(r1 + r2), 0.0)] {
            let b = Shape::Circle(c2Circle {
                p: c2v {
                    x: cx + dx,
                    y: cy + dy,
                },
                r: r2,
            });
            for ur in [0, 1] {
                let mut cfg = Cfg::new(a, b);
                cfg.use_radius = ur;
                cfg.cache = Some(c2GJKCache::default());
                differential(&format!("B55 tangent circles ur={ur} #{i}"), &cfg);
            }
        }
        // AABBs sharing an edge exactly
        let bb1 = c2AABB {
            min: c2v { x: cx, y: cy },
            max: c2v {
                x: cx + r1,
                y: cy + r1,
            },
        };
        let bb2 = c2AABB {
            min: c2v {
                x: cx + r1,
                y: cy,
            },
            max: c2v {
                x: cx + r1 + r2,
                y: cy + r1,
            },
        };
        for ur in [0, 1] {
            let mut cfg = Cfg::new(Shape::Aabb(bb1), Shape::Aabb(bb2));
            cfg.use_radius = ur;
            cfg.cache = Some(c2GJKCache::default());
            differential(&format!("B55 edge-sharing aabb ur={ur} #{i}"), &cfg);
        }
        // Capsules whose end caps touch exactly
        let cap1 = c2Capsule {
            a: c2v { x: cx, y: cy },
            b: c2v { x: cx + 4.0, y: cy },
            r: r1,
        };
        let cap2 = c2Capsule {
            a: c2v {
                x: cx + 4.0 + r1 + r2,
                y: cy,
            },
            b: c2v {
                x: cx + 8.0 + r1 + r2,
                y: cy,
            },
            r: r2,
        };
        for ur in [0, 1] {
            let mut cfg = Cfg::new(Shape::Capsule(cap1), Shape::Capsule(cap2));
            cfg.use_radius = ur;
            cfg.cache = Some(c2GJKCache::default());
            differential(&format!("B55 tangent capsules ur={ur} #{i}"), &cfg);
        }
    }
}

#[test]
fn b56_identical_shapes() {
    let mut rng = Rng::new(0xB56);
    for &ta in TYPES.iter() {
        for i in 0..N * 4 {
            let a = gen_shape(&mut rng, ta, 30.0);
            for ur in [0, 1] {
                let mut cfg = Cfg::new(a, a);
                cfg.use_radius = ur;
                cfg.cache = Some(c2GJKCache::default());
                differential(&format!("B56 identical ur={ur} #{i}"), &cfg);
                // identical + identical transform
                let x = xform_kind(&mut rng, 2);
                let mut cfg2 = Cfg::new(a, a);
                cfg2.use_radius = ur;
                cfg2.ax = Some(x);
                cfg2.bx = Some(x);
                cfg2.cache = Some(c2GJKCache::default());
                differential(&format!("B56 identical+xform ur={ur} #{i}"), &cfg2);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B57 … B60 — cache states
// ---------------------------------------------------------------------------

#[test]
fn b57_cold_cache() {
    let mut rng = Rng::new(0xB57);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..N {
                let a = gen_shape(&mut rng, ta, 20.0);
                let b = gen_shape(&mut rng, tb, 20.0);
                let mut cfg = Cfg::new(a, b);
                cfg.use_radius = (i & 1) as c_int;
                // count == 0 but *garbage* metric/div/indices: the C must treat
                // the cache as "not good" and cold-start regardless.
                cfg.cache = Some(c2GJKCache {
                    metric: rng.wild_f32(),
                    count: 0,
                    iA: [7, -3, 99],
                    iB: [-1, 4, 0],
                    div: rng.wild_f32(),
                });
                differential(&format!("B57 cold #{i}"), &cfg);
            }
        }
    }
}

#[test]
fn b58_b59_b60_warm_cache() {
    let mut rng = Rng::new(0xB58);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for count in 1..=3i32 {
                for i in 0..N {
                    let a = gen_shape(&mut rng, ta, 20.0);
                    let b = gen_shape(&mut rng, tb, 20.0);
                    let na = a.proxy_count();
                    let nb = b.proxy_count();
                    let mut iA = [0i32; 3];
                    let mut iB = [0i32; 3];
                    for k in 0..3 {
                        iA[k] = rng.below(na as u32) as i32;
                        iB[k] = rng.below(nb as u32) as i32;
                    }
                    let mut cfg = Cfg::new(a, b);
                    cfg.use_radius = (i & 1) as c_int;
                    cfg.cache = Some(c2GJKCache {
                        metric: match rng.below(4) {
                            0 => 0.0,
                            1 => rng.uniform(-100.0, 100.0),
                            2 => -1.0e9, // pushes the metric guard
                            _ => rng.uniform(0.0, 1.0e6),
                        },
                        count,
                        iA,
                        iB,
                        div: match rng.below(4) {
                            0 => 1.0,
                            1 => 0.0,
                            2 => rng.uniform(-10.0, 10.0),
                            _ => rng.uniform(0.001, 100.0),
                        },
                    });
                    differential(&format!("B58-60 warm count={count} #{i}"), &cfg);
                }
            }
        }
    }
}

#[test]
fn b61_cache_feedback_loop() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xB61);
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for round in 0..N {
                let mut a = gen_shape(&mut rng, ta, 20.0);
                let mut b = gen_shape(&mut rng, tb, 20.0);
                let mut cache_c = c2GJKCache::default();
                let mut cache_r = c2GJKCache::default();
                let use_radius = (round & 1) as c_int;
                for step in 0..8 {
                    let mut cfg_c = Cfg::new(a, b);
                    cfg_c.use_radius = use_radius;
                    cfg_c.cache = Some(cache_c);
                    let mut cfg_r = cfg_c;
                    cfg_r.cache = Some(cache_r);

                    let oc = run(c, &cfg_c);
                    let or = run(r, &cfg_r);
                    let ctx = format!("B61 feedback round={round} step={step}");
                    eq_f32(&format!("{ctx} dist"), oc.dist, or.dist);
                    eq_v(&format!("{ctx} outA"), oc.outA.unwrap(), or.outA.unwrap());
                    eq_v(&format!("{ctx} outB"), oc.outB.unwrap(), or.outB.unwrap());
                    eq_int(&format!("{ctx} iters"), oc.iters.unwrap(), or.iters.unwrap());
                    eq_cache(
                        &format!("{ctx} cache"),
                        oc.cache.as_ref().unwrap(),
                        or.cache.as_ref().unwrap(),
                    );
                    cache_c = oc.cache.unwrap();
                    cache_r = or.cache.unwrap();
                    // perturb the shapes, as a physics step would
                    let d = c2v {
                        x: rng.uniform(-3.0, 3.0),
                        y: rng.uniform(-3.0, 3.0),
                    };
                    a = translate(a, d);
                    let d2 = c2v {
                        x: rng.uniform(-3.0, 3.0),
                        y: rng.uniform(-3.0, 3.0),
                    };
                    b = translate(b, d2);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B62 — the full 64-way NULL / non-NULL matrix
// ---------------------------------------------------------------------------

#[test]
fn b62_null_optional_pointer_matrix() {
    let mut rng = Rng::new(0xB62);
    for mask in 0u32..64 {
        for &ta in TYPES.iter() {
            for &tb in TYPES.iter() {
                for i in 0..24 {
                    let a = gen_shape(&mut rng, ta, 20.0);
                    let b = gen_shape(&mut rng, tb, 20.0);
                    let mut cfg = Cfg::new(a, b);
                    cfg.want_outA = mask & 1 != 0;
                    cfg.want_outB = mask & 2 != 0;
                    cfg.want_iters = mask & 4 != 0;
                    cfg.ax = (mask & 8 != 0).then(|| xform_kind(&mut rng, 2));
                    cfg.bx = (mask & 16 != 0).then(|| xform_kind(&mut rng, 2));
                    cfg.cache = (mask & 32 != 0).then(c2GJKCache::default);
                    cfg.use_radius = (i & 1) as c_int;
                    differential(&format!("B62 mask={mask:06b} #{i}"), &cfg);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B63 — degenerate proxies
// ---------------------------------------------------------------------------

#[test]
fn b63_degenerate_shapes() {
    let mut rng = Rng::new(0xB63);
    let degenerates: Vec<Shape> = vec![
        Shape::Circle(c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        }),
        Shape::Circle(c2Circle {
            p: c2v { x: 3.0, y: -4.0 },
            r: -5.0,
        }),
        Shape::Aabb(c2AABB {
            min: c2v { x: 1.0, y: 1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        }),
        Shape::Aabb(c2AABB {
            min: c2v { x: 5.0, y: 5.0 },
            max: c2v { x: -5.0, y: -5.0 },
        }), // inverted
        Shape::Aabb(c2AABB {
            min: c2v { x: -2.0, y: 3.0 },
            max: c2v { x: 2.0, y: 3.0 },
        }), // zero height
        Shape::Capsule(c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        }),
        Shape::Capsule(c2Capsule {
            a: c2v { x: 2.0, y: 2.0 },
            b: c2v { x: 2.0, y: 2.0 },
            r: 3.0,
        }),
        Shape::Capsule(c2Capsule {
            a: c2v { x: -1.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: -1.0,
        }),
    ];
    for (i, &a) in degenerates.iter().enumerate() {
        for (j, &b) in degenerates.iter().enumerate() {
            for ur in [0, 1] {
                for k in 0..8 {
                    let mut cfg = Cfg::new(a, b);
                    cfg.use_radius = ur;
                    cfg.cache = Some(c2GJKCache::default());
                    if k & 1 != 0 {
                        cfg.ax = Some(xform_kind(&mut rng, k % 5));
                    }
                    if k & 2 != 0 {
                        cfg.bx = Some(xform_kind(&mut rng, (k + 1) % 5));
                    }
                    differential(&format!("B63 degen {i}x{j} ur={ur} k={k}"), &cfg);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B64 — huge coordinates: overflow to inf/NaN inside the simplex
// ---------------------------------------------------------------------------

#[test]
fn b64_huge_coordinates() {
    let mut rng = Rng::new(0xB64);
    let mags = [1.0e18f32, 1.0e30, FLT_MAX, 1.0e-38];
    for &ta in TYPES.iter() {
        for &tb in TYPES.iter() {
            for i in 0..N * 2 {
                let m1 = mags[rng.below(4) as usize];
                let m2 = mags[rng.below(4) as usize];
                let a = match ta {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                        p: c2v {
                            x: rng.uniform(-1.0, 1.0) * m1,
                            y: rng.uniform(-1.0, 1.0) * m1,
                        },
                        r: rng.uniform(0.0, 1.0) * m1,
                    }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: c2v { x: -m1, y: -m1 },
                        max: c2v { x: m1, y: m1 },
                    }),
                    _ => Shape::Capsule(c2Capsule {
                        a: c2v { x: -m1, y: 0.0 },
                        b: c2v { x: m1, y: m1 },
                        r: m1,
                    }),
                };
                let b = match tb {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                        p: c2v {
                            x: rng.uniform(-1.0, 1.0) * m2,
                            y: rng.uniform(-1.0, 1.0) * m2,
                        },
                        r: rng.uniform(0.0, 1.0) * m2,
                    }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: c2v { x: -m2, y: -m2 },
                        max: c2v { x: m2, y: m2 },
                    }),
                    _ => Shape::Capsule(c2Capsule {
                        a: c2v { x: 0.0, y: -m2 },
                        b: c2v { x: m2, y: m2 },
                        r: m2,
                    }),
                };
                for ur in [0, 1] {
                    let mut cfg = Cfg::new(a, b);
                    cfg.use_radius = ur;
                    cfg.cache = Some(c2GJKCache::default());
                    differential(&format!("B64 huge ur={ur} #{i}"), &cfg);
                }
            }
        }
    }
}
