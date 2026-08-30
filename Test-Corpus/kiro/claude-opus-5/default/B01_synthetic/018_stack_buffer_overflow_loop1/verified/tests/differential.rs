//! Differential tests: run the C binary and the Rust binary as subprocesses,
//! feed both the identical bytes on stdin, and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! The Rust program is NEVER called as a library. Both are driven exactly the
//! way a shell would drive them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Path to the Rust binary under test. Cargo exports this for integration
/// tests, so it always points at the binary it just built.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Workspace root: the directory containing both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

static BUILD_C: Once = Once::new();

/// Build the C reference program with CMake once per test binary, then return
/// the path to it. A comparison against a program that did not build measures
/// nothing, so a build failure is a hard failure here.
fn c_bin() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build_dir = c_src.join("build");
    let exe = build_dir.join("driver");

    BUILD_C.call_once(|| {
        if exe.exists() {
            return;
        }
        std::fs::create_dir_all(&build_dir).expect("could not create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to invoke cmake (is it installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let build = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("failed to invoke cmake --build");
        assert!(
            build.status.success(),
            "cmake --build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
    });

    assert!(
        exe.exists(),
        "C reference binary missing at {}",
        exe.display()
    );
    exe
}

/// What a single run of a program produced.
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn exec(program: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        let bytes = stdin_bytes.to_vec();
        // Write on a helper thread so a program that never drains stdin cannot
        // deadlock the test on a full pipe buffer.
        std::thread::spawn(move || {
            let _ = sink.write_all(&bytes);
            let _ = sink.flush();
            // drop closes the pipe, signalling EOF
        });
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", program.display()));

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().unwrap_or(-1))
            }
            #[cfg(not(unix))]
            {
                Err(-1)
            }
        }
    };

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:02x?}"),
    }
}

/// Core assertion: identical stdout, identical stderr, identical exit status.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = exec(&c_bin(), stdin_bytes);
    let r = exec(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout mismatch for stdin {}\n  C:    {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr mismatch for stdin {}\n  C:    {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "[{label}] exit status mismatch for stdin {} (Ok=code, Err=signal)\n  C:    {:?}\n  Rust: {:?}",
        show(stdin_bytes),
        c.status,
        r.status
    );
}

fn check_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both programs exist and run.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = exec(&c_bin(), b"0");
    let r = exec(&rust_bin(), b"0");
    assert_eq!(c.status, Ok(0), "C program did not exit 0 on trivial input");
    assert_eq!(r.status, Ok(0), "Rust program did not exit 0 on trivial input");
    assert!(!c.stdout.is_empty(), "C program produced no stdout");
    assert!(!r.stdout.is_empty(), "Rust program produced no stdout");
}

// ---------------------------------------------------------------------------
// Phase B: the branches main() actually takes.
//
// main() is:   scanf("%d", &x);  if (x) good(); else bad();
// so the two reachable classes are "x parsed nonzero" and "x left/parsed zero"
// (the latter including every scanf failure, since x is initialised to 0 and
// scanf leaves the destination untouched on matching failure or EOF).
// ---------------------------------------------------------------------------

#[test]
fn empty_input_takes_the_bad_branch() {
    // scanf hits EOF immediately, returns EOF, x stays 0 -> bad()
    assert_same("empty", b"");
}

#[test]
fn zero_takes_the_bad_branch() {
    check_all(&[
        ("zero", b"0"),
        ("zero_newline", b"0\n"),
        ("negative_zero", b"-0"),
        ("plus_zero", b"+0"),
        ("many_zeros", b"0000000000"),
        ("zero_trailing_junk", b"0abc"),
    ]);
}

#[test]
fn nonzero_takes_the_good_branch() {
    check_all(&[
        ("one", b"1"),
        ("one_newline", b"1\n"),
        ("negative_one", b"-1"),
        ("plus_one", b"+1"),
        ("forty_two", b"42"),
        ("leading_zeros_then_one", b"0000000000000000000000001"),
    ]);
}

#[test]
fn single_item_and_maximum_values() {
    check_all(&[
        ("int_max", b"2147483647"),
        ("int_min", b"-2147483648"),
        ("int_max_plus_one", b"2147483648"),
        ("int_min_minus_one", b"-2147483649"),
        ("uint_max", b"4294967295"),
        // 2^32: truncating the accumulated long to int yields 0, which flips
        // main() onto the bad() branch. Both branches print the same thing, so
        // this is asserted rather than assumed.
        ("two_pow_32", b"4294967296"),
        ("two_pow_32_plus_one", b"4294967297"),
        ("long_max", b"9223372036854775807"),
        ("long_min", b"-9223372036854775808"),
        ("long_max_plus_one", b"9223372036854775808"),
        ("huge_overflow", b"99999999999999999999999999999999999999"),
        ("huge_overflow_negative", b"-99999999999999999999999999999999999999"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase B: scanf reading semantics. %d skips leading whitespace INCLUDING
// newlines (unlike fgets), then takes an optional sign and one or more digits.
// ---------------------------------------------------------------------------

#[test]
fn scanf_skips_leading_whitespace_across_newlines() {
    check_all(&[
        ("leading_spaces", b"   7"),
        ("leading_newlines", b"\n\n\n7"),
        ("leading_crlf", b"\r\n7"),
        ("leading_tabs", b"\t\t7"),
        ("leading_vtab_formfeed", b"\x0b\x0c7"),
        ("mixed_whitespace", b"  \n\t \r\n  7  \n"),
        ("mixed_whitespace_zero", b"  \n\t \r\n  0  \n"),
    ]);
}

#[test]
fn whitespace_only_input_is_eof_for_scanf() {
    check_all(&[
        ("spaces_only", b"   "),
        ("newlines_only", b"\n\n\n"),
        ("tabs_only", b"\t\t"),
        ("single_newline", b"\n"),
        ("all_whitespace", b" \t\n\x0b\x0c\r"),
    ]);
}

#[test]
fn scanf_matching_failures_leave_x_at_zero() {
    check_all(&[
        ("alpha", b"abc"),
        ("punctuation", b"!!!"),
        ("dot", b"."),
        ("leading_dot_number", b".5"),
        ("sign_only_minus", b"-"),
        ("sign_only_plus", b"+"),
        ("sign_then_newline", b"-\n"),
        ("sign_then_space_digit", b"- 5"),
        ("double_minus", b"--5"),
        ("plus_then_minus", b"+-1"),
        ("underscore", b"_1"),
        ("word_then_number", b"abc 5"),
    ]);
}

#[test]
fn scanf_stops_at_first_non_digit() {
    check_all(&[
        ("hex_literal", b"0x10"),   // parses 0, stops at 'x' -> bad()
        ("hex_nonzero", b"0X1F"),   // likewise 0
        ("float", b"3.7"),          // parses 3 -> good()
        ("float_zero", b"0.9"),     // parses 0 -> bad()
        ("exponent", b"1e9"),       // parses 1 -> good()
        ("digits_then_alpha", b"1abc"),
        ("digits_then_comma", b"12,34"),
        ("octal_looking", b"0755"),
        ("thousands_sep", b"1_000"),
    ]);
}

#[test]
fn only_the_first_conversion_is_consumed() {
    // main() performs exactly one scanf; trailing input is never read.
    check_all(&[
        ("three_numbers", b"7 8 9"),
        ("three_numbers_zero_first", b"0 1 2"),
        ("numbers_on_lines", b"5\n6\n7\n"),
        ("zero_then_nonzero", b"0\n1\n"),
        ("nonzero_then_zero", b"1\n0\n"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: input classes not covered above.
// ---------------------------------------------------------------------------

#[test]
fn binary_and_nul_bytes() {
    check_all(&[
        ("nul_only", b"\x00"),
        ("nul_then_digit", b"\x001"),
        ("digit_then_nul", b"1\x00"),
        ("zero_then_nul", b"0\x00"),
        ("high_bytes", b"\xff\xfe\xfd"),
        ("utf8_text", "héllo".as_bytes()),
        ("bom_then_digit", b"\xef\xbb\xbf1"),
        ("binary_noise", b"\x01\x02\x03\x04\x05"),
    ]);
}

#[test]
fn very_long_inputs() {
    // A digit run far longer than any buffer, plus a long whitespace run
    // before the digits, to exercise the scan loop rather than a fast path.
    let long_digits = vec![b'9'; 100_000];
    let long_zeros = vec![b'0'; 100_000];
    let mut long_ws = vec![b' '; 100_000];
    long_ws.push(b'1');
    let mut long_ws_zero = vec![b'\n'; 100_000];
    long_ws_zero.push(b'0');
    let long_alpha = vec![b'z'; 100_000];

    check_all(&[
        ("long_digits", &long_digits),
        ("long_zeros", &long_zeros),
        ("long_whitespace_then_one", &long_ws),
        ("long_newlines_then_zero", &long_ws_zero),
        ("long_alpha", &long_alpha),
    ]);
}

#[test]
fn every_single_digit() {
    // 0 is the bad() branch, 1..=9 the good() branch.
    for d in b'0'..=b'9' {
        assert_same(&format!("digit_{}", d as char), &[d]);
    }
}

#[test]
fn sign_and_digit_combinations() {
    for sign in ["", "+", "-"] {
        for digits in ["0", "1", "9", "10", "0000", "007"] {
            let input = format!("{sign}{digits}");
            assert_same(&format!("combo_{input}"), input.as_bytes());
        }
    }
}

#[test]
fn stdin_closed_immediately() {
    // Equivalent to `prog < /dev/null`: scanf sees EOF, x stays 0 -> bad().
    assert_same("closed_stdin", b"");
}

#[test]
fn output_is_exactly_one_line() {
    // Pin the concrete observable contract as well as the C/Rust agreement:
    // both good() and bad() copy a zero-filled array and print element 0.
    for input in [&b""[..], b"0", b"1", b"-1", b"abc", b"2147483648"] {
        let c = exec(&c_bin(), input);
        let r = exec(&rust_bin(), input);
        assert_eq!(c.stdout, b"0\n", "C stdout changed for {}", show(input));
        assert_eq!(r.stdout, b"0\n", "Rust stdout changed for {}", show(input));
        assert!(c.stderr.is_empty() && r.stderr.is_empty());
        assert_eq!(c.status, Ok(0));
        assert_eq!(r.status, Ok(0));
    }
}

#[test]
fn command_line_arguments_are_ignored() {
    // main() takes no parameters, so argv must not change behavior.
    for args in [vec!["extra"], vec!["-1", "--help"], vec!["", "0"]] {
        let mut c = Command::new(c_bin());
        let mut r = Command::new(rust_bin());
        let co = c
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("run C with args");
        let ro = r
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("run Rust with args");
        assert_eq!(co.stdout, ro.stdout, "stdout differs with args {args:?}");
        assert_eq!(co.stderr, ro.stderr, "stderr differs with args {args:?}");
        assert_eq!(
            co.status.code(),
            ro.status.code(),
            "exit status differs with args {args:?}"
        );
    }
}

#[test]
fn repeated_runs_are_deterministic() {
    // bad() writes past its alloca() region in C; confirm the observable
    // output is nonetheless stable run to run, and that Rust agrees each time.
    for _ in 0..25 {
        assert_same("determinism_bad", b"0");
        assert_same("determinism_good", b"1");
    }
}

/// Hand the program a stdout pipe whose read end is already closed and report
/// how it terminated. Returns `Ok(code)` / `Err(signal)` like `Run::status`.
#[cfg(unix)]
fn status_with_broken_stdout(program: &Path) -> Result<i32, i32> {
    use std::os::unix::process::ExitStatusExt;

    // Borrow a pipe from a helper child, then kill the helper so the read end
    // is gone while we still hold the write end.
    let mut sink = Command::new("cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn `cat` to borrow a pipe from");
    let write_end = sink.stdin.take().expect("cat stdin was piped");
    sink.kill().expect("kill helper `cat`");
    sink.wait().expect("reap helper `cat`");

    let status = Command::new(program)
        .stdin(Stdio::null())
        .stdout(Stdio::from(write_end))
        .stderr(Stdio::null())
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", program.display()));

    match status.code() {
        Some(code) => Ok(code),
        None => Err(status.signal().unwrap_or(-1)),
    }
}

#[test]
#[cfg(unix)]
fn broken_stdout_pipe_terminates_identically() {
    // The C program has SIGPIPE at its default disposition and is killed by
    // signal 13. The Rust runtime installs SIG_IGN for SIGPIPE before main, so
    // an unfixed translation exits 0 here instead -- a pure exit-status
    // divergence that a stdout-only comparison would never catch.
    let c = status_with_broken_stdout(&c_bin());
    let r = status_with_broken_stdout(&rust_bin());
    assert_eq!(
        c, r,
        "termination differs when stdout is a closed pipe (Ok=code, Err=signal): C={c:?} Rust={r:?}"
    );
}

#[test]
fn stdout_redirected_to_a_closed_descriptor() {
    // Distinct from the broken-pipe case: writes fail with EBADF, which is not
    // a signal. Both programs ignore the write error and exit 0.
    let c = exec_with_null_stdout(&c_bin());
    let r = exec_with_null_stdout(&rust_bin());
    assert_eq!(c, r, "termination differs with stdout discarded: C={c:?} Rust={r:?}");
}

fn exec_with_null_stdout(program: &Path) -> Result<i32, i32> {
    let status = Command::new(program)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", program.display()));
    match status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(status.signal().unwrap_or(-1))
            }
            #[cfg(not(unix))]
            {
                Err(-1)
            }
        }
    }
}
