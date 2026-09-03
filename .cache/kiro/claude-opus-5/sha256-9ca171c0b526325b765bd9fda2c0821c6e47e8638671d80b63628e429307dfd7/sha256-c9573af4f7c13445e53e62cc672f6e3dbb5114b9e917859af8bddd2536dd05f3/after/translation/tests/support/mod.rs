//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called through their exported `driver` symbol. The Rust implementation is
//! **never** called directly as a Rust function — that would bypass the
//! `#[no_mangle] extern "C"` wrapper we are trying to verify.
//!
//! `driver` returns `void`, so its complete observable behaviour is the byte
//! stream it writes to `stdout`. The harness therefore captures fd 1 around
//! each call and compares the captured bytes.

#![allow(dead_code)]

use std::ffi::CString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

pub type DriverFn = unsafe extern "C" fn(std::ffi::c_int, std::ffi::c_int);

/// Which implementation a captured output came from.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Absolute path to the C `.so`, built from `c_src/` by CMake.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().parent().expect("workspace root").to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         looked in: {candidates:?}"
    );
}

/// Absolute path to the Rust `cdylib`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    // Prefer whichever profile matches the running test binary, then fall back.
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<testbin>
        if let Some(profile_dir) = exe.parent().and_then(Path::parent) {
            candidates.push(profile_dir.join("libdriver.so"));
        }
    }
    candidates.push(base.join("debug/libdriver.so"));
    candidates.push(base.join("release/libdriver.so"));
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found; build it with `cargo build` / `cargo build --release`.\n\
         looked in: {candidates:?}"
    );
}

/// A loaded implementation plus a resolved `driver` symbol.
pub struct Loaded {
    // `_lib` must outlive `driver`; keep it alive for the process lifetime.
    _lib: &'static Library,
    pub driver: Symbol<'static, DriverFn>,
    pub which: Impl,
}

fn load(which: Impl) -> Loaded {
    let path = match which {
        Impl::C => c_so_path(),
        Impl::Rust => rust_so_path(),
    };
    // Leak the Library so the returned Symbol can be `'static`; the process
    // needs both libraries for its whole lifetime anyway.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&path).unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
    }));
    let driver: Symbol<'static, DriverFn> = unsafe {
        lib.get(b"driver\0")
            .unwrap_or_else(|e| panic!("symbol `driver` missing from {}: {e}", path.display()))
    };
    Loaded { _lib: lib, driver, which }
}

/// Both implementations, loaded once per test process.
///
/// `dlopen` is keyed on the resolved path, and `libloading` opens with
/// `RTLD_LOCAL`, so the two `driver` symbols do not collide even though both
/// libraries are named `libdriver.so`.
pub struct Pair {
    pub c: Loaded,
    pub rust: Loaded,
}

// Not `Sync` because `Symbol` isn't; we hand it out only under a mutex.
unsafe impl Send for Loaded {}
unsafe impl Sync for Loaded {}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair { c: load(Impl::C), rust: load(Impl::Rust) })
}

/// fd 1 is process-global state, so captures must be serialized even though
/// cargo runs tests on multiple threads.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Redirecting fd 1 is only sound if nothing else in the process writes to fd 1
/// while the redirect is in place. libtest's own progress output ("test foo ...
/// ok") is written to fd 1 from the thread driving each test, so the suite MUST
/// run single-threaded; otherwise another test's status line lands in our
/// captured bytes and manifests as a bogus "divergence".
///
/// This is checked once, loudly, instead of being left as a flake.
fn assert_single_threaded() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        // Either mechanism is acceptable: the env var, or the libtest CLI flag
        // (libtest reads the flag directly and does not export the env var).
        let via_env = std::env::var("RUST_TEST_THREADS").as_deref() == Ok("1");
        let args: Vec<String> = std::env::args().collect();
        let via_flag = args.iter().enumerate().any(|(i, a)| {
            a == "--test-threads=1" || (a == "--test-threads" && args.get(i + 1).map(String::as_str) == Some("1"))
        });
        assert!(
            via_env || via_flag,
            "\n\nThis suite captures fd 1 and therefore must run single-threaded.\n\
             Run it via `./run_tests.sh`, or force one thread yourself:\n\
             \n    RUST_TEST_THREADS=1 cargo test\n    cargo test -- --test-threads=1\n\n\
             (Otherwise libtest's own progress output lands in the captured bytes\n\
             and shows up as a bogus divergence.)\n"
        );
    });
}

fn scratch_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-diff-{}-{}-{}.out",
        std::process::id(),
        tag,
        n
    ))
}

/// Runs `f` with fd 1 redirected to a temp file and returns everything written.
///
/// `fflush(NULL)` is used on both sides of the redirect: the C `.so` and the
/// Rust `.so` both write through the *same* libc `stdout` FILE (the Rust
/// translation deliberately calls libc `printf` rather than Rust's `print!`),
/// so flushing all streams is what makes the captured bytes complete and
/// correctly ordered.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    assert_single_threaded();
    let _guard = capture_lock();
    let path = scratch_path(tag);
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();

    // Drain BOTH buffering layers before swapping fd 1:
    //   * `std::io::Stdout` has its own userspace `LineWriter`, which libtest
    //     fills with progress text like "test foo ... ". `fflush(NULL)` does not
    //     know about it, so without this it would be written out later, while
    //     fd 1 points at our capture file.
    //   * libc's `stdout` FILE buffer, which is what the C `.so` and the Rust
    //     `printf` calls use.
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    unsafe {
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = libc::open(cpath.as_ptr(), libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC, 0o600);
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

        f();

        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
        libc::close(fd);
    }

    let bytes = std::fs::read(&path).expect("read captured stdout");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Calls `driver` in the given implementation and returns its stdout bytes.
pub fn run(which: Impl, x: i32, y: i32) -> Vec<u8> {
    let p = pair();
    let (loaded, tag) = match which {
        Impl::C => (&p.c, "c"),
        Impl::Rust => (&p.rust, "rust"),
    };
    capture_stdout(tag, || unsafe { (loaded.driver)(x, y) })
}

/// True iff `driver(x, y)` returns. Proved from `c_src/src/driver.c`: `y--` is
/// guarded by `y != 0` and `x--` by `x > 0`, so neither counter crosses zero
/// from above, and every full body pass decrements one of them or exits. The
/// sole divergent class is `x > 0 && y < 0`, where `y` decreases without bound.
pub fn terminates(x: i32, y: i32) -> bool {
    y >= 0 || x <= 0
}

fn show(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= 40 {
        format!("{} bytes: {:?}", bytes.len(), lines)
    } else {
        format!(
            "{} bytes, {} lines; first 20 {:?}; last 20 {:?}",
            bytes.len(),
            lines.len(),
            &lines[..20],
            &lines[lines.len() - 20..]
        )
    }
}

/// The core differential assertion: C and Rust must emit identical bytes.
#[track_caller]
pub fn assert_same(row: &str, x: i32, y: i32) -> Vec<u8> {
    assert!(
        terminates(x, y),
        "{row}: driver({x}, {y}) does not terminate in C; \
         non-terminating inputs belong to ERRORS.md row 12"
    );
    let c = run(Impl::C, x, y);
    let r = run(Impl::Rust, x, y);
    if c != r {
        // Find the first differing byte to make the report actionable.
        let at = c.iter().zip(r.iter()).position(|(a, b)| a != b).unwrap_or(c.len().min(r.len()));
        panic!(
            "{row}: driver({x}, {y}) diverged at byte {at}\n  C   : {}\n  Rust: {}",
            show(&c),
            show(&r)
        );
    }
    c
}

/// Asserts a whole batch of inputs and reports how many were compared.
#[track_caller]
pub fn assert_same_all(row: &str, inputs: impl IntoIterator<Item = (i32, i32)>) -> usize {
    let mut n = 0;
    for (x, y) in inputs {
        assert_same(row, x, y);
        n += 1;
    }
    assert!(n > 0, "{row}: no inputs were compared");
    n
}

/// SplitMix64 — a tiny, fully deterministic PRNG so every run of the suite
/// compares the exact same randomized inputs.
pub struct Rng(u64);

pub const SEED: u64 = 0x5D1F_C0DE_1234_5678;

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
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i64, hi: i64) -> i32 {
        assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        (lo + (self.next_u64() % span) as i64) as i32
    }
}

/// A per-row RNG, seeded from the global seed mixed with the row name, so rows
/// are independent yet each is reproducible on its own.
pub fn rng_for(row: &str) -> Rng {
    let mut h = SEED;
    for b in row.as_bytes() {
        h = (h ^ *b as u64).wrapping_mul(0x1000_0000_01B3);
    }
    Rng::new(h)
}
