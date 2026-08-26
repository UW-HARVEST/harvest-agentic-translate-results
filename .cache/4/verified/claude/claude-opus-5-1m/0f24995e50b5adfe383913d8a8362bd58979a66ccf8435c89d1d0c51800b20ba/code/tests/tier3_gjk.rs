//! Phase B, `CONFIGS.md` rows 40–64: `c2GJK` — the lowest-level *composed*
//! entry point, driven directly rather than through the boolean convenience
//! wrappers.
//!
//! Every call compares FIVE observable outputs bit-for-bit: the `float` return
//! value, `*outA`, `*outB`, `*iterations`, and the whole 36-byte `c2GJKCache`
//! after the call -- plus the input shape buffers, which must not be modified.
//!
//! Exit-reason coverage (`hit`, iteration cap, radius-subtraction branch,
//! midpoint branch, warm cache accepted / rejected) is counted and asserted,
//! so a configuration that silently never reaches its branch fails the test.

#![allow(non_snake_case)]
#![allow(clippy::useless_format, clippy::manual_range_patterns, clippy::needless_late_init, clippy::excessive_precision, clippy::too_many_arguments, clippy::needless_range_loop)]

#[macro_use]
mod common;

use common::*;
use std::os::raw::{c_int, c_void};

const N: usize = 4_000;

// ---------------------------------------------------------------------------
// Shape plumbing
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn ty(&self) -> C2_TYPE {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    fn bytes(&self) -> Vec<u8> {
        match self {
            Shape::Circle(c) => raw(c).to_vec(),
            Shape::Aabb(b) => raw(b).to_vec(),
            Shape::Capsule(c) => raw(c).to_vec(),
        }
    }
    /// Same shape translated by `d`.
    fn translated(&self, d: c2v) -> Shape {
        let mv = |p: c2v| c2v {
            x: p.x + d.x,
            y: p.y + d.y,
        };
        match *self {
            Shape::Circle(c) => Shape::Circle(c2Circle { p: mv(c.p), r: c.r }),
            Shape::Aabb(b) => Shape::Aabb(c2AABB {
                min: mv(b.min),
                max: mv(b.max),
            }),
            Shape::Capsule(c) => Shape::Capsule(c2Capsule {
                a: mv(c.a),
                b: mv(c.b),
                r: c.r,
            }),
        }
    }
}

/// Number of vertices `c2MakeProxy` actually initialises for each type
/// (`lib.c:108`).  Anything at or beyond this index is uninitialised stack.
fn proxy_vert_count(ty: C2_TYPE) -> u32 {
    match ty {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_CAPSULE => 2,
        C2_TYPE_AABB => 4,
        _ => 0,
    }
}

fn rand_shape(rng: &mut Rng, ty: C2_TYPE) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(rng.circle()),
        C2_TYPE_AABB => Shape::Aabb(rng.aabb()),
        _ => Shape::Capsule(rng.capsule()),
    }
}

/// Small, well-conditioned shape near the origin (so overlap is likely).
fn near_shape(rng: &mut Rng, ty: C2_TYPE, spread: f32, rad_lo: f32, rad_hi: f32) -> Shape {
    let rad = if rad_lo == rad_hi { rad_lo } else { rng.range(rad_lo, rad_hi) };
    let p = c2v {
        x: rng.range(-spread, spread),
        y: rng.range(-spread, spread),
    };
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p, r: rad }),
        C2_TYPE_AABB => {
            let h = rng.range(0.1, spread.max(0.2));
            let w = rng.range(0.1, spread.max(0.2));
            Shape::Aabb(c2AABB {
                min: c2v {
                    x: p.x - w,
                    y: p.y - h,
                },
                max: c2v {
                    x: p.x + w,
                    y: p.y + h,
                },
            })
        }
        _ => {
            let d = c2v {
                x: rng.range(-spread, spread),
                y: rng.range(-spread, spread),
            };
            Shape::Capsule(c2Capsule {
                a: p,
                b: c2v {
                    x: p.x + d.x,
                    y: p.y + d.y,
                },
                r: rad,
            })
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Outcome {
    dist: f32,
    iters: c_int,
}

#[derive(Default)]
struct Cov {
    hit_zero: usize,
    nonzero: usize,
    iter_cap: usize,
    iter_low: usize,
    radius_sub: usize,
    midpoint: usize,
}

impl Cov {
    fn note(&mut self, o: Outcome, use_radius: c_int) {
        if o.dist == 0.0 {
            self.hit_zero += 1;
        } else {
            self.nonzero += 1;
        }
        if o.iters >= 20 {
            self.iter_cap += 1;
        } else {
            self.iter_low += 1;
        }
        if use_radius != 0 {
            if o.dist > 0.0 {
                self.radius_sub += 1;
            } else {
                self.midpoint += 1;
            }
        }
    }
    fn report(&self, name: &str) {
        eprintln!(
            "[coverage] {name}: dist==0 {} / dist!=0 {} / iter==20 {} / iter<20 {} / radius-sub {} / midpoint {}",
            self.hit_zero, self.nonzero, self.iter_cap, self.iter_low, self.radius_sub, self.midpoint
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn gjk_case(
    a: Shape,
    ax: Option<c2x>,
    b: Shape,
    bx: Option<c2x>,
    use_radius: c_int,
    with_outa: bool,
    with_outb: bool,
    with_iters: bool,
    cache: Option<c2GJKCache>,
    ctx: &str,
) -> (Outcome, Option<c2GJKCache>) {
    let (cf, rf) = fnpair!("c2GJK", FnGJK);

    let ab = a.bytes();
    let bb = b.bytes();
    // separate copies so we can also prove neither library writes to them
    let mut abuf_c = ab.clone();
    let mut abuf_r = ab.clone();
    let mut bbuf_c = bb.clone();
    let mut bbuf_r = bb.clone();

    let poison = c2v {
        x: f32::from_bits(0x1234_5678),
        y: f32::from_bits(0x8765_4321),
    };
    let (mut coa, mut cob) = (poison, poison);
    let (mut roa, mut rob) = (poison, poison);
    let (mut cit, mut rit) = (-12345i32, -12345i32);
    let mut cch = cache.unwrap_or_default();
    let mut rch = cache.unwrap_or_default();

    let axp = ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);

    let (cd, rd) = unsafe {
        let cd = cf(
            abuf_c.as_mut_ptr() as *const c_void,
            a.ty(),
            axp,
            bbuf_c.as_mut_ptr() as *const c_void,
            b.ty(),
            bxp,
            if with_outa { &mut coa } else { std::ptr::null_mut() },
            if with_outb { &mut cob } else { std::ptr::null_mut() },
            use_radius,
            if with_iters {
                &mut cit
            } else {
                std::ptr::null_mut()
            },
            if cache.is_some() {
                &mut cch
            } else {
                std::ptr::null_mut()
            },
        );
        let rd = rf(
            abuf_r.as_mut_ptr() as *const c_void,
            a.ty(),
            axp,
            bbuf_r.as_mut_ptr() as *const c_void,
            b.ty(),
            bxp,
            if with_outa { &mut roa } else { std::ptr::null_mut() },
            if with_outb { &mut rob } else { std::ptr::null_mut() },
            use_radius,
            if with_iters {
                &mut rit
            } else {
                std::ptr::null_mut()
            },
            if cache.is_some() {
                &mut rch
            } else {
                std::ptr::null_mut()
            },
        );
        (cd, rd)
    };

    let full = format!("c2GJK {ctx} A={a:?} ax={ax:?} B={b:?} bx={bx:?} use_radius={use_radius}");
    eq_f32(&format!("{full} [return]"), cd, rd);
    eq_raw(&format!("{full} [outA]"), &coa, &roa);
    eq_raw(&format!("{full} [outB]"), &cob, &rob);
    eq_int(&format!("{full} [iterations]"), cit, rit);
    eq_raw(&format!("{full} [cache]"), &cch, &rch);
    assert_eq!(abuf_c, abuf_r, "{full} [shape A modified differently]");
    assert_eq!(bbuf_c, bbuf_r, "{full} [shape B modified differently]");
    assert_eq!(abuf_c, ab, "{full} [shape A must be const]");
    assert_eq!(bbuf_c, bb, "{full} [shape B must be const]");
    if !with_outa {
        eq_raw(&format!("{full} [outA NULL untouched]"), &coa, &poison);
    }
    if !with_iters {
        eq_int(&format!("{full} [iters NULL untouched]"), cit, -12345);
    }

    (
        Outcome {
            dist: cd,
            iters: cit,
        },
        if cache.is_some() { Some(cch) } else { None },
    )
}

// ---------------------------------------------------------------------------
// rows 40–42 — CIRCLE × CIRCLE
// ---------------------------------------------------------------------------

#[test]
fn rows40to42_circle_circle() {
    let mut rng = Rng::new(SEED ^ 40);
    let mut cov = Cov::default();
    for i in 0..N {
        for &ur in &[0i32, 1] {
            // overlapping
            let a = near_shape(&mut rng, C2_TYPE_CIRCLE, 1.0, 0.5, 3.0);
            let b = near_shape(&mut rng, C2_TYPE_CIRCLE, 1.0, 0.5, 3.0);
            let (o, _) = gjk_case(
                a,
                None,
                b,
                None,
                ur,
                true,
                true,
                true,
                None,
                &format!("cc-overlap #{i}"),
            );
            cov.note(o, ur);
            // separated
            let a = near_shape(&mut rng, C2_TYPE_CIRCLE, 0.5, 0.0, 0.5);
            let b = Shape::Circle(c2Circle {
                p: c2v {
                    x: rng.range(20.0, 100.0),
                    y: rng.range(-100.0, 100.0),
                },
                r: rng.range(0.0, 0.5),
            });
            let (o, _) = gjk_case(
                a,
                None,
                b,
                None,
                ur,
                true,
                true,
                true,
                None,
                &format!("cc-far #{i}"),
            );
            cov.note(o, ur);
        }
    }
    cov.report("circle-circle");
    assert!(cov.hit_zero > 100 && cov.nonzero > 100);
    assert!(cov.radius_sub > 100 && cov.midpoint > 100);
}

// ---------------------------------------------------------------------------
// rows 43–49 — every ordered type pair, near and far, use_radius 0/1
// ---------------------------------------------------------------------------

#[test]
fn rows43to49_all_ordered_pairs_near_and_far() {
    let mut rng = Rng::new(SEED ^ 43);
    for &ta in ALL_TYPES.iter() {
        for &tb in ALL_TYPES.iter() {
            let mut cov = Cov::default();
            for i in 0..N {
                for &ur in &[0i32, 1] {
                    // overlapping / touching
                    let a = near_shape(&mut rng, ta, 1.0, 0.0, 2.0);
                    let b = near_shape(&mut rng, tb, 1.0, 0.0, 2.0);
                    let (o, _) = gjk_case(
                        a,
                        None,
                        b,
                        None,
                        ur,
                        true,
                        true,
                        true,
                        None,
                        &format!("{ta}x{tb} near #{i}"),
                    );
                    cov.note(o, ur);
                    // clearly separated
                    let a = near_shape(&mut rng, ta, 0.5, 0.0, 0.5);
                    let far = near_shape(&mut rng, tb, 0.5, 0.0, 0.5).translated(c2v {
                        x: rng.range(30.0, 200.0),
                        y: rng.range(-200.0, 200.0),
                    });
                    let (o, _) = gjk_case(
                        a,
                        None,
                        far,
                        None,
                        ur,
                        true,
                        true,
                        true,
                        None,
                        &format!("{ta}x{tb} far #{i}"),
                    );
                    cov.note(o, ur);
                }
            }
            cov.report(&format!("pair {ta}x{tb}"));
            assert!(
                cov.hit_zero > 10 && cov.nonzero > 10,
                "pair {ta}x{tb} did not reach both overlapping and separated outcomes"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 50 — all 9 pairs, fully randomised (wide value range), use_radius 0/1
// ---------------------------------------------------------------------------

#[test]
fn row50_all_pairs_fully_random() {
    let mut rng = Rng::new(SEED ^ 50);
    let mut cov = Cov::default();
    for i in 0..(N * 4) {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let a = rand_shape(&mut rng, ta);
        let b = rand_shape(&mut rng, tb);
        let ur = if rng.bool() { 1 } else { 0 };
        let (o, _) = gjk_case(
            a,
            None,
            b,
            None,
            ur,
            true,
            true,
            true,
            None,
            &format!("rand #{i} {ta}x{tb}"),
        );
        cov.note(o, ur);
    }
    cov.report("fully-random");
}

// ---------------------------------------------------------------------------
// rows 51–55 — transforms
// ---------------------------------------------------------------------------

#[test]
fn rows51to55_transforms() {
    let mut rng = Rng::new(SEED ^ 51);
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    let mut cov = Cov::default();
    for i in 0..N {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let a = near_shape(&mut rng, ta, 1.5, 0.0, 1.5);
        let b = near_shape(&mut rng, tb, 1.5, 0.0, 1.5);
        let ur = if rng.bool() { 1 } else { 0 };

        let th = rng.range(-7.0, 7.0);
        let rot = c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: c2r {
                c: th.cos(),
                s: th.sin(),
            },
        };
        let trans = c2x {
            p: rng.v(),
            r: c2r { c: 1.0, s: 0.0 },
        };
        let rt = c2x {
            p: rng.v(),
            r: c2r {
                c: th.cos(),
                s: th.sin(),
            },
        };
        let nonunit = c2x {
            p: rng.v(),
            r: c2r {
                c: rng.range(-3.0, 3.0),
                s: rng.range(-3.0, 3.0),
            },
        };

        // row 51: identity + NULL
        let (o, _) = gjk_case(
            a,
            Some(ident),
            b,
            None,
            ur,
            true,
            true,
            true,
            None,
            &format!("ident/null #{i}"),
        );
        cov.note(o, ur);
        // row 51b: NULL + identity
        let (o, _) = gjk_case(
            a,
            None,
            b,
            Some(ident),
            ur,
            true,
            true,
            true,
            None,
            &format!("null/ident #{i}"),
        );
        cov.note(o, ur);
        // row 52: translation × translation
        let (o, _) = gjk_case(
            a,
            Some(trans),
            b,
            Some(c2x {
                p: rng.v(),
                r: c2r { c: 1.0, s: 0.0 },
            }),
            ur,
            true,
            true,
            true,
            None,
            &format!("trans/trans #{i}"),
        );
        cov.note(o, ur);
        // row 53: rotation × identity
        let (o, _) = gjk_case(
            a,
            Some(rot),
            b,
            Some(ident),
            ur,
            true,
            true,
            true,
            None,
            &format!("rot/ident #{i}"),
        );
        cov.note(o, ur);
        // row 54: rotation+translation on both
        let (o, _) = gjk_case(
            a,
            Some(rt),
            b,
            Some(c2x {
                p: rng.v(),
                r: c2r {
                    c: (th * 0.5).cos(),
                    s: (th * 0.5).sin(),
                },
            }),
            ur,
            true,
            true,
            true,
            None,
            &format!("rt/rt #{i}"),
        );
        cov.note(o, ur);
        // row 55: non-unit rotation (scaling)
        let (o, _) = gjk_case(
            a,
            Some(nonunit),
            b,
            Some(rng.x()),
            ur,
            true,
            true,
            true,
            None,
            &format!("nonunit #{i}"),
        );
        cov.note(o, ur);
    }
    cov.report("transforms");
    assert!(cov.hit_zero > 100 && cov.nonzero > 100);
}

// ---------------------------------------------------------------------------
// row 56 — cold (zeroed) cache, single call: the cache must be WRITTEN
//          identically.
// ---------------------------------------------------------------------------

#[test]
fn row56_cold_cache() {
    let mut rng = Rng::new(SEED ^ 56);
    let mut wrote_count = [0usize; 5];
    for i in 0..N {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let a = near_shape(&mut rng, ta, 2.0, 0.0, 2.0);
        let b = near_shape(&mut rng, tb, 2.0, 0.0, 2.0);
        let ur = if rng.bool() { 1 } else { 0 };
        let (_, ch) = gjk_case(
            a,
            None,
            b,
            None,
            ur,
            true,
            true,
            true,
            Some(c2GJKCache::default()),
            &format!("cold-cache #{i}"),
        );
        let ch = ch.unwrap();
        if (0..=4).contains(&ch.count) {
            wrote_count[ch.count as usize] += 1;
        }
    }
    eprintln!("[coverage] cold cache written counts (index = cache.count) = {wrote_count:?}");
    assert!(wrote_count[1] > 0, "cache.count==1 never produced");
    assert!(wrote_count[2] > 0, "cache.count==2 never produced");
    assert!(wrote_count[3] > 0, "cache.count==3 never produced");
}

// ---------------------------------------------------------------------------
// rows 57–58 — warm cache carried across repeated / stepped calls
// ---------------------------------------------------------------------------

#[test]
fn row57_warm_cache_repeated_call() {
    let mut rng = Rng::new(SEED ^ 57);
    for i in 0..N {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let a = near_shape(&mut rng, ta, 2.0, 0.0, 2.0);
        let b = near_shape(&mut rng, tb, 2.0, 0.0, 2.0);
        let ur = if rng.bool() { 1 } else { 0 };
        // call 1: cold
        let (_, ch) = gjk_case(
            a,
            None,
            b,
            None,
            ur,
            true,
            true,
            true,
            Some(c2GJKCache::default()),
            &format!("warm1 #{i}"),
        );
        // call 2/3/4: reuse the cache produced by the previous call
        let mut cur = ch.unwrap();
        for step in 0..3 {
            let (_, ch2) = gjk_case(
                a,
                None,
                b,
                None,
                ur,
                true,
                true,
                true,
                Some(cur),
                &format!("warm{} #{i}", step + 2),
            );
            cur = ch2.unwrap();
        }
    }
}

#[test]
fn row58_warm_cache_across_motion() {
    let mut rng = Rng::new(SEED ^ 58);
    for i in 0..(N / 2) {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let a = near_shape(&mut rng, ta, 1.0, 0.0, 1.0);
        let b0 = near_shape(&mut rng, tb, 1.0, 0.0, 1.0);
        let ur = if rng.bool() { 1 } else { 0 };
        let step = c2v {
            x: rng.range(-2.0, 2.0),
            y: rng.range(-2.0, 2.0),
        };
        // a typical consumer loop: one persistent cache, B sweeping past A
        let mut cur = c2GJKCache::default();
        let mut b = b0.translated(c2v {
            x: step.x * -4.0,
            y: step.y * -4.0,
        });
        for k in 0..8 {
            let (_, ch) = gjk_case(
                a,
                None,
                b,
                None,
                ur,
                true,
                true,
                true,
                Some(cur),
                &format!("motion #{i} step={k}"),
            );
            cur = ch.unwrap();
            b = b.translated(step);
        }
    }
}

// ---------------------------------------------------------------------------
// row 59 — hand-crafted caches: count 1..3, in-range indices, random
//          metric/div (drives both sides of the L400 predicate)
// ---------------------------------------------------------------------------

#[test]
fn row59_handcrafted_cache() {
    let mut rng = Rng::new(SEED ^ 59);
    for i in 0..(N * 2) {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let a = near_shape(&mut rng, ta, 2.0, 0.0, 2.0);
        let b = near_shape(&mut rng, tb, 2.0, 0.0, 2.0);
        let ur = if rng.bool() { 1 } else { 0 };
        // The cached indices MUST stay inside the proxy's *initialised* vertex
        // range (1 for a circle, 2 for a capsule, 4 for an AABB).  The C never
        // range-checks them (`pA.verts[iA]`, lib.c:384) and `c2Proxy pA;`
        // (lib.c:371) is an uninitialised stack local, so a larger index reads
        // indeterminate bytes -- that is `ERRORS.md` row 90 (UB), not a
        // behaviour the translation can or should reproduce.
        let na = proxy_vert_count(ta);
        let nb = proxy_vert_count(tb);
        let mut ch = c2GJKCache {
            metric: match rng.below(5) {
                0 => 0.0,
                1 => rng.range(-1e9, -1e7), // straddles the -1.0e8f threshold
                2 => rng.range(-10.0, 10.0),
                3 => f32::NAN,
                _ => rng.range(0.0, 1e6),
            },
            count: 1 + (rng.below(3) as c_int),
            iA: [0; 3],
            iB: [0; 3],
            div: match rng.below(4) {
                0 => 1.0,
                1 => 0.0,
                2 => rng.range(-1e3, 1e3),
                _ => rng.range(1e-6, 1e6),
            },
        };
        for k in 0..3 {
            ch.iA[k] = rng.below(na) as c_int;
            ch.iB[k] = rng.below(nb) as c_int;
        }
        gjk_case(
            a,
            None,
            b,
            None,
            ur,
            true,
            true,
            true,
            Some(ch),
            &format!("handcrafted-cache #{i} {ch:?}"),
        );
    }
}

// ---------------------------------------------------------------------------
// rows 60–61 — out-parameter NULL/non-NULL cross product
// ---------------------------------------------------------------------------

#[test]
fn rows60to61_out_param_combinations() {
    let mut rng = Rng::new(SEED ^ 60);
    for i in 0..(N / 2) {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let a = near_shape(&mut rng, ta, 2.0, 0.0, 2.0);
        let b = near_shape(&mut rng, tb, 2.0, 0.0, 2.0);
        for ur in [0i32, 1] {
            for mask in 0..8u32 {
                for with_cache in [false, true] {
                    gjk_case(
                        a,
                        None,
                        b,
                        None,
                        ur,
                        mask & 1 != 0,
                        mask & 2 != 0,
                        mask & 4 != 0,
                        if with_cache {
                            Some(c2GJKCache::default())
                        } else {
                            None
                        },
                        &format!("outmask #{i} mask={mask} cache={with_cache}"),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 62–64 — identical shapes, degenerate shapes, extreme radii
// ---------------------------------------------------------------------------

#[test]
fn row62_identical_shapes() {
    let mut rng = Rng::new(SEED ^ 62);
    for i in 0..N {
        let ty = ALL_TYPES[rng.below(3) as usize];
        let s = near_shape(&mut rng, ty, 2.0, 0.0, 3.0);
        for ur in [0i32, 1] {
            let (o, _) = gjk_case(
                s,
                None,
                s,
                None,
                ur,
                true,
                true,
                true,
                Some(c2GJKCache::default()),
                &format!("identical #{i} ty={ty}"),
            );
            // identical shapes must be reported as touching (dist 0) by the C,
            // and the Rust already matched bit-for-bit above.
            assert_eq!(o.dist, 0.0, "identical shapes gave dist {}", o.dist);
        }
    }
}

#[test]
fn row63_degenerate_shapes() {
    let mut rng = Rng::new(SEED ^ 63);
    for i in 0..N {
        let p = rng.v();
        let q = rng.v();
        let degens = [
            // zero-radius circle
            Shape::Circle(c2Circle { p, r: 0.0 }),
            Shape::Circle(c2Circle { p, r: -0.0 }),
            // zero-size AABB
            Shape::Aabb(c2AABB { min: p, max: p }),
            // inverted AABB
            Shape::Aabb(c2AABB { min: q, max: p }),
            // degenerate capsule (a == b)
            Shape::Capsule(c2Capsule { a: p, b: p, r: 0.0 }),
            Shape::Capsule(c2Capsule {
                a: p,
                b: p,
                r: rng.range(0.0, 2.0),
            }),
            // zero-radius capsule with extent
            Shape::Capsule(c2Capsule { a: p, b: q, r: 0.0 }),
        ];
        for (j, &x) in degens.iter().enumerate() {
            for (k, &y) in degens.iter().enumerate() {
                for ur in [0i32, 1] {
                    gjk_case(
                        x,
                        None,
                        y,
                        None,
                        ur,
                        true,
                        true,
                        true,
                        Some(c2GJKCache::default()),
                        &format!("degen #{i} {j}x{k}"),
                    );
                }
            }
        }
    }
}

#[test]
fn row64_extreme_radii() {
    let mut rng = Rng::new(SEED ^ 64);
    let radii = [
        0.0f32,
        -0.0,
        f32::MIN_POSITIVE,
        FLT_EPSILON * 0.5,
        FLT_EPSILON,
        FLT_EPSILON * 2.0,
        1e-20,
        1.0,
        1e18,
        1e30,
        f32::MAX,
        -1.0,
        -1e18,
    ];
    for i in 0..(N / 4) {
        let p = c2v {
            x: rng.range(-3.0, 3.0),
            y: rng.range(-3.0, 3.0),
        };
        let q = c2v {
            x: rng.range(-3.0, 3.0),
            y: rng.range(-3.0, 3.0),
        };
        for &ra in radii.iter() {
            for &rb in radii.iter() {
                for ur in [0i32, 1] {
                    gjk_case(
                        Shape::Circle(c2Circle { p, r: ra }),
                        None,
                        Shape::Circle(c2Circle { p: q, r: rb }),
                        None,
                        ur,
                        true,
                        true,
                        true,
                        Some(c2GJKCache::default()),
                        &format!("radii #{i} ra={ra:?} rb={rb:?}"),
                    );
                    gjk_case(
                        Shape::Capsule(c2Capsule { a: p, b: q, r: ra }),
                        None,
                        Shape::Circle(c2Circle { p: q, r: rb }),
                        None,
                        ur,
                        true,
                        true,
                        true,
                        Some(c2GJKCache::default()),
                        &format!("radii-cap #{i} ra={ra:?} rb={rb:?}"),
                    );
                }
            }
        }
    }
}
