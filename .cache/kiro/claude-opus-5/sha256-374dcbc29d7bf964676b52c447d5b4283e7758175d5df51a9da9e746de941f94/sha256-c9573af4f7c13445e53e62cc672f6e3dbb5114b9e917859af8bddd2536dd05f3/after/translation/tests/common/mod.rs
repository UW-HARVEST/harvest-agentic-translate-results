//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls them only through
//! their exported C symbols, exactly as an external consumer would. The Rust
//! functions are never called directly, so the `#[no_mangle]` / `extern "C"`
//! export wrappers are part of what is under test.
//!
//! Both libraries keep their own copy of the `static` accumulator inside
//! `static_sum`. Every test therefore drives *the identical call sequence into
//! both libraries in lockstep*, comparing after each step. That makes the tests
//! order-independent (no `dlclose` state reset is required) while still
//! detecting any divergence at the exact call where it first appears.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits needed to capture the `printf` output of `driver`
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open C output stream, which is exactly
    /// what we need: both `.so`s write through the one process-wide glibc
    /// `stdout` buffer.
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libStaticLoop.so`, built by CMake.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("STATICLOOP_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libStaticLoop.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
    );
    p
}

/// `translation/target/{debug,release}/libStaticLoop.so`, built by cargo.
///
/// Set `STATICLOOP_RUST_SO` to pin a specific one (used to run the whole suite
/// against the release `cdylib` as well as the debug one).
pub fn rust_so_path() -> PathBuf {
    let path = rust_so_path_raw();
    assert_cdylib_is_fresh(&path);
    path
}

fn rust_so_path_raw() -> PathBuf {
    if let Ok(p) = std::env::var("STATICLOOP_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "STATICLOOP_RUST_SO does not exist: {p:?}");
        return p;
    }
    let target = manifest_dir().join("target");
    for profile in ["debug", "release"] {
        let p = target.join(profile).join("libStaticLoop.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib not found; run `cargo build` first");
}

/// **Critical guard.** `cargo test` compiles the lib only as a *test* target; it
/// does **not** relink the `cdylib` artifact. Without this check a stale
/// `libStaticLoop.so` from an earlier `cargo build` would be tested and every
/// divergence introduced since then would silently pass.
///
/// Verified experimentally: editing `src/lib.rs` and running `cargo test` leaves
/// `target/debug/libStaticLoop.so` byte-identical.
fn assert_cdylib_is_fresh(so: &std::path::Path) {
    let mtime = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    let so_time = mtime(so);
    let mut newest_src = std::time::SystemTime::UNIX_EPOCH;
    let mut newest_name = String::new();
    let src_dir = manifest_dir().join("src");
    let mut candidates: Vec<PathBuf> = vec![manifest_dir().join("Cargo.toml")];
    if let Ok(rd) = std::fs::read_dir(&src_dir) {
        for e in rd.flatten() {
            candidates.push(e.path());
        }
    }
    for p in candidates {
        let t = mtime(&p);
        if t > newest_src {
            newest_src = t;
            newest_name = p.display().to_string();
        }
    }
    assert!(
        so_time >= newest_src,
        "STALE cdylib: {so:?} is older than {newest_name}.\n\
         `cargo test` does NOT relink the cdylib artifact, so the tests would be \
         checking an out-of-date library.\n\
         Run `cargo build` (and/or `cargo build --release`) first, or use \
         ./run_all.sh which does it for you."
    );
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

type StaticSumFn = unsafe extern "C" fn(c_int) -> c_int;
type DriverFn = unsafe extern "C" fn(c_int);

/// One side of the comparison: a `dlopen`ed library plus its resolved exports.
pub struct Side {
    pub name: &'static str,
    pub static_sum: StaticSumFn,
    pub driver: DriverFn,
    _lib: Library,
}

impl Side {
    fn open(name: &'static str, path: PathBuf) -> Side {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({path:?}) failed: {e}"));
        // Resolve strictly by exported symbol name — this is what proves the
        // `#[no_mangle]` wrappers exist and have the right ABI.
        let static_sum: Symbol<StaticSumFn> = unsafe { lib.get(b"static_sum\0") }
            .unwrap_or_else(|e| panic!("{name}: symbol `static_sum` missing: {e}"));
        let driver: Symbol<DriverFn> = unsafe { lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("{name}: symbol `driver` missing: {e}"));
        let static_sum = *static_sum;
        let driver = *driver;
        Side {
            name,
            static_sum,
            driver,
            _lib: lib,
        }
    }
}

pub struct Harness {
    pub c: Side,
    pub rust: Side,
}

impl Harness {
    /// Differential call of the lowest-level entry point.
    ///
    /// Returns the (identical) value both libraries produced.
    #[track_caller]
    pub fn static_sum(&self, update: c_int, ctx: &str) -> c_int {
        let c = unsafe { (self.c.static_sum)(update) };
        let r = unsafe { (self.rust.static_sum)(update) };
        assert_eq!(
            c, r,
            "static_sum({update}) diverged [{ctx}]: C returned {c}, Rust returned {r}",
        );
        c
    }

    /// Differential call of the wrapper entry point, comparing the exact bytes
    /// each library wrote to stdout via `printf`.
    #[track_caller]
    pub fn driver(&self, stride: c_int, ctx: &str) -> Vec<u8> {
        let c = capture_stdout(|| unsafe { (self.c.driver)(stride) });
        let r = capture_stdout(|| unsafe { (self.rust.driver)(stride) });
        assert_eq!(
            c,
            r,
            "driver({stride}) stdout diverged [{ctx}]:\n  C    = {:?}\n  Rust = {:?}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
        );
        c
    }

    /// Push both accumulators to an exact value using only public calls.
    ///
    /// `static_sum` is the only way to observe or change the accumulator, so we
    /// read it with a no-op update and then add the delta needed. Both sides are
    /// driven through the same sequence, so they stay in lockstep.
    #[track_caller]
    pub fn park_accumulator_at(&self, target: c_int, ctx: &str) {
        let current = self.static_sum(0, ctx);
        let delta = target.wrapping_sub(current);
        let got = self.static_sum(delta, ctx);
        assert_eq!(got, target, "failed to park accumulator at {target} [{ctx}]");
    }
}

/// Global lock. Serialises access because (a) the accumulators are process-wide
/// state and (b) stdout capture temporarily rebinds file descriptor 1.
static HARNESS: OnceLock<Mutex<Harness>> = OnceLock::new();

pub fn with_libs<T>(f: impl FnOnce(&mut MutexGuard<'_, Harness>) -> T) -> T {
    let m = HARNESS.get_or_init(|| {
        Mutex::new(Harness {
            c: Side::open("C", c_so_path()),
            rust: Side::open("Rust", rust_so_path()),
        })
    });
    // Recover rather than propagate: a panic in one test must not turn every
    // later test into a poisoning artefact.
    let mut guard = match m.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    f(&mut guard)
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Redirect fd 1 to a temp file for the duration of `f` and return the bytes
/// written. Callers hold the global harness lock, so this is not racy.
pub fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "staticloop-capture-{}-{}.txt",
        std::process::id(),
        n
    ));

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");

    unsafe {
        // Flush anything already pending so it lands on the real stdout.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

        f();

        // Flush the library's buffered `printf` output into the capture file
        // *before* restoring fd 1, otherwise it would leak out later.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }

    let mut buf = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("rewind capture file");
    file.read_to_end(&mut buf).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&path);
    buf
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep Phase B reproducible
// ---------------------------------------------------------------------------

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

    /// Uniform over the whole `int` domain, including `INT_MIN` / `INT_MAX`.
    pub fn next_c_int(&mut self) -> c_int {
        self.next_u32() as i32 as c_int
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn next_in_range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }
}

/// Values that are interesting for *every* `int` parameter of this API.
pub const LANDMARKS: &[c_int] = &[
    0,
    1,
    -1,
    2,
    -2,
    7,
    -7,
    10,
    -10,
    255,
    256,
    -256,
    32767,
    -32768,
    65535,
    65536,
    1_000_000_007,
    -1_000_000_007,
    i32::MAX / 2,
    i32::MIN / 2,
    i32::MAX - 1,
    i32::MIN + 1,
    i32::MAX,
    i32::MIN,
];
