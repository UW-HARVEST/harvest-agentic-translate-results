//! Shared differential-test harness.
//!
//! Both libraries under test are loaded as shared objects with `libloading` and
//! called only through their exported `driver` symbol — the Rust implementation
//! is never called directly, so the `#[no_mangle] extern "C"` wrapper is part of
//! what gets exercised.
//!
//! `driver` returns `void` and reports its result by `printf`-ing to stdout, so
//! the observable output is a byte stream on file descriptor 1. The harness
//! captures it by redirecting fd 1 to a scratch file and reading back exactly
//! the bytes each individual call produced.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

use libloading::{Library, Symbol};

/// Signature of the one exported function: `void driver(const char*, const char*)`.
pub type DriverFn = unsafe extern "C" fn(*const c_char, *const c_char);

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, which is what we need:
    /// the C `.so` and the Rust `.so` both write through the process-wide
    /// `stdout`, and with fd 1 pointed at a file that stream is fully buffered.
    fn fflush(stream: *mut c_void) -> c_int;
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
}

const PROT_NONE: c_int = 0;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;
const MAP_FAILED: *mut c_void = usize::MAX as *mut c_void;
const PAGE: usize = 4096;

/// Serialises fd-1 redirection: the descriptor table is process-global, so two
/// captures must never overlap. The suites are additionally run with
/// `--test-threads=1`, this is belt and braces.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C `libdriver.so` produced by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("../c_src/build/libdriver.so")
}

/// Path to the Rust `cdylib`. Prefers the release artifact, falls back to debug.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let release = manifest_dir().join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    manifest_dir().join("target/debug/libdriver.so")
}

/// A loaded `.so` plus its resolved `driver` symbol.
pub struct Lib {
    // Field order matters: `driver` borrows from `lib`, so declare it first so
    // it is dropped first.
    driver: Symbol<'static, DriverFn>,
    _lib: &'static Library,
    pub label: &'static str,
}

impl Lib {
    fn open(path: &PathBuf, label: &'static str) -> Lib {
        // SAFETY: the path points at one of the two libraries under test; both
        // are plain C-ABI shared objects with no initialisers of their own.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), label));
        // Leaked deliberately: the libraries stay loaded for the whole process,
        // which lets the `Symbol` borrow live for `'static`.
        let lib: &'static Library = Box::leak(Box::new(lib));
        // SAFETY: `driver` is declared as `void driver(const char*, const char*)`
        // in `c_src/include/driver.h`, which is exactly `DriverFn`.
        let driver: Symbol<'static, DriverFn> = unsafe { lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("no `driver` symbol in {} ({}): {e}", path.display(), label));
        Lib {
            driver,
            _lib: lib,
            label,
        }
    }

    /// Invoke the library's exported `driver`.
    ///
    /// # Safety
    ///
    /// The pointers must satisfy whatever contract the call under test requires;
    /// error-path tests deliberately pass invalid ones and run in a child
    /// process for that reason.
    pub unsafe fn call(&self, s1: *const c_char, s2: *const c_char) {
        unsafe { (self.driver)(s1, s2) }
    }
}

/// The C library and the Rust library, both loaded via `libloading`.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Lib::open(&c_so_path(), "C"),
        rust: Lib::open(&rust_so_path(), "Rust"),
    }
}

pub fn load_one(which: &str) -> Lib {
    match which {
        "c" => Lib::open(&c_so_path(), "C"),
        "rust" => Lib::open(&rust_so_path(), "Rust"),
        other => panic!("unknown library selector {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// An active fd-1 redirection. Each `run` returns exactly the bytes that the
/// closure appended to the stream, so many calls can be compared inside a single
/// redirection without per-call file churn.
pub struct Capture {
    saved_fd: c_int,
    file: File,
    pos: u64,
    path: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl Capture {
    pub fn begin() -> Capture {
        let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let path = std::env::temp_dir().join(format!(
            "driver-difftest-{}-{:?}.out",
            std::process::id(),
            std::thread::current().id()
        ));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("create capture scratch file");

        // Flush anything already buffered on the real stdout so it does not end
        // up in our scratch file.
        // SAFETY: `fflush(NULL)` is always valid.
        unsafe { fflush(std::ptr::null_mut()) };

        // SAFETY: fd 1 is open; `dup`/`dup2` on it is well defined.
        let saved_fd = unsafe { dup(1) };
        assert!(saved_fd >= 0, "dup(1) failed");
        let rc = unsafe { dup2(file.as_raw_fd(), 1) };
        assert!(rc >= 0, "dup2 onto fd 1 failed");

        Capture {
            saved_fd,
            file,
            pos: 0,
            path,
            _guard: guard,
        }
    }

    /// Run `f` and return the bytes it wrote to stdout.
    pub fn run(&mut self, f: impl FnOnce()) -> Vec<u8> {
        f();
        // SAFETY: `fflush(NULL)` is always valid; it pushes the libraries'
        // buffered `printf` output into the scratch file.
        unsafe { fflush(std::ptr::null_mut()) };

        self.file
            .seek(SeekFrom::Start(self.pos))
            .expect("seek capture file");
        let mut buf = Vec::new();
        self.file
            .read_to_end(&mut buf)
            .expect("read capture file");
        self.pos += buf.len() as u64;
        buf
    }

    /// Restore the real stdout immediately. Idempotent, and called from `Drop`,
    /// so a panicking test's diagnostics still reach the terminal.
    pub fn restore(&mut self) {
        if self.saved_fd >= 0 {
            // SAFETY: restoring the saved descriptor onto fd 1 and closing the
            // copy; `saved_fd` is set to -1 so this runs at most once.
            unsafe {
                fflush(std::ptr::null_mut());
                dup2(self.saved_fd, 1);
                close(self.saved_fd);
            }
            self.saved_fd = -1;
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.restore();
        let _ = std::fs::remove_file(&self.path);
    }
}

/// `dup2(fd, 1)`, exposed for the error-path child process, which must point its
/// own stdout at the file the parent collects.
///
/// # Safety
///
/// `fd` must be an open file descriptor.
pub unsafe fn redirect_fd1(fd: c_int) -> c_int {
    unsafe { dup2(fd, 1) }
}

/// `fflush(NULL)`, exposed so callers can push both libraries' buffered output.
pub fn flush_all_streams() {
    // SAFETY: `fflush(NULL)` is always valid.
    unsafe { fflush(std::ptr::null_mut()) };
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // Any non-zero state works; fold the seed so seed 0 is still usable.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform-ish value in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }

    /// A non-NUL byte drawn from `alphabet`.
    pub fn pick(&mut self, alphabet: &[u8]) -> u8 {
        alphabet[self.below(alphabet.len())]
    }

    /// A random NUL-free byte string of the given length over `alphabet`.
    pub fn bytes(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len).map(|_| self.pick(alphabet)).collect()
    }

    /// A random NUL-free byte string whose length is drawn from
    /// `lo..=hi_inclusive`.
    pub fn bytes_range(&mut self, lo: usize, hi_inclusive: usize, alphabet: &[u8]) -> Vec<u8> {
        let len = self.range(lo, hi_inclusive);
        self.bytes(len, alphabet)
    }
}

/// Every byte value that may legally appear inside a C string.
pub fn full_alphabet() -> Vec<u8> {
    (1u8..=255).collect()
}

pub fn ascii_alphabet() -> Vec<u8> {
    (0x20u8..=0x7e).collect()
}

pub fn high_alphabet() -> Vec<u8> {
    (0x80u8..=0xff).collect()
}

// ---------------------------------------------------------------------------
// C-string buffers, including page-guarded ones
// ---------------------------------------------------------------------------

/// A NUL-terminated copy of `bytes` on the heap.
pub struct CStrBuf(Vec<u8>);

impl CStrBuf {
    pub fn new(bytes: &[u8]) -> CStrBuf {
        assert!(!bytes.contains(&0), "C strings cannot contain NUL");
        let mut v = Vec::with_capacity(bytes.len() + 1);
        v.extend_from_slice(bytes);
        v.push(0);
        CStrBuf(v)
    }

    pub fn ptr(&self) -> *const c_char {
        self.0.as_ptr() as *const c_char
    }
}

/// A NUL-terminated copy of `bytes` starting at `offset` inside a heap buffer,
/// used to vary pointer alignment (glibc's `strcspn` uses aligned SIMD loads).
pub struct OffsetStrBuf {
    buf: Vec<u8>,
    offset: usize,
}

impl OffsetStrBuf {
    pub fn new(bytes: &[u8], offset: usize) -> OffsetStrBuf {
        assert!(!bytes.contains(&0));
        let mut buf = vec![0x41u8; offset];
        buf.extend_from_slice(bytes);
        buf.push(0);
        OffsetStrBuf { buf, offset }
    }

    pub fn ptr(&self) -> *const c_char {
        // SAFETY: `offset` bytes were pushed before the payload.
        unsafe { self.buf.as_ptr().add(self.offset) as *const c_char }
    }
}

/// Two pages of memory whose second page is `PROT_NONE`, so any read past the
/// end of the first page faults. Strings are placed flush against that guard.
pub struct GuardedBuf {
    base: *mut u8,
    ptr: *const c_char,
}

impl GuardedBuf {
    fn map() -> *mut u8 {
        // SAFETY: a fresh anonymous mapping request with valid arguments.
        let base = unsafe {
            mmap(
                std::ptr::null_mut(),
                2 * PAGE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(base != MAP_FAILED && !base.is_null(), "mmap failed");
        // SAFETY: the second page belongs to the mapping we just created.
        let rc = unsafe { mprotect((base as *mut u8).add(PAGE) as *mut c_void, PAGE, PROT_NONE) };
        assert!(rc == 0, "mprotect failed");
        base as *mut u8
    }

    /// `bytes` followed by a NUL, positioned so that the NUL is the last
    /// readable byte before the guard page.
    pub fn terminated(bytes: &[u8]) -> GuardedBuf {
        assert!(!bytes.contains(&0));
        assert!(bytes.len() < PAGE);
        let base = Self::map();
        let start = PAGE - (bytes.len() + 1);
        // SAFETY: `start .. PAGE` lies inside the readable first page.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), base.add(start), bytes.len());
            *base.add(PAGE - 1) = 0;
            GuardedBuf {
                base,
                ptr: base.add(start) as *const c_char,
            }
        }
    }

    /// `bytes` with **no** terminator, positioned so the byte immediately after
    /// the payload is on the guard page: reading past the payload faults.
    pub fn unterminated(bytes: &[u8]) -> GuardedBuf {
        assert!(!bytes.contains(&0));
        assert!(!bytes.is_empty() && bytes.len() <= PAGE);
        let base = Self::map();
        let start = PAGE - bytes.len();
        // SAFETY: `start .. PAGE` lies inside the readable first page.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), base.add(start), bytes.len());
            GuardedBuf {
                base,
                ptr: base.add(start) as *const c_char,
            }
        }
    }

    /// A pointer into the `PROT_NONE` page: reading even its first byte faults.
    pub fn unmapped() -> GuardedBuf {
        let base = Self::map();
        // SAFETY: the guard page is part of our mapping (just unreadable).
        let ptr = unsafe { base.add(PAGE) as *const c_char };
        GuardedBuf { base, ptr }
    }

    pub fn ptr(&self) -> *const c_char {
        self.ptr
    }
}

impl Drop for GuardedBuf {
    fn drop(&mut self) {
        // SAFETY: unmapping exactly the region we mapped.
        unsafe { munmap(self.base as *mut c_void, 2 * PAGE) };
    }
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// A single differential case: call C then Rust with the same pointers inside
/// one capture and require byte-identical output.
///
/// # Safety
///
/// The pointers must be valid NUL-terminated C strings.
pub unsafe fn assert_same(
    pair: &Pair,
    cap: &mut Capture,
    s1: *const c_char,
    s2: *const c_char,
    describe: impl Fn() -> String,
) {
    let c_out = cap.run(|| unsafe { pair.c.call(s1, s2) });
    let rust_out = cap.run(|| unsafe { pair.rust.call(s1, s2) });
    if c_out != rust_out {
        // Restore stdout before panicking so the message is visible.
        cap.restore();
        panic!(
            "output mismatch for {}\n  C   : {:?}\n  Rust: {:?}",
            describe(),
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out),
        );
    }
    if c_out.is_empty() {
        cap.restore();
        panic!("expected the library to print something for {}", describe());
    }
}

/// Convenience wrapper for the common "two byte slices" case.
pub fn assert_same_bytes(pair: &Pair, cap: &mut Capture, s1: &[u8], s2: &[u8]) {
    let a = CStrBuf::new(s1);
    let b = CStrBuf::new(s2);
    // SAFETY: both buffers are NUL-terminated and outlive the calls.
    unsafe {
        assert_same(pair, cap, a.ptr(), b.ptr(), || {
            format!("s1={:?} s2={:?}", Escaped(s1), Escaped(s2))
        })
    }
}

/// Compact, deterministic rendering of arbitrary bytes for failure messages.
pub struct Escaped<'a>(pub &'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{} bytes]", self.0.len())?;
        let shown = &self.0[..self.0.len().min(48)];
        write!(f, " ")?;
        for b in shown {
            if b.is_ascii_graphic() {
                write!(f, "{}", *b as char)?;
            } else {
                write!(f, "\\x{b:02x}")?;
            }
        }
        if self.0.len() > shown.len() {
            write!(f, "...")?;
        }
        Ok(())
    }
}

/// Reference model of the C behaviour, used only to sanity-check that a
/// generated case really exercises the intended code path (never as the oracle —
/// the oracle is always the C `.so`).
pub fn expected_strcspn(s1: &[u8], s2: &[u8]) -> usize {
    s1.iter().position(|c| s2.contains(c)).unwrap_or(s1.len())
}
