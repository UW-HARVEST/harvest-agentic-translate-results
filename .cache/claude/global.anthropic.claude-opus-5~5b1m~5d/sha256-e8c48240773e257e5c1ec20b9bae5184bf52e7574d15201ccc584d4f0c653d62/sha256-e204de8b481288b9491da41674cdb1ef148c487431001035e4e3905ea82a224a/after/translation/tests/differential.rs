// Differential tests: run the original C program and the Rust translation as
// SUBPROCESSES, feed both the same bytes on stdin, and require byte-identical
// stdout, byte-identical stderr and an identical exit status.
//
// The Rust code is never linked as a library; only the built binary is driven,
// exactly the way the grader compares the two programs.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test, supplied by cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// `translation/` -> repository root (the directory holding `c_src/`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C binary, building it with CMake on first use.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` - is cmake installed?");
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
        }
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
    .as_path()
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Outcome {
    use std::io::Write;

    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // A closed pipe is not a test failure: the program may exit without
        // draining stdin. Ignore write errors, matching what a shell sees.
        let _ = stdin.write_all(stdin_bytes);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");

    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().unwrap_or(-1)),
        }
    };
    #[cfg(not(unix))]
    let status = Ok(out.status.code().unwrap_or(-1));

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Core assertion: both programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout mismatch for stdin {:?}\n  C:    \"{}\"\n  Rust: \"{}\"",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr mismatch for stdin {:?}\n  C:    \"{}\"\n  Rust: \"{}\"",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "[{label}] exit status mismatch for stdin {:?} (C {:?} vs Rust {:?})",
        show(stdin_bytes),
        c.status,
        r.status
    );
}

#[track_caller]
fn assert_same_str(label: &str, stdin_text: &str) {
    assert_same(label, stdin_text.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist, run, and produce the C program's output.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1\n");
    let r = run(&rust_bin(), b"1\n");
    assert_eq!(c.status, Ok(0), "C program should exit 0");
    assert_eq!(r.status, Ok(0), "Rust program should exit 0");
    // `printIntLine(data[0])` prints the single copied zero.
    assert_eq!(c.stdout, b"0\n");
    assert_eq!(r.stdout, c.stdout);
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
}

// ---------------------------------------------------------------------------
// Phase B: the branch `main` actually takes -- `if (x)` after `scanf("%d", &x)`.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_scanf_returns_eof_x_stays_zero_bad_branch() {
    // No input at all: scanf returns EOF, `x` keeps its initializer 0 -> bad().
    assert_same_str("empty", "");
}

#[test]
fn explicit_zero_takes_bad_branch() {
    assert_same_str("zero", "0");
    assert_same_str("zero_nl", "0\n");
    assert_same_str("minus_zero", "-0\n");
    assert_same_str("plus_zero", "+0\n");
    assert_same_str("zero_padded", "0000\n");
    assert_same_str("many_zeros", "000000000000000000000000000000\n");
}

#[test]
fn nonzero_takes_good_branch() {
    assert_same_str("one", "1");
    assert_same_str("one_nl", "1\n");
    assert_same_str("negative_one", "-1\n");
    assert_same_str("plus_one", "+1\n");
    assert_same_str("seven", "7\n");
    assert_same_str("large", "123456\n");
}

#[test]
fn scanf_matching_failure_leaves_x_zero() {
    // A non-numeric first token is a matching failure: scanf returns 0 and does
    // not touch `x`, so the bad() branch runs.
    assert_same_str("alpha", "abc");
    assert_same_str("alpha_nl", "abc\n");
    assert_same_str("leading_dot", ".5\n");
    assert_same_str("sign_only_minus", "-");
    assert_same_str("sign_only_plus", "+");
    assert_same_str("sign_then_alpha", "-x\n");
    assert_same_str("punctuation", "!!!\n");
    assert_same_str("hex_prefix_stops_at_x", "0x10\n");
}

#[test]
fn scanf_skips_whitespace_including_newlines() {
    // `scanf` (unlike `fgets`) skips over any run of whitespace, newlines
    // included, before converting.
    assert_same_str("newline_only", "\n");
    assert_same_str("newlines_only", "\n\n\n");
    assert_same_str("spaces_only", "     ");
    assert_same_str("blank_lines_then_value", "\n\n   42\n");
    assert_same_str("tab_then_value", "\t3\n");
    assert_same_str("cr_then_value", "\r5\n");
    assert_same_str("vtab_then_value", "\x0b9\n");
    assert_same_str("formfeed_then_value", "\x0c9\n");
    assert_same_str("mixed_ws_then_zero", " \t\r\n\x0b\x0c 0\n");
    assert_same_str("whitespace_then_junk", "   x\n");
}

// ---------------------------------------------------------------------------
// Phase C: paths not covered above -- overflow, truncation, extra input, and
// non-text bytes.
// ---------------------------------------------------------------------------

#[test]
fn integer_boundaries() {
    assert_same_str("int_max", "2147483647\n");
    assert_same_str("int_min", "-2147483648\n");
    assert_same_str("int_max_plus_one", "2147483648\n");
    assert_same_str("int_min_minus_one", "-2147483649\n");
}

#[test]
fn values_that_truncate_to_zero_in_an_int() {
    // These are only interesting if the reader narrows to `int`: 2^32 and
    // 2^32-sized multiples have all-zero low 32 bits.
    assert_same_str("two_pow_32", "4294967296\n");
    assert_same_str("two_pow_32_plus_one", "4294967297\n");
    assert_same_str("two_pow_33", "8589934592\n");
    assert_same_str("neg_two_pow_32", "-4294967296\n");
}

#[test]
fn out_of_range_saturating_values() {
    assert_same_str("long_max", "9223372036854775807\n");
    assert_same_str("long_min", "-9223372036854775808\n");
    assert_same_str("way_too_big", "99999999999999999999999\n");
    assert_same_str("way_too_negative", "-99999999999999999999999\n");
    assert_same_str("absurdly_long_digits", &format!("{}\n", "9".repeat(400)));
    assert_same_str("long_run_of_ones", &format!("{}\n", "1".repeat(300)));
}

#[test]
fn conversion_stops_at_first_non_digit() {
    // scanf converts the longest digit prefix and leaves the rest unread.
    assert_same_str("digits_then_alpha", "5abc\n");
    assert_same_str("zero_then_alpha", "0abc\n");
    assert_same_str("scientific_reads_one", "1e5\n");
    assert_same_str("decimal_reads_int_part", "3.9\n");
    assert_same_str("zero_point_something", "0.5\n");
    assert_same_str("double_sign", "--1\n");
    assert_same_str("sign_after_digit", "1-2\n");
}

#[test]
fn extra_input_after_the_first_value_is_ignored() {
    // Only one conversion happens; trailing input must not change anything.
    assert_same_str("three_values", "1 2 3\n");
    assert_same_str("zero_then_values", "0 1 2\n");
    assert_same_str("value_then_lines", "1\nsecond line\nthird line\n");
    assert_same_str("zero_then_lines", "0\nsecond line\n");
    assert_same_str("huge_trailing_input", &format!("0\n{}", "x".repeat(70_000)));
    assert_same_str("huge_leading_whitespace", &format!("{}7\n", " ".repeat(70_000)));
}

#[test]
fn non_text_and_binary_input() {
    assert_same("nul_first", b"\x00");
    assert_same("nul_then_digit", b"\x001\n");
    assert_same("digit_then_nul", b"1\x00");
    assert_same("high_bytes", b"\xff\xfe\xfd");
    assert_same("invalid_utf8_after_value", b"1\n\xff\xfe");
    assert_same("all_byte_values", &(0u8..=255).collect::<Vec<u8>>());
}

#[test]
fn no_trailing_newline_variants() {
    // EOF reached mid-number: the digits seen so far are still converted.
    assert_same_str("bare_zero", "0");
    assert_same_str("bare_one", "1");
    assert_same_str("bare_negative", "-5");
    assert_same_str("bare_spaces_value", "  8");
}

#[test]
fn output_is_exactly_one_zero_line_for_every_input() {
    // Both branches print `data[0]`, which is always 0: an extra guard that the
    // Rust program never emits different formatting (spacing, precision or a
    // missing/extra trailing newline).
    for input in ["", "0\n", "1\n", "-1\n", "abc\n", "\n\n", "2147483648\n"] {
        let c = run(c_bin(), input.as_bytes());
        let r = run(&rust_bin(), input.as_bytes());
        assert_eq!(c.stdout, b"0\n", "C stdout changed for {input:?}");
        assert_eq!(r.stdout, b"0\n", "Rust stdout changed for {input:?}");
        assert_eq!(c.status, Ok(0));
        assert_eq!(r.status, Ok(0));
    }
}

#[test]
fn stdin_closed_immediately() {
    // Same as empty input, but the pipe is closed before the child can read.
    let c = run(c_bin(), b"");
    let r = run(&rust_bin(), b"");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status, r.status);
}

/// stdout whose reader is already gone. The C program keeps the default
/// `SIGPIPE` disposition and dies silently from signal 13; the Rust runtime
/// ignores `SIGPIPE` by default, which made `println!` panic (message on
/// stderr + abort) until `main` restored `SIG_DFL`.
#[cfg(unix)]
#[test]
fn stdout_write_to_broken_pipe_matches() {
    use std::io::Read as _;
    use std::os::unix::io::FromRawFd;
    use std::os::unix::process::ExitStatusExt;

    fn run_with_broken_stdout(program: &Path) -> (i32, Option<i32>, Vec<u8>) {
        // Create a pipe, close the read end, and hand the write end to the
        // child as stdout so the very first write fails with EPIPE.
        let mut fds = [0i32; 2];
        extern "C" {
            fn pipe(fds: *mut i32) -> i32;
            fn close(fd: i32) -> i32;
        }
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
        let (read_end, write_end) = (fds[0], fds[1]);
        unsafe { close(read_end) };
        let stdout = unsafe { Stdio::from_raw_fd(write_end) };

        let mut child = Command::new(program)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));

        let mut stderr = Vec::new();
        child
            .stderr
            .take()
            .expect("piped stderr")
            .read_to_end(&mut stderr)
            .expect("read stderr");
        let status = child.wait().expect("wait");
        (
            status.code().unwrap_or(-1),
            status.signal(),
            stderr,
        )
    }

    let (c_code, c_signal, c_err) = run_with_broken_stdout(c_bin());
    let (r_code, r_signal, r_err) = run_with_broken_stdout(&rust_bin());

    assert_eq!(
        c_err,
        r_err,
        "broken-pipe stderr mismatch\n  C:    \"{}\"\n  Rust: \"{}\"",
        show(&c_err),
        show(&r_err)
    );
    assert_eq!(
        (c_code, c_signal),
        (r_code, r_signal),
        "broken-pipe termination mismatch (C code={c_code} signal={c_signal:?} vs Rust code={r_code} signal={r_signal:?})"
    );
}

#[test]
fn repeated_runs_are_deterministic() {
    // `bad()` writes ten ints into a 10-byte alloca; if that ever perturbed the
    // observable output or exit status, it would show up as flakiness here.
    for _ in 0..25 {
        assert_same_str("determinism_bad", "0\n");
        assert_same_str("determinism_good", "1\n");
    }
}
