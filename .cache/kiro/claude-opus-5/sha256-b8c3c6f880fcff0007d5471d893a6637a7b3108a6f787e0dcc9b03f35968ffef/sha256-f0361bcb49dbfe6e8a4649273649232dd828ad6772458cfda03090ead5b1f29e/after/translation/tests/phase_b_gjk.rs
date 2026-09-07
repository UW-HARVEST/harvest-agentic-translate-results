//! Phase B — valid-path differential tests for `c2GJK` and `gjk_cache`.
//! CONFIGS.md rows 23–50.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void};

/// One of the three shapes, stored in a 16-byte-aligned blob so it can be
/// handed to `c2GJK` as `const void *` exactly like the C caller does.
#[derive(Copy, Clone, Debug)]
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

#[repr(C, align(16))]
pub struct Blob([u8; 32]);

impl Shape {
    pub fn ty(&self) -> u32 {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    pub fn blob(&self) -> Blob {
        let mut b = Blob([0u8; 32]);
        unsafe {
            match self {
                Shape::Circle(c) => {
                    std::ptr::copy_nonoverlapping(
                        c as *const c2Circle as *const u8,
                        b.0.as_mut_ptr(),
                        std::mem::size_of::<c2Circle>(),
                    );
                }
                Shape::Aabb(v) => {
                    std::ptr::copy_nonoverlapping(
                        v as *const c2AABB as *const u8,
                        b.0.as_mut_ptr(),
                        std::mem::size_of::<c2AABB>(),
                    );
                }
                Shape::Capsule(v) => {
                    std::ptr::copy_nonoverlapping(
                        v as *const c2Capsule as *const u8,
                        b.0.as_mut_ptr(),
                        std::mem::size_of::<c2Capsule>(),
                    );
                }
            }
        }
        b
    }
}

pub const TYPES: [u32; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

/// `mode` selects the geometric regime (CONFIGS rows 40–46).
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Mode {
    Random,
    Overlapping,
    Touching,
    Far,
    Degenerate,
    Huge,
    Tiny,
    Wild,
}

pub fn make_shape(rng: &mut Rng, ty: u32, center: c2v, mode: Mode) -> Shape {
    let (scale, rscale) = match mode {
        Mode::Huge => (1e18, 1e17),
        Mode::Tiny => (1e-20, 1e-21),
        _ => (40.0, 20.0),
    };
    let deg = mode == Mode::Degenerate;
    let wild = mode == Mode::Wild;
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
            p: if wild { rng.any_v() } else { center },
            r: if deg {
                0.0
            } else if wild {
                rng.any_f32()
            } else {
                rng.unit() * rscale + rscale * 0.1
            },
        }),
        C2_TYPE_AABB => {
            if deg {
                let p = center;
                Shape::Aabb(c2AABB { min: p, max: p })
            } else if wild {
                Shape::Aabb(c2AABB { min: rng.any_v(), max: rng.any_v() })
            } else {
                let hw = rng.unit() * scale + scale * 0.05;
                let hh = rng.unit() * scale + scale * 0.05;
                Shape::Aabb(c2AABB {
                    min: c2v { x: center.x - hw, y: center.y - hh },
                    max: c2v { x: center.x + hw, y: center.y + hh },
                })
            }
        }
        _ => {
            if deg {
                Shape::Capsule(c2Capsule { a: center, b: center, r: 0.0 })
            } else if wild {
                Shape::Capsule(c2Capsule { a: rng.any_v(), b: rng.any_v(), r: rng.any_f32() })
            } else {
                let d = c2v { x: rng.sym(scale), y: rng.sym(scale) };
                Shape::Capsule(c2Capsule {
                    a: c2v { x: center.x - d.x, y: center.y - d.y },
                    b: c2v { x: center.x + d.x, y: center.y + d.y },
                    r: rng.unit() * rscale + rscale * 0.1,
                })
            }
        }
    }
}

pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: c_int,
    pub cache: Option<c2GJKCache>,
}

/// Calls `c2GJK` through a `.so` export.
#[allow(clippy::too_many_arguments)]
pub unsafe fn call_gjk(
    f: &libloading::Symbol<FnGJK>,
    A: &Blob,
    tA: u32,
    ax: Option<&c2x>,
    B: &Blob,
    tB: u32,
    bx: Option<&c2x>,
    use_radius: c_int,
    cache_in: Option<c2GJKCache>,
    want_out: bool,
) -> GjkOut {
    unsafe {
        let mut a = c2v { x: 1234.5, y: -1234.5 };
        let mut b = c2v { x: -4321.5, y: 4321.5 };
        let mut it: c_int = -777;
        let mut cache = cache_in;
        let dist = f(
            A.0.as_ptr() as *const c_void,
            tA,
            ax.map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            B.0.as_ptr() as *const c_void,
            tB,
            bx.map(|p| p as *const c2x).unwrap_or(std::ptr::null()),
            if want_out { &mut a } else { std::ptr::null_mut() },
            if want_out { &mut b } else { std::ptr::null_mut() },
            use_radius,
            if want_out { &mut it } else { std::ptr::null_mut() },
            cache.as_mut().map(|c| c as *mut c2GJKCache).unwrap_or(std::ptr::null_mut()),
        );
        GjkOut { dist, a, b, iters: it, cache }
    }
}

pub fn assert_gjk_eq(c: &GjkOut, r: &GjkOut, ctx: &str) {
    assert!(
        feq(c.dist, r.dist),
        "{ctx}: dist C={} R={}",
        fdesc(c.dist),
        fdesc(r.dist)
    );
    assert!(
        veq(c.a, r.a),
        "{ctx}: outA C={} R={} (dist C={} R={})",
        vdesc(c.a), vdesc(r.a), fdesc(c.dist), fdesc(r.dist)
    );
    assert!(veq(c.b, r.b), "{ctx}: outB C={} R={}", vdesc(c.b), vdesc(r.b));
    assert_eq!(c.iters, r.iters, "{ctx}: iterations");
    match (&c.cache, &r.cache) {
        (Some(cc), Some(rc)) => assert!(
            cache_eq(cc, rc),
            "{ctx}: cache C={:?} R={:?}",
            cc, rc
        ),
        (None, None) => {}
        _ => panic!("{ctx}: cache presence mismatch"),
    }
}

fn rot(rng: &mut Rng) -> c2r {
    let t = rng.unit() * std::f32::consts::TAU;
    c2r { c: t.cos(), s: t.sin() }
}

// ---------------------------------------------------------------------------
// Rows 23–31: every type pair, identity transforms, use_radius=1, cache=NULL
// Row 32: every type pair, use_radius=0
// ---------------------------------------------------------------------------

fn type_pair_sweep(use_radius: c_int, label: &str) {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    for tA in TYPES {
        for tB in TYPES {
            let mut rng = Rng::new(SEED ^ ((tA as u64) << 8) ^ (tB as u64) ^ (use_radius as u64) << 16);
            for i in 0..N {
                let mode = match i % 7 {
                    0 => Mode::Overlapping,
                    1 => Mode::Far,
                    2 => Mode::Degenerate,
                    3 => Mode::Huge,
                    4 => Mode::Tiny,
                    _ => Mode::Random,
                };
                let sep = match mode {
                    Mode::Overlapping => 5.0,
                    Mode::Far => 5000.0,
                    Mode::Huge => 1e18,
                    Mode::Tiny => 1e-20,
                    _ => rng.unit() * 300.0,
                };
                let ca = c2v { x: 0.0, y: 0.0 };
                let cb = c2v { x: rng.sym(sep), y: rng.sym(sep) };
                let A = make_shape(&mut rng, tA, ca, mode);
                let B = make_shape(&mut rng, tB, cb, mode);
                let (ba, bb) = (A.blob(), B.blob());
                unsafe {
                    let oc = call_gjk(&g_c, &ba, tA, None, &bb, tB, None, use_radius, None, true);
                    let or = call_gjk(&g_r, &ba, tA, None, &bb, tB, None, use_radius, None, true);
                    assert_gjk_eq(
                        &oc,
                        &or,
                        &format!("{label} tA{tA} tB{tB} i{i} mode{mode:?} A={A:?} B={B:?}"),
                    );
                }
            }
        }
    }
}

#[test]
fn row23_to_row31_all_type_pairs_radius() {
    type_pair_sweep(1, "rows23-31");
}

#[test]
fn row32_all_type_pairs_no_radius() {
    type_pair_sweep(0, "row32");
}

// ---------------------------------------------------------------------------
// Rows 33–35: transform combinations
// ---------------------------------------------------------------------------

#[test]
fn row33_row34_row35_transforms() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    // which: 0 -> ax only (row 33), 1 -> bx only (row 34), 2 -> both (row 35)
    for which in 0..3 {
        for tA in TYPES {
            for tB in TYPES {
                let mut rng = Rng::new(SEED ^ 0x33 ^ (which << 24) ^ ((tA as u64) << 8) ^ tB as u64);
                for i in 0..N / 2 {
                    let mode = match i % 5 {
                        0 => Mode::Overlapping,
                        1 => Mode::Far,
                        2 => Mode::Degenerate,
                        _ => Mode::Random,
                    };
                    let A = make_shape(&mut rng, tA, c2v { x: 0.0, y: 0.0 }, mode);
                    let B = make_shape(&mut rng, tB, c2v { x: 0.0, y: 0.0 }, mode);
                    let (ba, bb) = (A.blob(), B.blob());
                    let xa = c2x { p: rng.geo_v(200.0), r: rot(&mut rng) };
                    let xb = c2x { p: rng.geo_v(200.0), r: rot(&mut rng) };
                    let (pa, pb) = match which {
                        0 => (Some(&xa), None),
                        1 => (None, Some(&xb)),
                        _ => (Some(&xa), Some(&xb)),
                    };
                    let ur = (i % 2) as c_int;
                    unsafe {
                        let oc = call_gjk(&g_c, &ba, tA, pa, &bb, tB, pb, ur, None, true);
                        let or = call_gjk(&g_r, &ba, tA, pa, &bb, tB, pb, ur, None, true);
                        assert_gjk_eq(
                            &oc,
                            &or,
                            &format!("row33-35 which{which} tA{tA} tB{tB} i{i} A={A:?} B={B:?} xa={xa:?} xb={xb:?}"),
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 36–38: cache write path, warm cache, warm cache + moved transforms
// ---------------------------------------------------------------------------

#[test]
fn row36_row37_row38_cache() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    for tA in TYPES {
        for tB in TYPES {
            let mut rng = Rng::new(SEED ^ 0x36 ^ ((tA as u64) << 8) ^ tB as u64);
            for i in 0..N {
                let mode = match i % 6 {
                    0 => Mode::Overlapping,
                    1 => Mode::Far,
                    2 => Mode::Degenerate,
                    3 => Mode::Huge,
                    _ => Mode::Random,
                };
                let A = make_shape(&mut rng, tA, c2v { x: 0.0, y: 0.0 }, mode);
                let cb = rng.geo_v(300.0);
                let B = make_shape(&mut rng, tB, cb, mode);
                let (ba, bb) = (A.blob(), B.blob());
                let ur = (i % 2) as c_int;

                // Row 36: fresh cache, single call -> exercises only the write path.
                let fresh = c2GJKCache { metric: 0.0, count: 0, iA: [0; 3], iB: [0; 3], div: 0.0 };
                unsafe {
                    let oc = call_gjk(&g_c, &ba, tA, None, &bb, tB, None, ur, Some(fresh), true);
                    let or = call_gjk(&g_r, &ba, tA, None, &bb, tB, None, ur, Some(fresh), true);
                    assert_gjk_eq(&oc, &or, &format!("row36 tA{tA} tB{tB} i{i} A={A:?} B={B:?}"));

                    // Row 37: feed the produced cache back in (cache_was_read path),
                    // three times in a row, comparing after every call.
                    let mut kc = oc.cache.unwrap();
                    let mut kr = or.cache.unwrap();
                    for gn in 0..3 {
                        let oc2 = call_gjk(&g_c, &ba, tA, None, &bb, tB, None, ur, Some(kc), true);
                        let or2 = call_gjk(&g_r, &ba, tA, None, &bb, tB, None, ur, Some(kr), true);
                        assert_gjk_eq(
                            &oc2,
                            &or2,
                            &format!("row37 gn{gn} tA{tA} tB{tB} i{i} A={A:?} B={B:?} cacheIn={kc:?}"),
                        );
                        kc = oc2.cache.unwrap();
                        kr = or2.cache.unwrap();
                    }

                    // Row 38: reuse the warm cache but move the shapes via transforms.
                    let xa = c2x { p: rng.geo_v(400.0), r: rot(&mut rng) };
                    let xb = c2x { p: rng.geo_v(400.0), r: rot(&mut rng) };
                    let oc3 =
                        call_gjk(&g_c, &ba, tA, Some(&xa), &bb, tB, Some(&xb), ur, Some(kc), true);
                    let or3 =
                        call_gjk(&g_r, &ba, tA, Some(&xa), &bb, tB, Some(&xb), ur, Some(kr), true);
                    assert_gjk_eq(
                        &oc3,
                        &or3,
                        &format!("row38 tA{tA} tB{tB} i{i} A={A:?} B={B:?} cacheIn={kc:?}"),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 39: all out-params NULL (return value only)
// ---------------------------------------------------------------------------

#[test]
fn row39_null_out_params() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    for tA in TYPES {
        for tB in TYPES {
            let mut rng = Rng::new(SEED ^ 0x39 ^ ((tA as u64) << 8) ^ tB as u64);
            for i in 0..N {
                let mode = if i % 3 == 0 { Mode::Overlapping } else { Mode::Random };
                let A = make_shape(&mut rng, tA, c2v { x: 0.0, y: 0.0 }, mode);
                let cb = rng.geo_v(200.0);
                let B = make_shape(&mut rng, tB, cb, mode);
                let (ba, bb) = (A.blob(), B.blob());
                let ur = (i % 2) as c_int;
                unsafe {
                    let oc = call_gjk(&g_c, &ba, tA, None, &bb, tB, None, ur, None, false);
                    let or = call_gjk(&g_r, &ba, tA, None, &bb, tB, None, ur, None, false);
                    assert!(
                        feq(oc.dist, or.dist),
                        "row39 tA{tA} tB{tB} i{i}: dist C={} R={}",
                        fdesc(oc.dist), fdesc(or.dist)
                    );
                    // The sentinels must be untouched on both sides.
                    assert!(veq(oc.a, or.a) && veq(oc.b, or.b) && oc.iters == or.iters);
                    assert_eq!(oc.iters, -777, "row39: iterations sentinel overwritten by C");
                    assert_eq!(or.iters, -777, "row39: iterations sentinel overwritten by Rust");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 40–42: hit / touching / far, with a coverage check that the three
// terminal branches of c2GJK are actually reached.
// ---------------------------------------------------------------------------

#[test]
fn row40_row41_row42_hit_touch_far() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let mut zero_dist = 0usize;
    let mut pos_dist = 0usize;
    let mut collapsed = 0usize;

    for tA in TYPES {
        for tB in TYPES {
            let mut rng = Rng::new(SEED ^ 0x40 ^ ((tA as u64) << 8) ^ tB as u64);
            for i in 0..N {
                // Sweep the separation from deep overlap to well clear so the
                // `hit`, midpoint-collapse and radius-shrink branches all fire.
                let t = i as f32 / N as f32;
                let sep = t * 260.0;
                let ang = rng.unit() * std::f32::consts::TAU;
                let cb = c2v { x: sep * ang.cos(), y: sep * ang.sin() };
                let A = make_shape(&mut rng, tA, c2v { x: 0.0, y: 0.0 }, Mode::Random);
                let B = make_shape(&mut rng, tB, cb, Mode::Random);
                let (ba, bb) = (A.blob(), B.blob());
                unsafe {
                    let oc = call_gjk(&g_c, &ba, tA, None, &bb, tB, None, 1, None, true);
                    let or = call_gjk(&g_r, &ba, tA, None, &bb, tB, None, 1, None, true);
                    assert_gjk_eq(
                        &oc,
                        &or,
                        &format!("rows40-42 tA{tA} tB{tB} i{i} sep{sep} A={A:?} B={B:?}"),
                    );
                    if oc.dist == 0.0 {
                        zero_dist += 1;
                        if veq(oc.a, oc.b) {
                            collapsed += 1;
                        }
                    } else {
                        pos_dist += 1;
                    }
                }
            }
        }
    }
    assert!(zero_dist > 50, "row40/41: zero-distance branch under-covered ({zero_dist})");
    assert!(pos_dist > 50, "row42: positive-distance branch under-covered ({pos_dist})");
    assert!(collapsed > 50, "row41: midpoint collapse under-covered ({collapsed})");
}

// ---------------------------------------------------------------------------
// Rows 43 & 44: degenerate and inverted shapes
// ---------------------------------------------------------------------------

#[test]
fn row43_row44_degenerate_and_inverted() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let mut rng = Rng::new(SEED ^ 0x43);
    for tA in TYPES {
        for tB in TYPES {
            for i in 0..N {
                // kind 0..4 = degenerate variants, 5..7 = inverted AABBs
                let kind = i % 8;
                let mk = |rng: &mut Rng, ty: u32, ctr: c2v| -> Shape {
                    match (ty, kind) {
                        (C2_TYPE_CIRCLE, 0 | 5) => Shape::Circle(c2Circle { p: ctr, r: 0.0 }),
                        (C2_TYPE_CIRCLE, 1) => Shape::Circle(c2Circle { p: ctr, r: -5.0 }),
                        (C2_TYPE_AABB, 0 | 1) => Shape::Aabb(c2AABB { min: ctr, max: ctr }),
                        (C2_TYPE_AABB, 5 | 6 | 7) => {
                            // inverted: min > max
                            let h = rng.unit() * 30.0 + 1.0;
                            Shape::Aabb(c2AABB {
                                min: c2v { x: ctr.x + h, y: ctr.y + h },
                                max: c2v { x: ctr.x - h, y: ctr.y - h },
                            })
                        }
                        (C2_TYPE_CAPSULE, 0 | 5) => {
                            Shape::Capsule(c2Capsule { a: ctr, b: ctr, r: 0.0 })
                        }
                        (C2_TYPE_CAPSULE, 1) => Shape::Capsule(c2Capsule {
                            a: ctr,
                            b: ctr,
                            r: rng.unit() * 20.0,
                        }),
                        (C2_TYPE_CAPSULE, 2) => Shape::Capsule(c2Capsule {
                            a: ctr,
                            b: c2v { x: ctr.x + rng.sym(30.0), y: ctr.y + rng.sym(30.0) },
                            r: 0.0,
                        }),
                        _ => make_shape(rng, ty, ctr, Mode::Random),
                    }
                };
                let A = mk(&mut rng, tA, c2v { x: 0.0, y: 0.0 });
                let cb = rng.geo_v(120.0);
                let B = mk(&mut rng, tB, cb);
                let (ba, bb) = (A.blob(), B.blob());
                let ur = (i % 2) as c_int;
                unsafe {
                    let oc = call_gjk(&g_c, &ba, tA, None, &bb, tB, None, ur, None, true);
                    let or = call_gjk(&g_r, &ba, tA, None, &bb, tB, None, ur, None, true);
                    assert_gjk_eq(
                        &oc,
                        &or,
                        &format!("rows43-44 kind{kind} tA{tA} tB{tB} i{i} A={A:?} B={B:?}"),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 45 & 46: extreme magnitudes, iteration cap, no-progress and duplicate
// support breaks.
// ---------------------------------------------------------------------------

#[test]
fn row45_row46_extremes_and_loop_exits() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let mut iter_hist = [0usize; 21];
    for tA in TYPES {
        for tB in TYPES {
            let mut rng = Rng::new(SEED ^ 0x45 ^ ((tA as u64) << 8) ^ tB as u64);
            for i in 0..N * 2 {
                let mode = match i % 4 {
                    0 => Mode::Huge,
                    1 => Mode::Tiny,
                    2 => Mode::Wild,
                    _ => Mode::Random,
                };
                let ctr = match mode {
                    Mode::Huge => c2v { x: rng.sym(1e18), y: rng.sym(1e18) },
                    Mode::Tiny => c2v { x: rng.sym(1e-20), y: rng.sym(1e-20) },
                    _ => rng.geo_v(200.0),
                };
                let A = make_shape(&mut rng, tA, c2v { x: 0.0, y: 0.0 }, mode);
                let B = make_shape(&mut rng, tB, ctr, mode);
                let (ba, bb) = (A.blob(), B.blob());
                let ur = (i % 2) as c_int;
                unsafe {
                    let oc = call_gjk(&g_c, &ba, tA, None, &bb, tB, None, ur, None, true);
                    let or = call_gjk(&g_r, &ba, tA, None, &bb, tB, None, ur, None, true);
                    assert_gjk_eq(
                        &oc,
                        &or,
                        &format!("rows45-46 mode{mode:?} tA{tA} tB{tB} i{i} A={A:?} B={B:?}"),
                    );
                    if (0..=20).contains(&oc.iters) {
                        iter_hist[oc.iters as usize] += 1;
                    }
                }
            }
        }
    }
    // The loop must terminate through more than one exit.
    let distinct = iter_hist.iter().filter(|&&n| n > 0).count();
    assert!(distinct >= 3, "loop-exit coverage too narrow: {iter_hist:?}");
}

// ---------------------------------------------------------------------------
// Row 47: hand-built caches with count 1/2/3 and arbitrary in-range indices,
// random metric and div -> drives both the cache-accept and cache-reject arms.
// ---------------------------------------------------------------------------

#[test]
fn row47_handbuilt_cache() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let max_verts = |ty: u32| -> u32 {
        match ty {
            C2_TYPE_CIRCLE => 1,
            C2_TYPE_AABB => 4,
            _ => 2,
        }
    };

    for tA in TYPES {
        for tB in TYPES {
            let mut rng = Rng::new(SEED ^ 0x47 ^ ((tA as u64) << 8) ^ tB as u64);
            for count in 1..=3i32 {
                for i in 0..N {
                    let A = make_shape(&mut rng, tA, c2v { x: 0.0, y: 0.0 }, Mode::Random);
                    let cb = rng.geo_v(200.0);
                    let B = make_shape(&mut rng, tB, cb, Mode::Random);
                    let (ba, bb) = (A.blob(), B.blob());
                    let mut cache = c2GJKCache {
                        // Straddle the -1.0e8f floor and the *2.0f ratio so both
                        // the accept and the reject arm are taken.
                        metric: match i % 5 {
                            0 => -1e9,
                            1 => -1e7,
                            2 => 0.0,
                            3 => rng.sym(1e3),
                            _ => rng.sym(1e9),
                        },
                        count,
                        iA: [0; 3],
                        iB: [0; 3],
                        div: match i % 4 {
                            0 => 1.0,
                            1 => 0.0,
                            2 => rng.unit() * 1e4,
                            _ => rng.sym(100.0),
                        },
                    };
                    for k in 0..3 {
                        cache.iA[k] = rng.below(max_verts(tA)) as c_int;
                        cache.iB[k] = rng.below(max_verts(tB)) as c_int;
                    }
                    let ur = (i % 2) as c_int;
                    unsafe {
                        let oc =
                            call_gjk(&g_c, &ba, tA, None, &bb, tB, None, ur, Some(cache), true);
                        let or =
                            call_gjk(&g_r, &ba, tA, None, &bb, tB, None, ur, Some(cache), true);
                        assert_gjk_eq(
                            &oc,
                            &or,
                            &format!("row47 count{count} tA{tA} tB{tB} i{i} cache={cache:?} A={A:?} B={B:?}"),
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 48–50: gjk_cache, the one symbol declared in include/lib.h.
//
// `gjk_cache` never dereferences `a9`/`b9` and returns void, so the whole
// observable contract is: it must not crash, and it must not touch the caller's
// buffers. Both properties are asserted for C and Rust identically.
// ---------------------------------------------------------------------------

#[test]
fn row48_row49_row50_gjk_cache() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnGjkCache> = c.sym("gjk_cache");
    let f_r: libloading::Symbol<FnGjkCache> = r.sym("gjk_cache");

    let mut rng = Rng::new(SEED ^ 0x48);
    let revs: [c_char; 5] = [0, 1, -1, 0x7f, 2];
    for i in 0..N * 6 {
        let rev = revs[i % revs.len()];
        // pass kinds: 0..3 well-formed, 4 degenerate, 5 non-finite (row 50)
        let kind = i % 6;
        let (a1, a2, a3, a4, b1, b2, b3, b4, b5) = match kind {
            4 => {
                let p = rng.sym(100.0);
                (p, p, p, p, p, p, p, p, 0.0)
            }
            5 => (
                rng.any_f32(), rng.any_f32(), rng.any_f32(), rng.any_f32(),
                rng.any_f32(), rng.any_f32(), rng.any_f32(), rng.any_f32(), rng.any_f32(),
            ),
            _ => {
                let x = rng.sym(200.0);
                let y = rng.sym(200.0);
                let w = rng.unit() * 80.0;
                let h = rng.unit() * 80.0;
                (
                    x, y, x + w, y + h,
                    rng.sym(250.0), rng.sym(250.0), rng.sym(250.0), rng.sym(250.0),
                    rng.unit() * 30.0,
                )
            }
        };

        let sentinel_a = c2v { x: 11.25, y: -22.5 };
        let sentinel_b = c2v { x: -33.75, y: 44.0 };
        let (mut ac, mut bc) = (sentinel_a, sentinel_b);
        let (mut ar, mut br) = (sentinel_a, sentinel_b);
        unsafe {
            f_c(rev, &mut ac, &mut bc, a1, a2, a3, a4, b1, b2, b3, b4, b5);
            f_r(rev, &mut ar, &mut br, a1, a2, a3, a4, b1, b2, b3, b4, b5);
            // and once with NULL a9/b9, which the C also never dereferences
            f_c(rev, std::ptr::null_mut(), std::ptr::null_mut(), a1, a2, a3, a4, b1, b2, b3, b4, b5);
            f_r(rev, std::ptr::null_mut(), std::ptr::null_mut(), a1, a2, a3, a4, b1, b2, b3, b4, b5);
        }
        assert!(
            veq_strict(ac, ar) && veq_strict(bc, br),
            "gjk_cache i{i} rev{rev} kind{kind}: C=({},{}) R=({},{})",
            vdesc(ac), vdesc(bc), vdesc(ar), vdesc(br)
        );
        assert!(
            veq_strict(ac, sentinel_a) && veq_strict(bc, sentinel_b),
            "gjk_cache i{i}: C unexpectedly wrote to a9/b9"
        );
        assert!(
            veq_strict(ar, sentinel_a) && veq_strict(br, sentinel_b),
            "gjk_cache i{i}: Rust unexpectedly wrote to a9/b9"
        );
    }
}
