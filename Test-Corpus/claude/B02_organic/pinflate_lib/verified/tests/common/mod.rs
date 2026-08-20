//! Differential-test harness.
//!
//! Builds the C reference `.so` and the Rust `.so` and compares them **only**
//! through `dlopen`/`dlsym`, never by calling Rust functions directly, so the
//! `#[no_mangle] extern "C"` export wrappers are exercised too.

#![allow(dead_code)]

#[path = "shared.rs"]
pub mod shared;

#[path = "enc.rs"]
pub mod enc;

use shared::{Case, Outcome};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/debug/` (or wherever the test binary lives): the test executable is
/// at `<dir>/deps/<name>-<hash>`, so its grandparent is the profile directory.
pub fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf()
}

fn run(cmd: &mut Command, what: &str) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {what}: {e}"));
    if !out.status.success() {
        panic!(
            "{what} failed ({}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// Builds `c_src` into `target/c_build` with the configuration from the task
/// description. `-S`/`-B` keeps every generated file **outside** `c_src/`.
pub fn c_so() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = manifest_dir().join("c_src");
        let build = manifest_dir().join("target").join("c_build");
        std::fs::create_dir_all(&build).expect("mkdir target/c_build");
        run(
            Command::new("cmake")
                .arg("-S")
                .arg(&src)
                .arg("-B")
                .arg(&build)
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON"),
            "cmake configure",
        );
        run(
            Command::new("cmake").arg("--build").arg(&build),
            "cmake build",
        );
        let so = build.join("libtranslated_rust.so");
        assert!(so.exists(), "C .so not produced at {}", so.display());
        so
    })
    .as_path()
}

/// Builds the Rust `cdylib`. `cargo test` does **not** build `cdylib` output,
/// so it is built here into a private target directory (its own lock, so it
/// cannot deadlock against the `cargo test` invocation that is running us).
pub fn rust_so() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let td = manifest_dir().join("target").join("cdylib_build");
        let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.current_dir(manifest_dir())
            .arg("build")
            .arg("--offline")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&td);
        // don't inherit the outer invocation's per-crate cargo state
        for (k, _) in std::env::vars() {
            if k.starts_with("CARGO_PKG_")
                || k.starts_with("CARGO_CRATE_")
                || k == "CARGO_MANIFEST_DIR"
                || k == "CARGO_TARGET_DIR"
                || k == "RUSTC_WORKSPACE_WRAPPER"
            {
                cmd.env_remove(k);
            }
        }
        run(&mut cmd, "cargo build --lib (cdylib)");
        let so = td.join("debug").join("libpinflate_lib.so");
        assert!(so.exists(), "Rust .so not produced at {}", so.display());
        so
    })
    .as_path()
}

pub fn worker_bin() -> PathBuf {
    let p = profile_dir().join("examples").join("diffworker");
    assert!(
        p.exists(),
        "worker binary missing at {} -- `cargo test` builds examples, so this \
         should exist; run `cargo build --examples` if you invoked the test \
         binary directly",
        p.display()
    );
    p
}

// ---------------------------------------------------------------------------
// Isolated batch runner
// ---------------------------------------------------------------------------

struct Worker {
    child: Child,
    stdout: BufReader<std::process::ChildStdout>,
    /// glibc's `assert()` diagnostic (and Rust's `cp_assert_fail` equivalent) go
    /// to stderr; capturing them to a file lets the comparison check *which*
    /// assert fired, not merely that the process died with SIGABRT.
    stderr_path: PathBuf,
}

impl Worker {
    fn spawn(so: &Path) -> Worker {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = manifest_dir().join("target").join("diffworker-stderr");
        std::fs::create_dir_all(&dir).expect("mkdir diffworker-stderr");
        let stderr_path = dir.join(format!("w-{}-{}.log", std::process::id(), n));
        let errfile = std::fs::File::create(&stderr_path).expect("create stderr log");
        let mut child = Command::new(worker_bin())
            .arg(so)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(errfile))
            .spawn()
            .expect("spawn diffworker");
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Worker { child, stdout, stderr_path }
    }

    fn take_diag(&self) -> Option<String> {
        let text = std::fs::read_to_string(&self.stderr_path).unwrap_or_default();
        let d = shared::extract_assertion(&text);
        let _ = std::fs::remove_file(&self.stderr_path);
        d
    }
}

/// Runs every case against `so` in a crash-isolated worker, restarting the
/// worker after any case that kills it, and returns one `Outcome` per case.
pub fn run_batch(so: &Path, cases: &[Case]) -> Vec<Outcome> {
    let mut results: Vec<Option<Outcome>> = vec![None; cases.len()];
    let mut next = 0usize;

    while next < cases.len() {
        let mut w = Worker::spawn(so);
        let mut i = next;
        let mut line = String::new();
        while i < cases.len() {
            let enc = cases[i].encode();
            let stdin = w.child.stdin.as_mut().unwrap();
            if stdin.write_all(enc.as_bytes()).is_err() || stdin.write_all(b"\n").is_err() {
                break;
            }
            if stdin.flush().is_err() {
                break;
            }
            line.clear();
            match w.stdout.read_line(&mut line) {
                Ok(0) => break, // worker died on case i
                Ok(_) => {}
                Err(_) => break,
            }
            let l = line.trim_end();
            let Some(payload) = l.strip_prefix("#R# ") else {
                panic!("unexpected worker output: {l:?}");
            };
            results[i] = Some(Outcome::decode(payload));
            i += 1;
        }
        drop(w.child.stdin.take());
        let status = w.child.wait().expect("wait diffworker");
        if i < cases.len() {
            use std::os::unix::process::ExitStatusExt;
            let sig = status.signal().unwrap_or_else(|| {
                panic!(
                    "worker for {} exited with {status} without producing a result for case {i} \
                     ({:?})",
                    so.display(),
                    cases[i].label
                )
            });
            let diag = w.take_diag();
            results[i] = Some(Outcome::Signal { sig, diag });
            next = i + 1;
        } else {
            let _ = w.take_diag();
            next = cases.len();
        }
    }
    results.into_iter().map(|r| r.unwrap()).collect()
}

/// The core assertion: for every case, the C `.so` and the Rust `.so` must
/// agree on the return value, on `cp_error_reason`, on the **entire** output
/// buffer (padding included, so out-of-bounds writes are caught), and on
/// whether/how the process died.
pub fn assert_batch_matches(cases: &[Case]) {
    assert!(!cases.is_empty(), "empty case batch");
    let c = run_batch(c_so(), cases);
    let r = run_batch(rust_so(), cases);
    let mut failures = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        if c[i] != r[i] {
            let extra = match (&c[i], &r[i]) {
                (
                    Outcome::Ret { out: co, .. },
                    Outcome::Ret { out: ro, .. },
                ) => {
                    if co.len() != ro.len() {
                        format!(" out len {} vs {}", co.len(), ro.len())
                    } else {
                        match co.iter().zip(ro.iter()).position(|(a, b)| a != b) {
                            Some(k) => format!(
                                " first out byte diff at {k}: C=0x{:02x} R=0x{:02x}",
                                co[k], ro[k]
                            ),
                            None => String::new(),
                        }
                    }
                }
                _ => String::new(),
            };
            failures.push(format!(
                "case[{i}] {:?}\n  in   = {} (in_len={} out_size={} in_off={} out_off={})\n  \
                 C    = {:?}\n  Rust = {:?}{}",
                case.label,
                shared::hex(&case.data),
                case.in_len,
                case.out_size,
                case.in_off,
                case.out_off,
                c[i],
                r[i],
                extra
            ));
            if failures.len() >= 12 {
                break;
            }
        }
    }
    if !failures.is_empty() {
        panic!(
            "{} of {} differential cases diverged:\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n")
        );
    }
}

/// Same, but also asserts the outcome is a normal return with `ret == expect`
/// and the exact `cp_error_reason` text — used by the Phase C error rows so a
/// row cannot "pass" because both sides merely failed somehow.
pub fn assert_error_row(case: Case, expect_ret: i32, expect_err: Option<&str>) {
    let cases = [case];
    assert_batch_matches(&cases);
    let c = run_batch(c_so(), &cases);
    match &c[0] {
        Outcome::Ret { ret, err, .. } => {
            assert_eq!(
                *ret, expect_ret,
                "case {:?}: C returned {ret}, expected {expect_ret}",
                cases[0].label
            );
            let got = err.as_ref().map(|v| String::from_utf8_lossy(v).to_string());
            assert_eq!(
                got.as_deref(),
                expect_err,
                "case {:?}: C cp_error_reason mismatch",
                cases[0].label
            );
        }
        o => panic!("case {:?}: expected a normal return, got {o:?}", cases[0].label),
    }
}

/// Asserts that both libraries die the same way (same signal).
pub fn assert_signal_row(case: Case, expect_signal: i32) {
    let cases = [case];
    assert_batch_matches(&cases);
    let c = run_batch(c_so(), &cases);
    match &c[0] {
        Outcome::Signal { sig, .. } => assert_eq!(
            *sig, expect_signal,
            "case {:?}: C died with signal {sig}, expected {expect_signal}",
            cases[0].label
        ),
        o => panic!(
            "case {:?}: expected the C library to die with signal {expect_signal}, got {o:?}",
            cases[0].label
        ),
    }
}

/// Asserts that both libraries die from the *same* failed `assert()` -- same
/// `lib.c` line, same function, same expression -- not merely both with SIGABRT.
pub fn assert_assert_row(case: Case, line: u32, func: &str, expr: &str) {
    let want = format!("lib.c:{line}: {func}: Assertion `{expr}' failed.");
    let cases = [case];
    assert_batch_matches(&cases);
    for so in [c_so(), rust_so()] {
        let r = run_batch(so, &cases);
        match &r[0] {
            Outcome::Signal { sig, diag } => {
                assert_eq!(
                    *sig,
                    6,
                    "case {:?} on {}: expected SIGABRT, got {sig}",
                    cases[0].label,
                    so.display()
                );
                assert_eq!(
                    diag.as_deref(),
                    Some(want.as_str()),
                    "case {:?} on {}: wrong assert fired",
                    cases[0].label,
                    so.display()
                );
            }
            o => panic!(
                "case {:?} on {}: expected the {want:?} assert, got {o:?}",
                cases[0].label,
                so.display()
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) -- fixed seeds, reproducible runs
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
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// DEFLATE stream construction
// ---------------------------------------------------------------------------

/// Little-endian LSB-first bit writer, matching DEFLATE's bit order.
pub struct BitWriter {
    pub bytes: Vec<u8>,
    nbits: usize,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter { bytes: Vec::new(), nbits: 0 }
    }
    pub fn bit(&mut self, b: u32) {
        if self.nbits % 8 == 0 {
            self.bytes.push(0);
        }
        if b & 1 != 0 {
            let i = self.nbits / 8;
            self.bytes[i] |= 1 << (self.nbits % 8);
        }
        self.nbits += 1;
    }
    /// `n` bits of `v`, least-significant first (DEFLATE's "extra bits" order).
    pub fn bits(&mut self, v: u32, n: usize) {
        for i in 0..n {
            self.bit(v >> i);
        }
    }
    /// A Huffman code, most-significant bit first (DEFLATE's code order).
    pub fn code(&mut self, v: u32, n: usize) {
        for i in (0..n).rev() {
            self.bit(v >> i);
        }
    }
    pub fn align(&mut self) {
        while self.nbits % 8 != 0 {
            self.bit(0);
        }
    }
    pub fn bit_len(&self) -> usize {
        self.nbits
    }
}

/// Canonical Huffman code assignment, identical to `cp_build`'s (and RFC 1951's)
/// rule: shorter codes first, then by symbol.
pub fn canonical_codes(lens: &[u8]) -> Vec<u32> {
    let maxlen = 15usize;
    let mut counts = [0u32; 16];
    for &l in lens {
        if l != 0 {
            counts[l as usize] += 1;
        }
    }
    let mut next = [0u32; 16];
    let mut code = 0u32;
    for l in 1..=maxlen {
        code = (code + counts[l - 1]) << 1;
        next[l] = code;
    }
    let mut out = vec![0u32; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l != 0 {
            out[i] = next[l as usize];
            next[l as usize] += 1;
        }
    }
    out
}

/// The fixed literal/length code lengths, i.e. `cp_fixed_table[0..288]`.
pub fn fixed_lit_lens() -> Vec<u8> {
    let mut v = vec![8u8; 288];
    for i in 144..256 {
        v[i] = 9;
    }
    for i in 256..280 {
        v[i] = 7;
    }
    v
}

/// A stored (`btype == 0`) block.
pub fn stored_block(w: &mut BitWriter, payload: &[u8], bfinal: bool) {
    w.bit(bfinal as u32);
    w.bits(0, 2); // btype = 0
    w.align();
    let len = payload.len() as u16;
    w.bits(len as u32, 16);
    w.bits((!len) as u32, 16);
    for b in payload {
        w.bits(*b as u32, 8);
    }
}
