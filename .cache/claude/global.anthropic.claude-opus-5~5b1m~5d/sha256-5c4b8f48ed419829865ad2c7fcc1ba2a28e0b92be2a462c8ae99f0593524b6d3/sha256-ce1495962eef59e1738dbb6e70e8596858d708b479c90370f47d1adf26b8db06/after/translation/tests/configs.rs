//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`, each driven with many randomized inputs
//! from a fixed-seed PRNG. Both the high-level `driver` wrapper and the
//! low-level `run` entry point are called directly through the `.so` exports.

mod harness;
use harness::*;

const WS: &[u8] = b" \t\n\x0b\x0c\r";

// ===========================================================================
// C1 — bare small positive decimal
// ===========================================================================
#[test]
fn cfg_c1_bare_small_positive() {
    for s in ["0", "1", "2", "3", "7", "9", "10", "42", "100", "9999"] {
        assert_accepted(s.as_bytes(), s.parse().unwrap(), &format!("C1 {s}"));
    }
    let mut rng = Rng::new();
    for i in 0..400 {
        let v = rng.below(10_000) as i32;
        let s = v.to_string();
        assert_accepted(s.as_bytes(), v, &format!("C1 random#{i} {s}"));
    }
}

// ===========================================================================
// C2 — explicit '+' sign
// ===========================================================================
#[test]
fn cfg_c2_plus_sign() {
    for s in ["+0", "+1", "+42", "+2147483647", "+000123"] {
        let v: i32 = s.trim_start_matches('+').parse().unwrap();
        assert_accepted(s.as_bytes(), v, &format!("C2 {s}"));
    }
    let mut rng = Rng::new();
    for i in 0..400 {
        let v = rng.below(i32::MAX as u64 + 1) as i32;
        let s = format!("+{v}");
        assert_accepted(s.as_bytes(), v, &format!("C2 random#{i} {s}"));
    }
}

// ===========================================================================
// C3 — '-' sign
// ===========================================================================
#[test]
fn cfg_c3_minus_sign() {
    for s in ["-0", "-1", "-42", "-2147483648", "-000123"] {
        let v: i32 = s.parse().unwrap();
        assert_accepted(s.as_bytes(), v, &format!("C3 {s}"));
    }
    let mut rng = Rng::new();
    for i in 0..400 {
        let v = rng.range_i64(i32::MIN as i64, 0) as i32;
        let s = v.to_string();
        assert_accepted(s.as_bytes(), v, &format!("C3 random#{i} {s}"));
    }
}

// ===========================================================================
// C4 — every leading-whitespace class strtol skips
// ===========================================================================
#[test]
fn cfg_c4_leading_whitespace() {
    for &w in WS {
        for body in ["7", "+7", "-7", "2147483647", "-2147483648"] {
            let mut s = vec![w];
            s.extend_from_slice(body.as_bytes());
            let v: i32 = body.trim_start_matches('+').parse().unwrap();
            assert_accepted(&s, v, &format!("C4 ws {w:#04x} {body}"));
        }
    }
    let mut rng = Rng::new();
    for i in 0..400 {
        let mut s = Vec::new();
        for _ in 0..rng.below(6) {
            s.push(WS[rng.below(6) as usize]);
        }
        let v = rng.next_i32();
        if rng.next_u64() & 1 == 0 && v >= 0 {
            s.push(b'+');
        }
        s.extend_from_slice(v.to_string().as_bytes());
        assert_accepted(&s, v, &format!("C4 random#{i} {:?}", show(&s)));
    }
}

// ===========================================================================
// C5 — leading zeros
// ===========================================================================
#[test]
fn cfg_c5_leading_zeros() {
    assert_accepted(b"0000000042", 42, "C5 zeros42");
    assert_accepted(b"-0000001", -1, "C5 zeros-1");
    assert_accepted(b"+0000000", 0, "C5 zeros+0");
    let mut s = vec![b'0'; 64];
    s.extend_from_slice(b"12345");
    assert_accepted(&s, 12345, "C5 64 zeros");
    let mut s = vec![b'-'];
    s.extend(std::iter::repeat(b'0').take(64));
    s.extend_from_slice(b"2147483648"); // -2147483648 == INT_MIN, valid
    assert_accepted(&s, i32::MIN, "C5 64 zeros INT_MIN");

    let mut rng = Rng::new();
    for i in 0..300 {
        let v = rng.next_i32();
        let pad = rng.below(40) as usize;
        let mut s = Vec::new();
        if v < 0 {
            s.push(b'-');
        }
        s.extend(std::iter::repeat(b'0').take(pad));
        s.extend_from_slice(v.unsigned_abs().to_string().as_bytes());
        assert_accepted(&s, v, &format!("C5 random#{i} pad={pad} v={v}"));
    }
}

// ===========================================================================
// C6 — trailing garbage (the C never checks *endp)
// ===========================================================================
#[test]
fn cfg_c6_trailing_garbage() {
    for (s, v) in [
        ("12abc", 12),
        ("5 ", 5),
        ("5\n", 5),
        ("5\t9", 5),
        ("7,9", 7),
        ("0!", 0),
        ("-3zzz", -3),
        ("+8 more", 8),
        ("2147483647!!!", i32::MAX),
        ("-2147483648???", i32::MIN),
    ] {
        assert_accepted(s.as_bytes(), v, &format!("C6 {s}"));
    }

    let mut rng = Rng::new();
    let junk: Vec<u8> = (33u8..=126).filter(|b| !b.is_ascii_digit()).collect();
    for i in 0..500 {
        let v = rng.next_i32();
        let mut s = v.to_string().into_bytes();
        let len = 1 + rng.below(10) as usize;
        for _ in 0..len {
            s.push(junk[rng.below(junk.len() as u64) as usize]);
        }
        assert_accepted(&s, v, &format!("C6 random#{i} {:?}", show(&s)));
    }
}

// ===========================================================================
// C7 — hex-looking input under base 10
// ===========================================================================
#[test]
fn cfg_c7_hex_looking_base10() {
    for (s, v) in [
        ("0x10", 0),
        ("0X1f", 0),
        ("0b1", 0),
        ("0o7", 0),
        ("-0x10", 0),
        ("+0XABC", 0),
        ("0xFFFFFFFFFFFFFFFFFFFF", 0),
        ("00x5", 0),
        ("10x5", 10),
    ] {
        assert_accepted(s.as_bytes(), v, &format!("C7 {s}"));
    }
    let mut rng = Rng::new();
    for i in 0..200 {
        let hex = format!("0x{:x}", rng.next_u64());
        assert_accepted(hex.as_bytes(), 0, &format!("C7 random#{i} {hex}"));
    }
}

// ===========================================================================
// C8 — decimal point / exponent forms truncated at the first non-digit
// ===========================================================================
#[test]
fn cfg_c8_decimal_and_exponent_forms() {
    for (s, v) in [
        ("7.9", 7),
        ("-3.5", -3),
        ("0.0", 0),
        ("2e5", 2),
        ("2E5", 2),
        ("1_000", 1),
        ("1,000", 1),
        ("3.14159", 3),
        ("-0.5", 0),
        ("+9.99", 9),
        ("12345678901234567890.5", 0), // ERANGE actually -> handled in errors
    ]
    .iter()
    .take(10)
    {
        assert_accepted(s.as_bytes(), *v, &format!("C8 {s}"));
    }
    // The 11th case above overflows long -> rejected; assert that explicitly.
    assert_rejected(b"12345678901234567890.5", "C8 overflow with decimal point");

    let mut rng = Rng::new();
    for i in 0..300 {
        let ip = rng.range_i64(i32::MIN as i64, i32::MAX as i64) as i32;
        let frac = rng.below(1_000_000);
        let s = format!("{ip}.{frac}");
        assert_accepted(s.as_bytes(), ip, &format!("C8 random#{i} {s}"));
    }
}

// ===========================================================================
// C9 — embedded NUL
// ===========================================================================
#[test]
fn cfg_c9_embedded_nul() {
    let mut rng = Rng::new();
    for i in 0..200 {
        let v = rng.next_i32();
        let mut buf = v.to_string().into_bytes();
        buf.push(0);
        // Trailing bytes after the NUL must be invisible to both.
        for _ in 0..rng.below(8) {
            buf.push(b'0' + rng.below(10) as u8);
        }
        let out = diff_driver_raw(&buf, &format!("C9 random#{i} v={v}"));
        assert_eq!(out, model_driver(v), "C9 value mismatch for {v}");
    }
}

// ===========================================================================
// C10 — exact valid boundaries
// ===========================================================================
#[test]
fn cfg_c10_int_boundaries() {
    assert_accepted(b"2147483647", i32::MAX, "C10 INT_MAX");
    assert_accepted(b"-2147483648", i32::MIN, "C10 INT_MIN");
    assert_accepted(b"2147483646", i32::MAX - 1, "C10 INT_MAX-1");
    assert_accepted(b"-2147483647", i32::MIN + 1, "C10 INT_MIN+1");
    assert_accepted(b"0", 0, "C10 zero");
    assert_accepted(b"-0", 0, "C10 minus zero");
}

// ===========================================================================
// C11 — magnitudes that wrap `bedrooms` across driver's two run() calls
// ===========================================================================
#[test]
fn cfg_c11_bedrooms_wraparound_via_driver() {
    for v in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        1_073_741_824,
        2_147_483_640,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -1_073_741_824,
        -2_147_483_640,
    ] {
        let s = v.to_string();
        assert_accepted(s.as_bytes(), v, &format!("C11 {s}"));
    }
    // Fine sweep right where 5 + x overflows.
    for d in -8i64..=8 {
        let v = (i32::MAX as i64 - 5 + d).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        let s = v.to_string();
        assert_accepted(s.as_bytes(), v, &format!("C11 sweep+ {s}"));
        let v = (i32::MIN as i64 - 5 + d).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        let s = v.to_string();
        assert_accepted(s.as_bytes(), v, &format!("C11 sweep- {s}"));
    }
}

// ===========================================================================
// C12 — randomized full-i32-range decimal strings
// ===========================================================================
#[test]
fn cfg_c12_random_full_range() {
    let mut rng = Rng::new();
    for i in 0..1000 {
        let v = rng.next_i32();
        let s = v.to_string();
        assert_accepted(s.as_bytes(), v, &format!("C12 random#{i} {s}"));
    }
}

// ===========================================================================
// C13 — randomized composite strings (ws x sign x magnitude x suffix)
// ===========================================================================
#[test]
fn cfg_c13_random_composite_strings() {
    let mut rng = Rng::new();
    let junk: Vec<u8> = (33u8..=126).filter(|b| !b.is_ascii_digit()).collect();
    for i in 0..1000 {
        let mut s = Vec::new();
        for _ in 0..rng.below(4) {
            s.push(WS[rng.below(6) as usize]);
        }
        let v = rng.next_i32();
        if v >= 0 && rng.next_u64() & 1 == 0 {
            s.push(b'+');
        }
        if v < 0 {
            s.push(b'-');
        }
        for _ in 0..rng.below(5) {
            s.push(b'0');
        }
        s.extend_from_slice(v.unsigned_abs().to_string().as_bytes());
        for _ in 0..rng.below(6) {
            s.push(junk[rng.below(junk.len() as u64) as usize]);
        }
        assert_accepted(&s, v, &format!("C13 random#{i} {:?}", show(&s)));
    }
}

// ===========================================================================
// C14 — run() with driver's own starting state
// ===========================================================================
#[test]
fn cfg_c14_run_driver_default_state() {
    for extra in [0i32, 1, -1, 2, -2, 5, -5] {
        let (out, h) = diff_run(House::driver_default(), extra, &format!("C14 extra={extra}"));
        assert_eq!(
            out.split(|&b| b == b'\n').filter(|l| !l.is_empty()).count(),
            4,
            "run() must print exactly 4 lines"
        );
        assert_eq!(h.floors, 3);
        assert_eq!(h.bedrooms, 5i32.wrapping_add(extra));
        assert_eq!(h.bathrooms.to_bits(), 3.5f64.to_bits());
    }
}

// ===========================================================================
// C15 — floors boundary / wraparound
// ===========================================================================
#[test]
fn cfg_c15_run_floors_boundaries() {
    let mut rng = Rng::new();
    for floors in [0i32, 1, -1, 2, -2, i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1] {
        for _ in 0..8 {
            let extra = rng.next_i32();
            let (_, h) = diff_run(
                House::new(floors, rng.next_i32(), rng.next_finite_f64()),
                extra,
                &format!("C15 floors={floors} extra={extra}"),
            );
            assert_eq!(h.floors, floors.wrapping_add(1));
        }
    }
}

// ===========================================================================
// C16 — bedrooms boundary / wraparound
// ===========================================================================
#[test]
fn cfg_c16_run_bedrooms_boundaries() {
    let mut rng = Rng::new();
    for bedrooms in [0i32, 1, -1, i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1] {
        for extra in [0i32, 1, -1, i32::MAX, i32::MIN, 2, -2] {
            let (_, h) = diff_run(
                House::new(rng.next_i32(), bedrooms, 2.5),
                extra,
                &format!("C16 bedrooms={bedrooms} extra={extra}"),
            );
            assert_eq!(h.bedrooms, bedrooms.wrapping_add(extra));
        }
        for _ in 0..16 {
            let extra = rng.next_i32();
            diff_run(
                House::new(rng.next_i32(), bedrooms, rng.next_finite_f64()),
                extra,
                &format!("C16 random bedrooms={bedrooms} extra={extra}"),
            );
        }
    }
}

// ===========================================================================
// C17 — extra_bedrooms at the int extremes
// ===========================================================================
#[test]
fn cfg_c17_run_extra_extremes() {
    let mut rng = Rng::new();
    for extra in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, -1, 1] {
        for _ in 0..25 {
            let h = rng.random_house(true);
            diff_run(h, extra, &format!("C17 extra={extra}"));
        }
    }
}

// ===========================================================================
// C18 — negative floors / bedrooms (%d sign width)
// ===========================================================================
#[test]
fn cfg_c18_run_negative_counts() {
    let mut rng = Rng::new();
    for _ in 0..200 {
        let floors = -(rng.below(i32::MAX as u64) as i32);
        let bedrooms = -(rng.below(i32::MAX as u64) as i32);
        let extra = rng.next_i32();
        diff_run(
            House::new(floors, bedrooms, rng.next_finite_f64()),
            extra,
            &format!("C18 f={floors} b={bedrooms} e={extra}"),
        );
    }
    // Digit-count boundaries for %d.
    for mag in [0i64, 1, 9, 10, 99, 100, 999, 1000, 999_999_999, 1_000_000_000] {
        for sign in [1i64, -1] {
            let v = (sign * mag) as i32;
            diff_run(House::new(v, v, 2.5), v, &format!("C18 mag={v}"));
        }
    }
}

// ===========================================================================
// C19 / C20 — zero and negative zero bathrooms
// ===========================================================================
#[test]
fn cfg_c19_c20_zero_bathrooms() {
    let mut rng = Rng::new();
    for bits in [0.0f64.to_bits(), (-0.0f64).to_bits()] {
        let b = f64::from_bits(bits);
        for _ in 0..20 {
            let (_, h) = diff_run(
                House::new(rng.next_i32(), rng.next_i32(), b),
                rng.next_i32(),
                &format!("C19/C20 bathrooms bits {bits:#018x}"),
            );
            // -0.0 + 1.0 == 1.0, 0.0 + 1.0 == 1.0
            assert_eq!(h.bathrooms.to_bits(), 1.0f64.to_bits());
        }
    }
    // -0.0 must *print* as "-0.0" before the increment; verify the bytes.
    let (out, _) = diff_run(House::new(0, 0, -0.0), 0, "C20 negative zero print");
    assert!(
        out.starts_with(b"The house has 0 floors, 0 bedrooms, and -0.0 bathrooms\n"),
        "expected -0.0 to print with its sign, got {}",
        show(&out)
    );
}

// ===========================================================================
// C21 — %.1f rounding ties
// ===========================================================================
#[test]
fn cfg_c21_run_rounding_ties() {
    let vals: &[f64] = &[
        0.05, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85, 0.95, 1.05, 1.15, 1.25, 2.45, 2.5,
        3.5, -0.05, -0.25, -0.35, -1.05, -2.45, 0.949999999999999, 0.9500000000000001,
        0.04999999999999999, 1.0 / 3.0, 2.0 / 3.0, 0.1, 0.2, 0.3, 0.7, 1e-1, 1e-2, 1e-3, 1e-9,
        4.999999999999999, 5.000000000000001, -4.95, 9.95, 99.95, 0.4999999999999999,
    ];
    let mut rng = Rng::new();
    for &v in vals {
        for _ in 0..3 {
            diff_run(
                House::new(rng.next_i32(), rng.next_i32(), v),
                rng.next_i32(),
                &format!("C21 bathrooms={v:?}"),
            );
        }
    }
    // Randomized ties: k/20 for k in 0..400 (exactly the x.x5 family in decimal).
    for k in 0..400i64 {
        let v = k as f64 / 20.0;
        diff_run(House::new(0, 0, v), 0, &format!("C21 k/20 = {v:?}"));
        diff_run(House::new(0, 0, -v), 0, &format!("C21 -k/20 = {:?}", -v));
    }
}

// ===========================================================================
// C22 — subnormals
// ===========================================================================
#[test]
fn cfg_c22_run_subnormals() {
    let vals: &[f64] = &[
        5e-324,
        -5e-324,
        f64::from_bits(1),
        f64::from_bits(0x000f_ffff_ffff_ffff),
        f64::from_bits(0x800f_ffff_ffff_ffff),
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::MIN_POSITIVE / 2.0,
        1e-310,
        -1e-310,
    ];
    let mut rng = Rng::new();
    for &v in vals {
        for _ in 0..5 {
            let (_, h) = diff_run(
                House::new(rng.next_i32(), rng.next_i32(), v),
                rng.next_i32(),
                &format!("C22 bathrooms={v:e}"),
            );
            assert_eq!(h.bathrooms.to_bits(), (v + 1.0).to_bits());
        }
    }
    // Random subnormal bit patterns.
    for i in 0..200 {
        let bits = (rng.next_u64() & 0x000f_ffff_ffff_ffff) | ((rng.next_u64() & 1) << 63);
        let v = f64::from_bits(bits);
        diff_run(House::new(0, 0, v), 0, &format!("C22 random#{i} {bits:#018x}"));
    }
}

// ===========================================================================
// C23 — huge magnitudes where += 1.0 is a no-op (~310-digit %.1f output)
// ===========================================================================
#[test]
fn cfg_c23_run_huge_magnitudes() {
    let vals: &[f64] = &[
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        9007199254740992.0,       // 2^53
        9007199254740993.0,       // not representable -> 2^53
        9007199254740994.0,       // 2^53 + 2
        -9007199254740992.0,
        1e15,
        1e16,
        1e17,
        1e22,
        1e23,
        1e100,
        1e200,
        f64::from_bits(0x7fef_ffff_ffff_ffff),
        f64::from_bits(0xffef_ffff_ffff_ffff),
    ];
    let mut rng = Rng::new();
    for &v in vals {
        for _ in 0..4 {
            let (out, h) = diff_run(
                House::new(rng.next_i32(), rng.next_i32(), v),
                rng.next_i32(),
                &format!("C23 bathrooms={v:e}"),
            );
            assert_eq!(h.bathrooms.to_bits(), (v + 1.0).to_bits());
            assert!(!out.is_empty());
        }
    }
    // f64::MAX must produce a ~311 char number in %.1f.
    let (out, _) = diff_run(House::new(0, 0, f64::MAX), 0, "C23 f64::MAX width");
    let first = out.split(|&b| b == b'\n').next().unwrap();
    assert!(
        first.len() > 300,
        "expected a very long %.1f expansion, got {} bytes",
        first.len()
    );
}

// ===========================================================================
// C24 — infinities
// ===========================================================================
#[test]
fn cfg_c24_run_infinities() {
    let mut rng = Rng::new();
    for v in [f64::INFINITY, f64::NEG_INFINITY] {
        for _ in 0..10 {
            let (out, h) = diff_run(
                House::new(rng.next_i32(), rng.next_i32(), v),
                rng.next_i32(),
                &format!("C24 bathrooms={v}"),
            );
            assert_eq!(h.bathrooms.to_bits(), v.to_bits(), "inf + 1.0 must stay inf");
            assert!(
                out.windows(3).any(|w| w == b"inf"),
                "expected 'inf' in %.1f output, got {}",
                show(&out)
            );
        }
    }
}

// ===========================================================================
// C25 — NaNs (sign + payload propagation through += 1.0 and %.1f)
// ===========================================================================
#[test]
fn cfg_c25_run_nans() {
    let nan_bits: &[u64] = &[
        0x7ff8_0000_0000_0000, // canonical quiet NaN
        0xfff8_0000_0000_0000, // negative quiet NaN
        0x7ff8_0000_0000_0001,
        0x7fff_ffff_ffff_ffff,
        0xffff_ffff_ffff_ffff,
        0x7ff0_0000_0000_0001, // signalling NaN
        0xfff0_0000_0000_0001, // negative signalling NaN
        0x7ffa_aaaa_aaaa_aaaa,
        0xfff5_5555_5555_5555,
    ];
    let mut rng = Rng::new();
    for &bits in nan_bits {
        let v = f64::from_bits(bits);
        assert!(v.is_nan());
        for _ in 0..5 {
            let (out, _) = diff_run(
                House::new(rng.next_i32(), rng.next_i32(), v),
                rng.next_i32(),
                &format!("C25 NaN bits {bits:#018x}"),
            );
            assert!(
                out.windows(3).any(|w| w == b"nan"),
                "expected 'nan' in %.1f output, got {}",
                show(&out)
            );
        }
    }
}

// ===========================================================================
// C26 — fully randomized house_t with raw-bit-pattern bathrooms
// ===========================================================================
#[test]
fn cfg_c26_random_house_raw_bits() {
    let mut rng = Rng::new();
    for i in 0..2000 {
        let h = rng.random_house(false);
        let extra = rng.next_i32();
        diff_run(h, extra, &format!("C26 random#{i}"));
    }
}

// ===========================================================================
// C27 — fully randomized house_t with finite bathrooms over the exponent range
// ===========================================================================
#[test]
fn cfg_c27_random_house_finite() {
    let mut rng = Rng::with_seed(0xabcd_ef01_2345_6789);
    for i in 0..1000 {
        let h = rng.random_house(true);
        let extra = rng.next_i32();
        diff_run(h, extra, &format!("C27 random#{i}"));
    }
}

// ===========================================================================
// C28 — state accumulation: the same struct through run() 1..=16 times
// ===========================================================================
#[test]
fn cfg_c28_run_sequences() {
    let mut rng = Rng::new();
    for n in 1..=16usize {
        for rep in 0..4 {
            let start = rng.random_house(rep % 2 == 0);
            let extras: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
            diff_run_sequence(start, &extras, &format!("C28 n={n} rep={rep}"));
        }
    }
    // The exact shape driver() uses: 2 calls, same extra, from the default state.
    for x in [0i32, 1, -1, i32::MAX, i32::MIN, 12345] {
        diff_run_sequence(House::driver_default(), &[x, x], &format!("C28 driver-shape {x}"));
    }
    // Long sequence to accumulate floors/bathrooms far from the start.
    let extras: Vec<i32> = (0..200).map(|_| rng.next_i32()).collect();
    diff_run_sequence(House::new(i32::MAX - 100, i32::MIN + 3, 0.05), &extras, "C28 long");
}

// ===========================================================================
// C29 — interleaved driver/run calls (hidden global or errno coupling)
// ===========================================================================
#[test]
fn cfg_c29_interleaved_entry_points() {
    let mut rng = Rng::with_seed(0x1357_9bdf_0246_8ace);
    for i in 0..400 {
        match rng.below(4) {
            0 => {
                let v = rng.next_i32();
                let s = v.to_string();
                assert_accepted(s.as_bytes(), v, &format!("C29 driver-ok#{i}"));
            }
            1 => {
                assert_rejected(b"not-a-number", &format!("C29 driver-err#{i}"));
            }
            2 => {
                let h = rng.random_house(true);
                diff_run(h, rng.next_i32(), &format!("C29 run#{i}"));
            }
            _ => {
                let h = rng.random_house(false);
                diff_run(h, rng.next_i32(), &format!("C29 run-bits#{i}"));
            }
        }
    }
}

// ===========================================================================
// C30 — input length axis
// ===========================================================================
#[test]
fn cfg_c30_input_length_axis() {
    assert_rejected(b"", "C30 len 0");
    assert_accepted(b"1", 1, "C30 len 1");
    assert_accepted(b"12", 12, "C30 len 2");
    assert_accepted(b" 1", 1, "C30 len 2 ws");
    assert_accepted(b"+1", 1, "C30 len 2 sign");

    // 4096 bytes of leading zeros then a valid magnitude.
    let mut s = vec![b'0'; 4096];
    s.extend_from_slice(b"424242");
    assert_accepted(&s, 424242, "C30 4096 zeros");

    // Valid digit followed by 4096 bytes of junk.
    let mut s = b"-7".to_vec();
    s.extend(std::iter::repeat(b'Z').take(4096));
    assert_accepted(&s, -7, "C30 4096 junk suffix");

    // 4096 bytes of whitespace then a value.
    let mut s = vec![b'\t'; 4096];
    s.extend_from_slice(b"-2147483648");
    assert_accepted(&s, i32::MIN, "C30 4096 ws");

    // Randomized lengths from 1 to 512.
    let mut rng = Rng::new();
    for i in 0..200 {
        let pad = rng.below(512) as usize;
        let v = rng.next_i32();
        let mut s = Vec::new();
        if v < 0 {
            s.push(b'-');
        }
        s.extend(std::iter::repeat(b'0').take(pad));
        s.extend_from_slice(v.unsigned_abs().to_string().as_bytes());
        assert_accepted(&s, v, &format!("C30 random#{i} pad={pad}"));
    }
}
