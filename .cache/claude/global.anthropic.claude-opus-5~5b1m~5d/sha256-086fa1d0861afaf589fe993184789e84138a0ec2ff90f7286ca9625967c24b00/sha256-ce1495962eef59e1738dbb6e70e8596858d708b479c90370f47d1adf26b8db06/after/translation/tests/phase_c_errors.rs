//! Phase C — error-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md`. Every test asserts the two
//! implementations produce the *same* rejection (the same sentinel output and
//! the same `errno`), not merely "both failed somehow".

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/// A rejected `driver` call must print exactly `An error occurred\n` and
/// nothing else, in both implementations.
fn expect_reject(payload: &[u8]) {
    let out = assert_same(&Call::Driver(payload.to_vec()));
    assert_eq!(
        out,
        ERR_MSG,
        "expected exactly {:?} for input {:?}, got {:?}",
        String::from_utf8_lossy(ERR_MSG),
        String::from_utf8_lossy(payload),
        String::from_utf8_lossy(&out)
    );
    // Cross-check against the oracle: the C really does reject this.
    assert!(
        c_parse_val(payload).is_none(),
        "oracle says {:?} is acceptable — table row is wrong",
        String::from_utf8_lossy(payload)
    );
}

fn expect_accept(payload: &[u8]) {
    let out = assert_same(&Call::Driver(payload.to_vec()));
    assert_eq!(
        line_count(&out),
        DRIVER_OK_LINES,
        "expected acceptance for {:?}, got {:?}",
        String::from_utf8_lossy(payload),
        String::from_utf8_lossy(&out)
    );
    assert!(c_parse_val(payload).is_some());
}

// ===========================================================================
// Row 1 — endp == str: empty string.
// ===========================================================================
#[test]
fn err01_empty_string() {
    for _ in 0..5 {
        expect_reject(b"");
    }
}

// ===========================================================================
// Row 2 — endp == str: non-numeric text.
// ===========================================================================
#[test]
fn err02_non_numeric() {
    let cases: &[&[u8]] = &[
        b"abc",
        b"hello",
        b"++1",
        b"--1",
        b"+-1",
        b"-+1",
        b"+",
        b"-",
        b" ",
        b"\t\n",
        b"\t\n\x0b\x0c\r ",
        b".",
        b",",
        b"e5",
        b"NaN",
        b"nan",
        b"inf",
        b"Infinity",
        b"null",
        b"\xef\xbc\x90",         // U+FF10 FULLWIDTH DIGIT ZERO
        b"\x80",                 // lone continuation byte
        b"\xff\xfe",
        b"'1'",
        b"\"1\"",
        b"(1)",
        b"[1]",
        b"#10",
        b"$5",
        b"/3",
        b"*",
        b"~",
        b"_1",
        b"one",
        b"\x01\x02\x03",
        b" - 1",
        b"+ 1",
        b"- 1",
        b"\n\n\n",
        b"\x0b",
        b"\x0c",
        b"\r",
    ];
    for c in cases {
        expect_reject(c);
    }
}

// ===========================================================================
// Row 2b — randomized non-numeric garbage (property style).
// ===========================================================================
#[test]
fn err02b_random_non_numeric() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 0x2b);
    // Bytes that can never start a base-10 conversion. '+'/'-' are allowed
    // here only when *not* followed by a digit, which the generator ensures by
    // drawing exclusively from this alphabet.
    let alphabet: Vec<u8> = (1u8..=255)
        .filter(|b| !b.is_ascii_digit())
        .filter(|b| !b" \t\n\x0b\x0c\r".contains(b)) // no leading-ws-only strings here
        .collect();
    for _ in 0..400 {
        let n = 1 + rng.below(12);
        let mut s = Vec::with_capacity(n);
        for _ in 0..n {
            s.push(*rng.pick(&alphabet));
        }
        // Guard: the generated string must really be unparseable. If the
        // generator happened to produce something acceptable (it cannot, but
        // be safe), fall back to asserting whatever the C does.
        if c_parse_val(&s).is_none() {
            expect_reject(&s);
        } else {
            expect_accept(&s);
        }
    }
}

// ===========================================================================
// Row 3 — base-10 digit prefixes that consume nothing.
// ===========================================================================
#[test]
fn err03_wrong_base_prefixes() {
    for s in ["x10", "X10", "#10", "o17", "b101", "0"] {
        if s == "0" {
            expect_accept(s.as_bytes()); // control: plain zero IS accepted
        } else {
            expect_reject(s.as_bytes());
        }
    }
}

// ===========================================================================
// Row 4 — errno == ERANGE from long overflow.
// ===========================================================================
#[test]
fn err04_erange_overflow() {
    let cases = [
        "9223372036854775808".to_string(),
        "9223372036854775809".to_string(),
        "18446744073709551616".to_string(),
        "99999999999999999999999".to_string(),
        "1".to_string() + &"0".repeat(30),
        "9".repeat(400),
        "+".to_string() + &"9".repeat(100),
    ];
    for s in &cases {
        expect_reject(s.as_bytes());
        // The rejection reason really is ERANGE.
        assert_eq!(
            {
                let _ = c_parse_val(s.as_bytes());
                errno()
            },
            ERANGE,
            "expected ERANGE for {s:?}"
        );
    }
}

// ===========================================================================
// Row 5 — errno == ERANGE from long underflow.
// ===========================================================================
#[test]
fn err05_erange_underflow() {
    let cases = [
        "-9223372036854775809".to_string(),
        "-9223372036854775810".to_string(),
        "-18446744073709551616".to_string(),
        "-99999999999999999999999".to_string(),
        "-".to_string() + &"9".repeat(400),
    ];
    for s in &cases {
        expect_reject(s.as_bytes());
        assert_eq!(
            {
                let _ = c_parse_val(s.as_bytes());
                errno()
            },
            ERANGE,
            "expected ERANGE for {s:?}"
        );
    }
}

// ===========================================================================
// Row 6 — tmp > INT_MAX (fits in long, not in int).
// ===========================================================================
#[test]
fn err06_above_int_max() {
    for s in [
        "2147483648",
        "2147483649",
        "2147483650",
        "4294967295",
        "4294967296",
        "10000000000",
        "9223372036854775806",
        "9223372036854775807", // LONG_MAX exactly, errno stays 0
        "+2147483648",
        "0000002147483648",
        "  2147483648  ",
        "2147483648abc",
    ] {
        expect_reject(s.as_bytes());
        // Reason is the range check, not ERANGE.
        let _ = c_parse_val(s.as_bytes());
        assert_eq!(errno(), 0, "expected errno==0 (range check) for {s:?}");
    }
}

// ===========================================================================
// Row 7 — tmp < INT_MIN.
// ===========================================================================
#[test]
fn err07_below_int_min() {
    for s in [
        "-2147483649",
        "-2147483650",
        "-4294967296",
        "-10000000000",
        "-9223372036854775807",
        "-9223372036854775808", // LONG_MIN exactly, errno stays 0
        "-0000002147483649",
        "  -2147483649  ",
        "-2147483649xyz",
    ] {
        expect_reject(s.as_bytes());
        let _ = c_parse_val(s.as_bytes());
        assert_eq!(errno(), 0, "expected errno==0 (range check) for {s:?}");
    }
}

// ===========================================================================
// Rows 6/7 randomized — every long value one step or more outside int range.
// ===========================================================================
#[test]
fn err06_07_randomized_out_of_int_range() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 0x67);
    for _ in 0..250 {
        let above = rng.range_i64(c_int::MAX as i64 + 1, i64::MAX);
        expect_reject(above.to_string().as_bytes());
    }
    for _ in 0..250 {
        let below = rng.range_i64(i64::MIN, c_int::MIN as i64 - 1);
        expect_reject(below.to_string().as_bytes());
    }
}

// ===========================================================================
// Row 8 — run(INT_MAX): signed overflow of bedrooms, no rejection.
// ===========================================================================
#[test]
fn err08_run_int_max_overflow() {
    for _ in 0..8 {
        let out = assert_same(&Call::Run(c_int::MAX));
        assert_eq!(line_count(&out), RUN_LINES);
    }
}

// ===========================================================================
// Row 9 — run(INT_MIN): signed underflow of bedrooms.
// ===========================================================================
#[test]
fn err09_run_int_min_underflow() {
    for _ in 0..8 {
        let out = assert_same(&Call::Run(c_int::MIN));
        assert_eq!(line_count(&out), RUN_LINES);
    }
}

// ===========================================================================
// Row 10 — "out-of-range enum" equivalent: arbitrary 32-bit patterns in the
// only non-pointer parameter the API has.
// ===========================================================================
#[test]
fn err10_arbitrary_bit_patterns() {
    let pats: [u32; 16] = [
        0x0000_0000,
        0x0000_0001,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFF,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0xAAAA_AAAA,
        0x5555_5555,
        0xFFFF_0000,
        0x0000_FFFF,
        0x8000_0001,
        0x7FFF_FFFE,
        0xFEED_FACE,
        0x0BAD_F00D,
        0x1234_5678,
    ];
    for p in pats {
        let out = assert_same(&Call::Run(p as c_int));
        assert_eq!(line_count(&out), RUN_LINES, "pattern {p:#010x}");
    }
    // And exhaustively over a randomized sample.
    let mut rng = Rng::with_seed(Rng::SEED ^ 0x10);
    for _ in 0..300 {
        assert_same(&Call::Run(rng.next_i32()));
    }
}

// ===========================================================================
// Row 11 — one step INSIDE the rejected range must be ACCEPTED.
// ===========================================================================
#[test]
fn err11_boundary_inside_is_accepted() {
    for s in ["2147483647", "-2147483648", "2147483646", "-2147483647"] {
        expect_accept(s.as_bytes());
    }
    // Symmetry: one step outside is rejected.
    for s in ["2147483648", "-2147483649"] {
        expect_reject(s.as_bytes());
    }
}

// ===========================================================================
// Row 12 — zero-length input.
// ===========================================================================
#[test]
fn err12_zero_length() {
    expect_reject(b"");
    // A buffer whose first byte is NUL, with garbage after it: C sees "".
    let _g = lock();
    let p = pair();
    let buf: [u8; 8] = [0, b'4', b'2', 0, 0, 0, 0, 0];
    set_errno(0);
    let c_out = capture(|| unsafe { p.c.driver(buf.as_ptr() as *const c_char) });
    let c_errno = errno();
    set_errno(0);
    let rust_out = capture(|| unsafe { p.rust.driver(buf.as_ptr() as *const c_char) });
    let rust_errno = errno();
    assert_eq!(c_out, rust_out);
    assert_eq!(c_out, ERR_MSG);
    assert_eq!(c_errno, rust_errno);
}

// ===========================================================================
// Row 13 — oversized input.
// ===========================================================================
#[test]
fn err13_oversized_input() {
    // Valid prefix + 2 MiB of junk => ACCEPTED.
    let mut v = b"-99".to_vec();
    v.extend(std::iter::repeat(b'!').take(2 << 20));
    expect_accept(&v);

    // 100 000 digits => ERANGE => REJECTED.
    let huge = "1".repeat(100_000);
    expect_reject(huge.as_bytes());
    let neg_huge = format!("-{}", "7".repeat(100_000));
    expect_reject(neg_huge.as_bytes());

    // 100 000 leading zeros then a valid value => ACCEPTED.
    let zeros = format!("{}5", "0".repeat(100_000));
    expect_accept(zeros.as_bytes());

    // 100 000 leading spaces then nothing => REJECTED.
    let ws = " ".repeat(100_000);
    expect_reject(ws.as_bytes());
}

// ===========================================================================
// Row 14 — inputs that LOOK malformed but strtol accepts.
// ===========================================================================
#[test]
fn err14_looks_malformed_but_accepted() {
    for s in [
        "   42",
        "+42",
        "007",
        "\t\n\x0b\x0c\r 42",
        "-0",
        "+0",
        "          -0000000000000000007",
        "\r\n+2147483647",
    ] {
        expect_accept(s.as_bytes());
    }
}

// ===========================================================================
// Row 15 — trailing-garbage forms are accepted.
// ===========================================================================
#[test]
fn err15_trailing_garbage_accepted() {
    for s in [
        "42abc", "42 43", "1,000", "3.9", "12e3", "0x1A", "-5-6", "7+8", "9/10", "0.0",
        "2147483647junk",
    ] {
        expect_accept(s.as_bytes());
    }
}

// ===========================================================================
// Row 16 — NULL pointer. Undefined behaviour in the C (strtol(NULL, ...)):
// both implementations must fault identically.
// ===========================================================================
#[test]
fn err16_null_pointer_faults_identically() {
    let _g = lock();
    let p = pair();
    let c_run = p.c.driver_ptr();
    let r_run = p.rust.driver_ptr();

    let c_term = fork_probe(|| unsafe { c_run(std::ptr::null()) });
    let r_term = fork_probe(|| unsafe { r_run(std::ptr::null()) });

    assert_eq!(
        c_term, r_term,
        "driver(NULL) terminated differently: C={c_term:?} Rust={r_term:?}"
    );
    // Sanity: it really is a fatal fault in the C, not a graceful return.
    assert!(
        matches!(c_term, Term::Signaled(_)),
        "expected the C to fault on driver(NULL), got {c_term:?}"
    );
}

// ===========================================================================
// Row 17 — a pre-existing non-zero errno must NOT cause a rejection.
// ===========================================================================
#[test]
fn err17_preexisting_errno_neutralised() {
    for pre in [ERANGE, EINVAL, 1, 2, 5, 9, 13, 100, c_int::MAX] {
        let out = assert_same_errno(&Call::Driver(b"123".to_vec()), pre);
        assert_eq!(
            line_count(&out),
            DRIVER_OK_LINES,
            "pre-existing errno={pre} wrongly caused a rejection"
        );
        let out = assert_same_errno(&Call::Driver(b"".to_vec()), pre);
        assert_eq!(out, ERR_MSG);
    }
}

// ===========================================================================
// Row 18 — observable errno side effect after driver() returns.
// ===========================================================================
#[test]
fn err18_errno_side_effect_matches() {
    let _g = lock();
    let p = pair();

    let cases: &[(&str, bool)] = &[
        ("99999999999999999999999", true), // ERANGE expected
        ("-99999999999999999999999", true),
        ("42", false), // errno cleared to 0 and left alone
        ("", false),
        ("abc", false),
        ("2147483648", false), // range check, errno stays 0
        ("-2147483649", false),
    ];

    for &(s, expect_erange) in cases {
        let mut buf = s.as_bytes().to_vec();
        buf.push(0);

        set_errno(0x7f);
        let c_out = capture(|| unsafe { p.c.driver(buf.as_ptr() as *const c_char) });
        let c_errno = errno();

        set_errno(0x7f);
        let rust_out = capture(|| unsafe { p.rust.driver(buf.as_ptr() as *const c_char) });
        let rust_errno = errno();

        assert_eq!(
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out),
            "stdout divergence for {s:?}"
        );
        assert_eq!(
            c_errno, rust_errno,
            "errno divergence after driver({s:?}): C={c_errno} Rust={rust_errno}"
        );
        if expect_erange {
            assert_eq!(c_errno, ERANGE, "expected ERANGE after driver({s:?})");
        } else {
            assert_eq!(c_errno, 0, "expected errno==0 after driver({s:?})");
        }
    }
}

// ===========================================================================
// Extra generic boundary: `run` never rejects, exhaustive small neighbourhood
// around every power-of-two boundary.
// ===========================================================================
#[test]
fn err_extra_run_power_of_two_neighbourhoods() {
    let mut vals: Vec<c_int> = Vec::new();
    for bit in 0..31u32 {
        let v = 1i64 << bit;
        for d in -1i64..=1 {
            vals.push((v + d) as c_int);
            vals.push((-v + d) as c_int);
        }
    }
    vals.push(c_int::MAX);
    vals.push(c_int::MIN);
    for v in vals {
        assert_same(&Call::Run(v));
    }
}

// ===========================================================================
// Extra generic boundary: every string of length 0..=3 over a small but
// adversarial alphabet — exhaustive coverage of the accept/reject frontier.
// ===========================================================================
#[test]
fn err_extra_exhaustive_short_strings() {
    let alphabet: &[u8] = b"0192+- \t.xeE\n";
    let mut buf: Vec<u8> = Vec::new();
    // length 0
    check_agrees(&buf);
    for &a in alphabet {
        buf.clear();
        buf.push(a);
        check_agrees(&buf);
        for &b in alphabet {
            buf.clear();
            buf.extend_from_slice(&[a, b]);
            check_agrees(&buf);
            for &c in alphabet {
                buf.clear();
                buf.extend_from_slice(&[a, b, c]);
                check_agrees(&buf);
            }
        }
    }
}

/// Whatever the C decides, the Rust must decide identically.
fn check_agrees(payload: &[u8]) {
    let out = assert_same(&Call::Driver(payload.to_vec()));
    match c_parse_val(payload) {
        None => assert_eq!(
            out,
            ERR_MSG,
            "oracle says reject for {:?} but got {:?}",
            String::from_utf8_lossy(payload),
            String::from_utf8_lossy(&out)
        ),
        Some(_) => assert_eq!(
            line_count(&out),
            DRIVER_OK_LINES,
            "oracle says accept for {:?} but got {:?}",
            String::from_utf8_lossy(payload),
            String::from_utf8_lossy(&out)
        ),
    }
}
