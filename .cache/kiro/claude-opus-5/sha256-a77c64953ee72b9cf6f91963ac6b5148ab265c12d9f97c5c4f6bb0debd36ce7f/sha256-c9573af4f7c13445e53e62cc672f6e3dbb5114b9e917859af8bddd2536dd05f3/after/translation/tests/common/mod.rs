//! Shared differential-testing harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls the exported
//! `driver` symbol across the FFI boundary in each. The Rust implementation is
//! never called directly — only through `target/<profile>/libdriver.so`, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! `driver` returns `void` and its only observable is what it writes to stdout
//! via libc `printf`. Both `.so`s are dynamically linked against the *same*
//! `libc.so.6` as this test process, so redirecting fd 1 to a temporary file
//! and calling `fflush(NULL)` captures either library's output verbatim.

use std::ffi::c_int;
use std::fs;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

pub type DriverFn = unsafe extern "C" fn(c_int);

/// Which of the two libraries to exercise.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

struct Libs {
    c: Library,
    rust: Library,
}

// fd 1 is process-wide, so captures must be serialized across the test
// harness's worker threads.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static LIBS: OnceLock<Libs> = OnceLock::new();
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, built by CMake.
fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The Rust cdylib for exactly the profile the test binary was built under.
///
/// Strict on purpose: falling back to another profile's `.so` would silently
/// verify a stale artifact. `cargo test` does not build a `cdylib`-only lib
/// target, so `run_tests.sh` runs `cargo build` for the matching profile first.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // exe = target/<profile>/deps/<test>-<hash>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test binary lives in target/<profile>/deps");
    let so = profile_dir.join("libdriver.so");
    assert!(
        so.is_file(),
        "Rust cdylib not found at {}.\nBuild it for this profile first, e.g.\n  cd translation && cargo build\n(or use ./run_tests.sh, which does it for you)",
        so.display()
    );
    // Guard against verifying a stale cdylib.
    let src = manifest_dir().join("src/lib.rs");
    if let (Ok(a), Ok(b)) = (fs::metadata(&so), fs::metadata(&src)) {
        if let (Ok(ta), Ok(tb)) = (a.modified(), b.modified()) {
            assert!(
                ta >= tb,
                "{} is older than src/lib.rs — rebuild the cdylib before testing",
                so.display()
            );
        }
    }
    so
}

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("dlopen C libdriver.so");
        let rust = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");
        Libs { c, rust }
    })
}

/// Resolve `driver` from the requested `.so` (exercises the exported symbol).
pub fn driver_symbol(which: Impl) -> Symbol<'static, DriverFn> {
    let l = libs();
    let lib = match which {
        Impl::C => &l.c,
        Impl::Rust => &l.rust,
    };
    unsafe { lib.get(b"driver\0").expect("`driver` symbol must be exported") }
}

/// Run `f` with fd 1 redirected to a temp file; return everything written to it
/// (including anything the loaded libraries buffered in libc stdio).
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // The libtest harness writes its own progress text ("test foo ... ", "ok")
    // through the global `io::stdout()` LineWriter, from other worker threads.
    // Hold that lock for the whole capture and flush it first, so no harness
    // bytes can be buffered into — or emitted during — our redirect window.
    let mut rust_stdout = std::io::stdout().lock();
    let _ = rust_stdout.flush();

    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));

    let file = fs::File::create(&path).expect("create capture file");

    let result;
    unsafe {
        // Drain anything already pending in libc's stdio buffer.
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        result = f();

        // Flush the library's stdio buffer while fd 1 still points at the file.
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "restoring fd 1 failed");
        libc::close(saved);
    }
    drop(file);
    drop(rust_stdout);

    let data = fs::read(&path).expect("read capture file");
    let _ = fs::remove_file(&path);
    (result, data)
}

/// Call `driver(x)` in one implementation and return its stdout bytes.
pub fn run_one(which: Impl, x: i32) -> Vec<u8> {
    let sym = driver_symbol(which);
    capture(|| unsafe { sym(x) }).1
}

/// Differential assertion for a single input: C and Rust must emit identical
/// bytes. Returns the shared output.
pub fn assert_same(x: i32, ctx: &str) -> Vec<u8> {
    let c_out = run_one(Impl::C, x);
    let rust_out = run_one(Impl::Rust, x);
    assert_eq!(
        c_out,
        rust_out,
        "[{ctx}] divergence for driver({x}) (0x{x:08x}):\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );
    c_out
}

/// Differential assertion over many inputs.
pub fn assert_same_all(xs: impl IntoIterator<Item = i32>, ctx: &str) -> usize {
    let mut n = 0;
    for x in xs {
        assert_same(x, ctx);
        n += 1;
    }
    assert!(n > 0, "[{ctx}] no inputs were exercised");
    n
}

/// Deterministic PRNG (SplitMix64) so every row is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform-ish in `[lo, hi]` inclusive, works across the full i32 range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64) as u64 + 1;
        let off = self.next_u64() % span;
        (lo as i64 + off as i64) as i32
    }
}
