//! Phase B — CONFIGS.md rows 42–45: `c2CastRay`, the mode-dispatching entry
//! point. `C2_TYPE` is the library's only runtime mode flag, and it reinterprets
//! the `const void *B` payload, so each mode is a distinct configuration.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

const SEED: u64 = 0x5EED_C2A1;
const N: usize = 15_000;

fn norm(from: c2v, to: c2v) -> c2v {
    let l = libs();
    unsafe { (l.c.c2Norm)((l.c.c2Sub)(to, from)) }
}

/// Calls `c2CastRay` on both libraries with a poisoned out-param.
/// `payload` is a raw byte buffer so the test controls exactly what the C reads.
fn cast_both(
    ray: c2Ray,
    payload: &[u8],
    ty: i32,
) -> (i32, c2Raycast, i32, c2Raycast) {
    let l = libs();
    let mut co = POISON;
    let mut ro = POISON;
    let p = payload.as_ptr() as *const c_void;
    let cr = unsafe { (l.c.c2CastRay)(ray, p, ty, &mut co) };
    let rr = unsafe { (l.r.c2CastRay)(ray, p, ty, &mut ro) };
    (cr, co, rr, ro)
}

fn as_bytes<T: Copy>(v: &T) -> Vec<u8> {
    let n = std::mem::size_of::<T>();
    let mut b = vec![0u8; n];
    unsafe {
        std::ptr::copy_nonoverlapping(v as *const T as *const u8, b.as_mut_ptr(), n);
    }
    b
}

fn fmt_ray(r: c2Ray) -> String {
    format!("ray{{p:{} d:{} t:{}}}", fmt_v(r.p), fmt_v(r.d), fmt_f(r.t))
}

/// Row 42 — dispatcher in `C2_TYPE_CIRCLE` mode.
#[test]
fn cfg_42_castray_circle_mode() {
    let mut rng = Rng::new(SEED ^ 42);
    let mut d = Diff::new("row42 c2CastRay CIRCLE");
    let l = libs();
    let (mut hits, mut misses) = (0usize, 0usize);

    for _ in 0..N {
        let c = c2Circle {
            p: rng.vec_coord(),
            r: rng.radius(),
        };
        let origin = rng.vec_coord();
        let target = c2v {
            x: c.p.x + rng.range(-c.r * 2.0, c.r * 2.0),
            y: c.p.y + rng.range(-c.r * 2.0, c.r * 2.0),
        };
        let ray = c2Ray {
            p: origin,
            d: norm(origin, target),
            t: rng.range(0.0, 300.0),
        };
        let bytes = as_bytes(&c);
        let (cr, co, rr, ro) = cast_both(ray, &bytes, C2_TYPE_CIRCLE);
        if cr != 0 { hits += 1 } else { misses += 1 }
        d.check_ray(cr, co, rr, ro, || {
            format!("c2CastRay(CIRCLE) {} circle{{p:{} r:{}}}", fmt_ray(ray), fmt_v(c.p), fmt_f(c.r))
        });
        // The dispatcher must agree with the direct low-level call in BOTH libs.
        let mut direct_c = POISON;
        let mut direct_r = POISON;
        let dcr = unsafe { (l.c.c2RaytoCircle)(ray, c, &mut direct_c) };
        let drr = unsafe { (l.r.c2RaytoCircle)(ray, c, &mut direct_r) };
        d.check(cr == dcr && rc_eq(co, direct_c), || {
            "C: c2CastRay(CIRCLE) disagrees with C: c2RaytoCircle".into()
        });
        d.check(rr == drr && rc_eq(ro, direct_r), || {
            "Rust: c2CastRay(CIRCLE) disagrees with Rust: c2RaytoCircle".into()
        });
    }
    // Spicy bit patterns.
    for _ in 0..N {
        let c = c2Circle { p: rng.vec_spicy(), r: rng.spicy() };
        let ray = c2Ray { p: rng.vec_spicy(), d: rng.vec_spicy(), t: rng.spicy() };
        let bytes = as_bytes(&c);
        let (cr, co, rr, ro) = cast_both(ray, &bytes, C2_TYPE_CIRCLE);
        d.check_ray(cr, co, rr, ro, || format!("c2CastRay(CIRCLE) spicy {}", fmt_ray(ray)));
    }
    assert!(hits > 500 && misses > 500, "poor coverage {hits}/{misses}");
    d.finish();
}

/// Row 43 — dispatcher in `C2_TYPE_AABB` mode.
#[test]
fn cfg_43_castray_aabb_mode() {
    let mut rng = Rng::new(SEED ^ 43);
    let mut d = Diff::new("row43 c2CastRay AABB");
    let l = libs();
    let (mut hits, mut misses) = (0usize, 0usize);

    for _ in 0..N {
        let cx = rng.coord();
        let cy = rng.coord();
        let b = c2AABB {
            min: c2v { x: cx - rng.range(0.01, 30.0), y: cy - rng.range(0.01, 30.0) },
            max: c2v { x: cx + rng.range(0.01, 30.0), y: cy + rng.range(0.01, 30.0) },
        };
        let origin = rng.vec_coord();
        let target = c2v {
            x: rng.range(b.min.x - 10.0, b.max.x + 10.0),
            y: rng.range(b.min.y - 10.0, b.max.y + 10.0),
        };
        let ray = c2Ray {
            p: origin,
            d: norm(origin, target),
            t: rng.range(0.0, 400.0),
        };
        let bytes = as_bytes(&b);
        let (cr, co, rr, ro) = cast_both(ray, &bytes, C2_TYPE_AABB);
        if cr != 0 { hits += 1 } else { misses += 1 }
        d.check_ray(cr, co, rr, ro, || {
            format!("c2CastRay(AABB) {} box{{min:{} max:{}}}", fmt_ray(ray), fmt_v(b.min), fmt_v(b.max))
        });
        let mut direct_c = POISON;
        let mut direct_r = POISON;
        let dcr = unsafe { (l.c.c2RaytoAABB)(ray, b, &mut direct_c) };
        let drr = unsafe { (l.r.c2RaytoAABB)(ray, b, &mut direct_r) };
        d.check(cr == dcr && rc_eq(co, direct_c), || {
            "C: c2CastRay(AABB) disagrees with C: c2RaytoAABB".into()
        });
        d.check(rr == drr && rc_eq(ro, direct_r), || {
            "Rust: c2CastRay(AABB) disagrees with Rust: c2RaytoAABB".into()
        });
    }
    for _ in 0..N {
        let b = c2AABB { min: rng.vec_spicy(), max: rng.vec_spicy() };
        let ray = c2Ray { p: rng.vec_spicy(), d: rng.vec_spicy(), t: rng.spicy() };
        let bytes = as_bytes(&b);
        let (cr, co, rr, ro) = cast_both(ray, &bytes, C2_TYPE_AABB);
        d.check_ray(cr, co, rr, ro, || format!("c2CastRay(AABB) spicy {}", fmt_ray(ray)));
    }
    assert!(hits > 500 && misses > 500, "poor coverage {hits}/{misses}");
    d.finish();
}

/// Row 44 — dispatcher in `C2_TYPE_CAPSULE` mode (the 20-byte MEMORY-class
/// payload, re-passed by value to `c2RaytoCapsule` on the stack).
#[test]
fn cfg_44_castray_capsule_mode() {
    let mut rng = Rng::new(SEED ^ 44);
    let mut d = Diff::new("row44 c2CastRay CAPSULE");
    let l = libs();
    let (mut hits, mut misses) = (0usize, 0usize);

    for _ in 0..N {
        let a = rng.vec_coord();
        let ang = rng.range(0.0, 6.283_185_5);
        let len = rng.range(0.5, 60.0);
        let cap = c2Capsule {
            a,
            b: c2v { x: a.x + len * ang.cos(), y: a.y + len * ang.sin() },
            r: rng.radius(),
        };
        let mid = c2v { x: (cap.a.x + cap.b.x) * 0.5, y: (cap.a.y + cap.b.y) * 0.5 };
        let oang = rng.range(0.0, 6.283_185_5);
        let dist = rng.range(0.0, 120.0);
        let origin = c2v { x: mid.x + dist * oang.cos(), y: mid.y + dist * oang.sin() };
        let s = rng.range(-0.4, 1.4);
        let target = c2v {
            x: cap.a.x + (cap.b.x - cap.a.x) * s + rng.range(-cap.r * 3.0, cap.r * 3.0),
            y: cap.a.y + (cap.b.y - cap.a.y) * s + rng.range(-cap.r * 3.0, cap.r * 3.0),
        };
        let ray = c2Ray {
            p: origin,
            d: norm(origin, target),
            t: rng.range(0.0, 250.0),
        };
        let bytes = as_bytes(&cap);
        let (cr, co, rr, ro) = cast_both(ray, &bytes, C2_TYPE_CAPSULE);
        if cr != 0 { hits += 1 } else { misses += 1 }
        d.check_ray(cr, co, rr, ro, || {
            format!("c2CastRay(CAPSULE) {} cap{{a:{} b:{} r:{}}}", fmt_ray(ray), fmt_v(cap.a), fmt_v(cap.b), fmt_f(cap.r))
        });
        let mut direct_c = POISON;
        let mut direct_r = POISON;
        let dcr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut direct_c) };
        let drr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut direct_r) };
        d.check(cr == dcr && rc_eq(co, direct_c), || {
            "C: c2CastRay(CAPSULE) disagrees with C: c2RaytoCapsule".into()
        });
        d.check(rr == drr && rc_eq(ro, direct_r), || {
            "Rust: c2CastRay(CAPSULE) disagrees with Rust: c2RaytoCapsule".into()
        });
    }
    for _ in 0..N {
        let cap = c2Capsule { a: rng.vec_spicy(), b: rng.vec_spicy(), r: rng.spicy() };
        let ray = c2Ray { p: rng.vec_spicy(), d: rng.vec_spicy(), t: rng.spicy() };
        let bytes = as_bytes(&cap);
        let (cr, co, rr, ro) = cast_both(ray, &bytes, C2_TYPE_CAPSULE);
        d.check_ray(cr, co, rr, ro, || format!("c2CastRay(CAPSULE) spicy {}", fmt_ray(ray)));
    }
    assert!(hits > 500 && misses > 500, "poor coverage {hits}/{misses}");
    d.finish();
}

/// Row 45 — the SAME 20-byte payload read under all three modes, plus an
/// oversized (64-byte) buffer, so the mode flag alone changes the result and any
/// over-read past the shape would show up as a divergence.
#[test]
fn cfg_45_castray_same_payload_all_modes() {
    let mut rng = Rng::new(SEED ^ 45);
    let mut d = Diff::new("row45 c2CastRay shared payload, all modes");
    let mut differed_between_modes = 0usize;

    for _ in 0..N {
        // A 64-byte buffer whose first 20 bytes are 5 meaningful floats and
        // whose tail is a recognisable pattern. `c2Circle` reads 12 B,
        // `c2AABB` 16 B, `c2Capsule` 20 B -- none may touch the tail.
        let mut buf = vec![0u8; 64];
        let floats: [f32; 5] = [
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
        ];
        for (i, f) in floats.iter().enumerate() {
            buf[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
        }
        for b in buf[20..].iter_mut() {
            *b = 0xA5;
        }
        let origin = rng.vec_coord();
        let target = rng.vec_coord();
        let ray = c2Ray {
            p: origin,
            d: norm(origin, target),
            t: rng.range(0.0, 300.0),
        };
        let mut rets = [0i32; 3];
        for (i, ty) in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE]
            .into_iter()
            .enumerate()
        {
            let (cr, co, rr, ro) = cast_both(ray, &buf, ty);
            rets[i] = cr;
            d.check_ray(cr, co, rr, ro, || {
                format!(
                    "c2CastRay(mode={ty}) shared payload {:?} {}",
                    floats,
                    fmt_ray(ray)
                )
            });
        }
        if rets[0] != rets[1] || rets[1] != rets[2] {
            differed_between_modes += 1;
        }
        // Exactly-sized buffers must give identical answers to the oversized one
        // (proves nothing beyond the shape is read).
        for (ty, n) in [
            (C2_TYPE_CIRCLE, 12usize),
            (C2_TYPE_AABB, 16),
            (C2_TYPE_CAPSULE, 20),
        ] {
            let exact = buf[..n].to_vec();
            let (cr, co, rr, ro) = cast_both(ray, &exact, ty);
            let (cr2, co2, _, _) = cast_both(ray, &buf, ty);
            d.check(cr == cr2 && rc_eq(co, co2), || {
                format!("C read past the {n}-byte shape for mode {ty}")
            });
            d.check_ray(cr, co, rr, ro, || {
                format!("c2CastRay(mode={ty}) exact {n}-byte buffer")
            });
        }
    }
    assert!(
        differed_between_modes > 100,
        "the mode flag never changed the result ({differed_between_modes}); \
         the payload distribution is not discriminating"
    );
    eprintln!("    row45: mode flag changed the result in {differed_between_modes} cases");
    d.finish();
}
