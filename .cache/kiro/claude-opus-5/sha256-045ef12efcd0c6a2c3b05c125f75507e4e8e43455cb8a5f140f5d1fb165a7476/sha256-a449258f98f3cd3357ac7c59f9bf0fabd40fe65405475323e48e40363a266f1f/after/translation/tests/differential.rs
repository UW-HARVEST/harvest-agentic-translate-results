//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses with identical argv/stdin/env, and require byte-identical
//! stdout, byte-identical stderr, and an identical exit status.
//!
//! The Rust code is never linked as a library here — both programs are driven
//! exactly the way a shell would drive them, which is how they are compared.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

/// Repository root — the directory that holds both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the built C executable, building it with CMake on first use so that
/// `cargo test` is self-contained.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build_dir = c_src.join("build");
        let exe = build_dir.join("driver");
        if exe.is_file() {
            return exe;
        }

        std::fs::create_dir_all(&build_dir).expect("could not create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake ..` (is cmake installed?)");
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
            exe.is_file(),
            "C build reported success but {} does not exist",
            exe.display()
        );
        exe
    })
}

/// Path to the built Rust executable. Cargo guarantees this is built and
/// up to date before the integration test runs.
fn rust_binary() -> &'static Path {
    static RUST_BIN: OnceLock<PathBuf> = OnceLock::new();
    RUST_BIN.get_or_init(|| PathBuf::from(env!("CARGO_BIN_EXE_driver")))
}

// ---------------------------------------------------------------------------
// Running a program and capturing everything observable
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
    /// Terminating signal number, if any.
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("exit_code", &self.code)
            .field("signal", &self.signal)
            .field("stdout_len", &self.stdout.len())
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stderr_len", &self.stderr.len())
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .finish()
    }
}

/// Fully describes one invocation, so both programs get treated identically.
struct Invocation {
    args: Vec<OsString>,
    stdin: Vec<u8>,
    /// Extra environment variables layered on top of the inherited environment.
    env: Vec<(OsString, OsString)>,
}

impl Invocation {
    fn new() -> Self {
        Invocation {
            args: Vec::new(),
            stdin: Vec::new(),
            env: Vec::new(),
        }
    }

    fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|a| a.as_ref().to_os_string()));
        self
    }

    fn stdin(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.stdin = bytes.into();
        self
    }

    fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.env
            .push((key.as_ref().to_os_string(), value.as_ref().to_os_string()));
        self
    }

    fn run(&self, program: &Path) -> Outcome {
        use std::os::unix::process::ExitStatusExt;

        let mut cmd = Command::new(program);
        cmd.args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in &self.env {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

        // Write stdin on a helper thread so a program that never reads stdin
        // cannot deadlock us on a full pipe buffer.
        let mut sink = child.stdin.take().expect("piped stdin");
        let payload = self.stdin.clone();
        let writer = std::thread::spawn(move || {
            let _ = sink.write_all(&payload);
            let _ = sink.flush();
            drop(sink);
        });

        let output = child
            .wait_with_output()
            .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", program.display()));
        let _ = writer.join();

        Outcome {
            stdout: output.stdout,
            stderr: output.stderr,
            code: output.status.code(),
            signal: output.status.signal(),
        }
    }
}

/// Runs both programs under the same invocation and asserts stdout, stderr and
/// exit status all match byte for byte.
#[track_caller]
fn assert_same(label: &str, inv: &Invocation) -> Outcome {
    let c = inv.run(c_binary());
    let r = inv.run(rust_binary());

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout mismatch\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr mismatch\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{label}] exit status mismatch\n  C   : {c:?}\n  Rust: {r:?}"
    );
    c
}

// ---------------------------------------------------------------------------
// Phase A — both binaries exist and are runnable
// ---------------------------------------------------------------------------

#[test]
fn phase_a_both_binaries_build_and_run() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    assert!(r.is_file(), "Rust binary missing at {}", r.display());

    let out = assert_same("phase_a/plain", &Invocation::new());
    assert_eq!(out.code, Some(0), "C `main` returns 0");
    assert!(out.stderr.is_empty(), "C program writes nothing to stderr");
    assert!(!out.stdout.is_empty(), "C program writes to stdout");
}

// ---------------------------------------------------------------------------
// Phase B — the exact bytes the C program emits
// ---------------------------------------------------------------------------

/// The full expected transcript, derived by reading `c_src/src/main.c`:
/// `printLine` is `printf("%s\n", line)`, `good()` also calls `helperGood()`,
/// and `bad()` deliberately does *not* call `helperBad()`.
const EXPECTED_STDOUT: &[u8] = b"Calling good()...\n\
good()\n\
helperGood()\n\
Finished good()\n\
Calling bad()...\n\
bad()\n\
Finished bad()\n";

#[test]
fn phase_b_exact_stdout_transcript() {
    let out = assert_same("phase_b/transcript", &Invocation::new());
    assert_eq!(
        out.stdout,
        EXPECTED_STDOUT,
        "stdout transcript changed\n  got: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
    // Exactly seven lines, each terminated by a single '\n', no trailing blank.
    assert_eq!(out.stdout.iter().filter(|&&b| b == b'\n').count(), 7);
    assert!(out.stdout.ends_with(b"Finished bad()\n"));
    assert!(!out.stdout.ends_with(b"\n\n"));
    // No '\r', no stray spaces around the printed literals.
    assert!(!out.stdout.contains(&b'\r'));
}

#[test]
fn phase_b_helper_bad_is_never_called() {
    // `helperBad()` is `static` and unreferenced in the C source, so its string
    // must never appear on stdout. A translation that "fixed" bad() to call its
    // helper would fail here.
    let out = assert_same("phase_b/helper_bad", &Invocation::new());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("helperBad()"),
        "helperBad() must not run; stdout was {text:?}"
    );
    assert_eq!(text.matches("helperGood()").count(), 1);
}

#[test]
fn phase_b_stderr_is_empty_and_exit_is_zero() {
    let out = assert_same("phase_b/stderr_exit", &Invocation::new());
    assert_eq!(out.stderr, b"", "nothing is written to stderr");
    assert_eq!(out.code, Some(0));
    assert_eq!(out.signal, None);
}

#[test]
fn phase_b_output_is_deterministic_across_runs() {
    let inv = Invocation::new();
    let first = assert_same("phase_b/determinism/0", &inv);
    for i in 1..5 {
        let next = assert_same(&format!("phase_b/determinism/{i}"), &inv);
        assert_eq!(first.stdout, next.stdout, "run {i} stdout differs");
        assert_eq!(first.stderr, next.stderr, "run {i} stderr differs");
        assert_eq!(first.code, next.code, "run {i} exit code differs");
    }
}

// ---------------------------------------------------------------------------
// Phase C — argv classes. `main` takes argc/argv but branches on neither, so
// every one of these must produce the identical transcript.
// ---------------------------------------------------------------------------

#[test]
fn phase_c_argv_empty() {
    let out = assert_same("phase_c/argv/none", &Invocation::new());
    assert_eq!(out.stdout, EXPECTED_STDOUT);
}

#[test]
fn phase_c_argv_single_item() {
    let out = assert_same("phase_c/argv/one", &Invocation::new().args(["only"]));
    assert_eq!(out.stdout, EXPECTED_STDOUT, "argv is ignored by the C code");
}

#[test]
fn phase_c_argv_empty_string_argument() {
    // argc == 2 with a zero-length argument: distinct from "no arguments".
    let out = assert_same("phase_c/argv/empty_string", &Invocation::new().args([""]));
    assert_eq!(out.stdout, EXPECTED_STDOUT);
}

#[test]
fn phase_c_argv_flag_like_arguments() {
    // Nothing in the C source parses options; these must not change anything.
    for arg in ["-h", "--help", "-", "--", "--version", "-0", "1"] {
        let out = assert_same(
            &format!("phase_c/argv/flag/{arg}"),
            &Invocation::new().args([arg]),
        );
        assert_eq!(out.stdout, EXPECTED_STDOUT, "argument {arg:?} changed output");
        assert_eq!(out.code, Some(0), "argument {arg:?} changed exit status");
        assert!(out.stderr.is_empty(), "argument {arg:?} produced stderr");
    }
}

#[test]
fn phase_c_argv_non_utf8_argument() {
    // A C `char *` need not be valid UTF-8. The Rust program must not choke on
    // (or even look at) such an argument.
    let raw = OsStr::from_bytes(&[0xff, 0xfe, 0x80, 0x01, b'x']);
    let out = assert_same("phase_c/argv/non_utf8", &Invocation::new().args([raw]));
    assert_eq!(out.stdout, EXPECTED_STDOUT);
    assert_eq!(out.code, Some(0));
}

#[test]
fn phase_c_argv_very_long_argument() {
    let long: String = "A".repeat(64 * 1024);
    let out = assert_same("phase_c/argv/long", &Invocation::new().args([long]));
    assert_eq!(out.stdout, EXPECTED_STDOUT);
    assert_eq!(out.code, Some(0));
}

#[test]
fn phase_c_argv_many_arguments() {
    // A large argc, well beyond anything the program inspects.
    let many: Vec<String> = (0..2000).map(|i| format!("arg{i}")).collect();
    let out = assert_same("phase_c/argv/many", &Invocation::new().args(many));
    assert_eq!(out.stdout, EXPECTED_STDOUT);
    assert_eq!(out.code, Some(0));
}

// ---------------------------------------------------------------------------
// Phase C — stdin classes. The C program never reads stdin (no scanf, no
// fgets), so stdin content must be consumed by nobody and change nothing.
// ---------------------------------------------------------------------------

#[test]
fn phase_c_stdin_empty() {
    let out = assert_same("phase_c/stdin/empty", &Invocation::new().stdin(&b""[..]));
    assert_eq!(out.stdout, EXPECTED_STDOUT);
}

#[test]
fn phase_c_stdin_single_line() {
    let out = assert_same(
        "phase_c/stdin/one_line",
        &Invocation::new().stdin(&b"1\n"[..]),
    );
    assert_eq!(out.stdout, EXPECTED_STDOUT, "stdin is never read");
}

#[test]
fn phase_c_stdin_line_without_newline() {
    // Distinguishes fgets-style from scanf-style reading; neither is used, so
    // the transcript must be unchanged and stdin must be left untouched.
    let out = assert_same(
        "phase_c/stdin/no_trailing_newline",
        &Invocation::new().stdin(&b"no newline here"[..]),
    );
    assert_eq!(out.stdout, EXPECTED_STDOUT);
}

#[test]
fn phase_c_stdin_numeric_and_whitespace_soup() {
    // The kind of input a scanf-based program would branch on.
    let out = assert_same(
        "phase_c/stdin/soup",
        &Invocation::new().stdin(&b"  \t 42 \n\n -1\r\n 0 abc\n"[..]),
    );
    assert_eq!(out.stdout, EXPECTED_STDOUT);
    assert!(out.stderr.is_empty());
}

#[test]
fn phase_c_stdin_binary_with_nul_bytes() {
    let payload: Vec<u8> = (0u8..=255).collect();
    let out = assert_same("phase_c/stdin/binary", &Invocation::new().stdin(payload));
    assert_eq!(out.stdout, EXPECTED_STDOUT);
    assert_eq!(out.code, Some(0));
}

#[test]
fn phase_c_stdin_larger_than_a_pipe_buffer() {
    // 1 MiB that nobody reads: both programs must still exit 0 rather than
    // block or die on a write error in the feeding thread.
    let payload = vec![b'z'; 1024 * 1024];
    let out = assert_same("phase_c/stdin/large", &Invocation::new().stdin(payload));
    assert_eq!(out.stdout, EXPECTED_STDOUT);
    assert_eq!(out.code, Some(0));
}

// ---------------------------------------------------------------------------
// Phase C — environment. Nothing in the C source reads getenv or setlocale, so
// locale must not alter `printf` output.
// ---------------------------------------------------------------------------

#[test]
fn phase_c_environment_locale_does_not_change_output() {
    for (k, v) in [
        ("LC_ALL", "C"),
        ("LC_ALL", "en_US.UTF-8"),
        ("LC_ALL", "tr_TR.UTF-8"),
        ("LANG", "de_DE.UTF-8"),
        ("LC_NUMERIC", "de_DE.UTF-8"),
    ] {
        let out = assert_same(
            &format!("phase_c/env/{k}={v}"),
            &Invocation::new().env(k, v),
        );
        assert_eq!(out.stdout, EXPECTED_STDOUT, "{k}={v} changed output");
    }
}

// ---------------------------------------------------------------------------
// Phase C — output redirection classes. C stdio is fully buffered to a file or
// pipe and line buffered to a tty; the emitted bytes must be identical either
// way, and neither program may report an error.
// ---------------------------------------------------------------------------

#[test]
fn phase_c_stdout_redirected_to_a_file() {
    let dir = std::env::temp_dir().join(format!("driver_diff_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");

    let run_to_file = |program: &Path, name: &str| -> (Vec<u8>, Option<i32>) {
        let path = dir.join(name);
        let file = std::fs::File::create(&path).expect("create output file");
        let status = Command::new(program)
            .stdin(Stdio::null())
            .stdout(Stdio::from(file))
            .stderr(Stdio::null())
            .status()
            .expect("spawn");
        let bytes = std::fs::read(&path).expect("read back output");
        (bytes, status.code())
    };

    let (c_bytes, c_code) = run_to_file(c_binary(), "c.out");
    let (r_bytes, r_code) = run_to_file(rust_binary(), "r.out");

    assert_eq!(
        c_bytes,
        r_bytes,
        "file-redirected stdout differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_bytes),
        String::from_utf8_lossy(&r_bytes)
    );
    assert_eq!(c_code, r_code, "exit status differs when stdout is a file");
    assert_eq!(c_bytes, EXPECTED_STDOUT);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn phase_c_stdout_and_stderr_merged_into_one_stream() {
    // Interleaving check: if the Rust program wrote anything to stderr, or
    // flushed in a different order, a merged stream would expose it.
    let dir = std::env::temp_dir().join(format!("driver_merge_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");

    let run_merged = |program: &Path, name: &str| -> (Vec<u8>, Option<i32>) {
        let path = dir.join(name);
        let file = std::fs::File::create(&path).expect("create merged file");
        let dup = file.try_clone().expect("clone fd for stderr");
        let status = Command::new(program)
            .stdin(Stdio::null())
            .stdout(Stdio::from(file))
            .stderr(Stdio::from(dup))
            .status()
            .expect("spawn");
        (std::fs::read(&path).expect("read merged"), status.code())
    };

    let (c_bytes, c_code) = run_merged(c_binary(), "c.merged");
    let (r_bytes, r_code) = run_merged(rust_binary(), "r.merged");

    assert_eq!(
        c_bytes,
        r_bytes,
        "merged stdout+stderr differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_bytes),
        String::from_utf8_lossy(&r_bytes)
    );
    assert_eq!(c_code, r_code);
    assert_eq!(c_bytes, EXPECTED_STDOUT, "no stderr output is interleaved");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn phase_c_stdout_discarded_to_dev_null() {
    let inv_run = |program: &Path| -> Option<i32> {
        Command::new(program)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("spawn")
            .code()
    };
    assert_eq!(
        inv_run(c_binary()),
        inv_run(rust_binary()),
        "exit status differs when stdout is /dev/null"
    );
}

#[test]
fn phase_c_stdout_closed_write_errors_are_ignored() {
    // `printLine` checks the return value of nothing: C ignores printf failure
    // and `main` still returns 0. The Rust translation must ignore write errors
    // the same way instead of panicking or exiting non-zero.
    use std::os::unix::io::{FromRawFd, IntoRawFd};

    let run_with_full_stdout = |program: &Path| -> (Option<i32>, Option<i32>, Vec<u8>) {
        use std::os::unix::process::ExitStatusExt;
        // /dev/full accepts opens but fails every write with ENOSPC.
        let full = match std::fs::OpenOptions::new().write(true).open("/dev/full") {
            Ok(f) => f,
            Err(_) => return (Some(0), None, Vec::new()), // platform without /dev/full
        };
        let fd = full.into_raw_fd();
        let stdout = unsafe { Stdio::from_raw_fd(fd) };
        let out = Command::new(program)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        (out.status.code(), out.status.signal(), out.stderr)
    };

    let c = run_with_full_stdout(c_binary());
    let r = run_with_full_stdout(rust_binary());
    assert_eq!(
        c.0, r.0,
        "exit code differs when every stdout write fails (C: {:?}, Rust: {:?})",
        c.0, r.0
    );
    assert_eq!(c.1, r.1, "terminating signal differs on stdout write failure");
    assert_eq!(
        c.2,
        r.2,
        "stderr differs on stdout write failure\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.2),
        String::from_utf8_lossy(&r.2)
    );
}

#[test]
fn phase_c_stdout_descriptor_is_not_open_at_all() {
    // Distinct from /dev/full: fd 1 does not exist, so every write fails with
    // EBADF. C ignores printf's return value and still returns 0; the Rust
    // translation must do the same rather than panicking on the write error.
    let run_closed_stdout = |program: &Path| -> (Vec<u8>, Option<i32>, Option<i32>) {
        use std::os::unix::process::ExitStatusExt;
        let out = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("exec '{}' >&-", program.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn via sh");
        (out.stderr, out.status.code(), out.status.signal())
    };

    let c = run_closed_stdout(c_binary());
    let r = run_closed_stdout(rust_binary());
    assert_eq!(
        c.0,
        r.0,
        "stderr differs with stdout closed\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.0),
        String::from_utf8_lossy(&r.0)
    );
    assert_eq!(
        (c.1, c.2),
        (r.1, r.2),
        "exit status differs with stdout closed\n  C   : {c:?}\n  Rust: {r:?}"
    );
    assert!(c.0.is_empty(), "C reports nothing on stderr");
}

#[test]
fn phase_c_stdin_closed_rather_than_piped() {
    // Not merely empty: file descriptor 0 is not open at all.
    let run_no_stdin = |program: &Path| -> (Vec<u8>, Vec<u8>, Option<i32>) {
        // `sh` closes fd 0 for us with the `<&-` redirection.
        let out = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("exec '{}' <&-", program.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn via sh");
        (out.stdout, out.stderr, out.status.code())
    };

    let c = run_no_stdin(c_binary());
    let r = run_no_stdin(rust_binary());
    assert_eq!(c.0, r.0, "stdout differs with stdin closed");
    assert_eq!(c.1, r.1, "stderr differs with stdin closed");
    assert_eq!(c.2, r.2, "exit status differs with stdin closed");
    assert_eq!(c.0, EXPECTED_STDOUT);
}

// ---------------------------------------------------------------------------
// Phase C — stdout on a terminal. C stdio switches to line buffering when
// `isatty(1)`, and the Rust translation mirrors that, so the tty path is a
// distinct branch worth exercising.
// ---------------------------------------------------------------------------

extern "C" {
    fn posix_openpt(flags: i32) -> i32;
    fn grantpt(fd: i32) -> i32;
    fn unlockpt(fd: i32) -> i32;
    fn ptsname(fd: i32) -> *mut std::os::raw::c_char;
}

/// Runs `program` with stdout attached to a freshly allocated pseudo-terminal
/// and returns everything the terminal master saw plus the exit status.
fn run_with_tty_stdout(program: &Path) -> (Vec<u8>, Option<i32>, Option<i32>) {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;
    use std::os::unix::process::ExitStatusExt;

    const O_RDWR: i32 = 2;
    const O_NOCTTY: i32 = 0o400;

    // Safety: plain libc pty setup; every returned fd is checked.
    let (master, slave_path) = unsafe {
        let master = posix_openpt(O_RDWR | O_NOCTTY);
        assert!(master >= 0, "posix_openpt failed");
        assert_eq!(grantpt(master), 0, "grantpt failed");
        assert_eq!(unlockpt(master), 0, "unlockpt failed");
        let name = ptsname(master);
        assert!(!name.is_null(), "ptsname returned NULL");
        let path = std::ffi::CStr::from_ptr(name)
            .to_str()
            .expect("pts name is ASCII")
            .to_owned();
        (master, path)
    };

    let slave = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&slave_path)
        .expect("open pty slave");

    // `Stdio::from(File)` takes ownership of the descriptor: the child receives
    // a dup, and the parent's copy is closed as part of `spawn`. That matters
    // because the parent must not keep the slave open, or reading the master
    // would never reach EOF. Do not close it by hand as well — a double close
    // in a multi-threaded test binary can shut a descriptor that another test
    // has since been handed.
    let mut child = Command::new(program)
        .stdin(Stdio::null())
        .stdout(Stdio::from(slave))
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn with tty stdout");

    let status = child.wait().expect("wait for child");

    // Safety: `master` is a live descriptor we own.
    let mut master_file = unsafe { std::fs::File::from_raw_fd(master) };
    let mut seen = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match master_file.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => seen.extend_from_slice(&chunk[..n]),
            // Reading a master whose slave has closed yields EIO on Linux.
            Err(_) => break,
        }
    }

    (seen, status.code(), status.signal())
}

#[test]
fn phase_c_stdout_on_a_terminal_matches() {
    let (c_bytes, c_code, c_signal) = run_with_tty_stdout(c_binary());
    let (r_bytes, r_code, r_signal) = run_with_tty_stdout(rust_binary());

    assert_eq!(
        c_bytes,
        r_bytes,
        "tty stdout differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_bytes),
        String::from_utf8_lossy(&r_bytes)
    );
    assert_eq!((c_code, c_signal), (r_code, r_signal), "tty exit status differs");
    assert_eq!(c_code, Some(0));

    // The terminal driver's ONLCR turns each '\n' into "\r\n"; the payload
    // underneath must still be the same seven lines.
    let normalized: Vec<u8> = c_bytes
        .iter()
        .copied()
        .filter(|&b| b != b'\r')
        .collect();
    assert_eq!(
        normalized, EXPECTED_STDOUT,
        "tty transcript differs from the expected lines once \\r is removed"
    );
}

/// `SIGPIPE` on Linux.
const SIGPIPE: i32 = 13;

#[test]
fn phase_c_stdout_is_a_pipe_whose_reader_is_already_gone() {
    // Deterministic broken-pipe case: build a pipe, let its read end close for
    // certain, then hand the write end to the program as stdout. Every write
    // then fails with EPIPE.
    //
    // C leaves SIGPIPE at its default disposition, so the C program is killed by
    // signal 13 (status 141 as a shell reports it). The Rust runtime sets
    // SIGPIPE to SIG_IGN before `main`, which would make the translation exit 0
    // instead; `main` restores SIG_DFL so the two agree.
    let broken_pipe_stdout = || -> Stdio {
        let mut reader = Command::new("/bin/sh")
            .arg("-c")
            .arg("exit 0")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn pipe-reader helper");
        let write_end = reader.stdin.take().expect("helper stdin is a pipe");
        // Once the helper has exited, the read end is closed for good.
        reader.wait().expect("wait for helper");
        Stdio::from(write_end)
    };

    let run = |program: &Path| -> (Option<i32>, Option<i32>, Vec<u8>) {
        use std::os::unix::process::ExitStatusExt;
        let out = Command::new(program)
            .stdin(Stdio::null())
            .stdout(broken_pipe_stdout())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn target program");
        (out.status.code(), out.status.signal(), out.stderr)
    };

    let c = run(c_binary());
    let r = run(rust_binary());

    assert_eq!(
        (c.0, c.1),
        (r.0, r.1),
        "exit status differs on a broken stdout pipe\n  C   : code={:?} signal={:?}\n  Rust: code={:?} signal={:?}",
        c.0,
        c.1,
        r.0,
        r.1
    );
    assert_eq!(
        c.2,
        r.2,
        "stderr differs on a broken stdout pipe\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.2),
        String::from_utf8_lossy(&r.2)
    );
    // Pin down what C actually does, so a regression in either direction fails.
    assert_eq!(
        c.1,
        Some(SIGPIPE),
        "expected the C program to be killed by SIGPIPE, got code={:?} signal={:?}",
        c.0,
        c.1
    );
}

#[test]
fn phase_c_stdout_pipe_drained_normally() {
    // The ordinary pipe case, for contrast with the broken-pipe test above: the
    // reader stays around, so nothing fails and both must exit 0.
    let run = |program: &Path| -> (Option<i32>, Option<i32>, Vec<u8>) {
        use std::os::unix::process::ExitStatusExt;
        let out = Command::new(program)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        (out.status.code(), out.status.signal(), out.stdout)
    };
    let c = run(c_binary());
    let r = run(rust_binary());
    assert_eq!((c.0, c.1), (r.0, r.1), "status differs on a normal pipe");
    assert_eq!(c.2, r.2, "stdout differs on a normal pipe");
    assert_eq!(c.0, Some(0));
}

// ---------------------------------------------------------------------------
// Phase D — the release binary named in the task is the one compared, in
// addition to whatever profile `cargo test` happened to build.
// ---------------------------------------------------------------------------

#[test]
fn phase_d_release_binary_matches_when_present() {
    let release = repo_root().join("translation/target/release/driver");
    if !release.is_file() {
        // `cargo test` alone need not have produced a release build; the debug
        // binary is covered by every other test in this file.
        eprintln!("note: {} not built, skipping", release.display());
        return;
    }

    let inv = Invocation::new();
    let c = inv.run(c_binary());
    let r = inv.run(&release);
    assert_eq!(c.stdout, r.stdout, "release stdout differs from C");
    assert_eq!(c.stderr, r.stderr, "release stderr differs from C");
    assert_eq!((c.code, c.signal), (r.code, r.signal), "release status differs");
    assert_eq!(r.stdout, EXPECTED_STDOUT);
}

#[test]
fn phase_d_c_sources_are_untouched() {
    // Guards the rule that nothing under c_src/ may be modified: the two source
    // files must still be exactly what the task shipped.
    let main_c = repo_root().join("c_src/src/main.c");
    let text = std::fs::read_to_string(&main_c).expect("read c_src/src/main.c");
    for needle in [
        "void printLine(const char *line)",
        "if (line != NULL)",
        "printf(\"%s\\n\", line);",
        "static void helperBad()",
        "void bad()\n{\n    printLine(\"bad()\");\n}",
        "printLine(\"good()\");\n    helperGood();",
        "return 0;",
    ] {
        assert!(
            text.contains(needle),
            "c_src/src/main.c no longer contains {needle:?} — the C source must not be modified"
        );
    }
    // bad() must still not call helperBad().
    assert!(
        !text.contains("printLine(\"bad()\");\n    helperBad();"),
        "c_src/src/main.c appears to have been modified to call helperBad()"
    );
}
