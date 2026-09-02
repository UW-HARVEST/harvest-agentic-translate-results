// Differential test harness: loads BOTH shared objects with `libloading` and
// compares their observable behaviour through the FFI boundary.
//
//   C    -> c_src/build/libdriver.so
//   Rust -> translation/target/{debug,release}/libdriver.so
//
// The Rust side is NEVER called directly as a Rust crate; every call goes
// through `dlsym` on the built `.so`, so the `#[no_mangle]` / `extern "C"`
// export wrappers are under test too.
//
// All five exported functions return `void` and communicate solely by writing
// to the C `stdout`. The only way to compare them is therefore to capture
// `stdout` at the *file-descriptor* level (both `.so`s and this test process
// share the one glibc `stdout`), which is what `capture()` below does.

#![allow(non_snake_case)]

use std::ffi::{CString, c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed for fd-level stdout capture
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    // fflush(NULL) flushes *every* open output stream, which covers the
    // `stdout` used by libdriver.so without needing the `stdout` data symbol.
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FD: c_int = 1;

// ---------------------------------------------------------------------------
// Loading both libraries
// ---------------------------------------------------------------------------

/// The five exported symbols, resolved out of one `.so`.
pub struct Api {
    #[allow(dead_code)]
    pub which: &'static str,
    _lib: Library,
    printLine: unsafe extern "C" fn(*const c_char),
    printIntLine: unsafe extern "C" fn(c_int),
    bad: unsafe extern "C" fn(),
    good: unsafe extern "C" fn(),
    driver: unsafe extern "C" fn(c_int),
}

impl Api {
    fn load(which: &'static str, path: &PathBuf) -> Api {
        assert!(
            path.exists(),
            "{which} shared object not found at {}. Build it first \
             (C: cmake --build c_src/build ; Rust: cargo build)",
            path.display()
        );
        // RTLD_NOW so an unresolvable import fails here rather than silently.
        let lib = unsafe {
            libloading::os::unix::Library::open(
                Some(path),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let lib: Library = lib.into();

        unsafe fn sym<T: Copy>(lib: &Library, name: &[u8], which: &str) -> T {
            let s: Symbol<T> = unsafe { lib.get(name) }.unwrap_or_else(|e| {
                panic!(
                    "{which}: dlsym({}) failed: {e}",
                    String::from_utf8_lossy(&name[..name.len() - 1])
                )
            });
            *s
        }

        unsafe {
            Api {
                which,
                printLine: sym(&lib, b"printLine\0", which),
                printIntLine: sym(&lib, b"printIntLine\0", which),
                bad: sym(&lib, b"bad\0", which),
                good: sym(&lib, b"good\0", which),
                driver: sym(&lib, b"driver\0", which),
                _lib: lib,
            }
        }
    }

    pub fn print_line(&self, p: *const c_char) {
        unsafe { (self.printLine)(p) }
    }
    pub fn print_int_line(&self, v: c_int) {
        unsafe { (self.printIntLine)(v) }
    }
    pub fn bad(&self) {
        unsafe { (self.bad)() }
    }
    pub fn good(&self) {
        unsafe { (self.good)() }
    }
    pub fn driver(&self, use_good: c_int) {
        unsafe { (self.driver)(use_good) }
    }
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // Deterministic: `cargo test` builds the dev-profile cdylib, so that is the
    // default. Set DRIVER_RUST_SO to test the release cdylib (see
    // scripts/verify_all.sh, which runs the suite against BOTH profiles).
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("target/debug/libdriver.so")
}

// One shared instance; `dlopen` is refcounted anyway, but a single load keeps
// the process cheap and keeps capture serialised through the same mutex.
static PAIR: OnceLock<Pair> = OnceLock::new();
// fd 1 redirection is process-global, so captures must never run concurrently.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Api::load("C", &c_so_path()),
        rust: Api::load("Rust", &rust_so_path()),
    })
}

fn capture_guard() -> MutexGuard<'static, ()> {
    CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// fd-level stdout capture
// ---------------------------------------------------------------------------

/// Run `f`, capturing everything the C `stdout` receives while it runs.
/// Returns the raw bytes. The caller must hold `capture_guard()`.
fn capture_locked(f: impl FnOnce()) -> Vec<u8> {
    let mut tmp = tempfile();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(STDOUT_FD);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp.as_raw_fd(), STDOUT_FD) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FD) >= 0, "dup2 restore failed");
        close(saved);
    }
    let mut out = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    tmp.read_to_end(&mut out).expect("read capture");
    out
}

fn tempfile() -> std::fs::File {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let f = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&p)
        .expect("create temp capture file");
    // Unlink immediately; the fd keeps it alive and nothing is left behind.
    let _ = std::fs::remove_file(&p);
    f
}

/// Core differential primitive: run the same closure against the C `Api` and
/// the Rust `Api`, capture each side's `stdout`, and assert byte equality.
///
/// Returns the (identical) captured bytes so callers can additionally assert
/// on the content itself (e.g. "exactly zero bytes").
#[track_caller]
pub fn diff(label: &str, f: impl Fn(&Api)) -> Vec<u8> {
    let l = libs();
    let g = capture_guard();
    let c_out = capture_locked(|| f(&l.c));
    let r_out = capture_locked(|| f(&l.rust));
    drop(g);
    assert_eq!(
        c_out,
        r_out,
        "DIVERGENCE [{label}]\n  C    ({} bytes): {}\n  Rust ({} bytes): {}",
        c_out.len(),
        show(&c_out),
        r_out.len(),
        show(&r_out)
    );
    c_out
}

pub fn show(b: &[u8]) -> String {
    let head: Vec<u8> = b.iter().copied().take(400).collect();
    let s = String::from_utf8_lossy(&head).escape_debug().to_string();
    if b.len() > 400 { format!("\"{s}\"... (+{} bytes)", b.len() - 400) } else { format!("\"{s}\"") }
}

// ---------------------------------------------------------------------------
// Seeded PRNG (reproducible property-style inputs, no external dependency)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // Non-zero state for splitmix64.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A random NUL-free byte string of the given length.
    pub fn cstring_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.below(255) as u8).wrapping_add(1)).collect()
    }
}

/// Build a `CString` from bytes, asserting the bytes are NUL-free.
pub fn cstr(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("test input must be NUL-free")
}
