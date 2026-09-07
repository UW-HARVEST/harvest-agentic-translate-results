//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every row constructs the exact invalid input / rejection condition, calls
//! BOTH `.so`s, and asserts the same sentinel / fallback result. Because this
//! library has no error codes, the "same rejection" assertion is bit-equality
//! of the fallback value the C actually produces (documented per row).
//!
//! Rows 37-40 of `ERRORS.md` are C undefined behaviour (uninitialised stack /
//! out-of-bounds); they are `#[ignore]`d with an explanation rather than
//! asserted, because their C result is not a function of the inputs.

mod common;
use common::*;
use std::ffi::c_void;

/// Out-of-range `int`s that a C enum parameter legally accepts over FFI.
const BAD_ENUMS: [i32; 12] = [
    3,
    4,
    5,
    7,
    100,
    -1,
    -2,
    -999,
    i32::MIN,
    i32::MAX,
    i32::MIN + 1,
    i32::MAX - 1,
];

/// `int` values a `c2Simplex::count` can hold that are outside {1,2,3}.
const BAD_COUNTS: [i32; 10] = [0, 4, 5, 6, 100, -1, -2, -100, i32::MIN, i32::MAX];

// ---------------------------------------------------------------------------
// Row 1 — c2MakeProxy with an out-of-range C2_TYPE: the switch has no
// `default:`, so `*p` must be left COMPLETELY unmodified.
// ---------------------------------------------------------------------------

#[test]
fn err01_makeproxy_out_of_range_enum() {
    let p = api();
    let mut rng = Rng::new(0xC01);
    let shape = c2Capsule {
        a: c2v::new(1.0, 2.0),
        b: c2v::new(3.0, 4.0),
        r: 5.0,
    };
    let sp = &shape as *const c2Capsule as *const c_void;
    for &ty in &BAD_ENUMS {
        for _ in 0..64 {
            // A recognisable seed so "left unmodified" is verifiable.
            let mut seed = c2Proxy {
                radius: rng.wild_f32(),
                count: rng.next_u32() as i32,
                verts: [ZV; 8],
            };
            for k in 0..8 {
                seed.verts[k] = rng.wild_vec();
            }
            let mut cp = seed;
            let mut rp = seed;
            unsafe {
                (p.c.c2MakeProxy)(sp, ty, &mut cp);
                (p.r.c2MakeProxy)(sp, ty, &mut rp);
            }
            let ctx = format!("ty={ty}");
            // Same rejection: both must leave the struct byte-identical...
            eq_bits("err01 c2MakeProxy(bad enum)", &ctx, &cp, &rp);
            // ...and specifically identical to the input (the C no-op).
            eq_bits("err01 C left seed untouched", &ctx, &seed, &cp);
            eq_bits("err01 Rust left seed untouched", &ctx, &seed, &rp);
        }
    }
    // Also a NULL shape pointer with a bad enum: the C never dereferences it
    // because no case matches, so this must NOT crash in either library.
    for &ty in &BAD_ENUMS {
        let seed = c2Proxy::default();
        let mut cp = seed;
        let mut rp = seed;
        unsafe {
            (p.c.c2MakeProxy)(std::ptr::null(), ty, &mut cp);
            (p.r.c2MakeProxy)(std::ptr::null(), ty, &mut rp);
        }
        eq_bits("err01 null shape + bad enum", &format!("ty={ty}"), &cp, &rp);
        eq_bits("err01 null shape no-op", &format!("ty={ty}"), &seed, &cp);
    }
}

// ---------------------------------------------------------------------------
// Rows 2-4 — the three VALID enum values must leave `verts` beyond `count`
// untouched (partial initialisation is observable).
// ---------------------------------------------------------------------------

#[test]
fn err02_04_makeproxy_partial_initialisation() {
    let p = api();
    let mut rng = Rng::new(0xC02);
    // (type, how many verts the C writes)
    let cases: [(i32, usize); 3] = [
        (C2_TYPE_CIRCLE, 1),
        (C2_TYPE_AABB, 4),
        (C2_TYPE_CAPSULE, 2),
    ];
    for (ty, nwritten) in cases {
        for _ in 0..256 {
            let mut seed = c2Proxy {
                radius: rng.wild_f32(),
                count: rng.next_u32() as i32,
                verts: [ZV; 8],
            };
            for k in 0..8 {
                seed.verts[k] = c2v::new(9000.0 + k as f32, -9000.0 - k as f32);
            }
            // A shape blob big enough for any of the three types.
            let blob = c2Capsule {
                a: rng.wild_vec(),
                b: rng.wild_vec(),
                r: rng.wild_f32(),
            };
            let sp = &blob as *const c2Capsule as *const c_void;
            let mut cp = seed;
            let mut rp = seed;
            unsafe {
                (p.c.c2MakeProxy)(sp, ty, &mut cp);
                (p.r.c2MakeProxy)(sp, ty, &mut rp);
            }
            let ctx = format!("ty={ty}");
            eq_bits("err02-04 whole proxy", &ctx, &cp, &rp);
            // The tail beyond `nwritten` must still hold the seed pattern.
            for k in nwritten..8 {
                eq_v(
                    &format!("err02-04 C verts[{k}] untouched"),
                    &ctx,
                    seed.verts[k],
                    cp.verts[k],
                );
                eq_v(
                    &format!("err02-04 Rust verts[{k}] untouched"),
                    &ctx,
                    seed.verts[k],
                    rp.verts[k],
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — c2GJKSimplexMetric `default:` arm falls into `case 1:` ⇒ +0.0f
// ---------------------------------------------------------------------------

#[test]
fn err05_metric_out_of_range_count() {
    let p = api();
    let mut rng = Rng::new(0xC05);
    for &count in &BAD_COUNTS {
        for i in 0..128 {
            let mut s = rand_simplex(&mut rng, count, i % 2 == 0);
            s.count = count;
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe {
                (
                    (p.c.c2GJKSimplexMetric)(&mut cs),
                    (p.r.c2GJKSimplexMetric)(&mut rs),
                )
            };
            let ctx = format!("count={count}");
            eq_f32("err05 metric", &ctx, cv, rv);
            // The documented sentinel: exactly +0.0 (bits 0x00000000).
            assert_eq!(
                cv.to_bits(),
                0,
                "err05: C metric for count={count} was {cv:?}, expected +0.0"
            );
            assert_eq!(rv.to_bits(), 0, "err05: Rust metric was {rv:?}");
            eq_bits("err05 simplex untouched", &ctx, &cs, &rs);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 6-8 — c2D
// ---------------------------------------------------------------------------

/// Row 6: `count == 3` or any other value ⇒ `c2V(0, 0)`.
#[test]
fn err06_c2d_default_arm() {
    let p = api();
    let mut rng = Rng::new(0xC06);
    let mut counts = vec![3i32];
    counts.extend_from_slice(&BAD_COUNTS);
    for &count in &counts {
        for i in 0..128 {
            let mut s = rand_simplex(&mut rng, count, i % 2 == 0);
            s.count = count;
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { ((p.c.c2D)(&mut cs), (p.r.c2D)(&mut rs)) };
            let ctx = format!("count={count}");
            eq_v("err06 c2D", &ctx, cv, rv);
            // Sentinel: exactly (+0.0, +0.0).
            assert_eq!(
                (cv.x.to_bits(), cv.y.to_bits()),
                (0, 0),
                "err06: C c2D for count={count} was {cv:?}"
            );
            assert_eq!((rv.x.to_bits(), rv.y.to_bits()), (0, 0));
        }
    }
}

/// Row 7: `count == 1` with each non-finite / signed-zero `a.p` — pure sign flip.
#[test]
fn err07_c2d_count1_non_finite() {
    let p = api();
    let pats: [u32; 12] = [
        0x0000_0000,
        0x8000_0000,
        0x7F80_0000,
        0xFF80_0000,
        0x7FC0_0000,
        0xFFC0_0000,
        0x7F80_0001,
        0xFF80_0001,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
        0x0000_0001,
        0x8000_0001,
    ];
    for &x in &pats {
        for &y in &pats {
            let s = simplex_with(
                1,
                1.0,
                [c2v::new(f32::from_bits(x), f32::from_bits(y)), ZV, ZV],
                [1.0, 0.0, 0.0],
            );
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { ((p.c.c2D)(&mut cs), (p.r.c2D)(&mut rs)) };
            let ctx = format!("a.p=({x:08x},{y:08x})");
            eq_v("err07 c2D count=1", &ctx, cv, rv);
            // Documented C behaviour: a pure sign-bit flip, never a quieting.
            assert_eq!(cv.x.to_bits(), x ^ 0x8000_0000, "err07 C x [{ctx}]");
            assert_eq!(cv.y.to_bits(), y ^ 0x8000_0000, "err07 C y [{ctx}]");
        }
    }
}

/// Row 8: `count == 2` with a NaN determinant ⇒ the `c2CCW90` branch, not `c2Skew`.
#[test]
fn err08_c2d_count2_nan_determinant() {
    let p = api();
    let mut rng = Rng::new(0xC08);
    for i in 0..2000 {
        // Force det2(ab, -a.p) to be NaN by poisoning one component.
        let mut a = rng.vec();
        let mut b = rng.vec();
        match i % 4 {
            0 => a.x = f32::NAN,
            1 => a.y = f32::from_bits(0xFFC0_1234),
            2 => b.x = f32::INFINITY,
            _ => b.y = f32::NEG_INFINITY,
        }
        let s = simplex_with(2, 1.0, [a, b, ZV], [1.0, 1.0, 0.0]);
        let mut cs = s;
        let mut rs = s;
        let (cv, rv) = unsafe { ((p.c.c2D)(&mut cs), (p.r.c2D)(&mut rs)) };
        eq_v("err08 c2D nan-det", &format!("a={a:?} b={b:?}"), cv, rv);
        // When the determinant is genuinely NaN the C must take c2CCW90(ab).
        let ab = unsafe { (p.c.c2Sub)(b, a) };
        let det = unsafe { (p.c.c2Det2)(ab, (p.c.c2Neg)(a)) };
        if det.is_nan() {
            let expect = unsafe { (p.c.c2CCW90)(ab) };
            eq_v("err08 must be CCW90 branch", "", expect, cv);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 9-12 — c2L
// ---------------------------------------------------------------------------

/// Row 9: default arm ⇒ `c2V(0,0)` regardless of `div`.
#[test]
fn err09_c2l_default_arm() {
    let p = api();
    let mut rng = Rng::new(0xC09);
    for &count in &BAD_COUNTS {
        for i in 0..128 {
            let mut s = rand_simplex(&mut rng, count, i % 2 == 0);
            s.count = count;
            // Including the `div` values that make `den` non-finite.
            s.div = [0.0f32, -0.0, f32::NAN, f32::INFINITY, 1.0][i % 5];
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { ((p.c.c2L)(&mut cs), (p.r.c2L)(&mut rs)) };
            let ctx = format!("count={count} div={:?}", s.div);
            eq_v("err09 c2L", &ctx, cv, rv);
            assert_eq!(
                (cv.x.to_bits(), cv.y.to_bits()),
                (0, 0),
                "err09: C c2L was {cv:?} [{ctx}]"
            );
            assert_eq!((rv.x.to_bits(), rv.y.to_bits()), (0, 0));
        }
    }
}

/// Rows 10-12: `div` = +0 / -0 / NaN with `count == 2` ⇒ `1.0f/div` is
/// ±Inf / NaN and propagates; there is no guard in C.
#[test]
fn err10_12_c2l_degenerate_div() {
    let p = api();
    let mut rng = Rng::new(0xC10);
    let divs = [0.0f32, -0.0, f32::NAN, f32::from_bits(0xFFC0_ABCD)];
    for (di, &div) in divs.iter().enumerate() {
        for i in 0..500 {
            let mut s = rand_simplex(&mut rng, 2, i % 3 == 0);
            s.div = div;
            diff_c2L(&format!("err10-12 div_case={di} i={i} s={s:?}"), &s);
            // Also with count 1 (div is computed but unused).
            let mut s1 = s;
            s1.count = 1;
            diff_c2L(&format!("err10-12 count=1 div_case={di} i={i}"), &s1);
        }
    }
    // Confirm the C really does produce a non-finite result for div == 0 with
    // a nonzero p (i.e. that no guard exists), so the row is meaningful.
    let s = simplex_with(2, 0.0, [c2v::new(1.0, 2.0), c2v::new(3.0, 4.0), ZV], [1.0, 1.0, 0.0]);
    let mut cs = s;
    let cv = unsafe { (p.c.c2L)(&mut cs) };
    assert!(
        !cv.x.is_finite() || !cv.y.is_finite(),
        "err10: expected div==0 to produce a non-finite c2L, got {cv:?}"
    );
}

// ---------------------------------------------------------------------------
// Rows 13-14 — c2Witness
// ---------------------------------------------------------------------------

/// Row 13: default arm ⇒ `*a = *b = c2V(0,0)`.
#[test]
fn err13_witness_default_arm() {
    let p = api();
    let mut rng = Rng::new(0xC13);
    for &count in &BAD_COUNTS {
        for i in 0..128 {
            let mut s = rand_simplex(&mut rng, count, i % 2 == 0);
            s.count = count;
            let mut cs = s;
            let mut rs = s;
            let (mut ca, mut cb) = (c2v::new(1.0, 2.0), c2v::new(3.0, 4.0));
            let (mut ra, mut rb) = (c2v::new(1.0, 2.0), c2v::new(3.0, 4.0));
            unsafe {
                (p.c.c2Witness)(&mut cs, &mut ca, &mut cb);
                (p.r.c2Witness)(&mut rs, &mut ra, &mut rb);
            }
            let ctx = format!("count={count}");
            eq_v("err13 *a", &ctx, ca, ra);
            eq_v("err13 *b", &ctx, cb, rb);
            assert_eq!((ca.x.to_bits(), ca.y.to_bits()), (0, 0), "err13 C *a [{ctx}]");
            assert_eq!((cb.x.to_bits(), cb.y.to_bits()), (0, 0), "err13 C *b [{ctx}]");
            assert_eq!((ra.x.to_bits(), ra.y.to_bits()), (0, 0));
            assert_eq!((rb.x.to_bits(), rb.y.to_bits()), (0, 0));
        }
    }
}

/// Row 14: `div == 0` for each valid `count`.
#[test]
fn err14_witness_zero_div() {
    let mut rng = Rng::new(0xC14);
    for count in [1i32, 2, 3] {
        for i in 0..800 {
            let mut s = rand_simplex(&mut rng, count, i % 4 == 0);
            s.div = [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY][i % 5];
            diff_witness(&format!("err14 count={count} i={i} s={s:?}"), &s);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 15-17 — c2Support
// ---------------------------------------------------------------------------

/// Row 15: `count <= 0`. The C reads `verts[0]` unconditionally, so a valid
/// 1-element buffer is supplied; the loop never runs and the result is 0.
#[test]
fn err15_support_non_positive_count() {
    let p = api();
    let mut rng = Rng::new(0xC15);
    for &count in &[0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        for i in 0..256 {
            // One readable vertex, as the C unconditionally dereferences it.
            let verts = [if i % 2 == 0 { rng.vec() } else { rng.wild_vec() }];
            let d = if i % 3 == 0 { rng.wild_vec() } else { rng.vec() };
            let cv = unsafe { (p.c.c2Support)(verts.as_ptr(), count, d) };
            let rv = unsafe { (p.r.c2Support)(verts.as_ptr(), count, d) };
            let ctx = format!("count={count} v={verts:?} d={d:?}");
            eq_i32("err15 c2Support", &ctx, cv, rv);
            assert_eq!(cv, 0, "err15: C returned {cv} for count={count}");
            assert_eq!(rv, 0);
        }
    }
}

/// Row 16: ties and NaN dots — the FIRST index must win, never a later one.
#[test]
fn err16_support_ties_and_nan() {
    let p = api();
    let mut rng = Rng::new(0xC16);
    for count in [1i32, 2, 3, 4, 8] {
        // All vertices identical => every dot ties exactly.
        for i in 0..200 {
            let v0 = if i % 2 == 0 { rng.vec() } else { rng.wild_vec() };
            let verts = vec![v0; count as usize];
            let d = if i % 3 == 0 { rng.wild_vec() } else { rng.vec() };
            let cv = unsafe { (p.c.c2Support)(verts.as_ptr(), count, d) };
            let rv = unsafe { (p.r.c2Support)(verts.as_ptr(), count, d) };
            eq_i32("err16 ties", &format!("count={count} i={i}"), cv, rv);
            assert_eq!(cv, 0, "err16: exact ties must yield index 0, got {cv}");
        }
        // NaN direction => every `dot > dmax` is false.
        for i in 0..200 {
            let mut verts = vec![ZV; count as usize];
            for v in verts.iter_mut() {
                *v = rng.vec();
            }
            let d = [
                c2v::new(f32::NAN, f32::NAN),
                c2v::new(f32::NAN, 1.0),
                c2v::new(1.0, f32::from_bits(0xFFC0_9999)),
            ][i % 3];
            let cv = unsafe { (p.c.c2Support)(verts.as_ptr(), count, d) };
            let rv = unsafe { (p.r.c2Support)(verts.as_ptr(), count, d) };
            eq_i32("err16 nan dir", &format!("count={count} i={i}"), cv, rv);
            assert_eq!(cv, 0, "err16: NaN dots must yield index 0, got {cv}");
        }
    }
}

/// Row 17: `d == (0,0)` ⇒ every dot is `0.0` ⇒ index 0.
#[test]
fn err17_support_zero_direction() {
    let p = api();
    let mut rng = Rng::new(0xC17);
    for count in [1i32, 2, 4, 8] {
        for _ in 0..200 {
            let mut verts = vec![ZV; count as usize];
            for v in verts.iter_mut() {
                *v = rng.vec();
            }
            for d in [ZV, c2v::new(-0.0, 0.0), c2v::new(0.0, -0.0), c2v::new(-0.0, -0.0)] {
                let cv = unsafe { (p.c.c2Support)(verts.as_ptr(), count, d) };
                let rv = unsafe { (p.r.c2Support)(verts.as_ptr(), count, d) };
                eq_i32("err17 zero dir", &format!("count={count} d={d:?}"), cv, rv);
                assert_eq!(cv, 0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 18-22 — division / sqrt degeneracies
// ---------------------------------------------------------------------------

/// Row 18: `c2Norm(0,0)` ⇒ `1/0 = Inf`, `0*Inf` ⇒ NaN.
#[test]
fn err18_norm_zero_vector() {
    let p = api();
    for a in [
        ZV,
        c2v::new(-0.0, 0.0),
        c2v::new(0.0, -0.0),
        c2v::new(-0.0, -0.0),
    ] {
        let cv = unsafe { (p.c.c2Norm)(a) };
        let rv = unsafe { (p.r.c2Norm)(a) };
        let ctx = format!("a={a:?}");
        eq_v("err18 c2Norm(0)", &ctx, cv, rv);
        assert!(
            cv.x.is_nan() && cv.y.is_nan(),
            "err18: C c2Norm({a:?}) = {cv:?}, expected (NaN, NaN)"
        );
    }
}

/// Row 19: `Inf` components ⇒ `len == Inf`, `1/Inf == 0`, then `Inf*0` ⇒ NaN.
#[test]
fn err19_norm_infinite() {
    let p = api();
    let cases = [
        c2v::new(f32::INFINITY, 1.0),
        c2v::new(1.0, f32::INFINITY),
        c2v::new(f32::INFINITY, f32::INFINITY),
        c2v::new(f32::NEG_INFINITY, 0.0),
        c2v::new(f32::NEG_INFINITY, f32::INFINITY),
        c2v::new(FLT_MAX, FLT_MAX), // overflows to Inf inside c2Dot
    ];
    for a in cases {
        let cv = unsafe { (p.c.c2Norm)(a) };
        let rv = unsafe { (p.r.c2Norm)(a) };
        eq_v("err19 c2Norm(Inf)", &format!("a={a:?}"), cv, rv);
    }
}

/// Row 20: NaN components.
#[test]
fn err20_norm_nan() {
    let p = api();
    let nans = [
        f32::NAN,
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFFBF_FFFF),
    ];
    for &n in &nans {
        for a in [c2v::new(n, 1.0), c2v::new(1.0, n), c2v::new(n, n)] {
            let cv = unsafe { (p.c.c2Norm)(a) };
            let rv = unsafe { (p.r.c2Norm)(a) };
            eq_v("err20 c2Norm(NaN)", &format!("a={a:?}"), cv, rv);
            assert!(cv.x.is_nan() && cv.y.is_nan());
        }
    }
}

/// Row 21: `c2Div` by ±0.
#[test]
fn err21_div_by_zero() {
    let p = api();
    let mut rng = Rng::new(0xC21);
    for b in [0.0f32, -0.0] {
        for i in 0..500 {
            let a = if i % 3 == 0 { rng.wild_vec() } else { rng.vec() };
            let cv = unsafe { (p.c.c2Div)(a, b) };
            let rv = unsafe { (p.r.c2Div)(a, b) };
            eq_v("err21 c2Div by 0", &format!("a={a:?} b={b:?}"), cv, rv);
        }
    }
}

/// Row 22: `c2Len` of a NaN-bearing vector.
#[test]
fn err22_len_nan() {
    let p = api();
    let nans = [
        f32::NAN,
        f32::from_bits(0xFFC0_5555),
        f32::from_bits(0x7F80_0003),
        f32::from_bits(0xFF80_0001),
    ];
    for &n in &nans {
        for a in [
            c2v::new(n, 0.0),
            c2v::new(0.0, n),
            c2v::new(n, n),
            c2v::new(n, f32::INFINITY),
        ] {
            let cv = unsafe { (p.c.c2Len)(a) };
            let rv = unsafe { (p.r.c2Len)(a) };
            eq_f32("err22 c2Len(NaN)", &format!("a={a:?}"), cv, rv);
            assert!(cv.is_nan(), "err22: C c2Len({a:?}) = {cv:?}");
        }
    }
    // Also: Inf - Inf inside the dot product produces NaN, then sqrtf(NaN).
    for a in [
        c2v::new(f32::INFINITY, f32::NEG_INFINITY),
        c2v::new(FLT_MAX, -FLT_MAX),
    ] {
        eq_f32(
            "err22 c2Len(Inf mix)",
            &format!("a={a:?}"),
            unsafe { (p.c.c2Len)(a) },
            unsafe { (p.r.c2Len)(a) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 23-28 — c2GJK's six NULL-pointer guards, exercised INDEPENDENTLY
// ---------------------------------------------------------------------------

/// Rows 23-24: `ax_ptr` / `bx_ptr` NULL must be exactly equivalent to passing
/// an explicit `c2xIdentity()` — that is the documented substitution.
#[test]
fn err23_24_gjk_null_transforms() {
    let p = api();
    let ident = unsafe { (p.c.c2xIdentity)() };
    let mut rng = Rng::new(0xC23);
    for (ka, kb) in shape_pairs() {
        for i in 0..100 {
            let ca = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let cb = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let a = rand_shape(&mut rng, ka, ca, 1.0);
            let b = rand_shape(&mut rng, kb, cb, 1.0);
            let ctx = format!("{}/{} i={i}", kind_name(ka), kind_name(kb));
            // All four NULL/non-NULL combinations agree between the libraries.
            for (ax, bx) in [
                (None, None),
                (Some(ident), None),
                (None, Some(ident)),
                (Some(ident), Some(ident)),
            ] {
                diff_gjk(
                    &format!("err23-24 {ctx}"),
                    &a,
                    &b,
                    &GjkOpts {
                        ax,
                        bx,
                        cache: Some(c2GJKCache::default()),
                        ..Default::default()
                    },
                );
            }
            // And NULL is bit-identical to explicit identity within EACH library.
            let onull = GjkOpts {
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            let oident = GjkOpts {
                ax: Some(ident),
                bx: Some(ident),
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            for api_ in [&p.c, &p.r] {
                let n = call_gjk(api_, &a, &b, &onull);
                let e = call_gjk(api_, &a, &b, &oident);
                eq_f32(
                    &format!("err23-24 {} NULL==identity dist", api_.name),
                    &ctx,
                    n.dist,
                    e.dist,
                );
                eq_v(
                    &format!("err23-24 {} NULL==identity outA", api_.name),
                    &ctx,
                    n.a,
                    e.a,
                );
                eq_v(
                    &format!("err23-24 {} NULL==identity outB", api_.name),
                    &ctx,
                    n.b,
                    e.b,
                );
                eq_bits(
                    &format!("err23-24 {} NULL==identity cache", api_.name),
                    &ctx,
                    &n.cache,
                    &e.cache,
                );
            }
        }
    }
}

/// Rows 25-27: `outA` / `outB` / `iterations` NULL — silently discarded, and
/// the return value must be unaffected.
#[test]
fn err25_27_gjk_null_outputs() {
    let p = api();
    let mut rng = Rng::new(0xC25);
    for (ka, kb) in shape_pairs() {
        for i in 0..100 {
            let ca = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let cb = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let a = rand_shape(&mut rng, ka, ca, 1.0);
            let b = rand_shape(&mut rng, kb, cb, 1.0);
            let ctx = format!("{}/{} i={i}", kind_name(ka), kind_name(kb));
            let full = GjkOpts::default();
            let cfull = call_gjk(&p.c, &a, &b, &full);
            let rfull = call_gjk(&p.r, &a, &b, &full);
            eq_f32("err25-27 baseline", &ctx, cfull.dist, rfull.dist);
            for (wa, wb, wi) in [
                (false, true, true),
                (true, false, true),
                (true, true, false),
                (false, false, false),
            ] {
                let o = GjkOpts {
                    want_a: wa,
                    want_b: wb,
                    want_iters: wi,
                    ..Default::default()
                };
                diff_gjk(&format!("err25-27 {ctx} ({wa},{wb},{wi})"), &a, &b, &o);
                // Dropping an out-param must not change the return value.
                for api_ in [&p.c, &p.r] {
                    let base = call_gjk(api_, &a, &b, &full);
                    let some = call_gjk(api_, &a, &b, &o);
                    eq_f32(
                        &format!("err25-27 {} return unaffected", api_.name),
                        &ctx,
                        base.dist,
                        some.dist,
                    );
                }
            }
        }
    }
}

/// Row 28: `cache == NULL` skips BOTH the read and the write blocks.
#[test]
fn err28_gjk_null_cache() {
    let p = api();
    let mut rng = Rng::new(0xC28);
    for (ka, kb) in shape_pairs() {
        for i in 0..150 {
            let ca = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let cb = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let a = rand_shape(&mut rng, ka, ca, 1.0);
            let b = rand_shape(&mut rng, kb, cb, 1.0);
            let ctx = format!("{}/{} i={i}", kind_name(ka), kind_name(kb));
            let no_cache = GjkOpts {
                cache: None,
                ..Default::default()
            };
            diff_gjk(&format!("err28 {ctx}"), &a, &b, &no_cache);
            // A cold (all-zero) cache must give the same answer as no cache at
            // all, since `cache_was_good` is false either way.
            let cold = GjkOpts {
                cache: Some(c2GJKCache::default()),
                ..Default::default()
            };
            for api_ in [&p.c, &p.r] {
                let n = call_gjk(api_, &a, &b, &no_cache);
                let c = call_gjk(api_, &a, &b, &cold);
                eq_f32(
                    &format!("err28 {} NULL cache == cold cache", api_.name),
                    &ctx,
                    n.dist,
                    c.dist,
                );
                eq_v(&format!("err28 {} outA", api_.name), &ctx, n.a, c.a);
                eq_v(&format!("err28 {} outB", api_.name), &ctx, n.b, c.b);
                eq_i32(
                    &format!("err28 {} iters", api_.name),
                    &ctx,
                    n.iters,
                    c.iters,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 29-31 — the cache-validity branches
// ---------------------------------------------------------------------------

/// Row 29: `cache->count == 0` ⇒ `cache_was_good == 0`, read skipped, but the
/// cache is still WRITTEN on exit.
#[test]
fn err29_cache_count_zero() {
    let p = api();
    let mut rng = Rng::new(0xC29);
    for (ka, kb) in shape_pairs() {
        for i in 0..150 {
            let ca = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let cb = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let a = rand_shape(&mut rng, ka, ca, 1.0);
            let b = rand_shape(&mut rng, kb, cb, 1.0);
            // count == 0 but every OTHER field is garbage, to prove the read
            // really is skipped on `!!count == 0`.
            let cache = c2GJKCache {
                metric: rng.wild_f32(),
                count: 0,
                iA: [7, -3, 999],
                iB: [-1, 42, i32::MIN],
                div: rng.wild_f32(),
            };
            let ctx = format!("{}/{} i={i}", kind_name(ka), kind_name(kb));
            let out = diff_gjk(
                &format!("err29 {ctx}"),
                &a,
                &b,
                &GjkOpts {
                    cache: Some(cache),
                    ..Default::default()
                },
            );
            // The cache must have been overwritten (count is now the simplex
            // count, in 1..=3 for any well-formed run).
            assert!(
                (1..=3).contains(&out.cache.count),
                "err29: cache not written back: {:?}",
                out.cache
            );
            let _ = &p;
        }
    }
}

/// Row 30: `cache->count != 0` with the staleness test failing ⇒ the cached
/// simplex is discarded. Since `metric < -1.0e8f` is essentially never true,
/// this is the branch taken almost always — a C quirk to be replicated, and
/// the differential comparison of the full cache is what pins it down.
#[test]
fn err30_cache_staleness_branch() {
    let mut rng = Rng::new(0xC30);
    let vert_count = |k: i32| match k {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        _ => 2,
    };
    for (ka, kb) in shape_pairs() {
        let (na, nb) = (vert_count(ka), vert_count(kb));
        for i in 0..250 {
            let ca = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let cb = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let a = rand_shape(&mut rng, ka, ca, 1.0);
            let b = rand_shape(&mut rng, kb, cb, 1.0);
            // `metric` swept across the -1.0e8 threshold, both sides.
            let metric = [
                0.0f32,
                -1.0e7,
                -9.9e7,
                -1.0e8,
                -1.00001e8,
                -1.0e9,
                -1.0e20,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::NAN,
            ][i % 10];
            let count = 1 + (i % 3) as i32;
            let mut cache = c2GJKCache {
                metric,
                count,
                iA: [0; 3],
                iB: [0; 3],
                div: [1.0f32, 0.0, 2.5, f32::NAN][i % 4],
            };
            for k in 0..3 {
                cache.iA[k] = rng.below(na) as i32;
                cache.iB[k] = rng.below(nb) as i32;
            }
            diff_gjk(
                &format!(
                    "err30 {}/{} i={i} metric={metric:e} count={count}",
                    kind_name(ka),
                    kind_name(kb)
                ),
                &a,
                &b,
                &GjkOpts {
                    cache: Some(cache),
                    ..Default::default()
                },
            );
        }
    }
}

/// Row 31: NEGATIVE `cache->count`. `!!count` is 1, so `cache_was_good` is
/// true, the seeding loop body never runs, `s.count` becomes negative, and the
/// main loop breaks immediately. Documented C result: return `0.0f`,
/// `*outA == *outB == (0,0)`, `*iterations == 0`, `cache->count` negative.
#[test]
fn err31_cache_negative_count() {
    let p = api();
    let mut rng = Rng::new(0xC31);
    for (ka, kb) in shape_pairs() {
        for &bad in &[-1i32, -2, -3, -100, i32::MIN, i32::MIN + 1] {
            for i in 0..40 {
                let ca = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
                let cb = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
                let a = rand_shape(&mut rng, ka, ca, 1.0);
                let b = rand_shape(&mut rng, kb, cb, 1.0);
                let cache = c2GJKCache {
                    metric: rng.uniform(-20.0, 20.0),
                    count: bad,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: rng.uniform(0.5, 4.0),
                };
                let ctx = format!(
                    "err31 {}/{} bad={bad} i={i}",
                    kind_name(ka),
                    kind_name(kb)
                );
                for ur in [0i32, 1] {
                    let o = GjkOpts {
                        cache: Some(cache),
                        use_radius: ur,
                        ..Default::default()
                    };
                    let out = diff_gjk(&ctx, &a, &b, &o);
                    // Pin the documented sentinel from the C's control flow.
                    assert_eq!(out.iters, 0, "err31 iterations [{ctx}]");
                    assert_eq!(out.cache.count, bad, "err31 cache.count [{ctx}]");
                    assert_eq!(
                        (out.a.x.to_bits(), out.a.y.to_bits()),
                        (0, 0),
                        "err31 outA [{ctx}] = {:?}",
                        out.a
                    );
                    assert_eq!(
                        (out.b.x.to_bits(), out.b.y.to_bits()),
                        (0, 0),
                        "err31 outB [{ctx}] = {:?}",
                        out.b
                    );
                    if ur == 1 {
                        assert_eq!(out.dist.to_bits(), 0, "err31 return [{ctx}]");
                    }
                }
                let _ = &p;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 32-36 — the use_radius / hit / midpoint decision tree
// ---------------------------------------------------------------------------

/// Row 32: `use_radius == 0` skips the whole radius block. Every nonzero `int`
/// must behave like 1 (the C tests `else if (use_radius)`).
#[test]
fn err32_use_radius_values() {
    let p = api();
    let mut rng = Rng::new(0xC32);
    let vals = [0i32, 1, 2, -1, 7, i32::MIN, i32::MAX];
    for (ka, kb) in shape_pairs() {
        for i in 0..80 {
            let ca = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let cb = c2v::new(rng.uniform(-5.0, 5.0), rng.uniform(-5.0, 5.0));
            let a = rand_shape(&mut rng, ka, ca, 1.5);
            let b = rand_shape(&mut rng, kb, cb, 1.5);
            let ctx = format!("{}/{} i={i}", kind_name(ka), kind_name(kb));
            for &ur in &vals {
                diff_gjk(
                    &format!("err32 {ctx} ur={ur}"),
                    &a,
                    &b,
                    &GjkOpts {
                        use_radius: ur,
                        ..Default::default()
                    },
                );
            }
            // All nonzero values must form one group, in BOTH libraries.
            for api_ in [&p.c, &p.r] {
                let one = call_gjk(
                    api_,
                    &a,
                    &b,
                    &GjkOpts {
                        use_radius: 1,
                        ..Default::default()
                    },
                );
                for &ur in &vals[1..] {
                    let n = call_gjk(
                        api_,
                        &a,
                        &b,
                        &GjkOpts {
                            use_radius: ur,
                            ..Default::default()
                        },
                    );
                    eq_f32(
                        &format!("err32 {} nonzero-group ur={ur}", api_.name),
                        &ctx,
                        one.dist,
                        n.dist,
                    );
                    eq_v(
                        &format!("err32 {} nonzero-group outA ur={ur}", api_.name),
                        &ctx,
                        one.a,
                        n.a,
                    );
                }
            }
        }
    }
}

/// Row 33: `hit == 1` wins over the radius block (`else if`), so `a == b` and
/// the return is exactly 0 even with large radii.
#[test]
fn err33_hit_beats_radius() {
    let mut rng = Rng::new(0xC33);
    let mut hits = 0usize;
    for (ka, kb) in shape_pairs() {
        for i in 0..200 {
            // Deeply overlapping with large radii.
            let at = c2v::new(rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0));
            let big = |k: i32, c: c2v| -> Shape {
                match k {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: c, r: 8.0 }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB {
                        min: c2v::new(c.x - 4.0, c.y - 4.0),
                        max: c2v::new(c.x + 4.0, c.y + 4.0),
                    }),
                    _ => Shape::Capsule(c2Capsule {
                        a: c2v::new(c.x - 3.0, c.y),
                        b: c2v::new(c.x + 3.0, c.y),
                        r: 6.0,
                    }),
                }
            };
            let a = big(ka, at);
            let b = big(kb, c2v::new(at.x + rng.uniform(-0.5, 0.5), at.y));
            let out = diff_gjk(
                &format!("err33 {}/{} i={i}", kind_name(ka), kind_name(kb)),
                &a,
                &b,
                &GjkOpts::default(),
            );
            if out.dist == 0.0 && out.a == out.b {
                hits += 1;
            }
        }
    }
    assert!(hits > 100, "err33: too few hit-path samples ({hits})");
}

/// Rows 34-36: `dist <= rA+rB` (midpoint), and the `a == b` post-shrink clamp.
#[test]
fn err34_36_radius_decision_boundary() {
    let mut rng = Rng::new(0xC34);
    for (ka, kb) in shape_pairs() {
        for i in 0..300 {
            // Sweep the centre distance across `rA + rB` in fine steps so both
            // sides of `dist > rA + rB` are hit, including equality.
            let ra = rng.uniform(0.0, 2.0);
            let rb = rng.uniform(0.0, 2.0);
            let mk = |k: i32, c: c2v, r: f32| -> Shape {
                match k {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: c, r }),
                    // AABB always has radius 0 in c2MakeProxy.
                    C2_TYPE_AABB => Shape::Aabb(c2AABB { min: c, max: c }),
                    _ => Shape::Capsule(c2Capsule { a: c, b: c, r }),
                }
            };
            let sum = ra + rb;
            let mult = [
                0.0f32, 0.5, 0.999, 0.9999999, 1.0, 1.0000001, 1.001, 1.5, 3.0,
            ][i % 9];
            let d = sum * mult;
            let ca = c2v::new(rng.uniform(-3.0, 3.0), rng.uniform(-3.0, 3.0));
            let cb = c2v::new(ca.x + d, ca.y);
            diff_gjk(
                &format!(
                    "err34-36 {}/{} i={i} ra={ra} rb={rb} m={mult}",
                    kind_name(ka),
                    kind_name(kb)
                ),
                &mk(ka, ca, ra),
                &mk(kb, cb, rb),
                &GjkOpts::default(),
            );
        }
    }
    // Extremely small separations relative to huge radii: after subtracting
    // rA+rB and shifting, `a` and `b` can land on exactly the same float,
    // which forces `dist = 0` (row 36).
    for (ka, kb) in shape_pairs() {
        for i in 0..200 {
            let r = [1.0e6f32, 1.0e12, 1.0e18, 1.0e30][i % 4];
            let mk = |k: i32, c: c2v| -> Shape {
                match k {
                    C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: c, r }),
                    C2_TYPE_AABB => Shape::Aabb(c2AABB { min: c, max: c }),
                    _ => Shape::Capsule(c2Capsule { a: c, b: c, r }),
                }
            };
            let ca = ZV;
            let cb = c2v::new(r * rng.uniform(1.9, 2.1), 0.0);
            diff_gjk(
                &format!("err36 {}/{} i={i} r={r:e}", kind_name(ka), kind_name(kb)),
                &mk(ka, ca),
                &mk(kb, cb),
                &GjkOpts::default(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 37-40 — documented C undefined behaviour: NOT differentially assertable
// ---------------------------------------------------------------------------

#[test]
#[ignore = "ERRORS.md row 37: an out-of-range C2_TYPE makes c2MakeProxy a \
            no-op, so c2GJK reads the UNINITIALISED c2Proxy on its own stack. \
            The C result is not a function of the inputs, so 'C and Rust must \
            agree' is not a testable proposition. The no-op itself IS asserted \
            by err01_makeproxy_out_of_range_enum."]
fn err37_gjk_bad_enum_uninitialised_proxy() {}

#[test]
#[ignore = "ERRORS.md row 38: a NULL shape pointer with an in-range type makes \
            c2MakeProxy dereference NULL — both libraries segfault. Not \
            assertable in-process. The bad-enum + NULL-shape combination, \
            which is well defined because no case matches, IS asserted by \
            err01_makeproxy_out_of_range_enum."]
fn err38_gjk_null_shape_pointer() {}

#[test]
#[ignore = "ERRORS.md row 39: cache->count > 3 makes the C write verts[i] past \
            c2Simplex::d, corrupting div/count and its own stack frame, and \
            read cache->iA[i] past iA[3]. Stack corruption is not a function \
            of the inputs. In-range counts 1..=3 ARE asserted by \
            err30_cache_staleness_branch and row 62 of CONFIGS.md."]
fn err39_cache_count_too_large() {}

#[test]
#[ignore = "ERRORS.md row 40: cache->iA[i] >= pA.count (but < 8) reads a \
            c2Proxy::verts slot that c2MakeProxy never wrote. Uninitialised \
            stack, so not a function of the inputs. In-range indices ARE \
            asserted by row 62 of CONFIGS.md and err30_cache_staleness_branch."]
fn err40_cache_index_past_vertex_count() {}

// ---------------------------------------------------------------------------
// Rows 41-44 — NaN / degenerate branch fall-through in c22 and c23
// ---------------------------------------------------------------------------

/// Row 41: NaN `u`/`v` in `c22` ⇒ both `<= 0` tests false ⇒ the `else` branch.
#[test]
fn err41_c22_nan_falls_to_else() {
    let p = api();
    let mut rng = Rng::new(0xC41);
    for i in 0..3000 {
        let mut a = rng.vec();
        let mut b = rng.vec();
        match i % 4 {
            0 => a.x = f32::NAN,
            1 => a.y = f32::from_bits(0xFFC0_1234),
            2 => b.x = f32::INFINITY,
            _ => b.y = f32::NEG_INFINITY,
        }
        let s = simplex_with(2, 1.0, [a, b, ZV], [0.0; 3]);
        diff_c22(&format!("err41 i={i} a={a:?} b={b:?}"), &s);
        // If u and v really are NaN, the C must land in the count-2 `else`.
        let u = unsafe { (p.c.c2Dot)(b, (p.c.c2Sub)(b, a)) };
        let v = unsafe { (p.c.c2Dot)(a, (p.c.c2Sub)(a, b)) };
        if u.is_nan() && v.is_nan() {
            let mut cs = s;
            unsafe { (p.c.c22)(&mut cs) };
            assert_eq!(cs.count, 2, "err41: NaN u/v must reach the else branch");
        }
    }
}

/// Row 42: both `u <= 0` and `v <= 0` ⇒ the FIRST branch wins (`a` kept, not `b`).
#[test]
fn err42_c22_both_branches_eligible() {
    let p = api();
    // a == b == 0 makes u == v == 0, so both tests are true.
    let mut s = simplex_with(2, 7.0, [ZV, ZV, ZV], [3.0, 4.0, 5.0]);
    s.verts[0].sA = c2v::new(1.0, 2.0);
    s.verts[0].sB = c2v::new(3.0, 4.0);
    s.verts[0].iA = 1;
    s.verts[0].iB = 2;
    s.verts[1].sA = c2v::new(5.0, 6.0);
    s.verts[1].sB = c2v::new(7.0, 8.0);
    s.verts[1].iA = 3;
    s.verts[1].iB = 0;
    diff_c22("err42 a==b==0", &s);
    let mut cs = s;
    unsafe { (p.c.c22)(&mut cs) };
    assert_eq!(cs.count, 1);
    // `a` must be KEPT (the `v <= 0` branch), not replaced by `b`.
    assert_eq!(cs.verts[0].iA, 1, "err42: slot a must be kept");
    assert_eq!(cs.verts[0].iB, 2, "err42: slot a must be kept");
    // Same for other degenerate coincident pairs.
    let mut rng = Rng::new(0xC42);
    for i in 0..1000 {
        let v = if i % 3 == 0 { rng.wild_vec() } else { rng.vec() };
        let mut s = rand_simplex(&mut rng, 2, false);
        s.verts[0].p = v;
        s.verts[1].p = v;
        diff_c22(&format!("err42 coincident i={i}"), &s);
        // And the signed-zero variants.
        let mut s2 = s;
        s2.verts[0].p = c2v::new(-0.0, 0.0);
        s2.verts[1].p = c2v::new(0.0, -0.0);
        diff_c22(&format!("err42 signed-zero i={i}"), &s2);
    }
}

/// Row 43: NaN barycentrics in `c23` ⇒ all seven guards false ⇒ final `else`
/// with NaN `u`s and a NaN `div`.
#[test]
fn err43_c23_nan_falls_to_else() {
    let p = api();
    let mut rng = Rng::new(0xC43);
    let mut reached_else = 0usize;
    for i in 0..3000 {
        let mut ps = [rng.vec(), rng.vec(), rng.vec()];
        ps[i % 3] = match i % 6 {
            0 | 3 => c2v::new(f32::NAN, ps[i % 3].y),
            1 | 4 => c2v::new(ps[i % 3].x, f32::from_bits(0xFFC0_5678)),
            _ => c2v::new(f32::INFINITY, f32::NEG_INFINITY),
        };
        let mut s = rand_simplex(&mut rng, 3, false);
        s.verts[0].p = ps[0];
        s.verts[1].p = ps[1];
        s.verts[2].p = ps[2];
        diff_c23(&format!("err43 i={i} ps={ps:?}"), &s);
        let mut cs = s;
        unsafe { (p.c.c23)(&mut cs) };
        if cs.count == 3 && cs.div.is_nan() {
            reached_else += 1;
        }
    }
    assert!(
        reached_else > 100,
        "err43: the NaN fall-through branch was reached only {reached_else} times"
    );
}

/// Row 44: `area == 0` (collinear) ⇒ all three `*ABC` are 0, so the `*ABC <= 0`
/// guards are satisfiable and branch ORDER decides.
#[test]
fn err44_c23_zero_area() {
    let p = api();
    let mut rng = Rng::new(0xC44);
    for i in 0..4000 {
        let a = rng.vec();
        let b = rng.vec();
        // Exactly collinear with a and b.
        let t = [(-2.0f32), -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0, 3.0][i % 9];
        let c = c2v::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
        let mut s = rand_simplex(&mut rng, 3, false);
        s.verts[0].p = a;
        s.verts[1].p = b;
        s.verts[2].p = c;
        diff_c23(&format!("err44 i={i} t={t} a={a:?} b={b:?} c={c:?}"), &s);
    }
    // Fully coincident triples (area 0 and every u/v 0).
    for i in 0..1000 {
        let v = rng.vec();
        let mut s = rand_simplex(&mut rng, 3, false);
        s.verts[0].p = v;
        s.verts[1].p = v;
        s.verts[2].p = v;
        diff_c23(&format!("err44 coincident i={i}"), &s);
    }
    // All-zero triple: check the C's chosen branch is reproduced exactly.
    let s = simplex_with(3, 9.0, [ZV, ZV, ZV], [1.0, 2.0, 3.0]);
    diff_c23("err44 all-zero", &s);
    let mut cs = s;
    unsafe { (p.c.c23)(&mut cs) };
    assert_eq!(cs.count, 1, "err44: all-zero must take the first branch");
}

// ---------------------------------------------------------------------------
// Rows 45-50 — the `gjk` wrapper's rejection / no-validation surface
// ---------------------------------------------------------------------------

/// Rows 45-46: `reverse == 0` vs nonzero swaps the OPERANDS, so `*a` / `*b`
/// swap meaning. Verified by cross-checking against `c2GJK` directly.
#[test]
fn err45_46_gjk_reverse_operand_order() {
    let p = api();
    let mut rng = Rng::new(0xC45);
    for i in 0..3000 {
        let f: [f32; 9] = [
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
        ];
        for &rv in &[0i8, 1, -1, 0x7f, -128, 2] {
            diff_gjk_wrap(&format!("err45-46 i={i}"), rv, true, true, f);
        }
        // Cross-check against the low-level entry point: `gjk` must be exactly
        // `c2GJK` with the documented argument order.
        let bb = Shape::Aabb(c2AABB {
            min: c2v::new(f[0], f[1]),
            max: c2v::new(f[2], f[3]),
        });
        let cap = Shape::Capsule(c2Capsule {
            a: c2v::new(f[4], f[5]),
            b: c2v::new(f[6], f[7]),
            r: f[8],
        });
        let o = GjkOpts {
            use_radius: 1,
            want_iters: false,
            cache: None,
            ..Default::default()
        };
        for api_ in [&p.c, &p.r] {
            let fwd = call_gjk_wrap(api_, 0, true, true, f);
            let low = call_gjk(api_, &bb, &cap, &o);
            eq_v(
                &format!("err45 {} gjk(0).a == c2GJK(bb,cap).outA", api_.name),
                &format!("i={i}"),
                low.a,
                fwd.a,
            );
            eq_v(
                &format!("err45 {} gjk(0).b == c2GJK(bb,cap).outB", api_.name),
                &format!("i={i}"),
                low.b,
                fwd.b,
            );
            let rev = call_gjk_wrap(api_, 1, true, true, f);
            let lowr = call_gjk(api_, &cap, &bb, &o);
            eq_v(
                &format!("err46 {} gjk(1).a == c2GJK(cap,bb).outA", api_.name),
                &format!("i={i}"),
                lowr.a,
                rev.a,
            );
            eq_v(
                &format!("err46 {} gjk(1).b == c2GJK(cap,bb).outB", api_.name),
                &format!("i={i}"),
                lowr.b,
                rev.b,
            );
        }
    }
}

/// Row 47: `a` and/or `b` NULL must not crash and must not change the other.
#[test]
fn err47_gjk_null_out_pointers() {
    let p = api();
    let mut rng = Rng::new(0xC47);
    for i in 0..3000 {
        let f: [f32; 9] = [
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
        ];
        for &rv in &[0i8, 1] {
            diff_gjk_wrap(&format!("err47 i={i} a=NULL"), rv, false, true, f);
            diff_gjk_wrap(&format!("err47 i={i} b=NULL"), rv, true, false, f);
            diff_gjk_wrap(&format!("err47 i={i} both NULL"), rv, false, false, f);
            // Dropping one out-param must not perturb the other.
            for api_ in [&p.c, &p.r] {
                let both = call_gjk_wrap(api_, rv, true, true, f);
                let only_b = call_gjk_wrap(api_, rv, false, true, f);
                let only_a = call_gjk_wrap(api_, rv, true, false, f);
                eq_v(
                    &format!("err47 {} b unaffected by a=NULL", api_.name),
                    &format!("i={i} rv={rv}"),
                    both.b,
                    only_b.b,
                );
                eq_v(
                    &format!("err47 {} a unaffected by b=NULL", api_.name),
                    &format!("i={i} rv={rv}"),
                    both.a,
                    only_a.a,
                );
            }
        }
    }
}

/// Row 48: inverted AABB — no validation anywhere in C.
#[test]
fn err48_gjk_inverted_box() {
    let mut rng = Rng::new(0xC48);
    for i in 0..5000 {
        let (x0, y0) = (rng.coord(), rng.coord());
        let (x1, y1) = (rng.coord(), rng.coord());
        // Force min > max on both axes.
        let (lo_x, hi_x) = (x0.max(x1), x0.min(x1));
        let (lo_y, hi_y) = (y0.max(y1), y0.min(y1));
        let f = [
            lo_x,
            lo_y,
            hi_x,
            hi_y,
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
        ];
        for &rv in &[0i8, 1] {
            diff_gjk_wrap(&format!("err48 i={i}"), rv, true, true, f);
        }
    }
}

/// Row 49: negative capsule radius — no validation; can *grow* `dist`.
#[test]
fn err49_gjk_negative_radius() {
    let mut rng = Rng::new(0xC49);
    for i in 0..5000 {
        let mut f = [0.0f32; 9];
        for k in 0..8 {
            f[k] = rng.coord();
        }
        f[8] = -rng.uniform(0.0, 20.0);
        for &rv in &[0i8, 1] {
            diff_gjk_wrap(&format!("err49 i={i}"), rv, true, true, f);
        }
        // Very large negative radius.
        f[8] = [-1.0e6f32, -1.0e20, -FLT_MAX, f32::NEG_INFINITY][i % 4];
        for &rv in &[0i8, 1] {
            diff_gjk_wrap(&format!("err49-huge i={i}"), rv, true, true, f);
        }
    }
}

/// Row 50: non-finite floats in any of the 9 argument positions.
#[test]
fn err50_gjk_non_finite_arguments() {
    let mut rng = Rng::new(0xC50);
    let poisons = [
        f32::NAN,
        f32::from_bits(0xFFC0_0001),
        f32::from_bits(0x7F80_0002),
        f32::from_bits(0xFF80_0003),
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for i in 0..300 {
        let mut base = [0.0f32; 9];
        for k in 0..9 {
            base[k] = rng.coord();
        }
        // One poisoned argument at a time — all 9 × 6 combinations.
        for pos in 0..9 {
            for &pv in &poisons {
                let mut f = base;
                f[pos] = pv;
                for &rv in &[0i8, 1] {
                    diff_gjk_wrap(&format!("err50 i={i} pos={pos}"), rv, true, true, f);
                }
            }
        }
        // Two poisoned arguments at a time.
        for p1 in 0..9 {
            for p2 in (p1 + 1)..9 {
                let mut f = base;
                f[p1] = poisons[i % poisons.len()];
                f[p2] = poisons[(i + 3) % poisons.len()];
                diff_gjk_wrap(&format!("err50-2 i={i} {p1}/{p2}"), 0, true, true, f);
                diff_gjk_wrap(&format!("err50-2 i={i} {p1}/{p2}"), 1, true, true, f);
            }
        }
    }
    // All nine non-finite at once.
    for &pv in &poisons {
        for &rv in &[0i8, 1] {
            diff_gjk_wrap("err50-all", rv, true, true, [pv; 9]);
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-cutting NaN-payload provenance tests.
//
// Several arithmetic sites combine two operands that can BOTH be NaN at once
// (`den * u` in `c2Witness`/`c2L`, `u + v` in `c22`). SSE returns the
// *destination* operand's NaN in that case, so which NaN comes out is
// observable and is a distinct behaviour from either operand alone. Randomized
// tests reach these states only by coincidence, so they are pinned explicitly:
// every pair drawn from a set of NaNs with DISTINCT payloads and signs.
// ---------------------------------------------------------------------------

/// NaNs whose payloads and signs are all different, so that "which NaN won"
/// is unambiguous in the output bits.
const DISTINCT_NANS: [u32; 8] = [
    0x7FC0_0000, // default QNaN
    0xFFC0_0000, // negative QNaN
    0x7FC0_1234,
    0xFFC0_5678,
    0x7FFF_FFFF,
    0xFFAB_CDEF,
    0x7F80_0001, // sNaN (must be quieted)
    0xFF80_0002, // negative sNaN
];

/// `c2Witness`: `div` NaN ⇒ `den` NaN, AND `u` NaN — every combination, at
/// every valid `count`. Pins the `mulss` destination for `den * u`.
#[test]
fn nan_prov_witness_den_times_u() {
    for &dn in &DISTINCT_NANS {
        for &un in &DISTINCT_NANS {
            for count in [1i32, 2, 3] {
                let mut s = c2Simplex::default();
                s.count = count;
                s.div = f32::from_bits(dn);
                for k in 0..4 {
                    s.verts[k].u = f32::from_bits(un);
                    // Non-NaN sA/sB so the NaN can only come from den*u.
                    s.verts[k].sA = c2v::new(1.0 + k as f32, 2.0 + k as f32);
                    s.verts[k].sB = c2v::new(3.0 + k as f32, 4.0 + k as f32);
                    s.verts[k].p = c2v::new(5.0 + k as f32, 6.0 + k as f32);
                }
                diff_witness(
                    &format!("nan-prov witness count={count} div={dn:08x} u={un:08x}"),
                    &s,
                );
                diff_c2L(
                    &format!("nan-prov c2L count={count} div={dn:08x} u={un:08x}"),
                    &s,
                );
            }
        }
    }
    // Mixed: only ONE of the three `u`s is NaN, so a per-slot destination
    // mix-up cannot hide behind a uniform NaN.
    for &dn in &DISTINCT_NANS {
        for slot in 0..3 {
            for &un in &DISTINCT_NANS {
                let mut s = c2Simplex::default();
                s.count = 3;
                s.div = f32::from_bits(dn);
                for k in 0..4 {
                    s.verts[k].u = 1.0 + k as f32;
                    s.verts[k].sA = c2v::new(1.0 + k as f32, 2.0 + k as f32);
                    s.verts[k].sB = c2v::new(3.0 + k as f32, 4.0 + k as f32);
                    s.verts[k].p = c2v::new(5.0 + k as f32, 6.0 + k as f32);
                }
                s.verts[slot].u = f32::from_bits(un);
                diff_witness(
                    &format!("nan-prov witness slot={slot} div={dn:08x} u={un:08x}"),
                    &s,
                );
                diff_c2L(&format!("nan-prov c2L slot={slot} div={dn:08x} u={un:08x}"), &s);
            }
        }
    }
    // `div` finite but `u` NaN, and vice versa — the single-NaN baselines.
    for &n in &DISTINCT_NANS {
        for count in [1i32, 2, 3] {
            let mut s = c2Simplex::default();
            s.count = count;
            s.div = 3.0;
            for k in 0..4 {
                s.verts[k].u = f32::from_bits(n);
                s.verts[k].sA = c2v::new(1.0, 2.0);
                s.verts[k].sB = c2v::new(3.0, 4.0);
                s.verts[k].p = c2v::new(5.0, 6.0);
            }
            diff_witness(&format!("nan-prov u-only count={count} u={n:08x}"), &s);
            diff_c2L(&format!("nan-prov u-only c2L count={count} u={n:08x}"), &s);
            let mut s2 = s;
            s2.div = f32::from_bits(n);
            for k in 0..4 {
                s2.verts[k].u = 2.0;
            }
            diff_witness(&format!("nan-prov div-only count={count} div={n:08x}"), &s2);
            diff_c2L(&format!("nan-prov div-only c2L count={count} div={n:08x}"), &s2);
        }
    }
    // And NaN `sA`/`sB`/`p` combined with NaN `den`/`u`, so the NaN survives
    // both the `mulss` and the following `addss`.
    for &n1 in &DISTINCT_NANS {
        for &n2 in &DISTINCT_NANS {
            let mut s = c2Simplex::default();
            s.count = 3;
            s.div = f32::from_bits(n1);
            for k in 0..4 {
                s.verts[k].u = f32::from_bits(n2);
                s.verts[k].sA = c2v::new(f32::from_bits(n2), 1.0);
                s.verts[k].sB = c2v::new(1.0, f32::from_bits(n1));
                s.verts[k].p = c2v::new(f32::from_bits(n1), f32::from_bits(n2));
            }
            diff_witness(&format!("nan-prov all {n1:08x}/{n2:08x}"), &s);
            diff_c2L(&format!("nan-prov all c2L {n1:08x}/{n2:08x}"), &s);
        }
    }
}

/// `c22` / `c23`: `u + v` (and the three-term `uABC + vABC + wABC`) where the
/// addends are simultaneously NaN with distinct payloads. Pins the `addss`
/// destination for each `div` sum.
#[test]
fn nan_prov_simplex_div_sums() {
    let p = api();
    // c22's `else` branch is reachable with NaN u/v (both `<= 0` tests are
    // false when unordered), so `div = u + v` really does see two NaNs.
    for &n1 in &DISTINCT_NANS {
        for &n2 in &DISTINCT_NANS {
            // p values chosen so that u = dot(b, b-a) and v = dot(a, a-b)
            // are the two distinct NaNs.
            let mut s = c2Simplex::default();
            s.count = 2;
            s.div = 1.0;
            s.verts[0].p = c2v::new(f32::from_bits(n1), 1.0);
            s.verts[1].p = c2v::new(f32::from_bits(n2), 2.0);
            for k in 0..4 {
                s.verts[k].sA = c2v::new(10.0 + k as f32, 11.0);
                s.verts[k].sB = c2v::new(12.0, 13.0 + k as f32);
                s.verts[k].u = 0.5 + k as f32;
            }
            diff_c22(&format!("nan-prov c22 div {n1:08x}/{n2:08x}"), &s);
            let mut cs = s;
            unsafe { (p.c.c22)(&mut cs) };
            assert_eq!(cs.count, 2, "nan-prov: expected the c22 else branch");
            assert!(cs.div.is_nan(), "nan-prov: expected a NaN div");

            // Same for c23's three-term sum.
            let mut t = c2Simplex::default();
            t.count = 3;
            t.div = 1.0;
            t.verts[0].p = c2v::new(f32::from_bits(n1), 1.0);
            t.verts[1].p = c2v::new(f32::from_bits(n2), 2.0);
            t.verts[2].p = c2v::new(3.0, f32::from_bits(n1));
            for k in 0..4 {
                t.verts[k].sA = c2v::new(20.0 + k as f32, 21.0);
                t.verts[k].sB = c2v::new(22.0, 23.0 + k as f32);
                t.verts[k].u = 1.5 + k as f32;
            }
            diff_c23(&format!("nan-prov c23 div {n1:08x}/{n2:08x}"), &t);
            // Every permutation of which vertex carries which NaN.
            for perm in [[0usize, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] {
                let mut u = t;
                let nans = [f32::from_bits(n1), f32::from_bits(n2), 7.0];
                for k in 0..3 {
                    u.verts[k].p = c2v::new(nans[perm[k]], (k as f32) + 1.0);
                }
                diff_c23(&format!("nan-prov c23 perm {perm:?} {n1:08x}/{n2:08x}"), &u);
                diff_c22(&format!("nan-prov c22 perm {perm:?} {n1:08x}/{n2:08x}"), &u);
            }
        }
    }
}

/// Leaf-level NaN provenance: every ordered pair of distinct NaNs through every
/// two-operand leaf, so each `addss`/`subss`/`mulss`/`divss` destination choice
/// is pinned independently of the callers.
#[test]
fn nan_prov_leaf_operand_matrix() {
    let p = api();
    for &n1 in &DISTINCT_NANS {
        for &n2 in &DISTINCT_NANS {
            let (f1, f2) = (f32::from_bits(n1), f32::from_bits(n2));
            let ctx = format!("{n1:08x}/{n2:08x}");
            // Both components NaN, and one component NaN at a time.
            let vecs = [
                (c2v::new(f1, f1), c2v::new(f2, f2)),
                (c2v::new(f1, 1.0), c2v::new(f2, 2.0)),
                (c2v::new(1.0, f1), c2v::new(2.0, f2)),
                (c2v::new(f1, f2), c2v::new(f2, f1)),
                (c2v::new(f1, 1.0), c2v::new(1.0, f2)),
            ];
            for (a, b) in vecs {
                eq_v("np c2Add", &ctx, unsafe { (p.c.c2Add)(a, b) }, unsafe {
                    (p.r.c2Add)(a, b)
                });
                eq_v("np c2Sub", &ctx, unsafe { (p.c.c2Sub)(a, b) }, unsafe {
                    (p.r.c2Sub)(a, b)
                });
                eq_f32("np c2Dot", &ctx, unsafe { (p.c.c2Dot)(a, b) }, unsafe {
                    (p.r.c2Dot)(a, b)
                });
                eq_f32("np c2Det2", &ctx, unsafe { (p.c.c2Det2)(a, b) }, unsafe {
                    (p.r.c2Det2)(a, b)
                });
                eq_v("np c2Maxv", &ctx, unsafe { (p.c.c2Maxv)(a, b) }, unsafe {
                    (p.r.c2Maxv)(a, b)
                });
                eq_v("np c2Minv", &ctx, unsafe { (p.c.c2Minv)(a, b) }, unsafe {
                    (p.r.c2Minv)(a, b)
                });
                eq_v(
                    "np c2Clampv",
                    &ctx,
                    unsafe { (p.c.c2Clampv)(a, b, a) },
                    unsafe { (p.r.c2Clampv)(a, b, a) },
                );
                eq_f32("np c2Len", &ctx, unsafe { (p.c.c2Len)(a) }, unsafe {
                    (p.r.c2Len)(a)
                });
                eq_v("np c2Norm", &ctx, unsafe { (p.c.c2Norm)(a) }, unsafe {
                    (p.r.c2Norm)(a)
                });
                // scalar × vector, both NaN
                eq_v("np c2Mulvs", &ctx, unsafe { (p.c.c2Mulvs)(a, f2) }, unsafe {
                    (p.r.c2Mulvs)(a, f2)
                });
                eq_v("np c2Div", &ctx, unsafe { (p.c.c2Div)(a, f2) }, unsafe {
                    (p.r.c2Div)(a, f2)
                });
                // rotation × vector, both NaN
                let r = c2r { c: f1, s: f2 };
                eq_v("np c2Mulrv", &ctx, unsafe { (p.c.c2Mulrv)(r, b) }, unsafe {
                    (p.r.c2Mulrv)(r, b)
                });
                eq_v("np c2MulrvT", &ctx, unsafe { (p.c.c2MulrvT)(r, b) }, unsafe {
                    (p.r.c2MulrvT)(r, b)
                });
                let x = c2x { p: a, r };
                eq_v("np c2Mulxv", &ctx, unsafe { (p.c.c2Mulxv)(x, b) }, unsafe {
                    (p.r.c2Mulxv)(x, b)
                });
                // and the same rotation applied to a NaN-free vector, so the
                // NaN can only originate in the rotation itself
                let clean = c2v::new(1.5, -2.5);
                eq_v(
                    "np c2Mulrv clean-v",
                    &ctx,
                    unsafe { (p.c.c2Mulrv)(r, clean) },
                    unsafe { (p.r.c2Mulrv)(r, clean) },
                );
                eq_v(
                    "np c2MulrvT clean-v",
                    &ctx,
                    unsafe { (p.c.c2MulrvT)(r, clean) },
                    unsafe { (p.r.c2MulrvT)(r, clean) },
                );
            }
        }
    }
}
