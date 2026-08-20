//! C-vs-Rust differential tests driven entirely through the exported C ABI of
//! the two shared libraries (loaded with `libloading` inside
//! `examples/so_runner.rs`, one fresh process per case).
//!
//! * `config_*` tests cover the rows of `CONFIGS.md` (Phase B).
//! * `error_path_*` tests cover the rows of `ERRORS.md` (Phase C).
//! * `symbol_*` tests cover Phase D.

mod common;

use common::*;
use std::process::Command;

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================

/// Phase D / CONFIGS C25: every symbol defined by the C `.so` must also be
/// defined by the Rust `.so`, with the exact same name.
#[test]
fn symbol_parity_nm_defined_only() {
    fn defined(lib: &std::path::Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(lib)
            .output()
            .expect("run nm");
        assert!(
            out.status.success(),
            "nm failed on {lib:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c = defined(&c_lib_path());
    let r = defined(&rust_lib_path());
    assert!(
        c.contains(&"driver".to_string()) && c.contains(&"main".to_string()),
        "unexpected C export set: {c:?}"
    );
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C   : {c:?}\nRust: {r:?}"
    );
    assert_eq!(c, r, "export sets are not identical\nC: {c:?}\nRust: {r:?}");
}

/// Phase D / CONFIGS C25: `dlopen` + `dlsym` of every C symbol on both
/// libraries, through `libloading`.
#[test]
fn symbol_dlsym_both_libs() {
    for lib in [c_lib_path(), rust_lib_path()] {
        let libs = lib.to_str().unwrap().to_string();
        let out = run(
            &runner_path(),
            &[&libs, "symbols", "driver", "main"],
            b"",
            StdinKind::Null,
            StdoutKind::Pipe,
        );
        assert_eq!(out.status, Some(0), "dlsym failed for {lib:?}: {}", out.show());
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "driver FOUND\nmain FOUND\n",
            "unexpected dlsym report for {lib:?}"
        );
    }
}

/// Undefined symbols of the Rust `.so` must all be resolvable (libc/libgcc).
#[test]
fn symbol_no_unresolved_in_rust_so() {
    let out = Command::new("ldd")
        .arg(rust_lib_path())
        .output()
        .expect("run ldd");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("not found"),
        "unresolved shared-object dependency in the Rust .so:\n{text}"
    );
}

// ===========================================================================
// Phase B — valid paths (CONFIGS.md rows)
// ===========================================================================

/// C1, C2, C3: zero, small ± values and the `int` extremes, each in its own
/// process (one `driver` call per process).
#[test]
fn config_c1_c2_c3_driver_single_calls() {
    let values = [
        0i32,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    for v in values {
        let c = call_driver(&c_lib_path(), v);
        let r = call_driver(&rust_lib_path(), v);
        assert_eq!(
            c.stdout,
            r.stdout,
            "driver({v}) stdout differs\n  C   : {}\n  Rust: {}",
            c.show(),
            r.show()
        );
        assert_eq!(c.status, r.status, "driver({v}) exit status differs");
        assert_eq!(
            c.stdout,
            expected_image(v),
            "C driver({v}) disagrees with the struct-image oracle"
        );
    }
}

/// C4: byte/nibble patterns — makes every hex digit position of `print_hex`
/// take a non-trivial value.
#[test]
fn config_c4_driver_byte_patterns() {
    let pats: Vec<i32> = [
        0x0000_00ffu32,
        0x0000_ff00,
        0x00ff_0000,
        0xff00_0000,
        0x7f7f_7f7f,
        0x8080_8080,
        0xdead_beef,
        0xffff_ffff,
        0x0123_4567,
        0x89ab_cdef,
        0xfedc_ba98,
        0x7654_3210,
        0x0f0f_0f0f,
        0xf0f0_f0f0,
        0x0000_0001,
        0x1000_0000,
    ]
    .iter()
    .map(|&u| u as i32)
    .collect();
    assert_driver_batch(&pats, StdoutKind::Pipe, "C4 byte patterns");
    // also one-per-process, to prove the batch mode is not hiding anything
    for v in &pats {
        let c = call_driver(&c_lib_path(), *v);
        let r = call_driver(&rust_lib_path(), *v);
        assert_eq!(c.stdout, r.stdout, "driver({v:#x}) stdout differs");
        assert_eq!(c.status, r.status, "driver({v:#x}) status differs");
    }
}

/// C5: all powers of two and their negations.
#[test]
fn config_c5_driver_powers_of_two() {
    let mut vals = Vec::new();
    for s in 0..32u32 {
        let v = 1u32.wrapping_shl(s) as i32;
        vals.push(v);
        vals.push(v.wrapping_neg());
        vals.push(v.wrapping_sub(1));
        vals.push(v.wrapping_add(1));
    }
    assert_driver_batch(&vals, StdoutKind::Pipe, "C5 powers of two");
}

/// C6 + C7: 4096 uniformly random `i32` (fixed seed) — all in ONE process, so
/// this also covers the repeated-call/flush axis A7.
#[test]
fn config_c6_c7_driver_random_batch() {
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    let vals: Vec<i32> = (0..4096).map(|_| rng.next_i32()).collect();
    assert_driver_batch(&vals, StdoutKind::Pipe, "C6/C7 random batch");
}

/// C8: stdout is a regular file (fully buffered in C) rather than a pipe.
#[test]
fn config_c8_driver_stdout_to_file() {
    let mut rng = Rng::new(0x5EED_0008);
    let mut vals: Vec<i32> = (0..512).map(|_| rng.next_i32()).collect();
    vals.extend_from_slice(&[0, 1, -1, i32::MAX, i32::MIN]);
    assert_driver_batch(&vals, StdoutKind::File, "C8 stdout=file");
}

/// C9: plain decimal input, EOF terminator.
#[test]
fn config_c9_main_plain_decimal() {
    for s in ["0", "1", "2", "7", "9", "42", "1000", "123456789"] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "C9");
    }
}

/// C10: the sign axis.
#[test]
fn config_c10_main_signs() {
    for n in ["0", "1", "7", "42", "2147483647", "2147483648", "10000000000"] {
        for sign in ["", "+", "-"] {
            let s = format!("{sign}{n}");
            assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "C10");
        }
    }
}

/// C11: every `isspace` byte as a leading-whitespace prefix, alone, repeated and
/// mixed (scanf skips whitespace across newlines).
#[test]
fn config_c11_main_whitespace_prefixes() {
    let ws: [&[u8]; 6] = [b" ", b"\t", b"\n", b"\x0b", b"\x0c", b"\r"];
    for w in ws {
        for reps in [1usize, 2, 5] {
            let mut v = Vec::new();
            for _ in 0..reps {
                v.extend_from_slice(w);
            }
            v.extend_from_slice(b"42");
            assert_main_case(&v, StdinKind::Pipe, StdoutKind::Pipe, "C11");
        }
    }
    for mixed in [
        &b" \t\n-7"[..],
        &b"\r\n\r\n+8"[..],
        &b"\x0b\x0c \t\r\n9"[..],
        &b"\n\n\n\n0"[..],
        &b" \t \t 2147483647"[..],
    ] {
        assert_main_case(mixed, StdinKind::Pipe, StdoutKind::Pipe, "C11 mixed");
    }
}

/// C12: the terminator axis (what stops the digit run).
#[test]
fn config_c12_main_terminators() {
    for suffix in [
        "", "\n", " ", "\t", "\r", "\x0b", "\x0c", "x", "X", ".", ",", "-", "+", "e", "e5", " 43",
        "\n43", "43", ";", "/", ":", "\0", "\u{7f}",
    ] {
        for base in ["7", "-7", "+7", "0", "2147483647"] {
            let s = format!("{base}{suffix}");
            assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "C12");
        }
    }
}

/// C13: leading zeros, including runs long enough to matter for a naive
/// accumulator.
#[test]
fn config_c13_main_leading_zeros() {
    let mut cases: Vec<String> = vec![
        "0".into(),
        "00".into(),
        "000".into(),
        "007".into(),
        "-007".into(),
        "+00042".into(),
        "0000000000000000000".into(),
    ];
    for zeros in [1usize, 18, 19, 20, 21, 40, 5000] {
        cases.push(format!("{}5", "0".repeat(zeros)));
        cases.push(format!("-{}5", "0".repeat(zeros)));
        cases.push(format!("{}2147483648", "0".repeat(zeros)));
    }
    for s in &cases {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "C13");
    }
}

/// C14: digit-run length 1..=25 for repeated `9`s and for a rolling digit
/// pattern — crosses `int`, `long` and the saturation region.
#[test]
fn config_c14_main_digit_run_lengths() {
    let rolling = "1234567890123456789012345";
    for n in 1..=25usize {
        for s in [
            "9".repeat(n),
            format!("-{}", "9".repeat(n)),
            rolling[..n].to_string(),
            format!("-{}", &rolling[..n]),
        ] {
            assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "C14");
        }
    }
}

/// C15: `int` range boundaries.
#[test]
fn config_c15_main_int_boundaries() {
    for s in [
        "2147483645",
        "2147483646",
        "2147483647",
        "-2147483645",
        "-2147483646",
        "-2147483647",
        "-2147483648",
        "32767",
        "32768",
        "65535",
        "65536",
        "255",
        "256",
        "-32768",
        "-32769",
        "-65536",
    ] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "C15");
    }
}

/// C16: values that fit in `long` but not `int` — the C truncates silently.
#[test]
fn config_c16_main_long_not_int() {
    for s in [
        "2147483648",
        "2147483649",
        "-2147483649",
        "-2147483650",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967295",
        "-4294967296",
        "-4294967297",
        "8589934592",
        "1099511627776",
        "1234567890123",
        "999999999999999999",
        "-999999999999999999",
    ] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "C16");
    }
}

/// C17: the `long` boundary and the glibc saturation region.
#[test]
fn config_c17_main_long_boundaries() {
    let mut cases: Vec<String> = vec![
        "9223372036854775805".into(),
        "9223372036854775806".into(),
        "9223372036854775807".into(),
        "9223372036854775808".into(),
        "9223372036854775809".into(),
        "-9223372036854775806".into(),
        "-9223372036854775807".into(),
        "-9223372036854775808".into(),
        "-9223372036854775809".into(),
        "-9223372036854775810".into(),
        "18446744073709551615".into(),
        "18446744073709551616".into(),
        "18446744073709551617".into(),
        "-18446744073709551616".into(),
        "99999999999999999999".into(),
        "-99999999999999999999".into(),
    ];
    for len in [21usize, 40, 64, 1000] {
        cases.push("9".repeat(len));
        cases.push(format!("-{}", "9".repeat(len)));
        cases.push("1".repeat(len));
        cases.push(format!("-{}", "1".repeat(len)));
    }
    for s in &cases {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "C17");
    }
}

/// Random stdin generator mixing every axis of A3/A4 (and, 1 in 8 times, a
/// malformed input so the random corpus also crosses the error paths).
fn random_stdin(rng: &mut Rng) -> Vec<u8> {
    let ws: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
    let mut v = Vec::new();
    for _ in 0..rng.below(4) {
        v.push(*rng.pick(&ws));
    }
    if rng.below(8) == 0 {
        // malformed tail
        let bad: [&[u8]; 12] = [
            b"", b"-", b"+", b"abc", b".5", b"--1", b"- 5", b"+ 5", b"\0", b"\xff\xfe", b"x1",
            b"e10",
        ];
        let chosen = *rng.pick(&bad);
        v.extend_from_slice(chosen);
        return v;
    }
    match rng.below(3) {
        0 => v.push(b'-'),
        1 => v.push(b'+'),
        _ => {}
    }
    if rng.below(4) == 0 {
        for _ in 0..rng.below(6) {
            v.push(b'0');
        }
    }
    let ndig = 1 + rng.below(24);
    for _ in 0..ndig {
        v.push(b'0' + rng.below(10) as u8);
    }
    let tails: [&[u8]; 8] = [b"", b"\n", b" ", b"\t", b"x", b".5", b" 77", b"abc\n"];
    let tail = *rng.pick(&tails);
    v.extend_from_slice(tail);
    v
}

/// C18: randomized stdin corpus over a pipe (fixed seed).
#[test]
fn config_c18_main_random_pipe() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for i in 0..1500 {
        let input = random_stdin(&mut rng);
        assert_main_case(
            &input,
            StdinKind::Pipe,
            StdoutKind::Pipe,
            &format!("C18 #{i}"),
        );
    }
}

/// C19: the same kind of corpus, but stdin is a regular file (different glibc
/// buffering path).
#[test]
fn config_c19_main_random_file() {
    let mut rng = Rng::new(0x0BAD_C0DE_0BAD_C0DE);
    for i in 0..600 {
        let input = random_stdin(&mut rng);
        assert_main_case(
            &input,
            StdinKind::File,
            StdoutKind::File,
            &format!("C19 #{i}"),
        );
    }
}

/// C20: stdin is `/dev/null`.
#[test]
fn config_c20_main_devnull_stdin() {
    assert_main_case(b"", StdinKind::Null, StdoutKind::Pipe, "C20");
    assert_main_case(b"ignored", StdinKind::Null, StdoutKind::File, "C20 file out");
}

/// C21: pipe closed right after a partial/complete number with **no**
/// terminator byte — `scanf` must not need one, and the Rust reader must not
/// wait for more input than the C does.
#[test]
fn config_c21_main_no_terminator() {
    for s in ["42", "-", "+", "0", "-0", "2147483647", "99999999999999999999"] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "C21");
    }
}

/// C22: multi-line input whose first line is whitespace only — `scanf` crosses
/// newlines (a `fgets`-based translation would fail here).
#[test]
fn config_c22_main_multiline() {
    for s in [
        "\n42\n",
        "   \n\t\n  -13\n",
        "\n\n\n\n\n\n7",
        "\r\n\r\n123\r\n",
        "  \n  \n  \n0\n\n99\n",
    ] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "C22");
    }
}

/// C23: the real executables (CMake `add_executable` product vs the Rust bin).
#[test]
fn config_c23_executable_corpus() {
    let fixed: Vec<Vec<u8>> = [
        "", "0", "7", "-7", "+7", " 42xyz", "abc", "-", "0x10", "010", "2147483647", "2147483648",
        "-2147483648", "-2147483649", "4294967296", "9223372036854775807",
        "9223372036854775808", "-9223372036854775808", "-9223372036854775809",
        "99999999999999999999", "\n\n 5", " \t\r\n-6\n", "42 43", "007",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    for (i, input) in fixed.iter().enumerate() {
        assert_exe_case(
            input,
            StdinKind::Pipe,
            StdoutKind::Pipe,
            &format!("C23 fixed #{i}"),
        );
    }
    let mut rng = Rng::new(0x4558_4553_4545_4400);
    for i in 0..600 {
        let input = random_stdin(&mut rng);
        assert_exe_case(
            &input,
            StdinKind::Pipe,
            StdoutKind::Pipe,
            &format!("C23 random #{i}"),
        );
    }
    // executable with stdin as a regular file and stdout as a regular file
    for (i, input) in fixed.iter().enumerate() {
        assert_exe_case(
            input,
            StdinKind::File,
            StdoutKind::File,
            &format!("C23 file #{i}"),
        );
    }
    assert_exe_case(b"", StdinKind::Null, StdoutKind::Pipe, "C23 devnull");
    assert_exe_case(b"", StdinKind::WriteOnlyFd, StdoutKind::Pipe, "C23 EBADF");
    assert_exe_case(b"", StdinKind::Directory, StdoutKind::Pipe, "C23 EISDIR");
    assert_exe_case(b"", StdinKind::Closed, StdoutKind::Pipe, "C23 closed fd 0");
}

// ===========================================================================
// Phase C — error paths (ERRORS.md rows)
// ===========================================================================

/// E1: empty stdin (immediate EOF).
#[test]
fn error_path_e1_empty_stdin() {
    assert_main_case(b"", StdinKind::Pipe, StdoutKind::Pipe, "E1");
    assert_main_case(b"", StdinKind::File, StdoutKind::Pipe, "E1 file");
}

/// E2: whitespace-only stdin, for every `isspace` byte and mixtures.
#[test]
fn error_path_e2_whitespace_only() {
    let ws: [&[u8]; 6] = [b" ", b"\t", b"\n", b"\x0b", b"\x0c", b"\r"];
    for w in ws {
        for reps in [1usize, 3, 100] {
            let mut v = Vec::new();
            for _ in 0..reps {
                v.extend_from_slice(w);
            }
            assert_main_case(&v, StdinKind::Pipe, StdoutKind::Pipe, "E2");
        }
    }
    assert_main_case(
        b" \t\n\x0b\x0c\r \t\n\x0b\x0c\r",
        StdinKind::Pipe,
        StdoutKind::Pipe,
        "E2 mixed",
    );
}

/// E3: fd 0 unreadable (`read` fails with `EBADF`).
#[test]
fn error_path_e3_read_error_ebadf() {
    assert_main_case(b"", StdinKind::WriteOnlyFd, StdoutKind::Pipe, "E3 write-only fd");
    assert_main_case(b"", StdinKind::Closed, StdoutKind::Pipe, "E3 closed fd 0");
}

/// E4: fd 0 is a directory (`read` fails with `EISDIR`).
#[test]
fn error_path_e4_read_error_eisdir() {
    assert_main_case(b"", StdinKind::Directory, StdoutKind::Pipe, "E4");
}

/// E5: first non-space byte is neither a sign nor a digit.
#[test]
fn error_path_e5_leading_garbage() {
    for s in [
        "abc", "a1", "x", "X", ".5", ".", ",", "/", ":", ";", "e", "e5", "#1", "*2", "(3)", "'4",
        "\"5", "[6]", "z9999", "_7", "=8", "@9", "~0", "|1", "%2", "$3", "&4", "!5", "?6", "^7",
        "<8", ">9",
    ] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E5");
    }
    // ... and with a whitespace prefix, so the skip loop runs first
    for s in ["  abc", "\n\n.5", "\t\t/", " \r\n z"] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E5 prefixed");
    }
}

/// E6: `-` alone.
#[test]
fn error_path_e6_lone_minus() {
    for s in ["-", " -", "-\n", "\n-", "  -  "] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E6");
    }
}

/// E7: `+` alone.
#[test]
fn error_path_e7_lone_plus() {
    for s in ["+", " +", "+\n", "\n+", "  +  "] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E7");
    }
}

/// E8: sign followed by a non-digit.
#[test]
fn error_path_e8_sign_then_nondigit() {
    for s in [
        "- 5", "+ 5", "--1", "++1", "-+1", "+-1", "-.5", "+.5", "-a", "+a", "-\t7", "+\n7", "-x1",
        "+X1", "-,", "+;",
    ] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E8");
    }
}

/// E9: NUL byte first.
#[test]
fn error_path_e9_nul_byte() {
    for s in [&b"\0"[..], &b"\0\0\0"[..], &b"\05"[..], &b" \0 5"[..], &b"-\0"[..]] {
        assert_main_case(s, StdinKind::Pipe, StdoutKind::Pipe, "E9");
    }
}

/// E10: non-ASCII bytes and non-ASCII "digits".
#[test]
fn error_path_e10_non_ascii() {
    let cases: [&[u8]; 10] = [
        b"\xff",
        b"\xff\xfe\xfd",
        b"\xd9\xa5",             // ARABIC-INDIC DIGIT FIVE
        b"\xef\xbc\x90",         // FULLWIDTH DIGIT ZERO
        b"\xc2\xa0" as &[u8],    // NBSP (not isspace in the C locale)
        b"\x80\x81",
        b"\xe2\x82\xac5",        // EURO SIGN then 5
        b"5\xff",                // valid number then a high byte
        b"-\xff",
        b"\xff5",
    ];
    for s in cases {
        assert_main_case(s, StdinKind::Pipe, StdoutKind::Pipe, "E10");
    }
}

/// E11: magnitude above `LONG_MAX` → glibc saturates to `LONG_MAX`, truncated
/// to `int` = -1.
#[test]
fn error_path_e11_overflow_positive() {
    let mut cases: Vec<String> = vec![
        "9223372036854775808".into(),
        "9223372036854775809".into(),
        "18446744073709551616".into(),
        "99999999999999999999".into(),
        "12345678901234567890123456789".into(),
    ];
    cases.push("9".repeat(1000));
    cases.push(format!("+{}", "8".repeat(300)));
    for s in &cases {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E11");
    }
}

/// E12: magnitude below `LONG_MIN` → saturates to `LONG_MIN`, truncated = 0.
#[test]
fn error_path_e12_overflow_negative() {
    let mut cases: Vec<String> = vec![
        "-9223372036854775809".into(),
        "-9223372036854775810".into(),
        "-18446744073709551616".into(),
        "-99999999999999999999".into(),
    ];
    cases.push(format!("-{}", "9".repeat(1000)));
    cases.push(format!("-{}", "7".repeat(300)));
    for s in &cases {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E12");
    }
}

/// E13/E14: exact `LONG_MAX` / `LONG_MIN` (no ERANGE, pure truncation).
#[test]
fn error_path_e13_e14_long_exact_boundaries() {
    for s in [
        "9223372036854775807",
        "+9223372036854775807",
        "-9223372036854775808",
        "0009223372036854775807",
        "-0009223372036854775808",
    ] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E13/E14");
    }
}

/// E15/E16: one step past `INT_MAX` / `INT_MIN`.
#[test]
fn error_path_e15_e16_one_past_int_range() {
    for s in ["2147483648", "-2147483649", "+2147483648"] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E15/E16");
    }
}

/// E17: other in-`long`, out-of-`int` values (silent truncation).
#[test]
fn error_path_e17_int_truncation() {
    for s in [
        "4294967296",
        "-4294967295",
        "4294967297",
        "1099511627776",
        "281474976710656",
        "-281474976710657",
        "72057594037927936",
    ] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E17");
    }
}

/// E18: base prefixes — `%d` is base 10 only.
#[test]
fn error_path_e18_base_prefixes() {
    for s in [
        "0x10", "0X10", "0xff", "0b1", "0B1", "010", "0o10", "00x5", "-0x10", "+0x10", "0x",
    ] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E18");
    }
}

/// E19: digits followed by garbage.
#[test]
fn error_path_e19_trailing_garbage() {
    for s in [
        "42xyz", "42.9", "42 43", "42\n43", "42-43", "42+43", "-42abc", "+42abc", "7e5", "9,9",
        "1234567890abcdefghij",
    ] {
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E19");
    }
}

/// E20: very long leading-zero run (no overflow may be reported).
#[test]
fn error_path_e20_huge_leading_zeros() {
    for (zeros, tail) in [(5000usize, "5"), (5000, "0"), (1000, "2147483648"), (64, "9")] {
        let s = format!("{}{}", "0".repeat(zeros), tail);
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E20");
        let s = format!("-{}{}", "0".repeat(zeros), tail);
        assert_main_case(s.as_bytes(), StdinKind::Pipe, StdoutKind::Pipe, "E20 neg");
    }
}

/// E21: `driver`'s unvalidated `int` extremes across the FFI boundary.
#[test]
fn error_path_e21_driver_extremes() {
    for v in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        let c = call_driver(&c_lib_path(), v);
        let r = call_driver(&rust_lib_path(), v);
        assert_eq!(c.stdout, r.stdout, "driver({v}) stdout differs");
        assert_eq!(c.status, r.status, "driver({v}) status differs");
        assert_eq!(c.status, Some(0), "driver({v}) must not fail in C");
    }
}

/// E22: `print_hex(len <= 0)` is unreachable through the exported API — assert
/// that the observable output is always the full 16-byte image (32 hex digits
/// plus '\n'), for both libraries.
#[test]
fn error_path_e22_print_hex_len_is_always_16() {
    let mut rng = Rng::new(0xFEED_FACE);
    let vals: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();
    for lib in [c_lib_path(), rust_lib_path()] {
        let out = call_driver_batch(&lib, &vals, StdoutKind::Pipe);
        assert_eq!(out.status, Some(0), "{lib:?}: {}", out.show());
        assert_eq!(
            out.stdout.len(),
            vals.len() * 33,
            "{lib:?} produced an unexpected byte count"
        );
        for line in out.stdout.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
            assert_eq!(line.len(), 32, "{lib:?}: line is not 32 hex digits");
            assert!(
                line.iter().all(|c| c.is_ascii_hexdigit()
                    && !c.is_ascii_uppercase()),
                "{lib:?}: line is not lowercase hex: {:?}",
                String::from_utf8_lossy(line)
            );
        }
    }
}

// ===========================================================================
// Phase B (continued) — stream/termination configurations C26..C30
// ===========================================================================

/// C26: stdin pipe stays OPEN after the payload. `scanf("%d")` returns as soon
/// as a non-digit terminates the digit run, so both libraries must exit
/// promptly; a translation that read stdin to EOF would hang.
#[test]
fn config_c26_main_must_not_wait_for_eof() {
    let hold = std::time::Duration::from_secs(5);
    let limit = std::time::Duration::from_secs(3);
    for s in [
        "42\n",
        "  -13\n",
        "+7 ",
        "0\t",
        "99999999999999999999\n",
        "abc\n",  // matching failure is decided by the first byte
        "-x\n",
        " \n\n 5\n",
        "2147483648\n",
        "7 8 9\n",
    ] {
        assert_main_case_holding_stdin(s.as_bytes(), hold, limit, "C26");
    }
}

/// C26b: no terminator byte at all — here both implementations legitimately wait
/// for EOF, and must produce the same result once it arrives.
#[test]
fn config_c26b_main_waits_only_for_the_lookahead() {
    let hold = std::time::Duration::from_millis(400);
    let limit = std::time::Duration::from_secs(8);
    for s in ["42", "-", "+", "0", "-2147483649", ""] {
        assert_main_case_holding_stdin(s.as_bytes(), hold, limit, "C26b");
    }
}

/// C27: the number arrives in several `read()` chunks (buffer refill path).
#[test]
fn config_c27_main_chunked_stdin() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn chunked(lib: &std::path::Path, chunks: &[&[u8]]) -> (Vec<u8>, Option<i32>) {
        let mut child = Command::new(runner_path())
            .arg(lib)
            .arg("main")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn runner");
        let mut si = child.stdin.take().unwrap();
        let owned: Vec<Vec<u8>> = chunks.iter().map(|c| c.to_vec()).collect();
        let t = std::thread::spawn(move || {
            for c in owned {
                if si.write_all(&c).is_err() {
                    break;
                }
                let _ = si.flush();
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        });
        let out = child.wait_with_output().expect("wait");
        let _ = t.join();
        (out.stdout, out.status.code())
    }

    let cases: [&[&[u8]]; 6] = [
        &[b"1", b"2", b"3\n"],
        &[b"  ", b"-", b"4", b"2", b"\n"],
        &[b"9", b"9999999999", b"9999999999", b"\n"],
        &[b"+", b"0", b"0", b"7", b" "],
        &[b"\n", b"\n", b" ", b"5", b"\n"],
        &[b"a", b"b", b"c", b"\n"],
    ];
    for (i, chunks) in cases.iter().enumerate() {
        let (co, cs) = chunked(&c_lib_path(), chunks);
        let (ro, rs) = chunked(&rust_lib_path(), chunks);
        assert_eq!(
            co, ro,
            "[C27 #{i}] chunked stdin {chunks:?} stdout differs: C={:?} Rust={:?}",
            String::from_utf8_lossy(&co),
            String::from_utf8_lossy(&ro)
        );
        assert_eq!(cs, rs, "[C27 #{i}] exit status differs");
    }
}

/// C28: stdout is `/dev/full` — every `write` fails with `ENOSPC`, and the C
/// ignores `printf`'s return value.
#[test]
fn config_c28_stdout_dev_full() {
    for s in ["", "42", "-1", "99999999999999999999"] {
        let c = call_main(&c_lib_path(), s.as_bytes(), StdinKind::Pipe, StdoutKind::DevFull);
        let r = call_main(&rust_lib_path(), s.as_bytes(), StdinKind::Pipe, StdoutKind::DevFull);
        assert_eq!(
            (c.status, c.signal),
            (r.status, r.signal),
            "[C28] status differs for stdin {s:?}\n  C   : {}\n  Rust: {}",
            c.show(),
            r.show()
        );
        assert_eq!(c.status, Some(0), "[C28] the C must still exit 0");
    }
    assert_exe_case(b"42", StdinKind::Pipe, StdoutKind::DevFull, "C28 exe");
    assert_exe_case(b"", StdinKind::Pipe, StdoutKind::DevFull, "C28 exe empty");
}

/// C29: stdout is a pipe with a closed read end — the first write raises
/// `SIGPIPE`. Both executables must die the same way (this is why the Rust bin
/// uses `#![no_main]`: `std`'s runtime would otherwise install `SIG_IGN`).
#[test]
fn config_c29_stdout_closed_pipe_sigpipe() {
    for s in ["", "42", "-7"] {
        let c = run(
            &c_exe_path(),
            &[],
            s.as_bytes(),
            StdinKind::Pipe,
            StdoutKind::ClosedPipe,
        );
        let r = run(
            &rust_exe_path(),
            &[],
            s.as_bytes(),
            StdinKind::Pipe,
            StdoutKind::ClosedPipe,
        );
        assert_eq!(
            (c.status, c.signal),
            (r.status, r.signal),
            "[C29] termination differs for stdin {s:?}\n  C   : {}\n  Rust: {}",
            c.show(),
            r.show()
        );
        assert_eq!(
            c.signal,
            Some(13),
            "[C29] expected the C to die from SIGPIPE, got {}",
            c.show()
        );
    }
}

/// C30: extra `argv` entries are ignored by `int main()`.
#[test]
fn config_c30_argv_ignored() {
    for args in [
        &[][..],
        &["extra"][..],
        &["-x", "--help"][..],
        &["1", "2", "3"][..],
    ] {
        assert_exe_case_args(b"11", args, "C30");
        assert_exe_case_args(b"", args, "C30 empty stdin");
    }
}

/// E23: a failing `printf` (`/dev/full`) is ignored by the C — no output, no
/// error, exit 0.
#[test]
fn error_path_e23_write_failure_ignored() {
    let c = call_main(&c_lib_path(), b"42", StdinKind::Pipe, StdoutKind::DevFull);
    let r = call_main(&rust_lib_path(), b"42", StdinKind::Pipe, StdoutKind::DevFull);
    assert_eq!((c.status, c.signal), (r.status, r.signal), "[E23] status");
    assert_eq!(c.status, Some(0), "[E23] C exit status");
    assert!(c.stdout.is_empty() && r.stdout.is_empty(), "[E23] no stdout expected");
}

/// E24: `SIGPIPE` parity for the executables (see C29).
#[test]
fn error_path_e24_sigpipe_parity() {
    let c = run(&c_exe_path(), &[], b"42", StdinKind::Pipe, StdoutKind::ClosedPipe);
    let r = run(&rust_exe_path(), &[], b"42", StdinKind::Pipe, StdoutKind::ClosedPipe);
    assert_eq!((c.status, c.signal), (r.status, r.signal), "[E24] termination differs");
}

/// E25: gigantic non-numeric input (100 KiB) — matching failure, and the
/// program must not read/consume more than it needs.
#[test]
fn error_path_e25_huge_garbage_input() {
    let big = vec![b'a'; 100 * 1024];
    assert_main_case(&big, StdinKind::File, StdoutKind::Pipe, "E25 file");
    let mut ws = vec![b' '; 100 * 1024];
    ws.extend_from_slice(b"77");
    assert_main_case(&ws, StdinKind::File, StdoutKind::Pipe, "E25 whitespace run");
    assert_main_case(&ws, StdinKind::Pipe, StdoutKind::Pipe, "E25 whitespace run pipe");
    let mut digits = vec![b'1'; 100 * 1024];
    digits.push(b'\n');
    assert_main_case(&digits, StdinKind::File, StdoutKind::Pipe, "E25 100k digits");
}

/// C31: nothing in the environment may change the behaviour — the C never calls
/// `setlocale`, so it stays in the "C" locale whatever `LC_*`/`LANG` say (this
/// is what makes the fixed ASCII `isspace`/digit sets in the Rust translation
/// correct).
#[test]
fn config_c31_environment_independence() {
    let envs: [&[(&str, &str)]; 6] = [
        &[],
        &[("LC_ALL", "C")],
        &[("LC_ALL", "C.UTF-8")],
        &[("LC_ALL", "de_DE.UTF-8"), ("LANG", "de_DE.UTF-8")],
        &[("LC_NUMERIC", "de_DE.UTF-8"), ("LC_CTYPE", "tr_TR.UTF-8")],
        &[("LANG", "ar_SA.UTF-8"), ("TZ", "UTC"), ("LC_ALL", "")],
    ];
    let inputs: [&[u8]; 8] = [
        b"1234567", b"1.234.567", b"1,234", b"  -42", b"\xd9\xa5", b"+0", b"99999999999999999999",
        b"\t\n 7",
    ];
    for env in envs {
        for input in inputs {
            let c = run_env(
                &runner_path(),
                &[c_lib_path().to_str().unwrap(), "main"],
                env,
                input,
                StdinKind::Pipe,
                StdoutKind::Pipe,
            );
            let r = run_env(
                &runner_path(),
                &[rust_lib_path().to_str().unwrap(), "main"],
                env,
                input,
                StdinKind::Pipe,
                StdoutKind::Pipe,
            );
            assert_eq!(
                c.stdout,
                r.stdout,
                "[C31] stdout differs with env {env:?} stdin {:?}\n  C   : {}\n  Rust: {}",
                esc(input),
                c.show(),
                r.show()
            );
            assert_eq!(
                (c.status, c.signal),
                (r.status, r.signal),
                "[C31] status differs with env {env:?} stdin {:?}",
                esc(input)
            );
            // the executables too
            let ce = run_env(&c_exe_path(), &[], env, input, StdinKind::Pipe, StdoutKind::Pipe);
            let re = run_env(&rust_exe_path(), &[], env, input, StdinKind::Pipe, StdoutKind::Pipe);
            assert_eq!(
                ce.stdout,
                re.stdout,
                "[C31 exe] stdout differs with env {env:?} stdin {:?}",
                esc(input)
            );
        }
    }
}

/// C32: `driver` and `main` used together in ONE process (driver, main, driver)
/// — the composed pipeline, not just isolated per-symbol calls.
#[test]
fn config_c32_mixed_entry_points_one_process() {
    let mut rng = Rng::new(0x00C0_FFEE_1234_ABCD);
    let cases: Vec<(i32, i32, Vec<u8>)> = (0..40)
        .map(|i| {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let stdin = match i % 5 {
                0 => b"42\n".to_vec(),
                1 => b"".to_vec(),
                2 => b"  -99999999999999999999".to_vec(),
                3 => b"abc".to_vec(),
                _ => format!("{}\n", rng.next_i32()).into_bytes(),
            };
            (a, b, stdin)
        })
        .collect();
    for (i, (a, b, stdin)) in cases.iter().enumerate() {
        let (av, bv) = (a.to_string(), b.to_string());
        let cl = c_lib_path();
        let rl = rust_lib_path();
        let c = run(
            &runner_path(),
            &[cl.to_str().unwrap(), "mixed", &av, &bv],
            stdin,
            StdinKind::Pipe,
            StdoutKind::Pipe,
        );
        let r = run(
            &runner_path(),
            &[rl.to_str().unwrap(), "mixed", &av, &bv],
            stdin,
            StdinKind::Pipe,
            StdoutKind::Pipe,
        );
        assert_eq!(
            c.stdout,
            r.stdout,
            "[C32 #{i}] driver({a}) + main({:?}) + driver({b}) differs\n  C   : {}\n  Rust: {}",
            esc(stdin),
            c.show(),
            r.show()
        );
        assert_eq!((c.status, c.signal), (r.status, r.signal), "[C32 #{i}] status differs");
        assert_eq!(
            c.stdout.len(),
            3 * 33,
            "[C32 #{i}] expected exactly three output lines from C"
        );
    }
}
