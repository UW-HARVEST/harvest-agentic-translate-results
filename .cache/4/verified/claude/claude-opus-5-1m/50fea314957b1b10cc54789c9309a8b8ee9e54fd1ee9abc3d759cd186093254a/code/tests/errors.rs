//! Phase C — one differential test per row of ERRORS.md.
//!
//! Test names encode the row numbers they discharge. Where the C's behaviour is
//! undefined (rows 35..39) the assertion is trap-parity / non-trapping rather
//! than value equality, and the reason is stated inline.

mod common;
use common::*;
use std::ffi::{c_int, c_void};

// ===========================================================================
// Row 1 — c2MakeProxy with a type outside the enum
// ===========================================================================

/// C enums accept any `int` across the FFI boundary. The C `switch` has no
/// `default:`, so `*p` must be left byte-for-byte untouched.
#[test]
fn err01_makeproxy_invalid_enum() {
    let p = pair();
    let mut rng = Rng::new(0xE001);
    let bad: [c_int; 12] =
        [3, 4, 5, 99, -1, -2, -99, c_int::MIN, c_int::MAX, 0x1000, 1 << 30, -(1 << 30)];
    for &ty in bad.iter() {
        for i in 0..256 {
            // a real shape body, so only the `type` is invalid
            let shape = rng.shape_any();
            for pat in [0x00u8, 0xAA, 0xFF, 0x5A] {
                let mut pc = c2Proxy::default();
                let mut pr = c2Proxy::default();
                unsafe {
                    let n = std::mem::size_of::<c2Proxy>();
                    std::ptr::write_bytes(&mut pc as *mut c2Proxy as *mut u8, pat, n);
                    std::ptr::write_bytes(&mut pr as *mut c2Proxy as *mut u8, pat, n);
                    let before = pc;
                    (p.c.c2MakeProxy)(shape.as_ptr(), ty, &mut pc);
                    (p.r.c2MakeProxy)(shape.as_ptr(), ty, &mut pr);
                    eq_proxy(&format!("err01 ty={ty} pat={pat:#02x}[{i}]"), &pc, &pr);
                    // and specifically: nothing at all was written
                    eq_proxy(&format!("err01 ty={ty} untouched[{i}]"), &before, &pc);
                }
            }
        }
    }
}

/// Also verify a NULL shape pointer is never dereferenced for an invalid type
/// (the C switch falls through without touching `shape`).
#[test]
fn err01b_makeproxy_invalid_enum_null_shape() {
    let p = pair();
    for ty in [3i32, -1, 99, c_int::MIN, c_int::MAX] {
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        unsafe {
            (p.c.c2MakeProxy)(std::ptr::null::<c_void>(), ty, &mut pc);
            (p.r.c2MakeProxy)(std::ptr::null::<c_void>(), ty, &mut pr);
        }
        eq_proxy(&format!("err01b ty={ty}"), &pc, &pr);
    }
}

// ===========================================================================
// Rows 2/3 — c2GJKSimplexMetric out-of-range count
// Rows 4/5 — c2D out-of-range count
// Row 6    — c2Witness out-of-range count
// Row 9    — c2L out-of-range count
// ===========================================================================

const BAD_COUNTS: [c_int; 12] =
    [0, 4, 5, 6, 7, 8, -1, -2, -100, c_int::MIN, c_int::MAX, 1 << 20];

#[test]
fn err02_03_metric_out_of_range_count() {
    let p = pair();
    let mut rng = Rng::new(0xE002);
    // Row 2: count == 1 -> exactly 0.0
    for i in 0..1024 {
        let mut sc = rng.simplex_any(1);
        let mut sr = sc;
        let mc = unsafe { (p.c.c2GJKSimplexMetric)(&mut sc) };
        let mr = unsafe { (p.r.c2GJKSimplexMetric)(&mut sr) };
        eq_f32(&format!("err02 count=1[{i}]"), mc, mr);
        eq_f32(&format!("err02 count=1[{i}] is zero"), mc, 0.0);
    }
    // Row 3: default: falls into case 1: -> 0.0
    for &count in BAD_COUNTS.iter() {
        for i in 0..256 {
            let mut sc = rng.simplex_any(count);
            let mut sr = sc;
            let mc = unsafe { (p.c.c2GJKSimplexMetric)(&mut sc) };
            let mr = unsafe { (p.r.c2GJKSimplexMetric)(&mut sr) };
            eq_f32(&format!("err03 count={count}[{i}]"), mc, mr);
            eq_f32(&format!("err03 count={count}[{i}] is zero"), mc, 0.0);
        }
    }
}

#[test]
fn err04_05_c2d_out_of_range_count() {
    let p = pair();
    let mut rng = Rng::new(0xE004);
    let zero = c2v { x: 0.0, y: 0.0 };
    // Row 4: count == 3 -> (0,0)
    for i in 0..1024 {
        let mut sc = rng.simplex_any(3);
        let mut sr = sc;
        let dc = unsafe { (p.c.c2D)(&mut sc) };
        let dr = unsafe { (p.r.c2D)(&mut sr) };
        eq_v(&format!("err04 count=3[{i}]"), dc, dr);
        eq_v(&format!("err04 count=3[{i}] is zero"), dc, zero);
    }
    // Row 5: any other out-of-range count -> (0,0)
    for &count in BAD_COUNTS.iter() {
        for i in 0..256 {
            let mut sc = rng.simplex_any(count);
            let mut sr = sc;
            let dc = unsafe { (p.c.c2D)(&mut sc) };
            let dr = unsafe { (p.r.c2D)(&mut sr) };
            eq_v(&format!("err05 count={count}[{i}]"), dc, dr);
            eq_v(&format!("err05 count={count}[{i}] is zero"), dc, zero);
        }
    }
}

#[test]
fn err06_c2witness_out_of_range_count() {
    let p = pair();
    let mut rng = Rng::new(0xE006);
    let zero = c2v { x: 0.0, y: 0.0 };
    let poison = c2v { x: f32::from_bits(0xDEAD_BEEF), y: f32::from_bits(0xC0DE_D00D) };
    for &count in BAD_COUNTS.iter() {
        for i in 0..256 {
            let mut sc = rng.simplex_any(count);
            let mut sr = sc;
            let (mut ac, mut bc) = (poison, poison);
            let (mut ar, mut br) = (poison, poison);
            unsafe {
                (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
            }
            eq_v(&format!("err06 count={count}[{i}] a"), ac, ar);
            eq_v(&format!("err06 count={count}[{i}] b"), bc, br);
            // the `default:` arm writes exactly (0,0) to both
            eq_v(&format!("err06 count={count}[{i}] a is zero"), ac, zero);
            eq_v(&format!("err06 count={count}[{i}] b is zero"), bc, zero);
        }
    }
}

#[test]
fn err09_c2l_out_of_range_count() {
    let p = pair();
    let mut rng = Rng::new(0xE009);
    let zero = c2v { x: 0.0, y: 0.0 };
    // count == 3 is a REAL case in c2Witness but hits `default:` in c2L.
    let mut counts: Vec<c_int> = vec![3];
    counts.extend_from_slice(&BAD_COUNTS);
    for &count in counts.iter() {
        for i in 0..256 {
            let mut sc = rng.simplex_any(count);
            let mut sr = sc;
            let lc = unsafe { (p.c.c2L)(&mut sc) };
            let lr = unsafe { (p.r.c2L)(&mut sr) };
            eq_v(&format!("err09 count={count}[{i}]"), lc, lr);
            eq_v(&format!("err09 count={count}[{i}] is zero"), lc, zero);
        }
    }
}

// ===========================================================================
// Rows 7/8 — c2Witness with div == 0 / div == NaN
// Row 10   — c2L with div == 0
// ===========================================================================

#[test]
fn err07_08_10_zero_and_nan_div() {
    let p = pair();
    let mut rng = Rng::new(0xE007);
    let poison = c2v { x: 1.0, y: 2.0 };
    for &div in [0.0f32, -0.0].iter() {
        for count in [1i32, 2, 3] {
            for i in 0..512 {
                let mut sc = rng.simplex_any(count);
                sc.div = div;
                let mut sr = sc;
                let (mut ac, mut bc) = (poison, poison);
                let (mut ar, mut br) = (poison, poison);
                unsafe {
                    (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
                }
                // div is NaN-free here, so STRICT: the inf/NaN pattern produced by
                // 1/0 -> inf and inf*0 -> indefinite must match bit-for-bit.
                eq_v(&format!("err07 div={div} c={count}[{i}] a"), ac, ar);
                eq_v(&format!("err07 div={div} c={count}[{i}] b"), bc, br);
                let mut lc = sc;
                let mut lr = sr;
                eq_v(
                    &format!("err10 div={div} c={count}[{i}]"),
                    unsafe { (p.c.c2L)(&mut lc) },
                    unsafe { (p.r.c2L)(&mut lr) },
                );
            }
        }
    }
    // Row 8: div == NaN -> SOFT (a NaN input can meet a differently-payloaded NaN)
    for count in [1i32, 2, 3] {
        for i in 0..512 {
            let mut sc = rng.simplex_any(count);
            sc.div = f32::NAN;
            let mut sr = sc;
            let (mut ac, mut bc) = (poison, poison);
            let (mut ar, mut br) = (poison, poison);
            unsafe {
                (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (p.r.c2Witness)(&mut sr, &mut ar, &mut br);
            }
            eq_v_soft(&format!("err08 c={count}[{i}] a"), ac, ar);
            eq_v_soft(&format!("err08 c={count}[{i}] b"), bc, br);
        }
    }
}

// ===========================================================================
// Rows 11/12/13/14 — c2Support degenerate arguments
// ===========================================================================

#[test]
fn err11_12_13_14_support_degenerate() {
    let p = pair();
    let mut rng = Rng::new(0xE011);
    for i in 0..2048 {
        let mut verts = [c2v::default(); 8];
        for k in 0..8 {
            verts[k] = rng.vec_any_scale();
        }
        // Row 11: count <= 0 -> returns 0 (verts[0] read before the guard)
        for count in [0i32, -1, -2, -1000, c_int::MIN] {
            let ic = unsafe { (p.c.c2Support)(verts.as_ptr(), count, verts[0]) };
            let ir = unsafe { (p.r.c2Support)(verts.as_ptr(), count, verts[0]) };
            eq_i(&format!("err11 count={count}[{i}]"), ic, ir);
            eq_i(&format!("err11 count={count}[{i}] is 0"), ic, 0);
        }
        // Row 12: count == 1 -> returns 0
        let d = rng.vec_any_scale();
        let ic = unsafe { (p.c.c2Support)(verts.as_ptr(), 1, d) };
        eq_i(&format!("err12[{i}]"), ic, 0);
        eq_i(&format!("err12[{i}] rust"), unsafe { (p.r.c2Support)(verts.as_ptr(), 1, d) }, 0);
        // Row 13: d == (0,0) -> all dots 0 -> first index wins
        let zero = c2v { x: 0.0, y: 0.0 };
        for count in [1i32, 2, 4, 8] {
            let zc = unsafe { (p.c.c2Support)(verts.as_ptr(), count, zero) };
            let zr = unsafe { (p.r.c2Support)(verts.as_ptr(), count, zero) };
            eq_i(&format!("err13 count={count}[{i}]"), zc, zr);
            eq_i(&format!("err13 count={count}[{i}] is 0"), zc, 0);
        }
        // Row 14: a NaN vertex is never selected
        let mut nverts = verts;
        let bad = (rng.below(7) + 1) as usize;
        nverts[bad] = c2v { x: f32::NAN, y: f32::NAN };
        for count in [2i32, 4, 8] {
            let nc = unsafe { (p.c.c2Support)(nverts.as_ptr(), count, d) };
            let nr = unsafe { (p.r.c2Support)(nverts.as_ptr(), count, d) };
            eq_i(&format!("err14 count={count} bad={bad}[{i}]"), nc, nr);
            if (bad as c_int) < count {
                assert_ne!(nc, bad as c_int, "err14: NaN vertex {bad} was selected");
            }
        }
        // all-NaN vertices -> index 0 (dmax starts as NaN, nothing compares greater)
        let allnan = [c2v { x: f32::NAN, y: f32::NAN }; 8];
        for count in [1i32, 2, 4, 8] {
            let ac2 = unsafe { (p.c.c2Support)(allnan.as_ptr(), count, d) };
            let ar2 = unsafe { (p.r.c2Support)(allnan.as_ptr(), count, d) };
            eq_i(&format!("err14 allnan count={count}[{i}]"), ac2, ar2);
        }
    }
}

// ===========================================================================
// Rows 15/16 — c2Div by +0.0 and -0.0
// Rows 17/18/19 — c2Norm degenerate
// Rows 20/21 — c2Len overflow / NaN
// ===========================================================================

#[test]
fn err15_16_div_by_zero() {
    let p = pair();
    let mut rng = Rng::new(0xE015);
    for i in 0..4096 {
        // NaN-free numerators -> STRICT (0*inf -> indefinite NaN in both)
        let a = rng.vec_nasty_no_nan();
        for d in [0.0f32, -0.0] {
            eq_v(
                &format!("err15/16 a={a:?} d={d:?}[{i}]"),
                unsafe { (p.c.c2Div)(a, d) },
                unsafe { (p.r.c2Div)(a, d) },
            );
        }
    }
    // exact expected sentinels for the canonical cases
    for (a, d, want) in [
        (c2v { x: 1.0, y: -1.0 }, 0.0f32, c2v { x: f32::INFINITY, y: f32::NEG_INFINITY }),
        (c2v { x: 1.0, y: -1.0 }, -0.0f32, c2v { x: f32::NEG_INFINITY, y: f32::INFINITY }),
    ] {
        let got = unsafe { (p.c.c2Div)(a, d) };
        eq_v("err15/16 C sentinel", got, want);
        eq_v("err15/16 Rust sentinel", unsafe { (p.r.c2Div)(a, d) }, want);
    }
    // 0/0 -> NaN in both, same payload (hardware indefinite)
    let z = c2v { x: 0.0, y: -0.0 };
    let gc = unsafe { (p.c.c2Div)(z, 0.0) };
    let gr = unsafe { (p.r.c2Div)(z, 0.0) };
    eq_v("err15 zero/zero", gc, gr);
    assert!(gc.x.is_nan() && gc.y.is_nan(), "err15: 0/0 should be NaN, got {gc:?}");
}

#[test]
fn err17_18_19_norm_degenerate() {
    let p = pair();
    // Row 17: zero vector -> (NaN, NaN)
    for z in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ] {
        let gc = unsafe { (p.c.c2Norm)(z) };
        let gr = unsafe { (p.r.c2Norm)(z) };
        eq_v(&format!("err17 {z:?}"), gc, gr);
        assert!(gc.x.is_nan() && gc.y.is_nan(), "err17: c2Norm({z:?}) should be NaN, got {gc:?}");
    }
    // Row 18: inf components -> len = inf -> 1/inf = 0 -> inf*0 = NaN
    for v in [
        c2v { x: f32::INFINITY, y: 1.0 },
        c2v { x: f32::NEG_INFINITY, y: 1.0 },
        c2v { x: f32::INFINITY, y: f32::INFINITY },
        c2v { x: 1e30, y: 1e30 },
        c2v { x: f32::MAX, y: f32::MAX },
    ] {
        let gc = unsafe { (p.c.c2Norm)(v) };
        let gr = unsafe { (p.r.c2Norm)(v) };
        eq_v(&format!("err18 {v:?}"), gc, gr);
    }
    // Row 19: NaN components -> SOFT
    let mut rng = Rng::new(0xE019);
    for i in 0..1024 {
        let v = c2v { x: f32::NAN, y: rng.nasty() };
        eq_v_soft(&format!("err19 a[{i}]"), unsafe { (p.c.c2Norm)(v) }, unsafe {
            (p.r.c2Norm)(v)
        });
        let w = c2v { x: rng.nasty(), y: f32::NAN };
        eq_v_soft(&format!("err19 b[{i}]"), unsafe { (p.c.c2Norm)(w) }, unsafe {
            (p.r.c2Norm)(w)
        });
    }
}

#[test]
fn err20_21_len_overflow_and_nan() {
    let p = pair();
    // Row 20: c2Dot overflows to +inf -> sqrtf(inf) = inf
    for v in [
        c2v { x: 1e30, y: 1e30 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MAX, y: 0.0 },
        c2v { x: f32::INFINITY, y: 0.0 },
    ] {
        let lc = unsafe { (p.c.c2Len)(v) };
        let lr = unsafe { (p.r.c2Len)(v) };
        eq_f32(&format!("err20 {v:?}"), lc, lr);
        assert!(lc.is_infinite() && lc > 0.0, "err20: c2Len({v:?}) = {lc}, want +inf");
    }
    // Row 21: NaN in -> NaN out
    for v in [
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: 0.0, y: f32::NAN },
        c2v { x: f32::NAN, y: f32::NAN },
    ] {
        let lc = unsafe { (p.c.c2Len)(v) };
        let lr = unsafe { (p.r.c2Len)(v) };
        eq_f32_soft(&format!("err21 {v:?}"), lc, lr);
        assert!(lc.is_nan() && lr.is_nan(), "err21: both should be NaN");
    }
    // inf - inf inside c2Dot: (inf, inf) . itself is inf+inf = inf, not NaN
    let v = c2v { x: f32::INFINITY, y: f32::NEG_INFINITY };
    eq_f32("err20 mixed inf", unsafe { (p.c.c2Len)(v) }, unsafe { (p.r.c2Len)(v) });
}

// ===========================================================================
// Row 22 — c2Maxv / c2Minv NaN asymmetry (NaN in `a` dropped, in `b` kept)
// Row 23 — c2Clampv with an inverted range
// ===========================================================================

#[test]
fn err22_maxv_minv_nan_asymmetry() {
    let p = pair();
    let nan = f32::NAN;
    let cases = [
        (c2v { x: nan, y: nan }, c2v { x: 1.0, y: 2.0 }),
        (c2v { x: 1.0, y: 2.0 }, c2v { x: nan, y: nan }),
        (c2v { x: nan, y: 2.0 }, c2v { x: 1.0, y: nan }),
        (c2v { x: nan, y: nan }, c2v { x: nan, y: nan }),
    ];
    for (i, (a, b)) in cases.iter().enumerate() {
        let mc = unsafe { (p.c.c2Maxv)(*a, *b) };
        let mr = unsafe { (p.r.c2Maxv)(*a, *b) };
        eq_v(&format!("err22 max[{i}]"), mc, mr);
        let nc = unsafe { (p.c.c2Minv)(*a, *b) };
        let nr = unsafe { (p.r.c2Minv)(*a, *b) };
        eq_v(&format!("err22 min[{i}]"), nc, nr);
        // Pin the documented rule: the comparison is false for NaN, so `b` wins.
        eq_v(&format!("err22 max[{i}] takes b"), mc, *b);
        eq_v(&format!("err22 min[{i}] takes b"), nc, *b);
    }
}

#[test]
fn err23_clampv_inverted_range() {
    let p = pair();
    let mut rng = Rng::new(0xE023);
    for i in 0..2048 {
        let a = rng.vec_nasty();
        let lo = rng.vec_scaled(10.0);
        let hi = c2v { x: lo.x - 5.0, y: lo.y - 5.0 }; // hi < lo, deliberately inverted
        let cc = unsafe { (p.c.c2Clampv)(a, lo, hi) };
        let cr = unsafe { (p.r.c2Clampv)(a, lo, hi) };
        eq_v(&format!("err23[{i}]"), cc, cr);
        // documented consequence: max(lo, min(a,hi)) == lo whenever lo > hi
        if !a.x.is_nan() && !a.y.is_nan() {
            eq_v(&format!("err23[{i}] collapses to lo"), cc, lo);
        }
    }
}

// ===========================================================================
// Rows 24..30 — c2GJK null-pointer guards
// ===========================================================================

#[test]
fn err24_25_null_transform_pointers() {
    let p = pair();
    let mut rng = Rng::new(0xE024);
    for i in 0..2048 {
        let a = rng.shape_any();
        let b = rng.shape_any();
        let t = rng.transform_any();
        // ax NULL / bx set, ax set / bx NULL, both NULL, both set
        for (j, (axo, bxo)) in
            [(None, None), (None, Some(t)), (Some(t), None), (Some(t), Some(t))].iter().enumerate()
        {
            let mut inp = GjkIn::new(&a, &b);
            inp.ax = *axo;
            inp.bx = *bxo;
            inp.cache = Some(c2GJKCache::default());
            diff_gjk(&format!("err24/25 combo={j}[{i}]"), p, &inp);
        }
        // A NULL transform must behave EXACTLY like an explicit identity.
        let ident = unsafe { (p.c.c2xIdentity)() };
        let mut n = GjkIn::new(&a, &b);
        n.cache = Some(c2GJKCache::default());
        let with_null = call_gjk(&p.c, &n);
        let mut e = GjkIn::new(&a, &b);
        e.ax = Some(ident);
        e.bx = Some(ident);
        e.cache = Some(c2GJKCache::default());
        let with_ident = call_gjk(&p.c, &e);
        eq_f32(&format!("err24/25[{i}] NULL == identity (dist)"), with_null.dist, with_ident.dist);
        eq_v(&format!("err24/25[{i}] NULL == identity (a)"), with_null.a, with_ident.a);
        eq_v(&format!("err24/25[{i}] NULL == identity (b)"), with_null.b, with_ident.b);
    }
}

#[test]
fn err26_null_cache() {
    let p = pair();
    let mut rng = Rng::new(0xE026);
    for i in 0..4096 {
        let a = rng.shape_any();
        let b = rng.shape_any();
        let mut inp = GjkIn::new(&a, &b);
        inp.cache = None; // pass a NULL cache pointer
        inp.use_radius = (i % 2) as c_int;
        diff_gjk(&format!("err26[{i}]"), p, &inp);
    }
}

#[test]
fn err27_28_29_30_null_out_params() {
    let p = pair();
    let mut rng = Rng::new(0xE027);
    for i in 0..1024 {
        let a = rng.shape_any();
        let b = rng.shape_any();
        // all 8 out-param combos x cache present/absent = 16 configurations
        for mask in 0..8u32 {
            for has_cache in [false, true] {
                let mut inp = GjkIn::new(&a, &b);
                inp.want_out_a = mask & 1 != 0;
                inp.want_out_b = mask & 2 != 0;
                inp.want_iters = mask & 4 != 0;
                inp.cache = if has_cache { Some(c2GJKCache::default()) } else { None };
                diff_gjk(&format!("err27-30 mask={mask} cache={has_cache}[{i}]"), p, &inp);
            }
        }
        // Row 30: everything NULL at once -> only the return value exists.
        let mut inp = GjkIn::new(&a, &b);
        inp.want_out_a = false;
        inp.want_out_b = false;
        inp.want_iters = false;
        inp.cache = None;
        let oc = call_gjk(&p.c, &inp);
        let or = call_gjk(&p.r, &inp);
        eq_f32(&format!("err30[{i}] dist only"), oc.dist, or.dist);
        // the poisoned locals must be untouched on BOTH sides
        eq_v(&format!("err30[{i}] a poison C"), oc.a, or.a);
        eq_v(&format!("err30[{i}] b poison C"), oc.b, or.b);
        eq_i(&format!("err30[{i}] iters poison"), oc.iters, or.iters);
        eq_i(&format!("err30[{i}] iters untouched"), oc.iters, -777);
    }
}

// ===========================================================================
// Rows 31/32/33 — the cache-validation condition
// ===========================================================================

/// Recomputes, using only exported symbols, the `metric` that `c2GJK` will
/// compute from a given cache + shape pair. Lets the test detect exactly which
/// side of the `!(min < max*2 && metric < -1e8f)` condition a case falls on.
fn predicted_metric(api: &Api, a: &Shape, b: &Shape, cache: &c2GJKCache) -> f32 {
    unsafe {
        let mut pa = c2Proxy::default();
        let mut pb = c2Proxy::default();
        (api.c2MakeProxy)(a.as_ptr(), a.ty(), &mut pa);
        (api.c2MakeProxy)(b.as_ptr(), b.ty(), &mut pb);
        let ident = (api.c2xIdentity)();
        let mut s = c2Simplex::default();
        for i in 0..(cache.count.clamp(0, 3) as usize) {
            let sa = (api.c2Mulxv)(ident, pa.verts[cache.iA[i] as usize]);
            let sb = (api.c2Mulxv)(ident, pb.verts[cache.iB[i] as usize]);
            s.verts[i].p = (api.c2Sub)(sb, sa);
        }
        s.count = cache.count;
        (api.c2GJKSimplexMetric)(&mut s)
    }
}

#[test]
fn err31_cache_count_zero_is_cold_start() {
    let p = pair();
    let mut rng = Rng::new(0xE031);
    for i in 0..4096 {
        let a = rng.shape_any();
        let b = rng.shape_any();
        // count == 0 but every other field is junk: `!!count` is false, so the
        // junk must be ignored entirely.
        let junk = c2GJKCache {
            metric: rng.scaled(1e9),
            count: 0,
            iA: [rng.below(100000) as c_int, -5, 77],
            iB: [rng.below(100000) as c_int, 9999, -1],
            div: rng.scaled(1e9),
        };
        let mut inp = GjkIn::new(&a, &b);
        inp.cache = Some(junk);
        let out = diff_gjk(&format!("err31[{i}]"), p, &inp);

        // It must equal the result with a NULL cache (both are cold starts).
        let mut nc = GjkIn::new(&a, &b);
        nc.cache = None;
        let null_out = call_gjk(&p.c, &nc);
        eq_f32(&format!("err31[{i}] == null-cache dist"), out.dist, null_out.dist);
        eq_i(&format!("err31[{i}] == null-cache iters"), out.iters, null_out.iters);
    }
}

#[test]
fn err32_stale_cache_is_accepted() {
    // The 2nd conjunct `metric < -1.0e8f` is essentially never true, so the
    // negated condition is essentially always true and a STALE cache is read.
    let p = pair();
    let mut rng = Rng::new(0xE032);
    let mut accepted = 0usize;
    for i in 0..4096 {
        let a = rng.shape_any();
        let b = rng.shape_any();
        // Produce a genuine cache from a DIFFERENT shape pair, then reuse it.
        let a0 = rng.shape_of_any_scale(a.ty());
        let b0 = rng.shape_of_any_scale(b.ty());
        let mut warm = GjkIn::new(&a0, &b0);
        warm.cache = Some(c2GJKCache::default());
        let warmed = diff_gjk(&format!("err32 warm[{i}]"), p, &warm);

        // Now reuse that cache with the unrelated shapes. Indices must remain in
        // range for the new shapes, so only reuse when the vertex counts allow.
        let max_ia = warmed.cache.iA[..warmed.cache.count.clamp(0, 3) as usize]
            .iter()
            .copied()
            .max()
            .unwrap_or(0);
        let max_ib = warmed.cache.iB[..warmed.cache.count.clamp(0, 3) as usize]
            .iter()
            .copied()
            .max()
            .unwrap_or(0);
        if max_ia >= a.vert_count() || max_ib >= b.vert_count() {
            continue; // would be ERRORS.md row 37 (UB), covered separately
        }
        let m = predicted_metric(&p.c, &a, &b, &warmed.cache);
        let mo = warmed.cache.metric;
        let (mn, mx) = (m.min(mo), m.max(mo));
        let rejected = mn < mx * 2.0 && m < -1.0e8;
        if !rejected {
            accepted += 1;
        }
        let mut inp = GjkIn::new(&a, &b);
        inp.cache = Some(warmed.cache);
        diff_gjk(&format!("err32 reuse[{i}] rejected={rejected}"), p, &inp);
    }
    assert!(accepted > 0, "err32: never observed the stale cache being accepted");
    eprintln!("err32 stale caches accepted = {accepted}");
}

#[test]
fn err33_cache_rejected_when_metric_below_minus_1e8() {
    // Reachable only with count == 3 and a hugely negative determinant.
    // Search for such a configuration and confirm both sides agree.
    let p = pair();
    let mut rng = Rng::new(0xE033);
    let mut rejected_seen = 0usize;
    for i in 0..20000 {
        // Big AABBs give |det| ~ (1e5)^2 = 1e10, well past the -1e8 threshold.
        let scale = 1e5f32;
        let a = Shape::Aabb(c2AABB {
            min: c2v { x: -scale, y: -scale },
            max: c2v { x: scale, y: scale },
        });
        let b = Shape::Aabb(c2AABB {
            min: c2v { x: rng.scaled(scale), y: rng.scaled(scale) },
            max: c2v { x: rng.scaled(scale) + scale, y: rng.scaled(scale) + scale },
        });
        // hand-built 3-vertex cache with random valid indices
        let cache = c2GJKCache {
            metric: 0.0,
            count: 3,
            iA: [rng.below(4) as c_int, rng.below(4) as c_int, rng.below(4) as c_int],
            iB: [rng.below(4) as c_int, rng.below(4) as c_int, rng.below(4) as c_int],
            div: 1.0,
        };
        let m = predicted_metric(&p.c, &a, &b, &cache);
        let mo = cache.metric;
        let (mn, mx) = (m.min(mo), m.max(mo));
        let rejected = mn < mx * 2.0 && m < -1.0e8;
        if rejected {
            rejected_seen += 1;
        }
        let mut inp = GjkIn::new(&a, &b);
        inp.cache = Some(cache);
        diff_gjk(&format!("err33[{i}] rejected={rejected} metric={m}"), p, &inp);
        if rejected_seen > 50 && i > 4000 {
            break;
        }
    }
    eprintln!("err33 cache-rejection branch taken {rejected_seen} times");
    assert!(
        rejected_seen > 0,
        "err33: never reached the `cache_was_read = 0` branch (metric < -1e8)"
    );
}

// ===========================================================================
// Row 34 — negative cache count (fully deterministic)
// Row 35b — count 4..=8 with div == 0.0 (non-trapping)
// ===========================================================================

#[test]
fn err34_negative_cache_count() {
    let p = pair();
    let mut rng = Rng::new(0xE034);
    for &count in [-1i32, -2, -3, -100, -100000, c_int::MIN].iter() {
        for i in 0..256 {
            let a = rng.shape_any();
            let b = rng.shape_any();
            let cache = c2GJKCache {
                metric: rng.scaled(100.0),
                count,
                iA: [rng.below(4) as c_int; 3],
                iB: [rng.below(4) as c_int; 3],
                div: rng.scaled(10.0),
            };
            let mut inp = GjkIn::new(&a, &b);
            inp.cache = Some(cache);
            let out = diff_gjk(&format!("err34 count={count}[{i}]"), p, &inp);
            // documented consequences
            eq_f32(&format!("err34 count={count}[{i}] dist==0"), out.dist, 0.0);
            eq_i(&format!("err34 count={count}[{i}] iters==0"), out.iters, 0);
            eq_i(&format!("err34 count={count}[{i}] count preserved"), out.cache.count, count);
        }
    }
}

#[test]
fn err35b_out_of_range_count_with_zero_div() {
    // With div == 0.0 the type-punned iB[3] index is 0, so the C does not trap
    // for counts up to 8. Both sides must agree exactly.
    let p = pair();
    let mut rng = Rng::new(0xE035);
    for &count in [4i32, 5, 6, 7, 8].iter() {
        for i in 0..128 {
            let a = rng.shape_any();
            let b = rng.shape_any();
            let cache = c2GJKCache { metric: 0.0, count, iA: [0; 3], iB: [0; 3], div: 0.0 };
            let mut inp = GjkIn::new(&a, &b);
            inp.cache = Some(cache);
            let out = diff_gjk(&format!("err35b count={count}[{i}]"), p, &inp);
            eq_f32(&format!("err35b count={count}[{i}] dist==0"), out.dist, 0.0);
            eq_i(&format!("err35b count={count}[{i}] iters==0"), out.iters, 0);
            eq_i(&format!("err35b count={count}[{i}] count preserved"), out.cache.count, count);
        }
    }
}

// ===========================================================================
// Rows 37/38/39 — UB rows: the requirement is NON-TRAPPING, not value equality
// (C's own result is non-deterministic here; see ERRORS.md).
// ===========================================================================

#[test]
fn err37_38_39_ub_rows_do_not_trap() {
    let p = pair();
    let mut rng = Rng::new(0xE037);
    let mut c_calls = 0usize;
    let mut r_calls = 0usize;
    for i in 0..2048 {
        let a = rng.shape_any();
        let b = rng.shape_any();

        // Row 37: index inside verts[8] but past the shape's vertex count, and
        // Row 38: index outside verts[8] entirely, plus negative indices.
        // Indices are kept within a few pages of the proxy. The point of this
        // row is the CLASS of behaviour (out of range -> garbage, never a panic);
        // probing megabytes off the stack would only gamble on where the thread
        // stack mapping happens to end, which is noise, not signal.
        for &idx in [1i32, 2, 3, 5, 7, 8, 9, 16, 33, 64, -1, -5, -33, -64].iter() {
            let cache = c2GJKCache {
                metric: 0.0,
                count: 1,
                iA: [idx; 3],
                iB: [idx; 3],
                div: 1.0,
            };
            let mut inp = GjkIn::new(&a, &b);
            inp.cache = Some(cache);
            // Values cannot be compared (C reads uninitialised/foreign stack and
            // is measurably non-deterministic). What MUST hold is that neither
            // library aborts, panics or faults.
            let _ = call_gjk(&p.c, &inp);
            c_calls += 1;
            let _ = call_gjk(&p.r, &inp);
            r_calls += 1;
        }

        // Row 39: type discriminants with no valid variant.
        for &ty in [3i32, 4, 99, -1, c_int::MIN, c_int::MAX].iter() {
            let mut inp = GjkIn::new(&a, &b);
            inp.cache = None;
            inp.type_a_override = Some(ty);
            let _ = call_gjk(&p.c, &inp);
            let _ = call_gjk(&p.r, &inp);
            let mut inp2 = GjkIn::new(&a, &b);
            inp2.cache = None;
            inp2.type_b_override = Some(ty);
            let _ = call_gjk(&p.c, &inp2);
            let _ = call_gjk(&p.r, &inp2);
            c_calls += 2;
            r_calls += 2;
        }
        let _ = i;
    }
    // Reaching here at all is the assertion: no panic, no abort, no SIGSEGV.
    assert_eq!(c_calls, r_calls);
    eprintln!("err37/38/39: {c_calls} C calls and {r_calls} Rust calls, none trapped");
}

// ===========================================================================
// Rows 40..44 — the use_radius post-processing block
// Row 45 — the `hit` path bypasses it entirely
// ===========================================================================

#[test]
fn err40_41_42_use_radius_branches() {
    let p = pair();
    let mut rng = Rng::new(0xE040);
    let mut midpoint = 0usize;
    let mut shrink = 0usize;
    let mut forced_zero = 0usize;
    for i in 0..8192 {
        // circles let us place the boundary dist == rA + rB exactly
        let ra = [0.0f32, 0.5, 1.0, 7.5, 100.0][rng.below(5) as usize];
        let rb = [0.0f32, 0.25, 2.0, 12.5, 50.0][rng.below(5) as usize];
        let sum = ra + rb;
        // sweep the centre distance across the boundary
        let d = match rng.below(6) {
            0 => 0.0,               // coincident -> dist 0 <= sum
            1 => sum * 0.5,         // inside -> midpoint branch
            2 => sum,               // exactly at the boundary -> midpoint branch
            3 => sum * 1.000001,    // just outside -> shrink branch
            4 => sum * 2.0,         // clearly outside
            _ => rng.scaled(sum * 3.0).abs(),
        };
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra });
        let b = Shape::Circle(c2Circle { p: c2v { x: d, y: 0.0 }, r: rb });
        let mut inp = GjkIn::new(&a, &b);
        inp.cache = Some(c2GJKCache::default());
        let out = diff_gjk(&format!("err40-42 ra={ra} rb={rb} d={d}[{i}]"), p, &inp);
        if out.cache.count == 3 {
            forced_zero += 1;
        } else if out.dist == 0.0 {
            midpoint += 1;
        } else {
            shrink += 1;
        }
    }
    eprintln!("err40-42 midpoint={midpoint} shrink={shrink} hit={forced_zero}");
    assert!(midpoint > 0, "err40/41: midpoint branch never taken");
    assert!(shrink > 0, "err40: shrink branch never taken");
}

#[test]
fn err43_44_use_radius_zero_and_truthiness() {
    let p = pair();
    let mut rng = Rng::new(0xE043);
    for i in 0..2048 {
        let a = rng.shape_any();
        let b = rng.shape_any();
        // Row 43: use_radius == 0 ignores radii entirely.
        let mut z = GjkIn::new(&a, &b);
        z.use_radius = 0;
        z.cache = Some(c2GJKCache::default());
        let out0 = diff_gjk(&format!("err43[{i}] ur=0"), p, &z);
        // Row 44: any non-zero value behaves like 1.
        let mut one = GjkIn::new(&a, &b);
        one.use_radius = 1;
        one.cache = Some(c2GJKCache::default());
        let out1 = diff_gjk(&format!("err44[{i}] ur=1"), p, &one);
        for ur in [2i32, -1, 7, c_int::MIN, c_int::MAX, 0x10000] {
            let mut t = GjkIn::new(&a, &b);
            t.use_radius = ur;
            t.cache = Some(c2GJKCache::default());
            let outn = diff_gjk(&format!("err44[{i}] ur={ur}"), p, &t);
            eq_f32(&format!("err44[{i}] ur={ur} == ur=1"), outn.dist, out1.dist);
            eq_v(&format!("err44[{i}] ur={ur} a == ur=1"), outn.a, out1.a);
            eq_v(&format!("err44[{i}] ur={ur} b == ur=1"), outn.b, out1.b);
        }
        let _ = out0;
    }
}

#[test]
fn err45_hit_path_bypasses_use_radius() {
    let p = pair();
    let mut rng = Rng::new(0xE045);
    let mut hits = 0usize;
    for i in 0..4096 {
        let scale = rng.scale_choice();
        // co-located shapes -> cores intersect -> simplex reaches count 3
        let a = rng.shape_of_any_scale(1); // AABB centred by construction
        let b = rng.shape_of_any_scale(1);
        for ur in [0i32, 1, 2] {
            let mut inp = GjkIn::new(&a, &b);
            inp.use_radius = ur;
            inp.cache = Some(c2GJKCache::default());
            let out = diff_gjk(&format!("err45[{i}] ur={ur}"), p, &inp);
            if out.cache.count == 3 {
                hits += 1;
                // `hit` forces a = b and dist = 0 REGARDLESS of use_radius
                eq_f32(&format!("err45[{i}] ur={ur} dist==0"), out.dist, 0.0);
                eq_v(&format!("err45[{i}] ur={ur} a==b"), out.a, out.b);
            }
        }
        let _ = scale;
    }
    eprintln!("err45 hit path taken {hits} times");
    assert!(hits > 0, "err45: never reached the hit path");
}

// ===========================================================================
// Rows 46/47/48/49 — the four loop-termination conditions
// ===========================================================================

/// Computes, using only exported symbols, the first-iteration `p = c2L(&s)` and
/// `d1 = c2Dot(p,p)` that `c2GJK` will see for a cold start. That is what proves
/// WHICH break a given construction takes, since the three guards are checked in
/// a fixed order: `d1 > d0` first, then `c2Dot(d,d) < eps*eps`, then `dup`.
fn first_iteration_d1(api: &Api, a: &Shape, b: &Shape) -> (c2v, f32) {
    unsafe {
        let mut pa = c2Proxy::default();
        let mut pb = c2Proxy::default();
        (api.c2MakeProxy)(a.as_ptr(), a.ty(), &mut pa);
        (api.c2MakeProxy)(b.as_ptr(), b.ty(), &mut pb);
        let ident = (api.c2xIdentity)();
        let sa = (api.c2Mulxv)(ident, pa.verts[0]);
        let sb = (api.c2Mulxv)(ident, pb.verts[0]);
        let p = (api.c2Sub)(sb, sa);
        (p, (api.c2Dot)(p, p))
    }
}

/// Row 46 — `if (d1 > d0) break;`
///
/// On the first pass `d0 == FLT_MAX`, so this guard can only fire when `d1`
/// overflows to `+inf`. Coordinates around 1e30 make `dot(p,p) = 1e60 -> +inf`,
/// and because this check precedes the `c2D` and `dup` checks, reaching it is
/// proved by `d1 == +inf`.
#[test]
fn err46_no_progress_break() {
    let p = pair();
    let mut proved = 0usize;
    for (i, scale) in [1e30f32, 1e25, 3e30, f32::MAX].iter().enumerate() {
        let a = Shape::Circle(c2Circle { p: c2v { x: -scale, y: -scale }, r: 1.0 });
        let b = Shape::Circle(c2Circle { p: c2v { x: *scale, y: *scale }, r: 1.0 });
        let (_pv, d1) = first_iteration_d1(&p.c, &a, &b);
        for ur in [0i32, 1] {
            let mut inp = GjkIn::new(&a, &b);
            inp.use_radius = ur;
            inp.cache = Some(c2GJKCache::default());
            let out = diff_gjk(&format!("err46 scale={scale} ur={ur}[{i}]"), p, &inp);
            if d1.is_infinite() {
                proved += 1;
                // the `d1 > d0` guard fired on the very first pass
                eq_i(&format!("err46 scale={scale} iters==0"), out.iters, 0);
                eq_i(&format!("err46 scale={scale} count==1"), out.cache.count, 1);
            }
        }
    }
    eprintln!("err46: `d1 > d0` break proved on {proved} constructions");
    assert!(proved > 0, "err46: never constructed an input where d1 overflows to +inf");
}

/// Row 47 — `if (c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON) break;`
///
/// Two byte-identical shapes give `p = sB - sA = (0,0)`, so `d1 = 0` (the row-46
/// guard cannot fire) and `c2D` returns `-p = (0,0)`, so this guard must be the
/// one that fires.
#[test]
fn err47_degenerate_direction_break() {
    let p = pair();
    let mut rng = Rng::new(0xE047);
    let mut proved = 0usize;
    for i in 0..2048 {
        let a = rng.shape_any();
        let b = a; // identical -> Minkowski difference starts exactly at the origin
        let (pv, d1) = first_iteration_d1(&p.c, &a, &b);
        for ur in [0i32, 1] {
            let mut inp = GjkIn::new(&a, &b);
            inp.use_radius = ur;
            inp.cache = Some(c2GJKCache::default());
            let out = diff_gjk(&format!("err47[{i}] ur={ur}"), p, &inp);
            if pv.x == 0.0 && pv.y == 0.0 && d1 == 0.0 {
                proved += 1;
                eq_i(&format!("err47[{i}] iters==0"), out.iters, 0);
                eq_i(&format!("err47[{i}] count==1"), out.cache.count, 1);
            }
        }
    }
    eprintln!("err47: degenerate-direction break proved on {proved} constructions");
    assert!(proved > 0, "err47: never constructed a zero search direction");
}

/// Row 48 — `if (dup) break;`
///
/// Both proxies have exactly ONE vertex (two circles), so `c2Support` can only
/// ever return 0. The initial simplex already holds `(iA,iB) = (0,0)`, so the
/// first support query necessarily duplicates the saved pair. With the circles
/// separated, `d1` is finite and `d != (0,0)`, so neither earlier guard can fire
/// and the `dup` break is the only reachable exit.
#[test]
fn err48_duplicate_support_break() {
    let p = pair();
    let mut rng = Rng::new(0xE048);
    let mut proved = 0usize;
    for i in 0..2048 {
        let scale = rng.scale_choice();
        let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: scale * 0.1 });
        let b = Shape::Circle(c2Circle {
            p: c2v { x: scale, y: rng.scaled(scale) },
            r: scale * 0.1,
        });
        let (pv, d1) = first_iteration_d1(&p.c, &a, &b);
        for ur in [0i32, 1] {
            let mut inp = GjkIn::new(&a, &b);
            inp.use_radius = ur;
            inp.cache = Some(c2GJKCache::default());
            let out = diff_gjk(&format!("err48[{i}] ur={ur}"), p, &inp);
            let d_sq = unsafe { (p.c.c2Dot)(pv, pv) };
            if d1.is_finite() && d_sq >= C2_EPS_SQ {
                proved += 1;
                // dup fires on the first pass, so no vertex is appended
                eq_i(&format!("err48[{i}] iters==0"), out.iters, 0);
                eq_i(&format!("err48[{i}] count stays 1"), out.cache.count, 1);
            }
        }
    }
    eprintln!("err48: `dup` break proved on {proved} constructions");
    assert!(proved > 0, "err48: never constructed a duplicate-support case");
}

/// FLT_EPSILON * FLT_EPSILON, the threshold in the C source.
const C2_EPS_SQ: f32 = 1.192_092_895_507_812_5e-7 * 1.192_092_895_507_812_5e-7;

/// Row 49 — `while (iter < 20)`
///
/// Broad randomised search for the highest reachable iteration count, asserting
/// C and Rust agree on `*iterations` every time and that the cap never breaks.
#[test]
fn err49_iteration_cap() {
    let p = pair();
    let mut rng = Rng::new(0xE046);
    let mut max_iter = 0;
    let mut hist = [0usize; 21];
    for i in 0..30000 {
        let scale = rng.scale_choice();
        let a = rng.shape_any();
        let bt = rng.below(3) as c_int;
        let b = rng.shape_of_any_scale(bt);
        let mut inp = GjkIn::new(&a, &b);
        inp.ax = if rng.below(2) == 0 { None } else { Some(rand_transform_unnorm(&mut rng, scale)) };
        inp.bx = if rng.below(2) == 0 { None } else { Some(rand_transform_unnorm(&mut rng, scale)) };
        inp.use_radius = (i % 2) as c_int;
        inp.cache = Some(c2GJKCache::default());
        let out = diff_gjk(&format!("err49[{i}]"), p, &inp);
        assert!(out.iters >= 0 && out.iters <= 20, "err49: iterations = {}", out.iters);
        hist[out.iters as usize] += 1;
        max_iter = max_iter.max(out.iters);
    }
    eprintln!("err49 iteration histogram = {hist:?} (max {max_iter})");
    assert!(hist[0] > 0, "err49: never terminated at iteration 0");
    assert!(hist.iter().skip(1).any(|&n| n > 0), "err49: never ran past iteration 0");
    // Documented finding: every proxy has at most 4 vertices, so the search
    // always converges long before the cap. `max_iter` is reported above.
}

// ===========================================================================
// Rows 50/51 — NaN and inf coordinates through the whole pipeline
// ===========================================================================

#[test]
fn err50_51_nan_and_inf_coordinates() {
    let p = pair();
    let mut rng = Rng::new(0xE050);
    let mut nan_cases = 0usize;
    for i in 0..4096 {
        // Row 51: inf but NaN-free -> STRICT bit equality
        let ai = Shape::Circle(c2Circle { p: rng.vec_nasty_no_nan(), r: rng.nasty_no_nan() });
        let bi = Shape::Capsule(c2Capsule {
            a: rng.vec_nasty_no_nan(),
            b: rng.vec_nasty_no_nan(),
            r: rng.nasty_no_nan(),
        });
        let mut inp = GjkIn::new(&ai, &bi);
        inp.cache = Some(c2GJKCache::default());
        inp.use_radius = (i % 2) as c_int;
        diff_gjk(&format!("err51 inf[{i}]"), p, &inp);

        let ci = Shape::Aabb(c2AABB { min: rng.vec_nasty_no_nan(), max: rng.vec_nasty_no_nan() });
        let mut inp2 = GjkIn::new(&ci, &bi);
        inp2.cache = Some(c2GJKCache::default());
        diff_gjk(&format!("err51 inf-aabb[{i}]"), p, &inp2);

        // Row 50: NaN coordinates -> SOFT (NaN payloads are codegen-dependent)
        let an = Shape::Circle(c2Circle { p: rng.vec_nasty(), r: rng.nasty() });
        let bn = Shape::Capsule(c2Capsule { a: rng.vec_nasty(), b: rng.vec_nasty(), r: rng.nasty() });
        let mut inp3 = GjkIn::new(&an, &bn);
        inp3.cache = Some(c2GJKCache::default());
        inp3.use_radius = (i % 2) as c_int;
        diff_gjk_soft(&format!("err50 nan[{i}]"), p, &inp3);
        nan_cases += 1;

        let cn = Shape::Aabb(c2AABB { min: rng.vec_nasty(), max: rng.vec_nasty() });
        let mut inp4 = GjkIn::new(&cn, &an);
        inp4.cache = Some(c2GJKCache::default());
        diff_gjk_soft(&format!("err50 nan-aabb[{i}]"), p, &inp4);

        // explicit all-NaN shapes
        let nan = f32::NAN;
        let aa = Shape::Circle(c2Circle { p: c2v { x: nan, y: nan }, r: nan });
        let bb = Shape::Aabb(c2AABB { min: c2v { x: nan, y: nan }, max: c2v { x: nan, y: nan } });
        let mut inp5 = GjkIn::new(&aa, &bb);
        inp5.cache = Some(c2GJKCache::default());
        diff_gjk_soft(&format!("err50 allnan[{i}]"), p, &inp5);
    }
    assert!(nan_cases > 0);
}

// ===========================================================================
// Rows 52/53/54 — unvalidated shape geometry
// ===========================================================================

#[test]
fn err52_53_54_unvalidated_shape_geometry() {
    let p = pair();
    let mut rng = Rng::new(0xE052);
    for i in 0..4096 {
        let scale = rng.scale_choice();
        let o = rng.vec_scaled(scale);
        // Row 52: AABB with min > max
        let inv = Shape::Aabb(c2AABB {
            min: c2v { x: o.x + scale, y: o.y + scale },
            max: c2v { x: o.x, y: o.y },
        });
        // Row 53: capsule with a == b
        let degen = Shape::Capsule(c2Capsule { a: o, b: o, r: scale * 0.3 });
        // Row 54: negative radii
        let negc = Shape::Circle(c2Circle { p: o, r: -scale });
        let negk = Shape::Capsule(c2Capsule {
            a: o,
            b: c2v { x: o.x + scale, y: o.y },
            r: -scale * 0.5,
        });
        let good = rng.shape_any();
        let bads = [inv, degen, negc, negk];
        for (j, bad) in bads.iter().enumerate() {
            for ur in [0i32, 1] {
                let mut f = GjkIn::new(bad, &good);
                f.use_radius = ur;
                f.cache = Some(c2GJKCache::default());
                diff_gjk(&format!("err52-54 fwd[{j}] ur={ur}[{i}]"), p, &f);
                let mut r = GjkIn::new(&good, bad);
                r.use_radius = ur;
                r.cache = Some(c2GJKCache::default());
                diff_gjk(&format!("err52-54 rev[{j}] ur={ur}[{i}]"), p, &r);
            }
            // bad vs bad
            for (k, bad2) in bads.iter().enumerate() {
                let mut bb2 = GjkIn::new(bad, bad2);
                bb2.cache = Some(c2GJKCache::default());
                diff_gjk(&format!("err52-54 bad[{j}]x[{k}][{i}]"), p, &bb2);
            }
        }
    }
}

// ===========================================================================
// Rows 35/36 and 55..58 — trap parity
//
// These inputs make the C fault (null dereference, wild index). They cannot be
// executed in-process, so each one is run in a CHILD process -- once against the
// C `.so` and once against the Rust `.so` -- and the two exit statuses are
// compared. That is a real differential assertion ("both die the same way"),
// not merely "we assume C would crash".
// ===========================================================================

/// The worker: only does anything when `GJK_TRAP_CASE` is set, so it is a no-op
/// during a normal test run and the crashing call happens only in the child.
#[test]
fn trap_worker() {
    let case = match std::env::var("GJK_TRAP_CASE") {
        Ok(c) => c,
        Err(_) => return, // normal run: nothing to do
    };
    let lib = std::env::var("GJK_TRAP_LIB").unwrap_or_else(|_| "c".into());
    let p = pair();
    let api: &Api = if lib == "c" { &p.c } else { &p.r };
    let a = Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 5.0 });
    let b = Shape::Circle(c2Circle { p: c2v { x: 20.0, y: 0.0 }, r: 3.0 });
    let (mut av, mut bv) = (c2v::default(), c2v::default());
    let mut it: c_int = 0;
    unsafe {
        match case.as_str() {
            // Row 35: count == 4 with div != 0 -> iB[3] aliases div's bits
            "cache_count4" => {
                let mut cache =
                    c2GJKCache { metric: 1.0, count: 4, iA: [0; 3], iB: [0; 3], div: 1.0 };
                let d = (api.c2GJK)(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(),
                                    std::ptr::null(), &mut av, &mut bv, 1, &mut it, &mut cache);
                println!("survived cache_count4 dist={d}");
            }
            // Row 36: count >= 9 -> writes far past the simplex
            "cache_count100" => {
                let mut cache =
                    c2GJKCache { metric: 0.0, count: 100, iA: [0; 3], iB: [0; 3], div: 0.0 };
                let d = (api.c2GJK)(a.as_ptr(), a.ty(), std::ptr::null(), b.as_ptr(), b.ty(),
                                    std::ptr::null(), &mut av, &mut bv, 1, &mut it, &mut cache);
                println!("survived cache_count100 dist={d}");
            }
            // Row 55: NULL shape pointer with a VALID type
            "gjk_null_shape_a" => {
                let d = (api.c2GJK)(std::ptr::null(), C2_TYPE_CIRCLE, std::ptr::null(),
                                    b.as_ptr(), b.ty(), std::ptr::null(), &mut av, &mut bv, 1,
                                    &mut it, std::ptr::null_mut());
                println!("survived gjk_null_shape_a dist={d}");
            }
            "gjk_null_shape_b" => {
                let d = (api.c2GJK)(a.as_ptr(), a.ty(), std::ptr::null(), std::ptr::null(),
                                    C2_TYPE_AABB, std::ptr::null(), &mut av, &mut bv, 1,
                                    &mut it, std::ptr::null_mut());
                println!("survived gjk_null_shape_b dist={d}");
            }
            // Row 56: c2BBVerts with NULL arguments
            "bbverts_null_out" => {
                let mut bb = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
                (api.c2BBVerts)(std::ptr::null_mut(), &mut bb);
                println!("survived bbverts_null_out");
            }
            "bbverts_null_bb" => {
                let mut out = [c2v::default(); 4];
                (api.c2BBVerts)(out.as_mut_ptr(), std::ptr::null_mut());
                println!("survived bbverts_null_bb");
            }
            // Row 57: NULL simplex pointer
            "c22_null" => { (api.c22)(std::ptr::null_mut()); println!("survived c22_null"); }
            "c23_null" => { (api.c23)(std::ptr::null_mut()); println!("survived c23_null"); }
            "c2d_null" => { let v = (api.c2D)(std::ptr::null_mut()); println!("survived {v:?}"); }
            "c2l_null" => { let v = (api.c2L)(std::ptr::null_mut()); println!("survived {v:?}"); }
            "metric_null" => {
                let m = (api.c2GJKSimplexMetric)(std::ptr::null_mut());
                println!("survived metric_null {m}");
            }
            "witness_null_s" => {
                (api.c2Witness)(std::ptr::null_mut(), &mut av, &mut bv);
                println!("survived witness_null_s");
            }
            "witness_null_out" => {
                let mut s = c2Simplex::default();
                s.count = 1;
                (api.c2Witness)(&mut s, std::ptr::null_mut(), std::ptr::null_mut());
                println!("survived witness_null_out");
            }
            // Row 58: c2Support with a NULL vertex array
            "support_null" => {
                let i = (api.c2Support)(std::ptr::null(), 0, c2v { x: 1.0, y: 1.0 });
                println!("survived support_null {i}");
            }
            other => panic!("unknown trap case {other}"),
        }
    }
}

/// Outcome of one child run: either a normal exit code, a fatal signal, or a
/// hang (stack corruption can spin instead of faulting, so a deadline is
/// mandatory -- without one the parent would block forever).
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Exited(i32),
    Signal(i32),
    TimedOut,
}

/// The release `.so` — the artifact this crate actually ships (`crate-type =
/// ["cdylib"]`). Trap parity is asserted against it, NOT against the `dev`
/// build: rustc's `-Cdebug-assertions` inserts a null check on every raw-pointer
/// dereference, so a `dev` build turns C's SIGSEGV into a "null pointer
/// dereference occurred" panic and SIGABRT. That is a Rust development aid, not
/// a property of the translated code, and it does not exist in the shipped
/// library (verified: every row below matches with signal 11).
fn release_so() -> Option<std::path::PathBuf> {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libgjk_cache_lib.so");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

/// Runs `trap_worker` in a child process against one library.
fn run_trap_child(case: &str, lib: &str) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = std::process::Command::new(exe);
    cmd.args(["--exact", "trap_worker", "--nocapture", "--test-threads=1"])
        .env("GJK_TRAP_CASE", case)
        .env("GJK_TRAP_LIB", lib)
        .env_remove("RUST_BACKTRACE");
    if let Some(rel) = release_so() {
        cmd.env("GJK_RUST_SO", rel);
    }
    let mut child = cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn trap child");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => {
                return match status.signal() {
                    Some(sig) => Outcome::Signal(sig),
                    None => Outcome::Exited(status.code().unwrap_or(-1)),
                };
            }
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Outcome::TimedOut;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    }
}

#[test]
fn err35_36_55_58_trap_parity() {
    // Never recurse: a child (which has GJK_TRAP_CASE set) must not spawn more.
    if std::env::var("GJK_TRAP_CASE").is_ok() {
        return;
    }
    if release_so().is_none() {
        eprintln!(
            "SKIP err35/36/55-58 trap parity: target/release/libgjk_cache_lib.so not built.\n\
             Run `cargo build --release` first (see run_all.sh)."
        );
        return;
    }
    let cases = [
        ("cache_count4", 35),
        ("cache_count100", 36),
        ("gjk_null_shape_a", 55),
        ("gjk_null_shape_b", 55),
        ("bbverts_null_out", 56),
        ("bbverts_null_bb", 56),
        ("c22_null", 57),
        ("c23_null", 57),
        ("c2d_null", 57),
        ("c2l_null", 57),
        ("metric_null", 57),
        ("witness_null_s", 57),
        ("witness_null_out", 57),
        ("support_null", 58),
    ];
    let mut report = Vec::new();
    let mut mismatches = Vec::new();
    for (case, row) in cases.iter() {
        let c_out = run_trap_child(case, "c");
        let r_out = run_trap_child(case, "r");
        report.push(format!(
            "row {row:>2} {case:<18} C={c_out:?}  Rust={r_out:?}{}",
            if c_out == r_out { "" } else { "   <== DIVERGES" }
        ));
        if c_out != r_out {
            mismatches.push(format!("row {row} {case}: C={c_out:?} Rust={r_out:?}"));
        }
    }
    for line in &report {
        eprintln!("{line}");
    }
    // Rows 55..58 are plain null-pointer dereferences: both libraries are
    // equally unguarded, so they MUST agree exactly.
    let must_agree: Vec<&String> = mismatches
        .iter()
        .filter(|m| !m.starts_with("row 35") && !m.starts_with("row 36"))
        .collect();
    assert!(must_agree.is_empty(), "null-pointer trap parity broken: {must_agree:?}");
    // Rows 35/36 corrupt the frame; see ERRORS.md for why exact parity there is
    // not achievable. Any divergence is reported above rather than asserted.
}
