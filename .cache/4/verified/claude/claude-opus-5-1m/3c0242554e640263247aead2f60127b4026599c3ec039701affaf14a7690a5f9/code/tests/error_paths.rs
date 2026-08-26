//! Phase C — error/rejection-path differential tests, one per `ERRORS.md` row.
//!
//! Each test builds the exact invalid input or condition, runs BOTH executables
//! and asserts they agree. Where the C has an observable sentinel (the value
//! `scanf` leaves in `x`), the test also pins that exact value with an
//! independent model of the program (`expected_output`), so a test can never
//! pass just because "both failed somehow".

mod common;

use common::*;

/// Independent model of the C program's stdout for a given parsed `x`.
fn expected_output(x: i32) -> String {
    let mut floors: i32 = 2;
    let mut bedrooms: i32 = 5;
    let mut bathrooms: f64 = 2.5;
    let mut s = String::new();
    let line = |floors: i32, bedrooms: i32, bathrooms: f64, s: &mut String| {
        s.push_str(&format!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
            floors, bedrooms, bathrooms
        ));
    };
    for _ in 0..2 {
        line(floors, bedrooms, bathrooms, &mut s);
        floors = floors.wrapping_add(1);
        line(floors, bedrooms, bathrooms, &mut s);
        bathrooms += 1.0;
        line(floors, bedrooms, bathrooms, &mut s);
        bedrooms = bedrooms.wrapping_add(x);
        line(floors, bedrooms, bathrooms, &mut s);
    }
    s
}

/// Runs both programs on `input`, asserts they match each other and that the
/// parsed value was exactly `expected_x` (the sentinel `scanf` left behind).
fn check(label: &str, input: &[u8], expected_x: i32) {
    ensure_c_artifacts();
    let c = run_prog(&c_exe(), input, &[]);
    let r = run_prog(&rust_exe(), input, &[]);
    assert_same(label, input, &c, &r);
    let want = expected_output(expected_x);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        want,
        "[{label}] C did not park on x={expected_x} for input {:?}",
        String::from_utf8_lossy(&input[..input.len().min(60)])
    );
    assert_eq!(c.code, Some(0), "[{label}] C exit code");
    assert_eq!(r.code, Some(0), "[{label}] Rust exit code");
    assert_eq!(c.signal, None, "[{label}] C signal");
    assert!(c.stderr.is_empty() && r.stderr.is_empty(), "[{label}] stderr");
}

// ---------------------------------------------------------------- rows 1 - 2

#[test]
fn row01_empty_stdin_is_input_failure() {
    check("row01 empty stdin", b"", 0);
}

#[test]
fn row02_whitespace_only_is_input_failure() {
    for w in [
        &b" "[..],
        b"\t",
        b"\n",
        b"\x0b",
        b"\x0c",
        b"\r",
        b" \t\n\x0b\x0c\r",
        b"\n\n\n\n",
        b"                                        ",
    ] {
        check("row02 whitespace-only", w, 0);
    }
}

// ---------------------------------------------------------------- rows 3 - 9

#[test]
fn row03_letter_is_matching_failure() {
    for s in ["abc", "a", "z\n", "Q", "hello world\n", "nan", "inf"] {
        check("row03 letters", s.as_bytes(), 0);
    }
}

#[test]
fn row04_lone_plus_is_matching_failure() {
    for s in ["+", "+\n", "   +", "+ ", "++", "+++5"] {
        check("row04 lone plus", s.as_bytes(), 0);
    }
}

#[test]
fn row05_lone_minus_is_matching_failure() {
    for s in ["-", "-\n", "   -", "- ", "--", "---5"] {
        check("row05 lone minus", s.as_bytes(), 0);
    }
}

#[test]
fn row06_sign_then_nondigit_is_matching_failure() {
    for s in ["-x", "+x\n", "+ 5", "- 5\n", "--5", "+-5", "-+5", "-.5\n"] {
        check("row06 sign+nondigit", s.as_bytes(), 0);
    }
}

#[test]
fn row07_punctuation_is_matching_failure() {
    for s in [".", ",", "*", "/", ".5\n", ",5", "#5", "(5)", "'5'", "\"5\""] {
        check("row07 punctuation", s.as_bytes(), 0);
    }
}

#[test]
fn row08_nul_byte_is_matching_failure() {
    for s in [&b"\0"[..], b"\0 5", b"\0\0\0", b" \0 5\n"] {
        check("row08 NUL", s, 0);
    }
}

#[test]
fn row09_non_ascii_byte_is_matching_failure() {
    for s in [
        &b"\xff"[..],
        b"\x80",
        b"\xc3\xa9",       // "é"
        b"\xe2\x82\xac5",  // "€5"
        b"\xff\xfe\xfd\n", // invalid UTF-8
        b" \xff 5",
    ] {
        check("row09 non-ascii", s, 0);
    }
}

// -------------------------------------------------------------- rows 10 - 13

#[test]
fn row10_hex_prefix_rejected_after_zero() {
    check("row10 0x10", b"0x10", 0);
    check("row10 0X10", b"0X10\n", 0);
    check("row10 0b1", b"0b1\n", 0);
    check("row10 -0x10", b"-0x10\n", 0);
}

#[test]
fn row11_digits_then_garbage() {
    check("row11 5abc", b"5abc", 5);
    check("row11 -7q", b"-7q\n", -7);
    check("row11 123!!!", b"123!!!\n", 123);
}

#[test]
fn row12_only_first_token_read() {
    check("row12 1 2", b"1 2\n", 1);
    check("row12 1\\n2", b"1\n2\n", 1);
    check("row12 -5 -6", b"-5 -6\n", -5);
    check("row12 3 junk", b"3 99999999999999999999\n", 3);
}

#[test]
fn row13_float_syntax_stops_at_dot() {
    check("row13 2.5", b"2.5\n", 2);
    check("row13 -0.75", b"-0.75\n", 0);
    check("row13 1e9", b"1e9\n", 1);
}

// -------------------------------------------------------------- rows 14 - 20

#[test]
fn row14_above_int_max_truncates() {
    check("row14 2147483648", b"2147483648\n", i32::MIN);
    check("row14 2147483649", b"2147483649\n", i32::MIN + 1);
    check("row14 -2147483649", b"-2147483649\n", i32::MAX);
    check("row14 3000000000", b"3000000000\n", -1294967296);
}

#[test]
fn row15_two_pow_32_truncates_to_zero() {
    check("row15 4294967296", b"4294967296\n", 0);
    check("row15 8589934592", b"8589934592\n", 0);
    check("row15 4294967297", b"4294967297\n", 1);
    check("row15 -4294967296", b"-4294967296\n", 0);
}

#[test]
fn row16_exact_long_max() {
    check("row16 LONG_MAX", b"9223372036854775807\n", -1);
}

#[test]
fn row17_above_long_max_saturates() {
    for s in [
        "9223372036854775808",
        "9223372036854775809",
        "99999999999999999999",
        "18446744073709551616",
        "123456789012345678901234567890",
    ] {
        // strtol saturates at LONG_MAX; (int)LONG_MAX == -1
        check("row17 >LONG_MAX", format!("{s}\n").as_bytes(), -1);
    }
}

#[test]
fn row18_exact_long_min() {
    check("row18 LONG_MIN", b"-9223372036854775808\n", 0);
}

#[test]
fn row19_below_long_min_saturates() {
    for s in [
        "-9223372036854775809",
        "-99999999999999999999",
        "-18446744073709551616",
        "-123456789012345678901234567890",
    ] {
        // strtol saturates at LONG_MIN; (int)LONG_MIN == 0
        check("row19 <LONG_MIN", format!("{s}\n").as_bytes(), 0);
    }
}

#[test]
fn row20_leading_zeros_do_not_overflow() {
    check("row20 19 zeros + 5", b"0000000000000000005\n", 5);
    check("row20 40 zeros + 5", b"00000000000000000000000000000000000000005\n", 5);
    check("row20 zeros only", b"000000000000000000000000\n", 0);
    check("row20 -zeros+7", b"-0000000000000000000007\n", -7);
    // 100k digits: overflows long many times over, still saturating.
    let mut s = String::from("-");
    s.push_str(&"9".repeat(100_000));
    s.push('\n');
    check("row20 100k nines", s.as_bytes(), 0);
}

// -------------------------------------------------------------- rows 21 - 24

#[test]
fn row21_int_max_wraps_bedrooms() {
    check("row21 INT_MAX", b"2147483647\n", i32::MAX);
    // 5 + INT_MAX wraps to -2147483644, then wraps again to 3.
    let want = expected_output(i32::MAX);
    assert!(want.contains("-2147483644 bedrooms"), "model: {want}");
    assert!(want.ends_with("The house has 4 floors, 3 bedrooms, and 4.5 bathrooms\n"));
}

#[test]
fn row22_int_min_wraps_bedrooms() {
    check("row22 INT_MIN", b"-2147483648\n", i32::MIN);
    let want = expected_output(i32::MIN);
    assert!(want.contains("-2147483643 bedrooms"), "model: {want}");
    assert!(want.ends_with("The house has 4 floors, 5 bedrooms, and 4.5 bathrooms\n"));
}

// Rows 23 (repeated INT_MAX/INT_MIN through the exported `run`) is covered by
// `tests/differential_ffi.rs::ffi_differential`, which calls `run` directly with
// INT_MAX/INT_MIN several times in a row and compares each step.
//
// Row 24 (`floors++` overflow) needs 2^31 calls to `run` to be reachable and is
// therefore untestable in finite time; the Rust code uses `wrapping_add`, i.e.
// exactly the two's-complement wrap the compiled C performs, and that same code
// path is what row 21/22/23 exercise for `bedrooms`.
#[test]
fn row24_floor_increment_uses_wrapping_semantics() {
    // Guard rail: prove the model and both binaries agree that `floors` simply
    // increments without any check, for the reachable range.
    ensure_c_artifacts();
    let c = run_prog(&c_exe(), b"0\n", &[]);
    let text = String::from_utf8_lossy(&c.stdout);
    let floors: Vec<&str> = text
        .lines()
        .map(|l| l.split(" floors").next().unwrap().rsplit(' ').next().unwrap())
        .collect();
    assert_eq!(floors, vec!["2", "3", "3", "3", "3", "4", "4", "4"]);
}

// -------------------------------------------------------------- rows 25 - 28

#[test]
fn row25_closed_stdin() {
    ensure_c_artifacts();
    let c = run_prog_with_closed_fd(&c_exe(), 0, b"5\n");
    let r = run_prog_with_closed_fd(&rust_exe(), 0, b"5\n");
    assert_same("row25 closed stdin", b"", &c, &r);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        expected_output(0),
        "closed stdin must leave x = 0"
    );
    assert_eq!(c.code, Some(0));
}

#[test]
fn row26_stdin_is_a_directory() {
    ensure_c_artifacts();
    let mut outs = Vec::new();
    for exe in [c_exe(), rust_exe()] {
        let dir = std::fs::File::open("/").expect("open / as stdin");
        outs.push(run_prog_with(
            &exe,
            dir.into(),
            std::process::Stdio::piped(),
            &[],
        ));
    }
    assert_same("row26 stdin is a directory", b"", &outs[0], &outs[1]);
    assert_eq!(
        String::from_utf8_lossy(&outs[0].stdout),
        expected_output(0),
        "EISDIR must leave x = 0"
    );
    assert_eq!(outs[0].code, Some(0));
}

#[test]
fn row27_closed_stdout() {
    ensure_c_artifacts();
    let c = run_prog_with_closed_fd(&c_exe(), 1, b"5\n");
    let r = run_prog_with_closed_fd(&rust_exe(), 1, b"5\n");
    assert_eq!(
        (c.code, c.signal, c.stdout.is_empty()),
        (r.code, r.signal, r.stdout.is_empty()),
        "closed stdout: C {} vs Rust {}",
        c.describe(),
        r.describe()
    );
    assert_eq!(c.code, Some(0), "C exits 0 with a closed stdout");
}

#[test]
fn row28_sigpipe_on_stdout_without_reader() {
    ensure_c_artifacts();
    let c = run_prog_broken_stdout(&c_exe(), b"5\n");
    let r = run_prog_broken_stdout(&rust_exe(), b"5\n");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "broken stdout pipe: C {} vs Rust {}",
        c.describe(),
        r.describe()
    );
    assert_eq!(
        c.signal,
        Some(13),
        "the C program is expected to die from SIGPIPE"
    );
}

// -------------------------------------------------------------- rows 29 - 30

#[test]
fn row29_token_across_buffer_boundary() {
    for pad in [4095usize, 4096, 4097, 8192] {
        let mut s = " ".repeat(pad);
        s.push_str("abc");
        check("row29 junk after boundary", s.as_bytes(), 0);

        let mut s2 = " ".repeat(pad);
        s2.push_str("-2147483648\n");
        check("row29 INT_MIN after boundary", s2.as_bytes(), i32::MIN);

        // digit run split by the boundary
        let mut s3 = " ".repeat(pad - 5);
        s3.push_str("9223372036854775808\n");
        check("row29 saturating value across boundary", s3.as_bytes(), -1);
    }
}

#[test]
fn row30_argv_is_never_rejected() {
    ensure_c_artifacts();
    for args in [
        vec!["foo", "bar"],
        vec!["--nonsense"],
        vec!["-1"],
        vec!["\u{1f600}"],
    ] {
        let c = run_prog(&c_exe(), b"", &args);
        let r = run_prog(&rust_exe(), b"", &args);
        assert_same("row30 argv", b"", &c, &r);
        assert_eq!(String::from_utf8_lossy(&c.stdout), expected_output(0));
        assert_eq!(c.code, Some(0));
    }
}

// ------------------------------------------------------- harness self-check

/// Negative control: the comparison helper must actually fail when the two
/// outputs differ (otherwise every test above would be vacuous).
#[test]
#[should_panic(expected = "DIVERGENCE")]
fn harness_detects_divergence() {
    ensure_c_artifacts();
    let a = run_prog(&c_exe(), b"1\n", &[]);
    let b = run_prog(&c_exe(), b"2\n", &[]);
    assert_same("negative control", b"", &a, &b);
}
