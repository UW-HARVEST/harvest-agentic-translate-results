//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input/condition of its row, runs BOTH
//! artifacts, and asserts they reject it the same way — same stdout, same stderr,
//! same exit status (or the same fatal signal).  Where a row's rejection has a
//! documented observable signature, the test also pins the C's own output so the
//! row cannot silently stop triggering.

mod common;

use common::{
    assert_same, assert_same_expecting, assert_same_prefix, assert_same_with, c_bin, run,
    run_tracking_stdin_offset, run_with_args, rust_bin, writer_status_with_early_reader, In, Out,
    DEFAULT_TIMEOUT_SECS,
};

/// E1: conversion #1 input failure — stdin is empty, so `scanf` returns `EOF`,
/// `x` and `y` keep their `0` defaults and `foo(0,0)` prints nothing.
#[test]
fn e1_conv1_input_failure_empty() {
    assert_same_expecting("e1 empty stdin", b"", b"");
    assert_same_with(
        "e1 /dev/null stdin",
        In::Path("/dev/null"),
        Out::Pipe,
        DEFAULT_TIMEOUT_SECS,
    );
}

/// E2: conversion #1 input failure — whitespace only: `%d` skips it all and hits
/// end-of-file.
#[test]
fn e2_conv1_input_failure_whitespace_only() {
    for input in [
        &b" "[..],
        b"\n",
        b"\t",
        b"\x0b",
        b"\x0c",
        b"\r",
        b"   \n\t\x0b\x0c\r  \n",
        b"\n\n\n\n\n",
    ] {
        assert_same_expecting("e2 whitespace only", input, b"");
    }
    // A whitespace run far larger than any stdio buffer, still ending at EOF.
    let big = vec![b' '; 300_000];
    assert_same_expecting("e2 huge whitespace run", &big, b"");
}

/// E3: conversion #1 matching failure — the first non-whitespace byte cannot
/// start an integer, so `scanf` returns 0 and both variables keep `0`.
#[test]
fn e3_conv1_matching_failure_nondigit() {
    for input in [
        &b"abc"[..],
        b"x 1",
        b".5",
        b"/",
        b",",
        b"e5",
        b"nan",
        b"inf 3",
        b"  \n\t hello 4 5",
        b"'5' '6'",
        b"[5 6]",
        b"*",
    ] {
        assert_same_expecting("e3 non-digit start", input, b"");
    }
}

/// E4: conversion #1 matching failure after a sign — the sign is consumed but no
/// digit follows.
#[test]
fn e4_conv1_matching_failure_sign_only() {
    for input in [
        &b"-"[..], b"+", b"- 5", b"+ 5", b"+x", b"-x", b"--5", b"++5", b"+-5", b"-+5", b"- ", b"+\n",
        b"-.5", b"+.5",
    ] {
        assert_same_expecting("e4 sign without digits", input, b"");
    }
}

/// E5: conversion #1 matching failure on bytes that are neither `isspace` nor a
/// digit in the C locale — NUL and the high half of the byte range.
#[test]
fn e5_conv1_matching_failure_nul_and_high_bytes() {
    assert_same_expecting("e5 leading NUL", b"\x005 6", b"");
    assert_same_expecting("e5 NUL only", b"\x00", b"");
    assert_same_expecting("e5 NUL run", b"\x00\x00\x00\x00", b"");
    for b in [0x80u8, 0xa0, 0xc3, 0xff] {
        let input = [b, b'5', b' ', b'6'];
        assert_same_expecting("e5 high byte", &input, b"");
    }
    // UTF-8 digits are not C digits.
    assert_same_expecting("e5 arabic-indic digits", "١٢٣ ٤".as_bytes(), b"");
    assert_same_expecting("e5 fullwidth digits", "５ ６".as_bytes(), b"");
}

/// E6: conversion #2 input failure — end-of-file after the first integer, so `y`
/// keeps `0` and `foo(x,0)` runs.
#[test]
fn e6_conv2_input_failure_eof() {
    // foo(5,0) prints "loop\nx\n" once per remaining unit of x.
    assert_same_expecting("e6 bare int", b"5", &"loop\nx\n".repeat(5).into_bytes());
    assert_same_expecting("e6 trailing space", b"5 ", &"loop\nx\n".repeat(5).into_bytes());
    assert_same_expecting("e6 trailing newline", b"5\n", &"loop\nx\n".repeat(5).into_bytes());
    assert_same_expecting("e6 trailing ws run", b"5 \t\n\x0b\x0c\r ", &"loop\nx\n".repeat(5).into_bytes());
    // A non-positive first integer with no second one prints nothing at all.
    assert_same_expecting("e6 zero", b"0", b"");
    assert_same_expecting("e6 negative", b"-7", b"");
}

/// E7: conversion #2 matching failure — a non-integer follows the first integer.
#[test]
fn e7_conv2_matching_failure_nondigit() {
    let five = "loop\nx\n".repeat(5).into_bytes();
    for input in [&b"5 abc"[..], b"5abc", b"5.5", b"5 x", b"5,6", b"5;6", b"5 .6", b"5\x006"] {
        assert_same_expecting("e7 junk second token", input, &five);
    }
    // Hexadecimal is not parsed: "0x10" yields x=0 and then fails on 'x'.
    assert_same_expecting("e7 hex literal", b"0x10 5", b"");
}

/// E8: conversion #2 matching failure after a sign.
#[test]
fn e8_conv2_matching_failure_sign_only() {
    let five = "loop\nx\n".repeat(5).into_bytes();
    for input in [&b"5 -"[..], b"5 +", b"5 -x", b"5 +x", b"5-", b"5+", b"5 - 6", b"5 + 6"] {
        assert_same_expecting("e8 second sign without digits", input, &five);
    }
}

/// E9: overflow above `LONG_MAX` — glibc's `strtol` saturates to `LONG_MAX` and
/// the `%d` store truncates it to `-1`, which the loop guard then rejects.
#[test]
fn e9_overflow_above_long_max() {
    // x = -1, y = 1  =>  guard passes on y only: "loop\ny\n".
    for x in [
        "9223372036854775808",
        "9223372036854775809",
        "99999999999999999999999999",
        "18446744073709551616",
        "340282366920938463463374607431768211456",
    ] {
        assert_same_expecting(
            &format!("e9 x={x}"),
            format!("{x} 1").as_bytes(),
            b"loop\ny\n",
        );
        // With y = 0 the truncated -1 leaves the guard false: no output at all.
        assert_same_expecting(&format!("e9 x={x} y=0"), format!("{x} 0").as_bytes(), b"");
    }
    // Same on the second operand: y = -1 with x = 0 keeps the guard false.
    assert_same_expecting("e9 y overflow", b"0 99999999999999999999999999", b"");
    // A digit run far longer than any buffer.
    let huge = "7".repeat(100_000);
    assert_same_expecting("e9 100k digits", format!("{huge} 1").as_bytes(), b"loop\ny\n");
    assert_same_expecting("e9 1MiB digits", format!("{} 1", "9".repeat(1_048_576)).as_bytes(), b"loop\ny\n");
}

/// E10: overflow below `LONG_MIN` — saturates to `LONG_MIN`, truncating to `0`.
#[test]
fn e10_overflow_below_long_min() {
    for x in [
        "-9223372036854775809",
        "-99999999999999999999999999",
        "-18446744073709551616",
    ] {
        // x = 0, y = 1  =>  "loop\ny\n"
        assert_same_expecting(
            &format!("e10 x={x}"),
            format!("{x} 1").as_bytes(),
            b"loop\ny\n",
        );
        // x = 0, y = 0  =>  guard false, no output
        assert_same_expecting(&format!("e10 x={x} y=0"), format!("{x} 0").as_bytes(), b"");
        // y saturating to 0 turns the "unbounded negative y" case into a no-op.
        assert_same_expecting(&format!("e10 y={x}"), format!("0 {x}").as_bytes(), b"");
    }
}

/// E11: values inside `long` range but outside `int` range are silently
/// truncated, with no error reported anywhere.
#[test]
fn e11_int_truncation_in_long_range() {
    // 2147483648 -> INT_MIN (non-positive): guard passes on y only.
    assert_same_expecting("e11 INT_MAX+1", b"2147483648 1", b"loop\ny\n");
    assert_same_expecting("e11 INT_MAX+1 y=0", b"2147483648 0", b"");
    // 4294967296 -> 0
    assert_same_expecting("e11 UINT_MAX+1", b"4294967296 1", b"loop\ny\n");
    assert_same_expecting("e11 UINT_MAX+1 y=0", b"4294967296 0", b"");
    // 4294967295 -> -1
    assert_same_expecting("e11 UINT_MAX", b"4294967295 1", b"loop\ny\n");
    // 9223372036854775807 (LONG_MAX) -> -1
    assert_same_expecting("e11 LONG_MAX", b"9223372036854775807 1", b"loop\ny\n");
    // -9223372036854775808 (LONG_MIN) -> 0
    assert_same_expecting("e11 LONG_MIN", b"-9223372036854775808 1", b"loop\ny\n");
    // -2147483649 -> INT_MAX, which is positive: unbounded output, so compare a
    // prefix, and pin the shape that proves the value really was huge/positive.
    let c = common::stdout_prefix(&c_bin(), b"-2147483649 1", 32);
    assert!(
        c.starts_with(b"loop\nx\ny\n"),
        "e11 INT_MIN-1: expected the truncated value to be positive, got {:?}",
        String::from_utf8_lossy(&c)
    );
    assert_same_prefix("e11 INT_MIN-1", b"-2147483649 1", 64 * 1024);
    // On the second operand, 2147483648 -> INT_MIN, i.e. negative y: unbounded.
    assert_same_prefix("e11 y=INT_MAX+1", b"1 2147483648", 64 * 1024);
    assert_same_expecting("e11 y=UINT_MAX+1", b"0 4294967296", b"");
}

/// E12: the `int` boundaries themselves convert exactly.
#[test]
fn e12_int_boundaries_exact() {
    // INT_MIN is non-positive, so it never prints an "x" line.
    assert_same_expecting("e12 INT_MIN,1", b"-2147483648 1", b"loop\ny\n");
    assert_same_expecting("e12 INT_MIN,0", b"-2147483648 0", b"");
    assert_same_expecting("e12 0,INT_MIN", b"0 -2147483648", b"");
    assert_same_expecting("e12 INT_MIN,INT_MIN", b"-2147483648 -2147483648", b"");
    // INT_MAX is positive: unbounded output on either operand.
    assert_same_prefix("e12 INT_MAX,0", b"2147483647 0", 64 * 1024);
    assert_same_prefix("e12 0,INT_MAX", b"0 2147483647", 64 * 1024);
    assert_same_prefix("e12 INT_MAX,INT_MIN", b"2147483647 -2147483648", 64 * 1024);
    // One step inside the boundary.
    assert_same_expecting("e12 INT_MIN+1,1", b"-2147483647 1", b"loop\ny\n");
}

/// E13: `%d` has no digit-grouping flag, so a thousands separator ends the
/// conversion and makes the *second* conversion fail.
#[test]
fn e13_no_digit_grouping() {
    // "1,000 5" -> x = 1, y stays 0 -> foo(1,0) prints "loop\nx\n".
    assert_same_expecting("e13 grouped x", b"1,000 5", b"loop\nx\n");
    assert_same_expecting("e13 grouped zero", b"0,000 5", b"");
    // Grouping in the second operand truncates it to its first group.
    assert_same_expecting("e13 grouped y", b"0 1,000", b"loop\ny\n");
    // Underscores and spaces are not separators either.
    assert_same_expecting("e13 underscore", b"1_000 5", b"loop\nx\n");
}

/// E14: the loop guard `!(x > 0 || y > 0)` rejects the whole workload — zero
/// iterations, empty stdout, exit 0.
#[test]
fn e14_loop_guard_rejects_nonpositive() {
    let mut rng = common::Rng::new(0xE14);
    for input in [
        &b"0 0"[..],
        b"-1 0",
        b"0 -1",
        b"-1 -1",
        b"-2147483648 0",
        b"0 -2147483648",
        b"-2147483648 -2147483648",
        b"-0 -0",
    ] {
        assert_same_expecting("e14 guard false", input, b"");
    }
    for i in 0..24 {
        let x = rng.range(-100_000, 0);
        let y = rng.range(-100_000, 0);
        assert_same_expecting(&format!("e14 #{i} ({x},{y})"), format!("{x} {y}").as_bytes(), b"");
    }
}

/// E15: signed-overflow UB at `y--`.  `y` is decremented whenever `y != 0`, so a
/// negative `y` reached with a positive `x` wraps down through `INT_MIN` and the
/// program runs for ~2^32 iterations.  There is no guard and no diagnostic, so the
/// two artifacts are compared over a fixed-length output prefix.
#[test]
fn e15_signed_overflow_unbounded_prefix() {
    let n = 256 * 1024;
    for input in [&b"1 -1"[..], b"3 -3", b"5-6", b"1 -2147483648", b"2147483647 -1"] {
        assert_same_prefix("e15 unbounded overflow", input, n);
    }
    // The wrap really is unbounded: the C reference must still be running after
    // producing far more output than the workload's operands could explain.
    let produced = common::stdout_prefix(&c_bin(), b"1 -1", 512 * 1024);
    assert_eq!(produced.len(), 512 * 1024, "e15: C reference terminated early");
}

/// E16: `x--` is guarded by `if (x > 0)`, so `x <= 0` never decrements and never
/// prints an `"x"` line — no underflow is possible on that operand.
#[test]
fn e16_x_decrement_guarded() {
    assert_same_expecting("e16 INT_MIN,3", b"-2147483648 3", b"loop\ny\ny\ny\n");
    assert_same_expecting("e16 0,3", b"0 3", b"loop\ny\ny\ny\n");
    assert_same_expecting("e16 -1,3", b"-1 3", b"loop\ny\ny\ny\n");
    assert_same_expecting("e16 -5,1", b"-5 1", b"loop\ny\n");
    let out = run(&c_bin(), b"-2147483648 9").stdout;
    assert!(
        !out.contains(&b'x'),
        "e16: the C reference printed an x line for a non-positive x: {:?}",
        String::from_utf8_lossy(&out)
    );
}

/// E17: `printf`'s return value is discarded, so a failing stdout (`/dev/full`,
/// every write `ENOSPC`) is invisible: the program still exits 0.
#[test]
fn e17_stdout_write_failure_ignored() {
    for input in [&b"3 2"[..], b"9000 9000", b"0 0", b"abc"] {
        assert_same_with("e17 /dev/full", In::Bytes(input), Out::Path("/dev/full"), DEFAULT_TIMEOUT_SECS);
        let c = common::run_with(&c_bin(), In::Bytes(input), Out::Path("/dev/full"), DEFAULT_TIMEOUT_SECS);
        let r = common::run_with(&rust_bin(), In::Bytes(input), Out::Path("/dev/full"), DEFAULT_TIMEOUT_SECS);
        assert_eq!(c.code, 0, "e17: expected the C reference to ignore ENOSPC and exit 0");
        assert_eq!(r.code, 0, "e17: Rust must also ignore ENOSPC and exit 0");
    }
}

/// E18: stdout closed before exec — writes fail with `EBADF`, still unchecked,
/// still exit 0.
#[test]
fn e18_stdout_closed_fd() {
    for input in [&b"3 2"[..], b"5000 1", b"0 0", b"-"] {
        assert_same_with("e18 closed fd 1", In::Bytes(input), Out::Closed, DEFAULT_TIMEOUT_SECS);
        let c = common::run_with(&c_bin(), In::Bytes(input), Out::Closed, DEFAULT_TIMEOUT_SECS);
        assert_eq!(c.code, 0, "e18: expected the C reference to exit 0 with fd 1 closed");
    }
}

/// E19: the reader of stdout goes away mid-stream.  With the inherited default
/// `SIGPIPE` disposition the writer is *killed by signal 13* (wait status 141)
/// rather than exiting cleanly.
#[test]
fn e19_sigpipe_kills_writer() {
    for input in [&b"2000000 1"[..], b"1 -1", b"0 2000000"] {
        let c = writer_status_with_early_reader(&c_bin(), input, 100, DEFAULT_TIMEOUT_SECS);
        let r = writer_status_with_early_reader(&rust_bin(), input, 100, DEFAULT_TIMEOUT_SECS);
        assert_eq!(
            c, 141,
            "e19: expected the C reference to die from SIGPIPE (141), got {c}"
        );
        assert_eq!(
            c, r,
            "e19: writer wait status differs for {:?} (C={c}, Rust={r}); \
             Rust must not leave SIGPIPE ignored",
            String::from_utf8_lossy(input)
        );
    }
}

/// E20: `scanf`'s return value is discarded, so no parse failure is ever
/// reported: `foo` runs regardless and the exit status is always 0.
#[test]
fn e20_return_value_discarded_exit_zero() {
    for input in [
        &b""[..],
        b"   ",
        b"abc",
        b"-",
        b"5",
        b"5 abc",
        b"0 0",
        b"3 2",
        b"\x00",
        b"99999999999999999999 1",
        b"1,000 5",
        b"5 6 7 8",
    ] {
        let c = run(&c_bin(), input);
        let r = run(&rust_bin(), input);
        assert_eq!(
            c.code, 0,
            "e20: expected the C reference to exit 0 for {:?}",
            String::from_utf8_lossy(input)
        );
        assert_eq!(
            c.code, r.code,
            "e20: exit status differs for {:?}",
            String::from_utf8_lossy(input)
        );
        assert_eq!(c.stdout, r.stdout, "e20: stdout differs for {:?}", String::from_utf8_lossy(input));
        assert_eq!(c.stderr, r.stderr, "e20: stderr differs for {:?}", String::from_utf8_lossy(input));
    }
}

/// E21: `int main()` declares no parameters, so `argv` is never inspected —
/// extra arguments (including ones that look like the missing operands) are
/// ignored entirely.
#[test]
fn e21_argv_ignored() {
    for (args, input) in [
        (vec!["5", "6"], &b"3 2"[..]),
        (vec!["--help"], b"3 2"),
        (vec!["-x", "-y", "-z"], b""),
        (vec!["9999"], b"abc"),
        (vec![""], b"0 4"),
    ] {
        let c = run_with_args(&c_bin(), &args, input, DEFAULT_TIMEOUT_SECS);
        let r = run_with_args(&rust_bin(), &args, input, DEFAULT_TIMEOUT_SECS);
        // Identical to the same run without any arguments.
        let baseline = run(&c_bin(), input);
        assert_eq!(
            c.stdout, baseline.stdout,
            "e21: the C reference reacted to argv {args:?}"
        );
        assert_eq!(c.stdout, r.stdout, "e21: stdout differs with argv {args:?}");
        assert_eq!(c.stderr, r.stderr, "e21: stderr differs with argv {args:?}");
        assert_eq!(c.code, r.code, "e21: exit status differs with argv {args:?}");
    }
}

/// E22: an unbounded, never-matching stdin (`/dev/zero`) must be *rejected
/// immediately*, not drained: the first `%d` sees a NUL, fails, and the program
/// exits.  A translation that slurps stdin would hang or exhaust memory here.
#[test]
fn e22_unbounded_stdin_not_drained() {
    let secs = 10;
    let c = common::run_with(&c_bin(), In::Path("/dev/zero"), Out::Pipe, secs);
    let r = common::run_with(&rust_bin(), In::Path("/dev/zero"), Out::Pipe, secs);
    assert_eq!(c.code, 0, "e22: expected the C reference to exit 0 promptly");
    assert_ne!(c.code, 124, "e22: the C reference hit the wall-clock cap");
    assert_ne!(
        r.code, 124,
        "e22: Rust hit the {secs}s wall-clock cap on /dev/zero — stdin is being drained"
    );
    assert_eq!(c.code, r.code, "e22: exit status differs on /dev/zero");
    assert_eq!(c.stdout, r.stdout, "e22: stdout differs on /dev/zero");
    assert_eq!(c.stdout, b"", "e22: expected no output at all");

    // Timing-independent proof that the stream is not drained: give both
    // artifacts a *seekable* stdin they share with us and compare how far the
    // shared file offset advanced.
    for payload in [
        vec![0u8; 1 << 20],                                         // never matches
        b"z".repeat(1 << 20),                                       // never matches
        [b"5 6".to_vec(), b"z".repeat(1 << 20)].concat(),           // matches, then stops
        [b"    12    34".to_vec(), vec![b'\n'; 1 << 20]].concat(),  // matches, trailing space
    ] {
        let (c, c_off) = run_tracking_stdin_offset(&c_bin(), &payload, secs);
        let (r, r_off) = run_tracking_stdin_offset(&rust_bin(), &payload, secs);
        assert!(
            c_off < payload.len() as u64,
            "e22: expected the C reference to leave the stream unconsumed"
        );
        assert_eq!(
            c_off, r_off,
            "e22: shared stdin offset differs after exit (C={c_off}, Rust={r_off}) \
             for a {}-byte payload",
            payload.len()
        );
        assert_eq!(c.stdout, r.stdout, "e22: stdout differs for shared-stdin payload");
        assert_eq!(c.code, r.code, "e22: exit status differs for shared-stdin payload");
    }
}

/// E23: everything after the second conversion is ignored — the trailing input is
/// never consumed and never diagnosed.
#[test]
fn e23_extra_trailing_input_ignored() {
    for input in [
        &b"5 6 7 8 9"[..],
        b"5 6junk",
        b"0 4 the rest is ignored",
        b"2 3\n4 5\n6 7\n",
        b"1 2 -99999999999999999999",
        b"0 1 \x00\x01\x02",
    ] {
        assert_same("e23 trailing input", input);
    }
    // Trailing garbage cannot change the outcome relative to the bare pair.
    let bare = run(&c_bin(), b"5 6").stdout;
    let trailing = run(&c_bin(), b"5 6 7 8 9").stdout;
    assert_eq!(bare, trailing, "e23: the C reference consumed a third token");
    assert_same_expecting("e23 pinned", b"5 6 7 8 9", &bare);
}

// ---------------------------------------------------------------------------
// Generic API boundaries that every C program has, beyond the table rows.

/// There is no pointer-taking entry point to pass NULL to (`foo` is `static` and
/// nothing is exported — see `SYMBOLS.md`), so the analogous "absent input" cases
/// are an absent stream and absent bytes.  Both must be handled identically.
#[test]
fn boundary_absent_and_empty_inputs() {
    assert_same_with("boundary /dev/null", In::Path("/dev/null"), Out::Pipe, DEFAULT_TIMEOUT_SECS);
    assert_same_with("boundary empty pipe", In::Bytes(b""), Out::Pipe, DEFAULT_TIMEOUT_SECS);
    assert_same_with("boundary empty file", In::File(b""), Out::Pipe, DEFAULT_TIMEOUT_SECS);
    // Only one of the two conversions can be satisfied.
    assert_same("boundary one operand", b"4");
    assert_same("boundary zero operands", b"?");
}

/// Oversized inputs: a single token far larger than any buffer, and a payload far
/// larger than the pipe capacity.
#[test]
fn boundary_oversized_inputs() {
    let digits = "1".repeat(1 << 20);
    assert_same("boundary 1MiB token", format!("{digits} 2").as_bytes());
    let ws = " ".repeat(1 << 20);
    assert_same("boundary 1MiB whitespace", format!("{ws}2 3").as_bytes());
    let junk = "z".repeat(1 << 20);
    assert_same("boundary 1MiB junk", junk.as_bytes());
    assert_same("boundary 1MiB junk after pair", format!("2 3 {junk}").as_bytes());
}

/// Every byte value in the 0..=255 range, as the first byte of the stream: the
/// exhaustive version of "a value with no valid variant crosses the boundary".
#[test]
fn boundary_every_leading_byte() {
    for b in 0u8..=255 {
        let input = [b, b'7', b' ', b'2'];
        assert_same(&format!("boundary leading byte {b:#04x}"), &input);
    }
}

/// Every byte value as the separator between the two integers.
#[test]
fn boundary_every_separator_byte() {
    for b in 0u8..=255 {
        let input = [b'2', b, b'3'];
        // A '-' separator makes y negative, i.e. the unbounded overflow class.
        if b == b'-' {
            assert_same_prefix(&format!("boundary sep {b:#04x}"), &input, 64 * 1024);
        } else {
            assert_same(&format!("boundary sep {b:#04x}"), &input);
        }
    }
}
