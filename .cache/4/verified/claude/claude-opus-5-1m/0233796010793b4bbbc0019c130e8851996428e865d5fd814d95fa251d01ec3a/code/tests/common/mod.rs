//! Differential-test harness.
//!
//! The C artifact is an EXECUTABLE (`add_executable(driver src/main.c)`), not a
//! shared library, so the black-box boundary is the process ABI rather than a
//! set of exported symbols.  Every helper here launches BOTH real binaries as
//! child processes and compares the four externally observable results:
//!
//!   * stdout bytes
//!   * stderr bytes
//!   * exit status code
//!   * terminating signal (SIGPIPE matters for this program)
//!
//! No Rust function of the crate is ever called in-process: the tests only see
//! what an external caller sees.

#![allow(dead_code)]

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// argv[0] handed to BOTH children so that the `Usage: %s ...` message is
/// byte-comparable.
pub const ARGV0: &str = "driver";

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C reference binary produced by `c_src/CMakeLists.txt`.
pub fn c_bin() -> PathBuf {
    if let Some(p) = std::env::var_os("DIFF_C_BIN") {
        return PathBuf::from(p);
    }
    let p = crate_root().join("c_src/build/driver");
    assert!(
        p.exists(),
        "C reference binary not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The Rust binary under test.  Loaded from disk and executed exactly like the
/// C one; never linked into the test process.
pub fn rust_bin() -> PathBuf {
    if let Some(p) = std::env::var_os("DIFF_RUST_BIN") {
        return PathBuf::from(p);
    }
    let rel = crate_root().join("target/release/driver");
    if rel.exists() {
        return rel;
    }
    let dbg = crate_root().join("target/debug/driver");
    assert!(
        dbg.exists(),
        "Rust binary not found; run `cargo build --release` first (looked in \
         target/release/driver and target/debug/driver)"
    );
    dbg
}

#[derive(PartialEq, Eq)]
pub struct Out {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl std::fmt::Debug for Out {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "code={:?} signal={:?}\n      stdout={}\n      stderr={}",
            self.code,
            self.signal,
            esc(&self.stdout),
            esc(&self.stderr)
        )
    }
}

/// Escape a byte string for readable assertion messages (and truncate the
/// 309-digit / 100 000-digit monsters).
pub fn esc(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len().min(400) + 8);
    s.push('"');
    for &c in b.iter().take(400) {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{:02x}", c)),
        }
    }
    s.push('"');
    if b.len() > 400 {
        s.push_str(&format!(" ...(+{} bytes)", b.len() - 400));
    }
    s
}

fn osstr(b: &[u8]) -> OsString {
    OsStr::from_bytes(b).to_os_string()
}

/// Run one binary with raw byte arguments and a forced argv[0].
pub fn run_raw(bin: &Path, argv0: &str, args: &[&[u8]], env: Option<&[(&str, &str)]>) -> Out {
    let mut cmd = Command::new(bin);
    cmd.arg0(argv0);
    for a in args {
        cmd.arg(osstr(a));
    }
    if let Some(e) = env {
        cmd.env_clear();
        for (k, v) in e {
            cmd.env(k, v);
        }
    }
    cmd.stdin(Stdio::null());
    let o = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));
    Out {
        code: o.status.code(),
        signal: o.status.signal(),
        stdout: o.stdout,
        stderr: o.stderr,
    }
}

/// Core assertion: the two binaries agree byte-for-byte for these arguments.
pub fn assert_same_raw_env(row: &str, args: &[&[u8]], env: Option<&[(&str, &str)]>) -> Out {
    let c = run_raw(&c_bin(), ARGV0, args, env);
    let r = run_raw(&rust_bin(), ARGV0, args, env);
    if c != r {
        let shown: Vec<String> = args.iter().map(|a| esc(a)).collect();
        panic!(
            "[{row}] DIVERGENCE for argv = [{}] env={:?}\n  C    : {:?}\n  RUST : {:?}",
            shown.join(", "),
            env,
            c,
            r
        );
    }
    c
}

pub fn assert_same_raw(row: &str, args: &[&[u8]]) -> Out {
    assert_same_raw_env(row, args, None)
}

/// Convenience for the common two-argument (base, exponent) case.
pub fn assert_same(row: &str, base: &str, exp: &str) -> Out {
    assert_same_raw(row, &[base.as_bytes(), exp.as_bytes()])
}

/// Sanity guard: a differential test that never actually reaches the code under
/// test is worthless, so rows assert on which branch the C took.
pub fn is_result(o: &Out) -> bool {
    o.code == Some(0) && o.stdout.starts_with(b"Result: ") && o.stderr.is_empty()
}
pub fn is_err_exit(o: &Out) -> bool {
    o.code == Some(1) && o.stdout.is_empty() && !o.stderr.is_empty()
}
pub fn stderr_str(o: &Out) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}
pub fn stdout_str(o: &Out) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// Deterministic PRNG (fixed seed → reproducible property-style batches).
pub const SEED: u64 = 0x5EED_1234;

pub struct Rng(u64);

impl Rng {
    pub fn new(salt: u64) -> Self {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// inclusive range
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.below((hi - lo + 1) as u64) as i64)
    }
    pub fn f01(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    /// Random f64 built from raw bits (covers subnormals, inf, NaN).
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
}
