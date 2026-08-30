//! Shared harness for the differential tests.
//!
//! Both programs are driven as subprocesses exactly the way a shell would run
//! them: bytes on stdin, bytes captured from stdout and stderr, and the exit
//! status inspected afterwards. Nothing here loads the Rust crate as a library.

// Each integration test binary gets its own copy of this module and uses a
// different subset of it.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Result of running one of the two programs.
#[derive(Debug, PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if the process was signalled.
    pub code: Option<i32>,
    /// Terminating signal number, if the process died from a signal.
    pub signal: Option<i32>,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn c_build_dir() -> PathBuf {
    workspace_root().join("c_src").join("build")
}

/// Path to the reference C executable, building it on first use if needed.
///
/// `c_src/` is treated as read-only ground truth; only the out-of-source
/// `c_src/build/` directory that CMake owns is ever created here.
pub fn c_binary() -> PathBuf {
    static BUILD: Once = Once::new();
    let exe = c_build_dir().join("driver");

    BUILD.call_once(|| {
        if exe.exists() {
            return;
        }
        let src = workspace_root().join("c_src");
        let build = c_build_dir();
        std::fs::create_dir_all(&build).expect("cannot create c_src/build");

        let configure = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("failed to invoke cmake (is it installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let compile = Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .output()
            .expect("failed to invoke cmake --build");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );
    });

    assert!(
        exe.exists(),
        "reference C binary missing at {}; build it with \
         `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`",
        exe.display()
    );
    exe
}

/// Path to the Rust executable produced by `cargo build`.
pub fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Runs `exe` with `stdin_bytes` on stdin and `args` on the command line.
pub fn run(exe: &Path, args: &[&str], stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        let bytes = stdin_bytes.to_vec();
        // Write on a helper thread so a program that never drains stdin (or a
        // large payload that exceeds the pipe buffer) cannot deadlock the test.
        std::thread::spawn(move || {
            let _ = sink.write_all(&bytes);
            let _ = sink.flush();
        });
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", exe.display()));

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal,
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Asserts that both programs agree on stdout, stderr and exit status.
pub fn assert_same(label: &str, args: &[&str], stdin_bytes: &[u8]) {
    let c = run(&c_binary(), args, stdin_bytes);
    let r = run(&rust_binary(), args, stdin_bytes);

    let context = || {
        format!(
            "case {label}\n  args   : {args:?}\n  stdin  : \"{}\"\n\
             \n  C  stdout: \"{}\"\n  RS stdout: \"{}\"\
             \n  C  stderr: \"{}\"\n  RS stderr: \"{}\"\
             \n  C  exit  : code={:?} signal={:?}\n  RS exit  : code={:?} signal={:?}",
            show(stdin_bytes),
            show(&c.stdout),
            show(&r.stdout),
            show(&c.stderr),
            show(&r.stderr),
            c.code,
            c.signal,
            r.code,
            r.signal,
        )
    };

    assert_eq!(c.stdout, r.stdout, "stdout mismatch\n{}", context());
    assert_eq!(c.stderr, r.stderr, "stderr mismatch\n{}", context());
    assert_eq!(c.code, r.code, "exit code mismatch\n{}", context());
    assert_eq!(c.signal, r.signal, "exit signal mismatch\n{}", context());
}

/// Convenience wrapper for the common "no argv, text on stdin" case.
pub fn check(label: &str, stdin_text: &str) {
    assert_same(label, &[], stdin_text.as_bytes());
}
