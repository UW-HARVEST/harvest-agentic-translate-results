//! Level 3: the dispatcher `c2CastRay` and the public entry point `gen_ray`.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

type FnCastRay = unsafe extern "C" fn(C2Ray, *const c_void, c_int, *mut C2Raycast) -> c_int;

#[rustfmt::skip]
type FnGenRay = unsafe extern "C" fn(
    *mut C2Raycast, *mut C2Raycast, *mut C2Raycast,
    f32, f32,                 // mp
    f32, f32,                 // ray p
    f32, f32, f32,            // circle p, r
    f32, f32, f32, f32, f32,  // capsule a, b, r
    f32, f32, f32, f32,       // bb min, max
) -> c_int;

/// `c2CastRay` is only ever reached with the three enumerator values defined
/// in the C source; a `typeB` outside `0..=2` falls off the end of the C
/// `switch`, which is undefined behaviour, so it is not compared here.
#[test]
fn c2CastRay_matches() {
    let (c, r): (FnCastRay, FnCastRay) = syms(b"c2CastRay\0");
    let mut rng = Rng::new(301);

    let mut seen = [0usize; 3];
    for i in 0..iters(60_000) {
        let ray = rng.ray();
        let mut oc = SENTINEL;
        let mut or = SENTINEL;

        let (gc, gr, what) = match rng.below(3) {
            0 => {
                let circle = rng.circle();
                seen[0] += 1;
                let p = (&circle as *const C2Circle).cast::<c_void>();
                (
                    unsafe { c(ray, p, C2_TYPE_CIRCLE, &mut oc) },
                    unsafe { r(ray, p, C2_TYPE_CIRCLE, &mut or) },
                    format!("{circle:?}"),
                )
            }
            1 => {
                let bb = rng.aabb();
                seen[1] += 1;
                let p = (&bb as *const C2AABB).cast::<c_void>();
                (
                    unsafe { c(ray, p, C2_TYPE_AABB, &mut oc) },
                    unsafe { r(ray, p, C2_TYPE_AABB, &mut or) },
                    format!("{bb:?}"),
                )
            }
            _ => {
                let cap = rng.capsule();
                seen[2] += 1;
                let p = (&cap as *const C2Capsule).cast::<c_void>();
                (
                    unsafe { c(ray, p, C2_TYPE_CAPSULE, &mut oc) },
                    unsafe { r(ray, p, C2_TYPE_CAPSULE, &mut or) },
                    format!("{cap:?}"),
                )
            }
        };

        assert_eq!(
            gc, gr,
            "c2CastRay return mismatch at iter {i}\n  ray: {ray:?}\n  shape: {what}"
        );
        assert!(
            cast_eq(oc, or),
            "c2CastRay out mismatch at iter {i}\n  ray: {ray:?}\n  shape: {what}\n  \
             C:    {}\n  Rust: {}",
            show_cast(oc),
            show_cast(or)
        );
    }
    assert!(
        seen.iter().all(|&n| n > 0),
        "every shape type should have been dispatched: {seen:?}"
    );
}

struct GenRayInput {
    mp: C2v,
    ray_p: C2v,
    circle: C2Circle,
    cap: C2Capsule,
    bb: C2AABB,
}

impl GenRayInput {
    fn call(&self, f: FnGenRay) -> (c_int, [C2Raycast; 3]) {
        let mut o = [SENTINEL; 3];
        let ret = unsafe {
            f(
                &mut o[0],
                &mut o[1],
                &mut o[2],
                self.mp.x,
                self.mp.y,
                self.ray_p.x,
                self.ray_p.y,
                self.circle.p.x,
                self.circle.p.y,
                self.circle.r,
                self.cap.a.x,
                self.cap.a.y,
                self.cap.b.x,
                self.cap.b.y,
                self.cap.r,
                self.bb.min.x,
                self.bb.min.y,
                self.bb.max.x,
                self.bb.max.y,
            )
        };
        (ret, o)
    }

    fn describe(&self) -> String {
        format!(
            "mp={} ray_p={} circle={:?} cap={:?} bb={:?}",
            show_v(self.mp),
            show_v(self.ray_p),
            self.circle,
            self.cap,
            self.bb
        )
    }
}

fn compare(c: FnGenRay, r: FnGenRay, input: &GenRayInput, tag: &str) -> c_int {
    let (rc, oc) = input.call(c);
    let (rr, or) = input.call(r);
    assert_eq!(
        rc,
        rr,
        "gen_ray return mismatch ({tag})\n  {}",
        input.describe()
    );
    for k in 0..3 {
        assert!(
            cast_eq(oc[k], or[k]),
            "gen_ray cast{} mismatch ({tag})\n  {}\n  C:    {}\n  Rust: {}",
            k + 1,
            input.describe(),
            show_cast(oc[k]),
            show_cast(or[k])
        );
    }
    rc
}

#[test]
fn gen_ray_matches_random() {
    let (c, r): (FnGenRay, FnGenRay) = syms(b"gen_ray\0");
    let mut rng = Rng::new(302);

    // The return value packs the three hit flags into bits 0..2, so all eight
    // combinations should be reachable.
    let mut seen = [0usize; 8];
    for i in 0..iters(80_000) {
        let input = GenRayInput {
            mp: rng.vec(),
            ray_p: rng.vec(),
            circle: rng.circle(),
            cap: rng.capsule(),
            bb: rng.aabb(),
        };
        let ret = compare(c, r, &input, &format!("iter {i}"));
        if (0..8).contains(&ret) {
            seen[ret as usize] += 1;
        }
    }
    assert!(
        seen[0] > 0 && seen.iter().skip(1).sum::<usize>() > 0,
        "expected both hits and misses, saw {seen:?}"
    );
}

/// Shapes clustered around the origin so that the ray from `ray_p` to `mp`
/// frequently intersects all three of them, driving the hit paths.
#[test]
fn gen_ray_matches_clustered() {
    let (c, r): (FnGenRay, FnGenRay) = syms(b"gen_ray\0");
    let mut rng = Rng::new(303);

    let mut seen = [0usize; 8];
    for i in 0..iters(80_000) {
        let small = |rng: &mut Rng| C2v {
            x: (rng.unit() - 0.5) * 6.0,
            y: (rng.unit() - 0.5) * 6.0,
        };
        let mp = small(&mut rng);
        let ray_p = C2v {
            x: mp.x + (rng.unit() - 0.5) * 20.0,
            y: mp.y + (rng.unit() - 0.5) * 20.0,
        };
        let bmin = small(&mut rng);
        let bmax = small(&mut rng);
        let input = GenRayInput {
            mp,
            ray_p,
            circle: C2Circle {
                p: small(&mut rng),
                r: rng.unit() * 3.0,
            },
            cap: C2Capsule {
                a: small(&mut rng),
                b: small(&mut rng),
                r: rng.unit() * 2.0,
            },
            bb: C2AABB {
                min: C2v {
                    x: bmin.x.min(bmax.x),
                    y: bmin.y.min(bmax.y),
                },
                max: C2v {
                    x: bmin.x.max(bmax.x),
                    y: bmin.y.max(bmax.y),
                },
            },
        };
        let ret = compare(c, r, &input, &format!("iter {i}"));
        if (0..8).contains(&ret) {
            seen[ret as usize] += 1;
        }
    }
    let distinct = seen.iter().filter(|&&n| n > 0).count();
    assert!(
        distinct >= 6,
        "expected most hit-flag combinations to be reachable, saw {seen:?}"
    );
}

/// Grid sweep on quarter-integer coordinates: exact ties in the `<=` / `>=`
/// comparisons inside `c2RaytoAABB` and `c2RaytoCapsule` are only reached with
/// representable, aligned inputs.
#[test]
fn gen_ray_matches_grid() {
    let (c, r): (FnGenRay, FnGenRay) = syms(b"gen_ray\0");
    let g = [-2.0f32, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0];
    let mut n = 0usize;
    for &mpx in &g {
        for &mpy in &g {
            for &rpx in &g {
                for &rpy in &g {
                    for &rad in &[0.0f32, 0.5, 1.0] {
                        let input = GenRayInput {
                            mp: C2v { x: mpx, y: mpy },
                            ray_p: C2v { x: rpx, y: rpy },
                            circle: C2Circle {
                                p: C2v { x: 0.0, y: 0.0 },
                                r: rad,
                            },
                            cap: C2Capsule {
                                a: C2v { x: 0.0, y: -1.0 },
                                b: C2v { x: 0.0, y: 1.0 },
                                r: rad,
                            },
                            bb: C2AABB {
                                min: C2v { x: -1.0, y: -1.0 },
                                max: C2v { x: 1.0, y: 1.0 },
                            },
                        };
                        compare(c, r, &input, "grid");
                        n += 1;
                    }
                }
            }
        }
    }
    assert!(n > 1000, "grid sweep ran only {n} cases");
}

/// Non-finite and degenerate inputs: `mp == ray_p` makes `c2Norm` divide by
/// zero, and infinities propagate NaN through the whole pipeline.
#[test]
fn gen_ray_matches_degenerate() {
    let (c, r): (FnGenRay, FnGenRay) = syms(b"gen_ray\0");
    let odd = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        f32::MIN_POSITIVE,
        1.0,
    ];
    for &a in &odd {
        for &b in &odd {
            for &d in &odd {
                let input = GenRayInput {
                    mp: C2v { x: a, y: b },
                    // identical to mp in the first sweep position: zero-length
                    // ray direction
                    ray_p: C2v { x: a, y: b },
                    circle: C2Circle {
                        p: C2v { x: d, y: a },
                        r: b,
                    },
                    cap: C2Capsule {
                        a: C2v { x: d, y: b },
                        b: C2v { x: a, y: d },
                        r: d,
                    },
                    bb: C2AABB {
                        min: C2v { x: b, y: d },
                        max: C2v { x: a, y: a },
                    },
                };
                compare(c, r, &input, "degenerate/zero-length");

                let input = GenRayInput {
                    mp: C2v { x: a, y: b },
                    ray_p: C2v { x: d, y: 1.0 },
                    circle: C2Circle {
                        p: C2v { x: b, y: d },
                        r: a,
                    },
                    cap: C2Capsule {
                        a: C2v { x: a, y: a },
                        b: C2v { x: b, y: b },
                        r: d,
                    },
                    bb: C2AABB {
                        min: C2v { x: d, y: a },
                        max: C2v { x: b, y: b },
                    },
                };
                compare(c, r, &input, "degenerate/nonfinite");
            }
        }
    }
}
