//! Shared harness for the differential tests.
//!
//! Both programs are driven as *subprocesses*, exactly the way a shell would
//! run them, and stdout / stderr / exit status are compared byte for byte.
//! Nothing here loads the Rust code as a library.

use std::ffi::OsStr;
use std::fmt::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// `argv[0]` used for *both* programs.
///
/// `main.c` prints `argv[0]` in its usage message
/// (`fprintf(stderr, "%s requires 4 inputs\n", argv[0])`), so the two binaries
/// can only produce byte-identical stderr when they are executed with the same
/// `argv[0]`. `execve` lets `argv[0]` differ from the path that is executed
/// (that is what `exec -a NAME ...` does in a shell), so the harness pins it.
pub const ARGV0: &str = "driver";

#[derive(Debug, PartialEq, Eq)]
pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

/// Path of the Rust binary produced by this crate.
pub fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Repository root (the directory holding `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path of the C binary. Uses `c_src/build/driver` when it is already there,
/// otherwise configures and builds the CMake project into the test scratch
/// directory (never writing anything into `c_src/`).
pub fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = repo_root();
        let c_src = root.join("c_src");
        let prebuilt = c_src.join("build").join("driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        let build_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("c_build");
        std::fs::create_dir_all(&build_dir).expect("create scratch build dir");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&c_src)
            .arg("-B")
            .arg(&build_dir)
            .output()
            .expect("cmake must be installed to build the C reference program");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .expect("run cmake --build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        let bin = build_dir.join("driver");
        assert!(bin.is_file(), "C binary not found at {}", bin.display());
        bin
    })
}

/// Run `bin` with raw byte arguments (and an optional extra environment),
/// capturing stdout, stderr and the exit status.
pub fn run_with_env(bin: &Path, args: &[&[u8]], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(bin);
    cmd.arg0(ARGV0);
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.stdin(Stdio::null());
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    Output {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Run `bin` with an explicit `argv[0]` (what `exec -a NAME prog` does).
pub fn run_with_arg0(bin: &Path, arg0: &[u8], args: &[&[u8]]) -> Output {
    let mut cmd = Command::new(bin);
    cmd.arg0(OsStr::from_bytes(arg0));
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    cmd.stdin(Stdio::null());
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    Output {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Run `bin` with stdout pointed at `path` (used to exercise a failing write).
/// Returns stderr and the exit status.
pub fn run_stdout_to(bin: &Path, args: &[&[u8]], path: &str) -> Output {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap_or_else(|e| panic!("open {path}: {e}"));
    let mut cmd = Command::new(bin);
    cmd.arg0(ARGV0);
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::from(file));
    cmd.stderr(Stdio::piped());
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    Output {
        stdout: Vec::new(),
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn escape(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() + 2);
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

fn describe(args: &[&[u8]]) -> String {
    args.iter()
        .map(|a| format!("\"{}\"", escape(a)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Core assertion: the C program and the Rust program must agree on stdout,
/// stderr and exit status for these arguments.
pub fn assert_same_with_env(args: &[&[u8]], env: &[(&str, &str)]) {
    let c = run_with_env(c_bin(), args, env);
    let r = run_with_env(rust_bin(), args, env);

    if c == r {
        return;
    }

    let mut msg = String::new();
    let _ = writeln!(msg, "output mismatch for argv = [{}]", describe(args));
    if !env.is_empty() {
        let _ = writeln!(msg, "  env: {env:?}");
    }
    if c.stdout != r.stdout {
        let _ = writeln!(msg, "  stdout C: \"{}\"", escape(&c.stdout));
        let _ = writeln!(msg, "  stdout R: \"{}\"", escape(&r.stdout));
    }
    if c.stderr != r.stderr {
        let _ = writeln!(msg, "  stderr C: \"{}\"", escape(&c.stderr));
        let _ = writeln!(msg, "  stderr R: \"{}\"", escape(&r.stderr));
    }
    if c.code != r.code || c.signal != r.signal {
        let _ = writeln!(
            msg,
            "  status C: code={:?} signal={:?}   status R: code={:?} signal={:?}",
            c.code, c.signal, r.code, r.signal
        );
    }
    panic!("{msg}");
}

pub fn assert_same(args: &[&[u8]]) {
    assert_same_with_env(args, &[]);
}

/// Convenience wrapper for the common "three numeric arguments" shape.
pub fn assert_same3(a: &str, b: &str, c: &str) {
    assert_same(&[a.as_bytes(), b.as_bytes(), c.as_bytes()]);
}

/// Deterministic xorshift64* PRNG so the fuzz cases are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}
