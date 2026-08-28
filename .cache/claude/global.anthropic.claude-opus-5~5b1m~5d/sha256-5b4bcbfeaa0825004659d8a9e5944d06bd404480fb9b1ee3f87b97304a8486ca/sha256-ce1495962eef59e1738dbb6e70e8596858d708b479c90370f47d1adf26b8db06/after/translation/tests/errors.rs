//! Phase C — error/rejection-path differential tests, one per ERRORS.md row.
//!
//! Rows whose trigger makes the *C* library die (undefined behaviour: the
//! zero-length-VLA out-of-bounds store, the wrapped-around `memcpy` size, null
//! dereferences, stack exhaustion) are run in a **child process** so a
//! `SIGSEGV` does not take the test binary with it. Everything else runs
//! in-process and is compared bit-for-bit.

mod common;

use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

use common::*;

// ===========================================================================
// In-process helpers for raw / null pointer calls
// ===========================================================================

fn sc_raw(api: &Api, a: *mut f32, b: *mut f32, len: c_int) -> u64 {
    unsafe { (api.spectral_contrast)(a, b, len).to_bits() }
}

fn match_raw(api: &Api, t: *mut f64, r: *mut f64, bins: c_int, thr: f64) -> c_int {
    unsafe { (api.matchfn)(t, r, bins, thr) }
}

// ===========================================================================
// E1 / E2 / E3 / E5 / E6 — the one explicit `if` in the library
// ===========================================================================

/// E1: the energy gate rejects, and it does so *before* any preprocessing.
///
/// `test` is scaled far below `reference`, so `total(test) < threshold *
/// total(reference)` for any `threshold >= 1`.
#[test]
fn e1_energy_gate_rejects() {
    let p = pair();
    let mut rng = Rng::new(101);
    for &bins in &[1i32, 2, 3, 16, 17, 33, 64] {
        let n = bins as usize;
        for _ in 0..200 {
            let refv: Vec<f64> = (0..n).map(|_| 1.0 + rng.unit()).collect();
            let mut buf: Vec<u64> = refv.iter().map(|v| (v * 1e-12).to_bits()).collect();
            buf.extend(refv.iter().map(|v| v.to_bits()));
            for &thr in &[1.0f64, 2.0, 1e6] {
                let c = match_call(&p.c, &buf, 0, n, bins, thr);
                let r = match_call(&p.rust, &buf, 0, n, bins, thr);
                assert_match_eq("E1", &format!("bins={bins} thr={thr}"), &buf, &c, &r);
                assert_eq!(c.ret, 0, "E1: C should have taken the `return 0` gate");
            }
        }
    }
}

/// E2: `threshold * total(reference)` overflows to `+inf`.
#[test]
fn e2_energy_gate_product_overflows() {
    let p = pair();
    for &bins in &[1i32, 2, 16, 17, 64] {
        let n = bins as usize;
        let mut buf: Vec<u64> = (0..n).map(|_| 1.0f64.to_bits()).collect();
        buf.extend((0..n).map(|_| 1e300f64.to_bits()));
        for &thr in &[f64::MAX, 1e300, f64::INFINITY] {
            let c = match_call(&p.c, &buf, 0, n, bins, thr);
            let r = match_call(&p.rust, &buf, 0, n, bins, thr);
            assert_match_eq("E2", &format!("bins={bins} thr={thr}"), &buf, &c, &r);
            assert_eq!(c.ret, 0, "E2: overflowing product must reject");
        }
    }
}

/// E3: `threshold * total(reference)` is NaN (`+inf * 0`), so the *ordered* `<`
/// is false and the gate does **not** reject — the call falls through to the
/// contrast test, which then also fails because the contrast is compared
/// against `+inf`.
#[test]
fn e3_gate_product_is_nan_falls_through() {
    let p = pair();
    for &bins in &[1i32, 2, 3, 16, 17, 64] {
        let n = bins as usize;
        let mut buf: Vec<u64> = (0..n).map(|i| ((i + 1) as f64).to_bits()).collect();
        buf.extend((0..n).map(|_| 0.0f64.to_bits())); // total(reference) == 0
        for &thr in &[f64::INFINITY, f64::NEG_INFINITY] {
            let c = match_call(&p.c, &buf, 0, n, bins, thr);
            let r = match_call(&p.rust, &buf, 0, n, bins, thr);
            assert_match_eq("E3", &format!("bins={bins} thr={thr}"), &buf, &c, &r);
            assert_eq!(c.ret, 0, "E3");
        }
    }
}

/// E4: `threshold` itself is NaN. Both ordered comparisons are false, so the
/// gate does not reject but the final `>=` does: the result is always `0`.
#[test]
fn e4_threshold_nan() {
    let p = pair();
    let mut rng = Rng::new(104);
    let nans: [u64; 4] = [
        0x7FF8_0000_0000_0000,
        0xFFF8_0000_0000_0000,
        0x7FF0_0000_0000_0001, // sNaN
        0x7FFF_FFFF_FFFF_FFFF,
    ];
    for &bins in &[1i32, 2, 3, 16, 17, 64] {
        let n = bins as usize;
        for _ in 0..100 {
            let buf: Vec<u64> = (0..2 * n).map(|_| rng.pos_normal_f64().to_bits()).collect();
            for &nb in &nans {
                let thr = f64::from_bits(nb);
                let c = match_call(&p.c, &buf, 0, n, bins, thr);
                let r = match_call(&p.rust, &buf, 0, n, bins, thr);
                assert_match_eq("E4", &format!("bins={bins} thr={nb:#018x}"), &buf, &c, &r);
                assert_eq!(c.ret, 0, "E4: NaN threshold must yield 0");
            }
        }
    }
}

/// E5: `total(test)` is NaN, so `NaN < x` is false and the gate does not reject.
#[test]
fn e5_total_test_is_nan() {
    let p = pair();
    let mut rng = Rng::new(105);
    for &bins in &[1i32, 2, 3, 16, 17, 64] {
        let n = bins as usize;
        for _ in 0..200 {
            let mut buf: Vec<u64> = (0..2 * n).map(|_| rng.pos_normal_f64().to_bits()).collect();
            let i = rng.below(n);
            buf[i] = rng.qnan_f64_bits();
            for &thr in &[0.0f64, 0.5, 1.0, 1e6] {
                let c = match_call(&p.c, &buf, 0, n, bins, thr);
                let r = match_call(&p.rust, &buf, 0, n, bins, thr);
                assert_match_eq("E5", &format!("bins={bins} i={i} thr={thr}"), &buf, &c, &r);
            }
        }
    }
}

/// E6: the *second* rejection — the contrast cut-off. `threshold` is set just
/// above the achievable contrast (`1.0`), so the gate passes but the cut-off
/// rejects.
#[test]
fn e6_contrast_cutoff_rejects() {
    let p = pair();
    let mut rng = Rng::new(106);
    for &bins in &[2i32, 3, 16, 17, 33, 64] {
        let n = bins as usize;
        for _ in 0..200 {
            let half: Vec<f64> = (0..n).map(|_| 1.0 + rng.unit()).collect();
            let mut buf: Vec<u64> = half.iter().map(|v| v.to_bits()).collect();
            buf.extend(half.iter().map(|v| v.to_bits()));
            // threshold slightly above 1.0: energy gate passes (ratio == 1),
            // contrast cut-off cannot be met (contrast <= 1).
            for &thr in &[1.0000001f64, 1.5, 2.0] {
                let c = match_call(&p.c, &buf, 0, n, bins, thr);
                let r = match_call(&p.rust, &buf, 0, n, bins, thr);
                assert_match_eq("E6", &format!("bins={bins} thr={thr}"), &buf, &c, &r);
            }
        }
    }
}

/// E7: the contrast is NaN because a preprocessed buffer normalises to `0/0`.
/// All-zero input makes `total == 0` (gate passes for `threshold <= 0`, and for
/// `threshold > 0` because `0 < 0` is false), then `differentiate` leaves all
/// zeroes, so `sqrt(dot) == 0` and every element becomes a NaN.
#[test]
fn e7_zero_magnitude_gives_nan_contrast() {
    let p = pair();
    for &bins in &[1i32, 2, 3, 15, 16, 17, 33, 64, 257] {
        let n = bins as usize;
        for &z in &[0u64, 0x8000_0000_0000_0000u64] {
            let buf: Vec<u64> = vec![z; 2 * n];
            for thr in thresholds() {
                let c = match_call(&p.c, &buf, 0, n, bins, thr);
                let r = match_call(&p.rust, &buf, 0, n, bins, thr);
                assert_match_eq("E7", &format!("bins={bins} z={z:#x} thr={thr}"), &buf, &c, &r);
            }
        }
    }
}

// ===========================================================================
// E14 / E15 / E18 / E33 — non-positive lengths for `spectral_contrast`
// (these are the *safe* out-of-range values: the loop guards are `i < length`
// with `i` starting at 0, so nothing is dereferenced)
// ===========================================================================

#[test]
fn e14_sc_length_zero() {
    let p = pair();
    let mut rng = Rng::new(114);
    for _ in 0..200 {
        let buf: Vec<u32> = (0..8).map(|_| rng.any_f32_bits()).collect();
        let c = sc_call(&p.c, &buf, 0, 4, 0);
        let r = sc_call(&p.rust, &buf, 0, 4, 0);
        assert_sc_eq("E14", "length=0", &buf, &c, &r);
        assert_eq!(c.ret, 0u64, "E14: C must return +0.0 for length == 0");
        assert_eq!(&c.buf[..], &buf[..], "E14: buffer must be untouched");
    }
}

#[test]
fn e15_sc_negative_length() {
    let p = pair();
    let mut rng = Rng::new(115);
    let lens: [c_int; 7] = [-1, -2, -7, -16, -1000, c_int::MIN, c_int::MIN + 1];
    for &len in &lens {
        for _ in 0..100 {
            let buf: Vec<u32> = (0..8).map(|_| rng.any_f32_bits()).collect();
            let c = sc_call(&p.c, &buf, 0, 4, len);
            let r = sc_call(&p.rust, &buf, 0, 4, len);
            assert_sc_eq("E15", &format!("length={len}"), &buf, &c, &r);
            assert_eq!(c.ret, 0u64, "E15: C must return +0.0 for length {len}");
            assert_eq!(&c.buf[..], &buf[..], "E15: buffer must be untouched");
        }
    }
}

/// E18: null pointers are fine as long as `length <= 0`, because neither loop
/// body executes. Both libraries must return `+0.0` rather than crash.
#[test]
fn e18_sc_null_pointers_with_nonpositive_length() {
    let p = pair();
    let lens: [c_int; 6] = [0, -1, -2, -1000, c_int::MIN, c_int::MIN + 1];
    for &len in &lens {
        let cr = sc_raw(&p.c, std::ptr::null_mut(), std::ptr::null_mut(), len);
        let rr = sc_raw(&p.rust, std::ptr::null_mut(), std::ptr::null_mut(), len);
        assert_eq!(cr, rr, "E18: NULL/NULL length={len}");
        assert_eq!(cr, 0u64, "E18: must be +0.0");

        let mut one = [1.0f32; 1];
        let q = one.as_mut_ptr();
        assert_eq!(
            sc_raw(&p.c, std::ptr::null_mut(), q, len),
            sc_raw(&p.rust, std::ptr::null_mut(), q, len),
            "E18: NULL/valid length={len}"
        );
        assert_eq!(
            sc_raw(&p.c, q, std::ptr::null_mut(), len),
            sc_raw(&p.rust, q, std::ptr::null_mut(), len),
            "E18: valid/NULL length={len}"
        );
    }
}

// ===========================================================================
// E19..E24 — degenerate magnitudes (the unguarded division in `normalize`)
// ===========================================================================

#[test]
fn e19_sc_zero_magnitude_on_a() {
    let p = pair();
    let mut rng = Rng::new(119);
    for &len in LENGTHS {
        let n = len as usize;
        let mut buf: Vec<u32> = vec![0x0000_0000; n];
        buf.extend((0..n).map(|_| rng.signed_normal_f32().to_bits()));
        let c = sc_call(&p.c, &buf, 0, n, len);
        let r = sc_call(&p.rust, &buf, 0, n, len);
        assert_sc_eq("E19", &format!("len={len}"), &buf, &c, &r);
        // The x86 default QNaN produced by 0.0/0.0 has its sign bit SET.
        for i in 0..n {
            assert_eq!(c.buf[i], 0xFFC0_0000, "E19: C a[{i}] should be the default QNaN");
        }
        assert_eq!(c.ret, 0xFFF8_0000_0000_0000, "E19: return should be -NaN");
    }
}

#[test]
fn e20_sc_zero_magnitude_on_b_only() {
    let p = pair();
    let mut rng = Rng::new(120);
    for &len in LENGTHS {
        let n = len as usize;
        let mut buf: Vec<u32> = (0..n).map(|_| rng.signed_normal_f32().to_bits()).collect();
        buf.extend(std::iter::repeat(0x0000_0000u32).take(n));
        let c = sc_call(&p.c, &buf, 0, n, len);
        let r = sc_call(&p.rust, &buf, 0, n, len);
        assert_sc_eq("E20", &format!("len={len}"), &buf, &c, &r);
    }
}

#[test]
fn e21_sc_negative_zero_elements() {
    let p = pair();
    for &len in LENGTHS {
        let n = len as usize;
        for &(za, zb) in &[
            (0x8000_0000u32, 0x8000_0000u32),
            (0x8000_0000, 0x0000_0000),
            (0x0000_0000, 0x8000_0000),
        ] {
            let mut buf: Vec<u32> = vec![za; n];
            buf.extend(std::iter::repeat(zb).take(n));
            let c = sc_call(&p.c, &buf, 0, n, len);
            let r = sc_call(&p.rust, &buf, 0, n, len);
            assert_sc_eq("E21", &format!("len={len} za={za:#x} zb={zb:#x}"), &buf, &c, &r);
            for i in 0..n {
                assert_eq!(c.buf[i], 0xFFC0_0000, "E21: C a[{i}]");
            }
        }
    }
}

#[test]
fn e22_sc_subnormal_underflowing_magnitude() {
    let p = pair();
    let mut rng = Rng::new(122);
    for &len in LENGTHS {
        let n = len as usize;
        for _ in 0..100 {
            // exponent 0, tiny mantissa: x*x underflows f32 to +0 => magnitude 0
            let g = |r: &mut Rng| {
                let sign = (r.next_u32() & 1) << 31;
                let m = 1 + (r.next_u32() % 64);
                sign | m
            };
            let buf: Vec<u32> = (0..2 * n).map(|_| g(&mut rng)).collect();
            let c = sc_call(&p.c, &buf, 0, n, len);
            let r = sc_call(&p.rust, &buf, 0, n, len);
            assert_sc_eq("E22", &format!("len={len}"), &buf, &c, &r);
        }
    }
}

#[test]
fn e23_sc_overflowing_magnitude() {
    let p = pair();
    let mut rng = Rng::new(123);
    for &len in LENGTHS {
        let n = len as usize;
        for _ in 0..100 {
            let g = |r: &mut Rng| {
                let sign = (r.next_u32() & 1) << 31;
                let exp = (250 + (r.next_u32() % 5)) << 23;
                sign | exp | (r.next_u32() & 0x007F_FFFF)
            };
            let buf: Vec<u32> = (0..2 * n).map(|_| g(&mut rng)).collect();
            let c = sc_call(&p.c, &buf, 0, n, len);
            let r = sc_call(&p.rust, &buf, 0, n, len);
            assert_sc_eq("E23", &format!("len={len}"), &buf, &c, &r);
        }
    }
}

#[test]
fn e24_sc_infinities() {
    let p = pair();
    let mut rng = Rng::new(124);
    for &len in LENGTHS {
        let n = len as usize;
        for pattern in 0..4 {
            let mut buf: Vec<u32> = (0..2 * n).map(|_| rng.signed_normal_f32().to_bits()).collect();
            match pattern {
                0 => buf[0] = 0x7F80_0000,
                1 => buf[0] = 0xFF80_0000,
                2 => {
                    for i in 0..n {
                        buf[i] = 0x7F80_0000;
                    }
                }
                _ => {
                    for i in 0..2 * n {
                        buf[i] = if i % 2 == 0 { 0x7F80_0000 } else { 0xFF80_0000 };
                    }
                }
            }
            let c = sc_call(&p.c, &buf, 0, n, len);
            let r = sc_call(&p.rust, &buf, 0, n, len);
            assert_sc_eq("E24", &format!("len={len} pattern={pattern}"), &buf, &c, &r);
        }
    }
}

// ===========================================================================
// E25..E28 — NaN handling and payload precedence
// ===========================================================================

#[test]
fn e25_sc_quiet_nan_passthrough() {
    let p = pair();
    let mut rng = Rng::new(125);
    for &len in LENGTHS {
        let n = len as usize;
        for _ in 0..300 {
            let mut buf: Vec<u32> = (0..2 * n).map(|_| rng.signed_normal_f32().to_bits()).collect();
            let i = rng.below(n);
            buf[i] = rng.qnan_f32_bits();
            let c = sc_call(&p.c, &buf, 0, n, len);
            let r = sc_call(&p.rust, &buf, 0, n, len);
            assert_sc_eq("E25", &format!("len={len} i={i}"), &buf, &c, &r);
        }
    }
}

#[test]
fn e26_sc_signalling_nan_is_quieted() {
    let p = pair();
    let mut rng = Rng::new(126);
    let fixed: [u32; 4] = [0x7F80_0001, 0x7FA0_0001, 0xFF80_0001, 0xFFBF_FFFF];
    for &len in LENGTHS {
        let n = len as usize;
        for &s in &fixed {
            let mut buf: Vec<u32> = (0..2 * n).map(|_| rng.signed_normal_f32().to_bits()).collect();
            buf[0] = s;
            let c = sc_call(&p.c, &buf, 0, n, len);
            let r = sc_call(&p.rust, &buf, 0, n, len);
            assert_sc_eq("E26", &format!("len={len} snan={s:#010x}"), &buf, &c, &r);
            // The `divsd` in `normalize` quiets it in place.
            assert_eq!(c.buf[0], s | 0x0040_0000, "E26: sNaN must be quieted in place");
        }
        for _ in 0..200 {
            let mut buf: Vec<u32> = (0..2 * n).map(|_| rng.signed_normal_f32().to_bits()).collect();
            let i = rng.below(n);
            buf[i] = rng.snan_f32_bits();
            let c = sc_call(&p.c, &buf, 0, n, len);
            let r = sc_call(&p.rust, &buf, 0, n, len);
            assert_sc_eq("E26", &format!("len={len} random sNaN i={i}"), &buf, &c, &r);
        }
    }
}

/// E27: NaN in `a[i]` *and* `b[i]` with different payloads. `mulss`'s
/// destination operand is `b[i]`, so `b`'s payload survives.
#[test]
fn e27_sc_nan_payload_precedence_in_mulss() {
    let p = pair();
    // The exact case that distinguishes -O0 from -O2.
    let buf: Vec<u32> = vec![0x7FC0_0001, 0x7FC0_0002];
    let c = sc_call(&p.c, &buf, 0, 1, 1);
    let r = sc_call(&p.rust, &buf, 0, 1, 1);
    assert_sc_eq("E27", "a=0x7FC00001 b=0x7FC00002 len=1", &buf, &c, &r);

    let mut rng = Rng::new(127);
    for &len in LENGTHS {
        let n = len as usize;
        for _ in 0..400 {
            let mut buf: Vec<u32> = (0..2 * n).map(|_| rng.signed_normal_f32().to_bits()).collect();
            let i = rng.below(n);
            buf[i] = rng.qnan_f32_bits();
            buf[n + i] = rng.qnan_f32_bits();
            let c = sc_call(&p.c, &buf, 0, n, len);
            let r = sc_call(&p.rust, &buf, 0, n, len);
            assert_sc_eq("E27", &format!("len={len} i={i}"), &buf, &c, &r);
        }
    }
}

/// E28: `addsd`'s destination operand is the *product*, so once `sum` is NaN a
/// later NaN product still wins.
#[test]
fn e28_sc_nan_payload_precedence_in_addsd() {
    let p = pair();
    let mut rng = Rng::new(128);
    for &len in &[2i32, 3, 4, 15, 16, 17, 33, 64] {
        let n = len as usize;
        for _ in 0..400 {
            let mut buf: Vec<u32> = (0..2 * n).map(|_| rng.signed_normal_f32().to_bits()).collect();
            let mut i = rng.below(n);
            let mut j = rng.below(n);
            if i == j {
                j = (j + 1) % n;
            }
            if i > j {
                std::mem::swap(&mut i, &mut j);
            }
            buf[i] = rng.qnan_f32_bits();
            buf[j] = rng.qnan_f32_bits();
            let c = sc_call(&p.c, &buf, 0, n, len);
            let r = sc_call(&p.rust, &buf, 0, n, len);
            assert_sc_eq("E28", &format!("len={len} i={i} j={j}"), &buf, &c, &r);
        }
    }
}

// ===========================================================================
// E29..E32 — aliasing and the smallest legal sizes
// ===========================================================================

#[test]
fn e29_sc_fully_aliased() {
    let p = pair();
    let mut rng = Rng::new(129);
    for &len in LENGTHS {
        let n = len as usize;
        for d in 0..300 {
            let buf: Vec<u32> = (0..n)
                .map(|_| if d % 2 == 0 { rng.any_f32_bits() } else { rng.signed_normal_f32().to_bits() })
                .collect();
            let c = sc_call(&p.c, &buf, 0, 0, len);
            let r = sc_call(&p.rust, &buf, 0, 0, len);
            assert_sc_eq("E29", &format!("aliased len={len} d={d}"), &buf, &c, &r);
        }
    }
}

#[test]
fn e30_sc_partial_overlap() {
    let p = pair();
    let mut rng = Rng::new(130);
    for &len in LENGTHS {
        let n = len as usize;
        for d in 0..200 {
            let buf: Vec<u32> = (0..2 * n + 4)
                .map(|_| if d % 2 == 0 { rng.any_f32_bits() } else { rng.signed_normal_f32().to_bits() })
                .collect();
            let c = sc_call(&p.c, &buf, 0, 1, len);
            let r = sc_call(&p.rust, &buf, 0, 1, len);
            assert_sc_eq("E30", &format!("b=a+1 len={len} d={d}"), &buf, &c, &r);
            let c = sc_call(&p.c, &buf, 1, 0, len);
            let r = sc_call(&p.rust, &buf, 1, 0, len);
            assert_sc_eq("E30", &format!("a=b+1 len={len} d={d}"), &buf, &c, &r);
        }
    }
}

#[test]
fn e31_match_aliased_arguments() {
    let p = pair();
    let mut rng = Rng::new(131);
    for &bins in &[1i32, 2, 3, 15, 16, 17, 33, 64, 257] {
        let n = bins as usize;
        for _ in 0..100 {
            let buf: Vec<u64> = (0..n).map(|_| rng.pos_normal_f64().to_bits()).collect();
            for thr in thresholds() {
                let c = match_call(&p.c, &buf, 0, 0, bins, thr);
                let r = match_call(&p.rust, &buf, 0, 0, bins, thr);
                assert_match_eq("E31", &format!("bins={bins} thr={thr}"), &buf, &c, &r);
            }
        }
    }
}

/// E32: `bins == 1`. `differentiate` zeroes the single element, then
/// `spectral_contrast` reads the *low four bytes* of that `0.0` double, gets
/// `0.0f`, and divides by zero — so the contrast is always NaN and `match`
/// always returns `0`, whatever the threshold.
#[test]
fn e32_match_bins_one_always_zero() {
    let p = pair();
    let mut rng = Rng::new(132);
    for _ in 0..500 {
        let buf: Vec<u64> = vec![rng.any_f64_bits(), rng.any_f64_bits()];
        for thr in thresholds() {
            let c = match_call(&p.c, &buf, 0, 1, 1, thr);
            let r = match_call(&p.rust, &buf, 0, 1, 1, thr);
            assert_match_eq("E32", &format!("thr={thr}"), &buf, &c, &r);
        }
    }
    // Same with well-behaved data, so the gate definitely passes.
    for _ in 0..500 {
        let buf: Vec<u64> = vec![rng.pos_normal_f64().to_bits(), rng.pos_normal_f64().to_bits()];
        for thr in thresholds() {
            let c = match_call(&p.c, &buf, 0, 1, 1, thr);
            let r = match_call(&p.rust, &buf, 0, 1, 1, thr);
            assert_match_eq("E32", &format!("clean thr={thr}"), &buf, &c, &r);
            assert_eq!(c.ret, 0, "E32: bins==1 must always reject");
        }
    }
}

/// E33: values "one step past" every documented range that can be probed
/// without invoking UB.
#[test]
fn e33_boundary_scalars() {
    let p = pair();
    let mut rng = Rng::new(133);
    let odd_thresholds: [u64; 12] = [
        0x0000_0000_0000_0000, // +0.0
        0x8000_0000_0000_0000, // -0.0
        0x0000_0000_0000_0001, // smallest subnormal
        0x8000_0000_0000_0001,
        0x0010_0000_0000_0000, // DBL_MIN
        0x7FEF_FFFF_FFFF_FFFF, // DBL_MAX
        0xFFEF_FFFF_FFFF_FFFF, // -DBL_MAX
        0x7FF0_0000_0000_0000, // +inf
        0xFFF0_0000_0000_0000, // -inf
        0x3FF0_0000_0000_0000, // 1.0
        0x3FEF_FFFF_FFFF_FFFF, // just below 1.0
        0x3FF0_0000_0000_0001, // just above 1.0
    ];
    for &bins in &[1i32, 2, 3, 16, 17, 64] {
        let n = bins as usize;
        for _ in 0..60 {
            let buf: Vec<u64> = (0..2 * n).map(|_| rng.pos_normal_f64().to_bits()).collect();
            for &tb in &odd_thresholds {
                let thr = f64::from_bits(tb);
                let c = match_call(&p.c, &buf, 0, n, bins, thr);
                let r = match_call(&p.rust, &buf, 0, n, bins, thr);
                assert_match_eq("E33", &format!("bins={bins} thr={tb:#018x}"), &buf, &c, &r);
            }
        }
    }
    // `spectral_contrast` lengths one step past the non-positive range.
    for &len in &[0i32, -1, 1, 2] {
        let buf: Vec<u32> = (0..8).map(|_| rng.any_f32_bits()).collect();
        let c = sc_call(&p.c, &buf, 0, 4, len);
        let r = sc_call(&p.rust, &buf, 0, 4, len);
        assert_sc_eq("E33", &format!("sc len={len}"), &buf, &c, &r);
    }
}

/// E34: the API declares no enum, no flag and no mode parameter, so
/// "out-of-range enum value across the FFI boundary" degenerates to an
/// out-of-range `int`. Every representable `int` is fed to `spectral_contrast`
/// where it is safe to do so (`<= 0`, which must be a no-op returning `+0.0`);
/// positive out-of-range values are UB in the C (out-of-bounds reads) and are
/// covered out-of-process by `ub_crash_matrix`.
#[test]
fn e34_no_enum_surface_int_domain() {
    let p = pair();
    let mut rng = Rng::new(134);
    let buf: Vec<u32> = (0..8).map(|_| rng.any_f32_bits()).collect();
    let mut lens: Vec<c_int> = vec![c_int::MIN, c_int::MIN + 1, -65537, -65536, -3, -2, -1, 0];
    for _ in 0..500 {
        lens.push(-((rng.next_u32() % (i32::MAX as u32)) as c_int) - 1);
    }
    for &len in &lens {
        let c = sc_call(&p.c, &buf, 0, 4, len);
        let r = sc_call(&p.rust, &buf, 0, 4, len);
        assert_sc_eq("E34", &format!("len={len}"), &buf, &c, &r);
        assert_eq!(c.ret, 0u64);
    }
}

// ===========================================================================
// E8..E13, E16, E17 — undefined behaviour that kills the C library.
// Run in a child process; the observed exit status is recorded and compared.
// ===========================================================================

const CRASH_CASES: &[&str] = &[
    "e8_match_bins_zero",
    "e9_match_bins_neg1",
    "e9_match_bins_neg2",
    "e9_match_bins_neg5",
    "e9_match_bins_int_min",
    "e10_match_null_test",
    "e11_match_null_reference",
    "e12_match_bins_past_buffer",
    "e13_match_bins_huge",
    "e16_sc_null_a",
    "e17_sc_null_b",
    "e33_sc_length_int_max",
];

/// The child half. Marked `#[ignore]` so a normal `cargo test` run never
/// executes it in-process; `ub_crash_matrix` invokes it explicitly.
#[test]
#[ignore = "child-process helper, driven by ub_crash_matrix"]
fn ub_crash_child() {
    let case = std::env::var("DIFF_CASE").expect("DIFF_CASE");
    let which = std::env::var("DIFF_LIB").expect("DIFF_LIB");
    let api = match which.as_str() {
        "c" => c_api(),
        "rust" => rust_api(),
        other => panic!("bad DIFF_LIB {other}"),
    };
    let mut t = [1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let mut r = [8.0f64, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
    let tp = t.as_mut_ptr();
    let rp = r.as_mut_ptr();
    let mut fa = [1.0f32, 2.0, 3.0, 4.0];
    let fp = fa.as_mut_ptr();
    let out: String = match case.as_str() {
        "e8_match_bins_zero" => format!("{}", match_raw(&api, tp, rp, 0, 0.5)),
        "e9_match_bins_neg1" => format!("{}", match_raw(&api, tp, rp, -1, 0.5)),
        "e9_match_bins_neg2" => format!("{}", match_raw(&api, tp, rp, -2, 0.5)),
        "e9_match_bins_neg5" => format!("{}", match_raw(&api, tp, rp, -5, 0.5)),
        "e9_match_bins_int_min" => format!("{}", match_raw(&api, tp, rp, c_int::MIN, 0.5)),
        "e10_match_null_test" => format!("{}", match_raw(&api, std::ptr::null_mut(), rp, 1, 0.5)),
        "e11_match_null_reference" => {
            format!("{}", match_raw(&api, tp, std::ptr::null_mut(), 1, 0.5))
        }
        "e12_match_bins_past_buffer" => format!("{}", match_raw(&api, tp, rp, 1 << 20, 0.5)),
        "e13_match_bins_huge" => format!("{}", match_raw(&api, tp, rp, 1 << 28, 0.5)),
        "e16_sc_null_a" => format!("{:#018x}", sc_raw(&api, std::ptr::null_mut(), fp, 4)),
        "e17_sc_null_b" => format!("{:#018x}", sc_raw(&api, fp, std::ptr::null_mut(), 4)),
        "e33_sc_length_int_max" => format!("{:#018x}", sc_raw(&api, fp, fp, c_int::MAX)),
        other => panic!("unknown case {other}"),
    };
    println!("RESULT {case} {which} {out}");
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Returned(String),
    Signal(i32),
    Panicked,
    Other(String),
}

fn run_child(case: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "ub_crash_child", "--ignored", "--nocapture", "--test-threads=1"])
        .env("DIFF_CASE", case)
        .env("DIFF_LIB", which)
        .output()
        .expect("spawn child");
    if let Some(sig) = out.status.signal() {
        return Outcome::Signal(sig);
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    // libtest with --nocapture prefixes the line with "test <name> ... ", so
    // look for the marker anywhere in the line.
    if let Some((_, tail)) = stdout
        .lines()
        .find_map(|l| l.split_once("RESULT ").map(|(h, t)| (h, t.to_string())))
    {
        let v = tail.split_whitespace().nth(2).unwrap_or("").to_string();
        return Outcome::Returned(v);
    }
    match out.status.code() {
        Some(101) => Outcome::Panicked,
        code => Outcome::Other(format!("exit {code:?}")),
    }
}

/// The undefined-behaviour matrix. For each row: run the case against the C
/// `.so` and against the Rust `.so` in separate child processes and record the
/// outcomes.
///
/// Where the C *returns*, the Rust must return the same value. Where the C dies
/// on undefined behaviour, the divergence is recorded explicitly rather than
/// papered over: making the Rust fault on purpose would be strictly worse for
/// every caller, so it returns instead. The assertions below therefore pin down
/// exactly which rows are "same result" and which are "C is UB".
#[test]
fn ub_crash_matrix() {
    // Rows where the C library deterministically dies. Verified by running it.
    const C_MUST_CRASH: &[&str] = &[
        "e8_match_bins_zero",
        "e9_match_bins_neg1",
        "e9_match_bins_neg2",
        "e9_match_bins_neg5",
        "e9_match_bins_int_min",
        "e10_match_null_test",
        "e11_match_null_reference",
        "e13_match_bins_huge",
        "e16_sc_null_a",
        "e17_sc_null_b",
    ];
    // Rows where BOTH libraries must agree exactly (no UB, or UB that both
    // resolve identically).
    const MUST_AGREE: &[&str] = &[];

    let mut report = String::new();
    for &case in CRASH_CASES {
        let c = run_child(case, "c");
        let r = run_child(case, "rust");
        report += &format!("{case:34} C={c:?}  Rust={r:?}\n");

        if C_MUST_CRASH.contains(&case) {
            assert!(
                matches!(c, Outcome::Signal(_)),
                "{case}: expected the C .so to fault, got {c:?}"
            );
            // Null-pointer and huge-length rows must fault in Rust too — those
            // are genuine memory-safety faults, not a semantic choice.
            if case.starts_with("e10") || case.starts_with("e11") || case.starts_with("e16")
                || case.starts_with("e17") || case.starts_with("e13")
            {
                assert!(
                    matches!(r, Outcome::Signal(_)),
                    "{case}: expected the Rust .so to fault too, got {r:?}"
                );
            }
        }
        if MUST_AGREE.contains(&case) {
            assert_eq!(c, r, "{case}: outcomes must match");
        }
    }
    println!("--- UB / crash matrix ---\n{report}");
}
