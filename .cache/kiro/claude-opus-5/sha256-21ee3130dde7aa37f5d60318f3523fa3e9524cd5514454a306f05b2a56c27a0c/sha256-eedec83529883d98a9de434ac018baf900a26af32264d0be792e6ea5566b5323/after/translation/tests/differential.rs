//! Differential tests: the C program in `c_src/` is the ground truth, and the
//! Rust program in `translation/` must be byte-identical on stdout and stderr
//! and must exit the same way.
//!
//! The cases below are derived from the branches the C source actually has:
//!
//! ```c
//! typedef struct { unsigned int x : 2; unsigned int y : 3; bool b : 1; int z; } foo_t;
//! void print_foo(const foo_t *f) { printf("%u %u %d %d\n", f->x, f->y, f->b, f->z); }
//! int main() {
//!     unsigned int x = 0, y = 0; int b = 0, z = 0;
//!     scanf("%u", &x); scanf("%u", &y); scanf("%d", &b); scanf("%d", &z);
//!     driver(x, y, !!b, z);
//!     return 0;
//! }
//! ```
//!
//! There is no explicit control flow in `main`, so every branch lives either in
//! `scanf` (success / matching failure / input failure, sign handling, overflow
//! saturation) or in the bit-field stores (`x & 0x3`, `y & 0x7`, the one-bit
//! `bool`). Each of those is an input class and each gets a case here.

mod common;

use common::{assert_same, assert_same_str, c_bin, rust_bin, run, Rng};

// ---------------------------------------------------------------------------
// How many of the four conversions succeed: 0, 1, 2, 3, 4, and more than 4.
// A failed scanf leaves its destination at the initializer, so this controls
// how many zeros appear in the output.
// ---------------------------------------------------------------------------

#[test]
fn no_input_at_all() {
    assert_same_str("empty input", "");
}

#[test]
fn whitespace_only_input() {
    // Every conversion suffers an input failure while skipping whitespace.
    assert_same_str("spaces only", "     ");
    assert_same_str("newlines only", "\n\n\n");
    assert_same_str("mixed whitespace only", " \t\n\r\x0b\x0c ");
}

#[test]
fn one_value_only() {
    assert_same_str("single item", "5");
    assert_same_str("single item, trailing newline", "5\n");
}

#[test]
fn two_values_only() {
    assert_same_str("two items", "1 2");
}

#[test]
fn three_values_only() {
    assert_same_str("three items", "1 2 1");
}

#[test]
fn all_four_values() {
    assert_same_str("four items", "1 2 1 42");
    assert_same_str("four items, trailing newline", "1 2 1 42\n");
}

#[test]
fn surplus_values_are_ignored() {
    assert_same_str("six items, only four read", "1 2 3 4 5 6");
}

#[test]
fn every_truncated_prefix_of_a_full_input() {
    // Walks the input one byte at a time, so each conversion is cut off at a
    // sign, mid-number and at a separator.
    let full = "12 -34 56 -78";
    for len in 0..=full.len() {
        assert_same_str("prefix", &full[..len]);
    }
}

// ---------------------------------------------------------------------------
// `scanf` skips leading whitespace and therefore reads across newlines. This
// is the documented difference from `fgets`, so it is pinned down explicitly.
// ---------------------------------------------------------------------------

#[test]
fn conversions_span_lines_and_separators() {
    assert_same_str("space separated", "1 2 3 4");
    assert_same_str("newline separated", "1\n2\n3\n4\n");
    assert_same_str("tab separated", "1\t2\t3\t4");
    assert_same_str("crlf separated", "1\r\n2\r\n3\r\n4\r\n");
    assert_same_str("vertical tab and form feed", "1\x0b2\x0c3 4");
    assert_same_str("carriage return only", "1\r2\r3\r4");
    assert_same_str("run of blank lines between values", "1\n\n\n2\n\n3\n\n\n\n4");
    assert_same_str("leading and trailing whitespace", "   1   2   3   4   \n");
    assert_same_str("all four on one line, wide gaps", "1     2     3     4");
}

// ---------------------------------------------------------------------------
// `unsigned int x : 2` keeps the low two bits; `unsigned int y : 3` keeps the
// low three. Sweep both fields past their widths.
// ---------------------------------------------------------------------------

#[test]
fn x_field_truncates_to_two_bits() {
    for x in 0u32..=20 {
        assert_same_str("x sweep", &format!("{x} 0 0 0"));
    }
    for x in [31u32, 32, 63, 64, 255, 256, 1023, 4095] {
        assert_same_str("x larger", &format!("{x} 0 0 0"));
    }
}

#[test]
fn y_field_truncates_to_three_bits() {
    for y in 0u32..=20 {
        assert_same_str("y sweep", &format!("0 {y} 0 0"));
    }
    for y in [31u32, 32, 63, 64, 255, 256, 1023, 4095] {
        assert_same_str("y larger", &format!("0 {y} 0 0"));
    }
}

#[test]
fn both_bit_fields_at_their_maximum() {
    assert_same_str("x and y all ones", "3 7 1 -1");
    assert_same_str("x and y just past all ones", "4 8 0 0");
}

// ---------------------------------------------------------------------------
// `!!b` collapses any non-zero to 1, and `bool b : 1` stores 0 or 1.
// ---------------------------------------------------------------------------

#[test]
fn bool_field_collapses_every_nonzero() {
    for b in [
        "0",
        "1",
        "-1",
        "2",
        "-2",
        "5",
        "-3",
        "256",
        "2147483647",
        "-2147483648",
        "2147483648",
        "4294967296",
        "-4294967296",
        "9223372036854775807",
        "-9223372036854775808",
        "99999999999999999999999",
        "-99999999999999999999999",
    ] {
        assert_same_str("bool field", &format!("0 0 {b} 0"));
    }
}

// ---------------------------------------------------------------------------
// `int z` is stored without truncation, so the full signed range matters,
// as does the wrap that `%d` performs when the value exceeds `int`.
// ---------------------------------------------------------------------------

#[test]
fn z_field_across_the_int_range() {
    for z in [
        "0",
        "1",
        "-1",
        "42",
        "-42",
        "2147483646",
        "2147483647",  // INT_MAX
        "2147483648",  // INT_MAX + 1, wraps
        "2147483649",
        "-2147483647",
        "-2147483648", // INT_MIN
        "-2147483649", // INT_MIN - 1, wraps
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967296",
    ] {
        assert_same_str("z value", &format!("0 0 0 {z}"));
    }
}

// ---------------------------------------------------------------------------
// Overflow, truncation and signedness exactly as the C library performs them:
// `%u` goes through `strtoul` (wraps a negative, saturates at `ULONG_MAX`),
// `%d` through `strtol` (saturates at `LONG_MIN`/`LONG_MAX`), and the 64-bit
// result is then truncated into the 32-bit destination.
// ---------------------------------------------------------------------------

#[test]
fn unsigned_conversion_boundaries() {
    for v in [
        "4294967295",           // UINT_MAX
        "4294967296",           // 2^32, truncates to 0
        "4294967297",           // 2^32 + 1
        "9223372036854775807",  // LONG_MAX
        "9223372036854775808",  // 2^63
        "18446744073709551615", // ULONG_MAX
        "18446744073709551616", // ULONG_MAX + 1, saturates
        "18446744073709551617",
        "99999999999999999999999999999999",
    ] {
        assert_same_str("u boundary in x", &format!("{v} 0 0 0"));
        assert_same_str("u boundary in y", &format!("0 {v} 0 0"));
    }
}

#[test]
fn negative_values_for_an_unsigned_conversion() {
    // `%u` accepts a sign; the value wraps modulo 2^64 before truncation.
    for v in [
        "-0",
        "-1",
        "-2",
        "-3",
        "-4",
        "-8",
        "-4294967295",
        "-4294967296",
        "-4294967297",
        "-18446744073709551615", // wraps to 1
        "-18446744073709551616", // saturates
        "-99999999999999999999999",
    ] {
        assert_same_str("negative u in x", &format!("{v} 0 0 0"));
        assert_same_str("negative u in y", &format!("0 {v} 0 0"));
    }
}

#[test]
fn signed_conversion_boundaries() {
    for v in [
        "9223372036854775806",
        "9223372036854775807",  // LONG_MAX
        "9223372036854775808",  // LONG_MAX + 1, saturates then truncates
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808", // LONG_MIN
        "-9223372036854775809", // LONG_MIN - 1, saturates then truncates
        "18446744073709551615",
        "18446744073709551616",
        "99999999999999999999999",
        "-99999999999999999999999",
    ] {
        assert_same_str("d boundary in z", &format!("0 0 0 {v}"));
        assert_same_str("d boundary in b", &format!("0 0 {v} 0"));
    }
}

#[test]
fn very_long_digit_runs() {
    for len in [19usize, 20, 21, 39, 40, 128, 1000] {
        let nines = "9".repeat(len);
        assert_same_str("long run of nines for u", &format!("{nines} 0 0 0"));
        assert_same_str("long run of nines for d", &format!("0 0 0 {nines}"));
        assert_same_str("long negative run for d", &format!("0 0 0 -{nines}"));
    }
}

#[test]
fn leading_zeros_do_not_change_the_value() {
    assert_same_str("leading zeros", "0003 0007 0001 0009");
    assert_same_str("many leading zeros", "0000000000000000005 0 0 0");
    assert_same_str("negative with leading zeros", "0 0 0 -0000000000042");
    assert_same_str("zeros only", "00000000 00000000 00000000 00000000");
    // No `%i`, so a `0x` prefix is not special: the `0` converts and `x` stops
    // the scan, poisoning every later conversion.
    assert_same_str("hex-looking input", "0x10 0 0 0");
}

// ---------------------------------------------------------------------------
// Sign handling, including the cases where a sign is accepted and then no
// digit follows (a matching failure that leaves the destination untouched).
// ---------------------------------------------------------------------------

#[test]
fn explicit_plus_signs_are_accepted() {
    assert_same_str("all plus", "+1 +2 +1 +7");
    assert_same_str("plus zero", "+0 +0 +0 +0");
    assert_same_str("mixed signs", "+3 -1 +0 -5");
}

#[test]
fn a_sign_with_no_digits_is_a_matching_failure() {
    assert_same_str("lone minus then space", "- 1 2 3");
    assert_same_str("lone plus then space", "+ 1 2 3");
    assert_same_str("double minus", "--1 2 3 4");
    assert_same_str("plus then minus", "+-1 2 3 4");
    assert_same_str("minus then plus", "-+1 2 3 4");
    assert_same_str("lone minus at end of input", "1 2 3 -");
    assert_same_str("lone plus at end of input", "1 2 3 +");
    assert_same_str("only a minus", "-");
    assert_same_str("only a plus", "+");
    assert_same_str("only signs", "----");
    assert_same_str("sign then newline", "-\n1 2 3");
    assert_same_str("sign then letter", "-a 1 2 3");
}

// ---------------------------------------------------------------------------
// Matching failures at each of the four positions. The offending byte stays in
// the stream, so once a conversion fails on a non-digit every later conversion
// fails too; that stickiness is part of the behaviour being matched.
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_at_each_position() {
    assert_same_str("bad first value", "x 1 2 3");
    assert_same_str("bad second value", "1 x 2 3");
    assert_same_str("bad third value", "1 2 x 4");
    assert_same_str("bad fourth value", "1 2 3 x");
}

#[test]
fn a_stuck_stream_stays_stuck() {
    assert_same_str("junk first, digits after", "junk 1 2 3");
    assert_same_str("junk in the middle", "1 2 junk 3 4");
    assert_same_str("only junk", "hello world");
}

#[test]
fn non_digit_characters_that_terminate_a_scan() {
    for sep in [
        ".", ",", ";", ":", "/", "\\", "|", "*", "#", "@", "!", "?", "(", ")", "[", "]", "{", "}",
        "<", ">", "=", "\"", "'", "`", "~", "^", "&", "%", "$", "_",
    ] {
        assert_same_str("separator", &format!("1{sep}2 3 4"));
        assert_same_str("leading separator", &format!("{sep}1 2 3 4"));
    }
}

#[test]
fn number_like_but_not_integers() {
    assert_same_str("decimals", "1.5 2.5 3.5 4.5");
    assert_same_str("scientific notation", "1e3 2e3 3e3 4e3");
    assert_same_str("thousands separators", "1,000 2,000 3,000 4,000");
    assert_same_str("digits glued to letters", "1a 2b 3c 4d");
    assert_same_str("underscore separator", "1_2 3 4 5");
    assert_same_str("infinity", "inf inf inf inf");
    assert_same_str("not a number", "nan nan nan nan");
    assert_same_str("nil", "(nil) 0 0 0");
}

// ---------------------------------------------------------------------------
// Bytes outside the printable ASCII range: NUL terminates nothing here (the
// stream is byte-oriented), and no high byte is whitespace in the C locale.
// ---------------------------------------------------------------------------

#[test]
fn embedded_nul_and_high_bytes() {
    assert_same("leading NUL", b"\x001 2 3 4");
    assert_same("NUL between values", b"1\x002 3 4");
    assert_same("NUL after a value", b"1\x00");
    assert_same("trailing NUL", b"1 2 3 4\x00");
    assert_same("0xa0 (non-breaking space in latin-1)", b"\xa01 2 3 4");
    assert_same("0xff", b"\xff1 2 3 4");
    assert_same("0x85 (NEL)", b"1\x852 3 4");
    assert_same("utf-8 non-breaking space", b"\xc2\xa01 2 3 4");
    assert_same("all high bytes", b"\x80\x81\x82\x83");
    assert_same("control bytes", b"1\x1c2\x1d3\x1e4");
    assert_same("bell and backspace", b"1\x072\x083 4");
    assert_same("escape byte", b"1\x1b2 3 4");
    assert_same("every byte value in turn", &(0u8..=255).collect::<Vec<u8>>());
}

// ---------------------------------------------------------------------------
// A single write to stdout is the whole output, so the ways that write can
// fail are also observable differences. The C program keeps the default
// `SIGPIPE` disposition and dies from it; the Rust program must too.
// ---------------------------------------------------------------------------

#[test]
fn dying_from_sigpipe_when_stdout_has_no_reader() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // Closing the read end before any input is written makes this
    // deterministic: neither program can write to stdout until it has been fed
    // stdin, and by then the pipe is already broken.
    let observe = |program: &std::path::Path| {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");

        drop(child.stdout.take().expect("stdout was piped"));

        {
            let mut stdin = child.stdin.take().expect("stdin was piped");
            let _ = stdin.write_all(b"1 2 3 4\n");
            let _ = stdin.flush();
        }

        let status = child.wait().expect("wait");
        use std::os::unix::process::ExitStatusExt;
        (status.code(), status.signal())
    };

    let c = observe(&c_bin());
    let r = observe(&rust_bin());
    assert_eq!(
        c, r,
        "exit status differs when stdout's reader is gone: C {c:?} vs Rust {r:?}"
    );
}

#[test]
fn a_failing_stdout_write_is_still_a_clean_exit() {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::process::{Command, Stdio};

    // `/dev/full` accepts the open and fails the write with ENOSPC. Where it is
    // unavailable, `/dev/null` still exercises the redirected-stdout path.
    let sink = if std::path::Path::new("/dev/full").exists() {
        "/dev/full"
    } else {
        "/dev/null"
    };

    let observe = |program: &std::path::Path| {
        let out = OpenOptions::new().write(true).open(sink).expect("open sink");
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(out))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        {
            let mut stdin = child.stdin.take().expect("stdin was piped");
            let _ = stdin.write_all(b"1 2 3 4\n");
            let _ = stdin.flush();
        }
        let output = child.wait_with_output().expect("wait");
        use std::os::unix::process::ExitStatusExt;
        (output.status.code(), output.status.signal(), output.stderr)
    };

    let c = observe(&c_bin());
    let r = observe(&rust_bin());
    assert_eq!(c, r, "behaviour differs when writing to {sink}");
}

#[test]
fn stdin_closed_immediately_looks_like_end_of_file() {
    use std::process::{Command, Stdio};

    let observe = |program: &std::path::Path| {
        let output = Command::new(program)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run with /dev/null on stdin");
        use std::os::unix::process::ExitStatusExt;
        (output.stdout, output.stderr, output.status.code(), output.status.signal())
    };

    assert_eq!(
        observe(&c_bin()),
        observe(&rust_bin()),
        "behaviour differs with /dev/null on stdin"
    );
}

// ---------------------------------------------------------------------------
// Output formatting: one line, four fields, single spaces, one trailing
// newline and nothing on stderr. Checked against the C program and also
// pinned to the literal bytes.
// ---------------------------------------------------------------------------

#[test]
fn output_is_a_single_line_with_a_trailing_newline() {
    let c = run(&c_bin(), b"7 15 9 -12");
    let r = run(&rust_bin(), b"7 15 9 -12");
    assert_eq!(c.stdout, b"3 7 1 -12\n".to_vec(), "C output shape changed");
    assert_eq!(r.stdout, c.stdout);
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
    assert_eq!(c.status, Ok(0));
    assert_eq!(r.status, Ok(0));
}

// ---------------------------------------------------------------------------
// Broad randomized sweeps with a fixed seed, to catch combinations the
// hand-written cases above do not name.
// ---------------------------------------------------------------------------

#[test]
fn randomized_numeric_combinations() {
    const INTERESTING: [&str; 24] = [
        "0",
        "1",
        "2",
        "3",
        "4",
        "7",
        "8",
        "15",
        "16",
        "255",
        "2147483647",
        "2147483648",
        "2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "9223372036854775807",
        "9223372036854775808",
        "18446744073709551615",
        "18446744073709551616",
        "10000000000000000000000000",
        "000012",
        "+5",
        "-0",
    ];
    const SEPARATORS: [&str; 6] = [" ", "\n", "\t", "  ", "\r\n", " \n\t "];

    let mut rng = Rng::new(0xC0FF_EE12_3456_789A);
    for _ in 0..400 {
        let mut input = String::new();
        for i in 0..4 {
            if i > 0 {
                input.push_str(rng.pick(&SEPARATORS));
            }
            if rng.below(4) == 0 {
                input.push('-');
            }
            input.push_str(rng.pick(&INTERESTING));
        }
        if rng.below(2) == 0 {
            input.push('\n');
        }
        assert_same_str("random numeric combination", &input);
    }
}

#[test]
fn randomized_byte_soup() {
    const BYTES: [u8; 22] = [
        b'0', b'1', b'2', b'4', b'8', b'9', b'-', b'+', b' ', b'\n', b'\t', b'\r', b'\x0b', b'x',
        b'.', b',', b'e', b'a', 0x00, 0xff, 0x80, b'/',
    ];

    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for _ in 0..600 {
        let len = rng.below(26);
        let input: Vec<u8> = (0..len).map(|_| *rng.pick(&BYTES)).collect();
        assert_same("random byte soup", &input);
    }
}
