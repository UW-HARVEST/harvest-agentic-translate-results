//! High-volume randomized differential sweep across many seeds.
//!
//! `CONFIGS.md`/`ERRORS.md` pin down *which* configurations must be covered;
//! this file is the wide net: it randomises the configuration axes themselves
//! (types, transforms, `use_radius`, cache state, output pointers, geometry
//! regime, magnitude regime) together, so that combinations no single table row
//! names are still exercised.

mod common;

use common::*;
use std::ffi::c_int;

/// Number of independent seeds; each runs `PER_SEED` randomized configurations.
const SEEDS: u64 = 24;
const PER_SEED: usize = 2500;

fn rand_shape(rng: &mut Rng, ty: c_int, centre: c2v, ext: f32, regime: u32) -> Shape {
    match regime {
        // degenerate variants the table calls out, mixed in randomly
        0 => match ty {
            C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: centre, r: 0.0 }),
            C2_TYPE_AABB => Shape::Aabb(c2AABB { min: centre, max: centre }),
            _ => Shape::Capsule(c2Capsule { a: centre, b: centre, r: 0.0 }),
        },
        1 => match ty {
            // inverted / negative-radius variants
            C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: centre, r: -ext }),
            C2_TYPE_AABB => Shape::Aabb(c2AABB {
                min: c2v { x: centre.x + ext, y: centre.y + ext },
                max: c2v { x: centre.x - ext, y: centre.y - ext },
            }),
            _ => Shape::Capsule(c2Capsule {
                a: centre,
                b: c2v { x: centre.x + ext, y: centre.y },
                r: -ext,
            }),
        },
        _ => gen_shape(rng, ty, centre, ext),
    }
}

fn sweep(seed: u64) {
    let p = load_pair();
    let mut rng = Rng::new(seed);
    unsafe {
        for i in 0..PER_SEED {
            let ta = ALL_TYPES[rng.below(3) as usize];
            let tb = ALL_TYPES[rng.below(3) as usize];

            // magnitude regime
            let scale = match rng.below(5) {
                0 => 1.0e-30f32,
                1 => 1.0e-3,
                2 => 1.0,
                3 => 1.0e6,
                _ => 1.0e18,
            };
            let ca = c2v { x: rng.scaled(scale), y: rng.scaled(scale) };
            // separation regime
            let sep = match rng.below(4) {
                0 => 0.0f32,
                1 => scale * 0.05,
                2 => scale * 0.5,
                _ => scale * 8.0,
            };
            let cb = c2v { x: ca.x + sep, y: ca.y + rng.scaled(sep.abs().max(scale * 0.01)) };
            let reg_a = rng.below(6);
            let reg_b = rng.below(6);
            let sa = rand_shape(&mut rng, ta, ca, scale * 0.2, reg_a);
            let sb = rand_shape(&mut rng, tb, cb, scale * 0.2, reg_b);

            // transform regime
            let axv = match rng.below(5) {
                0 => c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } },
                1 => c2x { p: c2v { x: rng.scaled(scale), y: rng.scaled(scale) }, r: c2r { c: 1.0, s: 0.0 } },
                2 => c2x { p: c2v { x: 0.0, y: 0.0 }, r: rng.rot() },
                3 => c2x { p: c2v { x: rng.scaled(scale), y: rng.scaled(scale) }, r: rng.rot() },
                _ => c2x {
                    p: c2v { x: rng.scaled(scale), y: rng.scaled(scale) },
                    r: c2r { c: rng.scaled(4.0), s: rng.scaled(4.0) },
                },
            };
            let bxv = match rng.below(5) {
                0 => c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } },
                1 => c2x { p: c2v { x: rng.scaled(scale), y: rng.scaled(scale) }, r: c2r { c: 1.0, s: 0.0 } },
                2 => c2x { p: c2v { x: 0.0, y: 0.0 }, r: rng.rot() },
                3 => c2x { p: c2v { x: rng.scaled(scale), y: rng.scaled(scale) }, r: rng.rot() },
                _ => c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 0.0, s: 0.0 } },
            };
            let use_ax = rng.below(3) != 0;
            let use_bx = rng.below(3) != 0;
            let ax = if use_ax { Some(&axv) } else { None };
            let bx = if use_bx { Some(&bxv) } else { None };

            let ur: c_int = match rng.below(4) {
                0 => 0,
                1 => 1,
                2 => -1,
                _ => rng.next_u32() as c_int,
            };
            let want_a = rng.below(4) != 0;
            let want_b = rng.below(4) != 0;
            let want_it = rng.below(4) != 0;

            // cache regime: none / cold / hand-built / carried forward
            let cache_mode = rng.below(4);
            let na = match ta { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
            let nb = match tb { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
            let base = match cache_mode {
                0 | 1 => c2GJKCache::default(),
                _ => c2GJKCache {
                    metric: rng.finite(),
                    count: rng.below(4) as c_int,
                    iA: [rng.below(na) as c_int, rng.below(na) as c_int, rng.below(na) as c_int],
                    iB: [rng.below(nb) as c_int, rng.below(nb) as c_int, rng.below(nb) as c_int],
                    div: if rng.below(6) == 0 { 0.0 } else { rng.finite() },
                },
            };
            let mut cc = base;
            let mut cr = base;
            let use_cache = cache_mode != 0;

            let oc = call_gjk(
                &p.c, &sa, ax, &sb, bx, ur, want_a, want_b, want_it,
                if use_cache { Some(&mut cc) } else { None },
            );
            let or = call_gjk(
                &p.r, &sa, ax, &sb, bx, ur, want_a, want_b, want_it,
                if use_cache { Some(&mut cr) } else { None },
            );
            let ctx = format!(
                "seed={seed} #{i} ta={ta} tb={tb} regA={reg_a} regB={reg_b} \
                 scale={scale:e} sep={sep:e} ur={ur} ax={use_ax} bx={use_bx} cache={cache_mode}"
            );
            eq_gjk_out(&ctx, &oc, &or);
            if use_cache {
                eq_cache(&format!("{ctx} writeback"), &cc, &cr);
                // carry the cache forward for a second call (warm reuse)
                let oc2 = call_gjk(&p.c, &sa, ax, &sb, bx, ur, true, true, true, Some(&mut cc));
                let or2 = call_gjk(&p.r, &sa, ax, &sb, bx, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("{ctx} warm"), &oc2, &or2);
                eq_cache(&format!("{ctx} warm writeback"), &cc, &cr);
            }

            // and the public wrapper on the same numbers
            let mut wac = c2v::default();
            let mut wbc = c2v::default();
            let mut war = c2v::default();
            let mut wbr = c2v::default();
            let f: [f32; 9] = [
                ca.x, ca.y, cb.x, cb.y,
                rng.scaled(scale), rng.scaled(scale),
                rng.scaled(scale), rng.scaled(scale),
                rng.scaled(scale).abs(),
            ];
            let rev = (rng.next_u32() & 0xff) as u8 as i8;
            (p.c.gjk)(rev, &mut wac, &mut wbc, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8]);
            (p.r.gjk)(rev, &mut war, &mut wbr, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8]);
            eq_v(&format!("{ctx} wrapper a"), wac, war);
            eq_v(&format!("{ctx} wrapper b"), wbc, wbr);
        }
    }
}

#[test]
fn fuzz_all_axes_many_seeds() {
    for s in 0..SEEDS {
        sweep(0xA5A5_0000 + s * 0x9E37);
    }
}

/// Independent sweep of the low-level entry points with fully random struct
/// contents, including out-of-range `count`s and arbitrary indices.
#[test]
fn fuzz_low_level_entry_points() {
    let p = load_pair();
    let mut rng = Rng::new(0xF00DBEEF);
    unsafe {
        for i in 0..200_000usize {
            match i % 8 {
                0 => {
                    let (a, b) = (rng.v(), rng.v());
                    eq_f32("fuzz dot", (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
                    eq_f32("fuzz det", (p.c.c2Det2)(a, b), (p.r.c2Det2)(a, b));
                    eq_v("fuzz add", (p.c.c2Add)(a, b), (p.r.c2Add)(a, b));
                    eq_v("fuzz sub", (p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b));
                    eq_v("fuzz max", (p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b));
                    eq_v("fuzz min", (p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b));
                }
                1 => {
                    let a = rng.v();
                    let s = rng.finite();
                    eq_v("fuzz mulvs", (p.c.c2Mulvs)(a, s), (p.r.c2Mulvs)(a, s));
                    eq_v("fuzz div", (p.c.c2Div)(a, s), (p.r.c2Div)(a, s));
                    eq_v("fuzz norm", (p.c.c2Norm)(a), (p.r.c2Norm)(a));
                    eq_f32("fuzz len", (p.c.c2Len)(a), (p.r.c2Len)(a));
                }
                2 => {
                    let r = c2r { c: rng.finite(), s: rng.finite() };
                    let b = rng.v();
                    eq_v("fuzz mulrv", (p.c.c2Mulrv)(r, b), (p.r.c2Mulrv)(r, b));
                    eq_v("fuzz mulrvT", (p.c.c2MulrvT)(r, b), (p.r.c2MulrvT)(r, b));
                    let x = c2x { p: rng.v(), r };
                    eq_v("fuzz mulxv", (p.c.c2Mulxv)(x, b), (p.r.c2Mulxv)(x, b));
                }
                3 => {
                    let (a, lo, hi) = (rng.v(), rng.v(), rng.v());
                    eq_v("fuzz clamp", (p.c.c2Clampv)(a, lo, hi), (p.r.c2Clampv)(a, lo, hi));
                    eq_v("fuzz neg", (p.c.c2Neg)(a), (p.r.c2Neg)(a));
                    eq_v("fuzz skew", (p.c.c2Skew)(a), (p.r.c2Skew)(a));
                    eq_v("fuzz ccw", (p.c.c2CCW90)(a), (p.r.c2CCW90)(a));
                }
                4 => {
                    let mut verts = [c2v::default(); 8];
                    for v in verts.iter_mut() {
                        *v = rng.v();
                    }
                    let count = (rng.next_u32() % 11) as c_int - 2;
                    let d = rng.v();
                    eq_i(
                        "fuzz support",
                        (p.c.c2Support)(verts.as_ptr(), count.max(0), d),
                        (p.r.c2Support)(verts.as_ptr(), count.max(0), d),
                    );
                }
                5 => {
                    let mut bb = c2AABB { min: rng.v(), max: rng.v() };
                    let mut oc = [c2v { x: 9.0, y: 8.0 }; 4];
                    let mut orr = oc;
                    (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bb);
                    (p.r.c2BBVerts)(orr.as_mut_ptr(), &mut bb);
                    for k in 0..4 {
                        eq_v("fuzz bbverts", oc[k], orr[k]);
                    }
                }
                6 => {
                    let mut sc = c2Simplex::default();
                    for k in 0..4 {
                        sc.verts[k] = c2sv {
                            sA: rng.v(),
                            sB: rng.v(),
                            p: rng.v(),
                            u: rng.finite(),
                            iA: rng.next_u32() as c_int,
                            iB: rng.next_u32() as c_int,
                        };
                    }
                    sc.div = if rng.below(8) == 0 { 0.0 } else { rng.finite() };
                    sc.count = (rng.next_u32() % 7) as c_int - 1;
                    let mut sr = sc;
                    eq_f32(
                        "fuzz metric",
                        (p.c.c2GJKSimplexMetric)(&mut sc),
                        (p.r.c2GJKSimplexMetric)(&mut sr),
                    );
                    eq_v("fuzz c2D", (p.c.c2D)(&mut sc), (p.r.c2D)(&mut sr));
                    eq_v("fuzz c2L", (p.c.c2L)(&mut sc), (p.r.c2L)(&mut sr));
                    let mut ac = c2v { x: 1.0, y: 2.0 };
                    let mut bc = c2v { x: 3.0, y: 4.0 };
                    let mut ar = ac;
                    let mut br = bc;
                    (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
                    eq_v("fuzz witness a", ac, ar);
                    eq_v("fuzz witness b", bc, br);
                    eq_simplex("fuzz witness struct", &sc, &sr);
                }
                _ => {
                    let mut sc = c2Simplex::default();
                    for k in 0..4 {
                        sc.verts[k] = c2sv {
                            sA: rng.v_coord(),
                            sB: rng.v_coord(),
                            p: rng.v_coord(),
                            u: rng.finite(),
                            iA: rng.below(4) as c_int,
                            iB: rng.below(4) as c_int,
                        };
                    }
                    sc.div = rng.finite();
                    let mut a2 = sc;
                    let mut b2 = sc;
                    a2.count = 2;
                    b2.count = 2;
                    (p.c.c22)(&mut a2);
                    (p.r.c22)(&mut b2);
                    eq_simplex("fuzz c22", &a2, &b2);
                    let mut a3 = sc;
                    let mut b3 = sc;
                    a3.count = 3;
                    b3.count = 3;
                    (p.c.c23)(&mut a3);
                    (p.r.c23)(&mut b3);
                    eq_simplex("fuzz c23", &a3, &b3);
                }
            }
        }
    }
}
