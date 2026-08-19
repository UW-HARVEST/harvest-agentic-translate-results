// Phase C — error-path differential tests.
// One test per row of ERRORS.md.  Each constructs the exact rejecting
// condition, calls BOTH the C `.so` and the Rust `.so`, and asserts they reject
// in the SAME way (the same sentinel output, or the same fatal signal) — not
// merely "both failed somehow".

mod common;
use common::*;

// ---------------------------------------------------------------------------
// Row 1 — endp == str: empty string
// ---------------------------------------------------------------------------
#[test]
fn row01_empty_string() {
    assert_rejected(b"\0", "row01 empty");
    diff_driver_raw(b"\0", "row01 empty");
}

// ---------------------------------------------------------------------------
// Row 2 — endp == str: whitespace only
// ---------------------------------------------------------------------------
#[test]
fn row02_whitespace_only() {
    for s in [
        " \0".as_bytes(),
        "\t\0".as_bytes(),
        "\n\0".as_bytes(),
        "\r\0".as_bytes(),
        "\x0b\0".as_bytes(),
        "\x0c\0".as_bytes(),
        " \t\n\x0b\x0c\r \0".as_bytes(),
        "                    \0".as_bytes(),
    ] {
        assert_rejected(s, "row02 whitespace-only");
    }
    // long whitespace run
    let long = format!("{}\0", " \t\n".repeat(1000));
    assert_rejected(long.as_bytes(), "row02 long whitespace-only");
}

// ---------------------------------------------------------------------------
// Row 3 — endp == str: leading non-numeric byte
// ---------------------------------------------------------------------------
#[test]
fn row03_leading_non_numeric() {
    for s in [
        "abc", "+", "-", "++1", "+-1", "--5", "-+5", ".5", ",", "x10", "e5", "/", ":",
        "n", "inf", "nan", " - 5", "\u{7f}", "%d", "\t+", " -", "- 5", "+ 5",
    ] {
        assert_rejected(format!("{s}\0").as_bytes(), &format!("row03({s})"));
    }
    // `"0 x"` deliberately NOT in the list above: it starts with a digit, so the
    // C ACCEPTS it (strtol consumes "0", endp != str). Pin that down instead of
    // "fixing" it.
    assert_accepted(b"0 x\0", "row03(\"0 x\" is accepted by the C)");
    // high-bit / non-ASCII leading bytes
    for b in [0x80u8, 0xc3, 0xff, 0xfe, 0x81] {
        let buf = [b, b'5', 0];
        assert_rejected(&buf, &format!("row03(0x{b:02x})"));
    }
    // every byte that must be rejected as a 1-byte input
    for b in 1u16..=255 {
        let bb = b as u8;
        let is_digit = bb.is_ascii_digit();
        let buf = [bb, 0u8];
        if is_digit {
            assert_accepted(&buf, &format!("row03 accept(0x{b:02x})"));
        } else {
            assert_rejected(&buf, &format!("row03 reject(0x{b:02x})"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — errno == ERANGE: overflows long
// ---------------------------------------------------------------------------
#[test]
fn row04_erange_overflow() {
    for s in [
        "9223372036854775808",
        "9223372036854775809",
        "99999999999999999999",
        "+9223372036854775808",
        "18446744073709551616",
        "123456789012345678901234567890",
    ] {
        assert_rejected(format!("{s}\0").as_bytes(), &format!("row04({s})"));
    }
    let huge = format!("{}\0", "9".repeat(4096));
    assert_rejected(huge.as_bytes(), "row04 4096 nines");
    // ERANGE must not leak into a later successful call
    assert_accepted(b"7\0", "row04 errno reset after ERANGE");
}

// ---------------------------------------------------------------------------
// Row 5 — errno == ERANGE: underflows long
// ---------------------------------------------------------------------------
#[test]
fn row05_erange_underflow() {
    for s in [
        "-9223372036854775809",
        "-9223372036854775810",
        "-99999999999999999999",
        "-123456789012345678901234567890",
    ] {
        assert_rejected(format!("{s}\0").as_bytes(), &format!("row05({s})"));
    }
    let huge = format!("-{}\0", "9".repeat(4096));
    assert_rejected(huge.as_bytes(), "row05 -4096 nines");
    assert_accepted(b"-7\0", "row05 errno reset after ERANGE");
}

// ---------------------------------------------------------------------------
// Row 6 — tmp > INT_MAX with errno == 0
// ---------------------------------------------------------------------------
#[test]
fn row06_above_int_max() {
    for s in [
        "2147483648",
        "2147483649",
        "+2147483648",
        "4294967295",
        "4294967296",
        "9223372036854775807", // exactly LONG_MAX, errno == 0
        "1000000000000",
    ] {
        assert_rejected(format!("{s}\0").as_bytes(), &format!("row06({s})"));
    }
    // one step either side of the boundary
    assert_accepted(b"2147483647\0", "row06 INT_MAX accepted");
    assert_rejected(b"2147483648\0", "row06 INT_MAX+1 rejected");
}

// ---------------------------------------------------------------------------
// Row 7 — tmp < INT_MIN with errno == 0
// ---------------------------------------------------------------------------
#[test]
fn row07_below_int_min() {
    for s in [
        "-2147483649",
        "-2147483650",
        "-4294967296",
        "-9223372036854775808", // exactly LONG_MIN, errno == 0
        "-1000000000000",
    ] {
        assert_rejected(format!("{s}\0").as_bytes(), &format!("row07({s})"));
    }
    assert_accepted(b"-2147483648\0", "row07 INT_MIN accepted");
    assert_rejected(b"-2147483649\0", "row07 INT_MIN-1 rejected");
}

// ---------------------------------------------------------------------------
// Row 8 — driver(NULL): unchecked dereference inside strtol
// ---------------------------------------------------------------------------
#[test]
fn row08_driver_null_pointer() {
    let l = libs();
    let sc = child_status(|| unsafe { (l.c.driver)(std::ptr::null()) });
    let sr = child_status(|| unsafe { (l.rust.driver)(std::ptr::null()) });
    assert_eq!(
        (term_signal(sc), exit_code(sc)),
        (term_signal(sr), exit_code(sr)),
        "driver(NULL): C {} vs Rust {}",
        describe_status(sc),
        describe_status(sr)
    );
    assert_eq!(
        term_signal(sc),
        Some(11),
        "expected the C behaviour to be SIGSEGV, got {}",
        describe_status(sc)
    );
}

// ---------------------------------------------------------------------------
// Row 9 — run(NULL, x): unchecked dereference in print_house
// ---------------------------------------------------------------------------
#[test]
fn row09_run_null_pointer() {
    let l = libs();
    for extra in [0i32, 1, -1, i32::MAX, i32::MIN] {
        let sc = child_status(|| unsafe { (l.c.run)(std::ptr::null_mut(), extra) });
        let sr = child_status(|| unsafe { (l.rust.run)(std::ptr::null_mut(), extra) });
        assert_eq!(
            (term_signal(sc), exit_code(sc)),
            (term_signal(sr), exit_code(sr)),
            "run(NULL,{extra}): C {} vs Rust {}",
            describe_status(sc),
            describe_status(sr)
        );
        assert_eq!(
            term_signal(sc),
            Some(11),
            "expected SIGSEGV from C, got {}",
            describe_status(sc)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 10 — run() with a wild, non-null pointer
// ---------------------------------------------------------------------------
#[test]
fn row10_run_wild_pointer() {
    let l = libs();
    for addr in [1usize, 3, 0xdead_beef, 0xffff_ffff_ffff_fff0] {
        let sc = child_status(|| unsafe { (l.c.run)(addr as *mut HouseT, 1) });
        let sr = child_status(|| unsafe { (l.rust.run)(addr as *mut HouseT, 1) });
        assert_eq!(
            (term_signal(sc), exit_code(sc)),
            (term_signal(sr), exit_code(sr)),
            "run(0x{addr:x}): C {} vs Rust {}",
            describe_status(sc),
            describe_status(sr)
        );
        assert!(
            term_signal(sc).is_some(),
            "expected a fatal signal from C, got {}",
            describe_status(sc)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 11 — signed int overflow in add_bedrooms (C UB; gcc -O0 wraps)
// ---------------------------------------------------------------------------
#[test]
fn row11_bedrooms_overflow() {
    let mut rng = Rng::new(SEED ^ 111);
    let cases: [(i32, i32); 6] = [
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (2_000_000_000, 2_000_000_000),
        (-2_000_000_000, -2_000_000_000),
    ];
    for (i, &(bed, extra)) in cases.iter().enumerate() {
        diff_run(HouseT::new(0, bed, 2.5), extra, 1, &format!("row11#{i}"));
        diff_run(HouseT::new(0, bed, 2.5), extra, 3, &format!("row11x3#{i}"));
    }
    // randomized: values whose sum overflows i32
    for i in 0..200 {
        let bed = rng.next_i32();
        let extra = rng.next_i32();
        if bed.checked_add(extra).is_none() {
            diff_run(
                HouseT::new(1, bed, 0.5),
                extra,
                1,
                &format!("row11rand#{i}({bed}+{extra})"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — signed int overflow in add_floor
// ---------------------------------------------------------------------------
#[test]
fn row12_floors_overflow() {
    diff_run(HouseT::new(i32::MAX, 0, 2.5), 0, 1, "row12 INT_MAX");
    diff_run(HouseT::new(i32::MAX, 0, 2.5), 0, 4, "row12 INT_MAX x4");
    diff_run(HouseT::new(i32::MAX - 2, 0, 2.5), 0, 6, "row12 near INT_MAX");
    diff_run(HouseT::new(i32::MIN, 0, 2.5), 0, 3, "row12 INT_MIN");
}

// ---------------------------------------------------------------------------
// Row 13 — non-finite bathrooms through %.1f
// ---------------------------------------------------------------------------
#[test]
fn row13_non_finite_bathrooms() {
    let vals: [f64; 8] = [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::from_bits(0xfff8_0000_0000_0001),
        f64::from_bits(0x7ff0_0000_0000_0001), // sNaN
        f64::from_bits(0xfff0_0000_0000_0000), // -inf bit pattern
    ];
    for (i, &b) in vals.iter().enumerate() {
        diff_run(HouseT::new(1, 2, b), 3, 1, &format!("row13#{i}"));
        diff_run(HouseT::new(1, 2, b), 3, 4, &format!("row13x4#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 14 — every int bit pattern is a legal `extra_bedrooms` (there is no enum
// in this API, so the "out-of-range enum value" class degenerates to: any int).
// ---------------------------------------------------------------------------
#[test]
fn row14_full_int_range_extra_bedrooms() {
    // hand-picked "no valid variant" style values an enum-typed param would get
    for extra in [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        99,
        255,
        256,
        65535,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ] {
        diff_run(HouseT::canonical(), extra, 1, &format!("row14({extra})"));
        diff_run(HouseT::canonical(), extra, 2, &format!("row14x2({extra})"));
    }
    let mut rng = Rng::new(SEED ^ 14_14);
    for i in 0..300 {
        diff_run(
            HouseT::canonical(),
            rng.next_i32(),
            1,
            &format!("row14rand#{i}"),
        );
    }
}

// ---------------------------------------------------------------------------
// Generic boundaries every C API has: zero / oversized lengths, one step past
// each documented range, and byte-level fuzz of the reject surface.
// ---------------------------------------------------------------------------
#[test]
fn generic_boundary_fuzz() {
    // zero length
    assert_rejected(b"\0", "generic zero-length");

    // one step past each range boundary in parse_val
    let boundary_pairs: [(&str, bool); 8] = [
        ("2147483646", true),
        ("2147483647", true),
        ("2147483648", false),
        ("2147483649", false),
        ("-2147483647", true),
        ("-2147483648", true),
        ("-2147483649", false),
        ("-2147483650", false),
    ];
    for (s, accept) in boundary_pairs {
        let buf = format!("{s}\0");
        if accept {
            assert_accepted(buf.as_bytes(), &format!("generic accept({s})"));
        } else {
            assert_rejected(buf.as_bytes(), &format!("generic reject({s})"));
        }
    }

    // LONG boundaries (errno == 0 vs ERANGE)
    for s in [
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
    ] {
        assert_rejected(format!("{s}\0").as_bytes(), &format!("generic long({s})"));
    }

    // oversized inputs
    for n in [1usize, 2, 19, 20, 100, 1000, 8192] {
        let digits = format!("{}\0", "1".repeat(n));
        diff_driver_raw(digits.as_bytes(), &format!("generic {n} digits"));
        let alpha = format!("{}\0", "z".repeat(n));
        assert_rejected(alpha.as_bytes(), &format!("generic {n} letters"));
        let ws = format!("{}5\0", " ".repeat(n));
        assert_accepted(ws.as_bytes(), &format!("generic {n} spaces + 5"));
    }

    // full-range fuzz over the reject surface, deterministic
    let mut rng = Rng::new(SEED ^ 0xBEEF);
    for i in 0..800 {
        let len = rng.range_usize(0, 40);
        let mut buf: Vec<u8> = (0..len).map(|_| 1 + (rng.next_u64() % 255) as u8).collect();
        buf.push(0);
        diff_driver_raw(&buf, &format!("generic fuzz#{i}"));
    }
}
