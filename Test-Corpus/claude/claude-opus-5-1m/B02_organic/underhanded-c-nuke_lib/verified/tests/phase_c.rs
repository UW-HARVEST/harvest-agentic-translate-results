//! Phase C -- error-path / rejection differential tests.
//!
//! One `#[test]` per row of ERRORS.md, in the same order. Each constructs the
//! exact invalid input or degenerate condition and asserts that BOTH shared
//! objects return the *same* sentinel (`0`/`1` for `match`, the same NaN /
//! infinity / zero bit pattern for `spectral_contrast`) -- not merely that both
//! "failed somehow".

mod common;

use common::*;

/// The library has no error codes: `match` rejects with `0`.
const REJECT: i32 = 0;

// ---------------------------------------------------------------------------
// Row 1 -- the one and only explicit rejection in the whole library.
// ---------------------------------------------------------------------------

#[test]
fn err_row01_energy_gate_rejects() {
    let both = both();
    for bins in [1, 2, 3, 16, 17, 33, 64] {
        for k in 0..64 {
            let mut rng = Rng::new(0xC001 ^ (bins as u64) << 20 ^ k);
            // test energy 0, reference energy > 0, threshold > 0  =>  0 < thr*E
            let t = gen_f64_bits(F64Shape::Zeros, bins as usize, &mut rng);
            let r = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            for &th in &[5.0e-324, 1.0e-300, 0.25, 0.5, 1.0, 2.0, 1.0e308] {
                diff_match(&t, &r, bins, th, "row01 energy gate");
                // ... and pin the actual sentinel the C returns.
                let mut tv: Vec<f64> = t.iter().map(|&x| f64::from_bits(x)).collect();
                let mut rv: Vec<f64> = r.iter().map(|&x| f64::from_bits(x)).collect();
                let got = unsafe { (both.c.r#match)(tv.as_mut_ptr(), rv.as_mut_ptr(), bins, th) };
                assert_eq!(got, REJECT, "row01: C must reject (bins={bins} thr={th})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 2 -- gate passes, contrast below threshold.
// ---------------------------------------------------------------------------

#[test]
fn err_row02_contrast_below_threshold() {
    let both = both();
    let mut saw_reject = false;
    let mut saw_accept = false;
    for bins in [2, 3, 16, 17, 33] {
        for k in 0..64 {
            let mut rng = Rng::new(0xC002 ^ (bins as u64) << 20 ^ k);
            let t = gen_f64_bits(F64Shape::Peaked, bins as usize, &mut rng);
            let r = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            // threshold 0.0: the gate cannot reject (x < 0*E == 0 only if x<0,
            // and the data is positive), so the verdict comes from the contrast.
            for &th in &[0.0, 0.25, 0.5, 0.9, 1.0] {
                diff_match(&t, &r, bins, th, "row02 contrast verdict");
                let mut tv: Vec<f64> = t.iter().map(|&x| f64::from_bits(x)).collect();
                let mut rv: Vec<f64> = r.iter().map(|&x| f64::from_bits(x)).collect();
                match unsafe { (both.c.r#match)(tv.as_mut_ptr(), rv.as_mut_ptr(), bins, th) } {
                    0 => saw_reject = true,
                    1 => saw_accept = true,
                    other => panic!("match returned {other}, expected 0 or 1"),
                }
            }
        }
    }
    assert!(saw_reject, "row02 never exercised the reject verdict");
    assert!(saw_accept, "row02 never exercised the accept verdict");
}

// ---------------------------------------------------------------------------
// Row 3 -- threshold = NaN: the gate must NOT short-circuit, verdict is 0.
// ---------------------------------------------------------------------------

#[test]
fn err_row03_threshold_nan() {
    let both = both();
    for nan_bits in [
        0x7FF8_0000_0000_0000u64, // default quiet NaN
        0xFFF8_0000_0000_0000,    // negative quiet NaN
        0x7FF0_0000_0000_0001,    // signaling NaN
        0x7FFF_FFFF_FFFF_FFFF,    // all-payload NaN
    ] {
        let th = f64::from_bits(nan_bits);
        for bins in [1, 2, 3, 16, 17, 33] {
            for k in 0..32 {
                let mut rng = Rng::new(0xC003 ^ (bins as u64) << 20 ^ k ^ nan_bits);
                let t = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
                let r = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
                diff_match(&t, &r, bins, th, "row03 NaN threshold");
                let mut tv: Vec<f64> = t.iter().map(|&x| f64::from_bits(x)).collect();
                let mut rv: Vec<f64> = r.iter().map(|&x| f64::from_bits(x)).collect();
                let got = unsafe { (both.c.r#match)(tv.as_mut_ptr(), rv.as_mut_ptr(), bins, th) };
                assert_eq!(got, REJECT, "row03: NaN threshold must yield 0 (bins={bins})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 -- threshold = +inf with finite data: gate rejects.
// ---------------------------------------------------------------------------

#[test]
fn err_row04_threshold_pos_inf() {
    let both = both();
    for bins in [1, 2, 3, 16, 17, 33] {
        for k in 0..32 {
            let mut rng = Rng::new(0xC004 ^ (bins as u64) << 20 ^ k);
            let t = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            let r = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            diff_match(&t, &r, bins, f64::INFINITY, "row04 +inf threshold");
            let mut tv: Vec<f64> = t.iter().map(|&x| f64::from_bits(x)).collect();
            let mut rv: Vec<f64> = r.iter().map(|&x| f64::from_bits(x)).collect();
            let got =
                unsafe { (both.c.r#match)(tv.as_mut_ptr(), rv.as_mut_ptr(), bins, f64::INFINITY) };
            assert_eq!(got, REJECT, "row04: finite < +inf must reject (bins={bins})");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 -- threshold = -inf: nothing is ever rejected by the gate.
// ---------------------------------------------------------------------------

#[test]
fn err_row05_threshold_neg_inf() {
    let both = both();
    for bins in [2, 3, 16, 17, 33] {
        for k in 0..32 {
            let mut rng = Rng::new(0xC005 ^ (bins as u64) << 20 ^ k);
            let t = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            let r = gen_f64_bits(F64Shape::Positive, bins as usize, &mut rng);
            diff_match(&t, &r, bins, f64::NEG_INFINITY, "row05 -inf threshold");
            // -inf accepts everything whose contrast is not NaN.
            let mut tv: Vec<f64> = t.iter().map(|&x| f64::from_bits(x)).collect();
            let mut rv: Vec<f64> = r.iter().map(|&x| f64::from_bits(x)).collect();
            let got = unsafe {
                (both.c.r#match)(tv.as_mut_ptr(), rv.as_mut_ptr(), bins, f64::NEG_INFINITY)
            };
            assert!(got == 0 || got == 1, "row05: unexpected {got}");
        }
    }
    // bins == 1 always produces a NaN contrast, so even -inf is rejected.
    let t = [1.0f64.to_bits()];
    let r = [2.0f64.to_bits()];
    diff_match(&t, &r, 1, f64::NEG_INFINITY, "row05 bins=1 NaN contrast");
    let mut tv = [1.0f64];
    let mut rv = [2.0f64];
    let got =
        unsafe { (both.c.r#match)(tv.as_mut_ptr(), rv.as_mut_ptr(), 1, f64::NEG_INFINITY) };
    assert_eq!(got, REJECT, "row05: NaN >= -inf is false");
}

// ---------------------------------------------------------------------------
// Row 6 -- zero data and zero threshold: gate passes into the 0/0 NaN path.
// ---------------------------------------------------------------------------

#[test]
fn err_row06_zero_data_zero_threshold() {
    let both = both();
    for bins in [1, 2, 3, 16, 17, 33] {
        for &th in &[0.0f64, -0.0] {
            let z = vec![0.0f64.to_bits(); bins as usize];
            diff_match(&z, &z, bins, th, "row06 zeros, zero threshold");
            let mut tv = vec![0.0f64; bins as usize];
            let mut rv = vec![0.0f64; bins as usize];
            let got = unsafe { (both.c.r#match)(tv.as_mut_ptr(), rv.as_mut_ptr(), bins, th) };
            assert_eq!(got, REJECT, "row06: NaN >= 0 is false (bins={bins})");
        }
    }
    // -0.0 data as well
    for bins in [2, 17] {
        let z = vec![(-0.0f64).to_bits(); bins as usize];
        diff_match(&z, &z, bins, 0.0, "row06 negative zeros");
    }
}

// ---------------------------------------------------------------------------
// Row 7 -- the gate's product is NaN (0 * inf, or inf + -inf in `total`).
// ---------------------------------------------------------------------------

#[test]
fn err_row07_gate_nan_product() {
    let both = both();
    // (a) total(reference) == 0 and threshold == +-inf  =>  0 * inf = NaN
    for bins in [1, 2, 3, 16, 17] {
        let t = vec![1.0f64.to_bits(); bins as usize];
        let r = vec![0.0f64.to_bits(); bins as usize];
        for &th in &[f64::INFINITY, f64::NEG_INFINITY] {
            diff_match(&t, &r, bins, th, "row07 0*inf gate");
            let mut tv = vec![1.0f64; bins as usize];
            let mut rv = vec![0.0f64; bins as usize];
            let got = unsafe { (both.c.r#match)(tv.as_mut_ptr(), rv.as_mut_ptr(), bins, th) };
            // A NaN gate product cannot reject: `x < NaN` is false.
            assert!(got == 0 || got == 1, "row07: unexpected {got}");
        }
    }
    // (b) total(reference) itself is NaN: +inf and -inf in the same buffer.
    for bins in [2, 3, 16, 17] {
        let mut r = vec![1.0f64.to_bits(); bins as usize];
        r[0] = f64::INFINITY.to_bits();
        r[1] = f64::NEG_INFINITY.to_bits();
        let t = vec![1.0f64.to_bits(); bins as usize];
        for &th in &[0.0, 0.5, 1.0, f64::INFINITY, f64::NAN] {
            diff_match(&t, &r, bins, th, "row07 inf-inf total");
            diff_match(&r, &t, bins, th, "row07 inf-inf total (swapped)");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 -- spectral_contrast: zero magnitude => division by zero => NaN.
// ---------------------------------------------------------------------------

#[test]
fn err_row08_zero_magnitude_nan() {
    let both = both();
    for len in [1, 2, 3, 16, 17, 64] {
        let zeros = vec![0.0f32.to_bits(); len as usize];
        let mut rng = Rng::new(0xC008 ^ len as u64);
        let other = gen_f32_bits(F32Shape::Normal, len as usize, &mut rng);
        diff_sc(&zeros, &other, len, "row08 a=zeros");
        diff_sc(&other, &zeros, len, "row08 b=zeros");
        diff_sc(&zeros, &zeros, len, "row08 both zeros");

        // Pin the sentinel: C returns NaN and fills the zero buffer with NaN.
        let mut a = vec![0.0f32; len as usize];
        let mut b = vec![1.0f32; len as usize];
        let ret = unsafe { (both.c.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), len) };
        assert!(ret.is_nan(), "row08: C must return NaN, got {ret}");
        assert!(a.iter().all(|x| x.is_nan()), "row08: 0/0 must fill `a` with NaN");
        // ... and it is the x86 "indefinite" QNaN, not an arbitrary one.
        assert_eq!(
            a[0].to_bits(),
            0xFFC0_0000,
            "row08: expected the x86 default QNaN from 0.0/0.0"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 9 -- spectral_contrast: NaN somewhere in the input.
// ---------------------------------------------------------------------------

#[test]
fn err_row09_nan_input() {
    let both = both();
    for len in [1, 2, 3, 16, 17, 64] {
        for pos in 0..len as usize {
            for nan in [0x7FC0_0000u32, 0xFFC0_0000, 0x7F80_0001, 0xFFBF_FFFF] {
                let mut rng = Rng::new(0xC009 ^ len as u64 ^ (pos as u64) << 8);
                let mut a = gen_f32_bits(F32Shape::Normal, len as usize, &mut rng);
                let b = gen_f32_bits(F32Shape::Normal, len as usize, &mut rng);
                a[pos] = nan;
                diff_sc(&a, &b, len, "row09 NaN in a");
                diff_sc(&b, &a, len, "row09 NaN in b");
            }
        }
    }
    // Sentinel: a single NaN poisons the whole result and both buffers.
    let mut a = vec![1.0f32; 8];
    let mut b = vec![1.0f32; 8];
    a[3] = f32::NAN;
    let ret = unsafe { (both.c.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), 8) };
    assert!(ret.is_nan(), "row09: C must return NaN, got {ret}");
    assert!(a.iter().all(|x| x.is_nan()), "row09: NaN magnitude poisons all of `a`");
}

// ---------------------------------------------------------------------------
// Row 10 -- spectral_contrast: magnitude overflows to +inf.
// ---------------------------------------------------------------------------

#[test]
fn err_row10_magnitude_overflow() {
    let both = both();
    for len in [1, 2, 3, 16, 17, 64] {
        for k in 0..64 {
            let mut rng = Rng::new(0xC010 ^ (len as u64) << 16 ^ k);
            let a = gen_f32_bits(F32Shape::Huge, len as usize, &mut rng);
            let b = gen_f32_bits(F32Shape::Huge, len as usize, &mut rng);
            diff_sc(&a, &b, len, "row10 overflow");
            let n = gen_f32_bits(F32Shape::Normal, len as usize, &mut rng);
            diff_sc(&a, &n, len, "row10 overflow vs normal");
        }
    }
    // Sentinel: with `length >= 2` the f32 products overflow, sqrt(+inf) = +inf,
    // every element becomes +-0.0 and the contrast is +-0.0 (not an error code).
    let mut a = vec![3.0e38f32; 4];
    let mut b = vec![3.0e38f32; 4];
    let ret = unsafe { (both.c.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), 4) };
    assert_eq!(ret.to_bits(), 0.0f64.to_bits(), "row10: expected +0.0, got {ret}");
    assert!(a.iter().all(|x| *x == 0.0), "row10: data destroyed to zeros");
}

// ---------------------------------------------------------------------------
// Rows 11-13 -- the size / pointer domain boundaries of spectral_contrast.
// ---------------------------------------------------------------------------

#[test]
fn err_row11_sc_length_zero() {
    let both = both();
    let mut rng = Rng::new(0xC011);
    for n in [0usize, 1, 4, 16] {
        let a = gen_f32_bits(F32Shape::Normal, n, &mut rng);
        let b = gen_f32_bits(F32Shape::Normal, n, &mut rng);
        diff_sc(&a, &b, 0, "row11 length=0");
    }
    // Sentinel: exactly +0.0, and the buffers are untouched.
    let mut a = [1.5f32, 2.5, 3.5];
    let mut b = [4.5f32, 5.5, 6.5];
    let ret = unsafe { (both.c.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), 0) };
    assert_eq!(ret.to_bits(), 0.0f64.to_bits(), "row11: expected +0.0, got {ret}");
    assert_eq!(a, [1.5, 2.5, 3.5], "row11: buffer must be untouched");
    assert_eq!(b, [4.5, 5.5, 6.5], "row11: buffer must be untouched");
}

#[test]
fn err_row12_sc_length_negative() {
    let both = both();
    let mut rng = Rng::new(0xC012);
    for len in [-1i32, -2, -16, -1000, i32::MIN, i32::MIN + 1] {
        for n in [0usize, 1, 8] {
            let a = gen_f32_bits(F32Shape::RawBits, n, &mut rng);
            let b = gen_f32_bits(F32Shape::RawBits, n, &mut rng);
            diff_sc(&a, &b, len, &format!("row12 length={len}"));
        }
        let mut a = [1.5f32, 2.5];
        let mut b = [3.5f32, 4.5];
        let ret = unsafe { (both.c.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), len) };
        assert_eq!(ret.to_bits(), 0.0f64.to_bits(), "row12: expected +0.0 for length={len}");
        assert_eq!(a, [1.5, 2.5], "row12: buffer must be untouched (length={len})");
        assert_eq!(b, [3.5, 4.5], "row12: buffer must be untouched (length={len})");
    }
}

#[test]
fn err_row13_sc_null_ptrs_len_le_zero() {
    let both = both();
    for len in [0i32, -1, -7, i32::MIN] {
        diff_sc_raw_ptrs(std::ptr::null_mut(), std::ptr::null_mut(), len, "row13 both null");
        let mut real = [1.0f32, 2.0, 3.0, 4.0];
        diff_sc_raw_ptrs(std::ptr::null_mut(), real.as_mut_ptr(), len, "row13 a null");
        diff_sc_raw_ptrs(real.as_mut_ptr(), std::ptr::null_mut(), len, "row13 b null");
        // Sentinel: +0.0, no dereference.
        let ret = unsafe {
            (both.c.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), len)
        };
        assert_eq!(ret.to_bits(), 0.0f64.to_bits(), "row13: expected +0.0 for length={len}");
    }
}

// ---------------------------------------------------------------------------
// Rows 14-15 -- unguarded aliasing.
// ---------------------------------------------------------------------------

#[test]
fn err_row14_sc_aliased_buffers() {
    let both = both();
    for len in [1, 2, 3, 16, 17, 64] {
        for k in 0..64 {
            let mut rng = Rng::new(0xC014 ^ (len as u64) << 16 ^ k);
            for shape in [F32Shape::Normal, F32Shape::RawBits, F32Shape::Denormal, F32Shape::Huge] {
                let a = gen_f32_bits(shape, len as usize, &mut rng);
                diff_sc_aliased(&a, len, &format!("row14 aliased shape={shape:?}"));
            }
        }
    }
    // Sentinel: normalizing twice leaves a (near-)unit vector, so the contrast
    // lands on 1.0 up to the `float` rounding of the second normalisation --
    // and both libraries must land on the *same* value, bit for bit.
    let mut v = [3.0f32, 4.0];
    let p = v.as_mut_ptr();
    let c_ret = unsafe { (both.c.spectral_contrast)(p, p, 2) };
    let mut v2 = [3.0f32, 4.0];
    let p2 = v2.as_mut_ptr();
    let rs_ret = unsafe { (both.rs.spectral_contrast)(p2, p2, 2) };
    assert_eq!(c_ret.to_bits(), rs_ret.to_bits(), "row14 sentinel divergence");
    assert_eq!(v.map(f32::to_bits), v2.map(f32::to_bits), "row14 sentinel buffer divergence");
    assert!(
        (c_ret - 1.0).abs() < 1.0e-6,
        "row14: aliased contrast must be ~1.0, got {c_ret}"
    );
}

#[test]
fn err_row15_match_aliased_buffers() {
    let both = both();
    for bins in [1, 2, 3, 16, 17, 33] {
        for k in 0..64 {
            let mut rng = Rng::new(0xC015 ^ (bins as u64) << 16 ^ k);
            for shape in [F64Shape::Positive, F64Shape::RawBits, F64Shape::Peaked] {
                let v = gen_f64_bits(shape, bins as usize, &mut rng);
                for &th in THRESHOLDS {
                    diff_match_aliased(&v, bins, th, &format!("row15 aliased shape={shape:?}"));
                }
            }
        }
    }
    // Sentinel: identical spectra, threshold 1.0 -> contrast is exactly 1.0.
    let mut v: Vec<f64> = (0..32).map(|i| 1.0 + (i as f64) * 0.5).collect();
    let p = v.as_mut_ptr();
    let got = unsafe { (both.c.r#match)(p, p, 32, 1.0) };
    let got_rs = unsafe { (both.rs.r#match)(p, p, 32, 1.0) };
    assert_eq!(got, got_rs, "row15 sentinel divergence");
}

// ---------------------------------------------------------------------------
// Rows 16-17 -- `bins <= 0`: outside the defined domain of the C.
//
// `float_t t[bins]` is a zero/negative-size VLA and `differentiate` stores to
// `v[-1]`, so there is no defined C result to match. These tests
//   (a) verify the Rust `.so` is memory-safe and deterministic there, and
//   (b) record the C behaviour observed in a *subprocess*, so a SIGSEGV in the
//       C library cannot take the test runner down with it.
// ---------------------------------------------------------------------------

/// Re-executes this test binary to run `ub_probe_child` against one library.
fn probe_subprocess(which: &str, bins: i32) -> (Option<i32>, Option<i32>, String) {
    probe_subprocess_mode(which, bins, "bins")
}

fn probe_subprocess_mode(
    which: &str,
    bins: i32,
    mode: &str,
) -> (Option<i32>, Option<i32>, String) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["ub_probe_child", "--exact", "--nocapture", "--test-threads=1"])
        .env("UB_PROBE_IMPL", which)
        .env("UB_PROBE_BINS", bins.to_string())
        .env("UB_PROBE_MODE", mode)
        .output()
        .expect("spawn ub probe");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    // libtest prints `test ub_probe_child ... ` without a trailing newline, so
    // the marker can appear mid-line -- search for it as a substring.
    let ret = text.find("UB_PROBE_RESULT=").and_then(|i| {
        text[i + "UB_PROBE_RESULT=".len()..]
            .split(|c: char| c.is_whitespace())
            .next()
            .and_then(|v| v.parse::<i32>().ok())
    });
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;
    (ret, signal, text)
}

/// The child half of `probe_subprocess`; a no-op during a normal test run.
#[test]
fn ub_probe_child() {
    let Ok(bins) = std::env::var("UB_PROBE_BINS") else { return };
    let bins: i32 = bins.parse().expect("UB_PROBE_BINS");
    let mode = std::env::var("UB_PROBE_MODE").unwrap_or_else(|_| "bins".to_string());
    let which = std::env::var("UB_PROBE_IMPL").unwrap_or_else(|_| "rs".to_string());
    let both = both();
    let imp = if which == "c" { &both.c } else { &both.rs };
    let ret = match mode.as_str() {
        // ERRORS rows 16/17: degenerate `bins` with small, valid buffers.
        "bins" => {
            let mut t = vec![1.0f64; 8];
            let mut r = vec![2.0f64; 8];
            unsafe { (imp.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins, 0.5) }
        }
        // ERRORS row 19: null buffers with a positive `bins`.
        "null" => unsafe {
            (imp.r#match)(std::ptr::null_mut(), std::ptr::null_mut(), bins, 0.5)
        },
        // ERRORS row 18: `bins` large enough that two VLAs blow the stack.
        "big" => {
            let mut t = vec![1.0f64; bins as usize];
            let mut r = vec![2.0f64; bins as usize];
            for i in 0..bins as usize {
                t[i] = 1.0 + (i % 97) as f64;
                r[i] = 2.0 + (i % 89) as f64;
            }
            unsafe { (imp.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins, 0.5) }
        }
        other => panic!("unknown UB_PROBE_MODE {other:?}"),
    };
    println!("UB_PROBE_RESULT={ret}");
    use std::io::Write;
    std::io::stdout().flush().ok();
    std::process::exit(0);
}

#[test]
fn err_row16_match_bins_zero_is_ub() {
    let (c_ret, c_sig, _) = probe_subprocess("c", 0);
    let (rs_ret, rs_sig, txt) = probe_subprocess("rs", 0);
    println!("bins=0: C ret={c_ret:?} signal={c_sig:?} | Rust ret={rs_ret:?} signal={rs_sig:?}");
    // The Rust translation must stay memory-safe and produce a verdict.
    assert!(rs_sig.is_none(), "Rust must not crash for bins=0 (signal {rs_sig:?})\n{txt}");
    assert!(rs_ret.is_some(), "Rust must return a verdict for bins=0\n{txt}");
    assert!(
        matches!(rs_ret, Some(0) | Some(1)),
        "Rust must return 0/1 for bins=0, got {rs_ret:?}"
    );
    // Document the C: it either dies or yields stack-layout-dependent garbage.
    // (Observed on this platform: SIGSEGV, because `differentiate` writes v[-1].)
    if c_sig.is_none() && c_ret != rs_ret {
        println!(
            "NOTE: C survived bins=0 with {c_ret:?} while Rust returned {rs_ret:?}; \
             `float_t t[0]` + the `v[-1]` store are undefined behaviour, so there is \
             no defined C result to match here (see ERRORS.md row 16)."
        );
    }
}

#[test]
fn err_row17_match_bins_negative_is_ub() {
    for bins in [-1i32, -2, -3, -4, -5, -8, -16] {
        let (c_ret, c_sig, _) = probe_subprocess("c", bins);
        let (rs_ret, rs_sig, txt) = probe_subprocess("rs", bins);
        println!(
            "bins={bins}: C ret={c_ret:?} signal={c_sig:?} | Rust ret={rs_ret:?} signal={rs_sig:?}"
        );
        assert!(rs_sig.is_none(), "Rust must not crash for bins={bins} (signal {rs_sig:?})\n{txt}");
        assert!(
            matches!(rs_ret, Some(0) | Some(1)),
            "Rust must return 0/1 for bins={bins}, got {rs_ret:?}"
        );
    }
    // Rust must at least be *deterministic* and safe across repeats.
    let first = probe_subprocess("rs", -5).0;
    for _ in 0..3 {
        assert_eq!(probe_subprocess("rs", -5).0, first, "Rust must be deterministic for bins<0");
    }
}

// ---------------------------------------------------------------------------
// Row 18 -- `bins` so large that the two VLAs exhaust the caller's stack.
//
// This is a property of the *caller's* stack limit, not of the library's logic:
// on a small stack the C dies, on a big enough stack it must agree with Rust.
// Both halves are asserted.
// ---------------------------------------------------------------------------

const BIG_BINS: i32 = 300_000; // 2 * 300000 * 8 = 4.8 MB of VLA

#[test]
fn err_row18_match_huge_bins() {
    // (a) Default 2 MiB test-thread stack: the C library cannot survive.
    let (c_ret, c_sig, _) = probe_subprocess_mode("c", BIG_BINS, "big");
    let (rs_ret, rs_sig, txt) = probe_subprocess_mode("rs", BIG_BINS, "big");
    println!(
        "bins={BIG_BINS} on the default stack: C ret={c_ret:?} signal={c_sig:?} | \
         Rust ret={rs_ret:?} signal={rs_sig:?}"
    );
    assert!(rs_sig.is_none(), "Rust must not crash for bins={BIG_BINS}\n{txt}");
    assert!(
        matches!(rs_ret, Some(0) | Some(1)),
        "Rust must return 0/1 for bins={BIG_BINS}, got {rs_ret:?}"
    );
    if c_sig.is_some() {
        println!(
            "NOTE: the C library died with signal {c_sig:?} -- `float_t t[300000]` \
             overflows the default thread stack. Rust heap-allocates its VLA stand-in \
             and survives (ERRORS.md row 18)."
        );
    }

    // (b) On a stack that *can* hold the VLAs, the two must agree exactly.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let mut rng = Rng::new(0xC018);
            let t = gen_f64_bits(F64Shape::Peaked, BIG_BINS as usize, &mut rng);
            let r = gen_f64_bits(F64Shape::Peaked, BIG_BINS as usize, &mut rng);
            for &th in &[0.0f64, 0.5, 1.0] {
                diff_match(&t, &r, BIG_BINS, th, "row18 big bins, big stack");
            }
            diff_match_boundary(&t, &r, BIG_BINS, "row18 big bins boundary");
        })
        .expect("spawn big-stack thread")
        .join()
        .expect("big-stack thread panicked");
}

// ---------------------------------------------------------------------------
// Row 19 -- null buffers with a positive `bins`: `total()` dereferences them.
// Both libraries must fail the same way (same fatal signal).
// ---------------------------------------------------------------------------

#[test]
fn err_row19_match_null_pointers() {
    for bins in [1i32, 2, 16, 17] {
        let (c_ret, c_sig, _) = probe_subprocess_mode("c", bins, "null");
        let (rs_ret, rs_sig, txt) = probe_subprocess_mode("rs", bins, "null");
        println!(
            "null buffers, bins={bins}: C ret={c_ret:?} signal={c_sig:?} | \
             Rust ret={rs_ret:?} signal={rs_sig:?}"
        );
        // Neither may survive: `total()` dereferences the null pointer.
        assert_eq!(c_ret, None, "C must not return a value for null buffers (bins={bins})");
        assert_eq!(rs_ret, None, "Rust must not return a value for null buffers (bins={bins})");
        assert_eq!(c_sig, Some(11), "C is expected to take SIGSEGV (bins={bins})");

        if cfg!(debug_assertions) {
            // A debug build compiles in Rust's `ub_checks` null-dereference
            // assertion, which panics; unwinding out of an `extern "C"` function
            // then aborts, so the fatal signal is SIGABRT (6) rather than
            // SIGSEGV (11). That check is a *debug-only* diagnostic; it is
            // disabled in the deliverable profile (see `[profile.release]`
            // `debug-assertions = false` in Cargo.toml), which is asserted below.
            assert!(
                rs_sig == Some(6) || rs_sig == Some(11),
                "Rust must die on a null dereference (bins={bins}), got {rs_sig:?}\n{txt}"
            );
        } else {
            assert_eq!(
                c_sig, rs_sig,
                "with UB checks disabled the null-pointer failure must be identical \
                 for bins={bins}: C signal {c_sig:?} vs Rust signal {rs_sig:?}\n{txt}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary checks (CONFIGS row 40).
//
// `bins`/`length` are plain `int`s and the API has no enum parameters, so the
// "out-of-range enum value" class reduces to out-of-range integer sizes. Every
// value that C survives is compared; the ones it cannot survive are in rows
// 16-19 of ERRORS.md.
// ---------------------------------------------------------------------------

#[test]
fn err_row40_boundary_sizes_and_pointers() {
    // spectral_contrast tolerates every non-positive length, including the
    // extreme int values, with any pointer whatsoever.
    for len in [0i32, -1, -2, -127, -128, -32768, i32::MIN, i32::MIN + 1, -1_000_000] {
        diff_sc_raw_ptrs(std::ptr::null_mut(), std::ptr::null_mut(), len, "row40 null+nonpos");
        let mut a = [7.0f32; 2];
        let mut b = [9.0f32; 2];
        diff_sc_raw_ptrs(a.as_mut_ptr(), b.as_mut_ptr(), len, "row40 real+nonpos");
        // dangling-but-aligned pointers are never dereferenced either
        diff_sc_raw_ptrs(8usize as *mut f32, 16usize as *mut f32, len, "row40 dangling+nonpos");
    }
    // Smallest legal positive length, at the very edges of the value range.
    for bits in [0x0000_0001u32, 0x007F_FFFF, 0x0080_0000, 0x7F7F_FFFF, 0x7F80_0000, 0xFF80_0000] {
        diff_sc(&[bits], &[bits], 1, "row40 extreme f32 magnitudes");
        diff_sc(&[bits, bits], &[bits, 0], 2, "row40 extreme f32 magnitudes len=2");
    }
    // `match` at its smallest legal `bins`, with extreme f64 magnitudes.
    for bits in [
        0x0000_0000_0000_0001u64,
        0x000F_FFFF_FFFF_FFFF,
        0x0010_0000_0000_0000,
        0x7FEF_FFFF_FFFF_FFFF,
        0x7FF0_0000_0000_0000,
        0xFFF0_0000_0000_0000,
    ] {
        for bins in [1, 2, 3, 17] {
            let v = vec![bits; bins as usize];
            for &th in THRESHOLDS {
                diff_match(&v, &v, bins, th, "row40 extreme f64 magnitudes");
            }
        }
    }
}
