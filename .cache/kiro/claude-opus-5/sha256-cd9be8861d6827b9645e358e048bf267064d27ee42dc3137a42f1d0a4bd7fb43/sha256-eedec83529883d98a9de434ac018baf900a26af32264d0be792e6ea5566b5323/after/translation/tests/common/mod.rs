//! Shared helpers for the differential tests.
//!
//! Every test in this suite runs the *built binaries* as subprocesses and diffs
//! stdout, stderr and exit status. Nothing here links the Rust crate as a
//! library.

// This module is compiled into each integration-test binary, and no single one
// of them uses every helper.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Repository root (the directory holding both `c_src/` and `translation/`).
pub fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

pub fn c_src_dir() -> PathBuf {
    repo_root().join("c_src")
}

fn run(cmd: &mut Command) -> std::process::Output {
    let rendered = format!("{:?}", cmd);
    match cmd.output() {
        Ok(o) => o,
        Err(e) => panic!("failed to spawn {rendered}: {e}"),
    }
}

/// Build the C reference program with CMake exactly as the task describes, and
/// return the path to the `driver` executable.
///
/// The build directory lives inside `c_src/build`, which is what
/// `cmake .. && cmake --build .` produces; no C source file is touched.
pub fn c_driver() -> PathBuf {
    // Tests inside one binary run in parallel; build at most once.
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(build_c_driver).clone()
}

fn build_c_driver() -> PathBuf {
    let c_src = c_src_dir();
    let build_dir = c_src.join("build");
    let exe = build_dir.join("driver");
    if exe.is_file() {
        return exe;
    }

    std::fs::create_dir_all(&build_dir).expect("create c_src/build");

    // NOTE: no CMAKE_BUILD_TYPE is passed, matching the documented build
    // command. A `Release` build would define NDEBUG and disable every
    // `assert()` in main.c, which changes the program's behaviour.
    let out = run(Command::new("cmake").arg("..").current_dir(&build_dir));
    assert!(
        out.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let out = run(Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build_dir));
    assert!(
        out.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(exe.is_file(), "expected {} to exist", exe.display());
    exe
}

/// Compile an auxiliary C program against the *unmodified* `c_src` sources.
///
/// Used to reach library branches that `main.c` never calls. `c_src/` is only
/// ever read from.
pub fn compile_c_aux(source: &Path, out_name: &str) -> PathBuf {
    let c_src = c_src_dir();
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("cdiff");
    std::fs::create_dir_all(&out_dir).expect("create target/cdiff");
    let exe = out_dir.join(out_name);
    // Link into a scratch name and rename, so a concurrently running copy of a
    // previous build is never overwritten in place ("Text file busy").
    let staged = out_dir.join(format!("{out_name}.{}", std::process::id()));

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let out = run(Command::new(cc)
        .arg("-std=c11")
        .arg("-O0")
        .arg("-g")
        .arg("-I")
        .arg(c_src.join("include"))
        .arg(source)
        .arg(c_src.join("src/hashmap.c"))
        .arg(c_src.join("src/tree.c"))
        .arg("-o")
        .arg(&staged));
    assert!(
        out.status.success(),
        "compiling {} failed:\n{}\n{}",
        source.display(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::rename(&staged, &exe).expect("install compiled probe");
    exe
}

/// One captured run of a program.
pub struct Captured {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    pub code: Option<i32>,
    /// `Some(signal)` when terminated by a signal.
    pub signal: Option<i32>,
}

impl Captured {
    fn describe(&self) -> String {
        match (self.code, self.signal) {
            (Some(c), _) => format!("exit={c}"),
            (None, Some(s)) => format!("signal={s}"),
            _ => "unknown-status".to_string(),
        }
    }
}

/// Run `exe` with the given argv tail, stdin bytes and extra environment.
pub fn capture(exe: &Path, args: &[&str], stdin_bytes: Option<&[u8]>) -> Captured {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(if stdin_bytes.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

    if let Some(bytes) = stdin_bytes {
        let mut sin = child.stdin.take().expect("piped stdin");
        // The programs never read stdin, so a short write is possible only if
        // the child exits first; ignore that case deliberately.
        let _ = sin.write_all(bytes);
        drop(sin);
    }

    let out = child.wait_with_output().expect("wait for child");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Captured {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal,
    }
}

fn render(label: &str, bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{label} ({} bytes):\n{s}", bytes.len()),
        Err(_) => format!("{label} ({} bytes, not UTF-8): {:x?}", bytes.len(), bytes),
    }
}

/// Assert that two captured runs are byte-for-byte identical on stdout, stderr
/// and exit status.
pub fn assert_same(case: &str, c: &Captured, r: &Captured) {
    let mut problems = Vec::new();

    if c.stdout != r.stdout {
        problems.push(format!(
            "stdout differs\n{}\n{}\nfirst difference at byte {:?}",
            render("C stdout", &c.stdout),
            render("Rust stdout", &r.stdout),
            first_diff(&c.stdout, &r.stdout)
        ));
    }
    if c.stderr != r.stderr {
        problems.push(format!(
            "stderr differs\n{}\n{}\nfirst difference at byte {:?}",
            render("C stderr", &c.stderr),
            render("Rust stderr", &r.stderr),
            first_diff(&c.stderr, &r.stderr)
        ));
    }
    if c.code != r.code || c.signal != r.signal {
        problems.push(format!(
            "status differs: C {} vs Rust {}",
            c.describe(),
            r.describe()
        ));
    }

    assert!(
        problems.is_empty(),
        "case `{case}` mismatched:\n{}",
        problems.join("\n---\n")
    );
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return Some(i);
        }
    }
    if a.len() == b.len() {
        None
    } else {
        Some(n)
    }
}
