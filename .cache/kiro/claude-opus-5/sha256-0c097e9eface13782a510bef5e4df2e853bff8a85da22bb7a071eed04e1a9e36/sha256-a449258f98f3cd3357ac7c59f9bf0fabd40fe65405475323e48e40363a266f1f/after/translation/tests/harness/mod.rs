//! Shared harness for the differential tests.
//!
//! Both programs are driven exactly the way a shell would drive them: spawn the
//! executable, write the case bytes to its stdin, close stdin, then compare
//! stdout, stderr and the exit status (including the terminating signal, since
//! some inputs make both programs die on SIGSEGV).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// This crate's binary, as built by `cargo test`.
pub fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // .../<repo>/translation  ->  .../<repo>
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The CMake-built C reference binary. Built on first use if it is missing so
/// that a bare `cargo test` works from a clean checkout.
pub fn c_bin() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let root = workspace_root();
        let src = root.join("c_src");
        let build = src.join("build");
        let exe = build.join("driver");
        if !exe.is_file() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("cmake must be installed to build the C reference");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr)
            );
            let compile = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("cmake --build");
            assert!(
                compile.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            );
        }
        assert!(
            exe.is_file(),
            "C reference binary not found at {}",
            exe.display()
        );
        exe
    })
}

/// stdout, stderr and exit status of one run.
#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    pub status: Result<i32, i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self.status {
            Ok(c) => format!("exit {}", c),
            Err(s) => format!("signal {}", s),
        };
        write!(
            f,
            "{{ {}, stdout: {:?}, stderr: {:?} }}",
            status,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

pub fn run(exe: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(stdin_bytes)
        // A program that dies before draining stdin gives EPIPE; that is a
        // legitimate observation, not a harness failure.
        .ok();
    let out = child.wait_with_output().expect("wait_with_output");
    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().expect("signal or code"))
            }
            #[cfg(not(unix))]
            {
                panic!("process ended without an exit code")
            }
        }
    };
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

/// Assert that the C reference and the Rust port agree on stdout, stderr and
/// the exit status for `stdin_bytes`.
pub fn check(name: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);
    assert_eq!(
        c,
        r,
        "case `{}` diverged\n  stdin: {:?}\n  C   : {:?}\n  Rust: {:?}",
        name,
        String::from_utf8_lossy(stdin_bytes),
        c,
        r
    );
}
