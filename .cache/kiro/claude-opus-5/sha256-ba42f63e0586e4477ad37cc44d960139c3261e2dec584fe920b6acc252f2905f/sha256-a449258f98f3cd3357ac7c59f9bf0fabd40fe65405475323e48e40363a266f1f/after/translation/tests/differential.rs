//! Differential tests: run the C program and the Rust program as subprocesses on the
//! same stdin and require byte-identical stdout, byte-identical stderr and the same
//! exit status.
//!
//! # What the C program branches on
//!
//! ```c
//! int main() {
//!     int x = 0;
//!     scanf("%d", &x);
//!     if (x) { good(); } else { bad(); }
//!     return 0;
//! }
//! ```
//!
//! There is exactly one branch in `main`, but the input classes that select it come
//! from `scanf("%d", &x)`, which has many distinct outcomes:
//!
//! 1. EOF before any conversion — `x` keeps its initializer `0` → `bad()`.
//! 2. Whitespace-only input (`%d` skips whitespace, including newlines, then hits
//!    EOF) → `bad()`.
//! 3. Matching failure: the first non-whitespace byte cannot start an integer, or a
//!    sign is not followed by a digit — `x` is left untouched at `0` → `bad()`.
//! 4. Successful conversion of a zero value (`0`, `-0`, `+0`, a run of zeros) →
//!    `bad()`.
//! 5. Successful conversion of a nonzero value → `good()`.
//! 6. Successful conversion whose `long` result *truncates to zero* when stored into
//!    the `int` argument (any nonzero multiple of 2^32) → `bad()`. This is the
//!    counter-intuitive class: the text is nonzero but the branch is the `bad()` one.
//! 7. Out-of-range input: glibc converts `%d` through `strtol`, which saturates at
//!    `LONG_MAX` / `LONG_MIN`; the saturated value is then truncated to `int`.
//!    `LONG_MAX as int` is `-1` (nonzero → `good()`), while `LONG_MIN as int` is `0`
//!    (→ `bad()`).
//!
//! `bad()` reads an uninitialized `int *`, so it is the program's error/undefined path;
//! it is exercised by classes 1, 2, 3, 4, 6 and the negative half of 7.

mod harness;

use harness::{
    assert_same, assert_same_and_stdout, assert_same_devnull, c_bin, rust_bin, BAD_STDOUT,
    GOOD_STDOUT,
};

// ---------------------------------------------------------------------------
// Phase A: both programs exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    assert!(r.is_file(), "Rust binary missing at {}", r.display());
    // A trivial run of each, to prove they execute at all before anything is compared.
    assert_same("smoke", b"1\n");
}

// ---------------------------------------------------------------------------
// Class 1 & 2: no conversion happens at all -> bad()
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    assert_same_and_stdout("empty", b"", BAD_STDOUT);
}

#[test]
fn stdin_is_dev_null() {
    // Not a pipe: already at EOF when the first read happens.
    assert_same_devnull("dev_null");
}

#[test]
fn whitespace_only_inputs() {
    // Every byte C's isspace() accepts in the C locale, alone and combined.
    for (name, input) in [
        ("space", &b" "[..]),
        ("newline", b"\n"),
        ("many_newlines", b"\n\n\n\n"),
        ("tab", b"\t\t\t"),
        ("carriage_return", b"\r\r\r"),
        ("vertical_tab", b"\x0b"),
        ("form_feed", b"\x0c"),
        ("all_space_bytes", b" \t\n\x0b\x0c\r"),
        ("space_run", b"                                "),
    ] {
        assert_same_and_stdout(name, input, BAD_STDOUT);
    }
}

// ---------------------------------------------------------------------------
// Class 3: matching failure -> x untouched -> bad()
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_non_numeric() {
    for (name, input) in [
        ("letters", &b"abc"[..]),
        ("single_letter", b"x"),
        ("dot", b"."),
        ("comma", b","),
        ("underscore", b"_"),
        ("letters_then_digits", b"abc123"),
        ("long_letters", b"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"),
        ("leading_ws_then_letters", b"   \n\t abc"),
    ] {
        assert_same_and_stdout(name, input, BAD_STDOUT);
    }
}

#[test]
fn matching_failure_sign_without_digits() {
    for (name, input) in [
        ("plus_only", &b"+"[..]),
        ("minus_only", b"-"),
        ("plus_then_eof_after_ws", b"   +"),
        ("plus_then_space", b"+ 5"),
        ("minus_then_space", b"- 5"),
        ("minus_then_newline", b"-\n5"),
        ("double_minus", b"--5"),
        ("double_plus", b"++5"),
        ("plus_minus", b"+-5"),
        ("minus_then_letter", b"-a"),
        ("plus_then_dot", b"+."),
    ] {
        assert_same_and_stdout(name, input, BAD_STDOUT);
    }
}

#[test]
fn matching_failure_non_ascii_and_control_bytes() {
    for (name, input) in [
        ("nul_byte", &b"\x00"[..]),
        ("nul_then_digit", b"\x00 5"),
        ("control_bytes", b"\x01\x02\x03"),
        ("high_byte", b"\xff"),
        ("utf8_two_byte", "ø".as_bytes()),
        ("utf8_then_digit", "ø5".as_bytes()),
        ("invalid_utf8", b"\x80\x81\x82"),
        ("bom", b"\xef\xbb\xbf5"),
    ] {
        assert_same_and_stdout(name, input, BAD_STDOUT);
    }
}

// ---------------------------------------------------------------------------
// Class 4: converts to zero -> bad()
// ---------------------------------------------------------------------------

#[test]
fn zero_values_take_the_bad_branch() {
    for (name, input) in [
        ("zero", &b"0"[..]),
        ("zero_newline", b"0\n"),
        ("negative_zero", b"-0"),
        ("plus_zero", b"+0"),
        ("zero_run", b"000000"),
        ("zero_then_letter", b"0abc"),
        ("zero_then_space_five", b"0 5"),
        ("zero_then_newline_five", b"0\n5\n"),
        ("leading_ws_zero", b"   \n\n  0"),
        ("negative_zero_run", b"-0000"),
    ] {
        assert_same_and_stdout(name, input, BAD_STDOUT);
    }
}

// ---------------------------------------------------------------------------
// Class 5: converts to nonzero -> good()
// ---------------------------------------------------------------------------

#[test]
fn nonzero_values_take_the_good_branch() {
    for (name, input) in [
        ("one", &b"1"[..]),
        ("one_newline", b"1\n"),
        ("five", b"5\n"),
        ("plus_seven", b"+7"),
        ("negative_three", b"-3"),
        ("leading_zeros", b"007"),
        ("many_leading_zeros_then_one", b"0000000000000000001"),
        ("digits_then_letters", b"7abc"),
        ("digits_then_space_junk", b"5 junk\n"),
        ("digits_then_dot", b"5.5"),
        ("hex_prefix_stops_at_x", b"0x10"), // %d is base 10: converts 0, stops at 'x'
        ("binary_prefix_stops_at_b", b"0b1"),
        ("leading_ws_across_newlines", b"   \n\n  7\n"),
        ("all_space_bytes_then_digit", b" \t\n\x0b\x0c\r9"),
        ("no_trailing_newline", b"42"),
        ("crlf", b"42\r\n"),
    ] {
        let expected = if input == b"0x10" || input == b"0b1" {
            // Both convert the leading `0`, so these are actually zero -> bad().
            BAD_STDOUT
        } else {
            GOOD_STDOUT
        };
        assert_same_and_stdout(name, input, expected);
    }
}

// ---------------------------------------------------------------------------
// Class 6: nonzero text that truncates to a zero int -> bad()
// ---------------------------------------------------------------------------

#[test]
fn long_to_int_truncation_to_zero_takes_the_bad_branch() {
    for (name, input) in [
        ("two_pow_32", &b"4294967296"[..]),
        ("two_pow_33", b"8589934592"),
        ("negative_two_pow_32", b"-4294967296"),
        ("large_multiple_of_two_pow_32", b"9223372032559808512"),
        ("negative_large_multiple", b"-9223372032559808512"),
        ("two_pow_32_times_three", b"12884901888"),
    ] {
        assert_same_and_stdout(name, input, BAD_STDOUT);
    }
}

#[test]
fn long_to_int_truncation_to_nonzero_takes_the_good_branch() {
    for (name, input) in [
        ("two_pow_32_plus_one", &b"4294967297"[..]),
        ("u32_max", b"4294967295"),
        ("negative_u32_max", b"-4294967295"),
        ("int_max_plus_one_as_long", b"2147483648"),
        ("int_min_minus_one_as_long", b"-2147483649"),
    ] {
        assert_same_and_stdout(name, input, GOOD_STDOUT);
    }
}

// ---------------------------------------------------------------------------
// Boundaries and class 7: strtol saturation, then truncation
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    for (name, input, expected) in [
        ("int_max", &b"2147483647"[..], GOOD_STDOUT),
        ("int_min", b"-2147483648", GOOD_STDOUT),
        ("int_max_minus_one", b"2147483646", GOOD_STDOUT),
        ("int_min_plus_one", b"-2147483647", GOOD_STDOUT),
    ] {
        assert_same_and_stdout(name, input, expected);
    }
}

#[test]
fn long_boundaries_and_overflow_saturation() {
    for (name, input, expected) in [
        // LONG_MAX itself: truncates to -1, nonzero -> good().
        ("long_max", &b"9223372036854775807"[..], GOOD_STDOUT),
        // Above LONG_MAX: saturates to LONG_MAX -> -1 -> good().
        ("long_max_plus_one", b"9223372036854775808", GOOD_STDOUT),
        ("two_pow_64", b"18446744073709551616", GOOD_STDOUT),
        ("twenty_three_nines", b"99999999999999999999999", GOOD_STDOUT),
        // Below LONG_MIN: saturates to LONG_MIN, which truncates to 0 -> bad().
        ("long_min", b"-9223372036854775808", BAD_STDOUT),
        ("long_min_minus_one", b"-9223372036854775809", BAD_STDOUT),
        ("negative_two_pow_64", b"-18446744073709551616", BAD_STDOUT),
        ("negative_many_nines", b"-99999999999999999999999", BAD_STDOUT),
    ] {
        assert_same_and_stdout(name, input, expected);
    }
}

#[test]
fn very_long_digit_runs() {
    // Long enough to push glibc's scanf past its internal buffer boundaries.
    let nines = vec![b'9'; 10_000];
    assert_same_and_stdout("ten_thousand_nines", &nines, GOOD_STDOUT);

    let zeros = vec![b'0'; 10_000];
    assert_same_and_stdout("ten_thousand_zeros", &zeros, BAD_STDOUT);

    let mut zeros_then_one = vec![b'0'; 5_000];
    zeros_then_one.push(b'1');
    assert_same_and_stdout("zeros_then_one", &zeros_then_one, GOOD_STDOUT);

    let mut minus_nines = vec![b'-'];
    minus_nines.extend(std::iter::repeat(b'9').take(10_000));
    assert_same_and_stdout("minus_ten_thousand_nines", &minus_nines, BAD_STDOUT);

    // Non-numeric input longer than any plausible buffer.
    let junk = vec![b'q'; 65_536];
    assert_same_and_stdout("sixty_four_k_of_junk", &junk, BAD_STDOUT);

    // Whitespace longer than any plausible buffer, then a digit: %d must skip all of it.
    let mut ws_then_digit = vec![b'\n'; 65_536];
    ws_then_digit.push(b'7');
    assert_same_and_stdout("sixty_four_k_of_newlines_then_digit", &ws_then_digit, GOOD_STDOUT);
}

#[test]
fn buffer_boundary_sizes() {
    // Digit runs sized around common buffer lengths, in both the zero and nonzero forms.
    for n in [1usize, 7, 63, 64, 65, 511, 512, 1023, 1024, 4095, 4096, 4097] {
        let zeros = vec![b'0'; n];
        assert_same_and_stdout(&format!("zeros_{n}"), &zeros, BAD_STDOUT);

        let junk = vec![b'q'; n];
        assert_same_and_stdout(&format!("junk_{n}"), &junk, BAD_STDOUT);

        let mut padded_one = vec![b'0'; n];
        padded_one.push(b'1');
        assert_same_and_stdout(&format!("zeros_{n}_then_one"), &padded_one, GOOD_STDOUT);
    }
}

// ---------------------------------------------------------------------------
// Only the first conversion matters: everything after it is never read.
// ---------------------------------------------------------------------------

#[test]
fn trailing_input_is_ignored() {
    for (name, input, expected) in [
        ("zero_then_lots", &b"0\n1\n2\n3\n4\n5\n"[..], BAD_STDOUT),
        ("five_then_lots", b"5\n0\n0\n0\n", GOOD_STDOUT),
        ("zero_then_junk", b"0 the rest is ignored", BAD_STDOUT),
        ("junk_then_number", b"abc 5", BAD_STDOUT),
    ] {
        assert_same_and_stdout(name, input, expected);
    }
}

// ---------------------------------------------------------------------------
// Repeat the two branches to confirm they are deterministic run to run. `bad()`
// reads an uninitialized pointer, so a nondeterministic result there would mean
// the whole comparison is unstable and must be reported rather than papered over.
// ---------------------------------------------------------------------------

#[test]
fn both_branches_are_deterministic_across_repeated_runs() {
    for _ in 0..25 {
        assert_same_and_stdout("bad_repeat", b"0\n", BAD_STDOUT);
        assert_same_and_stdout("good_repeat", b"1\n", GOOD_STDOUT);
    }
}

#[test]
fn no_output_is_written_to_stderr_on_either_branch() {
    // Pinned explicitly: `assert_same` already compares stderr, but the C program
    // never writes to stderr, and a Rust panic message would show up there.
    for input in [&b""[..], b"0", b"1", b"abc", b"-9223372036854775808"] {
        assert_same("stderr_check", input);
    }
}
