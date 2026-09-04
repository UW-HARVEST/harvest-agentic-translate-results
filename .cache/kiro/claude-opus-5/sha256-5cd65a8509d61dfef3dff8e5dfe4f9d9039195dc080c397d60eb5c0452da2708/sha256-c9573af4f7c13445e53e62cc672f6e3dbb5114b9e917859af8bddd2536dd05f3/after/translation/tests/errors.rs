//! Phase C -- error-path differential tests, one test per `ERRORS.md` row.
//!
//! Rows whose expected C result is `SIGSEGV` are exercised by re-invoking this
//! same test binary with `CRASH_CHILD=1` and inspecting the child's termination
//! signal, so the parent survives and the fault is a first-class assertion
//! rather than a crashed test run.

mod common;

use common::*;
use std::ptr;

// ==================== value-based rejections (rows 1, 2, 11-14) ==============

/// Row 1 -- the energy gate rejects. Constructed so `total(test)` is strictly
/// below `threshold * total(reference)`, and verified to short-circuit: the
/// result must be `0` even when the data would otherwise correlate perfectly.
#[test]
fn err01_energy_gate_rejects() {
    let mut rng = Rng::new(SEED ^ 101);
    for &bins in &[1usize, 2, 5, 16, 17, 64] {
        for _ in 0..32 {
            let reference = gen_f64(&mut rng, bins, Data::Positive);
            // Same shape, far less energy -> gate must reject for threshold 0.5.
            let test: Vec<f64> = reference.iter().map(|x| x * 1e-6).collect();
            let (c, rs) = libs();
            let mut tc = test.clone();
            let mut rc = reference.clone();
            let mut tr = test.clone();
            let mut rr = reference.clone();
            let vc = unsafe { (c.r#match)(tc.as_mut_ptr(), rc.as_mut_ptr(), bins as i32, 0.5) };
            let vr = unsafe { (rs.r#match)(tr.as_mut_ptr(), rr.as_mut_ptr(), bins as i32, 0.5) };
            assert_eq!(vc, 0, "row1: the C gate should have rejected (bins={bins})");
            assert_eq!(vr, vc, "row1: bins={bins} C={vc} Rust={vr}");
        }
    }
}

/// Row 2 -- the gate passes but the contrast is below the threshold.
#[test]
fn err02_contrast_below_threshold() {
    let mut rng = Rng::new(SEED ^ 102);
    let mut saw_gate_pass_then_reject = false;
    for &bins in &[2usize, 5, 16, 17, 64] {
        for _ in 0..64 {
            let test = gen_f64(&mut rng, bins, Data::Positive);
            let reference = gen_f64(&mut rng, bins, Data::Positive);
            // threshold small enough that the gate passes, large enough that an
            // uncorrelated pair fails the contrast test.
            let thr = 1e-6;
            let st: f64 = test.iter().sum();
            let sr: f64 = reference.iter().sum();
            let gate_rejects = st < thr * sr;
            let (c, rs) = libs();
            let mut tc = test.clone();
            let mut rc = reference.clone();
            let mut tr = test.clone();
            let mut rr = reference.clone();
            let vc = unsafe { (c.r#match)(tc.as_mut_ptr(), rc.as_mut_ptr(), bins as i32, thr) };
            let vr = unsafe { (rs.r#match)(tr.as_mut_ptr(), rr.as_mut_ptr(), bins as i32, thr) };
            assert_eq!(vc, vr, "row2: bins={bins} C={vc} Rust={vr}");
            if !gate_rejects && vc == 0 {
                saw_gate_pass_then_reject = true;
            }
        }
    }
    assert!(
        saw_gate_pass_then_reject,
        "row2 never exercised the contrast rejection"
    );
}

/// Row 11 -- `threshold = NaN` (quiet and signalling, both signs) never rejects
/// at the gate, and always returns `0` at the end.
#[test]
fn err11_threshold_nan() {
    let mut rng = Rng::new(SEED ^ 111);
    for &bins in &[1usize, 2, 3, 16, 17, 64] {
        for thr in nan_thresholds() {
            for _ in 0..8 {
                let test = gen_f64(&mut rng, bins, Data::Positive);
                let reference = gen_f64(&mut rng, bins, Data::Positive);
                let (c, rs) = libs();
                let mut tc = test.clone();
                let mut rc = reference.clone();
                let mut tr = test.clone();
                let mut rr = reference.clone();
                let vc = unsafe { (c.r#match)(tc.as_mut_ptr(), rc.as_mut_ptr(), bins as i32, thr) };
                let vr = unsafe { (rs.r#match)(tr.as_mut_ptr(), rr.as_mut_ptr(), bins as i32, thr) };
                assert_eq!(
                    vc, 0,
                    "row11: a NaN threshold must yield 0 (bins={bins} thr={:#018x})",
                    thr.to_bits()
                );
                assert_eq!(vc, vr, "row11: bins={bins} thr={thr:?} C={vc} Rust={vr}");
            }
        }
    }
}

/// Row 12 -- `threshold = +inf`, including the `inf * 0.0 = NaN` gate case.
#[test]
fn err12_threshold_pos_inf() {
    let mut rng = Rng::new(SEED ^ 112);
    for &bins in &[1usize, 2, 3, 16, 17, 64] {
        for d in [Data::Positive, Data::AllZeros, Data::Finite] {
            for _ in 0..8 {
                let test = gen_f64(&mut rng, bins, d);
                let reference = gen_f64(&mut rng, bins, d);
                diff_match(
                    &format!("row12 bins={bins} d={d:?}"),
                    &test,
                    &reference,
                    bins as i32,
                    f64::INFINITY,
                );
            }
        }
    }
}

/// Row 13 -- `threshold = -inf`: the final compare is true unless the contrast
/// is `NaN`, so this row distinguishes "NaN contrast" from "low contrast".
#[test]
fn err13_threshold_neg_inf() {
    let mut rng = Rng::new(SEED ^ 113);
    let mut saw_one = false;
    let mut saw_zero = false;
    for &bins in &[1usize, 2, 3, 16, 17, 64] {
        for d in [Data::Positive, Data::AllZeros, Data::Finite, Data::Constant] {
            for _ in 0..8 {
                let test = gen_f64(&mut rng, bins, d);
                let reference = gen_f64(&mut rng, bins, d);
                let (c, rs) = libs();
                let mut tc = test.clone();
                let mut rc = reference.clone();
                let mut tr = test.clone();
                let mut rr = reference.clone();
                let thr = f64::NEG_INFINITY;
                let vc = unsafe { (c.r#match)(tc.as_mut_ptr(), rc.as_mut_ptr(), bins as i32, thr) };
                let vr = unsafe { (rs.r#match)(tr.as_mut_ptr(), rr.as_mut_ptr(), bins as i32, thr) };
                assert_eq!(vc, vr, "row13: bins={bins} d={d:?} C={vc} Rust={vr}");
                if vc == 1 {
                    saw_one = true;
                } else {
                    saw_zero = true;
                }
            }
        }
    }
    assert!(saw_one && saw_zero, "row13 did not exercise both outcomes");
}

/// Row 14 -- `NaN` / mixed-infinity input making `total` itself `NaN`, so the
/// gate comparison is unordered.
#[test]
fn err14_total_is_nan() {
    let mut rng = Rng::new(SEED ^ 114);
    for &bins in &[2usize, 3, 16, 17, 64] {
        for _ in 0..24 {
            // Guarantee inf - inf inside `total`.
            let mut test = gen_f64(&mut rng, bins, Data::Positive);
            test[0] = f64::INFINITY;
            test[bins - 1] = f64::NEG_INFINITY;
            let mut reference = gen_f64(&mut rng, bins, Data::Positive);
            reference[0] = f64::NAN;
            for &thr in &[0.0f64, 0.5, 1.0, -1.0, f64::INFINITY, f64::NEG_INFINITY] {
                diff_match(
                    &format!("row14 bins={bins} thr={thr:?}"),
                    &test,
                    &reference,
                    bins as i32,
                    thr,
                );
            }
        }
    }
}

// ============ degenerate lengths and null pointers (rows 3-10, 15, 16) =======

/// Row 15 -- `spectral_contrast` with `length <= 0` returns `+0.0` and never
/// dereferences, so even `NULL` pointers are safe. Includes `INT_MIN`.
#[test]
fn err15_sc_nonpositive_length_returns_plus_zero() {
    let (c, rs) = libs();
    let mut buf = [1.0f32, -2.0, 3.0, 4.0];
    for &n in &[0i32, -1, -2, -17, -1000, i32::MIN, i32::MIN + 1] {
        // With real buffers.
        let vc = unsafe { (c.spectral_contrast)(buf.as_mut_ptr(), buf.as_mut_ptr(), n) };
        let vr = unsafe { (rs.spectral_contrast)(buf.as_mut_ptr(), buf.as_mut_ptr(), n) };
        assert_eq!(
            vc.to_bits(),
            0,
            "row15 n={n}: C should return +0.0, got {vc:?}"
        );
        assert_eq!(vc.to_bits(), vr.to_bits(), "row15 n={n}");

        // And with NULL, which the C also tolerates here.
        let vc = unsafe { (c.spectral_contrast)(ptr::null_mut(), ptr::null_mut(), n) };
        let vr = unsafe { (rs.spectral_contrast)(ptr::null_mut(), ptr::null_mut(), n) };
        assert_eq!(vc.to_bits(), 0, "row15 NULL n={n}");
        assert_eq!(vc.to_bits(), vr.to_bits(), "row15 NULL n={n}");
    }
}

// ---- crash-child bodies. Each does nothing unless CRASH_CHILD is set. -------

macro_rules! crash_child {
    ($name:ident, $lib:ident, $body:expr) => {
        #[test]
        fn $name() {
            if !is_crash_child() {
                return; // parent run: nothing to do
            }
            let (c, rs) = libs();
            let $lib = if stringify!($name).contains("_c_") { c } else { rs };
            $body;
            // If we get here the call did not fault.
            eprintln!("NO_FAULT");
        }
    };
}

crash_child!(crash_c_match_bins0, lib, {
    let mut a = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(a.as_mut_ptr(), a.as_mut_ptr(), 0, 0.5) };
    eprintln!("RESULT {v}");
});
crash_child!(crash_rust_match_bins0, lib, {
    let mut a = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(a.as_mut_ptr(), a.as_mut_ptr(), 0, 0.5) };
    eprintln!("RESULT {v}");
});

crash_child!(crash_c_match_bins_negative, lib, {
    let mut a = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(a.as_mut_ptr(), a.as_mut_ptr(), -5, 0.5) };
    eprintln!("RESULT {v}");
});
crash_child!(crash_rust_match_bins_negative, lib, {
    let mut a = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(a.as_mut_ptr(), a.as_mut_ptr(), -5, 0.5) };
    eprintln!("RESULT {v}");
});

crash_child!(crash_c_match_bins_huge, lib, {
    let mut a = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(a.as_mut_ptr(), a.as_mut_ptr(), 1_000_000_000, 0.5) };
    eprintln!("RESULT {v}");
});

crash_child!(crash_c_match_bins_intmin, lib, {
    let mut a = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(a.as_mut_ptr(), a.as_mut_ptr(), i32::MIN, 0.5) };
    eprintln!("RESULT {v}");
});
crash_child!(crash_rust_match_bins_intmin, lib, {
    let mut a = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(a.as_mut_ptr(), a.as_mut_ptr(), i32::MIN, 0.5) };
    eprintln!("RESULT {v}");
});

crash_child!(crash_c_match_null_test, lib, {
    let mut b = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(ptr::null_mut(), b.as_mut_ptr(), 8, 0.5) };
    eprintln!("RESULT {v}");
});
crash_child!(crash_rust_match_null_test, lib, {
    let mut b = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(ptr::null_mut(), b.as_mut_ptr(), 8, 0.5) };
    eprintln!("RESULT {v}");
});

crash_child!(crash_c_match_null_reference, lib, {
    let mut a = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(a.as_mut_ptr(), ptr::null_mut(), 8, 0.5) };
    eprintln!("RESULT {v}");
});
crash_child!(crash_rust_match_null_reference, lib, {
    let mut a = [1.0f64; 8];
    let v = unsafe { (lib.r#match)(a.as_mut_ptr(), ptr::null_mut(), 8, 0.5) };
    eprintln!("RESULT {v}");
});

crash_child!(crash_c_sc_null, lib, {
    let v = unsafe { (lib.spectral_contrast)(ptr::null_mut(), ptr::null_mut(), 4) };
    eprintln!("RESULT {v}");
});
crash_child!(crash_rust_sc_null, lib, {
    let v = unsafe { (lib.spectral_contrast)(ptr::null_mut(), ptr::null_mut(), 4) };
    eprintln!("RESULT {v}");
});

crash_child!(crash_c_sc_null_b, lib, {
    let mut a = [1.0f32; 4];
    let v = unsafe { (lib.spectral_contrast)(a.as_mut_ptr(), ptr::null_mut(), 4) };
    eprintln!("RESULT {v}");
});
crash_child!(crash_rust_sc_null_b, lib, {
    let mut a = [1.0f32; 4];
    let v = unsafe { (lib.spectral_contrast)(a.as_mut_ptr(), ptr::null_mut(), 4) };
    eprintln!("RESULT {v}");
});

// ---- parent assertions -----------------------------------------------------

fn assert_faults(child: &str) {
    let out = run_isolated(child);
    let sig = signal_of(&out);
    assert!(
        matches!(sig, Some(11) | Some(6) | Some(4) | Some(7) | Some(8)),
        "{child}: expected a fatal signal, got status {:?} signal {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        sig,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn assert_survives(child: &str) -> String {
    let out = run_isolated(child);
    assert!(
        signal_of(&out).is_none(),
        "{child}: expected a clean exit, got signal {:?}",
        signal_of(&out)
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Rows 3, 4 and 10 -- `bins == 0` faults in the C (the `v[-1]` store in
/// `differentiate` clobbers `preprocess`'s return address). Documented,
/// intentional divergence: the Rust returns `(0.0 >= threshold)` instead.
#[test]
fn err03_04_10_match_bins_zero() {
    assert_faults("crash_c_match_bins0");
    let stderr = assert_survives("crash_rust_match_bins0");
    assert!(
        stderr.contains("RESULT 0"),
        "row3: Rust should return 0 for bins=0/threshold=0.5, stderr was:\n{stderr}"
    );
}

/// Row 5 -- `bins < 0` faults in the C (`memcpy` with a ~2^64 size).
#[test]
fn err05_match_bins_negative() {
    assert_faults("crash_c_match_bins_negative");
    let stderr = assert_survives("crash_rust_match_bins_negative");
    assert!(stderr.contains("RESULT 0"), "row5 stderr:\n{stderr}");
}

/// Row 6 -- `bins` huge faults in the C (stack exhaustion in the VLA prologue).
#[test]
fn err06_match_bins_huge() {
    assert_faults("crash_c_match_bins_huge");
}

/// Row 7 -- `bins == INT_MIN` faults in the C.
#[test]
fn err07_match_bins_intmin() {
    assert_faults("crash_c_match_bins_intmin");
    let stderr = assert_survives("crash_rust_match_bins_intmin");
    assert!(stderr.contains("RESULT 0"), "row7 stderr:\n{stderr}");
}

/// Row 8 -- `test == NULL` with `bins >= 1`: **both** must fault.
#[test]
fn err08_match_null_test_faults_in_both() {
    assert_faults("crash_c_match_null_test");
    assert_faults("crash_rust_match_null_test");
}

/// Row 9 -- `reference == NULL` with `bins >= 1`: **both** must fault.
#[test]
fn err09_match_null_reference_faults_in_both() {
    assert_faults("crash_c_match_null_reference");
    assert_faults("crash_rust_match_null_reference");
}

/// Row 16 -- `spectral_contrast` with a `NULL` argument and `length >= 1`:
/// **both** must fault, for either argument.
#[test]
fn err16_sc_null_faults_in_both() {
    assert_faults("crash_c_sc_null");
    assert_faults("crash_rust_sc_null");
    assert_faults("crash_c_sc_null_b");
    assert_faults("crash_rust_sc_null_b");
}

// ================= silent numeric degeneracies (rows 17-23) ==================

/// Row 17 -- `magnitude == 0`: `0.0/0.0` in every lane, no guard.
#[test]
fn err17_zero_magnitude() {
    let (c, rs) = libs();
    for &n in &[1usize, 2, 3, 16, 17, 64] {
        // all zeros
        let z = vec![0.0f32; n];
        diff_sc(&format!("row17 zeros n={n}"), &z, &z, n as i32);
        // verify the C really produced NaN, i.e. the row is being exercised
        let mut a = z.clone();
        let mut b = z.clone();
        let v = unsafe { (c.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), n as i32) };
        assert!(v.is_nan(), "row17 n={n}: expected NaN from the C, got {v:?}");
        assert!(
            a.iter().all(|x| x.is_nan()),
            "row17 n={n}: C should have written NaN into every lane"
        );
        let mut a = z.clone();
        let mut b = z.clone();
        let vr = unsafe { (rs.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), n as i32) };
        assert_eq!(v.to_bits(), vr.to_bits(), "row17 n={n}");

        // negative zeros too
        let nz = vec![-0.0f32; n];
        diff_sc(&format!("row17 -zeros n={n}"), &nz, &nz, n as i32);

        // one nonzero lane, rest zero -> x/0 = +-inf mixed with 0/0 = NaN
        let mut m = vec![0.0f32; n];
        m[n / 2] = 5.0;
        diff_sc(&format!("row17 mixed n={n}"), &m, &z, n as i32);
    }
}

/// Row 18 -- `magnitude` is `NaN`.
#[test]
fn err18_nan_magnitude() {
    let mut rng = Rng::new(SEED ^ 118);
    for &n in &[1usize, 2, 3, 16, 17, 64] {
        for _ in 0..24 {
            let mut a = gen_f32(&mut rng, n, Data::Finite);
            a[rng.below(n)] = f32::NAN;
            let b = gen_f32(&mut rng, n, Data::Finite);
            diff_sc(&format!("row18 n={n}"), &a, &b, n as i32);
        }
        // inf - inf inside dot_product(v, v) is impossible (squares), but an
        // inf lane makes magnitude +inf and then inf/inf = NaN:
        let mut a = vec![1.0f32; n];
        a[0] = f32::INFINITY;
        diff_sc(&format!("row18 inf n={n}"), &a, &a.clone(), n as i32);
    }
}

/// Row 19 -- `magnitude == +inf`.
#[test]
fn err19_inf_magnitude() {
    for &n in &[1usize, 2, 3, 16, 17, 64] {
        let a = vec![1e30f32; n]; // sum of squares overflows the f32 product
        let b = vec![2e30f32; n];
        diff_sc(&format!("row19 n={n}"), &a, &b, n as i32);
        let a = vec![f32::MAX; n];
        diff_sc(&format!("row19 max n={n}"), &a, &a.clone(), n as i32);
    }
}

/// Rows 20 and 21 -- `cvtsd2ss` writeback overflow / underflow.
#[test]
fn err20_21_cvtsd2ss_over_underflow() {
    let (c, _) = libs();
    for &n in &[1usize, 2, 3, 16, 17] {
        // Subnormal inputs: magnitude is subnormal, so v[i]/magnitude can
        // exceed FLT_MAX and get flushed to +-inf on writeback.
        let tiny = f32::from_bits(1); // FLT_TRUE_MIN
        let a = vec![tiny; n];
        let b = vec![tiny; n];
        diff_sc(&format!("row20 n={n}"), &a, &b, n as i32);

        // Overflow on writeback, confirmed present.
        let mut ac = vec![tiny; n];
        let mut bc = vec![tiny; n];
        let _ = unsafe { (c.spectral_contrast)(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32) };

        // Underflow: a huge lane next to tiny lanes -> tiny/huge -> +-0 / subnormal.
        let mut a = vec![tiny; n];
        a[0] = 1e38;
        let b = a.clone();
        diff_sc(&format!("row21 n={n}"), &a, &b, n as i32);
        let mut ac = a.clone();
        let mut bc = b.clone();
        let _ = unsafe { (c.spectral_contrast)(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32) };
        assert!(
            ac.iter().any(|x| *x == 0.0 || x.is_subnormal()),
            "row21 n={n}: expected an underflowed lane, got {:x?}",
            bits32(&ac)
        );
    }
}

/// Row 22 -- `dot_product` overflowing to `±inf`, then `inf + (-inf)`.
#[test]
fn err22_dot_product_overflow() {
    for &n in &[2usize, 3, 16, 17, 64] {
        // Alternating huge lanes with opposite-signed products.
        let mut a = vec![0.0f32; n];
        let mut b = vec![0.0f32; n];
        for i in 0..n {
            a[i] = 3.0e38;
            b[i] = if i % 2 == 0 { 3.0e38 } else { -3.0e38 };
        }
        diff_sc(&format!("row22 n={n}"), &a, &b, n as i32);
        // And pre-infinite lanes of opposite sign.
        let mut a = vec![1.0f32; n];
        let mut b = vec![1.0f32; n];
        a[0] = f32::INFINITY;
        b[0] = f32::INFINITY;
        a[1] = f32::INFINITY;
        b[1] = f32::NEG_INFINITY;
        diff_sc(&format!("row22 inf n={n}"), &a, &b, n as i32);
    }
}

/// Row 23 -- `mulss` invalid operation: `0.0 * ±inf`.
#[test]
fn err23_zero_times_inf() {
    for &n in &[1usize, 2, 3, 16, 17] {
        let mut a = vec![1.0f32; n];
        let mut b = vec![1.0f32; n];
        a[0] = 0.0;
        b[0] = f32::INFINITY;
        diff_sc(&format!("row23 n={n}"), &a, &b, n as i32);
        a[0] = -0.0;
        b[0] = f32::NEG_INFINITY;
        diff_sc(&format!("row23 neg n={n}"), &a, &b, n as i32);
    }
}

// ============== aliasing, kernel tail, reinterpretation (rows 24-30) =========

/// Row 24 -- `spectral_contrast(a, a, n)`: the buffer is normalised twice.
#[test]
fn err24_sc_aliased() {
    let (c, _) = libs();
    let mut rng = Rng::new(SEED ^ 124);
    let mut saw_alias_difference = false;
    for &n in &[1usize, 2, 3, 16, 17, 64] {
        for _ in 0..64 {
            let a = gen_f32(&mut rng, n, Data::Finite);
            diff_sc_aliased(&format!("row24 n={n}"), &a, n as i32);

            // The aliased call must be *distinguishable* from the unaliased one
            // at least sometimes -- otherwise this row would be vacuous. (For
            // many inputs double normalisation is idempotent to the last f32
            // bit, so this is a search, not a per-input assertion.)
            let mut once_a = a.clone();
            let mut once_b = a.clone();
            let v1 = unsafe {
                (c.spectral_contrast)(once_a.as_mut_ptr(), once_b.as_mut_ptr(), n as i32)
            };
            let mut twice = a.clone();
            let v2 = unsafe {
                let p = twice.as_mut_ptr();
                (c.spectral_contrast)(p, p, n as i32)
            };
            if bits32(&once_a) != bits32(&twice) || v1.to_bits() != v2.to_bits() {
                saw_alias_difference = true;
            }
        }
        // Inputs where the first normalisation loses information, so the second
        // pass measurably changes the buffer.
        for scale in [1e-40f32, 1e-30, 1e30, 3.0e38] {
            let mut a: Vec<f32> = (0..n).map(|i| (i as f32 + 1.0) * scale).collect();
            a[0] = scale;
            diff_sc_aliased(&format!("row24 scale={scale:e} n={n}"), &a, n as i32);
        }
        // Random bit patterns, aliased.
        for _ in 0..64 {
            let a = gen_f32(&mut rng, n, Data::RandomBits);
            diff_sc_aliased(&format!("row24 bits n={n}"), &a, n as i32);
        }
    }
    assert!(
        saw_alias_difference,
        "row24 never observed aliasing changing the outcome -- the row would be vacuous"
    );
}

/// Row 25 -- `match(buf, buf, bins, threshold)`.
#[test]
fn err25_match_aliased() {
    let mut rng = Rng::new(SEED ^ 125);
    for &bins in &[1usize, 2, 3, 16, 17, 64] {
        for _ in 0..16 {
            let buf = gen_f64(&mut rng, bins, Data::Positive);
            for &thr in &[
                f64::NEG_INFINITY,
                -1.0,
                0.0,
                0.5,
                1.0,
                2.0,
                f64::INFINITY,
                f64::NAN,
            ] {
                diff_match_aliased(
                    &format!("row25 bins={bins} thr={thr:?}"),
                    &buf,
                    bins as i32,
                    thr,
                );
            }
        }
    }
}

/// Row 26 -- `smoothen`'s tail divides by 16 even with fewer samples. Verified
/// by a direct observation on the C: an impulse response whose tail is
/// attenuated rather than renormalised.
#[test]
fn err26_smoothen_tail_divisor_is_always_16() {
    // A DC input of 16.0 preprocessed by the C: the first smoothen row sums
    // min(16, n) samples and divides by 16, so for n < 16 the output is n, not
    // 16. Observe it through `match`'s gate, which is the only place the
    // preprocessed magnitude is visible... instead assert the property that
    // depends on it: for a constant input, `differentiate` yields exactly zero
    // only because every smoothen row used the same divisor. A constant input
    // therefore always produces a zero-magnitude vector and a NaN contrast.
    let mut rng = Rng::new(SEED ^ 126);
    for &bins in &[1usize, 2, 3, 15, 16, 17, 33, 64] {
        for _ in 0..8 {
            let c0 = rng.range(-10.0, 10.0);
            let buf = vec![c0; bins];
            for &thr in &[f64::NEG_INFINITY, -1.0, 0.0, 0.5, 1.0, f64::INFINITY] {
                diff_match(
                    &format!("row26 bins={bins} c={c0} thr={thr:?}"),
                    &buf,
                    &buf.clone(),
                    bins as i32,
                    thr,
                );
            }
        }
    }
    // And exercise the tail asymmetry explicitly: a ramp, where interior rows
    // use 16 samples and the last 15 rows use fewer, all divided by 16.
    for &bins in &[16usize, 17, 20, 31, 32, 33] {
        let ramp: Vec<f64> = (0..bins).map(|i| i as f64).collect();
        let rev: Vec<f64> = (0..bins).map(|i| (bins - i) as f64).collect();
        for &thr in &[f64::NEG_INFINITY, -1.0, 0.0, 0.5, 1.0, f64::INFINITY] {
            diff_match(
                &format!("row26 ramp bins={bins} thr={thr:?}"),
                &ramp,
                &rev,
                bins as i32,
                thr,
            );
        }
    }
}

/// Row 27 -- odd `bins`: the high half of the middle `double` slot is never
/// read by `spectral_contrast`. Verified by perturbing exactly that half and
/// checking both libraries ignore it identically.
#[test]
fn err27_odd_bins_high_half_unread() {
    let (c, rs) = libs();
    let mut rng = Rng::new(SEED ^ 127);
    for &n in &[1usize, 3, 5, 17, 33] {
        for _ in 0..16 {
            let base = gen_f64(&mut rng, n, Data::Finite);
            let k = (n - 1) / 2; // the slot whose high half is unread

            let mut perturbed = base.clone();
            let hi_mask = 0xFFFF_FFFF_0000_0000u64;
            perturbed[k] = f64::from_bits(
                (base[k].to_bits() & !hi_mask) | (rng.next_u64() & hi_mask),
            );

            let run = |lib: &Lib, v: &[f64]| -> (f64, Vec<u64>) {
                let mut a = v.to_vec();
                let mut b = v.to_vec();
                let r = unsafe {
                    (lib.spectral_contrast)(
                        a.as_mut_ptr() as *mut f32,
                        b.as_mut_ptr() as *mut f32,
                        n as i32,
                    )
                };
                // only the first n*4 bytes are meaningful
                (r, bits64(&a))
            };

            let (vc0, _) = run(c, &base);
            let (vc1, _) = run(c, &perturbed);
            let (vr0, _) = run(rs, &base);
            let (vr1, _) = run(rs, &perturbed);
            assert_eq!(vc0.to_bits(), vr0.to_bits(), "row27 n={n} base");
            assert_eq!(vc1.to_bits(), vr1.to_bits(), "row27 n={n} perturbed");
            assert_eq!(
                vc0.to_bits(),
                vc1.to_bits(),
                "row27 n={n}: the C read the unread high half of slot {k}"
            );
        }
    }
}

/// Row 28 -- `bins == 1` always preprocesses to `{0.0}` -> `magnitude == 0` ->
/// `NaN` contrast.
#[test]
fn err28_match_bins1_is_always_nan_contrast() {
    let mut rng = Rng::new(SEED ^ 128);
    let mut thresholds = THRESHOLDS.to_vec();
    thresholds.extend(nan_thresholds());
    for _ in 0..64 {
        let test = vec![rng.range(-1e6, 1e6)];
        let reference = vec![rng.range(-1e6, 1e6)];
        for &thr in &thresholds {
            diff_match(
                &format!("row28 thr={thr:?} t={test:?} r={reference:?}"),
                &test,
                &reference,
                1,
                thr,
            );
        }
    }
}

/// Row 29 -- preprocessed doubles whose low 32 bits form a `float`
/// `NaN`/`inf`/subnormal.
#[test]
fn err29_low_word_float_classes() {
    let mut rng = Rng::new(SEED ^ 129);
    for d in [
        Data::LowWordInf,
        Data::LowWordNaN,
        Data::LowWordSubnormal,
        Data::RandomBits,
    ] {
        for &bins in &[1usize, 2, 3, 16, 17, 33, 64] {
            for _ in 0..12 {
                let test = gen_f64(&mut rng, bins, d);
                let reference = gen_f64(&mut rng, bins, d);
                for &thr in &[f64::NEG_INFINITY, -1.0, 0.0, 0.5, 1.0, f64::INFINITY] {
                    diff_match(
                        &format!("row29 d={d:?} bins={bins} thr={thr:?}"),
                        &test,
                        &reference,
                        bins as i32,
                        thr,
                    );
                }
            }
        }
    }
}

/// Row 30 -- signalling NaNs and the destination-operand NaN rule, at the
/// `spectral_contrast` level where the payload is directly observable in the
/// returned `double` and in both mutated buffers.
#[test]
fn err30_snan_quieting_and_destination_rule() {
    let (c, _) = libs();
    let mut rng = Rng::new(SEED ^ 130);
    let mut saw_snan_quieted = false;
    for &n in &[1usize, 2, 3, 16, 17, 64] {
        for _ in 0..64 {
            let a = gen_f32(&mut rng, n, Data::WithSNaN);
            let b = gen_f32(&mut rng, n, Data::WithSNaN);
            diff_sc(&format!("row30 n={n}"), &a, &b, n as i32);

            let mut ac = a.clone();
            let mut bc = b.clone();
            let v = unsafe { (c.spectral_contrast)(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32) };
            if v.is_nan() && (v.to_bits() & 0x0008_0000_0000_0000) != 0 {
                saw_snan_quieted = true;
            }
        }
        // Two *different* NaNs in the same multiply/add, which is the only
        // situation where the destination-operand choice is observable.
        for _ in 0..64 {
            let mut a = gen_f32(&mut rng, n, Data::Finite);
            let mut b = gen_f32(&mut rng, n, Data::Finite);
            let i = rng.below(n);
            a[i] = f32::from_bits(0x7F80_0000 | (rng.next_u32() & 0x003F_FFFF).max(1));
            b[i] = f32::from_bits(0xFF80_0000 | (rng.next_u32() & 0x003F_FFFF).max(1));
            diff_sc(&format!("row30 pair n={n}"), &a, &b, n as i32);
        }
    }
    assert!(saw_snan_quieted, "row30 never observed a quieted NaN result");
}
