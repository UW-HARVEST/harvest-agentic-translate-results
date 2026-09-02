//! Differential integration tests: run the C binary and the Rust binary as
//! subprocesses over identical stdin and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status (including death by
//! signal).
//!
//! The Rust code is never linked as a library here; both programs are driven
//! exactly the way a shell drives them.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Everything a shell can observe about one run.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signum)` when killed by a signal.
    status: Result<i32, i32>,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test, as built by cargo for this test run.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C binary, building it with cmake if it is not present.
///
/// Nothing inside `c_src/` is modified; only the out-of-source `c_src/build`
/// directory is populated, which is the documented way to build this project.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");

        // Accept a binary that is already built (including the multi-config
        // layouts cmake generators sometimes produce).
        for candidate in [
            build.join("driver"),
            build.join("Release").join("driver"),
            build.join("Debug").join("driver"),
        ] {
            if candidate.is_file() {
                return candidate;
            }
        }

        std::fs::create_dir_all(&build).expect("could not create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake ..` -- is cmake installed?");
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
            .expect("failed to run `cmake --build .`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        let built = build.join("driver");
        assert!(
            built.is_file(),
            "C binary missing after build: {}",
            built.display()
        );
        built
    })
}

/// Run `bin` with `input` on stdin and capture everything observable.
fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // Feed stdin from a helper thread. Either program may exit before reading
    // all of the input (scanf stops as soon as its conversions are done), which
    // closes the pipe; a broken pipe is expected and is not a test failure.
    let mut stdin = child.stdin.take().expect("stdin was piped");
    let payload = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&payload);
        let _ = stdin.flush();
        drop(stdin);
    });

    let output = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", bin.display()));
    writer.join().expect("stdin writer thread panicked");

    let status = match output.status.code() {
        Some(code) => Ok(code),
        None => Err(output
            .status
            .signal()
            .expect("a process with no exit code must have been signalled")),
    };

    Outcome {
        stdout: output.stdout,
        stderr: output.stderr,
        status,
    }
}

/// Assert that C and Rust agree on stdout, stderr and exit status.
fn assert_identical(case: &str, input: &[u8]) {
    let c = run(c_binary(), input);
    let rust = run(rust_binary(), input);

    assert_eq!(
        c.stdout,
        rust.stdout,
        "case `{case}`: stdout differs\n  input: {:?}\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&rust.stdout)
    );
    assert_eq!(
        c.stderr,
        rust.stderr,
        "case `{case}`: stderr differs\n  input: {:?}\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(
        c.status, rust.status,
        "case `{case}`: exit status differs\n  input: {:?}\n  C: {:?}, Rust: {:?}",
        String::from_utf8_lossy(input),
        c.status,
        rust.status
    );
}

/// Run a table of `(name, input)` cases, reporting every mismatch at once.
fn assert_all(cases: &[(&str, &[u8])]) {
    for (name, input) in cases {
        assert_identical(name, input);
    }
}

// ---------------------------------------------------------------------------
// Sanity: the two binaries exist and are runnable at all.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_binary(), b"6 3");
    let rust = run(rust_binary(), b"6 3");
    assert_eq!(c.stdout, b"quotient: 2, remainder: 0\n".to_vec());
    assert_eq!(c, rust);
}

// ---------------------------------------------------------------------------
// Happy path: both operands parse, division is well defined.
// ---------------------------------------------------------------------------

#[test]
fn exact_and_inexact_division() {
    assert_all(&[
        ("exact", b"6 3"),
        ("remainder", b"7 2"),
        ("divisor_larger", b"2 7"),
        ("by_one", b"7 1"),
        ("self", b"7 7"),
        ("zero_dividend", b"0 5"),
        ("negative_zero_dividend", b"-0 5"),
        ("remainder_sign", b"7 3"),
        ("large_pair", b"2147483647 -2147483648"),
    ]);
}

/// `div()` truncates toward zero, so the remainder carries the dividend's sign.
#[test]
fn truncation_toward_zero_for_every_sign_combination() {
    assert_all(&[
        ("pos_pos", b"7 2"),
        ("neg_pos", b"-7 2"),
        ("pos_neg", b"7 -2"),
        ("neg_neg", b"-7 -2"),
        ("neg_rem_pos", b"-7 3"),
        ("pos_rem_neg", b"7 -3"),
        ("neg_rem_neg", b"-7 -3"),
    ]);
}

#[test]
fn int_boundary_operands() {
    assert_all(&[
        ("intmax_by_one", b"2147483647 1"),
        ("intmin_by_one", b"-2147483648 1"),
        ("intmax_by_intmax", b"2147483647 2147483647"),
        ("intmin_by_two", b"-2147483648 2"),
        ("intmin_by_intmin", b"-2147483648 -2147483648"),
        ("one_by_intmin", b"1 -2147483648"),
        ("five_by_intmin", b"5 -2147483648"),
    ]);
}

// ---------------------------------------------------------------------------
// Undefined-division paths: the process dies from SIGFPE with no stdout.
// These are the cases where checking stdout alone would not notice a
// difference -- the exit status is the whole signal.
// ---------------------------------------------------------------------------

#[test]
fn division_by_zero_kills_the_process() {
    assert_all(&[
        ("positive_over_zero", b"5 0"),
        ("negative_over_zero", b"-5 0"),
        ("zero_over_zero", b"0 0"),
        ("intmax_over_zero", b"2147483647 0"),
        ("intmin_over_zero", b"-2147483648 0"),
        ("negative_zero_divisor", b"5 -0"),
        ("zero_divisor_with_newline", b"5 0\n"),
    ]);
}

#[test]
fn intmin_over_minus_one_kills_the_process() {
    assert_all(&[
        ("intmin_neg1", b"-2147483648 -1"),
        // 2147483648 narrows to INT_MIN, reaching the same trap.
        ("twopow31_neg1", b"2147483648 -1"),
        // 4294967296 narrows to 0 and 8589934592 narrows to 0 -> zero divisor.
        ("wrapped_zero_divisor", b"5 4294967296"),
        ("wrapped_zero_divisor_big", b"5 8589934592"),
    ]);
}

/// The default divisor is 1, so a failed second conversion cannot trap; only an
/// explicitly supplied 0 or the INT_MIN/-1 pair can.
#[test]
fn trap_requires_an_explicit_divisor() {
    assert_all(&[
        ("no_divisor_intmin", b"-2147483648"),
        ("unparsable_divisor_intmin", b"-2147483648 x"),
    ]);
}

// ---------------------------------------------------------------------------
// scanf failure paths: x and y keep their initializers (1 and 1) when a
// conversion does not happen.
// ---------------------------------------------------------------------------

#[test]
fn empty_and_whitespace_only_input_leaves_both_defaults() {
    assert_all(&[
        ("empty", b""),
        ("single_newline", b"\n"),
        ("many_newlines", b"\n\n\n"),
        ("spaces", b"   "),
        ("tabs", b"\t\t"),
        ("mixed_whitespace", b" \t\n\r\x0b\x0c "),
    ]);
}

#[test]
fn only_one_operand_leaves_the_divisor_at_one() {
    assert_all(&[
        ("bare", b"7"),
        ("trailing_newline", b"7\n"),
        ("trailing_space", b"7 "),
        ("trailing_spaces", b"7   "),
        ("leading_newlines", b"\n\n7"),
        ("negative", b"-7"),
        ("zero", b"0"),
    ]);
}

#[test]
fn matching_failure_on_the_first_conversion_leaves_both_defaults() {
    assert_all(&[
        ("letters", b"abc"),
        ("letters_then_numbers", b"abc 5 2"),
        ("punctuation", b"!!"),
        ("dot", b".5 2"),
        ("sign_only_minus", b"-"),
        ("sign_only_plus", b"+"),
        ("sign_then_space", b"- 5"),
        ("sign_then_letter", b"-a"),
        ("double_minus", b"--5 2"),
        ("double_plus", b"++5 2"),
        ("minus_newline", b"-\n5"),
    ]);
}

#[test]
fn matching_failure_on_the_second_conversion_leaves_the_divisor_at_one() {
    assert_all(&[
        ("letter", b"5 abc"),
        ("dot", b"5 .2"),
        ("sign_only", b"5 -"),
        ("double_sign", b"5 --2"),
        ("sign_then_letter", b"5 -x"),
        ("punctuation", b"5 ,"),
    ]);
}

/// `%d` stops at the first non-digit, so these all parse a prefix.
#[test]
fn numeric_prefix_is_taken_and_the_rest_abandoned() {
    assert_all(&[
        ("hex_like", b"0x10 2"),
        ("float_like", b"5.7 2"),
        ("exponent_like", b"5e3 2"),
        ("comma", b"5,2"),
        ("underscore", b"5_2"),
        ("trailing_letter", b"5a 2"),
        ("divisor_float_like", b"6 3.9"),
        ("divisor_hex_like", b"6 0x2"),
        // No separator: the sign starts the second field.
        ("adjacent_minus", b"5-2"),
        ("adjacent_plus", b"5+2"),
    ]);
}

// ---------------------------------------------------------------------------
// Whitespace handling: `scanf` skips it freely, including across newlines.
// ---------------------------------------------------------------------------

#[test]
fn operands_may_be_separated_by_any_whitespace() {
    assert_all(&[
        ("space", b"8 3"),
        ("newline", b"8\n3"),
        ("blank_lines", b"8\n\n\n3"),
        ("tab", b"\t8\t3"),
        ("crlf", b"8\r\n3"),
        ("vtab_formfeed", b"\x0b\x0c8 3"),
        ("leading_and_trailing", b"   8   3  "),
        ("leading_before_signs", b"  -8   -3 "),
        ("no_trailing_newline", b"8 3"),
    ]);
}

#[test]
fn extra_input_after_the_two_operands_is_ignored() {
    assert_all(&[
        ("third_number", b"9 4 99"),
        ("trailing_text", b"9 4 hello world"),
        ("trailing_lines", b"9 4\nignored\nalso ignored\n"),
    ]);
}

// ---------------------------------------------------------------------------
// Sign and zero-padding forms accepted by `%d`.
// ---------------------------------------------------------------------------

#[test]
fn explicit_signs_and_leading_zeros() {
    assert_all(&[
        ("plus_both", b"+5 +2"),
        ("plus_dividend", b"+5 2"),
        ("plus_divisor", b"5 +2"),
        ("minus_zero_dividend", b"-0 3"),
        ("leading_zeros_dividend", b"0000000000000000000000005 2"),
        ("leading_zeros_divisor", b"5 000000000000000002"),
        ("all_zeros_dividend", b"00000000000000000000000000 5"),
        ("signed_leading_zeros", b"-0000007 2"),
        ("plus_leading_zeros", b"+0000007 2"),
    ]);
}

// ---------------------------------------------------------------------------
// Out-of-range fields. glibc converts with `strtol` (saturating at
// LONG_MAX/LONG_MIN) and then narrows to `int`, so these wrap in ways that
// look wrong but must be reproduced exactly.
// ---------------------------------------------------------------------------

#[test]
fn values_above_int_range_narrow_to_int() {
    assert_all(&[
        ("twopow31", b"2147483648 1"),
        ("twopow31_plus_one", b"2147483649 1"),
        ("twopow32", b"4294967296 1"),
        ("twopow32_plus_one", b"4294967297 1"),
        ("twopow32_plus_five", b"4294967301 1"),
        ("eleven_digits", b"99999999999 2"),
        ("negative_eleven_digits", b"-99999999999 2"),
        ("divisor_narrows", b"4294967301 4294967298"),
    ]);
}

#[test]
fn values_beyond_long_range_saturate_then_narrow() {
    assert_all(&[
        ("long_max", b"9223372036854775807 1"),
        ("long_max_plus_one", b"9223372036854775808 1"),
        ("long_min", b"-9223372036854775808 1"),
        ("long_min_minus_one", b"-9223372036854775809 1"),
        ("far_above", b"999999999999999999999999 2"),
        ("far_below", b"-99999999999999999999999999 1"),
        ("divisor_saturates", b"5 99999999999999999999999"),
        ("divisor_saturates_negative", b"5 -99999999999999999999999"),
        ("both_saturate", b"99999999999999999999 99999999999999999999"),
    ]);
}

#[test]
fn very_long_digit_runs() {
    let nines = "9".repeat(400);
    let neg_nines = format!("-{nines}");
    let padded = format!("{}7", "0".repeat(400));
    let cases: Vec<(String, String)> = vec![
        ("four_hundred_nines".into(), format!("{nines} 2")),
        ("four_hundred_nines_negative".into(), format!("{neg_nines} 2")),
        ("four_hundred_leading_zeros".into(), format!("{padded} 2")),
        ("long_run_as_divisor".into(), format!("5 {nines}")),
        ("both_long_runs".into(), format!("{nines} {neg_nines}")),
    ];
    for (name, input) in &cases {
        assert_identical(name, input.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Odd byte streams and stdin shapes.
// ---------------------------------------------------------------------------

#[test]
fn non_utf8_and_nul_bytes_in_the_stream() {
    assert_all(&[
        ("nul_between_digits", b"5\x002"),
        ("leading_nul", b"\x005 2"),
        ("high_bytes", b"\xff\xfe 5 2"),
        ("high_bytes_after_digit", b"5\xff2"),
        ("invalid_utf8_only", b"\x80\x81\x82"),
        ("digits_then_invalid_utf8", b"9 4\x80\x81"),
    ]);
}

/// A large stdin exercises buffered reading in both stdio implementations.
#[test]
fn large_inputs() {
    let leading = format!("{}8 3", " ".repeat(100_000));
    let trailing = format!("8 3 {}", "x".repeat(200_000));
    let junk_first = format!("{} 8 3", "z".repeat(100_000));
    let many_newlines = format!("{}8 3", "\n".repeat(50_000));
    for (name, input) in [
        ("leading_whitespace_100k", leading),
        ("trailing_junk_200k", trailing),
        ("leading_junk_100k", junk_first),
        ("leading_newlines_50k", many_newlines),
    ] {
        assert_identical(name, input.as_bytes());
    }
}

#[test]
fn closed_and_empty_stdin() {
    // An immediately-EOF stdin is the `Stdio::null()` equivalent of empty input.
    for bin in [c_binary(), rust_binary()] {
        let out = Command::new(bin)
            .stdin(Stdio::null())
            .output()
            .expect("spawn with null stdin");
        assert_eq!(out.stdout, b"quotient: 1, remainder: 0\n".to_vec());
        assert!(out.stderr.is_empty());
        assert_eq!(out.status.code(), Some(0));
    }
    assert_identical("empty_again", b"");
}

/// stdin as a regular file rather than a pipe: a different stdio buffering mode
/// on the C side, and it must not change the output.
#[test]
fn stdin_from_a_regular_file() {
    let dir = std::env::temp_dir().join(format!("driver_difftest_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("input.txt");

    let mut outcomes = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        std::fs::write(&path, b"9 4\n").expect("write temp input");
        let file = std::fs::File::open(&path).expect("open temp input");
        let out = Command::new(bin)
            .stdin(Stdio::from(file))
            .output()
            .expect("spawn with file stdin");
        outcomes.push((out.stdout, out.stderr, out.status.code(), out.status.signal()));
    }
    assert_eq!(outcomes[0], outcomes[1], "file-backed stdin results differ");
    assert_eq!(outcomes[0].0, b"quotient: 2, remainder: 1\n".to_vec());

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

// ---------------------------------------------------------------------------
// Output shape: the exact printf format, including the trailing newline.
// ---------------------------------------------------------------------------

#[test]
fn output_format_is_byte_exact() {
    // Checked against the C program's own bytes, then pinned to the literal
    // format string from the C source.
    assert_identical("format", b"-17 5");
    let c = run(c_binary(), b"-17 5");
    let rust = run(rust_binary(), b"-17 5");
    assert_eq!(c.stdout, b"quotient: -3, remainder: -2\n".to_vec());
    assert_eq!(rust.stdout, c.stdout);
    // No padding, no extra newline, nothing on stderr.
    assert_eq!(rust.stdout.iter().filter(|&&b| b == b'\n').count(), 1);
    assert!(rust.stderr.is_empty());
}

/// Neither program writes anything to stderr on any path.
#[test]
fn stderr_is_always_empty() {
    for input in [
        &b""[..],
        b"7",
        b"7 2",
        b"abc",
        b"5 0",
        b"-2147483648 -1",
        b"99999999999 2",
    ] {
        let c = run(c_binary(), input);
        let rust = run(rust_binary(), input);
        assert!(
            c.stderr.is_empty(),
            "C wrote to stderr for {:?}",
            String::from_utf8_lossy(input)
        );
        assert_eq!(
            c.stderr,
            rust.stderr,
            "stderr differs for {:?}",
            String::from_utf8_lossy(input)
        );
    }
}

// ---------------------------------------------------------------------------
// Broad sweep: every small operand pair, so no arithmetic or trap case in the
// int range near zero can differ.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_small_operand_sweep() {
    for x in -12i32..=12 {
        for y in -12i32..=12 {
            let input = format!("{x} {y}");
            assert_identical(&format!("sweep_{x}_{y}"), input.as_bytes());
        }
    }
}

#[test]
fn boundary_operand_sweep() {
    let interesting: [i64; 14] = [
        i32::MIN as i64,
        i32::MIN as i64 + 1,
        -1000,
        -3,
        -1,
        0,
        1,
        3,
        1000,
        i32::MAX as i64 - 1,
        i32::MAX as i64,
        i32::MAX as i64 + 1, // narrows to INT_MIN
        4294967296,          // narrows to 0
        9223372036854775807, // narrows to -1
    ];
    for x in interesting {
        for y in interesting {
            let input = format!("{x} {y}");
            assert_identical(&format!("boundary_{x}_{y}"), input.as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Process-level behavior that is not driven by stdin content: argv, and the
// signal dispositions and stdout failure modes the program inherits.
// ---------------------------------------------------------------------------

/// `main()` takes no parameters, so arguments must be ignored identically.
#[test]
fn command_line_arguments_are_ignored() {
    for args in [
        vec!["extra"],
        vec!["--flag", "-x"],
        vec!["1", "2", "3"],
        vec![""],
    ] {
        let mut outs = Vec::new();
        for bin in [c_binary(), rust_binary()] {
            let mut child = Command::new(bin)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn with args");
            let mut stdin = child.stdin.take().expect("stdin piped");
            let writer = std::thread::spawn(move || {
                let _ = stdin.write_all(b"9 4");
            });
            let out = child.wait_with_output().expect("wait");
            writer.join().expect("writer thread");
            outs.push((out.stdout, out.stderr, out.status.code(), out.status.signal()));
        }
        assert_eq!(outs[0], outs[1], "argv {args:?} results differ");
        assert_eq!(outs[0].0, b"quotient: 2, remainder: 1\n".to_vec());
    }
}

/// Writing to a closed stdout pipe must kill the process with `SIGPIPE`, the
/// way it kills the C program. The Rust runtime installs `SIG_IGN` for
/// `SIGPIPE` before `main`, so the translation has to restore `SIG_DFL`;
/// without that the C program dies from signal 13 while Rust exits 0.
#[test]
fn closed_stdout_pipe_raises_sigpipe() {
    use std::os::fd::{FromRawFd, OwnedFd};

    /// Build a pipe, close the read end, and hand the write end to the child as
    /// stdout so the very first write fails with `EPIPE`.
    fn run_with_closed_stdout(bin: &Path, input: &[u8]) -> (Vec<u8>, Option<i32>, Option<i32>) {
        extern "C" {
            fn pipe(fds: *mut i32) -> i32;
            fn close(fd: i32) -> i32;
        }
        let mut fds = [0i32; 2];
        // SAFETY: `fds` is a valid two-element array for `pipe` to fill in.
        let rc = unsafe { pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0, "pipe() failed");
        // SAFETY: `pipe` succeeded, so both descriptors are open and owned here.
        let write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };
        // SAFETY: closing the read end we own; nothing else refers to it.
        unsafe { close(fds[0]) };

        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(write_end))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn with closed-pipe stdout");

        let mut stdin = child.stdin.take().expect("stdin piped");
        let payload = input.to_vec();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(&payload);
        });
        let out = child.wait_with_output().expect("wait");
        writer.join().expect("writer thread");
        (out.stderr, out.status.code(), out.status.signal())
    }

    for (label, input) in [
        ("normal_output", &b"9 4"[..]),
        ("defaulted_output", b""),
        ("single_operand", b"7"),
        // This one traps before ever writing, so SIGFPE wins over SIGPIPE.
        ("traps_before_writing", b"5 0"),
    ] {
        let c = run_with_closed_stdout(c_binary(), input);
        let rust = run_with_closed_stdout(rust_binary(), input);
        assert_eq!(
            c, rust,
            "case `{label}`: closed-stdout behavior differs (stderr, code, signal)"
        );
    }

    // Pin the expectation, so the test cannot pass by both programs exiting 0.
    let (_, code, signal) = run_with_closed_stdout(c_binary(), b"9 4");
    assert_eq!((code, signal), (None, Some(13)), "expected death by SIGPIPE");
    let (_, code, signal) = run_with_closed_stdout(rust_binary(), b"9 4");
    assert_eq!((code, signal), (None, Some(13)), "expected death by SIGPIPE");

    // And that a trap still beats it.
    let (_, code, signal) = run_with_closed_stdout(rust_binary(), b"5 0");
    assert_eq!((code, signal), (None, Some(8)), "expected death by SIGFPE");
}

/// An unreadable stdin makes the first conversion fail, so both defaults stand.
#[test]
fn unreadable_stdin_leaves_both_defaults() {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    let dir = std::fs::File::open("/").expect("open / as a directory");

    let mut outs = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        // Reading from a directory descriptor fails with EISDIR.
        let dup = {
            extern "C" {
                fn dup(fd: i32) -> i32;
            }
            // SAFETY: `dir` is an open descriptor; `dup` returns a new owned one.
            let fd = unsafe { dup(dir.as_raw_fd()) };
            assert!(fd >= 0, "dup() failed");
            // SAFETY: `fd` is a fresh descriptor owned by this scope.
            unsafe { OwnedFd::from_raw_fd(fd) }
        };
        let out = Command::new(bin)
            .stdin(Stdio::from(dup))
            .output()
            .expect("spawn with directory stdin");
        outs.push((out.stdout, out.stderr, out.status.code(), out.status.signal()));
    }
    assert_eq!(outs[0], outs[1], "unreadable-stdin results differ");
    assert_eq!(outs[0].0, b"quotient: 1, remainder: 0\n".to_vec());
}

/// stdout on a device that always reports ENOSPC: `printf`'s failure is ignored
/// and the program still exits 0.
#[test]
fn full_stdout_device_is_ignored() {
    let full = match std::fs::OpenOptions::new().write(true).open("/dev/full") {
        Ok(f) => f,
        // Not every environment provides /dev/full; nothing to compare then.
        Err(_) => return,
    };
    drop(full);

    let mut outs = Vec::new();
    for bin in [c_binary(), rust_binary()] {
        let dev = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("open /dev/full");
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(dev))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn with /dev/full stdout");
        let mut stdin = child.stdin.take().expect("stdin piped");
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(b"9 4");
        });
        let out = child.wait_with_output().expect("wait");
        writer.join().expect("writer thread");
        outs.push((out.stderr, out.status.code(), out.status.signal()));
    }
    assert_eq!(outs[0], outs[1], "/dev/full stdout results differ");
}
