//! The `index == ~0` paths of `c2CapsuletoPolyManifold`.
//!
//! When every polygon normal is NaN, none of the `d > sep` / `dot < min_dot`
//! comparisons in `c2CapsuletoPolyManifold` and `c2Incident` can succeed, so
//! `index` keeps its initial `~0` and the C reads `p->verts[-1]` — the four bytes
//! in front of the struct followed by `count`. The translation reproduces that
//! read with the same address arithmetic (`poly_vert`), so with a caller-owned
//! polygon both sides read identical bytes and must agree.
//!
//! `c2AABBtoCapsuleManifold` hits the same paths with a stack-local `c2Poly`; the
//! reference build places `A.max.y` in the preceding word (`p` at `rbp-0xa0`, the
//! by-value `c2AABB` at `rbp-0xb0`, so `verts[-1].x` is `rbp-0xa4` == `A.max.y`),
//! which is what `c2AABBtoCapsuleManifold` in the translation models.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// A polygon with a caller-controlled word in front of it, so `verts[-1]` is
/// well-defined memory that both libraries observe identically.
#[repr(C)]
#[derive(Copy, Clone)]
struct Framed {
    pad: f32,
    preceding: f32,
    poly: c2Poly,
}

type CapPolyFn =
    unsafe extern "C" fn(c2Capsule, *const c2Poly, *const c2x, *mut c2Manifold) -> ();
type NormsFn = unsafe extern "C" fn(*mut c2v, *mut c2v, c_int) -> ();

fn seed_manifold() -> c2Manifold {
    c2Manifold {
        count: -889_275_714,
        depths: [-13.5, 24.25],
        contact_points: [c2v { x: 1.0, y: 2.0 }, c2v { x: 3.0, y: 4.0 }],
        n: c2v { x: 5.0, y: 6.0 },
    }
}

#[test]
fn degenerate_poly_negative_index() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<CapPolyFn>("c2CapsuletoPolyManifold");
    let (cn, _) = l.pair::<NormsFn>("c2Norms");

    // Capsules that straddle the origin, so `c2GJK` (with its all-zero polygon
    // proxy) returns ~0 and the `d < 1.0e-6f` branch with the `code` switch runs.
    // `a == b` makes `ab` NaN too, which keeps `code` at 0 and drives
    // `c2SidePlanesFromPoly(.., -1, ..)`; `a != b` gives `code == 1` and drives
    // `c2Incident` with `index == -1`.
    let capsules = [
        c2Capsule { a: c2v { x: -1.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.5 },
        c2Capsule { a: c2v { x: 0.0, y: -1.0 }, b: c2v { x: 0.0, y: 1.0 }, r: 1.0 },
        c2Capsule { a: c2v { x: -1.0, y: -1.0 }, b: c2v { x: 1.0, y: 1.0 }, r: 0.25 },
        c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 0.75 },
        c2Capsule { a: c2v { x: 0.5, y: 0.5 }, b: c2v { x: 0.5, y: 0.5 }, r: 2.0 },
    ];
    let precedings = [0.0f32, 1.0, -3.5, 1000.0, f32::NAN, f32::INFINITY, -0.125];
    let transforms = [
        None,
        Some(c2x { p: c2v { x: 0.25, y: -0.5 }, r: c2r { c: 1.0, s: 0.0 } }),
        Some(c2x { p: c2v { x: -2.0, y: 1.0 }, r: c2r { c: 0.6, s: 0.8 } }),
    ];

    let mut checked = 0usize;
    // Results keyed by (capsule, transform, count) so we can prove the outcome
    // really does depend on the `verts[-1]` read.
    let mut sensitive = 0usize;

    for (ci, cap) in capsules.iter().enumerate() {
        for (ti, tx) in transforms.iter().enumerate() {
            for count in 3..=8i32 {
                let mut per_preceding: Vec<String> = Vec::new();
                for &pre in &precedings {
                    // Degenerate polygon: all vertices coincide, so every normal
                    // computed by c2Norms is NaN.
                    let mut fr = Framed {
                        pad: 12.5,
                        preceding: pre,
                        poly: c2Poly::default(),
                    };
                    fr.poly.count = count;
                    for i in 0..8 {
                        fr.poly.verts[i] = c2v { x: 2.0, y: -1.0 };
                    }
                    unsafe {
                        cn(
                            fr.poly.verts.as_mut_ptr(),
                            fr.poly.norms.as_mut_ptr(),
                            count,
                        )
                    };
                    let txp = tx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);

                    let fc = fr;
                    let fr2 = fr;
                    let mut mc = seed_manifold();
                    let mut mr = seed_manifold();
                    scrub_stack();
                    unsafe { cf(*cap, &fc.poly, txp, &mut mc) };
                    unsafe { rf(*cap, &fr2.poly, txp, &mut mr) };
                    assert_same_lazy(&mc, &mr, || {
                        format!(
                            "c2CapsuletoPolyManifold degenerate poly: cap#{ci} tx#{ti} \
                             count={count} preceding={pre:e}"
                        )
                    });
                    // The polygon itself must not have been modified by either side.
                    assert_same_lazy(&fc.poly, &fr2.poly, || {
                        format!("poly mutated: cap#{ci} tx#{ti} count={count}")
                    });
                    per_preceding.push(hex(bytes_of(&mc)));
                    checked += 1;
                }
                if per_preceding.iter().any(|x| *x != per_preceding[0]) {
                    sensitive += 1;
                }
            }
        }
    }

    assert!(checked > 500, "too few cases: {checked}");
    assert!(
        sensitive > 0,
        "no case depended on verts[-1]; the index == ~0 path was never reached, \
         so this test is not verifying anything"
    );
    println!("{checked} cases, {sensitive} configurations sensitive to verts[-1]");
}

/// `c2Incident` / `c2SidePlanesFromPoly` are `static`, so they are exercised
/// only through `c2CapsuletoPolyManifold`. This drives well-formed polygons with
/// NaN and infinite vertices mixed in, which is where the surviving comparisons
/// pick unusual indices.
#[test]
fn poly_with_extreme_vertices() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<CapPolyFn>("c2CapsuletoPolyManifold");
    let mut rng = Rng::new(5150);
    for it in 0..40_000 {
        let count = 3 + rng.below(6) as c_int;
        let mut fr = Framed {
            pad: -7.25,
            preceding: rng.tame(),
            poly: c2Poly::default(),
        };
        fr.poly.count = count;
        for i in 0..8usize {
            fr.poly.verts[i] = if rng.below(5) == 0 {
                rng.vec_wild()
            } else {
                rng.vec_tame()
            };
            fr.poly.norms[i] = if rng.below(5) == 0 {
                rng.vec_wild()
            } else {
                rng.vec_tame()
            };
        }
        let cap = c2Capsule {
            a: rng.vec_tame(),
            b: rng.vec_tame(),
            r: rng.radius(),
        };
        let tx = if rng.below(2) == 0 {
            Some(c2x {
                p: rng.vec_tame(),
                r: c2r {
                    c: rng.tame(),
                    s: rng.tame(),
                },
            })
        } else {
            None
        };
        let txp = tx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
        let fc = fr;
        let fr2 = fr;
        let mut mc = seed_manifold();
        let mut mr = seed_manifold();
        scrub_stack();
        unsafe { cf(cap, &fc.poly, txp, &mut mc) };
        unsafe { rf(cap, &fr2.poly, txp, &mut mr) };
        assert_same_lazy(&mc, &mr, || {
            format!("c2CapsuletoPolyManifold #{it} cap={cap:?} count={count} tx={tx:?}")
        });
    }
}

/// `c2GJK` driven with an explicit polygon operand, so the (uninitialised) proxy
/// path is compared directly rather than only through the manifold functions.
#[test]
fn gjk_with_poly_operand() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    type GjkFn = unsafe extern "C" fn(
        *const c_void,
        C2_TYPE,
        *const c2x,
        *const c_void,
        C2_TYPE,
        *const c2x,
        *mut c2v,
        *mut c2v,
        c_int,
        *mut c_int,
        *mut c2GJKCache,
    ) -> f32;
    let (cf, rf) = l.pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(6161);
    for it in 0..40_000 {
        let mut poly = c2Poly::default();
        poly.count = 3 + rng.below(6) as c_int;
        for i in 0..8usize {
            poly.verts[i] = rng.vec_tame();
            poly.norms[i] = rng.vec_tame();
        }
        let capsule = c2Capsule {
            a: rng.vec_tame(),
            b: rng.vec_tame(),
            r: rng.radius(),
        };
        let use_radius = rng.below(2) as c_int;
        let mut out = Vec::with_capacity(2);
        for (i, f) in [&cf, &rf].into_iter().enumerate() {
            let mut a = c2v { x: 42.0, y: -42.0 };
            let mut b = c2v { x: -43.0, y: 43.0 };
            let mut iters: c_int = -5;
            if i == 0 {
                scrub_stack();
            }
            let d = unsafe {
                f(
                    &capsule as *const _ as *const c_void,
                    C2_TYPE_CAPSULE,
                    std::ptr::null(),
                    &poly as *const _ as *const c_void,
                    C2_TYPE_POLY,
                    std::ptr::null(),
                    &mut a,
                    &mut b,
                    use_radius,
                    &mut iters,
                    std::ptr::null_mut(),
                )
            };
            out.push((d.to_bits(), a, b, iters));
        }
        let (c, r) = (out[0], out[1]);
        assert_same_lazy(&c, &r, || {
            format!("c2GJK poly operand #{it} cap={capsule:?} ur={use_radius}")
        });
    }
}
