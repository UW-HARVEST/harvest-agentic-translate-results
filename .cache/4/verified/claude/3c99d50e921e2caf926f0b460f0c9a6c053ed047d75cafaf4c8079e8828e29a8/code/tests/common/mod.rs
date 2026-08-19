//! Shared differential-test harness for the StaticLoop library.
//!
//! Both the **C** shared library (`c_src/build/libStaticLoop.so`) and the
//! **Rust** shared library (`target/<profile>/libStaticLoop.so`) are loaded with
//! `libloading` and driven exclusively through their exported C symbols — the
//! Rust functions are never called directly, so the `#[unsafe(no_mangle)]`
//! `extern "C"` wrappers are part of what is under test.
//!
//! ## Why every call is paired and serialized
//!
//! `static_sum` keeps a *hidden* `static int sum` per loaded library. The two
//! libraries each own their own copy. The harness therefore applies **the exact
//! same operation sequence, in the same order, to C first and then to Rust**,
//! under a process-wide mutex. That keeps the two accumulators in lockstep, so
//! any divergence in a single call is caught immediately (and the absolute value
//! of the accumulator never has to be assumed).
//!
//! `driver` writes to `stdout` via `printf`. Its output is compared
//! byte-for-byte by redirecting file descriptor 1 to a temporary file around
//! each call (fd-level redirection, so the C library's `printf` is captured too).

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type SumFn = unsafe extern "C" fn(c_int) -> c_int;
pub type DriverFn = unsafe extern "C" fn(c_int);

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

/// Crate root (`translated_rust/`).
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library, built by CMake.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("STATICLOOP_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libStaticLoop.so")
}

/// Path to the Rust shared library (`cdylib`).
///
/// Prefers the build profile the current test binary was compiled with, so
/// `cargo test` and `cargo test --release` each exercise their own `.so`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("STATICLOOP_RUST_SO") {
        return PathBuf::from(p);
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    // target/<profile>/deps/<test-bin>  ->  target/<profile>/libStaticLoop.so
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            candidates.push(deps.join("libStaticLoop.so"));
            if let Some(profile_dir) = deps.parent() {
                candidates.push(profile_dir.join("libStaticLoop.so"));
            }
        }
    }
    candidates.push(manifest_dir().join("target/debug/libStaticLoop.so"));
    candidates.push(manifest_dir().join("target/release/libStaticLoop.so"));

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }

    // Last resort: build it. (`run_tests.sh` normally does this up front.)
    let _ = std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["build", "--offline"])
        .current_dir(manifest_dir())
        .status();
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }

    panic!(
        "could not locate the Rust libStaticLoop.so; tried {:?}. \
         Run `cargo build` first or set STATICLOOP_RUST_SO.",
        candidates
    );
}

pub fn load(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "shared library not found: {} — build it first",
        path.display()
    );
    unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
}

/// `dlopen`s a *pristine copy* of `src` from a unique temporary path, so the
/// dynamic loader really creates a new mapping (dlopen'ing the same path twice
/// just bumps a refcount and keeps the old, already-mutated `static` data).
///
/// Used to verify that both libraries start with a zero-initialised
/// accumulator.
pub fn load_fresh_copy(src: &Path, tag: &str) -> (Library, PathBuf) {
    let seq = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let dst = std::env::temp_dir().join(format!(
        "staticloop_fresh_{tag}_{}_{seq}.so",
        std::process::id()
    ));
    std::fs::copy(src, &dst)
        .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    let lib = load(&dst);
    (lib, dst)
}

pub fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe { lib.get(name) }.unwrap_or_else(|e| {
        panic!(
            "missing exported symbol {:?}: {e}",
            String::from_utf8_lossy(name)
        )
    });
    *s
}

// ---------------------------------------------------------------------------
// stdout capture (fd level, so C `printf` output is captured too)
// ---------------------------------------------------------------------------

static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// every byte that was written to it.
pub fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    let seq = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "staticloop_capture_{}_{}.txt",
        std::process::id(),
        seq
    ));

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("cannot create capture file {}: {e}", path.display()));

    unsafe {
        // Flush anything already buffered so it is not mis-attributed:
        // `fflush(NULL)` for the C stdio buffers that both libraries share, and
        // Rust's own `Stdout` line buffer for the test harness's progress text.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        // The library's stdout is fully buffered when it points at a file.
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
    }

    let mut buf = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek capture file");
    file.read_to_end(&mut buf).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&path);
    buf
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds keep every row reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform over the whole `i32` range.
    pub fn i32_any(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform over the inclusive range `[lo, hi]`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// The harness itself
// ---------------------------------------------------------------------------

pub struct Harness {
    // Keep the libraries alive for the whole process.
    _c_lib: Library,
    _r_lib: Library,
    c_sum: SumFn,
    r_sum: SumFn,
    c_driver: DriverFn,
    r_driver: DriverFn,
    /// Number of paired operations performed so far (for diagnostics).
    pub ops: u64,
}

static HARNESS: OnceLock<Mutex<Harness>> = OnceLock::new();

/// Locks the (process-global) harness. Every test uses this, which serializes
/// all operations and keeps the two hidden accumulators in lockstep.
pub fn harness() -> MutexGuard<'static, Harness> {
    let m = HARNESS.get_or_init(|| {
        // `driver` output is compared by redirecting fd 1, which is a
        // process-wide resource: libtest's own progress text would otherwise
        // land inside a capture window. `.cargo/config.toml` sets
        // RUST_TEST_THREADS=1 for exactly this reason.
        assert_eq!(
            std::env::var("RUST_TEST_THREADS").as_deref(),
            Ok("1"),
            "these differential tests capture file descriptor 1 and must run \
             sequentially; use ./run_tests.sh (or `cargo test -- --test-threads=1`)"
        );
        let c_path = c_so_path();
        let r_path = rust_so_path();
        let c_lib = load(&c_path);
        let r_lib = load(&r_path);
        let h = Harness {
            c_sum: sym(&c_lib, b"static_sum\0"),
            r_sum: sym(&r_lib, b"static_sum\0"),
            c_driver: sym(&c_lib, b"driver\0"),
            r_driver: sym(&r_lib, b"driver\0"),
            _c_lib: c_lib,
            _r_lib: r_lib,
            ops: 0,
        };
        Mutex::new(h)
    });
    // A failing assertion in one test poisons the mutex; recover so the
    // remaining rows still report their own results.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Harness {
    /// `static_sum(update)` on C then on Rust; asserts the two returns match.
    pub fn sum(&mut self, update: c_int) -> c_int {
        let c = unsafe { (self.c_sum)(update) };
        let r = unsafe { (self.r_sum)(update) };
        assert_eq!(
            c, r,
            "static_sum({update}) diverged at op #{}: C returned {c}, Rust returned {r}",
            self.ops
        );
        self.ops += 1;
        c
    }

    /// `driver(stride)` on C then on Rust; asserts the captured stdout bytes
    /// match exactly. Returns the (identical) bytes.
    pub fn driver(&mut self, stride: c_int) -> Vec<u8> {
        let cd = self.c_driver;
        let rd = self.r_driver;
        let c_out = capture_stdout(move || unsafe { cd(stride) });
        let r_out = capture_stdout(move || unsafe { rd(stride) });
        assert_eq!(
            c_out,
            r_out,
            "driver({stride}) stdout diverged at op #{}:\n  C   = {:?}\n  Rust= {:?}",
            self.ops,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
        self.ops += 1;
        c_out
    }

    /// Current accumulator value. `static_sum(0)` is side-effect free
    /// (`sum += 0`) and returns the running total, so this also differentially
    /// checks that both libraries agree on their state.
    pub fn state(&mut self) -> c_int {
        self.sum(0)
    }

    /// Drives both accumulators to exactly `target` using one paired
    /// `static_sum` call, and verifies both got there.
    pub fn set_state(&mut self, target: c_int) {
        let cur = self.state();
        let delta = target.wrapping_sub(cur);
        let got = self.sum(delta);
        assert_eq!(
            got, target,
            "set_state({target}) failed: cur={cur} delta={delta} got={got}"
        );
    }
}

/// Splits captured `driver` output into its lines (without the `\n`).
pub fn lines_of(out: &[u8]) -> Vec<String> {
    let s = String::from_utf8(out.to_vec()).expect("driver output must be valid UTF-8/ASCII");
    assert!(
        s.is_empty() || s.ends_with('\n'),
        "driver output must be newline terminated, got {s:?}"
    );
    s.lines().map(|l| l.to_string()).collect()
}

/// The exact bytes the C `driver(stride)` must print when the accumulator
/// starts at `start`: ten `%d\n` lines of the running total, all arithmetic
/// wrapping two's-complement exactly like the `-O0` C build.
pub fn expected_driver_output(start: c_int, stride: c_int) -> Vec<u8> {
    let mut sum = start;
    let mut out = String::new();
    for i in 0..10i32 {
        sum = sum.wrapping_add(i.wrapping_mul(stride));
        out.push_str(&format!("{sum}\n"));
    }
    out.into_bytes()
}
