//! Phase B — valid-path differential tests for `c2GJK` and `gjk`
//! (`CONFIGS.md` rows 28-52).

mod common;

use common::*;
use std::ffi::{c_char, c_int};

const N: usize = 1200;

/// Places two shapes with a controlled relationship.
/// `mode`: 0 = far apart, 1 = near, 2 = overlapping, 3 = contained, 4 = coincident centres
fn two_shapes(rng: &mut Rng, ta: c_int, tb: c_int, mode: u32) -> (Shape, Shape) {
    let ca = rng.v_coord();
    match mode {
        0 => {
            let sa = gen_shape(rng, ta, ca, 2.0);
            let cb = c2v {
                x: ca.x + 60.0 + rng.unit().abs() * 100.0,
                y: ca.y + rng.scaled(30.0),
            };
            let sb = gen_shape(rng, tb, cb, 2.0);
            (sa, sb)
        }
        1 => {
            let sa = gen_shape(rng, ta, ca, 2.0);
            let cb = c2v {
                x: ca.x + 4.0 + rng.scaled(0.25),
                y: ca.y + rng.scaled(0.25),
            };
            let sb = gen_shape(rng, tb, cb, 2.0);
            (sa, sb)
        }
        2 => {
            let sa = gen_shape(rng, ta, ca, 5.0);
            let cb = c2v {
                x: ca.x + rng.scaled(2.0),
                y: ca.y + rng.scaled(2.0),
            };
            let sb = gen_shape(rng, tb, cb, 5.0);
            (sa, sb)
        }
        3 => {
            let sa = gen_shape(rng, ta, ca, 20.0);
            let sb = gen_shape(rng, tb, ca, 0.5);
            (sa, sb)
        }
        _ => {
            let sa = gen_shape(rng, ta, ca, 3.0);
            let sb = gen_shape(rng, tb, ca, 3.0);
            (sa, sb)
        }
    }
}

/// Core driver: one configuration, many randomized inputs.
#[allow(clippy::too_many_arguments)]
fn gjk_sweep(
    label: &str,
    seed: u64,
    mode: u32,
    use_radius: c_int,
    transforms: u32, // 0=null,1=identity,2=translate,3=rotate,4=both,5=non-unit
    with_cache: bool,
) {
    let p = load_pair();
    let mut rng = Rng::new(seed);
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let (sa, sb) = two_shapes(&mut rng, ta, tb, mode);

            let (axv, bxv) = match transforms {
                1 => ((p.c.c2xIdentity)(), (p.c.c2xIdentity)()),
                2 => (
                    c2x { p: rng.v_coord(), r: c2r { c: 1.0, s: 0.0 } },
                    c2x { p: rng.v_coord(), r: c2r { c: 1.0, s: 0.0 } },
                ),
                3 => (
                    c2x { p: c2v { x: 0.0, y: 0.0 }, r: rng.rot() },
                    c2x { p: c2v { x: 0.0, y: 0.0 }, r: rng.rot() },
                ),
                4 => (rng.x_transform(), rng.x_transform()),
                5 => (
                    c2x { p: rng.v_coord(), r: c2r { c: rng.scaled(3.0), s: rng.scaled(3.0) } },
                    c2x { p: rng.v_coord(), r: c2r { c: 0.0, s: 0.0 } },
                ),
                _ => ((p.c.c2xIdentity)(), (p.c.c2xIdentity)()),
            };
            let (ax, bx) = if transforms == 0 {
                (None, None)
            } else {
                (Some(&axv), Some(&bxv))
            };

            let mut cc = c2GJKCache::default();
            let mut cr = c2GJKCache::default();
            let oc = call_gjk(
                &p.c, &sa, ax, &sb, bx, use_radius, true, true, true,
                if with_cache { Some(&mut cc) } else { None },
            );
            let or = call_gjk(
                &p.r, &sa, ax, &sb, bx, use_radius, true, true, true,
                if with_cache { Some(&mut cr) } else { None },
            );
            eq_gjk_out(&format!("{label} #{i} ta={ta} tb={tb}"), &oc, &or);
            if with_cache {
                eq_cache(&format!("{label} #{i} writeback"), &cc, &cr);
            }
        }
    }
}

// --- rows 28-32: type cross-product x use_radius x geometry ----------------

#[test]
fn row28_all_type_pairs_separated_radius_on() {
    gjk_sweep("row28", 0x2828, 0, 1, 0, false);
}

#[test]
fn row29_all_type_pairs_separated_radius_off() {
    gjk_sweep("row29", 0x2929, 0, 0, 0, false);
}

#[test]
fn row30_all_type_pairs_overlapping_radius_on() {
    gjk_sweep("row30", 0x3030, 2, 1, 0, false);
}

#[test]
fn row31_all_type_pairs_overlapping_radius_off() {
    gjk_sweep("row31", 0x3131, 2, 0, 0, false);
}

#[test]
fn row32_all_type_pairs_near_touching() {
    gjk_sweep("row32a", 0x3232, 1, 1, 0, false);
    gjk_sweep("row32b", 0x3233, 1, 0, 0, false);
}

// --- rows 33-37: transform axis -------------------------------------------

#[test]
fn row33_explicit_identity_matches_null() {
    let p = load_pair();
    let mut rng = Rng::new(0x3333);
    unsafe {
        let id = (p.c.c2xIdentity)();
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let (sa, sb) = two_shapes(&mut rng, ta, tb, i as u32 % 5);
            let ur = (i % 2) as c_int;
            let oc = call_gjk(&p.c, &sa, Some(&id), &sb, Some(&id), ur, true, true, true, None);
            let or = call_gjk(&p.r, &sa, Some(&id), &sb, Some(&id), ur, true, true, true, None);
            eq_gjk_out(&format!("row33 #{i} explicit-id"), &oc, &or);

            // and explicit identity must equal the NULL-pointer path in BOTH
            let nc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, None);
            let nr = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, None);
            eq_gjk_out(&format!("row33 #{i} null"), &nc, &nr);
            eq_f32(&format!("row33 #{i} C id==null"), oc.dist, nc.dist);
            eq_f32(&format!("row33 #{i} R id==null"), or.dist, nr.dist);
        }
    }
}

#[test]
fn row34_translation_one_side_only() {
    let p = load_pair();
    let mut rng = Rng::new(0x3434);
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let (sa, sb) = two_shapes(&mut rng, ta, tb, i as u32 % 5);
            let t = c2x { p: rng.v_coord(), r: c2r { c: 1.0, s: 0.0 } };
            let ur = (i % 2) as c_int;
            // ax set, bx NULL
            let oc = call_gjk(&p.c, &sa, Some(&t), &sb, None, ur, true, true, true, None);
            let or = call_gjk(&p.r, &sa, Some(&t), &sb, None, ur, true, true, true, None);
            eq_gjk_out(&format!("row34 #{i} ax-only"), &oc, &or);
            // mirror: ax NULL, bx set
            let oc2 = call_gjk(&p.c, &sa, None, &sb, Some(&t), ur, true, true, true, None);
            let or2 = call_gjk(&p.r, &sa, None, &sb, Some(&t), ur, true, true, true, None);
            eq_gjk_out(&format!("row34 #{i} bx-only"), &oc2, &or2);
        }
    }
}

#[test]
fn row35_pure_rotation() {
    gjk_sweep("row35a", 0x3535, 0, 1, 3, false);
    gjk_sweep("row35b", 0x3536, 2, 1, 3, false);
    gjk_sweep("row35c", 0x3537, 1, 0, 3, false);
}

#[test]
fn row36_translation_and_rotation_both() {
    gjk_sweep("row36a", 0x3636, 0, 1, 4, false);
    gjk_sweep("row36b", 0x3637, 2, 1, 4, false);
    gjk_sweep("row36c", 0x3638, 3, 0, 4, false);
    gjk_sweep("row36d", 0x3639, 4, 1, 4, false);
}

#[test]
fn row37_non_unit_and_zero_rotations() {
    gjk_sweep("row37a", 0x3737, 0, 1, 5, false);
    gjk_sweep("row37b", 0x3738, 2, 0, 5, false);
}

// --- rows 38-42: cache axis ------------------------------------------------

#[test]
fn row38_cold_cache_writeback() {
    gjk_sweep("row38a", 0x3838, 0, 1, 0, true);
    gjk_sweep("row38b", 0x3839, 2, 1, 0, true);
    gjk_sweep("row38c", 0x383a, 1, 0, 4, true);
    gjk_sweep("row38d", 0x383b, 4, 1, 4, true);
}

#[test]
fn row39_warm_cache_same_shapes() {
    let p = load_pair();
    let mut rng = Rng::new(0x3939);
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let (sa, sb) = two_shapes(&mut rng, ta, tb, i as u32 % 5);
            let ur = (i % 2) as c_int;
            let mut cc = c2GJKCache::default();
            let mut cr = c2GJKCache::default();
            let o1c = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
            let o1r = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
            eq_gjk_out(&format!("row39 #{i} call1"), &o1c, &o1r);
            eq_cache(&format!("row39 #{i} cache1"), &cc, &cr);
            let o2c = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
            let o2r = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
            eq_gjk_out(&format!("row39 #{i} call2"), &o2c, &o2r);
            eq_cache(&format!("row39 #{i} cache2"), &cc, &cr);
        }
    }
}

#[test]
fn row40_warm_cache_moved_shapes() {
    let p = load_pair();
    let mut rng = Rng::new(0x4040);
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let (sa, sb) = two_shapes(&mut rng, ta, tb, i as u32 % 5);
            let ur = (i % 2) as c_int;
            let mut cc = c2GJKCache::default();
            let mut cr = c2GJKCache::default();
            let x1 = rng.x_transform();
            let o1c = call_gjk(&p.c, &sa, Some(&x1), &sb, None, ur, true, true, true, Some(&mut cc));
            let o1r = call_gjk(&p.r, &sa, Some(&x1), &sb, None, ur, true, true, true, Some(&mut cr));
            eq_gjk_out(&format!("row40 #{i} call1"), &o1c, &o1r);
            eq_cache(&format!("row40 #{i} cache1"), &cc, &cr);
            let x2 = rng.x_transform();
            let o2c = call_gjk(&p.c, &sa, Some(&x2), &sb, None, ur, true, true, true, Some(&mut cc));
            let o2r = call_gjk(&p.r, &sa, Some(&x2), &sb, None, ur, true, true, true, Some(&mut cr));
            eq_gjk_out(&format!("row40 #{i} call2"), &o2c, &o2r);
            eq_cache(&format!("row40 #{i} cache2"), &cc, &cr);
        }
    }
}

#[test]
fn row41_long_warm_cache_chain() {
    let p = load_pair();
    let mut rng = Rng::new(0x4141);
    unsafe {
        for i in 0..(N / 4) {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let (sa, sb) = two_shapes(&mut rng, ta, tb, i as u32 % 5);
            let ur = (i % 2) as c_int;
            let mut cc = c2GJKCache::default();
            let mut cr = c2GJKCache::default();
            let mut ax = rng.x_transform();
            for step in 0..8 {
                ax.p.x += rng.scaled(1.5);
                ax.p.y += rng.scaled(1.5);
                let oc = call_gjk(&p.c, &sa, Some(&ax), &sb, None, ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, &sa, Some(&ax), &sb, None, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row41 #{i} step{step}"), &oc, &or);
                eq_cache(&format!("row41 #{i} step{step} cache"), &cc, &cr);
            }
        }
    }
}

#[test]
fn row42_handbuilt_caches() {
    let p = load_pair();
    let mut rng = Rng::new(0x4242);
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let (sa, sb) = two_shapes(&mut rng, ta, tb, i as u32 % 5);
            let ur = (i % 2) as c_int;
            // proxy vertex counts, so the hand-built indices stay in range
            let na = match ta { C2_TYPE_CIRCLE => 1, C2_TYPE_AABB => 4, _ => 2 };
            let nb = match tb { C2_TYPE_CIRCLE => 1, C2_TYPE_AABB => 4, _ => 2 };
            let count = (i % 3 + 1) as c_int;
            let mut base = c2GJKCache {
                metric: if i % 6 == 0 { 0.0 } else { rng.finite() },
                count,
                iA: [0; 3],
                iB: [0; 3],
                div: if i % 7 == 0 { 0.0 } else { rng.finite() },
            };
            for k in 0..3 {
                base.iA[k] = rng.below(na) as c_int;
                base.iB[k] = rng.below(nb) as c_int;
            }
            let mut cc = base;
            let mut cr = base;
            let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
            let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
            eq_gjk_out(&format!("row42 #{i} count={count}"), &oc, &or);
            eq_cache(&format!("row42 #{i} writeback"), &cc, &cr);
        }
    }
}

// --- row 43: output-pointer combinations -----------------------------------

#[test]
fn row43_output_pointer_combinations() {
    let p = load_pair();
    let mut rng = Rng::new(0x4343);
    unsafe {
        for i in 0..N {
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let (sa, sb) = two_shapes(&mut rng, ta, tb, i as u32 % 5);
            let ur = (i % 2) as c_int;
            for combo in 0..8 {
                let want_a = combo & 1 != 0;
                let want_b = combo & 2 != 0;
                let want_it = combo & 4 != 0;
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, want_a, want_b, want_it, None);
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, want_a, want_b, want_it, None);
                eq_gjk_out(&format!("row43 #{i} combo={combo}"), &oc, &or);
            }
        }
    }
}

// --- rows 44-48: degenerate but valid shapes -------------------------------

#[test]
fn row44_zero_extent_shapes() {
    let p = load_pair();
    let mut rng = Rng::new(0x4444);
    unsafe {
        for i in 0..N {
            let ca = rng.v_coord();
            let cb = c2v { x: ca.x + rng.scaled(8.0), y: ca.y + rng.scaled(8.0) };
            let shapes_a = [
                Shape::Aabb(c2AABB { min: ca, max: ca }),
                Shape::Capsule(c2Capsule { a: ca, b: ca, r: 0.0 }),
                Shape::Circle(c2Circle { p: ca, r: 0.0 }),
            ];
            let shapes_b = [
                Shape::Aabb(c2AABB { min: cb, max: cb }),
                Shape::Capsule(c2Capsule { a: cb, b: cb, r: 0.0 }),
                Shape::Circle(c2Circle { p: cb, r: 0.0 }),
            ];
            let sa = &shapes_a[i % 3];
            let sb = &shapes_b[(i / 3) % 3];
            for &ur in &[0i32, 1] {
                let mut cc = c2GJKCache::default();
                let mut cr = c2GJKCache::default();
                let oc = call_gjk(&p.c, sa, None, sb, None, ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, sa, None, sb, None, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row44 #{i} ur={ur}"), &oc, &or);
                eq_cache(&format!("row44 #{i} ur={ur} cache"), &cc, &cr);
            }
        }
    }
}

#[test]
fn row45_coincident_shapes() {
    let p = load_pair();
    let mut rng = Rng::new(0x4545);
    unsafe {
        for i in 0..N {
            let ty = ALL_TYPES[i % 3];
            let ctr = rng.v_coord();
            // build the exact same shape twice
            let mut r2 = Rng::new(0x5000 + i as u64);
            let sa = gen_shape(&mut r2, ty, ctr, 3.0);
            let mut r3 = Rng::new(0x5000 + i as u64);
            let sb = gen_shape(&mut r3, ty, ctr, 3.0);
            for &ur in &[0i32, 1] {
                let mut cc = c2GJKCache::default();
                let mut cr = c2GJKCache::default();
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row45 #{i} ty={ty} ur={ur}"), &oc, &or);
                eq_cache(&format!("row45 #{i} cache"), &cc, &cr);
            }
        }
    }
}

#[test]
fn row46_containment() {
    gjk_sweep("row46a", 0x4646, 3, 1, 0, true);
    gjk_sweep("row46b", 0x4647, 3, 0, 0, true);
    gjk_sweep("row46c", 0x4648, 3, 1, 4, true);
}

#[test]
fn row47_extreme_magnitudes() {
    let p = load_pair();
    let mut rng = Rng::new(0x4747);
    unsafe {
        for i in 0..N {
            let scale = if i % 2 == 0 { 1.0e18f32 } else { 1.0e-30f32 };
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let ca = c2v { x: rng.scaled(scale), y: rng.scaled(scale) };
            let cb = c2v { x: rng.scaled(scale), y: rng.scaled(scale) };
            let sa = gen_shape(&mut rng, ta, ca, scale * 0.1);
            let sb = gen_shape(&mut rng, tb, cb, scale * 0.1);
            for &ur in &[0i32, 1] {
                let mut cc = c2GJKCache::default();
                let mut cr = c2GJKCache::default();
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, Some(&mut cc));
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, Some(&mut cr));
                eq_gjk_out(&format!("row47 #{i} scale={scale:e} ur={ur}"), &oc, &or);
                eq_cache(&format!("row47 #{i} cache"), &cc, &cr);
            }
        }
    }
}

#[test]
fn row48_radius_dominated() {
    let p = load_pair();
    let mut rng = Rng::new(0x4848);
    unsafe {
        for i in 0..N {
            let ca = rng.v_coord();
            let cb = c2v { x: ca.x + rng.scaled(10.0), y: ca.y + rng.scaled(10.0) };
            let big = 50.0 + rng.unit().abs() * 500.0;
            let sa = Shape::Capsule(c2Capsule {
                a: ca,
                b: c2v { x: ca.x + 0.01, y: ca.y - 0.01 },
                r: big,
            });
            let sb = Shape::Circle(c2Circle { p: cb, r: big * 0.5 });
            for &ur in &[0i32, 1] {
                let oc = call_gjk(&p.c, &sa, None, &sb, None, ur, true, true, true, None);
                let or = call_gjk(&p.r, &sa, None, &sb, None, ur, true, true, true, None);
                eq_gjk_out(&format!("row48 #{i} ur={ur}"), &oc, &or);
                // and the reverse pairing
                let oc2 = call_gjk(&p.c, &sb, None, &sa, None, ur, true, true, true, None);
                let or2 = call_gjk(&p.r, &sb, None, &sa, None, ur, true, true, true, None);
                eq_gjk_out(&format!("row48 #{i} rev ur={ur}"), &oc2, &or2);
            }
        }
    }
}

// --- rows 49-52: the `gjk` wrapper ----------------------------------------

#[allow(clippy::too_many_arguments)]
unsafe fn call_wrapper(
    im: &Impl,
    reverse: c_char,
    bb: &c2AABB,
    cap: &c2Capsule,
) -> (c2v, c2v) {
    unsafe {
        let mut a = c2v { x: 31.5, y: -13.25 };
        let mut b = c2v { x: -6.75, y: 42.0 };
        (im.gjk)(
            reverse,
            &mut a,
            &mut b,
            bb.min.x,
            bb.min.y,
            bb.max.x,
            bb.max.y,
            cap.a.x,
            cap.a.y,
            cap.b.x,
            cap.b.y,
            cap.r,
        );
        (a, b)
    }
}

fn wrapper_sweep(label: &str, seed: u64, reverses: &[c_char], degenerate: bool) {
    let p = load_pair();
    let mut rng = Rng::new(seed);
    unsafe {
        for i in 0..N {
            let ctr = rng.v_coord();
            let bb = if degenerate {
                match i % 3 {
                    0 => c2AABB { min: ctr, max: ctr },
                    1 => c2AABB {
                        min: c2v { x: ctr.x + 5.0, y: ctr.y + 5.0 },
                        max: ctr,
                    },
                    _ => c2AABB { min: ctr, max: c2v { x: ctr.x, y: ctr.y + 4.0 } },
                }
            } else {
                let hx = rng.unit().abs() * 8.0 + 0.001;
                let hy = rng.unit().abs() * 8.0 + 0.001;
                c2AABB {
                    min: c2v { x: ctr.x - hx, y: ctr.y - hy },
                    max: c2v { x: ctr.x + hx, y: ctr.y + hy },
                }
            };
            let ca = c2v { x: ctr.x + rng.scaled(25.0), y: ctr.y + rng.scaled(25.0) };
            let cap = if degenerate {
                match i % 3 {
                    0 => c2Capsule { a: ca, b: ca, r: 0.0 },
                    1 => c2Capsule { a: ca, b: ca, r: rng.unit().abs() * 5.0 },
                    _ => c2Capsule {
                        a: ca,
                        b: c2v { x: ca.x + rng.scaled(6.0), y: ca.y + rng.scaled(6.0) },
                        r: 0.0,
                    },
                }
            } else {
                c2Capsule {
                    a: ca,
                    b: c2v { x: ca.x + rng.scaled(10.0), y: ca.y + rng.scaled(10.0) },
                    r: rng.unit().abs() * 6.0,
                }
            };
            for &rev in reverses {
                let (ac, bc) = call_wrapper(&p.c, rev, &bb, &cap);
                let (ar, br) = call_wrapper(&p.r, rev, &bb, &cap);
                eq_v(&format!("{label} #{i} rev={rev} a"), ac, ar);
                eq_v(&format!("{label} #{i} rev={rev} b"), bc, br);
            }
        }
    }
}

#[test]
fn row49_gjk_wrapper_forward() {
    wrapper_sweep("row49", 0x4949, &[0], false);
}

#[test]
fn row50_gjk_wrapper_reverse_nonzero() {
    wrapper_sweep("row50", 0x5050, &[1, 2, -1, 0x7f], false);
}

#[test]
fn row51_gjk_wrapper_geometry_sweep() {
    let p = load_pair();
    let mut rng = Rng::new(0x5151);
    unsafe {
        for i in 0..(N * 2) {
            let ctr = rng.v_coord();
            let hx = rng.unit().abs() * 6.0 + 0.01;
            let hy = rng.unit().abs() * 6.0 + 0.01;
            let bb = c2AABB {
                min: c2v { x: ctr.x - hx, y: ctr.y - hy },
                max: c2v { x: ctr.x + hx, y: ctr.y + hy },
            };
            // separated / touching / overlapping / contained
            let (off, rad) = match i % 4 {
                0 => (40.0f32, 1.0f32),
                1 => (hx + 2.0, 2.0),
                2 => (hx * 0.3, 1.0),
                _ => (0.0, hx * 4.0),
            };
            let ca = c2v { x: ctr.x + off, y: ctr.y + rng.scaled(1.0) };
            let cap = c2Capsule {
                a: ca,
                b: c2v { x: ca.x + rng.scaled(4.0), y: ca.y + rng.scaled(4.0) },
                r: rad,
            };
            for &rev in &[0i8, 1] {
                let (ac, bc) = call_wrapper(&p.c, rev, &bb, &cap);
                let (ar, br) = call_wrapper(&p.r, rev, &bb, &cap);
                eq_v(&format!("row51 #{i} rev={rev} a"), ac, ar);
                eq_v(&format!("row51 #{i} rev={rev} b"), bc, br);
            }
        }
    }
}

#[test]
fn row52_gjk_wrapper_degenerate() {
    wrapper_sweep("row52", 0x5252, &[0, 1], true);
}
