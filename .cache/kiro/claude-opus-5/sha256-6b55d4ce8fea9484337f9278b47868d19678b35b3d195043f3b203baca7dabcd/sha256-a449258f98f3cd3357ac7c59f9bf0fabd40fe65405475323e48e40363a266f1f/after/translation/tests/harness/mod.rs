//! Shared harness: run the C program and the Rust program as subprocesses on
//! the same stdin and compare stdout, stderr and exit status byte for byte.
//!
//! Nothing here links against the translated code as a library -- the Rust
//! program is driven exactly the way a shell drives the C program, because that
//! is what the two are compared on.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// What one run of a program produced.
#[derive(PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when the process was killed.
    pub status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={:?} stdout={} bytes stderr={:?}",
            self.status,
            self.stdout.len(),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// Path of the compiled C program, building it with CMake if it is not there
/// yet.  A comparison against a program that did not build measures nothing, so
/// this panics loudly rather than skipping.
pub fn c_binary() -> PathBuf {
    let root = workspace_root();
    let bin = root.join("c_src/build/driver");
    if bin.exists() {
        return bin;
    }
    let build_dir = root.join("c_src/build");
    std::fs::create_dir_all(&build_dir).expect("create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build_dir)
        .output()
        .expect("run cmake");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&configure.stderr)
    );
    let build = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build_dir)
        .output()
        .expect("run cmake --build");
    assert!(
        build.status.success(),
        "cmake --build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(bin.exists(), "cmake did not produce {}", bin.display());
    bin
}

/// Path of the compiled Rust program.  Cargo builds the `driver` binary for
/// integration tests and points this at it, so the test always exercises the
/// same profile it was built with.
pub fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

pub fn run(binary: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", binary.display()));

    // Feed stdin from another thread: the program writes a lot more than a pipe
    // buffer holds, so writing and reading have to overlap.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("wait for child");
    writer.join().expect("stdin writer");

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match out.status.code() {
            Some(code) => Ok(code),
            None => Err(out.status.signal().expect("signal or code")),
        },
    }
}

pub fn run_c(input: &[u8]) -> Outcome {
    run(&c_binary(), input)
}

#[allow(dead_code)]
pub fn run_rust(input: &[u8]) -> Outcome {
    run(&rust_binary(), input)
}

fn show(label: &str, a: &[u8], b: &[u8]) -> String {
    let at = a
        .iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let from = at.saturating_sub(60);
    format!(
        "{label} differs at byte {at} (C {} bytes, Rust {} bytes)\n  C   ...{:?}\n  Rust...{:?}\n",
        a.len(),
        b.len(),
        String::from_utf8_lossy(&a[from..(at + 60).min(a.len())]),
        String::from_utf8_lossy(&b[from..(at + 60).min(b.len())]),
    )
}

/// Assert that the Rust program reproduces the C program exactly: stdout,
/// stderr and exit status.
#[track_caller]
#[allow(dead_code)]
pub fn assert_identical(name: &str, input: &[u8]) {
    let c = run_c(input);
    let r = run_rust(input);
    let mut problems = String::new();
    if c.stdout != r.stdout {
        problems.push_str(&show("stdout", &c.stdout, &r.stdout));
    }
    if c.stderr != r.stderr {
        problems.push_str(&show("stderr", &c.stderr, &r.stderr));
    }
    if c.status != r.status {
        problems.push_str(&format!(
            "exit status differs: C {:?}, Rust {:?}\n",
            c.status, r.status
        ));
    }
    assert!(
        problems.is_empty(),
        "case {name}: C and Rust disagree\n{problems}\ninput ({} bytes): {:?}",
        input.len(),
        String::from_utf8_lossy(&input[..input.len().min(2000)])
    );
}

/// Join menu lines into a stdin image, with a trailing newline.
#[allow(dead_code)]
pub fn script(lines: &[&str]) -> Vec<u8> {
    let mut s = lines.join("\n");
    s.push('\n');
    s.into_bytes()
}
