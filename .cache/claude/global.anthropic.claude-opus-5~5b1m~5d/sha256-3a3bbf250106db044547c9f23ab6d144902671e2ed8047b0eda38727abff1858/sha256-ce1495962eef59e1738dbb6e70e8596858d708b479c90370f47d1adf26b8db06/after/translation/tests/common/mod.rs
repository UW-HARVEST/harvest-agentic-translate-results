//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and captures the raw file-descriptor-1 output of each call so the two can be
//! compared byte-for-byte.
//!
//! The Rust library is never called directly — always through its exported
//! `driver` symbol, exactly as an external C consumer would.

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open output stream, which is all we need to
    /// drain whatever `printf`/`puts` buffered inside either library.
    fn fflush(stream: *mut c_void) -> c_int;
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path of the C shared library produced by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?} — build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Path of the Rust `cdylib` under test.
///
/// `cargo test` does **not** build a `cdylib`-only library target, so simply
/// looking inside `target/{debug,release}` would happily load a **stale** `.so`
/// left over from an earlier `cargo build` and silently verify nothing. The
/// harness therefore builds the `cdylib` itself, into a dedicated target
/// directory (so it cannot deadlock on the target-dir lock held by `cargo test`),
/// and returns that freshly built artifact.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
            let p = PathBuf::from(p);
            assert!(p.exists(), "DRIVER_RUST_SO points at a missing file: {p:?}");
            return p;
        }

        let target_dir = manifest_dir().join("target/so-under-test");
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let out = std::process::Command::new(cargo)
            .current_dir(manifest_dir())
            .args(["build", "--offline", "--release", "--lib", "--target-dir"])
            .arg(&target_dir)
            // Do not inherit the outer `cargo test` invocation's state.
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_TARGET_DIR")
            .output()
            .expect("failed to spawn cargo to build the cdylib under test");
        assert!(
            out.status.success(),
            "building the Rust cdylib failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );

        let so = target_dir.join("release/libdriver.so");
        assert!(so.exists(), "cargo did not produce {so:?}");

        // Staleness guard: the artifact must be at least as new as the source.
        let so_time = fs::metadata(&so).and_then(|m| m.modified()).unwrap();
        let src_time = fs::metadata(manifest_dir().join("src/lib.rs"))
            .and_then(|m| m.modified())
            .unwrap();
        assert!(
            so_time >= src_time,
            "{so:?} is older than src/lib.rs — the tests would verify a stale library"
        );
        so
    })
    .clone()
}

struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        // Default libloading flags are RTLD_LAZY | RTLD_LOCAL, so the two
        // identically-named `driver` symbols do not collide.
        Libs {
            c: Library::new(c_so_path()).expect("failed to dlopen the C .so"),
            rust: Library::new(rust_so_path()).expect("failed to dlopen the Rust .so"),
        }
    })
}

pub fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C .so exports no `driver`") }
}

pub fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe {
        libs()
            .rust
            .get(b"driver\0")
            .expect("Rust .so exports no `driver` (missing #[no_mangle] wrapper?)")
    }
}

/// Redirecting fd 1 is process-global, so captures must be serialized even when
/// the test harness runs tests on several threads.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Because fd 1 is redirected process-wide, concurrent tests would capture each
/// other's (and libtest's) output. `.cargo/config.toml` sets
/// `RUST_TEST_THREADS = "1"`; fail loudly if that was bypassed.
fn require_sequential_tests() {
    let threads = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
    assert_eq!(
        threads, "1",
        "these differential tests redirect file descriptor 1 process-wide and must \
         run sequentially; run them via `cargo test` in translation/ (which sets \
         RUST_TEST_THREADS=1 through .cargo/config.toml) or pass --test-threads=1"
    );
}

/// Runs `f` with file descriptor 1 pointed at a temporary file and returns every
/// byte written to it.
pub fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let _guard = capture_lock();

    require_sequential_tests();
    // Force the (possibly cargo-invoking) library load to happen *before* fd 1 is
    // redirected, so no build output can land in the capture.
    let _ = libs();

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "driver_capture_{}_{}.bin",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    // Drain everything already pending so it is not attributed to this capture:
    // libtest's own `test <name> ... ` progress text sits unterminated in Rust's
    // line-buffered `io::stdout`, and the C libraries buffer inside libc FILEs.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe { fflush(std::ptr::null_mut()) };

    let file = fs::File::create(&path).expect("cannot create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);

    let data = fs::read(&path).expect("cannot read capture file");
    let _ = fs::remove_file(&path);

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    data
}

/// Output of the C `driver(x, y)` for one call.
pub fn c_output(x: i32, y: i32) -> Vec<u8> {
    let f = c_driver();
    capture_stdout(|| unsafe { f(x, y) })
}

/// Output of the Rust `driver(x, y)` for one call, via the `.so` export.
pub fn rust_output(x: i32, y: i32) -> Vec<u8> {
    let f = rust_driver();
    capture_stdout(|| unsafe { f(x, y) })
}

fn render(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    if s.len() <= 400 {
        format!("{s:?} ({} bytes)", bytes.len())
    } else {
        format!(
            "{:?}…{:?} ({} bytes)",
            &s[..200],
            &s[s.len() - 200..],
            bytes.len()
        )
    }
}

/// The core differential assertion: C and Rust must emit identical bytes.
#[track_caller]
pub fn assert_same(row: &str, x: i32, y: i32) {
    assert!(
        !is_excluded(x, y),
        "[{row}] refusing to execute driver({x}, {y}): {} (see ERRORS.md rows 14-15)",
        if is_ub(x, y) {
            "the C code has signed-overflow UB / never terminates here"
        } else {
            "the C code terminates only after an intractable amount of output"
        }
    );
    let c = c_output(x, y);
    let r = rust_output(x, y);
    assert!(
        c == r,
        "[{row}] divergence for driver({x}, {y})\n  C   : {}\n  Rust: {}",
        render(&c),
        render(&r)
    );
}

/// Same, for a whole sequence of calls issued into a single capture (catches
/// residual state and buffering differences).
#[track_caller]
pub fn assert_same_sequence(row: &str, calls: &[(i32, i32)]) {
    let cf = c_driver();
    let rf = rust_driver();
    let c = capture_stdout(|| {
        for &(x, y) in calls {
            unsafe { cf(x, y) };
        }
    });
    let r = capture_stdout(|| {
        for &(x, y) in calls {
            unsafe { rf(x, y) };
        }
    });
    assert!(
        c == r,
        "[{row}] sequence divergence for {calls:?}\n  C   : {}\n  Rust: {}",
        render(&c),
        render(&r)
    );
}

/// Largest amount of "work" (positive-argument magnitude) a test case may have.
/// The C loop emits on the order of `x.max(0) + y.max(0)` lines, so this caps
/// each captured file at a few hundred kilobytes.
pub const WORK_LIMIT: i64 = 50_000;

/// `x > 0 && y < 0` makes the C code loop until signed-integer overflow (`y` is
/// decremented forever because it never reaches 0) — UB in C and unbounded
/// output: `ERRORS.md` row 14.
pub fn is_ub(x: i32, y: i32) -> bool {
    x > 0 && y < 0
}

/// Terminating, but with ~`x.max(0) + y.max(0)` lines of output — anything near
/// `INT_MAX` is not executable in a test: `ERRORS.md` row 15.
pub fn is_intractable(x: i32, y: i32) -> bool {
    x.max(0) as i64 + y.max(0) as i64 > WORK_LIMIT
}

/// Inputs that must never be executed against the C library.
pub fn is_excluded(x: i32, y: i32) -> bool {
    is_ub(x, y) || is_intractable(x, y)
}

/// Deterministic xorshift64* PRNG so every row is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x2026_0828_D21F_11A3;

impl Rng {
    pub fn new(stream: u64) -> Self {
        // Mix the row-specific stream id into the shared seed.
        Rng(SEED ^ (stream.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform-ish value in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i32 {
        assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        (lo + (self.next_u64() % span) as i64) as i32
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}
