// Shared differential-test harness.
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
// ONLY through their exported `driver` symbol, exactly as an external consumer
// would. The Rust crate is never linked or called directly, so the
// `#[no_mangle] extern "C"` wrapper is under test too.
//
// The library's entire observable behaviour is what it writes to stdout, so the
// harness captures file descriptor 1 around each call.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type DriverFn = unsafe extern "C" fn(c_int, c_int, c_int);

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, which is what we need:
    /// the C `.so`, the Rust `.so` and this test binary all share the one libc
    /// `stdout` FILE object.
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    workspace_root()
        .parent()
        .expect("translation/ has a parent")
        .join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    // Prefer the release cdylib (the artifact SYMBOLS.md is derived from), but
    // fall back to debug if only that has been built.
    let base = workspace_root().join("target");
    let release = base.join("release/libdriver.so");
    if release.exists() {
        return release;
    }
    base.join("debug/libdriver.so")
}

/// Both loaded shared objects plus their resolved `driver` symbol addresses.
pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_driver: DriverFn,
    pub rust_driver: DriverFn,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Serialises fd-1 redirection, which is process-global state. `cargo test`
/// runs test functions on parallel threads, so every capture must hold this.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

fn capture_guard() -> MutexGuard<'static, ()> {
    CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Capturing file descriptor 1 is a PROCESS-global operation, so the test
/// harness must not run test functions concurrently: another thread's libtest
/// progress output would be written into our capture file and be misreported as
/// a C/Rust divergence. `RUST_TEST_THREADS=1` is therefore mandatory.
/// Use `scripts/run_tests.sh`, or `RUST_TEST_THREADS=1 cargo test --release`.
fn require_serial_harness() {
    let v = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
    assert_eq!(
        v, "1",
        "these differential tests capture the process-wide stdout file \
         descriptor and MUST run serially.\n\
         Run them with:  RUST_TEST_THREADS=1 cargo test --release\n\
         or:             ./scripts/run_tests.sh"
    );
}

pub fn libs() -> &'static Libs {
    require_serial_harness();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not built at {c_path:?}\n\
             build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not built at {rust_path:?}\n\
             build it with: cd translation && cargo build --release"
        );

        // SAFETY: loading two independent, self-contained C-ABI shared objects.
        // A `dlopen` failure here (e.g. an unresolved non-libc symbol) is a
        // Phase A / Phase D failure and is surfaced as a panic.
        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({c_path:?}) failed: {e}"));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({rust_path:?}) failed: {e}"));

            let c_sym: Symbol<DriverFn> = c
                .get(b"driver\0")
                .expect("C .so must export `driver`");
            let rust_sym: Symbol<DriverFn> = rust
                .get(b"driver\0")
                .expect("Rust .so must export `driver` (check #[no_mangle])");

            let c_driver = *c_sym;
            let rust_driver = *rust_sym;

            Libs { _c: c, _rust: rust, c_driver, rust_driver }
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// the raw bytes it wrote.
fn capture_fd1<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut tmp = tempfile();

    // Drain BOTH buffering layers that sit in front of fd 1 before we steal it,
    // otherwise their leftovers would land in the capture file and look like a
    // divergence:
    //   * Rust's `std::io::Stdout` LineWriter (holds libtest's partial
    //     "test foo ... " progress line, which has no trailing newline);
    //   * libc's `stdout` FILE buffer, shared by this binary and both `.so`s.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    // SAFETY: plain POSIX fd juggling; every fd we create is closed, and fd 1
    // is restored from the saved duplicate before returning.
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }

    use std::io::Seek;
    tmp.rewind().expect("rewind capture file");
    let mut out = Vec::new();
    tmp.read_to_end(&mut out).expect("read capture file");
    out
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = dir.join(format!(
        "driver_difftest_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture temp file");
    // Unlink immediately; the open handle keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

/// Calls the C `driver` through the `.so` and returns its stdout bytes.
pub fn c_output(x: i32, y: i32, z: i32) -> Vec<u8> {
    let f = libs().c_driver;
    let _g = capture_guard();
    capture_fd1(|| unsafe { f(x, y, z) })
}

/// Calls the Rust `driver` through the `.so` and returns its stdout bytes.
pub fn rust_output(x: i32, y: i32, z: i32) -> Vec<u8> {
    let f = libs().rust_driver;
    let _g = capture_guard();
    capture_fd1(|| unsafe { f(x, y, z) })
}

/// Calls a whole SEQUENCE of `driver` invocations inside a single capture, so
/// that any cross-call state leakage shows up in the concatenated output.
pub fn c_output_seq(calls: &[(i32, i32, i32)]) -> Vec<u8> {
    let f = libs().c_driver;
    let _g = capture_guard();
    capture_fd1(|| {
        for &(x, y, z) in calls {
            unsafe { f(x, y, z) }
        }
    })
}

pub fn rust_output_seq(calls: &[(i32, i32, i32)]) -> Vec<u8> {
    let f = libs().rust_driver;
    let _g = capture_guard();
    capture_fd1(|| {
        for &(x, y, z) in calls {
            unsafe { f(x, y, z) }
        }
    })
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// The core differential assertion: byte-for-byte equality of the two stdout
/// streams for the same input.
#[track_caller]
pub fn assert_same(x: i32, y: i32, z: i32) -> Vec<u8> {
    let c = c_output(x, y, z);
    let r = rust_output(x, y, z);
    assert_eq!(
        c,
        r,
        "stdout divergence for driver({x}, {y}, {z}):\n  C    = \"{}\"\n  Rust = \"{}\"",
        show(&c),
        show(&r)
    );
    c
}

#[track_caller]
pub fn assert_same_seq(calls: &[(i32, i32, i32)]) -> Vec<u8> {
    let c = c_output_seq(calls);
    let r = rust_output_seq(calls);
    assert_eq!(
        c,
        r,
        "stdout divergence for call sequence {calls:?}:\n  C    = \"{}\"\n  Rust = \"{}\"",
        show(&c),
        show(&r)
    );
    c
}

/// Asserts C and Rust agree AND that the shared output is exactly `expected`,
/// i.e. that the C ground truth itself is what ERRORS.md/CONFIGS.md predicts.
#[track_caller]
pub fn assert_same_and_eq(x: i32, y: i32, z: i32, expected: &str) {
    let got = assert_same(x, y, z);
    assert_eq!(
        got,
        expected.as_bytes(),
        "driver({x}, {y}, {z}) produced \"{}\", expected \"{}\"",
        show(&got),
        expected.escape_debug()
    );
}

// ---------------------------------------------------------------------------
// Expected-output model, derived from c_src/src/driver.c
// ---------------------------------------------------------------------------

pub const OK: &str = "Ok!\nResult: 0\n";
pub const ERR_X: &str = "Error: x != 1\nOperation failed\nResult: 1\n";
pub const ERR_Y: &str = "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n";
pub const ERR_Z: &str = "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n";

/// Model of the C control flow (driver.c:31-63), used to cross-check that the
/// captured C output is the one the source implies.
pub fn expected(x: i32, y: i32, z: i32) -> &'static str {
    if x != 1 {
        ERR_X
    } else if y != 2 {
        ERR_Y
    } else if z != 3 {
        ERR_Z
    } else {
        OK
    }
}

/// The `result` code the C `multi_stage` returns for these inputs.
pub fn expected_code(x: i32, y: i32, z: i32) -> i32 {
    if x != 1 {
        1
    } else if y != 2 {
        2
    } else if z != 3 {
        3
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, reproducible; no external dev-dep needed)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Avoid the zero state of xorshift64*.
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform over the whole `i32` range, including `INT_MIN`/`INT_MAX`.
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// An `i32` guaranteed different from `avoid` (for the `x != 1` style rows).
    pub fn i32_not(&mut self, avoid: i32) -> i32 {
        loop {
            let v = self.next_i32();
            if v != avoid {
                return v;
            }
        }
    }

    /// Draws from the interesting/boundary set most of the time, otherwise a
    /// fully random `i32`. Keeps deep stages of `multi_stage` reachable.
    pub fn biased_i32(&mut self) -> i32 {
        const SPECIAL: [i32; 12] = [
            i32::MIN,
            i32::MIN + 1,
            -2,
            -1,
            0,
            1,
            2,
            3,
            4,
            123,
            i32::MAX - 1,
            i32::MAX,
        ];
        if self.below(4) == 0 {
            self.next_i32()
        } else {
            SPECIAL[self.below(SPECIAL.len() as u64) as usize]
        }
    }

    /// Like `biased_i32`, but returns the stage's *valid* constant `valid` about
    /// a third of the time. Needed so that random triples actually reach the
    /// deeper stages of `multi_stage` (with a uniform-over-specials draw the
    /// all-valid triple has probability ~2e-4 and the success path is
    /// effectively never hit).
    pub fn biased_i32_favouring(&mut self, valid: i32) -> i32 {
        if self.below(3) == 0 {
            valid
        } else {
            self.biased_i32()
        }
    }
}

/// The boundary / interesting values used by the exhaustive cross-product row.
pub const SPECIAL_VALUES: [i32; 12] = [
    i32::MIN,
    i32::MIN + 1,
    -2,
    -1,
    0,
    1,
    2,
    3,
    4,
    123,
    i32::MAX - 1,
    i32::MAX,
];
