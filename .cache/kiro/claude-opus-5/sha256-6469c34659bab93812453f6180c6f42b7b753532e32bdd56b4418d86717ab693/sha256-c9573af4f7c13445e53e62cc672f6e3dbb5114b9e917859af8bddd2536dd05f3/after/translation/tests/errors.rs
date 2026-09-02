//! Phase C — error / rejection-path differential tests, one `#[test]` per row of
//! `ERRORS.md`, plus the generic C-API boundaries.
//!
//! Rows whose trigger is *undefined behaviour* in the C are verified by running
//! the C `.so` in a forked child and observing that it dies on a fatal signal;
//! see `ERRORS.md` § "Deliberate non-reproduction of UB".

mod common;

use std::ffi::c_int;

use common::*;

// ---------------------------------------------------------------------------
// Row 1 — `match`: energy gate rejects.
// ---------------------------------------------------------------------------

#[test]
fn err01_match_energy_gate_rejects() {
    let p = pair();
    let mut rng = Rng::new(0xE001);
    let mut saw_zero = 0usize;
    for it in 0..iters() {
        let bins = rng.range_usize(1, 40);
        // total(test) == 0 and total(reference) > 0 with threshold > 0 makes
        // `0 < threshold * total(reference)` true -> early `return 0`.
        let test = vec![0.0f64; bins];
        let reference: Vec<f64> = (0..bins).map(|_| rng.unit() + 1.0).collect();
        let thr = rng.range_f64(0.25, 4.0);
        let ctx = format!("err01 it={it} bins={bins} thr={thr}");
        diff_match(&p, &ctx, &test, &reference, bins as c_int, thr);
        let mut t = test.clone();
        let mut r = reference.clone();
        let v = unsafe { (p.c.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins as c_int, thr) };
        assert_eq!(v, 0, "{ctx}: C should reject via the energy gate");
        saw_zero += 1;
    }
    assert_eq!(saw_zero, iters(), "every iteration must exercise the rejection");
}

// ---------------------------------------------------------------------------
// Row 2 — `match`: contrast gate rejects.
// ---------------------------------------------------------------------------

#[test]
fn err02_match_contrast_gate_rejects() {
    let p = pair();
    let mut rng = Rng::new(0xE002);
    // Count the iterations that provably rejected at the *contrast* gate (energy
    // gate not taken), so the test cannot silently degenerate into row 1.
    let mut isolated = 0usize;
    for it in 0..iters() {
        let bins = rng.range_usize(2, 40);
        let test = gen_unit_f64(&mut rng, bins);
        let reference = gen_unit_f64(&mut rng, bins);
        let ctx = format!("err02 it={it} bins={bins}");

        // Thresholds that reach the contrast gate but cannot reject there.
        for &thr in &[-0.0f64, -1.0, -1e300] {
            diff_match(&p, &format!("{ctx} thr={thr:?}"), &test, &reference, bins as c_int, thr);
        }

        // MEASURE the contrast the C actually produces for this input by running
        // `match`'s pipeline and calling the low-level `spectral_contrast` export
        // on the reinterpreted buffers, then pick a threshold one ULP above it.
        // (Do not *assume* the contrast is <= 1.0: normalising in `double` and
        // storing back as `float` can round the dot product a ULP above 1.0.)
        let mut tp = preprocess_ref(&test);
        let mut rp = preprocess_ref(&reference);
        let mut a = as_f32_lanes(&mut tp, bins);
        let mut b = as_f32_lanes(&mut rp, bins);
        let contrast =
            unsafe { (p.c.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), bins as c_int) };
        let thr = if contrast.is_nan() {
            // A NaN contrast fails `>=` against anything, so any finite threshold
            // rejects at the contrast gate.
            0.0
        } else {
            next_up(contrast)
        };

        // Is the energy gate provably NOT taken? `total` sums left-to-right in
        // `double`, so it is reproducible bit-exactly here.
        let st = fold_sum(&test);
        let sr = fold_sum(&reference);
        let energy_gate_taken = st < thr * sr;

        let mut t = test.clone();
        let mut r = reference.clone();
        let v =
            unsafe { (p.c.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins as c_int, thr) };
        assert_eq!(
            v, 0,
            "{ctx}: threshold {thr:?} is one ULP above the measured contrast \
             {contrast:?}, so match() must reject"
        );
        if !energy_gate_taken {
            isolated += 1;
        }
        diff_match(&p, &format!("{ctx} thr=contrast+1ulp"), &test, &reference, bins as c_int, thr);
    }
    assert!(
        isolated > 0,
        "no iteration isolated the contrast gate (energy gate always fired first)"
    );
}

/// `total()` from `match.c`: a left-to-right `double` accumulation.
fn fold_sum(v: &[f64]) -> f64 {
    let mut s = 0.0f64;
    for &x in v {
        s += x;
    }
    s
}

/// The next representable `f64` above `x` (finite `x`).
fn next_up(x: f64) -> f64 {
    if x.is_sign_negative() && x != 0.0 {
        f64::from_bits(x.to_bits() - 1)
    } else if x == 0.0 {
        f64::from_bits(1)
    } else {
        f64::from_bits(x.to_bits() + 1)
    }
}

// ---------------------------------------------------------------------------
// Rows 3, 4 — `match` with `bins == 0`.
//
// These live in `tests/ub.rs`: the C `.so` *segfaults*. A zero-length VLA makes
// `differentiate` store `v[length-1]` == `v[-1]`, and the VLA base is `match`'s
// own `%rsp`, so that store overwrites the return address `call preprocess`
// pushed at `%rsp-8` with zero. Calling it in-process would take the test
// binary down, so it is probed in a forked child instead.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Row 9 — `match` with `threshold` = NaN (both `comisd`s unordered).
// Row 10 — `threshold` = ±inf against a zero-energy reference.
// ---------------------------------------------------------------------------

#[test]
fn err09_match_threshold_nan() {
    let p = pair();
    let mut rng = Rng::new(0xE009);
    for nan in [f64::NAN, -f64::NAN, f64::from_bits(0x7FF8_0000_DEAD_BEEF)] {
        for it in 0..iters() {
            let bins = rng.range_usize(1, 40);
            let test = gen_signed_f64(&mut rng, bins);
            let reference = gen_signed_f64(&mut rng, bins);
            let ctx = format!("err09 it={it} bins={bins} nan=0x{:016x}", nan.to_bits());
            diff_match(&p, &ctx, &test, &reference, bins as c_int, nan);
            let mut t = test.clone();
            let mut r = reference.clone();
            let v = unsafe { (p.c.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins as c_int, nan) };
            assert_eq!(v, 0, "{ctx}: a NaN threshold can never be met");
        }
    }
}

#[test]
fn err10_match_threshold_infinite_zero_reference() {
    let p = pair();
    for bins in [1usize, 2, 15, 16, 17, 40] {
        let test = vec![0.0f64; bins];
        let reference = vec![0.0f64; bins];
        for &thr in &[f64::INFINITY, f64::NEG_INFINITY] {
            let ctx = format!("err10 bins={bins} thr={thr:?}");
            diff_match(&p, &ctx, &test, &reference, bins as c_int, thr);
            let mut t = test.clone();
            let mut r = reference.clone();
            let v = unsafe { (p.c.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins as c_int, thr) };
            assert_eq!(v, 0, "{ctx}: inf * 0 is NaN, so neither gate can be met");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 11, 12, 13 — `spectral_contrast` with non-positive `length`.
// ---------------------------------------------------------------------------

#[test]
fn err11_spectral_length_zero() {
    let p = pair();
    let mut rng = Rng::new(0xE011);
    for it in 0..iters() {
        let n = rng.range_usize(1, 16);
        let a = gen_signed_f32(&mut rng, n);
        let b = gen_signed_f32(&mut rng, n);
        let ctx = format!("err11 it={it}");
        // Non-empty allocations, length = 0: nothing is read or written.
        let (mut ca, mut cb) = (a.clone(), b.clone());
        let cr = unsafe { (p.c.spectral_contrast)(ca.as_mut_ptr(), cb.as_mut_ptr(), 0) };
        let (mut ra, mut rb) = (a.clone(), b.clone());
        let rr = unsafe { (p.rust.spectral_contrast)(ra.as_mut_ptr(), rb.as_mut_ptr(), 0) };
        assert_f64_bits_eq(&ctx, cr, rr);
        assert_eq!(cr.to_bits(), 0u64, "{ctx}: C must return +0.0, got {cr:?}");
        assert_slice32_bits_eq(&ctx, "a untouched", &a, &ca);
        assert_slice32_bits_eq(&ctx, "b untouched", &b, &cb);
        assert_slice32_bits_eq(&ctx, "a untouched (Rust)", &a, &ra);
        assert_slice32_bits_eq(&ctx, "b untouched (Rust)", &b, &rb);
    }
}

#[test]
fn err12_spectral_length_negative() {
    let p = pair();
    let mut rng = Rng::new(0xE012);
    // `i < length` is a signed `int` comparison, so every negative length is an
    // immediate zero-iteration loop -- no cast to `size_t`, hence no crash.
    let mut lengths: Vec<c_int> = vec![-1, -2, -16, -17, -1000, c_int::MIN, c_int::MIN + 1];
    for _ in 0..32 {
        lengths.push(-((rng.next_u32() % 1_000_000) as c_int) - 1);
    }
    for len in lengths {
        let n = rng.range_usize(1, 16);
        let a = gen_signed_f32(&mut rng, n);
        let b = gen_signed_f32(&mut rng, n);
        let ctx = format!("err12 len={len}");
        let (mut ca, mut cb) = (a.clone(), b.clone());
        let cr = unsafe { (p.c.spectral_contrast)(ca.as_mut_ptr(), cb.as_mut_ptr(), len) };
        let (mut ra, mut rb) = (a.clone(), b.clone());
        let rr = unsafe { (p.rust.spectral_contrast)(ra.as_mut_ptr(), rb.as_mut_ptr(), len) };
        assert_f64_bits_eq(&ctx, cr, rr);
        assert_eq!(cr.to_bits(), 0u64, "{ctx}: C must return +0.0, got {cr:?}");
        assert_slice32_bits_eq(&ctx, "a untouched", &a, &ca);
        assert_slice32_bits_eq(&ctx, "b untouched", &b, &cb);
        assert_slice32_bits_eq(&ctx, "a untouched (Rust)", &a, &ra);
        assert_slice32_bits_eq(&ctx, "b untouched (Rust)", &b, &rb);
    }
}

#[test]
fn err13_spectral_null_with_nonpositive_length() {
    let p = pair();
    for len in [0 as c_int, -1, -16, c_int::MIN] {
        let ctx = format!("err13 len={len}");
        let cr =
            unsafe { (p.c.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), len) };
        let rr =
            unsafe { (p.rust.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), len) };
        assert_f64_bits_eq(&ctx, cr, rr);
        assert_eq!(cr.to_bits(), 0u64, "{ctx}: C must return +0.0");
    }
}

// ---------------------------------------------------------------------------
// Row 15 — `spectral_contrast` on a zero-magnitude vector: `0.0 / 0.0`.
// ---------------------------------------------------------------------------

#[test]
fn err15_spectral_zero_magnitude_divide_by_zero() {
    let p = pair();
    const INDEFINITE: u64 = 0xFFF8_0000_0000_0000; // x86 "real indefinite" QNaN
    for len in 1..=40usize {
        for &z in &[0.0f32, -0.0f32] {
            let a = vec![z; len];
            let b = vec![z; len];
            let ctx = format!("err15 len={len} z={z:?}");
            diff_spectral(&p, &ctx, &a, &b, len as c_int);
            let (mut ca, mut cb) = (a.clone(), b.clone());
            let cr = unsafe { (p.c.spectral_contrast)(ca.as_mut_ptr(), cb.as_mut_ptr(), len as c_int) };
            assert_eq!(
                cr.to_bits(),
                INDEFINITE,
                "{ctx}: expected the x86 indefinite QNaN, got 0x{:016x}",
                cr.to_bits()
            );
        }
    }
    // And through `match`: `bins == 1` always preprocesses to all-zero, so the
    // contrast is that same NaN and `match` must return 0 for every threshold.
    for &thr in SPECIAL_THRESHOLDS {
        let test = vec![3.0f64];
        let reference = vec![3.0f64];
        diff_match(&p, &format!("err15 match thr={thr:?}"), &test, &reference, 1, thr);
    }
}

// ---------------------------------------------------------------------------
// Generic boundary: `bins`/`length` == INT_MAX and other out-of-domain ints,
// exercised on the paths that are *defined*.
// ---------------------------------------------------------------------------

#[test]
fn err_int_domain_sweep_defined_paths() {
    let p = pair();
    // `spectral_contrast`: every non-positive int, including the extremes.
    for len in [c_int::MIN, c_int::MIN + 1, -3, -1, 0] {
        let cr = unsafe { (p.c.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), len) };
        let rr =
            unsafe { (p.rust.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), len) };
        assert_f64_bits_eq(&format!("int-sweep spectral len={len}"), cr, rr);
    }
    // `match` has no non-positive `bins` with defined behaviour (`bins == 0`
    // already corrupts its own return address, `bins < 0` does a ~2^64-byte
    // memcpy), so the whole non-positive domain is covered by `tests/ub.rs`.
    // The smallest *defined* value is 1, swept here against every threshold.
    let mut rng = Rng::new(0xE0FF);
    for &thr in SPECIAL_THRESHOLDS {
        for bins in 1..=3usize {
            let t = gen_signed_f64(&mut rng, bins);
            let r = gen_signed_f64(&mut rng, bins);
            diff_match(
                &p,
                &format!("int-sweep match bins={bins} thr={thr:?}"),
                &t,
                &r,
                bins as c_int,
                thr,
            );
        }
    }
}
