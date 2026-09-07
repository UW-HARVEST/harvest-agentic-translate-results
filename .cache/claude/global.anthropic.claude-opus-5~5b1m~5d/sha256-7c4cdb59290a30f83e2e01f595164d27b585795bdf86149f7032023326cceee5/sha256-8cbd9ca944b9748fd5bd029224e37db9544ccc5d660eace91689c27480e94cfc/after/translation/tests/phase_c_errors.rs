// Phase C — error-path / boundary differential tests.
//
// The C library (`c_src/src/driver.c`) contains ZERO rejection paths: no
// `return`, no error macro, no `assert`, no range check, no null check, no
// `errno`. `ERRORS.md` therefore has an empty error table, and the phase's
// obligation is discharged against the generic boundaries every C API has,
// reinterpreted for `void driver(double)` — rows E1..E9 of `ERRORS.md`.
//
// For this API "returns the same error/sentinel" means "writes the same bytes to
// stdout", since that is the function's only observable result. Each test loads
// both `.so`s and compares byte-for-byte, one input at a time so a divergence
// names the exact offending value.

mod harness;

use harness::*;

/// Per-input (not batched) comparison, so each boundary value is individually
/// attributed.
fn assert_each(row: &str, xs: &[f64]) {
    let cf = c_driver();
    let rf = rust_driver();
    for &x in xs {
        let c = run_one(cf, x);
        let r = run_one(rf, x);
        assert_eq!(
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
            "[{row}] divergence on input {:?} (bits 0x{:016x})",
            x,
            x.to_bits()
        );
        assert!(!c.is_empty(), "[{row}] C produced no output for {x:?}");
    }
}

fn assert_each_bits(row: &str, bits: &[u64]) {
    let xs: Vec<f64> = bits.iter().copied().map(f64::from_bits).collect();
    assert_each(row, &xs);
}

fn next_up(x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY {
        return x;
    }
    if x == 0.0 {
        return f64::from_bits(1);
    }
    let b = x.to_bits();
    f64::from_bits(if x > 0.0 { b + 1 } else { b - 1 })
}

fn next_down(x: f64) -> f64 {
    if x.is_nan() || x == f64::NEG_INFINITY {
        return x;
    }
    if x == 0.0 {
        return f64::from_bits(1 | (1u64 << 63));
    }
    let b = x.to_bits();
    f64::from_bits(if x > 0.0 { b - 1 } else { b + 1 })
}

// ---------------------------------------------------------------------------
// E1 — null pointer
// ---------------------------------------------------------------------------
/// `driver` takes no pointer, so there is no null-pointer input to construct.
/// This test pins that fact down structurally: the symbol is called through a
/// strictly by-value `extern "C" fn(f64)` signature in BOTH libraries, and both
/// resolve. If either implementation ever grew a pointer parameter the ABI
/// mismatch would show up here.
#[test]
fn err_no_pointer_parameters() {
    let cf: DriverFn = c_driver();
    let rf: DriverFn = rust_driver();
    // Both symbols resolved under `extern "C" fn(f64)`; exercise them once.
    let c = run_one(cf, 1.5);
    let r = run_one(rf, 1.5);
    assert_eq!(c, r, "E1: by-value ABI mismatch");
    // The format string is private to each library and never caller-supplied,
    // so there is no pointer for a caller to pass as NULL.
    assert!(!c.is_empty());
}

// ---------------------------------------------------------------------------
// E2 — "zero length": there is no length parameter; nearest analogue is zero
// ---------------------------------------------------------------------------
#[test]
fn err_zero_and_signed_zero() {
    assert_each("E2 zeros", &[0.0, -0.0, 1.0, -1.0]);
    assert_each_bits("E2 zero bit patterns", &[0x0000_0000_0000_0000, 0x8000_0000_0000_0000]);
}

// ---------------------------------------------------------------------------
// E3 — "oversized length": no length parameter; nearest analogue is the maximal
// `%.4f` field width
// ---------------------------------------------------------------------------
#[test]
fn err_oversized_field_width() {
    let xs = [
        f64::MAX,
        -f64::MAX,
        f64::from_bits(0x7fef_ffff_ffff_ffff), // 0x1.fffffffffffffp+1023
        f64::from_bits(0xffef_ffff_ffff_ffff),
        1e308,
        -1e308,
        next_down(f64::MAX),
        next_up(-f64::MAX),
    ];
    assert_each("E3 oversized field width", &xs);

    // Confirm the boundary really is the extreme one: ~310 integer digits.
    let out = run_one(c_driver(), f64::MAX);
    assert!(
        out.len() > 300,
        "E3 expected a maximal-width line, got {} bytes",
        out.len()
    );
}

// ---------------------------------------------------------------------------
// E4 — one step past the top of the valid range
// ---------------------------------------------------------------------------
#[test]
fn err_one_past_range_infinities() {
    // `nextafter(DBL_MAX, +inf)` is exactly +inf: the first value past the
    // representable finite range.
    let past_max = f64::from_bits(f64::MAX.to_bits() + 1);
    assert_eq!(past_max, f64::INFINITY);
    let past_min = f64::from_bits((-f64::MAX).to_bits() + 1);
    assert_eq!(past_min, f64::NEG_INFINITY);

    assert_each(
        "E4 one past range",
        &[past_max, past_min, f64::INFINITY, f64::NEG_INFINITY, f64::MAX, -f64::MAX],
    );

    // And one step past infinity itself is a NaN — still a legal input.
    assert_each_bits("E4 one past infinity", &[0x7ff0_0000_0000_0001, 0xfff0_0000_0000_0001]);
}

// ---------------------------------------------------------------------------
// E5 — one step past the bottom of the range (underflow into subnormals)
// ---------------------------------------------------------------------------
#[test]
fn err_one_past_range_subnormals() {
    let min_sub = f64::from_bits(1);
    let max_sub = f64::from_bits((1u64 << 52) - 1);
    let min_norm = f64::MIN_POSITIVE;

    assert_eq!(next_up(0.0), min_sub);
    assert_eq!(next_down(min_norm), max_sub);
    assert_eq!(next_up(max_sub), min_norm);

    let xs = [
        min_sub,
        -min_sub,
        max_sub,
        -max_sub,
        min_norm,
        -min_norm,
        next_up(min_sub),
        next_down(-min_sub),
        next_down(0.0),
        next_up(0.0),
        next_up(min_norm),
        next_down(min_norm),
    ];
    assert_each("E5 one past range (subnormals)", &xs);
}

// ---------------------------------------------------------------------------
// E6 — the "out-of-range enum value" analogue: bit patterns with no valid
// numeric variant, i.e. every NaN class, passed across the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn err_nan_variants_no_valid_variant() {
    let bits = [
        0x7ff8_0000_0000_0000, // +quiet NaN (canonical)
        0xfff8_0000_0000_0000, // -quiet NaN
        0x7ff0_0000_0000_0001, // +signalling NaN, minimal payload
        0xfff0_0000_0000_0001, // -signalling NaN, minimal payload
        0x7ff7_ffff_ffff_ffff, // +sNaN, maximal payload
        0xfff7_ffff_ffff_ffff, // -sNaN, maximal payload
        0x7ff8_dead_beef_cafe, // +qNaN, arbitrary payload
        0xfff8_dead_beef_cafe, // -qNaN, arbitrary payload
        0x7fff_ffff_ffff_ffff, // all payload bits set, positive
        0xffff_ffff_ffff_ffff, // all bits set
        0x7ffa_5555_5555_5555,
        0xfffa_aaaa_aaaa_aaaa,
    ];
    for &b in &bits {
        assert!(f64::from_bits(b).is_nan(), "0x{b:016x} should be a NaN");
    }
    assert_each_bits("E6 NaN variants", &bits);

    // Exhaustive-ish: every NaN payload with a single mantissa bit set, both
    // signs — 52 x 2 patterns with no valid numeric variant.
    let mut sweep = Vec::new();
    for bit in 0..52u64 {
        sweep.push((2047u64 << 52) | (1 << bit));
        sweep.push((1u64 << 63) | (2047u64 << 52) | (1 << bit));
    }
    assert_each_bits("E6 NaN single-bit payload sweep", &sweep);
}

// ---------------------------------------------------------------------------
// E7 — every IEEE-754 class, built from raw bits
// ---------------------------------------------------------------------------
#[test]
fn err_all_ieee754_classes() {
    // All 2048 exponent fields x a mantissa that is zero, minimal, mid and
    // maximal x both signs: covers zero, subnormal, normal, infinity and NaN,
    // including patterns no decimal literal can name.
    const MS: &[u64] = &[0, 1, 0x8_0000_0000_0000, 0xf_ffff_ffff_ffff];
    let mut bits = Vec::with_capacity(2048 * MS.len() * 2);
    for e in 0..2048u64 {
        for &m in MS {
            bits.push((e << 52) | m);
            bits.push((1u64 << 63) | (e << 52) | m);
        }
    }

    // Batched comparison for the bulk (16k inputs), which still bisects to the
    // exact value on failure.
    assert_same_bits("E7 all IEEE-754 classes", &bits);

    // Verify the corpus really spans every class.
    let classes: Vec<_> = bits.iter().map(|&b| f64::from_bits(b).classify()).collect();
    use std::num::FpCategory::*;
    for want in [Zero, Subnormal, Normal, Infinite, Nan] {
        assert!(classes.contains(&want), "corpus missing class {want:?}");
    }
}

// ---------------------------------------------------------------------------
// E8 — rounding-tie boundary: the silent-wrong-answer class
// ---------------------------------------------------------------------------
#[test]
fn err_rounding_ties() {
    let mut xs = Vec::new();
    let bases: &[f64] = &[
        0.00005, 0.00015, 0.00025, 0.00035, 2.00005, 1.0000500000000001, 0.5 / 10000.0,
        1.5 / 10000.0, 2.5 / 10000.0, 0.99995, 9.99995,
    ];
    for &b in bases {
        for &sv in &[b, -b] {
            xs.push(sv);
            xs.push(next_up(sv));
            xs.push(next_down(sv));
        }
    }
    assert_each("E8 rounding ties", &xs);
}

// ---------------------------------------------------------------------------
// E9 — re-entrancy / interleaving with the caller's own C stdout
// ---------------------------------------------------------------------------
#[test]
fn err_interleaved_with_caller_stdout() {
    let cf = c_driver();
    let rf = rust_driver();
    let mut r = Rng::new(0xE9_E9_E9);
    let sample: Vec<f64> = (0..500).map(|_| f64::from_bits(r.next_u64())).collect();

    let run = |f: DriverFn| {
        capture_stdout(|| {
            for (i, &x) in sample.iter().enumerate() {
                caller_printf_marker(i);
                unsafe { f(x) };
                unsafe { f(x) };
                caller_printf_marker(i);
            }
        })
    };

    let c = run(cf);
    let s = run(rf);
    assert_eq!(
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&s),
        "E9 interleaved stream divergence"
    );
    assert!(c.len() > 1000, "E9 capture unexpectedly small: {} bytes", c.len());
}

// ---------------------------------------------------------------------------
// Extra: both libraries must export exactly the symbol the header declares, and
// nothing the test relies on may be a stub. A stub would produce no output.
// ---------------------------------------------------------------------------
#[test]
fn err_rust_export_is_not_a_stub() {
    let out = run_one(rust_driver(), 1.0);
    assert_eq!(
        String::from_utf8_lossy(&out),
        "3ff0000000000000 0x1p+0 1.0000\n",
        "the Rust export must do the real work, not stub out"
    );
    let c = run_one(c_driver(), 1.0);
    assert_eq!(out, c);
}
