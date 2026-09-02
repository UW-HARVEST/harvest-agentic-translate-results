//! Differential-test harness.
//!
//! Both programs are driven as SUBPROCESSES exactly the way a shell would:
//! bytes on stdin, bytes captured from stdout/stderr, exit status compared.
//! Nothing in the Rust crate is ever called as a library.

// This module is compiled into each integration-test binary, and each one uses
// only the helpers it needs.
#![allow(dead_code)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Wall-clock budget for a single run of a program that is expected to finish.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

/// Repository root: the directory that holds both `c_src/` and `translation/`.
fn repo_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .expect("translation/ must have a parent directory")
            .to_path_buf();
        assert!(
            root.join("c_src/src/main.c").is_file(),
            "expected the C source at {}/c_src/src/main.c",
            root.display()
        );
        root
    })
}

/// Path to the C executable, building it with CMake the first time if needed.
pub fn c_binary() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
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
            .expect("failed to run `cmake` -- it must be installed to build the C reference");
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
            .expect("failed to run `cmake --build`");
        assert!(
            build.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );

        assert!(
            exe.is_file(),
            "C build reported success but {} is missing",
            exe.display()
        );
        exe
    })
}

/// Path to the Rust executable under test. Cargo builds it before the test runs.
pub fn rust_binary() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| PathBuf::from(env!("CARGO_BIN_EXE_driver")))
}

/// Outcome of one subprocess run.
#[derive(PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Some(code)` for a normal exit.
    pub code: Option<i32>,
    /// `Some(signal)` when killed by a signal.
    pub signal: Option<i32>,
    /// True when the program was still running/producing output when the byte
    /// cap was reached (only possible via [`run_capped`]).
    pub truncated: bool,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("code", &self.code)
            .field("signal", &self.signal)
            .field("truncated", &self.truncated)
            .field("stdout_len", &self.stdout.len())
            .field("stdout", &Preview(&self.stdout))
            .field("stderr_len", &self.stderr.len())
            .field("stderr", &Preview(&self.stderr))
            .finish()
    }
}

struct Preview<'a>(&'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head = &self.0[..self.0.len().min(240)];
        write!(f, "{:?}", String::from_utf8_lossy(head))?;
        if self.0.len() > head.len() {
            write!(f, "...(+{} bytes)", self.0.len() - head.len())?;
        }
        Ok(())
    }
}

fn spawn(bin: &Path, input: &[u8]) -> Child {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // The inputs here are tiny (far below the pipe buffer), so a straight write
    // then close cannot deadlock. A closed pipe is not an error: the program is
    // allowed to stop reading stdin at any point.
    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }
    child
}

fn exit_bits(status: std::process::ExitStatus) -> (Option<i32>, Option<i32>) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        (status.code(), status.signal())
    }
    #[cfg(not(unix))]
    {
        (status.code(), None)
    }
}

/// Run `bin` to completion with `input` on stdin.
pub fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = spawn(bin, input);

    let mut out = child.stdout.take().expect("stdout was piped");
    let mut err = child.stderr.take().expect("stderr was piped");
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err.read_to_end(&mut buf);
        buf
    });

    let mut stdout = Vec::new();
    let _ = out.read_to_end(&mut stdout);
    let stderr = reader.join().expect("stderr reader thread panicked");

    let deadline = Instant::now() + RUN_TIMEOUT;
    let status = loop {
        match child.try_wait().expect("try_wait failed") {
            Some(s) => break s,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                panic!(
                    "{} did not exit within {:?} on input {:?}",
                    bin.display(),
                    RUN_TIMEOUT,
                    String::from_utf8_lossy(input)
                );
            }
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    };

    let (code, signal) = exit_bits(status);
    Outcome {
        stdout,
        stderr,
        code,
        signal,
        truncated: false,
    }
}

/// Run `bin` but stop after `limit` bytes of stdout, killing it if it is still
/// going. Used for the input classes where the C program's output is unbounded
/// (see `ERRORS.md`); the observable prefixes must still match byte for byte.
pub fn run_capped(bin: &Path, input: &[u8], limit: usize) -> Outcome {
    let mut child = spawn(bin, input);
    let mut out = child.stdout.take().expect("stdout was piped");

    let mut stdout = Vec::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut truncated = false;
    while stdout.len() < limit {
        match out.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => stdout.extend_from_slice(&buf[..n]),
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => panic!("read from {} failed: {e}", bin.display()),
        }
    }
    if stdout.len() >= limit {
        stdout.truncate(limit);
        truncated = true;
        let _ = child.kill();
    }
    drop(out);

    let mut err = child.stderr.take().expect("stderr was piped");
    let mut stderr = Vec::new();
    let _ = err.read_to_end(&mut stderr);

    let status = child.wait().expect("wait failed");
    let (code, signal) = exit_bits(status);
    Outcome {
        stdout,
        stderr,
        code,
        signal,
        // When we killed the child, its exit status is ours, not the program's.
        truncated,
    }
}

/// The core assertion: for one input, stdout, stderr AND exit status of the
/// Rust program must be identical to the C program's.
#[track_caller]
pub fn assert_identical(label: &str, input: &[u8]) {
    let c = run(c_binary(), input);
    let r = run(rust_binary(), input);
    compare(label, input, &c, &r);
}

/// Same as [`assert_identical`] but only over the first `limit` stdout bytes,
/// for inputs on which the C program never terminates.
#[track_caller]
pub fn assert_identical_prefix(label: &str, input: &[u8], limit: usize) {
    let c = run_capped(c_binary(), input, limit);
    let r = run_capped(rust_binary(), input, limit);
    assert!(
        c.truncated,
        "[{label}] expected the C program to still be producing output at the \
         {limit}-byte cap; it stopped after {} bytes",
        c.stdout.len()
    );
    assert_eq!(
        c.stdout.len(),
        limit,
        "[{label}] C prefix length is not the cap"
    );
    assert_eq!(
        r.truncated, c.truncated,
        "[{label}] truncation mismatch: C={:?} Rust={:?}",
        c.truncated, r.truncated
    );
    assert_eq!(
        c.stdout, r.stdout,
        "[{label}] stdout prefix mismatch (first {limit} bytes) for input {:?}\n first difference at byte {:?}",
        String::from_utf8_lossy(input),
        c.stdout.iter().zip(&r.stdout).position(|(a, b)| a != b)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr mismatch: C={:?} Rust={:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
}

/// For inputs whose termination is not known ahead of time (fuzzing): compare
/// everything observable up to `limit` stdout bytes. When the C program did
/// finish inside the cap this is a full comparison, exit status included.
#[track_caller]
pub fn assert_compatible(label: &str, input: &[u8], limit: usize) {
    let c = run_capped(c_binary(), input, limit);
    let r = run_capped(rust_binary(), input, limit);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout mismatch for input {:?}\n  C   = {:?}\n  Rust= {:?}\n  first differing byte index: {:?}",
        String::from_utf8_lossy(input),
        Preview(&c.stdout),
        Preview(&r.stdout),
        c.stdout.iter().zip(&r.stdout).position(|(a, b)| a != b)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr mismatch for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        c.truncated,
        r.truncated,
        "[{label}] one program kept producing output past the cap and the other \
         did not, for input {:?}: C truncated={} Rust truncated={}",
        String::from_utf8_lossy(input),
        c.truncated,
        r.truncated
    );
    if !c.truncated {
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "[{label}] exit status mismatch for input {:?}",
            String::from_utf8_lossy(input)
        );
    }
}

#[track_caller]
pub fn compare(label: &str, input: &[u8], c: &Outcome, r: &Outcome) {
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] STDOUT mismatch for input {:?}\n  C   = {:?}\n  Rust= {:?}\n  first differing byte index: {:?}",
        String::from_utf8_lossy(input),
        Preview(&c.stdout),
        Preview(&r.stdout),
        c.stdout.iter().zip(&r.stdout).position(|(a, b)| a != b)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] STDERR mismatch for input {:?}\n  C   = {:?}\n  Rust= {:?}",
        String::from_utf8_lossy(input),
        Preview(&c.stderr),
        Preview(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{label}] EXIT STATUS mismatch for input {:?}: C=(code {:?}, signal {:?}) Rust=(code {:?}, signal {:?})",
        String::from_utf8_lossy(input),
        c.code,
        c.signal,
        r.code,
        r.signal
    );
}

/// Convenience: build the `"<x> <y>"` stdin a run of the program expects.
pub fn pair(x: i64, y: i64) -> Vec<u8> {
    format!("{x} {y}").into_bytes()
}
