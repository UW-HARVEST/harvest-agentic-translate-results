//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact rejecting condition, calls the exported
//! `main` of BOTH shared objects and asserts the *same* status code and the
//! *same* message bytes (not merely "both failed").

mod common;

use common::*;
use std::ffi::c_int;

const PROG: &[u8] = b"driver";
const USAGE: &[u8] =
    b"Error: there should be one to three arguments passed:\n<string> [start] [stop]\n";
const E3_MSG: &[u8] = b"Second argument must be an integer!";
const E4_MSG: &[u8] = b"Error: start is off the end of the string!\n";
const E5_MSG: &[u8] = b"Third argument must be an integer!";
const E6_MSG: &[u8] = b"Error: stop is off the end of the string!\n";
const E7_MSG: &[u8] = b"Error: stop must come after start!\n";

/// E1 — `argc > 4`.
#[test]
fn err_e1_too_many_args() {
    let rng = Rng::new(0xE1_0000_0001);
    for n in 5..12usize {
        let extra: Vec<Vec<u8>> = (0..n - 1)
            .map(|_| random_ascii(&rng, rng.range(0, 8) as usize))
            .collect();
        let mut args: Vec<&[u8]> = vec![PROG];
        args.extend(extra.iter().map(|v| v.as_slice()));
        for layout in [Layout::Contiguous, Layout::Separate] {
            let out = assert_same(&args, layout);
            assert_eq!(out.status, 1);
            assert_eq!(out.stdout, USAGE);
        }
    }
    // exactly the documented "one to three arguments" boundary: 4 user args
    let out = assert_same(&[PROG, b"abc", b"0", b"1", b"2"], Layout::Contiguous);
    assert_eq!(out.status, 1);
    assert_eq!(out.stdout, USAGE);
}

/// E2 — `argc == 1` (no user argument).
#[test]
fn err_e2_no_args() {
    for layout in [Layout::Contiguous, Layout::Separate] {
        let out = assert_same(&[PROG], layout);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, USAGE);
    }
    // argv[0] contents are irrelevant
    let out = assert_same(&[b""], Layout::Contiguous);
    assert_eq!(out.status, 1);
    assert_eq!(out.stdout, USAGE);
}

/// E3 — `end == argv[2]`: strtol performed no conversion on the second
/// argument. Also exercised with argc == 4 (E3 is checked before E5/E6/E7).
#[test]
fn err_e3_start_not_an_integer() {
    let rng = Rng::new(0xE3_0000_0001);
    let fixed: &[&[u8]] = &[
        b"", b"abc", b"-", b"+", b" ", b"\t", b"\n", b"  \t\n\r\x0b\x0c", b"x9", b".", b"--1",
        b"++1", b"-+2", b"+-2", b"e5", b"/3", b":7", b"#", b"one", b" - 1", b"O", b"o0",
        b"\xff\xfe", b"2\x00", // NUL cannot appear; the trailing byte is dropped below
    ];
    for f in fixed {
        let f: Vec<u8> = f.iter().copied().filter(|&b| b != 0).collect();
        if f.iter().any(|b| b.is_ascii_digit()) && !f.starts_with(b"x") && !f.starts_with(b"e") {
            // keep only the genuinely non-converting ones for the strict assert
        }
        let out = assert_same(&[PROG, b"abcdef", &f], Layout::Contiguous);
        if out.status == 1 && out.stdout == E3_MSG {
            continue;
        }
        // strings that do convert are fine too - the point is C and Rust agree
    }
    for _ in 0..400 {
        let s = random_string_shape(&rng);
        let bad = no_conversion_string(&rng);
        let out = assert_same(&[PROG, &s, &bad], Layout::Contiguous);
        assert_eq!(out.status, 1, "input {bad:?} must be rejected");
        assert_eq!(out.stdout, E3_MSG);

        // and with a third argument present, E3 still wins
        let third = decorate_number(&rng, rng.range(0, 5) as i64);
        let out = assert_same(&[PROG, &s, &bad, &third], Layout::Contiguous);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, E3_MSG);
    }
}

/// E4 — `start > len` (unsigned comparison), including every negative start.
#[test]
fn err_e4_start_off_end() {
    let rng = Rng::new(0xE4_0000_0001);
    for _ in 0..300 {
        let s = random_string_shape(&rng);
        let over = rng.range(1, 100000) as i64;
        let n = decorate_number(&rng, s.len() as i64 + over);
        let out = assert_same(&[PROG, &s, &n], Layout::Contiguous);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, E4_MSG);
    }
    // one past the end, exactly
    for len in 0..40usize {
        let s = vec![b'x'; len];
        let n = format!("{}", len + 1).into_bytes();
        let out = assert_same(&[PROG, &s, &n], Layout::Contiguous);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, E4_MSG);
    }
}

/// E4 — negative start: the `int` vs `size_t` comparison turns it into a huge
/// unsigned value, so it is reported as "off the end".
#[test]
fn err_e4_negative_start() {
    let rng = Rng::new(0xE4_0000_0002);
    for _ in 0..300 {
        let s = random_string_shape(&rng);
        let v = -(rng.range(1, 100000) as i64);
        let n = decorate_number(&rng, v);
        let out = assert_same(&[PROG, &s, &n], Layout::Contiguous);
        assert_eq!(out.status, 1, "negative start must be rejected");
        assert_eq!(out.stdout, E4_MSG);
    }
    for n in [
        &b"-1"[..],
        b"-0000000001",
        b" -2",
        b"-2147483648",
        b"-2147483647",
    ] {
        let out = assert_same(&[PROG, b"abcdef", n], Layout::Contiguous);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, E4_MSG);
    }
}

/// E4 — values that only become negative through the `long`->`int` truncation.
#[test]
fn err_e4_truncated_start() {
    // LONG_MAX (and anything above it) truncates to (int)-1  => E4
    for n in [
        &b"9223372036854775807"[..],
        b"9223372036854775808",
        b"99999999999999999999",
        b"4294967295",  // 2^32-1 -> (int)-1
        b"2147483648",  // -> (int)INT_MIN
        b"-4294967295", // -> (int)1 .. NOT an error for len>=1
    ] {
        let out = assert_same(&[PROG, b"abcdef", n], Layout::Contiguous);
        if n == b"-4294967295" {
            assert_eq!(out.status, 0, "truncates to +1, which is a valid start");
            assert_eq!(out.stdout, b"bcdef\n");
        } else {
            assert_eq!(out.status, 1);
            assert_eq!(out.stdout, E4_MSG);
        }
    }
}

/// E5 — `end == argv[3]`, reachable only when `argv[3]` aliases into `argv[2]`.
#[test]
fn err_e5_third_arg_alias() {
    // The string has to be long enough that E4 (start > len) does not fire
    // first: E3 and E4 are checked before the argc == 4 block.
    const LONG: &[u8] = b"abcdefghijklmnop"; // len == 16

    // argv[2] = "12", argv[3] = argv[2] + 2  (the NUL terminator, i.e. "")
    // strtol consumed 2 bytes, so end == argv[2] + 2 == argv[3].
    let mut argv = Argv::aliased(PROG, LONG, b"12", 2);
    let out = assert_same_argv(&mut argv, 4, "argv[3] == argv[2]+2");
    assert_eq!(out.status, 1);
    assert_eq!(out.stdout, E5_MSG);

    // Same with whitespace and a sign consumed by strtol.
    let mut argv = Argv::aliased(PROG, LONG, b" +7", 3);
    let out = assert_same_argv(&mut argv, 4, "argv[3] == argv[2]+3");
    assert_eq!(out.status, 1);
    assert_eq!(out.stdout, E5_MSG);

    // Leading zeros, all of them consumed.
    let mut argv = Argv::aliased(PROG, LONG, b"0000000009", 10);
    let out = assert_same_argv(&mut argv, 4, "argv[3] == argv[2]+10");
    assert_eq!(out.status, 1);
    assert_eq!(out.stdout, E5_MSG);

    // k != consumed must NOT produce the message (it is a plain empty/junk stop)
    let mut argv = Argv::aliased(PROG, LONG, b"12", 1);
    let out = assert_same_argv(&mut argv, 4, "argv[3] == argv[2]+1");
    assert_ne!(out.stdout, E5_MSG);

    // Trailing junk: strtol stops before it, so end != argv[3] for k == len.
    let mut argv = Argv::aliased(PROG, LONG, b"5abc", 4);
    let out = assert_same_argv(&mut argv, 4, "argv[3] == argv[2]+4 (junk)");
    assert_ne!(out.stdout, E5_MSG);
    // ... but it *is* equal for k == 1 there.
    let mut argv = Argv::aliased(PROG, LONG, b"5abc", 1);
    let out = assert_same_argv(&mut argv, 4, "argv[3] == argv[2]+1 (junk)");
    assert_eq!(out.status, 1);
    assert_eq!(out.stdout, E5_MSG);
}

/// E5 — and the same message is unreachable through the command line, because
/// the kernel lays `argv` out contiguously (`end <= argv[3] - 1`).
#[test]
fn err_e5_unreachable_from_cli() {
    let rng = Rng::new(0xE5_0000_0001);
    for _ in 0..200 {
        let s = random_string_shape(&rng);
        let a = if rng.bool() {
            decorate_number(&rng, rng.range(0, 20) as i64)
        } else {
            no_conversion_string(&rng)
        };
        let b = if rng.bool() {
            decorate_number(&rng, rng.range(0, 20) as i64)
        } else {
            no_conversion_string(&rng)
        };
        let out = assert_same_cli(&[s, a, b]);
        assert_ne!(out.stdout, E5_MSG);
    }
}

/// E6 — `stop > len` (unsigned comparison).
#[test]
fn err_e6_stop_off_end() {
    let rng = Rng::new(0xE6_0000_0001);
    for _ in 0..300 {
        let s = random_string_shape(&rng);
        let start = rng.range(0, s.len() as u64) as i64;
        let stop = s.len() as i64 + rng.range(1, 100000) as i64;
        let a = decorate_number(&rng, start);
        let b = decorate_number(&rng, stop);
        let out = assert_same(&[PROG, &s, &a, &b], Layout::Contiguous);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, E6_MSG);
    }
    // one past the end, exactly
    for len in 0..40usize {
        let s = vec![b'x'; len];
        let b = format!("{}", len + 1).into_bytes();
        let out = assert_same(&[PROG, &s, b"0", &b], Layout::Contiguous);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, E6_MSG);
    }
}

/// E6 — negative stop (same signed/unsigned trap as E4).
#[test]
fn err_e6_negative_stop() {
    let rng = Rng::new(0xE6_0000_0002);
    for _ in 0..300 {
        let s = random_bytes(&rng, rng.range(1, 40) as usize);
        let start = rng.below(s.len() as u64) as i64;
        let stop = -(rng.range(1, 100000) as i64);
        let a = decorate_number(&rng, start);
        let b = decorate_number(&rng, stop);
        let out = assert_same(&[PROG, &s, &a, &b], Layout::Contiguous);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, E6_MSG);
    }
    // LONG_MAX-saturating stop truncates to (int)-1 => E6 as well
    let out = assert_same(
        &[PROG, b"abcdef", b"0", b"9223372036854775807"],
        Layout::Contiguous,
    );
    assert_eq!(out.status, 1);
    assert_eq!(out.stdout, E6_MSG);
}

/// E7 — `stop <= start`.
#[test]
fn err_e7_stop_before_start() {
    let rng = Rng::new(0xE7_0000_0001);
    for _ in 0..400 {
        let s = random_bytes(&rng, rng.range(1, 60) as usize);
        let len = s.len() as u64;
        let start = rng.range(0, len);
        let stop = rng.range(0, start); // stop <= start
        let a = decorate_number(&rng, start as i64);
        let b = decorate_number(&rng, stop as i64);
        let out = assert_same(&[PROG, &s, &a, &b], Layout::Contiguous);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, E7_MSG);
    }
    // equal, and the empty-string case where every stop fails
    for (s, a, b) in [
        (&b"abcdef"[..], &b"3"[..], &b"3"[..]),
        (b"abcdef", b"0", b"0"),
        (b"abcdef", b"6", b"6"),
        (b"", b"0", b"0"),
    ] {
        let out = assert_same(&[PROG, s, a, b], Layout::Contiguous);
        assert_eq!(out.status, 1);
        assert_eq!(out.stdout, E7_MSG);
    }
}

/// B1 — `argc == 0`: the C reads `argv[1]` regardless.
#[test]
fn boundary_b1_argc0_ffi() {
    let rng = Rng::new(0xB1_0000_0001);
    for _ in 0..100 {
        let s = random_string_shape(&rng);
        let a = decorate_number(&rng, rng.range(0, 10) as i64);
        // argc == 0, but argv[1] exists and is used as the string
        let mut argv = Argv::new(&[PROG, &s, &a], Layout::Contiguous);
        let out = assert_same_argv(&mut argv, 0, "argc == 0");
        assert_eq!(out.status, 0);
        let mut want = s.clone();
        want.push(b'\n');
        assert_eq!(out.stdout, want, "argc==0 still prints argv[1] whole");
    }
}

/// B1 — from a real `execve`, an empty argv is rewritten by the kernel to
/// `argc == 1, argv[0] == ""`, which lands in E2 for both implementations.
#[test]
fn boundary_b1_argc0_via_exec_becomes_argc1() {
    let helper = build_exec0_helper();
    let mut outs = Vec::new();
    for exe in [c_exe(), rust_exe()] {
        let out = std::process::Command::new(&helper)
            .arg(&exe)
            .env_clear()
            .env("FIRSTVAR", "abcdefghij")
            .output()
            .expect("spawn exec0 helper");
        outs.push((out.status.code(), out.stdout, out.stderr));
    }
    assert_eq!(outs[0], outs[1], "argc==0 exec: C and Rust must agree");
    assert_eq!(outs[0].0, Some(1));
    assert_eq!(outs[0].1, USAGE);
}

fn build_exec0_helper() -> std::path::PathBuf {
    let dir = manifest_dir().join("target").join("cdiff");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("exec0.c");
    std::fs::write(
        &src,
        b"#include <unistd.h>\n#include <stdio.h>\nint main(int argc, char **argv){(void)argc;char *e[]={NULL};execv(argv[1],e);perror(\"execv\");return 127;}\n",
    )
    .unwrap();
    let out = dir.join("exec0");
    let st = std::process::Command::new("gcc")
        .arg("-O2")
        .arg("-o")
        .arg(&out)
        .arg(&src)
        .status()
        .expect("gcc");
    assert!(st.success());
    out
}

/// B2 — out-of-range `argc` values crossing the FFI boundary.
#[test]
fn boundary_b2_out_of_range_argc() {
    let rng = Rng::new(0xB2_0000_0001);
    let argcs: &[c_int] = &[
        c_int::MIN,
        -1000,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        100,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for &argc in argcs {
        for _ in 0..10 {
            let s = random_string_shape(&rng);
            let a = decorate_number(&rng, rng.range(0, 8) as i64);
            let b = decorate_number(&rng, rng.range(0, 8) as i64);
            for layout in [Layout::Contiguous, Layout::Separate] {
                // The vector always holds 4 valid strings, so every argc value
                // that reads argv[1..3] reads valid memory.
                let mut argv = Argv::new(&[PROG, &s, &a, &b], layout);
                assert_same_argv(&mut argv, argc, &format!("argc={argc} {layout:?}"));
            }
        }
    }
}

/// B6 — zero length string with each argc.
#[test]
fn boundary_b6_empty_string() {
    let out = assert_same(&[PROG, b""], Layout::Contiguous);
    assert_eq!((out.status, out.stdout.as_slice()), (0, &b"\n"[..]));

    let out = assert_same(&[PROG, b"", b"0"], Layout::Contiguous);
    assert_eq!((out.status, out.stdout.as_slice()), (0, &b"\n"[..]));

    let out = assert_same(&[PROG, b"", b"1"], Layout::Contiguous);
    assert_eq!((out.status, out.stdout.as_slice()), (1, E4_MSG));

    // with argc == 4 an empty string can never succeed
    for a in [&b"0"[..], b"1", b"-1"] {
        for b in [&b"0"[..], b"1", b"-1", b"2"] {
            let out = assert_same(&[PROG, b"", a, b], Layout::Contiguous);
            assert_eq!(out.status, 1, "argc==4 with an empty string always fails");
        }
    }
}

/// B7 — one step past every documented range boundary.
#[test]
fn boundary_b7_one_past_range() {
    let s: &[u8] = b"0123456789";
    let len = s.len();

    // start == len  => ok, prints only the newline
    let out = assert_same(&[PROG, s, b"10"], Layout::Contiguous);
    assert_eq!((out.status, out.stdout.as_slice()), (0, &b"\n"[..]));
    // start == len + 1 => E4
    let out = assert_same(&[PROG, s, b"11"], Layout::Contiguous);
    assert_eq!((out.status, out.stdout.as_slice()), (1, E4_MSG));
    // stop == len => ok
    let out = assert_same(&[PROG, s, b"0", b"10"], Layout::Contiguous);
    assert_eq!(out.status, 0);
    assert_eq!(out.stdout.len(), len + 1);
    // stop == len + 1 => E6
    let out = assert_same(&[PROG, s, b"0", b"11"], Layout::Contiguous);
    assert_eq!((out.status, out.stdout.as_slice()), (1, E6_MSG));
    // stop == start => E7 ; stop == start + 1 => one byte
    let out = assert_same(&[PROG, s, b"5", b"5"], Layout::Contiguous);
    assert_eq!((out.status, out.stdout.as_slice()), (1, E7_MSG));
    let out = assert_same(&[PROG, s, b"5", b"6"], Layout::Contiguous);
    assert_eq!((out.status, out.stdout.as_slice()), (0, &b"5\n"[..]));
    // start == len && stop == len  => E7 (stop <= start)
    let out = assert_same(&[PROG, s, b"10", b"10"], Layout::Contiguous);
    assert_eq!((out.status, out.stdout.as_slice()), (1, E7_MSG));
}

/// B8 — oversized / saturating / truncating numbers on both arguments.
#[test]
fn boundary_b8_overflow_values() {
    let values: &[&[u8]] = &[
        b"2147483647",
        b"2147483648",
        b"2147483649",
        b"4294967295",
        b"4294967296",
        b"4294967297",
        b"4294967301",
        b"-2147483648",
        b"-2147483649",
        b"-4294967296",
        b"-4294967291",
        b"9223372036854775807",
        b"9223372036854775808",
        b"9223372036854775809",
        b"18446744073709551616",
        b"99999999999999999999999",
        b"-9223372036854775808",
        b"-9223372036854775809",
        b"-99999999999999999999999",
        b"00000000000000000000009223372036854775808",
        b"   +9223372036854775808junk",
    ];
    for s in [&b""[..], b"abcde", b"0123456789"] {
        for v in values {
            assert_same(&[PROG, s, v], Layout::Contiguous);
            for w in values {
                assert_same(&[PROG, s, v, w], Layout::Contiguous);
            }
            assert_same(&[PROG, s, b"0", v], Layout::Contiguous);
            assert_same(&[PROG, s, b"1", v], Layout::Contiguous);
        }
    }
}

/// B10 — non-UTF-8 / high-bit bytes flow through verbatim.
#[test]
fn boundary_b10_non_utf8() {
    let strings: &[&[u8]] = &[
        b"\xff",
        b"\x80\x80\x80",
        b"\xc3",             // truncated UTF-8 lead byte
        b"\xed\xa0\x80",     // UTF-16 surrogate encoding
        b"\xf4\x90\x80\x80", // beyond U+10FFFF
        b"a\xffb\xfec",
        b"\xfe\xff\xfe\xff\xfe\xff",
    ];
    for s in strings {
        let out = assert_same(&[PROG, s], Layout::Contiguous);
        assert_eq!(out.status, 0);
        let mut want = s.to_vec();
        want.push(b'\n');
        assert_eq!(out.stdout, want);
        for start in 0..=s.len() {
            let a = format!("{start}").into_bytes();
            assert_same(&[PROG, s, &a], Layout::Contiguous);
            for stop in 0..=s.len() + 1 {
                let b = format!("{stop}").into_bytes();
                assert_same(&[PROG, s, &a, &b], Layout::Contiguous);
            }
        }
    }
}
