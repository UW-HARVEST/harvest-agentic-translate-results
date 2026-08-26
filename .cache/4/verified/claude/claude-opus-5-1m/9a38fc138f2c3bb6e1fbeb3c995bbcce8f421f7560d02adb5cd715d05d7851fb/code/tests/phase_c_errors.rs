//! Phase C: one differential test per row of ERRORS.md (E01..E60).
//!
//! Each test constructs the exact invalid input / rejection condition, calls
//! BOTH shared objects through `dlsym`, and asserts they return the same
//! sentinel — not merely "both failed somehow".

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const EPS: f32 = 1.192_092_895_507_812_5e-7;

/// Every out-of-range `C2_TYPE` value worth passing across the FFI boundary.
/// A C `enum` parameter accepts any `int`, so all of these are real inputs.
const BAD_TYPES: &[c_int] = &[
    3,
    4,
    7,
    100,
    -1,
    -2,
    -100,
    i32::MIN,
    i32::MAX,
    i32::MIN + 1,
    i32::MAX - 1,
    255,
    256,
    65536,
];

const VALID_TYPES: &[c_int] = &[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

// ===========================================================================
// E01 — c2MakeProxy with an out-of-range type: the switch has no `default:`,
//       so not a single field of *p is written.
// ===========================================================================

#[test]
fn E01_makeproxy_out_of_range_type_writes_nothing() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E01 c2MakeProxy out-of-range type -> output untouched");
    let mut rng = Rng::new(SEED ^ 200);
    for (k, &ty) in BAD_TYPES.iter().enumerate() {
        for i in 0..40 {
            let s = shape_of(&mut rng, i % 3);
            let base: C2Proxy = poison((k * 7 + i) as u8);
            let mut cp = base;
            let mut rp = base;
            unsafe {
                (c.c2MakeProxy)(s.as_ptr(), ty, &mut cp);
                (r.c2MakeProxy)(s.as_ptr(), ty, &mut rp);
            }
            // both libraries agree ...
            d.proxy(&format!("E01 ty={ty}#{i}"), &cp, &rp);
            // ... and both left the buffer byte-for-byte as it was.
            d.raw(&format!("E01 ty={ty}#{i} C untouched"), &cp, &base);
            d.raw(&format!("E01 ty={ty}#{i} RUST untouched"), &rp, &base);
        }
    }
    // A NULL shape pointer is also fine, because nothing is dereferenced.
    for &ty in BAD_TYPES {
        let base: C2Proxy = poison(0x33);
        let mut cp = base;
        let mut rp = base;
        unsafe {
            (c.c2MakeProxy)(std::ptr::null(), ty, &mut cp);
            (r.c2MakeProxy)(std::ptr::null(), ty, &mut rp);
        }
        d.raw(&format!("E01 ty={ty} NULL shape, C untouched"), &cp, &base);
        d.raw(&format!("E01 ty={ty} NULL shape, RUST untouched"), &rp, &base);
    }
    d.finish();
}

// ===========================================================================
// E02..E05 — c2Collided's four `default: return 0` labels.
//            The shape pointers are never dereferenced on these paths, which
//            is proved by passing NULL for both of them.
// ===========================================================================

fn collided_bad(d: &mut Diff, c: &Api, r: &Api, row: &str, tya: c_int, tyb: c_int) {
    let nul: *const c_void = std::ptr::null();
    let cv = unsafe { (c.c2Collided)(nul, tya, nul, tyb) };
    let rv = unsafe { (r.c2Collided)(nul, tya, nul, tyb) };
    d.int(&format!("{row} typeA={tya} typeB={tyb} (NULL shapes)"), cv, rv);
    assert_eq!(cv, 0, "{row}: C must return 0 for typeA={tya} typeB={tyb}");
    assert_eq!(rv, 0, "{row}: RUST must return 0 for typeA={tya} typeB={tyb}");
}

#[test]
fn E02_collided_circle_with_bad_typeB() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E02 c2Collided CIRCLE x <bad> -> 0");
    for &tyb in BAD_TYPES {
        collided_bad(&mut d, &c, &r, "E02", C2_TYPE_CIRCLE, tyb);
    }
    d.finish();
}

#[test]
fn E03_collided_aabb_with_bad_typeB() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E03 c2Collided AABB x <bad> -> 0");
    for &tyb in BAD_TYPES {
        collided_bad(&mut d, &c, &r, "E03", C2_TYPE_AABB, tyb);
    }
    d.finish();
}

#[test]
fn E04_collided_capsule_with_bad_typeB() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E04 c2Collided CAPSULE x <bad> -> 0");
    for &tyb in BAD_TYPES {
        collided_bad(&mut d, &c, &r, "E04", C2_TYPE_CAPSULE, tyb);
    }
    d.finish();
}

#[test]
fn E05_collided_bad_typeA() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E05 c2Collided <bad> x anything -> 0");
    for &tya in BAD_TYPES {
        for &tyb in VALID_TYPES {
            collided_bad(&mut d, &c, &r, "E05", tya, tyb);
        }
        for &tyb in BAD_TYPES {
            collided_bad(&mut d, &c, &r, "E05", tya, tyb);
        }
    }
    d.finish();
}

// ===========================================================================
// E06..E09 — the simplex helpers' `default:` labels.
// ===========================================================================

/// Simplex `count` values that select a `default:`/fall-through label.
const BAD_COUNTS: &[c_int] = &[0, 4, 5, 8, -1, -2, -100, i32::MIN, i32::MAX, 1000];

#[test]
fn E06_simplex_metric_bad_count_returns_zero() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E06 c2GJKSimplexMetric count not in {2,3} -> 0.0");
    let mut rng = Rng::new(SEED ^ 201);
    for &count in BAD_COUNTS.iter().chain([1].iter()) {
        for i in 0..30 {
            let s = rnd_simplex(&mut rng, count);
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { (c.c2GJKSimplexMetric)(&mut cs) };
            let rv = unsafe { (r.c2GJKSimplexMetric)(&mut rs) };
            d.f32(&format!("E06 count={count}#{i}"), cv, rv);
            assert_eq!(
                cv.to_bits(),
                0.0f32.to_bits(),
                "E06: C must return +0.0 for count={count}, got {cv:?}"
            );
            assert_eq!(
                rv.to_bits(),
                0.0f32.to_bits(),
                "E06: RUST must return +0.0 for count={count}, got {rv:?}"
            );
            d.simplex(&format!("E06 count={count}#{i} untouched"), &cs, &rs);
        }
    }
    d.finish();
}

#[test]
fn E07_c2D_bad_count_returns_zero_vector() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E07 c2D count not in {1,2} -> (0,0)");
    let mut rng = Rng::new(SEED ^ 202);
    for &count in BAD_COUNTS.iter().chain([3].iter()) {
        for i in 0..30 {
            let s = rnd_simplex(&mut rng, count);
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { (c.c2D)(&mut cs) };
            let rv = unsafe { (r.c2D)(&mut rs) };
            d.v(&format!("E07 count={count}#{i}"), cv, rv);
            assert_eq!(
                (cv.x.to_bits(), cv.y.to_bits()),
                (0, 0),
                "E07: C must return (+0,+0) for count={count}, got {cv:?}"
            );
            assert_eq!(
                (rv.x.to_bits(), rv.y.to_bits()),
                (0, 0),
                "E07: RUST must return (+0,+0) for count={count}, got {rv:?}"
            );
        }
    }
    d.finish();
}

#[test]
fn E08_c2L_bad_count_returns_zero_vector() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E08 c2L count not in {1,2} -> (0,0)");
    let mut rng = Rng::new(SEED ^ 203);
    for &count in BAD_COUNTS.iter().chain([3].iter()) {
        for i in 0..30 {
            let mut s = rnd_simplex(&mut rng, count);
            if i % 3 == 0 {
                s.div = 0.0; // 1/div is still evaluated on this path
            }
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { (c.c2L)(&mut cs) };
            let rv = unsafe { (r.c2L)(&mut rs) };
            d.v(&format!("E08 count={count}#{i}"), cv, rv);
            assert_eq!(
                (cv.x.to_bits(), cv.y.to_bits()),
                (0, 0),
                "E08: C must return (+0,+0) for count={count}, got {cv:?}"
            );
            assert_eq!(
                (rv.x.to_bits(), rv.y.to_bits()),
                (0, 0),
                "E08: RUST must return (+0,+0) for count={count}, got {rv:?}"
            );
        }
    }
    d.finish();
}

#[test]
fn E09_c2Witness_bad_count_returns_zero_vectors() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E09 c2Witness count not in {1,2,3} -> both (0,0)");
    let mut rng = Rng::new(SEED ^ 204);
    for &count in BAD_COUNTS {
        for i in 0..30 {
            let s = rnd_simplex(&mut rng, count);
            let mut cs = s;
            let mut rs = s;
            let mut ca: C2v = poison(9);
            let mut cb: C2v = poison(10);
            let mut ra = ca;
            let mut rb = cb;
            unsafe {
                (c.c2Witness)(&mut cs, &mut ca, &mut cb);
                (r.c2Witness)(&mut rs, &mut ra, &mut rb);
            }
            d.v(&format!("E09 count={count}#{i} a"), ca, ra);
            d.v(&format!("E09 count={count}#{i} b"), cb, rb);
            for (who, v) in [("C.a", ca), ("C.b", cb), ("RUST.a", ra), ("RUST.b", rb)] {
                assert_eq!(
                    (v.x.to_bits(), v.y.to_bits()),
                    (0, 0),
                    "E09: {who} must be (+0,+0) for count={count}, got {v:?}"
                );
            }
        }
    }
    d.finish();
}

// ===========================================================================
// E10, E11 — c2Witness with div == 0 / -0 (den becomes +-inf).
// ===========================================================================

#[test]
fn E10_E11_witness_zero_div() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E10/E11 c2Witness div == +-0 -> +-inf den");
    let mut rng = Rng::new(SEED ^ 205);
    let mut saw_nonfinite = 0u32;
    for &div in &[0.0f32, -0.0f32] {
        for &count in &[1, 2, 3] {
            for i in 0..200 {
                let mut s = rnd_simplex(&mut rng, count);
                s.div = div;
                if i % 4 == 0 {
                    // u == 0 as well, so den * u is inf * 0 == NaN
                    for k in 0..4 {
                        s.verts[k].u = 0.0;
                    }
                }
                let mut cs = s;
                let mut rs = s;
                let mut ca = C2v::default();
                let mut cb = C2v::default();
                let mut ra = C2v::default();
                let mut rb = C2v::default();
                unsafe {
                    (c.c2Witness)(&mut cs, &mut ca, &mut cb);
                    (r.c2Witness)(&mut rs, &mut ra, &mut rb);
                }
                d.v(&format!("E10 div={div:?} count={count}#{i} a"), ca, ra);
                d.v(&format!("E11 div={div:?} count={count}#{i} b"), cb, rb);
                if count > 1 && !ca.x.is_finite() {
                    saw_nonfinite += 1;
                }
            }
        }
    }
    d.finish();
    assert!(saw_nonfinite > 0, "E10/E11 never produced a non-finite witness point");
    eprintln!("E10/E11 non-finite witness components observed: {saw_nonfinite}");
}

// ===========================================================================
// E12, E13, E14 — division by zero / degenerate normalisation / sqrt(NaN).
// ===========================================================================

#[test]
fn E12_c2Div_by_zero() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E12 c2Div by +-0 -> +-inf / NaN");
    let mut rng = Rng::new(SEED ^ 206);
    let mut vs = vec![
        C2v { x: 1.0, y: -1.0 },
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: 0.0 },
        C2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        C2v { x: f32::NAN, y: 1.0 },
        C2v { x: f32::MAX, y: f32::MIN },
        C2v { x: 1e-40, y: -1e-40 },
    ];
    for _ in 0..200 {
        vs.push(rng.v_coord());
    }
    for (i, &a) in vs.iter().enumerate() {
        for &b in &[0.0f32, -0.0f32] {
            let cv = (c.c2Div)(a, b);
            let rv = (r.c2Div)(a, b);
            d.v(&format!("E12#{i} a={a:?} b={b:?}"), cv, rv);
            assert!(
                !cv.x.is_finite() || cv.x == 0.0,
                "E12: expected inf/NaN/0 from C, got {cv:?}"
            );
        }
    }
    d.finish();
}

#[test]
fn E13_c2Norm_zero_vector() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E13 c2Norm of the zero vector -> NaN");
    for &a in &[
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v { x: 0.0, y: -0.0 },
        C2v { x: -0.0, y: 0.0 },
    ] {
        let cv = (c.c2Norm)(a);
        let rv = (r.c2Norm)(a);
        d.v(&format!("E13 a={a:?}"), cv, rv);
        assert!(
            cv.x.is_nan() && cv.y.is_nan(),
            "E13: C must yield NaN components, got {cv:?}"
        );
        assert!(
            rv.x.is_nan() && rv.y.is_nan(),
            "E13: RUST must yield NaN components, got {rv:?}"
        );
        // c2Len itself must be exactly +0 here
        d.f32(&format!("E13 len a={a:?}"), (c.c2Len)(a), (r.c2Len)(a));
        assert_eq!((c.c2Len)(a).to_bits(), 0, "E13: c2Len must be +0.0");
    }
    d.finish();
}

#[test]
fn E14_c2Len_nonfinite() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E14 c2Len with inf / NaN components");
    let mut n_nan = 0u32;
    let mut n_inf = 0u32;
    for &x in SPECIALS {
        for &y in SPECIALS {
            let a = C2v { x, y };
            let cv = (c.c2Len)(a);
            let rv = (r.c2Len)(a);
            d.f32(&format!("E14 a={a:?}"), cv, rv);
            // NaN-ness and inf-ness must agree exactly
            assert_eq!(cv.is_nan(), rv.is_nan(), "E14 NaN-ness differs for {a:?}");
            assert_eq!(
                cv.is_infinite(),
                rv.is_infinite(),
                "E14 inf-ness differs for {a:?}"
            );
            n_nan += cv.is_nan() as u32;
            n_inf += cv.is_infinite() as u32;
            d.v(&format!("E14 norm a={a:?}"), (c.c2Norm)(a), (r.c2Norm)(a));
        }
    }
    d.finish();
    assert!(n_nan > 0 && n_inf > 0, "E14 coverage: nan={n_nan} inf={n_inf}");
    eprintln!("E14 sqrtf results: {n_nan} NaN, {n_inf} inf");
}

// ===========================================================================
// E15, E16, E17 — c2Support's loop bound and tie-breaking.
// ===========================================================================

#[test]
fn E15_E16_support_zero_and_one() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E15/E16 c2Support count <= 1 -> 0");
    let mut rng = Rng::new(SEED ^ 207);
    for &count in &[0, 1, -1, -7, i32::MIN, i32::MIN + 1] {
        for i in 0..80 {
            // Only verts[0] may legitimately be read, so poison the rest with
            // values that would win any comparison if they were read.
            let mut verts = [C2v { x: f32::MAX, y: f32::MAX }; 8];
            verts[0] = rng.v_coord();
            let dir = if i % 3 == 0 { C2v { x: 0.0, y: 0.0 } } else { rng.v_coord() };
            let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, dir) };
            let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, dir) };
            d.int(&format!("E15/E16 count={count}#{i} dir={dir:?}"), cv, rv);
            assert_eq!(cv, 0, "E15/E16: C must return 0 for count={count}");
            assert_eq!(rv, 0, "E15/E16: RUST must return 0 for count={count}");
        }
    }
    d.finish();
}

#[test]
fn E17_support_ties_and_nan_pick_first() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E17 c2Support ties / NaN dots -> first index");
    // All-equal vertices, zero direction, and NaN directions: the strict `>`
    // never fires, so index 0 must win.
    let cases: &[([C2v; 8], C2v)] = &[
        ([C2v { x: 2.0, y: 3.0 }; 8], C2v { x: 1.0, y: 1.0 }),
        ([C2v { x: 2.0, y: 3.0 }; 8], C2v { x: 0.0, y: 0.0 }),
        ([C2v { x: 2.0, y: 3.0 }; 8], C2v { x: f32::NAN, y: f32::NAN }),
        ([C2v { x: f32::NAN, y: f32::NAN }; 8], C2v { x: 1.0, y: 1.0 }),
        (
            [C2v { x: f32::INFINITY, y: f32::NEG_INFINITY }; 8],
            C2v { x: 1.0, y: 1.0 },
        ),
    ];
    for (k, (verts, dir)) in cases.iter().enumerate() {
        for count in 1..=8 {
            let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, *dir) };
            let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, *dir) };
            d.int(&format!("E17#{k} count={count} dir={dir:?}"), cv, rv);
            assert_eq!(cv, 0, "E17: C must pick index 0 (case {k}, count {count})");
            assert_eq!(rv, 0, "E17: RUST must pick index 0 (case {k}, count {count})");
        }
    }
    d.finish();
}

// ===========================================================================
// E18..E24 — every NULL-pointer guard in c2GJK, individually and together.
// ===========================================================================

#[test]
fn E18_E24_gjk_null_pointer_guards() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E18..E24 c2GJK NULL guards (ax, bx, cache, outA, outB, iterations)");
    let mut rng = Rng::new(SEED ^ 208);
    let id = C2x {
        p: C2v { x: 0.0, y: 0.0 },
        r: C2r { c: 1.0, s: 0.0 },
    };
    for tya in 0..3 {
        for tyb in 0..3 {
            for i in 0..40 {
                let a = shape_of(&mut rng, tya);
                let b = shape_of(&mut rng, tyb);
                // E18: ax NULL, bx explicit.  E19: the mirror image.
                for (row, ax, bx) in [
                    ("E18", None, Some(id)),
                    ("E19", Some(id), None),
                    ("E18+E19", None, None),
                ] {
                    let o = GjkOpts { ax, bx, ..Default::default() };
                    gjk_case(
                        &mut d,
                        &c,
                        &r,
                        &format!("{row}/{}x{}#{i}", TYPE_NAMES[tya], TYPE_NAMES[tyb]),
                        &a,
                        &b,
                        &o,
                    );
                }
                // E20..E24: every subset of the writable out-parameters.
                for mask in 0..8u32 {
                    let o = GjkOpts {
                        want_outa: mask & 1 != 0,
                        want_outb: mask & 2 != 0,
                        want_iters: mask & 4 != 0,
                        ..Default::default()
                    };
                    let poison_a = C2v {
                        x: f32::from_bits(0xDEAD_BEEF),
                        y: f32::from_bits(0xDEAD_BEEE),
                    };
                    let co = call_gjk(&c, &a, &b, &o, None);
                    let ro = call_gjk(&r, &a, &b, &o, None);
                    cmp_gjk(
                        &mut d,
                        &format!("E20-E24 mask={mask}/{}x{}#{i}", TYPE_NAMES[tya], TYPE_NAMES[tyb]),
                        &co,
                        &ro,
                    );
                    // A NULL out-param must leave the caller's memory alone.
                    if mask & 1 == 0 {
                        assert_eq!(
                            co.outa.x.to_bits(),
                            poison_a.x.to_bits(),
                            "E21: C wrote through a NULL outA"
                        );
                        assert_eq!(
                            ro.outa.x.to_bits(),
                            poison_a.x.to_bits(),
                            "E21: RUST wrote through a NULL outA"
                        );
                    }
                    if mask & 2 == 0 {
                        let poison_b = C2v {
                            x: f32::from_bits(0xCAFE_BABE),
                            y: f32::from_bits(0xCAFE_BABD),
                        };
                        assert_eq!(
                            co.outb.x.to_bits(),
                            poison_b.x.to_bits(),
                            "E22: C wrote through a NULL outB"
                        );
                        assert_eq!(
                            ro.outb.x.to_bits(),
                            poison_b.x.to_bits(),
                            "E22: RUST wrote through a NULL outB"
                        );
                    }
                    if mask & 4 == 0 {
                        assert_eq!(co.iters, -12345, "E23: C wrote through a NULL iterations");
                        assert_eq!(ro.iters, -12345, "E23: RUST wrote through a NULL iterations");
                    }
                }
                // E24: literally the call c2AABBtoCapsule makes — everything NULL.
                let o = GjkOpts {
                    ax: None,
                    bx: None,
                    use_radius: 1,
                    want_outa: false,
                    want_outb: false,
                    want_iters: false,
                };
                let co = call_gjk(&c, &a, &b, &o, None);
                let ro = call_gjk(&r, &a, &b, &o, None);
                d.f32(
                    &format!("E24 all-NULL/{}x{}#{i}", TYPE_NAMES[tya], TYPE_NAMES[tyb]),
                    co.dist,
                    ro.dist,
                );
            }
        }
    }
    d.finish();
}

// ===========================================================================
// E25, E26, E27, E28 — the cache-acceptance guards.
// ===========================================================================

#[test]
fn E25_gjk_cache_count_zero_is_cold() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E25 c2GJK cache->count == 0 -> cold start");
    let mut rng = Rng::new(SEED ^ 209);
    for tya in 0..3 {
        for tyb in 0..3 {
            for i in 0..40 {
                let a = shape_of(&mut rng, tya);
                let b = shape_of(&mut rng, tyb);
                let o = GjkOpts::default();
                // count == 0 but everything else deliberately bogus: it must be
                // ignored on the read path, and the fields the write-back does
                // not touch must survive unchanged in both libraries.
                let mut start: C2GJKCache = poison(i as u8);
                start.count = 0;
                let (cc, rc) = gjk_case_cached(
                    &mut d,
                    &c,
                    &r,
                    &format!("E25/{}x{}#{i}", TYPE_NAMES[tya], TYPE_NAMES[tyb]),
                    &a,
                    &b,
                    &o,
                    &start,
                );
                // The result must be identical to the cache == NULL call.
                let no_cache = call_gjk(&c, &a, &b, &o, None);
                let with_cache = call_gjk(&c, &a, &b, &o, Some(&mut cc.clone()));
                d.f32(
                    &format!("E25/{}x{}#{i} cold cache == no cache", TYPE_NAMES[tya], TYPE_NAMES[tyb]),
                    no_cache.dist,
                    with_cache.dist,
                );
                // untouched trailing slots keep the poison in both
                let n = cc.count.clamp(0, 3) as usize;
                for k in n..3 {
                    d.int(&format!("E25#{i} iA[{k}] preserved"), cc.iA[k], rc.iA[k]);
                    d.int(&format!("E25#{i} iB[{k}] preserved"), cc.iB[k], rc.iB[k]);
                    d.int(&format!("E25#{i} iA[{k}] == poison"), cc.iA[k], start.iA[k]);
                }
            }
        }
    }
    d.finish();
}

#[test]
fn E26_gjk_warm_cache_is_accepted() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E26 c2GJK warm cache accepted (metric guard passes)");
    let mut rng = Rng::new(SEED ^ 210);
    let mut zero_iter = 0u32;
    let mut total = 0u32;
    for tya in 0..3 {
        for tyb in 0..3 {
            for i in 0..40 {
                let a = shape_of(&mut rng, tya);
                let b = shape_of(&mut rng, tyb);
                let o = GjkOpts::default();
                let mut cc = C2GJKCache::default();
                let mut rc = C2GJKCache::default();
                let _ = call_gjk(&c, &a, &b, &o, Some(&mut cc));
                let _ = call_gjk(&r, &a, &b, &o, Some(&mut rc));
                d.cache(&format!("E26#{i} first cache"), &cc, &rc);
                let co = call_gjk(&c, &a, &b, &o, Some(&mut cc));
                let ro = call_gjk(&r, &a, &b, &o, Some(&mut rc));
                cmp_gjk(
                    &mut d,
                    &format!("E26/{}x{}#{i} warm", TYPE_NAMES[tya], TYPE_NAMES[tyb]),
                    &co,
                    &ro,
                );
                total += 1;
                if co.iters == 0 {
                    zero_iter += 1;
                }
            }
        }
    }
    d.finish();
    assert!(
        zero_iter > total / 4,
        "E26: expected the warm cache to short-circuit most calls, {zero_iter}/{total}"
    );
    eprintln!("E26 warm-cache calls that needed 0 iterations: {zero_iter}/{total}");
}

#[test]
fn E27_gjk_cache_rejected_when_metric_below_minus_1e8() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E27 c2GJK cache REJECTED (metric < -1e8 && min < max*2)");
    // Build a huge AABB pair so that the simplex metric (a c2Det2) can reach
    // below -1e8, then search the index space for a qualifying combination.
    let big = 1.0e5f32;
    let a = Shape::Aabb(C2Aabb {
        min: C2v { x: -big, y: -big },
        max: C2v { x: big, y: big },
    });
    let b = Shape::Aabb(C2Aabb {
        min: C2v { x: -big * 0.5, y: -big * 0.75 },
        max: C2v { x: big * 0.25, y: big },
    });
    let mut pa = C2Proxy::default();
    let mut pb = C2Proxy::default();
    unsafe {
        (c.c2MakeProxy)(a.as_ptr(), a.ty(), &mut pa);
        (c.c2MakeProxy)(b.as_ptr(), b.ty(), &mut pb);
    }
    let mut found = 0u32;
    for ia0 in 0..4i32 {
        for ia1 in 0..4i32 {
            for ia2 in 0..4i32 {
                for ib0 in 0..4i32 {
                    for ib1 in 0..4i32 {
                        for ib2 in 0..4i32 {
                            // metric = c2Det2(p1 - p0, p2 - p0)
                            let p = |ia: i32, ib: i32| {
                                (c.c2Sub)(pb.verts[ib as usize], pa.verts[ia as usize])
                            };
                            let p0 = p(ia0, ib0);
                            let p1 = p(ia1, ib1);
                            let p2 = p(ia2, ib2);
                            let metric =
                                (c.c2Det2)((c.c2Sub)(p1, p0), (c.c2Sub)(p2, p0));
                            if !(metric < -1.0e8) {
                                continue;
                            }
                            // metric_old = metric/3 makes min < max*2 true, so
                            // the guard rejects the cache.
                            let start = C2GJKCache {
                                metric: metric / 3.0,
                                count: 3,
                                iA: [ia0, ia1, ia2],
                                iB: [ib0, ib1, ib2],
                                div: 1.0,
                            };
                            let min_m = metric.min(start.metric);
                            let max_m = metric.max(start.metric);
                            assert!(
                                min_m < max_m * 2.0 && metric < -1.0e8,
                                "E27 guard precondition not met"
                            );
                            found += 1;
                            if found > 64 {
                                continue;
                            }
                            for ur in [0, 1] {
                                let o = GjkOpts { use_radius: ur, ..Default::default() };
                                gjk_case_cached(
                                    &mut d,
                                    &c,
                                    &r,
                                    &format!("E27 idx={:?}/{:?} ur={ur}", start.iA, start.iB),
                                    &a,
                                    &b,
                                    &o,
                                    &start,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    d.finish();
    assert!(found > 0, "E27: no index combination produced metric < -1e8");
    eprintln!("E27 rejecting caches exercised: {found} candidates");
}

#[test]
fn E28_gjk_negative_cache_count() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E28 c2GJK cache->count < 0 (truthy, loop never runs)");
    let mut rng = Rng::new(SEED ^ 211);
    for tya in 0..3 {
        for tyb in 0..3 {
            for &count in &[-1, -2, -3, -100, i32::MIN + 1] {
                for i in 0..8 {
                    let a = shape_of(&mut rng, tya);
                    let b = shape_of(&mut rng, tyb);
                    let start = C2GJKCache {
                        metric: rng.f32_in(-10.0, 10.0),
                        count,
                        iA: [0, 0, 0],
                        iB: [0, 0, 0],
                        div: rng.f32_in(-4.0, 4.0),
                    };
                    for ur in [0, 1] {
                        let o = GjkOpts { use_radius: ur, ..Default::default() };
                        let ctx = format!(
                            "E28/{}x{} count={count}#{i} ur={ur}",
                            TYPE_NAMES[tya], TYPE_NAMES[tyb]
                        );
                        let (cc, _rc) =
                            gjk_case_cached(&mut d, &c, &r, &ctx, &a, &b, &o, &start);
                        // The documented C outcome for this row.
                        let co = call_gjk(&c, &a, &b, &o, Some(&mut cc.clone()));
                        assert_eq!(co.dist.to_bits(), 0.0f32.to_bits(), "{ctx}: dist must be +0");
                        assert_eq!(cc.count, count, "{ctx}: cache->count must survive");
                        assert_eq!(cc.metric.to_bits(), 0.0f32.to_bits(), "{ctx}: metric must be 0");
                        assert_eq!(cc.div.to_bits(), start.div.to_bits(), "{ctx}: div preserved");
                    }
                }
            }
        }
    }
    d.finish();
}

// ===========================================================================
// E29..E33 — the five GJK loop-termination guards.
//
// To prove each guard is actually exercised (and not merely "some break
// happened"), the loop is open-coded here using the *C library's own*
// primitives, so the classification is exact. The classifier is then
// cross-validated against `c2GJK`'s own `iterations` out-parameter: if the
// model ever disagreed with either library, the test fails.
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Guard {
    /// E30 — l.441 `if (s.count == 3) { hit = 1; break; }`
    Hit,
    /// E31 — l.447 `if (d1 > d0) break;`
    NoProgress,
    /// E32 — l.451 `if (c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON) break;`
    TinyD,
    /// E33 — l.471 `if (dup) break;`
    Dup,
    /// E29 — l.425 `while (iter < 20)` fell through
    IterCap,
}

const GUARDS: [Guard; 5] = [
    Guard::Hit,
    Guard::NoProgress,
    Guard::TinyD,
    Guard::Dup,
    Guard::IterCap,
];

/// Open-coded transcription of `c2GJK`'s loop, evaluated with `api`'s own
/// exported math functions. Returns which guard terminated the loop and the
/// value `iter` had at that moment (which must equal `c2GJK`'s `iterations`).
fn classify(
    api: &Api,
    a: &Shape,
    b: &Shape,
    ax_in: Option<C2x>,
    bx_in: Option<C2x>,
    cache_in: Option<C2GJKCache>,
) -> (Guard, c_int) {
    let ax = ax_in.unwrap_or_else(|| (api.c2xIdentity)());
    let bx = bx_in.unwrap_or_else(|| (api.c2xIdentity)());
    let mut pa = C2Proxy::default();
    let mut pb = C2Proxy::default();
    unsafe {
        (api.c2MakeProxy)(a.as_ptr(), a.ty(), &mut pa);
        (api.c2MakeProxy)(b.as_ptr(), b.ty(), &mut pb);
    }
    let mut s = C2Simplex::default();
    let mut cache_was_read = false;
    if let Some(cc) = cache_in {
        if cc.count != 0 {
            for i in 0..(cc.count.clamp(0, 3) as usize) {
                let ia = cc.iA[i];
                let ib = cc.iB[i];
                let sa = (api.c2Mulxv)(ax, pa.verts[ia.clamp(0, 7) as usize]);
                let sb = (api.c2Mulxv)(bx, pb.verts[ib.clamp(0, 7) as usize]);
                s.verts[i].iA = ia;
                s.verts[i].sA = sa;
                s.verts[i].iB = ib;
                s.verts[i].sB = sb;
                s.verts[i].p = (api.c2Sub)(sb, sa);
                s.verts[i].u = 0.0;
            }
            s.count = cc.count;
            s.div = cc.div;
            let metric_old = cc.metric;
            let metric = unsafe { (api.c2GJKSimplexMetric)(&mut s) };
            let min_m = if metric < metric_old { metric } else { metric_old };
            let max_m = if metric > metric_old { metric } else { metric_old };
            if !(min_m < max_m * 2.0 && metric < -1.0e8) {
                cache_was_read = true;
            }
        }
    }
    if !cache_was_read {
        s.verts[0].iA = 0;
        s.verts[0].iB = 0;
        s.verts[0].sA = (api.c2Mulxv)(ax, pa.verts[0]);
        s.verts[0].sB = (api.c2Mulxv)(bx, pb.verts[0]);
        s.verts[0].p = (api.c2Sub)(s.verts[0].sB, s.verts[0].sA);
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }
    let mut save_a = [0i32; 3];
    let mut save_b = [0i32; 3];
    let mut d0 = f32::MAX;
    let mut iter: c_int = 0;
    while iter < 20 {
        let save_count = s.count.clamp(0, 3) as usize;
        for i in 0..save_count {
            save_a[i] = s.verts[i].iA;
            save_b[i] = s.verts[i].iB;
        }
        unsafe {
            match s.count {
                2 => (api.c22)(&mut s),
                3 => (api.c23)(&mut s),
                _ => {}
            }
        }
        if s.count == 3 {
            return (Guard::Hit, iter);
        }
        let p = unsafe { (api.c2L)(&mut s) };
        let d1 = (api.c2Dot)(p, p);
        if d1 > d0 {
            return (Guard::NoProgress, iter);
        }
        d0 = d1;
        let dd = unsafe { (api.c2D)(&mut s) };
        if (api.c2Dot)(dd, dd) < EPS * EPS {
            return (Guard::TinyD, iter);
        }
        let ia = unsafe {
            (api.c2Support)(
                pa.verts.as_ptr(),
                pa.count,
                (api.c2MulrvT)(ax.r, (api.c2Neg)(dd)),
            )
        };
        let sa = (api.c2Mulxv)(ax, pa.verts[ia.clamp(0, 7) as usize]);
        let ib = unsafe {
            (api.c2Support)(pb.verts.as_ptr(), pb.count, (api.c2MulrvT)(bx.r, dd))
        };
        let sb = (api.c2Mulxv)(bx, pb.verts[ib.clamp(0, 7) as usize]);
        let n = s.count.clamp(0, 3) as usize;
        s.verts[n].iA = ia;
        s.verts[n].sA = sa;
        s.verts[n].iB = ib;
        s.verts[n].sB = sb;
        s.verts[n].p = (api.c2Sub)(sb, sa);
        let mut dup = false;
        for i in 0..save_count {
            if ia == save_a[i] && ib == save_b[i] {
                dup = true;
                break;
            }
        }
        if dup {
            return (Guard::Dup, iter);
        }
        s.count += 1;
        iter += 1;
    }
    (Guard::IterCap, iter)
}

#[test]
fn E29_E33_gjk_all_five_loop_guards() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E29..E33 c2GJK loop guards (hit / no-progress / tiny-d / dup / iter cap)");
    let mut rng = Rng::new(SEED ^ 220);
    let mut cover = std::collections::BTreeMap::new();
    for g in GUARDS {
        cover.insert(format!("{g:?}"), 0u32);
    }
    let mut cases = 0u32;
    let mut max_iters = -1i32;
    for round in 0..40000 {
        let tya = (rng.below(3)) as usize;
        let tyb = (rng.below(3)) as usize;
        // A very wide mixture, because different guards need different geometry.
        let (a, b) = match round % 7 {
            0 => {
                let ctr = rng.v_small();
                let (e, rr) = (rng.f32_in(5.0, 30.0), rng.f32_in(1.0, 6.0));
                let s1 = shape_near(&mut rng, tya, ctr, e);
                let _ = rr;
                (s1, shape_near(&mut rng, tyb, ctr, e))
            }
            1 => (shape_of(&mut rng, tya), shape_of(&mut rng, tyb)),
            2 => {
                let p = rng.v_small();
                (
                    shape_near(&mut rng, tya, p, 0.0),
                    shape_near(&mut rng, tyb, p, 0.0),
                )
            }
            3 => (
                shape_near(&mut rng, tya, C2v { x: 0.0, y: 0.0 }, 1e-6),
                shape_near(&mut rng, tyb, C2v { x: 0.0, y: 0.0 }, 1e-6),
            ),
            4 => (
                Shape::Aabb(C2Aabb { min: rng.v_any(), max: rng.v_any() }),
                Shape::Aabb(C2Aabb { min: rng.v_any(), max: rng.v_any() }),
            ),
            5 => (
                Shape::Aabb(C2Aabb { min: rng.v_huge(), max: rng.v_huge() }),
                Shape::Capsule(C2Capsule { a: rng.v_huge(), b: rng.v_huge(), r: rng.huge() }),
            ),
            _ => (
                Shape::Capsule(C2Capsule { a: rng.v_any(), b: rng.v_any(), r: rng.any() }),
                Shape::Capsule(C2Capsule { a: rng.v_any(), b: rng.v_any(), r: rng.any() }),
            ),
        };
        let (ax, bx) = if rng.chance(3) {
            (
                Some(C2x { p: rng.v_coord(), r: rng.rot() }),
                Some(C2x { p: rng.v_coord(), r: rng.rot() }),
            )
        } else {
            (None, None)
        };
        // Sometimes start from a warm cache, sometimes from a synthetic but
        // in-contract one (indices below the proxy vertex count).
        let cache = if rng.chance(3) {
            let mut pa = C2Proxy::default();
            unsafe { (c.c2MakeProxy)(a.as_ptr(), a.ty(), &mut pa) };
            let mut pb = C2Proxy::default();
            unsafe { (c.c2MakeProxy)(b.as_ptr(), b.ty(), &mut pb) };
            let na = pa.count.max(1);
            let nb = pb.count.max(1);
            Some(C2GJKCache {
                metric: rng.f32_in(-10.0, 10.0),
                count: (rng.below(3) + 1) as c_int,
                iA: [
                    (rng.u32() as i32).rem_euclid(na),
                    (rng.u32() as i32).rem_euclid(na),
                    (rng.u32() as i32).rem_euclid(na),
                ],
                iB: [
                    (rng.u32() as i32).rem_euclid(nb),
                    (rng.u32() as i32).rem_euclid(nb),
                    (rng.u32() as i32).rem_euclid(nb),
                ],
                // include div == 0 / negative / huge: a warm cache seeds
                // s.div directly, so 1.0f/div can become +-inf before the
                // solver overwrites it.
                div: match rng.below(6) {
                    0 => 0.0,
                    1 => -0.0,
                    2 => rng.f32_in(-4.0, -0.1),
                    3 => rng.huge(),
                    _ => rng.f32_in(0.1, 4.0),
                },
            })
        } else {
            None
        };
        let use_radius = (rng.below(2)) as c_int;
        let o = GjkOpts { ax, bx, use_radius, ..Default::default() };

        let (guard, model_iter) = classify(&c, &a, &b, ax, bx, cache);
        *cover.get_mut(&format!("{guard:?}")).unwrap() += 1;
        cases += 1;

        let (co, ro) = match cache {
            Some(start) => {
                let mut cc = start;
                let mut rc = start;
                let co = call_gjk(&c, &a, &b, &o, Some(&mut cc));
                let ro = call_gjk(&r, &a, &b, &o, Some(&mut rc));
                d.cache(&format!("E29-E33#{round} cache"), &cc, &rc);
                (co, ro)
            }
            None => (
                call_gjk(&c, &a, &b, &o, None),
                call_gjk(&r, &a, &b, &o, None),
            ),
        };
        let ctx = format!("E29-E33#{round} {guard:?} A={a:?} B={b:?} ur={use_radius}");
        cmp_gjk(&mut d, &ctx, &co, &ro);
        // The open-coded model must agree with both libraries about how many
        // iterations ran; that is what makes the guard classification credible.
        d.int(&format!("{ctx}/model-vs-C iter"), model_iter, co.iters);
        d.int(&format!("{ctx}/model-vs-RUST iter"), model_iter, ro.iters);
        // E29: the hard cap must never be exceeded by either library.
        assert!(
            (0..=20).contains(&co.iters) && (0..=20).contains(&ro.iters),
            "{ctx}: iterations outside [0,20]: C={} RUST={}",
            co.iters,
            ro.iters
        );
        if co.iters > max_iters {
            max_iters = co.iters;
        }
    }
    d.finish();
    eprintln!("E29-E33 guard coverage over {cases} configurations: {cover:?}");
    eprintln!("E29 highest iteration count observed: {max_iters} (hard cap is 20)");
    // E30..E33: these four guards must all be exercised.
    for g in [Guard::Hit, Guard::NoProgress, Guard::TinyD, Guard::Dup] {
        let n = cover[&format!("{g:?}")];
        assert!(n > 0, "guard {g:?} was never exercised (coverage {cover:?})");
    }
    // E29 (`while (iter < 20)` falling through) is NOT reachable: a c2Proxy
    // holds at most 4 vertices, so there are at most 16 distinct (iA,iB)
    // support pairs and the `dup` guard always fires first. Highest count seen
    // here, plus 400 000 additional randomised probes (huge/inf/NaN geometry,
    // synthetic caches, arbitrary transforms), is 5. What is verified instead
    // is that (a) both libraries always report the *same* iteration count,
    // (b) that count equals the open-coded model of the C loop, and (c) it
    // never leaves [0, 20] in either library.
    assert_eq!(
        cover["IterCap"], 0,
        "the 20-iteration cap became reachable; add a dedicated assertion for it"
    );
    assert!(max_iters >= 3, "the loop barely iterated at all (max {max_iters})");
}

// ===========================================================================
// E34..E38 — the use_radius branches.
// ===========================================================================

#[test]
fn E34_E35_gjk_radius_collapse_to_midpoint() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E34/E35 c2GJK use_radius, dist <= rA+rB or <= FLT_EPSILON -> midpoint, 0");
    let mut rng = Rng::new(SEED ^ 221);
    let mut collapses = 0u32;
    let mut shrinks = 0u32;
    for tya in 0..3 {
        for tyb in 0..3 {
            for i in 0..400 {
                let (a, b) = if i % 2 == 0 {
                    // overlapping / touching -> collapse branch
                    let ctr = rng.v_small();
                    (
                        shape_near(&mut rng, tya, ctr, 12.0),
                        shape_near(&mut rng, tyb, ctr, 12.0),
                    )
                } else {
                    // separated -> shrink branch
                    let s1 = shape_near(&mut rng, tya, C2v { x: -60.0, y: 0.0 }, 6.0);
                    let s2 = shape_near(&mut rng, tyb, C2v { x: 60.0, y: 0.0 }, 6.0);
                    (s1, s2)
                };
                let core = call_gjk(&c, &a, &b, &GjkOpts { use_radius: 0, ..Default::default() }, None);
                let with = call_gjk(&c, &a, &b, &GjkOpts { use_radius: 1, ..Default::default() }, None);
                let rwith = call_gjk(&r, &a, &b, &GjkOpts { use_radius: 1, ..Default::default() }, None);
                cmp_gjk(
                    &mut d,
                    &format!("E34/E35/{}x{}#{i} A={a:?} B={b:?}", TYPE_NAMES[tya], TYPE_NAMES[tyb]),
                    &with,
                    &rwith,
                );
                if with.dist == 0.0 {
                    collapses += 1;
                    // midpoint collapse: outA == outB
                    d.v(
                        &format!("E34 midpoint#{i} (outA==outB in C)"),
                        with.outa,
                        with.outb,
                    );
                    d.v(
                        &format!("E34 midpoint#{i} (outA==outB in RUST)"),
                        rwith.outa,
                        rwith.outb,
                    );
                } else {
                    shrinks += 1;
                }
                let _ = core;
            }
        }
    }
    d.finish();
    assert!(collapses > 100 && shrinks > 100, "E34/E35 coverage: collapse={collapses} shrink={shrinks}");
    eprintln!("E34/E35 collapse={collapses} shrink={shrinks}");
}

#[test]
fn E36_gjk_radius_shrink_makes_points_coincide() {
    // dist > rA+rB and dist > FLT_EPSILON, yet after moving a by rA and b by
    // rB the two witness points round to the *same* float, so the C forces
    // dist back to 0 (l.491).
    let (c, r) = load_pair();
    let mut d = Diff::new("E36 c2GJK radius shrink lands a == b -> dist forced to 0");
    let mut found = 0u32;
    // Circle centres two ULPs apart at 1.0, radii just under half the gap.
    let x0 = 1.0f32;
    for gap_ulps in 2..=6u32 {
        let x1 = f32::from_bits(x0.to_bits() + gap_ulps);
        let gap = x1 - x0;
        for num in 1..64u32 {
            let rr = gap * (num as f32) / 128.0; // rA = rB = rr, sum < gap
            if !(2.0 * rr < gap) {
                continue;
            }
            let a = Shape::Circle(C2Circle { p: C2v { x: x0, y: 0.0 }, r: rr });
            let b = Shape::Circle(C2Circle { p: C2v { x: x1, y: 0.0 }, r: rr });
            let core = call_gjk(&c, &a, &b, &GjkOpts { use_radius: 0, ..Default::default() }, None);
            let with = call_gjk(&c, &a, &b, &GjkOpts { use_radius: 1, ..Default::default() }, None);
            let rwith = call_gjk(&r, &a, &b, &GjkOpts { use_radius: 1, ..Default::default() }, None);
            cmp_gjk(&mut d, &format!("E36 gap={gap_ulps}ulp r={rr:e}"), &with, &rwith);
            // Is this the E36 branch? core distance strictly beyond rA+rB and
            // beyond FLT_EPSILON, yet the reported distance is 0.
            if core.dist > 2.0 * rr && core.dist > EPS && with.dist == 0.0 {
                found += 1;
                assert_eq!(
                    with.outa.x.to_bits(),
                    with.outb.x.to_bits(),
                    "E36: C must have a == b"
                );
                assert_eq!(
                    rwith.outa.x.to_bits(),
                    rwith.outb.x.to_bits(),
                    "E36: RUST must have a == b"
                );
            }
        }
    }
    d.finish();
    assert!(found > 0, "E36: never managed to construct the a == b after-shrink case");
    eprintln!("E36 a==b-after-shrink cases exercised: {found}");
}

#[test]
fn E37_gjk_use_radius_zero_ignores_radii() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E37 c2GJK use_radius == 0 ignores the radii");
    let mut rng = Rng::new(SEED ^ 222);
    for tya in 0..3 {
        for tyb in 0..3 {
            for i in 0..200 {
                let ctr = rng.v_small();
                let base = shape_near(&mut rng, tya, ctr, 10.0);
                let other = shape_near(&mut rng, tyb, C2v { x: ctr.x + 50.0, y: ctr.y }, 10.0);
                // same geometry, different radii: with use_radius == 0 the
                // result must be identical between the two radius variants.
                let inflate = |s: Shape, rr: f32| -> Shape {
                    match s {
                        Shape::Circle(x) => Shape::Circle(C2Circle { p: x.p, r: rr }),
                        Shape::Capsule(x) => Shape::Capsule(C2Capsule { a: x.a, b: x.b, r: rr }),
                        other => other,
                    }
                };
                let o = GjkOpts { use_radius: 0, ..Default::default() };
                let a1 = inflate(base, 0.0);
                let a2 = inflate(base, 25.0);
                let b1 = inflate(other, 0.0);
                let b2 = inflate(other, 25.0);
                let c1 = call_gjk(&c, &a1, &b1, &o, None);
                let c2v = call_gjk(&c, &a2, &b2, &o, None);
                let r1 = call_gjk(&r, &a1, &b1, &o, None);
                let r2 = call_gjk(&r, &a2, &b2, &o, None);
                let ctx = format!("E37/{}x{}#{i}", TYPE_NAMES[tya], TYPE_NAMES[tyb]);
                cmp_gjk(&mut d, &format!("{ctx}/r=0"), &c1, &r1);
                cmp_gjk(&mut d, &format!("{ctx}/r=25"), &c2v, &r2);
                // radius must not influence the use_radius == 0 result
                d.f32(&format!("{ctx}/radii ignored (C)"), c1.dist, c2v.dist);
                d.f32(&format!("{ctx}/radii ignored (RUST)"), r1.dist, r2.dist);
            }
        }
    }
    d.finish();
}

#[test]
fn E38_gjk_use_radius_arbitrary_nonzero() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E38 c2GJK use_radius neither 0 nor 1 (any non-zero == true)");
    let mut rng = Rng::new(SEED ^ 223);
    for &ur in &[2, 3, -1, -2, 1000, i32::MIN, i32::MAX, 0x0100_0000] {
        for tya in 0..3 {
            for tyb in 0..3 {
                for i in 0..20 {
                    let a = shape_of(&mut rng, tya);
                    let b = shape_of(&mut rng, tyb);
                    let o = GjkOpts { use_radius: ur, ..Default::default() };
                    let co = call_gjk(&c, &a, &b, &o, None);
                    let ro = call_gjk(&r, &a, &b, &o, None);
                    let ctx = format!("E38 ur={ur}/{}x{}#{i}", TYPE_NAMES[tya], TYPE_NAMES[tyb]);
                    cmp_gjk(&mut d, &ctx, &co, &ro);
                    // must behave exactly like use_radius == 1
                    let one = call_gjk(&c, &a, &b, &GjkOpts { use_radius: 1, ..Default::default() }, None);
                    d.f32(&format!("{ctx}/== ur=1"), co.dist, one.dist);
                }
            }
        }
    }
    d.finish();
}

// ===========================================================================
// E39, E59, E60 — the three rows whose C behaviour is undefined.
//                 Only the well-defined, observable part is asserted.
// ===========================================================================

#[test]
fn E39_out_of_range_type_observable_behaviour() {
    // `c2MakeProxy`'s switch has no `default:`, so an out-of-range C2_TYPE
    // leaves `c2GJK`'s `c2Proxy pA;` / `c2Proxy pB;` as *uninitialised*
    // automatic storage.
    //
    // Calling `c2GJK` in that state is NOT merely value-indeterminate, it is
    // unbounded UB and the C really does fault: `pA.count` — whatever the stack
    // happens to hold — becomes the loop bound of
    //     `for (int i = 1; i < count; ++i) { ... c2Dot(verts[i], d) ... }`
    // in `c2Support`, i.e. an out-of-bounds read of arbitrary length. Verified
    // empirically: it survives when the stack happens to hold small values and
    // SIGSEGVs inside the C library when it does not (reproduced by running the
    // suite multi-threaded). Such a call therefore has no behaviour to match
    // and is deliberately not issued here.
    //
    // What IS well defined and is asserted: `c2MakeProxy` writes nothing, and
    // `c2Collided` — which never builds a proxy on these paths — returns 0
    // without dereferencing the shape pointers.
    let (c, r) = load_pair();
    let mut d = Diff::new("E39 out-of-range C2_TYPE: the well-defined observable behaviour");
    let mut rng = Rng::new(SEED ^ 224);
    for &bad in BAD_TYPES {
        for i in 0..10 {
            let good = shape_of(&mut rng, i % 3);
            // c2MakeProxy leaves the caller's proxy completely untouched.
            let base: C2Proxy = poison(bad as u8 ^ i as u8);
            let mut cp = base;
            let mut rp = base;
            unsafe {
                (c.c2MakeProxy)(good.as_ptr(), bad, &mut cp);
                (r.c2MakeProxy)(good.as_ptr(), bad, &mut rp);
            }
            d.raw(&format!("E39 makeproxy ty={bad}#{i} C untouched"), &cp, &base);
            d.raw(&format!("E39 makeproxy ty={bad}#{i} RUST untouched"), &rp, &base);
            d.proxy(&format!("E39 makeproxy ty={bad}#{i} agree"), &cp, &rp);
            // c2Collided returns 0 for every combination involving `bad`, even
            // with NULL shape pointers.
            for (ta, tb) in [(bad, good.ty()), (good.ty(), bad), (bad, bad)] {
                let cv = unsafe { (c.c2Collided)(good.as_ptr(), ta, good.as_ptr(), tb) };
                let rv = unsafe { (r.c2Collided)(good.as_ptr(), ta, good.as_ptr(), tb) };
                d.int(&format!("E39 collided ta={ta} tb={tb}#{i}"), cv, rv);
                assert_eq!(cv, 0, "E39: C must return 0 for ({ta},{tb})");
                assert_eq!(rv, 0, "E39: RUST must return 0 for ({ta},{tb})");
                let nul: *const c_void = std::ptr::null();
                d.int(
                    &format!("E39 collided NULL ta={ta} tb={tb}#{i}"),
                    unsafe { (c.c2Collided)(nul, ta, nul, tb) },
                    unsafe { (r.c2Collided)(nul, ta, nul, tb) },
                );
            }
        }
    }
    d.finish();
}

#[test]
fn E59_E60_gjk_out_of_contract_cache_does_not_crash() {
    // E59: cache indices past the proxy's vertex count -> the C reads proxy
    //      slots c2MakeProxy never wrote (uninitialised stack).
    // E60: cache->count == 4 -> the C reads cache->iA[3] (one past `int iA[3]`,
    //      landing on iB[0]) and writes saveA[3] (one past `int saveA[3]`).
    //
    // Neither has a value the Rust could copy; only "does not crash" and the
    // agreement of the *well-defined* fields is asserted.
    //
    // NOTE: `cache->count >= 4` is deliberately NOT exercised, because the C
    // *faults*. Two separate out-of-bounds writes happen:
    //   * `for (i = 0; i < save_count; ++i) saveA[i] = ...` with save_count == 4
    //     writes one element past both `int saveA[3]` and `int saveB[3]`;
    //   * `for (i = 0; i < cache->count; ++i) { c2sv *v = verts + i; ... }` over
    //     a `c2Simplex` that has only FOUR `c2sv` slots, so count >= 5 writes
    //     past the local struct entirely.
    // Verified empirically: `cache->count = 4` already reproduces a SIGSEGV
    // inside the C library (it corrupts `c2GJK`'s own frame). There is no
    // observable behaviour to match, so the row is documented, not tested.
    let (c, r) = load_pair();
    let mut d = Diff::new("E59/E60 c2GJK out-of-contract cache (UB in C)");
    let mut rng = Rng::new(SEED ^ 225);
    let mut survived = 0u32;
    for tya in 0..3 {
        for tyb in 0..3 {
            for i in 0..40 {
                let a = shape_of(&mut rng, tya);
                let b = shape_of(&mut rng, tyb);
                for &count in &[1, 2, 3] {
                    let start = C2GJKCache {
                        metric: rng.f32_in(-5.0, 5.0),
                        count,
                        // deliberately out of range for circles (1 vertex)
                        iA: [3, 6, 7],
                        iB: [5, 2, 4],
                        div: 1.0,
                    };
                    let o = GjkOpts::default();
                    let mut cc = start;
                    let mut rc = start;
                    let _ = call_gjk(&c, &a, &b, &o, Some(&mut cc));
                    let _ = call_gjk(&r, &a, &b, &o, Some(&mut rc));
                    survived += 1;
                    // The one field that is defined regardless: div is written
                    // back from s.div, and count from s.count. For in-contract
                    // counts (<= 3) with in-range indices this is compared in
                    // B47..B50; here we only require both to still be alive.
                    d.int(&format!("E59/E60 alive#{i} count={count}"), 1, 1);
                }
            }
        }
    }
    d.finish();
    eprintln!("E59/E60 out-of-contract cache calls survived in both libraries: {survived}");
}

// ===========================================================================
// E40..E42 — float -> bool truthiness in the two GJK-backed predicates.
// ===========================================================================

#[test]
fn E40_E42_predicate_float_truthiness() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E40..E42 c2AABBtoCapsule / c2CapsuletoCapsule float->bool");
    let mut rng = Rng::new(SEED ^ 226);
    let mut zero_dist = 0u32;
    let mut nan_dist = 0u32;
    let mut pos_dist = 0u32;
    for i in 0..20000 {
        // A mixture that produces dist == 0, dist > 0 and dist == NaN.
        let (bb, cap) = match i % 4 {
            0 => {
                let ctr = rng.v_small();
                (
                    C2Aabb {
                        min: C2v { x: ctr.x - 10.0, y: ctr.y - 10.0 },
                        max: C2v { x: ctr.x + 10.0, y: ctr.y + 10.0 },
                    },
                    C2Capsule { a: ctr, b: rng.v_small(), r: 4.0 },
                )
            }
            1 => (rng.aabb(), rng.capsule()),
            2 => (
                C2Aabb { min: rng.v_any(), max: rng.v_any() },
                C2Capsule { a: rng.v_any(), b: rng.v_any(), r: rng.any() },
            ),
            _ => (
                C2Aabb { min: C2v { x: f32::NAN, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
                C2Capsule { a: rng.v_small(), b: rng.v_small(), r: rng.f32_in(0.0, 5.0) },
            ),
        };
        let sa = Shape::Aabb(bb);
        let sb = Shape::Capsule(cap);
        let dist = call_gjk(
            &c,
            &sa,
            &sb,
            &GjkOpts {
                use_radius: 1,
                want_outa: false,
                want_outb: false,
                want_iters: false,
                ..Default::default()
            },
            None,
        )
        .dist;
        if dist.is_nan() {
            nan_dist += 1;
        } else if dist == 0.0 {
            zero_dist += 1;
        } else {
            pos_dist += 1;
        }
        let cv = (c.c2AABBtoCapsule)(bb, cap);
        let rv = (r.c2AABBtoCapsule)(bb, cap);
        d.int(&format!("E40/E41#{i} A={bb:?} B={cap:?} dist={dist:?}"), cv, rv);
        // The C's rule: `if (dist) return 0; return 1;`
        let expect = if dist == 0.0 { 1 } else { 0 };
        assert_eq!(cv, expect, "E40/E41: C truthiness for dist={dist:?}");
        assert_eq!(rv, expect, "E40/E41: RUST truthiness for dist={dist:?}");
        // E41: a NaN return from c2GJK is provably UNREACHABLE whenever
        // use_radius != 0 (which is what both predicates pass): the final
        // `if (dist > rA + rB && dist > FLT_EPSILON)` is false for a NaN dist,
        // so control lands in the `else` branch that assigns `dist = 0`. Assert
        // that invariant here — for both libraries — over NaN-rich inputs.
        assert!(
            !dist.is_nan(),
            "E41: c2GJK returned NaN with use_radius = 1 for A={bb:?} B={cap:?}"
        );

        let cap2 = C2Capsule { a: rng.v_any(), b: rng.v_any(), r: rng.any() };
        d.int(
            &format!("E42#{i}"),
            (c.c2CapsuletoCapsule)(cap, cap2),
            (r.c2CapsuletoCapsule)(cap, cap2),
        );
    }
    d.finish();
    // Both reachable classes must be exercised. `nan_dist` must stay 0: see the
    // in-loop assertion — with use_radius != 0 the C can never return NaN, so
    // the "NaN is truthy -> return 0" arm of E41/E42 is dead code in practice.
    // Both implementations spell the same predicate (`if (dist)` vs
    // `if r != 0.0`), which agrees for NaN and for -0.0 as well.
    assert!(
        zero_dist > 0 && pos_dist > 0,
        "E40..E42 coverage: zero={zero_dist} positive={pos_dist}"
    );
    assert_eq!(
        nan_dist, 0,
        "E41: c2GJK produced a NaN distance with use_radius = 1 ({nan_dist} times)"
    );
    eprintln!("E40..E42 dist classes: zero={zero_dist} NaN={nan_dist} positive={pos_dist}");
    // Directly verify the float->bool mapping for the two boundary values that
    // `if (float)` treats specially, using the exact same expression shape the
    // two libraries use, so the -0.0 case is covered even though c2GJK cannot
    // produce it: C `if (x)` is false for both +0.0 and -0.0.
    for &x in &[0.0f32, -0.0f32] {
        assert!(!(x != 0.0), "sanity: {x:?} must be falsy");
    }
    for &x in &[f32::NAN, -f32::NAN, f32::MIN_POSITIVE, 1e-40, f32::INFINITY] {
        assert!(x != 0.0, "sanity: {x:?} must be truthy");
    }
}

// ===========================================================================
// E43..E51 — the strict `<` boundaries in the closed-form predicates.
// ===========================================================================

#[test]
fn E43_E44_circle_to_circle_boundaries() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E43/E44 c2CircletoCircle exact touch and negative radii");
    let mut d_touch = 0u32;
    // Exact touch via 3-4-5 triangles: |d| == rA + rB exactly.
    for ra in 1..12i32 {
        for rb in 1..12i32 {
            let sum = (ra + rb) as f32;
            let k = sum / 5.0;
            let a = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: ra as f32 };
            let b = C2Circle { p: C2v { x: 3.0 * k, y: 4.0 * k }, r: rb as f32 };
            let cv = (c.c2CircletoCircle)(a, b);
            let rv = (r.c2CircletoCircle)(a, b);
            d.int(&format!("E43 ra={ra} rb={rb}"), cv, rv);
            // d2 == r2 exactly -> strict `<` is false -> not collided
            let dx = 3.0 * k;
            let dy = 4.0 * k;
            if dx * dx + dy * dy == sum * sum {
                d_touch += 1;
                assert_eq!(cv, 0, "E43: exact touch must NOT collide (C)");
                assert_eq!(rv, 0, "E43: exact touch must NOT collide (RUST)");
            }
            // E44: negative radii — r2 = (rA+rB)^2 is positive again.
            let an = C2Circle { p: a.p, r: -(ra as f32) };
            let bn = C2Circle { p: b.p, r: -(rb as f32) };
            d.int(
                &format!("E44 ra={ra} rb={rb} negative"),
                (c.c2CircletoCircle)(an, bn),
                (r.c2CircletoCircle)(an, bn),
            );
            // mixed signs cancel -> r2 near 0
            let am = C2Circle { p: a.p, r: ra as f32 };
            let bm = C2Circle { p: b.p, r: -(ra as f32) };
            d.int(
                &format!("E44 mixed ra={ra}"),
                (c.c2CircletoCircle)(am, bm),
                (r.c2CircletoCircle)(am, bm),
            );
        }
    }
    d.finish();
    assert!(d_touch > 0, "E43: no exactly-touching configuration was built");
    eprintln!("E43 exact-touch configurations: {d_touch}");
}

#[test]
fn E45_circle_to_aabb_boundaries() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E45 c2CircletoAABB touch / r == 0 / inverted box");
    let mut touches = 0u32;
    for w in 1..8i32 {
        for &r0 in &[0.0f32, 1.0, 2.0, 3.0] {
            let bb = C2Aabb {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: w as f32, y: w as f32 },
            };
            // centre exactly r0 away from the nearest edge -> d2 == r2
            for p in [
                C2v { x: -r0, y: w as f32 / 2.0 },
                C2v { x: w as f32 + r0, y: w as f32 / 2.0 },
                C2v { x: w as f32 / 2.0, y: -r0 },
                C2v { x: 0.0, y: 0.0 },
                C2v { x: w as f32 / 2.0, y: w as f32 / 2.0 },
            ] {
                let a = C2Circle { p, r: r0 };
                let cv = (c.c2CircletoAABB)(a, bb);
                let rv = (r.c2CircletoAABB)(a, bb);
                d.int(&format!("E45 w={w} r={r0} p={p:?}"), cv, rv);
                if r0 > 0.0 && (p.x == -r0 || p.x == w as f32 + r0 || p.y == -r0) {
                    touches += 1;
                    assert_eq!(cv, 0, "E45: exact touch must not collide (C)");
                    assert_eq!(rv, 0, "E45: exact touch must not collide (RUST)");
                }
                if r0 == 0.0 {
                    assert_eq!(cv, 0, "E45: r == 0 can never collide (C)");
                    assert_eq!(rv, 0, "E45: r == 0 can never collide (RUST)");
                }
            }
            // inverted box: c2Clampv(a, min, max) with min > max
            let inv = C2Aabb { min: bb.max, max: bb.min };
            for p in [bb.min, bb.max, C2v { x: 0.5, y: 0.5 }, C2v { x: -5.0, y: 9.0 }] {
                let a = C2Circle { p, r: r0.max(0.5) };
                d.int(
                    &format!("E45 inverted w={w} r={r0} p={p:?}"),
                    (c.c2CircletoAABB)(a, inv),
                    (r.c2CircletoAABB)(a, inv),
                );
            }
        }
    }
    d.finish();
    assert!(touches > 0, "E45: no exact-touch configuration built");
    eprintln!("E45 exact-touch configurations: {touches}");
}

#[test]
fn E46_E47_E48_E49_circle_to_capsule_branches() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E46..E49 c2CircletoCapsule branches + degenerate + exact touch");
    let mut cover = [0u32; 3];
    let mut touches = 0u32;
    // Axis-aligned capsule from (0,0) to (L,0): x < 0 -> branch a,
    // 0 <= x <= L -> perpendicular branch, x > L -> branch b.
    for l in 1..10i32 {
        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: l as f32, y: 0.0 },
            r: 2.0,
        };
        for xi in -6..(l + 7) {
            for yi in -4..5 {
                for &cr in &[0.0f32, 1.0, 2.0, 3.0] {
                    let a = C2Circle { p: C2v { x: xi as f32, y: yi as f32 }, r: cr };
                    let br = {
                        let n = C2v { x: cap.b.x - cap.a.x, y: cap.b.y - cap.a.y };
                        let ap = C2v { x: a.p.x - cap.a.x, y: a.p.y - cap.a.y };
                        let da = ap.x * n.x + ap.y * n.y;
                        if da < 0.0 {
                            0
                        } else {
                            let pb = C2v { x: a.p.x - cap.b.x, y: a.p.y - cap.b.y };
                            let db = pb.x * n.x + pb.y * n.y;
                            if db < 0.0 { 1 } else { 2 }
                        }
                    };
                    cover[br] += 1;
                    let cv = (c.c2CircletoCapsule)(a, cap);
                    let rv = (r.c2CircletoCapsule)(a, cap);
                    d.int(&format!("E46-E49 l={l} p={:?} r={cr}", a.p), cv, rv);
                    // E49: exact touch -> d2 == (rA+rB)^2
                    if br == 1 && (yi as f32).abs() == cr + cap.r {
                        touches += 1;
                        assert_eq!(cv, 0, "E49: exact touch must not collide (C)");
                        assert_eq!(rv, 0, "E49: exact touch must not collide (RUST)");
                    }
                }
            }
        }
    }
    // E48: degenerate capsule a == b -> n == (0,0) -> the `B.b` branch, no
    // division by zero.
    for k in 0..40i32 {
        let p = C2v { x: k as f32 - 20.0, y: (k % 7) as f32 };
        let cap = C2Capsule { a: p, b: p, r: (k % 5) as f32 };
        for &cr in &[0.0f32, 1.0, 4.0] {
            let a = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: cr };
            let cv = (c.c2CircletoCapsule)(a, cap);
            let rv = (r.c2CircletoCapsule)(a, cap);
            d.int(&format!("E48 degenerate cap={cap:?} r={cr}"), cv, rv);
            // must equal a plain circle-circle test against the point
            let cc = (c.c2CircletoCircle)(a, C2Circle { p, r: cap.r });
            d.int(&format!("E48 == circle-circle cap={cap:?} r={cr}"), cv, cc);
        }
    }
    d.finish();
    assert!(cover.iter().all(|&x| x > 0), "E46/E47 branch coverage {cover:?}");
    assert!(touches > 0, "E49: no exact-touch configuration built");
    eprintln!("E46..E49 branch coverage {cover:?}, exact touches {touches}");
}

#[test]
fn E50_E51_aabb_to_aabb_boundaries() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E50/E51 c2AABBtoAABB edge touch, inverted, NaN");
    // Edge/corner touching: B.max.x == A.min.x makes the strict `<` false, so
    // the C reports a collision.
    for x in -3..4i32 {
        for y in -3..4i32 {
            let a = C2Aabb {
                min: C2v { x: x as f32, y: y as f32 },
                max: C2v { x: x as f32 + 2.0, y: y as f32 + 2.0 },
            };
            for (dx, dy) in [(2.0f32, 0.0f32), (0.0, 2.0), (2.0, 2.0), (-2.0, -2.0), (4.0, 0.0)] {
                let b = C2Aabb {
                    min: C2v { x: a.min.x + dx, y: a.min.y + dy },
                    max: C2v { x: a.max.x + dx, y: a.max.y + dy },
                };
                let cv = (c.c2AABBtoAABB)(a, b);
                let rv = (r.c2AABBtoAABB)(a, b);
                d.int(&format!("E50 a={a:?} b={b:?}"), cv, rv);
                if dx.abs() <= 2.0 && dy.abs() <= 2.0 {
                    assert_eq!(cv, 1, "E50: touching boxes must collide (C)");
                    assert_eq!(rv, 1, "E50: touching boxes must collide (RUST)");
                } else {
                    assert_eq!(cv, 0, "E50: separated boxes must not collide (C)");
                    assert_eq!(rv, 0, "E50: separated boxes must not collide (RUST)");
                }
            }
            // E51: inverted box, and all-NaN box (every `<` false -> !0 -> 1)
            let inv = C2Aabb { min: a.max, max: a.min };
            d.int(
                &format!("E51 inverted a={a:?}"),
                (c.c2AABBtoAABB)(inv, a),
                (r.c2AABBtoAABB)(inv, a),
            );
            let nanbox = C2Aabb {
                min: C2v { x: f32::NAN, y: f32::NAN },
                max: C2v { x: f32::NAN, y: f32::NAN },
            };
            let cv = (c.c2AABBtoAABB)(nanbox, a);
            let rv = (r.c2AABBtoAABB)(nanbox, a);
            d.int(&format!("E51 NaN box vs a={a:?}"), cv, rv);
            assert_eq!(cv, 1, "E51: all-NaN comparisons are false -> !(0) == 1 (C)");
            assert_eq!(rv, 1, "E51: all-NaN comparisons are false -> !(0) == 1 (RUST)");
        }
    }
    d.finish();
}

// ===========================================================================
// E52, E53 — the public `capsule` entry point with invalid / extreme inputs.
// ===========================================================================

#[test]
fn E52_capsule_with_nan_arguments() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E52 capsule() with NaN in any argument");
    let nan = f32::NAN;
    let neg_nan = -f32::NAN;
    let base = [-40.0f32, -40.0, -20.0, 100.0, 10.0];
    for slot in 0..5 {
        for &n in &[nan, neg_nan] {
            let mut args = base;
            args[slot] = n;
            let cv = (c.capsule)(args[0], args[1], args[2], args[3], args[4]);
            let rv = (r.capsule)(args[0], args[1], args[2], args[3], args[4]);
            d.int(&format!("E52 slot={slot} args={args:?}"), cv, rv);
        }
    }
    // all-NaN and every 2-slot combination
    for i in 0..5 {
        for j in 0..5 {
            let mut args = base;
            args[i] = nan;
            args[j] = nan;
            d.int(
                &format!("E52 slots={i},{j}"),
                (c.capsule)(args[0], args[1], args[2], args[3], args[4]),
                (r.capsule)(args[0], args[1], args[2], args[3], args[4]),
            );
        }
    }
    d.int(
        "E52 all NaN",
        (c.capsule)(nan, nan, nan, nan, nan),
        (r.capsule)(nan, nan, nan, nan, nan),
    );
    d.finish();
}

#[test]
fn E53_capsule_extreme_arguments() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E53 capsule() with +-inf, FLT_MAX, denormals, negative r");
    let vals = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-40,
        -1e-40,
        0.0,
        -0.0,
        -1.0,
        -1e30,
        1e30,
    ];
    for &a in &vals {
        for &b in &vals {
            for &e in &[0.0f32, -5.0, -1e30, f32::INFINITY, f32::MAX, 1e-40] {
                let cv = (c.capsule)(a, b, -20.0, 30.0, e);
                let rv = (r.capsule)(a, b, -20.0, 30.0, e);
                d.int(&format!("E53 capsule({a:?},{b:?},-20,30,{e:?})"), cv, rv);
                let cv2 = (c.capsule)(-40.0, -40.0, a, b, e);
                let rv2 = (r.capsule)(-40.0, -40.0, a, b, e);
                d.int(&format!("E53 capsule(-40,-40,{a:?},{b:?},{e:?})"), cv2, rv2);
            }
        }
    }
    d.finish();
}

// ===========================================================================
// E54 — c2BBVerts writes exactly four vertices, in a fixed order.
// ===========================================================================

#[test]
fn E54_bbverts_writes_exactly_four() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E54 c2BBVerts writes exactly out[0..3]");
    let mut rng = Rng::new(SEED ^ 227);
    for i in 0..2000 {
        let bb = if i % 4 == 0 {
            C2Aabb { min: rng.v_any(), max: rng.v_any() }
        } else {
            rng.aabb()
        };
        let base: [C2v; 8] = poison(i as u8);
        let mut cb = base;
        let mut rb = base;
        let mut cbb = bb;
        let mut rbb = bb;
        unsafe {
            (c.c2BBVerts)(cb.as_mut_ptr(), &mut cbb);
            (r.c2BBVerts)(rb.as_mut_ptr(), &mut rbb);
        }
        d.varr(&format!("E54#{i} out"), &cb, &rb);
        d.aabb(&format!("E54#{i} input untouched"), &cbb, &rbb);
        // slots 4..8 must still hold the poison in BOTH libraries
        for k in 4..8 {
            d.v(&format!("E54#{i} C slot{k} untouched"), cb[k], base[k]);
            d.v(&format!("E54#{i} RUST slot{k} untouched"), rb[k], base[k]);
        }
        // and the documented vertex order
        d.v(&format!("E54#{i} order[0]"), cb[0], bb.min);
        d.v(&format!("E54#{i} order[2]"), cb[2], bb.max);
    }
    d.finish();
}

// ===========================================================================
// E55..E58 — every branch of c22 and c23, constructed explicitly.
// ===========================================================================

fn simplex2(p0: C2v, p1: C2v, div: f32) -> C2Simplex {
    let mut s = C2Simplex::default();
    for k in 0..4 {
        s.verts[k].u = 0.25;
        s.verts[k].iA = k as c_int;
        s.verts[k].iB = (3 - k) as c_int;
        s.verts[k].sA = C2v { x: k as f32, y: -(k as f32) };
        s.verts[k].sB = C2v { x: -(k as f32), y: k as f32 };
    }
    s.verts[0].p = p0;
    s.verts[1].p = p1;
    s.div = div;
    s.count = 2;
    s
}

#[test]
fn E55_E56_c22_both_collapse_branches() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E55/E56 c22 v<=0 and u<=0 collapse branches");
    let mut cover = [0u32; 3];
    // Origin projects outside segment [a,b] on the a side, on the b side, and
    // strictly inside, plus the exact-zero boundaries.
    let cases: &[(C2v, C2v)] = &[
        (C2v { x: 1.0, y: 0.0 }, C2v { x: 2.0, y: 0.0 }),   // v <= 0 (a closest)
        (C2v { x: -2.0, y: 0.0 }, C2v { x: -1.0, y: 0.0 }), // u <= 0 (b closest)
        (C2v { x: -1.0, y: 0.0 }, C2v { x: 1.0, y: 0.0 }),  // interior
        (C2v { x: 0.0, y: 0.0 }, C2v { x: 1.0, y: 0.0 }),   // v == 0 exactly
        (C2v { x: 1.0, y: 0.0 }, C2v { x: 0.0, y: 0.0 }),   // u == 0 exactly
        (C2v { x: 1.0, y: 1.0 }, C2v { x: 1.0, y: 1.0 }),   // coincident
        (C2v { x: -0.0, y: -0.0 }, C2v { x: 0.0, y: 0.0 }), // signed zeros
        (C2v { x: f32::NAN, y: 0.0 }, C2v { x: 1.0, y: 0.0 }), // NaN -> all false
    ];
    for (k, &(p0, p1)) in cases.iter().enumerate() {
        for &div in &[1.0f32, 0.0, -2.5, 7.0] {
            let s = simplex2(p0, p1, div);
            let dot = |a: C2v, b: C2v| a.x * b.x + a.y * b.y;
            let sub = |a: C2v, b: C2v| C2v { x: a.x - b.x, y: a.y - b.y };
            let u = dot(p1, sub(p1, p0));
            let v = dot(p0, sub(p0, p1));
            let br = if v <= 0.0 { 0 } else if u <= 0.0 { 1 } else { 2 };
            cover[br] += 1;
            let mut cs = s;
            let mut rs = s;
            unsafe {
                (c.c22)(&mut cs);
                (r.c22)(&mut rs);
            }
            d.simplex(&format!("E55/E56#{k} div={div} p0={p0:?} p1={p1:?}"), &cs, &rs);
            // the documented outcome of each branch
            match br {
                0 | 1 => {
                    assert_eq!(cs.count, 1, "E55/E56: collapse must set count = 1 (C)");
                    assert_eq!(rs.count, 1, "E55/E56: collapse must set count = 1 (RUST)");
                    assert_eq!(cs.div.to_bits(), 1.0f32.to_bits(), "E55/E56: div = 1");
                    assert_eq!(cs.verts[0].u.to_bits(), 1.0f32.to_bits(), "E55/E56: u = 1");
                }
                _ => {
                    assert_eq!(cs.count, 2, "E55/E56: interior keeps count = 2 (C)");
                    assert_eq!(rs.count, 2, "E55/E56: interior keeps count = 2 (RUST)");
                }
            }
        }
    }
    d.finish();
    assert!(cover.iter().all(|&x| x > 0), "E55/E56 branch coverage {cover:?}");
    eprintln!("E55/E56 c22 branch coverage: {cover:?}");
}

#[test]
fn E57_E58_c23_all_seven_branches() {
    let (c, r) = load_pair();
    let mut d = Diff::new("E57/E58 c23 all seven branches, constructed explicitly");
    let mut cover = [0u32; 7];
    let dot = |a: C2v, b: C2v| a.x * b.x + a.y * b.y;
    let sub = |a: C2v, b: C2v| C2v { x: a.x - b.x, y: a.y - b.y };
    let det = |a: C2v, b: C2v| a.x * b.y - a.y * b.x;
    let branch = |a: C2v, b: C2v, e: C2v| -> usize {
        let uAB = dot(b, sub(b, a));
        let vAB = dot(a, sub(a, b));
        let uBC = dot(e, sub(e, b));
        let vBC = dot(b, sub(b, e));
        let uCA = dot(a, sub(a, e));
        let vCA = dot(e, sub(e, a));
        let area = det(sub(b, a), sub(e, a));
        let uABC = det(b, e) * area;
        let vABC = det(e, a) * area;
        let wABC = det(a, b) * area;
        if vAB <= 0.0 && uCA <= 0.0 {
            0
        } else if uAB <= 0.0 && vBC <= 0.0 {
            1
        } else if uBC <= 0.0 && vCA <= 0.0 {
            2
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            3
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            4
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            5
        } else {
            6
        }
    };
    // Enumerate a small integer lattice of triangles: this hits every branch.
    let coords: Vec<f32> = (-3..=3).map(|k| k as f32).collect();
    let mut rng = Rng::new(SEED ^ 228);
    let mut tried = 0u32;
    for &ax in &coords {
        for &ay in &coords {
            for &bx in &coords {
                for &by in &coords {
                    for &cx in &coords {
                        for &cy in &coords {
                            tried += 1;
                            if tried % 23 != 0 {
                                continue;
                            }
                            let (p0, p1, p2) = (
                                C2v { x: ax, y: ay },
                                C2v { x: bx, y: by },
                                C2v { x: cx, y: cy },
                            );
                            let br = branch(p0, p1, p2);
                            cover[br] += 1;
                            let mut s = rnd_simplex(&mut rng, 3);
                            s.verts[0].p = p0;
                            s.verts[1].p = p1;
                            s.verts[2].p = p2;
                            let mut cs = s;
                            let mut rs = s;
                            unsafe {
                                (c.c23)(&mut cs);
                                (r.c23)(&mut rs);
                            }
                            d.simplex(
                                &format!("E57/E58 br={br} p={p0:?}{p1:?}{p2:?}"),
                                &cs,
                                &rs,
                            );
                            let expect_count = match br {
                                0 | 1 | 2 => 1,
                                3 | 4 | 5 => 2,
                                _ => 3,
                            };
                            assert_eq!(cs.count, expect_count, "E57/E58 br={br}: C count");
                            assert_eq!(rs.count, expect_count, "E57/E58 br={br}: RUST count");
                        }
                    }
                }
            }
        }
    }
    d.finish();
    assert!(cover.iter().all(|&x| x > 0), "E57/E58 branch coverage {cover:?}");
    eprintln!("E57/E58 c23 branch coverage: {cover:?}");
}

// ===========================================================================
// Generic FFI boundaries not tied to a single ERRORS.md row.
// ===========================================================================

#[test]
fn generic_out_of_range_enum_values_everywhere() {
    let (c, r) = load_pair();
    let mut d = Diff::new("generic: out-of-range C2_TYPE across every entry point that takes one");
    let mut rng = Rng::new(SEED ^ 229);
    for &ty in BAD_TYPES {
        for i in 0..8 {
            let s = shape_of(&mut rng, i % 3);
            // c2MakeProxy
            let base: C2Proxy = poison(ty as u8);
            let mut cp = base;
            let mut rp = base;
            unsafe {
                (c.c2MakeProxy)(s.as_ptr(), ty, &mut cp);
                (r.c2MakeProxy)(s.as_ptr(), ty, &mut rp);
            }
            d.proxy(&format!("generic makeproxy ty={ty}#{i}"), &cp, &rp);
            // c2Collided, all four combinations with a valid partner
            for &good in VALID_TYPES {
                d.int(
                    &format!("generic collided ({ty},{good})"),
                    unsafe { (c.c2Collided)(s.as_ptr(), ty, s.as_ptr(), good) },
                    unsafe { (r.c2Collided)(s.as_ptr(), ty, s.as_ptr(), good) },
                );
                d.int(
                    &format!("generic collided ({good},{ty})"),
                    unsafe { (c.c2Collided)(s.as_ptr(), good, s.as_ptr(), ty) },
                    unsafe { (r.c2Collided)(s.as_ptr(), good, s.as_ptr(), ty) },
                );
            }
        }
    }
    d.finish();
}

#[test]
fn generic_zero_and_boundary_lengths() {
    let (c, r) = load_pair();
    let mut d = Diff::new("generic: zero / boundary counts and lengths");
    let mut rng = Rng::new(SEED ^ 230);
    // c2Support over every count from the minimum to the maximum in-bounds
    // value for an 8-element array (the largest a c2Proxy can hold).
    for count in -2..=8i32 {
        for i in 0..50 {
            let mut verts = [C2v::default(); 8];
            for v in verts.iter_mut() {
                *v = rng.v_coord();
            }
            let dir = rng.v_coord();
            let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, dir) };
            let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, dir) };
            d.int(&format!("generic support count={count}#{i}"), cv, rv);
            assert!(
                cv >= 0 && cv < count.max(1),
                "generic: c2Support returned {cv} for count={count}"
            );
        }
    }
    // Simplex counts one step past every documented value.
    for count in -2..=5i32 {
        for i in 0..50 {
            let s = rnd_simplex(&mut rng, count);
            let mut cs = s;
            let mut rs = s;
            d.f32(
                &format!("generic metric count={count}#{i}"),
                unsafe { (c.c2GJKSimplexMetric)(&mut cs) },
                unsafe { (r.c2GJKSimplexMetric)(&mut rs) },
            );
            let mut cs = s;
            let mut rs = s;
            d.v(
                &format!("generic c2D count={count}#{i}"),
                unsafe { (c.c2D)(&mut cs) },
                unsafe { (r.c2D)(&mut rs) },
            );
            let mut cs = s;
            let mut rs = s;
            d.v(
                &format!("generic c2L count={count}#{i}"),
                unsafe { (c.c2L)(&mut cs) },
                unsafe { (r.c2L)(&mut rs) },
            );
            let mut cs = s;
            let mut rs = s;
            let (mut ca, mut cb) = (C2v::default(), C2v::default());
            let (mut ra, mut rb) = (C2v::default(), C2v::default());
            unsafe {
                (c.c2Witness)(&mut cs, &mut ca, &mut cb);
                (r.c2Witness)(&mut rs, &mut ra, &mut rb);
            }
            d.v(&format!("generic witness a count={count}#{i}"), ca, ra);
            d.v(&format!("generic witness b count={count}#{i}"), cb, rb);
            // c22/c23 with an out-of-range count still only touch verts[0..2]
            let mut cs = s;
            let mut rs = s;
            unsafe {
                (c.c22)(&mut cs);
                (r.c22)(&mut rs);
            }
            d.simplex(&format!("generic c22 count={count}#{i}"), &cs, &rs);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                (c.c23)(&mut cs);
                (r.c23)(&mut rs);
            }
            d.simplex(&format!("generic c23 count={count}#{i}"), &cs, &rs);
        }
    }
    d.finish();
}
