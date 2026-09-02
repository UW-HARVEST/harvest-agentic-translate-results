//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded through `libloading` as an external caller would;
//! no Rust function is ever called directly, so the `#[no_mangle]` export
//! wrappers are exercised too.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need for fd-level stdout capture and fork-based crash tests.
// These resolve against the glibc already linked into the test binary.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

pub type FooFn = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
/// Deliberately declared with an `int` second parameter so a test can put
/// garbage in the upper bits of the argument register (`ERRORS.md` row 8).
pub type FooIntFn = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
pub type DriverFn = unsafe extern "C" fn(*const c_char);

// ---------------------------------------------------------------------------
// Library locations
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let root = workspace_root();
    for cand in [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "C shared library not found; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
}

pub fn rust_so_path() -> PathBuf {
    // Allows the sweep script to point the tests at a specific build profile.
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO={} does not exist", p.display());
        return p;
    }
    let root = workspace_root();
    for cand in [
        root.join("translation/target/release/libdriver.so"),
        root.join("translation/target/debug/libdriver.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust shared library not found; build it with:\n  cd translation && cargo build --release"
    );
}

/// The two loaded libraries. Leaked so the returned `Symbol`s stay valid for
/// the whole test process.
pub struct Libs {
    pub c: &'static Library,
    pub rust: &'static Library,
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        // SAFETY: both paths point at plain C-ABI shared objects with no
        // initialisers beyond the usual libc ones.
        let c = unsafe { Library::new(c_so_path()) }.expect("load C .so");
        let rust = unsafe { Library::new(rust_so_path()) }.expect("load Rust .so");
        Libs {
            c: Box::leak(Box::new(c)),
            rust: Box::leak(Box::new(rust)),
        }
    })
}

pub fn sym<T>(lib: &'static Library, name: &[u8]) -> Symbol<'static, T> {
    unsafe { lib.get::<T>(name) }
        .unwrap_or_else(|e| panic!("symbol {} missing: {e}", String::from_utf8_lossy(name)))
}

/// `foo` from both libraries.
pub fn foo_pair() -> (Symbol<'static, FooFn>, Symbol<'static, FooFn>) {
    let l = libs();
    (sym(l.c, b"foo\0"), sym(l.rust, b"foo\0"))
}

/// `foo` from both libraries, viewed as taking an `int` search value.
pub fn foo_int_pair() -> (Symbol<'static, FooIntFn>, Symbol<'static, FooIntFn>) {
    let l = libs();
    (sym(l.c, b"foo\0"), sym(l.rust, b"foo\0"))
}

/// `driver` from both libraries.
pub fn driver_pair() -> (Symbol<'static, DriverFn>, Symbol<'static, DriverFn>) {
    let l = libs();
    (sym(l.c, b"driver\0"), sym(l.rust, b"driver\0"))
}

// ---------------------------------------------------------------------------
// NUL-terminated byte buffers
// ---------------------------------------------------------------------------

/// A NUL-terminated buffer whose payload starts at a chosen offset inside a
/// 64-byte-aligned allocation, so tests can vary alignment (`CONFIGS.md` A5).
pub struct CStrBuf {
    backing: Vec<u8>,
    offset: usize,
}

impl CStrBuf {
    /// `bytes` must not contain an interior NUL.
    pub fn new(bytes: &[u8]) -> Self {
        Self::with_alignment(bytes, 0)
    }

    pub fn with_alignment(bytes: &[u8], offset: usize) -> Self {
        assert!(!bytes.contains(&0), "interior NUL is not a C string");
        // Over-allocate, then find an index that is 64-byte aligned and add
        // `offset` to it.
        let mut backing = vec![0u8; bytes.len() + 192];
        let base = backing.as_ptr() as usize;
        let pad = (64 - (base % 64)) % 64;
        let start = pad + offset;
        assert!(start + bytes.len() + 1 <= backing.len());
        backing[start..start + bytes.len()].copy_from_slice(bytes);
        backing[start + bytes.len()] = 0;
        Self {
            backing,
            offset: start,
        }
    }

    pub fn as_ptr(&self) -> *const c_char {
        unsafe { self.backing.as_ptr().add(self.offset) as *const c_char }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

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
    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// A byte in `1..=255` — never 0, so it can be part of a C string or a
    /// legal (non-UB) search value.
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.next_u64() % 255) as u8 + 1
    }
    /// A random string of `len` bytes, all non-zero.
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// fd-level stdout capture (both libraries printf() to fd 1)
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Run `f` with fd 1 redirected into a temporary file and return the bytes it
/// wrote. Serialised globally, because fd 1 is process-wide state.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{:p}.out",
        std::process::id(),
        &_guard as *const _
    ));
    let _ = std::fs::remove_file(&path);

    unsafe {
        // Flush anything already pending so it lands on the real stdout.
        fflush(std::ptr::null_mut());
    }

    let file = std::fs::File::create(&path).expect("create capture file");
    let file_fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file_fd, 1) } >= 0, "dup2 failed");

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);

    let mut out = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut out)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// fork-based crash comparison (for the SIGSEGV rows of ERRORS.md)
// ---------------------------------------------------------------------------

/// How a child process that ran a callback terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    /// Exited normally with this status (callback returned).
    Exited(c_int),
    /// Killed by this signal number.
    Signalled(c_int),
}

/// Fork, run `f` in the child, and report how the child terminated.
///
/// The child calls `_exit` so no Rust destructors, atexit handlers or stdio
/// flushes run, keeping the observation to just "did it fault".
pub fn run_in_child<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        fflush(std::ptr::null_mut());
    }
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        f();
        unsafe { _exit(0) };
    }
    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    // WIFSIGNALED / WTERMSIG / WEXITSTATUS, open-coded for Linux.
    let term_sig = status & 0x7f;
    if term_sig != 0 && term_sig != 0x7f {
        Outcome::Signalled(term_sig)
    } else {
        Outcome::Exited((status >> 8) & 0xff)
    }
}
