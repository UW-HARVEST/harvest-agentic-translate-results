//! Differential tests: run the C binary and the Rust binary as subprocesses on
//! the same stdin and require byte-identical stdout, byte-identical stderr and
//! an identical exit status (including death by signal).
//!
//! The Rust program is never loaded as a library; it is executed exactly the
//! way a shell would run it, because that is how it is compared against the C.

use std::ffi::OsStr;
use std::fmt;
use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating / building the two binaries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // tests/ live in translation/, so the shared root is one level up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C program, configuring and building it on first use so
/// that `cargo test` works from a clean checkout.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run cmake (is it installed?)");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&cfg.stderr)
            );
            let out = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("failed to run cmake --build");
            assert!(
                out.status.success(),
                "cmake build failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        assert!(bin.exists(), "C binary missing at {}", bin.display());
        bin
    })
}

/// Path to the Rust program. Cargo builds it before running integration tests
/// and hands us the location through this environment variable.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// observable behaviour of one run
// ---------------------------------------------------------------------------

/// Everything the two programs are compared on.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(n)` for a normal exit.
    code: Option<i32>,
    /// `Some(n)` when the process was killed by signal `n`.
    signal: Option<i32>,
}

impl fmt::Debug for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Output can contain bytes that are not valid UTF-8 (e.g. `%c` of 0xff),
        // so show it escaped rather than lossily.
        write!(
            f,
            "Outcome {{ code: {:?}, signal: {:?}, stdout: {:?}, stderr: {:?} }}",
            self.code,
            self.signal,
            Escaped(&self.stdout),
            Escaped(&self.stderr)
        )
    }
}

struct Escaped<'a>(&'a [u8]);

impl fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        for &b in self.0 {
            match b {
                b'\n' => f.write_str("\\n")?,
                b'\t' => f.write_str("\\t")?,
                b'\\' => f.write_str("\\\\")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{:02x}", b)?,
            }
        }
        f.write_str("\"")
    }
}

/// Run `prog` with `args`, feeding `stdin` in full, and capture the result.
fn run<S: AsRef<OsStr>>(prog: &Path, args: &[S], stdin: &[u8]) -> Outcome {
    let mut child = Command::new(prog)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", prog.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        // The program reads a single byte, so it may exit while we are still
        // writing. A broken pipe here is expected, not a test failure.
        let _ = sink.write_all(stdin);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("failed to collect output");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Assert the two programs behave identically for `stdin`.
fn assert_same(label: &str, stdin: &[u8]) {
    let c = run(c_binary(), &[] as &[&str], stdin);
    let r = run(rust_binary(), &[] as &[&str], stdin);
    assert_eq!(
        c, r,
        "\nmismatch for {label} (stdin = {:?})\n     C: {:?}\n  Rust: {:?}\n",
        Escaped(stdin),
        c,
        r
    );
}

// ---------------------------------------------------------------------------
// Phase B/C: the inputs the C program branches on
// ---------------------------------------------------------------------------

/// `getchar()` returns EOF, so `char c` becomes -1 and every class lookup uses
/// the negative half of glibc's table.
#[test]
fn empty_input_eof_path() {
    assert_same("empty input (EOF)", b"");
}

/// A single byte: the ordinary path, one case per input.
#[test]
fn single_byte_ascii_letter() {
    assert_same("lowercase 'a'", b"a");
    assert_same("uppercase 'A'", b"A");
    assert_same("digit '0'", b"0");
}

/// Every distinct byte value the program can read. This is the complete input
/// space for the classification branches: upper, lower, alpha, digit, xdigit,
/// space, blank, print, graph, cntrl, punct and both case conversions.
#[test]
fn every_byte_value() {
    for b in 0u8..=255 {
        assert_same(&format!("byte 0x{b:02x}"), &[b]);
    }
}

/// Boundary bytes on either side of every range test in the ctype tables.
/// Redundant with `every_byte_value` but names the branch each one probes, so a
/// failure points straight at the rule that broke.
#[test]
fn class_range_boundaries() {
    let cases: &[(u8, &str)] = &[
        (0x00, "NUL: lowest cntrl, and the byte a C string terminator uses"),
        (0x08, "backspace: cntrl but not space"),
        (0x09, "tab: space and blank"),
        (0x0a, "newline: space, not blank"),
        (0x0b, "vertical tab: last of the \\t..\\r space run"),
        (0x0d, "carriage return: end of the space run"),
        (0x0e, "just past the space run"),
        (0x1f, "last cntrl below space"),
        (0x20, "space: print and blank but not graph"),
        (0x21, "'!': first graph, punct"),
        (0x2f, "'/': punct just below '0'"),
        (0x30, "'0': first digit"),
        (0x39, "'9': last digit"),
        (0x3a, "':': punct just above '9'"),
        (0x40, "'@': punct just below 'A'"),
        (0x41, "'A': first upper, xdigit"),
        (0x46, "'F': last upper xdigit"),
        (0x47, "'G': upper, not xdigit"),
        (0x5a, "'Z': last upper"),
        (0x5b, "'[': punct just above 'Z'"),
        (0x60, "'`': punct just below 'a'"),
        (0x61, "'a': first lower, xdigit"),
        (0x66, "'f': last lower xdigit"),
        (0x67, "'g': lower, not xdigit"),
        (0x7a, "'z': last lower"),
        (0x7b, "'{': punct just above 'z'"),
        (0x7e, "'~': last graph"),
        (0x7f, "DEL: cntrl above graph"),
        (0x80, "first byte that is negative as a signed char"),
        (0xa0, "high byte, no class in the C locale"),
        (0xfe, "high byte just below 0xff"),
        (0xff, "0xff: negative char equal to (char)EOF"),
    ];
    for (b, why) in cases {
        assert_same(&format!("byte 0x{b:02x} ({why})"), &[*b]);
    }
}

/// Only the first byte is consumed; the rest of stdin is ignored. Includes a
/// leading newline, which `getchar` returns as data rather than skipping.
#[test]
fn only_first_byte_is_consumed() {
    assert_same("multi-byte 'abc'", b"abc");
    assert_same("leading newline then text", b"\nxyz");
    assert_same("byte then newline", b"A\n");
    assert_same("leading space then text", b" leading space");
    assert_same("leading NUL then text", b"\0after nul");
    assert_same("high byte then ascii", b"\xffabc");
    assert_same("utf-8 multibyte char", "\u{e9}".as_bytes());
    assert_same("long input", &vec![b'Z'; 100_000]);
}

/// `main` takes no parameters, so arguments cannot change behaviour.
#[test]
fn arguments_are_ignored() {
    for args in [vec!["foo"], vec!["-h"], vec!["a", "b", "c"]] {
        let c = run(c_binary(), &args, b"a");
        let r = run(rust_binary(), &args, b"a");
        assert_eq!(c, r, "mismatch with args {args:?}\n C: {c:?}\n R: {r:?}");
    }
}

/// Output is written raw, so a byte such as 0xff appears verbatim and stdout is
/// not valid UTF-8. Guards against the Rust side encoding `%c` as UTF-8.
#[test]
fn high_byte_output_is_raw_not_utf8() {
    let c = run(c_binary(), &[] as &[&str], b"\xff");
    let r = run(rust_binary(), &[] as &[&str], b"\xff");
    assert_eq!(c, r);
    assert!(
        c.stdout.ends_with(b"to upper: \xff\n"),
        "expected a raw 0xff byte from %c, got {:?}",
        Escaped(&c.stdout)
    );
    assert!(
        String::from_utf8(c.stdout.clone()).is_err(),
        "stdout should not be valid UTF-8 for input 0xff"
    );
}

// ---------------------------------------------------------------------------
// Phase C: stdin and stdout conditions that are not byte values
// ---------------------------------------------------------------------------

extern "C" {
    fn close(fd: i32) -> i32;
    /// `pipe2` rather than `pipe`: without `O_CLOEXEC` the child inherits the
    /// read end of its own stdout pipe and can never see a broken pipe.
    fn pipe2(fds: *mut i32, flags: i32) -> i32;
}

/// Linux `O_CLOEXEC`.
const O_CLOEXEC: i32 = 0o2_000_000;

/// Run `prog` with file descriptor 0 closed, so the read fails with `EBADF`
/// rather than returning data or a clean EOF.
fn run_with_closed_stdin(prog: &Path) -> Outcome {
    use std::os::unix::process::CommandExt;
    let mut cmd = Command::new(prog);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(|| {
            if close(0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let out = cmd.output().expect("failed to run with closed stdin");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// A failed read must be treated the same as EOF, exactly as `getchar` does.
#[test]
fn closed_stdin_read_error_path() {
    let c = run_with_closed_stdin(c_binary());
    let r = run_with_closed_stdin(rust_binary());
    assert_eq!(c, r, "mismatch with fd 0 closed\n C: {c:?}\n R: {r:?}");
}

/// Reading from a directory fails with `EISDIR`, another non-EOF read failure.
#[test]
fn stdin_is_a_directory_read_error_path() {
    let dir = std::fs::File::open(workspace_root()).expect("cannot open workspace root");
    let dir2 = dir.try_clone().expect("cannot clone dir handle");

    let out_c = Command::new(c_binary())
        .stdin(Stdio::from(dir))
        .output()
        .expect("failed to run C binary");
    let out_r = Command::new(rust_binary())
        .stdin(Stdio::from(dir2))
        .output()
        .expect("failed to run Rust binary");

    let c = Outcome {
        stdout: out_c.stdout,
        stderr: out_c.stderr,
        code: out_c.status.code(),
        signal: out_c.status.signal(),
    };
    let r = Outcome {
        stdout: out_r.stdout,
        stderr: out_r.stderr,
        code: out_r.status.code(),
        signal: out_r.status.signal(),
    };
    assert_eq!(c, r, "mismatch with a directory as stdin\n C: {c:?}\n R: {r:?}");
}

/// Give the child a stdout pipe whose read end is already closed, then unblock
/// it by writing its input byte. Its first write must hit a broken pipe.
///
/// Ordering matters: the child cannot produce output until it has read a byte,
/// so closing the read end before writing to stdin makes this deterministic.
fn run_with_broken_stdout(prog: &Path) -> Outcome {
    let mut fds = [-1i32; 2];
    assert_eq!(
        unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) },
        0,
        "pipe2() failed"
    );
    let (read_end, write_end) = (fds[0], fds[1]);

    // `dup2` onto fd 1 clears O_CLOEXEC, so the child still gets a usable stdout.
    let stdout = unsafe { Stdio::from_raw_fd(write_end) };
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(stdout)
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn with a pipe for stdout");

    // Drop the last reader while the child is still blocked on stdin.
    assert_eq!(unsafe { close(read_end) }, 0, "close(read end) failed");

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        let _ = sink.write_all(b"a");
    }

    let out = child.wait_with_output().expect("failed to collect output");
    Outcome {
        stdout: out.stdout, // empty: stdout was not captured
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// The C program keeps the default `SIGPIPE`, so it dies from signal 13. The
/// Rust runtime ignores `SIGPIPE` by default, which would turn this into a
/// clean exit 0 unless the translation restores the default disposition.
#[test]
fn broken_stdout_pipe_exit_status() {
    let c = run_with_broken_stdout(c_binary());
    let r = run_with_broken_stdout(rust_binary());
    assert_eq!(c, r, "mismatch on a broken stdout pipe\n C: {c:?}\n R: {r:?}");
    assert_eq!(
        c.signal,
        Some(13),
        "expected the C program to die from SIGPIPE, got {c:?}"
    );
}

/// Closing fd 1 outright makes every write fail with `EBADF`, which C's stdio
/// swallows: the process still exits 0 with no diagnostic.
#[test]
fn closed_stdout_is_silently_ignored() {
    use std::os::unix::process::CommandExt;
    let outcome = |prog: &Path| {
        let mut cmd = Command::new(prog);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                if close(1) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = cmd.spawn().expect("failed to spawn with fd 1 closed");
        {
            let mut sink = child.stdin.take().expect("stdin was piped");
            let _ = sink.write_all(b"a");
        }
        let out = child.wait_with_output().expect("failed to collect output");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };
    let c = outcome(c_binary());
    let r = outcome(rust_binary());
    assert_eq!(c, r, "mismatch with fd 1 closed\n C: {c:?}\n R: {r:?}");
}

/// Redirecting stdout to a file changes stdio buffering (fully buffered instead
/// of line buffered); the bytes on disk must still be identical.
#[test]
fn stdout_to_file_matches() {
    let dir = std::env::temp_dir();
    let read_back = |prog: &Path, tag: &str| -> Vec<u8> {
        let path = dir.join(format!("driver_diff_{tag}_{}.out", std::process::id()));
        let file = std::fs::File::create(&path).expect("cannot create temp file");
        let mut child = Command::new(prog)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(file))
            .spawn()
            .expect("failed to spawn with a file for stdout");
        {
            let mut sink = child.stdin.take().expect("stdin was piped");
            let _ = sink.write_all(b"Q");
        }
        assert!(child.wait().expect("wait failed").success());
        let bytes = std::fs::read(&path).expect("cannot read temp file");
        let _ = std::fs::remove_file(&path);
        bytes
    };
    assert_eq!(read_back(c_binary(), "c"), read_back(rust_binary(), "rust"));
}
