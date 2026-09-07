//! Phase C — error/boundary-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! The C API has no error codes, no sentinels, no pointers and no enums (see
//! `ERRORS.md` for the mechanical grep proving this), so each row asserts that
//! C and Rust agree bit-for-bit on the concrete result the C `.so` produces for
//! the boundary / undefined-behaviour condition in question.

mod common;
use common::{check, pair, y_all, Rng};

// ------------------------------------------------------------ ERRORS row 1
// exp_q2 < 0  =>  negative shift count  =>  UB in C, `sar %cl` on x86.
#[test]
fn err01_negative_shift_count_ub() {
    let mut rng = Rng::new(0xE001);
    // Exhaustive over two full 128-wide aliasing periods plus randomized.
    for e in -1024..=-1 {
        for y in y_all() {
            check(y, e);
        }
        for _ in 0..16 {
            check(rng.next_f32_bits(), e);
        }
    }
    for _ in 0..20_000 {
        check(rng.next_f32_bits(), rng.in_range(i32::MIN, -1));
    }
}

// ------------------------------------------------------------ ERRORS row 2
// exp_q2 in {-1,-2,-3,-4} => integer factor is exactly 0.
#[test]
fn err02_zero_integer_factor() {
    let p = pair();
    for e in [-1i32, -2, -3, -4] {
        for y in y_all() {
            check(y, e);
        }
        // Sanity on the C side: finite y collapses to a signed zero, and
        // inf/NaN become NaN. Asserted on the *C* result, then matched.
        let cz = unsafe { (p.c)(1.0, e) };
        assert_eq!(cz.to_bits(), 0.0f32.to_bits(), "C: 1.0 * 0 must be +0.0");
        let cnz = unsafe { (p.c)(-1.0, e) };
        assert_eq!(cnz.to_bits(), (-0.0f32).to_bits(), "C: -1.0 * 0 must be -0.0");
        let cinf = unsafe { (p.c)(f32::INFINITY, e) };
        assert!(cinf.is_nan(), "C: inf * 0 must be NaN");
        check(f32::INFINITY, e);
        check(f32::NEG_INFINITY, e);
    }
    // Same condition reached from far-negative exponents in the same residue
    // class modulo the 128 aliasing period.
    for k in 1..=64i32 {
        for r in 1..=4i32 {
            let e = -r - 128 * k;
            for y in y_all() {
                check(y, e);
            }
        }
    }
}

// ------------------------------------------------------------ ERRORS row 3
// exp_q2 == 0 still runs the do/while body once; factor is exactly 1.0f.
#[test]
fn err03_exp_zero_runs_body_once() {
    let p = pair();
    for y in y_all() {
        check(y, 0);
        let c = unsafe { (p.c)(y, 0) };
        // g_expfrac[0] is bit-exactly 2^-30, so the factor is 1.0f and this is
        // the identity — except for a *signalling* NaN, which `mulss` quiets
        // (0x7f800001 -> 0x7fc00001). That quieting is hardware behaviour and
        // is checked differentially by `check()` above.
        if y.is_nan() {
            assert!(c.is_nan(), "C: NaN in must give NaN out");
        } else {
            assert_eq!(
                c.to_bits(),
                y.to_bits(),
                "C: ldexp_q2(y, 0) must be the identity for y=0x{:08x}",
                y.to_bits()
            );
        }
    }
    let mut rng = Rng::new(0xE003);
    for _ in 0..50_000 {
        check(rng.next_f32_bits(), 0);
    }
}

// ------------------------------------------------------------ ERRORS rows 4-6
// The 30*4 clamp: 119 / 120 / 121.
#[test]
fn err04_06_clamp_boundary_119_120_121() {
    let mut rng = Rng::new(0xE004);
    for e in [118, 119, 120, 121, 122] {
        for y in y_all() {
            check(y, e);
        }
        for _ in 0..20_000 {
            check(rng.next_f32_bits(), e);
        }
    }
    // 121 must compose two chunks (120 then 1), not behave like 2^(121/4).
    let p = pair();
    let one_shot = unsafe { (p.c)(1.0, 121) };
    let composed = unsafe { (p.c)((p.c)(1.0, 120), 1) };
    assert_eq!(
        one_shot.to_bits(),
        composed.to_bits(),
        "C: exp 121 must equal chunk(120) then chunk(1)"
    );
    check(1.0, 121);
}

// ------------------------------------------------------------ ERRORS row 7
#[test]
fn err07_exp_i32_min() {
    let p = pair();
    for y in y_all() {
        check(y, i32::MIN);
        // e & 3 == 0 and (e >> 2) & 31 == 0 => factor exactly 1.0f, so this is
        // the identity (modulo `mulss` quieting a signalling NaN).
        let c = unsafe { (p.c)(y, i32::MIN) };
        if y.is_nan() {
            assert!(c.is_nan(), "C: NaN in must give NaN out");
        } else {
            assert_eq!(
                c.to_bits(),
                y.to_bits(),
                "C: ldexp_q2(y, INT_MIN) must be the identity for y=0x{:08x}",
                y.to_bits()
            );
        }
    }
    let mut rng = Rng::new(0xE007);
    for _ in 0..50_000 {
        check(rng.next_f32_bits(), i32::MIN);
    }
}

// ------------------------------------------------------------ ERRORS row 8
#[test]
fn err08_exp_i32_min_plus_1_to_3() {
    let mut rng = Rng::new(0xE008);
    for d in 1..=8i32 {
        let e = i32::MIN + d;
        for y in y_all() {
            check(y, e);
        }
        for _ in 0..10_000 {
            check(rng.next_f32_bits(), e);
        }
    }
}

// ------------------------------------------------------------ ERRORS row 9
#[test]
fn err09_exp_i32_max() {
    // ~18 M iterations per call; a handful of representative y values.
    for y in [
        1.0f32,
        -1.0f32,
        0.0f32,
        -0.0f32,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x0000_0001),
    ] {
        check(y, i32::MAX);
    }
}

// ------------------------------------------------------------ ERRORS row 10
#[test]
fn err10_nan_payload_propagation() {
    let mut rng = Rng::new(0xE010);
    let exps = [
        0, 1, 2, 3, 4, 119, 120, 121, 240, 1200, -1, -2, -3, -4, -5, -128, -129, -512, 100_000,
        -100_000, i32::MIN, i32::MIN + 1,
    ];
    for &e in &exps {
        for &b in &[
            0x7fc0_0000u32,
            0xffc0_0000,
            0x7fc0_dead,
            0xffc0_beef,
            0x7f80_0001,
            0xff80_0001,
            0x7fff_ffff,
            0xffff_ffff,
            0x7fbf_ffff,
        ] {
            check(f32::from_bits(b), e);
        }
    }
    // Randomized NaN payloads.
    for _ in 0..20_000 {
        let b = 0x7f80_0000u32 | (rng.next_u32() & 0x807f_ffff);
        if b & 0x007f_ffff == 0 {
            continue; // that's an infinity, covered by row 11/15
        }
        check(f32::from_bits(b), rng.in_range(-4096, 4096));
    }
}

// ------------------------------------------------------------ ERRORS row 11
#[test]
fn err11_inf_times_zero_factor() {
    for e in [-1i32, -2, -3, -4, -129, -130, -131, -132, -257, -258, -259, -260] {
        check(f32::INFINITY, e);
        check(f32::NEG_INFINITY, e);
    }
}

// ------------------------------------------------------------ ERRORS row 12
#[test]
fn err12_underflow_to_zero() {
    let tiny = [
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007f_ffff),
        f32::from_bits(0x807f_ffff),
    ];
    for &y in &tiny {
        // Strongly negative-scaling exponents (negative e => tiny factors).
        for e in -600..=-5 {
            check(y, e);
        }
        // and small positive exponents that renormalise the subnormal
        for e in 0..=600 {
            check(y, e);
        }
    }
}

// ------------------------------------------------------------ ERRORS row 13
#[test]
fn err13_overflow_to_inf() {
    for &y in &[f32::MAX, f32::MIN, 1e38f32, -1e38f32] {
        for e in 0..=2000 {
            check(y, e);
        }
        for e in [10_000, 100_000, 1 << 20] {
            check(y, e);
        }
    }
}

// ------------------------------------------------------------ ERRORS row 14
#[test]
fn err14_shift_mask_period_128_aliasing() {
    // (e >> 2) & 31 has period 128 in e for negative e; walk several periods
    // exhaustively so every residue class is compared.
    let ys = [1.0f32, -1.0f32, 3.141_592_7f32, f32::MIN_POSITIVE, f32::MAX];
    for e in -2048..=-1 {
        for &y in &ys {
            check(y, e);
        }
    }
    for base in [-128i32, -256, -384, -508, -512, -1024, -100_000, -1 << 20] {
        for d in -4..=4 {
            for &y in &ys {
                check(y, base + d);
            }
        }
    }
}

// ------------------------------------------------------------ ERRORS row 15
#[test]
fn err15_inf_with_huge_positive_exp() {
    for e in [121, 240, 1200, 10_000, 100_000, 1 << 20, i32::MAX] {
        check(f32::INFINITY, e);
        check(f32::NEG_INFINITY, e);
    }
}

// ------------------------------------------------------------ ERRORS row 16
// N/A in the table (no enums, no pointers). What *is* testable is that the full
// int domain of exp_q2 — including values that no sane caller would pass —
// behaves identically. This is the analogue of "out-of-range enum value".
#[test]
fn err16_full_int_domain_sweep() {
    let mut rng = Rng::new(0xE016);
    let mut n = 0usize;
    while n < 30_000 {
        let e = rng.next_i32();
        if e > 300_000 {
            continue; // runtime guard only; the tail is covered by err09/err15
        }
        check(rng.next_f32_bits(), e);
        n += 1;
    }
    // Deliberately absurd values.
    for e in [
        i32::MIN,
        i32::MIN + 1,
        -1_073_741_824,
        -1_000_000_007,
        -999_983,
        -121,
        121,
        999_983,
        300_000,
    ] {
        for y in y_all() {
            check(y, e);
        }
    }
}
