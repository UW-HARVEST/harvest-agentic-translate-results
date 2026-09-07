//! High-volume cross-entry-point stress: drives every public entry point with a
//! large randomized corpus and compares bit-for-bit. Complements the per-row
//! tests by mixing configurations that the targeted generators keep separate.
//!
//! The seed is fixed by default; override with `STRESS_SEED` to explore more.

mod common;
use common::*;
use std::ffi::c_void;

fn iters() -> usize {
    std::env::var("STRESS_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300_000)
}

fn seed() -> u64 {
    std::env::var("STRESS_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0x5EED_1234_ABCD_0001)
}

/// One randomized case: pick a value class, build a ray/shape, run every
/// relevant entry point on both libraries.
#[test]
fn stress_all_entry_points() {
    let l = libs();
    let mut rng = Rng::new(seed());
    let n = iters();
    for i in 0..n {
        // Vary the "value class" so plain, wide-exponent, special and raw-bit
        // inputs are all mixed into the same corpus.
        let class = i % 4;
        let mut f = |rng: &mut Rng| -> f32 {
            match class {
                0 => rng.sym(50.0),
                1 => rng.wide(20),
                2 => rng.special(1e6),
                _ => rng.any_bits(),
            }
        };

        let ray = C2Ray {
            p: C2v { x: f(&mut rng), y: f(&mut rng) },
            d: C2v { x: f(&mut rng), y: f(&mut rng) },
            t: f(&mut rng),
        };
        let cirs = C2Circle {
            p: C2v { x: f(&mut rng), y: f(&mut rng) },
            r: f(&mut rng),
        };
        let bx = C2AABB {
            min: C2v { x: f(&mut rng), y: f(&mut rng) },
            max: C2v { x: f(&mut rng), y: f(&mut rng) },
        };
        let cap = C2Capsule {
            a: C2v { x: f(&mut rng), y: f(&mut rng) },
            b: C2v { x: f(&mut rng), y: f(&mut rng) },
            r: f(&mut rng),
        };
        let m = C2m {
            x: C2v { x: f(&mut rng), y: f(&mut rng) },
            y: C2v { x: f(&mut rng), y: f(&mut rng) },
        };
        let s = f(&mut rng);

        // --- vector primitives -------------------------------------------
        diff_eq!(format!("stress c2V {i}"), vb((l.c.c2V)(ray.p.x, ray.p.y)), vb((l.r.c2V)(ray.p.x, ray.p.y)));
        diff_eq!(format!("stress c2Dot {i}"), fb((l.c.c2Dot)(ray.p, ray.d)), fb((l.r.c2Dot)(ray.p, ray.d)));
        diff_eq!(format!("stress c2Len {i}"), fb((l.c.c2Len)(ray.p)), fb((l.r.c2Len)(ray.p)));
        diff_eq!(format!("stress c2Add {i}"), vb((l.c.c2Add)(ray.p, ray.d)), vb((l.r.c2Add)(ray.p, ray.d)));
        diff_eq!(format!("stress c2Sub {i}"), vb((l.c.c2Sub)(ray.p, ray.d)), vb((l.r.c2Sub)(ray.p, ray.d)));
        diff_eq!(format!("stress c2Mulvs {i}"), vb((l.c.c2Mulvs)(ray.p, s)), vb((l.r.c2Mulvs)(ray.p, s)));
        diff_eq!(format!("stress c2Div {i}"), vb((l.c.c2Div)(ray.p, s)), vb((l.r.c2Div)(ray.p, s)));
        diff_eq!(format!("stress c2Norm {i}"), vb((l.c.c2Norm)(ray.p)), vb((l.r.c2Norm)(ray.p)));
        diff_eq!(format!("stress c2Minv {i}"), vb((l.c.c2Minv)(ray.p, ray.d)), vb((l.r.c2Minv)(ray.p, ray.d)));
        diff_eq!(format!("stress c2Maxv {i}"), vb((l.c.c2Maxv)(ray.p, ray.d)), vb((l.r.c2Maxv)(ray.p, ray.d)));
        diff_eq!(format!("stress c2Skew {i}"), vb((l.c.c2Skew)(ray.p)), vb((l.r.c2Skew)(ray.p)));
        diff_eq!(format!("stress c2Absv {i}"), vb((l.c.c2Absv)(ray.p)), vb((l.r.c2Absv)(ray.p)));
        diff_eq!(format!("stress c2CCW90 {i}"), vb((l.c.c2CCW90)(ray.p)), vb((l.r.c2CCW90)(ray.p)));
        diff_eq!(format!("stress c2MulmvT {i}"), vb((l.c.c2MulmvT)(m, ray.p)), vb((l.r.c2MulmvT)(m, ray.p)));

        // --- predicates ---------------------------------------------------
        let b2 = C2AABB {
            min: C2v { x: f(&mut rng), y: f(&mut rng) },
            max: C2v { x: f(&mut rng), y: f(&mut rng) },
        };
        diff_eq!(format!("stress c2AABBtoAABB {i}"), (l.c.c2AABBtoAABB)(bx, b2), (l.r.c2AABBtoAABB)(bx, b2));
        diff_eq!(format!("stress c2AABBtoPoint {i}"), (l.c.c2AABBtoPoint)(bx, ray.p), (l.r.c2AABBtoPoint)(bx, ray.p));
        diff_eq!(format!("stress c2CircleToPoint {i}"), (l.c.c2CircleToPoint)(cirs, ray.p), (l.r.c2CircleToPoint)(cirs, ray.p));

        // --- raycasts (low-level, direct) --------------------------------
        {
            let mut oc = SENTINEL;
            let mut or = SENTINEL;
            let rc = unsafe { (l.c.c2RaytoCircle)(ray, cirs, &mut oc) };
            let rr = unsafe { (l.r.c2RaytoCircle)(ray, cirs, &mut or) };
            diff_eq!(format!("stress c2RaytoCircle {i} {ray:?} {cirs:?}"), (rc, rcb(oc)), (rr, rcb(or)));
        }
        {
            let mut oc = SENTINEL;
            let mut or = SENTINEL;
            let rc = unsafe { (l.c.c2RaytoAABB)(ray, bx, &mut oc) };
            let rr = unsafe { (l.r.c2RaytoAABB)(ray, bx, &mut or) };
            diff_eq!(format!("stress c2RaytoAABB {i} {ray:?} {bx:?}"), (rc, rcb(oc)), (rr, rcb(or)));
        }
        {
            let mut oc = SENTINEL;
            let mut or = SENTINEL;
            let rc = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut oc) };
            let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut or) };
            diff_eq!(format!("stress c2RaytoCapsule {i} {ray:?} {cap:?}"), (rc, rcb(oc)), (rr, rcb(or)));
        }

        // --- dispatcher, all three valid modes over the same bytes -------
        let mut buf = [0u8; 32];
        unsafe {
            std::ptr::copy_nonoverlapping(
                &cap as *const C2Capsule as *const u8,
                buf.as_mut_ptr(),
                std::mem::size_of::<C2Capsule>(),
            )
        };
        for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            let mut oc = SENTINEL;
            let mut or = SENTINEL;
            let p = buf.as_ptr() as *const c_void;
            let rc = unsafe { (l.c.c2CastRay)(ray, p, ty, &mut oc) };
            let rr = unsafe { (l.r.c2CastRay)(ray, p, ty, &mut or) };
            diff_eq!(format!("stress c2CastRay {i} ty={ty} {ray:?}"), (rc, rcb(oc)), (rr, rcb(or)));
        }

        // --- top-level entry point ---------------------------------------
        let mut a = [0.0f32; 16];
        for v in a.iter_mut() {
            *v = f(&mut rng);
        }
        let mut c1 = SENTINEL;
        let mut c2 = SENTINEL;
        let mut c3 = SENTINEL;
        let mut r1 = SENTINEL;
        let mut r2 = SENTINEL;
        let mut r3 = SENTINEL;
        let rc = unsafe {
            (l.c.gen_ray)(
                &mut c1, &mut c2, &mut c3, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                a[8], a[9], a[10], a[11], a[12], a[13], a[14], a[15],
            )
        };
        let rr = unsafe {
            (l.r.gen_ray)(
                &mut r1, &mut r2, &mut r3, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                a[8], a[9], a[10], a[11], a[12], a[13], a[14], a[15],
            )
        };
        diff_eq!(
            format!("stress gen_ray {i} {a:?}"),
            (rc, rcb(c1), rcb(c2), rcb(c3)),
            (rr, rcb(r1), rcb(r2), rcb(r3))
        );
    }
    eprintln!("stress: {n} cases x 24 entry-point calls compared bit-for-bit");
}
