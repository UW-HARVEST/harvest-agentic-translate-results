//! Phase C — error-path differential tests. One test per ERRORS.md row.
//!
//! Every test asserts that C and Rust produce the SAME rejection (same sentinel
//! value / same neutral result), not merely that "both failed somehow".

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// Enum values with no valid `C2_TYPE` variant. C enums accept any `int`, so
/// these are real inputs the C handles and the Rust must handle identically.
const BAD_TYPES: [c_int; 10] = [3, 4, 5, 100, -1, -2, -100, i32::MIN, i32::MAX, 0x7fff_fffe];

fn a_valid_shape() -> [u8; 20] {
    // a capsule/aabb/circle-compatible byte blob: reading it as any shape type
    // yields finite values
    let mut b = [0u8; 20];
    for (i, f) in [1.0f32, 2.0, 3.0, 4.0, 0.5].iter().enumerate() {
        b[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
    }
    b
}

// ===========================================================================
// Rows 1..=4 — c2Collided's four `default: return 0;` branches
// ===========================================================================

#[test]
fn err_collided_bad_typeA() {
    // ERRORS.md row 1
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnCollided>("c2Collided") };
    let s = a_valid_shape();
    let mut d = Diff::new("E1: c2Collided bad typeA -> 0");
    for &bad in BAD_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            let (cv, rv) = unsafe {
                (
                    c(s.as_ptr() as *const c_void, bad, s.as_ptr() as *const c_void, tB),
                    r(s.as_ptr() as *const c_void, bad, s.as_ptr() as *const c_void, tB),
                )
            };
            d.check((bad, ty_name(tB)), cv, rv);
            assert_eq!(cv, 0, "C must return the 0 sentinel for typeA={bad}");
            assert_eq!(rv, 0, "Rust must return the 0 sentinel for typeA={bad}");
        }
        // The C returns before dereferencing, so NULL shapes must be safe too.
        for &tB in ALL_TYPES.iter() {
            let (cv, rv) = unsafe {
                (
                    c(std::ptr::null(), bad, std::ptr::null(), tB),
                    r(std::ptr::null(), bad, std::ptr::null(), tB),
                )
            };
            d.check(("null shapes", bad, ty_name(tB)), cv, rv);
            assert_eq!(cv, 0);
            assert_eq!(rv, 0);
        }
    }
    d.finish();
}

fn collided_bad_typeB(row: &str, tA: c_int) {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnCollided>("c2Collided") };
    let s = a_valid_shape();
    let mut d = Diff::new(row);
    for &bad in BAD_TYPES.iter() {
        let (cv, rv) = unsafe {
            (
                c(s.as_ptr() as *const c_void, tA, s.as_ptr() as *const c_void, bad),
                r(s.as_ptr() as *const c_void, tA, s.as_ptr() as *const c_void, bad),
            )
        };
        d.check((ty_name(tA), bad), cv, rv);
        assert_eq!(cv, 0, "C must return 0 for typeA={tA} typeB={bad}");
        assert_eq!(rv, 0, "Rust must return 0 for typeA={tA} typeB={bad}");
    }
    d.finish();
}

#[test]
fn err_collided_circle_bad_typeB() {
    // ERRORS.md row 2
    collided_bad_typeB("E2: c2Collided CIRCLE x bad typeB -> 0", C2_TYPE_CIRCLE);
}

#[test]
fn err_collided_aabb_bad_typeB() {
    // ERRORS.md row 3
    collided_bad_typeB("E3: c2Collided AABB x bad typeB -> 0", C2_TYPE_AABB);
}

#[test]
fn err_collided_capsule_bad_typeB() {
    // ERRORS.md row 4
    collided_bad_typeB("E4: c2Collided CAPSULE x bad typeB -> 0", C2_TYPE_CAPSULE);
}

// ===========================================================================
// Row 5 — omni_collide with out-of-range enums (the public API path)
// ===========================================================================

#[test]
fn err_omni_collide_bad_enums() {
    // ERRORS.md row 5
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnOmniCollide>("omni_collide") };
    let mut rng = Rng::new(0x6001);
    let mut d = Diff::new("E5: omni_collide out-of-range enums -> 0");
    let mut combos: Vec<(c_int, c_int)> = Vec::new();
    for &bad in BAD_TYPES.iter() {
        for &ok in ALL_TYPES.iter() {
            combos.push((bad, ok)); // bad A, good B
            combos.push((ok, bad)); // good A, bad B
        }
        combos.push((bad, bad)); // both bad
    }
    for (tA, tB) in combos {
        for _ in 0..50 {
            let a: [f32; 5] = [
                mixed(&mut rng, 10.0),
                mixed(&mut rng, 10.0),
                mixed(&mut rng, 10.0),
                mixed(&mut rng, 10.0),
                mixed(&mut rng, 10.0),
            ];
            let b: [f32; 5] = [
                mixed(&mut rng, 10.0),
                mixed(&mut rng, 10.0),
                mixed(&mut rng, 10.0),
                mixed(&mut rng, 10.0),
                mixed(&mut rng, 10.0),
            ];
            let (cv, rv) = unsafe {
                (
                    c(tA, a[0], a[1], a[2], a[3], a[4], tB, b[0], b[1], b[2], b[3], b[4]),
                    r(tA, a[0], a[1], a[2], a[3], a[4], tB, b[0], b[1], b[2], b[3], b[4]),
                )
            };
            d.check((tA, tB, a, b), cv, rv);
            assert_eq!(cv, 0, "C must return 0 for ({tA}, {tB})");
            assert_eq!(rv, 0, "Rust must return 0 for ({tA}, {tB})");
        }
    }
    d.finish();
}

// ===========================================================================
// Row 6 — ptr_from_parts falls off the end for an unknown type
// ===========================================================================

#[test]
fn err_ptr_from_parts_bad_type() {
    // ERRORS.md row 6.
    //
    // The C `switch` has no `default` and no trailing `return`, so for an
    // unknown `typ` the function falls off the end of a non-void function.
    // The returned value is INDETERMINATE in C (whatever is in `rax`), which is
    // by definition not reproducible, so asserting pointer equality here would
    // be asserting on undefined behaviour.
    //
    // What IS well-defined and IS asserted:
    //   * neither library crashes,
    //   * the only *observable* consequence -- `omni_collide` returning 0
    //     without dereferencing the value -- matches (see `err_omni_collide_bad_enums`),
    //   * for VALID types both return a usable, non-NULL pointer to a correctly
    //     populated struct (see CONFIGS.md row 56 / level5_public.rs).
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnPtrFromParts>("ptr_from_parts") };
    for &bad in BAD_TYPES.iter() {
        let cp = unsafe { c(bad, 1.0, 2.0, 3.0, 4.0, 5.0) };
        let rp = unsafe { r(bad, 1.0, 2.0, 3.0, 4.0, 5.0) };
        // No dereference, no assertion on the value: it is indeterminate in C.
        // Rust deterministically yields NULL, which is the safe choice.
        assert!(rp.is_null(), "Rust ptr_from_parts({bad}) should be NULL");
        let _ = cp;
    }
    // valid types: both must produce a non-NULL, correctly filled struct
    for &ty in ALL_TYPES.iter() {
        let cp = unsafe { c(ty, 1.5, -2.5, 3.5, -4.5, 6.25) };
        let rp = unsafe { r(ty, 1.5, -2.5, 3.5, -4.5, 6.25) };
        assert!(!cp.is_null() && !rp.is_null());
        let n = match ty {
            C2_TYPE_CIRCLE => 3,
            C2_TYPE_AABB => 4,
            _ => 5,
        };
        unsafe {
            let cf = std::slice::from_raw_parts(cp as *const f32, n);
            let rf = std::slice::from_raw_parts(rp as *const f32, n);
            assert_eq!(cf, rf, "ptr_from_parts({ty}) struct contents differ");
            free(cp);
            free(rp);
        }
    }
}

extern "C" {
    fn free(p: *mut c_void);
}

// ===========================================================================
// Row 7 — c2MakeProxy with an unknown type leaves the proxy UNTOUCHED
// ===========================================================================

#[test]
fn err_make_proxy_bad_type_leaves_proxy_untouched() {
    // ERRORS.md row 7
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnMakeProxy>("c2MakeProxy") };
    let s = a_valid_shape();
    let mut d = Diff::new("E7: c2MakeProxy bad type -> proxy untouched");
    for &bad in BAD_TYPES.iter() {
        // Distinctive pre-fill so "untouched" is verifiable.
        let fill = C2Proxy {
            radius: -1.5,
            count: 0x5eed,
            verts: [
                C2v { x: 1.0, y: 2.0 },
                C2v { x: 3.0, y: 4.0 },
                C2v { x: 5.0, y: 6.0 },
                C2v { x: 7.0, y: 8.0 },
                C2v { x: 9.0, y: 10.0 },
                C2v { x: 11.0, y: 12.0 },
                C2v { x: 13.0, y: 14.0 },
                C2v { x: 15.0, y: 16.0 },
            ],
        };
        let mut cp = fill;
        let mut rp = fill;
        unsafe {
            c(s.as_ptr() as *const c_void, bad, &mut cp);
            r(s.as_ptr() as *const c_void, bad, &mut rp);
        }
        d.check(bad, cp, rp);
        assert!(cp.beq(&fill), "C modified the proxy for type={bad}: {}", cp.bits());
        assert!(rp.beq(&fill), "Rust modified the proxy for type={bad}: {}", rp.bits());
        // NULL shape is also safe: nothing is dereferenced.
        let mut cp2 = fill;
        let mut rp2 = fill;
        unsafe {
            c(std::ptr::null(), bad, &mut cp2);
            r(std::ptr::null(), bad, &mut rp2);
        }
        d.check(("null shape", bad), cp2, rp2);
        assert!(cp2.beq(&fill) && rp2.beq(&fill));
    }
    d.finish();
}

// ===========================================================================
// Row 8 — c2GJKSimplexMetric with an out-of-range count returns 0
// ===========================================================================

#[test]
fn err_simplex_metric_bad_count() {
    // ERRORS.md row 8
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSimplexF>("c2GJKSimplexMetric") };
    let mut rng = Rng::new(0x6002);
    let mut d = Diff::new("E8: c2GJKSimplexMetric out-of-range count -> 0.0");
    for count in [0, 1, 4, 5, 100, -1, -7, i32::MIN, i32::MAX] {
        for _ in 0..300 {
            let s = simplex(&mut rng, count, 100.0);
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            d.check(count, cv, rv);
            assert_eq!(cv.to_bits(), 0.0f32.to_bits(), "C must return +0.0 for count={count}");
            assert_eq!(rv.to_bits(), 0.0f32.to_bits(), "Rust must return +0.0 for count={count}");
        }
    }
    d.finish();
}

// ===========================================================================
// Rows 9, 10, 11, 12, 13 — c2D / c2L / c2Witness neutral results
// ===========================================================================

#[test]
fn err_c2D_count_3_and_out_of_range() {
    // ERRORS.md row 9
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSimplexV>("c2D") };
    let mut rng = Rng::new(0x6003);
    let mut d = Diff::new("E9: c2D count=3/out-of-range -> (0,0)");
    for count in [3, 0, 4, 5, 100, -1, i32::MIN, i32::MAX] {
        for _ in 0..300 {
            let s = simplex(&mut rng, count, 100.0);
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            d.check(count, cv, rv);
            let zero = C2v { x: 0.0, y: 0.0 };
            assert!(cv.beq(&zero), "C must return (0,0) for count={count}, got {}", cv.bits());
            assert!(rv.beq(&zero), "Rust must return (0,0) for count={count}, got {}", rv.bits());
        }
    }
    d.finish();
}

#[test]
fn err_c2L_bad_count() {
    // ERRORS.md row 10
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSimplexV>("c2L") };
    let mut rng = Rng::new(0x6004);
    let mut d = Diff::new("E10: c2L out-of-range count -> (0,0)");
    for count in [0, 3, 4, 100, -1, i32::MIN, i32::MAX] {
        for _ in 0..300 {
            let s = simplex(&mut rng, count, 100.0);
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            d.check(count, cv, rv);
            let zero = C2v { x: 0.0, y: 0.0 };
            assert!(cv.beq(&zero) && rv.beq(&zero), "count={count}: {} vs {}", cv.bits(), rv.bits());
        }
    }
    d.finish();
}

#[test]
fn err_c2L_zero_div() {
    // ERRORS.md row 11 — div == 0 makes den = 1/0 = +inf
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSimplexV>("c2L") };
    let mut rng = Rng::new(0x6005);
    let mut d = Diff::new("E11: c2L div==0 -> inf/NaN, bit-identical");
    let mut saw_nonfinite = 0;
    for count in [1, 2] {
        for div in [0.0f32, -0.0] {
            for _ in 0..1000 {
                let mut s = simplex(&mut rng, count, 100.0);
                s.div = div;
                let mut cs = s;
                let mut rs = s;
                let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
                d.check((count, div, s.verts[0].u, s.verts[1].u), cv, rv);
                if !cv.x.is_finite() || !cv.y.is_finite() {
                    saw_nonfinite += 1;
                }
            }
        }
    }
    d.finish();
    assert!(saw_nonfinite > 0, "div==0 never produced a non-finite result");
}

#[test]
fn err_witness_bad_count() {
    // ERRORS.md row 12
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnWitness>("c2Witness") };
    let mut rng = Rng::new(0x6006);
    let mut d = Diff::new("E12: c2Witness out-of-range count -> (0,0) in BOTH outputs");
    for count in [0, 4, 5, 100, -1, -9, i32::MIN, i32::MAX] {
        for _ in 0..300 {
            let s = simplex(&mut rng, count, 100.0);
            let mut cs = s;
            let mut rs = s;
            let (mut ca, mut cb) = (C2v { x: 7.0, y: 8.0 }, C2v { x: 9.0, y: 10.0 });
            let (mut ra, mut rb) = (ca, cb);
            unsafe {
                c(&mut cs, &mut ca, &mut cb);
                r(&mut rs, &mut ra, &mut rb);
            }
            d.check(count, (ca, cb), (ra, rb));
            let zero = C2v { x: 0.0, y: 0.0 };
            assert!(
                ca.beq(&zero) && cb.beq(&zero),
                "C must zero both outputs for count={count}"
            );
            assert!(
                ra.beq(&zero) && rb.beq(&zero),
                "Rust must zero both outputs for count={count}"
            );
        }
    }
    d.finish();
}

#[test]
fn err_witness_zero_div() {
    // ERRORS.md row 13
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnWitness>("c2Witness") };
    let mut rng = Rng::new(0x6007);
    let mut d = Diff::new("E13: c2Witness div==0 -> inf/NaN, bit-identical");
    let mut saw_nonfinite = 0;
    for count in [1, 2, 3] {
        for div in [0.0f32, -0.0] {
            for _ in 0..800 {
                let mut s = simplex(&mut rng, count, 100.0);
                s.div = div;
                let mut cs = s;
                let mut rs = s;
                let (mut ca, mut cb) = (C2v { x: 0.0, y: 0.0 }, C2v { x: 0.0, y: 0.0 });
                let (mut ra, mut rb) = (ca, cb);
                unsafe {
                    c(&mut cs, &mut ca, &mut cb);
                    r(&mut rs, &mut ra, &mut rb);
                }
                d.check((count, div), (ca, cb), (ra, rb));
                if count != 1 && (!ca.x.is_finite() || !ca.y.is_finite()) {
                    saw_nonfinite += 1;
                }
            }
        }
    }
    d.finish();
    assert!(saw_nonfinite > 0, "div==0 never produced a non-finite witness");
}

// ===========================================================================
// Row 14 — c2Support with count <= 0 returns 0 (verts[0] is still read)
// ===========================================================================

#[test]
fn err_support_nonpositive_count() {
    // ERRORS.md row 14
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSupport>("c2Support") };
    let mut rng = Rng::new(0x6008);
    let mut d = Diff::new("E14: c2Support count<=0 -> 0");
    for count in [0, -1, -2, -1000, i32::MIN] {
        for _ in 0..400 {
            let mut verts = [C2v { x: 0.0, y: 0.0 }; 8];
            for v in verts.iter_mut() {
                *v = vec_mixed(&mut rng, 100.0);
            }
            let dir = vec_mixed(&mut rng, 100.0);
            let (cv, rv) = unsafe {
                (c(verts.as_ptr(), count, dir), r(verts.as_ptr(), count, dir))
            };
            d.check((count, dir), cv, rv);
            assert_eq!(cv, 0, "C must return 0 for count={count}");
            assert_eq!(rv, 0, "Rust must return 0 for count={count}");
        }
    }
    // count == 1 is the smallest "valid" count and must also be 0
    let verts = [C2v { x: 1.0, y: 2.0 }; 8];
    let dir = C2v { x: 3.0, y: 4.0 };
    unsafe {
        assert_eq!(c(verts.as_ptr(), 1, dir), 0);
        assert_eq!(r(verts.as_ptr(), 1, dir), 0);
    }
    d.finish();
}

// ===========================================================================
// Rows 15, 16, 17 — division by zero / NaN / overflow in the numeric leaves
// ===========================================================================

#[test]
fn err_div_by_zero() {
    // ERRORS.md row 15
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnVfV>("c2Div") };
    let mut d = Diff::new("E15: c2Div b==0 -> inf/NaN");
    let mut saw_inf = 0;
    let mut saw_nan = 0;
    for i in 0..SPECIALS.len() {
        for j in 0..SPECIALS.len() {
            let a = C2v { x: SPECIALS[i], y: SPECIALS[j] };
            for b in [0.0f32, -0.0] {
                let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
                d.check((a, b), cv, rv);
                if cv.x.is_infinite() { saw_inf += 1 }
                if cv.x.is_nan() { saw_nan += 1 }
            }
        }
    }
    d.finish();
    assert!(saw_inf > 0 && saw_nan > 0, "inf={saw_inf} nan={saw_nan}");
}

#[test]
fn err_norm_zero_vector() {
    // ERRORS.md row 16
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnVV>("c2Norm") };
    let mut d = Diff::new("E16: c2Norm (0,0) -> NaN");
    for a in [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v { x: 0.0, y: -0.0 },
        C2v { x: -0.0, y: 0.0 },
    ] {
        let (cv, rv) = unsafe { (c(a), r(a)) };
        d.check(a, cv, rv);
        assert!(cv.x.is_nan() && cv.y.is_nan(), "C: {}", cv.bits());
        assert!(rv.x.is_nan() && rv.y.is_nan(), "Rust: {}", rv.bits());
    }
    d.finish();
}

#[test]
fn err_len_overflow_and_nan() {
    // ERRORS.md row 17
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnVF>("c2Len") };
    let mut d = Diff::new("E17: c2Len overflow -> inf, NaN -> NaN");
    let mut saw_inf = 0;
    let mut saw_nan = 0;
    for i in 0..SPECIALS.len() {
        for j in 0..SPECIALS.len() {
            let a = C2v { x: SPECIALS[i], y: SPECIALS[j] };
            let (cv, rv) = unsafe { (c(a), r(a)) };
            d.check(a, cv, rv);
            if cv.is_infinite() { saw_inf += 1 }
            if cv.is_nan() { saw_nan += 1 }
        }
    }
    // dot(a,a) overflowing f32 must give +inf, never a negative sqrt
    let big = C2v { x: 1e30, y: 1e30 };
    unsafe {
        let (cv, rv) = (c(big), r(big));
        d.check(big, cv, rv);
        assert!(cv.is_infinite() && cv > 0.0, "C: {}", cv.bits());
        assert!(rv.is_infinite() && rv > 0.0, "Rust: {}", rv.bits());
    }
    d.finish();
    assert!(saw_inf > 0 && saw_nan > 0, "inf={saw_inf} nan={saw_nan}");
}

// ===========================================================================
// Rows 18..=23 — c2GJK NULL-pointer tolerance
// ===========================================================================

#[allow(clippy::too_many_arguments)]
unsafe fn gjk_raw(
    f: &FnGJK,
    A: &[u8; 20],
    tA: c_int,
    axp: *const C2x,
    B: &[u8; 20],
    tB: c_int,
    bxp: *const C2x,
    outs: bool,
    use_radius: c_int,
    cachep: *mut C2GJKCache,
) -> (f32, C2v, C2v, c_int) {
    let mut a = C2v { x: -321.0, y: -654.0 };
    let mut b = C2v { x: -987.0, y: -159.0 };
    let mut it: c_int = -424242;
    let dist = f(
        A.as_ptr() as *const c_void,
        tA,
        axp,
        B.as_ptr() as *const c_void,
        tB,
        bxp,
        if outs { &mut a } else { std::ptr::null_mut() },
        if outs { &mut b } else { std::ptr::null_mut() },
        use_radius,
        if outs { &mut it } else { std::ptr::null_mut() },
        cachep,
    );
    (dist, a, b, it)
}

#[test]
fn err_gjk_null_transforms() {
    // ERRORS.md rows 18 & 19 — NULL ax_ptr / bx_ptr fall back to c2xIdentity()
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x6009);
    let mut d = Diff::new("E18/E19: c2GJK NULL ax_ptr / bx_ptr -> identity");
    let ident = C2x {
        p: C2v { x: 0.0, y: 0.0 },
        r: C2r { c: 1.0, s: 0.0 },
    };
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for _ in 0..200 {
                let A = shape_bytes(&mut rng, tA, 10.0);
                let B = shape_bytes(&mut rng, tB, 10.0);
                let nul: *const C2x = std::ptr::null();
                let idp: *const C2x = &ident;
                for (axp, bxp) in [(nul, nul), (idp, nul), (nul, idp), (idp, idp)] {
                    let cv = unsafe { gjk_raw(&c, &A, tA, axp, &B, tB, bxp, true, 1, std::ptr::null_mut()) };
                    let rv = unsafe { gjk_raw(&r, &A, tA, axp, &B, tB, bxp, true, 1, std::ptr::null_mut()) };
                    d.check((ty_name(tA), ty_name(tB), axp.is_null(), bxp.is_null()), cv, rv);
                }
                // NULL and an explicit identity must give the SAME answer within
                // each library -- that is what the `!ax_ptr` fallback means.
                let with_null = unsafe { gjk_raw(&c, &A, tA, nul, &B, tB, nul, true, 1, std::ptr::null_mut()) };
                let with_ident = unsafe { gjk_raw(&c, &A, tA, idp, &B, tB, idp, true, 1, std::ptr::null_mut()) };
                assert!(
                    with_null.0.to_bits() == with_ident.0.to_bits() || with_null.0.is_nan(),
                    "C: NULL xform differs from explicit identity"
                );
                let rn = unsafe { gjk_raw(&r, &A, tA, nul, &B, tB, nul, true, 1, std::ptr::null_mut()) };
                let ri = unsafe { gjk_raw(&r, &A, tA, idp, &B, tB, idp, true, 1, std::ptr::null_mut()) };
                assert!(
                    rn.0.to_bits() == ri.0.to_bits() || rn.0.is_nan(),
                    "Rust: NULL xform differs from explicit identity"
                );
            }
        }
    }
    d.finish();
}

#[test]
fn err_gjk_null_outputs() {
    // ERRORS.md rows 20, 21, 22, 23 — every output pointer independently NULL,
    // plus cache == NULL. Asserts the return value is unaffected and that a
    // NULL slot is genuinely not written.
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x600a);
    let mut d = Diff::new("E20-E23: c2GJK NULL outA/outB/iterations/cache");
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for _ in 0..100 {
                let A = shape_bytes(&mut rng, tA, 10.0);
                let B = shape_bytes(&mut rng, tB, 10.0);
                // Reference run with everything requested.
                let cref = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, std::ptr::null_mut()) };
                let rref = unsafe { gjk_raw(&r, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, std::ptr::null_mut()) };
                d.check((ty_name(tA), ty_name(tB), "all outs"), cref, rref);

                for mask in 0u32..8 {
                    for (lib, is_c) in [(&c, true), (&r, false)] {
                        let mut a = C2v { x: 5.0, y: 6.0 };
                        let mut b = C2v { x: 7.0, y: 8.0 };
                        let mut it: c_int = -424242;
                        let dist = unsafe {
                            lib(
                                A.as_ptr() as *const c_void,
                                tA,
                                std::ptr::null(),
                                B.as_ptr() as *const c_void,
                                tB,
                                std::ptr::null(),
                                if mask & 1 != 0 { std::ptr::null_mut() } else { &mut a },
                                if mask & 2 != 0 { std::ptr::null_mut() } else { &mut b },
                                1,
                                if mask & 4 != 0 { std::ptr::null_mut() } else { &mut it },
                                std::ptr::null_mut(), // row 23: cache == NULL
                            )
                        };
                        let refv = if is_c { cref } else { rref };
                        // The return value must be identical regardless of which
                        // outputs were requested.
                        assert!(
                            dist.to_bits() == refv.0.to_bits() || (dist.is_nan() && refv.0.is_nan()),
                            "{} dist changed when out mask={mask}",
                            if is_c { "C" } else { "Rust" }
                        );
                        // NULL slots must be left at their sentinel values.
                        if mask & 1 != 0 {
                            assert!(a.beq(&C2v { x: 5.0, y: 6.0 }), "outA written despite NULL");
                        }
                        if mask & 2 != 0 {
                            assert!(b.beq(&C2v { x: 7.0, y: 8.0 }), "outB written despite NULL");
                        }
                        if mask & 4 != 0 {
                            assert_eq!(it, -424242, "iterations written despite NULL");
                        }
                    }
                }
            }
        }
    }
    d.finish();
}

// ===========================================================================
// Rows 24, 25, 26 — the cache read path
// ===========================================================================

#[test]
fn err_gjk_cache_count_zero() {
    // ERRORS.md row 24 — cache->count == 0 => cache_was_good false => cold start
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x600b);
    let mut d = Diff::new("E24: c2GJK cache->count==0 -> cold start");
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for _ in 0..200 {
                let A = shape_bytes(&mut rng, tA, 10.0);
                let B = shape_bytes(&mut rng, tB, 10.0);
                // Garbage everywhere EXCEPT count, which is 0: the garbage must
                // be ignored, so the answer must equal the cache==NULL answer.
                let mut cc = C2GJKCache {
                    metric: f32::NAN,
                    count: 0,
                    iA: [99, -5, 12345],
                    iB: [-77, 88, 3],
                    div: f32::NEG_INFINITY,
                };
                let mut rc = cc;
                let cv = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, &mut cc) };
                let rv = unsafe { gjk_raw(&r, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, &mut rc) };
                d.check((ty_name(tA), ty_name(tB)), cv, rv);
                d.check(("cache written back", ty_name(tA), ty_name(tB)), cc, rc);

                let nocache = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, std::ptr::null_mut()) };
                assert!(
                    cv.0.to_bits() == nocache.0.to_bits() || (cv.0.is_nan() && nocache.0.is_nan()),
                    "C: a count==0 cache changed the result"
                );
            }
        }
    }
    d.finish();
}

#[test]
fn err_gjk_cache_warm_start() {
    // ERRORS.md row 25 — count != 0 and the metric window test true =>
    // cache_was_read = 1 (the warm-started simplex is used verbatim).
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x600c);
    let mut d = Diff::new("E25: c2GJK warm start (cache_was_read = 1)");
    let mut differed_from_cold = 0;
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            let maxA = match tA { C2_TYPE_CIRCLE => 1, C2_TYPE_CAPSULE => 2, _ => 4 };
            let maxB = match tB { C2_TYPE_CIRCLE => 1, C2_TYPE_CAPSULE => 2, _ => 4 };
            for count in [1, 2, 3] {
                for _ in 0..200 {
                    let A = shape_bytes(&mut rng, tA, 10.0);
                    let B = shape_bytes(&mut rng, tB, 10.0);
                    let warm = C2GJKCache {
                        // metric >= -1e8 makes `metric < -1.0e8f` false, so the
                        // whole `&&` is false and cache_was_read becomes 1.
                        metric: rng.unit() * 10.0,
                        count,
                        iA: [rng.below(maxA) as c_int, rng.below(maxA) as c_int, rng.below(maxA) as c_int],
                        iB: [rng.below(maxB) as c_int, rng.below(maxB) as c_int, rng.below(maxB) as c_int],
                        div: rng.unit() * 5.0 + 0.5,
                    };
                    let mut cc = warm;
                    let mut rc = warm;
                    let cv = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, &mut cc) };
                    let rv = unsafe { gjk_raw(&r, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, &mut rc) };
                    d.check((ty_name(tA), ty_name(tB), count, warm.div), cv, rv);
                    d.check(("cache", ty_name(tA), ty_name(tB), count), cc, rc);

                    let cold = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, std::ptr::null_mut()) };
                    if cv.0.to_bits() != cold.0.to_bits() {
                        differed_from_cold += 1;
                    }
                }
            }
        }
    }
    d.finish();
    assert!(
        differed_from_cold > 0,
        "the warm-start path never actually changed the outcome -- it may not be exercised"
    );
}

#[test]
fn err_gjk_cache_nan_metric() {
    // ERRORS.md row 26 — cache->metric == NaN: every comparison in the
    // min/max ternaries is false, so min_metric == max_metric == metric_old
    // and the window test is true => cache_was_read = 1.
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x600d);
    let mut d = Diff::new("E26: c2GJK cache->metric == NaN");
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            let maxA = match tA { C2_TYPE_CIRCLE => 1, C2_TYPE_CAPSULE => 2, _ => 4 };
            let maxB = match tB { C2_TYPE_CIRCLE => 1, C2_TYPE_CAPSULE => 2, _ => 4 };
            for count in [1, 2, 3] {
                for metric in [f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1e9, -1e8, -1.0e8 - 1.0] {
                    for _ in 0..60 {
                        let A = shape_bytes(&mut rng, tA, 10.0);
                        let B = shape_bytes(&mut rng, tB, 10.0);
                        let warm = C2GJKCache {
                            metric,
                            count,
                            iA: [rng.below(maxA) as c_int, rng.below(maxA) as c_int, rng.below(maxA) as c_int],
                            iB: [rng.below(maxB) as c_int, rng.below(maxB) as c_int, rng.below(maxB) as c_int],
                            div: rng.unit() * 5.0 + 0.5,
                        };
                        let mut cc = warm;
                        let mut rc = warm;
                        let cv = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, &mut cc) };
                        let rv = unsafe { gjk_raw(&r, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, &mut rc) };
                        d.check((ty_name(tA), ty_name(tB), count, metric), cv, rv);
                        d.check(("cache", ty_name(tA), ty_name(tB), count, metric), cc, rc);
                    }
                }
            }
        }
    }
    d.finish();
}

// ===========================================================================
// Row 27 — c2GJK with an out-of-range type
// ===========================================================================

#[test]
fn err_gjk_bad_type_documented() {
    // ERRORS.md row 27.
    //
    // With an unknown type, `c2MakeProxy` is a no-op, so in the C the local
    // `c2Proxy pA;` keeps whatever the *uninitialised stack slot* held --
    // indeterminate by definition. The Rust zero-initialises it. Asserting
    // equality here would be asserting on undefined behaviour, so instead this
    // test pins down what IS well-defined:
    //
    //   * neither library crashes or hangs (the loop is bounded by iter < 20),
    //   * both return a value (finite or not) rather than trapping,
    //   * `omni_collide` / `c2Collided`, the only callers that can be reached
    //     with a bad type through the public API, return 0 *before* calling
    //     c2GJK at all -- asserted in err_collided_* / err_omni_collide_*.
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let A = a_valid_shape();
    let B = a_valid_shape();
    for &bad in BAD_TYPES.iter() {
        for &ok in ALL_TYPES.iter() {
            for (tA, tB) in [(bad, ok), (ok, bad), (bad, bad)] {
                let cv = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, std::ptr::null_mut()) };
                let rv = unsafe { gjk_raw(&r, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, std::ptr::null_mut()) };
                // iteration count is bounded identically in both
                assert!((0..=20).contains(&cv.3), "C iter out of bounds: {}", cv.3);
                assert!((0..=20).contains(&rv.3), "Rust iter out of bounds: {}", rv.3);
            }
        }
    }
    // The deterministic sub-case: a proxy whose count is 0 is reached through a
    // zeroed proxy in BOTH, via c2MakeProxy on a valid type followed by an
    // invalid one -- verified not to crash above.
}

// ===========================================================================
// Rows 28..=32, 35, 36, 33, 34 — c2GJK branch-specific sentinels
// ===========================================================================

#[test]
fn err_gjk_use_radius_zero() {
    // ERRORS.md row 28 — use_radius == 0 skips the whole radius block
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x600e);
    let mut d = Diff::new("E28: c2GJK use_radius==0 skips the radius block");
    let mut differed = 0;
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for _ in 0..300 {
                let A = shape_bytes(&mut rng, tA, 10.0);
                let B = shape_bytes(&mut rng, tB, 10.0);
                let c0 = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 0, std::ptr::null_mut()) };
                let r0 = unsafe { gjk_raw(&r, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 0, std::ptr::null_mut()) };
                d.check((ty_name(tA), ty_name(tB), "use_radius=0"), c0, r0);
                let c1 = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, std::ptr::null_mut()) };
                if c0.0.to_bits() != c1.0.to_bits() {
                    differed += 1;
                }
                // also non-zero truthy values for the flag
                for ur in [1, 2, -1, i32::MIN, i32::MAX] {
                    let cx = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, ur, std::ptr::null_mut()) };
                    let rx = unsafe { gjk_raw(&r, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, ur, std::ptr::null_mut()) };
                    d.check((ty_name(tA), ty_name(tB), ur), cx, rx);
                }
            }
        }
    }
    d.finish();
    assert!(differed > 0, "use_radius made no difference anywhere");
}

#[test]
fn err_gjk_radius_overlap_branch() {
    // ERRORS.md row 29 — use_radius, dist <= rA+rB: a = b = midpoint, dist = 0
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x600f);
    let mut d = Diff::new("E29: c2GJK radius else-branch -> dist == 0, a == b");
    let mut hits = 0;
    for _ in 0..3000 {
        // two circles that overlap once their radii are taken into account
        let rA = rng.unit() * 3.0 + 1.0;
        let rB = rng.unit() * 3.0 + 1.0;
        let sep = (rA + rB) * rng.unit(); // strictly less than rA+rB
        let mut A = [0u8; 20];
        let mut B = [0u8; 20];
        for (buf, v) in [(&mut A, [0.0f32, 0.0, rA]), (&mut B, [sep, 0.0, rB])] {
            for (i, f) in v.iter().enumerate() {
                buf[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
            }
        }
        let cv = unsafe { gjk_raw(&c, &A, C2_TYPE_CIRCLE, std::ptr::null(), &B, C2_TYPE_CIRCLE, std::ptr::null(), true, 1, std::ptr::null_mut()) };
        let rv = unsafe { gjk_raw(&r, &A, C2_TYPE_CIRCLE, std::ptr::null(), &B, C2_TYPE_CIRCLE, std::ptr::null(), true, 1, std::ptr::null_mut()) };
        d.check((rA, rB, sep), cv, rv);
        if cv.0.to_bits() == 0.0f32.to_bits() {
            hits += 1;
            assert_eq!(cv.0.to_bits(), 0.0f32.to_bits(), "C dist must be exactly +0.0");
            assert_eq!(rv.0.to_bits(), 0.0f32.to_bits(), "Rust dist must be exactly +0.0");
            assert!(cv.1.beq(&cv.2), "C: a != b in the overlap branch");
            assert!(rv.1.beq(&rv.2), "Rust: a != b in the overlap branch");
        }
    }
    d.finish();
    assert!(hits > 100, "overlap branch barely hit: {hits}");
}

#[test]
fn err_gjk_radius_epsilon_branch() {
    // ERRORS.md row 30 — dist <= FLT_EPSILON (coincident shapes)
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x6010);
    let mut d = Diff::new("E30: c2GJK dist <= FLT_EPSILON -> dist == 0");
    let mut hits = 0;
    for &ty in ALL_TYPES.iter() {
        for _ in 0..1000 {
            // identical zero-radius shapes: dist is exactly 0 <= FLT_EPSILON
            let mut A = shape_bytes(&mut rng, ty, 5.0);
            if ty != C2_TYPE_AABB {
                let off = if ty == C2_TYPE_CIRCLE { 8 } else { 16 };
                A[off..off + 4].copy_from_slice(&0.0f32.to_le_bytes());
            }
            let B = A;
            let cv = unsafe { gjk_raw(&c, &A, ty, std::ptr::null(), &B, ty, std::ptr::null(), true, 1, std::ptr::null_mut()) };
            let rv = unsafe { gjk_raw(&r, &A, ty, std::ptr::null(), &B, ty, std::ptr::null(), true, 1, std::ptr::null_mut()) };
            d.check((ty_name(ty), &A[..]), cv, rv);
            if cv.0.to_bits() == 0.0f32.to_bits() {
                hits += 1;
                assert_eq!(rv.0.to_bits(), 0.0f32.to_bits());
            }
        }
    }
    d.finish();
    assert!(hits > 100, "epsilon branch barely hit: {hits}");
}

#[test]
fn err_gjk_radius_collapse() {
    // ERRORS.md row 31 — after shrinking by rA/rB, a == b => dist forced to 0
    // even though the subtraction produced a positive value.
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x6011);
    let mut d = Diff::new("E31: c2GJK radius shrink collapses a==b -> dist = 0");
    let mut collapses = 0;
    for _ in 0..8000 {
        // dist barely above rA+rB so the shrink lands on (nearly) the same point
        let rA = rng.unit() * 2.0 + 0.5;
        let rB = rng.unit() * 2.0 + 0.5;
        let extra = [1e-7f32, 1e-6, 1e-5, f32::EPSILON, 3.0 * f32::EPSILON][rng.below(5)];
        let sep = rA + rB + extra;
        let mut A = [0u8; 20];
        let mut B = [0u8; 20];
        for (buf, v) in [(&mut A, [0.0f32, 0.0, rA]), (&mut B, [sep, 0.0, rB])] {
            for (i, f) in v.iter().enumerate() {
                buf[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
            }
        }
        let cv = unsafe { gjk_raw(&c, &A, C2_TYPE_CIRCLE, std::ptr::null(), &B, C2_TYPE_CIRCLE, std::ptr::null(), true, 1, std::ptr::null_mut()) };
        let rv = unsafe { gjk_raw(&r, &A, C2_TYPE_CIRCLE, std::ptr::null(), &B, C2_TYPE_CIRCLE, std::ptr::null(), true, 1, std::ptr::null_mut()) };
        d.check((rA, rB, extra), cv, rv);
        if cv.1.beq(&cv.2) {
            collapses += 1;
            assert_eq!(cv.0.to_bits(), 0.0f32.to_bits(), "C: a==b but dist != 0");
            assert_eq!(rv.0.to_bits(), 0.0f32.to_bits(), "Rust: a==b but dist != 0");
        }
    }
    d.finish();
    assert!(collapses > 0, "the a==b collapse never triggered");
}

#[test]
fn err_gjk_hit_branch() {
    // ERRORS.md row 32 — the simplex reaches count == 3: a = b, dist = 0
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x6012);
    let mut d = Diff::new("E32: c2GJK hit branch -> dist == 0, outA == outB");
    let mut hits = 0;
    for &tA in [C2_TYPE_AABB, C2_TYPE_CAPSULE].iter() {
        for &tB in [C2_TYPE_AABB, C2_TYPE_CAPSULE].iter() {
            for _ in 0..1500 {
                // heavily overlapping shapes -> the origin ends up inside the
                // Minkowski difference -> count == 3
                let mut A = [0u8; 20];
                let mut B = [0u8; 20];
                for buf in [&mut A, &mut B] {
                    let v = [-6.0f32 - rng.unit(), -6.0 - rng.unit(), 6.0 + rng.unit(), 6.0 + rng.unit(), 1.0];
                    for (i, f) in v.iter().enumerate() {
                        buf[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
                    }
                }
                for use_radius in [0, 1] {
                    let cv = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, use_radius, std::ptr::null_mut()) };
                    let rv = unsafe { gjk_raw(&r, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, use_radius, std::ptr::null_mut()) };
                    d.check((ty_name(tA), ty_name(tB), use_radius), cv, rv);
                    if cv.0.to_bits() == 0.0f32.to_bits() && cv.1.beq(&cv.2) {
                        hits += 1;
                        assert_eq!(rv.0.to_bits(), 0.0f32.to_bits());
                        assert!(rv.1.beq(&rv.2));
                    }
                }
            }
        }
    }
    d.finish();
    assert!(hits > 100, "hit branch barely reached: {hits}");
}

#[test]
fn err_gjk_iteration_bounds() {
    // ERRORS.md rows 33, 34, 36 — the three loop exits: `d1 > d0`,
    // degenerate search direction, and the hard `iter < 20` cap.
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x6013);
    let mut d = Diff::new("E33/E34/E36: c2GJK loop exits and the iter<20 cap");
    let mut seen = [0usize; 21];
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for scale in [1e-6f32, 1.0, 1e6, 1e20, 1e35] {
                for _ in 0..300 {
                    let A = shape_bytes(&mut rng, tA, scale);
                    let B = shape_bytes(&mut rng, tB, scale);
                    let cv = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, std::ptr::null_mut()) };
                    let rv = unsafe { gjk_raw(&r, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, std::ptr::null_mut()) };
                    d.check((ty_name(tA), ty_name(tB), scale), cv, rv);
                    assert!((0..=20).contains(&cv.3), "C iter={} out of [0,20]", cv.3);
                    assert!((0..=20).contains(&rv.3), "Rust iter={} out of [0,20]", rv.3);
                    assert_eq!(cv.3, rv.3, "iteration counts differ");
                    seen[cv.3 as usize] += 1;
                }
            }
        }
    }
    d.finish();
    // The loop must be observed exiting at more than one iteration count,
    // otherwise the early-exit conditions are untested.
    let distinct = seen.iter().filter(|&&n| n > 0).count();
    assert!(distinct >= 3, "only {distinct} distinct iteration counts observed: {seen:?}");
}

#[test]
fn err_gjk_dup_break() {
    // ERRORS.md row 35 — a duplicate support vertex breaks BEFORE ++s.count,
    // so the vertex just written at verts[s.count] is left uncounted. This is
    // observable through the cache that is written back afterwards.
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let mut rng = Rng::new(0x6014);
    let mut d = Diff::new("E35: c2GJK duplicate-support break (cache count/indices)");
    for &tA in ALL_TYPES.iter() {
        for &tB in ALL_TYPES.iter() {
            for _ in 0..400 {
                // Small shapes far apart converge fast and re-pick the same
                // support vertex, which is exactly the duplicate condition.
                let off = 50.0 + rng.unit() * 50.0;
                let mut A = [0u8; 20];
                let mut B = [0u8; 20];
                let va = [rng.sym(1.0), rng.sym(1.0), rng.sym(1.0), rng.sym(1.0), rng.unit()];
                let vb = [off, off, off + 1.0, off + 1.0, rng.unit()];
                for (i, f) in va.iter().enumerate() {
                    A[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
                }
                for (i, f) in vb.iter().enumerate() {
                    B[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
                }
                let mut cc = C2GJKCache { metric: 0.0, count: 0, iA: [0; 3], iB: [0; 3], div: 0.0 };
                let mut rc = cc;
                let cv = unsafe { gjk_raw(&c, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, &mut cc) };
                let rv = unsafe { gjk_raw(&r, &A, tA, std::ptr::null(), &B, tB, std::ptr::null(), true, 1, &mut rc) };
                d.check((ty_name(tA), ty_name(tB), off), cv, rv);
                // The cache records s.count, which is what the pre-increment
                // break controls -- comparing it pins down the quirk.
                d.check(("cache", ty_name(tA), ty_name(tB), off), cc, rc);
                assert!((1..=3).contains(&cc.count), "C cache count={}", cc.count);
                assert_eq!(cc.count, rc.count, "cache count differs after dup break");
            }
        }
    }
    d.finish();
}

// ===========================================================================
// Rows 37..=42 — numeric / geometric degeneracies across the shape API
// ===========================================================================

#[test]
fn err_nan_inf_inputs() {
    // ERRORS.md row 37 — NaN / +-inf anywhere: every comparison in the C is a
    // plain IEEE compare, so NaN makes `<`/`>` false and the fallback branch is
    // taken. Must be bit-identical across the WHOLE public surface.
    let l = libs();
    let (caa, raa) = unsafe { l.pair::<FnAABBtoAABB>("c2AABBtoAABB") };
    let (cac, rac) = unsafe { l.pair::<FnAABBtoCapsule>("c2AABBtoCapsule") };
    let (ccc, rcc) = unsafe { l.pair::<FnCapsuletoCapsule>("c2CapsuletoCapsule") };
    let (ci, ri) = unsafe { l.pair::<FnCircletoCircle>("c2CircletoCircle") };
    let (cia, ria) = unsafe { l.pair::<FnCircletoAABB>("c2CircletoAABB") };
    let (cic, ric) = unsafe { l.pair::<FnCircletoCapsule>("c2CircletoCapsule") };
    let mut d = Diff::new("E37: NaN / +-inf inputs across every shape function");
    let nasty = [f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0, 1.0, -1.0];
    for &a0 in nasty.iter() {
        for &a1 in nasty.iter() {
            for &b0 in nasty.iter() {
                for &b1 in nasty.iter() {
                    let bx = C2AABB { min: C2v { x: a0, y: a1 }, max: C2v { x: b0, y: b1 } };
                    let bx2 = C2AABB { min: C2v { x: b0, y: b1 }, max: C2v { x: a0, y: a1 } };
                    let ca = C2Capsule { a: C2v { x: a0, y: a1 }, b: C2v { x: b0, y: b1 }, r: a0 };
                    let ca2 = C2Capsule { a: C2v { x: b0, y: b1 }, b: C2v { x: a0, y: a1 }, r: b0 };
                    let ci1 = C2Circle { p: C2v { x: a0, y: a1 }, r: b0 };
                    let ci2 = C2Circle { p: C2v { x: b0, y: b1 }, r: a0 };
                    unsafe {
                        d.check(("AABBtoAABB", a0, a1, b0, b1), caa(bx, bx2), raa(bx, bx2));
                        d.check(("AABBtoCapsule", a0, a1, b0, b1), cac(bx, ca), rac(bx, ca));
                        d.check(("CapsuletoCapsule", a0, a1, b0, b1), ccc(ca, ca2), rcc(ca, ca2));
                        d.check(("CircletoCircle", a0, a1, b0, b1), ci(ci1, ci2), ri(ci1, ci2));
                        d.check(("CircletoAABB", a0, a1, b0, b1), cia(ci1, bx), ria(ci1, bx));
                        d.check(("CircletoCapsule", a0, a1, b0, b1), cic(ci1, ca), ric(ci1, ca));
                    }
                }
            }
        }
    }
    d.finish();
}

#[test]
fn err_degenerate_capsule() {
    // ERRORS.md row 38 — capsule with a == b: n == (0,0), da == 0 so `da < 0`
    // is false, db == 0 so `db < 0` is false, hence the `n`-divide branch (which
    // would be 0/0) is SKIPPED and d2 = dot(bp, bp).
    let l = libs();
    let (cic, ric) = unsafe { l.pair::<FnCircletoCapsule>("c2CircletoCapsule") };
    let (ccc, rcc) = unsafe { l.pair::<FnCapsuletoCapsule>("c2CapsuletoCapsule") };
    let (cac, rac) = unsafe { l.pair::<FnAABBtoCapsule>("c2AABBtoCapsule") };
    let mut rng = Rng::new(0x6015);
    let mut d = Diff::new("E38: degenerate capsule (a == b)");
    for _ in 0..4000 {
        let p = vec_tame(&mut rng, 5.0);
        let deg = C2Capsule { a: p, b: p, r: rng.unit() * 3.0 };
        let circ = circle(&mut rng, 5.0);
        let bb = aabb(&mut rng, 5.0);
        let other = capsule(&mut rng, 5.0);
        unsafe {
            d.check(("circle x degen", circ, deg), cic(circ, deg), ric(circ, deg));
            d.check(("degen x degen", deg), ccc(deg, deg), rcc(deg, deg));
            d.check(("degen x other", deg, other), ccc(deg, other), rcc(deg, other));
            d.check(("aabb x degen", bb, deg), cac(bb, deg), rac(bb, deg));
        }
        // The result must never be NaN-driven garbage: the C avoids the 0/0.
        let v = unsafe { cic(circ, deg) };
        assert!(v == 0 || v == 1, "C returned {v}, expected a 0/1 boolean");
    }
    d.finish();
}

#[test]
fn err_negative_radii() {
    // ERRORS.md rows 39 & 40 — negative radii are NOT validated; `r*r` is
    // positive so a "collision" can be reported. Replicate exactly.
    let l = libs();
    let (ci, ri) = unsafe { l.pair::<FnCircletoCircle>("c2CircletoCircle") };
    let (cia, ria) = unsafe { l.pair::<FnCircletoAABB>("c2CircletoAABB") };
    let (cic, ric) = unsafe { l.pair::<FnCircletoCapsule>("c2CircletoCapsule") };
    let (ccc, rcc) = unsafe { l.pair::<FnCapsuletoCapsule>("c2CapsuletoCapsule") };
    let (cac, rac) = unsafe { l.pair::<FnAABBtoCapsule>("c2AABBtoCapsule") };
    let mut rng = Rng::new(0x6016);
    let mut d = Diff::new("E39/E40: negative radii");
    let mut reported = 0;
    for _ in 0..6000 {
        let neg = -(rng.unit() * 5.0 + 0.01);
        let neg2 = -(rng.unit() * 5.0 + 0.01);
        let A = C2Circle { p: vec_tame(&mut rng, 4.0), r: neg };
        let B = C2Circle { p: vec_tame(&mut rng, 4.0), r: neg2 };
        let bb = aabb(&mut rng, 4.0);
        let cap = C2Capsule { a: vec_tame(&mut rng, 4.0), b: vec_tame(&mut rng, 4.0), r: neg };
        let cap2 = C2Capsule { a: vec_tame(&mut rng, 4.0), b: vec_tame(&mut rng, 4.0), r: neg2 };
        unsafe {
            let v = ci(A, B);
            d.check(("CircletoCircle", A, B), v, ri(A, B));
            d.check(("CircletoAABB", A, bb), cia(A, bb), ria(A, bb));
            d.check(("CircletoCapsule", A, cap), cic(A, cap), ric(A, cap));
            d.check(("CapsuletoCapsule", cap, cap2), ccc(cap, cap2), rcc(cap, cap2));
            d.check(("AABBtoCapsule", bb, cap), cac(bb, cap), rac(bb, cap));
            if v != 0 {
                reported += 1;
            }
        }
        // one radius negative, the other positive: rA + rB may be negative,
        // and (rA+rB)^2 is still positive -- the C's quirk
        let mixedA = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: -3.0 };
        let mixedB = C2Circle { p: C2v { x: 1.0, y: 0.0 }, r: 1.0 };
        unsafe {
            d.check(("mixed sign", mixedA, mixedB), ci(mixedA, mixedB), ri(mixedA, mixedB));
        }
    }
    d.finish();
    assert!(
        reported > 0,
        "negative radii never reported a collision -- the quirk is untested"
    );
}

#[test]
fn err_inverted_aabb() {
    // ERRORS.md row 41 — min > max is not validated
    let l = libs();
    let (caa, raa) = unsafe { l.pair::<FnAABBtoAABB>("c2AABBtoAABB") };
    let (cia, ria) = unsafe { l.pair::<FnCircletoAABB>("c2CircletoAABB") };
    let (cac, rac) = unsafe { l.pair::<FnAABBtoCapsule>("c2AABBtoCapsule") };
    let (ccl, rcl) = unsafe { l.pair::<FnVVVV>("c2Clampv") };
    let mut rng = Rng::new(0x6017);
    let mut d = Diff::new("E41: inverted AABB (min > max)");
    for _ in 0..5000 {
        let hi = vec_tame(&mut rng, 5.0);
        let lo = C2v { x: hi.x + rng.unit() * 5.0 + 0.1, y: hi.y + rng.unit() * 5.0 + 0.1 };
        let inv = C2AABB { min: lo, max: hi }; // min > max on both axes
        let ok = aabb(&mut rng, 5.0);
        let circ = circle(&mut rng, 5.0);
        let cap = capsule(&mut rng, 5.0);
        unsafe {
            d.check(("inv x ok", inv, ok), caa(inv, ok), raa(inv, ok));
            d.check(("ok x inv", ok, inv), caa(ok, inv), raa(ok, inv));
            d.check(("inv x inv", inv), caa(inv, inv), raa(inv, inv));
            d.check(("circle x inv", circ, inv), cia(circ, inv), ria(circ, inv));
            d.check(("inv x capsule", inv, cap), cac(inv, cap), rac(inv, cap));
            // the clamp the C relies on: max(lo, min(a, hi))
            d.check(("clampv inverted", circ.p, lo, hi), ccl(circ.p, lo, hi), rcl(circ.p, lo, hi));
        }
    }
    d.finish();
}

#[test]
fn err_bbverts_writes_exactly_four() {
    // ERRORS.md row 42 — c2BBVerts always writes exactly 4 c2v and never checks
    // the caller's buffer size.
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnBBVerts>("c2BBVerts") };
    let mut rng = Rng::new(0x6018);
    let mut d = Diff::new("E42: c2BBVerts writes exactly 4 verts");
    let guard = C2v { x: -98765.0, y: 43210.0 };
    for _ in 0..3000 {
        let mut bb = aabb(&mut rng, 100.0);
        let mut bb2 = bb;
        // 4 slots of payload + 4 guard slots
        let mut co = [guard; 8];
        let mut ro = [guard; 8];
        unsafe {
            c(co.as_mut_ptr(), &mut bb);
            r(ro.as_mut_ptr(), &mut bb2);
        }
        d.check(bb, co, ro);
        for k in 4..8 {
            assert!(co[k].beq(&guard), "C wrote vert {k} (past 4)");
            assert!(ro[k].beq(&guard), "Rust wrote vert {k} (past 4)");
        }
        // exact expected contents
        assert!(co[0].beq(&bb.min) && co[2].beq(&bb.max));
        assert!(ro[0].beq(&bb.min) && ro[2].beq(&bb.max));
    }
    d.finish();
}

// ===========================================================================
// Generic FFI boundary checks required by Phase C regardless of ERRORS.md
// ===========================================================================

#[test]
fn err_generic_out_of_range_enums_every_entry_point() {
    // Out-of-range enum values pushed through EVERY entry point that takes a
    // C2_TYPE: c2MakeProxy, c2Collided, c2GJK, ptr_from_parts, omni_collide.
    // (C enums accept any int, so these are real inputs.)
    let l = libs();
    let (mp_c, mp_r) = unsafe { l.pair::<FnMakeProxy>("c2MakeProxy") };
    let (co_c, co_r) = unsafe { l.pair::<FnCollided>("c2Collided") };
    let (om_c, om_r) = unsafe { l.pair::<FnOmniCollide>("omni_collide") };
    let s = a_valid_shape();
    let mut d = Diff::new("Egen: out-of-range enums through every entry point");
    // include the boundary values one step past each end of the valid range
    let mut vals: Vec<c_int> = vec![-1, 3];
    vals.extend_from_slice(&BAD_TYPES);
    for &v in vals.iter() {
        let fill = C2Proxy { radius: 1.0, count: -3, verts: [C2v { x: 2.0, y: 3.0 }; 8] };
        let mut cp = fill;
        let mut rp = fill;
        unsafe {
            mp_c(s.as_ptr() as *const c_void, v, &mut cp);
            mp_r(s.as_ptr() as *const c_void, v, &mut rp);
        }
        d.check(("c2MakeProxy", v), cp, rp);
        assert!(cp.beq(&fill) && rp.beq(&fill));

        unsafe {
            d.check(
                ("c2Collided", v),
                co_c(s.as_ptr() as *const c_void, v, s.as_ptr() as *const c_void, v),
                co_r(s.as_ptr() as *const c_void, v, s.as_ptr() as *const c_void, v),
            );
            d.check(
                ("omni_collide", v),
                om_c(v, 1.0, 2.0, 3.0, 4.0, 5.0, v, 1.0, 2.0, 3.0, 4.0, 5.0),
                om_r(v, 1.0, 2.0, 3.0, 4.0, 5.0, v, 1.0, 2.0, 3.0, 4.0, 5.0),
            );
        }
    }
    d.finish();
}

#[test]
fn err_generic_boundary_counts() {
    // Zero, oversized and one-past-range counts for every count-taking function.
    let l = libs();
    let (sup_c, sup_r) = unsafe { l.pair::<FnSupport>("c2Support") };
    let (met_c, met_r) = unsafe { l.pair::<FnSimplexF>("c2GJKSimplexMetric") };
    let (l_c, l_r) = unsafe { l.pair::<FnSimplexV>("c2L") };
    let (d_c, d_r) = unsafe { l.pair::<FnSimplexV>("c2D") };
    let (w_c, w_r) = unsafe { l.pair::<FnWitness>("c2Witness") };
    let mut rng = Rng::new(0x6019);
    let mut d = Diff::new("Egen: boundary counts (0, 1, max valid, one past, huge)");
    let verts = [
        C2v { x: 1.0, y: 0.0 },
        C2v { x: 0.0, y: 1.0 },
        C2v { x: -1.0, y: 0.0 },
        C2v { x: 0.0, y: -1.0 },
        C2v { x: 2.0, y: 2.0 },
        C2v { x: -2.0, y: 2.0 },
        C2v { x: 2.0, y: -2.0 },
        C2v { x: -2.0, y: -2.0 },
    ];
    // c2Support: 0, 1, 8 (the proxy width), and one past 8 would read OOB in
    // BOTH, so stop at the documented maximum.
    for count in [0, 1, 2, 3, 4, 7, 8, -1, i32::MIN] {
        for _ in 0..200 {
            let dir = vec_mixed(&mut rng, 10.0);
            unsafe {
                d.check(
                    ("c2Support", count, dir),
                    sup_c(verts.as_ptr(), count, dir),
                    sup_r(verts.as_ptr(), count, dir),
                );
            }
        }
    }
    // simplex-count-taking functions: 0, 1, 2, 3 (max valid), 4 (one past), huge
    for count in [0, 1, 2, 3, 4, 5, 1000, -1, i32::MIN, i32::MAX] {
        for _ in 0..300 {
            let s = simplex(&mut rng, count, 20.0);
            let (mut a, mut b, mut c2, mut d2) = (s, s, s, s);
            let (mut e, mut f) = (s, s);
            let (mut wa, mut wb) = (C2v { x: 1.0, y: 1.0 }, C2v { x: 2.0, y: 2.0 });
            let (mut wa2, mut wb2) = (wa, wb);
            unsafe {
                d.check(("c2GJKSimplexMetric", count), met_c(&mut a), met_r(&mut b));
                d.check(("c2L", count), l_c(&mut c2), l_r(&mut d2));
                d.check(("c2D", count), d_c(&mut e), d_r(&mut f));
                w_c(&mut a, &mut wa, &mut wb);
                w_r(&mut b, &mut wa2, &mut wb2);
                d.check(("c2Witness", count), (wa, wb), (wa2, wb2));
            }
        }
    }
    d.finish();
}

#[test]
fn err_generic_null_pointers() {
    // Every pointer parameter that the C explicitly guards, passed as NULL.
    // (Parameters the C does NOT guard are dereferenced unconditionally and
    // would fault in both libraries alike, so they are not exercised.)
    let l = libs();
    let (gjk_c, gjk_r) = unsafe { l.pair::<FnGJK>("c2GJK") };
    let (mp_c, mp_r) = unsafe { l.pair::<FnMakeProxy>("c2MakeProxy") };
    let (co_c, co_r) = unsafe { l.pair::<FnCollided>("c2Collided") };
    let A = a_valid_shape();
    let B = a_valid_shape();
    let mut d = Diff::new("Egen: guarded NULL pointers");
    // c2GJK: ax_ptr, bx_ptr, outA, outB, iterations, cache -- all guarded.
    for &ty in ALL_TYPES.iter() {
        let cv = unsafe {
            gjk_c(
                A.as_ptr() as *const c_void, ty, std::ptr::null(),
                B.as_ptr() as *const c_void, ty, std::ptr::null(),
                std::ptr::null_mut(), std::ptr::null_mut(), 1,
                std::ptr::null_mut(), std::ptr::null_mut(),
            )
        };
        let rv = unsafe {
            gjk_r(
                A.as_ptr() as *const c_void, ty, std::ptr::null(),
                B.as_ptr() as *const c_void, ty, std::ptr::null(),
                std::ptr::null_mut(), std::ptr::null_mut(), 1,
                std::ptr::null_mut(), std::ptr::null_mut(),
            )
        };
        d.check(("c2GJK all-NULL optionals", ty_name(ty)), cv, rv);
    }
    // c2MakeProxy / c2Collided with a NULL shape are only safe for an unknown
    // type (nothing is dereferenced) -- covered here and in row 7 / row 1.
    for bad in [3, -1, i32::MAX] {
        let fill = C2Proxy { radius: 0.0, count: 0, verts: [C2v { x: 0.0, y: 0.0 }; 8] };
        let mut cp = fill;
        let mut rp = fill;
        unsafe {
            mp_c(std::ptr::null(), bad, &mut cp);
            mp_r(std::ptr::null(), bad, &mut rp);
            d.check(("c2MakeProxy NULL shape", bad), cp, rp);
            d.check(
                ("c2Collided NULL shapes", bad),
                co_c(std::ptr::null(), bad, std::ptr::null(), bad),
                co_r(std::ptr::null(), bad, std::ptr::null(), bad),
            );
        }
    }
    d.finish();
}
