//! Differential-test harness.
//!
//! `c_src/CMakeLists.txt` builds an **executable** (`add_executable(driver …)`),
//! not a shared library, and neither binary exports a callable symbol (see
//! `SYMBOLS.md`).  The ABI under test is therefore the process boundary, so
//! every test here spawns *both* real binaries -- the CMake-built C one and the
//! Cargo-built Rust one -- feeds them the identical stdin bytes and compares
//! stdout, stderr and the exit status / terminating signal byte-for-byte.
//! Nothing is ever called in-process.

#![allow(dead_code)]

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// binaries
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust binary produced by this crate (`[[bin]] name = "driver"`).
///
/// `DRIVER_RUST_BIN` overrides it so the very same suite can be pointed at the
/// release build (`--release` uses `panic = "abort"`, a different profile).
pub fn rust_bin() -> PathBuf {
    match std::env::var_os("DRIVER_RUST_BIN") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_BIN_EXE_driver")),
    }
}

/// The reference C binary; built with CMake on first use.
pub fn c_bin() -> PathBuf {
    let build = manifest_dir().join("c_src/build");
    let exe = build.join("driver");
    if exe.exists() {
        return exe;
    }
    std::fs::create_dir_all(&build).expect("create c_src/build");
    let cfg = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build)
        .output()
        .expect("run cmake (is cmake installed?)");
    assert!(
        cfg.status.success() || exe.exists(),
        "cmake configure failed: {}",
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .current_dir(&build)
        .output()
        .expect("run cmake --build");
    assert!(
        bld.status.success() || exe.exists(),
        "cmake build failed: {}",
        String::from_utf8_lossy(&bld.stderr)
    );
    assert!(exe.exists(), "reference binary was not produced");
    exe
}

// ---------------------------------------------------------------------------
// running
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
pub struct Out {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// `"exit:<code>"` or `"signal:<n>"` -- distinguishes "exited with 139"
    /// from "killed by SIGSEGV", which the C program does.
    pub status: String,
}

pub fn run(bin: &Path, input: &[u8]) -> Out {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // keep `ctime()` output deterministic between the two runs
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", bin.display()));

    // Feed stdin from a helper thread: the child may produce far more output
    // than a pipe buffer holds (or die early -> EPIPE), and we must not
    // deadlock on either.
    let mut sink = child.stdin.take().expect("stdin pipe");
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&data);
        let _ = sink.flush();
        drop(sink);
    });

    let out = child.wait_with_output().expect("wait for child");
    let _ = writer.join();

    let status = match (out.status.code(), out.status.signal()) {
        (Some(c), _) => format!("exit:{c}"),
        (None, Some(s)) => format!("signal:{s}"),
        _ => "unknown".to_string(),
    };
    Out {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

/// `ctime()` embeds the current wall clock, which can tick between the two
/// runs; replace the timestamp with a marker but keep its exact length so a
/// formatting difference is still caught.
pub fn normalize_time(bytes: &[u8]) -> Vec<u8> {
    const TAG: &[u8] = b"Current time: ";
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i..].starts_with(TAG) {
            out.extend_from_slice(TAG);
            i += TAG.len();
            let mut n = 0usize;
            while i < bytes.len() && bytes[i] != b'\n' {
                n += 1;
                i += 1;
            }
            out.extend_from_slice(format!("<ctime:{n}>").as_bytes());
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// comparing
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(2000) {
        match b {
            b'\n' => s.push_str("\\n\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > 2000 {
        s.push_str(&format!("\n… (+{} bytes)", bytes.len() - 2000));
    }
    s
}

fn first_diff(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return i;
        }
    }
    n
}

/// Runs both binaries on `input` and asserts byte-identical behaviour.
/// Returns the reference (C) output so a test can additionally assert *which*
/// documented rejection fired.
pub fn diff_case(label: &str, input: &[u8]) -> Out {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    let cn = normalize_time(&c.stdout);
    let rn = normalize_time(&r.stdout);

    if cn != rn || c.stderr != r.stderr || c.status != r.status {
        let at = first_diff(&cn, &rn);
        let lo = at.saturating_sub(120);
        panic!(
            "\n[{label}] DIVERGENCE\n\
             --- input ({} bytes) ---\n{}\n\
             --- status: C={} RUST={} ---\n\
             --- stdout len: C={} RUST={} ; first difference at byte {at} ---\n\
             --- C   stdout[{lo}..] ---\n{}\n\
             --- RUST stdout[{lo}..] ---\n{}\n\
             --- C stderr ---\n{}\n--- RUST stderr ---\n{}\n",
            input.len(),
            show(input),
            c.status,
            r.status,
            c.stdout.len(),
            r.stdout.len(),
            show(&cn[lo.min(cn.len())..]),
            show(&rn[lo.min(rn.len())..]),
            show(&c.stderr),
            show(&r.stderr),
        );
    }
    c
}

pub fn contains(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Phase C helper: assert the two binaries agree **and** that the reference
/// binary really produced the documented rejection message (so a row cannot be
/// "passed" by a test that never triggers it).
pub fn diff_expect(label: &str, input: &[u8], expect: &[u8]) {
    let c = diff_case(label, input);
    assert!(
        contains(&c.stdout, expect),
        "[{label}] reference output does not contain the expected rejection\n\
         expected: {}\nactual C stdout:\n{}\ninput:\n{}",
        show(expect),
        show(&c.stdout),
        show(input)
    );
}

/// Assert the two binaries agree **and** that the reference binary ended with
/// the given status (e.g. `"signal:11"` for its own buffer-overflow crashes).
pub fn diff_expect_status(label: &str, input: &[u8], status: &str) {
    let c = diff_case(label, input);
    assert_eq!(
        c.status, status,
        "[{label}] reference status {} != expected {status}; input:\n{}",
        c.status,
        show(input)
    );
}

/// Assert agreement and that the reference output does **not** contain a string.
pub fn diff_expect_absent(label: &str, input: &[u8], absent: &[u8]) {
    let c = diff_case(label, input);
    assert!(
        !contains(&c.stdout, absent),
        "[{label}] reference output unexpectedly contains {}\nC stdout:\n{}",
        show(absent),
        show(&c.stdout)
    );
}

/// Convenience: build a script from lines (each gets a trailing `\n`).
pub fn script(lines: &[&str]) -> Vec<u8> {
    let mut v = Vec::new();
    for l in lines {
        v.extend_from_slice(l.as_bytes());
        v.push(b'\n');
    }
    v
}

pub fn diff_lines(label: &str, lines: &[&str]) {
    diff_case(label, &script(lines));
}

// ---------------------------------------------------------------------------
// deterministic RNG (xorshift64*) -- no external crates, fixed seed per row
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    /// uniform in `0..n`
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            lo
        } else {
            lo + self.below(hi - lo)
        }
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
    pub fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    /// A token: never contains ' ', '\t', '\n' or NUL (those are separators /
    /// terminators handled by dedicated tests).
    pub fn token(&mut self, len: usize) -> Vec<u8> {
        const ALPHA: &[u8] = b"abcXYZ019_.-/+";
        (0..len).map(|_| *self.pick(ALPHA)).collect()
    }

    /// A token of random length in `lo..hi`.
    pub fn tok(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.token(n)
    }

    /// A wild token of random length in `lo..hi`.
    pub fn wild_tok(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.wild_token(n)
    }

    /// A token that may contain arbitrary non-separator bytes, including
    /// non-ASCII ones (the C compares with `unsigned char` semantics).
    pub fn wild_token(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| loop {
                let b = (self.next_u64() & 0xff) as u8;
                if b != 0 && b != b' ' && b != b'\t' && b != b'\n' {
                    return b;
                }
            })
            .collect()
    }
}
