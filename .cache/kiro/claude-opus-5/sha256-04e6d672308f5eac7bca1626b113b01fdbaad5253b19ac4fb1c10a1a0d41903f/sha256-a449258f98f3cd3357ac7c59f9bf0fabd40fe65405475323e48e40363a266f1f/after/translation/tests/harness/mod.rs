//! Shared harness for the differential tests.
//!
//! Both programs are driven as **subprocesses**, exactly the way a shell would
//! run them: argv on the command line, the test input on stdin, and stdout,
//! stderr and the exit status captured and compared byte for byte. Nothing here
//! links against the Rust crate as a library.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Captured result of one subprocess run.
#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    pub status: Result<i32, i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={:?} stdout={:?} stderr={:?}",
            self.status,
            Escaped(&self.stdout),
            Escaped(&self.stderr)
        )
    }
}

/// Renders arbitrary bytes readably, so a failure message stays legible even
/// when the output contains NULs or non-UTF-8 bytes.
pub struct Escaped<'a>(pub &'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"")?;
        for &b in self.0 {
            match b {
                b'\n' => f.write_str("\\n")?,
                b'\t' => f.write_str("\\t")?,
                b'\r' => f.write_str("\\r")?,
                b'"' => f.write_str("\\\"")?,
                b'\\' => f.write_str("\\\\")?,
                0x20..=0x7e => f.write_str(std::str::from_utf8(&[b]).unwrap())?,
                _ => write!(f, "\\x{:02x}", b)?,
            }
        }
        f.write_str("\"")
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the Rust binary under test. Cargo builds it before running the
/// integration test and hands us the path.
pub fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C reference binary, building it with CMake on first use so that
/// a bare `cargo test` is self-contained.
pub fn c_bin() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run cmake (is cmake installed?)");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );
        let bld = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("run cmake --build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );
        assert!(exe.is_file(), "C binary missing after build: {}", exe.display());
        exe
    })
}

/// Run one program with `args` on the command line and `stdin_data` on stdin.
///
/// stdin is written from a helper thread so that inputs larger than the pipe
/// buffer cannot deadlock against the child's output.
pub fn run(exe: &Path, args: &[&[u8]], stdin_data: &[u8]) -> Run {
    use std::os::unix::ffi::OsStrExt;

    let mut cmd = Command::new(exe);
    for a in args {
        cmd.arg(std::ffi::OsStr::from_bytes(a));
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

    let mut sink = child.stdin.take().expect("piped stdin");
    let data = stdin_data.to_vec();
    let writer = std::thread::spawn(move || {
        // A closed pipe (the child exited early, e.g. the argc error path) is
        // expected; ignore the resulting EPIPE.
        let _ = sink.write_all(&data);
        let _ = sink.flush();
    });

    let out = child.wait_with_output().expect("wait_with_output");
    writer.join().expect("stdin writer thread");

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            use std::os::unix::process::ExitStatusExt;
            Err(out.status.signal().unwrap_or(-1))
        }
    };
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

/// Assert that both programs agree on stdout, stderr and exit status.
pub fn assert_same(label: &str, args: &[&[u8]], stdin_data: &[u8]) {
    let c = run(c_bin(), args, stdin_data);
    let r = run(rust_bin(), args, stdin_data);

    if c == r {
        return;
    }

    let shown: Vec<String> = args
        .iter()
        .map(|a| format!("{:?}", Escaped(a)))
        .collect();
    panic!(
        "differential mismatch in case `{label}`\n  \
         argv    : [{}]\n  \
         stdin   : {:?}\n  \
         C       : {:?}\n  \
         Rust    : {:?}\n  \
         stdout equal: {}\n  \
         stderr equal: {}\n  \
         status equal: {}",
        shown.join(", "),
        Escaped(stdin_data),
        c,
        r,
        c.stdout == r.stdout,
        c.stderr == r.stderr,
        c.status == r.status,
    );
}

/// The usual "match everything" argument vector.
pub const WILD: &[&[u8]] = &[b"-", b"-", b"-", b"-"];

/// Convenience wrapper for the common `- - - -` argv.
pub fn check(label: &str, stdin_data: &[u8]) {
    assert_same(label, WILD, stdin_data);
}

/// Run a program whose stdout pipe is closed by the reader before the program
/// gets a chance to drain it. Returns the exit status only, since no output can
/// be collected.
///
/// With enough output to overflow the 64 KiB pipe buffer the program is
/// guaranteed to still be writing when the read end disappears, which makes the
/// resulting `SIGPIPE`/`EPIPE` deterministic.
pub fn run_with_dropped_stdout(exe: &Path, args: &[&[u8]], stdin_data: &[u8]) -> Result<i32, i32> {
    use std::os::unix::ffi::OsStrExt;

    let mut cmd = Command::new(exe);
    for a in args {
        cmd.arg(std::ffi::OsStr::from_bytes(a));
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

    let mut sink = child.stdin.take().expect("piped stdin");
    let data = stdin_data.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&data);
        let _ = sink.flush();
    });

    // Close the read end of stdout without reading a single byte.
    drop(child.stdout.take());

    let status = child.wait().expect("wait");
    writer.join().expect("stdin writer thread");

    match status.code() {
        Some(code) => Ok(code),
        None => {
            use std::os::unix::process::ExitStatusExt;
            Err(status.signal().unwrap_or(-1))
        }
    }
}
