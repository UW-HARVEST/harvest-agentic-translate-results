//! Phase C — error-path differential tests.
//!
//! One test (or test group) per row of `ERRORS.md`. Every test constructs the
//! exact invalid input/condition, calls BOTH the C `.so` and the Rust `.so`
//! through `libloading`, and asserts they produce the SAME sentinel / clamp /
//! signal — not merely "both failed somehow".

mod harness;

use harness::*;
use std::ffi::{c_int, c_void};

/// Assert C and Rust agree *and* that the shared value is the sentinel the C
/// source specifies for this rejection.
#[track_caller]
fn diff_sdti_expect(d: f64, expected: c_int, row: &str) {
    let (c, r) = both();
    let cv = c.safe_double_to_int(d);
    let rv = r.safe_double_to_int(d);
    let ctx = format!("ERRORS {row}: safe_double_to_int({d:?}, bits {:#018x})", d.to_bits());
    assert_int_eq(&ctx, cv, rv);
    assert_eq!(cv, expected, "{ctx}: C sentinel changed from the documented {expected}");
}

// ===========================================================================
// ERRORS row 1 — d > (double)INT_MAX  =>  INT_MAX
// ===========================================================================
fn err01_sdti_above_int_max_clamps() {
    let mut rng = Rng::new(0xE770_0001);
    for d in [
        2147483648.0f64,
        2147483649.0,
        2147483647.5,
        1e15,
        1e100,
        1e300,
        f64::MAX,
        4294967296.0,
    ] {
        diff_sdti_expect(d, c_int::MAX, "1");
    }
    for _ in 0..5_000 {
        // Anything strictly greater than 2147483647.0.
        let d = 2147483648.0f64 + (rng.next_u32() as f64) * 1e3;
        diff_sdti_expect(d, c_int::MAX, "1");
    }
}

// ===========================================================================
// ERRORS row 2 — one ULP past (double)INT_MAX  =>  INT_MAX
// ===========================================================================
fn err02_sdti_one_ulp_above_int_max() {
    let hi = c_int::MAX as f64;
    diff_sdti_expect(hi.next_up(), c_int::MAX, "2");
    diff_sdti_expect(f64::from_bits(hi.to_bits() + 1), c_int::MAX, "2");
}

// ===========================================================================
// ERRORS row 3 — +INFINITY  =>  INT_MAX (first branch)
// ===========================================================================
fn err03_sdti_positive_infinity() {
    diff_sdti_expect(f64::INFINITY, c_int::MAX, "3");
    diff_sdti_expect(1.0 / 0.0, c_int::MAX, "3");
}

// ===========================================================================
// ERRORS row 4 — d < (double)INT_MIN  =>  INT_MIN
// ===========================================================================
fn err04_sdti_below_int_min_clamps() {
    let mut rng = Rng::new(0xE770_0004);
    for d in [
        -2147483649.0f64,
        -2147483648.5,
        -2147483650.0,
        -1e15,
        -1e100,
        -1e300,
        f64::MIN,
        -4294967296.0,
    ] {
        diff_sdti_expect(d, c_int::MIN, "4");
    }
    for _ in 0..5_000 {
        let d = -2147483649.0f64 - (rng.next_u32() as f64) * 1e3;
        diff_sdti_expect(d, c_int::MIN, "4");
    }
}

// ===========================================================================
// ERRORS row 5 — one ULP past (double)INT_MIN  =>  INT_MIN
// ===========================================================================
fn err05_sdti_one_ulp_below_int_min() {
    let lo = c_int::MIN as f64;
    diff_sdti_expect(lo.next_down(), c_int::MIN, "5");
    // For a negative double, increasing the raw bits moves away from zero.
    diff_sdti_expect(f64::from_bits(lo.to_bits() + 1), c_int::MIN, "5");
}

// ===========================================================================
// ERRORS row 6 — -INFINITY  =>  INT_MIN (second branch)
// ===========================================================================
fn err06_sdti_negative_infinity() {
    diff_sdti_expect(f64::NEG_INFINITY, c_int::MIN, "6");
    diff_sdti_expect(-1.0 / 0.0, c_int::MIN, "6");
}

// ===========================================================================
// ERRORS row 7 — quiet NaN  =>  0 (both comparisons false, isnan arm)
// ===========================================================================
fn err07_sdti_quiet_nan_returns_zero() {
    diff_sdti_expect(f64::NAN, 0, "7");
    diff_sdti_expect(0.0 / 0.0, 0, "7");
    diff_sdti_expect(f64::INFINITY - f64::INFINITY, 0, "7");
    diff_sdti_expect((-1.0f64).sqrt(), 0, "7");
}

// ===========================================================================
// ERRORS row 8 — signalling / sign-negative / payload NaNs  =>  0
// ===========================================================================
fn err08_sdti_all_nan_encodings_return_zero() {
    let mut rng = Rng::new(0xE770_0008);
    diff_sdti_expect(-f64::NAN, 0, "8");
    // Canonical quiet NaN, signalling NaN, and min/max payloads, both signs.
    for payload in [1u64, 2, 0x7_FFFF_FFFF_FFFF, 0x8_0000_0000_0001, 0xF_FFFF_FFFF_FFFF] {
        for sign in [0u64, 1u64 << 63] {
            let bits = sign | 0x7FF0_0000_0000_0000 | (payload & 0x000F_FFFF_FFFF_FFFF);
            let d = f64::from_bits(bits);
            assert!(d.is_nan(), "test setup: {bits:#018x} should be NaN");
            diff_sdti_expect(d, 0, "8");
        }
    }
    // Randomized NaN payloads.
    let mut seen = 0usize;
    for _ in 0..20_000 {
        let d = rng.next_f64_bits();
        if d.is_nan() {
            seen += 1;
            diff_sdti_expect(d, 0, "8");
        }
    }
    assert!(seen > 10, "expected random bit patterns to include NaNs, saw {seen}");
}

// ===========================================================================
// ERRORS rows 9-10 — the boundary values that are *inside* the range and must
// NOT be clamped (they go through the `(int)d` cast instead).
// ===========================================================================
fn err09_sdti_exactly_int_max_is_not_clamped_by_the_guard() {
    diff_sdti_expect(c_int::MAX as f64, c_int::MAX, "9");
    diff_sdti_expect(2147483646.0, 2147483646, "9");
    diff_sdti_expect(2147483646.999, 2147483646, "9");
}

fn err10_sdti_exactly_int_min_is_not_clamped_by_the_guard() {
    diff_sdti_expect(c_int::MIN as f64, c_int::MIN, "10");
    diff_sdti_expect(-2147483647.0, -2147483647, "10");
    diff_sdti_expect(-2147483647.999, -2147483647, "10");
}

// ===========================================================================
// ERRORS rows 11-13 — process_with_fallthrough `default:` arm returns -1.
// This includes out-of-range "enum-like" ints crossing the FFI boundary.
// ===========================================================================
#[track_caller]
fn diff_pwf_expect(code: c_int, base: c_int, expected: c_int, row: &str) {
    let (c, r) = both();
    let cv = c.process_with_fallthrough(code, base);
    let rv = r.process_with_fallthrough(code, base);
    let ctx = format!("ERRORS {row}: process_with_fallthrough({code}, {base})");
    assert_int_eq(&ctx, cv, rv);
    assert_eq!(cv, expected, "{ctx}: C result changed from the documented {expected}");
}

fn err11_fallthrough_negative_codes_return_minus_one() {
    let mut rng = Rng::new(0xE770_000B);
    for code in [-1i32, -2, -3, -4, -5, -6, -7, -100, c_int::MIN, c_int::MIN + 1] {
        for base in [0i32, 1, -1, 12345, c_int::MAX, c_int::MIN] {
            diff_pwf_expect(code, base, -1, "11");
        }
    }
    for _ in 0..5_000 {
        let code = -(1 + (rng.next_u32() % 1_000_000) as i32);
        diff_pwf_expect(code, rng.next_i32(), -1, "11");
    }
}

fn err12_fallthrough_one_past_range_returns_minus_one() {
    // `code == 6` is exactly one step past the largest valid case label.
    for base in [0i32, 1, -1, 999, c_int::MAX, c_int::MIN] {
        diff_pwf_expect(6, base, -1, "12");
        diff_pwf_expect(7, base, -1, "12");
    }
}

fn err13_fallthrough_far_out_of_range_enum_values_return_minus_one() {
    let mut rng = Rng::new(0xE770_000D);
    for code in [c_int::MAX, c_int::MAX - 1, 1 << 30, 1 << 20, 0x7FFF_FFFE] {
        diff_pwf_expect(code, 42, -1, "13");
    }
    for _ in 0..5_000 {
        let code = 6 + (rng.next_u32() % (i32::MAX as u32 - 6)) as i32;
        assert!(code > 5, "test setup");
        diff_pwf_expect(code, rng.next_i32(), -1, "13");
    }
}

// ===========================================================================
// ERRORS row 14 — `code == 0` is a *valid* sentinel-looking value that forces
// the result to 0 and discards `base_value` entirely.
// ===========================================================================
fn err14_fallthrough_code_zero_discards_base_value() {
    let mut rng = Rng::new(0xE770_000E);
    for base in [0i32, 1, -1, c_int::MAX, c_int::MIN, 987_654_321] {
        diff_pwf_expect(0, base, 0, "14");
    }
    for _ in 0..5_000 {
        diff_pwf_expect(0, rng.next_i32(), 0, "14");
    }
}

// ===========================================================================
// ERRORS rows 15-16 — signed overflow / underflow inside the fall-through chain
// ===========================================================================
fn err15_fallthrough_overflow_wraps() {
    diff_pwf_expect(5, c_int::MAX, c_int::MAX.wrapping_add(150), "15");
    assert_eq!(c_int::MAX.wrapping_add(150), -2147483499, "documented value");
    // Every partially-overflowing step of the chain.
    // Fall-through totals from lib.c:54-72: 5→50+40+30+20+10, 4→40+30+20+10,
    // 3→30+20+10, 2→20+10, 1→10.
    for (code, add) in [(5i32, 150i32), (4, 100), (3, 60), (2, 30), (1, 10)] {
        diff_pwf_expect(code, c_int::MAX, c_int::MAX.wrapping_add(add), "15");
        diff_pwf_expect(
            code,
            c_int::MAX - add + 1,
            (c_int::MAX - add + 1).wrapping_add(add),
            "15",
        );
        diff_pwf_expect(code, c_int::MAX - add, c_int::MAX, "15");
    }
}

fn err16_fallthrough_underflow_boundary() {
    diff_pwf_expect(5, c_int::MIN, c_int::MIN.wrapping_add(150), "16");
    assert_eq!(c_int::MIN.wrapping_add(150), -2147483498, "documented value");
    // Fall-through totals from lib.c:54-72: 5→50+40+30+20+10, 4→40+30+20+10,
    // 3→30+20+10, 2→20+10, 1→10.
    for (code, add) in [(5i32, 150i32), (4, 100), (3, 60), (2, 30), (1, 10)] {
        diff_pwf_expect(code, c_int::MIN, c_int::MIN.wrapping_add(add), "16");
        diff_pwf_expect(code, c_int::MIN + 1, (c_int::MIN + 1).wrapping_add(add), "16");
    }
}

// ===========================================================================
// ERRORS rows 17-19 — copy_data_block has NO null check. The C dereferences
// unconditionally, so the observable behaviour is "the process dies with a
// signal". We fork a child for each library and require the SAME signal.
// ===========================================================================

fn null_outcome_pair(dest_null: bool, src_null: bool) -> (ChildOutcome, ChildOutcome) {
    let (capi, rapi) = both();
    let run = |api: &'static Api| {
        run_in_child(move || {
            let mut buf = RawBlock::filled(0x33);
            let dest: *mut c_void = if dest_null {
                std::ptr::null_mut()
            } else {
                buf.0.as_mut_ptr().cast()
            };
            let src: *const c_void = if src_null {
                std::ptr::null()
            } else {
                buf.0.as_ptr().cast()
            };
            unsafe { api.copy_data_block_raw(dest, src) };
            // Defeat any chance of the call being optimised away.
            std::hint::black_box(&buf);
        })
    };
    (run(capi), run(rapi))
}

fn err17_copy_data_block_null_dest_same_signal() {
    let (c, r) = null_outcome_pair(true, false);
    assert_eq!(c, r, "ERRORS 17: dest=NULL — C {c:?} vs Rust {r:?}");
    assert_eq!(
        c,
        ChildOutcome::Signalled(11),
        "ERRORS 17: expected SIGSEGV from the unchecked memcpy, got {c:?}"
    );
}

fn err18_copy_data_block_null_src_same_signal() {
    let (c, r) = null_outcome_pair(false, true);
    assert_eq!(c, r, "ERRORS 18: src=NULL — C {c:?} vs Rust {r:?}");
    assert_eq!(
        c,
        ChildOutcome::Signalled(11),
        "ERRORS 18: expected SIGSEGV from the unchecked memcpy, got {c:?}"
    );
}

fn err19_copy_data_block_both_null_same_signal() {
    let (c, r) = null_outcome_pair(true, true);
    assert_eq!(c, r, "ERRORS 19: dest=src=NULL — C {c:?} vs Rust {r:?}");
    assert_eq!(
        c,
        ChildOutcome::Signalled(11),
        "ERRORS 19: expected SIGSEGV from the unchecked memcpy, got {c:?}"
    );
}

/// Same class, non-null but unmapped pointer — still no validation, still the
/// identical fault in both libraries.
fn err19b_copy_data_block_unmapped_pointer_same_signal() {
    let (capi, rapi) = both();
    let bad: usize = 0x1; // misaligned and unmapped
    let run = |api: &'static Api| {
        run_in_child(move || {
            let mut buf = RawBlock::filled(0x77);
            unsafe {
                api.copy_data_block_raw(buf.0.as_mut_ptr().cast(), bad as *const c_void);
            }
            std::hint::black_box(&buf);
        })
    };
    let (c, r) = (run(capi), run(rapi));
    assert_eq!(c, r, "ERRORS 19b: unmapped src — C {c:?} vs Rust {r:?}");
    assert!(
        matches!(c, ChildOutcome::Signalled(_)),
        "expected a fatal signal, got {c:?}"
    );
}

// ===========================================================================
// ERRORS row 20 — copy_data_block always reads/writes the FULL 40-byte object,
// padding included; there is no length parameter and no truncation.
// ===========================================================================
fn err20_copy_data_block_reads_full_struct_incl_padding() {
    let mut rng = Rng::new(0xE770_0014);
    let (capi, rapi) = both();

    // A source whose padding bytes and un-terminated label are non-zero:
    // a length-validating implementation would drop them.
    for _ in 0..2_000 {
        let mut src = RawBlock::zeroed();
        for i in 0..DATA_BLOCK_SIZE {
            src.0[i] = (rng.next_u32() | 1) as u8; // never zero
        }
        let cv = capi.copy_block(&src, 0x00);
        let rv = rapi.copy_block(&src, 0x00);
        assert_bytes_eq("ERRORS 20: full 40-byte copy", &cv.0, &rv.0);
        assert_bytes_eq("ERRORS 20: C copied all 40 bytes", &src.0, &cv.0);
        assert!(
            cv.0.iter().all(|&b| b != 0),
            "no byte of the destination was left untouched"
        );
    }

    // Exactly-40-byte source buffer that is *not* zero-terminated anywhere:
    // still copied wholesale, no rejection, no NUL added.
    let src = RawBlock::filled(0xA5);
    let cv = capi.copy_block(&src, 0x00);
    let rv = rapi.copy_block(&src, 0x00);
    assert_bytes_eq("ERRORS 20: unterminated source", &cv.0, &rv.0);
    assert_eq!(cv.0, [0xA5u8; DATA_BLOCK_SIZE]);
}

// ===========================================================================
// ERRORS rows 21-22 — handle_pointer_operations `value * 2` signed overflow
// ===========================================================================
fn err21_22_handle_pointer_operations_overflow() {
    let (c, r) = both();
    // INT_MAX * 2 wraps to -2, so the result is 98 (NOT a saturated value).
    let cases = [
        (c_int::MAX, 98i32),  // row 21
        (c_int::MIN, 100),    // row 22
        (c_int::MAX - 1, 96), // (-4) + 100
        (c_int::MIN + 1, 102),
        (1 << 30, -2147483548), // 2^31 wraps to INT_MIN, + 100
    ];
    for (v, expected) in cases {
        let cv = c.handle_pointer_operations(v);
        let rv = r.handle_pointer_operations(v);
        let ctx = format!("ERRORS 21/22: handle_pointer_operations({v})");
        assert_int_eq(&ctx, cv, rv);
        assert_eq!(
            cv, expected,
            "{ctx}: C result changed from the documented {expected}"
        );
        assert_eq!(cv, v.wrapping_mul(2).wrapping_add(100), "{ctx}: wrap model");
    }
    // Randomized sweep restricted to inputs that DO overflow.
    let mut rng = Rng::new(0xE770_0015);
    for _ in 0..10_000 {
        let v = rng.next_i32();
        if v.checked_mul(2).is_some() {
            continue;
        }
        let cv = c.handle_pointer_operations(v);
        let rv = r.handle_pointer_operations(v);
        assert_int_eq(&format!("ERRORS 21/22 random overflow ({v})"), cv, rv);
        assert_eq!(cv, v.wrapping_mul(2).wrapping_add(100));
    }
}

// ===========================================================================
// ERRORS rows 23-24 — negative `a` makes `a % 6` negative, so `overunder`'s
// internal switch takes the `default:` arm and folds -1 into `total`.
// ===========================================================================
fn err23_overunder_negative_a_takes_default_arm() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0xE770_0017);
    for _ in 0..400 {
        let a = -(1 + (rng.next_u32() % 100_000_000) as i32);
        assert!(a < 0 && a % 6 <= 0, "test setup: a={a}, a%6={}", a % 6);
        let b = rng.next_i32_bounded(100_000);
        let c = rng.next_i32_bounded(100_000);
        let d = rng.next_i32_bounded(20_000);
        diff_overunder(&mut cap, a, b, c, d);
    }
    // Verify through stdout that the default arm really was taken (-1), for the
    // negative residues where C's truncating `%` guarantees it.
    let (capi, rapi) = both();
    for a in [-1i32, -2, -3, -4, -5, -7, -8, -11, -13] {
        let (c_out, c_ret) = cap.run(|| capi.overunder(a, 7, 9, 11));
        let (r_out, r_ret) = cap.run(|| rapi.overunder(a, 7, 9, 11));
        assert_int_eq(&format!("ERRORS 23: overunder({a},7,9,11)"), c_ret, r_ret);
        assert_bytes_eq("ERRORS 23 stdout", &c_out, &r_out);
        let text = String::from_utf8(c_out).unwrap();
        assert!(
            text.contains("Switch fall-through result: -1"),
            "a={a} (a%6={}) should hit the default arm; got:\n{text}",
            a % 6
        );
    }
}

fn err24_overunder_int_min_a_default_arm_and_square_overflow() {
    let mut cap = Capturer::new();
    let (capi, rapi) = both();
    assert_eq!(c_int::MIN % 6, -2, "C truncating remainder");
    for d in [0i32, 1, -1, c_int::MAX, c_int::MIN, 46341, 100_000] {
        let (c_out, c_ret) = cap.run(|| capi.overunder(c_int::MIN, 3, 4, d));
        let (r_out, r_ret) = cap.run(|| rapi.overunder(c_int::MIN, 3, 4, d));
        assert_int_eq(&format!("ERRORS 24: overunder(INT_MIN,3,4,{d})"), c_ret, r_ret);
        assert_bytes_eq("ERRORS 24 stdout", &c_out, &r_out);
        let text = String::from_utf8(c_out).unwrap();
        assert!(
            text.contains("Switch fall-through result: -1"),
            "INT_MIN % 6 == -2 must reach the default arm; got:\n{text}"
        );
    }
}

// ===========================================================================
// ERRORS row 25 — `d*d + a*a` overflows negative => sqrt(NaN) => conv4 == 0
// ===========================================================================
fn err25_overunder_sqrt_of_negative_yields_zero_conversion() {
    let mut cap = Capturer::new();
    let (capi, rapi) = both();
    let mut rng = Rng::new(0xE770_0019);
    let mut hits = 0usize;

    for _ in 0..4_000 {
        let a = rng.next_i32();
        let d = rng.next_i32();
        if a.wrapping_mul(a).wrapping_add(d.wrapping_mul(d)) >= 0 {
            continue;
        }
        hits += 1;
        diff_overunder(&mut cap, a, 5, 7, d);
    }
    assert!(hits > 100, "expected many negative-sum samples, got {hits}");

    // A hand-verified case: a = 0, d = 46341 => 46341^2 = 2147488281 wraps to
    // -2147479015, so `d*d + a*a` is negative and `sqrt` yields NaN.
    // (Note a = d = 46341 does NOT work: the two wrapped halves sum back to a
    // positive 9266, which is exactly why this needs to be computed, not guessed.)
    let (a, d) = (0i32, 46341i32);
    assert!(
        d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a)) < 0,
        "test setup: expected a negative sum, got {}",
        d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a))
    );
    let (c_out, c_ret) = cap.run(|| capi.overunder(a, 5, 7, d));
    let (r_out, r_ret) = cap.run(|| rapi.overunder(a, 5, 7, d));
    assert_int_eq("ERRORS 25", c_ret, r_ret);
    assert_bytes_eq("ERRORS 25 stdout", &c_out, &r_out);
    let text = String::from_utf8(c_out).unwrap();
    let conv_line = text
        .lines()
        .find(|l| l.starts_with("Converted values: "))
        .unwrap();
    let last = conv_line.rsplit(", ").next().unwrap();
    assert_eq!(last, "0", "sqrt(negative) must convert to 0; line = {conv_line:?}");
}

// ===========================================================================
// ERRORS rows 26-27 — the a*1.5 / b*2.7 products clamp inside overunder
// ===========================================================================
fn err26_overunder_a_product_clamps_to_int_max() {
    let mut cap = Capturer::new();
    let (capi, rapi) = both();
    for a in [c_int::MAX, c_int::MAX - 1, 1_500_000_000, 2_000_000_000] {
        let (c_out, c_ret) = cap.run(|| capi.overunder(a, 1, 1, 1));
        let (r_out, r_ret) = cap.run(|| rapi.overunder(a, 1, 1, 1));
        assert_int_eq(&format!("ERRORS 26: overunder({a},1,1,1)"), c_ret, r_ret);
        assert_bytes_eq("ERRORS 26 stdout", &c_out, &r_out);
        let text = String::from_utf8(c_out).unwrap();
        let conv = text.lines().find(|l| l.starts_with("Converted values: ")).unwrap();
        let first = conv.trim_start_matches("Converted values: ").split(", ").next().unwrap();
        assert_eq!(first, "2147483647", "a*1.5 must clamp; line = {conv:?}");
    }
    for a in [c_int::MIN, c_int::MIN + 1, -1_500_000_000, -2_000_000_000] {
        let (c_out, c_ret) = cap.run(|| capi.overunder(a, 1, 1, 1));
        let (r_out, r_ret) = cap.run(|| rapi.overunder(a, 1, 1, 1));
        assert_int_eq(&format!("ERRORS 26: overunder({a},1,1,1)"), c_ret, r_ret);
        assert_bytes_eq("ERRORS 26 stdout", &c_out, &r_out);
        let text = String::from_utf8(c_out).unwrap();
        let conv = text.lines().find(|l| l.starts_with("Converted values: ")).unwrap();
        let first = conv.trim_start_matches("Converted values: ").split(", ").next().unwrap();
        assert_eq!(first, "-2147483648", "a*1.5 must clamp low; line = {conv:?}");
    }
}

fn err27_overunder_b_product_clamps() {
    let mut cap = Capturer::new();
    let (capi, rapi) = both();
    let check = |cap: &mut Capturer, b: c_int, expect: &str| {
        let (c_out, c_ret) = cap.run(|| capi.overunder(1, b, 1, 1));
        let (r_out, r_ret) = cap.run(|| rapi.overunder(1, b, 1, 1));
        assert_int_eq(&format!("ERRORS 27: overunder(1,{b},1,1)"), c_ret, r_ret);
        assert_bytes_eq("ERRORS 27 stdout", &c_out, &r_out);
        let text = String::from_utf8(c_out).unwrap();
        let conv = text.lines().find(|l| l.starts_with("Converted values: ")).unwrap();
        let second = conv
            .trim_start_matches("Converted values: ")
            .split(", ")
            .nth(1)
            .unwrap();
        assert_eq!(second, expect, "b*2.7 clamp; line = {conv:?}");
    };
    for b in [c_int::MAX, c_int::MAX - 1, 1_000_000_000, 900_000_000] {
        check(&mut cap, b, "2147483647");
    }
    for b in [c_int::MIN, c_int::MIN + 1, -1_000_000_000, -900_000_000] {
        check(&mut cap, b, "-2147483648");
    }
}

// ===========================================================================
// ERRORS row 28 — total accumulation wraps two's-complement, no saturation
// ===========================================================================
fn err28_overunder_total_wraps_not_saturates() {
    let mut cap = Capturer::new();
    let mut rng = Rng::new(0xE770_001C);
    let mut saw_positive = false;
    let mut saw_negative = false;
    for _ in 0..400 {
        let off = (rng.next_u32() % 5) as i32;
        let ret = diff_overunder(&mut cap, c_int::MAX - off, c_int::MAX, c_int::MAX, c_int::MAX);
        if ret > 0 {
            saw_positive = true;
        } else if ret < 0 {
            saw_negative = true;
        }
        diff_overunder(&mut cap, c_int::MIN + off, c_int::MIN, c_int::MIN, c_int::MIN);
    }
    assert!(
        saw_positive || saw_negative,
        "sanity: overunder returned only zero for extremal inputs"
    );
    // Explicit non-saturation evidence: extremal inputs do not pin the result
    // at INT_MAX / INT_MIN the way a saturating implementation would.
    let r1 = diff_overunder(&mut cap, c_int::MAX, c_int::MAX, c_int::MAX, c_int::MAX);
    let r2 = diff_overunder(&mut cap, c_int::MIN, c_int::MIN, c_int::MIN, c_int::MIN);
    assert!(
        r1 != c_int::MAX || r2 != c_int::MIN,
        "results look saturated rather than wrapped: {r1}, {r2}"
    );
}

// ===========================================================================
// ERRORS row 29 — the hard-coded ±1e15 clamps run on every overunder call
// ===========================================================================
fn err29_overunder_hardcoded_clamp_lines_always_present() {
    let mut cap = Capturer::new();
    let (capi, rapi) = both();
    let mut rng = Rng::new(0xE770_001D);
    for _ in 0..200 {
        let (a, b, c, d) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        let (c_out, c_ret) = cap.run(|| capi.overunder(a, b, c, d));
        let (r_out, r_ret) = cap.run(|| rapi.overunder(a, b, c, d));
        assert_int_eq(&format!("ERRORS 29: overunder({a},{b},{c},{d})"), c_ret, r_ret);
        assert_bytes_eq("ERRORS 29 stdout", &c_out, &r_out);
        let text = String::from_utf8(c_out).unwrap();
        assert!(
            text.contains("Overflow protected conversion: 2147483647"),
            "missing the 1e15 clamp line:\n{text}"
        );
        assert!(
            text.contains("Underflow protected conversion: -2147483648"),
            "missing the -1e15 clamp line:\n{text}"
        );
    }
    // The same two constants, called directly through the low-level export.
    diff_sdti_expect(1e15, c_int::MAX, "29");
    diff_sdti_expect(-1e15, c_int::MIN, "29");
}

// ===========================================================================
// Generic FFI-boundary boundaries (beyond the ERRORS table):
// zero / one-past-range / oversized values on every scalar entry point.
// ===========================================================================
fn generic_boundary_sweep_all_entry_points() {
    let (c, r) = both();
    let ints = [
        0i32,
        1,
        -1,
        2,
        -2,
        5,
        6,
        -5,
        -6,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i16::MAX as i32,
        i16::MIN as i32,
        i16::MAX as i32 + 1,
        i16::MIN as i32 - 1,
        u16::MAX as i32,
        u16::MAX as i32 + 1,
        1 << 30,
        -(1 << 30),
        46340,
        46341,
        -46340,
        -46341,
    ];

    for &v in &ints {
        assert_int_eq(
            &format!("handle_pointer_operations({v})"),
            c.handle_pointer_operations(v),
            r.handle_pointer_operations(v),
        );
        assert_int_eq(
            &format!("safe_double_to_int({v} as f64)"),
            c.safe_double_to_int(v as f64),
            r.safe_double_to_int(v as f64),
        );
        for &b in &ints {
            assert_int_eq(
                &format!("process_with_fallthrough({v}, {b})"),
                c.process_with_fallthrough(v, b),
                r.process_with_fallthrough(v, b),
            );
        }
    }

    // Every `double` special value through the boundary.
    for d in [
        0.0f64,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::EPSILON,
        i32::MAX as f64,
        i32::MIN as f64,
        i32::MAX as f64 + 1.0,
        i32::MIN as f64 - 1.0,
        u32::MAX as f64,
        i64::MAX as f64,
        i64::MIN as f64,
    ] {
        assert_int_eq(
            &format!("safe_double_to_int({d:?})"),
            c.safe_double_to_int(d),
            r.safe_double_to_int(d),
        );
    }
}

// ===========================================================================
// Sequential entry point (`harness = false`, see Cargo.toml).
//
// Every case below corresponds to a numbered row of the Phase A artifacts and
// is listed here explicitly so a forgotten registration is visible in review.
// ===========================================================================
fn main() -> ! {
    let cases: &[harness::Case] = &[
        ("err01_sdti_above_int_max_clamps", err01_sdti_above_int_max_clamps as fn()),
        ("err02_sdti_one_ulp_above_int_max", err02_sdti_one_ulp_above_int_max as fn()),
        ("err03_sdti_positive_infinity", err03_sdti_positive_infinity as fn()),
        ("err04_sdti_below_int_min_clamps", err04_sdti_below_int_min_clamps as fn()),
        ("err05_sdti_one_ulp_below_int_min", err05_sdti_one_ulp_below_int_min as fn()),
        ("err06_sdti_negative_infinity", err06_sdti_negative_infinity as fn()),
        ("err07_sdti_quiet_nan_returns_zero", err07_sdti_quiet_nan_returns_zero as fn()),
        ("err08_sdti_all_nan_encodings_return_zero", err08_sdti_all_nan_encodings_return_zero as fn()),
        ("err09_sdti_exactly_int_max_is_not_clamped_by_the_guard", err09_sdti_exactly_int_max_is_not_clamped_by_the_guard as fn()),
        ("err10_sdti_exactly_int_min_is_not_clamped_by_the_guard", err10_sdti_exactly_int_min_is_not_clamped_by_the_guard as fn()),
        ("err11_fallthrough_negative_codes_return_minus_one", err11_fallthrough_negative_codes_return_minus_one as fn()),
        ("err12_fallthrough_one_past_range_returns_minus_one", err12_fallthrough_one_past_range_returns_minus_one as fn()),
        ("err13_fallthrough_far_out_of_range_enum_values_return_minus_one", err13_fallthrough_far_out_of_range_enum_values_return_minus_one as fn()),
        ("err14_fallthrough_code_zero_discards_base_value", err14_fallthrough_code_zero_discards_base_value as fn()),
        ("err15_fallthrough_overflow_wraps", err15_fallthrough_overflow_wraps as fn()),
        ("err16_fallthrough_underflow_boundary", err16_fallthrough_underflow_boundary as fn()),
        ("err17_copy_data_block_null_dest_same_signal", err17_copy_data_block_null_dest_same_signal as fn()),
        ("err18_copy_data_block_null_src_same_signal", err18_copy_data_block_null_src_same_signal as fn()),
        ("err19_copy_data_block_both_null_same_signal", err19_copy_data_block_both_null_same_signal as fn()),
        ("err19b_copy_data_block_unmapped_pointer_same_signal", err19b_copy_data_block_unmapped_pointer_same_signal as fn()),
        ("err20_copy_data_block_reads_full_struct_incl_padding", err20_copy_data_block_reads_full_struct_incl_padding as fn()),
        ("err21_22_handle_pointer_operations_overflow", err21_22_handle_pointer_operations_overflow as fn()),
        ("err23_overunder_negative_a_takes_default_arm", err23_overunder_negative_a_takes_default_arm as fn()),
        ("err24_overunder_int_min_a_default_arm_and_square_overflow", err24_overunder_int_min_a_default_arm_and_square_overflow as fn()),
        ("err25_overunder_sqrt_of_negative_yields_zero_conversion", err25_overunder_sqrt_of_negative_yields_zero_conversion as fn()),
        ("err26_overunder_a_product_clamps_to_int_max", err26_overunder_a_product_clamps_to_int_max as fn()),
        ("err27_overunder_b_product_clamps", err27_overunder_b_product_clamps as fn()),
        ("err28_overunder_total_wraps_not_saturates", err28_overunder_total_wraps_not_saturates as fn()),
        ("err29_overunder_hardcoded_clamp_lines_always_present", err29_overunder_hardcoded_clamp_lines_always_present as fn()),
        ("generic_boundary_sweep_all_entry_points", generic_boundary_sweep_all_entry_points as fn())
    ];
    harness::run_suite("phase_c_errors", cases)
}
