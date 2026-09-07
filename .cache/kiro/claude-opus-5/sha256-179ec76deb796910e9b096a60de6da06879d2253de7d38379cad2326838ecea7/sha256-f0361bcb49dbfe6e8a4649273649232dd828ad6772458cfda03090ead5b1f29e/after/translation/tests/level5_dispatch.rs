//! Phase B — CONFIGS.md rows 85-96: `c2Collided`, `ptr_from_parts` and the
//! public `omni_collide` entry point.
//!
//! `c2Collided`'s mixed-type cases **swap the arguments** (e.g. for
//! `typeA == AABB, typeB == CIRCLE` it calls `c2CircletoAABB(*(c2Circle*)B,
//! *(c2AABB*)A)`), so these rows verify the dispatch mapping itself, not just
//! the underlying predicates.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

const N: usize = 20_000;

/// Row 85 — all 9 valid `(typeA, typeB)` combinations of `c2Collided`.
#[test]
fn row85_collided_all_valid_pairs() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 85);
    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            for i in 0..N / 4 {
                let a = rand_shape(&mut g, ta);
                let b = rand_shape(&mut g, tb);
                let cv = unsafe { (c.c2Collided)(a.as_ptr(), ta, b.as_ptr(), tb) };
                let rv = unsafe { (r.c2Collided)(a.as_ptr(), ta, b.as_ptr(), tb) };
                eq_int(
                    &format!(
                        "row85 {}/{} #{i}",
                        type_name(ta),
                        type_name(tb)
                    ),
                    cv,
                    rv,
                );
            }
            // Wild-float variant.
            for i in 0..N / 8 {
                let a = match ta {
                    C2_TYPE_CIRCLE => ShapeBuf::from_circle(c2Circle {
                        p: g.wild_v(),
                        r: g.wild(),
                    }),
                    C2_TYPE_AABB => ShapeBuf::from_aabb(c2AABB {
                        min: g.wild_v(),
                        max: g.wild_v(),
                    }),
                    _ => ShapeBuf::from_capsule(c2Capsule {
                        a: g.wild_v(),
                        b: g.wild_v(),
                        r: g.wild(),
                    }),
                };
                let b = match tb {
                    C2_TYPE_CIRCLE => ShapeBuf::from_circle(c2Circle {
                        p: g.wild_v(),
                        r: g.wild(),
                    }),
                    C2_TYPE_AABB => ShapeBuf::from_aabb(c2AABB {
                        min: g.wild_v(),
                        max: g.wild_v(),
                    }),
                    _ => ShapeBuf::from_capsule(c2Capsule {
                        a: g.wild_v(),
                        b: g.wild_v(),
                        r: g.wild(),
                    }),
                };
                let cv = unsafe { (c.c2Collided)(a.as_ptr(), ta, b.as_ptr(), tb) };
                let rv = unsafe { (r.c2Collided)(a.as_ptr(), ta, b.as_ptr(), tb) };
                eq_int(
                    &format!("row85 wild {}/{} #{i}", type_name(ta), type_name(tb)),
                    cv,
                    rv,
                );
            }
        }
    }
}

/// Row 85 (dispatch mapping) — the mixed cases must route to the same
/// underlying predicate with the same argument order as the C. Verified by
/// calling the predicate directly and requiring the dispatcher to agree with it
/// on both sides.
#[test]
fn row85_dispatch_mapping_matches_direct_predicates() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 0x85_11);
    for i in 0..N {
        let circ = g.circle();
        let bb = g.aabb();
        let cap = g.capsule();
        let cb = ShapeBuf::from_circle(circ);
        let bbb = ShapeBuf::from_aabb(bb);
        let capb = ShapeBuf::from_capsule(cap);

        for api in [c, r] {
            let d = |a: *const c_void, ta, b: *const c_void, tb| unsafe {
                (api.c2Collided)(a, ta, b, tb)
            };
            // CIRCLE x AABB  -> c2CircletoAABB(A, B)
            assert_eq!(
                d(cb.as_ptr(), C2_TYPE_CIRCLE, bbb.as_ptr(), C2_TYPE_AABB),
                (api.c2CircletoAABB)(circ, bb),
                "{} row85 CIRCLE/AABB dispatch #{i}",
                api.tag
            );
            // AABB x CIRCLE  -> c2CircletoAABB(B, A)  (arguments swapped)
            assert_eq!(
                d(bbb.as_ptr(), C2_TYPE_AABB, cb.as_ptr(), C2_TYPE_CIRCLE),
                (api.c2CircletoAABB)(circ, bb),
                "{} row85 AABB/CIRCLE swap #{i}",
                api.tag
            );
            // CIRCLE x CAPSULE -> c2CircletoCapsule(A, B)
            assert_eq!(
                d(cb.as_ptr(), C2_TYPE_CIRCLE, capb.as_ptr(), C2_TYPE_CAPSULE),
                (api.c2CircletoCapsule)(circ, cap),
                "{} row85 CIRCLE/CAPSULE dispatch #{i}",
                api.tag
            );
            // CAPSULE x CIRCLE -> c2CircletoCapsule(B, A)  (swapped)
            assert_eq!(
                d(capb.as_ptr(), C2_TYPE_CAPSULE, cb.as_ptr(), C2_TYPE_CIRCLE),
                (api.c2CircletoCapsule)(circ, cap),
                "{} row85 CAPSULE/CIRCLE swap #{i}",
                api.tag
            );
            // AABB x CAPSULE -> c2AABBtoCapsule(A, B)
            assert_eq!(
                d(bbb.as_ptr(), C2_TYPE_AABB, capb.as_ptr(), C2_TYPE_CAPSULE),
                (api.c2AABBtoCapsule)(bb, cap),
                "{} row85 AABB/CAPSULE dispatch #{i}",
                api.tag
            );
            // CAPSULE x AABB -> c2AABBtoCapsule(B, A)  (swapped)
            assert_eq!(
                d(capb.as_ptr(), C2_TYPE_CAPSULE, bbb.as_ptr(), C2_TYPE_AABB),
                (api.c2AABBtoCapsule)(bb, cap),
                "{} row85 CAPSULE/AABB swap #{i}",
                api.tag
            );
        }
    }
}

/// Rows 86-87 (ERRORS.md rows 37-40) — out-of-range enum values. A C enum
/// accepts any `int`, so these are real inputs. Each of the four `default:`
/// arms of `c2Collided` must return 0 on both sides.
#[test]
fn row86_87_collided_invalid_types() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 86);
    for &bad in &BAD_TYPES {
        // Row 86: valid typeA, invalid typeB -> the inner `default:` arms.
        for &good in &VALID_TYPES {
            for i in 0..64 {
                let a = rand_shape(&mut g, good);
                let b = rand_shape(&mut g, good);
                let cv = unsafe { (c.c2Collided)(a.as_ptr(), good, b.as_ptr(), bad) };
                let rv = unsafe { (r.c2Collided)(a.as_ptr(), good, b.as_ptr(), bad) };
                eq_int(
                    &format!("row86 {}/{bad} #{i}", type_name(good)),
                    cv,
                    rv,
                );
                eq_int(&format!("row86 {}/{bad} #{i} is 0", type_name(good)), cv, 0);
            }
            // Row 87: invalid typeA -> the outer `default:` arm. The C never
            // dereferences B on this path, so a NULL B is a valid input.
            for i in 0..64 {
                let a = rand_shape(&mut g, good);
                let b = rand_shape(&mut g, good);
                let cv = unsafe { (c.c2Collided)(a.as_ptr(), bad, b.as_ptr(), good) };
                let rv = unsafe { (r.c2Collided)(a.as_ptr(), bad, b.as_ptr(), good) };
                eq_int(&format!("row87 {bad}/{} #{i}", type_name(good)), cv, rv);
                eq_int(&format!("row87 {bad}/{} #{i} is 0", type_name(good)), cv, 0);

                let cn = unsafe { (c.c2Collided)(std::ptr::null(), bad, std::ptr::null(), good) };
                let rn = unsafe { (r.c2Collided)(std::ptr::null(), bad, std::ptr::null(), good) };
                eq_int(&format!("row87 null ptrs {bad}/{}", type_name(good)), cn, rn);
                eq_int(&format!("row87 null ptrs {bad} is 0"), cn, 0);
            }
        }
        // Both invalid.
        for &bad2 in &BAD_TYPES {
            let cv = unsafe { (c.c2Collided)(std::ptr::null(), bad, std::ptr::null(), bad2) };
            let rv = unsafe { (r.c2Collided)(std::ptr::null(), bad, std::ptr::null(), bad2) };
            eq_int(&format!("row86/87 {bad}/{bad2}"), cv, rv);
            eq_int(&format!("row86/87 {bad}/{bad2} is 0"), cv, 0);
        }
    }
}

/// Rows 88-90 — `ptr_from_parts` for each valid type: the heap struct it builds
/// must be field-identical. The pointer *value* obviously differs; the contents
/// must not. Both allocations use `malloc`, so both are freed with `free`.
#[test]
fn rows88_90_ptr_from_parts_valid() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 88);

    unsafe extern "C" {
        fn free(p: *mut c_void);
    }

    for i in 0..N {
        let p: [f32; 5] = [g.wild(), g.wild(), g.wild(), g.wild(), g.wild()];

        // Row 88: CIRCLE -> { p = (a,b), r = c }; d and e are ignored.
        unsafe {
            let cp = (c.ptr_from_parts)(C2_TYPE_CIRCLE, p[0], p[1], p[2], p[3], p[4])
                as *mut c2Circle;
            let rp = (r.ptr_from_parts)(C2_TYPE_CIRCLE, p[0], p[1], p[2], p[3], p[4])
                as *mut c2Circle;
            assert!(!cp.is_null() && !rp.is_null(), "row88 #{i}: null allocation");
            eq_v(&format!("row88 #{i} .p"), (*cp).p, (*rp).p);
            eq_f32(&format!("row88 #{i} .r"), (*cp).r, (*rp).r);
            eq_bytes(&format!("row88 #{i} bytes"), &*cp, &*rp);
            free(cp as *mut c_void);
            free(rp as *mut c_void);
        }

        // Row 89: AABB -> { min = (a,b), max = (c,d) }; e is ignored.
        unsafe {
            let cp =
                (c.ptr_from_parts)(C2_TYPE_AABB, p[0], p[1], p[2], p[3], p[4]) as *mut c2AABB;
            let rp =
                (r.ptr_from_parts)(C2_TYPE_AABB, p[0], p[1], p[2], p[3], p[4]) as *mut c2AABB;
            assert!(!cp.is_null() && !rp.is_null(), "row89 #{i}: null allocation");
            eq_v(&format!("row89 #{i} .min"), (*cp).min, (*rp).min);
            eq_v(&format!("row89 #{i} .max"), (*cp).max, (*rp).max);
            eq_bytes(&format!("row89 #{i} bytes"), &*cp, &*rp);
            free(cp as *mut c_void);
            free(rp as *mut c_void);
        }

        // Row 90: CAPSULE -> { a = (a,b), b = (c,d), r = e }.
        unsafe {
            let cp = (c.ptr_from_parts)(C2_TYPE_CAPSULE, p[0], p[1], p[2], p[3], p[4])
                as *mut c2Capsule;
            let rp = (r.ptr_from_parts)(C2_TYPE_CAPSULE, p[0], p[1], p[2], p[3], p[4])
                as *mut c2Capsule;
            assert!(!cp.is_null() && !rp.is_null(), "row90 #{i}: null allocation");
            eq_v(&format!("row90 #{i} .a"), (*cp).a, (*rp).a);
            eq_v(&format!("row90 #{i} .b"), (*cp).b, (*rp).b);
            eq_f32(&format!("row90 #{i} .r"), (*cp).r, (*rp).r);
            eq_bytes(&format!("row90 #{i} bytes"), &*cp, &*rp);
            free(cp as *mut c_void);
            free(rp as *mut c_void);
        }
    }
}

/// Row 91 (ERRORS.md rows 41-42) — `ptr_from_parts` with an out-of-range type.
///
/// The C `switch` has no `default:` and the function **falls off the end of a
/// non-`void` function**, which is undefined behaviour: the returned pointer is
/// whatever happens to be in the return register. The Rust returns `NULL`.
/// The pointer value therefore cannot be compared. What *is* well defined and
/// is asserted here:
///   * neither build crashes;
///   * the only consumer, `omni_collide`, discards the pointer via
///     `c2Collided`'s `default:` arm, so the observable result agrees (row 94).
#[test]
fn row91_ptr_from_parts_invalid_type_is_ub() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 91);
    let mut c_null = 0usize;
    let mut r_null = 0usize;
    for &bad in &BAD_TYPES {
        for _ in 0..64 {
            let p: [f32; 5] = [g.coord(), g.coord(), g.coord(), g.coord(), g.coord()];
            // Must not crash. The returned pointer is deliberately NOT
            // dereferenced and NOT freed: in the C it is not a valid
            // allocation.
            let cp = unsafe { (c.ptr_from_parts)(bad, p[0], p[1], p[2], p[3], p[4]) };
            let rp = unsafe { (r.ptr_from_parts)(bad, p[0], p[1], p[2], p[3], p[4]) };
            if cp.is_null() {
                c_null += 1;
            }
            if rp.is_null() {
                r_null += 1;
            }
        }
    }
    println!(
        "row91 ptr_from_parts(invalid): C returned NULL {c_null} times, Rust {r_null} times \
         (C is UB — no return statement on that path)"
    );
    // The Rust's choice is NULL on every invalid input, which is the safest
    // reading of the C's missing return.
    assert_eq!(
        r_null,
        BAD_TYPES.len() * 64,
        "row91: Rust must consistently return NULL for an invalid type"
    );
}

/// Rows 92-93 — the public `omni_collide`: all 9 valid type pairs, heavily
/// randomized, plus structured overlap / touch / apart / contained / coincident
/// configurations.
#[test]
fn rows92_93_omni_collide_valid() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 92);
    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            // Row 92: 2000 randomized parameter tuples per pair.
            for i in 0..2000 {
                let p: [f32; 10] = [
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.radius(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.radius(),
                ];
                let cv = unsafe {
                    (c.omni_collide)(
                        ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                    )
                };
                let rv = unsafe {
                    (r.omni_collide)(
                        ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                    )
                };
                eq_int(
                    &format!(
                        "row92 {}/{} #{i} {p:?}",
                        type_name(ta),
                        type_name(tb)
                    ),
                    cv,
                    rv,
                );
            }

            // Row 93: structured shapes. `omni_collide` takes flat floats, so
            // build the same geometry the shape constructors would.
            for &sep in &[
                0.0f32, 1e-7, 0.001, 0.5, 1.0, 1.5, 1.999, 2.0, 2.001, 3.0, 50.0, 1e6,
            ] {
                let (a1, a2, a3, a4, a5) = flat_shape(ta, 0.0, 0.0, 1.0);
                let (b1, b2, b3, b4, b5) = flat_shape(tb, sep, 0.0, 1.0);
                let cv = unsafe {
                    (c.omni_collide)(ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5)
                };
                let rv = unsafe {
                    (r.omni_collide)(ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5)
                };
                eq_int(
                    &format!("row93 {}/{} sep={sep}", type_name(ta), type_name(tb)),
                    cv,
                    rv,
                );
                // containment
                let (c1, c2_, c3, c4, c5) = flat_shape(ta, 0.0, 0.0, 40.0);
                let (d1, d2, d3, d4, d5) = flat_shape(tb, sep.min(10.0), 0.0, 0.2);
                let cv2 = unsafe {
                    (c.omni_collide)(ta, c1, c2_, c3, c4, c5, tb, d1, d2, d3, d4, d5)
                };
                let rv2 = unsafe {
                    (r.omni_collide)(ta, c1, c2_, c3, c4, c5, tb, d1, d2, d3, d4, d5)
                };
                eq_int(
                    &format!("row93 contain {}/{} sep={sep}", type_name(ta), type_name(tb)),
                    cv2,
                    rv2,
                );
            }
            // coincident
            let (a1, a2, a3, a4, a5) = flat_shape(ta, 0.0, 0.0, 1.0);
            let (b1, b2, b3, b4, b5) = flat_shape(tb, 0.0, 0.0, 1.0);
            let cv = unsafe { (c.omni_collide)(ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5) };
            let rv = unsafe { (r.omni_collide)(ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5) };
            eq_int(
                &format!("row93 coincident {}/{}", type_name(ta), type_name(tb)),
                cv,
                rv,
            );
        }
    }
}

/// The five flat floats `omni_collide` needs for a shape of type `t` centred at
/// `(cx, cy)` with extent `s`, matching `ptr_from_parts`' field order.
fn flat_shape(t: C2_TYPE, cx: f32, cy: f32, s: f32) -> (f32, f32, f32, f32, f32) {
    match t {
        // circle: p = (a1,a2), r = a3
        C2_TYPE_CIRCLE => (cx, cy, s, 0.0, 0.0),
        // aabb: min = (a1,a2), max = (a3,a4)
        C2_TYPE_AABB => (cx - s, cy - s, cx + s, cy + s, 0.0),
        // capsule: a = (a1,a2), b = (a3,a4), r = a5
        _ => (cx - s, cy, cx + s, cy, s * 0.5),
    }
}

/// Row 94 (ERRORS.md rows 41-42) — out-of-range `type_a` / `type_b` in the
/// public API. `ptr_from_parts` returns an indeterminate pointer, but
/// `c2Collided` rejects the unknown tag before dereferencing it, so the result
/// must be a deterministic 0 on both sides.
#[test]
fn row94_omni_collide_invalid_types() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 94);
    for &bad in &BAD_TYPES {
        for &good in &VALID_TYPES {
            for i in 0..256 {
                let p: [f32; 10] = [
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.radius(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.radius(),
                ];
                for (ta, tb) in [(bad, good), (good, bad), (bad, bad)] {
                    let cv = unsafe {
                        (c.omni_collide)(
                            ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                        )
                    };
                    let rv = unsafe {
                        (r.omni_collide)(
                            ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                        )
                    };
                    eq_int(&format!("row94 {ta}/{tb} #{i}"), cv, rv);
                    eq_int(&format!("row94 {ta}/{tb} #{i} is 0"), cv, 0);
                }
            }
        }
    }
}

/// Row 95 — degenerate float inputs to the public API: all zeros, all NaN, all
/// ±inf, subnormals, and mixed `±0.0`.
#[test]
fn row95_omni_collide_degenerate_floats() {
    let (c, r) = apis();
    let sets: [[f32; 10]; 8] = [
        [0.0; 10],
        [-0.0; 10],
        [f32::NAN; 10],
        [f32::INFINITY; 10],
        [f32::NEG_INFINITY; 10],
        [f32::from_bits(1); 10],
        [f32::MAX; 10],
        [
            0.0,
            -0.0,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            f32::MAX,
            f32::MIN,
            f32::EPSILON,
        ],
    ];
    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            for (k, p) in sets.iter().enumerate() {
                let cv = unsafe {
                    (c.omni_collide)(
                        ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                    )
                };
                let rv = unsafe {
                    (r.omni_collide)(
                        ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                    )
                };
                eq_int(
                    &format!("row95 set{k} {}/{}", type_name(ta), type_name(tb)),
                    cv,
                    rv,
                );
            }
        }
    }
}

/// Row 96 — the trailing parameters a shape does not use must not change the
/// result (circle ignores `a4`/`a5`, AABB ignores `a5`). Verified on both
/// builds so that a translation that accidentally read the wrong field would be
/// caught.
#[test]
fn row96_unused_trailing_params_are_ignored() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 96);
    for &ta in &VALID_TYPES {
        for &tb in &VALID_TYPES {
            for i in 0..2000 {
                let base: [f32; 10] = [
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.radius(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.coord(),
                    g.radius(),
                ];
                let mut alt = base;
                // Perturb only the parameters that shape type ignores.
                let junk = [1e9f32, -7.5, f32::NAN, 0.0][i % 4];
                match ta {
                    C2_TYPE_CIRCLE => {
                        alt[3] = junk;
                        alt[4] = junk;
                    }
                    C2_TYPE_AABB => alt[4] = junk,
                    _ => {}
                }
                match tb {
                    C2_TYPE_CIRCLE => {
                        alt[8] = junk;
                        alt[9] = junk;
                    }
                    C2_TYPE_AABB => alt[9] = junk,
                    _ => {}
                }
                let run = |api: &Api, p: &[f32; 10]| unsafe {
                    (api.omni_collide)(
                        ta, p[0], p[1], p[2], p[3], p[4], tb, p[5], p[6], p[7], p[8], p[9],
                    )
                };
                let cb = run(c, &base);
                let rb = run(r, &base);
                let cal = run(c, &alt);
                let ral = run(r, &alt);
                eq_int(&format!("row96 base {}/{} #{i}", type_name(ta), type_name(tb)), cb, rb);
                eq_int(&format!("row96 alt {}/{} #{i}", type_name(ta), type_name(tb)), cal, ral);
                // And the ignored parameters really are ignored, identically.
                assert_eq!(
                    cb, cal,
                    "row96 C changed answer from an ignored parameter ({}/{} #{i})",
                    type_name(ta),
                    type_name(tb)
                );
                assert_eq!(
                    rb, ral,
                    "row96 Rust changed answer from an ignored parameter ({}/{} #{i})",
                    type_name(ta),
                    type_name(tb)
                );
            }
        }
    }
}
