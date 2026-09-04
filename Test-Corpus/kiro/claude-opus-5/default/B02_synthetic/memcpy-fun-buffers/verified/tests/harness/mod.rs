//! Shared differential-test harness.
//!
//! Both programs are driven the way a shell drives them: spawn the built
//! executable, write bytes to its stdin, read stdout/stderr, wait for the exit
//! status. The Rust code is never linked in as a library, because the C program
//! is being compared as a process.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Captured result of one process run.
#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Run {{ code: {:?}, signal: {:?}, stdout: {:?}, stderr: {:?} }}",
            self.code,
            self.signal,
            Truncated(&self.stdout),
            Truncated(&self.stderr),
        )
    }
}

struct Truncated<'a>(&'a [u8]);

impl std::fmt::Debug for Truncated<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf8_lossy(self.0);
        if s.len() > 400 {
            write!(f, "{}… ({} bytes total)", &s[..400], self.0.len())
        } else {
            write!(f, "{}", s)
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the Rust executable under test (`translation/target/*/driver`).
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the reference C executable, building it on first use if needed.
pub fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let root = manifest_dir()
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf();
    let c_src = root.join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");

    BUILD.call_once(|| {
        if exe.exists() {
            return;
        }
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run cmake (is cmake installed?)");
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
            .expect("run cmake --build");
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

/// Run one executable with `input` on stdin, exactly as a shell pipeline would.
pub fn run_one(exe: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    // Write on a helper thread so a program that never drains stdin (every
    // early-error path here) cannot deadlock the test.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("wait_with_output");
    let _ = writer.join();

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
#[track_caller]
pub fn assert_same(label: &str, input: &[u8]) {
    let c = run_one(&c_bin(), input);
    let r = run_one(&rust_bin(), input);

    let shown = String::from_utf8_lossy(&input[..input.len().min(300)]).into_owned();

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for [{label}]\n  input: {shown:?}\n  C  stdout: {:?}\n  Rust stdout: {:?}",
        Truncated(&c.stdout),
        Truncated(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for [{label}]\n  input: {shown:?}\n  C  stderr: {:?}\n  Rust stderr: {:?}",
        Truncated(&c.stderr),
        Truncated(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for [{label}]\n  input: {shown:?}\n  C: code={:?} signal={:?}\n  Rust: code={:?} signal={:?}",
        c.code,
        c.signal,
        r.code,
        r.signal
    );
}

/// Convenience wrapper for the common case of textual input.
#[track_caller]
pub fn same(label: &str, input: &str) {
    assert_same(label, input.as_bytes());
}

/// Render `n` bytes of deterministic buffer payload as `"<len> <b0> <b1> …"`,
/// i.e. exactly what `read_buffer` consumes.
pub fn buf(n: usize, base: usize) -> String {
    let mut s = n.to_string();
    for k in 0..n {
        s.push(' ');
        s.push_str(&((base + k * 3) % 256).to_string());
    }
    s
}
