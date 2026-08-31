//! Differential tests: the C program in `c_src/` is the ground truth, and the
//! Rust binary must reproduce its stdout, stderr and exit status byte for byte.
//!
//! Both are executed as subprocesses. The Rust crate is never loaded as a
//! library.
//!
//! # The branches the C program actually has
//!
//! ```c
//! void driver(int x) { register int y = 2*x; y += 300; printf("%d\n", y); }
//! int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
//! ```
//!
//! There is no `if` in the C source, so the decision points live inside
//! `scanf("%d", &x)` and inside the signed arithmetic:
//!
//! 1. **Input failure** — EOF (or a read error) before any non-whitespace
//!    character. `scanf` returns `EOF` and leaves `x` at its initialiser `0`,
//!    so the program prints `300`.
//! 2. **Matching failure** — a non-numeric character, or a lone sign, where a
//!    digit was required. `scanf` returns `0` and again leaves `x` at `0`.
//! 3. **Successful conversion** — leading whitespace skipped (newlines
//!    included: `%d` reads *across* lines, unlike `fgets`), an optional `+`/`-`
//!    sign, then base-10 digits, stopping at the first non-digit.
//! 4. **Range error** — glibc converts through a `long`; on overflow it stores
//!    `LONG_MAX`/`LONG_MIN`, and `%d` then truncates that `long` to `int`.
//! 5. **Signed overflow in `driver`** — `2*x` and `y+300` wrap around two's
//!    complement as gcc/clang compile them.
//! 6. **argv is never read**, so extra arguments must change nothing.
//!
//! Every group below maps onto one of those.

mod common;

use common::{assert_same, assert_same_endless, assert_same_with_args};

// ---------------------------------------------------------------------------
// 1. Input failure: scanf sees EOF, x keeps its initial 0.
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    assert_same("empty stdin", b"");
}

#[test]
fn whitespace_only_input() {
    // Whitespace is consumed by the directive, then EOF arrives with no digits.
    for (label, input) in [
        ("single space", &b" "[..]),
        ("single newline", b"\n"),
        ("single tab", b"\t"),
        ("carriage return", b"\r"),
        ("vertical tab", b"\x0b"),
        ("form feed", b"\x0c"),
        ("every isspace char", b" \t\n\x0b\x0c\r"),
        ("many blank lines", b"\n\n\n\n"),
        ("mixed run", b"   \t \r\n  \n\t"),
    ] {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// 2. Matching failure: a digit was required and not found. x keeps 0.
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_leaves_x_at_zero() {
    for (label, input) in [
        ("letters", &b"abc"[..]),
        ("lone minus", b"-"),
        ("lone plus", b"+"),
        ("sign then letter", b"-a"),
        ("sign then newline", b"-\n"),
        ("plus then newline", b"+\n"),
        ("leading dot", b"."),
        ("decimal without integer part", b".5"),
        ("hex prefix is not %d", b"0x10"), // "0" converts; the rest is left
        ("underscore", b"_1"),
        ("comma", b",1"),
        ("NUL byte", b"\x00"),
        ("NUL then digits", b"\x001"),
        ("high byte", b"\xff"),
        ("whitespace then letters", b"   \n  zzz"),
        ("double sign", b"--1"),
        ("plus minus", b"+-1"),
        ("sign after digit-less junk", b"x-5"),
    ] {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// 3. Successful conversion: the happy path and its shape.
// ---------------------------------------------------------------------------

#[test]
fn single_item_no_trailing_newline() {
    // The smallest successful input, with no terminating newline at all.
    assert_same("bare 1, no newline", b"1");
}

#[test]
fn small_values() {
    for n in -20i32..=20 {
        assert_same("small value", n.to_string().as_bytes());
    }
}

#[test]
fn zero_prints_the_same_as_a_failed_scan() {
    // Worth pinning down: "0" and "" both print 300, but by different routes.
    assert_same("explicit zero", b"0");
    assert_same("negative zero", b"-0");
    assert_same("plus zero", b"+0");
    assert_same("padded zero", b"0000");
}

#[test]
fn explicit_sign_is_accepted() {
    for (label, input) in [
        ("plus seven", &b"+7"[..]),
        ("minus seven", b"-7"),
        ("plus with leading space", b"  +7"),
        ("minus with leading newline", b"\n-7"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    for (label, input) in [
        ("octal-looking 010", &b"010"[..]),
        ("long zero run", b"000000000000000000000000000123"),
        ("zeros then sign-free max", b"0000002147483647"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn scanf_reads_across_newlines_unlike_fgets() {
    // This is the distinguishing behaviour: a leading newline does NOT end the
    // read, so the number on the *second* line is what gets converted.
    for (label, input) in [
        ("newline then number", &b"\n42"[..]),
        ("blank lines then number", b"\n\n\n42\n"),
        ("crlf then number", b"\r\n42"),
        ("spaces and newlines", b"  \n \t \n  42  \n"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn conversion_stops_at_first_non_digit() {
    // Only one directive runs, so everything after the digit run is ignored.
    for (label, input) in [
        ("digits then letters", &b"12abc"[..]),
        ("digits then dot", b"12.75"),
        ("digits then exponent", b"12e5"),
        ("two numbers", b"3 4"),
        ("two numbers on two lines", b"3\n4\n"),
        ("number then NUL then number", b"3\x004"),
        ("number then sign", b"3-4"),
        ("many numbers", b"1 2 3 4 5 6 7 8 9 10\n"),
        ("trailing newline", b"7\n"),
        ("trailing spaces", b"7   "),
    ] {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// 4. Range errors: the maximum the code handles, and past it.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    for (label, input) in [
        ("INT_MAX", &b"2147483647"[..]),
        ("INT_MAX + 1", b"2147483648"),
        ("INT_MAX + 2", b"2147483649"),
        ("INT_MIN", b"-2147483648"),
        ("INT_MIN - 1", b"-2147483649"),
        ("INT_MIN - 2", b"-2147483650"),
        ("UINT_MAX", b"4294967295"),
        ("UINT_MAX + 1 truncates to 0", b"4294967296"),
        ("UINT_MAX + 2 truncates to 1", b"4294967297"),
        ("-(UINT_MAX + 2)", b"-4294967297"),
        ("2^31 + 2^32", b"6442450944"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn long_boundaries_saturate_then_truncate() {
    // glibc converts via a long: past LONG_MAX/LONG_MIN it clamps and sets
    // ERANGE, and %d truncates the clamped long to int.
    for (label, input) in [
        ("LONG_MAX", &b"9223372036854775807"[..]),
        ("LONG_MAX + 1 saturates", b"9223372036854775808"),
        ("LONG_MAX + 2 saturates", b"9223372036854775809"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MIN - 1 saturates", b"-9223372036854775809"),
        ("ULONG_MAX", b"18446744073709551615"),
        ("ULONG_MAX + 1", b"18446744073709551616"),
        ("30 nines", b"999999999999999999999999999999"),
        ("30 nines negative", b"-999999999999999999999999999999"),
        ("padded overflow", b"000000009223372036854775808"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn very_long_digit_runs() {
    // Far beyond any internal buffer: forces the saturating path and exercises
    // reading a large amount of stdin.
    for len in [64usize, 1024, 4095, 4096, 4097, 100_000] {
        let mut pos = vec![b'1'; len];
        assert_same("long digit run", &pos);

        pos.insert(0, b'-');
        assert_same("long negative digit run", &pos);
    }
}

#[test]
fn long_leading_whitespace_run() {
    // Whitespace skipping must also survive crossing the stdio buffer size.
    for len in [4095usize, 4096, 4097, 100_000] {
        let mut input = vec![b' '; len];
        input.push(b'9');
        assert_same("long whitespace run then digit", &input);

        let only_ws = vec![b'\n'; len];
        assert_same("long whitespace run then EOF", &only_ws);
    }
}

// ---------------------------------------------------------------------------
// 5. Signed overflow inside driver(): 2*x and y+300 wrap around.
// ---------------------------------------------------------------------------

#[test]
fn arithmetic_overflow_wraps() {
    // 2*x + 300 overflows int for x >= 1073741674 and underflows for
    // x <= -1073741824. These straddle both thresholds.
    for (label, input) in [
        ("largest x with no overflow", &b"1073741673"[..]),
        ("first x that overflows", b"1073741674"),
        ("one past that", b"1073741675"),
        ("2^30", b"1073741824"),
        ("INT_MAX doubles to -2", b"2147483647"),
        ("INT_MIN doubles to 0", b"-2147483648"),
        ("negative overflow threshold", b"-1073741824"),
        ("just inside negative", b"-1073741823"),
        ("deep negative", b"-1073741974"),
        ("x = 2^30 - 150", b"1073741674"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn overflow_threshold_sweep() {
    // Walk the exact wraparound boundary one integer at a time, in both
    // directions, so an off-by-one in the wrapping arithmetic cannot hide.
    for x in 1_073_741_668i64..=1_073_741_680 {
        assert_same("positive overflow boundary", x.to_string().as_bytes());
    }
    for x in -1_073_741_830i64..=-1_073_741_818 {
        assert_same("negative overflow boundary", x.to_string().as_bytes());
    }
}

// ---------------------------------------------------------------------------
// 6. argv is never inspected by the C program.
// ---------------------------------------------------------------------------

#[test]
fn command_line_arguments_are_ignored() {
    assert_same_with_args("args with a number", &["5"], b"9");
    assert_same_with_args("several args", &["a", "b", "c"], b"9");
    assert_same_with_args("args and empty stdin", &["--help"], b"");
    assert_same_with_args("arg that looks like a flag", &["-x"], b"1");
}

// ---------------------------------------------------------------------------
// 7. stdin that never reaches EOF.
//
// `scanf` returns as soon as it has seen the character that terminates the
// number, so the program must exit even while its stdin stays open. Reading
// stdin to EOF instead would hang forever on `yes 1 | driver`.
// ---------------------------------------------------------------------------

#[test]
fn endless_stdin_that_terminates_the_number() {
    // The delimiter arrives, so both programs must print and exit.
    assert_same_endless("endless \"1\\n\"", b"1\n");
    assert_same_endless("endless \"42 \"", b"42 ");
    assert_same_endless("endless \"-7\\n\"", b"-7\n");
    assert_same_endless("endless \"  8\\t\"", b"  8\t");
}

#[test]
fn endless_stdin_that_fails_to_match() {
    // A non-digit where a digit was required: matching failure, x stays 0.
    assert_same_endless("endless \"y\\n\"", b"y\n");
    assert_same_endless("endless \"-a\"", b"-a");
    assert_same_endless("endless \".\"", b".");
}

#[test]
fn endless_stdin_that_legitimately_blocks() {
    // Here the C program itself never finishes, because it can never rule out
    // another whitespace-then-digit or another digit. The Rust program must
    // block in the same way rather than inventing an EOF.
    assert_same_endless("endless spaces", b" ");
    assert_same_endless("endless newlines", b"\n");
    assert_same_endless("endless digits", b"1234567890");
}

// ---------------------------------------------------------------------------
// Broad sweep: every byte as the first character, plus structured combinations.
// ---------------------------------------------------------------------------

#[test]
fn every_leading_byte() {
    // Guarantees each of the whitespace-skip, sign, digit and matching-failure
    // branches is entered from a real input rather than by inspection.
    for b in 0u8..=255 {
        assert_same("single byte", &[b]);
        assert_same("byte then digits", &[b, b'4', b'2']);
    }
}

#[test]
fn structured_combinations() {
    let leads: [&[u8]; 6] = [b"", b" ", b"\n", b"\t\n ", b"\r\n", b"   \n\n"];
    let bodies: [&[u8]; 12] = [
        b"", b"0", b"7", b"-7", b"+7", b"-", b"+", b"a", b"0x1f", b"00012",
        b"2147483648", b"9223372036854775808",
    ];
    let tails: [&[u8]; 9] = [
        b"", b"\n", b" ", b"abc", b".5", b"e5", b"\x00", b"\xff", b"999",
    ];

    for lead in leads {
        for body in bodies {
            for tail in tails {
                let mut input = Vec::new();
                input.extend_from_slice(lead);
                input.extend_from_slice(body);
                input.extend_from_slice(tail);
                assert_same("lead/body/tail combination", &input);
            }
        }
    }
}

#[test]
fn deterministic_pseudo_random_inputs() {
    // A small xorshift keeps this dependency-free and reproducible. Bytes are
    // drawn from an alphabet dense in the characters %d actually branches on.
    const ALPHABET: &[u8] = b" \t\n\r\x0b\x0c+-0123456789aAxX.,\x00\xff";

    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..1500 {
        let len = (next() % 14) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_same("pseudo-random input", &input);
    }
}

#[test]
fn random_digit_strings_of_every_length() {
    // Covers each digit count from 1 to 25, i.e. from well inside int range to
    // well past LONG_MAX, with and without a sign.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for len in 1usize..=25 {
        for sign in ["", "-", "+"] {
            for _ in 0..8 {
                let digits: String = (0..len)
                    .map(|_| char::from(b'0' + (next() % 10) as u8))
                    .collect();
                let input = format!("{sign}{digits}");
                assert_same("random digit string", input.as_bytes());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Output shape: the printf format itself.
// ---------------------------------------------------------------------------

#[test]
fn output_is_exactly_one_decimal_line() {
    // Pins the "%d\n" format: no padding, no prefix, exactly one trailing
    // newline, nothing on stderr, exit status 0.
    let out = std::process::Command::new(common::c_bin())
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run C binary");
    assert_eq!(out.stdout, b"300\n", "C reference output shape changed");
    assert!(out.stderr.is_empty());
    assert_eq!(out.status.code(), Some(0));

    // And the Rust binary must agree, byte for byte.
    assert_same("no stdin at all", b"");
}
