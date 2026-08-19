//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Channel `S` rows drive the *lowest-level* export (`printLine`) directly
//! through `libloading`, not just the `main` one-shot wrapper; channel `E` rows
//! drive the composed program end to end. Every row uses many property-style
//! randomized inputs from a fixed-seed SplitMix64 PRNG so that value-dependent
//! paths (accumulator overflow, low-word-zero truncation, …) are actually hit.

mod common;

use common::*;

// Per-row sample counts. Fixed seeds keep every run reproducible.
const N_ASCII: usize = 1000;
const N_BYTES: usize = 1000;
const N_PER_DIGIT_COUNT: usize = 20;
const N_I32: usize = 1200;
const N_I64: usize = 800;
const N_LONG_OVERFLOW: usize = 800;
const N_LOW_WORD: usize = 400;
const N_TERMINATORS: usize = 300;
const N_FUZZ_BYTES: usize = 1500;
const N_FUZZ_NUMERICISH: usize = 1500;

// ===========================================================================
// Rows 1-8: `printLine`, the lowest-level entry point, via the .so (channel S)
// ===========================================================================

/// Row 1 — non-NULL, length 0.
#[test]
fn cfg_printline_empty() {
    assert_so_print_line_same("row1/empty", Some(b""));
}

/// Row 2 — non-NULL, length 1, every byte value 1..=255.
#[test]
fn cfg_printline_single_byte_all_values() {
    for b in 1u8..=255 {
        let p = [b];
        let c = so_print_line(Side::C, Some(&p));
        let r = so_print_line(Side::Rust, Some(&p));
        assert_bytes_eq(&format!("row2/0x{b:02x}"), &p, &c, &r);
        assert_eq!(c, [b, b'\n']);
    }
}

/// Row 3 — random printable-ASCII payloads, length 1..=64.
#[test]
fn cfg_printline_random_ascii() {
    let mut rng = Rng::new(0x5EED_0003);
    for i in 0..N_ASCII {
        let len = rng.range(1, 64) as usize;
        let payload: Vec<u8> = (0..len).map(|_| rng.range(0x20, 0x7e) as u8).collect();
        let c = so_print_line(Side::C, Some(&payload));
        let r = so_print_line(Side::Rust, Some(&payload));
        assert_bytes_eq(&format!("row3/ascii#{i}"), &payload, &c, &r);
    }
}

/// Row 4 — random arbitrary non-NUL byte payloads (invalid UTF-8 included),
/// length 1..=256.
#[test]
fn cfg_printline_random_bytes() {
    let mut rng = Rng::new(0x5EED_0004);
    for i in 0..N_BYTES {
        let len = rng.range(1, 256) as usize;
        // Any byte except NUL, which would terminate the C string early.
        let payload: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        let c = so_print_line(Side::C, Some(&payload));
        let r = so_print_line(Side::Rust, Some(&payload));
        assert_bytes_eq(&format!("row4/bytes#{i}"), &payload, &c, &r);
    }
}

/// Row 5 — payloads built from embedded whitespace / control runs.
#[test]
fn cfg_printline_embedded_whitespace() {
    let ws = [b'\n', b'\r', b'\t', 0x0b, 0x0c, b' '];
    let mut rng = Rng::new(0x5EED_0005);
    for i in 0..200 {
        let len = rng.range(1, 40) as usize;
        let payload: Vec<u8> = (0..len)
            .map(|_| {
                if rng.bool() {
                    *rng.pick(&ws)
                } else {
                    rng.range(b'a' as u64, b'z' as u64) as u8
                }
            })
            .collect();
        let c = so_print_line(Side::C, Some(&payload));
        let r = so_print_line(Side::Rust, Some(&payload));
        assert_bytes_eq(&format!("row5/ws#{i}"), &payload, &c, &r);
    }
}

/// Row 6 — the payload is a printf format string; it must be treated as data.
#[test]
fn cfg_printline_format_specifiers() {
    let atoms: [&[u8]; 10] = [
        b"%s", b"%d", b"%n", b"%%", b"%999999d", b"%p", b"%x", b"%c", b"%lln",
        b"%.*f",
    ];
    let mut rng = Rng::new(0x5EED_0006);
    for i in 0..200 {
        let n = rng.range(1, 6) as usize;
        let mut payload = Vec::new();
        for _ in 0..n {
            payload.extend_from_slice(*rng.pick(&atoms));
            if rng.bool() {
                payload.push(b' ');
            }
        }
        let c = so_print_line(Side::C, Some(&payload));
        let r = so_print_line(Side::Rust, Some(&payload));
        assert_bytes_eq(&format!("row6/fmt#{i}"), &payload, &c, &r);
    }
}

/// Row 7 — sizes straddling stdio buffer boundaries, up to 1 MiB.
#[test]
fn cfg_printline_buffer_boundaries() {
    for len in [
        1usize, 2, 127, 128, 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192,
        8193, 65535, 65536, 65537, 1 << 20,
    ] {
        let payload: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        let c = so_print_line(Side::C, Some(&payload));
        let r = so_print_line(Side::Rust, Some(&payload));
        assert_bytes_eq(&format!("row7/len{len}"), &payload, &c, &r);
        assert_eq!(c.len(), len + 1, "payload + newline");
    }
}

/// Row 8 — 100 back-to-back `printLine` calls in one capture: outputs must
/// concatenate identically, i.e. there is no per-call hidden state.
#[test]
fn cfg_printline_repeated_calls() {
    let payloads: Vec<Vec<u8>> = (0..100)
        .map(|i| format!("line-{i}").into_bytes())
        .collect();
    let mut c_all = Vec::new();
    let mut r_all = Vec::new();
    for p in &payloads {
        c_all.extend(so_print_line(Side::C, Some(p)));
        r_all.extend(so_print_line(Side::Rust, Some(p)));
    }
    assert_bytes_eq("row8/repeated", b"", &c_all, &r_all);
    assert_eq!(c_all.iter().filter(|&&b| b == b'\n').count(), 100);
}

// ===========================================================================
// Rows 9-10: the other two leaf exports (channel S)
// ===========================================================================

/// Row 9 — `good()`, single and repeated.
#[test]
fn cfg_good_direct() {
    for times in [1usize, 2, 3, 100] {
        let c = so_call_void(Side::C, "good", times);
        let r = so_call_void(Side::Rust, "good", times);
        assert_bytes_eq(&format!("row9/good-x{times}"), b"", &c, &r);
        assert_eq!(c.len(), 7 * times, "`string\\n` per call");
    }
}

/// Row 10 — `bad()` called in isolation: the uninitialised-pointer UB.
///
/// The C side has no defined behaviour here (it has been observed printing
/// `"\n"`, printing its own machine code, and dying from `SIGSEGV` — all from
/// this one `main.c`), so only the Rust side can be pinned down. See ERRORS.md
/// row 22; the byte-exact differential assertion for the `bad()` path lives in
/// `cfg_so_main_bad_path` and the executable rows, where the C is reproducible.
#[test]
fn cfg_bad_direct_ub_unspecified() {
    for times in [1usize, 2, 10] {
        let (r, r_status) = so_call_bad_tolerant(Side::Rust, times);
        assert!(
            r_status.is_clean(),
            "Rust bad() must not crash, but {}",
            r_status.describe()
        );
        assert_eq!(
            r,
            "\n".repeat(times).into_bytes(),
            "Rust bad() must print exactly one newline per call"
        );

        // Record what the C did, without asserting on undefined behaviour.
        let (c, c_status) = so_call_bad_tolerant(Side::C, times);
        eprintln!(
            "row10 isolated C bad() x{times}: {} — {:?}",
            c_status.describe(),
            pretty(&c)
        );
    }
}

// ===========================================================================
// Rows 11-12: `main` through the .so (channel S, hermetic subprocess)
// ===========================================================================

/// Row 11 — `.so` `main`, scanf succeeds with a non-zero value (`good()` branch).
#[test]
fn cfg_so_main_good_path() {
    for s in [
        "1", "-1", "7", "2147483647", "-2147483648", "  42", "+7", "9999999999",
    ] {
        assert_so_main_same("row11/so-main-good", s.as_bytes());
    }
    let mut rng = Rng::new(0x5EED_0011);
    for _ in 0..40 {
        let v = rng.next_u32() as i32;
        if v == 0 {
            continue;
        }
        assert_so_main_same("row11/so-main-good-rand", v.to_string().as_bytes());
    }
}

/// Row 12 — `.so` `main`, the `bad()` branch (zero value, matching failure, EOF).
#[test]
fn cfg_so_main_bad_path() {
    for s in [
        "0", "-0", "+0", "000", "abc", "", "   ", "-", "+", "4294967296",
        "-9223372036854775809", "0x10",
    ] {
        assert_so_main_same("row12/so-main-bad", s.as_bytes());
    }
    let mut rng = Rng::new(0x5EED_0012);
    for _ in 0..40 {
        let k = rng.range(1, 1 << 20);
        let v = (k as u128) << 32; // low 32 bits zero => x == 0 => bad()
        assert_so_main_same("row12/so-main-bad-rand", v.to_string().as_bytes());
    }
}

// ===========================================================================
// Rows 13-25: `main` through the executables (channel E)
// ===========================================================================

/// Row 13 — one digit, no sign, no whitespace: all of `0..=9`.
#[test]
fn cfg_exe_single_digit() {
    for d in b'0'..=b'9' {
        assert_exe_same(&format!("row13/digit-{}", d as char), &[d]);
    }
}

/// Row 14 — every whitespace character `%d` skips, in runs of 1 / 3 / 1000,
/// before a zero and a non-zero value.
#[test]
fn cfg_exe_leading_whitespace_kinds() {
    for ws in [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'] {
        for n in [1usize, 3, 1000] {
            for tail in [&b"0"[..], b"5"] {
                let mut input = vec![ws; n];
                input.extend_from_slice(tail);
                assert_exe_same(&format!("row14/ws-{ws:#04x}-x{n}"), &input);
            }
        }
    }
    // Mixed whitespace kinds.
    let mut rng = Rng::new(0x5EED_0014);
    let ws = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
    for i in 0..100 {
        let n = rng.range(1, 30) as usize;
        let mut input: Vec<u8> = (0..n).map(|_| *rng.pick(&ws)).collect();
        input.extend_from_slice(rng.range(0, 9).to_string().as_bytes());
        assert_exe_same(&format!("row14/ws-mixed#{i}"), &input);
    }
}

/// Row 14b — **exhaustive** byte classification. For every one of the 256 byte
/// values, is it whitespace that `%d` skips, a sign, a digit, or a terminator?
/// Three probes per value pin down all four answers:
///
/// * `[b, '5']`     — a leading `b` that is skipped yields 5 (`good()`),
///                    otherwise the conversion fails (`bad()`).
/// * `['-', b, '5']` — a `b` between the sign and the digits.
/// * `['1', b, '2']` — a `b` right after the first digit: does the number
///                    continue, and does the terminator matter?
///
/// This is the test that catches a wrong `isspace()` set, which whitespace-only
/// inputs cannot distinguish (both a skip and a matching failure end up at
/// `x == 0`).
#[test]
fn cfg_exe_byte_classification_exhaustive() {
    for b in 0u8..=255 {
        assert_exe_same(&format!("row14b/lead-{b:#04x}"), &[b, b'5']);
        assert_exe_same(&format!("row14b/after-sign-{b:#04x}"), &[b'-', b, b'5']);
        assert_exe_same(&format!("row14b/after-digit-{b:#04x}"), &[b'1', b, b'2']);
    }
}

/// Row 14c — the same exhaustive sweep, but where the byte decides between the
/// `good()` and `bad()` branches through the *sign*: `[b, '-', '1']` and
/// `[b, '+', '1']`, plus a doubled leading byte to catch "only one whitespace
/// character is skipped" bugs.
#[test]
fn cfg_exe_byte_classification_sign_and_runs() {
    for b in 0u8..=255 {
        assert_exe_same(&format!("row14c/minus-{b:#04x}"), &[b, b'-', b'1']);
        assert_exe_same(&format!("row14c/plus-{b:#04x}"), &[b, b'+', b'1']);
        assert_exe_same(&format!("row14c/doubled-{b:#04x}"), &[b, b, b'7']);
    }
}

/// Row 15 — the sign axis crossed with magnitude.
#[test]
fn cfg_exe_sign_matrix() {
    for sign in ["", "+", "-"] {
        for mag in ["0", "1", "9", "10", "2147483647", "2147483648", "4294967296"] {
            assert_exe_same_str("row15/sign-matrix", &format!("{sign}{mag}"));
        }
    }
}

/// Row 16 — the digit-count axis, with random digits at each length.
#[test]
fn cfg_exe_digit_counts() {
    let mut rng = Rng::new(0x5EED_0016);
    for n in [1usize, 2, 3, 9, 10, 11, 18, 19, 20, 21, 39, 64, 1000, 5000] {
        for i in 0..N_PER_DIGIT_COUNT {
            let mut s: Vec<u8> = Vec::with_capacity(n + 1);
            if rng.bool() {
                s.push(if rng.bool() { b'-' } else { b'+' });
            }
            for k in 0..n {
                // Allow a leading zero sometimes, to exercise that path too.
                let lo = if k == 0 && !rng.bool() { b'1' } else { b'0' };
                s.push(rng.range(lo as u64, b'9' as u64) as u8);
            }
            assert_exe_same(&format!("row16/len{n}#{i}"), &s);
        }
    }
}

/// Row 17 — random values that fit in `int` (the common case).
#[test]
fn cfg_exe_random_i32() {
    let mut rng = Rng::new(0x5EED_0017);
    for i in 0..N_I32 {
        let v = rng.next_u32() as i32;
        assert_exe_same(&format!("row17/i32#{i}"), v.to_string().as_bytes());
    }
    for v in [0i32, 1, -1, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        assert_exe_same("row17/i32-edge", v.to_string().as_bytes());
    }
}

/// Row 18 — random values that fit in `long` but not in `int`, so `%d`
/// truncates.
#[test]
fn cfg_exe_random_i64_beyond_i32() {
    let mut rng = Rng::new(0x5EED_0018);
    let mut i = 0;
    while i < N_I64 {
        let v = rng.next_u64() as i64;
        if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
            continue; // that is row 17's job
        }
        assert_exe_same(&format!("row18/i64#{i}"), v.to_string().as_bytes());
        i += 1;
    }
    for v in [
        i64::MAX,
        i64::MIN,
        i64::MAX - 1,
        i64::MIN + 1,
        i32::MAX as i64 + 1,
        i32::MIN as i64 - 1,
        1i64 << 32,
        -(1i64 << 32),
    ] {
        assert_exe_same("row18/i64-edge", v.to_string().as_bytes());
    }
}

/// Row 19 — digit strings that overflow glibc's `long` accumulator, both signs.
#[test]
fn cfg_exe_random_long_overflow() {
    let mut rng = Rng::new(0x5EED_0019);
    for i in 0..N_LONG_OVERFLOW {
        let n = rng.range(20, 40) as usize;
        let mut s: Vec<u8> = Vec::new();
        if rng.bool() {
            s.push(if rng.bool() { b'-' } else { b'+' });
        }
        s.push(rng.range(b'1' as u64, b'9' as u64) as u8);
        for _ in 1..n {
            s.push(rng.range(b'0' as u64, b'9' as u64) as u8);
        }
        assert_exe_same(&format!("row19/overflow#{i}"), &s);
    }
}

/// Row 20 — non-zero `long` whose low 32-bit word is 0, so `x == 0` and the
/// `bad()` branch is taken even though the number is huge.
#[test]
fn cfg_exe_low_word_zero() {
    let mut rng = Rng::new(0x5EED_0020);
    for i in 0..N_LOW_WORD {
        let k = rng.range(1, 1 << 30);
        let v = (k as i128) << 32;
        let s = if rng.bool() {
            v.to_string()
        } else {
            (-v).to_string()
        };
        assert_exe_same(&format!("row20/lowzero#{i}"), s.as_bytes());
    }
}

/// Row 21 — the mirror of row 20: value > `INT_MAX` but with a non-zero low
/// word, so `good()` runs.
#[test]
fn cfg_exe_low_word_nonzero() {
    let mut rng = Rng::new(0x5EED_0021);
    for i in 0..N_LOW_WORD {
        let k = rng.range(1, 1 << 30);
        let r = rng.range(1, u32::MAX as u64);
        let v = ((k as i128) << 32) + r as i128;
        let s = if rng.bool() {
            v.to_string()
        } else {
            (-v).to_string()
        };
        assert_exe_same(&format!("row21/lownonzero#{i}"), s.as_bytes());
    }
}

/// Row 22 — the exact boundary constants of every magnitude class.
#[test]
fn cfg_exe_boundary_constants() {
    for s in [
        "0",
        "1",
        "-1",
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967295",
        "-4294967296",
        "-4294967297",
        "8589934592",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "18446744073709551617",
    ] {
        assert_exe_same_str("row22/boundary", s);
    }
}

/// Row 23 — leading zeros must not change the value or overflow the accumulator.
#[test]
fn cfg_exe_leading_zeros() {
    for s in ["0", "00", "007", "-007", "+007", "0000000000000000001"] {
        assert_exe_same_str("row23/leading-zeros", s);
    }
    for n in [64usize, 1000, 5000] {
        let mut z = vec![b'0'; n];
        assert_exe_same(&format!("row23/{n}-zeros"), &z);
        z.extend_from_slice(b"4294967296"); // low word 0 after the zeros
        assert_exe_same(&format!("row23/{n}-zeros+2^32"), &z);
        let mut z2 = vec![b'-'];
        z2.extend(std::iter::repeat(b'0').take(n));
        z2.extend_from_slice(b"5");
        assert_exe_same(&format!("row23/minus-{n}-zeros+5"), &z2);
    }
}

/// Row 24 — the terminator axis: what follows the converted number.
#[test]
fn cfg_exe_terminators() {
    let tails: [&[u8]; 12] = [
        b"", b"\n", b"\r\n", b" ", b"a", b".", b"-", b"+", b"\t", b"\x00",
        b" 12345", b"\nabc",
    ];
    let mut rng = Rng::new(0x5EED_0024);
    for i in 0..N_TERMINATORS {
        let v = rng.next_u32() as i32;
        let mut input = v.to_string().into_bytes();
        input.extend_from_slice(*rng.pick(&tails));
        assert_exe_same(&format!("row24/terminator#{i}"), &input);
    }
    // And exhaustively for a couple of fixed values.
    for v in ["0", "1", "-1"] {
        for t in tails {
            let mut input = v.as_bytes().to_vec();
            input.extend_from_slice(t);
            assert_exe_same("row24/terminator-fixed", &input);
        }
    }
}

/// Row 25 — several numbers on stdin: only the first `%d` conversion happens.
#[test]
fn cfg_exe_multiple_numbers() {
    for s in [
        "0 1", "1 0", "0\n1", "1\n0", "0 0 0 1", "1 1 1 0", "  0   7  ",
        "-0 5", "5 -0", "0\t\t9",
    ] {
        assert_exe_same_str("row25/multiple-numbers", s);
    }
}

// ===========================================================================
// Rows 26-30: stream plumbing (channel E)
// ===========================================================================

/// Rows 26/27 — the same inputs over a pipe (the default everywhere else) and
/// over a seekable regular file must behave identically.
#[test]
fn cfg_exe_stdin_regular_file() {
    for s in ["0", "1", "-1", "abc", "", "   7", "4294967296"] {
        assert_exe_same_with(
            "row27/stdin-file",
            s.as_bytes(),
            StdinKind::File,
            StdoutKind::Pipe,
        );
        assert_exe_same_with(
            "row26/stdin-pipe",
            s.as_bytes(),
            StdinKind::Pipe,
            StdoutKind::Pipe,
        );
    }
    // A large input via each plumbing.
    let big = vec![b'9'; 5000];
    assert_exe_same_with("row27/stdin-file-big", &big, StdinKind::File, StdoutKind::Pipe);
}

/// Row 28 — fd 0 is `/dev/null`: immediate EOF.
#[test]
fn cfg_exe_stdin_devnull() {
    assert_exe_same_with(
        "row28/stdin-devnull",
        b"",
        StdinKind::DevNull,
        StdoutKind::Pipe,
    );
}

/// Row 29 — fd 1 is a regular file (fully buffered) for both branches.
#[test]
fn cfg_exe_stdout_regular_file() {
    for s in ["0", "1", "abc", "", "-5", "4294967296"] {
        assert_exe_same_with(
            "row29/stdout-file",
            s.as_bytes(),
            StdinKind::Pipe,
            StdoutKind::File,
        );
    }
}

/// Row 30 — fd 1 is `/dev/null`.
#[test]
fn cfg_exe_stdout_devnull() {
    for s in ["0", "1", "abc", ""] {
        assert_exe_same_with(
            "row30/stdout-devnull",
            s.as_bytes(),
            StdinKind::Pipe,
            StdoutKind::DevNull,
        );
    }
}

// ===========================================================================
// Rows 31-32: unconstrained fuzz (channel E)
// ===========================================================================

/// Row 31 — completely arbitrary stdin bytes, including NUL and 0x80..0xff.
#[test]
fn cfg_exe_fuzz_arbitrary_bytes() {
    let mut rng = Rng::new(0x5EED_0031);
    for i in 0..N_FUZZ_BYTES {
        let len = rng.below(33) as usize; // 0..=32, empty included
        let input: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        assert_exe_same(&format!("row31/fuzz#{i}"), &input);
    }
}

/// Row 32 — grammar-aware fuzz over the alphabet `%d` actually cares about, so
/// that sign / whitespace / digit / terminator transitions are hit densely.
#[test]
fn cfg_exe_fuzz_numericish() {
    const ALPHABET: &[u8] = b"0123456789      \t\n\r\x0b\x0c+++---..xXeE\x00abz9990";
    let mut rng = Rng::new(0x5EED_0032);
    for i in 0..N_FUZZ_NUMERICISH {
        let len = rng.below(25) as usize;
        let input: Vec<u8> = (0..len).map(|_| *rng.pick(ALPHABET)).collect();
        assert_exe_same(&format!("row32/numericish#{i}"), &input);
    }
    // Longer ones, where the accumulator can overflow mid-string.
    for i in 0..300 {
        let len = rng.range(25, 120) as usize;
        let input: Vec<u8> = (0..len).map(|_| *rng.pick(ALPHABET)).collect();
        assert_exe_same(&format!("row32/numericish-long#{i}"), &input);
    }
}

// ===========================================================================
// Rows 34-36: scale, process environment, and the NUL terminator
// ===========================================================================

/// Row 34 — multi-megabyte stdin through a regular file: glibc's converter grows
/// its own work buffer for digit runs this long, and the accumulator saturates
/// very early.
#[test]
fn cfg_exe_multi_megabyte_inputs() {
    let cases: [(&str, Vec<u8>); 5] = [
        ("1MB-nines", vec![b'9'; 1_000_000]),
        ("1MB-zeros", vec![b'0'; 1_000_000]),
        ("1MB-spaces-then-7", {
            let mut v = vec![b' '; 1_000_000];
            v.push(b'7');
            v
        }),
        ("1MB-letters", vec![b'z'; 1_000_000]),
        ("minus-1MB-nines", {
            let mut v = vec![b'-'];
            v.extend(std::iter::repeat(b'9').take(1_000_000));
            v
        }),
    ];
    for (name, input) in cases {
        assert_exe_same_with(
            &format!("row34/{name}"),
            &input,
            StdinKind::File,
            StdoutKind::Pipe,
        );
    }
}

/// Row 35 — `int main()` takes no parameters and the program reads no
/// environment, so extra argv entries and hostile locale settings must change
/// nothing. (glibc's `scanf` consults the locale, but `main.c` never calls
/// `setlocale`, so it stays in the "C" locale whatever the environment says —
/// this row is what proves that rather than assuming it.)
#[test]
fn cfg_exe_argv_and_env_invariance() {
    let inputs: [&[u8]; 6] = [b"0", b"1", b"  5", b"\x0b7", b"abc", b""];
    let arg_sets: [&[&str]; 4] = [&[], &["extra"], &["-1", "0"], &["a", "b", "c"]];
    let env_sets: [&[(&str, &str)]; 6] = [
        &[],
        &[("LC_ALL", "tr_TR.UTF-8"), ("LANG", "tr_TR.UTF-8")],
        &[("LC_ALL", "C.UTF-8")],
        &[("LC_ALL", "en_US.UTF-8")],
        &[("LC_ALL", "POSIX"), ("LC_NUMERIC", "de_DE.UTF-8")],
        &[("LC_NUMERIC", "de_DE.UTF-8"), ("LC_CTYPE", "ja_JP.eucJP")],
    ];
    for input in inputs {
        for args in arg_sets {
            for envs in env_sets {
                assert_exe_same_extras("row35/argv-env", input, &Extras { args, envs });
            }
        }
    }
}

/// Row 36 — a payload containing an embedded NUL: `printf("%s")` stops at the
/// first NUL, so the bytes after it must be invisible on both sides.
#[test]
fn cfg_printline_embedded_nul() {
    let cases: [&[u8]; 8] = [
        b"\x00",
        b"\x00abc",
        b"a\x00b",
        b"abc\x00",
        b"abc\x00def",
        b"\x00\x00\x00",
        b"x\x00\xff\xfe",
        b"long-prefix-then\x00-hidden-tail",
    ];
    for p in cases {
        let c = so_print_line(Side::C, Some(p));
        let r = so_print_line(Side::Rust, Some(p));
        assert_bytes_eq("row36/embedded-nul", p, &c, &r);
        // Both must have stopped at the first NUL.
        let visible = &p[..p.iter().position(|&b| b == 0).unwrap_or(p.len())];
        let mut want = visible.to_vec();
        want.push(b'\n');
        assert_eq!(c, want, "C must stop at the first NUL");
    }
    // Randomized: a NUL somewhere inside random bytes.
    let mut rng = Rng::new(0x5EED_0036);
    for i in 0..200 {
        let len = rng.range(1, 40) as usize;
        let mut p: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        let at = rng.below(p.len() as u64) as usize;
        p[at] = 0;
        let c = so_print_line(Side::C, Some(&p));
        let r = so_print_line(Side::Rust, Some(&p));
        assert_bytes_eq(&format!("row36/rand-nul#{i}"), &p, &c, &r);
    }
}

// ===========================================================================
// Rows 37-38: stream state — the divergences a single call cannot show
// ===========================================================================

/// Row 37 — the exported `main` called **repeatedly** on one stream.
///
/// libc's `stdin` is one process-global `FILE`, so conversion *n+1* continues
/// where *n* stopped, including the single character *n* pushed back. This is the
/// row that found the push-back bug: a translation that consumes the terminating
/// character answers `good,good` for `"1-9223372036854775809"` where the C
/// answers `good,bad` (its second conversion sees the `-`, clamps to `LONG_MIN`,
/// and truncates to 0).
#[test]
fn cfg_so_main_repeated_calls_share_the_stream() {
    let cases: [&str; 24] = [
        "5x7",
        "-a5",
        "+.5",
        "1 2 3",
        "0x10",
        "12abc",
        "1-2-3",
        "9-0",
        "--5",
        "++5",
        "1..2",
        "1-9223372036854775809",
        "0 0",
        "1 0 1 0",
        "1x2x3",
        "- 5",
        "+ 5",
        "-x-5",
        "0-1",
        "5-0",
        "5-1",
        "1-2-3-4-5",
        "  7  8  ",
        "",
    ];
    // Primary, UB-free assertion: the stream position both libraries leave
    // behind after N calls. `bad()`'s undefined output is never observed.
    for s in cases {
        for times in [1usize, 2, 3, 5] {
            assert_so_main_repeat_leftover_same("row37/repeat", s.as_bytes(), times);
        }
    }
    // Randomized: strings over exactly the alphabet that makes push-back matter.
    const ALPHABET: &[u8] = b"0123456789+-. x\t\n";
    let mut rng = Rng::new(0x5EED_0037);
    for i in 0..250 {
        let len = rng.range(1, 14) as usize;
        let input: Vec<u8> = (0..len).map(|_| *rng.pick(ALPHABET)).collect();
        let times = rng.range(2, 4) as usize;
        assert_so_main_repeat_leftover_same(&format!("row37/rand#{i}"), &input, times);
    }
}

/// Row 37b — byte-exact stdout for repeated calls, on inputs where **every**
/// call converts a non-zero value.
///
/// Restricting to the all-`good()` case is what makes a byte comparison legal
/// here: `bad()` never runs, so nothing undefined is ever printed. (With
/// `bad()` in the mix the C is genuinely unpredictable — on `"--5"` × 3 the
/// release-profile runner had it print `"string"`, the pointer the preceding
/// `good()` left in the same stack slot, while the debug one printed `"\n"`.)
#[test]
fn cfg_so_main_repeated_calls_all_good_output() {
    for (s, times) in [
        ("7", 1usize),
        ("1 2", 2),
        ("1 2 3", 3),
        ("1-2-3-4-5", 5),
        ("  3   4  ", 2),
        ("12 34 56", 3),
        ("9-8-7", 3),
        ("-1 -2 -3", 3),
        ("2147483647 -2147483648", 2),
    ] {
        assert_so_main_repeat_all_good("row37b/all-good", s.as_bytes(), times);
    }
    // Randomized: `times` non-zero decimal numbers separated by single spaces.
    let mut rng = Rng::new(0x5EED_0037 + 1);
    for i in 0..120 {
        let times = rng.range(1, 4) as usize;
        let mut input = Vec::new();
        for k in 0..times {
            if k > 0 {
                input.push(b' ');
            }
            let mut v = rng.next_u32() as i32;
            if v == 0 {
                v = 1;
            }
            input.extend_from_slice(v.to_string().as_bytes());
        }
        assert_so_main_repeat_all_good(&format!("row37b/rand#{i}"), &input, times);
    }
}

/// Row 38 — what the program leaves on a **shared** fd 0 when it exits.
///
/// Two separate C behaviours are observable through this:
/// * a *seekable* stdin is rewound to the logical stream position by libc's
///   exit-time cleanup, so `{ ./driver; cat; } < "1 hello world"` prints
///   `" hello world"` — the buffered-but-unread bytes come back;
/// * a *pipe* cannot be rewound, so exactly one `st_blksize` refill (4096 here)
///   is gone for good.
///
/// A translation using `std::io::stdin()` swallows its own 8192-byte buffer and
/// never rewinds, which this row rejects.
#[test]
fn cfg_exe_shared_stdin_leftover() {
    assert_exe_leftover_same("row38/small", b"1 hello world");
    assert_exe_leftover_same("row38/zero", b"0 hello world");
    assert_exe_leftover_same("row38/fail", b"abc def ghi");
    assert_exe_leftover_same("row38/empty", b"");
    assert_exe_leftover_same("row38/ws-only", b"    ");
    assert_exe_leftover_same("row38/no-terminator", b"42");
    assert_exe_leftover_same("row38/newline", b"42\nrest");
    assert_exe_leftover_same("row38/sign-fail", b"-a rest");
    // Straddle the 4096-byte refill boundary.
    for len in [4095usize, 4096, 4097, 8191, 8192, 50_000] {
        let mut v = b"1 ".to_vec();
        v.extend(std::iter::repeat(b'A').take(len));
        assert_exe_leftover_same(&format!("row38/len{len}"), &v);
    }
    // Randomized.
    let mut rng = Rng::new(0x5EED_0038);
    for i in 0..120 {
        let len = rng.range(1, 6000) as usize;
        let input: Vec<u8> = (0..len).map(|_| *rng.pick(b"0123456789 \tabc-+.")).collect();
        assert_exe_leftover_same(&format!("row38/rand#{i}"), &input);
    }
}

// ===========================================================================
// Row 33: the exit status, on every input above
// ===========================================================================

/// Row 33 — `assert_exe_same*` already compares exit code *and* signal on every
/// single case; this test pins the invariant itself.
#[test]
fn cfg_exit_status_invariant() {
    let mut rng = Rng::new(0x5EED_0033);
    for _ in 0..200 {
        let len = rng.below(20) as usize;
        let input: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let c = run_exe(&c_exe(), &input, StdinKind::Pipe, StdoutKind::Pipe);
        let r = run_exe(&rust_exe(), &input, StdinKind::Pipe, StdoutKind::Pipe);
        assert_eq!(c.code, Some(0));
        assert_eq!(r.code, Some(0));
        assert_eq!(c.signal, None);
        assert_eq!(r.signal, None);
        assert!(c.stderr.is_empty() && r.stderr.is_empty());
    }
}
