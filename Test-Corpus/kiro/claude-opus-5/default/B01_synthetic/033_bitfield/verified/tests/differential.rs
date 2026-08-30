//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses with identical stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust code is never linked as a library here — only the built executable
//! is driven, exactly the way the C program is driven when the two are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Absolute path of the `translation/` crate root.
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Absolute path of `c_src/`, which sits next to `translation/`.
fn c_src_dir() -> PathBuf {
    crate_dir().parent().expect("crate has a parent dir").join("c_src")
}

/// Path of the Rust executable under test (built by cargo for us).
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path of the C executable, building it with cmake on first use if needed.
fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let build_dir = c_src_dir().join("build");
    let bin = build_dir.join("driver");

    BUILD.call_once(|| {
        if bin.exists() {
            return;
        }
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .status()
            .expect("run cmake (is cmake installed?)");
        assert!(configure.success(), "cmake configure failed");
        let build = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .status()
            .expect("run cmake --build");
        assert!(build.success(), "cmake --build failed");
    });

    assert!(
        bin.exists(),
        "C reference binary missing at {}; build it with:\n  \
         cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .",
        bin.display()
    );
    bin
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
    signal: Option<i32>,
}

fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        // The child may exit before consuming all input; a broken pipe here is
        // not a test failure, it is information the comparison already covers.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
        signal,
    }
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:?}"),
    }
}

/// Runs both binaries on `input` and asserts stdout, stderr and exit status
/// all agree byte for byte.
fn assert_same(name: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{name}] stdout differs for input {}\n  C   : {}\n  Rust: {}",
        show(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{name}] stderr differs for input {}\n  C   : {}\n  Rust: {}",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "[{name}] exit status differs for input {}: C={:?} Rust={:?}",
        show(input),
        c.status,
        r.status
    );
    assert_eq!(
        c.signal,
        r.signal,
        "[{name}] terminating signal differs for input {}: C={:?} Rust={:?}",
        show(input),
        c.signal,
        r.signal
    );
}

fn check_all(cases: &[(&str, &[u8])]) {
    for (name, input) in cases {
        assert_same(name, input);
    }
}

// ---------------------------------------------------------------------------
// Input counts: main() issues exactly four scanf() calls. Anything not
// converted leaves the corresponding variable at its initial 0.
// ---------------------------------------------------------------------------

#[test]
fn item_counts_zero_through_five() {
    check_all(&[
        ("empty", b""),
        ("one", b"1"),
        ("two", b"1 2"),
        ("three", b"1 2 3"),
        ("four", b"1 2 3 4"),
        ("five_extra_ignored", b"1 2 3 4 5"),
        ("many_extra_ignored", b"1 2 3 4 5 6 7 8 9 10"),
    ]);
}

// ---------------------------------------------------------------------------
// scanf() skips leading whitespace, so conversions cross newlines. fgets()
// semantics (line-bounded reads) must NOT appear.
// ---------------------------------------------------------------------------

#[test]
fn whitespace_and_line_crossing() {
    check_all(&[
        ("newline_separated", b"1\n2\n3\n4\n"),
        ("one_per_line_no_trailing_nl", b"1\n2\n3\n4"),
        ("crlf", b"1\r\n2\r\n3\r\n4\r\n"),
        ("tabs", b"\t1\t2\t3\t4\t"),
        ("leading_blank_lines", b"   \n\n  1 2 3 4"),
        ("vertical_tab", b"\x0b1\x0b2\x0b3\x0b4"),
        ("form_feed", b"\x0c1\x0c2\x0c3\x0c4"),
        ("all_whitespace_only", b"   \t\n\x0b\x0c\r  "),
        ("single_newline", b"\n"),
        ("only_newlines", b"\n\n\n\n"),
        ("blank_then_eof", b"        "),
    ]);
}

#[test]
fn long_whitespace_run() {
    let mut input = vec![b' '; 4096];
    input.extend_from_slice(b"1 2 3 4");
    assert_same("long_whitespace_run", &input);

    let only_ws = vec![b'\n'; 4096];
    assert_same("long_newline_run_only", &only_ws);
}

// ---------------------------------------------------------------------------
// Bit-field truncation: `unsigned int x : 2` keeps 2 bits, `y : 3` keeps 3.
// Exhaustive sweep over both fields' wrap boundaries.
// ---------------------------------------------------------------------------

#[test]
fn bitfield_truncation_sweep() {
    for x in 0u32..=20 {
        for y in 0u32..=20 {
            let input = format!("{x} {y} 0 0");
            assert_same(&format!("sweep_x{x}_y{y}"), input.as_bytes());
        }
    }
}

#[test]
fn bitfield_truncation_boundaries() {
    check_all(&[
        ("x_3_max", b"3 0 0 0"),
        ("x_4_wraps", b"4 0 0 0"),
        ("x_7_wraps", b"7 0 0 0"),
        ("y_7_max", b"0 7 0 0"),
        ("y_8_wraps", b"0 8 0 0"),
        ("y_15_wraps", b"15 15 0 0"),
        ("u32_max_both", b"4294967295 4294967295 0 0"),
        ("pow2_31", b"2147483648 2147483648 0 0"),
        ("pow2_32", b"4294967296 4294967297 0 0"),
    ]);
}

// ---------------------------------------------------------------------------
// `bool b : 1` fed with `!!b`: every non-zero int becomes 1, and truncation
// of the scanned value to `int` happens before the `!!`.
// ---------------------------------------------------------------------------

#[test]
fn bool_field_normalisation() {
    check_all(&[
        ("b_zero", b"1 2 0 9"),
        ("b_one", b"1 2 1 9"),
        ("b_negative_one", b"1 2 -1 9"),
        ("b_negative_zero", b"1 2 -0 9"),
        ("b_256", b"1 2 256 9"),
        ("b_2", b"1 2 2 9"),
        // 2^32 truncates to int 0, so !!b is 0 even though the text is non-zero.
        ("b_pow2_32_truncates_to_zero", b"1 2 4294967296 9"),
        ("b_pow2_32_plus_one", b"1 2 4294967297 9"),
        ("b_int_min", b"1 2 -2147483648 9"),
        ("b_int_max", b"1 2 2147483647 9"),
    ]);
}

// ---------------------------------------------------------------------------
// The plain `int z` field: signedness, truncation from long, saturation.
// ---------------------------------------------------------------------------

#[test]
fn z_signedness_and_truncation() {
    check_all(&[
        ("z_negative", b"1 2 3 -5"),
        ("z_int_min", b"0 0 0 -2147483648"),
        ("z_int_max", b"0 0 0 2147483647"),
        ("z_int_max_plus_one", b"0 0 0 2147483648"),
        ("z_int_min_minus_one", b"0 0 0 -2147483649"),
        ("z_pow2_32", b"0 0 0 4294967296"),
        ("z_i64_max", b"0 0 0 9223372036854775807"),
        ("z_i64_max_plus_one_saturates", b"0 0 0 9223372036854775808"),
        ("z_i64_min", b"0 0 0 -9223372036854775808"),
        ("z_i64_min_minus_one_saturates", b"0 0 0 -9223372036854775809"),
        ("z_huge_positive", b"0 0 0 99999999999999999999"),
        ("z_huge_negative", b"0 0 0 -99999999999999999999"),
        ("z_pow2_64_negative", b"0 0 0 -18446744073709551616"),
    ]);
}

// ---------------------------------------------------------------------------
// `%u` accepts a sign; strtoul wraps negatives modulo 2^64 and saturates to
// ULONG_MAX on overflow, and the result is then truncated to unsigned int.
// ---------------------------------------------------------------------------

#[test]
fn unsigned_conversion_of_signed_and_overflowing_text() {
    check_all(&[
        ("u_minus_one", b"-1 -1 0 0"),
        ("u_minus_three_minus_five", b"-3 -5 0 0"),
        ("u_minus_zero", b"-0 -0 -0 -0"),
        ("u_explicit_plus", b"+1 +2 +3 +4"),
        ("u_minus_u32_max", b"-4294967295 0 0 0"),
        ("u_u64_max", b"18446744073709551615 0 0 0"),
        ("u_u64_max_plus_one_saturates", b"18446744073709551616 0 0 0"),
        ("u_minus_pow2_64", b"-18446744073709551616 0 0 0"),
        ("u_minus_u64_max", b"-18446744073709551615 0 0 0"),
        ("u_minus_pow2_64_plus_one", b"-18446744073709551617 0 0 0"),
        ("u_huge", b"99999999999999999999 0 0 0"),
        ("u_very_huge", b"1000000000000000000000000000 0 0 0"),
        ("u_leading_zeros", b"007 0008 0009 0010"),
    ]);
}

#[test]
fn very_long_digit_strings() {
    let nines = format!("{} 0 0 0", "9".repeat(500));
    assert_same("five_hundred_nines", nines.as_bytes());

    let neg_nines = format!("-{} 0 0 0", "9".repeat(500));
    assert_same("five_hundred_nines_negative", neg_nines.as_bytes());

    let padded = format!("{}5 0 0 0", "0".repeat(500));
    assert_same("five_hundred_leading_zeros", padded.as_bytes());

    let padded_neg = format!("-{}5 0 0 0", "0".repeat(500));
    assert_same("five_hundred_leading_zeros_negative", padded_neg.as_bytes());

    let ten_k = format!("{} 2 3 4", "1".repeat(10_000));
    assert_same("ten_thousand_digits", ten_k.as_bytes());

    let z_long = format!("0 0 0 -{}", "7".repeat(1_000));
    assert_same("thousand_digit_negative_z", z_long.as_bytes());
}

// ---------------------------------------------------------------------------
// Matching failures. A failed conversion leaves its variable at 0 and leaves
// the offending character in the stream, so every later conversion fails too
// unless the failure stopped on whitespace.
// ---------------------------------------------------------------------------

#[test]
fn matching_failures() {
    check_all(&[
        ("all_letters", b"abc"),
        ("letter_first", b"a1 2 3 4"),
        ("letter_second", b"1 abc 3 4"),
        ("letter_third", b"1 2 a 4"),
        ("letter_fourth", b"1 2 3 a"),
        ("digits_then_letters", b"12a 34b 56c 78d"),
        ("hex_prefix", b"0x10 0x20 1 1"),
        ("floats", b"1.5 2.5 3.5 4.5"),
        ("exponent_notation", b"1e5 2 3 4"),
        ("comma_separated", b"1,2,3,4"),
        ("underscore", b"1_2 3 4 5"),
    ]);
}

#[test]
fn sign_only_and_stray_sign_handling() {
    check_all(&[
        ("minus_only", b"-"),
        ("plus_only", b"+"),
        ("minus_then_letter", b"-a 1 2 3"),
        ("plus_then_space", b"+ 1 2 3"),
        ("minus_then_space", b"- 1 2 3"),
        ("minus_then_newline", b"-\n1 2 3"),
        ("sign_then_sign", b"-+1 2 3 4"),
        ("plus_then_minus", b"+-1 2 3 4"),
        ("double_minus", b"--1 2 3 4"),
        ("sign_glued_mid_token", b"12+34 5 6"),
        ("minus_glued_mid_token", b"12-34 5 6"),
        ("lone_dash_between", b"1 - 2 3"),
        ("signs_only_four", b"- - - -"),
        ("plus_at_eof_after_three", b"1 2 3 +"),
    ]);
}

// ---------------------------------------------------------------------------
// Bytes that are neither digits, signs nor whitespace, including NUL and
// non-ASCII, must fail conversion identically.
// ---------------------------------------------------------------------------

#[test]
fn non_ascii_and_nul_bytes() {
    check_all(&[
        ("nul_after_four_values", b"1 2 3 4\x00 5"),
        ("nul_first", b"\x001 2 3 4"),
        ("nul_second", b"1 \x00 2 3"),
        ("utf8_leading", b"\xc3\xa91 2 3 4"),
        ("high_byte", b"\xff 1 2 3"),
        ("high_byte_after_two", b"1 2 \xfe 4"),
        ("all_high_bytes", b"\x80\x81\x82\x83"),
    ]);
}

// ---------------------------------------------------------------------------
// Token adjacency: no separator before EOF, and trailing junk after the
// fourth value (which is never read).
// ---------------------------------------------------------------------------

#[test]
fn adjacency_and_trailing_junk() {
    check_all(&[
        ("no_trailing_separator", b"1 2 3 4"),
        ("trailing_junk_unread", b"1 2 3 4no"),
        ("trailing_junk_with_newline", b"1 2 3 4\nnot read at all\n"),
        ("glued_digits", b"1234"),
        ("single_zero", b"0"),
        ("all_zeros", b"0 0 0 0"),
    ]);
}

// ---------------------------------------------------------------------------
// stdin closed immediately (no bytes at all) versus stdin held open with no
// data before EOF — both must yield the four initial zeros.
// ---------------------------------------------------------------------------

#[test]
fn empty_stdin_variants() {
    check_all(&[("closed_stdin", b""), ("whitespace_then_eof", b" \n\t")]);
}

// ---------------------------------------------------------------------------
// A broad randomised sweep, seeded deterministically, to catch anything the
// hand-written classes missed.
// ---------------------------------------------------------------------------

#[test]
fn randomised_sweep() {
    // xorshift64*, so the test needs no external crate and is reproducible.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };

    let alphabet: &[u8] = b"0123456789 \t\n+-abzX.,\x00\xff";

    for i in 0..300 {
        let len = (next() % 24) as usize;
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            let idx = (next() % alphabet.len() as u64) as usize;
            input.push(alphabet[idx]);
        }
        assert_same(&format!("random_{i}"), &input);
    }

    for i in 0..200 {
        let a = next() % 40;
        let b = next() % 40;
        let c = (next() % 80) as i64 - 40;
        let d = next() as i64;
        let input = format!("{a} {b} {c} {d}");
        assert_same(&format!("random_numeric_{i}"), input.as_bytes());
    }
}
