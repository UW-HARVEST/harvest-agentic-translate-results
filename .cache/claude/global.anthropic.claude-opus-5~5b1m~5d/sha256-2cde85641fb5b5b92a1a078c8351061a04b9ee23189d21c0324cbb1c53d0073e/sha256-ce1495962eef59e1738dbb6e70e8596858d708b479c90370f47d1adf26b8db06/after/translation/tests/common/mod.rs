//! Shared differential-test harness.
//!
//! Both shared objects are exercised **only** through their exported C ABI
//! symbols, loaded at runtime with `libloading`:
//!
//! * `c_src/build/libdriver.so`               — the ground truth
//! * `translation/target/<profile>/libdriver.so` — the Rust translation
//!
//! Nothing in these tests calls a Rust function directly, so the
//! `#[no_mangle] extern "C"` export wrappers are part of what is under test.
//!
//! ## Why the calls happen in a child process
//!
//! The library under test writes to `stdout` with the C runtime's `printf`, and
//! `bad()` deliberately performs an out-of-bounds stack write (CWE-121), which
//! for some indices *kills the process*. Each call is therefore replayed by the
//! `probe` example binary (see `examples/probe.rs`), which `dlopen`s exactly one
//! of the two objects. That gives us three things at once:
//!
//! 1. byte-exact capture of everything the library printed,
//! 2. the exit status / fatal signal, so "C crashed, Rust did not" is a
//!    detectable difference rather than a dead test runner,
//! 3. no symbol interposition — the two objects export identical names
//!    (`driver`, `bad`, ...), so loading both into one process and hoping the
//!    right one is called would be unsound.
//!
//! `dual_load()` additionally loads *both* objects into the test process at once
//! and resolves every exported symbol from each, proving the Rust `.so`'s export
//! table is callable exactly like the C one.

#![allow(dead_code)]

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Artifact discovery / build
// ---------------------------------------------------------------------------

pub struct Artifacts {
    pub c_so: PathBuf,
    pub rust_so: PathBuf,
    pub probe: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir().parent().expect("repo root").to_path_buf()
}

/// `target/<profile>` directory, derived from the running test executable
/// (`target/<profile>/deps/<name>-<hash>`).
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

/// Builds the C shared object with CMake, exactly as the task describes.
/// `c_src/` itself is never modified — only the ignored `c_src/build/` tree.
fn build_c_so() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let so = build.join("libdriver.so");
    if so.exists() {
        return so;
    }
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let cfg = Command::new("cmake")
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .current_dir(&build)
        .output()
        .expect("run cmake (is cmake installed?)");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("run cmake --build");
    assert!(
        bld.status.success(),
        "cmake --build failed:\n{}",
        String::from_utf8_lossy(&bld.stderr)
    );
    assert!(so.exists(), "cmake did not produce {}", so.display());
    so
}

/// Builds the Rust `cdylib` and the `probe` example for the profile the tests
/// are running under. `cargo test --test <x>` does not build either on its own.
fn build_rust_artifacts() -> (PathBuf, PathBuf) {
    let profile_dir = target_profile_dir();
    let so = profile_dir.join("libdriver.so");
    let probe = profile_dir.join("examples").join("probe");

    let release = profile_dir.file_name().map(|n| n == "release").unwrap_or(false);
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["build", "--offline", "--lib", "--example", "probe"]);
    if release {
        cmd.arg("--release");
    }
    let out = cmd
        .current_dir(manifest_dir())
        .output()
        .expect("spawn cargo build");
    assert!(
        out.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(so.exists(), "missing Rust cdylib at {}", so.display());
    assert!(probe.exists(), "missing probe at {}", probe.display());
    (so, probe)
}

pub fn artifacts() -> &'static Artifacts {
    static ARTIFACTS: OnceLock<Artifacts> = OnceLock::new();
    ARTIFACTS.get_or_init(|| {
        let c_so = build_c_so();
        let (rust_so, probe) = build_rust_artifacts();
        Artifacts { c_so, rust_so, probe }
    })
}

// ---------------------------------------------------------------------------
// Op script construction
// ---------------------------------------------------------------------------

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// One call into the library under test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// `printLine(<bytes>)` — the bytes must not contain an interior NUL.
    PrintLine(Vec<u8>),
    /// `printLine(NULL)`
    PrintLineNull,
    /// `printIntLine(n)`
    PrintIntLine(i32),
    /// `bad(n)`
    Bad(i32),
    /// `good(n)`
    Good(i32),
    /// `driver(good_data, bad_data)`
    Driver(i32, i32),
}

impl Op {
    fn encode(&self) -> String {
        match self {
            Op::PrintLine(b) => {
                assert!(
                    !b.contains(&0),
                    "a C string cannot carry an interior NUL byte"
                );
                format!("L {}", hex(b))
            }
            Op::PrintLineNull => "N".to_string(),
            Op::PrintIntLine(n) => format!("I {n}"),
            Op::Bad(n) => format!("B {n}"),
            Op::Good(n) => format!("G {n}"),
            Op::Driver(a, b) => format!("D {a} {b}"),
        }
    }

    pub fn print_line(s: &str) -> Op {
        Op::PrintLine(s.as_bytes().to_vec())
    }
}

pub fn script(ops: &[Op]) -> String {
    let mut s = String::new();
    for op in ops {
        s.push_str(&op.encode());
        s.push('\n');
    }
    s
}

// ---------------------------------------------------------------------------
// Running
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Everything the library printed, byte for byte.
    pub stdout: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    pub code: Option<i32>,
    /// `Some(signo)` if the process was killed by a fatal signal.
    pub signal: Option<i32>,
}

impl Outcome {
    pub fn crashed(&self) -> bool {
        self.signal.is_some()
    }
    pub fn lines(&self) -> Vec<&[u8]> {
        if self.stdout.is_empty() {
            return vec![];
        }
        let body = self
            .stdout
            .strip_suffix(b"\n")
            .unwrap_or(&self.stdout);
        body.split(|&b| b == b'\n').collect()
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

fn run_probe(so: &Path, script: &str, unbuffered: bool) -> Outcome {
    let a = artifacts();
    let mut cmd = Command::new(&a.probe);
    cmd.arg(so);
    if unbuffered {
        cmd.arg("flush");
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn probe");
    {
        let mut sin = child.stdin.take().expect("stdin");
        // A large script can exceed the pipe buffer, so write from a helper
        // thread to avoid deadlocking against the child's stdout.
        let s = script.to_owned();
        std::thread::spawn(move || {
            let _ = sin.write_all(s.as_bytes());
        });
    }
    let out = child.wait_with_output().expect("wait probe");
    Outcome {
        stdout: out.stdout,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

pub fn run_c(script: &str) -> Outcome {
    run_probe(&artifacts().c_so, script, false)
}

pub fn run_rust(script: &str) -> Outcome {
    run_probe(&artifacts().rust_so, script, false)
}

/// Runs `ops` against both objects and returns `(c, rust)`.
pub fn run_both(ops: &[Op]) -> (Outcome, Outcome) {
    let s = script(ops);
    (run_c(&s), run_rust(&s))
}

/// Like [`run_both`], but with `stdout` unbuffered, so output produced inside a
/// call that later dies on `ret` is still captured. Used to compare what the
/// library *printed* independently of whether the process survived.
pub fn run_both_unbuffered(ops: &[Op]) -> (Outcome, Outcome) {
    let s = script(ops);
    let a = artifacts();
    (
        run_probe(&a.c_so, &s, true),
        run_probe(&a.rust_so, &s, true),
    )
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn render_diff(label: &str, ops: &[Op], c: &Outcome, r: &Outcome) -> String {
    let cl = c.lines();
    let rl = r.lines();
    let mut msg = format!(
        "DIVERGENCE in {label}\n\
         ops ({} total, first 8 shown): {:?}\n\
         C   : {} bytes, {} lines, code={:?} signal={:?}\n\
         Rust: {} bytes, {} lines, code={:?} signal={:?}\n",
        ops.len(),
        &ops[..ops.len().min(8)],
        c.stdout.len(),
        cl.len(),
        c.code,
        c.signal,
        r.stdout.len(),
        rl.len(),
        r.code,
        r.signal,
    );
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).map(|b| String::from_utf8_lossy(b).into_owned());
        let b = rl.get(i).map(|b| String::from_utf8_lossy(b).into_owned());
        if a != b {
            msg.push_str(&format!(
                "first differing output line #{i}:\n  C   : {:?}\n  Rust: {:?}\n",
                a.as_deref().unwrap_or("<missing>"),
                b.as_deref().unwrap_or("<missing>")
            ));
            break;
        }
    }
    msg
}

/// The core Phase B / Phase C assertion: identical stdout bytes *and* identical
/// termination status.
pub fn assert_same(label: &str, ops: &[Op]) {
    let (c, r) = run_both(ops);
    assert!(
        c.stdout == r.stdout && c.code == r.code && c.signal == r.signal,
        "{}",
        render_diff(label, ops, &c, &r)
    );
}

/// Same as [`assert_same`], but additionally asserts neither side crashed, for
/// configurations that must be entirely well-defined.
pub fn assert_same_clean(label: &str, ops: &[Op]) {
    let (c, r) = run_both(ops);
    assert!(
        !c.crashed(),
        "{label}: the C reference crashed with signal {:?}; this input was \
         expected to be well-defined",
        c.signal
    );
    assert!(
        c.stdout == r.stdout && c.code == r.code && c.signal == r.signal,
        "{}",
        render_diff(label, ops, &c, &r)
    );
}

/// Runs each op on its own (one process per op) instead of batching. Slower, but
/// it isolates a crash to a single call and localises any divergence exactly.
pub fn assert_same_isolated(label: &str, ops: &[Op]) {
    for (i, op) in ops.iter().enumerate() {
        let one = [op.clone()];
        let (c, r) = run_both(&one);
        assert!(
            c.stdout == r.stdout && c.code == r.code && c.signal == r.signal,
            "{}",
            render_diff(&format!("{label} [op #{i}: {op:?}]"), &one, &c, &r)
        );
    }
}

// ---------------------------------------------------------------------------
// Undefined-behaviour classification (see UB.md)
// ---------------------------------------------------------------------------

/// `bad(data)` writes `buffer[data]` with **no upper bound check**. In the
/// reference build (`gcc -O0`, `buffer` at `-0x30(%rbp)`) index `data` lands at
/// `rbp - 0x30 + 4*data`, so:
///
/// | `data`   | target                        | consequence            |
/// |----------|-------------------------------|------------------------|
/// | `0..=9`  | `buffer` itself               | in bounds              |
/// | `10`     | frame padding                 | benign                 |
/// | `11`     | the loop counter `i`          | benign (`i` is re-initialised) |
/// | `12..=13`| **saved `rbp`**               | caller's frame pointer destroyed |
/// | `14..=15`| **return address**            | fatal on `ret`         |
/// | `>=16`   | the caller's frame, and its caller's, ... | corrupts whoever is up the stack |
///
/// From `data >= 12` onwards the write lands *outside `bad`'s own frame*, in
/// state that belongs to the caller. Which indices are then fatal depends
/// entirely on the frame layout of the whole call chain, so it differs between
/// the two builds in both directions — measured over `data` in `10..=700`:
///
/// ```text
/// bad(d)       C dies at: 12 13 14 15 202..205 208 209 222 223 240 241 246 247 342..345 ...
/// bad(d)    Rust dies at: 134 135 322..325 328 329 342 343 360 361 366 367 462..465 ...
/// driver(7,d)  C dies at: 12..15 20..23 210..213 216 217 230 231 248 249 ...
/// driver(7,d) Rust dies at: 132..135 140..143 330..333 336 337 350 351 ...
/// ```
///
/// No translation can reproduce that pattern; it is a property of the C
/// compiler's stack layout, not of the program. `data >= 12` is therefore
/// excluded from byte-exact status comparison and covered instead by
/// `tests/phase_b_ub.rs`, which pins down the invariant that *does* hold: what
/// `bad()` itself prints is byte-identical in both builds. See `UB.md`.
pub fn bad_index_is_ub_on_caller(data: i32) -> bool {
    !bad_index_is_comparable(data)
}

/// Largest `bad()` index that still lands inside `bad`'s own stack frame, and so
/// cannot disturb the caller: `buffer[11]` aliases the loop counter `i` at
/// `-0x4(%rbp)`, and `buffer[12]` is already the saved `rbp`.
pub const BAD_SELF_FRAME_MAX: i32 = 11;

/// Indices for which `bad()` is fully comparable (identical stdout *and*
/// identical exit status): the guarded negative branch, the ten in-bounds
/// indices, and the two overflow slots that still fall inside `bad`'s own frame.
pub fn bad_index_is_comparable(data: i32) -> bool {
    data <= BAD_SELF_FRAME_MAX
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (property-style testing with a fixed seed)
// ---------------------------------------------------------------------------

/// SplitMix64 — tiny, deterministic, dependency-free.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
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
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn usize_range(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next_u64() % (hi - lo + 1) as u64) as usize
    }
    /// A random byte string with no interior NUL (valid C string content).
    pub fn c_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                // 1..=255, never 0
                (self.next_u64() % 255) as u8 + 1
            })
            .collect()
    }
    /// Random printable ASCII (0x20..=0x7E).
    pub fn ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| 0x20 + (self.next_u64() % 0x5F) as u8)
            .collect()
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}
