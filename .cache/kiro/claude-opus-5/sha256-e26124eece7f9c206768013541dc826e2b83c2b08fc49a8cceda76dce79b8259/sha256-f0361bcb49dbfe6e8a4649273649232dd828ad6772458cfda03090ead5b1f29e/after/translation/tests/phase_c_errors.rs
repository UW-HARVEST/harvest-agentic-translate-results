//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Rows 1–32 cover the five low-level exports. Rows 33–40 involve `doubleneg`'s
//! stdout and therefore live in `tests/phase_d_pipeline.rs`.
//!
//! Every row asserts the *same specific* sentinel / error value from both
//! implementations, never merely "both failed".

mod common;

use std::ffi::{c_char, c_int};

use common::{apis, assert_f64_bits_eq, Rng};

/// Assert both implementations return the same value AND that it equals the
/// documented C sentinel.
#[track_caller]
fn expect_e2(buf: *const c_char, size: usize, sv: c_int, sentinel: Option<c_int>, ctx: &str) -> c_int {
    let (c, r) = apis();
    let cv = unsafe { (c.find_value_in_buffer)(buf, size, sv) };
    let rv = unsafe { (r.find_value_in_buffer)(buf, size, sv) };
    assert_eq!(cv, rv, "{ctx}: C = {cv} vs Rust = {rv}");
    if let Some(s) = sentinel {
        assert_eq!(cv, s, "{ctx}: expected C sentinel {s}, got {cv}");
    }
    cv
}

#[track_caller]
fn expect_e1(v: f64, expected: Option<c_int>, ctx: &str) {
    let (c, r) = apis();
    let cv = unsafe { (c.convert_double_to_int)(v) };
    let rv = unsafe { (r.convert_double_to_int)(v) };
    assert_eq!(
        cv, rv,
        "{ctx}: convert_double_to_int({v:?} bits {:#018x}) C = {cv} vs Rust = {rv}",
        v.to_bits()
    );
    if let Some(e) = expected {
        assert_eq!(cv, e, "{ctx}: expected C result {e}, got {cv}");
    }
}

#[track_caller]
fn expect_e3(v: c_int, expected: c_int) {
    let (c, r) = apis();
    let cv = unsafe { (c.process_negation)(v) };
    let rv = unsafe { (r.process_negation)(v) };
    assert_eq!(cv, rv, "process_negation({v}): C = {cv} vs Rust = {rv}");
    assert_eq!(cv, expected, "process_negation({v}): expected {expected}");
}

/// `create_numeric_buffer` must leave the destination untouched.
#[track_caller]
fn expect_e4_untouched(size: c_int, seed: c_int) {
    let (c, r) = apis();
    const S: i8 = 0x3C;
    let mut cb = [S; 64];
    let mut rb = [S; 64];
    unsafe { (c.create_numeric_buffer)(cb.as_mut_ptr(), size, seed) };
    unsafe { (r.create_numeric_buffer)(rb.as_mut_ptr(), size, seed) };
    assert!(
        cb.iter().all(|&b| b == S),
        "C wrote to the buffer for size={size}"
    );
    assert_eq!(cb, rb, "size={size} seed={seed}: buffers diverged");
}

#[track_caller]
fn expect_e4_eq(cap: usize, size: c_int, seed: c_int) -> Vec<i8> {
    let (c, r) = apis();
    let mut cb = vec![0x3Ci8; cap];
    let mut rb = vec![0x3Ci8; cap];
    unsafe { (c.create_numeric_buffer)(cb.as_mut_ptr(), size, seed) };
    unsafe { (r.create_numeric_buffer)(rb.as_mut_ptr(), size, seed) };
    assert_eq!(cb, rb, "size={size} seed={seed}: buffers diverged");
    cb
}

#[track_caller]
fn expect_e5(a: c_int, b: c_int, cc: c_int) -> f64 {
    let (c, r) = apis();
    let cv = unsafe { (c.calculate_with_doubles)(a, b, cc) };
    let rv = unsafe { (r.calculate_with_doubles)(a, b, cc) };
    assert_f64_bits_eq(cv, rv, &format!("calculate_with_doubles({a}, {b}, {cc})"));
    cv
}

fn rng(tag: u64) -> Rng {
    Rng::new(0xE770_5EED_0000_0000 ^ tag)
}

// ===========================================================================
// find_value_in_buffer — rows 1..9
// ===========================================================================

#[test]
fn err_row_01_e2_needle_absent_returns_minus_one() {
    let mut g = rng(1);
    for _ in 0..5000 {
        let missing = g.next_u8();
        let buf: Vec<i8> = (0..256)
            .map(|_| {
                let mut b = g.next_u8();
                if b == missing {
                    b = missing.wrapping_add(1);
                }
                b as i8
            })
            .collect();
        expect_e2(buf.as_ptr(), 256, missing as c_int, Some(-1), "absent needle");
    }
}

#[test]
fn err_row_02_e2_zero_size_returns_minus_one() {
    let buf = [1i8, 2, 3, 4];
    let mut g = rng(2);
    for sv in [0i32, 1, 2, 42, -1, 255, 256, i32::MIN, i32::MAX] {
        expect_e2(buf.as_ptr(), 0, sv, Some(-1), "size == 0");
    }
    for _ in 0..2000 {
        expect_e2(buf.as_ptr(), 0, g.next_i32(), Some(-1), "size == 0 random sv");
    }
}

#[test]
fn err_row_03_e2_null_pointer_zero_size() {
    let mut g = rng(3);
    for sv in [0i32, 1, 42, -1, 255, 256, i32::MIN, i32::MAX] {
        expect_e2(std::ptr::null(), 0, sv, Some(-1), "NULL + size 0");
    }
    for _ in 0..2000 {
        expect_e2(std::ptr::null(), 0, g.next_i32(), Some(-1), "NULL + size 0");
    }
}

#[test]
fn err_row_04_e2_search_val_above_char_range_narrows() {
    let mut g = rng(4);
    for _ in 0..3000 {
        // Buffer containing exactly one 0x41 and no other 0x41.
        let mut buf: Vec<i8> = (0..256)
            .map(|_| {
                let b = g.next_u8();
                if b == 0x41 { 0x42 } else { b }
            })
            .map(|b| b as i8)
            .collect();
        let pos = (g.next_u64() % 256) as usize;
        buf[pos] = 0x41;
        // 0x141 narrows to 0x41 and must MATCH, not be rejected.
        expect_e2(
            buf.as_ptr(),
            256,
            0x141,
            Some(pos as c_int),
            "0x141 narrows to 0x41",
        );
        expect_e2(buf.as_ptr(), 256, 0x41, Some(pos as c_int), "plain 0x41");
        expect_e2(
            buf.as_ptr(),
            256,
            0x1_0041,
            Some(pos as c_int),
            "0x10041 narrows to 0x41",
        );
        // A high-word-only value narrows to 0x00.
        expect_e2(buf.as_ptr(), 256, 0x100, None, "0x100 narrows to 0x00");
    }
}

#[test]
fn err_row_05_e2_negative_search_val() {
    let mut g = rng(5);
    for _ in 0..3000 {
        let mut buf: Vec<i8> = (0..256)
            .map(|_| {
                let b = g.next_u8();
                if b == 0xFF { 0xFE } else { b }
            })
            .map(|b| b as i8)
            .collect();
        let pos = (g.next_u64() % 256) as usize;
        buf[pos] = -1; // 0xFF
        expect_e2(buf.as_ptr(), 256, -1, Some(pos as c_int), "-1 matches 0xFF");
        expect_e2(buf.as_ptr(), 256, -257, Some(pos as c_int), "-257 -> 0xFF");
        expect_e2(buf.as_ptr(), 256, 255, Some(pos as c_int), "255 -> 0xFF");
    }
    // -128 -> 0x80
    for _ in 0..1000 {
        let mut buf: Vec<i8> = (0..256).map(|_| {
            let b = g.next_u8();
            if b == 0x80 { 0x81 } else { b }
        } as i8).collect();
        let pos = (g.next_u64() % 256) as usize;
        buf[pos] = -128;
        expect_e2(buf.as_ptr(), 256, -128, Some(pos as c_int), "-128 -> 0x80");
        expect_e2(buf.as_ptr(), 256, 128, Some(pos as c_int), "128 -> 0x80");
    }
}

#[test]
fn err_row_06_e2_high_bit_bytes_sign_extension() {
    let mut g = rng(6);
    for _ in 0..3000 {
        let buf: Vec<i8> = (0..256).map(|_| g.next_u8() as i8).collect();
        for byte in 0x80u32..=0xFF {
            let signed = (byte as u8 as i8) as c_int; // sign-extended form
            let plain = byte as c_int;
            let a = expect_e2(buf.as_ptr(), 256, plain, None, "plain high byte");
            let b = expect_e2(buf.as_ptr(), 256, signed, None, "sign-extended high byte");
            assert_eq!(a, b, "byte {byte:#04x}: plain vs sign-extended must agree");
        }
        break; // one exhaustive sweep is enough; randomized sweep follows
    }
    for _ in 0..3000 {
        let buf: Vec<i8> = (0..64).map(|_| g.next_u8() as i8).collect();
        let byte = 0x80u8 | (g.next_u8() & 0x7F);
        expect_e2(buf.as_ptr(), 64, byte as c_int, None, "random high byte");
        expect_e2(buf.as_ptr(), 64, (byte as i8) as c_int, None, "random, signed");
    }
}

#[test]
fn err_row_07_e2_zero_needle_without_nul() {
    let mut g = rng(7);
    for _ in 0..3000 {
        let buf: Vec<i8> = (0..256)
            .map(|_| {
                let b = g.next_u8();
                (if b == 0 { 1 } else { b }) as i8
            })
            .collect();
        expect_e2(buf.as_ptr(), 256, 0, Some(-1), "no NUL byte present");
        expect_e2(buf.as_ptr(), 256, 0x100, Some(-1), "0x100 -> NUL, absent");
        expect_e2(buf.as_ptr(), 256, -256, Some(-1), "-256 -> NUL, absent");
    }
}

#[test]
fn err_row_08_e2_match_past_size_is_rejected() {
    let mut g = rng(8);
    for _ in 0..5000 {
        let len = 2 + (g.next_u64() % 400) as usize;
        let cut = 1 + (g.next_u64() % (len - 1) as u64) as usize;
        // 0xAB appears only at index `cut` and later.
        let mut buf: Vec<i8> = (0..len)
            .map(|_| {
                let b = g.next_u8();
                (if b == 0xAB { 0xAC } else { b }) as i8
            })
            .collect();
        buf[cut] = 0xABu8 as i8;
        expect_e2(buf.as_ptr(), cut, 0xAB, Some(-1), "undersized size misses");
        expect_e2(
            buf.as_ptr(),
            cut + 1,
            0xAB,
            Some(cut as c_int),
            "size + 1 hits",
        );
        expect_e2(buf.as_ptr(), 0, 0xAB, Some(-1), "size 0 misses");
    }
}

#[test]
fn err_row_09_e2_extreme_search_val() {
    let mut g = rng(9);
    for _ in 0..3000 {
        let buf: Vec<i8> = (0..256).map(|_| g.next_u8() as i8).collect();
        // INT_MIN low byte is 0x00, INT_MAX low byte is 0xFF.
        let a = expect_e2(buf.as_ptr(), 256, i32::MIN, None, "INT_MIN");
        let b = expect_e2(buf.as_ptr(), 256, 0, None, "0x00");
        assert_eq!(a, b, "INT_MIN must behave as 0x00");
        let c = expect_e2(buf.as_ptr(), 256, i32::MAX, None, "INT_MAX");
        let d = expect_e2(buf.as_ptr(), 256, 0xFF, None, "0xFF");
        assert_eq!(c, d, "INT_MAX must behave as 0xFF");
        expect_e2(buf.as_ptr(), 256, i32::MIN + 1, None, "INT_MIN + 1");
        expect_e2(buf.as_ptr(), 256, i32::MAX - 1, None, "INT_MAX - 1");
    }
}

// ===========================================================================
// create_numeric_buffer — rows 10..15
// ===========================================================================

#[test]
fn err_row_10_e4_zero_size_writes_nothing() {
    for seed in [0i32, 1, -1, 42, i32::MAX, i32::MIN] {
        expect_e4_untouched(0, seed);
    }
    let mut g = rng(10);
    for _ in 0..2000 {
        expect_e4_untouched(0, g.next_i32());
    }
}

#[test]
fn err_row_11_e4_negative_size_writes_nothing() {
    for size in [-1i32, -2, -7, -256, -1000, i32::MIN, i32::MIN + 1] {
        for seed in [0i32, 1, -1, i32::MAX, i32::MIN] {
            expect_e4_untouched(size, seed);
        }
    }
    let mut g = rng(11);
    for _ in 0..2000 {
        let size = -((g.next_u32() >> 1) as i32) - 1;
        expect_e4_untouched(size, g.next_i32());
    }
}

#[test]
fn err_row_12_e4_null_buffer_no_write() {
    let (c, r) = apis();
    for size in [0i32, -1, -256, i32::MIN] {
        for seed in [0i32, 1, -1, i32::MAX, i32::MIN] {
            unsafe { (c.create_numeric_buffer)(std::ptr::null_mut(), size, seed) };
            unsafe { (r.create_numeric_buffer)(std::ptr::null_mut(), size, seed) };
        }
    }
}

#[test]
fn err_row_13_e4_negative_seed_stores_negative_bytes() {
    // C's `%` truncates toward zero, so (seed + 7i) % 256 is negative whenever
    // seed + 7i < 0. The stored `char` must therefore be negative, NOT
    // `seed mod 256`.
    let out = expect_e4_eq(256, 256, -1);
    assert_eq!(out[0], -1, "seed -1 must store -1, not 255");
    let out = expect_e4_eq(256, 256, -300);
    assert_eq!(out[0], (-300 % 256) as i8, "seed -300 -> -44");
    assert_eq!(out[0], -44);
    let mut g = rng(13);
    for _ in 0..3000 {
        let seed = -((g.next_u32() >> 1) as i32) - 1;
        let out = expect_e4_eq(300, 256, seed);
        // Verify the truncating-`%` semantics on the first few elements.
        for i in 0..8i32 {
            let expect = (seed.wrapping_add(i * 7) % 256) as i8;
            assert_eq!(out[i as usize], expect, "seed={seed} i={i}");
        }
    }
}

#[test]
fn err_row_14_e4_seed_int_max_overflow() {
    // seed + i*7 overflows int for i >= 1 when seed == INT_MAX.
    for seed in [i32::MAX, i32::MAX - 1, i32::MAX - 6, i32::MAX - 7] {
        for size in [1i32, 2, 8, 256, 1024] {
            expect_e4_eq(2048, size, seed);
        }
    }
    let mut g = rng(14);
    for _ in 0..1000 {
        let seed = i32::MAX - (g.next_u32() % 64) as i32;
        expect_e4_eq(600, 512, seed);
    }
}

#[test]
fn err_row_15_e4_seed_int_min() {
    for seed in [i32::MIN, i32::MIN + 1, i32::MIN + 7] {
        for size in [1i32, 2, 8, 256, 1024] {
            expect_e4_eq(2048, size, seed);
        }
    }
    let mut g = rng(15);
    for _ in 0..1000 {
        let seed = i32::MIN + (g.next_u32() % 64) as i32;
        expect_e4_eq(600, 512, seed);
    }
}

// ===========================================================================
// calculate_with_doubles — rows 16..20
// ===========================================================================

#[test]
fn err_row_16_e5_zero_divisor_never_divides() {
    let mut g = rng(16);
    for _ in 0..5000 {
        let a = g.next_i32();
        let cc = g.next_i32();
        let v = expect_e5(a, 0, cc);
        assert_eq!(
            v.to_bits(),
            0.0f64.to_bits(),
            "b == 0 must yield +0.0, got {v:?} for a={a} c={cc}"
        );
        assert!(!v.is_nan(), "b == 0 must not produce NaN");
    }
    for cc in [0i32, 9, -9, i32::MIN, i32::MAX] {
        for a in [0i32, 1, -1, i32::MIN, i32::MAX] {
            let v = expect_e5(a, 0, cc);
            assert_eq!(v.to_bits(), 0.0f64.to_bits());
        }
    }
}

#[test]
fn err_row_17_e5_negative_c_gives_negative_exponent() {
    for cc in -9..=-1i32 {
        let v = expect_e5(1, 1, cc);
        let expect = 10f64.powi(cc);
        assert_f64_bits_eq(expect, v, &format!("1/1 * 10^{cc}"));
        assert!(v < 1.0, "c={cc} must give a negative exponent");
    }
    let mut g = rng(17);
    for _ in 0..5000 {
        let cc = -1 - ((g.next_u32() >> 1) as i32);
        let mut b = g.next_i32();
        if b == 0 {
            b = 1;
        }
        expect_e5(g.next_i32(), b, cc);
    }
}

#[test]
fn err_row_18_e5_c_int_min() {
    assert_eq!(i32::MIN % 10, -8, "C: INT_MIN % 10 == -8");
    let v = expect_e5(1, 1, i32::MIN);
    assert_f64_bits_eq(1e-8, v, "1/1 * 10^-8");
    let mut g = rng(18);
    for _ in 0..3000 {
        let mut b = g.next_i32();
        if b == 0 {
            b = 1;
        }
        expect_e5(g.next_i32(), b, i32::MIN);
        expect_e5(g.next_i32(), b, i32::MIN + 1);
        expect_e5(g.next_i32(), b, i32::MAX);
    }
}

#[test]
fn err_row_19_e5_int_min_over_minus_one_no_trap() {
    for cc in -12..=12i32 {
        let v = expect_e5(i32::MIN, -1, cc);
        let expect = 2147483648.0f64 * 10f64.powi(cc % 10);
        assert_f64_bits_eq(expect, v, &format!("INT_MIN / -1 * 10^({cc}%10)"));
    }
    expect_e5(i32::MIN, -1, i32::MIN);
    expect_e5(i32::MIN, -1, i32::MAX);
}

#[test]
fn err_row_20_e5_zero_over_zero_stays_zero() {
    for cc in [9i32, -9, 0, i32::MIN, i32::MAX] {
        let v = expect_e5(0, 0, cc);
        assert_eq!(v.to_bits(), 0.0f64.to_bits(), "0/0-guarded must be +0.0");
    }
    let mut g = rng(20);
    for _ in 0..3000 {
        let v = expect_e5(0, 0, g.next_i32());
        assert_eq!(v.to_bits(), 0.0f64.to_bits());
    }
}

// ===========================================================================
// convert_double_to_int — rows 21..30
// ===========================================================================

#[test]
fn err_row_21_e1_above_int_max() {
    for v in [
        2147483648.0f64,
        2147483649.0,
        2147483648.5,
        3e9,
        1e18,
        1e300,
        f64::MAX,
    ] {
        expect_e1(v, Some(i32::MIN), "value > INT_MAX");
    }
    let mut g = rng(21);
    for _ in 0..5000 {
        let v = 2147483648.0f64 + (g.next_u32() as f64);
        expect_e1(v, Some(i32::MIN), "random > INT_MAX");
    }
}

#[test]
fn err_row_22_e1_below_int_min() {
    for v in [
        -2147483649.0f64,
        -2147483650.0,
        -3e9,
        -1e18,
        -1e300,
        f64::MIN,
    ] {
        expect_e1(v, Some(i32::MIN), "value < INT_MIN");
    }
    let mut g = rng(22);
    for _ in 0..5000 {
        let v = -2147483649.0f64 - (g.next_u32() as f64);
        expect_e1(v, Some(i32::MIN), "random < INT_MIN");
    }
}

#[test]
fn err_row_23_e1_positive_infinity() {
    expect_e1(f64::INFINITY, Some(i32::MIN), "+inf");
    expect_e1(1.0 / 0.0, Some(i32::MIN), "1.0/0.0");
}

#[test]
fn err_row_24_e1_negative_infinity() {
    expect_e1(f64::NEG_INFINITY, Some(i32::MIN), "-inf");
    expect_e1(-1.0 / 0.0, Some(i32::MIN), "-1.0/0.0");
}

#[test]
fn err_row_25_e1_nan() {
    expect_e1(f64::NAN, Some(i32::MIN), "NAN");
    expect_e1(0.0 / 0.0, Some(i32::MIN), "0.0/0.0");
}

#[test]
fn err_row_26_e1_nan_payload_variants() {
    for bits in [
        0x7FF8_0000_0000_0000u64,
        0xFFF8_0000_0000_0000,
        0x7FF0_0000_0000_0001,
        0xFFF0_0000_0000_0001,
        0x7FF4_2424_2424_2424,
        0xFFFF_FFFF_FFFF_FFFF,
        0x7FFF_FFFF_FFFF_FFFF,
    ] {
        expect_e1(f64::from_bits(bits), Some(i32::MIN), "NaN payload variant");
    }
    let mut g = rng(26);
    for _ in 0..5000 {
        let sign = (g.next_u64() & 1) << 63;
        let payload = g.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        let bits = sign | 0x7FF0_0000_0000_0000 | payload.max(1);
        expect_e1(f64::from_bits(bits), Some(i32::MIN), "random NaN");
    }
}

#[test]
fn err_row_27_e1_int_max_plus_half_is_in_range() {
    expect_e1(2147483647.5, Some(i32::MAX), "INT_MAX + 0.5 truncates in range");
    expect_e1(2147483647.999, Some(i32::MAX), "just under INT_MAX + 1");
    expect_e1(
        2147483647.0f64.next_up(),
        Some(i32::MAX),
        "nextafter(INT_MAX, +inf)",
    );
}

#[test]
fn err_row_28_e1_int_min_minus_half_is_in_range() {
    expect_e1(
        -2147483648.5,
        Some(i32::MIN),
        "INT_MIN - 0.5 truncates in range",
    );
    expect_e1(-2147483648.999, Some(i32::MIN), "just above INT_MIN - 1");
    expect_e1(
        (-2147483648.0f64).next_down(),
        Some(i32::MIN),
        "nextafter(INT_MIN, -inf)",
    );
}

#[test]
fn err_row_29_e1_one_step_past_range() {
    // 2147483648.0 is exactly out of range; its predecessor is not.
    expect_e1(2147483648.0, Some(i32::MIN), "exactly 2^31");
    expect_e1(2147483648.0f64.next_down(), Some(i32::MAX), "just below 2^31");
    // -2147483649.0 is out of range; -2147483648.0 is not.
    expect_e1(-2147483649.0, Some(i32::MIN), "exactly -(2^31 + 1)");
    expect_e1(-2147483648.0, Some(i32::MIN), "exactly -2^31 (in range)");
    expect_e1((-2147483649.0f64).next_up(), Some(i32::MIN), "just above -(2^31+1)");
    for base in [2147483647.0f64, 2147483648.0, -2147483648.0, -2147483649.0] {
        let mut v = base;
        for _ in 0..8 {
            v = v.next_up();
            expect_e1(v, None, "next_up sweep");
        }
        let mut v = base;
        for _ in 0..8 {
            v = v.next_down();
            expect_e1(v, None, "next_down sweep");
        }
    }
}

#[test]
fn err_row_30_e1_negative_zero_and_subnormals() {
    expect_e1(-0.0, Some(0), "-0.0");
    expect_e1(0.0, Some(0), "+0.0");
    expect_e1(5e-324, Some(0), "smallest subnormal");
    expect_e1(-5e-324, Some(0), "smallest negative subnormal");
    expect_e1(f64::MIN_POSITIVE, Some(0), "MIN_POSITIVE");
    expect_e1(-f64::MIN_POSITIVE, Some(0), "-MIN_POSITIVE");
    expect_e1(0.999_999_999_999, Some(0), "just under 1");
    expect_e1(-0.999_999_999_999, Some(0), "just over -1");
}

// ===========================================================================
// process_negation — rows 31..32
// ===========================================================================

#[test]
fn err_row_31_e3_extremes_are_one() {
    expect_e3(i32::MIN, 1);
    expect_e3(i32::MIN + 1, 1);
    expect_e3(i32::MAX, 1);
    expect_e3(i32::MAX - 1, 1);
    expect_e3(-1, 1);
    expect_e3(1, 1);
    let mut g = rng(31);
    for _ in 0..20000 {
        let v = g.next_i32();
        expect_e3(v, if v != 0 { 1 } else { 0 });
    }
}

#[test]
fn err_row_32_e3_only_zero_maps_to_zero() {
    expect_e3(0, 0);
    for v in -1000..=1000i32 {
        expect_e3(v, if v == 0 { 0 } else { 1 });
    }
}

// ===========================================================================
// Generic FFI boundary sweep (documented in ERRORS.md)
// ===========================================================================

#[test]
fn err_generic_out_of_range_int_values_across_ffi() {
    // C `int` parameters accept ANY 32-bit value; there is no valid-variant set
    // to violate, so every export must behave identically for arbitrary ints.
    let mut g = rng(999);
    let buf: Vec<i8> = (0..256).map(|_| g.next_u8() as i8).collect();
    let sentinels = [
        i32::MIN,
        i32::MIN + 1,
        -65537,
        -65536,
        -257,
        -256,
        -255,
        -1,
        0,
        1,
        255,
        256,
        257,
        65535,
        65536,
        65537,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &v in &sentinels {
        expect_e2(buf.as_ptr(), 256, v, None, "sentinel search_val");
        expect_e3(v, if v != 0 { 1 } else { 0 });
        expect_e4_eq(600, 512, v);
        for &w in &sentinels {
            expect_e5(v, w, 3);
            expect_e5(3, v, w);
        }
        expect_e1(v as f64, Some(v), "int-valued double round trip");
    }
    for _ in 0..5000 {
        let v = g.next_i32();
        expect_e2(buf.as_ptr(), g.range_usize(0, 256), v, None, "random sv/size");
        expect_e3(v, if v != 0 { 1 } else { 0 });
    }
}

#[test]
fn err_generic_oversized_and_zero_lengths() {
    let mut g = rng(998);
    let buf: Vec<i8> = (0..1024).map(|_| g.next_u8() as i8).collect();
    for size in [0usize, 1, 2, 255, 256, 257, 512, 1023, 1024] {
        for sv in [0i32, 1, 42, 100, 255, -1, 256, i32::MIN, i32::MAX] {
            expect_e2(buf.as_ptr(), size, sv, None, "size sweep");
        }
    }
    // create_numeric_buffer with a size larger than one `% 256` cycle
    for size in [0i32, 1, 255, 256, 257, 512, 1024, 4096] {
        expect_e4_eq(4096, size, g.next_i32());
    }
}
