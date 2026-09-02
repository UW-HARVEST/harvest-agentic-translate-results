//! Phase C -- error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI boundary cases G1..G8.
//! Every assertion checks that C and Rust return the *same specific* value
//! (error code / sentinel), never merely that "both failed".

mod common;

use common::*;

// ===========================================================================
// E1, E2, E3 -- isnan(d) => 0
// ===========================================================================
#[test]
fn err_e1_e3_nan() {
    let p = pair();
    let nans: &[(&str, f64)] = &[
        ("0.0/0.0 quiet NaN", 0.0f64 / 0.0f64),
        ("negated NaN", -(0.0f64 / 0.0f64)),
        ("f64::NAN", f64::NAN),
        ("-f64::NAN", -f64::NAN),
        ("inf - inf", f64::INFINITY - f64::INFINITY),
        ("inf * 0", f64::INFINITY * 0.0),
        ("quiet NaN payload 1", f64::from_bits(0x7FF8_0000_0000_0001)),
        ("quiet NaN payload max", f64::from_bits(0x7FFF_FFFF_FFFF_FFFF)),
        ("negative quiet NaN", f64::from_bits(0xFFF8_0000_0000_0001)),
        ("signalling NaN", f64::from_bits(0x7FF0_0000_0000_0001)),
        ("negative signalling NaN", f64::from_bits(0xFFF0_0000_0000_0001)),
        ("signalling NaN alt", f64::from_bits(0x7FF4_0000_0000_0000)),
    ];
    for &(label, d) in nans {
        assert!(d.is_nan(), "{label} is not NaN");
        let c = p.c.safe_double_to_int(d);
        let rs = p.rs.safe_double_to_int(d);
        assert_eq!(c, 0, "C must return 0 for {label}");
        assert_eq!(
            rs, c,
            "safe_double_to_int({label}, bits 0x{:016x}): C={c} Rust={rs}",
            d.to_bits()
        );
    }
    // Randomized NaN payloads.
    let mut r = Rng::new(0xE1_0001);
    for _ in 0..5000 {
        let bits = 0x7FF0_0000_0000_0000u64
            | (r.next_u64() & 0x000F_FFFF_FFFF_FFFF)
            | (r.next_u64() & (1u64 << 63));
        let d = f64::from_bits(bits);
        if !d.is_nan() {
            continue; // mantissa happened to be zero => infinity, that's E4/E5
        }
        let c = p.c.safe_double_to_int(d);
        let rs = p.rs.safe_double_to_int(d);
        assert_eq!(c, 0, "C must return 0 for NaN bits 0x{bits:016x}");
        assert_eq!(rs, c, "NaN bits 0x{bits:016x}: C={c} Rust={rs}");
    }
}

// ===========================================================================
// E4, E5 -- isinf(d) => INT_MAX / INT_MIN
// ===========================================================================
#[test]
fn err_e4_e5_inf() {
    let p = pair();
    for (d, want) in [
        (f64::INFINITY, i32::MAX),
        (f64::from_bits(0x7FF0_0000_0000_0000), i32::MAX),
        (1.0f64 / 0.0f64, i32::MAX),
        (f64::MAX * 2.0, i32::MAX),
        (f64::NEG_INFINITY, i32::MIN),
        (f64::from_bits(0xFFF0_0000_0000_0000), i32::MIN),
        (-1.0f64 / 0.0f64, i32::MIN),
        (f64::MIN * 2.0, i32::MIN),
    ] {
        let c = p.c.safe_double_to_int(d);
        let rs = p.rs.safe_double_to_int(d);
        assert_eq!(c, want, "C: safe_double_to_int({d:?}) should be {want}");
        assert_eq!(rs, c, "safe_double_to_int({d:?}): C={c} Rust={rs}");
    }
}

// ===========================================================================
// E6, E7 -- finite saturation at (double)INT_MAX / (double)INT_MIN
// ===========================================================================
#[test]
fn err_e6_e7_saturate() {
    let p = pair();

    // E6: the `>=` is inclusive, so exactly 2147483647.0 saturates.
    for (d, want) in [
        (2147483647.0f64, i32::MAX),
        (2147483647.5f64, i32::MAX),
        (2147483648.0f64, i32::MAX),
        (4e9f64, i32::MAX),
        (1e300f64, i32::MAX),
        (f64::MAX, i32::MAX),
        (2147483647.0f64.next_up(), i32::MAX),
        // Just below the guard: still truncates normally.
        (2147483646.9999997f64, 2147483646),
        (2147483647.0f64.next_down(), 2147483646),
    ] {
        let c = p.c.safe_double_to_int(d);
        let rs = p.rs.safe_double_to_int(d);
        assert_eq!(c, want, "C: safe_double_to_int({d:?}) should be {want}");
        assert_eq!(rs, c, "safe_double_to_int({d:?}): C={c} Rust={rs}");
    }

    // E7: the `<=` is inclusive, so exactly -2147483648.0 saturates.
    for (d, want) in [
        (-2147483648.0f64, i32::MIN),
        (-2147483648.5f64, i32::MIN),
        (-2147483649.0f64, i32::MIN),
        (-4e9f64, i32::MIN),
        (-1e300f64, i32::MIN),
        (f64::MIN, i32::MIN),
        ((-2147483648.0f64).next_down(), i32::MIN),
        // Just above the guard.
        (-2147483647.9999995f64, -2147483647),
        ((-2147483648.0f64).next_up(), -2147483647),
        (-2147483647.0f64, -2147483647),
    ] {
        let c = p.c.safe_double_to_int(d);
        let rs = p.rs.safe_double_to_int(d);
        assert_eq!(c, want, "C: safe_double_to_int({d:?}) should be {want}");
        assert_eq!(rs, c, "safe_double_to_int({d:?}): C={c} Rust={rs}");
    }
}

// ===========================================================================
// E8 -- allocate_and_compute: negative size => size_t wraparound => malloc NULL
// ===========================================================================
#[test]
fn err_e8_alloc_fail() {
    let p = pair();
    // These request 2^64 - 16*|size| bytes, which glibc always refuses.
    for size in [-1, -2, -3, -8, -9, -100, -1000, i32::MIN, i32::MIN + 1, -(1 << 20)] {
        let c = p.c.allocate_and_compute(size, 1.5);
        let rs = p.rs.allocate_and_compute(size, 1.5);
        assert_eq!(c, -1, "C: allocate_and_compute({size}, 1.5) should be -1");
        assert_eq!(rs, c, "allocate_and_compute({size}, 1.5): C={c} Rust={rs}");
    }
    // Negative size with every interesting multiplier: the multiplier is never
    // consulted because the function returns before the loops.
    let mut r = Rng::new(0xE8_0001);
    for _ in 0..2000 {
        let size = r.range_i32(i32::MIN, -1);
        let m = r.raw_f64();
        let c = p.c.allocate_and_compute(size, m);
        let rs = p.rs.allocate_and_compute(size, m);
        assert_eq!(c, -1, "C: allocate_and_compute({size}, {m:?}) should be -1");
        assert_eq!(rs, c, "allocate_and_compute({size}, {m:?}): C={c} Rust={rs}");
    }
}

// ===========================================================================
// E9 -- allocate_and_compute: huge positive size that malloc refuses
// ===========================================================================
#[test]
fn err_e9_alloc_fail_big() {
    let p = pair();
    // `size * 16` bytes. Where exactly malloc starts refusing is host dependent
    // (cgroup / overcommit), so the *differential* requirement -- C and Rust
    // must reach the same verdict -- is asserted unconditionally, while the
    // `-1` sentinel is asserted only for requests of 8 GiB and above, which
    // glibc refuses on this host. Sizes in the "malloc probably succeeds" band
    // are deliberately excluded: they would make the test allocate and touch
    // multiple GiB.
    for size in [i32::MAX, i32::MAX - 1, 1 << 30, 1 << 29] {
        let bytes = size as u64 * 16;
        let c = p.c.allocate_and_compute(size, 1.5);
        let rs = p.rs.allocate_and_compute(size, 1.5);
        assert_eq!(
            rs, c,
            "allocate_and_compute({size}, 1.5) [{bytes} bytes]: C={c} Rust={rs} \
             (same malloc outcome required)"
        );
        assert!(bytes >= 8 << 30);
        assert_eq!(
            c, -1,
            "C: allocate_and_compute({size}, 1.5) requests {bytes} bytes and should be -1"
        );
    }
}

// ===========================================================================
// E10 -- fallcalc's own malloc(20) guard. Documented as unreachable in
// practice; assert that neither implementation reports the failure sentinel
// for an input whose real result is not -1, and that they agree regardless.
// ===========================================================================
#[test]
fn err_e10_unreachable() {
    let p = pair();
    let mut r = Rng::new(0xEA_0001);
    for _ in 0..2000 {
        let (a, b, c_, d) = (
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
        let c = p.c.fallcalc(a, b, c_, d);
        let rs = p.rs.fallcalc(a, b, c_, d);
        cmp("fallcalc (E10)", (a, b, c_, d), c, rs);
        // The 20-byte malloc never fails, so the result is always the masked
        // value in 0..=511 -- never the -1 sentinel.
        assert!(
            (0..=0o777).contains(&c),
            "C fallcalc returned {c} (outside 0..=511) for {:?}",
            (a, b, c_, d)
        );
        assert_ne!(c, -1, "fallcalc hit its malloc-failure path unexpectedly");
    }
}

// ===========================================================================
// E11, E12 -- switch default arm (out-of-range `operation`)
// ===========================================================================
#[test]
fn err_e11_e12_default() {
    let p = pair();
    // One step past the label range in both directions, and far outside.
    let out_of_range = [
        -1, -2, -3, -4, -5, -6, -10, -100, -1000, 5, 6, 7, 8, 10, 100, 1000, i32::MAX,
        i32::MAX - 1, i32::MIN, i32::MIN + 1, 0x4000_0000, -0x4000_0000,
    ];
    let values = [
        0,
        1,
        -1,
        7,
        64,
        128,
        511,
        512,
        -512,
        i32::MAX,
        i32::MIN,
        123456789,
        -987654321,
    ];
    for &op in &out_of_range {
        for &v in &values {
            let c = p.c.switch_fallthrough_calculator(v, op);
            let rs = p.rs.switch_fallthrough_calculator(v, op);
            assert_eq!(
                c, 0,
                "C: switch_fallthrough_calculator({v}, {op}) must hit `default` and return 0"
            );
            assert_eq!(
                rs, c,
                "switch_fallthrough_calculator({v}, {op}): C={c} Rust={rs}"
            );
        }
    }
    // In-range labels must NOT return the default sentinel path spuriously:
    // confirm the two sides still agree for 0..=4 (positive control).
    for op in 0..=4 {
        for &v in &values {
            cmp_switch(p, v, op);
        }
    }
}

// ===========================================================================
// E13, E14 / G1, G2 -- process_array_reverse with count <= 0 (NULL is legal)
// ===========================================================================
#[test]
fn err_e13_reverse_nonpos() {
    let p = pair();
    for count in [0, -1, -2, -5, -100, i32::MIN, i32::MIN + 1, -(1 << 20)] {
        // NULL end pointer: the loop guard prevents any dereference.
        let c = p.c.process_array_reverse(std::ptr::null_mut(), count);
        let rs = p.rs.process_array_reverse(std::ptr::null_mut(), count);
        assert_eq!(c, 0, "C: process_array_reverse(NULL, {count}) should be 0");
        assert_eq!(
            rs, c,
            "process_array_reverse(NULL, {count}): C={c} Rust={rs}"
        );

        // Also with a real (dangling-free) buffer, to show the buffer is untouched.
        let mut a = vec![0x7FFF_FFFFi32; 8];
        let mut b = a.clone();
        let c = unsafe { p.c.process_array_reverse(a.as_mut_ptr().add(7), count) };
        let rs = unsafe { p.rs.process_array_reverse(b.as_mut_ptr().add(7), count) };
        assert_eq!(c, 0, "C: process_array_reverse(buf+7, {count}) should be 0");
        assert_eq!(rs, c, "process_array_reverse(buf+7, {count}): C={c} Rust={rs}");
        assert_eq!(a, b, "buffer diverged");
    }
    // Misaligned / arbitrary non-null pointer with count 0: still no deref.
    let bogus = 0x1usize as *mut i32;
    assert_eq!(p.c.process_array_reverse(bogus, 0), 0);
    assert_eq!(p.rs.process_array_reverse(bogus, 0), 0);
}

// ===========================================================================
// E15, E16 / G1, G2 -- foreach_sum with count <= 0 (NULL is legal)
// ===========================================================================
#[test]
fn err_e15_foreach_nonpos() {
    let p = pair();
    for count in [0, -1, -2, -5, -100, i32::MIN, i32::MIN + 1, -(1 << 20)] {
        let c = p.c.foreach_sum(std::ptr::null_mut(), count);
        let rs = p.rs.foreach_sum(std::ptr::null_mut(), count);
        assert_eq!(c, 0, "C: foreach_sum(NULL, {count}) should be 0");
        assert_eq!(rs, c, "foreach_sum(NULL, {count}): C={c} Rust={rs}");

        let mut a = vec![-1i32; 8];
        let mut b = a.clone();
        let c = p.c.foreach_sum(a.as_mut_ptr(), count);
        let rs = p.rs.foreach_sum(b.as_mut_ptr(), count);
        assert_eq!(c, 0, "C: foreach_sum(buf, {count}) should be 0");
        assert_eq!(rs, c, "foreach_sum(buf, {count}): C={c} Rust={rs}");
        assert_eq!(a, b, "buffer diverged");
    }
    let bogus = 0x1usize as *mut i32;
    assert_eq!(p.c.foreach_sum(bogus, 0), 0);
    assert_eq!(p.rs.foreach_sum(bogus, 0), 0);
}

// ===========================================================================
// E17 / G2 -- allocate_and_compute(0, m): malloc(0) is non-NULL on glibc, so
// the NULL guard does NOT fire and the result is 0, not -1.
// ===========================================================================
#[test]
fn err_e17_size_zero() {
    let p = pair();
    let mut r = Rng::new(0xE1_7000);
    let mults: Vec<f64> = [
        1.5,
        0.0,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN,
        1e-300,
    ]
    .into_iter()
    .chain((0..200).map(|_| r.raw_f64()))
    .collect();

    for m in mults {
        let c = p.c.allocate_and_compute(0, m);
        let rs = p.rs.allocate_and_compute(0, m);
        assert_eq!(
            c, 0,
            "C: allocate_and_compute(0, {m:?}) should be 0 (malloc(0) is non-NULL)"
        );
        assert_eq!(rs, c, "allocate_and_compute(0, {m:?}): C={c} Rust={rs}");
    }
}

// ===========================================================================
// E18 -- multiplier NaN => sum NaN => falls into E1 => 0
// ===========================================================================
#[test]
fn err_e18_mult_nan() {
    let p = pair();
    for m in [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001),
        f64::from_bits(0xFFF0_0000_0000_0001),
    ] {
        for size in [1, 2, 3, 5, 10, 64, 1000] {
            let c = p.c.allocate_and_compute(size, m);
            let rs = p.rs.allocate_and_compute(size, m);
            assert_eq!(
                c, 0,
                "C: allocate_and_compute({size}, NaN) should be 0 (sum is NaN)"
            );
            assert_eq!(rs, c, "allocate_and_compute({size}, NaN): C={c} Rust={rs}");
        }
    }
}

// ===========================================================================
// E19, E20 -- multiplier +-inf.
//
// For EVERY size >= 1 the i == 0 element has `value == 0` and
// `coefficient == 0.0 * (+-inf) == NaN`, so the very first term of the sum is
// `0 * NaN == NaN` and `sum` is NaN for the rest of the loop. The result is
// therefore 0 (via the isnan branch, E1) for all sizes -- it never saturates.
// For size == 0 the loops are skipped and `sum` stays 0.0, also giving 0.
// ===========================================================================
#[test]
fn err_e19_mult_inf() {
    let p = pair();
    for m in [f64::INFINITY, f64::NEG_INFINITY] {
        for size in [0, 1, 2, 3, 5, 10, 64, 1000] {
            let c = p.c.allocate_and_compute(size, m);
            let rs = p.rs.allocate_and_compute(size, m);
            assert_eq!(
                c, 0,
                "C: allocate_and_compute({size}, {m:?}) should be 0 (0.0*inf poisons sum with NaN)"
            );
            assert_eq!(
                rs, c,
                "allocate_and_compute({size}, {m:?}): C={c} Rust={rs}"
            );
        }
    }
}

// ===========================================================================
// E21 -- multiplier DBL_MAX => sum overflows to +inf => INT_MAX
// ===========================================================================
#[test]
fn err_e21_mult_dblmax() {
    let p = pair();
    for (m, want) in [(f64::MAX, i32::MAX), (f64::MIN, i32::MIN)] {
        for size in [2, 3, 5, 10, 64] {
            let c = p.c.allocate_and_compute(size, m);
            let rs = p.rs.allocate_and_compute(size, m);
            assert_eq!(
                c, want,
                "C: allocate_and_compute({size}, {m:?}) should be {want}"
            );
            assert_eq!(rs, c, "allocate_and_compute({size}, {m:?}): C={c} Rust={rs}");
        }
        // size == 1 is 0 * DBL_MAX == 0.0, not an overflow.
        let c = p.c.allocate_and_compute(1, m);
        let rs = p.rs.allocate_and_compute(1, m);
        assert_eq!(c, 0);
        assert_eq!(rs, c);
    }
}

// ===========================================================================
// E22 -- fallcalc with param4 % 10 + 1 <= 0 (nested E8 / E17)
// ===========================================================================
#[test]
fn err_e22_fallcalc_neg_size() {
    let p = pair();
    let mut r = Rng::new(0xE2_2000);
    for p4 in [
        -1,
        -2,
        -3,
        -4,
        -5,
        -6,
        -7,
        -8,
        -9,
        -11,
        -19,
        -29,
        -99,
        -1000000001,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 7,
    ] {
        let nested_size = p4 % 10 + 1;
        assert!(nested_size <= 0, "p4={p4} nested_size={nested_size}");
        // Confirm the nested call really takes the error path in C.
        let nested_c = p.c.allocate_and_compute(nested_size, 1.5);
        let nested_rs = p.rs.allocate_and_compute(nested_size, 1.5);
        assert_eq!(
            nested_c,
            if nested_size == 0 { 0 } else { -1 },
            "nested allocate_and_compute({nested_size}, 1.5)"
        );
        assert_eq!(nested_rs, nested_c);

        for _ in 0..300 {
            cmp_fallcalc(
                p,
                r.interesting_i32(),
                r.interesting_i32(),
                r.interesting_i32(),
                p4,
            );
        }
        // Deterministic spot checks too.
        for &a in &[0, 1, -1, i32::MAX, i32::MIN] {
            cmp_fallcalc(p, a, a, a, p4);
        }
    }
    // p4 == -10, -20 ... give p4 % 10 == 0 -> nested size 1 (positive control).
    for p4 in [-10, -20, -100, -2000000000] {
        assert_eq!(p4 % 10 + 1, 1);
        for _ in 0..200 {
            cmp_fallcalc(
                p,
                r.interesting_i32(),
                r.interesting_i32(),
                r.interesting_i32(),
                p4,
            );
        }
    }
}

// ===========================================================================
// E23 -- fallcalc driving switch into `default` (negative param3 % 5)
// ===========================================================================
#[test]
fn err_e23_fallcalc_default_arm() {
    let p = pair();
    let mut r = Rng::new(0xE2_3000);
    for p3 in [
        -1,
        -2,
        -3,
        -4,
        -6,
        -7,
        -8,
        -9,
        -11,
        -12,
        -13,
        -14,
        -101,
        -999999999,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
    ] {
        let op = p3 % 5;
        assert!(op < 0, "p3={p3} op={op}");
        // The nested switch really returns 0 for this operation.
        for &v in &[0, 1, -1, 511, i32::MAX, i32::MIN] {
            assert_eq!(p.c.switch_fallthrough_calculator(v, op), 0);
            assert_eq!(p.rs.switch_fallthrough_calculator(v, op), 0);
        }
        // param3 < 0 also means `param3 > 0200` is false.
        assert!(!(p3 > 0o200));
        for _ in 0..300 {
            cmp_fallcalc(
                p,
                r.interesting_i32(),
                r.interesting_i32(),
                p3,
                r.interesting_i32(),
            );
        }
    }
    // p3 negative multiples of 5 -> op == 0 -> arm 0 (positive control).
    for p3 in [-5, -10, -100, -2000000000] {
        assert_eq!(p3 % 5, 0);
        for _ in 0..200 {
            cmp_fallcalc(
                p,
                r.interesting_i32(),
                r.interesting_i32(),
                p3,
                r.interesting_i32(),
            );
        }
    }
}

// ===========================================================================
// E24 -- fallcalc's floating_calc saturating through E6 / E7
// ===========================================================================
#[test]
fn err_e24_fallcalc_saturate() {
    let p = pair();
    let mut r = Rng::new(0xE2_4000);

    // Choose params whose floating_calc is definitely out of int range.
    let saturating = [
        (i32::MAX, i32::MAX, 0),
        (i32::MAX, i32::MAX, i32::MIN),
        (i32::MIN, i32::MIN, 0),
        (i32::MIN, i32::MIN, i32::MAX),
        (i32::MAX, 0, i32::MIN),
        (i32::MIN, 0, i32::MAX),
        (0, i32::MAX, i32::MIN),
        (0, i32::MIN, i32::MAX),
        (1_000_000_000, 1_000_000_000, -1_000_000_000),
        (-1_000_000_000, -1_000_000_000, 1_000_000_000),
    ];
    for &(p1, p2, p3) in &saturating {
        let fc = (p1 as f64) * 3.7 + (p2 as f64) * 2.3 - (p3 as f64) * 0.5;
        assert!(
            fc >= 2147483647.0 || fc <= -2147483648.0,
            "floating_calc {fc} is not saturating for {:?}",
            (p1, p2, p3)
        );
        let want = if fc >= 2147483647.0 { i32::MAX } else { i32::MIN };
        assert_eq!(p.c.safe_double_to_int(fc), want);
        assert_eq!(p.rs.safe_double_to_int(fc), want);
        for p4 in [0, 1, 9, -1, -9, i32::MAX, i32::MIN] {
            cmp_fallcalc(p, p1, p2, p3, p4);
        }
        for _ in 0..200 {
            cmp_fallcalc(p, p1, p2, p3, r.next_i32());
        }
    }
}

// ===========================================================================
// G5 -- out-of-range "enum" values across the FFI boundary.
// `operation` is a C `int` switch selector, so *every* 32-bit value is a real
// input. Fuzz the full range, including bit patterns that no valid variant has.
// ===========================================================================
#[test]
fn err_g5_operation_fuzz() {
    let p = pair();
    let mut r = Rng::new(0x0555_0000);
    for _ in 0..20_000 {
        let op = r.next_i32();
        let v = r.interesting_i32();
        let c = p.c.switch_fallthrough_calculator(v, op);
        let rs = p.rs.switch_fallthrough_calculator(v, op);
        cmp("switch_fallthrough_calculator (G5)", (v, op), c, rs);
        if !(0..=4).contains(&op) {
            assert_eq!(c, 0, "C must return 0 for out-of-range operation {op}");
        }
    }
    // Exhaustive over a window straddling the valid range.
    for op in -64..=64 {
        for &v in &[0, 1, -1, 8, 64, 128, 511, i32::MAX, i32::MIN] {
            let c = p.c.switch_fallthrough_calculator(v, op);
            let rs = p.rs.switch_fallthrough_calculator(v, op);
            cmp("switch_fallthrough_calculator (G5 window)", (v, op), c, rs);
        }
    }
    // Single-bit and single-bit-complement patterns.
    for bit in 0..32 {
        for op in [1i32 << bit, !(1i32 << bit), (1i32 << bit).wrapping_neg()] {
            for &v in &[0, 1, -1, i32::MAX, i32::MIN] {
                let c = p.c.switch_fallthrough_calculator(v, op);
                let rs = p.rs.switch_fallthrough_calculator(v, op);
                cmp("switch_fallthrough_calculator (G5 bits)", (v, op), c, rs);
            }
        }
    }
}

// ===========================================================================
// G6 -- INT_MIN / INT_MAX for every integer parameter of every entry point.
// ===========================================================================
#[test]
fn err_g6_extreme_ints() {
    let p = pair();
    const EXTREMES: [i32; 6] = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        i32::MAX - 1,
        i32::MAX,
    ];

    // switch_fallthrough_calculator: both parameters.
    for &v in &EXTREMES {
        for &op in &EXTREMES {
            cmp_switch(p, v, op);
        }
        for op in 0..=4 {
            cmp_switch(p, v, op);
        }
    }

    // allocate_and_compute: extreme size (negative ones fail, 0 succeeds).
    for &size in &EXTREMES {
        for m in [1.5f64, 0.0, -1.0, f64::NAN, f64::INFINITY] {
            let c = p.c.allocate_and_compute(size, m);
            let rs = p.rs.allocate_and_compute(size, m);
            assert_eq!(
                rs, c,
                "allocate_and_compute({size}, {m:?}): C={c} Rust={rs}"
            );
        }
    }

    // foreach_sum / process_array_reverse: extreme counts (all <= 0 except the
    // huge positives, which we must not run -- they would read out of bounds in
    // BOTH implementations, which is the C's own UB, not a translation gap).
    for &count in &[i32::MIN, i32::MIN + 1, -1, 0] {
        let mut a = vec![7i32; 4];
        let mut b = a.clone();
        let c = p.c.foreach_sum(a.as_mut_ptr(), count);
        let rs = p.rs.foreach_sum(b.as_mut_ptr(), count);
        assert_eq!(rs, c, "foreach_sum(buf, {count}): C={c} Rust={rs}");
        let c = unsafe { p.c.process_array_reverse(a.as_mut_ptr().add(3), count) };
        let rs = unsafe { p.rs.process_array_reverse(b.as_mut_ptr().add(3), count) };
        assert_eq!(rs, c, "process_array_reverse(buf+3, {count}): C={c} Rust={rs}");
    }

    // fallcalc: all 6^4 = 1296 extreme combinations.
    for &a in &EXTREMES {
        for &b in &EXTREMES {
            for &c in &EXTREMES {
                for &d in &EXTREMES {
                    cmp_fallcalc(p, a, b, c, d);
                }
            }
        }
    }
}

// ===========================================================================
// G7 -- signed-overflow arithmetic. C is formally UB here; gcc at -O0 wraps and
// the Rust translation uses `wrapping_*`, so the two must agree bit for bit.
// ===========================================================================
#[test]
fn err_g7_overflow_wrap() {
    let p = pair();
    let mut r = Rng::new(0x0777_0000);

    // value * 8 (+128) overflow in arm 0.
    for delta in -4..=4i32 {
        for base in [i32::MAX / 8, i32::MIN / 8, i32::MAX, i32::MIN] {
            cmp_switch(p, base.wrapping_add(delta), 0);
        }
    }
    // value + 128 overflow in arms 0/1.
    for delta in -4..=4i32 {
        let v = (i32::MAX - 128).wrapping_add(delta);
        cmp_switch(p, v, 1);
        cmp_switch(p, v, 0);
    }
    // value * 3 (+64) overflow in arm 3.
    for delta in -4..=4i32 {
        for base in [i32::MAX / 3, i32::MIN / 3, i32::MAX, i32::MIN] {
            cmp_switch(p, base.wrapping_add(delta), 3);
        }
    }
    // value + 64 overflow in arm 4.
    for delta in -4..=4i32 {
        cmp_switch(p, (i32::MAX - 64).wrapping_add(delta), 4);
        cmp_switch(p, (i32::MIN + 64).wrapping_add(delta), 4);
    }
    // param1 * 64 + param2 overflow in fallcalc.
    for delta in -3..=3i32 {
        for base in [i32::MAX / 64, i32::MIN / 64, i32::MAX, i32::MIN] {
            for p2 in [i32::MAX, i32::MIN, 0, 1, -1] {
                cmp_fallcalc(p, base.wrapping_add(delta), p2, 1, 1);
            }
        }
    }
    // (i+1)*8 + param1 overflow in the array fill.
    for p1 in [i32::MAX, i32::MAX - 1, i32::MAX - 40, i32::MIN, i32::MIN + 40] {
        for _ in 0..200 {
            cmp_fallcalc(p, p1, r.next_i32(), r.next_i32(), r.next_i32());
        }
    }
    // Accumulator wrap in foreach_sum / process_array_reverse.
    for _ in 0..2000 {
        let n = r.range_i32(1, 40) as usize;
        let buf: Vec<i32> = (0..n)
            .map(|_| {
                if r.next_u64() & 1 == 0 {
                    i32::MAX - r.range_i32(0, 4)
                } else {
                    i32::MIN + r.range_i32(0, 4)
                }
            })
            .collect();
        cmp_foreach(p, &buf, n as i32);
        cmp_reverse(p, &buf, n - 1, n as i32);
    }
}

// ===========================================================================
// G3 -- oversized lengths for the pointer-taking functions.
// A positive `count` larger than the buffer is an out-of-bounds read in the C
// itself (UB), so instead of invoking it we verify the boundary: `count` equal
// to the buffer length is the largest legal value and must match.
// ===========================================================================
#[test]
fn err_g3_length_boundaries() {
    let p = pair();
    let mut r = Rng::new(0x0333_0000);
    for n in 1..=64usize {
        let buf: Vec<i32> = (0..n).map(|_| r.interesting_i32()).collect();
        // count == len (largest in-bounds), count == len - 1, count == 0.
        cmp_foreach(p, &buf, n as i32);
        cmp_foreach(p, &buf, n as i32 - 1);
        cmp_foreach(p, &buf, 0);
        cmp_reverse(p, &buf, n - 1, n as i32);
        cmp_reverse(p, &buf, n - 1, n as i32 - 1);
        cmp_reverse(p, &buf, n - 1, 0);
    }
    // allocate_and_compute: sizes that succeed but are far larger than anything
    // `fallcalc` produces (kept <= 16 MiB of payload so the test stays fast).
    for size in [1 << 16, 1 << 20] {
        let c = p.c.allocate_and_compute(size, 0.0);
        let rs = p.rs.allocate_and_compute(size, 0.0);
        assert_eq!(rs, c, "allocate_and_compute({size}, 0.0): C={c} Rust={rs}");
    }
}
