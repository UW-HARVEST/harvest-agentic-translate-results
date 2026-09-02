//! Differential tests: run the original C program and the Rust translation as
//! *subprocesses* with identical stdin and require byte-identical stdout,
//! byte-identical stderr and the same exit status.
//!
//! The Rust code is never called as a library; the built binary is driven the
//! way a shell would drive it, because that is how the two programs are
//! compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary produced by this crate.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // `translation/` -> repository root.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use if necessary.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }

        std::fs::create_dir_all(&build).expect("failed to create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("cmake is required to build the C reference program");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );
        let compile = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("failed to invoke cmake --build");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );
        assert!(exe.is_file(), "C reference binary missing at {:?}", exe);
        exe
    })
    .as_path()
}

/// Runs `program` with `stdin` piped in, capturing stdout and stderr.
fn run(program: &Path, stdin: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {e}", program));
    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(stdin)
        .or_else(|e| {
            // The program may exit before consuming all input; that is not a
            // test failure, both programs are treated identically.
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                Ok(())
            } else {
                Err(e)
            }
        })
        .expect("failed to write stdin");
    drop(child.stdin.take());
    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {:?}: {e}", program))
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Asserts stdout, stderr and exit status all match for one input.
#[track_caller]
fn assert_same(case: &str, stdin: &[u8]) {
    let c = run(c_bin(), stdin);
    let r = run(&rust_bin(), stdin);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for case `{case}` (stdin = \"{}\")\n  C:    \"{}\"\n  Rust: \"{}\"",
        show(stdin),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for case `{case}` (stdin = \"{}\")\n  C:    \"{}\"\n  Rust: \"{}\"",
        show(stdin),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for case `{case}` (stdin = \"{}\"): C {:?} vs Rust {:?}",
        show(stdin),
        c.status,
        r.status
    );
    assert_eq!(
        c.status.success(),
        r.status.success(),
        "exit success mismatch for case `{case}`"
    );
}

#[track_caller]
fn check_all(cases: &[(&str, &[u8])]) {
    for (name, input) in cases {
        assert_same(name, input);
    }
}

// ---------------------------------------------------------------------------
// Structural input classes.
//
// `main` calls, in order: goodG2B (no read), goodB2G (fgets #1), bad (fgets #2).
// Each `fgets` reads at most CHAR_ARRAY_SIZE - 1 == 19 bytes and stops after a
// newline, so the number of bytes available decides which branches are taken.
// ---------------------------------------------------------------------------

/// No input at all: both `fgets` calls return NULL and take the failure branch.
#[test]
fn empty_input_both_fgets_fail() {
    assert_same("empty", b"");
}

/// Exactly one line: goodB2G consumes it, bad's `fgets` hits EOF.
#[test]
fn single_line_second_fgets_fails() {
    check_all(&[
        ("one_line_2", b"2\n"),
        ("one_line_no_newline", b"2"),
        ("one_line_zero", b"0\n"),
        ("one_line_junk", b"abc\n"),
        ("one_empty_line", b"\n"),
    ]);
}

/// Two or more lines: both `fgets` calls succeed. Extra lines are never read.
#[test]
fn two_or_more_lines_both_fgets_succeed() {
    check_all(&[
        ("two_lines", b"2\n4\n"),
        ("two_lines_no_trailing_newline", b"2\n4"),
        ("three_lines", b"2\n4\n8\n"),
        ("many_lines", b"2\n4\n8\n16\n32\n"),
        ("blank_then_value", b"\n5\n"),
        ("value_then_blank", b"5\n\n"),
        ("only_newlines", b"\n\n\n\n\n"),
    ]);
}

/// The maximum a single `fgets` handles is 19 bytes; longer lines are split
/// across the two calls rather than read across the newline.
#[test]
fn line_longer_than_buffer_is_split_across_calls() {
    check_all(&[
        // 18 payload bytes + newline: fits with room to spare.
        ("len_18", b"123456789012345678\n"),
        // 19 payload bytes + newline: fills the buffer exactly, so the newline
        // is left behind for the second fgets.
        ("len_19_exact", b"1234567890123456789\n"),
        // 20 payload bytes: the tail "0\n" becomes bad()'s input.
        ("len_20", b"12345678901234567890\n"),
        ("len_22", b"1234567890123456789012\n"),
        ("len_40", b"1111111111111111111111111111111111111111\n"),
        // 19 bytes, no newline, then EOF: second fgets fails.
        ("len_19_no_newline", b"1234567890123456789"),
        ("nineteen_spaces", b"                   \n"),
        ("twenty_spaces", b"                    \n"),
        ("split_makes_number", b"0000000000000000002more\n"),
        ("split_dashes", b"-------------------5\n"),
    ]);
}

/// Command-line arguments are ignored by `main`.
#[test]
fn argv_is_ignored() {
    let stdin = b"2\n4\n";
    let c = Command::new(c_bin())
        .args(["alpha", "beta"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut ch| {
            ch.stdin.as_mut().unwrap().write_all(stdin)?;
            drop(ch.stdin.take());
            ch.wait_with_output()
        })
        .expect("C run with argv failed");
    let r = Command::new(rust_bin())
        .args(["alpha", "beta"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut ch| {
            ch.stdin.as_mut().unwrap().write_all(stdin)?;
            drop(ch.stdin.take());
            ch.wait_with_output()
        })
        .expect("Rust run with argv failed");
    assert_eq!(c.stdout, r.stdout, "stdout mismatch with argv");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with argv");
    assert_eq!(c.status.code(), r.status.code(), "status mismatch with argv");
}

// ---------------------------------------------------------------------------
// goodB2G's `fabs(data) > 0.000001` guard.
// ---------------------------------------------------------------------------

/// Values on the "print the quotient" side of the guard.
#[test]
fn b2g_guard_above_threshold() {
    check_all(&[
        ("two", b"2\n"),
        ("one", b"1\n"),
        ("hundred", b"100\n"),
        ("negative", b"-4\n"),
        ("fraction", b"0.3\n"),
        ("just_above", b"0.0000010000001\n"),
        ("just_above_neg", b"-0.0000010000001\n"),
        ("above_2e-6", b"0.000002\n"),
        ("huge", b"1e30\n"),
        ("inf", b"inf\n"),
        ("neg_inf", b"-inf\n"),
    ]);
}

/// Values on the "divide by zero" side of the guard, including the cases where
/// the comparison is false because the value is NaN.
#[test]
fn b2g_guard_at_or_below_threshold() {
    check_all(&[
        ("zero", b"0\n"),
        ("zero_point_zero", b"0.0\n"),
        ("negative_zero", b"-0\n"),
        ("negative_zero_float", b"-0.0\n"),
        ("exactly_1e-6", b"0.000001\n"),
        ("neg_exactly_1e-6", b"-0.000001\n"),
        ("just_below", b"0.0000009999999\n"),
        ("half_threshold", b"0.0000005\n"),
        ("1e-7", b"1e-7\n"),
        ("1e-30", b"1e-30\n"),
        ("neg_1e-30", b"-1e-30\n"),
        ("subnormal", b"1e-320\n"),
        ("underflow", b"1e-400\n"),
        ("nan", b"nan\n"),
        ("neg_nan", b"-nan\n"),
        ("nan_parens", b"nan(1)\n"),
        ("no_conversion", b"abc\n"),
        ("empty_line_is_zero", b"\n"),
    ]);
}

// ---------------------------------------------------------------------------
// bad(): the unguarded division, including the divide-by-zero and the
// out-of-range `(int)` conversion the C performs.
// ---------------------------------------------------------------------------

/// `data == 0` (or NaN) in bad(): `100.0 / data` is infinite/NaN and the
/// `(int)` conversion is out of range.
#[test]
fn bad_divide_by_zero_and_out_of_range_conversion() {
    check_all(&[
        ("second_line_zero", b"2\n0\n"),
        ("second_line_neg_zero", b"2\n-0\n"),
        ("second_line_junk", b"2\nabc\n"),
        ("second_line_blank", b"2\n\n"),
        ("second_line_nan", b"2\nnan\n"),
        ("second_fgets_eof", b"2\n"),
        // 100/data overflows int without being infinite.
        ("overflow_positive", b"2\n1e-30\n"),
        ("overflow_negative", b"2\n-1e-30\n"),
        ("overflow_subnormal", b"2\n1e-320\n"),
    ]);
}

/// Quotients that land inside `int` range, exercising truncation toward zero
/// in both directions and the boundary at 2^31.
#[test]
fn bad_in_range_and_boundary_conversions() {
    check_all(&[
        ("truncate_positive", b"2\n3\n"),
        ("truncate_negative", b"2\n-3\n"),
        ("truncate_to_zero", b"2\n1e30\n"),
        ("truncate_to_zero_neg", b"2\n-1e30\n"),
        ("exact", b"2\n4\n"),
        ("one", b"2\n1\n"),
        ("gt_int_max", b"2\n0.00000004656612\n"),
        ("near_int_max", b"2\n0.0000000465661\n"),
        ("near_int_min", b"2\n-0.0000000465661\n"),
        ("boundary_pos", b"2\n4.656612873e-8\n"),
        ("boundary_neg", b"2\n-4.656612873e-8\n"),
    ]);
}

// ---------------------------------------------------------------------------
// atof / strtod subject-sequence classes.
// ---------------------------------------------------------------------------

#[test]
fn atof_accepts_signs_whitespace_and_forms() {
    check_all(&[
        ("plus", b"+3\n+3\n"),
        ("minus", b"-3\n-3\n"),
        ("leading_spaces", b"   7\n   7\n"),
        ("leading_tab", b"\t9\n\t9\n"),
        ("all_c_spaces", b" \t\x0b\x0c\r5\n \t\x0b\x0c\r5\n"),
        ("trailing_junk", b"5abc\n5abc\n"),
        ("leading_dot", b".5\n.5\n"),
        ("trailing_dot", b"1.\n1.\n"),
        ("dot_then_exp", b"1.e2\n1.e2\n"),
        ("exponent_plus", b"5e+2\n5e+2\n"),
        ("exponent_minus", b"5e-2\n5e-2\n"),
        ("exponent_caps", b"5E2\n5E2\n"),
        ("leading_zeros", b"007\n007\n"),
        ("crlf", b"2\r\n4\r\n"),
    ]);
}

#[test]
fn atof_rejects_incomplete_subject_sequences() {
    check_all(&[
        ("sign_only", b"-\n-\n"),
        ("plus_only", b"+\n+\n"),
        ("dot_only", b".\n.\n"),
        ("double_sign", b"--1\n--1\n"),
        ("sign_space_digit", b"- 1\n- 1\n"),
        ("exponent_no_digits", b"5e\n5e\n"),
        ("exponent_sign_no_digits", b"5e+\n5e+\n"),
        ("e_alone", b"e5\n e5\n"),
        ("dot_e_only", b".e5\n.e5\n"),
        ("word_inf_prefix", b"in\nin\n"),
        ("word_nan_prefix", b"na\nna\n"),
        ("letters", b"xyz\nxyz\n"),
    ]);
}

#[test]
fn atof_infinity_and_nan_spellings() {
    check_all(&[
        ("inf_lower", b"inf\ninf\n"),
        ("inf_upper", b"INF\nINF\n"),
        ("infinity", b"infinity\ninfinity\n"),
        ("infinity_upper", b"INFINITY\nINFINITY\n"),
        ("inf_trailing", b"inf1\ninf1\n"),
        ("neg_infinity", b"-infinity\n-infinity\n"),
        ("nan_lower", b"nan\nnan\n"),
        ("nan_mixed", b"NaN\nNaN\n"),
        ("nan_empty_parens", b"nan()\nnan()\n"),
        ("nan_chars", b"nan(abc)\nnan(abc)\n"),
        ("nan_unterminated", b"nan(\nnan(\n"),
    ]);
}

#[test]
fn atof_hexadecimal_forms() {
    check_all(&[
        ("hex_int", b"0x10\n0x10\n"),
        ("hex_upper_x", b"0X1A\n0X1A\n"),
        ("hex_no_digits", b"0x\n0x\n"),
        ("hex_dot_only", b"0x.\n0x.\n"),
        ("hex_letters_after", b"0xyz\n0xyz\n"),
        ("hex_fraction", b"0x.8\n0x.8\n"),
        ("hex_p_positive", b"0x1p4\n0x1p4\n"),
        ("hex_p_negative", b"0x1p-4\n0x1p-4\n"),
        ("hex_p_caps", b"0X1.8P3\n0X1.8P3\n"),
        ("hex_p_no_digits", b"0x1.8p\n0x1.8p\n"),
        ("hex_p_sign_only", b"0x1.8p+\n0x1.8p+\n"),
        ("hex_double_dot", b"0x1..8\n0x1..8\n"),
        ("hex_min_subnormal", b"0x1p-1074\n0x1p-1074\n"),
        ("hex_below_subnormal", b"0x1p-1075\n0x1p-1075\n"),
        ("hex_overflow", b"0x1p1024\n0x1p1024\n"),
        ("hex_long_mantissa", b"0xabcdef0123456789\n"),
        ("hex_round_to_even", b"0x1.fffffffffffffp0\n"),
        ("hex_negative", b"-0x1p3\n-0x1p3\n"),
        ("hex_huge_exponent", b"0x1p99999999999\n"),
    ]);
}

#[test]
fn atof_float_rounding_boundaries() {
    // `data` is a `float`, so the double from strtod is narrowed; these values
    // sit at or near f32 rounding boundaries.
    check_all(&[
        ("f32_max", b"3.4028235e38\n3.4028235e38\n"),
        ("f32_overflow", b"3.4028236e38\n3.4028236e38\n"),
        ("f32_min_subnormal", b"1.4012984e-45\n1.4012984e-45\n"),
        ("f32_half_subnormal", b"7.006492e-46\n7.006492e-46\n"),
        ("f32_min_normal", b"1.1754944e-38\n1.1754944e-38\n"),
        ("f64_max", b"1.7976931348e308\n1.7976931348e308\n"),
        ("f64_overflow", b"1e400\n1e400\n"),
        ("f64_min_subnormal", b"5e-324\n5e-324\n"),
        ("f64_half_subnormal", b"2e-324\n2e-324\n"),
        ("pi", b"3.14159265358979\n3.14159265358979\n"),
        ("near_one_below", b"0.9999999999999999\n"),
        ("integer_16777217", b"16777217\n16777217\n"),
        ("long_digits", b"123456789012345678\n"),
    ]);
}

// ---------------------------------------------------------------------------
// Non-textual and hostile input: `fgets` is byte-oriented and `atof` stops at
// the first NUL, so neither program requires valid UTF-8.
// ---------------------------------------------------------------------------

#[test]
fn binary_and_non_utf8_input() {
    check_all(&[
        ("leading_nul", b"\x005\n7\n"),
        ("embedded_nul", b"3\x004\n5\n"),
        ("nul_line", b"\x00\n\x00\n"),
        ("nul_only_no_newline", b"\x00"),
        ("high_bytes", b"\xff\xfe\n\xff\xfe\n"),
        ("invalid_utf8_between_digits", b"1\xff2\n1\xff2\n"),
        ("utf8_multibyte", b"\xc3\xa9\n\xc3\xa9\n"),
        (
            "control_bytes",
            b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\n",
        ),
        ("carriage_returns_only", b"\r\r"),
        ("space_then_eof", b" "),
    ]);
}

/// Every byte value on its own line, so no single byte class is left untried.
#[test]
fn each_single_byte_line() {
    for byte in 0u8..=255 {
        let input = [byte, b'\n', byte, b'\n'];
        assert_same(&format!("single_byte_{byte:#04x}"), &input);
    }
}

/// A stdout that cannot be written to: C's `printf` discards the error and
/// still exits 0, so the translation must not panic either.
#[test]
fn unwritable_stdout_does_not_change_exit_status() {
    let full = Path::new("/dev/full");
    if !full.exists() {
        // Nothing to compare against on platforms without /dev/full; the
        // remaining tests still cover every documented input class.
        return;
    }

    let mut statuses = Vec::new();
    let mut errs = Vec::new();
    for program in [c_bin().to_path_buf(), rust_bin()] {
        let sink = std::fs::OpenOptions::new()
            .write(true)
            .open(full)
            .expect("failed to open /dev/full");
        let mut child = Command::new(&program)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(sink))
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn with /dev/full as stdout");
        let _ = child.stdin.as_mut().unwrap().write_all(b"2\n4\n");
        drop(child.stdin.take());
        let out = child.wait_with_output().expect("wait failed");
        statuses.push(out.status.code());
        errs.push(out.stderr);
    }
    assert_eq!(
        statuses[0], statuses[1],
        "exit status mismatch when stdout is unwritable: C {:?} vs Rust {:?}",
        statuses[0], statuses[1]
    );
    assert_eq!(
        errs[0],
        errs[1],
        "stderr mismatch when stdout is unwritable:\n  C:    \"{}\"\n  Rust: \"{}\"",
        show(&errs[0]),
        show(&errs[1])
    );
}

/// stdin that fails to read (a directory) rather than reaching a clean EOF:
/// `fgets` returns NULL in both programs.
#[test]
fn unreadable_stdin_takes_the_fgets_failure_branch() {
    let dir = workspace_root();
    let mut outputs = Vec::new();
    for program in [c_bin().to_path_buf(), rust_bin()] {
        let source = std::fs::File::open(&dir).expect("failed to open directory as stdin");
        let out = Command::new(&program)
            .stdin(Stdio::from(source))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run with a directory as stdin");
        outputs.push(out);
    }
    assert_eq!(
        outputs[0].stdout,
        outputs[1].stdout,
        "stdout mismatch with unreadable stdin:\n  C:    \"{}\"\n  Rust: \"{}\"",
        show(&outputs[0].stdout),
        show(&outputs[1].stdout)
    );
    assert_eq!(outputs[0].stderr, outputs[1].stderr, "stderr mismatch");
    assert_eq!(
        outputs[0].status.code(),
        outputs[1].status.code(),
        "exit status mismatch"
    );
}
