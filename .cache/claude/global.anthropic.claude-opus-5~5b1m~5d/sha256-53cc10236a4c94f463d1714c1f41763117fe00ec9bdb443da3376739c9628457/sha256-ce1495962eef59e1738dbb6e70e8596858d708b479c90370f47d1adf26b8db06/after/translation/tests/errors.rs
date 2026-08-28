//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input / rejection condition, calls
//! BOTH libraries through their `.so` exports, and asserts they produce the
//! SAME sentinel or error result. Where the C's answer is a fixed, known value
//! (`-1`, `INT_MIN`, `0.0`, "buffer untouched") the test also pins that absolute
//! value, so a harness that silently compared nothing would still fail.

mod common;

use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

use common::assert_f64_bits_eq;
use common::both;
use common::capture_stdout;
use common::Guarded;
use common::Rng;

const SEED: u64 = 0xE770_0000_1234_5678;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn find(content: &[i8], size: usize, needle: c_int) -> (c_int, c_int) {
    let (cl, rl) = both();
    let mut buf = Guarded::new(content.len());
    buf.set_body(content);
    let a = unsafe { (cl.find_value_in_buffer)(buf.const_ptr(), size, needle) };
    let b = unsafe { (rl.find_value_in_buffer)(buf.const_ptr(), size, needle) };
    buf.check_canaries(format!("find(len={}, size={size}, needle={needle})", content.len()));
    assert_eq!(
        a, b,
        "find_value_in_buffer(len={}, size={size}, needle={needle}): C {a} vs Rust {b}",
        content.len()
    );
    (a, b)
}

fn calc(a: c_int, b: c_int, c: c_int) -> f64 {
    let (cl, rl) = both();
    let x = unsafe { (cl.calculate_with_doubles)(a, b, c) };
    let y = unsafe { (rl.calculate_with_doubles)(a, b, c) };
    assert_f64_bits_eq(x, y, format!("calculate_with_doubles({a},{b},{c})"));
    x
}

fn cvt(v: f64) -> c_int {
    let (cl, rl) = both();
    let a = unsafe { (cl.convert_double_to_int)(v) };
    let b = unsafe { (rl.convert_double_to_int)(v) };
    assert_eq!(
        a, b,
        "convert_double_to_int({v:?} bits {:#018x}): C {a} vs Rust {b}",
        v.to_bits()
    );
    a
}

/// Runs `create_numeric_buffer` on both libraries over an all-canary region and
/// asserts nothing at all was written.
fn assert_create_writes_nothing(size: c_int, seed: c_int, scratch_len: usize) {
    let (cl, rl) = both();
    let mut cb = Guarded::new(scratch_len);
    let mut rb = Guarded::new(scratch_len);
    let before: Vec<i8> = cb.body().to_vec();

    unsafe {
        (cl.create_numeric_buffer)(cb.ptr(), size, seed);
        (rl.create_numeric_buffer)(rb.ptr(), size, seed);
    }

    let ctx = format!("create_numeric_buffer(size={size}, seed={seed})");
    cb.check_canaries(format!("{ctx} [C]"));
    rb.check_canaries(format!("{ctx} [Rust]"));
    assert_eq!(cb.body(), &before[..], "{ctx}: C wrote to the buffer");
    assert_eq!(rb.body(), &before[..], "{ctx}: Rust wrote to the buffer");
    assert_eq!(cb.body(), rb.body(), "{ctx}: buffers diverged");
}

/// The bytes both libraries produce for `create_numeric_buffer(size, seed)`,
/// asserted equal, returned for further inspection.
fn create_bytes(size: c_int, seed: c_int) -> Vec<i8> {
    let (cl, rl) = both();
    let len = size.max(0) as usize;
    let mut cb = Guarded::new(len);
    let mut rb = Guarded::new(len);
    unsafe {
        (cl.create_numeric_buffer)(cb.ptr(), size, seed);
        (rl.create_numeric_buffer)(rb.ptr(), size, seed);
    }
    let ctx = format!("create_numeric_buffer(size={size}, seed={seed})");
    cb.check_canaries(format!("{ctx} [C]"));
    rb.check_canaries(format!("{ctx} [Rust]"));
    assert_eq!(cb.body(), rb.body(), "{ctx}: bytes diverged");
    cb.body().to_vec()
}

// ===========================================================================
// find_value_in_buffer
// ===========================================================================

/// ERRORS row 1 — needle absent ⇒ `memchr` NULL ⇒ `-1`.
fn err_01_find_absent_needle() {
    // A buffer that deliberately contains only 0x01.
    for len in [1usize, 2, 7, 255, 256, 257, 1024] {
        let content = vec![0x01i8; len];
        for needle in [0, 2, 42, 100, 127, 128, 255, -1, 256, i32::MIN, i32::MAX] {
            let (a, _) = find(&content, len, needle);
            assert_eq!(a, -1, "expected -1 for absent needle {needle} (len {len})");
        }
    }

    // Randomised: build a buffer that provably excludes one chosen byte.
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..2_000 {
        let len = 1 + rng.below(512) as usize;
        let excluded = rng.next_u8();
        let mut content = vec![0i8; len];
        for slot in content.iter_mut() {
            let mut b = rng.next_u8();
            if b == excluded {
                b = excluded.wrapping_add(1);
            }
            *slot = b as i8;
        }
        let (a, _) = find(&content, len, excluded as c_int);
        assert_eq!(a, -1, "expected -1 for excluded byte {excluded}");
        // The same needle offset by a multiple of 256 must behave identically.
        let (a2, _) = find(&content, len, excluded as c_int + 256 * 3);
        assert_eq!(a2, -1);
        let (a3, _) = find(&content, len, excluded as c_int - 256 * 5);
        assert_eq!(a3, -1);
    }
}

/// ERRORS row 2 — `size == 0` ⇒ `-1`, even when the byte sits at index 0.
fn err_02_find_zero_size() {
    for len in [1usize, 2, 256] {
        let content = vec![0x42i8; len];
        let (a, _) = find(&content, 0, 0x42);
        assert_eq!(a, -1, "size==0 must reject even though byte 0x42 is present");
    }
    // Zero size with every interesting needle.
    let all: Vec<i8> = (0..256).map(|b| b as u8 as i8).collect();
    for needle in [0, 1, 42, 100, 255, -1, 256, i32::MIN, i32::MAX] {
        let (a, _) = find(&all, 0, needle);
        assert_eq!(a, -1);
    }
}

/// ERRORS row 3 — `buffer == NULL` with `size == 0` must not fault and must
/// return `-1` from both libraries.
fn err_03_find_null_zero_size() {
    let (cl, rl) = both();
    for needle in [0, 1, 42, 100, 255, -1, 256, i32::MIN, i32::MAX] {
        let a = unsafe { (cl.find_value_in_buffer)(ptr::null::<c_char>(), 0, needle) };
        let b = unsafe { (rl.find_value_in_buffer)(ptr::null::<c_char>(), 0, needle) };
        assert_eq!(a, -1, "C: find(NULL, 0, {needle})");
        assert_eq!(b, -1, "Rust: find(NULL, 0, {needle})");
    }
}

/// ERRORS row 4 — `search_val` outside `0..=255` is narrowed to its low byte and
/// is *never* rejected on that ground.
fn err_04_find_out_of_range_needle() {
    let all: Vec<i8> = (0..256).map(|b| b as u8 as i8).collect();

    // Every out-of-range needle must find the byte its low 8 bits denote.
    for base in 0..256i32 {
        for k in [-5i64, -3, -1, 0, 1, 2, 7] {
            let needle = (base as i64 + k * 256) as i32;
            let (a, _) = find(&all, 256, needle);
            assert_eq!(
                a, base,
                "needle {needle} should map to low byte {base} (found {a})"
            );
        }
    }

    // The documented examples: 300 -> byte 44, -1 -> byte 255.
    assert_eq!(find(&all, 256, 300).0, 44);
    assert_eq!(find(&all, 256, -1).0, 255);
    // INT_MIN's low byte is 0, INT_MAX's low byte is 255.
    assert_eq!(find(&all, 256, i32::MIN).0, 0);
    assert_eq!(find(&all, 256, i32::MAX).0, 255);

    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..5_000 {
        let needle = rng.next_i32();
        let (a, _) = find(&all, 256, needle);
        assert_eq!(a, (needle as u8) as c_int, "needle {needle}");
    }
}

/// ERRORS row 5 — needle at exactly `size-1`.
fn err_05_find_last_byte_boundary() {
    for len in [1usize, 2, 7, 255, 256, 257, 1024] {
        let mut content = vec![0x01i8; len];
        content[len - 1] = 0x42;
        let (a, _) = find(&content, len, 0x42);
        assert_eq!(a, (len - 1) as c_int, "len {len}");
    }
}

/// ERRORS row 6 — `size` stops exactly *at* the match, which must not be seen.
fn err_06_find_one_past_match() {
    for len in [1usize, 2, 7, 255, 256, 1024] {
        for pos in [0usize, len / 2, len - 1] {
            let mut content = vec![0x01i8; len];
            content[pos] = 0x42;
            // Range [0, pos) excludes the match.
            let (excl, _) = find(&content, pos, 0x42);
            assert_eq!(excl, -1, "len {len}, pos {pos}: match must be out of range");
            // Range [0, pos+1) includes it.
            let (incl, _) = find(&content, pos + 1, 0x42);
            assert_eq!(incl, pos as c_int, "len {len}, pos {pos}");
        }
    }
}

// ===========================================================================
// create_numeric_buffer
// ===========================================================================

/// ERRORS row 7 — `size == 0` writes nothing.
fn err_07_create_zero_size() {
    for seed in [0, 1, -1, 42, i32::MIN, i32::MAX] {
        assert_create_writes_nothing(0, seed, 64);
    }
}

/// ERRORS row 8 — `size < 0` writes nothing (loop guard), no fault.
fn err_08_create_negative_size() {
    for size in [-1, -2, -7, -255, -256, -1024, i32::MIN, i32::MIN + 1] {
        for seed in [0, 1, -1, 42, i32::MIN, i32::MAX] {
            assert_create_writes_nothing(size, seed, 64);
        }
    }
}

/// ERRORS row 9 — `buffer == NULL` with non-positive `size` must not fault.
fn err_09_create_null_nonpositive_size() {
    let (cl, rl) = both();
    for size in [0, -1, -256, i32::MIN] {
        for seed in [0, 1, -1, i32::MAX, i32::MIN] {
            unsafe {
                (cl.create_numeric_buffer)(ptr::null_mut::<c_char>(), size, seed);
                (rl.create_numeric_buffer)(ptr::null_mut::<c_char>(), size, seed);
            }
        }
    }
}

/// ERRORS row 10 — negative `seed` ⇒ C's `%` yields a negative remainder which
/// `(char)` then reinterprets. Pin the absolute expected bytes.
fn err_10_create_negative_seed() {
    // seed = -1: bytes are (char)((-1 + 7i) % 256).
    let bytes = create_bytes(8, -1);
    let expect: Vec<i8> = (0..8i32).map(|i| ((-1 + i * 7) % 256) as i8).collect();
    assert_eq!(bytes, expect, "seed -1");
    assert_eq!(bytes[0], -1, "seed -1, i=0 must be -1 (0xFF), not +255");

    // A seed whose remainder stays negative for a while: -300.
    let bytes = create_bytes(64, -300);
    let expect: Vec<i8> = (0..64i32).map(|i| ((-300 + i * 7) % 256) as i8).collect();
    assert_eq!(bytes, expect, "seed -300");

    for seed in [-1, -2, -7, -8, -127, -128, -255, -256, -257, -1000, -65_536] {
        for size in [1, 7, 8, 36, 256, 300] {
            let bytes = create_bytes(size, seed);
            let expect: Vec<i8> = (0..size)
                .map(|i| ((seed.wrapping_add(i.wrapping_mul(7))) % 256) as i8)
                .collect();
            assert_eq!(bytes, expect, "seed {seed}, size {size}");
        }
    }
}

/// ERRORS row 11 — `seed + i*7` overflows `int` mid-loop (UB in C; wraps at -O0).
fn err_11_create_seed_overflow() {
    for seed in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 3,
        i32::MAX - 6,
        i32::MAX - 7,
        i32::MAX - 699,
        i32::MAX - 700,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 6,
    ] {
        for size in [1, 2, 7, 8, 256, 300, 2048] {
            let bytes = create_bytes(size, seed);
            let expect: Vec<i8> = (0..size)
                .map(|i| ((seed.wrapping_add(i.wrapping_mul(7))) % 256) as i8)
                .collect();
            assert_eq!(bytes, expect, "seed {seed}, size {size}");
        }
    }
}

// ===========================================================================
// calculate_with_doubles
// ===========================================================================

/// ERRORS row 12 — `b == 0` skips the division, so the result is `+0.0`, never
/// `inf` or `nan`.
fn err_12_calc_zero_divisor() {
    for a in [0, 1, -1, 42, -42, i32::MIN, i32::MAX] {
        for c in -30..=30 {
            let r = calc(a, 0, c);
            assert_eq!(
                r.to_bits(),
                0.0_f64.to_bits(),
                "calculate_with_doubles({a},0,{c}) must be exactly +0.0, got {r:?}"
            );
            assert!(!r.is_nan() && r.is_finite());
        }
    }
    for c in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        let r = calc(7, 0, c);
        assert_eq!(r.to_bits(), 0.0_f64.to_bits());
    }
}

/// ERRORS row 13 — negative `c` ⇒ negative exponent.
fn err_13_calc_negative_exponent() {
    // 1/1 * 10^-9
    let r = calc(1, 1, -9);
    assert!(
        (r - 1e-9).abs() <= 1e-24,
        "calculate_with_doubles(1,1,-9) = {r:?}, expected ~1e-9"
    );
    for c in -9..=-1 {
        for (a, b) in [(1, 1), (-1, 1), (1, -1), (7, 3), (-7, -3)] {
            let r = calc(a, b, c);
            assert!(r.is_finite(), "({a},{b},{c}) -> {r:?}");
        }
    }
    // c = -10 wraps back to exponent 0.
    let r = calc(3, 2, -10);
    assert_eq!(r, 1.5, "c % 10 == 0 for c = -10");
}

/// ERRORS row 14 — `c == INT_MIN`: `INT_MIN % 10 == -8` and must not trap.
fn err_14_calc_c_int_min() {
    assert_eq!(i32::MIN % 10, -8, "sanity: C-style truncating remainder");
    let r = calc(1, 1, i32::MIN);
    assert!(
        (r - 1e-8).abs() <= 1e-23,
        "calculate_with_doubles(1,1,INT_MIN) = {r:?}, expected ~1e-8"
    );
    for a in [0, 1, -1, i32::MIN, i32::MAX] {
        for b in [0, 1, -1, i32::MIN, i32::MAX] {
            calc(a, b, i32::MIN);
            calc(a, b, i32::MIN + 1);
            calc(a, b, i32::MAX);
        }
    }
}

/// ERRORS row 15 — `INT_MIN / -1` would overflow in integer arithmetic; the C
/// converts to `double` first, so it must yield `2147483648.0 * 10^(c%10)`.
fn err_15_calc_int_min_over_minus_one() {
    let r = calc(i32::MIN, -1, 0);
    assert_eq!(r, 2_147_483_648.0_f64, "INT_MIN / -1 as doubles");
    for c in -9..=9 {
        let r = calc(i32::MIN, -1, c);
        assert!(r.is_finite() && r > 0.0, "c={c} -> {r:?}");
    }
    // The mirror case.
    let r = calc(i32::MIN, 1, 0);
    assert_eq!(r, -2_147_483_648.0_f64);
}

// ===========================================================================
// convert_double_to_int  (cvttsd2si "integer indefinite" behaviour)
// ===========================================================================

/// ERRORS row 16 — above `INT_MAX` ⇒ `INT_MIN`.
fn err_16_cvt_above_int_max() {
    for v in [
        2_147_483_648.0_f64,
        2_147_483_649.0,
        2_147_483_648.5,
        4e9,
        1e18,
        1e300,
        f64::MAX,
        (1u64 << 40) as f64,
    ] {
        assert_eq!(cvt(v), i32::MIN, "convert_double_to_int({v:?})");
    }
}

/// ERRORS row 17 — below `INT_MIN` ⇒ `INT_MIN`.
fn err_17_cvt_below_int_min() {
    for v in [
        -2_147_483_649.0_f64,
        -2_147_483_650.0,
        -4e9,
        -1.0 * (1u64 << 40) as f64,
        -1e18,
        -1e300,
        f64::MIN,
    ] {
        assert_eq!(cvt(v), i32::MIN, "convert_double_to_int({v:?})");
    }
}

/// ERRORS row 18 — `+INFINITY`.
fn err_18_cvt_pos_infinity() {
    assert_eq!(cvt(f64::INFINITY), i32::MIN);
    assert_eq!(cvt(1.0 / 0.0), i32::MIN);
}

/// ERRORS row 19 — `-INFINITY`.
fn err_19_cvt_neg_infinity() {
    assert_eq!(cvt(f64::NEG_INFINITY), i32::MIN);
    assert_eq!(cvt(-1.0 / 0.0), i32::MIN);
}

/// ERRORS row 20 — NaN, across quiet / signalling / arbitrary payloads.
fn err_20_cvt_nan_payloads() {
    let mut nans = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // canonical qNaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative qNaN
        f64::from_bits(0x7FF0_0000_0000_0001), // sNaN, smallest payload
        f64::from_bits(0xFFF0_0000_0000_0001), // negative sNaN
        f64::from_bits(0x7FF7_FFFF_FFFF_FFFF), // sNaN, largest payload
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // qNaN, all payload bits
        0.0_f64 / 0.0_f64,
    ];
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..2_000 {
        // Random NaN: exponent all ones, non-zero mantissa.
        let mantissa = (rng.next_u64() & 0x000F_FFFF_FFFF_FFFF) | 1;
        let sign = (rng.next_u64() & 1) << 63;
        nans.push(f64::from_bits(sign | 0x7FF0_0000_0000_0000 | mantissa));
    }
    for v in nans {
        assert!(v.is_nan(), "test bug: {v:?} is not NaN");
        assert_eq!(
            cvt(v),
            i32::MIN,
            "NaN with bits {:#018x} must convert to INT_MIN",
            v.to_bits()
        );
    }
}

/// ERRORS row 21 — one step past the valid range in both directions.
fn err_21_cvt_boundaries() {
    // In range, exact.
    assert_eq!(cvt(2_147_483_647.0), 2_147_483_647);
    assert_eq!(cvt(2_147_483_647.5), 2_147_483_647);
    // NOTE: the decimal literal `2147483647.9999999` is NOT representable and
    // rounds *up* to 2147483648.0, which is out of range. The genuine "largest
    // in-range double" is the next representable value below 2^31.
    let largest_in_range = f64::from_bits(2_147_483_648.0_f64.to_bits() - 1);
    assert!(largest_in_range < 2_147_483_648.0 && largest_in_range > 2_147_483_647.0);
    assert_eq!(cvt(largest_in_range), 2_147_483_647);
    assert_eq!(cvt(-2_147_483_648.0), i32::MIN);
    assert_eq!(cvt(-2_147_483_648.5), i32::MIN);
    assert_eq!(cvt(-2_147_483_648.999_999_9), i32::MIN);
    // One step past.
    assert_eq!(cvt(2_147_483_648.0), i32::MIN);
    assert_eq!(cvt(-2_147_483_649.0), i32::MIN);

    // Adjacent representable doubles around both bounds.
    for anchor in [2_147_483_647.0_f64, -2_147_483_648.0_f64, 2_147_483_648.0_f64] {
        let bits = anchor.to_bits();
        for d in -4i64..=4 {
            let v = f64::from_bits((bits as i64 + d) as u64);
            cvt(v);
        }
    }
}

/// ERRORS row 22 — `-0.0` and negative subnormals must convert to `0`, not `-1`.
fn err_22_cvt_negative_zero_and_subnormal() {
    assert_eq!(cvt(-0.0), 0);
    assert_eq!(cvt(0.0), 0);
    assert_eq!(cvt(-f64::MIN_POSITIVE), 0);
    assert_eq!(cvt(-5e-324), 0);
    assert_eq!(cvt(-0.999_999_999), 0);
    assert_eq!(cvt(0.999_999_999), 0);
    assert_eq!(cvt(f64::from_bits(0x8000_0000_0000_0001)), 0);
}

// ===========================================================================
// doubleneg
// ===========================================================================

fn doubleneg_output(p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> (c_int, String, c_int, String) {
    let (cl, rl) = both();
    let (c_ret, c_out) = capture_stdout(|| unsafe { (cl.doubleneg)(p1, p2, p3, p4) });
    let (r_ret, r_out) = capture_stdout(|| unsafe { (rl.doubleneg)(p1, p2, p3, p4) });
    let c_s = String::from_utf8_lossy(&c_out).into_owned();
    let r_s = String::from_utf8_lossy(&r_out).into_owned();
    assert!(c_s.len() > 500 && r_s.len() > 500, "capture produced no output");
    assert_eq!(c_s, r_s, "doubleneg({p1},{p2},{p3},{p4}) stdout differs");
    assert_eq!(c_ret, r_ret, "doubleneg({p1},{p2},{p3},{p4}) return differs");
    (c_ret, c_s, r_ret, r_s)
}

/// ERRORS row 23 — the `"Value %d not found"` branch is provably unreachable
/// because the generated buffer is a permutation of all 256 byte values. Both
/// libraries must agree on that (a Rust generator bug would break it).
fn err_23_doubleneg_value_not_found_is_unreachable() {
    // First prove the premise at the low-level export: every byte value 0..=255
    // is present in `create_numeric_buffer(_, 256, seed)` for every seed.
    for seed in [
        0,
        1,
        -1,
        100,
        -100,
        255,
        -255,
        256,
        -256,
        i32::MAX,
        i32::MIN,
        -8,
        264,
        12_345,
        -99_999,
    ] {
        let bytes = create_bytes(256, seed);
        let mut seen = [false; 256];
        for &b in &bytes {
            seen[b as u8 as usize] = true;
        }
        assert!(
            seen.iter().all(|&s| s),
            "seed {seed}: generated buffer is not a permutation of all 256 bytes"
        );
    }

    // Now the branch itself, via `doubleneg`, over a wide parameter sweep.
    for (p1, p2, p3, p4) in [
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -1, -1, -1),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (255, -300, 300, -255),
        (12_345, -99_999, 7, -7),
    ] {
        let (_, out, _, _) = doubleneg_output(p1, p2, p3, p4);
        assert!(
            !out.contains("not found"),
            "({p1},{p2},{p3},{p4}) unexpectedly reached the not-found branch:\n{out}"
        );
        assert_eq!(
            out.matches("Found value ").count(),
            4,
            "({p1},{p2},{p3},{p4}) should report all 4 searches as found:\n{out}"
        );
        // The combined-feature loop likewise always finds its byte.
        assert_eq!(out.matches("found=1").count(), 10, "combined loop:\n{out}");
        assert_eq!(out.matches("found=0").count(), 0, "combined loop:\n{out}");
    }

    // And the *reachable* form of this rejection at the low-level export.
    let content = vec![0x01i8; 256];
    assert_eq!(find(&content, 256, 42).0, -1);
}

/// ERRORS row 24 — `direct_search == NULL` is likewise unreachable: byte 100 is
/// always present, so the line is always printed with an identical offset.
fn err_24_doubleneg_direct_search_never_null() {
    for p1 in [-300, -256, -100, -1, 0, 1, 42, 100, 255, 256, 300, i32::MIN, i32::MAX] {
        let (_, out, _, _) = doubleneg_output(p1, 3, 4, 5);
        assert!(
            out.contains("Direct memchr found byte 100 at offset: "),
            "p1={p1}: the direct-memchr line is missing:\n{out}"
        );
    }
    // The reachable NULL form, through the low-level export.
    let content = vec![0x01i8; 256];
    assert_eq!(find(&content, 256, 100).0, -1);
}

/// ERRORS row 25 — `INT_MIN % 1000 == -648` must be reproduced (not `+648`, and
/// no Rust overflow panic).
fn err_25_doubleneg_int_min_modulo() {
    assert_eq!(i32::MIN % 1000, -648, "sanity: C-style truncating remainder");

    // `converted_neg` is always INT_MIN (it converts -2^40), so every call
    // exercises this. Pin it by checking the printed value.
    let (_, out, _, _) = doubleneg_output(0, 0, 0, 0);
    assert!(
        out.contains("Converted to int (UB likely): -2147483648"),
        "{out}"
    );

    // Force `converted_int` to be INT_MIN too: large |a/b| * 10^9.
    // a = INT_MAX, b = 1, c = 9  ->  2147483647 * 1e9, far out of range.
    let (_, out, _, _) = doubleneg_output(i32::MAX, 1, 9, 0);
    assert!(
        out.contains("Converted to int (may be UB): -2147483648"),
        "expected the indefinite value for an out-of-range conversion:\n{out}"
    );
}

/// ERRORS row 26 — `param % 256` where `param == INT_MIN`.
fn err_26_doubleneg_params_int_min() {
    assert_eq!(i32::MIN % 256, 0, "sanity");
    doubleneg_output(i32::MIN, 1, 1, 1);
    doubleneg_output(1, i32::MIN, 1, 1);
    doubleneg_output(1, 1, i32::MIN, 1);
    doubleneg_output(1, 1, 1, i32::MIN);
    doubleneg_output(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    doubleneg_output(i32::MIN + 1, i32::MIN + 1, i32::MIN + 1, i32::MIN + 1);
}

/// ERRORS row 27 — `param1 + i*param2` overflows `int` inside the
/// combined-feature loop.
fn err_27_doubleneg_stride_overflow() {
    // i*param2 overflows for i >= 1 with these magnitudes.
    for (p1, p2) in [
        (i32::MAX, i32::MAX),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MIN, i32::MIN),
        (1, i32::MAX),
        (-1, i32::MIN),
        (i32::MAX, 1_000_000_000),
        (i32::MIN, -1_000_000_000),
        (0, 715_827_883), // 3 * this overflows
    ] {
        doubleneg_output(p1, p2, 1, 1);
    }
}

/// ERRORS row 28 — there is no `enum` in the public API; the equivalent
/// "any `int` is a legal input" surface is swept here across every entry point
/// that takes an `int`, with full-range values including `INT_MIN`/`INT_MAX`.
fn err_28_no_enum_full_int_range_sweep() {
    let (cl, rl) = both();
    let mut rng = Rng::new(SEED ^ 28);

    let mut probes: Vec<i32> = vec![i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for _ in 0..3_000 {
        probes.push(rng.next_i32());
    }

    let all: Vec<i8> = (0..256).map(|b| b as u8 as i8).collect();

    for &v in &probes {
        // process_negation: any int.
        let a = unsafe { (cl.process_negation)(v) };
        let b = unsafe { (rl.process_negation)(v) };
        assert_eq!(a, b, "process_negation({v})");

        // find_value_in_buffer: any int needle.
        find(&all, 256, v);

        // create_numeric_buffer: any int seed, and any int size.
        create_bytes(64, v);
        if v <= 0 {
            assert_create_writes_nothing(v, 7, 32);
        }

        // calculate_with_doubles: any int in all three slots.
        calc(v, 3, 4);
        calc(3, v, 4);
        calc(3, 4, v);
    }
}

// ---------------------------------------------------------------------------
// Sequential entry point (`harness = false`; see Cargo.toml for why).
// ---------------------------------------------------------------------------
fn main() {
    common::run_sequentially(
        "errors",
        &[
            ("err_01_find_absent_needle", err_01_find_absent_needle as fn()),
            ("err_02_find_zero_size", err_02_find_zero_size as fn()),
            ("err_03_find_null_zero_size", err_03_find_null_zero_size as fn()),
            ("err_04_find_out_of_range_needle", err_04_find_out_of_range_needle as fn()),
            ("err_05_find_last_byte_boundary", err_05_find_last_byte_boundary as fn()),
            ("err_06_find_one_past_match", err_06_find_one_past_match as fn()),
            ("err_07_create_zero_size", err_07_create_zero_size as fn()),
            ("err_08_create_negative_size", err_08_create_negative_size as fn()),
            ("err_09_create_null_nonpositive_size", err_09_create_null_nonpositive_size as fn()),
            ("err_10_create_negative_seed", err_10_create_negative_seed as fn()),
            ("err_11_create_seed_overflow", err_11_create_seed_overflow as fn()),
            ("err_12_calc_zero_divisor", err_12_calc_zero_divisor as fn()),
            ("err_13_calc_negative_exponent", err_13_calc_negative_exponent as fn()),
            ("err_14_calc_c_int_min", err_14_calc_c_int_min as fn()),
            ("err_15_calc_int_min_over_minus_one", err_15_calc_int_min_over_minus_one as fn()),
            ("err_16_cvt_above_int_max", err_16_cvt_above_int_max as fn()),
            ("err_17_cvt_below_int_min", err_17_cvt_below_int_min as fn()),
            ("err_18_cvt_pos_infinity", err_18_cvt_pos_infinity as fn()),
            ("err_19_cvt_neg_infinity", err_19_cvt_neg_infinity as fn()),
            ("err_20_cvt_nan_payloads", err_20_cvt_nan_payloads as fn()),
            ("err_21_cvt_boundaries", err_21_cvt_boundaries as fn()),
            ("err_22_cvt_negative_zero_and_subnormal", err_22_cvt_negative_zero_and_subnormal as fn()),
            ("err_23_doubleneg_value_not_found_is_unreachable", err_23_doubleneg_value_not_found_is_unreachable as fn()),
            ("err_24_doubleneg_direct_search_never_null", err_24_doubleneg_direct_search_never_null as fn()),
            ("err_25_doubleneg_int_min_modulo", err_25_doubleneg_int_min_modulo as fn()),
            ("err_26_doubleneg_params_int_min", err_26_doubleneg_params_int_min as fn()),
            ("err_27_doubleneg_stride_overflow", err_27_doubleneg_stride_overflow as fn()),
            ("err_28_no_enum_full_int_range_sweep", err_28_no_enum_full_int_range_sweep as fn()),
        ],
    );
}
