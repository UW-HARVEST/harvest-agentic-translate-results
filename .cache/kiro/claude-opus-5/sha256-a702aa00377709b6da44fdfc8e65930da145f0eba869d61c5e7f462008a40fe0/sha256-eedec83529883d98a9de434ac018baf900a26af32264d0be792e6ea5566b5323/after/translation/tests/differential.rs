//! Differential tests: run the original C executable and the Rust executable as
//! subprocesses with identical stdin, and require byte-identical stdout, stderr
//! and identical termination status.
//!
//! The Rust code is never linked as a library here. Both programs are driven
//! exactly the way a shell drives them, because that is how they are compared.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary under test, provided by cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Repository root: the parent of the `translation/` crate directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with cmake on first use so that a bare
/// `cargo test` is self-sufficient.
fn c_bin() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build_dir = c_src.join("build");
    let exe = build_dir.join("driver");
    if exe.is_file() {
        return exe;
    }

    std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");

    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build_dir)
        .output()
        .expect("failed to run `cmake` - is cmake installed?");
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

    assert!(exe.is_file(), "C binary missing after build: {}", exe.display());
    exe
}

/// Everything observable about one run of a program.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} status={}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            match self.status {
                Ok(c) => format!("exit {}", c),
                Err(s) => format!("signal {}", s),
            }
        )
    }
}

/// Run `exe` with `args`, feeding `stdin_bytes` on stdin.
fn run(exe: &Path, args: &[&str], stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    // Write on a helper thread: a large payload can fill the pipe buffer while
    // the child is still producing output, which would deadlock a single thread.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let payload = stdin_bytes.to_vec();
    let writer = std::thread::spawn(move || {
        // A broken pipe here is normal (the child may exit before reading all
        // input), so the error is deliberately ignored.
        let _ = stdin.write_all(&payload);
        let _ = stdin.flush();
    });

    let out = child.wait_with_output().expect("failed to wait for child");
    writer.join().expect("stdin writer thread panicked");

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().expect("neither exit code nor signal")),
        },
    }
}

/// Assert the C and Rust programs are indistinguishable for one input.
#[track_caller]
fn assert_same_with_args(case: &str, args: &[&str], stdin_bytes: &[u8]) {
    let c = run(&c_bin(), args, stdin_bytes);
    let r = run(&rust_bin(), args, stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for case {case:?}\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for case {case:?}\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for case {case:?}\n  C:    {c:?}\n  Rust: {r:?}"
    );
}

#[track_caller]
fn assert_same(case: &str, stdin_bytes: &[u8]) {
    assert_same_with_args(case, &[], stdin_bytes);
}

// ---------------------------------------------------------------------------
// Phase A - both binaries exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = run(&c_bin(), &[], b"1\n");
    let r = run(&rust_bin(), &[], b"1\n");
    assert_eq!(c.status, Ok(0), "C program did not exit 0: {c:?}");
    assert_eq!(r.status, Ok(0), "Rust program did not exit 0: {r:?}");
    assert_eq!(c.stdout, b"302\n", "unexpected C baseline output: {c:?}");
    assert_eq!(r.stdout, c.stdout);
}

// ---------------------------------------------------------------------------
// Phase B - the input classes `scanf("%d")` branches on
// ---------------------------------------------------------------------------

/// EOF before any conversion: scanf returns EOF, the ignored return value
/// leaves `x` at its initializer 0, so the program prints 300.
#[test]
fn empty_input() {
    assert_same("empty", b"");
}

#[test]
fn whitespace_only_input() {
    for (name, bytes) in [
        ("single space", &b" "[..]),
        ("single newline", b"\n"),
        ("many newlines", b"\n\n\n\n"),
        ("mixed whitespace", b" \t\n\x0b\x0c\r "),
        ("crlf only", b"\r\n\r\n"),
    ] {
        assert_same(name, bytes);
    }
}

#[test]
fn single_item() {
    assert_same("single digit", b"7\n");
    assert_same("single digit no newline", b"7");
    assert_same("zero", b"0\n");
    assert_same("negative zero", b"-0\n");
}

/// `%d` skips leading whitespace including newlines, so the number may sit on
/// any later line - unlike `fgets`, which would stop at the first newline.
#[test]
fn scanf_reads_across_newlines() {
    assert_same("leading newlines", b"\n\n\n42\n");
    assert_same("leading blank and spaces", b"  \n\n  42  \n");
    assert_same("crlf separated", b"\r\n\r\n11\r\n");
    assert_same("every isspace class", b" \t\n\x0b\x0c\r-13\n");
    assert_same("tab then digits", b"\t\t9\n");
    assert_same("vtab then digits", b"\x0b8\n");
    assert_same("formfeed then digits", b"\x0c8\n");
}

#[test]
fn explicit_signs() {
    assert_same("plus", b"+7\n");
    assert_same("minus", b"-5\n");
    assert_same("plus zero", b"+0\n");
    assert_same("space between sign and digits", b"- 5\n");
    assert_same("double sign", b"--5\n");
    assert_same("plus minus", b"+-5\n");
}

/// Matching failure: scanf returns 0, `x` keeps 0, the program still prints 300.
#[test]
fn matching_failure_paths() {
    assert_same("alpha", b"abc\n");
    assert_same("sign only", b"-");
    assert_same("sign only newline", b"-\n");
    assert_same("sign then alpha", b"-x\n");
    assert_same("plus then eof", b"+");
    assert_same("leading dot", b".5\n");
    assert_same("hex prefix", b"0x1f\n");
    assert_same("punctuation", b"!!!\n");
    assert_same("underscore", b"_1\n");
    assert_same("comma", b",5\n");
    assert_same("high byte", &[0xff, 0xfe, b'\n']);
    assert_same("utf8 text", "héllo\n".as_bytes());
}

/// Only the first conversion runs; the rest of the stream is never consumed.
#[test]
fn only_first_number_is_read() {
    assert_same("three numbers", b"1 2 3\n");
    assert_same("numbers on lines", b"1\n2\n3\n");
    assert_same("trailing garbage", b"12abc\n");
    assert_same("digits then punctuation", b"12,34\n");
    assert_same("digits then dot", b"12.75\n");
}

#[test]
fn leading_zeros_are_decimal() {
    assert_same("many leading zeros", b"000000000000000123\n");
    assert_same("all zeros", b"00000000\n");
    assert_same("signed leading zeros", b"-000007\n");
    // Not octal: `%d` is strictly base 10.
    assert_same("looks octal", b"010\n");
}

// ---------------------------------------------------------------------------
// Phase B - integer boundaries, truncation and signedness
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    assert_same("INT_MAX", b"2147483647\n");
    assert_same("INT_MIN", b"-2147483648\n");
    assert_same("INT_MAX-1", b"2147483646\n");
    assert_same("INT_MIN+1", b"-2147483647\n");
}

/// `2*x + 300` overflows as two's-complement wraparound in the compiled C.
#[test]
fn arithmetic_overflow_wraps() {
    // Largest x whose result still fits: 2*1073741673 + 300 = 2147483646.
    assert_same("just below wrap", b"1073741673\n");
    // 2*1073741674 + 300 = 2147483648 -> wraps to INT_MIN.
    assert_same("exact wrap point", b"1073741674\n");
    assert_same("past wrap", b"1073741824\n");
    assert_same("negative wrap point", b"-1073741974\n");
    assert_same("negative past wrap", b"-2000000000\n");
}

/// glibc converts `%d` through a `long` and stores the low 32 bits, so values
/// that exceed `int` are truncated rather than rejected.
#[test]
fn values_beyond_int_are_truncated() {
    assert_same("INT_MAX+1", b"2147483648\n");
    assert_same("2^32", b"4294967296\n");
    assert_same("2^32+1", b"4294967297\n");
    assert_same("2^32+5", b"4294967301\n");
    assert_same("UINT_MAX", b"4294967295\n");
    assert_same("-(2^32)", b"-4294967296\n");
    assert_same("-(2^32+1)", b"-4294967297\n");
    assert_same("2^31 * 3", b"6442450944\n");
}

/// Beyond `LONG_MAX`/`LONG_MIN` glibc saturates, then truncates the saturated
/// value: LONG_MAX -> -1, LONG_MIN -> 0.
#[test]
fn values_beyond_long_saturate() {
    assert_same("LONG_MAX", b"9223372036854775807\n");
    assert_same("LONG_MAX+1", b"9223372036854775808\n");
    assert_same("LONG_MIN", b"-9223372036854775808\n");
    assert_same("LONG_MIN-1", b"-9223372036854775809\n");
    assert_same("huge positive", b"99999999999999999999999\n");
    assert_same("huge negative", b"-99999999999999999999999\n");
}

/// The maximum-sized numeric input the conversion has to walk: a digit run far
/// longer than any accumulator, and the same padded with leading zeros (which
/// do not contribute to overflow).
#[test]
fn very_long_digit_runs() {
    let nines = "9".repeat(2000);
    assert_same("2000 nines", format!("{nines}\n").as_bytes());
    assert_same("2000 nines negative", format!("-{nines}\n").as_bytes());

    let mut zeros_then_value = "0".repeat(5000);
    zeros_then_value.push_str("42\n");
    assert_same("5000 leading zeros", zeros_then_value.as_bytes());

    let ones = "1".repeat(10_000);
    assert_same("10000 ones", format!("{ones}\n").as_bytes());
}

/// A whitespace run large enough to span several stdio buffer refills.
#[test]
fn very_long_whitespace_run() {
    let mut input = " ".repeat(70_000);
    input.push_str("9\n");
    assert_same("70000 spaces then digit", input.as_bytes());

    let newlines = "\n".repeat(70_000);
    assert_same("70000 newlines only", newlines.as_bytes());
}

/// An input far larger than any pipe buffer, where only the head is consumed.
#[test]
fn large_unread_tail() {
    let mut input = b"5\n".to_vec();
    input.extend(std::iter::repeat(b'x').take(1 << 20));
    input.push(b'\n');
    assert_same("1MiB unread tail", &input);
}

// ---------------------------------------------------------------------------
// Phase C - paths not covered above
// ---------------------------------------------------------------------------

/// Embedded NUL bytes are ordinary non-digit bytes to the byte-oriented scan;
/// they are neither whitespace nor terminators.
#[test]
fn embedded_nul_bytes() {
    assert_same("nul then digit", b"\x005\n");
    assert_same("digit then nul", b"5\x006\n");
    assert_same("only nul", b"\x00");
    assert_same("nul run", b"\x00\x00\x00\n");
    assert_same("space nul digit", b" \x00 5\n");
}

/// argv is never inspected by the C program.
#[test]
fn arguments_are_ignored() {
    assert_same_with_args("one arg", &["ignored"], b"4\n");
    assert_same_with_args("many args", &["-h", "--version", "99"], b"4\n");
    assert_same_with_args("empty arg", &[""], b"");
}

/// stdin at immediate EOF from a non-pipe source.
#[test]
fn stdin_from_dev_null() {
    let c = Command::new(c_bin())
        .stdin(Stdio::from(std::fs::File::open("/dev/null").unwrap()))
        .output()
        .unwrap();
    let r = Command::new(rust_bin())
        .stdin(Stdio::from(std::fs::File::open("/dev/null").unwrap()))
        .output()
        .unwrap();
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

/// stdin closed outright: every read fails, which stdio reports like EOF.
#[test]
fn stdin_closed() {
    fn go(exe: PathBuf) -> (Vec<u8>, Vec<u8>, Option<i32>) {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} 0<&-", shell_quote(&exe)))
            .output()
            .unwrap();
        (out.stdout, out.stderr, out.status.code())
    }
    assert_eq!(go(c_bin()), go(rust_bin()));
}

/// stdin is a directory: reads fail with EISDIR rather than returning data.
#[test]
fn stdin_is_a_directory() {
    fn go(exe: PathBuf) -> (Vec<u8>, Vec<u8>, Option<i32>) {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {} < /tmp", shell_quote(&exe)))
            .output()
            .unwrap();
        (out.stdout, out.stderr, out.status.code())
    }
    assert_eq!(go(c_bin()), go(rust_bin()));
}

/// stdout is unwritable (`/dev/full` yields ENOSPC): the C ignores the failed
/// flush and still exits 0, so the Rust must not surface an error either.
#[test]
fn stdout_write_error_is_ignored() {
    fn go(exe: PathBuf) -> (Vec<u8>, Option<i32>) {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("printf '5\\n' | {} > /dev/full", shell_quote(&exe)))
            .output()
            .unwrap();
        (out.stderr, out.status.code())
    }
    let c = go(c_bin());
    let r = go(rust_bin());
    assert_eq!(c.1, r.1, "exit status mismatch on /dev/full: {c:?} vs {r:?}");
    assert_eq!(c.0, r.0, "stderr mismatch on /dev/full");
}

/// stdout is a pipe with no reader. The C process is killed by SIGPIPE; the
/// Rust runtime ignores SIGPIPE by default, so `main` must restore the default
/// disposition or the termination status diverges.
#[test]
fn sigpipe_termination_matches() {
    fn go(exe: PathBuf) -> String {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!(
                // `head -c 0` exits immediately, closing the read end.
                "printf '5\\n' | {} 2>/dev/null | head -c 0; echo ${{PIPESTATUS[1]:-$?}}",
                shell_quote(&exe)
            ))
            .env("POSIXLY_CORRECT", "1")
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }
    // Compare the raw wait status observed by a direct spawn instead of relying
    // on the shell, so this works under any /bin/sh.
    fn spawn_into_closed_pipe(exe: PathBuf) -> Result<i32, i32> {
        let mut child = Command::new(&exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut stdin = child.stdin.take().unwrap();
            let _ = stdin.write_all(b"5\n");
        }
        // Drop the read end of the child's stdout before it writes.
        drop(child.stdout.take());
        // Give the child a moment to reach its write.
        let status = loop {
            match child.try_wait().unwrap() {
                Some(s) => break s,
                None => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        };
        match status.code() {
            Some(c) => Ok(c),
            None => Err(status.signal().unwrap()),
        }
    }

    let _ = go(c_bin());
    let c = spawn_into_closed_pipe(c_bin());
    let r = spawn_into_closed_pipe(rust_bin());
    assert_eq!(
        c, r,
        "termination status mismatch when stdout is a closed pipe: C={c:?} Rust={r:?}"
    );
}

/// Exhaustive sweep over short byte strings drawn from the alphabet the
/// conversion actually distinguishes, to catch ordering differences between the
/// whitespace skip, the sign check and the digit check.
#[test]
fn exhaustive_short_inputs() {
    let alphabet: &[u8] = b" \n\t-+09a.\x00";
    for &a in alphabet {
        assert_same("len1", &[a]);
        for &b in alphabet {
            assert_same("len2", &[a, b]);
            for &c in alphabet {
                assert_same("len3", &[a, b, c]);
            }
        }
    }
}

/// Every result value near the printf boundaries, plus a spread of magnitudes,
/// to confirm `%d` formatting (no padding, no sign for positives, one newline).
#[test]
fn formatting_across_magnitudes() {
    let xs: [i64; 24] = [
        0,
        1,
        -1,
        9,
        -9,
        10,
        -10,
        99,
        -150,
        -151,
        100,
        -100,
        1000,
        -1000,
        12345,
        -12345,
        1_000_000,
        -1_000_000,
        123_456_789,
        -123_456_789,
        2_147_483_647,
        -2_147_483_648,
        1_073_741_673,
        1_073_741_674,
    ];
    for x in xs {
        assert_same(&format!("x={x}"), format!("{x}\n").as_bytes());
        assert_same(&format!("x={x} no newline"), format!("{x}").as_bytes());
    }
}

/// -150 makes the result exactly 0 and -151 makes it negative: the digit-count
/// and sign transitions of the output.
#[test]
fn output_sign_transitions() {
    assert_same("result 0", b"-150\n");
    assert_same("result -2", b"-151\n");
    assert_same("result 2", b"-149\n");
}

fn shell_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', r"'\''"))
}
