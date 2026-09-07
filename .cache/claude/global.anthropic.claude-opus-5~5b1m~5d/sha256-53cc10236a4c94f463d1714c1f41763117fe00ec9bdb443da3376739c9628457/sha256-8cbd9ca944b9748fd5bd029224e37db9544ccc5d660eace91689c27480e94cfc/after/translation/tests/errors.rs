//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input / rejection condition and
//! asserts that the C `.so` and the Rust `.so` produce the *same* sentinel,
//! error value, or (for the `void` function) the same absence of writes.

mod common;

use std::sync::Mutex;

use common::*;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

// ===========================================================================
// find_value_in_buffer — sentinel `-1` and the narrowing of `search_val`
// ===========================================================================

fn find_both(buf: *const i8, size: usize, needle: i32) -> (i32, i32) {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnFindValueInBuffer>(SYM_FIND) };
    unsafe { (c(buf, size, needle), r(buf, size, needle)) }
}

/// ERRORS.md #1 — needle absent => `memchr` returns NULL => `return -1`.
#[test]
fn err01_find_needle_absent_returns_minus_one() {
    let buf = vec![0x11i8; 512];
    for needle in [0, 1, 0x10, 0x12, 0x77, 255, -2, 300, i32::MAX, i32::MIN] {
        let (cv, rv) = find_both(buf.as_ptr(), buf.len(), needle);
        assert_eq!(cv, rv, "err01: needle={needle} => C {cv} vs Rust {rv}");
        assert_eq!(cv, -1, "err01: C should report the -1 sentinel for needle={needle}");
    }
}

/// ERRORS.md #2 — `size == 0`.
#[test]
fn err02_find_size_zero_returns_minus_one() {
    let buf = [7i8, 7, 7, 7];
    for needle in [7, 0, -1, 263, i32::MIN, i32::MAX] {
        let (cv, rv) = find_both(buf.as_ptr(), 0, needle);
        assert_eq!(cv, rv, "err02: needle={needle} => C {cv} vs Rust {rv}");
        assert_eq!(cv, -1, "err02: expected the -1 sentinel");
    }
}

/// ERRORS.md #3 — NULL buffer with `size == 0` (no dereference).
#[test]
fn err03_find_null_buffer_size_zero() {
    for needle in [0, 1, -1, 255, 256, i32::MIN, i32::MAX] {
        let (cv, rv) = find_both(std::ptr::null(), 0, needle);
        assert_eq!(cv, rv, "err03: needle={needle} => C {cv} vs Rust {rv}");
        assert_eq!(cv, -1, "err03: expected the -1 sentinel");
    }
}

/// ERRORS.md #4 — needle above the `unsigned char` range truncates.
#[test]
fn err04_find_needle_above_255_truncates() {
    // Buffer contains byte 44 (== 300 & 0xFF) at index 3 and nothing else that
    // could match, so a *correct* implementation returns 3 and a naive one
    // (comparing the full int) returns -1.
    let mut buf = vec![0x01i8; 16];
    buf[3] = 44;
    let (cv, rv) = find_both(buf.as_ptr(), buf.len(), 300);
    assert_eq!(cv, rv, "err04: C {cv} vs Rust {rv}");
    assert_eq!(cv, 3, "err04: (char)300 == 44 must match index 3");

    // And when byte 44 is absent the sentinel comes back on both sides.
    let plain = vec![0x01i8; 16];
    let (cv, rv) = find_both(plain.as_ptr(), plain.len(), 300);
    assert_eq!((cv, rv), (-1, -1), "err04: absent low byte => -1");

    // Sweep the whole second and third `unsigned char` "pages".
    let all: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    for needle in 256..=767 {
        let (cv, rv) = find_both(all.as_ptr(), 256, needle);
        assert_eq!(cv, rv, "err04: needle={needle} => C {cv} vs Rust {rv}");
    }
}

/// ERRORS.md #5 — negative needle: `(char)-1` re-widens to `0xFF`.
#[test]
fn err05_find_negative_needle() {
    let mut buf = vec![0x01i8; 16];
    buf[9] = -1; // == byte 0xFF
    let (cv, rv) = find_both(buf.as_ptr(), buf.len(), -1);
    assert_eq!(cv, rv, "err05: C {cv} vs Rust {rv}");
    assert_eq!(cv, 9, "err05: needle -1 must find byte 0xFF at index 9");

    let plain = vec![0x01i8; 16];
    let (cv, rv) = find_both(plain.as_ptr(), plain.len(), -1);
    assert_eq!((cv, rv), (-1, -1), "err05: absent 0xFF => -1");

    let all: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    for needle in -512..=-1 {
        let (cv, rv) = find_both(all.as_ptr(), 256, needle);
        assert_eq!(cv, rv, "err05: needle={needle} => C {cv} vs Rust {rv}");
    }
}

/// ERRORS.md #6 — `128`, one past the signed-`char` boundary.
#[test]
fn err06_find_signed_char_boundary_128() {
    let mut buf = vec![0x01i8; 16];
    buf[5] = -128; // == byte 0x80
    for needle in [128, -128, 384, -384] {
        let (cv, rv) = find_both(buf.as_ptr(), buf.len(), needle);
        assert_eq!(cv, rv, "err06: needle={needle} => C {cv} vs Rust {rv}");
        assert_eq!(cv, 5, "err06: needle={needle} must land on byte 0x80");
    }
    for needle in [127, 129] {
        let (cv, rv) = find_both(buf.as_ptr(), buf.len(), needle);
        assert_eq!((cv, rv), (-1, -1), "err06: needle={needle} must not match 0x80");
    }
}

/// ERRORS.md #7 — `INT_MIN` / `INT_MAX` needles (low byte `0x00` / `0xFF`).
#[test]
fn err07_find_int_extreme_needles() {
    // Filler 0x11 collides with none of the extremes' low bytes
    // (INT_MIN -> 0x00, INT_MIN+1 -> 0x01, INT_MAX-1 -> 0xFE, INT_MAX -> 0xFF).
    let mut buf = vec![0x11i8; 16];
    buf[2] = 0; // 0x00 -- low byte of INT_MIN
    buf[11] = -1; // 0xFF -- low byte of INT_MAX
    let (cv, rv) = find_both(buf.as_ptr(), buf.len(), i32::MIN);
    assert_eq!((cv, rv), (2, 2), "err07: INT_MIN => byte 0x00 at index 2 (C {cv}, Rust {rv})");
    let (cv, rv) = find_both(buf.as_ptr(), buf.len(), i32::MAX);
    assert_eq!((cv, rv), (11, 11), "err07: INT_MAX => byte 0xFF at index 11 (C {cv}, Rust {rv})");

    // Same but with the target bytes absent.
    let plain = vec![0x11i8; 16];
    for needle in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        let (cv, rv) = find_both(plain.as_ptr(), plain.len(), needle);
        assert_eq!(cv, rv, "err07: needle={needle} => C {cv} vs Rust {rv}");
        assert_eq!(cv, -1, "err07: expected sentinel for needle={needle}");
    }
}

// ===========================================================================
// create_numeric_buffer — the silent `size <= 0` rejection & wrapping seeds
// ===========================================================================

const CANARY: i8 = 0x5A;

fn create_both(size: i32, seed: i32, cap: usize) -> (Vec<i8>, Vec<i8>) {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnCreateNumericBuffer>(SYM_CREATE) };
    let mut cbuf = vec![CANARY; cap];
    let mut rbuf = vec![CANARY; cap];
    unsafe {
        c(cbuf.as_mut_ptr(), size, seed);
        r(rbuf.as_mut_ptr(), size, seed);
    }
    (cbuf, rbuf)
}

/// ERRORS.md #9 — `size == 0` writes nothing.
#[test]
fn err09_create_size_zero_writes_nothing() {
    for &seed in &[0, 1, -1, i32::MAX, i32::MIN] {
        let (cbuf, rbuf) = create_both(0, seed, 32);
        assert_eq!(cbuf, rbuf, "err09: seed={seed}");
        assert!(cbuf.iter().all(|&b| b == CANARY), "err09: C wrote with size=0");
        assert!(rbuf.iter().all(|&b| b == CANARY), "err09: Rust wrote with size=0");
    }
}

/// ERRORS.md #10 — negative `size` writes nothing.
#[test]
fn err10_create_negative_size_writes_nothing() {
    for &size in &[-1, -2, -7, -256, -1000, i32::MIN, i32::MIN + 1] {
        for &seed in &[0, 1, -1, 42, i32::MAX, i32::MIN] {
            let (cbuf, rbuf) = create_both(size, seed, 32);
            assert_eq!(cbuf, rbuf, "err10: size={size} seed={seed}");
            assert!(
                cbuf.iter().all(|&b| b == CANARY),
                "err10: C wrote with size={size}"
            );
            assert!(
                rbuf.iter().all(|&b| b == CANARY),
                "err10: Rust wrote with size={size}"
            );
        }
    }
}

/// ERRORS.md #11 — NULL buffer with `size <= 0` must not be dereferenced.
#[test]
fn err11_create_null_buffer_non_positive_size() {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnCreateNumericBuffer>(SYM_CREATE) };
    for &size in &[0, -1, -256, i32::MIN] {
        for &seed in &[0, -1, i32::MAX] {
            // Reaching the assertion below means neither call segfaulted.
            unsafe {
                c(std::ptr::null_mut(), size, seed);
                r(std::ptr::null_mut(), size, seed);
            }
        }
    }
}

/// ERRORS.md #12 — negative seed => negative remainders => high bytes.
#[test]
fn err12_create_negative_seed_negative_remainder() {
    for &seed in &[-1, -2, -7, -8, -255, -256, -257, -1000, -65535] {
        let (cbuf, rbuf) = create_both(300, seed, 300);
        assert_eq!(cbuf, rbuf, "err12: seed={seed} bytes differ");
    }
    // Spot-check the documented value: seed = -1, i = 0 => (char)(-1) == -1.
    let (cbuf, rbuf) = create_both(1, -1, 4);
    assert_eq!(cbuf[0], -1, "err12: C byte for seed=-1 should be -1");
    assert_eq!(rbuf[0], -1, "err12: Rust byte for seed=-1 should be -1");
}

/// ERRORS.md #13 — `seed + i*7` signed overflow (UB; gcc wraps).
#[test]
fn err13_create_seed_overflow_wraps() {
    for &seed in &[
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 6,
        i32::MAX - 7,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 7,
    ] {
        let (cbuf, rbuf) = create_both(1024, seed, 1024);
        assert_eq!(cbuf, rbuf, "err13: seed={seed} bytes differ (overflow path)");
    }
}

// ===========================================================================
// calculate_with_doubles — the `b != 0` guard
// ===========================================================================

fn calc_both(a: i32, b: i32, c: i32) -> (f64, f64) {
    let p = Pair::load();
    let (cf, rf) = unsafe { p.both::<FnCalculateWithDoubles>(SYM_CALC) };
    unsafe { (cf(a, b, c), rf(a, b, c)) }
}

/// ERRORS.md #14 — `b == 0` skips the division; result is exactly `0.0`.
#[test]
fn err14_calc_zero_divisor_yields_zero_not_inf() {
    for a in [i32::MIN, -1, 0, 1, i32::MAX, 12345] {
        for c in [i32::MIN, -25, -10, -9, -1, 0, 1, 9, 10, 25, i32::MAX] {
            let (cv, rv) = calc_both(a, 0, c);
            assert!(
                same_f64_bits(cv, rv),
                "err14: calculate_with_doubles({a}, 0, {c}) => C {} vs Rust {}",
                show_f64(cv),
                show_f64(rv)
            );
            assert!(
                cv.is_finite() && cv == 0.0,
                "err14: C must return an exact zero for b == 0, got {}",
                show_f64(cv)
            );
        }
    }
}

/// ERRORS.md #15 — negative `c` gives a negative `c % 10` exponent.
#[test]
fn err15_calc_negative_exponent() {
    for c in -30..=-1 {
        for (a, b) in [(1, 3), (-1, 3), (i32::MAX, 1), (i32::MIN, 1)] {
            let (cv, rv) = calc_both(a, b, c);
            assert!(
                same_f64_bits(cv, rv),
                "err15: calculate_with_doubles({a}, {b}, {c}) => C {} vs Rust {}",
                show_f64(cv),
                show_f64(rv)
            );
        }
    }
}

/// ERRORS.md #16 — `INT_MIN / -1` is performed in `double`, no trap.
#[test]
fn err16_calc_int_min_over_minus_one() {
    for c in [0, 1, -1, 9, -9, 10, i32::MIN, i32::MAX] {
        let (cv, rv) = calc_both(i32::MIN, -1, c);
        assert!(
            same_f64_bits(cv, rv),
            "err16: calculate_with_doubles(INT_MIN, -1, {c}) => C {} vs Rust {}",
            show_f64(cv),
            show_f64(rv)
        );
    }
}

// ===========================================================================
// convert_double_to_int — the out-of-range / NaN UB surface
// ===========================================================================

fn convert_both(v: f64) -> (i32, i32) {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnConvertDoubleToInt>(SYM_CONVERT) };
    unsafe { (c(v), r(v)) }
}

fn assert_convert(v: f64, expect: Option<i32>, row: &str) {
    let (cv, rv) = convert_both(v);
    assert_eq!(
        cv,
        rv,
        "{row}: convert_double_to_int({}) => C {cv} vs Rust {rv}",
        show_f64(v)
    );
    if let Some(e) = expect {
        assert_eq!(
            cv, e,
            "{row}: C returned {cv} for {} but the table says {e}",
            show_f64(v)
        );
    }
}

/// ERRORS.md #17 — NaN.
#[test]
fn err17_convert_nan() {
    for v in [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000),
        f64::from_bits(0xFFF8_0000_0000_0000),
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling
        f64::from_bits(0xFFFF_FFFF_FFFF_FFFF),
    ] {
        assert_convert(v, Some(i32::MIN), "err17");
    }
}

/// ERRORS.md #18/#19 — infinities.
#[test]
fn err18_err19_convert_infinities() {
    assert_convert(f64::INFINITY, Some(i32::MIN), "err18");
    assert_convert(f64::NEG_INFINITY, Some(i32::MIN), "err19");
}

/// ERRORS.md #20 — one step past `INT_MAX`.
#[test]
fn err20_convert_above_int_max() {
    for v in [
        2147483648.0,
        2147483649.0,
        2147483904.0,
        1e300,
        f64::MAX,
        2f64.powi(40),
        2f64.powi(63),
        2f64.powi(64),
    ] {
        assert_convert(v, Some(i32::MIN), "err20");
    }
}

/// ERRORS.md #21 — one step past `INT_MIN`.
#[test]
fn err21_convert_below_int_min() {
    for v in [
        -2147483649.0,
        -2147483650.0,
        -2147483904.0,
        -1e300,
        f64::MIN,
        -(2f64.powi(40)),
        -(2f64.powi(63)),
    ] {
        assert_convert(v, Some(i32::MIN), "err21");
    }
}

/// ERRORS.md #22 — in-range boundaries, including fractional values that
/// truncate back into range.
#[test]
fn err22_convert_in_range_boundaries() {
    assert_convert(2147483647.0, Some(i32::MAX), "err22");
    assert_convert(-2147483648.0, Some(i32::MIN), "err22");
    // 2147483647.5 is exactly representable; truncation gives INT_MAX.
    assert_convert(2147483647.5, Some(i32::MAX), "err22");
    assert_convert(-2147483648.5, Some(i32::MIN), "err22");
    assert_convert(2147483647.75, Some(i32::MAX), "err22");
    assert_convert(-2147483648.75, Some(i32::MIN), "err22");
    // ...but -2147483648.9999995 also truncates to INT_MIN.
    assert_convert(-2147483648.9999995, Some(i32::MIN), "err22");
    // The next representable double above 2147483647.0 that is < 2^31.
    let just_under = f64::from_bits(2147483648.0f64.to_bits() - 1);
    assert_convert(just_under, None, "err22");
    let just_below_min = f64::from_bits((-2147483648.0f64).to_bits() + 1);
    assert_convert(just_below_min, None, "err22");
}

/// ERRORS.md #23 — truncation toward zero, not floor.
#[test]
fn err23_convert_truncates_toward_zero() {
    assert_convert(-0.9, Some(0), "err23");
    assert_convert(-0.5, Some(0), "err23");
    assert_convert(-1.9, Some(-1), "err23");
    assert_convert(0.9, Some(0), "err23");
    assert_convert(1.9, Some(1), "err23");
}

/// ERRORS.md #24 — negative zero.
#[test]
fn err24_convert_negative_zero() {
    assert_convert(-0.0, Some(0), "err24");
    assert_convert(0.0, Some(0), "err24");
}

// ===========================================================================
// process_negation
// ===========================================================================

/// ERRORS.md #25/#26 — `!!` normalisation.
#[test]
fn err25_err26_process_negation_normalises() {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnProcessNegation>(SYM_NEG) };
    for v in [i32::MIN, i32::MIN + 1, -1, 1, i32::MAX - 1, i32::MAX, 0x8000_0000u32 as i32] {
        let (cv, rv) = unsafe { (c(v), r(v)) };
        assert_eq!(cv, rv, "err25: process_negation({v}) => C {cv} vs Rust {rv}");
        assert_eq!(cv, 1, "err25: expected 1 for {v}");
    }
    let (cv, rv) = unsafe { (c(0), r(0)) };
    assert_eq!((cv, rv), (0, 0), "err26: process_negation(0) => C {cv} Rust {rv}");
}

// ===========================================================================
// doubleneg — the two internal rejection branches
// ===========================================================================

fn doubleneg_both(a: i32, b: i32, c: i32, d: i32) -> (i32, Vec<u8>, i32, Vec<u8>) {
    let p = Pair::load();
    let (cf, rf) = unsafe { p.both::<FnDoubleneg>(SYM_DOUBLENEG) };
    let (cv, cout) = capture_stdout(|| unsafe { cf(a, b, c, d) });
    let (rv, rout) = capture_stdout(|| unsafe { rf(a, b, c, d) });
    (cv, cout, rv, rout)
}

fn assert_doubleneg(a: i32, b: i32, c: i32, d: i32, row: &str) -> Vec<u8> {
    let (cv, cout, rv, rout) = doubleneg_both(a, b, c, d);
    assert_eq!(
        cv, rv,
        "{row}: doubleneg({a},{b},{c},{d}) return => C {cv} vs Rust {rv}"
    );
    assert!(cout == rout, "{}", diff_report(row, &cout, &rout));
    cout
}

/// ERRORS.md #27/#28 — the `pos < 0` ("Value %d not found") and
/// `direct_search == NULL` branches.
///
/// The buffer `doubleneg` builds is `(param1 + 7*i) & 0xFF` for `i` in
/// `0..256`; because `gcd(7, 256) == 1` that sequence is a permutation of all
/// 256 byte values for *every* `param1` (the truncating `%` and the signed
/// overflow both preserve the low 8 bits). Both branches are therefore
/// unreachable, and the differential requirement is that **both** libraries
/// agree on that: neither may ever print "not found", and both must always
/// print the "Direct memchr" line.
#[test]
fn err27_err28_doubleneg_search_branches_agree() {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut rng = Rng::new(0x2728);
    let mut cases: Vec<(i32, i32, i32, i32)> = vec![
        (0, 0, 0, 0),
        (-1, -1, -1, -1),
        (1, 128, 200, 255),
        (-256, -128, -200, -255),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
    ];
    for _ in 0..200 {
        cases.push((
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        ));
    }
    for (a, b, c, d) in cases {
        let out = assert_doubleneg(a, b, c, d, "err27_err28");
        let s = String::from_utf8_lossy(&out).to_string();
        assert!(
            !s.contains("not found"),
            "err27: C unexpectedly reached the not-found branch for ({a},{b},{c},{d}); \
             the Rust side must match -- update ERRORS.md"
        );
        assert!(
            s.contains("Direct memchr found byte 100 at offset:"),
            "err28: C unexpectedly skipped the direct-memchr print for ({a},{b},{c},{d})"
        );
    }
}

/// ERRORS.md #29 — `param2 == 0`: the divisor guard plus a constant
/// `search_byte` in the combined-feature loop.
#[test]
fn err29_doubleneg_param2_zero() {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for a in [i32::MIN, -1, 0, 1, 255, 256, i32::MAX] {
        for c in [i32::MIN, -9, 0, 9, i32::MAX] {
            for d in [i32::MIN, 0, 42, i32::MAX] {
                assert_doubleneg(a, 0, c, d, "err29");
            }
        }
    }
}

/// ERRORS.md #30 — `param1 + i*param2` signed overflow.
#[test]
fn err30_doubleneg_search_byte_overflow() {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let extremes = [i32::MIN, i32::MIN + 1, -1, 1, i32::MAX - 1, i32::MAX];
    for &a in &extremes {
        for &b in &extremes {
            assert_doubleneg(a, b, 3, 7, "err30");
        }
    }
    // Values chosen so the running sum crosses INT_MAX partway through.
    for &b in &[i32::MAX / 3, i32::MAX / 5, i32::MAX / 9, 715827883] {
        assert_doubleneg(i32::MAX - 1, b, 1, 1, "err30");
        assert_doubleneg(i32::MIN + 1, -b, 1, 1, "err30");
    }
}

/// ERRORS.md #31 — all-zero parameters.
#[test]
fn err31_doubleneg_all_zero() {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let out = assert_doubleneg(0, 0, 0, 0, "err31");
    let s = String::from_utf8_lossy(&out).to_string();
    assert!(s.contains("After !!negation: 0"), "err31: unexpected C output:\n{s}");
}

// ===========================================================================
// ERRORS.md #32 — generic FFI boundary sweep of every int-typed parameter.
// The public API declares no enums, so the equivalent "invalid variant" input
// is an int one step past each documented range, on every parameter.
// ===========================================================================

#[test]
fn err32_int_parameter_boundary_sweep() {
    const EXTREMES: [i32; 9] = [
        i32::MIN,
        i32::MIN + 1,
        -256,
        -1,
        0,
        1,
        256,
        i32::MAX - 1,
        i32::MAX,
    ];

    let p = Pair::load();
    let (cneg, rneg) = unsafe { p.both::<FnProcessNegation>(SYM_NEG) };
    let (ccre, rcre) = unsafe { p.both::<FnCreateNumericBuffer>(SYM_CREATE) };
    let (cfind, rfind) = unsafe { p.both::<FnFindValueInBuffer>(SYM_FIND) };
    let (ccalc, rcalc) = unsafe { p.both::<FnCalculateWithDoubles>(SYM_CALC) };

    for &x in &EXTREMES {
        let (a, b) = unsafe { (cneg(x), rneg(x)) };
        assert_eq!(a, b, "err32: process_negation({x})");

        // create_numeric_buffer: sweep `size` and `seed` independently.
        for &y in &EXTREMES {
            let cap = 64usize;
            let mut cb = vec![CANARY; cap];
            let mut rb = vec![CANARY; cap];
            // Clamp `size` so we never write past the allocation; the point is
            // the *invalid* (non-positive / extreme) values, which write nothing.
            let size = if x > cap as i32 { cap as i32 } else { x };
            unsafe {
                ccre(cb.as_mut_ptr(), size, y);
                rcre(rb.as_mut_ptr(), size, y);
            }
            assert_eq!(cb, rb, "err32: create_numeric_buffer(size={size}, seed={y})");

            // find_value_in_buffer over that buffer with an extreme needle.
            let n = if size > 0 { size as usize } else { 0 };
            let (fa, fb) = unsafe { (cfind(cb.as_ptr(), n, y), rfind(cb.as_ptr(), n, y)) };
            assert_eq!(fa, fb, "err32: find_value_in_buffer(len={n}, needle={y})");

            // calculate_with_doubles across each parameter position.
            for &z in &EXTREMES {
                let (ca, ra) = unsafe { (ccalc(x, y, z), rcalc(x, y, z)) };
                assert!(
                    same_f64_bits(ca, ra),
                    "err32: calculate_with_doubles({x}, {y}, {z}) => C {} vs Rust {}",
                    show_f64(ca),
                    show_f64(ra)
                );
            }
        }
    }
}

/// ERRORS.md #32 (cont.) — `size_t` extremes for `find_value_in_buffer`.
/// Only the values the C can legally be handed are used (`0`, and lengths that
/// stay inside the allocation); an oversized `size` would be a read past the
/// end in the C too, which is not a rejection the C performs.
#[test]
fn err32b_find_size_boundaries() {
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    for size in [0usize, 1, 2, 255, 256] {
        for needle in [0, 1, 42, 128, 255, -1, 256, i32::MIN, i32::MAX] {
            let (cv, rv) = find_both(buf.as_ptr(), size, needle);
            assert_eq!(cv, rv, "err32b: size={size} needle={needle} => C {cv} vs Rust {rv}");
        }
    }
}
