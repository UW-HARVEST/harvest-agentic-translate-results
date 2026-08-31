//! Shared harness for the differential tests.
//!
//! Both programs are driven as subprocesses, exactly the way a shell would run
//! them. Nothing here links against the Rust crate as a library.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Path to the Rust binary under test. Cargo builds it for us and hands the
/// path to integration tests through this environment variable.
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Workspace root: the directory holding both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use.
///
/// `c_src/` is read-only ground truth; we only ever create the throwaway
/// `c_src/build/` directory that CMake needs, never touch a source file.
pub fn c_bin() -> &'static Path {
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
                .expect("cmake must be installed to run the differential tests");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr),
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
                String::from_utf8_lossy(&compile.stderr),
            );
        }

        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
    .as_path()
}

/// Runs `bin` with `stdin` piped in and the given argv, capturing both streams.
fn run(bin: &Path, args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // The child may exit without draining stdin (the C program stops as soon as
    // scanf is satisfied). A short write would then fail with EPIPE, which is
    // expected rather than a test failure, so the result is deliberately
    // ignored here.
    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        let _ = sink.write_all(stdin);
        let _ = sink.flush();
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", bin.display()))
}

/// Renders bytes so a failure message stays readable for binary input.
fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:x?}"),
    }
}

/// Feeds `stdin` to both programs and asserts stdout, stderr and exit status
/// are byte-for-byte / value identical.
///
/// `label` names the input class so a regression points straight at the branch
/// it came from.
pub fn assert_same_with_args(label: &str, args: &[&str], stdin: &[u8]) {
    let c = run(c_bin(), args, stdin);
    let r = run(&rust_bin(), args, stdin);

    let mut problems = Vec::new();

    if c.stdout != r.stdout {
        problems.push(format!(
            "stdout differs:\n     C: {}\n  Rust: {}",
            show(&c.stdout),
            show(&r.stdout)
        ));
    }
    if c.stderr != r.stderr {
        problems.push(format!(
            "stderr differs:\n     C: {}\n  Rust: {}",
            show(&c.stderr),
            show(&r.stderr)
        ));
    }
    if c.status.code() != r.status.code() {
        problems.push(format!(
            "exit status differs:\n     C: {:?}\n  Rust: {:?}",
            c.status.code(),
            r.status.code()
        ));
    }

    assert!(
        problems.is_empty(),
        "mismatch for [{label}] with stdin {} args {args:?}\n{}",
        show(stdin),
        problems.join("\n")
    );
}

/// The common case: no argv, just stdin.
pub fn assert_same(label: &str, stdin: &[u8]) {
    assert_same_with_args(label, &[], stdin);
}

// ---------------------------------------------------------------------------
// Endless-stdin harness.
//
// `scanf` stops at the character that ends the number and never waits for EOF,
// so the program must terminate even when its stdin stays open forever. The
// helpers below keep writing to the child until the write fails, which is the
// only way to observe that difference: a program that reads stdin to EOF hangs
// here while the C program exits.
// ---------------------------------------------------------------------------

/// What a program did when fed a stream that never ends.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        code: Option<i32>,
    },
    /// Still running when the deadline passed, i.e. it blocked.
    Blocked,
}

/// Runs `bin` with `chunk` written to its stdin over and over, never closing
/// the pipe, and reports whether it exited within `timeout`.
fn run_endless(bin: &Path, chunk: &'static [u8], timeout: Duration) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let mut sink = child.stdin.take().expect("stdin was piped");
    let mut out = child.stdout.take().expect("stdout was piped");
    let mut err = child.stderr.take().expect("stderr was piped");

    // Feed the child forever. The loop ends when the child goes away and the
    // write fails with EPIPE (Rust ignores SIGPIPE, so this returns an error
    // rather than killing the test process).
    let writer = std::thread::spawn(move || while sink.write_all(chunk).is_ok() {});

    let deadline = Instant::now() + timeout;
    let mut exited = None;
    while Instant::now() < deadline {
        match child.try_wait().expect("try_wait failed") {
            Some(status) => {
                exited = Some(status);
                break;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }

    let outcome = match exited {
        Some(status) => {
            // Output is a single short line, well under the pipe capacity, so
            // it is safe to read it only now.
            let mut so = Vec::new();
            let mut se = Vec::new();
            out.read_to_end(&mut so).expect("read stdout");
            err.read_to_end(&mut se).expect("read stderr");
            Outcome::Exited {
                stdout: so,
                stderr: se,
                code: status.code(),
            }
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            Outcome::Blocked
        }
    };

    // Dropping the pipe ends the writer loop if it has not ended already.
    let _ = writer.join();
    outcome
}

/// Asserts both programs react identically to an endless stdin made of
/// `chunk` repeated: same stdout, stderr and exit status, or both blocking.
pub fn assert_same_endless(label: &str, chunk: &'static [u8]) {
    // Long enough that a program which exits promptly always does so, short
    // enough that the deliberately-blocking cases stay quick.
    let timeout = Duration::from_secs(2);

    let c = run_endless(c_bin(), chunk, timeout);
    let r = run_endless(&rust_bin(), chunk, timeout);

    assert_eq!(
        c,
        r,
        "mismatch for [{label}] on an endless stdin of {}\n     C: {c:?}\n  Rust: {r:?}",
        show(chunk),
    );
}
