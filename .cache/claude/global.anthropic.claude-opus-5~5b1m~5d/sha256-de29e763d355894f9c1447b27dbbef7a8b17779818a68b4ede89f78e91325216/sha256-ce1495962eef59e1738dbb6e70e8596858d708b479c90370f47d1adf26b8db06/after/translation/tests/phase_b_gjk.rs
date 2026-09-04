//! Phase B — valid-path differential tests, rows C48..C83 of CONFIGS.md
//! (the full `c2GJK` pipeline with every transform / use_radius / cache /
//! out-param combination, the boolean collision routines, `c2Collided`'s 3x3
//! dispatch matrix and the `capsule` entry point).

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 1500;

// ---------------------------------------------------------------------------
// Shape plumbing: a `void*` + C2_TYPE pair, as the C API demands
// ---------------------------------------------------------------------------

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
    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const _ as *const c_void,
            Shape::Aabb(c) => c as *const _ as *const c_void,
            Shape::Capsule(c) => c as *const _ as *const c_void,
        }
    }
    /// Number of vertices `c2MakeProxy` fills in for this shape.
    fn nverts(&self) -> c_int {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
}

fn gen_shape(rng: &mut Rng, kind: u32) -> Shape {
    match kind {
        0 => Shape::Circle(rng.circle()),
        1 => Shape::Aabb(rng.aabb()),
        _ => Shape::Capsule(rng.capsule()),
    }
}

/// One fully specified `c2GJK` invocation, run against both libraries and
/// compared field by field (return value, both witness points, the iteration
/// counter and the written-back cache).
#[derive(Copy, Clone)]
struct GjkCfg {
    ax: Option<c2x>,
    bx: Option<c2x>,
    use_radius: c_int,
    want_out_a: bool,
    want_out_b: bool,
    want_iters: bool,
    cache: Option<c2GJKCache>,
}

impl Default for GjkCfg {
    fn default() -> Self {
        GjkCfg {
            ax: None,
            bx: None,
            use_radius: 1,
            want_out_a: true,
            want_out_b: true,
            want_iters: true,
            cache: None,
        }
    }
}

#[track_caller]
fn run_gjk(ctx: &str, a: &Shape, b: &Shape, cfg: GjkCfg) -> (f32, Option<c2GJKCache>) {
    let l = libs();
    let sentinel = c2v {
        x: -424242.0,
        y: 424242.0,
    };
    let mut oac = sentinel;
    let mut obc = sentinel;
    let mut itc: c_int = -777;
    let mut oar = sentinel;
    let mut obr = sentinel;
    let mut itr: c_int = -777;
    let mut cc = cfg.cache;
    let mut cr = cfg.cache;

    let axc = cfg.ax;
    let bxc = cfg.bx;

    let (dc, dr) = unsafe {
        let dc = (l.c.c2GJK)(
            a.ptr(),
            a.ty(),
            axc.as_ref().map_or(std::ptr::null(), |x| x as *const c2x),
            b.ptr(),
            b.ty(),
            bxc.as_ref().map_or(std::ptr::null(), |x| x as *const c2x),
            if cfg.want_out_a { &mut oac } else { std::ptr::null_mut() },
            if cfg.want_out_b { &mut obc } else { std::ptr::null_mut() },
            cfg.use_radius,
            if cfg.want_iters { &mut itc } else { std::ptr::null_mut() },
            cc.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut c2GJKCache),
        );
        let dr = (l.r.c2GJK)(
            a.ptr(),
            a.ty(),
            axc.as_ref().map_or(std::ptr::null(), |x| x as *const c2x),
            b.ptr(),
            b.ty(),
            bxc.as_ref().map_or(std::ptr::null(), |x| x as *const c2x),
            if cfg.want_out_a { &mut oar } else { std::ptr::null_mut() },
            if cfg.want_out_b { &mut obr } else { std::ptr::null_mut() },
            cfg.use_radius,
            if cfg.want_iters { &mut itr } else { std::ptr::null_mut() },
            cr.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut c2GJKCache),
        );
        (dc, dr)
    };

    eq_f32(&format!("{ctx} dist"), dc, dr);
    eq_v(&format!("{ctx} outA"), oac, oar);
    eq_v(&format!("{ctx} outB"), obc, obr);
    eq_i(&format!("{ctx} iterations"), itc, itr);
    match (cc, cr) {
        (Some(x), Some(y)) => eq_cache(&format!("{ctx} cache"), &x, &y),
        (None, None) => {}
        _ => panic!("{ctx}: cache presence mismatch"),
    }
    (dc, cc)
}

// ---------------------------------------------------------------------------
// C48..C55 — every type pair x use_radius, no transforms, no cache
// ---------------------------------------------------------------------------
#[test]
fn c48_to_c55_type_pairs() {
    let mut rng = Rng::new(0x48);
    for ka in 0..3u32 {
        for kb in 0..3u32 {
            for &ur in &[0i32, 1] {
                for i in 0..N {
                    let a = gen_shape(&mut rng, ka);
                    let b = gen_shape(&mut rng, kb);
                    run_gjk(
                        &format!("C48-55 pair({ka},{kb}) ur={ur} #{i}"),
                        &a,
                        &b,
                        GjkCfg {
                            use_radius: ur,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C56/C57/C58/C59 — transforms
// ---------------------------------------------------------------------------
#[test]
fn c56_to_c59_transforms() {
    let mut rng = Rng::new(0x56);
    let idr = c2r { c: 1.0, s: 0.0 };
    for ka in 0..3u32 {
        for kb in 0..3u32 {
            for mode in 0..4u32 {
                for &ur in &[0i32, 1] {
                    for i in 0..(N / 2) {
                        let a = gen_shape(&mut rng, ka);
                        let b = gen_shape(&mut rng, kb);
                        // translation-only / rotation-only / full / non-normalised
                        let trans = c2x {
                            p: rng.v(),
                            r: idr,
                        };
                        let rot = c2x {
                            p: c2v { x: 0.0, y: 0.0 },
                            r: rng.rot(),
                        };
                        let full = rng.x();
                        let weird = c2x {
                            p: rng.v(),
                            r: c2r {
                                c: rng.range(3.0),
                                s: rng.range(3.0),
                            },
                        };
                        let (ax, bx) = match mode {
                            0 => (Some(trans), None),
                            1 => (None, Some(rot)),
                            2 => (Some(full), Some(rng.x())),
                            _ => (Some(weird), Some(weird)),
                        };
                        run_gjk(
                            &format!("C56-59 pair({ka},{kb}) mode={mode} ur={ur} #{i}"),
                            &a,
                            &b,
                            GjkCfg {
                                ax,
                                bx,
                                use_radius: ur,
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C60..C63 — cold and warm caches
// ---------------------------------------------------------------------------
#[test]
fn c60_to_c63_caches() {
    let mut rng = Rng::new(0x60);
    for ka in 0..3u32 {
        for kb in 0..3u32 {
            for ccount in 0..4i32 {
                for &ur in &[0i32, 1] {
                    for i in 0..(N / 2) {
                        let a = gen_shape(&mut rng, ka);
                        let b = gen_shape(&mut rng, kb);
                        let na = a.nverts();
                        let nb = b.nverts();
                        let mut cache = c2GJKCache {
                            metric: if ccount == 0 { 0.0 } else { rng.coord() },
                            count: ccount,
                            iA: [0; 3],
                            iB: [0; 3],
                            div: if ccount == 0 { 0.0 } else { rng.coord() },
                        };
                        for k in 0..3 {
                            // indices must stay inside the proxy's filled range
                            cache.iA[k] = (rng.below(na as u32)) as c_int;
                            cache.iB[k] = (rng.below(nb as u32)) as c_int;
                        }
                        run_gjk(
                            &format!("C60-63 pair({ka},{kb}) ccount={ccount} ur={ur} #{i}"),
                            &a,
                            &b,
                            GjkCfg {
                                use_radius: ur,
                                cache: Some(cache),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C64 — cache round-trip across consecutive calls
// ---------------------------------------------------------------------------
#[test]
fn c64_cache_roundtrip() {
    let l = libs();
    let mut rng = Rng::new(0x64);
    for ka in 0..3u32 {
        for kb in 0..3u32 {
            for i in 0..N {
                let a0 = gen_shape(&mut rng, ka);
                let b0 = gen_shape(&mut rng, kb);
                let mut cc = c2GJKCache::default();
                let mut cr = c2GJKCache::default();
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let mut ar = c2v::default();
                let mut br = c2v::default();
                let mut ic: c_int = 0;
                let mut ir: c_int = 0;
                // three consecutive calls: cold -> warm -> warm with a moved B
                let shapes = [
                    (a0, b0),
                    (a0, b0),
                    (a0, gen_shape(&mut rng, kb)),
                ];
                for (step, (a, b)) in shapes.iter().enumerate() {
                    let ctx = format!("C64 pair({ka},{kb}) #{i} step{step}");
                    unsafe {
                        let dc = (l.c.c2GJK)(
                            a.ptr(),
                            a.ty(),
                            std::ptr::null(),
                            b.ptr(),
                            b.ty(),
                            std::ptr::null(),
                            &mut ac,
                            &mut bc,
                            1,
                            &mut ic,
                            &mut cc,
                        );
                        let dr = (l.r.c2GJK)(
                            a.ptr(),
                            a.ty(),
                            std::ptr::null(),
                            b.ptr(),
                            b.ty(),
                            std::ptr::null(),
                            &mut ar,
                            &mut br,
                            1,
                            &mut ir,
                            &mut cr,
                        );
                        eq_f32(&format!("{ctx} dist"), dc, dr);
                    }
                    eq_v(&format!("{ctx} outA"), ac, ar);
                    eq_v(&format!("{ctx} outB"), bc, br);
                    eq_i(&format!("{ctx} iters"), ic, ir);
                    eq_cache(&format!("{ctx} cache"), &cc, &cr);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C65 — selectively NULL out-params
// ---------------------------------------------------------------------------
#[test]
fn c65_null_outparams() {
    let mut rng = Rng::new(0x65);
    for ka in 0..3u32 {
        for kb in 0..3u32 {
            for mask in 0..8u32 {
                for i in 0..(N / 4) {
                    let a = gen_shape(&mut rng, ka);
                    let b = gen_shape(&mut rng, kb);
                    run_gjk(
                        &format!("C65 pair({ka},{kb}) mask={mask} #{i}"),
                        &a,
                        &b,
                        GjkCfg {
                            want_out_a: mask & 1 != 0,
                            want_out_b: mask & 2 != 0,
                            want_iters: mask & 4 != 0,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C66..C71 — geometric relations and degenerate shapes
// ---------------------------------------------------------------------------
#[test]
fn c66_to_c71_relations() {
    let mut rng = Rng::new(0x66);

    // deep overlap (hit == 1) and coincidence
    for i in 0..N {
        let p = rng.v();
        let r = 1.0 + rng.unit() * 20.0;
        let sets: Vec<(Shape, Shape)> = vec![
            (
                Shape::Circle(c2Circle { p, r }),
                Shape::Circle(c2Circle { p, r }),
            ),
            (
                Shape::Aabb(c2AABB {
                    min: c2v { x: p.x - r, y: p.y - r },
                    max: c2v { x: p.x + r, y: p.y + r },
                }),
                Shape::Aabb(c2AABB {
                    min: c2v { x: p.x - r * 0.5, y: p.y - r * 0.5 },
                    max: c2v { x: p.x + r * 0.5, y: p.y + r * 0.5 },
                }),
            ),
            (
                Shape::Capsule(c2Capsule {
                    a: p,
                    b: c2v { x: p.x + r, y: p.y },
                    r,
                }),
                Shape::Capsule(c2Capsule {
                    a: c2v { x: p.x, y: p.y + 0.1 },
                    b: c2v { x: p.x + r, y: p.y + 0.1 },
                    r,
                }),
            ),
            // fully contained
            (
                Shape::Circle(c2Circle { p, r: r * 4.0 }),
                Shape::Circle(c2Circle { p: c2v { x: p.x + 0.25, y: p.y }, r: 0.1 }),
            ),
        ];
        for (k, (a, b)) in sets.iter().enumerate() {
            for &ur in &[0i32, 1] {
                run_gjk(&format!("C66/C68 overlap set{k} ur={ur} #{i}"), a, b, GjkCfg { use_radius: ur, ..Default::default() });
            }
        }
    }

    // exactly touching: circle centres separated by exactly rA+rB
    for i in 0..N {
        let p = rng.v();
        let ra = rng.unit() * 10.0;
        let rb = rng.unit() * 10.0;
        let a = Shape::Circle(c2Circle { p, r: ra });
        let b = Shape::Circle(c2Circle {
            p: c2v { x: p.x + ra + rb, y: p.y },
            r: rb,
        });
        for &ur in &[0i32, 1] {
            run_gjk(&format!("C67 touching ur={ur} #{i}"), &a, &b, GjkCfg { use_radius: ur, ..Default::default() });
        }
    }

    // far separated / huge magnitudes
    for i in 0..N {
        let s = [1.0e3f32, 1.0e6, 1.0e12, 1.0e20, 1.0e30][(i % 5) as usize];
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                let mut a = gen_shape(&mut rng, ka);
                let mut b = gen_shape(&mut rng, kb);
                // push B far away along +x
                match &mut b {
                    Shape::Circle(c) => c.p.x += s,
                    Shape::Aabb(c) => {
                        c.min.x += s;
                        c.max.x += s;
                    }
                    Shape::Capsule(c) => {
                        c.a.x += s;
                        c.b.x += s;
                    }
                }
                let _ = &mut a;
                for &ur in &[0i32, 1] {
                    run_gjk(&format!("C69 far s={s} ({ka},{kb}) ur={ur} #{i}"), &a, &b, GjkCfg { use_radius: ur, ..Default::default() });
                }
            }
        }
    }

    // C70 — degenerate shapes: zero radius / zero length / zero area
    for i in 0..N {
        let p = rng.v();
        let q = rng.v();
        let degen: Vec<Shape> = vec![
            Shape::Circle(c2Circle { p, r: 0.0 }),
            Shape::Capsule(c2Capsule { a: p, b: p, r: 0.0 }),
            Shape::Capsule(c2Capsule { a: p, b: p, r: 5.0 }),
            Shape::Aabb(c2AABB { min: p, max: p }),
            Shape::Aabb(c2AABB { min: p, max: c2v { x: p.x, y: q.y } }),
        ];
        for (ja, a) in degen.iter().enumerate() {
            for (jb, b) in degen.iter().enumerate() {
                for &ur in &[0i32, 1] {
                    run_gjk(&format!("C70 degen({ja},{jb}) ur={ur} #{i}"), a, b, GjkCfg { use_radius: ur, ..Default::default() });
                }
            }
        }
    }

    // C71 — negative radii
    for i in 0..N {
        let p = rng.v();
        let q = rng.v();
        let neg: Vec<Shape> = vec![
            Shape::Circle(c2Circle { p, r: -rng.unit() * 20.0 }),
            Shape::Capsule(c2Capsule { a: p, b: q, r: -rng.unit() * 20.0 }),
            Shape::Circle(c2Circle { p: q, r: rng.unit() * 20.0 }),
        ];
        for (ja, a) in neg.iter().enumerate() {
            for (jb, b) in neg.iter().enumerate() {
                for &ur in &[0i32, 1] {
                    run_gjk(&format!("C71 neg({ja},{jb}) ur={ur} #{i}"), a, b, GjkCfg { use_radius: ur, ..Default::default() });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C72 — inf / NaN coordinates through the whole GJK pipeline
// ---------------------------------------------------------------------------
#[test]
fn c72_gjk_specials() {
    let mut rng = Rng::new(0x72);
    for ka in 0..3u32 {
        for kb in 0..3u32 {
            for i in 0..N {
                let wild = |rng: &mut Rng, k: u32| -> Shape {
                    match k {
                        0 => Shape::Circle(c2Circle {
                            p: rng.wild_v(),
                            r: rng.wild(),
                        }),
                        1 => Shape::Aabb(c2AABB {
                            min: rng.wild_v(),
                            max: rng.wild_v(),
                        }),
                        _ => Shape::Capsule(c2Capsule {
                            a: rng.wild_v(),
                            b: rng.wild_v(),
                            r: rng.wild(),
                        }),
                    }
                };
                let a = wild(&mut rng, ka);
                let b = wild(&mut rng, kb);
                for &ur in &[0i32, 1] {
                    run_gjk(
                        &format!("C72 wild({ka},{kb}) ur={ur} #{i}"),
                        &a,
                        &b,
                        GjkCfg {
                            use_radius: ur,
                            ..Default::default()
                        },
                    );
                }
                // ... and with wild transforms on top
                let ax = c2x {
                    p: rng.wild_v(),
                    r: c2r {
                        c: rng.wild(),
                        s: rng.wild(),
                    },
                };
                run_gjk(
                    &format!("C72 wild-x({ka},{kb}) #{i}"),
                    &a,
                    &b,
                    GjkCfg {
                        ax: Some(ax),
                        bx: Some(ax),
                        ..Default::default()
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C73 — c2AABBtoAABB
// ---------------------------------------------------------------------------
#[test]
fn c73_aabb_to_aabb() {
    let l = libs();
    let mut rng = Rng::new(0x73);
    unsafe {
        for _ in 0..(N * 8) {
            let a = rng.aabb();
            let b = rng.aabb();
            eq_i("C73 random", (l.c.c2AABBtoAABB)(a, b), (l.r.c2AABBtoAABB)(a, b));
            eq_i("C73 swapped", (l.c.c2AABBtoAABB)(b, a), (l.r.c2AABBtoAABB)(b, a));
            eq_i("C73 self", (l.c.c2AABBtoAABB)(a, a), (l.r.c2AABBtoAABB)(a, a));
        }
        // exactly touching on each axis, nested, ±0 edges, inf/NaN
        for _ in 0..N {
            let p = rng.v();
            let w = rng.unit() * 10.0;
            let base = c2AABB {
                min: p,
                max: c2v { x: p.x + w, y: p.y + w },
            };
            for shift in [
                c2v { x: w, y: 0.0 },
                c2v { x: 0.0, y: w },
                c2v { x: -w, y: 0.0 },
                c2v { x: 0.0, y: -w },
                c2v { x: 0.0, y: 0.0 },
                c2v { x: w * 0.5, y: w * 0.5 },
                c2v { x: w * 2.0, y: 0.0 },
            ] {
                let b = c2AABB {
                    min: c2v { x: base.min.x + shift.x, y: base.min.y + shift.y },
                    max: c2v { x: base.max.x + shift.x, y: base.max.y + shift.y },
                };
                eq_i("C73 touching", (l.c.c2AABBtoAABB)(base, b), (l.r.c2AABBtoAABB)(base, b));
            }
            let wa = c2AABB { min: rng.wild_v(), max: rng.wild_v() };
            let wb = c2AABB { min: rng.wild_v(), max: rng.wild_v() };
            eq_i("C73 wild", (l.c.c2AABBtoAABB)(wa, wb), (l.r.c2AABBtoAABB)(wa, wb));
        }
    }
}

// ---------------------------------------------------------------------------
// C74 — c2CircletoCircle
// ---------------------------------------------------------------------------
#[test]
fn c74_circle_to_circle() {
    let l = libs();
    let mut rng = Rng::new(0x74);
    unsafe {
        for _ in 0..(N * 8) {
            let a = rng.circle();
            let b = rng.circle();
            eq_i("C74 random", (l.c.c2CircletoCircle)(a, b), (l.r.c2CircletoCircle)(a, b));
            eq_i("C74 swapped", (l.c.c2CircletoCircle)(b, a), (l.r.c2CircletoCircle)(b, a));
        }
        for _ in 0..N {
            let p = rng.v();
            let ra = rng.unit() * 10.0;
            let rb = rng.unit() * 10.0;
            for k in [0.0f32, 0.5, 1.0, 1.0000001, 2.0] {
                let a = c2Circle { p, r: ra };
                let b = c2Circle {
                    p: c2v { x: p.x + (ra + rb) * k, y: p.y },
                    r: rb,
                };
                eq_i("C74 touching", (l.c.c2CircletoCircle)(a, b), (l.r.c2CircletoCircle)(a, b));
            }
            // zero / negative radii, concentric, wild
            let z = c2Circle { p, r: 0.0 };
            let n = c2Circle { p, r: -ra };
            eq_i("C74 zero", (l.c.c2CircletoCircle)(z, z), (l.r.c2CircletoCircle)(z, z));
            eq_i("C74 neg", (l.c.c2CircletoCircle)(n, n), (l.r.c2CircletoCircle)(n, n));
            let wa = c2Circle { p: rng.wild_v(), r: rng.wild() };
            let wb = c2Circle { p: rng.wild_v(), r: rng.wild() };
            eq_i("C74 wild", (l.c.c2CircletoCircle)(wa, wb), (l.r.c2CircletoCircle)(wa, wb));
        }
    }
}

// ---------------------------------------------------------------------------
// C75 — c2CircletoAABB (all 9 Voronoi regions of the box)
// ---------------------------------------------------------------------------
#[test]
fn c75_circle_to_aabb() {
    let l = libs();
    let mut rng = Rng::new(0x75);
    unsafe {
        for _ in 0..(N * 8) {
            let a = rng.circle();
            let b = rng.aabb();
            eq_i("C75 random", (l.c.c2CircletoAABB)(a, b), (l.r.c2CircletoAABB)(a, b));
        }
        for _ in 0..N {
            let p = rng.v();
            let w = 1.0 + rng.unit() * 10.0;
            let bb = c2AABB {
                min: p,
                max: c2v { x: p.x + w, y: p.y + w },
            };
            let r = rng.unit() * w;
            for dx in [-1.5f32, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5] {
                for dy in [-1.5f32, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5] {
                    let c = c2Circle {
                        p: c2v { x: p.x + w * dx, y: p.y + w * dy },
                        r,
                    };
                    eq_i("C75 regions", (l.c.c2CircletoAABB)(c, bb), (l.r.c2CircletoAABB)(c, bb));
                    let zr = c2Circle { p: c.p, r: 0.0 };
                    eq_i("C75 zero r", (l.c.c2CircletoAABB)(zr, bb), (l.r.c2CircletoAABB)(zr, bb));
                }
            }
            // inverted box
            let inv = c2AABB { min: bb.max, max: bb.min };
            let c = rng.circle();
            eq_i("C75 inverted", (l.c.c2CircletoAABB)(c, inv), (l.r.c2CircletoAABB)(c, inv));
            let wc = c2Circle { p: rng.wild_v(), r: rng.wild() };
            let wb = c2AABB { min: rng.wild_v(), max: rng.wild_v() };
            eq_i("C75 wild", (l.c.c2CircletoAABB)(wc, wb), (l.r.c2CircletoAABB)(wc, wb));
        }
    }
}

// ---------------------------------------------------------------------------
// C76 — c2CircletoCapsule (all three segment regions)
// ---------------------------------------------------------------------------
#[test]
fn c76_circle_to_capsule() {
    let l = libs();
    let mut rng = Rng::new(0x76);
    unsafe {
        for _ in 0..(N * 8) {
            let a = rng.circle();
            let b = rng.capsule();
            eq_i("C76 random", (l.c.c2CircletoCapsule)(a, b), (l.r.c2CircletoCapsule)(a, b));
        }
        for _ in 0..N {
            let a0 = rng.v();
            let dir = rng.v();
            let b0 = c2v { x: a0.x + dir.x, y: a0.y + dir.y };
            let cap = c2Capsule { a: a0, b: b0, r: rng.unit() * 5.0 };
            // t < 0 (before A), 0..1 (mid), > 1 (past B), plus perpendicular offsets
            for t in [-1.0f32, -0.25, 0.0, 0.5, 1.0, 1.25, 2.0] {
                for off in [0.0f32, 0.5, 2.0, -1.0] {
                    let perp = c2v { x: -dir.y, y: dir.x };
                    let c = c2Circle {
                        p: c2v {
                            x: a0.x + dir.x * t + perp.x * off,
                            y: a0.y + dir.y * t + perp.y * off,
                        },
                        r: rng.unit() * 5.0,
                    };
                    eq_i("C76 regions", (l.c.c2CircletoCapsule)(c, cap), (l.r.c2CircletoCapsule)(c, cap));
                }
            }
            // zero-length capsule (dot(n,n) == 0)
            let deg = c2Capsule { a: a0, b: a0, r: rng.unit() * 5.0 };
            let c = rng.circle();
            eq_i("C76 zero-length", (l.c.c2CircletoCapsule)(c, deg), (l.r.c2CircletoCapsule)(c, deg));
            let cc = c2Circle { p: a0, r: 0.0 };
            eq_i("C76 zero-length coincident", (l.c.c2CircletoCapsule)(cc, deg), (l.r.c2CircletoCapsule)(cc, deg));
            // negative radii and wild
            let nc = c2Circle { p: rng.v(), r: -rng.unit() * 10.0 };
            let ncap = c2Capsule { a: a0, b: b0, r: -rng.unit() * 10.0 };
            eq_i("C76 negative", (l.c.c2CircletoCapsule)(nc, ncap), (l.r.c2CircletoCapsule)(nc, ncap));
            let wc = c2Circle { p: rng.wild_v(), r: rng.wild() };
            let wcap = c2Capsule { a: rng.wild_v(), b: rng.wild_v(), r: rng.wild() };
            eq_i("C76 wild", (l.c.c2CircletoCapsule)(wc, wcap), (l.r.c2CircletoCapsule)(wc, wcap));
        }
    }
}

// ---------------------------------------------------------------------------
// C77 — c2AABBtoCapsule (full GJK pipeline behind a boolean)
// ---------------------------------------------------------------------------
#[test]
fn c77_aabb_to_capsule() {
    let l = libs();
    let mut rng = Rng::new(0x77);
    unsafe {
        for _ in 0..(N * 6) {
            let a = rng.aabb();
            let b = rng.capsule();
            eq_i("C77 random", (l.c.c2AABBtoCapsule)(a, b), (l.r.c2AABBtoCapsule)(a, b));
        }
        for _ in 0..N {
            // arranged hits and misses
            let p = rng.v();
            let w = 1.0 + rng.unit() * 10.0;
            let bb = c2AABB { min: p, max: c2v { x: p.x + w, y: p.y + w } };
            for k in [-2.0f32, -0.5, 0.0, 0.5, 1.0, 2.0, 5.0] {
                let cap = c2Capsule {
                    a: c2v { x: p.x + w * k, y: p.y - w },
                    b: c2v { x: p.x + w * k, y: p.y + 2.0 * w },
                    r: rng.unit() * w * 0.5,
                };
                eq_i("C77 arranged", (l.c.c2AABBtoCapsule)(bb, cap), (l.r.c2AABBtoCapsule)(bb, cap));
            }
            let deg = c2Capsule { a: p, b: p, r: 0.0 };
            eq_i("C77 degenerate", (l.c.c2AABBtoCapsule)(bb, deg), (l.r.c2AABBtoCapsule)(bb, deg));
            let wb = c2AABB { min: rng.wild_v(), max: rng.wild_v() };
            let wcap = c2Capsule { a: rng.wild_v(), b: rng.wild_v(), r: rng.wild() };
            eq_i("C77 wild", (l.c.c2AABBtoCapsule)(wb, wcap), (l.r.c2AABBtoCapsule)(wb, wcap));
        }
    }
}

// ---------------------------------------------------------------------------
// C78 — c2CapsuletoCapsule
// ---------------------------------------------------------------------------
#[test]
fn c78_capsule_to_capsule() {
    let l = libs();
    let mut rng = Rng::new(0x78);
    unsafe {
        for _ in 0..(N * 6) {
            let a = rng.capsule();
            let b = rng.capsule();
            eq_i("C78 random", (l.c.c2CapsuletoCapsule)(a, b), (l.r.c2CapsuletoCapsule)(a, b));
            eq_i("C78 self", (l.c.c2CapsuletoCapsule)(a, a), (l.r.c2CapsuletoCapsule)(a, a));
        }
        for _ in 0..N {
            let p = rng.v();
            let d = rng.v();
            let r = rng.unit() * 5.0;
            let a = c2Capsule { a: p, b: c2v { x: p.x + d.x, y: p.y + d.y }, r };
            // parallel offsets
            for off in [0.0f32, r * 0.5, r * 2.0, r * 5.0, -r] {
                let perp = c2v { x: -d.y, y: d.x };
                let b = c2Capsule {
                    a: c2v { x: p.x + perp.x * off, y: p.y + perp.y * off },
                    b: c2v { x: p.x + d.x + perp.x * off, y: p.y + d.y + perp.y * off },
                    r,
                };
                eq_i("C78 parallel", (l.c.c2CapsuletoCapsule)(a, b), (l.r.c2CapsuletoCapsule)(a, b));
            }
            // crossing (perpendicular through the middle)
            let mid = c2v { x: p.x + d.x * 0.5, y: p.y + d.y * 0.5 };
            let cross = c2Capsule {
                a: c2v { x: mid.x - d.y, y: mid.y + d.x },
                b: c2v { x: mid.x + d.y, y: mid.y - d.x },
                r,
            };
            eq_i("C78 crossing", (l.c.c2CapsuletoCapsule)(a, cross), (l.r.c2CapsuletoCapsule)(a, cross));
            // collinear extension
            let coll = c2Capsule {
                a: c2v { x: p.x + d.x, y: p.y + d.y },
                b: c2v { x: p.x + d.x * 2.0, y: p.y + d.y * 2.0 },
                r,
            };
            eq_i("C78 collinear", (l.c.c2CapsuletoCapsule)(a, coll), (l.r.c2CapsuletoCapsule)(a, coll));
            // degenerate + wild
            let deg = c2Capsule { a: p, b: p, r: 0.0 };
            eq_i("C78 degenerate", (l.c.c2CapsuletoCapsule)(deg, deg), (l.r.c2CapsuletoCapsule)(deg, deg));
            let wa = c2Capsule { a: rng.wild_v(), b: rng.wild_v(), r: rng.wild() };
            let wb = c2Capsule { a: rng.wild_v(), b: rng.wild_v(), r: rng.wild() };
            eq_i("C78 wild", (l.c.c2CapsuletoCapsule)(wa, wb), (l.r.c2CapsuletoCapsule)(wa, wb));
        }
    }
}

// ---------------------------------------------------------------------------
// C79/C80 — c2Collided over the full 3x3 dispatch matrix
// ---------------------------------------------------------------------------
#[test]
fn c79_c80_collided_matrix() {
    let l = libs();
    let mut rng = Rng::new(0x79);
    unsafe {
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                for _ in 0..(N * 3) {
                    let a = gen_shape(&mut rng, ka);
                    let b = gen_shape(&mut rng, kb);
                    eq_i(
                        &format!("C79 collided({ka},{kb})"),
                        (l.c.c2Collided)(a.ptr(), a.ty(), b.ptr(), b.ty()),
                        (l.r.c2Collided)(a.ptr(), a.ty(), b.ptr(), b.ty()),
                    );
                }
            }
        }
        // C80: deliberately asymmetric shapes on the argument-swapping rows,
        // so a translation that forgot to swap A/B would diverge.
        for _ in 0..(N * 4) {
            let circle = Shape::Circle(c2Circle { p: rng.v(), r: rng.unit() * 3.0 });
            let bb = Shape::Aabb(c2AABB {
                min: c2v { x: -100.0, y: -1.0 },
                max: c2v { x: -90.0, y: 1.0 },
            });
            let cap = Shape::Capsule(c2Capsule {
                a: c2v { x: 50.0, y: 50.0 },
                b: c2v { x: 55.0, y: 90.0 },
                r: rng.unit() * 8.0,
            });
            for (a, b) in [
                (bb, circle),
                (circle, bb),
                (cap, circle),
                (circle, cap),
                (cap, bb),
                (bb, cap),
            ] {
                eq_i(
                    "C80 collided swapped",
                    (l.c.c2Collided)(a.ptr(), a.ty(), b.ptr(), b.ty()),
                    (l.r.c2Collided)(a.ptr(), a.ty(), b.ptr(), b.ty()),
                );
            }
        }
        // wild values through the dispatcher
        for ka in 0..3u32 {
            for kb in 0..3u32 {
                for _ in 0..N {
                    let mk = |rng: &mut Rng, k: u32| match k {
                        0 => Shape::Circle(c2Circle { p: rng.wild_v(), r: rng.wild() }),
                        1 => Shape::Aabb(c2AABB { min: rng.wild_v(), max: rng.wild_v() }),
                        _ => Shape::Capsule(c2Capsule { a: rng.wild_v(), b: rng.wild_v(), r: rng.wild() }),
                    };
                    let a = mk(&mut rng, ka);
                    let b = mk(&mut rng, kb);
                    eq_i(
                        &format!("C79 collided wild({ka},{kb})"),
                        (l.c.c2Collided)(a.ptr(), a.ty(), b.ptr(), b.ty()),
                        (l.r.c2Collided)(a.ptr(), a.ty(), b.ptr(), b.ty()),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C81/C82/C83 — the `capsule` entry point from include/lib.h
// ---------------------------------------------------------------------------
#[test]
fn c81_to_c83_capsule_entry() {
    let l = libs();
    let mut rng = Rng::new(0x81);
    let mut seen = [false; 8];
    unsafe {
        for _ in 0..(N * 20) {
            let (a, b, c, d, e) = (rng.coord(), rng.coord(), rng.coord(), rng.coord(), rng.radius());
            let rc = (l.c.capsule)(a, b, c, d, e);
            let rr = (l.r.capsule)(a, b, c, d, e);
            eq_i(&format!("C81 capsule({a},{b},{c},{d},{e})"), rc, rr);
            if (0..8).contains(&rc) {
                seen[rc as usize] = true;
            }
        }
        // C83 — inputs aimed at each individual result bit
        //   bit0: circle  at (-70,0) r=20
        //   bit1: aabb    [-40,-40]..[-15,-15]
        //   bit2: capsule (-40,40)..(-20,100) r=10
        let targeted: &[(f32, f32, f32, f32, f32)] = &[
            (-70.0, 0.0, -70.0, 0.0, 1.0),        // bit0 only
            (-27.0, -27.0, -27.0, -27.0, 1.0),    // bit1 only
            (-30.0, 70.0, -30.0, 70.0, 1.0),      // bit2 only
            (-70.0, 0.0, -27.0, -27.0, 1.0),      // bits 0+1
            (-70.0, 0.0, -30.0, 70.0, 1.0),       // bits 0+2
            (-27.0, -27.0, -30.0, 70.0, 1.0),     // bits 1+2
            (-70.0, 0.0, -30.0, 70.0, 200.0),     // all three (fat capsule)
            (1000.0, 1000.0, 2000.0, 2000.0, 1.0),// none
        ];
        for &(a, b, c, d, e) in targeted {
            let rc = (l.c.capsule)(a, b, c, d, e);
            eq_i(&format!("C83 capsule targeted({a},{b},{c},{d},{e})"), rc, (l.r.capsule)(a, b, c, d, e));
            if (0..8).contains(&rc) {
                seen[rc as usize] = true;
            }
        }
        // C82 — boundary / degenerate / special arguments
        let specials = [
            0.0f32,
            -0.0,
            1.0,
            -1.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MAX,
            f32::MIN_POSITIVE / 3.0,
            -70.0,
            -20.0,
            1.0e30,
        ];
        for &a in &specials {
            for &b in &specials {
                for &e in &specials {
                    eq_i(
                        &format!("C82 capsule special({a},{b},{e})"),
                        (l.c.capsule)(a, b, a, b, e),
                        (l.r.capsule)(a, b, a, b, e),
                    );
                    eq_i(
                        &format!("C82 capsule special2({a},{b},{e})"),
                        (l.c.capsule)(a, b, b, a, e),
                        (l.r.capsule)(a, b, b, a, e),
                    );
                }
            }
        }
    }
    let n_seen = seen.iter().filter(|x| **x).count();
    assert!(
        n_seen >= 6,
        "C81: only {n_seen}/8 distinct `capsule` return values exercised: {seen:?}"
    );
}
