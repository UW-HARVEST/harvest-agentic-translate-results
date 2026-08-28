//! Phase B, addendum — NaN-payload discrimination stress test.
//!
//! `mulss`/`addss`/`subss`/`divss` return the **destination** operand when both
//! operands are NaN. That makes the C's operand order observable, but *only*
//! when the two NaNs carry different payloads. A fuzzer that uses a single
//! `f32::NAN` value can therefore never see it.
//!
//! These tests draw every float slot independently from a pool of many
//! *distinct* NaN payloads (mixed with normals so branches are still reachable),
//! which is what pins the `fadd`/`fmul` operand order in `src/lib.rs` down.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// 32 mutually distinguishable NaNs (both signs, quiet and signalling) plus a
/// handful of normals so that ordered comparisons can still succeed.
fn nan_pool() -> Vec<f32> {
    let mut out = Vec::new();
    for i in 0..12u32 {
        out.push(f32::from_bits(0x7fc0_0000 | (i * 0x1357 + 1))); // +qNaN
        out.push(f32::from_bits(0xffc0_0000 | (i * 0x2468 + 1))); // -qNaN
        out.push(f32::from_bits(0x7f80_0000 | (i * 0x0731 + 1))); // +sNaN
        out.push(f32::from_bits(0xff80_0000 | (i * 0x0913 + 1))); // -sNaN
    }
    out
}

fn mixed(rng: &mut Rng, pool: &[f32], nan_prob: u32) -> f32 {
    if rng.below(100) < nan_prob {
        pool[rng.below(pool.len() as u32) as usize]
    } else {
        match rng.below(8) {
            0 => 0.0,
            1 => -0.0,
            2 => 1.0,
            3 => -1.0,
            4 => f32::INFINITY,
            5 => f32::NEG_INFINITY,
            _ => rng.uniform(10.0),
        }
    }
}

#[test]
fn nan_payload_scalars() {
    let p = load();
    let mut d = Diff::new("nan_payload_scalars");
    let mut rng = Rng::new(0x1A5_0FEE_D5EC_0DE0);
    let pool = nan_pool();
    for _ in 0..300_000 {
        let a = c2v {
            x: mixed(&mut rng, &pool, 70),
            y: mixed(&mut rng, &pool, 70),
        };
        let b = c2v {
            x: mixed(&mut rng, &pool, 70),
            y: mixed(&mut rng, &pool, 70),
        };
        let s = mixed(&mut rng, &pool, 70);
        d.eq_f32(
            || format!("c2Dot({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Dot)(a, b) },
            unsafe { (p.r.c2Dot)(a, b) },
        );
        d.eq_f32(
            || format!("c2Len({})", vs(a)),
            unsafe { (p.c.c2Len)(a) },
            unsafe { (p.r.c2Len)(a) },
        );
        d.eq_v(
            || format!("c2Add({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Add)(a, b) },
            unsafe { (p.r.c2Add)(a, b) },
        );
        d.eq_v(
            || format!("c2Sub({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Sub)(a, b) },
            unsafe { (p.r.c2Sub)(a, b) },
        );
        d.eq_v(
            || format!("c2Mulvs({}, {})", vs(a), fs(s)),
            unsafe { (p.c.c2Mulvs)(a, s) },
            unsafe { (p.r.c2Mulvs)(a, s) },
        );
        d.eq_v(
            || format!("c2Div({}, {})", vs(a), fs(s)),
            unsafe { (p.c.c2Div)(a, s) },
            unsafe { (p.r.c2Div)(a, s) },
        );
        d.eq_v(
            || format!("c2Norm({})", vs(a)),
            unsafe { (p.c.c2Norm)(a) },
            unsafe { (p.r.c2Norm)(a) },
        );
        let m = c2m { x: a, y: b };
        let bv = c2v {
            x: mixed(&mut rng, &pool, 70),
            y: mixed(&mut rng, &pool, 70),
        };
        d.eq_v(
            || format!("c2MulmvT({:?}, {})", m, vs(bv)),
            unsafe { (p.c.c2MulmvT)(m, bv) },
            unsafe { (p.r.c2MulmvT)(m, bv) },
        );
    }
    d.finish();
}

#[test]
fn nan_payload_raycasters() {
    let p = load();
    let mut d = Diff::new("nan_payload_raycasters");
    let mut rng = Rng::new(0x2A5_0FEE_D5EC_0DE1);
    let pool = nan_pool();
    // A range of NaN densities: at 100% every comparison is unordered and most
    // branches are unreachable, so the interesting mixtures are the low ones.
    for &density in &[10u32, 20, 35, 50, 70, 90] {
        for _ in 0..60_000 {
            let a = c2Ray {
                p: c2v {
                    x: mixed(&mut rng, &pool, density),
                    y: mixed(&mut rng, &pool, density),
                },
                d: c2v {
                    x: mixed(&mut rng, &pool, density),
                    y: mixed(&mut rng, &pool, density),
                },
                t: mixed(&mut rng, &pool, density),
            };

            let ci = c2Circle {
                p: c2v {
                    x: mixed(&mut rng, &pool, density),
                    y: mixed(&mut rng, &pool, density),
                },
                r: mixed(&mut rng, &pool, density),
            };
            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = unsafe { (p.c.c2RaytoCircle)(a, ci, &mut co) };
            let rr = unsafe { (p.r.c2RaytoCircle)(a, ci, &mut ro) };
            d.eq_cast(|| format!("circle {:?} {:?}", a, ci), cr, &co, rr, &ro);

            let bx = c2AABB {
                min: c2v {
                    x: mixed(&mut rng, &pool, density),
                    y: mixed(&mut rng, &pool, density),
                },
                max: c2v {
                    x: mixed(&mut rng, &pool, density),
                    y: mixed(&mut rng, &pool, density),
                },
            };
            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = unsafe { (p.c.c2RaytoAABB)(a, bx, &mut co) };
            let rr = unsafe { (p.r.c2RaytoAABB)(a, bx, &mut ro) };
            d.eq_cast(|| format!("aabb {:?} {:?}", a, bx), cr, &co, rr, &ro);

            let cap = c2Capsule {
                a: c2v {
                    x: mixed(&mut rng, &pool, density),
                    y: mixed(&mut rng, &pool, density),
                },
                b: c2v {
                    x: mixed(&mut rng, &pool, density),
                    y: mixed(&mut rng, &pool, density),
                },
                r: mixed(&mut rng, &pool, density),
            };
            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = unsafe { (p.c.c2RaytoCapsule)(a, cap, &mut co) };
            let rr = unsafe { (p.r.c2RaytoCapsule)(a, cap, &mut ro) };
            d.eq_cast(|| format!("capsule {:?} {:?}", a, cap), cr, &co, rr, &ro);
        }
    }
    d.finish();
}

/// The same stress applied to a *nearly valid* geometric setup, so the deep
/// branches (side-wall hit, slab crossing, the four AABB normal branches) stay
/// reachable while one or two slots carry a distinctly-payloaded NaN.
#[test]
fn nan_payload_single_slot_poison() {
    let p = load();
    let mut d = Diff::new("nan_payload_single_slot_poison");
    let mut rng = Rng::new(0x3A5_0FEE_D5EC_0DE2);
    let pool = nan_pool();

    for _ in 0..40_000 {
        // A configuration that definitely hits all three shapes.
        let ray = [-5.0f32, 0.3, 1.0, 0.0, 100.0];
        let circ = [0.0f32, 0.0, 2.0];
        let bxx = [-1.0f32, -1.0, 1.0, 1.0];
        let capp = [0.0f32, -3.0, 0.0, 3.0, 1.0];

        // Poison 1..3 slots out of the 5 + 3/4/5 available, with *different*
        // NaN payloads each time.
        for n_poison in 1..=3u32 {
            let mut r = ray;
            let mut c = circ;
            let mut b = bxx;
            let mut k = capp;
            for _ in 0..n_poison {
                let val = pool[rng.below(pool.len() as u32) as usize];
                match rng.below(4) {
                    0 => r[rng.below(5) as usize] = val,
                    1 => c[rng.below(3) as usize] = val,
                    2 => b[rng.below(4) as usize] = val,
                    _ => k[rng.below(5) as usize] = val,
                }
            }
            let a = c2Ray {
                p: v(r[0], r[1]),
                d: v(r[2], r[3]),
                t: r[4],
            };
            let ci = c2Circle { p: v(c[0], c[1]), r: c[2] };
            let bx = c2AABB { min: v(b[0], b[1]), max: v(b[2], b[3]) };
            let cap = c2Capsule { a: v(k[0], k[1]), b: v(k[2], k[3]), r: k[4] };

            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = unsafe { (p.c.c2RaytoCircle)(a, ci, &mut co) };
            let rr = unsafe { (p.r.c2RaytoCircle)(a, ci, &mut ro) };
            d.eq_cast(|| format!("circle {:?} {:?}", a, ci), cr, &co, rr, &ro);

            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = unsafe { (p.c.c2RaytoAABB)(a, bx, &mut co) };
            let rr = unsafe { (p.r.c2RaytoAABB)(a, bx, &mut ro) };
            d.eq_cast(|| format!("aabb {:?} {:?}", a, bx), cr, &co, rr, &ro);

            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = unsafe { (p.c.c2RaytoCapsule)(a, cap, &mut co) };
            let rr = unsafe { (p.r.c2RaytoCapsule)(a, cap, &mut ro) };
            d.eq_cast(|| format!("capsule {:?} {:?}", a, cap), cr, &co, rr, &ro);

            // …and through the dispatcher and the public entry point.
            for (ty, ptr) in [
                (C2_TYPE_CIRCLE, (&raw const ci) as *const c_void),
                (C2_TYPE_AABB, (&raw const bx) as *const c_void),
                (C2_TYPE_CAPSULE, (&raw const cap) as *const c_void),
            ] {
                let mut co = sentinel();
                let mut ro = sentinel();
                let cr = unsafe { (p.c.c2CastRay)(a, ptr, ty, &mut co) };
                let rr = unsafe { (p.r.c2CastRay)(a, ptr, ty, &mut ro) };
                d.eq_cast(|| format!("castray ty={ty} {:?}", a), cr, &co, rr, &ro);
            }
        }
    }
    d.finish();
}

/// `gen_ray` with distinctly-payloaded NaNs sprinkled into its 16 arguments.
#[test]
fn nan_payload_gen_ray() {
    let p = load();
    let mut d = Diff::new("nan_payload_gen_ray");
    let mut rng = Rng::new(0x4A5_0FEE_D5EC_0DE3);
    let pool = nan_pool();
    let base = [
        10.0f32, 0.0, 0.0, 0.0, 5.0, 0.0, 1.0, 7.0, -2.0, 7.0, 2.0, 0.5, 8.0, -1.0, 9.0, 1.0,
    ];
    let call = |l: &Lib, f: &[f32; 16]| -> (c_int, [c2Raycast; 3]) {
        let mut o = [sentinel(); 3];
        let (a, rest) = o.split_at_mut(1);
        let (b, c) = rest.split_at_mut(1);
        let ret = unsafe {
            (l.gen_ray)(
                &mut a[0], &mut b[0], &mut c[0], f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7],
                f[8], f[9], f[10], f[11], f[12], f[13], f[14], f[15],
            )
        };
        (ret, o)
    };
    for n_poison in 1..=4u32 {
        for _ in 0..40_000 {
            let mut f = base;
            for slot in f.iter_mut() {
                *slot += rng.uniform(3.0);
            }
            for _ in 0..n_poison {
                f[rng.below(16) as usize] = pool[rng.below(pool.len() as u32) as usize];
            }
            let (cr, co) = call(&p.c, &f);
            let (rr, ro) = call(&p.r, &f);
            d.eq_i(|| format!("gen_ray{:?} ret", f), cr, rr);
            for i in 0..3 {
                d.eq_f32(|| format!("gen_ray{:?} cast{}.t", f, i + 1), co[i].t, ro[i].t);
                d.eq_v(|| format!("gen_ray{:?} cast{}.n", f, i + 1), co[i].n, ro[i].n);
            }
        }
    }
    d.finish();
}
