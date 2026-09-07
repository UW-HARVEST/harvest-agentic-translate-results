//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each test constructs the exact invalid
//! input, calls BOTH `.so` files, and asserts they produce the SAME rejection
//! (the exact `"An error occurred\n"` sentinel, or the same fatal signal for
//! the unchecked-NULL rows), not merely "both failed somehow".

mod common;

use common::*;

const SENTINEL: &[u8] = b"An error occurred\n";

/// Assert C and Rust agree AND that the result is exactly the C sentinel.
fn both_reject(input: &[u8]) {
    let c = c_impl().driver_bytes(input);
    let r = rust_impl().driver_bytes(input);
    assert_eq!(
        c, r,
        "stdout mismatch for {:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    assert_eq!(
        c,
        SENTINEL.to_vec(),
        "expected the exact rejection sentinel for {:?}, got {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c)
    );
}

fn both_accept(input: &[u8]) {
    let c = c_impl().driver_bytes(input);
    let r = rust_impl().driver_bytes(input);
    assert_eq!(
        c, r,
        "stdout mismatch for {:?}",
        String::from_utf8_lossy(input)
    );
    assert_ne!(
        c,
        SENTINEL.to_vec(),
        "expected acceptance for {:?}",
        String::from_utf8_lossy(input)
    );
}

// ===========================================================================
// Row 1 — endp == str: empty string
// ===========================================================================
#[test]
fn err_01_empty_string() {
    both_reject(b"");
}

// ===========================================================================
// Row 2 — endp == str: whitespace only
// ===========================================================================
#[test]
fn err_02_whitespace_only() {
    for s in [
        " ",
        "\t",
        "\n",
        "\r",
        "\x0b",
        "\x0c",
        "   ",
        " \t\r\n\x0b\x0c ",
        "\t\t\t\t\t\t\t\t",
    ] {
        both_reject(s.as_bytes());
    }
}

// ===========================================================================
// Row 3 — endp == str: sign with no digits
// ===========================================================================
#[test]
fn err_03_sign_without_digits() {
    for s in [
        "+", "-", "++", "--", "+-3", "-+3", "- 3", "+ 3", "  +", "  -", "+.", "-.", "+x1",
    ] {
        both_reject(s.as_bytes());
    }
}

// ===========================================================================
// Row 4 — endp == str: non-numeric leading text
// ===========================================================================
#[test]
fn err_04_non_numeric_prefix() {
    for s in [
        "abc", "x1", ".5", "-.5", "e5", "E5", "/9", ":9", "NaN", "nan", "inf", "Infinity", "null",
        "#42", "'42'", "\"42\"", "٣", "１２３", "\u{feff}42", "_42", "(42)", "[42]", "\\42",
    ] {
        both_reject(s.as_bytes());
    }
}

// ===========================================================================
// Row 5 — endp == str: hex digits are not decimal digits
// ===========================================================================
#[test]
fn err_05_hex_digits_base10() {
    for s in [
        "ABC", "abc", "deadbeef", "DEADBEEF", "xFF", "XFF", "FF", "ff", "-ff", "+FF", "aE9",
    ] {
        both_reject(s.as_bytes());
    }
}

// ===========================================================================
// Row 6 — errno == ERANGE: positive overflow past LONG_MAX
// ===========================================================================
#[test]
fn err_06_erange_overflow() {
    both_reject(b"9223372036854775808"); // LONG_MAX + 1
    both_reject(b"9223372036854775809");
    both_reject(b"18446744073709551616"); // 2^64
    both_reject(b"99999999999999999999999999");
    let long_number: String = std::iter::repeat('9').take(400).collect();
    both_reject(long_number.as_bytes());
    let mut with_junk = long_number.clone();
    with_junk.push_str("abc");
    both_reject(with_junk.as_bytes());
    both_reject(format!("  +{long_number}").as_bytes());
}

// ===========================================================================
// Row 7 — errno == ERANGE: negative overflow past LONG_MIN
// ===========================================================================
#[test]
fn err_07_erange_underflow() {
    both_reject(b"-9223372036854775809"); // LONG_MIN - 1
    both_reject(b"-18446744073709551616");
    both_reject(b"-1000000000000000000000000000000");
    let long_number: String = std::iter::once('-')
        .chain(std::iter::repeat('9').take(400))
        .collect();
    both_reject(long_number.as_bytes());
}

// ===========================================================================
// Row 8 — tmp < INT_MIN with errno == 0
// ===========================================================================
#[test]
fn err_08_below_int_min() {
    both_reject(b"-2147483649"); // INT_MIN - 1
    both_reject(b"-2147483650");
    both_reject(b"-3000000000");
    both_reject(b"-4294967296");
    both_reject(b"-9223372036854775807");
    both_reject(b"-9223372036854775808"); // exactly LONG_MIN, errno == 0
    both_reject(b"-0000000000002147483649"); // leading zeros, still out of range
}

// ===========================================================================
// Row 9 — tmp > INT_MAX with errno == 0
// ===========================================================================
#[test]
fn err_09_above_int_max() {
    both_reject(b"2147483648"); // INT_MAX + 1
    both_reject(b"2147483649");
    both_reject(b"4294967295");
    both_reject(b"4294967296");
    both_reject(b"9223372036854775806");
    both_reject(b"9223372036854775807"); // exactly LONG_MAX, errno == 0
    both_reject(b"+2147483648");
    both_reject(b"  0000002147483648");
}

// ===========================================================================
// Row 10 — the range check applies to the parsed prefix, garbage or not
// ===========================================================================
#[test]
fn err_10_range_check_with_trailing_garbage() {
    both_reject(b"2147483648abc");
    both_reject(b"-2147483649xyz");
    both_reject(b"99999999999999999999 and more");
    both_accept(b"2147483647abc");
    both_accept(b"-2147483648xyz");
    // The boundary crossing, one step apart, with identical garbage.
    both_accept(b"2147483647!!!");
    both_reject(b"2147483648!!!");
}

// ===========================================================================
// Row 11 — the boundaries themselves must NOT be rejected
// ===========================================================================
#[test]
fn err_11_boundaries_accepted() {
    for s in ["2147483647", "-2147483648", "+2147483647", "  2147483647"] {
        both_accept(s.as_bytes());
        let c = c_impl().driver_bytes(s.as_bytes());
        assert_eq!(
            c.iter().filter(|&&b| b == b'\n').count(),
            8,
            "expected 8 output lines for {s:?}"
        );
    }
}

// ===========================================================================
// Row 13 — NUL-terminator boundary: bytes after the terminator are invisible
// ===========================================================================
#[test]
fn err_13_nul_terminator_boundary() {
    // "\0abc" -> the visible string is empty -> rejected by both.
    let c = c_impl().driver_raw(b"\0abc\0");
    let r = rust_impl().driver_raw(b"\0abc\0");
    assert_eq!(c, r, "stdout mismatch for \"\\0abc\"");
    assert_eq!(c, SENTINEL.to_vec(), "expected rejection for \"\\0abc\"");

    // "42\0abc" -> the visible string is "42" -> accepted identically.
    let c = c_impl().driver_raw(b"42\0abc\0");
    let r = rust_impl().driver_raw(b"42\0abc\0");
    assert_eq!(c, r, "stdout mismatch for \"42\\0abc\"");
    assert_ne!(c, SENTINEL.to_vec());

    // A lone NUL.
    let c = c_impl().driver_raw(b"\0");
    let r = rust_impl().driver_raw(b"\0");
    assert_eq!(c, r);
    assert_eq!(c, SENTINEL.to_vec());
}

// ===========================================================================
// Row 15 — extra_bedrooms at the int extremes is not validated
// ===========================================================================
#[test]
fn err_15_int_extremes_not_rejected() {
    for &extra in &[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -1, 0, 1] {
        diff_run(DEFAULT_HOUSE, extra);
        diff_run(house_t::new(i32::MAX, i32::MAX, 2.5), extra);
        diff_run(house_t::new(i32::MIN, i32::MIN, 2.5), extra);
    }
}

// ===========================================================================
// Row 16 — floors == INT_MAX overflows without any check
// ===========================================================================
#[test]
fn err_16_floors_overflow_not_rejected() {
    diff_run(house_t::new(i32::MAX, 5, 2.5), 0);
    diff_run(house_t::new(i32::MAX, 5, 2.5), 1);
    diff_run_n(house_t::new(i32::MAX - 2, 5, 2.5), 0, 8);
    diff_run_n(house_t::new(i32::MIN, 5, 2.5), 0, 8);
}

// ===========================================================================
// Row 17 — no enum parameters exist: every int bit pattern is a valid input
// ===========================================================================
#[test]
fn err_17_no_enum_domain_all_int_bitpatterns_valid() {
    // Explicit "out of range enum value" style bit patterns.
    for &bits in &[
        0x0000_0000u32,
        0x0000_0001,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFF,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x0000_00FF,
        0x0000_FFFF,
    ] {
        let extra = bits as i32;
        diff_run(DEFAULT_HOUSE, extra);
        diff_run(house_t::new(bits as i32, bits as i32, 2.5), extra);
    }
    let mut rng = Rng::new(117);
    for _ in 0..512 {
        diff_run(DEFAULT_HOUSE, rng.next_i32());
    }
}

// ===========================================================================
// Rows 12 & 14 — unchecked NULL pointers: both must die the same way.
//
// The C code has no NULL guard, so the call segfaults. Each implementation is
// therefore exercised in a SEPARATE CHILD PROCESS (a re-exec of this test
// binary with `DRIVER_CRASH_CASE` set) and the termination signals compared.
// ===========================================================================

const CRASH_ENV: &str = "DRIVER_CRASH_CASE";

fn crash_child_if_requested() -> bool {
    let Ok(case) = std::env::var(CRASH_ENV) else {
        return false;
    };
    let (which, what) = case.split_once(':').expect("malformed DRIVER_CRASH_CASE");
    let imp = match which {
        "c" => c_impl(),
        "rust" => rust_impl(),
        other => panic!("unknown impl {other}"),
    };
    match what {
        "driver_null" => imp.driver_uncaptured_ptr(std::ptr::null()),
        "run_null" => imp.run_uncaptured_ptr(std::ptr::null_mut(), 1),
        other => panic!("unknown crash case {other}"),
    }
    // If we get here the call did NOT crash; report that distinctly.
    std::process::exit(66);
}

/// Runs `case` in a child and returns (signal, exit_code) as printable text.
fn child_outcome(test_name: &str, case: &str) -> String {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .arg("--exact")
        .arg(test_name)
        .arg("--test-threads=1")
        .arg("--nocapture")
        .env(CRASH_ENV, case)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn crash child");
    match out.status.signal() {
        Some(sig) => format!("signal {sig}"),
        None => format!("exit {}", out.status.code().unwrap_or(-1)),
    }
}

#[test]
fn err_12_null_pointer_crashes_identically() {
    if crash_child_if_requested() {
        return;
    }
    let name = "err_12_null_pointer_crashes_identically";
    let c = child_outcome(name, "c:driver_null");
    let r = child_outcome(name, "rust:driver_null");
    assert_eq!(
        c, r,
        "driver(NULL) must terminate identically: C = {c}, Rust = {r}"
    );
    assert_eq!(c, "signal 11", "expected SIGSEGV from driver(NULL), got {c}");
}

#[test]
fn err_14_run_null_house_crashes_identically() {
    if crash_child_if_requested() {
        return;
    }
    let name = "err_14_run_null_house_crashes_identically";
    let c = child_outcome(name, "c:run_null");
    let r = child_outcome(name, "rust:run_null");
    assert_eq!(
        c, r,
        "run(NULL, 1) must terminate identically: C = {c}, Rust = {r}"
    );
    assert_eq!(c, "signal 11", "expected SIGSEGV from run(NULL, 1), got {c}");
}

// ===========================================================================
// Generic boundaries required by the task beyond the table
// ===========================================================================

#[test]
fn err_generic_zero_and_oversized_lengths() {
    both_reject(b""); // zero length
    // Oversized: a 100 KiB input. The digit prefix decides the outcome.
    let big_digits: String = std::iter::repeat('7').take(100_000).collect();
    both_reject(big_digits.as_bytes()); // ERANGE
    let mut big_junk: String = String::from("42");
    big_junk.extend(std::iter::repeat('z').take(100_000));
    both_accept(big_junk.as_bytes()); // trailing garbage is ignored
    let big_ws: String = std::iter::repeat(' ').take(100_000).collect();
    both_reject(big_ws.as_bytes()); // whitespace only
    let mut ws_then_val: String = std::iter::repeat(' ').take(100_000).collect();
    ws_then_val.push_str("-13");
    both_accept(ws_then_val.as_bytes());
}

#[test]
fn err_generic_one_past_valid_range_each_direction() {
    // One step past each end of the documented valid range, both directions.
    both_accept(b"2147483647");
    both_reject(b"2147483648");
    both_accept(b"-2147483648");
    both_reject(b"-2147483649");
    // And one step past the long range, where the failing term changes from
    // the range check to the errno check.
    both_reject(b"9223372036854775807"); // in long, out of int
    both_reject(b"9223372036854775808"); // ERANGE
    both_reject(b"-9223372036854775808");
    both_reject(b"-9223372036854775809");
}

#[test]
fn err_generic_every_single_byte_input() {
    // Exhaustive single-byte inputs (0x01..=0xFF): every one-character string
    // the API can receive, accept or reject.
    for b in 1u8..=0xff {
        let input = [b];
        let c = c_impl().driver_bytes(&input);
        let r = rust_impl().driver_bytes(&input);
        assert_eq!(c, r, "single-byte input {b:#04x} diverged");
    }
}

#[test]
fn err_generic_exhaustive_two_byte_ascii() {
    // Exhaustive two-byte inputs over the interesting ASCII subset.
    const SET: &[u8] = b"0123456789+- \t\nabxX.eE/:";
    for &a in SET {
        for &b in SET {
            let input = [a, b];
            let c = c_impl().driver_bytes(&input);
            let r = rust_impl().driver_bytes(&input);
            assert_eq!(
                c,
                r,
                "two-byte input {:?} diverged",
                String::from_utf8_lossy(&input)
            );
        }
    }
}
