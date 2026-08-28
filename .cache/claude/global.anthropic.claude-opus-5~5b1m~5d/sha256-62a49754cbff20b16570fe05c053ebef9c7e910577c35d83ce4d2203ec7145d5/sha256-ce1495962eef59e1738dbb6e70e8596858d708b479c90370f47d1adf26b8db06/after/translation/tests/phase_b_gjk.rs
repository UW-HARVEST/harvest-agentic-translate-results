//! Phase B — valid-path differential tests, `CONFIGS.md` rows 36..=50.
//!
//! Group 4: `c2GJK`, the low-level composed pipeline. Every row sweeps all nine
//! ordered `(typeA, typeB)` pairs and compares the return value, `*outA`,
//! `*outB`, `*iterations` and the whole `c2GJKCache` bit-for-bit.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

#[repr(align(4))]
struct Buf([u8; 20]);

#[derive(Copy, Clone, Debug, PartialEq)]
enum XMode {
    Null,
    Identity,
    Translation,
    RotTrans,
    Weird,
}

#[derive(Copy, Clone, Debug)]
struct Cfg {
    use_radius: c_int,
    ax: XMode,
    bx: XMode,
    want_outa: bool,
    want_outb: bool,
    want_iters: bool,
    cache: bool,
}

impl Default for Cfg {
    fn default() -> Self {
        Cfg {
            use_radius: 1,
            ax: XMode::Null,
            bx: XMode::Null,
            want_outa: true,
            want_outb: true,
            want_iters: true,
            cache: false,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Out {
    dist: f32,
    a: c2v,
    b: c2v,
    iters: c_int,
    cache: c2GJKCache,
}

const POISON_V: c2v = c2v { x: -3.5e-17, y: 7.75e21 };
const POISON_I: c_int = -0x1234_5678;

fn make_x(rng: &mut Rng, mode: XMode, scale: f32) -> Option<c2x> {
    match mode {
        XMode::Null => None,
        XMode::Identity => Some(c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r { c: 1.0, s: 0.0 },
        }),
        XMode::Translation => Some(rng.xform_translation(scale)),
        XMode::RotTrans => Some(rng.xform_rot_trans(scale)),
        XMode::Weird => Some(rng.xform_weird(scale)),
    }
}

/// One `c2GJK` call on one library. `cache_in` is passed through by value so the
/// two libraries always start from the identical cache state.
#[allow(clippy::too_many_arguments)]
fn gjk_call(
    api: &Api,
    a: &Shape,
    b: &Shape,
    ax: Option<&c2x>,
    bx: Option<&c2x>,
    cfg: &Cfg,
    cache_in: c2GJKCache,
) -> Out {
    let ba = Buf(a.bytes());
    let bb = Buf(b.bytes());
    let mut oa = POISON_V;
    let mut ob = POISON_V;
    let mut it = POISON_I;
    let mut cache = cache_in;
    let dist = unsafe {
        (api.c2GJK)(
            ba.0.as_ptr() as *const c_void,
            a.ty(),
            ax.map_or(std::ptr::null(), |x| x as *const c2x),
            bb.0.as_ptr() as *const c_void,
            b.ty(),
            bx.map_or(std::ptr::null(), |x| x as *const c2x),
            if cfg.want_outa { &mut oa } else { std::ptr::null_mut() },
            if cfg.want_outb { &mut ob } else { std::ptr::null_mut() },
            cfg.use_radius,
            if cfg.want_iters { &mut it } else { std::ptr::null_mut() },
            if cfg.cache { &mut cache } else { std::ptr::null_mut() },
        )
    };
    Out {
        dist,
        a: oa,
        b: ob,
        iters: it,
        cache,
    }
}

fn assert_out_same(co: &Out, ro: &Out, ctx: &str) {
    assert!(
        f32_same(co.dist, ro.dist),
        "{ctx}: dist C {} vs R {}",
        fmt_f32(co.dist),
        fmt_f32(ro.dist)
    );
    assert!(
        v_same(co.a, ro.a),
        "{ctx}: outA C {} vs R {}",
        fmt_v(co.a),
        fmt_v(ro.a)
    );
    assert!(
        v_same(co.b, ro.b),
        "{ctx}: outB C {} vs R {}",
        fmt_v(co.b),
        fmt_v(ro.b)
    );
    assert_eq!(co.iters, ro.iters, "{ctx}: iterations");
    assert!(
        cache_same(&co.cache, &ro.cache),
        "{ctx}: cache\n  C: {}\n  R: {}",
        fmt_cache(&co.cache),
        fmt_cache(&ro.cache)
    );
}

/// Shape-generation mode for a row.
#[derive(Copy, Clone, Debug)]
enum ShapeMode {
    Random(f32),
    Overlapping,
    Coincident,
    Far,
    Degenerate,
    Inverted,
    Extreme,
}

fn make_pair(rng: &mut Rng, ta: c_int, tb: c_int, mode: ShapeMode) -> (Shape, Shape) {
    match mode {
        ShapeMode::Random(scale) => (
            Shape::random(rng, ta, scale),
            Shape::random(rng, tb, scale),
        ),
        ShapeMode::Overlapping => {
            // Both shapes built inside a small box around the origin so their
            // cores interpenetrate often (forces the `hit` / count==3 path).
            let a = match ta {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                    p: rng.v_ordinary(1.0),
                    r: 1.0 + rng.unit().abs() * 3.0,
                }),
                C2_TYPE_AABB => Shape::Aabb(c2AABB {
                    min: c2v { x: -2.0 - rng.unit().abs(), y: -2.0 - rng.unit().abs() },
                    max: c2v { x: 2.0 + rng.unit().abs(), y: 2.0 + rng.unit().abs() },
                }),
                _ => Shape::Capsule(c2Capsule {
                    a: c2v { x: -2.0, y: rng.ordinary(1.0) },
                    b: c2v { x: 2.0, y: rng.ordinary(1.0) },
                    r: rng.radius(1.0),
                }),
            };
            let b = match tb {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                    p: rng.v_ordinary(1.0),
                    r: 1.0 + rng.unit().abs() * 3.0,
                }),
                C2_TYPE_AABB => Shape::Aabb(c2AABB {
                    min: c2v { x: -1.5 - rng.unit().abs(), y: -1.5 - rng.unit().abs() },
                    max: c2v { x: 1.5 + rng.unit().abs(), y: 1.5 + rng.unit().abs() },
                }),
                _ => Shape::Capsule(c2Capsule {
                    a: c2v { x: rng.ordinary(1.0), y: -2.0 },
                    b: c2v { x: rng.ordinary(1.0), y: 2.0 },
                    r: rng.radius(1.0),
                }),
            };
            (a, b)
        }
        ShapeMode::Coincident => {
            let s = Shape::random(rng, ta, 10.0);
            // Same geometry, re-tagged as tb where the layouts allow it;
            // otherwise the identical shape twice.
            if ta == tb {
                (s, s)
            } else {
                (s, Shape::random(rng, tb, 10.0))
            }
        }
        ShapeMode::Far => {
            let scale = match rng.below(3) {
                0 => 1.0e6,
                1 => 1.0e8,
                _ => 1.0e9,
            };
            let a = Shape::random(rng, ta, 10.0);
            let b = match tb {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                    p: c2v { x: scale, y: scale * 0.5 },
                    r: rng.radius(10.0),
                }),
                C2_TYPE_AABB => Shape::Aabb(c2AABB {
                    min: c2v { x: scale, y: scale },
                    max: c2v { x: scale + 10.0, y: scale + 10.0 },
                }),
                _ => Shape::Capsule(c2Capsule {
                    a: c2v { x: scale, y: scale },
                    b: c2v { x: scale + 5.0, y: scale - 5.0 },
                    r: rng.radius(10.0),
                }),
            };
            (a, b)
        }
        ShapeMode::Degenerate => {
            let mk = |rng: &mut Rng, t: c_int| match t {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                    p: rng.v_ordinary(10.0),
                    r: 0.0,
                }),
                C2_TYPE_AABB => {
                    let p = rng.v_ordinary(10.0);
                    Shape::Aabb(c2AABB { min: p, max: p })
                }
                _ => {
                    let p = rng.v_ordinary(10.0);
                    Shape::Capsule(c2Capsule { a: p, b: p, r: 0.0 })
                }
            };
            (mk(rng, ta), mk(rng, tb))
        }
        ShapeMode::Inverted => (
            Shape::random_degenerate(rng, ta, 10.0),
            Shape::random_degenerate(rng, tb, 10.0),
        ),
        ShapeMode::Extreme => (
            Shape::random_extreme(rng, ta),
            Shape::random_extreme(rng, tb),
        ),
    }
}

/// Run one `CONFIGS.md` row: sweep all 9 type pairs with `iters` randomized
/// inputs each. Returns coverage counters `(zero_dist, max_iters, nonzero)`.
fn run_row(
    c: &Api,
    r: &Api,
    label: &str,
    row: &str,
    seed: u64,
    cfg: Cfg,
    mode: ShapeMode,
    iters: usize,
) -> (usize, usize, usize) {
    let mut rng = Rng::new(seed);
    let mut zero = 0usize;
    let mut maxed = 0usize;
    let mut nonzero = 0usize;
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for i in 0..iters {
                let (sa, sb) = make_pair(&mut rng, ta, tb, mode);
                let ax = make_x(&mut rng, cfg.ax, 20.0);
                let bx = make_x(&mut rng, cfg.bx, 20.0);
                let cache_in = c2GJKCache::default();
                let co = gjk_call(c, &sa, &sb, ax.as_ref(), bx.as_ref(), &cfg, cache_in);
                let ro = gjk_call(r, &sa, &sb, ax.as_ref(), bx.as_ref(), &cfg, cache_in);
                let ctx = format!(
                    "{label} {row} #{i} ta={ta} tb={tb} cfg={cfg:?} mode={mode:?}\n  A={sa:?}\n  B={sb:?}\n  ax={ax:?} bx={bx:?}"
                );
                assert_out_same(&co, &ro, &ctx);
                if co.dist == 0.0 {
                    zero += 1;
                } else {
                    nonzero += 1;
                }
                if co.iters == 20 {
                    maxed += 1;
                }
            }
        }
    }
    println!("{label} {row}: zero_dist={zero} nonzero={nonzero} iter20={maxed}");
    (zero, maxed, nonzero)
}

// ---------------------------------------------------------------------------
// Row 36 / 37 — use_radius on / off
// ---------------------------------------------------------------------------

#[test]
fn row36_gjk_use_radius_on() {
    for_each_pair(|c, r, label| {
        let (z, _, nz) = run_row(
            c,
            r,
            label,
            "row36",
            0x0100,
            Cfg::default(),
            ShapeMode::Random(20.0),
            N_SLOW,
        );
        assert!(z > 0 && nz > 0, "{label} row36: needs both overlap and separation");
    });
}

#[test]
fn row37_gjk_use_radius_off() {
    for_each_pair(|c, r, label| {
        let cfg = Cfg {
            use_radius: 0,
            ..Cfg::default()
        };
        run_row(c, r, label, "row37", 0x0101, cfg, ShapeMode::Random(20.0), N_SLOW);
        // also with a deliberately non-1 truthy value
        let cfg2 = Cfg {
            use_radius: 7,
            ..Cfg::default()
        };
        run_row(c, r, label, "row37b", 0x0102, cfg2, ShapeMode::Random(20.0), 200);
        let cfg3 = Cfg {
            use_radius: -1,
            ..Cfg::default()
        };
        run_row(c, r, label, "row37c", 0x0103, cfg3, ShapeMode::Random(20.0), 200);
    });
}

// ---------------------------------------------------------------------------
// Row 38..=41 — transform handling
// ---------------------------------------------------------------------------

#[test]
fn row38_gjk_identity_and_null_mixed() {
    for_each_pair(|c, r, label| {
        for (ax, bx) in [
            (XMode::Identity, XMode::Null),
            (XMode::Null, XMode::Identity),
            (XMode::Identity, XMode::Identity),
        ] {
            let cfg = Cfg { ax, bx, ..Cfg::default() };
            run_row(c, r, label, "row38", 0x0104, cfg, ShapeMode::Random(20.0), 300);
        }
    });
}

#[test]
fn row39_gjk_translation() {
    for_each_pair(|c, r, label| {
        let cfg = Cfg {
            ax: XMode::Translation,
            bx: XMode::Translation,
            ..Cfg::default()
        };
        run_row(c, r, label, "row39", 0x0105, cfg, ShapeMode::Random(20.0), N_SLOW);
    });
}

#[test]
fn row40_gjk_rot_trans() {
    for_each_pair(|c, r, label| {
        let cfg = Cfg {
            ax: XMode::RotTrans,
            bx: XMode::RotTrans,
            ..Cfg::default()
        };
        run_row(c, r, label, "row40", 0x0106, cfg, ShapeMode::Random(20.0), N_SLOW);
        let cfg2 = Cfg {
            ax: XMode::RotTrans,
            bx: XMode::Null,
            ..Cfg::default()
        };
        run_row(c, r, label, "row40b", 0x0107, cfg2, ShapeMode::Random(20.0), 300);
    });
}

#[test]
fn row41_gjk_weird_rotor() {
    for_each_pair(|c, r, label| {
        let cfg = Cfg {
            ax: XMode::Weird,
            bx: XMode::Weird,
            ..Cfg::default()
        };
        run_row(c, r, label, "row41", 0x0108, cfg, ShapeMode::Random(20.0), N_SLOW);
    });
}

// ---------------------------------------------------------------------------
// Row 42 / 43 — cache handling (cold, then warm chains)
// ---------------------------------------------------------------------------

#[test]
fn row42_gjk_cold_cache() {
    for_each_pair(|c, r, label| {
        let cfg = Cfg {
            cache: true,
            ..Cfg::default()
        };
        run_row(c, r, label, "row42", 0x0109, cfg, ShapeMode::Random(20.0), N_SLOW);
        // Also with a rotated/translated pose so the cached indices vary.
        let cfg2 = Cfg {
            cache: true,
            ax: XMode::RotTrans,
            bx: XMode::RotTrans,
            ..Cfg::default()
        };
        run_row(c, r, label, "row42b", 0x010A, cfg2, ShapeMode::Random(20.0), 300);
    });
}

#[test]
fn row43_gjk_warm_cache_chain() {
    for_each_pair(|c, r, label| {
        let cfg = Cfg {
            cache: true,
            ..Cfg::default()
        };
        let mut rng = Rng::new(0x010B);
        let mut warm_reads = 0usize;
        for &ta in VALID_TYPES.iter() {
            for &tb in VALID_TYPES.iter() {
                for i in 0..N_SLOW {
                    let (mut sa, mut sb) = make_pair(&mut rng, ta, tb, ShapeMode::Random(20.0));
                    let mut ccache = c2GJKCache::default();
                    let mut rcache = c2GJKCache::default();
                    // 4-call chain, nudging the shapes between calls so the
                    // cached simplex is re-validated against fresh geometry.
                    for step in 0..4 {
                        let co = gjk_call(c, &sa, &sb, None, None, &cfg, ccache);
                        let ro = gjk_call(r, &sa, &sb, None, None, &cfg, rcache);
                        let ctx = format!(
                            "{label} row43 #{i} step{step} ta={ta} tb={tb}\n  A={sa:?}\n  B={sb:?}\n  cache_in C={} R={}",
                            fmt_cache(&ccache),
                            fmt_cache(&rcache)
                        );
                        assert_out_same(&co, &ro, &ctx);
                        if step > 0 && ccache.count != 0 {
                            warm_reads += 1;
                        }
                        ccache = co.cache;
                        rcache = ro.cache;
                        // nudge
                        let d = c2v {
                            x: rng.ordinary(1.0),
                            y: rng.ordinary(1.0),
                        };
                        sa = nudge(sa, d);
                        sb = nudge(sb, c2v { x: -d.x, y: -d.y });
                    }
                }
            }
        }
        assert!(
            warm_reads > 0,
            "{label} row43: the warm-cache path was never exercised"
        );
        println!("{label} row43: warm cache reads={warm_reads}");
    });
}

fn nudge(s: Shape, d: c2v) -> Shape {
    match s {
        Shape::Circle(c) => Shape::Circle(c2Circle {
            p: c2v { x: c.p.x + d.x, y: c.p.y + d.y },
            r: c.r,
        }),
        Shape::Aabb(b) => Shape::Aabb(c2AABB {
            min: c2v { x: b.min.x + d.x, y: b.min.y + d.y },
            max: c2v { x: b.max.x + d.x, y: b.max.y + d.y },
        }),
        Shape::Capsule(c) => Shape::Capsule(c2Capsule {
            a: c2v { x: c.a.x + d.x, y: c.a.y + d.y },
            b: c2v { x: c.b.x + d.x, y: c.b.y + d.y },
            r: c.r,
        }),
    }
}

// ---------------------------------------------------------------------------
// Row 44 — NULL out params
// ---------------------------------------------------------------------------

#[test]
fn row44_gjk_null_out_params() {
    for_each_pair(|c, r, label| {
        let variants = [
            (false, true, true),
            (true, false, true),
            (true, true, false),
            (false, false, false),
        ];
        for (i, (oa, ob, it)) in variants.into_iter().enumerate() {
            let cfg = Cfg {
                want_outa: oa,
                want_outb: ob,
                want_iters: it,
                cache: i % 2 == 0,
                ..Cfg::default()
            };
            run_row(
                c,
                r,
                label,
                &format!("row44v{i}"),
                0x010C + i as u64,
                cfg,
                ShapeMode::Random(20.0),
                250,
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Row 45..=50 — geometric / degeneracy sweeps
// ---------------------------------------------------------------------------

#[test]
fn row45_gjk_overlapping_hit_path() {
    for_each_pair(|c, r, label| {
        let (zero, _, _) = run_row(
            c,
            r,
            label,
            "row45",
            0x0110,
            Cfg::default(),
            ShapeMode::Overlapping,
            N_SLOW,
        );
        assert!(zero > 0, "{label} row45: never produced an overlap");
        // Same shapes with use_radius = 0 (the `hit` path ignores use_radius).
        let cfg = Cfg { use_radius: 0, ..Cfg::default() };
        run_row(c, r, label, "row45b", 0x0111, cfg, ShapeMode::Overlapping, 300);
        // And with a cache, which stores the count==3 simplex.
        let cfg2 = Cfg { cache: true, ..Cfg::default() };
        run_row(c, r, label, "row45c", 0x0112, cfg2, ShapeMode::Overlapping, 300);
    });
}

#[test]
fn row46_gjk_coincident() {
    for_each_pair(|c, r, label| {
        run_row(c, r, label, "row46", 0x0113, Cfg::default(), ShapeMode::Coincident, N_SLOW);
        let cfg = Cfg { cache: true, use_radius: 0, ..Cfg::default() };
        run_row(c, r, label, "row46b", 0x0114, cfg, ShapeMode::Coincident, 300);
    });
}

#[test]
fn row47_gjk_far_apart() {
    for_each_pair(|c, r, label| {
        run_row(c, r, label, "row47", 0x0115, Cfg::default(), ShapeMode::Far, N_SLOW);
        let cfg = Cfg { cache: true, ..Cfg::default() };
        run_row(c, r, label, "row47b", 0x0116, cfg, ShapeMode::Far, 300);
    });
}

#[test]
fn row48_gjk_degenerate_shapes() {
    for_each_pair(|c, r, label| {
        run_row(c, r, label, "row48", 0x0117, Cfg::default(), ShapeMode::Degenerate, N_SLOW);
        let cfg = Cfg { use_radius: 0, cache: true, ..Cfg::default() };
        run_row(c, r, label, "row48b", 0x0118, cfg, ShapeMode::Degenerate, 300);
    });
}

#[test]
fn row49_gjk_inverted_and_negative_radius() {
    for_each_pair(|c, r, label| {
        run_row(c, r, label, "row49", 0x0119, Cfg::default(), ShapeMode::Inverted, N_SLOW);
        let cfg = Cfg { use_radius: 0, ..Cfg::default() };
        run_row(c, r, label, "row49b", 0x011A, cfg, ShapeMode::Inverted, 300);
        let cfg2 = Cfg { ax: XMode::RotTrans, bx: XMode::RotTrans, cache: true, ..Cfg::default() };
        run_row(c, r, label, "row49c", 0x011B, cfg2, ShapeMode::Inverted, 300);
    });
}

#[test]
fn row50_gjk_extreme_coordinates() {
    for_each_pair(|c, r, label| {
        run_row(c, r, label, "row50", 0x011C, Cfg::default(), ShapeMode::Extreme, N_SLOW);
        let cfg = Cfg { use_radius: 0, ..Cfg::default() };
        run_row(c, r, label, "row50b", 0x011D, cfg, ShapeMode::Extreme, 300);
    });
}
