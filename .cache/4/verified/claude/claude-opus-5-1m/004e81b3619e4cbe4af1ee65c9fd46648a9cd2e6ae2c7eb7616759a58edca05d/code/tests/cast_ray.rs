//! Phase B — CONFIGS.md rows 90..97 (`c2CastRay` dispatch).

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_void;
use std::mem::size_of;

const N: usize = 4096;

fn ray(px: f32, py: f32, dx: f32, dy: f32, t: f32) -> C2Ray {
    C2Ray {
        p: v(px, py),
        d: v(dx, dy),
        t,
    }
}

/// Differentially compare `c2CastRay` between the two libraries.
fn cmp_cast(A: C2Ray, B: *const c_void, bx: *const C2x, typeB: u32, what: &str) {
    let (c, r) = (c(), rs());
    for seed in [0x0000_0000u32, 0xffff_ffff, 0x5555_5555] {
        let mut oc = poison(seed);
        let mut orr = poison(seed);
        let rc = unsafe { (c.c2CastRay)(A, B, bx, typeB, &mut oc) };
        let rr = unsafe { (r.c2CastRay)(A, B, bx, typeB, &mut orr) };
        assert_eq!(
            rc, rr,
            "c2CastRay({what}, typeB={typeB}) return: C={rc} RUST={rr}\n  ray p={} d={} t={}",
            vshow(A.p),
            vshow(A.d),
            fshow(A.t)
        );
        assert!(
            rceq(oc, orr),
            "c2CastRay({what}, typeB={typeB}) out: C={} RUST={}\n  ray p={} d={} t={} (poison 0x{seed:08x})",
            rcshow(oc),
            rcshow(orr),
            vshow(A.p),
            vshow(A.d),
            fshow(A.t)
        );
    }
}

/// `c2CastRay` must agree with the direct low-level entry point, in *both*
/// libraries and against each other.
fn cast_matches_direct_circle(A: C2Ray, B: C2Circle) {
    let (c, r) = (c(), rs());
    for api in [c, r] {
        let mut o1 = poison(3);
        let mut o2 = poison(3);
        let r1 = unsafe { (api.c2RaytoCircle)(A, B, &mut o1) };
        let r2 = unsafe {
            (api.c2CastRay)(
                A,
                (&B as *const C2Circle) as *const c_void,
                std::ptr::null(),
                C2_TYPE_CIRCLE,
                &mut o2,
            )
        };
        assert_eq!(r1, r2, "{}: c2CastRay(CIRCLE) != c2RaytoCircle", api.name);
        assert!(rceq(o1, o2), "{}: CIRCLE out differs", api.name);
    }
    cmp_cast(
        A,
        (&B as *const C2Circle) as *const c_void,
        std::ptr::null(),
        C2_TYPE_CIRCLE,
        "circle",
    );
}

// --- row 90 --------------------------------------------------------------
#[test]
fn row90_dispatch_circle() {
    let mut rng = Rng::new(0x9090);
    for _ in 0..N {
        cast_matches_direct_circle(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            C2Circle {
                p: rng.geom_v(),
                r: rng.geom(),
            },
        );
    }
    for _ in 0..N {
        cast_matches_direct_circle(
            C2Ray {
                p: rng.wild_v(),
                d: rng.wild_v(),
                t: rng.wild(),
            },
            C2Circle {
                p: rng.wild_v(),
                r: rng.wild(),
            },
        );
    }
}

// --- row 91 --------------------------------------------------------------
#[test]
fn row91_dispatch_aabb() {
    let (c, r) = (c(), rs());
    let mut rng = Rng::new(0x9191);
    for i in 0..(2 * N) {
        let A = if i % 2 == 0 {
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom())
        } else {
            C2Ray {
                p: rng.wild_v(),
                d: rng.wild_v(),
                t: rng.wild(),
            }
        };
        let B = if i % 2 == 0 {
            C2AABB {
                min: rng.geom_v(),
                max: rng.geom_v(),
            }
        } else {
            C2AABB {
                min: rng.wild_v(),
                max: rng.wild_v(),
            }
        };
        for api in [c, r] {
            let mut o1 = poison(3);
            let mut o2 = poison(3);
            let r1 = unsafe { (api.c2RaytoAABB)(A, B, &mut o1) };
            let r2 = unsafe {
                (api.c2CastRay)(
                    A,
                    (&B as *const C2AABB) as *const c_void,
                    std::ptr::null(),
                    C2_TYPE_AABB,
                    &mut o2,
                )
            };
            assert_eq!(r1, r2, "{}: c2CastRay(AABB) != c2RaytoAABB", api.name);
            assert!(rceq(o1, o2), "{}: AABB out differs", api.name);
        }
        cmp_cast(
            A,
            (&B as *const C2AABB) as *const c_void,
            std::ptr::null(),
            C2_TYPE_AABB,
            "aabb",
        );
    }
}

// --- row 92 --------------------------------------------------------------
#[test]
fn row92_dispatch_capsule() {
    let (c, r) = (c(), rs());
    let mut rng = Rng::new(0x9292);
    for i in 0..(2 * N) {
        let A = if i % 2 == 0 {
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom())
        } else {
            C2Ray {
                p: rng.wild_v(),
                d: rng.wild_v(),
                t: rng.wild(),
            }
        };
        let B = if i % 2 == 0 {
            C2Capsule {
                a: rng.geom_v(),
                b: rng.geom_v(),
                r: rng.geom(),
            }
        } else {
            C2Capsule {
                a: rng.wild_v(),
                b: rng.wild_v(),
                r: rng.wild(),
            }
        };
        for api in [c, r] {
            let mut o1 = poison(3);
            let mut o2 = poison(3);
            let r1 = unsafe { (api.c2RaytoCapsule)(A, B, &mut o1) };
            let r2 = unsafe {
                (api.c2CastRay)(
                    A,
                    (&B as *const C2Capsule) as *const c_void,
                    std::ptr::null(),
                    C2_TYPE_CAPSULE,
                    &mut o2,
                )
            };
            assert_eq!(r1, r2, "{}: c2CastRay(CAPSULE) != c2RaytoCapsule", api.name);
            assert!(rceq(o1, o2), "{}: CAPSULE out differs", api.name);
        }
        cmp_cast(
            A,
            (&B as *const C2Capsule) as *const c_void,
            std::ptr::null(),
            C2_TYPE_CAPSULE,
            "capsule",
        );
    }
}

fn ccw90(a: C2v) -> C2v {
    v(a.y, -a.x)
}

fn ngon(count: i32, radius: f32, phase: f32) -> C2Poly {
    let mut p = C2Poly::default();
    p.count = count;
    let n = count.clamp(1, 8);
    for i in 0..n {
        let a = phase + (i as f32) * std::f32::consts::TAU / (n as f32);
        p.verts[i as usize] = v(radius * a.cos(), radius * a.sin());
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let e = v(
            p.verts[j as usize].x - p.verts[i as usize].x,
            p.verts[j as usize].y - p.verts[i as usize].y,
        );
        let s = ccw90(e);
        let l = (s.x * s.x + s.y * s.y).sqrt();
        p.norms[i as usize] = v(s.x / l, s.y / l);
    }
    p
}

// --- rows 93 & 94 -------------------------------------------------------
#[test]
fn row93_row94_dispatch_poly() {
    let (c, r) = (c(), rs());
    let mut rng = Rng::new(0x9394);
    for i in 0..(2 * N) {
        let n = rng.below(9) as i32;
        let mut p = ngon(n.max(1), 0.5 + rng.unit(5.0).abs(), rng.unit(3.2));
        p.count = n;
        let A = ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom());
        // row 93: bx == NULL
        let bxs: [*const C2x; 1] = [std::ptr::null()];
        for &bxp in bxs.iter() {
            for api in [c, r] {
                let mut o1 = poison(3);
                let mut o2 = poison(3);
                let r1 = unsafe { (api.c2RaytoPoly)(A, &p, bxp, &mut o1) };
                let r2 = unsafe {
                    (api.c2CastRay)(
                        A,
                        (&p as *const C2Poly) as *const c_void,
                        bxp,
                        C2_TYPE_POLY,
                        &mut o2,
                    )
                };
                assert_eq!(r1, r2, "{}: c2CastRay(POLY) != c2RaytoPoly", api.name);
                assert!(rceq(o1, o2), "{}: POLY out differs", api.name);
            }
            cmp_cast(
                A,
                (&p as *const C2Poly) as *const c_void,
                bxp,
                C2_TYPE_POLY,
                "poly/null-bx",
            );
        }
        // row 94: bx != NULL (rotation + translation sweep)
        let ang = (i as f32) * 0.01;
        let bx = C2x {
            p: rng.geom_v(),
            r: C2r {
                c: ang.cos(),
                s: ang.sin(),
            },
        };
        for api in [c, r] {
            let mut o1 = poison(3);
            let mut o2 = poison(3);
            let r1 = unsafe { (api.c2RaytoPoly)(A, &p, &bx, &mut o1) };
            let r2 = unsafe {
                (api.c2CastRay)(
                    A,
                    (&p as *const C2Poly) as *const c_void,
                    &bx,
                    C2_TYPE_POLY,
                    &mut o2,
                )
            };
            assert_eq!(r1, r2, "{}: c2CastRay(POLY,bx) != c2RaytoPoly", api.name);
            assert!(rceq(o1, o2), "{}: POLY(bx) out differs", api.name);
        }
        cmp_cast(
            A,
            (&p as *const C2Poly) as *const c_void,
            &bx,
            C2_TYPE_POLY,
            "poly/bx",
        );
    }
}

// --- row 95: bx must be ignored for the non-POLY types ------------------
#[test]
fn row95_bx_ignored_for_non_poly() {
    let (c, r) = (c(), rs());
    let mut rng = Rng::new(0x9595);
    for _ in 0..N {
        let A = ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom());
        let bx = C2x {
            p: rng.wild_v(),
            r: C2r {
                c: rng.wild(),
                s: rng.wild(),
            },
        };
        let circle = C2Circle {
            p: rng.geom_v(),
            r: rng.geom(),
        };
        let aabb = C2AABB {
            min: rng.geom_v(),
            max: rng.geom_v(),
        };
        let capsule = C2Capsule {
            a: rng.geom_v(),
            b: rng.geom_v(),
            r: rng.geom(),
        };
        let shapes: [(u32, *const c_void, &str); 3] = [
            (
                C2_TYPE_CIRCLE,
                (&circle as *const C2Circle) as *const c_void,
                "circle",
            ),
            (C2_TYPE_AABB, (&aabb as *const C2AABB) as *const c_void, "aabb"),
            (
                C2_TYPE_CAPSULE,
                (&capsule as *const C2Capsule) as *const c_void,
                "capsule",
            ),
        ];
        for (ty, ptr, name) in shapes {
            cmp_cast(A, ptr, &bx, ty, name);
            // and: the non-NULL bx must not change anything vs NULL
            for api in [c, r] {
                let mut o1 = poison(9);
                let mut o2 = poison(9);
                let r1 = unsafe { (api.c2CastRay)(A, ptr, std::ptr::null(), ty, &mut o1) };
                let r2 = unsafe { (api.c2CastRay)(A, ptr, &bx, ty, &mut o2) };
                assert_eq!(r1, r2, "{}: {name}: bx changed the return value", api.name);
                assert!(rceq(o1, o2), "{}: {name}: bx changed *out", api.name);
            }
        }
    }
}

// --- row 96: out-of-range typeB ---------------------------------------
#[test]
fn row96_invalid_type_enum() {
    let mut rng = Rng::new(0x9696);
    let bad: [u32; 16] = [
        4,
        5,
        6,
        7,
        8,
        16,
        100,
        255,
        256,
        1000,
        0x7fff_ffff,             // i32::MAX
        0x8000_0000,             // i32::MIN
        0xffff_ffff,             // -1
        0xffff_fffc,             // -4
        0x0000_0004,
        0xdead_beef,
    ];
    let circle = C2Circle {
        p: v(0.0, 0.0),
        r: 1.0,
    };
    let poly = ngon(4, 2.0, 0.0);
    let bx = C2x {
        p: v(1.0, 2.0),
        r: C2r { c: 1.0, s: 0.0 },
    };
    for &ty in bad.iter() {
        for A in [
            ray(-4.0, 0.0, 1.0, 0.0, 10.0),
            ray(0.0, 0.0, 0.0, 0.0, 0.0),
            ray(f32::NAN, 1.0, 1.0, 1.0, f32::INFINITY),
        ] {
            cmp_cast(
                A,
                (&circle as *const C2Circle) as *const c_void,
                std::ptr::null(),
                ty,
                "invalid/circle-bytes",
            );
            cmp_cast(
                A,
                (&poly as *const C2Poly) as *const c_void,
                &bx,
                ty,
                "invalid/poly-bytes",
            );
            // B == NULL is safe here: an invalid typeB never dereferences B.
            cmp_cast(A, std::ptr::null(), std::ptr::null(), ty, "invalid/null-B");
            cmp_cast(A, std::ptr::null(), &bx, ty, "invalid/null-B+bx");
        }
    }
    for _ in 0..N {
        let ty = rng.next_u32();
        if ty <= 3 {
            continue;
        }
        cmp_cast(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            std::ptr::null(),
            std::ptr::null(),
            ty,
            "invalid/random",
        );
    }
}

// --- row 97: the same bytes reinterpreted under each valid typeB --------
#[test]
fn row97_same_bytes_all_types() {
    let mut rng = Rng::new(0x9797);
    // A 4-byte-aligned buffer big enough for a c2Poly plus slack, so that the
    // POLY reinterpretation never reads uninitialised memory.
    let words = (size_of::<C2Poly>() + 512) / 4;
    for trial in 0..64 {
        let mut buf: Vec<u32> = (0..words)
            .map(|i| rng.next_u32() ^ (i as u32).wrapping_mul(0x85eb_ca6b))
            .collect();
        // Keep `count` (the first 4 bytes) in a sane range so the POLY
        // interpretation stays inside the buffer.
        buf[0] = (trial % 9) as u32;
        // Make the float fields mostly reasonable rather than pure noise for
        // half the trials.
        if trial % 2 == 0 {
            for w in buf.iter_mut().skip(1) {
                *w = rng.geom().to_bits();
            }
            buf[0] = (trial % 9) as u32;
        }
        let ptr = buf.as_ptr() as *const c_void;
        let bx = C2x {
            p: rng.geom_v(),
            r: C2r {
                c: rng.geom(),
                s: rng.geom(),
            },
        };
        for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE, C2_TYPE_POLY] {
            for k in 0..8 {
                let ang = (k as f32) * std::f32::consts::TAU / 8.0;
                let A = ray(
                    ang.cos() * 7.0,
                    ang.sin() * 7.0,
                    -ang.cos(),
                    -ang.sin(),
                    14.0,
                );
                cmp_cast(A, ptr, std::ptr::null(), ty, "reinterp/null-bx");
                cmp_cast(A, ptr, &bx, ty, "reinterp/bx");
            }
        }
    }
}
