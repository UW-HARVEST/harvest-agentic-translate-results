//! Differential tests: run the C binary and the Rust binary as subprocesses and
//! compare stdout, stderr and exit status byte for byte / value for value.
//!
//! The Rust program is NEVER linked as a library here. It is executed the way a
//! shell would execute it, because that is how it is graded against the C.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::fd::FromRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test, as built by cargo for this test run.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with cmake on first use if necessary.
///
/// `c_src/` is treated as read-only ground truth; only the out-of-source
/// `c_src/build/` directory is created.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build_dir = c_src.join("build");
        let exe = build_dir.join("driver");
        if exe.exists() {
            return exe;
        }

        std::fs::create_dir_all(&build_dir).expect("create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake ..` — is cmake installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr),
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
            String::from_utf8_lossy(&build.stderr),
        );

        assert!(exe.exists(), "C binary missing after build: {}", exe.display());
        exe
    })
}

// ---------------------------------------------------------------------------
// Running a program and capturing everything observable
// ---------------------------------------------------------------------------

/// How stdin should be wired up for a case.
#[derive(Clone)]
enum StdinMode {
    /// /dev/null-like: immediate EOF.
    Empty,
    /// Feed these exact bytes on stdin.
    Bytes(Vec<u8>),
    /// File descriptor 0 is closed before exec.
    Closed,
}

/// How stdout should be wired up for a case.
#[derive(Clone, Copy, PartialEq)]
enum StdoutMode {
    /// Captured through a pipe (the normal case).
    Capture,
    /// File descriptor 1 is closed before exec: every write fails with EBADF.
    Closed,
    /// Redirected to /dev/full: every write fails with ENOSPC.
    DevFull,
    /// Redirected to the write end of a pipe whose read end is already closed:
    /// the first write raises SIGPIPE (or fails with EPIPE if it is ignored).
    NoReaderPipe,
}

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// Exit code, or None if terminated by a signal.
    code: Option<i32>,
    /// Terminating signal, or None if exited normally.
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stdout_bytes", &self.stdout)
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

fn run(exe: &Path, args: &[OsString], stdin: &StdinMode, stdout_mode: StdoutMode) -> Outcome {
    let mut cmd = Command::new(exe);
    cmd.args(args);
    cmd.stderr(Stdio::piped());

    match stdout_mode {
        StdoutMode::Capture => {
            cmd.stdout(Stdio::piped());
        }
        StdoutMode::Closed => {
            cmd.stdout(Stdio::null());
            // SAFETY: close(2) is async-signal-safe and legal between fork and exec.
            unsafe {
                cmd.pre_exec(|| {
                    libc_close(1);
                    Ok(())
                });
            }
        }
        StdoutMode::DevFull => {
            let full = std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .expect("open /dev/full");
            cmd.stdout(Stdio::from(full));
        }
        StdoutMode::NoReaderPipe => {
            let (read_end, write_end) = libc_pipe();
            // Close the read end now: the child will write into a pipe that has
            // no readers at all, which is deterministic rather than a race.
            libc_close(read_end);
            // SAFETY: `write_end` is a fresh, owned descriptor from pipe(2).
            let w = unsafe { std::fs::File::from_raw_fd(write_end) };
            cmd.stdout(Stdio::from(w));
        }
    }

    match stdin {
        StdinMode::Empty => {
            cmd.stdin(Stdio::null());
        }
        StdinMode::Bytes(_) => {
            cmd.stdin(Stdio::piped());
        }
        StdinMode::Closed => {
            cmd.stdin(Stdio::null());
            // SAFETY: as above.
            unsafe {
                cmd.pre_exec(|| {
                    libc_close(0);
                    Ok(())
                });
            }
        }
    }

    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    if let StdinMode::Bytes(bytes) = stdin {
        let mut si = child.stdin.take().expect("piped stdin");
        // The programs never read stdin, so the write may fail once the child
        // exits; that is not an error for the purposes of this comparison.
        let _ = si.write_all(bytes);
        drop(si);
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Minimal `close(2)` shim so the crate needs no external dependencies.
fn libc_close(fd: i32) {
    unsafe {
        extern "C" {
            fn close(fd: i32) -> i32;
        }
        let _ = close(fd);
    }
}

/// Minimal `pipe(2)` shim; returns `(read_end, write_end)`.
fn libc_pipe() -> (i32, i32) {
    let mut fds = [-1i32; 2];
    // SAFETY: `fds` is a valid two-element array of C ints.
    let rc = unsafe {
        extern "C" {
            fn pipe(fds: *mut i32) -> i32;
        }
        pipe(fds.as_mut_ptr())
    };
    assert_eq!(rc, 0, "pipe(2) failed");
    (fds[0], fds[1])
}

// ---------------------------------------------------------------------------
// The core assertion
// ---------------------------------------------------------------------------

fn assert_same(case: &str, args: &[OsString], stdin: StdinMode, stdout_mode: StdoutMode) -> Outcome {
    let c = run(c_bin(), args, &stdin, stdout_mode);
    let r = run(&rust_bin(), args, &stdin, stdout_mode);

    assert_eq!(
        c.stdout, r.stdout,
        "[{case}] stdout differs\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "[{case}] stderr differs\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(c.code, r.code, "[{case}] exit code differs");
    assert_eq!(c.signal, r.signal, "[{case}] terminating signal differs");

    c
}

fn args<I, S>(items: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    items.into_iter().map(|s| s.as_ref().to_os_string()).collect()
}

const NO_ARGS: &[OsString] = &[];

// ---------------------------------------------------------------------------
// Phase A — both binaries exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = run(c_bin(), NO_ARGS, &StdinMode::Empty, StdoutMode::Capture);
    let r = run(&rust_bin(), NO_ARGS, &StdinMode::Empty, StdoutMode::Capture);
    assert!(!c.stdout.is_empty(), "C produced no stdout");
    assert!(!r.stdout.is_empty(), "Rust produced no stdout");
}

// ---------------------------------------------------------------------------
// Phase B — the golden output, pinning every branch main() actually takes
// ---------------------------------------------------------------------------

/// The exact bytes the C program emits. This pins the intentional defect in
/// `bad()`: `intOne + intTwo;` is discarded, so `intSum` prints 0 twice, while
/// `good()` prints 0 then 2.
const EXPECTED_STDOUT: &[u8] =
    b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n";

#[test]
fn c_output_matches_expected_bytes() {
    let c = run(c_bin(), NO_ARGS, &StdinMode::Empty, StdoutMode::Capture);
    assert_eq!(
        c.stdout, EXPECTED_STDOUT,
        "C ground truth changed unexpectedly:\n{}",
        String::from_utf8_lossy(&c.stdout)
    );
    assert_eq!(c.stderr, b"", "C wrote to stderr");
    assert_eq!(c.code, Some(0));
}

#[test]
fn no_args_matches() {
    let c = assert_same("no args", NO_ARGS, StdinMode::Empty, StdoutMode::Capture);
    assert_eq!(c.stdout, EXPECTED_STDOUT);
    assert_eq!(c.stderr, b"");
    assert_eq!(c.code, Some(0));
    assert_eq!(c.signal, None);
}

#[test]
fn good_and_bad_sections_are_distinguished() {
    // Guards against a translation that "fixes" bad() into good(): the two
    // halves of the output must not be identical.
    let out = run(&rust_bin(), NO_ARGS, &StdinMode::Empty, StdoutMode::Capture).stdout;
    let text = String::from_utf8(out).expect("utf-8 output");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        vec![
            "Calling good()...",
            "0",
            "2",
            "Finished good()",
            "Calling bad()...",
            "0",
            "0",
            "Finished bad()",
        ]
    );
    assert!(text.ends_with('\n'), "output must end with a trailing newline");
}

// ---------------------------------------------------------------------------
// Phase B/C — argv input classes. `main` ignores argc/argv entirely, so every
// one of these must produce the identical result; a translation that parsed
// arguments or indexed argv[1] would diverge here.
// ---------------------------------------------------------------------------

#[test]
fn single_arg_matches() {
    assert_same("single arg", &args(["item"]), StdinMode::Empty, StdoutMode::Capture);
}

#[test]
fn empty_string_arg_matches() {
    assert_same("empty arg", &args([""]), StdinMode::Empty, StdoutMode::Capture);
}

#[test]
fn flag_like_args_match() {
    for a in ["--help", "-h", "-v", "--version", "-", "--", "-0", "--bad", "--good"] {
        assert_same(a, &args([a]), StdinMode::Empty, StdoutMode::Capture);
    }
}

#[test]
fn many_args_match() {
    let many: Vec<String> = (0..256).map(|i| format!("arg{i}")).collect();
    assert_same("256 args", &args(&many), StdinMode::Empty, StdoutMode::Capture);
}

#[test]
fn arg_with_embedded_newline_matches() {
    assert_same(
        "newline in arg",
        &args(["first\nsecond\n"]),
        StdinMode::Empty,
        StdoutMode::Capture,
    );
}

#[test]
fn non_utf8_arg_matches() {
    // Invalid UTF-8 in argv: a Rust translation using `String`-based arg APIs
    // (e.g. `std::env::args()`) would panic here where the C is unaffected.
    let raw = OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x00_u8.wrapping_add(0x41)]).to_os_string();
    assert_same("non-utf8 arg", &[raw], StdinMode::Empty, StdoutMode::Capture);
}

#[test]
fn very_long_arg_matches() {
    let long = "x".repeat(100_000);
    assert_same("100k-byte arg", &args([long]), StdinMode::Empty, StdoutMode::Capture);
}

// ---------------------------------------------------------------------------
// Phase C — stdin input classes. The C reads nothing from stdin (no scanf, no
// fgets, no getchar), so stdin content must never change the output, and the
// stream must be left unconsumed.
// ---------------------------------------------------------------------------

#[test]
fn empty_stdin_matches() {
    assert_same("empty stdin", NO_ARGS, StdinMode::Empty, StdoutMode::Capture);
}

#[test]
fn single_line_stdin_matches() {
    assert_same(
        "one line on stdin",
        NO_ARGS,
        StdinMode::Bytes(b"1\n".to_vec()),
        StdoutMode::Capture,
    );
}

#[test]
fn multi_line_stdin_matches() {
    assert_same(
        "many lines on stdin",
        NO_ARGS,
        StdinMode::Bytes(b"1\n2\n3\n4\n5\n".to_vec()),
        StdoutMode::Capture,
    );
}

#[test]
fn stdin_without_trailing_newline_matches() {
    assert_same(
        "no trailing newline",
        NO_ARGS,
        StdinMode::Bytes(b"42".to_vec()),
        StdoutMode::Capture,
    );
}

#[test]
fn non_numeric_stdin_matches() {
    assert_same(
        "non-numeric stdin",
        NO_ARGS,
        StdinMode::Bytes(b"not a number at all\n".to_vec()),
        StdoutMode::Capture,
    );
}

#[test]
fn integer_overflow_shaped_stdin_matches() {
    // Values that would overflow / wrap a C `int` if they were ever parsed.
    let payload = b"2147483647\n-2147483648\n2147483648\n99999999999999999999\n".to_vec();
    assert_same(
        "overflow-shaped stdin",
        NO_ARGS,
        StdinMode::Bytes(payload),
        StdoutMode::Capture,
    );
}

#[test]
fn binary_stdin_with_nuls_matches() {
    let mut payload = Vec::new();
    for b in 0u16..=255 {
        payload.push(b as u8);
    }
    assert_same(
        "binary stdin",
        NO_ARGS,
        StdinMode::Bytes(payload),
        StdoutMode::Capture,
    );
}

#[test]
fn large_stdin_matches() {
    // Larger than a pipe buffer, so the writer would block if either program
    // failed to exit without draining stdin.
    let payload = vec![b'A'; 1 << 20];
    assert_same("1MiB stdin", NO_ARGS, StdinMode::Bytes(payload), StdoutMode::Capture);
}

#[test]
fn closed_stdin_matches() {
    assert_same("fd 0 closed", NO_ARGS, StdinMode::Closed, StdoutMode::Capture);
}

#[test]
fn neither_program_consumes_stdin() {
    // Run the program with a shared stdin, then read what is left: both must
    // leave every byte in place.
    fn leftover(exe: &Path) -> Vec<u8> {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "c2rust-stdin-{}-{}.txt",
            std::process::id(),
            exe.file_name().unwrap().to_string_lossy()
        ));
        std::fs::write(&path, b"LINE1\nLINE2\nLINE3\n").expect("write temp input");

        let script = format!(
            "'{}' > /dev/null 2>&1; cat",
            exe.display().to_string().replace('\'', "'\\''")
        );
        let out = Command::new("sh")
            .arg("-c")
            .arg(script)
            .stdin(Stdio::from(std::fs::File::open(&path).expect("open temp input")))
            .output()
            .expect("run under sh");
        let _ = std::fs::remove_file(&path);
        out.stdout
    }

    let c = leftover(c_bin());
    let r = leftover(&rust_bin());
    assert_eq!(c, b"LINE1\nLINE2\nLINE3\n", "C consumed part of stdin");
    assert_eq!(c, r, "stdin consumption differs between C and Rust");
}

// ---------------------------------------------------------------------------
// Phase C — write-failure paths. Neither program checks printf's return value,
// so a failing stdout must still yield exit status 0 with no stderr output and
// no signal death. A Rust translation using `println!` would panic (exit 101)
// here, and one that let SIGPIPE stay at its default would die by signal.
// ---------------------------------------------------------------------------

#[test]
fn stdout_closed_matches() {
    let c = assert_same("fd 1 closed", NO_ARGS, StdinMode::Empty, StdoutMode::Closed);
    assert_eq!(c.code, Some(0), "C must still exit 0 with stdout closed");
    assert_eq!(c.signal, None);
    assert_eq!(c.stderr, b"", "no panic message may be printed");
}

#[test]
fn stdout_write_error_matches() {
    if !Path::new("/dev/full").exists() {
        return;
    }
    let c = assert_same("stdout=/dev/full", NO_ARGS, StdinMode::Empty, StdoutMode::DevFull);
    assert_eq!(c.code, Some(0), "C must still exit 0 when writes fail");
    assert_eq!(c.signal, None);
    assert_eq!(c.stderr, b"");
}

#[test]
fn stdout_pipe_with_no_reader_matches() {
    // Deterministic broken-pipe case: the read end is closed before the child is
    // even spawned, so the child's first write hits a pipe with zero readers.
    // The C program inherits SIG_DFL for SIGPIPE and dies with signal 13; a Rust
    // program that left the runtime's SIG_IGN in place would exit 0 instead.
    let c = assert_same(
        "stdout pipe with no reader",
        NO_ARGS,
        StdinMode::Empty,
        StdoutMode::NoReaderPipe,
    );
    assert_eq!(c.signal, Some(13), "C must die with SIGPIPE");
    assert_eq!(c.code, None);
    assert_eq!(c.stderr, b"", "no panic or error message may be printed");
}

// ---------------------------------------------------------------------------
// Phase C — environment and invocation shape must not matter
// ---------------------------------------------------------------------------

#[test]
fn empty_environment_matches() {
    fn run_env_cleared(exe: &Path) -> Outcome {
        let out = Command::new(exe)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn with cleared env");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    }
    assert_eq!(run_env_cleared(c_bin()), run_env_cleared(&rust_bin()));
}

#[test]
fn locale_does_not_change_number_formatting() {
    // A locale with a different numeric separator must not alter "%d" output.
    for loc in ["C", "de_DE.UTF-8", "en_US.UTF-8", "invalid.locale"] {
        fn run_locale(exe: &Path, loc: &str) -> Outcome {
            let out = Command::new(exe)
                .env("LC_ALL", loc)
                .env("LANG", loc)
                .env("LC_NUMERIC", loc)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("spawn with locale");
            Outcome {
                stdout: out.stdout,
                stderr: out.stderr,
                code: out.status.code(),
                signal: out.status.signal(),
            }
        }
        let c = run_locale(c_bin(), loc);
        let r = run_locale(&rust_bin(), loc);
        assert_eq!(c, r, "locale {loc} changed behavior");
        assert_eq!(c.stdout, EXPECTED_STDOUT, "locale {loc} changed C output");
    }
}

#[test]
fn output_is_deterministic_across_runs() {
    let mut seen: Option<Outcome> = None;
    for _ in 0..20 {
        let c = run(c_bin(), NO_ARGS, &StdinMode::Empty, StdoutMode::Capture);
        let r = run(&rust_bin(), NO_ARGS, &StdinMode::Empty, StdoutMode::Capture);
        assert_eq!(c, r, "run-to-run divergence");
        match &seen {
            None => seen = Some(c),
            Some(prev) => assert_eq!(*prev, c, "C output is not deterministic"),
        }
    }
}

#[test]
fn stdout_redirected_to_file_matches() {
    // A file (not a pipe) changes libc's buffering mode; the byte stream must
    // be unchanged.
    fn to_file(exe: &Path, tag: &str) -> Vec<u8> {
        let path = std::env::temp_dir().join(format!("c2rust-out-{}-{tag}.txt", std::process::id()));
        let f = std::fs::File::create(&path).expect("create temp out");
        let status = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::from(f))
            .stderr(Stdio::null())
            .status()
            .expect("run to file");
        assert_eq!(status.code(), Some(0));
        let bytes = std::fs::read(&path).expect("read temp out");
        let _ = std::fs::remove_file(&path);
        bytes
    }
    let c = to_file(c_bin(), "c");
    let r = to_file(&rust_bin(), "r");
    assert_eq!(c, r);
    assert_eq!(c, EXPECTED_STDOUT);
}

/// The graded artifact is the release binary; make sure it matches too, not
/// just the debug binary cargo builds for this test.
#[test]
fn release_binary_matches_when_present() {
    let release = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("driver");
    if !release.exists() {
        return; // `cargo build --release` has not been run; nothing to check.
    }
    let c = run(c_bin(), NO_ARGS, &StdinMode::Empty, StdoutMode::Capture);
    let r = run(&release, NO_ARGS, &StdinMode::Empty, StdoutMode::Capture);
    assert_eq!(c, r, "release binary diverges from C");
}
