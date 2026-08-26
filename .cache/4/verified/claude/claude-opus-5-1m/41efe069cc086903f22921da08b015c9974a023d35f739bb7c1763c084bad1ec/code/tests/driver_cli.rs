//! CLI-level differential tests: `c_src/src/main.c` vs `src/main.rs`.
//!
//! Covers rows 25-33 of `ERRORS.md` (the `scanf` / length-limit rejections) and
//! the happy path of the whole program.  stdout, stderr and the exit status must
//! match byte for byte.
//!
//! ### Why some valid inputs are excluded
//!
//! `main.c` declares `uint8_t buffer[256]`, but `compact_runs()` can grow the
//! logical length beyond `length` when the threshold is `1` (`flags & 0x02` with
//! `param1 == 1`).  The C program then writes - and prints - past its own stack
//! array, which is undefined behaviour (ERRORS.md U4): the extra bytes it emits
//! are whatever the compiler happened to place behind the array.  Those inputs
//! are therefore excluded from *CLI* comparison; the same code path is covered
//! exhaustively at the library level (`tests/valid_paths.rs::c1_threshold_one_grows`),
//! where the harness owns a large enough buffer for the comparison to be
//! well defined.

mod common;

use common::*;
use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run_bytes(exe: &std::path::Path, input: &[u8]) -> Output {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    let _ = child.stdin.as_mut().unwrap().write_all(input);
    child.wait_with_output().unwrap()
}

fn run(exe: &std::path::Path, input: &str) -> Output {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn c_exe() -> std::path::PathBuf {
    c_driver_path()
}

fn rust_exe() -> std::path::PathBuf {
    rust_driver_path()
}

#[track_caller]
fn assert_cli_same(input: &str) {
    let c = run(&c_exe(), input);
    let r = run(&rust_exe(), input);
    let show = |o: &Output| {
        format!(
            "status={:?} stdout={:?} stderr={:?}",
            o.status.code(),
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
    };
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs for input {input:?}\n  C   : {}\n  Rust: {}",
        show(&c),
        show(&r)
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input {input:?}\n  C   : {}\n  Rust: {}",
        show(&c),
        show(&r)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input {input:?}\n  C   : {}\n  Rust: {}",
        show(&c),
        show(&r)
    );
}

#[track_caller]
fn assert_cli_same_bytes(input: &[u8]) {
    let c = run_bytes(&c_exe(), input);
    let r = run_bytes(&rust_exe(), input);
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs for input {input:?}"
    );
    assert_eq!(c.stdout, r.stdout, "stdout differs for input {input:?}");
    assert_eq!(c.stderr, r.stderr, "stderr differs for input {input:?}");
}

/// `true` when the C program stays inside its `uint8_t buffer[256]` for this
/// input (see the module comment).
fn c_stays_in_bounds(flags: u32, param1: i32, data: &[u8]) -> bool {
    if flags & 0x02 == 0 || param1 != 1 {
        // Thresholds >= 2 replace >= 2 bytes with 2 bytes, so the length never
        // grows; without `flags & 0x02` nothing changes the length at all.
        return true;
    }
    let runs = {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < data.len() {
            let v = data[i];
            n += 1;
            while i < data.len() && data[i] == v {
                i += 1;
            }
        }
        n
    };
    // Peak logical length is bounded by `len + runs`, final length by `2 * runs`.
    data.len() + runs <= 256 && 2 * runs <= 256
}

fn encode(flags: u32, param1: i32, param2: i32, data: &[u8]) -> String {
    let mut s = format!("{} {} {} {}", flags, param1, param2, data.len());
    for b in data {
        s.push(' ');
        s.push_str(&b.to_string());
    }
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------
// Rows 25-28 — scanf matching failures
// ---------------------------------------------------------------------------

#[test]
fn row25_flags_unreadable() {
    for input in [
        "", " ", "\n", "   \n\t\n", "abc", "abc 1 2 3", "-", "+", "-x", "+ 1 2 3", ".", "0x10 1 2 0",
        "e5 0 0 0",
    ] {
        assert_cli_same(input);
    }
}

#[test]
fn row26_param1_unreadable() {
    for input in [
        "5", "5 ", "5\n", "5 abc", "5 -", "5 +", "5 x 2 0", "5 - 2 0", "5 . 2 0",
    ] {
        assert_cli_same(input);
    }
}

#[test]
fn row27_param2_unreadable() {
    for input in ["5 1", "5 1 ", "5 1\n", "5 1 zz", "5 1 - 0", "5 1 + 0", "5 -1 q 0"] {
        assert_cli_same(input);
    }
}

#[test]
fn row28_length_unreadable() {
    for input in [
        "5 1 2", "5 1 2 ", "5 1 2\n", "5 1 2 q", "5 1 2 -", "5 1 2 +", "5 1 2 abc 1 2 3",
    ] {
        assert_cli_same(input);
    }
}

// ---------------------------------------------------------------------------
// Row 29 — length > 256
// ---------------------------------------------------------------------------

#[test]
fn row29_length_above_maximum() {
    for input in [
        "5 1 2 257",
        "5 1 2 258 1 2 3",
        "5 1 2 1000",
        "0 0 0 4294967296",
        "0 0 0 18446744073709551615",
        "0 0 0 18446744073709551616",
        "0 0 0 99999999999999999999999999",
        "0 0 0 -1",
        "0 0 0 -256",
        "0 0 0 -18446744073709551615",
    ] {
        assert_cli_same(input);
    }
}

// ---------------------------------------------------------------------------
// Row 30 — not enough data bytes
// ---------------------------------------------------------------------------

#[test]
fn row30_missing_data_bytes() {
    for input in [
        "0 0 0 1",
        "0 0 0 2 7",
        "0 0 0 3 7 8",
        "0 0 0 5 1 2 3 4",
        "0 0 0 4 1 2 x 4",
        "0 0 0 4 1 2 3 -",
        "0 0 0 256 1 2 3",
        "31 3 1 10 1 2 3 4 5",
    ] {
        assert_cli_same(input);
    }
}

// ---------------------------------------------------------------------------
// Row 31 — out-of-range values are accepted (strtol/strtoul saturation)
// ---------------------------------------------------------------------------

#[test]
fn row31_out_of_range_values_accepted() {
    for input in [
        // flags: %u  -> strtoul, truncated to unsigned int
        "4294967296 0 0 4 1 2 3 4",
        "4294967297 0 0 4 1 2 3 4",
        "18446744073709551615 0 0 4 1 2 3 4",
        "99999999999999999999999 0 0 4 1 2 3 4",
        "-1 0 0 4 1 2 3 4",
        "-2 0 0 4 1 2 3 4",
        "+31 3 1 4 1 2 3 4",
        // param1 / param2: %d -> strtol, truncated to int
        "2 2147483647 0 4 1 1 1 1",
        "2 2147483648 0 4 1 1 1 1",
        "2 -2147483648 0 4 1 1 1 1",
        "2 -2147483649 0 4 1 1 1 1",
        "2 9223372036854775807 0 4 1 1 1 1",
        "2 9223372036854775808 0 4 1 1 1 1",
        "2 -9223372036854775808 0 4 1 1 1 1",
        "2 -9223372036854775809 0 4 1 1 1 1",
        "2 99999999999999999999999 0 4 1 1 1 1",
        "4 0 99999999999999999999999 4 1 1 2 2",
        "4 0 -99999999999999999999999 4 1 1 2 2",
        "4 0 4294967296 4 1 1 2 2",
        // data bytes: %u -> strtoul, truncated to uint8_t
        "0 0 0 6 300 256 255 4294967296 4294967295 -1",
        "0 0 0 4 99999999999999999999999 -300 65536 65537",
        "16 4 0 8 +1 +2 +3 +4 -1 -2 -3 -4",
    ] {
        assert_cli_same(input);
    }
}

// ---------------------------------------------------------------------------
// Rows 32 / 33 — boundary lengths
// ---------------------------------------------------------------------------

#[test]
fn row32_length_exactly_256() {
    let data: Vec<u8> = (0..256u32).map(|i| (i % 256) as u8).collect();
    for flags in 0u32..0x20 {
        for p1 in [0i32, 2, 3, 4, 5, 255, 256, -1, i32::MIN, i32::MAX] {
            for p2 in [0, 1] {
                if !c_stays_in_bounds(flags, p1, &data) {
                    continue;
                }
                assert_cli_same(&encode(flags, p1, p2, &data));
            }
        }
    }
}

#[test]
fn row33_length_zero() {
    for input in [
        "0 0 0 0",
        "31 3 1 0",
        "4294967295 -1 -1 0",
        "2 1 0 0",
        "0 0 0 0 1 2 3",
    ] {
        assert_cli_same(input);
    }
}

// ---------------------------------------------------------------------------
// Whitespace / formatting acceptance (scanf skips arbitrary whitespace)
// ---------------------------------------------------------------------------

#[test]
fn scanf_whitespace_handling() {
    for input in [
        "0 0 0 3 1 2 3",
        "0\n0\n0\n3\n1\n2\n3\n",
        "  0\t0\r\n0 \t3\n\n 1  2   3  \n",
        "0 0 0 3 1 2 3 4 5 6",
        "31 3 1 4 1 1 2 2\n",
        "\n\n\n0 0 0 1 9\n",
    ] {
        assert_cli_same(input);
    }
}

#[test]
fn non_utf8_and_nul_bytes_on_stdin() {
    // `scanf` operates on bytes; the translation must not choke on input that is
    // not valid UTF-8 nor on embedded NUL bytes.
    let cases: &[&[u8]] = &[
        b"\xff",
        b"\xff\xfe 1 2 0",
        b"0 0 0 3 1 2 3\xff",
        b"0 0 0 3 1 2 \xff",
        b"0 0 0 3 1 \xff 3",
        b"0 \xc3\xa9 0 0",
        b"\x000 0 0 0",
        b"0 0 0 3 1 2 3\x00 4 5",
        b"0\x000 0 0",
        b"31 3 1 4 1 2 3 4\n\xff\xff\xff",
        b"\r\n\t\x0b\x0c 0 0 0 2 9 9",
    ];
    for input in cases {
        assert_cli_same_bytes(input);
    }
}

#[test]
fn no_trailing_newline_and_partial_tokens() {
    for input in [
        "0 0 0 2 1 2",   // no trailing newline
        "0 0 0 2 1",     // truncated mid-list, no newline
        "0 0 0",         // truncated after param2
        "0 0",           // truncated after param1
        "0",             // truncated after flags
        "0 0 0 1 25",    // complete, no newline
        "000000 0 0 1 007",
        "0 0 0 1 0000000000000000000000009",
    ] {
        assert_cli_same(input);
    }
}

// ---------------------------------------------------------------------------
// Happy path — randomised end-to-end sweep
// ---------------------------------------------------------------------------

#[test]
fn cli_happy_path_sweep() {
    let mut rng = Rng::new(0xC11_0001);
    let mut checked = 0usize;
    for &shape in &ALL_SHAPES {
        for len in [0usize, 1, 2, 3, 4, 5, 7, 8, 16, 17, 63, 64, 127, 128, 255, 256] {
            for flags in 0u32..0x20 {
                let data = make_input(shape, len, &mut rng);
                let p1 = match rng.below(8) {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    3 => 3,
                    4 => 4,
                    5 => 255,
                    6 => 256,
                    _ => rng.range_i32(-300, 300),
                };
                let p2 = if rng.below(2) == 0 { 0 } else { 1 };
                if !c_stays_in_bounds(flags, p1, &data) {
                    continue;
                }
                assert_cli_same(&encode(flags, p1, p2, &data));
                checked += 1;
            }
        }
    }
    assert!(checked > 2000, "only {checked} CLI cases ran");
}

#[test]
fn cli_threshold_one_growth_within_bounds() {
    // `param1 == 1` *is* comparable at CLI level as long as the C program stays
    // inside its 256 byte array; those are exactly the inputs with few runs.
    let mut rng = Rng::new(0xC11_0002);
    let mut checked = 0usize;
    for len in [1usize, 2, 3, 4, 8, 16, 32, 64, 100, 120, 127, 128] {
        for shape in [
            Shape::Constant,
            Shape::TwoBlocks,
            Shape::LongRuns,
            Shape::SmallAlphabet,
            Shape::Random,
            Shape::Alternating,
        ] {
            for _ in 0..4 {
                let data = make_input(shape, len, &mut rng);
                for flags in [0x02u32, 0x03, 0x06, 0x0A, 0x12, 0x1F] {
                    if !c_stays_in_bounds(flags, 1, &data) {
                        continue;
                    }
                    assert_cli_same(&encode(flags, 1, 1, &data));
                    assert_cli_same(&encode(flags, 1, 0, &data));
                    checked += 2;
                }
            }
        }
    }
    assert!(checked > 100, "only {checked} growth CLI cases ran");
}
