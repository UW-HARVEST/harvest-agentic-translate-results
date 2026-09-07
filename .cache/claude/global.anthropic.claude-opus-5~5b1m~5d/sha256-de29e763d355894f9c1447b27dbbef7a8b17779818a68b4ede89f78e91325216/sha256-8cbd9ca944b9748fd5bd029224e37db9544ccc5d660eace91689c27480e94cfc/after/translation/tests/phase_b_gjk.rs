//! Phase B — CONFIGS.md rows 39..59: `c2GJK`, the lowest-level composed entry
//! point, driven directly across the full cross-product of its options
//! (`use_radius` x transforms x cache x out-params x shape-type pair).

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 600;

// ---------------------------------------------------------------------------
// Shape plumbing: the C reads shapes through an unchecked `void*`, so both
// libraries get a pointer to byte-identical, identically aligned storage.
// ---------------------------------------------------------------------------

#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct ShapeBuf {
    b: [u8; 32],
}

#[derive(Copy, Clone, Debug)]
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    pub fn ty(&self) -> c_int {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    pub fn buf(&self) -> ShapeBuf {
        let mut s = ShapeBuf { b: [0u8; 32] };
        unsafe {
            match self {
                Shape::Circle(c) => std::ptr::copy_nonoverlapping(
                    c as *const c2Circle as *const u8,
                    s.b.as_mut_ptr(),
                    std::mem::size_of::<c2Circle>(),
                ),
                Shape::Aabb(c) => std::ptr::copy_nonoverlapping(
                    c as *const c2AABB as *const u8,
                    s.b.as_mut_ptr(),
                    std::mem::size_of::<c2AABB>(),
                ),
                Shape::Capsule(c) => std::ptr::copy_nonoverlapping(
                    c as *const c2Capsule as *const u8,
                    s.b.as_mut_ptr(),
                    std::mem::size_of::<c2Capsule>(),
                ),
            }
        }
        s
    }
    /// Number of proxy vertices this shape type produces (`c2MakeProxy`).
    pub fn nverts(&self) -> c_int {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
}

pub fn rand_shape(rng: &mut Rng, ty: c_int) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(rng.circle()),
        C2_TYPE_AABB => Shape::Aabb(rng.aabb()),
        _ => Shape::Capsule(rng.capsule()),
    }
}

/// How the transform arguments are supplied.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum XMode {
    Null,
    Identity,
    Translation,
    UnitRot,
    NonUnitRot,
}

pub fn make_x(rng: &mut Rng, m: XMode) -> Option<c2x> {
    match m {
        XMode::Null => None,
        XMode::Identity => Some(c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 1.0, s: 0.0 },
        }),
        XMode::Translation => Some(c2x {
            p: rng.v(),
            r: c2r { c: 1.0, s: 0.0 },
        }),
        XMode::UnitRot => {
            let t = rng.f32_in(-3.14159265, 3.14159265);
            Some(c2x {
                p: rng.v(),
                r: c2r {
                    c: t.cos(),
                    s: t.sin(),
                },
            })
        }
        XMode::NonUnitRot => Some(c2x {
            p: rng.v(),
            r: c2r {
                c: rng.f32_in(-3.0, 3.0),
                s: rng.f32_in(-3.0, 3.0),
            },
        }),
    }
}

/// How the cache argument is supplied.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CacheMode {
    Null,
    Fresh,
    Warm,
    Handmade(c_int),
}

/// The full observable result of one `c2GJK` call.
#[derive(Copy, Clone, Debug)]
pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: c_int,
    pub cache: c2GJKCache,
}

#[derive(Copy, Clone, Debug)]
pub struct GjkCfg {
    pub use_radius: c_int,
    pub want_out: bool,
    pub want_iters: bool,
}

impl Default for GjkCfg {
    fn default() -> Self {
        GjkCfg {
            use_radius: 1,
            want_out: true,
            want_iters: true,
        }
    }
}

/// One `c2GJK` invocation against one library.
#[allow(clippy::too_many_arguments)]
unsafe fn call_gjk(
    f: FnGJK,
    ab: &ShapeBuf,
    ta: c_int,
    ax: Option<c2x>,
    bb: &ShapeBuf,
    tb: c_int,
    bx: Option<c2x>,
    cfg: GjkCfg,
    cache: Option<&mut c2GJKCache>,
) -> GjkOut {
    let mut oa = c2v { x: 12.5, y: -37.25 };
    let mut ob = c2v { x: -101.0, y: 6.75 };
    let mut it: c_int = -12345;
    let axp = ax.as_ref().map_or(std::ptr::null(), |v| v as *const c2x);
    let bxp = bx.as_ref().map_or(std::ptr::null(), |v| v as *const c2x);
    let seen_cache = cache.as_ref().map(|c| **c);
    let cp = match cache {
        Some(c) => c as *mut c2GJKCache,
        None => std::ptr::null_mut(),
    };
    let dist = f(
        ab.b.as_ptr() as *const c_void,
        ta,
        axp,
        bb.b.as_ptr() as *const c_void,
        tb,
        bxp,
        if cfg.want_out { &mut oa } else { std::ptr::null_mut() },
        if cfg.want_out { &mut ob } else { std::ptr::null_mut() },
        cfg.use_radius,
        if cfg.want_iters { &mut it } else { std::ptr::null_mut() },
        cp,
    );
    GjkOut {
        dist,
        a: oa,
        b: ob,
        iters: it,
        cache: if cp.is_null() {
            seen_cache.unwrap_or_default()
        } else {
            *cp
        },
    }
}

/// Runs the same `c2GJK` configuration against both libraries and compares
/// every observable byte.
#[track_caller]
pub fn gjk_case(
    rng: &mut Rng,
    sa: Shape,
    sb: Shape,
    axm: XMode,
    bxm: XMode,
    cfg: GjkCfg,
    cm: CacheMode,
) {
    let p = apis();
    let ab = sa.buf();
    let bb = sb.buf();
    // Both libs must see identical transforms, so generate once.
    let ax = make_x(rng, axm);
    let bx = make_x(rng, bxm);
    let ctx = (sa, sb, axm, bxm, cfg, cm, ax, bx);

    let mut cc: Option<c2GJKCache>;
    let mut rc: Option<c2GJKCache>;
    match cm {
        CacheMode::Null => {
            cc = None;
            rc = None;
        }
        CacheMode::Fresh => {
            let z = c2GJKCache {
                metric: 0.0,
                count: 0,
                iA: [0; 3],
                iB: [0; 3],
                div: 0.0,
            };
            cc = Some(z);
            rc = Some(z);
        }
        CacheMode::Warm => {
            // Prime each library's cache with its own first call, then compare
            // both the priming call and the second (cache-reading) call.
            let z = c2GJKCache {
                metric: 0.0,
                count: 0,
                iA: [0; 3],
                iB: [0; 3],
                div: 0.0,
            };
            cc = Some(z);
            rc = Some(z);
            unsafe {
                let o1c = call_gjk(p.c.c2GJK, &ab, sa.ty(), ax, &bb, sb.ty(), bx, cfg, cc.as_mut());
                let o1r = call_gjk(p.r.c2GJK, &ab, sa.ty(), ax, &bb, sb.ty(), bx, cfg, rc.as_mut());
                cmp_out("c2GJK/warm-prime", &ctx, &o1c, &o1r);
            }
        }
        CacheMode::Handmade(count) => {
            let nva = sa.nverts();
            let nvb = sb.nverts();
            let mut mk = |rng: &mut Rng| c2GJKCache {
                metric: match rng.below(6) {
                    0 => 0.0,
                    1 => -1.0e9,
                    2 => f32::MAX,
                    _ => rng.f32_in(-50.0, 50.0),
                },
                count,
                // indices must stay inside the *initialised* part of the proxy;
                // beyond `count` the C reads uninitialised stack (see ERRORS.md)
                iA: [
                    (rng.next_u32() as c_int).rem_euclid(nva),
                    (rng.next_u32() as c_int).rem_euclid(nva),
                    (rng.next_u32() as c_int).rem_euclid(nva),
                ],
                iB: [
                    (rng.next_u32() as c_int).rem_euclid(nvb),
                    (rng.next_u32() as c_int).rem_euclid(nvb),
                    (rng.next_u32() as c_int).rem_euclid(nvb),
                ],
                div: match rng.below(5) {
                    0 => 0.0,
                    1 => 1.0,
                    _ => rng.f32_in(-6.0, 6.0),
                },
            };
            let h = mk(rng);
            cc = Some(h);
            rc = Some(h);
        }
    }

    unsafe {
        let oc = call_gjk(p.c.c2GJK, &ab, sa.ty(), ax, &bb, sb.ty(), bx, cfg, cc.as_mut());
        let or_ = call_gjk(p.r.c2GJK, &ab, sa.ty(), ax, &bb, sb.ty(), bx, cfg, rc.as_mut());
        cmp_out("c2GJK", &ctx, &oc, &or_);
    }
    // and the caller-visible cache state afterwards
    if let (Some(a), Some(b)) = (cc, rc) {
        eq_bytes("c2GJK/cache", &ctx, &a, &b);
    }
}

#[track_caller]
fn cmp_out(what: &str, ctx: &dyn std::fmt::Debug, c: &GjkOut, r: &GjkOut) {
    eq_f32(&format!("{what}.dist"), ctx, c.dist, r.dist);
    eq_v(&format!("{what}.outA"), ctx, c.a, r.a);
    eq_v(&format!("{what}.outB"), ctx, c.b, r.b);
    eq_int(&format!("{what}.iterations"), ctx, c.iters, r.iters);
    eq_bytes(&format!("{what}.cache"), ctx, &c.cache, &r.cache);
}

const ALL_TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// Places `sb` at a controlled separation from `sa` so the "disjoint far",
/// "near", "touching" and "overlapping" regimes are all hit.
fn shifted(s: Shape, dx: f32, dy: f32) -> Shape {
    let sh = |v: c2v| c2v {
        x: v.x + dx,
        y: v.y + dy,
    };
    match s {
        Shape::Circle(c) => Shape::Circle(c2Circle { p: sh(c.p), r: c.r }),
        Shape::Aabb(c) => Shape::Aabb(c2AABB {
            min: sh(c.min),
            max: sh(c.max),
        }),
        Shape::Capsule(c) => Shape::Capsule(c2Capsule {
            a: sh(c.a),
            b: sh(c.b),
            r: c.r,
        }),
    }
}

// ------------------------------------------------------------- rows 39..45
/// One test per type pair, sweeping `use_radius` and the near/far/overlap regimes.
fn type_pair_sweep(seed: u64, ta: c_int, tb: c_int) {
    let mut rng = Rng::new(seed);
    for _ in 0..n_cases(N) {
        let sa = rand_shape(&mut rng, ta);
        let sb0 = rand_shape(&mut rng, tb);
        for &ur in &[0i32, 1] {
            let cfg = GjkCfg {
                use_radius: ur,
                ..Default::default()
            };
            // far, near, touching-ish, overlapping, coincident
            for &sc in &[0.0f32, 0.5, 1.0, 5.0, 200.0] {
                let sb = shifted(sb0, sc * rng.f32_in(-1.0, 1.0), sc * rng.f32_in(-1.0, 1.0));
                gjk_case(
                    &mut rng,
                    sa,
                    sb,
                    XMode::Null,
                    XMode::Null,
                    cfg,
                    CacheMode::Null,
                );
            }
        }
    }
}

#[test]
fn row39_row40_gjk_circle_circle() {
    type_pair_sweep(0x3900, C2_TYPE_CIRCLE, C2_TYPE_CIRCLE);
}
#[test]
fn row41_gjk_circle_aabb() {
    type_pair_sweep(0x4100, C2_TYPE_CIRCLE, C2_TYPE_AABB);
    type_pair_sweep(0x4101, C2_TYPE_AABB, C2_TYPE_CIRCLE);
}
#[test]
fn row42_gjk_circle_capsule() {
    type_pair_sweep(0x4200, C2_TYPE_CIRCLE, C2_TYPE_CAPSULE);
    type_pair_sweep(0x4201, C2_TYPE_CAPSULE, C2_TYPE_CIRCLE);
}
#[test]
fn row43_gjk_aabb_aabb() {
    type_pair_sweep(0x4300, C2_TYPE_AABB, C2_TYPE_AABB);
}
#[test]
fn row44_gjk_aabb_capsule() {
    type_pair_sweep(0x4400, C2_TYPE_AABB, C2_TYPE_CAPSULE);
    type_pair_sweep(0x4401, C2_TYPE_CAPSULE, C2_TYPE_AABB);
}
#[test]
fn row45_gjk_capsule_capsule() {
    type_pair_sweep(0x4500, C2_TYPE_CAPSULE, C2_TYPE_CAPSULE);
    // explicitly parallel and crossing capsules
    let mut rng = Rng::new(0x4502);
    for _ in 0..n_cases(N) {
        let a0 = rng.v();
        let d = rng.v();
        let off = rng.v();
        let A = Shape::Capsule(c2Capsule {
            a: a0,
            b: c2v { x: a0.x + d.x, y: a0.y + d.y },
            r: rng.radius(),
        });
        // parallel
        let B = Shape::Capsule(c2Capsule {
            a: c2v { x: a0.x + off.x, y: a0.y + off.y },
            b: c2v {
                x: a0.x + off.x + d.x,
                y: a0.y + off.y + d.y,
            },
            r: rng.radius(),
        });
        // crossing (perpendicular)
        let C = Shape::Capsule(c2Capsule {
            a: c2v { x: a0.x - d.y, y: a0.y + d.x },
            b: c2v { x: a0.x + d.y, y: a0.y - d.x },
            r: rng.radius(),
        });
        for &ur in &[0i32, 1] {
            let cfg = GjkCfg { use_radius: ur, ..Default::default() };
            for other in [B, C, A] {
                gjk_case(&mut rng, A, other, XMode::Null, XMode::Null, cfg, CacheMode::Null);
            }
        }
    }
}

// ------------------------------------------------------------- rows 46..48
#[test]
fn row46_row48_gjk_transforms() {
    let mut rng = Rng::new(0x4600);
    let modes = [
        XMode::Null,
        XMode::Identity,
        XMode::Translation,
        XMode::UnitRot,
        XMode::NonUnitRot,
    ];
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for &axm in &modes {
                for &bxm in &modes {
                    for &ur in &[0i32, 1] {
                        for _ in 0..n_cases(20) {
                            let sa = rand_shape(&mut rng, ta);
                            let sb = rand_shape(&mut rng, tb);
                            gjk_case(
                                &mut rng,
                                sa,
                                sb,
                                axm,
                                bxm,
                                GjkCfg {
                                    use_radius: ur,
                                    ..Default::default()
                                },
                                CacheMode::Null,
                            );
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------- rows 49, 50
#[test]
fn row49_row50_gjk_null_outparams_and_iterations() {
    let mut rng = Rng::new(0x4900);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for &(want_out, want_iters) in &[(false, false), (false, true), (true, false), (true, true)] {
                for &ur in &[0i32, 1] {
                    for _ in 0..n_cases(60) {
                        let sa = rand_shape(&mut rng, ta);
                        let sb = rand_shape(&mut rng, tb);
                        gjk_case(
                            &mut rng,
                            sa,
                            sb,
                            XMode::Null,
                            XMode::UnitRot,
                            GjkCfg {
                                use_radius: ur,
                                want_out,
                                want_iters,
                            },
                            CacheMode::Null,
                        );
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------- rows 51..54
#[test]
fn row51_gjk_fresh_cache() {
    let mut rng = Rng::new(0x5100);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for &ur in &[0i32, 1] {
                for _ in 0..n_cases(120) {
                    let sa = rand_shape(&mut rng, ta);
                    let sb = rand_shape(&mut rng, tb);
                    gjk_case(
                        &mut rng,
                        sa,
                        sb,
                        XMode::Null,
                        XMode::Null,
                        GjkCfg { use_radius: ur, ..Default::default() },
                        CacheMode::Fresh,
                    );
                }
            }
        }
    }
}

#[test]
fn row52_gjk_warm_cache_same_shapes() {
    let mut rng = Rng::new(0x5200);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for &ur in &[0i32, 1] {
                for _ in 0..n_cases(120) {
                    let sa = rand_shape(&mut rng, ta);
                    let sb = rand_shape(&mut rng, tb);
                    gjk_case(
                        &mut rng,
                        sa,
                        sb,
                        XMode::Null,
                        XMode::Null,
                        GjkCfg { use_radius: ur, ..Default::default() },
                        CacheMode::Warm,
                    );
                }
            }
        }
    }
}

#[test]
fn row53_gjk_stale_cache_different_shapes() {
    // Exercises the inverted validity test at lib.c:405 -- a cache primed on
    // one shape pair is then (wrongly, but deliberately) trusted for another.
    let p = apis();
    let mut rng = Rng::new(0x5300);
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for _ in 0..n_cases(150) {
                let z = c2GJKCache::default();
                let mut cc = z;
                let mut rc = z;
                let cfg = GjkCfg::default();
                // prime on pair #1, then reuse the SAME cache for pairs #2..#4
                for step in 0..4 {
                    let sa = rand_shape(&mut rng, ta);
                    let sb = rand_shape(&mut rng, tb);
                    let ab = sa.buf();
                    let bb = sb.buf();
                    unsafe {
                        let oc = call_gjk(
                            p.c.c2GJK, &ab, sa.ty(), None, &bb, sb.ty(), None, cfg, Some(&mut cc),
                        );
                        let or_ = call_gjk(
                            p.r.c2GJK, &ab, sa.ty(), None, &bb, sb.ty(), None, cfg, Some(&mut rc),
                        );
                        cmp_out("c2GJK/stale", &(sa, sb, step), &oc, &or_);
                    }
                    eq_bytes("c2GJK/stale-cache", &(sa, sb, step), &cc, &rc);
                }
            }
        }
    }
}

#[test]
fn row54_gjk_handmade_cache() {
    let mut rng = Rng::new(0x5400);
    for &count in &[1i32, 2, 3] {
        for &ta in &ALL_TYPES {
            for &tb in &ALL_TYPES {
                for &ur in &[0i32, 1] {
                    for _ in 0..n_cases(60) {
                        let sa = rand_shape(&mut rng, ta);
                        let sb = rand_shape(&mut rng, tb);
                        gjk_case(
                            &mut rng,
                            sa,
                            sb,
                            XMode::Null,
                            XMode::Null,
                            GjkCfg { use_radius: ur, ..Default::default() },
                            CacheMode::Handmade(count),
                        );
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------- rows 55..58
#[test]
fn row55_gjk_identical_shapes() {
    let mut rng = Rng::new(0x5500);
    for &ta in &ALL_TYPES {
        for _ in 0..n_cases(400) {
            let s = rand_shape(&mut rng, ta);
            for &ur in &[0i32, 1] {
                let cfg = GjkCfg { use_radius: ur, ..Default::default() };
                gjk_case(&mut rng, s, s, XMode::Null, XMode::Null, cfg, CacheMode::Null);
                gjk_case(&mut rng, s, s, XMode::Null, XMode::Null, cfg, CacheMode::Fresh);
            }
        }
    }
}

#[test]
fn row56_gjk_degenerate_shapes() {
    let mut rng = Rng::new(0x5600);
    for _ in 0..n_cases(400) {
        let pt = rng.v();
        let degen = [
            Shape::Circle(c2Circle { p: pt, r: 0.0 }),
            Shape::Aabb(c2AABB { min: pt, max: pt }),
            Shape::Capsule(c2Capsule { a: pt, b: pt, r: 0.0 }),
            Shape::Capsule(c2Capsule { a: pt, b: pt, r: rng.radius() }),
            Shape::Aabb(c2AABB {
                min: pt,
                max: c2v { x: pt.x, y: pt.y + rng.f32_in(0.0, 10.0) },
            }),
        ];
        for &sa in &degen {
            for &sb in &degen {
                for &ur in &[0i32, 1] {
                    gjk_case(
                        &mut rng,
                        sa,
                        sb,
                        XMode::Null,
                        XMode::Null,
                        GjkCfg { use_radius: ur, ..Default::default() },
                        CacheMode::Fresh,
                    );
                }
            }
        }
    }
}

#[test]
fn row57_gjk_exactly_touching() {
    // dist == rA + rB is the exact boundary of the lib.c:485 test.
    let mut rng = Rng::new(0x5700);
    for _ in 0..n_cases(1500) {
        let rA = (rng.below(40) as f32) * 0.5;
        let rB = (rng.below(40) as f32) * 0.5;
        let gap = rA + rB; // exact in binary
        let cy = (rng.below(21) as f32) - 10.0;
        let A = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: cy }, r: rA });
        for &d in &[gap, gap - 0.5, gap + 0.5, 0.0] {
            let B = Shape::Circle(c2Circle { p: c2v { x: d, y: cy }, r: rB });
            for &ur in &[0i32, 1] {
                gjk_case(
                    &mut rng,
                    A,
                    B,
                    XMode::Null,
                    XMode::Null,
                    GjkCfg { use_radius: ur, ..Default::default() },
                    CacheMode::Null,
                );
            }
        }
        // and the same with capsules (radius on a 2-vertex proxy)
        let A2 = Shape::Capsule(c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 0.0, y: 4.0 },
            r: rA,
        });
        let B2 = Shape::Capsule(c2Capsule {
            a: c2v { x: gap, y: 0.0 },
            b: c2v { x: gap, y: 4.0 },
            r: rB,
        });
        for &ur in &[0i32, 1] {
            gjk_case(
                &mut rng,
                A2,
                B2,
                XMode::Null,
                XMode::Null,
                GjkCfg { use_radius: ur, ..Default::default() },
                CacheMode::Null,
            );
        }
    }
}

#[test]
fn row58_gjk_extreme_magnitudes() {
    let mut rng = Rng::new(0x5800);
    let mags = [1.0e-30f32, 1.0e-8, 1.0, 1.0e8, 1.0e18, 1.0e30, f32::MAX];
    for &m in &mags {
        for _ in 0..n_cases(120) {
            let sc = |rng: &mut Rng| c2v {
                x: rng.f32_in(-1.0, 1.0) * m,
                y: rng.f32_in(-1.0, 1.0) * m,
            };
            for &ta in &ALL_TYPES {
                for &tb in &ALL_TYPES {
                    let mk = |rng: &mut Rng, t: c_int| match t {
                        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                            p: sc(rng),
                            r: rng.f32_in(0.0, 1.0) * m,
                        }),
                        C2_TYPE_AABB => {
                            let a = sc(rng);
                            let b = sc(rng);
                            Shape::Aabb(c2AABB {
                                min: c2v { x: a.x.min(b.x), y: a.y.min(b.y) },
                                max: c2v { x: a.x.max(b.x), y: a.y.max(b.y) },
                            })
                        }
                        _ => Shape::Capsule(c2Capsule {
                            a: sc(rng),
                            b: sc(rng),
                            r: rng.f32_in(0.0, 1.0) * m,
                        }),
                    };
                    let sa = mk(&mut rng, ta);
                    let sb = mk(&mut rng, tb);
                    for &ur in &[0i32, 1] {
                        gjk_case(
                            &mut rng,
                            sa,
                            sb,
                            XMode::Null,
                            XMode::Null,
                            GjkCfg { use_radius: ur, ..Default::default() },
                            CacheMode::Fresh,
                        );
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------- row 59
#[test]
fn row59_gjk_full_cross_product() {
    // 3 typeA x 3 typeB x use_radius{0,1} x xform{Null,UnitRot,NonUnitRot}^2
    // x cache{Null,Fresh,Warm,Handmade(1..3)} -- randomized shapes each time.
    let mut rng = Rng::new(0x5900);
    let xmodes = [XMode::Null, XMode::UnitRot, XMode::NonUnitRot];
    let cmodes = [
        CacheMode::Null,
        CacheMode::Fresh,
        CacheMode::Warm,
        CacheMode::Handmade(1),
        CacheMode::Handmade(2),
        CacheMode::Handmade(3),
    ];
    let mut n = 0usize;
    for &ta in &ALL_TYPES {
        for &tb in &ALL_TYPES {
            for &ur in &[0i32, 1] {
                for &axm in &xmodes {
                    for &bxm in &xmodes {
                        for &cm in &cmodes {
                            for _ in 0..n_cases(8) {
                                let sa = rand_shape(&mut rng, ta);
                                let sb = rand_shape(&mut rng, tb);
                                gjk_case(
                                    &mut rng,
                                    sa,
                                    sb,
                                    axm,
                                    bxm,
                                    GjkCfg { use_radius: ur, ..Default::default() },
                                    cm,
                                );
                                n += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(n >= 1900, "cross product too small: {n}");
}
