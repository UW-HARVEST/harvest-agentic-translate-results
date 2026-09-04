// Differential integration tests: run the C reference binary and the Rust
// binary as subprocesses on identical stdin and compare stdout, stderr and
// exit status byte for byte.
//
// Nothing here loads the Rust code as a library -- the program is graded by
// running it, so that is how it is tested.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// The Rust binary under test, as built by cargo for this test run.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C reference binary, built on demand (once per test process).
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(build_c_reference).as_path()
}

fn build_c_reference() -> PathBuf {
    let c_src = manifest_dir().join("../c_src");
    let main_c = c_src.join("src/main.c");
    assert!(
        main_c.is_file(),
        "C source not found at {}",
        main_c.display()
    );

    // Prefer a binary that was already built in-tree.
    let prebuilt = c_src.join("build/driver");
    if prebuilt.is_file() {
        return prebuilt;
    }

    // Otherwise build out-of-tree so that c_src/ is left untouched.
    let build_dir = manifest_dir().join("target/c_reference");
    std::fs::create_dir_all(&build_dir).expect("create C build dir");

    let configured = Command::new("cmake")
        .arg("-S")
        .arg(&c_src)
        .arg("-B")
        .arg(&build_dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if configured {
        let built = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        let candidate = build_dir.join("driver");
        if built && candidate.is_file() {
            return candidate;
        }
    }

    // Last resort: invoke the C compiler directly.
    let candidate = build_dir.join("driver");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let out = Command::new(&cc)
        .arg("-O2")
        .arg("-o")
        .arg(&candidate)
        .arg(&main_c)
        .arg("-lm")
        .output()
        .expect("failed to run the C compiler");
    assert!(
        out.status.success(),
        "compiling {} failed: {}",
        main_c.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    candidate
}

/// `(exit code, terminating signal)` -- both halves matter, because a program
/// killed by `SIGPIPE` has no exit code at all.
fn status_of(output: &Output) -> (Option<i32>, Option<i32>) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        (output.status.code(), output.status.signal())
    }
    #[cfg(not(unix))]
    {
        (output.status.code(), None)
    }
}

fn run(bin: &Path, args: &[&str], stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(stdin_bytes)
        // A short-lived program could conceivably exit before we finish
        // writing; that is not a test failure by itself.
        .unwrap_or(());
    drop(child.stdin.take());
    child.wait_with_output().expect("collect child output")
}

/// Asserts the C and Rust binaries agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same_with_args(name: &str, args: &[&str], stdin_bytes: &[u8]) {
    let c = run(c_bin(), args, stdin_bytes);
    let r = run(rust_bin(), args, stdin_bytes);

    let describe = |o: &Output| {
        let (code, signal) = status_of(o);
        format!(
            "code={code:?} signal={signal:?}\n  stdout={:?}\n  stderr={:?}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
    };

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{name}] stdout differs for input {stdin_bytes:?}\n  C:    {}\n  Rust: {}",
        describe(&c),
        describe(&r)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{name}] stderr differs for input {stdin_bytes:?}\n  C:    {}\n  Rust: {}",
        describe(&c),
        describe(&r)
    );
    assert_eq!(
        status_of(&c),
        status_of(&r),
        "[{name}] exit status differs for input {stdin_bytes:?}\n  C:    {}\n  Rust: {}",
        describe(&c),
        describe(&r)
    );
}

#[track_caller]
fn assert_same(name: &str, stdin_bytes: &[u8]) {
    assert_same_with_args(name, &[], stdin_bytes);
}

#[track_caller]
fn assert_same_all(cases: &[(&str, &[u8])]) {
    for (name, input) in cases {
        assert_same(name, input);
    }
}

// ---------------------------------------------------------------------------
// fgets(): the NULL / non-NULL branches in both goodB2G() and bad()
// ---------------------------------------------------------------------------

#[test]
fn empty_stdin_fails_both_fgets_calls() {
    // Both fgets() calls return NULL: "fgets() failed." twice, data stays 0.0F.
    assert_same("empty", b"");
}

#[test]
fn single_line_leaves_bad_at_eof() {
    // goodB2G() consumes the only line; bad()'s fgets() then returns NULL.
    assert_same_all(&[
        ("one_line_two", b"2.0\n"),
        ("one_line_zero", b"0\n"),
        ("one_line_no_newline", b"7"),
        ("one_line_bare_newline", b"\n"),
    ]);
}

#[test]
fn two_lines_feed_both_readers() {
    assert_same_all(&[
        ("two_lines", b"2.0\n5.0\n"),
        ("two_lines_zero", b"0\n0\n"),
        ("two_lines_mixed", b"0\n4\n"),
        ("second_line_no_newline", b"4\n5"),
    ]);
}

#[test]
fn fgets_does_not_read_across_newlines() {
    // With fgets(), goodB2G() gets "1\n" and bad() gets "2\n"; the remaining
    // lines are never read. scanf()-style reading would give other numbers.
    assert_same_all(&[
        ("many_lines", b"1\n2\n3\n4\n5\n"),
        ("blank_then_number", b"\n5\n"),
        ("number_then_blank", b"5\n\n"),
        ("two_blank_lines", b"\n\n"),
    ]);
}

// ---------------------------------------------------------------------------
// CHAR_ARRAY_SIZE: at most 19 bytes are stored per fgets() call
// ---------------------------------------------------------------------------

#[test]
fn buffer_capacity_boundaries() {
    assert_same_all(&[
        // 18 data bytes + newline: fits exactly.
        ("len18_plus_newline", b"12345678901234567\n12345678901234567\n"),
        // 19 data bytes: fills the buffer, newline is left for the next call.
        ("len19", b"1234567890123456789\n1234567890123456789\n"),
        ("len20", b"12345678901234567890\n"),
        // A single long line is picked up piecewise by the two callers.
        ("len25_split", b"1234567890123456789012345\n"),
        ("long_decimal_split", b"0.5000000000000000000000001\n"),
        ("no_newline_long", b"123456789012345678901234"),
    ]);
}

#[test]
fn very_long_single_line() {
    let mut input = vec![b'9'; 5000];
    input.push(b'\n');
    assert_same("huge_line", &input);
}

// ---------------------------------------------------------------------------
// goodB2G(): fabs(data) > 0.000001 -- both sides of the guard
// ---------------------------------------------------------------------------

#[test]
fn divide_by_zero_guard_boundaries() {
    assert_same_all(&[
        // Exactly the threshold: not greater, so the guard rejects it.
        ("eps_exact", b"0.000001\n0.000001\n"),
        ("eps_below", b"0.0000009\n0.0000009\n"),
        ("eps_above", b"0.0000011\n0.0000011\n"),
        ("neg_eps_below", b"-0.0000009\n-0.0000009\n"),
        ("neg_eps_above", b"-0.0000011\n-0.0000011\n"),
        ("negative_zero", b"-0.0\n-0.0\n"),
        ("positive_zero", b"0.0\n0.0\n"),
        // fabs(NaN) > eps is false, so NaN takes the "divide by zero" branch.
        ("nan", b"nan\nnan\n"),
        ("nan_upper", b"NAN\nNAN\n"),
        ("nan_parens", b"nan(0x1)\nnan(0x1)\n"),
        ("neg_nan", b"-nan\n-nan\n"),
    ]);
}

// ---------------------------------------------------------------------------
// atof(): what strtod() accepts, and what it rejects (result 0.0)
// ---------------------------------------------------------------------------

#[test]
fn atof_rejected_inputs_yield_zero() {
    assert_same_all(&[
        ("junk", b"abc\nabc\n"),
        ("dot_only", b".\n.\n"),
        ("minus_only", b"-\n-\n"),
        ("plus_only", b"+\n+\n"),
        ("exponent_only", b"e5\ne5\n"),
        ("hex_prefix_only", b"0x\n0x\n"),
        ("hex_dot_only", b"0x.\n0x.\n"),
        ("nul_first", b"\x005\n\x005\n"),
        ("high_bytes", b"\xff\xfe\n\xff\xfe\n"),
        ("utf8_bytes", b"\xc3\xa92\n\xc3\xa92\n"),
    ]);
}

#[test]
fn atof_accepted_prefixes() {
    assert_same_all(&[
        ("leading_spaces", b"   12   \n   12   \n"),
        ("leading_tab", b"\t7\n\t7\n"),
        ("plus_sign", b"+5\n+5\n"),
        ("negative", b"-4\n-4\n"),
        ("trailing_junk", b"12abc\n12abc\n"),
        ("leading_dot", b".5\n.5\n"),
        ("trailing_dot", b"5.\n5.\n"),
        // Incomplete exponents stop the conversion at the significand.
        ("exponent_no_digits", b"5e\n5e\n"),
        ("exponent_sign_only", b"5e+\n5e+\n"),
        ("exponent_upper", b"5E1\n5E1\n"),
        ("d_suffix", b"1d5\n1d5\n"),
        ("crlf", b"4\r\n4\r\n"),
        // A NUL terminates the C string mid-buffer.
        ("nul_midway", b"1\x002\n1\x002\n"),
    ]);
}

#[test]
fn atof_hex_forms() {
    assert_same_all(&[
        ("hex_int", b"0x10\n0x10\n"),
        ("hex_upper", b"0X10\n0X10\n"),
        ("hex_fraction", b"0x1.8p3\n0x1.8p3\n"),
        ("hex_leading_dot", b"0x.8\n0x.8\n"),
        ("hex_two_dots", b"0x1.2.3\n0x1.2.3\n"),
        ("hex_p_no_digits", b"0x1p\n0x1p\n"),
        ("hex_p_sign_only", b"0x1p+\n0x1p+\n"),
        ("hex_tiny", b"0x1p-200\n0x1p-200\n"),
        ("hex_subnormal", b"0x1p-1074\n0x1p-1074\n"),
        ("hex_underflow", b"0x1p-1075\n0x1p-1075\n"),
        ("hex_overflow", b"0x1p1024\n0x1p1024\n"),
        ("hex_exp_saturating", b"0x1p2147483648\n"),
        ("hex_many_digits", b"0x123456789abcdef\n"),
    ]);
}

#[test]
fn atof_infinities_and_extremes() {
    assert_same_all(&[
        ("inf", b"inf\ninf\n"),
        ("infinity_word", b"infinity\ninfinity\n"),
        ("neg_inf", b"-inf\n-inf\n"),
        ("inf_upper", b"INF\nINF\n"),
        ("overflow_to_inf", b"1e999\n1e999\n"),
        ("underflow_to_zero", b"1e-999\n1e-999\n"),
        ("exp_saturating", b"1e2147483648\n"),
        ("float_overflow", b"1e40\n1e40\n"),
        // (float) of these underflows to 0.0F or a subnormal.
        ("float_underflow", b"1e-40\n1e-40\n"),
        ("float_underflow_2", b"1e-45\n1e-45\n"),
        ("double_tiny", b"1e-300\n1e-300\n"),
        ("neg_float_underflow", b"-1e-40\n-1e-40\n"),
    ]);
}

// ---------------------------------------------------------------------------
// (int)(100.0 / data): C's out-of-range / NaN conversions
// ---------------------------------------------------------------------------

#[test]
fn int_conversion_boundaries() {
    assert_same_all(&[
        // 100.0 / 0x1.9p-25 is exactly 2147483648.0 -- one past INT_MAX.
        ("int_max_plus_one", b"0x1.9p-25\n0x1.9p-25\n"),
        ("just_inside_int_max", b"0x1.90001p-25\n0x1.90001p-25\n"),
        ("just_outside_int_max", b"0x1.8ffffp-25\n0x1.8ffffp-25\n"),
        ("int_min", b"-0x1.9p-25\n-0x1.9p-25\n"),
        ("just_inside_int_min", b"-0x1.90001p-25\n-0x1.90001p-25\n"),
        ("truncation", b"3\n3\n"),
        ("truncation_negative", b"-3\n-3\n"),
        ("exact_one", b"1\n1\n"),
        ("small_quotient", b"200\n200\n"),
        ("large_quotient", b"0.01\n0.01\n"),
        ("half", b"1.5\n1.5\n"),
        ("recurring", b"0.3333333333333333\n0.3333333333333333\n"),
    ]);
}

#[test]
fn mixed_branches_across_the_two_readers() {
    assert_same_all(&[
        ("inf_then_zero", b"inf\n0\n"),
        ("zero_then_inf", b"0\ninf\n"),
        ("nan_then_number", b"nan\n8\n"),
        ("number_then_nan", b"8\nnan\n"),
        ("junk_then_number", b"abc\n8\n"),
        ("number_then_junk", b"8\nabc\n"),
    ]);
}

// ---------------------------------------------------------------------------
// Process-level behaviour
// ---------------------------------------------------------------------------

#[test]
fn command_line_arguments_are_ignored() {
    // main() never looks at argc/argv.
    assert_same_with_args("argv_extra", &["a", "b", "c"], b"3\n3\n");
    assert_same_with_args("argv_flags", &["-h", "--version"], b"");
}

#[test]
fn broken_pipe_produces_no_rust_panic_message() {
    // The reader is gone before either program writes, so both must die from
    // the default SIGPIPE disposition (nothing on stderr) rather than
    // reporting a Rust panic.
    let stderr_for = |bin: &Path| {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("'{}' < /dev/null | true", bin.display()))
            .stdin(Stdio::null())
            .output()
            .expect("run shell");
        out.stderr
    };
    let c_stderr = stderr_for(c_bin());
    let r_stderr = stderr_for(rust_bin());
    assert_eq!(
        String::from_utf8_lossy(&c_stderr),
        String::from_utf8_lossy(&r_stderr),
        "stderr differs when stdout is a closed pipe"
    );
}

#[test]
fn broken_pipe_exit_status_matches() {
    use std::os::unix::process::ExitStatusExt;
    // Spawn with a piped stdout that we drop immediately, so the very first
    // write hits EPIPE.
    let status_for = |bin: &Path| {
        let mut child = Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        drop(child.stdout.take());
        let out = child.wait_with_output().expect("wait");
        (out.status.code(), out.status.signal(), out.stderr)
    };
    let c = status_for(c_bin());
    let r = status_for(rust_bin());
    assert_eq!(
        (c.0, c.1),
        (r.0, r.1),
        "exit status differs on broken pipe: C {:?} vs Rust {:?} (Rust stderr: {})",
        (c.0, c.1),
        (r.0, r.1),
        String::from_utf8_lossy(&r.2)
    );
    assert_eq!(
        String::from_utf8_lossy(&c.2),
        String::from_utf8_lossy(&r.2),
        "stderr differs on broken pipe"
    );
}

#[test]
fn stdout_content_is_the_expected_shape() {
    // A direct check on the C reference output, so a regression that changes
    // both programs identically still gets noticed.
    let c = run(c_bin(), &[], b"4\n4\n");
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        "Calling good()...\n50\n25\nFinished good()\nCalling bad()...\n25\nFinished bad()\n"
    );
    assert!(c.stderr.is_empty());
    assert_eq!(c.status.code(), Some(0));
    let r = run(rust_bin(), &[], b"4\n4\n");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}
