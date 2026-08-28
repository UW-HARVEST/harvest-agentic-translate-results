//! Phase B — CONFIGS.md rows 46..56 and 58: the `c2CastRay` dispatcher (all
//! three valid `C2_TYPE` modes) and the `gen_ray` public entry point.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// c2CastRay helpers — one per shape, so `B` really is a `const void *`.
// ---------------------------------------------------------------------------

fn cast_circle(p: &Pair, a: c2Ray, b: c2Circle, ty: c_int) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { (p.c.c2CastRay)(a, (&raw const b) as *const c_void, ty, &mut co) };
    let rr = unsafe { (p.r.c2CastRay)(a, (&raw const b) as *const c_void, ty, &mut ro) };
    (cr, co, rr, ro)
}

fn cast_aabb(p: &Pair, a: c2Ray, b: c2AABB, ty: c_int) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { (p.c.c2CastRay)(a, (&raw const b) as *const c_void, ty, &mut co) };
    let rr = unsafe { (p.r.c2CastRay)(a, (&raw const b) as *const c_void, ty, &mut ro) };
    (cr, co, rr, ro)
}

fn cast_capsule(
    p: &Pair,
    a: c2Ray,
    b: c2Capsule,
    ty: c_int,
) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { (p.c.c2CastRay)(a, (&raw const b) as *const c_void, ty, &mut co) };
    let rr = unsafe { (p.r.c2CastRay)(a, (&raw const b) as *const c_void, ty, &mut ro) };
    (cr, co, rr, ro)
}

// ---------------------------------------------------------------------------
// gen_ray helper
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub struct GenArgs {
    pub f: [f32; 16],
}

impl GenArgs {
    fn call(&self, l: &Lib, o: &mut [c2Raycast; 3]) -> c_int {
        let f = self.f;
        // Split the borrow so three distinct pointers are handed over.
        let (a, rest) = o.split_at_mut(1);
        let (b, c) = rest.split_at_mut(1);
        unsafe {
            (l.gen_ray)(
                &mut a[0], &mut b[0], &mut c[0], f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7],
                f[8], f[9], f[10], f[11], f[12], f[13], f[14], f[15],
            )
        }
    }
}

struct GenOut {
    ret: c_int,
    out: [c2Raycast; 3],
}

fn gen_pair(p: &Pair, args: &GenArgs) -> (GenOut, GenOut) {
    let mut co = [sentinel(); 3];
    let mut ro = [sentinel(); 3];
    let cr = args.call(&p.c, &mut co);
    let rr = args.call(&p.r, &mut ro);
    (
        GenOut { ret: cr, out: co },
        GenOut { ret: rr, out: ro },
    )
}

fn check_gen(d: &mut Diff, p: &Pair, args: &GenArgs) -> c_int {
    let (c, r) = gen_pair(p, args);
    d.eq_i(|| format!("gen_ray{:?} ret", args.f), c.ret, r.ret);
    for i in 0..3 {
        let ci = c.out[i];
        let ri = r.out[i];
        d.eq_f32(
            || format!("gen_ray{:?} cast{}.t", args.f, i + 1),
            ci.t,
            ri.t,
        );
        d.eq_v(
            || format!("gen_ray{:?} cast{}.n", args.f, i + 1),
            ci.n,
            ri.n,
        );
    }
    c.ret
}

// ===========================================================================
// Rows 46, 47, 48 — c2CastRay in each of its three valid modes
// ===========================================================================

fn rand_ray(rng: &mut Rng, scale: f32) -> c2Ray {
    c2Ray {
        p: rng.vec_uniform(scale),
        d: rng.vec_uniform(scale),
        t: rng.uniform(scale),
    }
}

fn normalized_ray(rng: &mut Rng, scale: f32) -> c2Ray {
    let p = rng.vec_uniform(scale);
    let target = rng.vec_uniform(scale);
    let dx = target.x - p.x;
    let dy = target.y - p.y;
    let l = (dx * dx + dy * dy).sqrt();
    let d = v(dx / l, dy / l);
    c2Ray {
        p,
        d,
        t: (target.x * d.x + target.y * d.y) - (p.x * d.x + p.y * d.y),
    }
}

fn rand_box(rng: &mut Rng, scale: f32) -> c2AABB {
    let a = rng.vec_uniform(scale);
    let b = rng.vec_uniform(scale);
    c2AABB {
        min: v(a.x.min(b.x), a.y.min(b.y)),
        max: v(a.x.max(b.x), a.y.max(b.y)),
    }
}

#[test]
fn cfg_46_castray_circle() {
    let p = load();
    let mut d = Diff::new("cfg_46_castray_circle");
    let mut rng = Rng::new(0x4646);
    let mut hits = 0u32;
    for scale in [1e-3f32, 1.0, 1e3, 1e15] {
        for _ in 0..30_000 {
            for a in [rand_ray(&mut rng, scale), normalized_ray(&mut rng, scale)] {
                let b = c2Circle {
                    p: rng.vec_uniform(scale),
                    r: rng.positive(scale),
                };
                let (cr, co, rr, ro) = cast_circle(&p, a, b, C2_TYPE_CIRCLE);
                d.eq_cast(
                    || format!("c2CastRay(CIRCLE, {:?}, {:?})", a, b),
                    cr,
                    &co,
                    rr,
                    &ro,
                );
                hits += (cr == 1) as u32;
            }
        }
    }
    // Special / bit-pattern populations through the dispatcher.
    for _ in 0..80_000 {
        let a = c2Ray {
            p: rng.vec_spicy(10.0),
            d: rng.vec_spicy(10.0),
            t: rng.spicy(10.0),
        };
        let b = c2Circle {
            p: rng.vec_spicy(10.0),
            r: rng.spicy(10.0),
        };
        let (cr, co, rr, ro) = cast_circle(&p, a, b, C2_TYPE_CIRCLE);
        d.eq_cast(
            || format!("c2CastRay(CIRCLE, {:?}, {:?})", a, b),
            cr,
            &co,
            rr,
            &ro,
        );
    }
    for _ in 0..80_000 {
        let a = c2Ray {
            p: rng.vec_bits(),
            d: rng.vec_bits(),
            t: rng.any_bits(),
        };
        let b = c2Circle {
            p: rng.vec_bits(),
            r: rng.any_bits(),
        };
        let (cr, co, rr, ro) = cast_circle(&p, a, b, C2_TYPE_CIRCLE);
        d.eq_cast(
            || format!("c2CastRay(CIRCLE, {:?}, {:?})", a, b),
            cr,
            &co,
            rr,
            &ro,
        );
    }
    assert!(hits > 100, "dispatcher never produced a circle hit");
    d.finish();
}

#[test]
fn cfg_47_castray_aabb() {
    let p = load();
    let mut d = Diff::new("cfg_47_castray_aabb");
    let mut rng = Rng::new(0x4747);
    let mut hits = 0u32;
    for scale in [1e-3f32, 1.0, 1e3, 1e15] {
        for _ in 0..30_000 {
            for a in [rand_ray(&mut rng, scale), normalized_ray(&mut rng, scale)] {
                let b = rand_box(&mut rng, scale);
                let (cr, co, rr, ro) = cast_aabb(&p, a, b, C2_TYPE_AABB);
                d.eq_cast(
                    || format!("c2CastRay(AABB, {:?}, {:?})", a, b),
                    cr,
                    &co,
                    rr,
                    &ro,
                );
                hits += (cr == 1) as u32;
            }
        }
    }
    for _ in 0..80_000 {
        let a = c2Ray {
            p: rng.vec_spicy(10.0),
            d: rng.vec_spicy(10.0),
            t: rng.spicy(10.0),
        };
        let b = c2AABB {
            min: rng.vec_spicy(10.0),
            max: rng.vec_spicy(10.0),
        };
        let (cr, co, rr, ro) = cast_aabb(&p, a, b, C2_TYPE_AABB);
        d.eq_cast(
            || format!("c2CastRay(AABB, {:?}, {:?})", a, b),
            cr,
            &co,
            rr,
            &ro,
        );
    }
    for _ in 0..80_000 {
        let a = c2Ray {
            p: rng.vec_bits(),
            d: rng.vec_bits(),
            t: rng.any_bits(),
        };
        let b = c2AABB {
            min: rng.vec_bits(),
            max: rng.vec_bits(),
        };
        let (cr, co, rr, ro) = cast_aabb(&p, a, b, C2_TYPE_AABB);
        d.eq_cast(
            || format!("c2CastRay(AABB, {:?}, {:?})", a, b),
            cr,
            &co,
            rr,
            &ro,
        );
    }
    assert!(hits > 100, "dispatcher never produced an AABB hit");
    d.finish();
}

#[test]
fn cfg_48_castray_capsule() {
    let p = load();
    let mut d = Diff::new("cfg_48_castray_capsule");
    let mut rng = Rng::new(0x4848);
    let mut hits = 0u32;
    for scale in [1e-3f32, 1.0, 1e3, 1e15] {
        for _ in 0..30_000 {
            for a in [rand_ray(&mut rng, scale), normalized_ray(&mut rng, scale)] {
                let b = c2Capsule {
                    a: rng.vec_uniform(scale),
                    b: rng.vec_uniform(scale),
                    r: rng.positive(scale * 0.5),
                };
                let (cr, co, rr, ro) = cast_capsule(&p, a, b, C2_TYPE_CAPSULE);
                d.eq_cast(
                    || format!("c2CastRay(CAPSULE, {:?}, {:?})", a, b),
                    cr,
                    &co,
                    rr,
                    &ro,
                );
                hits += (cr == 1) as u32;
            }
        }
    }
    for _ in 0..80_000 {
        let a = c2Ray {
            p: rng.vec_spicy(10.0),
            d: rng.vec_spicy(10.0),
            t: rng.spicy(10.0),
        };
        let b = c2Capsule {
            a: rng.vec_spicy(10.0),
            b: rng.vec_spicy(10.0),
            r: rng.spicy(10.0),
        };
        let (cr, co, rr, ro) = cast_capsule(&p, a, b, C2_TYPE_CAPSULE);
        d.eq_cast(
            || format!("c2CastRay(CAPSULE, {:?}, {:?})", a, b),
            cr,
            &co,
            rr,
            &ro,
        );
    }
    for _ in 0..80_000 {
        let a = c2Ray {
            p: rng.vec_bits(),
            d: rng.vec_bits(),
            t: rng.any_bits(),
        };
        let b = c2Capsule {
            a: rng.vec_bits(),
            b: rng.vec_bits(),
            r: rng.any_bits(),
        };
        let (cr, co, rr, ro) = cast_capsule(&p, a, b, C2_TYPE_CAPSULE);
        d.eq_cast(
            || format!("c2CastRay(CAPSULE, {:?}, {:?})", a, b),
            cr,
            &co,
            rr,
            &ro,
        );
    }
    assert!(hits > 100, "dispatcher never produced a capsule hit");
    d.finish();
}

// ===========================================================================
// Rows 49..55 — gen_ray
// ===========================================================================

#[test]
fn cfg_49_gen_ray_uniform_random() {
    let p = load();
    let mut d = Diff::new("cfg_49_gen_ray_uniform_random");
    let mut rng = Rng::new(0x4949);
    for scale in [1e-3f32, 1.0, 1e2, 1e6, 1e18] {
        for _ in 0..30_000 {
            let mut f = [0f32; 16];
            for slot in f.iter_mut() {
                *slot = rng.uniform(scale);
            }
            check_gen(&mut d, &p, &GenArgs { f });
        }
    }
    d.finish();
}

#[test]
fn cfg_50_gen_ray_realistic() {
    let p = load();
    let mut d = Diff::new("cfg_50_gen_ray_realistic");
    let mut rng = Rng::new(0x5050);
    let mut hits = 0u32;
    for _ in 0..200_000 {
        // mp / r_p in [-100,100]; circle & capsule radii in (0,50]; a sorted bb.
        let bb0 = rng.vec_uniform(100.0);
        let bb1 = rng.vec_uniform(100.0);
        let f = [
            rng.uniform(100.0), // mp_x
            rng.uniform(100.0), // mp_y
            rng.uniform(100.0), // r_p_x
            rng.uniform(100.0), // r_p_y
            rng.uniform(100.0), // c_p_x
            rng.uniform(100.0), // c_p_y
            rng.positive(50.0), // c_r
            rng.uniform(100.0), // cap_a_x
            rng.uniform(100.0), // cap_a_y
            rng.uniform(100.0), // cap_b_x
            rng.uniform(100.0), // cap_b_y
            rng.positive(50.0), // cap_r
            bb0.x.min(bb1.x),
            bb0.y.min(bb1.y),
            bb0.x.max(bb1.x),
            bb0.y.max(bb1.y),
        ];
        let r = check_gen(&mut d, &p, &GenArgs { f });
        hits += (r != 0) as u32;
    }
    assert!(hits > 1_000, "realistic population barely hit anything ({hits})");
    d.finish();
}

#[test]
fn cfg_51_gen_ray_degenerate_ray() {
    let p = load();
    let mut d = Diff::new("cfg_51_gen_ray_degenerate_ray");
    let mut rng = Rng::new(0x5151);
    for _ in 0..50_000 {
        let mp = rng.vec_uniform(100.0);
        let bb0 = rng.vec_uniform(100.0);
        let bb1 = rng.vec_uniform(100.0);
        // mp == r_p  →  c2Norm((0,0)) = (NaN, NaN)
        let f = [
            mp.x, mp.y, mp.x, mp.y,
            rng.uniform(100.0), rng.uniform(100.0), rng.positive(50.0),
            rng.uniform(100.0), rng.uniform(100.0), rng.uniform(100.0), rng.uniform(100.0),
            rng.positive(50.0),
            bb0.x.min(bb1.x), bb0.y.min(bb1.y), bb0.x.max(bb1.x), bb0.y.max(bb1.y),
        ];
        check_gen(&mut d, &p, &GenArgs { f });
        // Also the -0.0 / +0.0 variant of the same degeneracy.
        let mut f2 = f;
        f2[2] = -mp.x;
        f2[3] = -mp.y;
        check_gen(&mut d, &p, &GenArgs { f: f2 });
    }
    d.finish();
}

/// Row 52 — reach all eight values of the `1|2|4` hit bitmask.
#[test]
fn cfg_52_gen_ray_hit_bitmask() {
    let p = load();
    let mut d = Diff::new("cfg_52_gen_ray_hit_bitmask");
    let mut rng = Rng::new(0x5252);
    let mut seen: BTreeSet<c_int> = BTreeSet::new();

    // Hand-built configurations: ray along +x from the origin towards (10,0).
    // Each shape is then placed either on the ray or far away.
    for k in 0..40_000 {
        let on_circle = k & 1 != 0;
        let on_capsule = k & 2 != 0;
        let on_bb = k & 4 != 0;
        let far = 1e4;
        let f = [
            10.0, 0.0, // mp
            0.0, 0.0, // r_p
            if on_circle { 5.0 } else { far },
            0.0,
            1.0, // c_r
            if on_capsule { 7.0 } else { far },
            -2.0,
            if on_capsule { 7.0 } else { far },
            2.0,
            0.5, // cap_r
            if on_bb { 8.0 } else { far },
            -1.0,
            if on_bb { 9.0 } else { far + 1.0 },
            1.0,
        ];
        let r = check_gen(&mut d, &p, &GenArgs { f });
        seen.insert(r);
    }
    // Randomised sweep so the bitmask is exercised with value-dependent inputs.
    for _ in 0..200_000 {
        let far = 1e4;
        let pick = |b: bool, on: f32, rng: &mut Rng| if b { on + rng.uniform(0.4) } else { far };
        let bits = rng.below(8);
        let f = [
            10.0 + rng.uniform(1.0),
            rng.uniform(1.0),
            rng.uniform(1.0),
            rng.uniform(1.0),
            pick(bits & 1 != 0, 5.0, &mut rng),
            if bits & 1 != 0 { rng.uniform(0.4) } else { far },
            1.0 + rng.positive(0.5),
            pick(bits & 2 != 0, 7.0, &mut rng),
            -2.0,
            pick(bits & 2 != 0, 7.0, &mut rng),
            2.0,
            0.5 + rng.positive(0.3),
            pick(bits & 4 != 0, 8.0, &mut rng),
            -1.0,
            pick(bits & 4 != 0, 9.0, &mut rng) + 1.0,
            1.0,
        ];
        seen.insert(check_gen(&mut d, &p, &GenArgs { f }));
    }
    let want: BTreeSet<c_int> = (0..8).collect();
    assert!(
        want.is_subset(&seen),
        "not all 8 hit bitmask values reached: {:?}",
        seen
    );
    d.finish();
}

#[test]
fn cfg_53_gen_ray_specials() {
    let p = load();
    let mut d = Diff::new("cfg_53_gen_ray_specials");
    let mut rng = Rng::new(0x5353);
    let sp = specials();
    // Sweep each of the 16 argument slots against the whole special pool while
    // the others hold a configuration that hits all three shapes.
    let base = [
        10.0f32, 0.0, 0.0, 0.0, 5.0, 0.0, 1.0, 7.0, -2.0, 7.0, 2.0, 0.5, 8.0, -1.0, 9.0, 1.0,
    ];
    for slot in 0..16usize {
        for &s in &sp {
            for &s2 in &sp {
                let mut f = base;
                f[slot] = s;
                f[(slot + 7) % 16] = s2;
                check_gen(&mut d, &p, &GenArgs { f });
            }
        }
    }
    // Randomised "spicy" and full-bit-pattern populations (row 58).
    for _ in 0..80_000 {
        let mut f = [0f32; 16];
        for slot in f.iter_mut() {
            *slot = rng.spicy(100.0);
        }
        check_gen(&mut d, &p, &GenArgs { f });
    }
    for _ in 0..80_000 {
        let mut f = [0f32; 16];
        for slot in f.iter_mut() {
            *slot = rng.any_bits();
        }
        check_gen(&mut d, &p, &GenArgs { f });
    }
    d.finish();
}

/// Row 54 — out-parameter aliasing: the C writes `cast1`, `cast2`, `cast3` in
/// that order, so with a single shared pointer the last cast wins.
#[test]
fn cfg_54_gen_ray_out_aliasing() {
    let p = load();
    let mut d = Diff::new("cfg_54_gen_ray_out_aliasing");
    let mut rng = Rng::new(0x5454);

    let call_aliased = |l: &Lib, f: &[f32; 16], mode: u8| -> (c_int, [c2Raycast; 3]) {
        let mut a = sentinel();
        let mut b = sentinel();
        let mut c = sentinel();
        let ret = unsafe {
            let (p1, p2, p3): (*mut c2Raycast, *mut c2Raycast, *mut c2Raycast) = match mode {
                0 => (&mut a, &mut a, &mut a),   // full aliasing
                1 => (&mut a, &mut b, &mut a),   // cast1 == cast3
                2 => (&mut a, &mut a, &mut c),   // cast1 == cast2
                _ => (&mut a, &mut b, &mut b),   // cast2 == cast3
            };
            (l.gen_ray)(
                p1, p2, p3, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8], f[9], f[10],
                f[11], f[12], f[13], f[14], f[15],
            )
        };
        (ret, [a, b, c])
    };

    let base = [
        10.0f32, 0.0, 0.0, 0.0, 5.0, 0.0, 1.0, 7.0, -2.0, 7.0, 2.0, 0.5, 8.0, -1.0, 9.0, 1.0,
    ];
    for mode in 0..4u8 {
        for _ in 0..40_000 {
            let mut f = base;
            // jitter every slot so all hit combinations occur
            for slot in f.iter_mut() {
                *slot += rng.uniform(6.0);
            }
            let (cr, co) = call_aliased(&p.c, &f, mode);
            let (rr, ro) = call_aliased(&p.r, &f, mode);
            d.eq_i(|| format!("gen_ray aliased(mode={mode}) {:?} ret", f), cr, rr);
            for i in 0..3 {
                d.eq_f32(
                    || format!("gen_ray aliased(mode={mode}) {:?} slot{}.t", f, i),
                    co[i].t,
                    ro[i].t,
                );
                d.eq_v(
                    || format!("gen_ray aliased(mode={mode}) {:?} slot{}.n", f, i),
                    co[i].n,
                    ro[i].n,
                );
            }
        }
    }
    d.finish();
}

/// Row 55 — configurations that make the derived `ray.t` exactly zero or
/// negative (the mouse point behind / on the ray origin).
#[test]
fn cfg_55_gen_ray_zero_and_negative_t() {
    let p = load();
    let mut d = Diff::new("cfg_55_gen_ray_zero_and_negative_t");
    let mut rng = Rng::new(0x5555);
    for _ in 0..60_000 {
        let bb0 = rng.vec_uniform(50.0);
        let bb1 = rng.vec_uniform(50.0);
        let tail = [
            rng.uniform(50.0),
            rng.uniform(50.0),
            rng.positive(20.0),
            rng.uniform(50.0),
            rng.uniform(50.0),
            rng.uniform(50.0),
            rng.uniform(50.0),
            rng.positive(20.0),
            bb0.x.min(bb1.x),
            bb0.y.min(bb1.y),
            bb0.x.max(bb1.x),
            bb0.y.max(bb1.y),
        ];
        // Axis-aligned mouse/ray pairs → ray.d is an exact unit axis vector and
        // ray.t is an exact float.
        let o = rng.vec_uniform(50.0);
        let cases: [[f32; 4]; 6] = [
            [o.x + 1.0, o.y, o.x, o.y],
            [o.x - 1.0, o.y, o.x, o.y],
            [o.x, o.y + 1.0, o.x, o.y],
            [o.x, o.y - 1.0, o.x, o.y],
            [o.x, o.y, o.x, o.y],             // ray.t = NaN (degenerate)
            [o.x + 1e-30, o.y, o.x, o.y],     // underflowing direction
        ];
        for head in cases {
            let mut f = [0f32; 16];
            f[..4].copy_from_slice(&head);
            f[4..].copy_from_slice(&tail);
            check_gen(&mut d, &p, &GenArgs { f });
        }
    }
    d.finish();
}

/// Row 56 — the sentinel contract, spelled out as its own test: on a *reject*
/// `c2RaytoCircle` / `c2RaytoAABB` must leave `*out` byte-identical, while
/// `c2RaytoCapsule` always writes it. Verified against the C, not assumed.
#[test]
fn cfg_56_out_param_write_contract() {
    let p = load();
    let mut d = Diff::new("cfg_56_out_param_write_contract");
    let mut rng = Rng::new(0x5656);
    let s = sentinel();
    let mut circle_untouched = 0u32;
    let mut aabb_untouched = 0u32;
    let mut capsule_written = 0u32;
    for _ in 0..60_000 {
        let a = normalized_ray(&mut rng, 50.0);
        // A shape guaranteed far away → reject.
        let far = 1e6;
        let ci = c2Circle { p: v(far, far), r: 1.0 };
        let bx = c2AABB { min: v(far, far), max: v(far + 1.0, far + 1.0) };
        let cap = c2Capsule { a: v(far, far), b: v(far, far + 1.0), r: 1.0 };

        let mut co = s;
        let mut ro = s;
        let cr = unsafe { (p.c.c2RaytoCircle)(a, ci, &mut co) };
        let rr = unsafe { (p.r.c2RaytoCircle)(a, ci, &mut ro) };
        d.eq_cast(|| format!("circle reject {:?}", a), cr, &co, rr, &ro);
        if cr == 0 && rcbits(&co) == rcbits(&s) {
            circle_untouched += 1;
        }

        let mut co = s;
        let mut ro = s;
        let cr = unsafe { (p.c.c2RaytoAABB)(a, bx, &mut co) };
        let rr = unsafe { (p.r.c2RaytoAABB)(a, bx, &mut ro) };
        d.eq_cast(|| format!("aabb reject {:?}", a), cr, &co, rr, &ro);
        if cr == 0 && rcbits(&co) == rcbits(&s) {
            aabb_untouched += 1;
        }

        let mut co = s;
        let mut ro = s;
        let cr = unsafe { (p.c.c2RaytoCapsule)(a, cap, &mut co) };
        let rr = unsafe { (p.r.c2RaytoCapsule)(a, cap, &mut ro) };
        d.eq_cast(|| format!("capsule reject {:?}", a), cr, &co, rr, &ro);
        if cr == 0 && rcbits(&co) != rcbits(&s) {
            capsule_written += 1;
        }
    }
    assert!(
        circle_untouched > 0,
        "c2RaytoCircle reject never left *out untouched"
    );
    assert!(
        aabb_untouched > 0,
        "c2RaytoAABB reject never left *out untouched"
    );
    assert!(
        capsule_written > 0,
        "c2RaytoCapsule reject never wrote *out (it always should)"
    );
    d.finish();
}
