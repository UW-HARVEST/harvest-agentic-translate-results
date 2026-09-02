//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported `encode_base64` symbol — the Rust
//! function is never called directly, so the `#[no_mangle] extern "C"` wrapper
//! and the C ABI are part of what is under test.
//!
//! ## How the allocation is compared
//!
//! The `calloc` that both `.so`s import is **interposed** by the
//! `#[no_mangle] calloc` below: the test executable heads the global dynamic
//! symbol scope, so a `dlopen`ed object binds its `calloc` relocation to us
//! (verified by `harness_self_test::interposition_is_active`). Every call is
//! forwarded verbatim to `__libc_calloc`, and the exact `(nmemb, size)` request
//! is recorded. That makes the C expression `size * 4 / 3 + 4` — including its
//! signed-`int` overflow and its sign-extending conversion to `size_t` —
//! directly observable and comparable, instead of relying on
//! `malloc_usable_size`, which reflects allocator state rather than the
//! requested size and is not reproducible.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

extern "C" {
    fn free(p: *mut c_void);
    fn __libc_calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn pthread_self() -> usize;
}

// ---------------------------------------------------------------- interposer

/// Only the thread whose id is stored here records; every other thread's
/// `calloc` traffic (libtest, `dlopen`, std) is forwarded untouched. Without
/// this filter the counters would pick up unrelated allocations.
static REC_TID: AtomicUsize = AtomicUsize::new(0);
static CALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static CALLOC_LAST_NMEMB: AtomicUsize = AtomicUsize::new(usize::MAX);
static CALLOC_LAST_SIZE: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Interposes the `calloc` imported by both `.so`s. Forwards verbatim.
#[no_mangle]
pub unsafe extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void {
    if REC_TID.load(Ordering::SeqCst) == pthread_self() {
        CALLOC_LAST_NMEMB.store(nmemb, Ordering::SeqCst);
        CALLOC_LAST_SIZE.store(size, Ordering::SeqCst);
        CALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
    }
    __libc_calloc(nmemb, size)
}

/// Serializes use of the interposer's single set of recording slots.
static GUARD: Mutex<()> = Mutex::new(());

/// What one `.so` asked the allocator for during one call.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AllocInfo {
    /// Number of `calloc` calls the `.so` made (0 when it returned early).
    pub calls: usize,
    /// The `(nmemb, size)` pair of the last `calloc` request, if any.
    pub request: Option<(usize, usize)>,
}

/// Invoke `d.encode_base64(size, src)` with `calloc` recording enabled for the
/// duration of the call only. The returned pointer is **not** freed.
pub unsafe fn measured_call(
    d: &Driver,
    size: c_int,
    src: *const c_char,
) -> (*mut c_char, AllocInfo) {
    let _lock = GUARD.lock().unwrap_or_else(|e| e.into_inner());
    CALLOC_CALLS.store(0, Ordering::SeqCst);
    CALLOC_LAST_NMEMB.store(usize::MAX, Ordering::SeqCst);
    CALLOC_LAST_SIZE.store(usize::MAX, Ordering::SeqCst);
    REC_TID.store(pthread_self(), Ordering::SeqCst);

    let ptr = (d.encode_base64)(size, src);

    REC_TID.store(0, Ordering::SeqCst);
    let calls = CALLOC_CALLS.load(Ordering::SeqCst);
    let request = if calls > 0 {
        Some((
            CALLOC_LAST_NMEMB.load(Ordering::SeqCst),
            CALLOC_LAST_SIZE.load(Ordering::SeqCst),
        ))
    } else {
        None
    };
    (ptr, AllocInfo { calls, request })
}

// ------------------------------------------------------------ library loading

pub type EncodeFn = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

pub struct Driver {
    pub name: &'static str,
    _lib: Library,
    pub encode_base64: EncodeFn,
}

impl Driver {
    fn open(name: &'static str, path: &Path) -> Driver {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
            let sym: Symbol<EncodeFn> = lib
                .get(b"encode_base64\0")
                .unwrap_or_else(|e| panic!("{} does not export encode_base64: {e}", name));
            let f = *sym;
            Driver {
                name,
                _lib: lib,
                encode_base64: f,
            }
        }
    }
}

pub struct Pair {
    pub c: Driver,
    pub rs: Driver,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {} — build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    // Prefer the profile this test binary was built with, so a stale artifact
    // from the other profile is never silently tested instead.
    let order: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for prof in order {
        let p = base.join(prof).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {} — run `cargo build --release`",
        base.display()
    );
}

pub fn drivers() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c = c_so_path();
        let rs = rust_so_path();
        assert_not_stale(&rs, &manifest_dir().join("src/lib.rs"));
        assert_not_stale(&c, &manifest_dir().join("../c_src/src/lib.c"));
        eprintln!("[harness] C   .so: {}", c.display());
        eprintln!("[harness] Rust .so: {}", rs.display());
        Pair {
            c: Driver::open("C", &c),
            rs: Driver::open("Rust", &rs),
        }
    })
}

/// Refuse to test an artifact that is older than its source — `cargo test`
/// alone does not necessarily rebuild the `cdylib`, and silently comparing a
/// stale `.so` would make the whole suite meaningless.
fn assert_not_stale(artifact: &Path, source: &Path) {
    let m = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|md| md.modified())
            .unwrap_or_else(|e| panic!("cannot stat {}: {e}", p.display()))
    };
    assert!(
        m(artifact) >= m(source),
        "{} is OLDER than {} — rebuild it before testing \
         (cargo build --release, or cmake --build for the C library)",
        artifact.display(),
        source.display()
    );
}

// ------------------------------------------------------------------- model

/// The C capacity expression `size * 4 / 3 + 4` in signed `int` arithmetic.
pub fn cap_of(effective_size: c_int) -> c_int {
    effective_size
        .wrapping_mul(4)
        .wrapping_div(3)
        .wrapping_add(4)
}

/// What the C computes for `size` after the `if (!size) size = strlen(src);`
/// mode switch on line 37.
pub fn effective_size(size: c_int, src: *const c_char) -> c_int {
    if size != 0 {
        return size;
    }
    if src.is_null() {
        return 0;
    }
    let mut n = 0usize;
    unsafe {
        while *src.add(n) != 0 {
            n += 1;
        }
    }
    n as c_int
}

// ------------------------------------------------------------------ compare

struct Outcome {
    null: bool,
    /// The whole `calloc`ed buffer, byte for byte (`None` when `cap <= 0`).
    buf: Option<Vec<u8>>,
    alloc: AllocInfo,
}

unsafe fn run(d: &Driver, size: c_int, src: *const c_char) -> Outcome {
    let (ptr, alloc) = measured_call(d, size, src);
    if ptr.is_null() {
        return Outcome {
            null: true,
            buf: None,
            alloc,
        };
    }
    let cap = cap_of(effective_size(size, src));
    let buf = if cap > 0 {
        Some(std::slice::from_raw_parts(ptr as *const u8, cap as usize).to_vec())
    } else {
        None
    };
    free(ptr as *mut c_void);
    Outcome {
        null: false,
        buf,
        alloc,
    }
}

/// Call BOTH `.so`s with the same arguments and assert identical behaviour:
///
/// * the same number of `calloc` calls and the same `(nmemb, size)` request,
/// * the same NULL-ness of the return value,
/// * the same bytes across the *entire* allocated buffer.
///
/// Returns the shared result as a byte string (up to the first NUL).
pub fn assert_same_raw(size: c_int, src: *const c_char, ctx: &str) -> Option<Vec<u8>> {
    let p = drivers();
    unsafe {
        let a = run(&p.c, size, src);
        let b = run(&p.rs, size, src);

        assert_eq!(
            a.alloc, b.alloc,
            "calloc request mismatch [{ctx}] size={size}: C {:?} vs Rust {:?}",
            a.alloc, b.alloc
        );

        // Cross-check the C against the capacity model documented in CONFIGS.md.
        if let Some((nmemb, req)) = a.alloc.request {
            let cap = cap_of(effective_size(size, src));
            assert_eq!(nmemb, 1, "[{ctx}] C calloc nmemb should be sizeof(char)");
            assert_eq!(
                req, cap as usize,
                "[{ctx}] size={size}: C requested {req} bytes, model says {cap}"
            );
        }

        assert_eq!(
            a.null,
            b.null,
            "NULL-ness mismatch [{ctx}] size={size}: C {} vs Rust {}",
            if a.null { "NULL" } else { "non-NULL" },
            if b.null { "NULL" } else { "non-NULL" },
        );

        match (&a.buf, &b.buf) {
            (Some(x), Some(y)) => {
                assert_eq!(
                    x,
                    y,
                    "output mismatch [{ctx}] size={size}\n  C   ={:?}\n  Rust={:?}",
                    String::from_utf8_lossy(x),
                    String::from_utf8_lossy(y)
                );
                Some(x.iter().copied().take_while(|&c| c != 0).collect())
            }
            (None, None) => None,
            _ => unreachable!("buf presence is a pure function of size/src"),
        }
    }
}

/// Convenience wrapper for a byte slice payload (not NUL-terminated).
pub fn assert_same(size: c_int, payload: &[u8], ctx: &str) -> Option<Vec<u8>> {
    assert_same_raw(size, payload.as_ptr() as *const c_char, ctx)
}

/// Call both sides and return `(c_is_null, rust_is_null)` without touching the
/// returned buffers (for zero-length allocations, which must not be read).
/// Also asserts that both made the identical `calloc` request.
pub fn null_ness(size: c_int, src: *const c_char) -> (bool, bool) {
    let p = drivers();
    unsafe {
        let (a, ia) = measured_call(&p.c, size, src);
        let (b, ib) = measured_call(&p.rs, size, src);
        assert_eq!(
            ia, ib,
            "calloc request mismatch size={size}: C {ia:?} vs Rust {ib:?}"
        );
        let r = (a.is_null(), b.is_null());
        if !a.is_null() {
            free(a as *mut c_void);
        }
        if !b.is_null() {
            free(b as *mut c_void);
        }
        r
    }
}

// --------------------------------------------------------------------- rng

/// Deterministic xorshift64* PRNG so every "randomized" row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() >> 1) as usize % n
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
}

pub const SEED: u64 = 0x0002_0240_601C_0FEE;
