//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! `c_src/CMakeLists.txt` builds an executable, so the process boundary *is* the
//! public API: `main` and the five `static` helpers (`parse_val`,
//! `print_the_house`, `add_floor`, `add_floor_to_the_house`, `add_bedrooms`) are
//! only reachable this way. Each row drives both the C executable and the Rust
//! executable with the same stdin bytes and requires byte-identical stdout,
//! byte-identical stderr and an identical exit status (code *and* signal).
//!
//! Every row that has a value/shape domain is swept with many randomized inputs
//! from a fixed-seed xorshift64* generator, so the corpus is reproducible.

mod common;

use common::*;
use std::path::PathBuf;

struct Env {
    c: PathBuf,
    r: PathBuf,
    dir: PathBuf,
    row: &'static str,
    rng: Rng,
}

fn env(row: &'static str, seed: u64) -> Env {
    Env {
        c: c_exe().to_path_buf(),
        r: rust_exe(),
        dir: scratch(&format!("proc/{row}")),
        row,
        rng: Rng::new(seed),
    }
}

impl Env {
    /// Feed `input` to both executables (stdin from a regular file) and compare.
    fn check(&self, input: &[u8]) {
        let c = run_stdin_file(&self.c, &self.dir, input);
        let r = run_stdin_file(&self.r, &self.dir, input);
        assert_same(self.row, &describe(input), &c, &r);
    }
    fn check_str(&self, input: &str) {
        self.check(input.as_bytes());
    }
}

const WS: &[u8] = b" \t\n\x0b\x0c\r";

// ---------------------------------------------------------------------------
// C1 / C2 — plain small values
// ---------------------------------------------------------------------------

#[test]
fn cfg_c1_small_non_negative() {
    let mut e = env("c1", 0x1111);
    e.check_str("0\n");
    e.check_str("1\n");
    e.check_str("5\n");
    for _ in 0..200 {
        let v = e.rng.below(100_001) as i64;
        e.check_str(&format!("{v}\n"));
    }
}

#[test]
fn cfg_c2_small_negative() {
    let mut e = env("c2", 0x2222);
    e.check_str("-0\n");
    e.check_str("-1\n");
    for _ in 0..200 {
        let v = -(e.rng.below(100_001) as i64);
        e.check_str(&format!("{v}\n"));
    }
}

// ---------------------------------------------------------------------------
// C3 — no trailing newline (fgets stops at EOF instead)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c3_no_trailing_newline() {
    let mut e = env("c3", 0x3333);
    for _ in 0..100 {
        let v = e.rng.i32() as i64;
        e.check_str(&format!("{v}"));
    }
    for &v in BOUNDARY_I64 {
        e.check_str(&format!("{v}"));
    }
}

// ---------------------------------------------------------------------------
// C4 — explicit '+' sign
// ---------------------------------------------------------------------------

#[test]
fn cfg_c4_plus_sign() {
    let mut e = env("c4", 0x4444);
    e.check_str("+0\n");
    e.check_str("+2147483647\n");
    e.check_str("+2147483648\n");
    for _ in 0..100 {
        let v = e.rng.below(1 << 31) as u64;
        e.check_str(&format!("+{v}\n"));
    }
}

// ---------------------------------------------------------------------------
// C5 — leading zeros of every length 1..=40
// ---------------------------------------------------------------------------

#[test]
fn cfg_c5_leading_zeros() {
    let mut e = env("c5", 0x5555);
    for n in 1..=40usize {
        let v = e.rng.below(1000);
        e.check_str(&format!("{}{}\n", "0".repeat(n), v));
        e.check_str(&format!("-{}{}\n", "0".repeat(n), v));
        e.check_str(&format!("{}\n", "0".repeat(n)));
    }
}

// ---------------------------------------------------------------------------
// C6 — each individual whitespace character as prefix
// ---------------------------------------------------------------------------

#[test]
fn cfg_c6_single_whitespace_prefix() {
    let e = env("c6", 0x6666);
    for &w in WS {
        let mut v = Vec::new();
        v.push(w);
        v.extend_from_slice(b"42\n");
        e.check(&v);
    }
}

// ---------------------------------------------------------------------------
// C7 — random whitespace x sign x digits
// ---------------------------------------------------------------------------

#[test]
fn cfg_c7_random_whitespace_sign_digits() {
    let mut e = env("c7", 0x7777);
    for _ in 0..300 {
        let mut v = Vec::new();
        let n = e.rng.below(21);
        for _ in 0..n {
            // Exclude '\n' here: fgets would stop there. C8 covers that case.
            v.push(*e.rng.pick(b" \t\x0b\x0c\r"));
        }
        match e.rng.below(3) {
            0 => v.push(b'-'),
            1 => v.push(b'+'),
            _ => {}
        }
        let digits = e.rng.below(12);
        for _ in 0..digits {
            v.push(b'0' + e.rng.below(10) as u8);
        }
        if e.rng.below(2) == 0 {
            v.push(b'\n');
        }
        e.check(&v);
    }
}

// ---------------------------------------------------------------------------
// C8 — leading '\n' truncates the line before the value
// ---------------------------------------------------------------------------

#[test]
fn cfg_c8_leading_newline() {
    let e = env("c8", 0x8888);
    e.check_str("\n");
    e.check_str("\n5\n");
    e.check_str("\n\n5\n");
    e.check_str(" \n5\n");
    e.check_str("\t\n-7\n");
    for i in 0..16 {
        e.check_str(&format!("{}{}\n", "\n".repeat(i), 12345));
    }
}

// ---------------------------------------------------------------------------
// C9 — whitespace only
// ---------------------------------------------------------------------------

#[test]
fn cfg_c9_whitespace_only() {
    let mut e = env("c9", 0x9999);
    e.check_str("");
    e.check_str(" ");
    e.check_str("   ");
    e.check_str("\t\x0b\x0c\r ");
    e.check_str("\t\x0b\x0c\r \n");
    for _ in 0..20 {
        let n = e.rng.below(30);
        let mut v = Vec::new();
        for _ in 0..n {
            v.push(*e.rng.pick(b" \t\x0b\x0c\r"));
        }
        e.check(&v);
    }
}

// ---------------------------------------------------------------------------
// C10 — garbage with no digits at all
// ---------------------------------------------------------------------------

#[test]
fn cfg_c10_no_digits() {
    let mut e = env("c10", 0xA1A1);
    for s in [
        "abc", "abc\n", "x1", ".5", "-.5", "/9", ":9", "!", "?", "~", "#42", "$5", "%", "&",
        "'", "(", ")", "*", ",", "=", "@", "[", "]", "^", "_", "`", "{", "|", "}", "e5", "E5",
        "inf", "nan", "NULL", "true", "0b101".trim_start_matches('0'),
    ] {
        e.check_str(s);
    }
    for _ in 0..40 {
        let n = 1 + e.rng.below(12);
        let mut v = Vec::new();
        for _ in 0..n {
            // Printable non-digit, non-sign, non-whitespace bytes.
            loop {
                let b = 0x21 + e.rng.below(0x5e) as u8;
                if !b.is_ascii_digit() && b != b'+' && b != b'-' {
                    v.push(b);
                    break;
                }
            }
        }
        e.check(&v);
    }
}

// ---------------------------------------------------------------------------
// C11 — sign followed by a non-digit
// ---------------------------------------------------------------------------

#[test]
fn cfg_c11_sign_without_digits() {
    let e = env("c11", 0xB1B1);
    for s in [
        "-", "+", "-\n", "+\n", "- 5", "+ 5", "--5", "++5", "+-5", "-+5", "-x", "+x", "-.5",
        "+.5", "-\t5", "-abc", "   -   ", "   +",
    ] {
        e.check_str(s);
    }
}

// ---------------------------------------------------------------------------
// C12 — trailing garbage after a valid prefix
// ---------------------------------------------------------------------------

#[test]
fn cfg_c12_trailing_garbage() {
    let mut e = env("c12", 0xC1C1);
    for s in [
        "42abc", "42abc\n", "0x10", "0X10", "0b101", "1e5", "1E5", "5.9", "5,9", "5 6", "5\t6",
        "5-6", "5+6", "12/34", "7)", "9\n8\n", "-3xyz", "+8zzz", "2147483647abc",
        "2147483648abc", "-2147483648xx", "-2147483649xx",
    ] {
        e.check_str(s);
    }
    for _ in 0..40 {
        let v = e.rng.i32();
        let n = 1 + e.rng.below(8);
        let mut s = format!("{v}");
        for _ in 0..n {
            s.push((0x61 + e.rng.below(26)) as u8 as char);
        }
        s.push('\n');
        e.check_str(&s);
    }
}

// ---------------------------------------------------------------------------
// C13 / C14 — exact 32-bit and 64-bit boundaries
// ---------------------------------------------------------------------------

#[test]
fn cfg_c13_int_boundaries() {
    let e = env("c13", 0xD1D1);
    for &v in BOUNDARY_I64 {
        e.check_str(&format!("{v}\n"));
        e.check_str(&format!("{v}"));
        e.check_str(&format!("  {v}  \n"));
    }
}

#[test]
fn cfg_c14_long_boundaries() {
    let e = env("c14", 0xE1E1);
    for s in [
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "-9223372036854775810",
        "18446744073709551615",
        "18446744073709551616",
        "-18446744073709551615",
        "+9223372036854775807",
        "+9223372036854775808",
        "00009223372036854775807",
        "00009223372036854775808",
    ] {
        e.check_str(&format!("{s}\n"));
        e.check_str(s);
    }
}

// ---------------------------------------------------------------------------
// C15 — 20..98 digit values (far beyond LONG_MAX -> ERANGE)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c15_very_long_digit_runs() {
    let mut e = env("c15", 0xF1F1);
    for n in 20..=98usize {
        let mut s = String::new();
        s.push('1');
        for _ in 1..n {
            s.push((b'0' + e.rng.below(10) as u8) as char);
        }
        e.check_str(&format!("{s}\n"));
        e.check_str(&format!("-{s}\n"));
        e.check_str(&"9".repeat(n));
    }
}

// ---------------------------------------------------------------------------
// C16 / C17 — uniformly random 32-bit and 64-bit values
// ---------------------------------------------------------------------------

#[test]
fn cfg_c16_random_i32() {
    let mut e = env("c16", 0x0102);
    for _ in 0..500 {
        let v = e.rng.i32() as i64;
        e.check_str(&format!("{v}\n"));
        e.check_str(&format!("{}\n", v + 1));
        e.check_str(&format!("{}\n", v - 1));
    }
}

#[test]
fn cfg_c17_random_i64() {
    let mut e = env("c17", 0x0304);
    for _ in 0..500 {
        let v = e.rng.i64();
        e.check_str(&format!("{v}\n"));
    }
}

// ---------------------------------------------------------------------------
// C18 — line-length sweep across the 99-byte fgets bound
// ---------------------------------------------------------------------------

#[test]
fn cfg_c18_length_sweep() {
    let mut e = env("c18", 0x0506);
    for n in 0..=102usize {
        // A line of exactly n bytes made of digits.
        let mut s: String = (0..n)
            .map(|_| (b'1' + e.rng.below(9) as u8) as char)
            .collect();
        e.check_str(&s);
        s.push('\n');
        e.check_str(&s);
        // Same length, but with the digits at the very end so the 99-byte cut
        // lands before / inside them.
        let pad = n.saturating_sub(3);
        let s2 = format!("{}{}", " ".repeat(pad), "742"[..n.min(3)].to_string());
        e.check_str(&s2);
    }
}

// ---------------------------------------------------------------------------
// C19 — the 99-byte prefix is a *different valid* value
// ---------------------------------------------------------------------------

#[test]
fn cfg_c19_truncation_changes_value() {
    let e = env("c19", 0x0708);
    for prefix in [96usize, 97, 98, 99, 100] {
        e.check_str(&format!("{}42\n", " ".repeat(prefix)));
        e.check_str(&format!("{}-42\n", " ".repeat(prefix)));
        e.check_str(&format!("{}+42\n", " ".repeat(prefix)));
    }
    // 90 spaces then a 12-digit number: only the first 9 digits survive.
    e.check_str(&format!("{}123456789012\n", " ".repeat(90)));
    e.check_str(&format!("{}-123456789012\n", " ".repeat(90)));
}

// ---------------------------------------------------------------------------
// C20 — truncation turns the value into an ERANGE case
// ---------------------------------------------------------------------------

#[test]
fn cfg_c20_truncation_causes_erange() {
    let e = env("c20", 0x090A);
    for n in [99usize, 100, 120, 200] {
        e.check_str(&format!("{}\n", "9".repeat(n)));
        e.check_str(&format!("-{}\n", "9".repeat(n)));
    }
    // 99 bytes of digits followed by more digits -> the tail is dropped.
    e.check_str(&format!("{}{}\n", "1".repeat(99), "2".repeat(50)));
}

// ---------------------------------------------------------------------------
// C21 — embedded NUL bytes
// ---------------------------------------------------------------------------

#[test]
fn cfg_c21_embedded_nul() {
    let e = env("c21", 0x0B0C);
    let cases: &[&[u8]] = &[
        b"\x0042\n",
        b"4\x002\n",
        b"42\x00\n",
        b"42\n\x00",
        b"42\x0034\n",
        b"\x00\x00\x0042\n",
        b"  \x0042\n",
        b"-\x0042\n",
        b"-4\x002\n",
        b"2147483647\x008\n",
        b"214748364\x007\n",
        b"\x00",
        b"\x00\n",
    ];
    for c in cases {
        e.check(c);
    }
    // A NUL at every offset of a fixed line.
    let base = b"  -1234567890\n";
    for i in 0..base.len() {
        let mut v = base.to_vec();
        v[i] = 0;
        e.check(&v);
    }
}

// ---------------------------------------------------------------------------
// C22 — high bytes (>= 0x80), invalid UTF-8
// ---------------------------------------------------------------------------

#[test]
fn cfg_c22_high_bytes() {
    let mut e = env("c22", 0x0D0E);
    for b in [0x80u8, 0x9f, 0xa0, 0xc2, 0xe2, 0xf0, 0xfe, 0xff] {
        e.check(&[b, b'4', b'2', b'\n']);
        e.check(&[b'4', b'2', b, b'\n']);
        e.check(&[b'-', b, b'4', b'2', b'\n']);
        e.check(&[b, b]);
        // UTF-8 NBSP / ideographic space: whitespace in Unicode, not in C.
        e.check(&[0xc2, 0xa0, b'4', b'2', b'\n']);
        e.check(&[0xe3, 0x80, 0x80, b'4', b'2', b'\n']);
    }
    for _ in 0..100 {
        let n = 1 + e.rng.below(10);
        let mut v = Vec::new();
        for _ in 0..n {
            v.push(0x80 | (e.rng.byte() & 0x7f));
        }
        v.extend_from_slice(b"42\n");
        e.check(&v);
    }
}

// ---------------------------------------------------------------------------
// C23 — fully random byte strings
// ---------------------------------------------------------------------------

#[test]
fn cfg_c23_random_bytes() {
    let mut e = env("c23", 0x0F10);
    for _ in 0..1000 {
        let n = e.rng.below(201);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(e.rng.byte());
        }
        e.check(&v);
    }
}

// ---------------------------------------------------------------------------
// C24 — grammar-directed fuzz (biased towards digits/signs/spaces/NUL)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c24_grammar_fuzz() {
    let mut e = env("c24", 0x1112);
    let alphabet: &[u8] = b"0123456789 \t\r\x0b\x0c+-abcxX.,eE\n\x00\x7f\xff/:";
    for _ in 0..1000 {
        let n = e.rng.below(120);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(*e.rng.pick(alphabet));
        }
        e.check(&v);
    }
}

// ---------------------------------------------------------------------------
// C25 / C26 / C27 — int overflow in add_bedrooms
// ---------------------------------------------------------------------------

#[test]
fn cfg_c25_overflow_on_first_run() {
    let e = env("c25", 0x1314);
    // bedrooms starts at 5, so anything > INT_MAX-5 overflows on the first run.
    for v in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 2,
        i32::MAX - 3,
        i32::MAX - 4,
        i32::MAX - 5,
        i32::MAX - 6,
        2147483643,
    ] {
        e.check_str(&format!("{v}\n"));
    }
}

#[test]
fn cfg_c26_overflow_on_second_run() {
    let e = env("c26", 0x1516);
    // Overflow appears only once extra_bedrooms has been added twice.
    for v in [
        1073741821i32,
        1073741822,
        1073741823,
        1073741824,
        1073741825,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
    ] {
        e.check_str(&format!("{v}\n"));
    }
}

#[test]
fn cfg_c27_negative_overflow() {
    let e = env("c27", 0x1718);
    for v in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 5,
        i32::MIN + 6,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
        -1073741824,
        -1073741825,
    ] {
        e.check_str(&format!("{v}\n"));
    }
}

// ---------------------------------------------------------------------------
// C28..C31 — stdin shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg_c28_stdin_eof() {
    let e = env("c28", 0x191A);
    e.check(b"");
    let c = run_stdin_file(&e.c, &e.dir, b"");
    let r = run_stdin_file(&e.r, &e.dir, b"");
    assert_same("c28", "empty", &c, &r);
    assert_eq!(c.stdout, b"An error occurred\n");
    // /dev/null
    let dn = std::path::Path::new("/dev/null");
    let cf = std::fs::File::open(dn).unwrap();
    let rf = std::fs::File::open(dn).unwrap();
    use std::process::{Command, Stdio};
    let co = Command::new(&e.c).stdin(Stdio::from(cf)).output().unwrap();
    let ro = Command::new(&e.r).stdin(Stdio::from(rf)).output().unwrap();
    assert_eq!(co.stdout, ro.stdout, "c28 /dev/null stdout");
    assert_eq!(co.status.code(), ro.status.code(), "c28 /dev/null status");
}

#[test]
fn cfg_c29_stdin_pipe() {
    let mut e = env("c29", 0x1B1C);
    for _ in 0..60 {
        let v = e.rng.i32() as i64;
        let s = format!("{v}\n");
        let c = run_stdin_pipe(&e.c, &[s.as_bytes()]);
        let r = run_stdin_pipe(&e.r, &[s.as_bytes()]);
        assert_same("c29", &s, &c, &r);
    }
    for s in ["", "abc", "  12", "9999999999999999999999\n"] {
        let c = run_stdin_pipe(&e.c, &[s.as_bytes()]);
        let r = run_stdin_pipe(&e.r, &[s.as_bytes()]);
        assert_same("c29", s, &c, &r);
    }
}

#[test]
fn cfg_c30_stdin_pipe_chunked() {
    let e = env("c30", 0x1D1E);
    let cases: &[&[&[u8]]] = &[
        &[b"1", b"2", b"3", b"\n"],
        &[b"  ", b"-", b"4", b"2", b"\n"],
        &[b"21474836", b"47", b"\n"],
        &[b"21474836", b"48", b"\n"],
        &[b"abc", b"42", b"\n"],
        &[b"", b"7", b"\n"],
        &[b"7"],
        &[b"\n", b"7", b"\n"],
    ];
    for chunks in cases {
        let c = run_stdin_pipe(&e.c, chunks);
        let r = run_stdin_pipe(&e.r, chunks);
        assert_same("c30", &format!("{chunks:?}"), &c, &r);
    }
}

#[test]
fn cfg_c31_stdin_unreadable() {
    let e = env("c31", 0x1F20);
    // fd 0 closed before exec
    let c = run_with_closed_fds(&e.c, &e.dir, b"42\n", &[0]);
    let r = run_with_closed_fds(&e.r, &e.dir, b"42\n", &[0]);
    assert_same("c31", "fd0 closed", &c, &r);
    // stdin opened on a directory -> read() fails with EISDIR
    let c = run_stdin_directory(&e.c);
    let r = run_stdin_directory(&e.r);
    assert_same("c31", "stdin=directory", &c, &r);
}

// ---------------------------------------------------------------------------
// C32..C35 — stdout shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg_c32_stdout_regular_file() {
    let mut e = env("c32", 0x2122);
    for i in 0..40 {
        let v = e.rng.i32() as i64;
        let s = format!("{v}\n");
        let tag = format!("{i}");
        let c = run_stdout_file(&e.c, &e.dir, s.as_bytes(), &format!("c{tag}"));
        let r = run_stdout_file(&e.r, &e.dir, s.as_bytes(), &format!("r{tag}"));
        assert_same("c32", &s, &c, &r);
    }
    for s in ["", "abc", "2147483648\n"] {
        let c = run_stdout_file(&e.c, &e.dir, s.as_bytes(), "cx");
        let r = run_stdout_file(&e.r, &e.dir, s.as_bytes(), "rx");
        assert_same("c32", s, &c, &r);
    }
}

#[test]
fn cfg_c33_stdout_live_pipe() {
    // `run_stdin_file` already collects stdout through a pipe; assert explicitly
    // that the byte stream is identical for a drained pipe, which is a different
    // libc buffering mode than a regular file.
    let mut e = env("c33", 0x2324);
    for _ in 0..40 {
        let v = e.rng.i32() as i64;
        e.check_str(&format!("{v}\n"));
    }
}

#[test]
fn cfg_c34_stdout_closed_pipe_sigpipe() {
    let e = env("c34", 0x2526);
    for s in ["5\n", "-7\n", "abc\n", ""] {
        let c = run_stdout_closed_pipe(&e.c, &e.dir, s.as_bytes());
        let r = run_stdout_closed_pipe(&e.r, &e.dir, s.as_bytes());
        assert_same("c34", s, &c, &r);
    }
}

#[test]
fn cfg_c35_stdout_closed_fd() {
    let e = env("c35", 0x2728);
    for s in ["5\n", "abc\n", ""] {
        let c = run_with_closed_fds(&e.c, &e.dir, s.as_bytes(), &[1]);
        let r = run_with_closed_fds(&e.r, &e.dir, s.as_bytes(), &[1]);
        assert_same("c35", s, &c, &r);
    }
}

// ---------------------------------------------------------------------------
// C36 — argv is ignored by `int main()`
// ---------------------------------------------------------------------------

#[test]
fn cfg_c36_argv_ignored() {
    let e = env("c36", 0x292A);
    for args in [
        vec![],
        vec!["7"],
        vec!["--help"],
        vec!["-1", "-2", "-3"],
        vec![""],
    ] {
        for s in ["5\n", "abc\n"] {
            let c = run_with_args(&e.c, &e.dir, s.as_bytes(), &args);
            let r = run_with_args(&e.r, &e.dir, s.as_bytes(), &args);
            assert_same("c36", &format!("{args:?} {s:?}"), &c, &r);
        }
    }
}

// ---------------------------------------------------------------------------
// The C's own optimisation level must not change the answer either — this pins
// down that the wrapping the Rust reproduces is what gcc emits at every -O.
// ---------------------------------------------------------------------------

#[test]
fn c_optimization_levels_agree_with_rust() {
    let dir = scratch("copt");
    let variants: Vec<PathBuf> = ["", "-O0", "-O1", "-O2", "-O3", "-Os"]
        .iter()
        .map(|o| build_c_exe(&dir, o))
        .collect();
    let r = rust_exe();
    let mut rng = Rng::new(0x2B2C);
    let mut inputs: Vec<String> = BOUNDARY_I64.iter().map(|v| format!("{v}\n")).collect();
    inputs.extend(
        [
            "9223372036854775808\n",
            "-9223372036854775809\n",
            "abc\n",
            "",
            "  +0042xyz\n",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    for _ in 0..100 {
        inputs.push(format!("{}\n", rng.i32()));
    }
    for input in &inputs {
        let expect = run_stdin_file(&r, &dir, input.as_bytes());
        for c in &variants {
            let got = run_stdin_file(c, &dir, input.as_bytes());
            assert_same(
                "copt",
                &format!("{} @ {}", describe(input.as_bytes()), c.display()),
                &got,
                &expect,
            );
        }
    }
}
