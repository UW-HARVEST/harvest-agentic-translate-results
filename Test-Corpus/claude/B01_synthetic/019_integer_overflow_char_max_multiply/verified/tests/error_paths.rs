//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input / rejecting condition named in
//! the table, drives both the C `.so` and the Rust `.so` with it, and asserts
//! they reject *identically* (same bytes on stdout, same exit status, same
//! sentinel behaviour) — not merely "both failed somehow".
//!
//! `main.c` has no error codes: `printLine` signals rejection by printing
//! nothing, `goodB2G` by printing its diagnostic line, and `main` by silently
//! discarding `scanf`'s return value so `x` keeps its `0` initialiser and the
//! process still exits `0`. The assertions below therefore pin the exact
//! observable consequence of each rejection.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};

// ------------------------------------------------------------ printLine ----

/// E1 — `printLine(NULL)`: the `main.c:30` guard, expected to emit nothing.
#[test]
fn e1_print_line_null() {
    let pair = load_pair();
    assert_same(&pair, "E1 printLine(NULL)", |lib| unsafe {
        (lib.print_line)(std::ptr::null())
    });
    // Pin the absolute expectation too, so a "both print something" regression
    // cannot pass by accident.
    let c_out = capture_stdout(|| unsafe { (pair.c.print_line)(std::ptr::null()) });
    assert!(
        c_out.is_empty(),
        "C printLine(NULL) unexpectedly produced {}",
        escape(&c_out)
    );
    let r_out = capture_stdout(|| unsafe { (pair.rust.print_line)(std::ptr::null()) });
    assert!(
        r_out.is_empty(),
        "Rust printLine(NULL) produced {}",
        escape(&r_out)
    );
    // Repeated NULL calls must stay a no-op (no latent state).
    assert_same(&pair, "E1 printLine(NULL) x8", |lib| {
        for _ in 0..8 {
            unsafe { (lib.print_line)(std::ptr::null()) }
        }
    });
}

/// E2 — empty string: guard passes, exactly one newline.
#[test]
fn e2_print_line_empty() {
    let pair = load_pair();
    let s = cstring(b"");
    assert_same(&pair, "E2 printLine(\"\")", |lib| unsafe {
        (lib.print_line)(s.as_ptr() as *const c_char)
    });
    let out = capture_stdout(|| unsafe { (pair.rust.print_line)(s.as_ptr() as *const c_char) });
    assert_eq!(out, b"\n", "E2 expected exactly one newline");
}

/// E3 — payload that is not valid UTF-8: bytes must be copied verbatim.
#[test]
fn e3_print_line_non_utf8() {
    let pair = load_pair();
    let payloads: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xff],
        vec![0xfe, 0xff],
        vec![0xc3, 0x28],             // bad continuation
        vec![0xe0, 0x80, 0x80],       // overlong
        vec![0xed, 0xa0, 0x80],       // UTF-16 surrogate
        vec![0xf5, 0x80, 0x80, 0x80], // > U+10FFFF
        vec![0xf8, 0x88, 0x80, 0x80, 0x80],
        (1u8..=255).collect(), // every non-NUL byte at once
    ];
    for p in &payloads {
        let s = cstring(p);
        assert_same(&pair, &format!("E3 printLine({})", escape(p)), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
        // No lossy replacement allowed.
        let out = capture_stdout(|| unsafe { (pair.rust.print_line)(s.as_ptr() as *const c_char) });
        let mut want = p.clone();
        want.push(b'\n');
        assert_eq!(out, want, "E3 Rust altered the bytes for {}", escape(p));
    }
}

/// E4 — payload containing `printf` conversion specifiers: treated as data.
#[test]
fn e4_print_line_format_specifiers() {
    let pair = load_pair();
    for p in [
        &b"%s"[..],
        b"%n",
        b"%d",
        b"%%",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"100%",
        b"%",
        b"%1$s %2$n",
        b"%p %x %.99999f",
    ] {
        let s = cstring(p);
        assert_same(&pair, &format!("E4 printLine({})", escape(p)), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
        let out = capture_stdout(|| unsafe { (pair.rust.print_line)(s.as_ptr() as *const c_char) });
        let mut want = p.to_vec();
        want.push(b'\n');
        assert_eq!(out, want, "E4 format specifier was interpreted");
    }
}

/// E5 — payload larger than the stdio buffer: no truncation.
#[test]
fn e5_print_line_very_long() {
    let pair = load_pair();
    for &len in &[4096usize, 8192, 65536, 131072] {
        let bytes: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        let s = cstring(&bytes);
        assert_same(&pair, &format!("E5 printLine len={len}"), |lib| unsafe {
            (lib.print_line)(s.as_ptr() as *const c_char)
        });
        let out = capture_stdout(|| unsafe { (pair.rust.print_line)(s.as_ptr() as *const c_char) });
        assert_eq!(out.len(), len + 1, "E5 truncated at len={len}");
    }
}

// ----------------------------------------------------- printHexCharLine ----

/// E6 — negative `char` sign-extends: `ffffffff` / `fffffffe` / `ffffff80`.
#[test]
fn e6_print_hex_char_line_negative() {
    let pair = load_pair();
    for (v, want) in [
        (-1i8, &b"ffffffff\n"[..]),
        (-2, b"fffffffe\n"),
        (-16, b"fffffff0\n"),
        (-127, b"ffffff81\n"),
        (i8::MIN, b"ffffff80\n"),
    ] {
        assert_same(&pair, &format!("E6 printHexCharLine({v})"), |lib| unsafe {
            (lib.print_hex_char_line)(v as c_char)
        });
        let c_out = capture_stdout(|| unsafe { (pair.c.print_hex_char_line)(v as c_char) });
        assert_eq!(
            c_out, want,
            "E6 sanity: C output for {v} is not the documented sign-extension"
        );
    }
    // And the full negative half of the domain.
    for v in i8::MIN..0 {
        assert_same(&pair, &format!("E6 sweep {v}"), |lib| unsafe {
            (lib.print_hex_char_line)(v as c_char)
        });
    }
}

/// E7 — zero: `%02x` zero-pads to the minimum field width.
#[test]
fn e7_print_hex_char_line_zero() {
    let pair = load_pair();
    assert_same(&pair, "E7 printHexCharLine(0)", |lib| unsafe {
        (lib.print_hex_char_line)(0)
    });
    let out = capture_stdout(|| unsafe { (pair.rust.print_hex_char_line)(0) });
    assert_eq!(out, b"00\n", "E7 expected zero padding to width 2");
    // Also the other sub-width values, where padding is observable.
    for v in 0i8..16 {
        assert_same(&pair, &format!("E7 pad {v}"), |lib| unsafe {
            (lib.print_hex_char_line)(v as c_char)
        });
    }
}

/// E8 — values pushed across the FFI boundary that do not fit in a `char`.
///
/// This is the "out-of-range value with no valid variant" case for this API:
/// the only integral parameter is a `char`, and the x86-64 SysV ABI leaves the
/// upper bits of the argument register unspecified, so the callee reads the low
/// byte. Both implementations must agree on what that means.
#[test]
fn e8_print_hex_char_line_out_of_range_int() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE8);
    let mut values: Vec<c_int> = vec![
        0, 1, 127, 128, 129, 255, 256, 257, 300, -1, -127, -128, -129, -255, -256, -1000, 0x1234_5678,
        0x0000_00ff, 0x0000_0100, 0x7fff_ffff, -0x8000_0000, 0x7f, -0x7f,
    ];
    for _ in 0..500 {
        values.push(rng.next_u64() as u32 as i32);
    }
    for v in values {
        assert_same(
            &pair,
            &format!("E8 printHexCharLine(int {v:#x})"),
            |lib| unsafe { (lib.print_hex_char_line_as_int)(v) },
        );
    }
}

// ------------------------------------------------------------ bad / good ---

/// E9 — `bad()` always takes the overflowing branch.
#[test]
fn e9_bad_always_overflows() {
    let pair = load_pair();
    assert_same(&pair, "E9 bad()", |lib| unsafe { (lib.bad)() });
    let c_out = capture_stdout(|| unsafe { (pair.c.bad)() });
    assert_eq!(
        c_out, b"fffffffe\n",
        "E9 sanity: C bad() must show the CWE-190 overflow"
    );
    let r_out = capture_stdout(|| unsafe { (pair.rust.bad)() });
    assert_eq!(r_out, b"fffffffe\n", "E9 Rust bad() diverged");
}

/// E10 — `goodB2G`'s range check rejects `CHAR_MAX` (`127 < 63` is false).
#[test]
fn e10_good_b2g_rejects_large_value() {
    let pair = load_pair();
    const MSG: &[u8] = b"data value is too large to perform arithmetic safely.\n";
    let c_out = capture_stdout(|| unsafe { (pair.c.good)() });
    let r_out = capture_stdout(|| unsafe { (pair.rust.good)() });
    assert_eq!(escape(&c_out), escape(&r_out), "E10 good() diverged");
    assert!(
        c_out.ends_with(MSG),
        "E10 sanity: C good() must end with the rejection message, got {}",
        escape(&c_out)
    );
    assert!(
        r_out.ends_with(MSG),
        "E10 Rust good() is missing the rejection message, got {}",
        escape(&r_out)
    );
    // The doubling must NOT have happened: only goodG2B's "04" precedes it.
    assert_eq!(
        r_out.len(),
        3 + MSG.len(),
        "E10 goodB2G performed the unsafe doubling"
    );
}

/// E11 — `good()` runs `goodG2B` then `goodB2G`, in that order.
#[test]
fn e11_good_order() {
    let pair = load_pair();
    assert_same(&pair, "E11 good()", |lib| unsafe { (lib.good)() });
    let out = capture_stdout(|| unsafe { (pair.rust.good)() });
    assert_eq!(
        out,
        b"04\ndata value is too large to perform arithmetic safely.\n",
        "E11 order/content diverged"
    );
    assert_same(&pair, "E11 good() x16", |lib| {
        for _ in 0..16 {
            unsafe { (lib.good)() }
        }
    });
}

// ----------------------------------------------------------------- main ----

/// E12 — empty stdin: `scanf` returns EOF, `x` stays 0, exit status 0.
#[test]
fn e12_main_empty_stdin() {
    assert_same_stdin("E12 empty", b"");
    let r = run_main_via_so(&rust_so_path(), b"");
    assert_eq!(r.stdout, b"fffffffe\n", "E12 must fall into bad()");
    assert_eq!(r.status, Some(0), "E12 exit status must still be 0");
    let c = run_main_via_so(&c_so_path(), b"");
    assert_eq!(c.status, Some(0), "E12 sanity: C exits 0");
}

/// E12b — stdin that cannot be read at all (not merely empty): closed fd 0,
/// `/dev/null`, and a directory opened as stdin (`read()` fails with `EISDIR`).
///
/// C distinguishes none of these: `scanf` reports `EOF` for both an input
/// failure and a read error, the return value is discarded, and `x` keeps `0`.
/// The Rust `Stdin::next_byte` must map a read *error* to the same outcome as
/// EOF, not propagate or panic.
#[test]
fn e12b_main_unreadable_stdin() {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let cases: Vec<(&str, Box<dyn Fn(&mut Command)>)> = vec![
        (
            "/dev/null",
            Box::new(|c: &mut Command| {
                c.stdin(Stdio::from(
                    std::fs::File::open("/dev/null").expect("open /dev/null"),
                ));
            }),
        ),
        (
            "directory as stdin",
            Box::new(|c: &mut Command| {
                c.stdin(Stdio::from(
                    std::fs::File::open(manifest_dir()).expect("open manifest dir"),
                ));
            }),
        ),
        (
            "closed stdin",
            Box::new(|c: &mut Command| {
                c.stdin(Stdio::null());
                unsafe {
                    c.pre_exec(|| {
                        // Actually close fd 0 in the child.
                        libc::close(0);
                        Ok(())
                    });
                }
            }),
        ),
    ];

    for (what, setup) in cases {
        let run = |exe: std::path::PathBuf| {
            let mut cmd = Command::new(exe);
            setup(&mut cmd);
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
            let out = cmd.output().expect("spawn");
            (out.stdout, out.stderr, out.status.code())
        };
        let c = run(c_exe_path());
        let r = run(rust_exe_path());
        assert_eq!(
            (escape(&c.0), escape(&c.1), c.2),
            (escape(&r.0), escape(&r.1), r.2),
            "E12b mismatch for {what}"
        );
        assert_eq!(c.0, b"fffffffe\n", "E12b {what}: C must fall into bad()");
        assert_eq!(c.2, Some(0), "E12b {what}: exit status must be 0");
    }
}

/// E13 — whitespace-only stdin: all skipped, then EOF.
#[test]
fn e13_main_whitespace_only() {
    let mut cases: Vec<Vec<u8>> = vec![
        b" ".to_vec(),
        b"\n".to_vec(),
        b"\t".to_vec(),
        vec![0x0b],
        vec![0x0c],
        b"\r".to_vec(),
        b" \t\n\x0b\x0c\r".to_vec(),
        vec![b' '; 5000],
    ];
    cases.push(C_WHITESPACE.repeat(300));
    for c in &cases {
        assert_same_stdin(&format!("E13 {}", escape(c)), c);
        let r = run_main_via_so(&rust_so_path(), c);
        assert_eq!(r.stdout, b"fffffffe\n", "E13 must fall into bad()");
        assert_eq!(r.status, Some(0));
    }
}

/// E14 — matching failure: first non-space byte is neither digit nor sign.
#[test]
fn e14_main_matching_failure() {
    for c in [
        &b"abc"[..],
        b"x",
        b".",
        b".5",
        b"e5",
        b"'",
        b"*7",
        b"/",
        b":",
        b"\x00",
        b"\x005",
        b"0x10", // `%d` is base 10: stops after the `0`
        b"  \n\txyz",
        b"\xff\xfe",
        b"[]",
    ] {
        assert_same_stdin(&format!("E14 {}", escape(c)), c);
    }
    // `0x10` and `abc` must both land in bad(), but for different reasons; pin
    // the shared observable outcome.
    for c in [&b"abc"[..], b"0x10"] {
        let r = run_main_via_so(&rust_so_path(), c);
        assert_eq!(r.stdout, b"fffffffe\n", "E14 {} must reach bad()", escape(c));
        assert_eq!(r.status, Some(0));
    }
}

/// E15 — sign with no digits after it.
#[test]
fn e15_main_sign_without_digits() {
    for c in [
        &b"-"[..], b"+", b"- 5", b"+ 5", b"+abc", b"-abc", b"--5", b"++5", b"+-5", b"-+5", b"-\n5",
        b"+.", b"   -", b"\x0b+",
    ] {
        assert_same_stdin(&format!("E15 {}", escape(c)), c);
        let r = run_main_via_so(&rust_so_path(), c);
        assert_eq!(
            r.stdout,
            b"fffffffe\n",
            "E15 {} must reach bad()",
            escape(c)
        );
        assert_eq!(r.status, Some(0));
    }
}

/// E16 — the vertical tab (0x0B) *is* C whitespace, and must be skipped.
///
/// Rust's `u8::is_ascii_whitespace()` excludes 0x0B, so a translation that uses
/// it diverges here: C reaches `good()`, the naive Rust reaches `bad()`.
#[test]
fn e16_main_vertical_tab_is_whitespace() {
    for c in [
        &b"\x0b5"[..],
        b"\x0b\x0b\x0b5",
        b"\x0b-5",
        b"\x0b+5",
        b"\x0b0",
        b" \x0b\t5",
        b"\x0b\x0c\r\n\t 42",
    ] {
        assert_same_stdin(&format!("E16 {}", escape(c)), c);
    }
    // The decisive one: 0x0B then a nonzero digit must reach good().
    let r = run_main_via_so(&rust_so_path(), b"\x0b5");
    assert_eq!(
        r.stdout, b"04\ndata value is too large to perform arithmetic safely.\n",
        "E16 vertical tab was not treated as whitespace"
    );
}

/// E17 — bytes that are not `isspace` in the C locale must NOT be skipped.
#[test]
fn e17_main_lookalike_space_bytes() {
    for b in [
        0x00u8, 0x01, 0x07, 0x08, 0x0e, 0x0f, 0x1c, 0x1d, 0x1e, 0x1f, 0x7f, 0x85, 0xa0, 0xc2, 0xff,
    ] {
        let input = vec![b, b'5'];
        assert_same_stdin(&format!("E17 0x{b:02x}"), &input);
        let r = run_main_via_so(&rust_so_path(), &input);
        assert_eq!(
            r.stdout,
            b"fffffffe\n",
            "E17 0x{b:02x} must NOT be treated as whitespace"
        );
    }
}

/// E18 — above `LONG_MAX`: saturates to `LONG_MAX`, truncates to `-1` (nonzero).
#[test]
fn e18_main_above_long_max() {
    let long_run = format!("{}", "9".repeat(100));
    for c in [
        "9223372036854775808",
        "9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "99999999999999999999",
        long_run.as_str(),
        "+99999999999999999999",
    ] {
        assert_same_stdin(&format!("E18 {c}"), c.as_bytes());
        let r = run_main_via_so(&rust_so_path(), c.as_bytes());
        assert_eq!(
            r.stdout, b"04\ndata value is too large to perform arithmetic safely.\n",
            "E18 {c}: LONG_MAX truncated to int is -1, which is nonzero -> good()"
        );
    }
}

/// E19 — below `LONG_MIN`: saturates to `LONG_MIN`, truncates to `0` (false!).
#[test]
fn e19_main_below_long_min() {
    let long_run = format!("-{}", "9".repeat(100));
    for c in [
        "-9223372036854775809",
        "-18446744073709551616",
        "-99999999999999999999",
        long_run.as_str(),
    ] {
        assert_same_stdin(&format!("E19 {c}"), c.as_bytes());
        let r = run_main_via_so(&rust_so_path(), c.as_bytes());
        assert_eq!(
            r.stdout, b"fffffffe\n",
            "E19 {c}: LONG_MIN truncated to int is 0 -> bad()"
        );
    }
    // LONG_MIN exactly is 0x8000000000000000 too.
    assert_same_stdin("E19 -9223372036854775808", b"-9223372036854775808");
    let r = run_main_via_so(&rust_so_path(), b"-9223372036854775808");
    assert_eq!(r.stdout, b"fffffffe\n");
}

/// E20 — successful parse whose low 32 bits are zero still means `x == 0`.
#[test]
fn e20_main_low_32_bits_zero() {
    for c in [
        "4294967296",          // 2^32
        "8589934592",          // 2^33
        "-4294967296",
        "1099511627776",       // 2^40
        "4294967296000000000", // still 0 mod 2^32? -> checked differentially
        "9223372036854775808", // LONG_MAX+1 saturates, handled in E18
    ] {
        assert_same_stdin(&format!("E20 {c}"), c.as_bytes());
    }
    let r = run_main_via_so(&rust_so_path(), b"4294967296");
    assert_eq!(
        r.stdout, b"fffffffe\n",
        "E20 2^32 truncates to int 0 -> bad()"
    );
}

/// E21 — every spelling of zero.
#[test]
fn e21_main_explicit_zero_spellings() {
    for c in [
        &b"0"[..], b"-0", b"+0", b"00", b"0000", b"  0  ", b"\t0\n", b"\x0b0", b"-000", b"+000",
        b"0\n0", b"0abc", &[b'0'; 500],
    ] {
        assert_same_stdin(&format!("E21 {}", escape(c)), c);
        let r = run_main_via_so(&rust_so_path(), c);
        assert_eq!(r.stdout, b"fffffffe\n", "E21 {} must reach bad()", escape(c));
        assert_eq!(r.status, Some(0));
    }
}

/// E22 — `INT_MIN` / `INT_MAX` and one step past each.
#[test]
fn e22_main_int_boundaries() {
    for c in [
        "2147483646",
        "2147483647", // INT_MAX
        "2147483648", // INT_MAX + 1
        "2147483649",
        "-2147483647",
        "-2147483648", // INT_MIN
        "-2147483649", // INT_MIN - 1
        "-2147483650",
        "4294967295", // UINT_MAX -> int -1
        "-4294967295",
        "-1",
        "1",
    ] {
        assert_same_stdin(&format!("E22 {c}"), c.as_bytes());
        let r = run_main_via_so(&rust_so_path(), c.as_bytes());
        assert_eq!(
            r.stdout, b"04\ndata value is too large to perform arithmetic safely.\n",
            "E22 {c} truncates to a nonzero int -> good()"
        );
    }
}

/// E23 — digits followed immediately by junk: conversion already succeeded.
#[test]
fn e23_main_digits_then_junk() {
    for c in [
        &b"12abc"[..], b"5-", b"7.", b"1e5", b"3+4", b"9\x00", b"8\xff", b"2 junk", b"6)", b"4/2",
    ] {
        assert_same_stdin(&format!("E23 {}", escape(c)), c);
        let r = run_main_via_so(&rust_so_path(), c);
        assert_eq!(
            r.stdout, b"04\ndata value is too large to perform arithmetic safely.\n",
            "E23 {} parsed a nonzero prefix -> good()",
            escape(c)
        );
    }
}

/// E24 — long leading-zero runs.
#[test]
fn e24_main_long_leading_zero_runs() {
    let mut z400_1 = vec![b'0'; 400];
    z400_1.push(b'1');
    let z400 = vec![b'0'; 400];
    let mut z10000_7 = vec![b'0'; 10000];
    z10000_7.push(b'7');
    for c in [z400_1.as_slice(), z400.as_slice(), z10000_7.as_slice()] {
        assert_same_stdin(&format!("E24 len={}", c.len()), c);
    }
    let r = run_main_via_so(&rust_so_path(), &z400_1);
    assert_eq!(
        r.stdout, b"04\ndata value is too large to perform arithmetic safely.\n",
        "E24 400 zeros then 1 == 1 -> good()"
    );
    let r = run_main_via_so(&rust_so_path(), &z400);
    assert_eq!(r.stdout, b"fffffffe\n", "E24 400 zeros == 0 -> bad()");
}

/// E25 — stdin far larger than the stdio buffer.
#[test]
fn e25_main_oversized_stdin() {
    // Number first, then a lot of trailing data that must never be examined.
    let mut big = b"3".to_vec();
    big.extend(std::iter::repeat(b'z').take(200_000));
    assert_same_stdin("E25 number then 200k junk", &big);

    // Whitespace far exceeding the buffer, then the number.
    let mut ws = vec![b' '; 200_000];
    ws.push(b'4');
    assert_same_stdin("E25 200k spaces then number", &ws);

    // A very long digit run (well past the buffer) that saturates.
    let digits = vec![b'8'; 100_000];
    assert_same_stdin("E25 100k digits", &digits);

    // Junk far exceeding the buffer with no number at all.
    let junk = vec![b'q'; 200_000];
    assert_same_stdin("E25 200k junk", &junk);
}

/// E26 — recorded for completeness: this API has no `enum`-typed parameter, so
/// the "out-of-range enum variant across FFI" class collapses onto E8 (an
/// out-of-range integer in the argument register). Re-assert it here so the row
/// has its own passing test.
#[test]
fn e26_no_enum_parameters_out_of_range_int_is_the_analogue() {
    let pair = load_pair();
    // Values chosen so that *no* valid `char` produces them directly.
    for v in [
        i32::MIN,
        i32::MAX,
        0x0000_0080,
        0x0000_ff00,
        0xdead_beefu32 as i32,
        0x7fff_ff80,
        1 << 8,
        (1 << 8) + 5,
        -(1 << 8),
    ] {
        assert_same(&pair, &format!("E26 int {v:#x}"), |lib| unsafe {
            (lib.print_hex_char_line_as_int)(v)
        });
    }
}
