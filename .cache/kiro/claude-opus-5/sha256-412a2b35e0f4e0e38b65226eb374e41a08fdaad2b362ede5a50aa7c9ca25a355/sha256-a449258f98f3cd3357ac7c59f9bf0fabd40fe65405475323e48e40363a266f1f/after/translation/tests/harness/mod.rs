//! Differential test harness.
//!
//! Both programs are driven as subprocesses, exactly the way a shell would run
//! them, and stdout, stderr and the exit status are compared byte for byte.
//! Nothing here links the translation as a library.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Output of one run: stdout, stderr and the exit status.
#[derive(PartialEq, Eq)]
pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` if the process was killed.
    pub status: Result<i32, i32>,
}

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={:?} stdout={:?} stderr={:?}",
            self.status,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// Workspace root: the directory holding `c_src/` and `translation/`.
fn workspace_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        // CARGO_MANIFEST_DIR is `<root>/translation`.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .expect("translation/ must have a parent directory")
            .to_path_buf()
    })
}

/// Path to the C reference binary, building it with CMake on first use.
pub fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("cmake not available - it is required to build the C reference");
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
                .expect("cmake --build failed to start");
            assert!(
                compile.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            );
        }
        assert!(bin.exists(), "C reference binary missing at {:?}", bin);
        bin
    })
}

/// Path to the translated binary, built by Cargo for the current profile.
pub fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Runs `exe` with `args`, forcing `argv[0]` to a fixed string.
///
/// The `Usage:` message echoes `argv[0]`, so without this the two differently
/// located binaries could never produce identical stderr.
fn run(exe: &Path, args: &[&[u8]]) -> Output {
    let mut cmd = Command::new(exe);
    cmd.arg0("driver");
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    let out = cmd.output().unwrap_or_else(|e| panic!("failed to run {exe:?}: {e}"));
    Output {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match out.status.code() {
            Some(c) => Ok(c),
            None => {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().unwrap_or(-1))
            }
        },
    }
}

/// Asserts that the C program and the translation agree on stdout, stderr and
/// exit status for one argument vector.
pub fn assert_same(args: &[&[u8]]) {
    let c = run(c_binary(), args);
    let r = run(rust_binary(), args);
    if c != r {
        let shown: Vec<String> = args
            .iter()
            .map(|a| format!("{:?}", String::from_utf8_lossy(a)))
            .collect();
        panic!(
            "mismatch for argv = [{}]\n     C: {:?}\n  Rust: {:?}",
            shown.join(", "),
            c,
            r
        );
    }
}

/// `assert_same` for the common case of two `&str` arguments.
pub fn check(base: &str, exponent: &str) {
    assert_same(&[base.as_bytes(), exponent.as_bytes()]);
}

/// Runs every pair drawn from `bases` x `exponents`.
pub fn check_cross(bases: &[&str], exponents: &[&str]) {
    for b in bases {
        for e in exponents {
            check(b, e);
        }
    }
}

/// Deterministic 64-bit LCG, so the generated sweeps are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        // Numerical Recipes' LCG constants.
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}
