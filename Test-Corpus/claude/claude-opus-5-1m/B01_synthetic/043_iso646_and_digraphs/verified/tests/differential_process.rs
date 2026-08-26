// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Phase B - valid-path differential tests at the process boundary.
//
// The C executable and the Rust executable are spawned with byte-identical
// stdin and their stdout, stderr, exit code and terminating signal are compared.
// This is the boundary a real consumer uses, and the only one where exit status,
// `SIGPIPE` disposition and lazy stdin consumption are observable.
//
// One test per row of CONFIGS.md (rows 7-39; rows 1-6 are the FFI rows in
// `differential_ffi.rs`, row 40 is the profile sweep driven by
// `scripts/verify.sh`).

mod common;

use common::{assert_same, assert_same_cfg, Rng, StdinKind, StdoutKind, SEED};

/// Every byte `isspace()` accepts in the `"C"` locale.
const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];

fn ws_run(rng: &mut Rng, max: usize) -> Vec<u8> {
    let n = rng.below(max) + 1;
    (0..n).map(|_| *rng.pick(&WS)).collect()
}

/// A decimal string of `digits` digits (no sign), never all-zero-length.
fn digit_string(rng: &mut Rng, digits: usize) -> Vec<u8> {
    (0..digits.max(1))
        .map(|_| b'0' + (rng.below(10) as u8))
        .collect()
}

// ---------------------------------------------------------------------------
// Row 7 - the baseline shape
// ---------------------------------------------------------------------------

#[test]
fn cfg07_two_ints_space_separated() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..600 {
        let input = format!("{} {}", rng.i32v(), rng.i32v());
        assert_same("cfg07", input.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 8 - separator sweep
// ---------------------------------------------------------------------------

#[test]
fn cfg08_separator_sweep() {
    // Every single whitespace byte, and the fixed multi-byte runs.
    let fixed: Vec<Vec<u8>> = WS
        .iter()
        .map(|&c| vec![c])
        .chain([b"\r\n".to_vec(), b"\n\n".to_vec(), b"  \t  ".to_vec()])
        .collect();
    for sep in &fixed {
        let mut input = b"12345".to_vec();
        input.extend_from_slice(sep);
        input.extend_from_slice(b"-6789");
        assert_same("cfg08/fixed", &input);
    }

    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..300 {
        let mut input = format!("{}", rng.i32v()).into_bytes();
        input.extend_from_slice(&ws_run(&mut rng, 12));
        input.extend_from_slice(format!("{}", rng.i32v()).as_bytes());
        assert_same("cfg08/random", &input);
    }
}

// ---------------------------------------------------------------------------
// Row 9 - leading whitespace, including runs past the 4096-byte stdio buffer
// ---------------------------------------------------------------------------

#[test]
fn cfg09_leading_whitespace() {
    for &c in &WS {
        for reps in [1usize, 2, 7, 4095, 4096, 4097, 9000] {
            let mut input = vec![c; reps];
            input.extend_from_slice(b"42 -7");
            assert_same("cfg09/fixed", &input);
        }
    }

    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..200 {
        let mut input = ws_run(&mut rng, 40);
        input.extend_from_slice(format!("{} {}", rng.i32v(), rng.i32v()).as_bytes());
        assert_same("cfg09/random", &input);
    }
}

// ---------------------------------------------------------------------------
// Row 10 - what follows the second token
// ---------------------------------------------------------------------------

#[test]
fn cfg10_trailing_bytes() {
    let tails: [&[u8]; 10] = [
        b"", b"\n", b"\r\n", b" ", b"\t\t\n", b"x", b"abc", b".", b",", b"-",
    ];
    let mut rng = Rng::new(SEED ^ 10);
    for tail in tails {
        for _ in 0..25 {
            let mut input = format!("{} {}", rng.i32v(), rng.i32v()).into_bytes();
            input.extend_from_slice(tail);
            assert_same("cfg10", &input);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 11 & 12 - explicit signs
// ---------------------------------------------------------------------------

#[test]
fn cfg11_plus_sign_combinations() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..200 {
        let a = (rng.next_u32() >> 1) as i64;
        let b = (rng.next_u32() >> 1) as i64;
        for (sa, sb) in [("+", ""), ("", "+"), ("+", "+")] {
            let input = format!("{sa}{a} {sb}{b}");
            assert_same("cfg11", input.as_bytes());
        }
    }
}

#[test]
fn cfg12_minus_sign_combinations() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..200 {
        let a = (rng.next_u32() >> 1) as i64;
        let b = (rng.next_u32() >> 1) as i64;
        for (sa, sb) in [("-", ""), ("", "-"), ("-", "-")] {
            let input = format!("{sa}{a} {sb}{b}");
            assert_same("cfg12", input.as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 13 & 14 - leading zeros must NOT turn into octal, and zero itself
// ---------------------------------------------------------------------------

#[test]
fn cfg13_leading_zeros() {
    let mut rng = Rng::new(SEED ^ 13);
    for zeros in 1..=40usize {
        for sign in ["", "-", "+"] {
            let v = rng.next_u32() >> 8;
            let input = format!("{sign}{}{v} {sign}{}{v}", "0".repeat(zeros), "0".repeat(zeros));
            assert_same("cfg13", input.as_bytes());
        }
    }
    // Octal-looking values: 010 must read as ten, not eight.
    for lit in ["010", "0777", "08", "09", "0000000019"] {
        let input = format!("{lit} {lit}");
        assert_same("cfg13/octal", input.as_bytes());
    }
}

#[test]
fn cfg14_zero_forms() {
    let forms = ["0", "-0", "+0", "00000", "-00000", "+00000"];
    for a in forms {
        for b in forms {
            let input = format!("{a} {b}");
            assert_same("cfg14", input.as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 15-18 - the conversion range boundaries
// ---------------------------------------------------------------------------

#[test]
fn cfg15_int_boundaries() {
    let vals = [
        i32::MIN as i64,
        i32::MIN as i64 + 1,
        -1,
        0,
        1,
        i32::MAX as i64 - 1,
        i32::MAX as i64,
    ];
    for a in vals {
        for b in vals {
            let input = format!("{a} {b}");
            assert_same("cfg15", input.as_bytes());
        }
    }
}

#[test]
fn cfg16_just_past_int_range() {
    let vals = [
        "2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967296",
        "-4294967295",
        "8589934592",
    ];
    for a in vals {
        for b in vals {
            let input = format!("{a} {b}");
            assert_same("cfg16", input.as_bytes());
        }
    }
}

#[test]
fn cfg17_long_boundaries_exact() {
    let vals = [
        "9223372036854775807",  // LONG_MAX
        "-9223372036854775808", // LONG_MIN
        "9223372036854775806",
        "-9223372036854775807",
    ];
    for a in vals {
        for b in vals {
            let input = format!("{a} {b}");
            assert_same("cfg17", input.as_bytes());
        }
    }
}

#[test]
fn cfg18_past_long_range_erange_clamp() {
    let vals = [
        "9223372036854775808",             // LONG_MAX + 1
        "-9223372036854775809",            // LONG_MIN - 1
        "18446744073709551615",            // UINT64_MAX
        "18446744073709551616",            // 2^64
        "99999999999999999999999999",      // 26 digits
        "-99999999999999999999999999",
        "1111111111111111111111111111111111111111",  // 40 digits
        "-1111111111111111111111111111111111111111",
    ];
    for a in vals {
        for b in vals {
            let input = format!("{a} {b}");
            assert_same("cfg18", input.as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 - randomized digit strings of 1..25 digits, so the int/uint/long
// boundaries are crossed by construction
// ---------------------------------------------------------------------------

#[test]
fn cfg19_randomized_digit_lengths() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..600 {
        let mut input = Vec::new();
        for i in 0..2 {
            if i == 1 {
                input.push(b' ');
            }
            match rng.below(3) {
                0 => input.push(b'-'),
                1 => input.push(b'+'),
                _ => {}
            }
            let digits = rng.below(25) + 1;
            input.extend_from_slice(&digit_string(&mut rng, digits));
        }
        assert_same("cfg19", &input);
    }
}

// ---------------------------------------------------------------------------
// Row 20 - absurdly long digit runs
// ---------------------------------------------------------------------------

#[test]
fn cfg20_very_long_digit_run() {
    let mut rng = Rng::new(SEED ^ 20);
    for sign in ["", "-", "+"] {
        let a = digit_string(&mut rng, 10_000);
        let b = digit_string(&mut rng, 10_000);
        let mut input = Vec::new();
        input.extend_from_slice(sign.as_bytes());
        input.extend_from_slice(&a);
        input.push(b' ');
        input.extend_from_slice(sign.as_bytes());
        input.extend_from_slice(&b);
        assert_same("cfg20", &input);
    }
    // A long run of zeros followed by a small value is still that small value.
    let mut input = vec![b'0'; 10_000];
    input.extend_from_slice(b"7 ");
    input.extend(std::iter::repeat(b'0').take(10_000));
    input.extend_from_slice(b"9");
    assert_same("cfg20/zeros", &input);
}

// ---------------------------------------------------------------------------
// Rows 21 & 22 - too few and too many tokens
// ---------------------------------------------------------------------------

#[test]
fn cfg21_single_token_only() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..200 {
        let v = rng.i32v();
        for tail in ["", "\n", " ", "\t\n "] {
            let input = format!("{v}{tail}");
            assert_same("cfg21", input.as_bytes());
        }
    }
}

#[test]
fn cfg22_more_than_two_tokens() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..200 {
        let n = rng.below(6) + 3; // 3..8 tokens
        let parts: Vec<String> = (0..n).map(|_| format!("{}", rng.i32v())).collect();
        assert_same("cfg22", parts.join(" ").as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Rows 23-25 - the byte that terminates a token
// ---------------------------------------------------------------------------

#[test]
fn cfg23_digits_then_nondigit() {
    let cases = [
        "5abc", "5.75", "1e5", "0x5", "0X5", "5-3", "5+3", "12,34", "7)", "9_9", "3/4", "8:2",
    ];
    for a in cases {
        assert_same("cfg23/first", a.as_bytes());
        for b in cases {
            let input = format!("{a} {b}");
            assert_same("cfg23/both", input.as_bytes());
        }
    }
}

#[test]
fn cfg24_hex_prefix_forms() {
    // base 10 stops at `x`, so "0x1F" converts as 0 and leaves "x1F" behind
    let cases = ["0x1F", "0X1f", "0x", "0X", "-0x10", "+0X10", "00x5"];
    for a in cases {
        assert_same("cfg24/first", a.as_bytes());
        for b in cases {
            let input = format!("{a} {b}");
            assert_same("cfg24/both", input.as_bytes());
        }
    }
}

#[test]
fn cfg25_adjacent_tokens_no_separator() {
    let cases = [
        "12-34", "12+34", "-12-34", "-12+34", "+12-34", "0-0", "2147483647-2147483648",
    ];
    for c in cases {
        assert_same("cfg25", c.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Rows 26 & 27 - non-ASCII and NUL bytes
// ---------------------------------------------------------------------------

#[test]
fn cfg26_embedded_nul_bytes() {
    let cases: [&[u8]; 8] = [
        b"\x005 7",
        b"5\x007",
        b"5 \x007",
        b"5\x00 7",
        b"5 7\x00",
        b"\x00\x00\x00",
        b"-\x005",
        b"5\x00-7",
    ];
    for c in cases {
        assert_same("cfg26", c);
    }
}

#[test]
fn cfg27_high_bytes() {
    for b in 0x80u8..=0xff {
        // as the leading byte, as a terminator, and as a separator
        assert_same("cfg27/lead", &[b, b'5', b' ', b'7']);
        assert_same("cfg27/term", &[b'5', b, b' ', b'7']);
        assert_same("cfg27/sep", &[b'5', b, b'7']);
        assert_same("cfg27/alone", &[b]);
    }
}

// ---------------------------------------------------------------------------
// Rows 28 & 29 - fuzz
// ---------------------------------------------------------------------------

/// Structured fuzz: build inputs out of the token classes the scanner
/// distinguishes, so a large fraction of the cases reach deep code paths.
#[test]
fn cfg28_structured_fuzz() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..1000 {
        let pieces = rng.below(6) + 1;
        let mut input = Vec::new();
        for _ in 0..pieces {
            match rng.below(8) {
                0 => input.extend_from_slice(&ws_run(&mut rng, 4)),
                1 => input.push(*rng.pick(&[b'-', b'+'])),
                2 => {
                    let n = rng.below(24) + 1;
                    input.extend_from_slice(&digit_string(&mut rng, n));
                }
                3 => input.push(b'a' + rng.below(26) as u8),
                4 => input.push(*rng.pick(b".,;:/*#()[]{}%$@!?~^&|<>=\"'\\`")),
                5 => input.push(0),
                6 => input.push(0x80 + rng.below(128) as u8),
                _ => input.extend_from_slice(format!("{}", rng.i32v()).as_bytes()),
            }
        }
        assert_same("cfg28", &input);
    }
}

/// Raw byte fuzz: uniformly random bytes, including lengths of zero.
#[test]
fn cfg29_raw_byte_fuzz() {
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..1000 {
        let n = rng.below(65);
        let input: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
        assert_same("cfg29", &input);
    }
}

// ---------------------------------------------------------------------------
// Row 30 - stdin that never reaches EOF
// ---------------------------------------------------------------------------

#[test]
fn cfg30_unbounded_stdin() {
    use common::{c_exe, run_unbounded, rust_exe};
    use std::time::Duration;

    // `scanf` is lazy: two conversions are all it needs, so both builds must
    // finish promptly even though the producer never stops.
    //
    // A stream of pure whitespace is the exception: `%d` skips whitespace
    // without bound, so the C program genuinely never returns. Rust has to block
    // in exactly the same way, which is asserted separately below.
    for pattern in [&b"5 7 "[..], &b"1 "[..], &b"\x00"[..], &b"x"[..], &b"-"[..]] {
        let budget = Duration::from_secs(20);

        let c = run_unbounded(c_exe(), pattern, budget).unwrap_or_else(|e| {
            panic!("the C build itself hung on an unbounded stdin after {e:?} - pattern {pattern:?}")
        });
        let r = run_unbounded(rust_exe(), pattern, budget).unwrap_or_else(|e| {
            panic!(
                "Rust hung on an unbounded stdin (pattern {pattern:?}) for {e:?} while C finished \
                 in {:?}. `scanf` reads lazily; the translation must not slurp stdin.",
                c.elapsed
            )
        });

        assert_eq!(
            c.outcome, r.outcome,
            "[cfg30] unbounded stdin (pattern {pattern:?}) diverged:\n  C   -> {:?}\n  Rust-> {:?}",
            c.outcome, r.outcome
        );

        // How much of the endless stream each build actually swallowed. `scanf`
        // needs two conversions and nothing more, so the C build accepts only a
        // buffer or two; the pipe itself holds 64 KiB, so allow 1 MiB of slack.
        // An implementation that reads to end-of-file first lands in the
        // gigabytes, which this catches regardless of machine speed.
        const LAZY_LIMIT: u64 = 1024 * 1024;
        assert!(
            c.bytes_fed < LAZY_LIMIT,
            "[cfg30] sanity check: the C build consumed {} bytes of the endless stream, \
             so the 1 MiB laziness limit is not a valid reference point",
            c.bytes_fed
        );
        assert!(
            r.bytes_fed < LAZY_LIMIT,
            "[cfg30] Rust consumed {} bytes of an endless stdin (pattern {pattern:?}) whereas C \
             consumed {}. `scanf` reads lazily - the translation must not slurp stdin \
             (it took {:?} versus C's {:?}).",
            r.bytes_fed,
            c.bytes_fed,
            r.elapsed,
            c.elapsed
        );
        // Wall clock is a weaker signal, but a lazy implementation is never slow.
        assert!(
            r.elapsed < Duration::from_secs(2),
            "[cfg30] Rust took {:?} on an unbounded stdin (C took {:?})",
            r.elapsed,
            c.elapsed
        );
    }

    // Endless whitespace: `%d` skips it forever, so *neither* build may exit.
    // A translation that read to EOF first would also hang here, but one that
    // treated a full buffer as end-of-input would wrongly terminate, so this
    // pins the shared blocking behaviour down.
    let hang_budget = Duration::from_secs(3);
    for (name, exe) in [("C", c_exe()), ("Rust", rust_exe())] {
        let outcome = run_unbounded(exe, b" \t\n", hang_budget);
        assert!(
            outcome.is_err(),
            "[cfg30] the {name} build exited on an endless whitespace stream, but `%d` \
             skips whitespace without bound so it must block: {:?}",
            outcome.map(|r| r.outcome)
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 31 & 32 - multi-megabyte streams
// ---------------------------------------------------------------------------

#[test]
fn cfg31_huge_stdin_valid_prefix() {
    let mut input = b"1234 -5678 ".to_vec();
    input.extend(std::iter::repeat(b'9').take(8 * 1024 * 1024));
    assert_same_cfg("cfg31/pipe", &input, StdinKind::Pipe, StdoutKind::Pipe, &[]);
    assert_same_cfg("cfg31/file", &input, StdinKind::File, StdoutKind::Pipe, &[]);
}

#[test]
fn cfg32_huge_stdin_no_valid_token() {
    let mut input = Vec::with_capacity(8 * 1024 * 1024);
    // No digit anywhere, so both conversions fail no matter how much is read.
    while input.len() < 8 * 1024 * 1024 {
        input.extend_from_slice(b"abcdefgh.,;:!? \t\n");
    }
    assert_same_cfg("cfg32/pipe", &input, StdinKind::Pipe, StdoutKind::Pipe, &[]);
    assert_same_cfg("cfg32/file", &input, StdinKind::File, StdoutKind::Pipe, &[]);
}

// ---------------------------------------------------------------------------
// Rows 33 & 34 - degenerate streams
// ---------------------------------------------------------------------------

#[test]
fn cfg33_empty_stdin() {
    assert_same_cfg("cfg33/pipe", b"", StdinKind::Pipe, StdoutKind::Pipe, &[]);
    assert_same_cfg("cfg33/file", b"", StdinKind::File, StdoutKind::Pipe, &[]);
}

#[test]
fn cfg34_whitespace_only_stdin() {
    for reps in [1usize, 10, 4095, 4096, 4097, 100_000] {
        for &c in &WS {
            let input = vec![c; reps];
            assert_same("cfg34", &input);
        }
    }
    let mut rng = Rng::new(SEED ^ 34);
    for _ in 0..100 {
        let input = ws_run(&mut rng, 200);
        assert_same("cfg34/random", &input);
    }
}

// ---------------------------------------------------------------------------
// Rows 35-37 - descriptor shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg35_stdin_file_vs_pipe() {
    let mut rng = Rng::new(SEED ^ 35);
    let mut cases: Vec<Vec<u8>> = vec![
        b"5 7".to_vec(),
        b"".to_vec(),
        b"   ".to_vec(),
        b"abc".to_vec(),
        b"-".to_vec(),
        b"--5 3".to_vec(),
        b"9223372036854775808".to_vec(),
    ];
    for _ in 0..60 {
        cases.push(format!("{} {}", rng.i32v(), rng.i32v()).into_bytes());
    }
    for input in &cases {
        for kind in [StdinKind::Pipe, StdinKind::File] {
            assert_same_cfg("cfg35", input, kind, StdoutKind::Pipe, &[]);
        }
    }
}

#[test]
fn cfg36_stdout_file_vs_pipe() {
    let mut rng = Rng::new(SEED ^ 36);
    for _ in 0..60 {
        let input = format!("{} {}", rng.i32v(), rng.i32v()).into_bytes();
        for kind in [StdoutKind::Pipe, StdoutKind::File] {
            assert_same_cfg("cfg36", &input, StdinKind::Pipe, kind, &[]);
        }
    }
}

#[test]
fn cfg37_stdin_closed() {
    assert_same_cfg("cfg37", b"", StdinKind::Closed, StdoutKind::Pipe, &[]);
}

// ---------------------------------------------------------------------------
// Row 38 - the program never calls setlocale, so the locale must not matter
// ---------------------------------------------------------------------------

#[test]
fn cfg38_locale_is_irrelevant() {
    let locales = ["C", "POSIX", "en_US.UTF-8", "de_DE.UTF-8", "tr_TR.UTF-8"];
    let inputs: [&[u8]; 5] = [b"5 7", b"1234567 -7654321", b"-0 +0", b"1,5 2", b"2147483648 1"];
    for loc in locales {
        for input in inputs {
            assert_same_cfg(
                "cfg38",
                input,
                StdinKind::Pipe,
                StdoutKind::Pipe,
                &[("LC_ALL", loc), ("LANG", loc), ("LC_NUMERIC", loc)],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 39 - one byte at a time, forcing short reads
// ---------------------------------------------------------------------------

#[test]
fn cfg39_dripped_stdin() {
    let cases: [&[u8]; 8] = [
        b"5 7",
        b"  -42\t\t99  ",
        b"",
        b"-",
        b"--5 3",
        b"9223372036854775808 -9223372036854775809",
        b"0x5 7",
        b"abc",
    ];
    for input in cases {
        assert_same_cfg("cfg39", input, StdinKind::Drip, StdoutKind::Pipe, &[]);
    }
}

// ---------------------------------------------------------------------------
// Row 41 - the program reads neither argv nor the environment
//
// `int main()` is declared without parameters and the body never touches
// `getenv`, so command-line arguments and stray environment variables must not
// change a single byte of output.
// ---------------------------------------------------------------------------

#[test]
fn cfg41_argv_and_env_are_ignored() {
    use common::{c_exe, rust_exe, Outcome};
    use std::process::{Command, Stdio};
    use std::io::Write;
    use std::os::unix::process::ExitStatusExt;

    fn run_with_args(exe: &std::path::Path, args: &[&str], env: &[(&str, &str)], input: &[u8]) -> Outcome {
        let mut child = Command::new(exe)
            .args(args)
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let mut sink = child.stdin.take().unwrap();
        let _ = sink.write_all(input);
        drop(sink);
        let out = child.wait_with_output().expect("wait");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }

    let arg_sets: [&[&str]; 5] = [
        &[],
        &["-h"],
        &["--help"],
        &["1", "2", "3"],
        &["-x", "--verbose", "/dev/null"],
    ];
    let env_sets: [&[(&str, &str)]; 3] = [
        &[],
        &[("DRIVER_X", "999"), ("DRIVER_Y", "999")],
        &[("NLSPATH", "/nonexistent"), ("TZ", "UTC")],
    ];
    let inputs: [&[u8]; 4] = [b"5 7", b"", b"abc", b"-42 +17"];

    for args in arg_sets {
        for env in env_sets {
            for input in inputs {
                let c = run_with_args(c_exe(), args, env, input);
                let r = run_with_args(rust_exe(), args, env, input);
                assert_eq!(
                    c, r,
                    "[cfg41] diverged with args={args:?} env={env:?} stdin={:?}:\n  C   -> {c:?}\n  Rust-> {r:?}",
                    String::from_utf8_lossy(input)
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 42 - dense sweep of the neighbourhoods around every conversion boundary
//
// This is where the subtle logic lives: `strtol` clamps at LONG_MAX/LONG_MIN and
// the result is then truncated to `int`. Values a few steps either side of
// 2^31, 2^32, 2^63 and 2^64 exercise the clamp, the wrap, and the interaction
// between them far more effectively than uniform random numbers do.
// ---------------------------------------------------------------------------

#[test]
fn cfg42_conversion_boundary_neighbourhoods() {
    let mut values: Vec<String> = Vec::new();
    for &centre in &[
        0u128,
        1 << 15,
        1 << 16,
        1 << 31,          // INT_MAX + 1
        1 << 32,          // UINT_MAX + 1
        1 << 63,          // LONG_MAX + 1
        1 << 64,          // ULONG_MAX + 1
        (1 << 64) + (1 << 63),
        1 << 65,
        10_000_000_000_000_000_000,
    ] {
        for delta in -4i128..=4 {
            let v = centre as i128 + delta;
            if v < 0 {
                continue;
            }
            values.push(v.to_string());
            values.push(format!("-{v}"));
            values.push(format!("+{v}"));
            // Leading zeros must not change the value.
            values.push(format!("000{v}"));
            values.push(format!("-000{v}"));
        }
    }

    // Each value in the `y` position, where the truncated result is directly
    // visible as `0 | ~y`, and in the `x` position.
    for v in &values {
        assert_same("cfg42/y", format!("0 {v}").as_bytes());
        assert_same("cfg42/x", format!("{v} -1").as_bytes());
    }

    // And paired against each other, so both conversions are exercised at once.
    let mut rng = Rng::new(SEED ^ 42);
    for _ in 0..500 {
        let a = &values[rng.below(values.len())];
        let b = &values[rng.below(values.len())];
        assert_same("cfg42/pair", format!("{a} {b}").as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 43 - the C reference must not be optimisation-dependent
//
// `c_src/CMakeLists.txt` compiles with no explicit `-O` level while the FFI
// reference is built at `-O2`. If those two disagreed anywhere, the "C is ground
// truth" premise would be ambiguous and the expected values baked into
// ERRORS.md would be pinned to one particular build. They must agree, and Rust
// must agree with both.
// ---------------------------------------------------------------------------

#[test]
fn cfg43_c_reference_is_optimisation_independent() {
    use common::{c_exe, c_exe_o0, run, rust_exe};

    let mut rng = Rng::new(SEED ^ 43);
    let mut cases: Vec<Vec<u8>> = vec![
        b"5 7".to_vec(),
        b"".to_vec(),
        b"-".to_vec(),
        b"--5 3".to_vec(),
        b"abc".to_vec(),
        b"0x5 7".to_vec(),
        b"9223372036854775808 -9223372036854775809".to_vec(),
        b"2147483648 -2147483649".to_vec(),
        b"-2147483648 2147483647".to_vec(),
        b"   \t\n 42 \r\n -7 ".to_vec(),
    ];
    for _ in 0..300 {
        let n = rng.below(40);
        cases.push((0..n).map(|_| rng.next_u32() as u8).collect());
    }
    for _ in 0..200 {
        cases.push(format!("{} {}", rng.i32v(), rng.i32v()).into_bytes());
    }

    for input in &cases {
        let o2 = run(c_exe(), input);
        let o0 = run(c_exe_o0(), input);
        assert_eq!(
            o2, o0,
            "[cfg43] the C build disagrees with itself across optimisation levels for {:?}:\n  \
             -O2 -> {o2:?}\n  -O0 -> {o0:?}",
            String::from_utf8_lossy(input)
        );
        let r = run(rust_exe(), input);
        assert_eq!(
            o2, r,
            "[cfg43] Rust diverged from the C reference for {:?}:\n  C   -> {o2:?}\n  Rust-> {r:?}",
            String::from_utf8_lossy(input)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 44 - stdin is a descriptor that cannot be read at all
//
// Opening a directory read-only succeeds, but `read(2)` on it fails with EISDIR.
// The C library reports that as end-of-file to `scanf`, so both conversions fail
// and the variables keep their initialisers.
// ---------------------------------------------------------------------------

#[test]
fn cfg44_stdin_is_a_directory() {
    use common::{c_exe, rust_exe, Outcome};
    use std::process::{Command, Stdio};
    use std::os::unix::process::ExitStatusExt;

    fn run_with_dir_stdin(exe: &std::path::Path) -> Outcome {
        let dir = std::fs::File::open(std::env::temp_dir()).expect("open temp dir");
        let out = Command::new(exe)
            .stdin(Stdio::from(dir))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }

    let c = run_with_dir_stdin(c_exe());
    let r = run_with_dir_stdin(rust_exe());
    assert_eq!(
        c, r,
        "[cfg44] diverged with a directory as stdin:\n  C   -> {c:?}\n  Rust-> {r:?}"
    );
    // Both conversions fail, so x == y == 0 and 0 | ~0 == -1.
    assert_eq!(String::from_utf8_lossy(&c.stdout), "-1\n");
    assert_eq!(c.code, Some(0));
}
