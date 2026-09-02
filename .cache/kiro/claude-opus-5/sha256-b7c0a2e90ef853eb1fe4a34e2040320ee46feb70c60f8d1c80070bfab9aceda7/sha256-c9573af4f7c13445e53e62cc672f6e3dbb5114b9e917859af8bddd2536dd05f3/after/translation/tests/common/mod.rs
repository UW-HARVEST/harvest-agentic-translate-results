//! Shared machinery for the C-vs-Rust differential tests.
//!
//! Both libraries are exercised *only* through their `.so` exports, loaded with
//! `libloading` — never by calling the Rust crate directly. That way the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.
//!
//! Because every function in this library reports its result on `stdout` (and
//! because `bad()` can corrupt its own stack frame for large indices), each
//! measurement runs in a **fresh child process**: the test binary re-executes
//! itself with `--child <lib> <op> <args…>`, the child `dlopen`s exactly one of
//! the two libraries, performs the calls, flushes and exits. The parent captures
//! the byte stream plus the exit status / fatal signal. One library per process
//! means there is no chance of the dynamic loader interposing C symbols onto the
//! Rust library or vice versa.

#![allow(dead_code)]

use std::ffi::{OsStr, OsString};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Library locations
// ---------------------------------------------------------------------------

/// `translation/` — the crate root.
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C shared object built from `c_src/`.
pub fn c_lib() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("crate root has a parent")
        .join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The Rust `cdylib` under test.
///
/// IMPORTANT: `cargo test` does **not** rebuild a `crate-type = ["cdylib"]`
/// library (an integration test cannot link against a cdylib, so Cargo has no
/// reason to build it). Testing whatever `.so` happens to be lying in `target/`
/// would silently verify a stale artifact — every mutation to `src/lib.rs` would
/// appear to pass. So this function
///   1. runs `cargo build` (matching the current profile) itself, and
///   2. refuses to continue if the resulting `.so` is older than the sources.
pub fn rust_lib() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(build_and_locate_rust_lib).clone()
}

fn profile_dir() -> PathBuf {
    // …/target/<profile>/deps/differential-<hash>  ->  …/target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

/// `true` when the test binary was built with optimisations (release profile).
fn is_release() -> bool {
    profile_dir()
        .file_name()
        .map(|n| n != "debug")
        .unwrap_or(false)
}

fn build_and_locate_rust_lib() -> PathBuf {
    let manifest = manifest_dir();
    let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    cmd.arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(manifest.join("Cargo.toml"));
    if is_release() {
        cmd.arg("--release");
    }
    // Honour the feature selection the test itself was compiled with, so each
    // feature combination tests the cdylib built the same way.
    if let Some(f) = std::env::var_os("DRIVER_TEST_CARGO_FLAGS") {
        for part in f.to_string_lossy().split_whitespace() {
            cmd.arg(part);
        }
    }
    let out = cmd.output().expect("run `cargo build --lib`");
    if !out.status.success() {
        panic!(
            "`cargo build --lib` failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let path = profile_dir().join("libdriver.so");
    assert!(
        path.is_file(),
        "Rust cdylib not found at {} after `cargo build --lib`",
        path.display()
    );

    // Staleness guard: the .so must be at least as new as every crate source.
    let so_mtime = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");
    for src in [manifest.join("src/lib.rs"), manifest.join("Cargo.toml")] {
        let src_mtime = std::fs::metadata(&src)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("stat {}: {e}", src.display()));
        assert!(
            so_mtime >= src_mtime,
            "STALE ARTIFACT: {} is older than {} — the differential test would be \
             verifying an out-of-date library. Run `cargo build{}` and retry.",
            path.display(),
            src.display(),
            if is_release() { " --release" } else { "" }
        );
    }
    path
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// One call into the library under test, encoded so it can cross an `argv`
/// boundary. Payload strings are hex-encoded so arbitrary (non-UTF-8) bytes
/// survive the trip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// `printLine(NULL)`
    PrintLineNull,
    /// `printLine(<bytes>)` — `bytes` must not contain an interior NUL.
    PrintLine(Vec<u8>),
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
    /// One line of the child's spec file. Payloads are hex so arbitrary
    /// (non-UTF-8) bytes survive; a file is used instead of `argv` because the
    /// 64 KiB+ `printLine` payloads exceed `ARG_MAX`.
    fn to_spec(&self) -> String {
        match self {
            Op::PrintLineNull => "printline NULL".to_string(),
            Op::PrintLine(b) => format!("printline {}", hex_encode(b)),
            Op::PrintIntLine(n) => format!("printintline {n}"),
            Op::Bad(n) => format!("bad {n}"),
            Op::Good(n) => format!("good {n}"),
            Op::Driver(g, b) => format!("driver {g} {b}"),
        }
    }

    /// Short human-readable form used in failure reports.
    pub fn label(&self) -> String {
        match self {
            Op::PrintLineNull => "printLine(NULL)".to_string(),
            Op::PrintLine(b) => {
                if b.len() > 24 {
                    format!("printLine(<{} bytes>)", b.len())
                } else {
                    format!("printLine({:?})", String::from_utf8_lossy(b))
                }
            }
            Op::PrintIntLine(n) => format!("printIntLine({n})"),
            Op::Bad(n) => format!("bad({n})"),
            Op::Good(n) => format!("good({n})"),
            Op::Driver(g, b) => format!("driver({g}, {b})"),
        }
    }
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    s
}

pub fn hex_decode(raw: &[u8]) -> Vec<u8> {
    assert!(raw.len() % 2 == 0, "odd-length hex payload");
    let nib = |c: u8| (c as char).to_digit(16).expect("hex digit") as u8;
    raw.chunks(2).map(|c| (nib(c[0]) << 4) | nib(c[1])).collect()
}

// ---------------------------------------------------------------------------
// Running
// ---------------------------------------------------------------------------

/// What a child process produced.
#[derive(Clone, PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    /// Normal exit code, if the process exited normally.
    pub code: Option<i32>,
    /// Fatal signal number, if the process was killed.
    pub signal: Option<i32>,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        self.code == Some(0)
    }
    pub fn describe(&self) -> String {
        let status = match (self.code, self.signal) {
            (Some(c), _) => format!("exit {c}"),
            (_, Some(s)) => format!("signal {s}"),
            _ => "unknown status".to_string(),
        };
        format!(
            "[{status}] {} byte(s): {:?}",
            self.stdout.len(),
            String::from_utf8_lossy(&truncate(&self.stdout, 400))
        )
    }
}

fn truncate(b: &[u8], n: usize) -> Vec<u8> {
    if b.len() <= n {
        b.to_vec()
    } else {
        let mut v = b[..n].to_vec();
        v.extend_from_slice(b"...<truncated>");
        v
    }
}

/// Run a batch of operations, in order, inside one child process holding `lib`.
///
/// Batching keeps the process count sane while additionally exercising the
/// *shared stdout stream* across successive calls (ordering / flush behaviour).
pub fn run_batch(lib: &Path, ops: &[Op]) -> Outcome {
    let spec: String = ops
        .iter()
        .map(|o| o.to_spec())
        .collect::<Vec<_>>()
        .join("\n");
    let path = temp_spec_path();
    std::fs::write(&path, spec.as_bytes()).expect("write spec file");

    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .arg("--child")
        .arg(lib)
        .arg(&path)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("spawn child");
    let _ = std::fs::remove_file(&path);
    Outcome {
        stdout: out.stdout,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn temp_spec_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-difftest-{}-{}.spec",
        std::process::id(),
        n
    ))
}

pub fn run_one(lib: &Path, op: &Op) -> Outcome {
    run_batch(lib, std::slice::from_ref(op))
}

// ---------------------------------------------------------------------------
// Child-side dispatch
// ---------------------------------------------------------------------------

type FnPrintLine = unsafe extern "C" fn(*const std::ffi::c_char);
type FnPrintIntLine = unsafe extern "C" fn(std::ffi::c_int);
type FnBad = unsafe extern "C" fn(std::ffi::c_int);
type FnGood = unsafe extern "C" fn(std::ffi::c_int);
type FnDriver = unsafe extern "C" fn(std::ffi::c_int, std::ffi::c_int);

unsafe extern "C" {
    fn fflush(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
}

/// If invoked as `<exe> --child <lib> <specfile>`, perform the calls and exit.
/// Returns normally when this process is the parent.
pub fn maybe_run_as_child() {
    let argv: Vec<OsString> = std::env::args_os().collect();
    if argv.len() < 2 || argv[1] != OsStr::new("--child") {
        return;
    }
    assert_eq!(argv.len(), 4, "usage: --child <lib> <specfile>");
    let lib_path = PathBuf::from(&argv[2]);
    let spec = std::fs::read(&argv[3]).expect("read spec file");

    let lib = unsafe { libloading::Library::new(&lib_path) }
        .unwrap_or_else(|e| panic!("dlopen {}: {e}", lib_path.display()));

    // Resolve every symbol up front: this is also a hard check that the `.so`
    // really exports all five names.
    let print_line = unsafe { lib.get::<FnPrintLine>(b"printLine\0") }.expect("printLine");
    let print_int_line =
        unsafe { lib.get::<FnPrintIntLine>(b"printIntLine\0") }.expect("printIntLine");
    let bad = unsafe { lib.get::<FnBad>(b"bad\0") }.expect("bad");
    let good = unsafe { lib.get::<FnGood>(b"good\0") }.expect("good");
    let driver = unsafe { lib.get::<FnDriver>(b"driver\0") }.expect("driver");

    for line in spec.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&[u8]> = line.split(|&b| b == b' ').collect();
        let int = |i: usize| -> i32 {
            std::str::from_utf8(parts[i])
                .expect("utf8 int arg")
                .parse()
                .expect("i32 arg")
        };
        unsafe {
            match parts[0] {
                b"printline" => {
                    if parts[1] == b"NULL" {
                        print_line(std::ptr::null());
                    } else {
                        let mut bytes = hex_decode(parts[1]);
                        bytes.push(0);
                        print_line(bytes.as_ptr() as *const std::ffi::c_char);
                    }
                }
                b"printintline" => print_int_line(int(1)),
                b"bad" => bad(int(1)),
                b"good" => good(int(1)),
                b"driver" => driver(int(1), int(2)),
                other => panic!("unknown op {:?}", String::from_utf8_lossy(other)),
            }
        }
    }

    // Flush libc's stdout (the libraries write through `printf`/`puts`, which
    // is fully buffered when stdout is a pipe) before leaving.
    unsafe {
        fflush(std::ptr::null_mut());
    }
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `lo..=hi` (inclusive), works across the whole `i32` range.
    pub fn in_range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

pub struct Report {
    pub passed: usize,
    pub failed: Vec<String>,
    pub notes: Vec<String>,
}

impl Report {
    pub fn new() -> Self {
        Report {
            passed: 0,
            failed: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Compare one batch across the two libraries byte-for-byte (stdout *and*
    /// exit status). `row` identifies the CONFIGS.md / ERRORS.md row.
    pub fn diff(&mut self, row: &str, ops: &[Op]) {
        let c = run_batch(&c_lib(), ops);
        let r = run_batch(&rust_lib(), ops);
        if c == r {
            self.passed += 1;
            return;
        }
        // Localise: replay each op on its own to find the first divergence.
        let mut detail = String::new();
        for op in ops {
            let c1 = run_one(&c_lib(), op);
            let r1 = run_one(&rust_lib(), op);
            if c1 != r1 {
                detail = format!(
                    "  first diverging call: {}\n    C   : {}\n    Rust: {}",
                    op.label(),
                    c1.describe(),
                    r1.describe()
                );
                break;
            }
        }
        if detail.is_empty() {
            detail = format!(
                "  batch-level divergence only (ordering/flush)\n    C   : {}\n    Rust: {}",
                c.describe(),
                r.describe()
            );
        }
        self.failed.push(format!("{row}\n{detail}"));
    }

    pub fn note(&mut self, s: String) {
        self.notes.push(s);
    }

    pub fn check(&mut self, row: &str, cond: bool, msg: &str) {
        if cond {
            self.passed += 1;
        } else {
            self.failed.push(format!("{row}\n  {msg}"));
        }
    }

    pub fn finish(&self, title: &str) -> bool {
        println!("\n===== {title} =====");
        for n in &self.notes {
            println!("note: {n}");
        }
        println!("checks passed : {}", self.passed);
        println!("checks failed : {}", self.failed.len());
        for f in &self.failed {
            println!("FAIL {f}");
        }
        self.failed.is_empty()
    }
}
