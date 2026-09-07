//! Differential test harness: C `libSieve.so` vs Rust `libSieve.so`.
//!
//! Both libraries are exercised **only** through their exported `sieve` symbol,
//! loaded with `libloading`, exactly as an external C consumer would — the Rust
//! implementation is never called directly, so its `#[no_mangle]`
//! `extern "C"` wrapper is under test too.
//!
//! `sieve` returns `void`; its sole observable effect is the bytes it writes to
//! libc `stdout`. Since file descriptor 1 is process-global (and shared with the
//! test harness's own progress output), each measurement is taken in a child
//! process — `examples/runner.rs` — whose stdout is a private pipe.

#![allow(dead_code)] // shared module: each test binary uses a subset

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating artifacts
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find(env_key: &str, candidates: &[PathBuf], hint: &str) -> PathBuf {
    if let Ok(p) = std::env::var(env_key) {
        let p = PathBuf::from(p);
        assert!(p.exists(), "{env_key}={} does not exist", p.display());
        return p;
    }
    for c in candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "could not find any of {:?}.\n{hint}\n(or set {env_key})",
        candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()
    );
}

pub struct Paths {
    pub c_so: PathBuf,
    pub r_so: PathBuf,
    pub runner: PathBuf,
}

pub fn paths() -> &'static Paths {
    static P: OnceLock<Paths> = OnceLock::new();
    P.get_or_init(|| {
        let root = manifest_dir();
        let target = root.join("target");
        let c_so = find(
            "SIEVE_C_SO",
            &[root.parent().unwrap().join("c_src/build/libSieve.so")],
            "Build the C library with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        );
        let r_so = find(
            "SIEVE_RUST_SO",
            &[
                target.join("release/libSieve.so"),
                target.join("debug/libSieve.so"),
            ],
            "Build the Rust library with `cargo build --release`",
        );
        let runner = find(
            "SIEVE_RUNNER",
            &[
                target.join("release/examples/runner"),
                target.join("debug/examples/runner"),
            ],
            "Build the test runner with `cargo build --release --examples`",
        );
        Paths { c_so, r_so, runner }
    })
}

/// Fail loudly if a `.so` under test is older than the source it was built
/// from. `cargo build --release --examples` notably does **not** rebuild the
/// `cdylib`, so without this guard an edit to `src/lib.rs` could be "verified"
/// against a stale artifact and pass vacuously.
pub fn assert_artifacts_fresh() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let p = paths();
        let root = manifest_dir();
        let c_root = root.parent().unwrap().join("c_src");
        let check = |artifact: &Path, sources: &[PathBuf], how: &str| {
            let a = std::fs::metadata(artifact)
                .and_then(|m| m.modified())
                .expect("artifact mtime");
            for s in sources {
                let m = std::fs::metadata(s)
                    .and_then(|m| m.modified())
                    .unwrap_or_else(|e| panic!("source mtime for {}: {e}", s.display()));
                assert!(
                    a >= m,
                    "{} is OLDER than {} — the tests would verify a stale artifact. \
                     Rebuild with: {how}",
                    artifact.display(),
                    s.display(),
                );
            }
        };
        check(
            &p.r_so,
            &[root.join("src/lib.rs"), root.join("Cargo.toml")],
            "cd translation && cargo build --release --lib --examples",
        );
        check(
            &p.c_so,
            &[
                c_root.join("src/sieve.c"),
                c_root.join("include/sieve.h"),
                c_root.join("CMakeLists.txt"),
            ],
            "cd c_src/build && cmake --build .",
        );
        check(
            &p.runner,
            &[root.join("examples/runner.rs")],
            "cd translation && cargo build --release --lib --examples",
        );
    });
}

/// Sanity check performed once: the two libraries must be distinct objects.
/// The C `.so` carries `SONAME libSieve.so`, so if the dynamic loader ever
/// de-duplicated the two `dlopen`s, every comparison in this suite would
/// trivially pass while testing nothing.
pub fn assert_distinct_libraries() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        assert_artifacts_fresh();
        let p = paths();
        unsafe {
            let c = libloading::Library::new(&p.c_so).expect("dlopen C libSieve.so");
            let r = libloading::Library::new(&p.r_so).expect("dlopen Rust libSieve.so");
            let cs: libloading::Symbol<unsafe extern "C" fn(std::ffi::c_int)> =
                c.get(b"sieve\0").expect("C .so must export `sieve`");
            let rs: libloading::Symbol<unsafe extern "C" fn(std::ffi::c_int)> =
                r.get(b"sieve\0").expect("Rust .so must export `sieve`");
            assert_ne!(
                *cs as usize, *rs as usize,
                "C and Rust `sieve` resolved to the same address — comparison would be vacuous"
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Side {
    C,
    Rust,
}

impl Side {
    fn tag(self) -> &'static str {
        match self {
            Side::C => "c",
            Side::Rust => "r",
        }
    }
}

/// One `sieve` invocation: which library, and the argument spelling handed to
/// the runner (decimal, or `0x…` for a raw 32-bit pattern).
#[derive(Clone, Debug)]
pub struct Step(pub Side, pub String);

pub fn step(side: Side, val: i32) -> Step {
    Step(side, val.to_string())
}

pub fn step_raw(side: Side, spelling: &str) -> Step {
    Step(side, spelling.to_string())
}

fn spawn(steps: &[Step]) -> Child {
    assert!(!steps.is_empty(), "no steps: the test would measure nothing");
    let p = paths();
    let mut cmd = Command::new(&p.runner);
    cmd.arg(&p.c_so).arg(&p.r_so);
    for Step(side, val) in steps {
        cmd.arg(format!("{}:{}", side.tag(), val));
    }
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn runner")
}

/// Run a sequence of `sieve` calls to completion in a child process and return
/// every byte written to stdout.
pub fn run(steps: &[Step]) -> Vec<u8> {
    assert_distinct_libraries();
    let mut child = spawn(steps);
    let mut out = Vec::new();
    child
        .stdout
        .take()
        .expect("piped stdout")
        .read_to_end(&mut out)
        .expect("read runner stdout");
    let mut err = String::new();
    let _ = child
        .stderr
        .take()
        .expect("piped stderr")
        .read_to_string(&mut err);
    let status = child.wait().expect("wait for runner");
    assert!(
        status.success(),
        "runner failed for {steps:?}: {status:?}\nstderr: {err}"
    );
    out
}

/// Output of the **C** library for `val`, through its `.so` export.
pub fn c_out(val: i32) -> Vec<u8> {
    run(&[step(Side::C, val)])
}

/// Output of the **Rust** library for `val`, through its `.so` export.
pub fn r_out(val: i32) -> Vec<u8> {
    run(&[step(Side::Rust, val)])
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

fn brief(bytes: &[u8]) -> String {
    const N: usize = 200;
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(N)])
        .replace('\n', "\\n");
    if bytes.len() > N {
        format!("\"{head}\"… ({} bytes total)", bytes.len())
    } else {
        format!("\"{head}\" ({} bytes total)", bytes.len())
    }
}

#[track_caller]
fn assert_bytes_eq(row: &str, what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let at = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c.len().min(r.len()));
        let ctx = at.saturating_sub(40);
        panic!(
            "[{row}] {what} diverged at byte {at}\n  C   : {}\n  Rust: {}\n  C   @{ctx}: {}\n  \
             Rust@{ctx}: {}",
            brief(c),
            brief(r),
            brief(&c[ctx.min(c.len())..]),
            brief(&r[ctx.min(r.len())..]),
        );
    }
}

/// Assert C and Rust emit byte-identical output for `val`, and that the bytes
/// match an independent model of the C's documented behaviour (so a *shared*
/// mistake cannot pass silently).
#[track_caller]
pub fn assert_same(val: i32, row: &str) {
    let c = c_out(val);
    let r = r_out(val);
    assert_bytes_eq(row, &format!("sieve({val})"), &c, &r);
    assert_eq!(
        c,
        expected_output(val).as_bytes(),
        "[{row}] C output for sieve({val}) disagrees with the model derived from the C source"
    );
}

/// Same as [`assert_same`] over a whole batch. One child process per side per
/// value.
#[track_caller]
pub fn assert_same_batch(vals: &[i32], row: &str) {
    assert!(!vals.is_empty(), "[{row}] empty input set — row not exercised");
    for &v in vals {
        assert_same(v, row);
    }
}

/// Independent reimplementation of the behaviour written in `c_src/src/sieve.c`,
/// used as a third opinion. Valid only for starts whose run terminates without
/// signed overflow.
pub fn expected_output(val: i32) -> String {
    let mut s = String::new();
    let mut v: i64 = val as i64;
    loop {
        s.push_str(&format!("{v}\n"));
        // C's `%` truncates toward zero, so a negative `v` yields a
        // non-positive remainder that can never equal 9.
        if v >= 0 && v % 10 == 9 {
            break;
        }
        v += 1;
        assert!(
            v <= i32::MAX as i64,
            "model invoked on a start that overflows ({val}) — use the prefix comparison instead"
        );
    }
    s
}

// ---------------------------------------------------------------------------
// Bounded prefix comparison (for the multi-gigabyte / UB inputs)
// ---------------------------------------------------------------------------

pub struct PrefixResult {
    pub bytes: Vec<u8>,
    /// `true` if the child closed stdout on its own before the byte budget was
    /// reached, i.e. `sieve` returned.
    pub terminated: bool,
}

/// Run `sieve(spelling)` on `side` in a child process, read at most `limit`
/// bytes of stdout, then kill it. Reports whether it finished by itself.
pub fn prefix(side: Side, spelling: &str, limit: usize) -> PrefixResult {
    assert_distinct_libraries();
    let steps = [step_raw(side, spelling)];
    let mut child = spawn(&steps);
    let mut out = child.stdout.take().expect("piped stdout");
    let mut bytes = Vec::with_capacity(limit.min(1 << 20));
    let mut chunk = vec![0u8; 64 * 1024];
    while bytes.len() < limit {
        let want = (limit - bytes.len()).min(chunk.len());
        match out.read(&mut chunk[..want]) {
            Ok(0) => break, // EOF: sieve returned and the process exited
            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
            Err(e) => panic!("reading runner stdout: {e}"),
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(50));
    let terminated = match child.try_wait().expect("try_wait") {
        Some(status) => {
            assert!(
                status.success(),
                "runner for {side:?} sieve({spelling}) exited unsuccessfully ({status:?}) — the \
                 library trapped instead of producing output"
            );
            true
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            false
        }
    };
    drop(out);
    PrefixResult { bytes, terminated }
}

/// Prefix-compare C and Rust for one argument spelling: same bytes, and the
/// same answer to "did it stop within the budget?".
#[track_caller]
pub fn assert_same_prefix(spelling: &str, limit: usize, row: &str) {
    let c = prefix(Side::C, spelling, limit);
    let r = prefix(Side::Rust, spelling, limit);
    assert_eq!(
        c.terminated, r.terminated,
        "[{row}] sieve({spelling}): C terminated={} but Rust terminated={} within the \
         {limit}-byte budget — one side stopped early or trapped",
        c.terminated, r.terminated
    );
    assert_bytes_eq(row, &format!("sieve({spelling}) prefix"), &c.bytes, &r.bytes);
    assert!(
        !c.bytes.is_empty(),
        "[{row}] sieve({spelling}) produced no output — nothing was exercised"
    );
}

// ---------------------------------------------------------------------------
// Symbol inspection
// ---------------------------------------------------------------------------

/// Sorted list of dynamic, defined symbol names exported by `so`.
pub fn exported_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Default for Rng {
    fn default() -> Self {
        Self::new()
    }
}

impl Rng {
    pub const SEED: u64 = 0x5EED_1E55_5EED_1E55;

    pub fn new() -> Self {
        Rng(Self::SEED)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
}
