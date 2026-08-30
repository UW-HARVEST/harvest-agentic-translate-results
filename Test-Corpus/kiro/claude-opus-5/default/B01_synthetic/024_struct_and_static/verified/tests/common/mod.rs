//! Shared harness: builds/locates the two executables and runs them as
//! subprocesses so they can be compared exactly as a shell would drive them.

// Each integration-test binary compiles this module in full but uses only part
// of it.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Result of running one program on one input.
#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    pub code: Option<i32>,
    /// Terminating signal number, if any (Unix only).
    pub signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit={:?} signal={:?}\n  stdout({} bytes)={:?}\n  stderr({} bytes)={:?}",
            self.code,
            self.signal,
            self.stdout.len(),
            String::from_utf8_lossy(&self.stdout),
            self.stderr.len(),
            String::from_utf8_lossy(&self.stderr),
        )
    }
}

/// Repository root: the directory containing `c_src/` and `translation/`.
fn repo_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .expect("translation/ must have a parent directory")
            .to_path_buf()
    })
}

/// Path to the Rust executable under test. Cargo builds it for us.
pub fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C executable, building it with CMake on first use.
///
/// `c_src/` is read-only ground truth; only the out-of-source `build/`
/// directory is created.
pub fn c_bin() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");

        // Common single- and multi-config output locations.
        let candidates = [
            build.join("driver"),
            build.join("driver.exe"),
            build.join("Release").join("driver.exe"),
            build.join("Debug").join("driver.exe"),
        ];
        if let Some(found) = candidates.iter().find(|p| p.is_file()) {
            return found.clone();
        }

        std::fs::create_dir_all(&build).expect("create c_src/build");

        let configure = Command::new("cmake")
            .current_dir(&build)
            .arg("..")
            .output()
            .expect("cmake must be installed to build the C reference program");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr),
        );

        let compile = Command::new("cmake")
            .current_dir(&build)
            .args(["--build", "."])
            .output()
            .expect("run cmake --build");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr),
        );

        candidates
            .iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("C executable not found under {}", build.display()))
            .clone()
    })
}

/// Run `bin` with `args`, writing `input` to its stdin, and capture everything.
pub fn run_with_args(bin: &Path, input: &[u8], args: &[&str]) -> Run {
    run_full(bin, input, args, &[])
}

/// Run `bin` with `args` and additional environment variables.
pub fn run_full(bin: &Path, input: &[u8], args: &[&str], env: &[(&str, &str)]) -> Run {
    let mut cmd = Command::new(bin);
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // Feed stdin on a helper thread so a large input cannot deadlock against a
    // full stdout pipe.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let owned = input.to_vec();
    let writer = std::thread::spawn(move || {
        // A program may exit before consuming all input; a broken pipe here is
        // expected and not a test failure.
        let _ = stdin.write_all(&owned);
        let _ = stdin.flush();
    });

    let out = child.wait_with_output().expect("wait for child");
    writer.join().expect("stdin writer thread");

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

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
pub fn assert_same(name: &str, input: &[u8]) {
    assert_same_with_args(name, input, &[]);
}

/// Same as [`assert_same`], with command-line arguments.
pub fn assert_same_with_args(name: &str, input: &[u8], args: &[&str]) {
    assert_same_full(name, input, args, &[]);
}

/// Same as [`assert_same`], with command-line arguments and environment
/// variables.
pub fn assert_same_full(name: &str, input: &[u8], args: &[&str], env: &[(&str, &str)]) {
    let c = run_full(c_bin(), input, args, env);
    let r = run_full(rust_bin(), input, args, env);

    let preview = preview(input);

    assert_eq!(
        c.stdout, r.stdout,
        "case `{name}` (input {preview}, args {args:?}): stdout differs\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr, r.stderr,
        "case `{name}` (input {preview}, args {args:?}): stderr differs\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.code, r.code,
        "case `{name}` (input {preview}, args {args:?}): exit code differs (C {:?} vs Rust {:?})",
        c.code, r.code,
    );
    assert_eq!(
        c.signal, r.signal,
        "case `{name}` (input {preview}, args {args:?}): terminating signal differs (C {:?} vs Rust {:?})",
        c.signal, r.signal,
    );
}

fn preview(input: &[u8]) -> String {
    const MAX: usize = 48;
    let shown = &input[..input.len().min(MAX)];
    let mut s = format!("{:?}", String::from_utf8_lossy(shown));
    if input.len() > MAX {
        s.push_str(&format!(" (+{} more bytes)", input.len() - MAX));
    }
    s
}
