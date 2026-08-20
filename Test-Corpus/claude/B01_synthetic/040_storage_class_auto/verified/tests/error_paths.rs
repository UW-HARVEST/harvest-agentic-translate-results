//! Phase C — error-path differential tests, one `#[test]` per row of
//! `ERRORS.md`.
//!
//! `c_src/src/main.c` ignores the return value of both `scanf` and `printf`, so
//! "the same error" is observable as "the same stdout bytes and the same exit
//! status / termination signal". Each test therefore pins the exact ground-truth
//! bytes produced by the C library **and** asserts the Rust library matches.

mod common;

use common::{
    assert_driver_eq_expect, assert_main_eq_expect, call_main, driver_once, In, Out, Rng, Side,
};

// ---------------------------------------------------------------------------
// `main` / `scanf("%d", &x)` failure modes — x keeps its initial value 0
// ---------------------------------------------------------------------------

/// Row 1 — empty stdin: immediate EOF (input failure).
#[test]
fn row01_empty_stdin() {
    assert_main_eq_expect(b"", "300\n");
}

/// Row 2 — whitespace only, then EOF.
#[test]
fn row02_whitespace_only() {
    for input in [
        &b" "[..],
        b"\n",
        b"\t",
        b"\x0b",
        b"\x0c",
        b"\r",
        b"   \t\n\r\x0b\x0c   ",
        b"\n\n\n\n\n",
    ] {
        assert_main_eq_expect(input, "300\n");
    }
    // Whitespace-only input longer than one read chunk: the refill loop must
    // still end in an input failure rather than in a stale byte.
    for n in [4095usize, 4096, 4097, 9000] {
        assert_main_eq_expect(&vec![b' '; n], "300\n");
        assert_main_eq_expect(&vec![b'\n'; n], "300\n");
    }
}

/// Row 3 — first non-whitespace byte is not `[+-0-9]` (matching failure).
#[test]
fn row03_leading_non_numeric() {
    for input in [
        "abc", ".5", "x", "/", "*", "e", "!", "~", ",", "]", "\"", "'", " \n a", "A1", "z9",
    ] {
        assert_main_eq_expect(input.as_bytes(), "300\n");
    }
}

/// Row 4 — a sign followed by EOF.
#[test]
fn row04_sign_then_eof() {
    for input in ["+", "-", "  +", "\n-", "   \t-"] {
        assert_main_eq_expect(input.as_bytes(), "300\n");
    }
    // The sign is the last byte of / just past a read chunk.
    for n in [4094usize, 4095, 4096, 4097] {
        let mut input = vec![b' '; n];
        input.push(b'-');
        assert_main_eq_expect(&input, "300\n");
        let mut input = vec![b' '; n];
        input.extend_from_slice(b"+x");
        assert_main_eq_expect(&input, "300\n");
    }
}

/// Row 5 — a sign followed by a non-digit (matching failure).
#[test]
fn row05_sign_then_non_digit() {
    for input in [
        "+-3", "- 5", "+a", "++", "--", "-+7", "+ 1", "-\n2", "+.5", "-x10",
    ] {
        assert_main_eq_expect(input.as_bytes(), "300\n");
    }
}

/// Row 6 — fd 0 closed: the read fails (`EBADF`), so `scanf` reports an input
/// failure and `x` stays 0.
#[test]
fn row06_stdin_closed() {
    let c = call_main(Side::C, In::Closed, Out::File);
    let r = call_main(Side::Rust, In::Closed, Out::File);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        "300\n",
        "C ground truth with fd 0 closed"
    );
    assert_eq!(c.status, Some(0));
    assert_eq!(c, r, "closed stdin: C={c:?} Rust={r:?}");
}

/// Row 7 — non-ASCII / binary first byte.
#[test]
fn row07_binary_first_byte() {
    for input in [
        &b"\xff12"[..],
        b"\x0012",
        b"\x8012",
        b"\xc3\xa912",
        b"\x7f",
        b"\x01\x02\x03",
        b"\xff",
    ] {
        assert_main_eq_expect(input, "300\n");
    }
}

/// Row 8 — `%d` is decimal-only, so `"0x10"` converts just the leading `0`.
#[test]
fn row08_hex_prefix_stops_at_x() {
    assert_main_eq_expect(b"0x10", "300\n");
    assert_main_eq_expect(b"0X1F", "300\n");
    assert_main_eq_expect(b"-0x10", "300\n");
    assert_main_eq_expect(b"0b101", "300\n");
    assert_main_eq_expect(b"010", "320\n"); // no octal interpretation either
}

/// Row 9 — an exponent is not part of `%d`.
#[test]
fn row09_exponent_not_consumed() {
    assert_main_eq_expect(b"1e5", "302\n");
    assert_main_eq_expect(b"-2E3", "296\n");
}

/// Row 10 — digits above `LONG_MAX` saturate to `LONG_MAX`, truncating to `-1`.
#[test]
fn row10_above_long_max() {
    for input in [
        "9223372036854775808",
        "9223372036854775809",
        "99999999999999999999999",
        "18446744073709551615",
        "123456789012345678901234567890",
        "+9223372036854775808",
    ] {
        assert_main_eq_expect(input.as_bytes(), "298\n");
    }
}

/// Row 11 — digits below `LONG_MIN` saturate to `LONG_MIN`, truncating to `0`.
#[test]
fn row11_below_long_min() {
    for input in [
        "-9223372036854775809",
        "-99999999999999999999",
        "-18446744073709551615",
        "-123456789012345678901234567890",
    ] {
        assert_main_eq_expect(input.as_bytes(), "300\n");
    }
}

/// Row 12 — `INT_MAX < value <= LONG_MAX`: `long` → `int` truncation.
#[test]
fn row12_between_int_max_and_long_max() {
    assert_main_eq_expect(b"2147483648", "300\n");
    assert_main_eq_expect(b"2147483649", "302\n");
    assert_main_eq_expect(b"4294967296", "300\n");
    assert_main_eq_expect(b"4294967297", "302\n");
    assert_main_eq_expect(b"9223372036854775807", "298\n");
}

/// Row 13 — `LONG_MIN <= value < INT_MIN`: `long` → `int` truncation.
#[test]
fn row13_between_long_min_and_int_min() {
    assert_main_eq_expect(b"-2147483649", "298\n");
    assert_main_eq_expect(b"-2147483650", "296\n");
    assert_main_eq_expect(b"-4294967296", "300\n");
    assert_main_eq_expect(b"-4294967297", "298\n");
    assert_main_eq_expect(b"-9223372036854775808", "300\n");
}

/// Row 14 — a valid prefix followed by garbage: the rest is never read.
#[test]
fn row14_valid_prefix_then_garbage() {
    assert_main_eq_expect(b"12abc", "324\n");
    assert_main_eq_expect(b"1 2", "302\n");
    assert_main_eq_expect(b"5)", "310\n");
    assert_main_eq_expect(b"-3.5", "294\n");
    assert_main_eq_expect(b"42\n99", "384\n");
}

/// Row 15 — an over-long, `LONG_MAX`-overflowing digit run that spans more than
/// one stdio buffer.
#[test]
fn row15_overflowing_digit_run_across_buffers() {
    for n in [4095usize, 4096, 4097, 8192, 10_000] {
        assert_main_eq_expect(&vec![b'9'; n], "298\n");
    }
}

// ---------------------------------------------------------------------------
// `driver` — unchecked signed overflow and output failures
// ---------------------------------------------------------------------------

/// Row 16 — `x = INT_MAX`, `2*x` overflows.
#[test]
fn row16_driver_int_max() {
    assert_driver_eq_expect(i32::MAX, "298\n");
}

/// Row 17 — `x = INT_MIN`, `2*x` overflows.
#[test]
fn row17_driver_int_min() {
    assert_driver_eq_expect(i32::MIN, "300\n");
}

/// Row 18 — `2*x` overflows for every `x >= 2^30`.
#[test]
fn row18_driver_double_overflow_threshold() {
    assert_driver_eq_expect(1_073_741_824, "-2147483348\n");
    assert_driver_eq_expect(1_073_741_825, "-2147483346\n");
    let mut rng = Rng::new(0xE770_0018);
    let xs: Vec<i32> = (0..64)
        .map(|_| rng.range(1_073_741_824, i32::MAX as i64) as i32)
        .collect();
    common::assert_driver_eq_all(&xs);
}

/// Row 19 — `2*x` underflows for every `x <= -2^30`.
#[test]
fn row19_driver_double_underflow_threshold() {
    assert_driver_eq_expect(-1_073_741_824, "-2147483348\n");
    assert_driver_eq_expect(-1_073_741_825, "-2147483350\n");
    let mut rng = Rng::new(0xE770_0019);
    let xs: Vec<i32> = (0..64)
        .map(|_| rng.range(i32::MIN as i64, -1_073_741_824) as i32)
        .collect();
    common::assert_driver_eq_all(&xs);
}

/// Row 20 — `2*x` fits but `y += 300` overflows.
#[test]
fn row20_driver_plus_300_overflow() {
    assert_driver_eq_expect(1_073_741_674, "-2147483648\n");
    assert_driver_eq_expect(1_073_741_823, "-2147483350\n");
    assert_driver_eq_expect(1_073_741_673, "2147483646\n");
}

/// Row 21 — `printf` fails with `EBADF` because fd 1 is closed; the ignored
/// return value means the call still completes with no output.
#[test]
fn row21_driver_closed_stdout() {
    for x in [0, 7, -7, i32::MAX, i32::MIN] {
        let c = driver_once(Side::C, x, Out::Closed);
        let r = driver_once(Side::Rust, x, Out::Closed);
        assert_eq!(c.status, Some(0), "C exit status with fd 1 closed");
        assert_eq!(c.signal, None);
        assert_eq!(c, r, "driver({x}) with fd 1 closed: C={c:?} Rust={r:?}");
    }
    let c = call_main(Side::C, In::Bytes(b"5"), Out::Closed);
    let r = call_main(Side::Rust, In::Bytes(b"5"), Out::Closed);
    assert_eq!(c.status, Some(0));
    assert_eq!(c, r, "main() with fd 1 closed: C={c:?} Rust={r:?}");
}

/// Row 22 — writing to a pipe with no reader raises `SIGPIPE`, whose default
/// disposition terminates the process.
#[test]
fn row22_broken_pipe_signal() {
    let c = driver_once(Side::C, 5, Out::BrokenPipe);
    let r = driver_once(Side::Rust, 5, Out::BrokenPipe);
    assert_eq!(
        c, r,
        "driver(5) writing to a broken pipe: C={c:?} Rust={r:?}"
    );
    let c = call_main(Side::C, In::Bytes(b"5"), Out::BrokenPipe);
    let r = call_main(Side::Rust, In::Bytes(b"5"), Out::BrokenPipe);
    assert_eq!(c, r, "main() writing to a broken pipe: C={c:?} Rust={r:?}");
}

/// Row 23 — raw 32-bit patterns crossing the FFI boundary as `int`. Every bit
/// pattern is a valid `int`, so none of them is rejected; this is the `int`
/// analogue of "an enum value with no valid variant" (`ERRORS.md` rows 24–25
/// record why the pointer/enum boundaries do not exist here).
#[test]
fn row23_ffi_out_of_range_bit_patterns() {
    assert_driver_eq_expect(0x8000_0000u32 as i32, "300\n");
    assert_driver_eq_expect(0xFFFF_FFFFu32 as i32, "298\n");
    let xs: Vec<i32> = [
        0x8000_0000u32,
        0x8000_0001,
        0xFFFF_FFFF,
        0xFFFF_FFFE,
        0x7FFF_FFFF,
        0xAAAA_AAAA,
        0x5555_5555,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x0000_0000,
    ]
    .iter()
    .map(|&p| p as i32)
    .collect();
    common::assert_driver_eq_all(&xs);
}

/// Rows 24–26 — the generic C-API boundaries that do **not** exist in this
/// library, asserted structurally so the claim cannot rot: neither exported
/// function takes a pointer, an enum, or a length.
#[test]
fn rows24to26_absent_boundaries_are_really_absent() {
    let src = std::fs::read_to_string(common::manifest_dir().join("c_src/src/main.c"))
        .expect("read C source");
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("void driver(int x)") && code.contains("int main()"),
        "the exported prototypes changed; re-derive ERRORS.md rows 24-26"
    );
    // No enum, no allocation, no assert, no length type, and no pointer
    // declarator (`2*x` is a multiplication, not a `*` declarator).
    for forbidden in [
        "enum", "size_t", "malloc", "assert", "* ", " *", "*)", "*,", "[]",
    ] {
        assert!(
            !code.contains(forbidden),
            "C source now contains `{forbidden}`; the null-pointer / enum / \
             length boundary is no longer vacuous and needs real test rows"
        );
    }
    // Zero-length input (row 26) is the only "length" analogue and is covered.
    assert_main_eq_expect(b"", "300\n");
}
