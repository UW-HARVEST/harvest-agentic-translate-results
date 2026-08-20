// Phase C — error / rejection-path differential tests.
// One test per row of ERRORS.md (rows 1..24 and 30).  Rows 25..29 involve
// `doubleneg`, which writes to stdout, so they live in
// `tests/phase_c_doubleneg_errors.rs` (a single-`#[test]` binary).
//
// Every test asserts the two implementations return the SAME sentinel / value,
// never merely "both failed somehow".

mod common;

use common::{assert_bytes_eq, assert_f64_bits_eq, assert_i32_eq, c, rs, Rng};
use std::ffi::c_char;
use std::ptr;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn find_pair(buf: *const c_char, size: usize, sv: i32) -> (i32, i32) {
    (
        unsafe { (c().find_value_in_buffer)(buf, size, sv) },
        unsafe { (rs().find_value_in_buffer)(buf, size, sv) },
    )
}

fn cmp_find(buf: &[i8], size: usize, sv: i32, ctx: &str) -> i32 {
    let (cv, rv) = find_pair(buf.as_ptr() as *const c_char, size, sv);
    assert_i32_eq(cv, rv, ctx);
    cv
}

fn cmp_convert(v: f64) -> i32 {
    let cv = unsafe { (c().convert_double_to_int)(v) };
    let rv = unsafe { (rs().convert_double_to_int)(v) };
    assert_i32_eq(
        cv,
        rv,
        &format!("convert_double_to_int({v:?} / bits {:#018x})", v.to_bits()),
    );
    cv
}

fn cmp_calc(a: i32, b: i32, cc: i32) -> f64 {
    let cv = unsafe { (c().calculate_with_doubles)(a, b, cc) };
    let rv = unsafe { (rs().calculate_with_doubles)(a, b, cc) };
    assert_f64_bits_eq(cv, rv, &format!("calculate_with_doubles({a}, {b}, {cc})"));
    cv
}

fn cmp_create_full(cap: usize, size: i32, seed: i32, sentinel: i8) -> Vec<u8> {
    let mut cbuf = vec![sentinel; cap];
    let mut rbuf = vec![sentinel; cap];
    unsafe { (c().create_numeric_buffer)(cbuf.as_mut_ptr() as *mut c_char, size, seed) };
    unsafe { (rs().create_numeric_buffer)(rbuf.as_mut_ptr() as *mut c_char, size, seed) };
    let cb: Vec<u8> = cbuf.iter().map(|&b| b as u8).collect();
    let rb: Vec<u8> = rbuf.iter().map(|&b| b as u8).collect();
    assert_bytes_eq(
        &cb,
        &rb,
        &format!("create_numeric_buffer(cap={cap}, size={size}, seed={seed})"),
    );
    cb
}

// ===========================================================================
// Row 1 — absent byte -> memchr NULL -> return -1
// ===========================================================================
#[test]
fn err01_fvib_absent_returns_minus_one() {
    let mut rng = Rng::new(0xC001);
    for _ in 0..3000 {
        let len = 1 + rng.below(300) as usize;
        // Fill with a single byte, then search for a different one.
        let filler = rng.next_u8();
        let buf = vec![filler as i8; len];
        let missing = filler.wrapping_add(1 + rng.next_u8() % 254);
        let got = cmp_find(
            &buf,
            len,
            missing as i32,
            &format!("row1 absent byte {missing} in {len} x {filler}"),
        );
        assert_eq!(got, -1, "row1: expected the -1 sentinel, got {got}");
    }
    // Exhaustive on a small buffer: every byte except the filler is absent.
    let buf = [0x33i8; 8];
    for b in 0u32..256 {
        let got = cmp_find(&buf, 8, b as i32, &format!("row1 exhaustive {b}"));
        assert_eq!(got, if b == 0x33 { 0 } else { -1 });
    }
}

// ===========================================================================
// Row 2 — size == 0
// ===========================================================================
#[test]
fn err02_fvib_zero_size_returns_minus_one() {
    let mut rng = Rng::new(0xC002);
    for _ in 0..2000 {
        let len = 1 + rng.below(64) as usize;
        let buf: Vec<i8> = (0..len).map(|_| rng.next_u8() as i8).collect();
        // Even a byte that IS at index 0 must not be found with size == 0.
        let present = buf[0] as i32;
        for sv in [present, 0, 42, -1, rng.next_i32()] {
            let got = cmp_find(&buf, 0, sv, &format!("row2 size=0 sv={sv}"));
            assert_eq!(got, -1, "row2: size==0 must yield -1, got {got}");
        }
    }
}

// ===========================================================================
// Row 3 — NULL buffer with size == 0
// ===========================================================================
#[test]
fn err03_fvib_null_buffer_zero_size() {
    for sv in [
        0,
        1,
        42,
        -1,
        255,
        256,
        i32::MAX,
        i32::MIN,
        0x1FF,
        -0x1FF,
        0x7F,
        0x80,
    ] {
        let (cv, rv) = find_pair(ptr::null(), 0, sv);
        assert_i32_eq(cv, rv, &format!("row3 NULL buffer, size=0, sv={sv}"));
        assert_eq!(cv, -1, "row3: expected -1 for a NULL/empty range");
    }
}

// ===========================================================================
// Row 4 — search_val outside 0..=255 is narrowed by (char) then unsigned char
// ===========================================================================
#[test]
fn err04_fvib_search_val_out_of_byte_range() {
    let mut rng = Rng::new(0xC004);
    // Buffer holding every byte value exactly once at a known index.
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    for _ in 0..4000 {
        let sv = rng.next_i32();
        let got = cmp_find(&buf, 256, sv, &format!("row4 sv={sv}"));
        // The narrowing the C performs: (char)sv, then memchr's unsigned char.
        let expected = (sv as u8) as i32;
        assert_eq!(
            got, expected,
            "row4: search_val {sv} should narrow to byte {expected}, got {got}"
        );
    }
    // Values equal modulo 256 must behave identically.
    for base in [0i32, 1, 42, 127, 128, 255] {
        let mut prev: Option<i32> = None;
        for k in -8i32..=8 {
            let sv = base + 256 * k;
            let got = cmp_find(&buf, 256, sv, &format!("row4 mod256 sv={sv}"));
            if let Some(p) = prev {
                assert_eq!(got, p, "row4: sv={sv} diverged from an sv 256 apart");
            }
            prev = Some(got);
        }
    }
    // Out-of-byte-range value whose narrowed byte is ABSENT -> -1.
    let ones = [0x11i8; 16];
    let got = cmp_find(&ones, 16, 0x1FF, "row4 narrow-to-absent 0x1FF");
    assert_eq!(got, -1);
    let got = cmp_find(&ones, 16, 0x111, "row4 narrow-to-present 0x111");
    assert_eq!(got, 0);
}

// ===========================================================================
// Row 5 — match at index 0 is 0, not -1 and not "NULL"
// ===========================================================================
#[test]
fn err05_fvib_match_at_index_zero_is_not_error() {
    let mut rng = Rng::new(0xC005);
    for _ in 0..3000 {
        let len = 1 + rng.below(300) as usize;
        let needle = rng.next_u8();
        let mut buf = vec![needle.wrapping_add(1) as i8; len];
        buf[0] = needle as i8;
        let got = cmp_find(&buf, len, needle as i32, "row5 match at index 0");
        assert_eq!(got, 0, "row5: expected index 0, got {got}");
    }
    // Including the byte 0 at index 0.
    let buf = [0i8, 1, 2, 3];
    assert_eq!(cmp_find(&buf, 4, 0, "row5 NUL at index 0"), 0);
}

// ===========================================================================
// Row 6 — NUL is a normal searchable value (memchr is length-based)
// ===========================================================================
#[test]
fn err06_fvib_nul_byte_is_searchable() {
    let mut rng = Rng::new(0xC006);
    for _ in 0..2000 {
        let len = 2 + rng.below(300) as usize;
        let mut buf: Vec<i8> = (0..len).map(|_| (1 + rng.next_u8() % 255) as i8).collect();
        let idx = 1 + rng.below(len as u64 - 1) as usize;
        buf[idx] = 0;
        let got = cmp_find(&buf, len, 0, "row6 NUL search");
        assert_eq!(got, idx as i32, "row6: expected NUL at {idx}, got {got}");

        // A non-NUL byte placed AFTER a NUL is still reachable.
        if idx + 1 < len {
            buf[idx + 1] = 0x5E;
            let mut first = None;
            for (i, &b) in buf.iter().enumerate() {
                if b == 0x5E {
                    first = Some(i);
                    break;
                }
            }
            let got = cmp_find(&buf, len, 0x5E, "row6 byte after NUL");
            assert_eq!(got, first.unwrap() as i32);
        }
    }
    // No NUL at all -> -1 (not "end of string").
    let buf: Vec<i8> = (1..=64).map(|i| i as i8).collect();
    assert_eq!(cmp_find(&buf, 64, 0, "row6 no NUL"), -1);
}

// ===========================================================================
// Row 7 — INT_MIN / INT_MAX search_val
// ===========================================================================
#[test]
fn err07_fvib_extreme_search_val() {
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    // INT_MIN = 0x80000000 -> low byte 0x00 ; INT_MAX = 0x7FFFFFFF -> 0xFF
    let got = cmp_find(&buf, 256, i32::MIN, "row7 INT_MIN");
    assert_eq!(got, 0x00, "row7: INT_MIN should narrow to byte 0x00");
    let got = cmp_find(&buf, 256, i32::MAX, "row7 INT_MAX");
    assert_eq!(got, 0xFF, "row7: INT_MAX should narrow to byte 0xFF");
    for sv in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 127,
        i32::MIN + 128,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 127,
        i32::MAX - 128,
    ] {
        let got = cmp_find(&buf, 256, sv, &format!("row7 sv={sv}"));
        assert_eq!(got, (sv as u8) as i32);
    }
    // Same extremes against a buffer where that byte is absent -> -1.
    let zeros_absent = [0x7Fi8; 32];
    assert_eq!(
        cmp_find(&zeros_absent, 32, i32::MIN, "row7 INT_MIN absent"),
        -1
    );
    assert_eq!(
        cmp_find(&zeros_absent, 32, i32::MAX, "row7 INT_MAX absent"),
        -1
    );
}

// ===========================================================================
// Row 8 — b == 0 guard: result stays 0.0
// ===========================================================================
#[test]
fn err08_cwd_zero_divisor_guard() {
    let mut rng = Rng::new(0xC008);
    for _ in 0..3000 {
        let a = rng.interesting_i32();
        let cc = rng.interesting_i32();
        let v = cmp_calc(a, 0, cc);
        assert_eq!(
            v.to_bits(),
            0.0f64.to_bits(),
            "row8: b==0 must give +0.0, got {v:?} for a={a}, c={cc}"
        );
    }
    for cc in [0, 1, -1, 9, -9, 10, -10, i32::MAX, i32::MIN] {
        for a in [0, 1, -1, i32::MAX, i32::MIN] {
            let v = cmp_calc(a, 0, cc);
            assert_eq!(v.to_bits(), 0.0f64.to_bits());
        }
    }
}

// ===========================================================================
// Row 9 — c == INT_MIN -> c % 10 == -8
// ===========================================================================
#[test]
fn err09_cwd_int_min_exponent() {
    assert_eq!(i32::MIN % 10, -8, "sanity: INT_MIN % 10");
    let mut rng = Rng::new(0xC009);
    for _ in 0..2000 {
        let a = rng.interesting_i32();
        let b = loop {
            let v = rng.interesting_i32();
            if v != 0 {
                break v;
            }
        };
        let v = cmp_calc(a, b, i32::MIN);
        let expect = (a as f64 / b as f64) * 1e-8f64.max(f64::MIN_POSITIVE);
        let _ = expect; // exact pow value comes from libm; C is the oracle.
        assert!(v.is_finite() || v.is_nan() || v.is_infinite());
    }
    for cc in [i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MIN + 8, i32::MIN + 9] {
        cmp_calc(1, 1, cc);
        cmp_calc(i32::MIN, i32::MAX, cc);
        cmp_calc(0, 0, cc);
    }
}

// ===========================================================================
// Row 10 — a == INT_MIN, b == -1 (the integer-overflow pair, widened to double)
// ===========================================================================
#[test]
fn err10_cwd_int_min_over_minus_one() {
    for cc in -30..=30 {
        let v = cmp_calc(i32::MIN, -1, cc);
        assert!(
            v > 0.0,
            "row10: INT_MIN / -1 must be positive, got {v:?} for c={cc}"
        );
    }
    let v = cmp_calc(i32::MIN, -1, 0);
    assert_eq!(v, 2147483648.0, "row10: expected exactly 2^31");
    cmp_calc(i32::MIN, 1, 0);
    cmp_calc(i32::MAX, -1, 0);
    cmp_calc(-1, i32::MIN, 0);
    for cc in [i32::MAX, i32::MIN, 0] {
        cmp_calc(i32::MIN, -1, cc);
        cmp_calc(i32::MIN, i32::MIN, cc);
    }
}

// ===========================================================================
// Row 11 — negative c gives a NEGATIVE C remainder (not Euclidean)
// ===========================================================================
#[test]
fn err11_cwd_negative_exponent_sign() {
    // c = -3 -> pow(10, -3), i.e. the result shrinks. A Euclidean modulus would
    // give 7 and blow the value up instead.
    let v = cmp_calc(1, 1, -3);
    assert!(
        v < 1.0 && v > 0.0,
        "row11: c=-3 must scale down (pow(10,-3)), got {v:?}"
    );
    let w = cmp_calc(1, 1, 7);
    assert!(w > 1.0e6, "row11: c=7 must scale up, got {w:?}");
    assert_ne!(v.to_bits(), w.to_bits(), "row11: -3 and 7 must differ");

    let mut rng = Rng::new(0xC011);
    for _ in 0..3000 {
        let cc = -(1 + rng.below(1_000_000) as i32);
        let a = rng.range_i32(-10_000, 10_000);
        let b = loop {
            let v = rng.range_i32(-10_000, 10_000);
            if v != 0 {
                break v;
            }
        };
        cmp_calc(a, b, cc);
    }
    for cc in -9..=-1 {
        cmp_calc(1, 1, cc);
        cmp_calc(-1, 1, cc);
        cmp_calc(i32::MAX, 1, cc);
    }
}

// ===========================================================================
// Rows 12..17 — convert_double_to_int out-of-range / NaN / infinity
// ===========================================================================
#[test]
fn err12_cdti_above_int_max() {
    for v in [
        2147483648.0,
        2147483649.0,
        2147483648.5,
        3e9,
        1e10,
        1e18,
        1e300,
        f64::MAX,
        4294967295.0,
        4294967296.0,
    ] {
        let got = cmp_convert(v);
        assert_eq!(
            got,
            i32::MIN,
            "row12: {v:?} is out of int range; cvttsd2si gives 0x80000000, got {got}"
        );
    }
    let mut rng = Rng::new(0xC012);
    for _ in 0..3000 {
        // Anything strictly above 2^31 truncates out of range.
        let v = 2147483648.0f64 + (rng.next_u32() as f64) + (rng.next_u32() as f64) * 1e6;
        assert_eq!(cmp_convert(v), i32::MIN);
    }
}

#[test]
fn err13_cdti_below_int_min() {
    for v in [
        -2147483649.0,
        -2147483650.0,
        -3e9,
        -1e10,
        -1e18,
        -1e300,
        f64::MIN,
        -4294967296.0,
        -1099511627776.0, // -1.0 * pow(2, 40), exactly what doubleneg computes
    ] {
        let got = cmp_convert(v);
        assert_eq!(got, i32::MIN, "row13: {v:?} out of range, got {got}");
    }
    let mut rng = Rng::new(0xC013);
    for _ in 0..3000 {
        let v = -2147483649.0f64 - (rng.next_u32() as f64) - (rng.next_u32() as f64) * 1e6;
        assert_eq!(cmp_convert(v), i32::MIN);
    }
}

#[test]
fn err14_cdti_nan() {
    let mut nans = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // canonical quiet NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // maximal payload
        f64::from_bits(0x7FF8_0000_DEAD_BEEF),
    ];
    let mut rng = Rng::new(0xC014);
    for _ in 0..2000 {
        // Random NaN payloads (exponent all ones, non-zero mantissa).
        let mantissa = (rng.next_u64() & 0x000F_FFFF_FFFF_FFFF) | 1;
        nans.push(f64::from_bits(0x7FF0_0000_0000_0000 | mantissa));
        nans.push(f64::from_bits(0xFFF0_0000_0000_0000 | mantissa));
    }
    for v in nans {
        assert!(v.is_nan(), "test bug: {v:?} is not NaN");
        let got = cmp_convert(v);
        assert_eq!(
            got,
            i32::MIN,
            "row14: NaN (bits {:#018x}) must convert to 0x80000000, got {got}",
            v.to_bits()
        );
    }
}

#[test]
fn err15_cdti_pos_infinity() {
    let got = cmp_convert(f64::INFINITY);
    assert_eq!(got, i32::MIN, "row15: +inf must give 0x80000000, got {got}");
    assert_eq!(cmp_convert(f64::from_bits(0x7FF0_0000_0000_0000)), i32::MIN);
    // Also via the library's own arithmetic: 1.0 / 0.0.
    assert_eq!(cmp_convert(1.0f64 / 0.0), i32::MIN);
}

#[test]
fn err16_cdti_neg_infinity() {
    let got = cmp_convert(f64::NEG_INFINITY);
    assert_eq!(got, i32::MIN, "row16: -inf must give 0x80000000, got {got}");
    assert_eq!(cmp_convert(f64::from_bits(0xFFF0_0000_0000_0000)), i32::MIN);
    assert_eq!(cmp_convert(-1.0f64 / 0.0), i32::MIN);
}

#[test]
fn err17_cdti_one_step_past_range() {
    // In range (truncates toward zero).
    assert_eq!(cmp_convert(2147483647.0), i32::MAX);
    assert_eq!(cmp_convert(2147483647.5), i32::MAX);
    assert_eq!(cmp_convert(-2147483648.0), i32::MIN);
    assert_eq!(cmp_convert(-2147483648.5), i32::MIN); // truncates to -2^31
    assert_eq!(cmp_convert(-2147483648.999), i32::MIN);
    // The largest double strictly below 2^31.
    let just_below = f64::from_bits(2147483648.0f64.to_bits() - 1);
    assert!(just_below < 2147483648.0);
    assert_eq!(cmp_convert(just_below), i32::MAX);
    // One step past.
    assert_eq!(cmp_convert(2147483648.0), i32::MIN);
    let just_above = f64::from_bits(2147483648.0f64.to_bits() + 1);
    assert_eq!(cmp_convert(just_above), i32::MIN);
    assert_eq!(cmp_convert(-2147483649.0), i32::MIN);

    // Walk every representable neighbour of both endpoints.
    for anchor in [2147483648.0f64, -2147483648.0f64] {
        let mut up = anchor;
        let mut down = anchor;
        for _ in 0..64 {
            up = f64::from_bits(up.to_bits() + 1);
            down = f64::from_bits(down.to_bits() - 1);
            cmp_convert(up);
            cmp_convert(down);
            cmp_convert(-up);
            cmp_convert(-down);
        }
    }
}

// ===========================================================================
// Rows 18..22 — create_numeric_buffer boundaries
// ===========================================================================
#[test]
fn err18_cnb_zero_size_writes_nothing() {
    let mut rng = Rng::new(0xC018);
    for _ in 0..2000 {
        let seed = rng.next_i32();
        let cap = 1 + rng.below(64) as usize;
        let sentinel = rng.next_u8() as i8;
        let out = cmp_create_full(cap, 0, seed, sentinel);
        assert!(
            out.iter().all(|&b| b == sentinel as u8),
            "row18: size==0 must not write; seed={seed}"
        );
    }
}

#[test]
fn err19_cnb_negative_size_writes_nothing() {
    let mut rng = Rng::new(0xC019);
    for size in [-1i32, -2, -7, -256, -1000, i32::MIN, i32::MIN + 1, -0x4000_0000] {
        for seed in [0, 1, -1, 42, i32::MAX, i32::MIN] {
            let out = cmp_create_full(32, size, seed, 0x5A);
            assert!(
                out.iter().all(|&b| b == 0x5A),
                "row19: size={size} must not write (no wrap to a huge count)"
            );
        }
    }
    for _ in 0..2000 {
        let size = -(1 + rng.below(2_000_000_000) as i32);
        let out = cmp_create_full(32, size, rng.next_i32(), 0x5A);
        assert!(out.iter().all(|&b| b == 0x5A), "row19: size={size} wrote");
    }
}

#[test]
fn err20_cnb_null_buffer_nonpositive_size() {
    // The pointer is never dereferenced when size <= 0, so passing NULL is safe
    // and both implementations must simply return.
    for size in [0i32, -1, -256, i32::MIN] {
        for seed in [0, 1, -1, i32::MAX, i32::MIN] {
            unsafe { (c().create_numeric_buffer)(ptr::null_mut(), size, seed) };
            unsafe { (rs().create_numeric_buffer)(ptr::null_mut(), size, seed) };
        }
    }
    // Reaching here at all is the assertion: neither library touched the NULL.
}

#[test]
fn err21_cnb_seed_overflow_wraps() {
    let mut rng = Rng::new(0xC021);
    for delta in 0..64i32 {
        cmp_create_full(600, 512, i32::MAX - delta, 0x5A);
        cmp_create_full(600, 512, i32::MIN + delta, 0x5A);
    }
    for _ in 0..600 {
        let d = rng.below(10_000) as i32;
        cmp_create_full(300, 256, i32::MAX - d, 0x5A);
        cmp_create_full(300, 256, i32::MIN + d, 0x5A);
    }
    // The very first element already overflows for seed == INT_MAX and i >= 1.
    let out = cmp_create_full(8, 8, i32::MAX, 0x5A);
    assert_eq!(out[0], (i32::MAX % 256) as u8, "row21: element 0");
}

#[test]
fn err22_cnb_negative_seed_signed_char() {
    // (seed + i*7) % 256 is negative in C for negative sums; the (char) cast
    // then stores that negative value.
    let out = cmp_create_full(4, 4, -1, 0x5A);
    assert_eq!(out[0], 0xFF, "row22: seed=-1 -> (char)(-1) == 0xFF");
    let out = cmp_create_full(4, 4, -300, 0x5A);
    assert_eq!(out[0], (-44i8) as u8, "row22: seed=-300 -> -300%256 == -44");

    let mut rng = Rng::new(0xC022);
    for seed in -3000..0 {
        cmp_create_full(40, 33, seed, 0x5A);
    }
    for _ in 0..2000 {
        let seed = -(1 + rng.below(2_000_000_000) as i32);
        cmp_create_full(40, 33, seed, 0x5A);
    }
}

// ===========================================================================
// Rows 23, 24 — process_negation
// ===========================================================================
#[test]
fn err23_pn_zero() {
    let cv = unsafe { (c().process_negation)(0) };
    let rv = unsafe { (rs().process_negation)(0) };
    assert_i32_eq(cv, rv, "row23 process_negation(0)");
    assert_eq!(cv, 0, "row23: !!0 must be 0");
}

#[test]
fn err24_pn_nonzero_incl_extremes() {
    let mut vals: Vec<i32> = vec![
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        256,
        -256,
        0x1_0000,
        -0x1_0000,
        0x4000_0000,
        -0x4000_0000,
    ];
    for k in 0..32 {
        vals.push(1i32 << k);
        vals.push((1i32 << k).wrapping_neg());
    }
    let mut rng = Rng::new(0xC024);
    for _ in 0..4000 {
        let v = rng.next_i32();
        if v != 0 {
            vals.push(v);
        }
    }
    for v in vals {
        if v == 0 {
            continue;
        }
        let cv = unsafe { (c().process_negation)(v) };
        let rv = unsafe { (rs().process_negation)(v) };
        assert_i32_eq(cv, rv, &format!("row24 process_negation({v})"));
        assert_eq!(cv, 1, "row24: !!{v} must be 1, got {cv}");
    }
}

// ===========================================================================
// Row 30 — no enums exist; the whole int domain is valid for every parameter,
// so nothing may be rejected and every value must agree.
// ===========================================================================
#[test]
fn err30_no_enum_all_ints_accepted() {
    let extremes: [i32; 14] = [
        i32::MIN,
        i32::MIN + 1,
        -0x4000_0000,
        -65536,
        -256,
        -1,
        0,
        1,
        255,
        256,
        65535,
        0x4000_0000,
        i32::MAX - 1,
        i32::MAX,
    ];

    // process_negation: total function over the whole domain.
    for &v in &extremes {
        let cv = unsafe { (c().process_negation)(v) };
        let rv = unsafe { (rs().process_negation)(v) };
        assert_i32_eq(cv, rv, &format!("row30 process_negation({v})"));
    }

    // find_value_in_buffer: search_val has no valid subrange.
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    for &v in &extremes {
        let got = cmp_find(&buf, 256, v, &format!("row30 find sv={v}"));
        assert_eq!(got, (v as u8) as i32, "row30: sv={v} narrowing");
        cmp_find(&buf, 0, v, &format!("row30 find size=0 sv={v}"));
        cmp_find(&buf, 1, v, &format!("row30 find size=1 sv={v}"));
    }

    // create_numeric_buffer: seed has no valid subrange (size is bounded by the
    // allocation, so only non-positive and in-capacity sizes are exercised).
    for &seed in &extremes {
        cmp_create_full(64, 64, seed, 0x5A);
        cmp_create_full(64, 0, seed, 0x5A);
        cmp_create_full(64, -1, seed, 0x5A);
    }

    // calculate_with_doubles: full cross-product of extremes.
    for &a in &extremes {
        for &b in &extremes {
            cmp_calc(a, b, 3);
            cmp_calc(a, 3, b);
        }
    }
    for &cc in &extremes {
        cmp_calc(7, 11, cc);
        cmp_calc(7, 0, cc);
    }

    // convert_double_to_int: every f64 bit pattern is a valid input.
    let mut rng = Rng::new(0xC030);
    for _ in 0..8000 {
        cmp_convert(f64::from_bits(rng.next_u64()));
    }
}
