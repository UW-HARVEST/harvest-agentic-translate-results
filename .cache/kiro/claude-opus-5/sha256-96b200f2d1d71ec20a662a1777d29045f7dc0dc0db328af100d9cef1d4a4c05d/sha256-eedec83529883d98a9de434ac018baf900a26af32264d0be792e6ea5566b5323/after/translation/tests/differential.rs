//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! Nothing here links the Rust code as a library; both programs are driven
//! exactly the way a shell drives them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Path to the Rust binary under test, provided by Cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Workspace root: the directory that holds both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C binary, building it with CMake if it is missing.
///
/// `c_src/` itself is never modified; only the out-of-source `c_src/build`
/// directory is created, which is what `CMakeLists.txt` expects.
fn c_bin() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build_dir = c_src.join("build");
    let exe = build_dir.join("driver");
    if exe.exists() {
        return exe;
    }

    std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");

    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build_dir)
        .output()
        .expect("failed to run `cmake` - is CMake installed?");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&configure.stdout),
        String::from_utf8_lossy(&configure.stderr)
    );

    let build = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build_dir)
        .output()
        .expect("failed to run `cmake --build .`");
    assert!(
        build.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    assert!(
        exe.exists(),
        "C binary still missing after build: {}",
        exe.display()
    );
    exe
}

/// Run `program` with `input` on stdin and capture everything it produced.
fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        // The programs may exit without draining stdin; a broken pipe here is
        // not a test failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", program.display()))
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
fn assert_same(name: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for case `{name}` (input {:?})\n  C   : {}\n  Rust: {}",
        show(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for case `{name}` (input {:?})\n  C   : {}\n  Rust: {}",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for case `{name}` (input {:?}): C {:?} vs Rust {:?}",
        show(input),
        c.status,
        r.status
    );
}

fn assert_all(cases: &[(&str, Vec<u8>)]) {
    for (name, input) in cases {
        assert_same(name, input);
    }
}

fn repeat(byte: u8, n: usize) -> Vec<u8> {
    vec![byte; n]
}

// ---------------------------------------------------------------------------
// Phase A - both programs exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = run(&c_bin(), b"1\n");
    let r = run(&rust_bin(), b"1\n");
    assert!(c.status.success(), "C binary did not exit successfully");
    assert!(r.status.success(), "Rust binary did not exit successfully");
    assert!(!c.stdout.is_empty(), "C binary produced no stdout");
    assert!(!r.stdout.is_empty(), "Rust binary produced no stdout");
}

/// The C program always returns 0 from `main`, on both the success and the
/// error path, and never writes to stderr.
#[test]
fn exit_status_is_zero_and_stderr_empty_on_both_paths() {
    for input in [&b"5\n"[..], &b"oops\n"[..], &b""[..]] {
        for bin in [c_bin(), rust_bin()] {
            let out = run(&bin, input);
            assert_eq!(
                out.status.code(),
                Some(0),
                "{} exited non-zero on {:?}",
                bin.display(),
                show(input)
            );
            assert!(
                out.stderr.is_empty(),
                "{} wrote to stderr on {:?}: {}",
                bin.display(),
                show(input),
                show(&out.stderr)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Phase B - the branches the C source actually takes.
// ---------------------------------------------------------------------------

/// `fgets` returns NULL on immediate EOF and leaves `in` as the empty string,
/// so `parse_val` fails and the error path runs.
#[test]
fn empty_input_takes_the_error_path() {
    assert_same("empty stdin (immediate EOF)", b"");
}

/// A single well-formed item: the ordinary success path through `run` twice.
#[test]
fn single_valid_value() {
    assert_all(&[
        ("zero", b"0\n".to_vec()),
        ("one", b"1\n".to_vec()),
        ("small positive", b"7\n".to_vec()),
        ("small negative", b"-3\n".to_vec()),
        ("explicit plus sign", b"+42\n".to_vec()),
        ("leading zeros", b"007\n".to_vec()),
        ("no trailing newline", b"5".to_vec()),
    ]);
}

/// `run` mutates the same `house_t` and is called twice, so state carries over
/// between calls. This pins the full eight-line transcript.
#[test]
fn state_carries_between_the_two_run_calls() {
    let out = run(&rust_bin(), b"10\n");
    let expected = "\
The house has 2 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 3.5 bathrooms
The house has 3 floors, 15 bedrooms, and 3.5 bathrooms
The house has 3 floors, 15 bedrooms, and 3.5 bathrooms
The house has 4 floors, 15 bedrooms, and 3.5 bathrooms
The house has 4 floors, 15 bedrooms, and 4.5 bathrooms
The house has 4 floors, 25 bedrooms, and 4.5 bathrooms
";
    assert_eq!(String::from_utf8_lossy(&out.stdout), expected);
    // And the C program agrees, byte for byte.
    assert_same("accumulating state across both run calls", b"10\n");
}

/// `strtol` stops at the first non-digit; `endp != str` is still true, so a
/// partially numeric string is accepted with the prefix value.
#[test]
fn trailing_junk_is_accepted_using_the_parsed_prefix() {
    assert_all(&[
        ("digits then letters", b"42abc\n".to_vec()),
        ("base-10 sees 0 in 0x10", b"0x10\n".to_vec()),
        ("decimal point truncates", b"3.9\n".to_vec()),
        ("digits then space", b"12 34\n".to_vec()),
        ("digits then comma", b"8,9\n".to_vec()),
    ]);
}

/// `strtol` skips leading C whitespace before the sign and digits.
#[test]
fn leading_whitespace_is_skipped() {
    assert_all(&[
        ("spaces then digits", b"   42\n".to_vec()),
        ("tab then digits", b"\t8\n".to_vec()),
        ("carriage return then digits", b"\r12\n".to_vec()),
        (
            "every C whitespace class",
            b" \t\x0b\x0c\r 12\n".to_vec(),
        ),
    ]);
}

/// `endp == str`: no conversion happened, so `parse_val` returns false.
#[test]
fn no_conversion_takes_the_error_path() {
    assert_all(&[
        ("newline only", b"\n".to_vec()),
        ("letters", b"abc\n".to_vec()),
        ("plus sign alone", b"+\n".to_vec()),
        ("minus sign alone", b"-\n".to_vec()),
        ("sign separated from digits", b"- 5\n".to_vec()),
        ("whitespace only", b"    \n".to_vec()),
        ("leading decimal point", b".5\n".to_vec()),
        ("punctuation", b"!!!\n".to_vec()),
        ("empty line before digits", b"\n5\n".to_vec()),
    ]);
}

/// `tmp >= INT_MIN && tmp <= INT_MAX` fails: in range for `long`, out of range
/// for `int`, so the error path runs even though `strtol` succeeded.
#[test]
fn value_outside_int_range_takes_the_error_path() {
    assert_all(&[
        ("INT_MAX + 1", b"2147483648\n".to_vec()),
        ("INT_MIN - 1", b"-2147483649\n".to_vec()),
        ("ten billion", b"10000000000\n".to_vec()),
        ("LONG_MAX", b"9223372036854775807\n".to_vec()),
        ("LONG_MIN", b"-9223372036854775808\n".to_vec()),
    ]);
}

/// `errno != 0` (ERANGE) after `strtol`: outside `long` range entirely.
#[test]
fn strtol_erange_takes_the_error_path() {
    assert_all(&[
        ("LONG_MAX + 1", b"9223372036854775808\n".to_vec()),
        ("LONG_MIN - 1", b"-9223372036854775809\n".to_vec()),
        ("far above LONG_MAX", b"99999999999999999999\n".to_vec()),
        ("far below LONG_MIN", b"-99999999999999999999\n".to_vec()),
        ("many nines", {
            let mut v = repeat(b'9', 50);
            v.push(b'\n');
            v
        }),
    ]);
}

/// The extremes that are still valid `int`s, where `5 + x` and `5 + 2x`
/// overflow the way the C compiler performs them.
#[test]
fn int_range_extremes_overflow_identically() {
    assert_all(&[
        ("INT_MAX", b"2147483647\n".to_vec()),
        ("INT_MAX - 4", b"2147483643\n".to_vec()),
        ("INT_MIN", b"-2147483648\n".to_vec()),
        ("INT_MIN + 5", b"-2147483643\n".to_vec()),
        ("2^30", b"1073741824\n".to_vec()),
        ("-2^30", b"-1073741824\n".to_vec()),
    ]);
}

/// `fgets` stops at the first newline, so a second line is never read.
#[test]
fn fgets_does_not_read_past_the_first_line() {
    assert_all(&[
        ("second line ignored", b"7\n9\n".to_vec()),
        ("second line would be invalid", b"7\nabc\n".to_vec()),
        ("first line invalid, second valid", b"abc\n7\n".to_vec()),
        ("many following lines", b"1\n2\n3\n4\n5\n".to_vec()),
        (
            "value split across lines (scanf would join)",
            b"1\n2\n".to_vec(),
        ),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C - the remaining input classes.
// ---------------------------------------------------------------------------

/// `char in[100]` with `fgets(in, sizeof(in), stdin)` keeps at most 99 bytes.
/// These cases straddle that cut so the parsed value differs from the value in
/// the full input.
#[test]
fn fgets_truncates_at_ninety_nine_bytes() {
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();

    // 98 spaces + "12\n" -> only "...1" survives, so x == 1, not 12.
    let mut split_number = repeat(b' ', 98);
    split_number.extend_from_slice(b"12\n");
    cases.push(("98 spaces then 12, cut inside the number", split_number));

    // 96 spaces + "42\n" is exactly 99 bytes: the newline still fits.
    let mut exactly_99 = repeat(b' ', 96);
    exactly_99.extend_from_slice(b"42\n");
    cases.push(("exactly 99 bytes including the newline", exactly_99));

    // 90 spaces + 9 digits = 99 bytes: whole number survives.
    let mut fits = repeat(b' ', 90);
    fits.extend_from_slice(b"123456789\n");
    cases.push(("nine digits ending exactly at the limit", fits));

    // 91 spaces + 9 digits = 100 bytes: the last digit is dropped.
    let mut one_over = repeat(b' ', 91);
    one_over.extend_from_slice(b"123456789\n");
    cases.push(("one byte over the limit drops a digit", one_over));

    // 99 digits: truncated to 99 digits, still ERANGE.
    let mut long_digits = repeat(b'1', 99);
    long_digits.push(b'\n');
    cases.push(("99 digits", long_digits));

    // 99 spaces then a digit: the digit is cut, leaving whitespace only.
    let mut spaces_then_digit = repeat(b' ', 99);
    spaces_then_digit.extend_from_slice(b"5\n");
    cases.push(("99 spaces then a digit, digit cut", spaces_then_digit));

    // Far longer than the buffer, no newline at all.
    let mut very_long = repeat(b'7', 500);
    cases.push(("500 sevens, no newline", std::mem::take(&mut very_long)));

    // Far longer than the buffer with many newlines after the cut.
    let mut long_multiline = repeat(b'3', 250);
    long_multiline.extend_from_slice(b"\n42\n");
    cases.push(("250 threes then more lines", long_multiline));

    assert_all(&cases);
}

/// `in` is used as a NUL-terminated string, so a NUL byte hides the rest of
/// the line from `strtol` even though `fgets` read it.
#[test]
fn embedded_nul_bytes_terminate_the_string() {
    assert_all(&[
        ("leading NUL then digits", b"\x005\n".to_vec()),
        ("digit then NUL then digits", b"5\x006\n".to_vec()),
        ("digit then NUL then letters", b"5\x00abc\n".to_vec()),
        ("NUL only", b"\x00\n".to_vec()),
        ("spaces then NUL", b"  \x0042\n".to_vec()),
        ("NUL immediately, no newline", b"\x00".to_vec()),
    ]);
}

/// The C code reads bytes, not text, so non-UTF-8 input must behave the same.
#[test]
fn non_utf8_input_is_handled_as_bytes() {
    assert_all(&[
        ("lone 0xff", b"\xff\n".to_vec()),
        ("0xff 0xfe", b"\xff\xfe\n".to_vec()),
        ("digits then 0xff", b"5\xff\n".to_vec()),
        ("0xff then digits", b"\xff5\n".to_vec()),
        ("truncated UTF-8 sequence", b"\xe2\x82\n".to_vec()),
        ("high bytes then valid number", b"\x80\x81 12\n".to_vec()),
    ]);
}

/// No newline anywhere: `fgets` reads to EOF and still NUL-terminates.
#[test]
fn input_without_any_newline() {
    assert_all(&[
        ("single digit, no newline", b"9".to_vec()),
        ("negative, no newline", b"-9".to_vec()),
        ("letters, no newline", b"zz".to_vec()),
        ("single space", b" ".to_vec()),
        ("single NUL", b"\x00".to_vec()),
        ("sign only, no newline", b"-".to_vec()),
    ]);
}

/// Every digit boundary where the `%d` field width or the sign changes.
#[test]
fn formatting_of_bedroom_counts() {
    assert_all(&[
        ("x = -5 makes bedrooms 0", b"-5\n".to_vec()),
        ("x = -10 makes bedrooms negative", b"-10\n".to_vec()),
        ("x makes bedrooms exactly INT_MAX", b"2147483642\n".to_vec()),
        ("large positive", b"1000000\n".to_vec()),
        ("large negative", b"-1000000\n".to_vec()),
    ]);
}

/// A broad sweep so no single-value case slips through unchecked.
#[test]
fn sweep_of_representative_values() {
    let mut cases: Vec<(&'static str, Vec<u8>)> = Vec::new();
    for v in [
        -2147483648i64,
        -2147483647,
        -100000,
        -128,
        -2,
        -1,
        0,
        1,
        2,
        127,
        128,
        255,
        256,
        32767,
        32768,
        65535,
        65536,
        1000000,
        2147483646,
        2147483647,
    ] {
        cases.push(("swept value", format!("{v}\n").into_bytes()));
    }
    assert_all(&cases);
}
