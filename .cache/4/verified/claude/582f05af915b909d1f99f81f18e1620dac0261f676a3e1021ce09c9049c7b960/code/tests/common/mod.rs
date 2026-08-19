//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called through their exported `extern "C"` symbols, exactly as an external
//! consumer would. The Rust functions are never called directly, so the
//! `#[unsafe(no_mangle)]` export wrappers are under test too.
//!
//! ## How output is captured
//!
//! The library under test writes to `stdout` via `printf`, so the harness must
//! intercept C stdio. It does **not** redirect file descriptor 1: libtest's own
//! progress output ("test foo ... ok") is written to fd 1 from the harness
//! thread while other tests are running, and that text would be interleaved
//! into the captured bytes, producing bogus "divergences".
//!
//! Instead the harness temporarily reassigns glibc's `stdout` `FILE *` global
//! (a documented, writable glibc extension) to a private stream. `printf` and
//! `putchar` read that global on every call, in every shared object, because
//! they all resolve it through the GOT to the single copy in libc. Rust's
//! `std::io::stdout()` is a completely separate buffer writing straight to
//! fd 1, so libtest output can no longer contaminate a capture.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

unsafe extern "C" {
    /// glibc exposes `stdout` as an assignable `FILE *` global.
    static mut stdout: *mut c_void;

    fn open_memstream(bufp: *mut *mut c_char, sizep: *mut usize) -> *mut c_void;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn free(ptr: *mut c_void);
}

const IOFBF: c_int = 0; // fully buffered
const IONBF: c_int = 2; // unbuffered

/// `void driver(int)` — the single exported entry point.
pub type DriverFn = unsafe extern "C" fn(c_int);

pub struct Libs {
    pub c: Library,
    pub rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// `libloading::Library` is Send + Sync on unix; the wrapper only adds paths.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Serializes reassignment of the process-global C `stdout`.
static STDIO_LOCK: Mutex<()> = Mutex::new(());
static SEQ: AtomicU64 = AtomicU64::new(0);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the Rust `cdylib` next to the running test executable
/// (`target/<profile>/deps/<test>` -> `target/<profile>/libdriver.so`).
fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().map(Path::to_path_buf);
    let mut candidates: Vec<PathBuf> = Vec::new();
    while let Some(d) = dir {
        candidates.push(d.join("libdriver.so"));
        dir = d.parent().map(Path::to_path_buf);
        if candidates.len() > 6 {
            break;
        }
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "could not locate the Rust cdylib (libdriver.so). Tried: {candidates:#?}\n\
         Run `cargo build` first so the cdylib exists."
    );
}

fn find_c_so() -> PathBuf {
    let base = manifest_dir();
    let candidates = [
        base.join("c_src/build/libdriver.so"),
        base.join("c_src/build/lib/libdriver.so"),
        base.join("c_src/build/Release/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "could not locate the C shared library. Tried: {candidates:#?}\n\
         Build it with:\n  cd c_src && mkdir -p build && cd build \\\n\
         \x20   && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        // RTLD_LOCAL (libloading's default) keeps the two `driver` symbols from
        // colliding in the global namespace; each is resolved via its handle.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

pub fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0") }.expect("symbol `driver` missing from the C .so")
}

pub fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0") }.expect("symbol `driver` missing from the Rust .so")
}

fn scratch_path(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("driver-diff-{}-{tag}-{n}.out", std::process::id()))
}

/// Where the captured C stdio bytes are collected (CONFIGS row C24).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sink {
    /// In-memory stream (`open_memstream`), fully buffered.
    Memory,
    /// A real on-disk file (`fopen`), fully buffered — real `write(2)` calls.
    TempFile,
    /// A real on-disk file with `setvbuf(_IONBF)` — one `write(2)` per byte.
    TempFileUnbuffered,
}

/// Reassign C `stdout` to a private stream, run `f`, then restore and return
/// every byte the library wrote.
pub fn capture_with<F: FnOnce()>(tag: &str, sink: Sink, f: F) -> Vec<u8> {
    let _guard: MutexGuard<'_, ()> = STDIO_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    match sink {
        Sink::Memory => unsafe {
            let mut buf: *mut c_char = std::ptr::null_mut();
            let mut size: usize = 0;
            let ms = open_memstream(&mut buf, &mut size);
            assert!(!ms.is_null(), "open_memstream failed");

            let saved = stdout;
            stdout = ms;
            f();
            fflush(ms);
            stdout = saved;

            let out = if buf.is_null() || size == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(buf as *const u8, size).to_vec()
            };
            fclose(ms);
            if !buf.is_null() {
                free(buf as *mut c_void);
            }
            out
        },
        Sink::TempFile | Sink::TempFileUnbuffered => {
            let path = scratch_path(tag);
            let cpath = CString::new(path.to_str().unwrap()).unwrap();
            let mode = CString::new("w").unwrap();
            unsafe {
                let fh = fopen(cpath.as_ptr(), mode.as_ptr());
                assert!(!fh.is_null(), "fopen {} failed", path.display());

                if sink == Sink::TempFileUnbuffered {
                    setvbuf(fh, std::ptr::null_mut(), IONBF, 0);
                } else {
                    setvbuf(fh, std::ptr::null_mut(), IOFBF, 0);
                }

                let saved = stdout;
                stdout = fh;
                f();
                fflush(fh);
                stdout = saved;
                fclose(fh);
            }
            let out = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let _ = std::fs::remove_file(&path);
            out
        }
    }
}

pub fn capture<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    capture_with(tag, Sink::Memory, f)
}

/// Call `driver` in the C `.so` and return its stdout bytes.
pub fn run_c(x: c_int) -> Vec<u8> {
    let f = c_driver();
    capture("c", || unsafe { f(x) })
}

/// Call `driver` in the Rust `.so` and return its stdout bytes.
pub fn run_rust(x: c_int) -> Vec<u8> {
    let f = rust_driver();
    capture("rust", || unsafe { f(x) })
}

pub fn run_c_sink(x: c_int, sink: Sink) -> Vec<u8> {
    let f = c_driver();
    capture_with("c", sink, || unsafe { f(x) })
}

pub fn run_rust_sink(x: c_int, sink: Sink) -> Vec<u8> {
    let f = rust_driver();
    capture_with("rust", sink, || unsafe { f(x) })
}

/// Many calls inside a single buffering window (CONFIGS row C21).
pub fn run_c_batch(xs: &[c_int]) -> Vec<u8> {
    let f = c_driver();
    capture("c-batch", || {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

pub fn run_rust_batch(xs: &[c_int]) -> Vec<u8> {
    let f = rust_driver();
    capture("rust-batch", || {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// The core differential assertion: identical stdout bytes for the same input.
#[track_caller]
pub fn assert_same(x: c_int) -> Vec<u8> {
    let c = run_c(x);
    let r = run_rust(x);
    assert_eq!(
        c,
        r,
        "divergence for driver({x}) [0x{:08x}]\n  C   : \"{}\"\n  Rust: \"{}\"",
        x as u32,
        show(&c),
        show(&r)
    );
    // Guard against a harness that captures nothing and trivially "matches".
    assert!(
        !c.is_empty(),
        "captured no output for driver({x}); the stdout capture harness is broken"
    );
    c
}

#[track_caller]
pub fn assert_same_many(xs: &[c_int]) {
    for &x in xs {
        assert_same(x);
    }
}

/// Independent model of the C behaviour (CONFIGS row C25):
/// `{ int floors; int bedrooms=3; double bathrooms=2.0; }` little-endian,
/// 16 bytes, hex-encoded lowercase with `%02x`, then `'\n'`.
pub fn expected_bytes(x: i32) -> Vec<u8> {
    let mut raw = Vec::with_capacity(16);
    raw.extend_from_slice(&x.to_le_bytes());
    raw.extend_from_slice(&3i32.to_le_bytes());
    raw.extend_from_slice(&2.0f64.to_le_bytes());
    let mut out = Vec::with_capacity(33);
    for b in &raw {
        out.extend_from_slice(format!("{b:02x}").as_bytes());
    }
    out.push(b'\n');
    out
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) so every row is property-tested reproducibly.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
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

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in the inclusive range `[lo, hi]`.
    pub fn in_range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }

    pub fn i32_in_range(&mut self, lo: i32, hi: i32) -> i32 {
        self.in_range(lo as i64, hi as i64) as i32
    }

    /// A random byte value, optionally forced nonzero.
    pub fn byte(&mut self, nonzero: bool) -> u8 {
        loop {
            let b = (self.next_u32() & 0xff) as u8;
            if !nonzero || b != 0 {
                return b;
            }
        }
    }
}

/// Number of randomized samples per configuration row.
pub const SAMPLES: usize = 200;
