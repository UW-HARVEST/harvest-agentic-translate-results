//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded through `libloading` and driven only through
//! their exported `driver` symbol — the Rust implementation is never called
//! directly as a Rust function, so the `#[no_mangle] extern "C"` wrapper is
//! part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

pub type DriverFn = unsafe extern "C" fn(*const c_char, *const c_char);

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
}

const PROT_NONE: c_int = 0;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 2;
const MAP_ANONYMOUS: c_int = 0x20;
pub const PAGE: usize = 4096;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // The integration-test executable lives in target/<profile>/deps/, so the
    // cdylib built by the very same `cargo test` invocation is two levels up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|d| d.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let here = profile_dir.join("libdriver.so");
    if here.exists() {
        return here;
    }
    for cand in ["target/release/libdriver.so", "target/debug/libdriver.so"] {
        let p = repo_root().join("translation").join(cand);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found (looked in {} and target/{{release,debug}}). \
         Build it with `cargo build --release`.",
        here.display()
    );
}

// ---------------------------------------------------------------------------
// Loaded pair
// ---------------------------------------------------------------------------

pub struct Impls {
    pub c: DriverFn,
    pub rust: DriverFn,
}

fn load(path: &PathBuf) -> DriverFn {
    // Leaked so the returned function pointer stays valid for the whole test.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
    }));
    let sym: Symbol<'static, DriverFn> = unsafe {
        lib.get(b"driver\0")
            .unwrap_or_else(|e| panic!("dlsym driver in {}: {e}", path.display()))
    };
    *sym
}

impl Impls {
    pub fn load() -> Impls {
        Impls {
            c: load(&c_so_path()),
            rust: load(&rust_so_path()),
        }
    }
}

// ---------------------------------------------------------------------------
// In-process stdout capture
// ---------------------------------------------------------------------------

/// Serializes stdout capture: fd 1 is process-wide, so two test threads
/// redirecting it at once would interleave or lose each other's output.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// the raw bytes written. Both implementations `printf` through the same libc
/// `stdout`, so the FILE is flushed either side of the redirection.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Held for the whole redirection window. Poisoning is irrelevant here: a
    // panicking test has already failed, and the guard only protects fd 1.
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp = std::env::temp_dir();
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    tmp.push(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp)
        .expect("open capture file");

    let bytes;
    unsafe {
        fflush(std::ptr::null_mut()); // flush everything before stealing fd 1
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut()); // push libc's buffer into the file
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);
        bytes = std::fs::read(&tmp).expect("read capture file");
    }
    let _ = std::fs::remove_file(&tmp);
    bytes
}

/// Calls `driver` in both libraries on the same arguments and asserts the
/// stdout bytes are identical.
///
/// # Safety
/// `s1`/`s2` must be valid NUL-terminated C strings; use the process-isolated
/// [`run_child_case`] path for inputs that are expected to fault.
pub unsafe fn assert_same(impls: &Impls, s1: *const c_char, s2: *const c_char, ctx: &str) {
    let c_out = capture_stdout(|| (impls.c)(s1, s2));
    let r_out = capture_stdout(|| (impls.rust)(s1, s2));
    assert_eq!(
        c_out,
        r_out,
        "stdout differs for {ctx}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    assert!(!c_out.is_empty(), "C produced no output for {ctx}");
}

/// Same as [`assert_same`] but for byte slices, which are copied into freshly
/// NUL-terminated buffers.
pub fn assert_same_bytes(impls: &Impls, s1: &[u8], s2: &[u8], ctx: &str) {
    let a = cstr(s1);
    let b = cstr(s2);
    unsafe { assert_same(impls, a.as_ptr() as *const c_char, b.as_ptr() as *const c_char, ctx) }
}

/// Also returns the (identical) output so callers can additionally check the
/// value against an independently computed expectation.
pub fn same_bytes_out(impls: &Impls, s1: &[u8], s2: &[u8], ctx: &str) -> Vec<u8> {
    let a = cstr(s1);
    let b = cstr(s2);
    let (p, q) = (a.as_ptr() as *const c_char, b.as_ptr() as *const c_char);
    let c_out = capture_stdout(|| unsafe { (impls.c)(p, q) });
    let r_out = capture_stdout(|| unsafe { (impls.rust)(p, q) });
    assert_eq!(
        c_out,
        r_out,
        "stdout differs for {ctx}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    c_out
}

pub fn cstr(b: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(b.len() + 1);
    v.extend_from_slice(b);
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Guarded (page-protected) buffers, for over-read / unterminated cases
// ---------------------------------------------------------------------------

/// Maps `rw_pages` readable+writable pages followed by one `PROT_NONE` guard
/// page. Reading past the end of the readable region faults, exactly as it
/// would at the end of a real mapping.
pub fn guarded_region(rw_pages: usize) -> *mut u8 {
    let total = (rw_pages + 1) * PAGE;
    unsafe {
        let base = mmap(
            std::ptr::null_mut(),
            total,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        );
        assert!(base as isize != -1, "mmap failed");
        assert!(
            mprotect(base.add(rw_pages * PAGE), PAGE, PROT_NONE) == 0,
            "mprotect guard page failed"
        );
        base as *mut u8
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    /// A non-NUL byte drawn from `lo..=hi`.
    pub fn byte_in(&mut self, lo: u8, hi: u8) -> u8 {
        lo + (self.next_u64() % (hi as u64 - lo as u64 + 1)) as u8
    }
    pub fn bytes(&mut self, len: usize, lo: u8, hi: u8) -> Vec<u8> {
        (0..len).map(|_| self.byte_in(lo, hi)).collect()
    }
}

pub const SEED: u64 = 0x5EED_1234;
