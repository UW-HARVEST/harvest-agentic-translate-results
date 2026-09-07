//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! invoked purely through their exported C symbols — the Rust functions are
//! never called directly, so the `#[no_mangle]` export wrappers are part of
//! what is under test.

#![allow(dead_code)]

use libloading::os::unix::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type CleanupFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type PrintResultFn = unsafe extern "C" fn(*const c_char, c_int);
pub type CleanupResourcesFn = unsafe extern "C" fn(*mut c_char);

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub cleanup: Symbol<CleanupFn>,
    pub print_result: Symbol<PrintResultFn>,
    pub cleanup_resources: Symbol<CleanupResourcesFn>,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        let cleanup = unsafe { lib.get::<CleanupFn>(b"cleanup\0") }
            .unwrap_or_else(|e| panic!("{name}: missing symbol `cleanup`: {e}"));
        let print_result = unsafe { lib.get::<PrintResultFn>(b"print_result\0") }
            .unwrap_or_else(|e| panic!("{name}: missing symbol `print_result`: {e}"));
        let cleanup_resources = unsafe { lib.get::<CleanupResourcesFn>(b"cleanup_resources\0") }
            .unwrap_or_else(|e| panic!("{name}: missing symbol `cleanup_resources`: {e}"));
        Impl {
            name,
            path,
            _lib: lib,
            cleanup,
            print_result,
            cleanup_resources,
        }
    }
}

/// Host libc entry points used by the harness itself (fd plumbing + heap).
struct Host {
    _lib: Library,
    dup: Symbol<unsafe extern "C" fn(c_int) -> c_int>,
    dup2: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int>,
    fflush: Symbol<unsafe extern "C" fn(*mut c_void) -> c_int>,
    malloc: Symbol<unsafe extern "C" fn(usize) -> *mut c_void>,
}

impl Host {
    fn load() -> Host {
        // dlopen(NULL) resolves through the global scope, which already
        // contains the process's libc.
        let lib = Library::this();
        macro_rules! sym {
            ($l:expr, $n:literal, $t:ty) => {
                unsafe { $l.get::<$t>($n) }.unwrap_or_else(|e| panic!("host libc: {}: {e}", String::from_utf8_lossy($n)))
            };
        }
        let dup = sym!(lib, b"dup\0", unsafe extern "C" fn(c_int) -> c_int);
        let dup2 = sym!(lib, b"dup2\0", unsafe extern "C" fn(c_int, c_int) -> c_int);
        let fflush = sym!(lib, b"fflush\0", unsafe extern "C" fn(*mut c_void) -> c_int);
        let malloc = sym!(lib, b"malloc\0", unsafe extern "C" fn(usize) -> *mut c_void);
        Host {
            _lib: lib,
            dup,
            dup2,
            fflush,
            malloc,
        }
    }
}

pub struct Libs {
    pub c: Impl,
    pub rust: Impl,
    host: Host,
    stdout_lock: Mutex<u64>,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.len() {
        0 => panic!(
            "no C .so found in {} — build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        ),
        _ => found.remove(0),
    }
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("CDIFF_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "CDIFF_RUST_SO does not exist: {}", p.display());
        return p;
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libcleanup_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "no Rust cdylib found under {} — build it with `cargo build --release`",
        target.display()
    );
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// The fd-1 redirection used by `capture_stdout` is process-global, and
/// libtest's own progress output also goes to fd 1. Concurrent test threads
/// would therefore contaminate each other's captures, so the suite requires
/// single-threaded execution (set via `translation/.cargo/config.toml`).
fn assert_single_threaded() {
    let n = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
    assert_eq!(
        n, "1",
        "this differential suite captures stdout by redirecting fd 1 and must run \
         single-threaded; expected RUST_TEST_THREADS=1 (got {n:?}). Run it via \
         `cargo test` from the `translation` directory so .cargo/config.toml applies, \
         or pass `-- --test-threads=1`."
    );
}

pub fn libs() -> &'static Libs {
    assert_single_threaded();
    LIBS.get_or_init(|| Libs {
        c: Impl::load("C", find_c_so()),
        rust: Impl::load("Rust", find_rust_so()),
        host: Host::load(),
        stdout_lock: Mutex::new(0),
    })
}

/// Redirect fd 1 to a temporary file, run `f`, and return its result together
/// with every byte the callee wrote to stdout.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let l = libs();
    let mut guard: MutexGuard<u64> = l.stdout_lock.lock().unwrap_or_else(|e| e.into_inner());
    *guard += 1;
    let nonce = *guard;

    let path = std::env::temp_dir().join(format!(
        "cdiff_{}_{}_{}.out",
        std::process::id(),
        nonce,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));

    // Flush anything already buffered so it is not captured.
    unsafe { (l.host.fflush)(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let file = fs::File::create(&path).expect("create capture temp file");
    let saved = unsafe { (l.host.dup)(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { (l.host.dup2)(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let out = f();

    unsafe { (l.host.fflush)(std::ptr::null_mut()) };
    assert!(unsafe { (l.host.dup2)(saved, 1) } >= 0, "dup2 restore failed");
    drop(unsafe { OwnedFd::from_raw_fd(saved) });
    drop(file);

    let mut buf = Vec::new();
    fs::File::open(&path)
        .expect("reopen capture temp file")
        .read_to_end(&mut buf)
        .expect("read capture temp file");
    let _ = fs::remove_file(&path);

    (out, buf)
}

/// `malloc` from the host libc, so the pointer is legitimately `free`able by
/// either implementation's `cleanup_resources`.
pub fn host_malloc(n: usize) -> *mut c_char {
    unsafe { (libs().host.malloc)(n) as *mut c_char }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE_5EED_C0DE;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    /// An `int` that is *not* one of the `switch` case labels, so it lands in
    /// `default:`.
    pub fn non_case_i32(&mut self) -> i32 {
        loop {
            let v = self.next_i32();
            if !matches!(v, 10 | 20 | 30 | 40) {
                return v;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Run `cleanup` over `inputs` in BOTH implementations and assert that the
/// per-input return values and the complete stdout byte streams are identical.
///
/// stdout is captured once per implementation over the whole batch (fast); on
/// any divergence the batch is replayed input-by-input to pinpoint the first
/// differing call.
pub fn diff_cleanup(row: &str, inputs: &[[i32; 4]]) {
    assert!(!inputs.is_empty(), "{row}: empty input set");
    let l = libs();

    let (rc, oc) = capture_stdout(|| {
        inputs
            .iter()
            .map(|q| unsafe { (l.c.cleanup)(q[0], q[1], q[2], q[3]) })
            .collect::<Vec<c_int>>()
    });
    let (rr, or) = capture_stdout(|| {
        inputs
            .iter()
            .map(|q| unsafe { (l.rust.cleanup)(q[0], q[1], q[2], q[3]) })
            .collect::<Vec<c_int>>()
    });

    for (i, q) in inputs.iter().enumerate() {
        assert_eq!(
            rc[i], rr[i],
            "{row}: cleanup({}, {}, {}, {}) returned C={} Rust={}",
            q[0], q[1], q[2], q[3], rc[i], rr[i]
        );
    }

    if oc != or {
        // Localize.
        for q in inputs.iter() {
            let (_, a) = capture_stdout(|| unsafe { (l.c.cleanup)(q[0], q[1], q[2], q[3]) });
            let (_, b) = capture_stdout(|| unsafe { (l.rust.cleanup)(q[0], q[1], q[2], q[3]) });
            assert_eq!(
                show(&a),
                show(&b),
                "{row}: stdout diverged for cleanup({}, {}, {}, {})",
                q[0],
                q[1],
                q[2],
                q[3]
            );
        }
        panic!(
            "{row}: batched stdout diverged but no single input did.\n\
             C bytes={} Rust bytes={}\nfirst difference at offset {}",
            oc.len(),
            or.len(),
            first_diff(&oc, &or)
        );
    }
}

/// Run `print_result` over `inputs` in BOTH implementations and compare stdout.
pub fn diff_print_result(row: &str, inputs: &[(CString, c_int)]) {
    assert!(!inputs.is_empty(), "{row}: empty input set");
    let l = libs();

    let (_, oc) = capture_stdout(|| {
        for (s, r) in inputs {
            unsafe { (l.c.print_result)(s.as_ptr(), *r) };
        }
    });
    let (_, or) = capture_stdout(|| {
        for (s, r) in inputs {
            unsafe { (l.rust.print_result)(s.as_ptr(), *r) };
        }
    });

    if oc != or {
        for (s, r) in inputs {
            let (_, a) = capture_stdout(|| unsafe { (l.c.print_result)(s.as_ptr(), *r) });
            let (_, b) = capture_stdout(|| unsafe { (l.rust.print_result)(s.as_ptr(), *r) });
            assert_eq!(
                show(&a),
                show(&b),
                "{row}: stdout diverged for print_result({:?}, {})",
                s,
                r
            );
        }
        panic!(
            "{row}: batched stdout diverged but no single input did.\n\
             C bytes={} Rust bytes={}\nfirst difference at offset {}",
            oc.len(),
            or.len(),
            first_diff(&oc, &or)
        );
    }
}

fn first_diff(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()))
}

/// Every `int` value class the `switch` distinguishes.
pub const CASE_LABELS: [i32; 4] = [10, 20, 30, 40];

/// Marker used when building cross-products: a slot that must land in
/// `default:`.
pub const DEFAULT_SLOT: i32 = i32::MIN + 12345;
