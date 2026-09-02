//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical stdin and require byte-identical stdout, stderr
//! and an identical exit status (including termination by signal).
//!
//! The Rust code is never called as a library; only the built `driver` binary is
//! driven, the same way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Everything an observer of the process can see.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some` when the process exited normally.
    code: Option<i32>,
    /// `Some` when the process was killed by a signal.
    signal: Option<i32>,
}

impl Outcome {
    fn describe(&self) -> String {
        format!(
            "stdout={:?} stderr={:?} code={:?} signal={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.code,
            self.signal
        )
    }
}

fn workspace_root() -> PathBuf {
    // tests/ lives in translation/, whose parent holds c_src/ and translation/.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test, provided by cargo.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, built with cmake on first use.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build_dir = c_src.join("build");
        let exe = build_dir.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build_dir).expect("create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build_dir)
                .output()
                .expect("run cmake (is cmake installed?)");
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
                .expect("run cmake --build");
            assert!(
                build.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
        }
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
    .as_path()
}

fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        None
    }
}

/// Runs `exe` with `input` on stdin, capturing stdout, stderr and the status.
fn run(exe: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        // A closed stdout on the peer side is not expected here, but do not let
        // a write error abort the test harness.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: signal_of(&out.status),
    }
}

/// The core assertion: both programs behave identically for `input`.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_binary(), input);
    let r = run(rust_binary(), input);
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {:?}):\n  C:    {}\n  Rust: {}",
        String::from_utf8_lossy(input),
        c.describe(),
        r.describe()
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {:?}):\n  C:    {}\n  Rust: {}",
        String::from_utf8_lossy(input),
        c.describe(),
        r.describe()
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status mismatch for {label} (input {:?}):\n  C:    {}\n  Rust: {}",
        String::from_utf8_lossy(input),
        c.describe(),
        r.describe()
    );
}

#[track_caller]
fn assert_same_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Input classes the C program branches on.
//
// main() has exactly one branch point: `scanf("%d", &x)` either converts a
// value or fails (input failure at EOF / read error, or matching failure on a
// non-numeric byte), in which case `x` keeps its initializer 0. driver() then
// computes `2*x + 300` with C's wrapping int arithmetic and prints it with
// "%d\n".
// ---------------------------------------------------------------------------

/// scanf input failure: nothing at all to read. x stays 0 -> "300".
#[test]
fn empty_input() {
    assert_same("empty", b"");
}

/// scanf input failure: only whitespace, so the skip loop hits EOF.
#[test]
fn whitespace_only_input() {
    assert_same_all(&[
        ("single space", b" "),
        ("single newline", b"\n"),
        ("single tab", b"\t"),
        ("carriage return", b"\r"),
        ("vertical tab", b"\x0b"),
        ("form feed", b"\x0c"),
        ("all C whitespace", b" \t\n\x0b\x0c\r"),
        ("many newlines", b"\n\n\n\n\n"),
        ("long run of spaces", b"                                        "),
    ]);
}

/// scanf matching failure: first non-space byte cannot start an integer.
#[test]
fn matching_failure_non_numeric() {
    assert_same_all(&[
        ("letters", b"abc"),
        ("uppercase", b"XYZ"),
        ("leading dot", b".5"),
        ("comma", b",5"),
        ("slash", b"/5"),
        ("hash", b"#"),
        ("underscore", b"_1"),
        ("leading space then letter", b"   q"),
        ("newline then letter", b"\nz9"),
        ("nul byte first", b"\x005"),
        ("high byte first", b"\xff\xfe5"),
        ("utf8 multibyte first", "é7".as_bytes()),
    ]);
}

/// A sign with no digits after it is also a matching failure.
#[test]
fn matching_failure_sign_without_digits() {
    assert_same_all(&[
        ("lone minus", b"-"),
        ("lone plus", b"+"),
        ("minus then EOF after space", b"-\n"),
        ("minus space digit", b"- 5"),
        ("plus space digit", b"+ 5"),
        ("double minus", b"--5"),
        ("plus minus", b"+-5"),
        ("minus letter", b"-a"),
        ("minus dot", b"-."),
    ]);
}

/// The single-item happy path, positive and negative.
#[test]
fn single_value() {
    assert_same_all(&[
        ("zero", b"0"),
        ("negative zero", b"-0"),
        ("plus zero", b"+0"),
        ("one", b"1"),
        ("five", b"5"),
        ("negative five", b"-5"),
        ("plus seven", b"+7"),
        ("with trailing newline", b"5\n"),
        ("with CRLF", b"5\r\n"),
        ("three digits", b"123"),
        ("leading zeros", b"000000007"),
        ("many leading zeros", b"0000000000000000000000000042"),
    ]);
}

/// scanf skips leading whitespace, including across newlines (unlike fgets).
#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    assert_same_all(&[
        ("spaces then value", b"    12"),
        ("newlines then value", b"\n\n\n\n42"),
        ("mixed whitespace then value", b" \t\n\x0b\x0c\r 9"),
        ("newline before sign", b"\n-13"),
        ("tabs before sign", b"\t\t+13"),
        ("crlf then value", b"\r\n8"),
    ]);
}

/// The conversion stops at the first non-digit and the rest is never read.
#[test]
fn conversion_stops_at_first_non_digit() {
    assert_same_all(&[
        ("digits then letters", b"7abc"),
        ("digits then dot", b"7.9"),
        ("exponent form", b"1e5"),
        ("hex form reads only 0", b"0x10"),
        ("second number ignored", b"5 6"),
        ("many numbers, only first", b"11 22 33 44\n55\n"),
        ("value then nul then value", b"5\x006"),
        ("value then high byte", b"5\xff"),
        ("negative then junk", b"-5junk"),
        ("digits then minus", b"5-6"),
    ]);
}

/// int wrap-around in `2*x`, exactly as the C performs it.
#[test]
fn int_overflow_in_doubling() {
    assert_same_all(&[
        ("INT_MAX", b"2147483647"),
        ("INT_MAX-1", b"2147483646"),
        ("INT_MIN", b"-2147483648"),
        ("INT_MIN+1", b"-2147483647"),
        ("half INT_MAX", b"1073741823"),
        ("just over half", b"1073741824"),
        ("just under negative half", b"-1073741825"),
        ("boundary for +300 wrap", b"1073741674"),
        ("negative 150", b"-150"),
        ("negative 149", b"-149"),
        ("negative 151", b"-151"),
    ]);
}

/// Values beyond int range: glibc converts in long range and the store
/// truncates to int.
#[test]
fn truncation_of_out_of_int_range_values() {
    assert_same_all(&[
        ("INT_MAX+1", b"2147483648"),
        ("INT_MIN-1", b"-2147483649"),
        ("UINT_MAX", b"4294967295"),
        ("2^32", b"4294967296"),
        ("2^32+1", b"4294967297"),
        ("3*2^32", b"12884901888"),
        ("negative UINT_MAX", b"-4294967295"),
        ("negative 2^32", b"-4294967296"),
    ]);
}

/// Values beyond long range: glibc saturates at LONG_MAX / LONG_MIN before the
/// truncation to int.
#[test]
fn saturation_beyond_long_range() {
    assert_same_all(&[
        ("LONG_MAX", b"9223372036854775807"),
        ("LONG_MAX+1", b"9223372036854775808"),
        ("LONG_MAX+2", b"9223372036854775809"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MIN-1", b"-9223372036854775809"),
        ("2^64", b"18446744073709551616"),
        ("2^64+7", b"18446744073709551623"),
        ("1e30", b"1000000000000000000000000000000"),
        ("-1e30", b"-1000000000000000000000000000000"),
        ("leading zeros then huge", b"000000009223372036854775808"),
    ]);
}

/// The largest input the conversion has to cope with: a very long digit run.
#[test]
fn very_long_digit_runs() {
    let mut nines = vec![b'9'; 100_000];
    assert_same("100k nines", &nines);

    nines.insert(0, b'-');
    assert_same("100k nines negated", &nines);

    let mut zeros = vec![b'0'; 50_000];
    zeros.extend_from_slice(b"123");
    assert_same("50k zeros then 123", &zeros);

    let mut huge_line = vec![b' '; 10_000];
    huge_line.extend_from_slice(b"2147483647");
    huge_line.extend(std::iter::repeat(b'\n').take(10_000));
    assert_same("padded INT_MAX", &huge_line);
}

/// stdin that cannot be read at all (a directory) is a scanf input failure.
#[cfg(unix)]
#[test]
fn unreadable_stdin_is_input_failure() {
    fn run_with_stdin_dir(exe: &Path) -> Outcome {
        let dir = std::fs::File::open(workspace_root()).expect("open workspace dir");
        let out = Command::new(exe)
            .stdin(Stdio::from(dir))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with directory stdin");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: signal_of(&out.status),
        }
    }
    let c = run_with_stdin_dir(c_binary());
    let r = run_with_stdin_dir(rust_binary());
    assert_eq!(c, r, "C: {}\nRust: {}", c.describe(), r.describe());
}

/// main() ignores argv; extra arguments must not change anything.
#[test]
fn command_line_arguments_are_ignored() {
    fn run_with_args(exe: &Path, args: &[&str]) -> Outcome {
        let mut child = Command::new(exe)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"5\n")
            .expect("write stdin");
        let out = child.wait_with_output().expect("wait");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: signal_of(&out.status),
        }
    }
    let args = ["foo", "-h", "--version", "42"];
    let c = run_with_args(c_binary(), &args);
    let r = run_with_args(rust_binary(), &args);
    assert_eq!(c, r, "C: {}\nRust: {}", c.describe(), r.describe());
}

/// A stdout that fails every write. C's `printf` return value is ignored, so
/// the failure is silent and the process still exits 0.
#[cfg(unix)]
#[test]
fn failing_stdout_is_silent() {
    let dev_full = Path::new("/dev/full");
    if !dev_full.exists() {
        // Nothing to compare on a system without /dev/full; the remaining tests
        // still cover every source branch.
        return;
    }
    fn run_to_dev_full(exe: &Path) -> Outcome {
        let sink = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("open /dev/full");
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(sink))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"5\n")
            .expect("write stdin");
        let out = child.wait_with_output().expect("wait");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: signal_of(&out.status),
        }
    }
    let c = run_to_dev_full(c_binary());
    let r = run_to_dev_full(rust_binary());
    assert_eq!(c, r, "C: {}\nRust: {}", c.describe(), r.describe());
}

/// Writing to a pipe whose reader is gone: a C program inherits the default
/// SIGPIPE disposition and is killed by signal 13.
#[cfg(unix)]
#[test]
fn broken_stdout_pipe_matches() {
    use std::os::fd::FromRawFd;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    fn run_with_broken_stdout(exe: &Path) -> Outcome {
        let mut fds = [-1i32; 2];
        // SAFETY: `fds` is a valid two-element array for pipe(2) to fill in.
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe(2) failed");
        let (read_fd, write_fd) = (fds[0], fds[1]);

        // SAFETY: `write_fd` is a fresh, owned descriptor; Stdio takes ownership
        // and closes the parent's copy once the child has been spawned.
        let stdout = unsafe { Stdio::from_raw_fd(write_fd) };
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");

        // Drop the read end so the child's write hits a broken pipe. The child
        // is still blocked reading stdin at this point.
        // SAFETY: `read_fd` is owned here and not used again.
        unsafe { close(read_fd) };

        let _ = child.stdin.take().unwrap().write_all(b"5\n");
        let out = child.wait_with_output().expect("wait");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: signal_of(&out.status),
        }
    }

    let c = run_with_broken_stdout(c_binary());
    let r = run_with_broken_stdout(rust_binary());
    assert_eq!(c, r, "C: {}\nRust: {}", c.describe(), r.describe());
}

/// Deterministic sweep over generated byte strings, to catch classes the hand
/// written cases missed. The generator is a fixed LCG, so failures reproduce.
#[test]
fn deterministic_random_sweep() {
    const ALPHABET: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabcXYZ.\x00\xff/e";
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = |bound: usize| -> usize {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 33) as usize) % bound
    };

    for i in 0..1200 {
        let len = next(15);
        let input: Vec<u8> = (0..len).map(|_| ALPHABET[next(ALPHABET.len())]).collect();
        assert_same(&format!("sweep #{i}"), &input);
    }

    for i in 0..300 {
        let len = 1 + next(40);
        let mut input: Vec<u8> = Vec::new();
        match next(5) {
            0 => input.push(b'-'),
            1 => input.push(b'+'),
            2 => input.extend_from_slice(b"  "),
            3 => input.extend_from_slice(b"000"),
            _ => {}
        }
        input.extend((0..len).map(|_| b"0123456789"[next(10)]));
        assert_same(&format!("numeric sweep #{i}"), &input);
    }
}
