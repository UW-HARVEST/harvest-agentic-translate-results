// Shared harness for the C-vs-Rust differential tests.
//
// Both libraries are loaded as shared objects through `libloading` and every
// call crosses the FFI boundary, so the `#[no_mangle] extern "C"` export
// wrappers are exercised exactly as an external consumer would exercise them.
// Rust functions are NEVER called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits used to capture what the loaded libraries print on fd 1.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Library locations
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, produced by the CMake build.
fn c_lib_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// `target/<profile>/libdriver.so`, produced by `cargo build`.
///
/// The profile directory is derived from the running test executable
/// (`target/<profile>/deps/<test>-<hash>`) so that a `--release` test run
/// checks the `--release` shared object.
fn rust_lib_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let candidate = dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    // Fall back to either profile directory.
    for profile in ["debug", "release"] {
        let p = manifest_dir().join("target").join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust shared library not found (looked at {}). Build it with: cargo build",
        candidate.display()
    );
}

// ---------------------------------------------------------------------------
// Loaded libraries. Leaked so the returned function pointers stay valid.
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    lib: &'static Library,
    path: PathBuf,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        Lib { name, lib: Box::leak(Box::new(lib)), path }
    }

    fn sym<T: Copy>(&self, name: &str) -> T {
        let s: Symbol<T> = unsafe { self.lib.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("{} does not export `{name}`: {e}", self.name));
        *s
    }

    /// `int foo(const char *in, char c)` — the exact C prototype.
    pub fn foo(&self) -> unsafe extern "C" fn(*const c_char, c_char) -> c_int {
        self.sym("foo")
    }

    /// Same symbol, but with the needle declared as a full-width `int`. C
    /// callers may legally push any `int`; this is how the out-of-range
    /// "enum-style" argument case is driven across the FFI boundary.
    pub fn foo_wide(&self) -> unsafe extern "C" fn(*const c_char, c_int) -> c_int {
        self.sym("foo")
    }

    /// `void driver(const char *in)`
    pub fn driver(&self) -> unsafe extern "C" fn(*const c_char) {
        self.sym("driver")
    }

    /// Whether `dlsym` resolves `name` in this shared object.
    pub fn exports(&self, name: &str) -> bool {
        unsafe { self.lib.get::<*const ()>(name.as_bytes()) }.is_ok()
    }

    /// Path of the loaded `.so` (used by the symbol-parity test to run `nm`).
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

pub fn c_lib_file() -> PathBuf {
    c_lib_path()
}
pub fn rust_lib_file() -> PathBuf {
    rust_lib_path()
}

pub fn c_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open("C libdriver.so", c_lib_path()))
}

pub fn rust_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open("Rust libdriver.so", rust_lib_path()))
}

// ---------------------------------------------------------------------------
// Differential call helpers
// ---------------------------------------------------------------------------

/// Build a NUL-terminated C string from raw bytes (no UTF-8 validation, no
/// interior NUL allowed - a C string cannot contain one).
pub fn cstr(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("test input must not contain an interior NUL")
}

/// Call `foo` in both libraries and assert the returned counts are identical.
pub fn diff_foo(haystack: &[u8], needle: u8, ctx: &str) -> c_int {
    let s = cstr(haystack);
    let c = needle as i8 as c_char;
    let c_res = unsafe { (c_lib().foo())(s.as_ptr(), c) };
    let r_res = unsafe { (rust_lib().foo())(s.as_ptr(), c) };
    assert_eq!(
        c_res, r_res,
        "foo divergence [{ctx}]: needle={needle:#04x} ({:?}) haystack(len={})={:?}\n  C   -> {c_res}\n  Rust-> {r_res}",
        needle as char,
        haystack.len(),
        Preview(haystack),
    );
    c_res
}

/// Call `foo` in both libraries with a full-width `int` needle.
pub fn diff_foo_wide(haystack: &[u8], needle: c_int, ctx: &str) -> c_int {
    let s = cstr(haystack);
    let c_res = unsafe { (c_lib().foo_wide())(s.as_ptr(), needle) };
    let r_res = unsafe { (rust_lib().foo_wide())(s.as_ptr(), needle) };
    assert_eq!(
        c_res, r_res,
        "foo(wide needle) divergence [{ctx}]: needle={needle} ({needle:#x}) haystack(len={})={:?}\n  C   -> {c_res}\n  Rust-> {r_res}",
        haystack.len(),
        Preview(haystack),
    );
    c_res
}

/// Run `f` with fd 1 redirected into a temporary file and return everything
/// written to it. `fflush(NULL)` is used on both sides of the call so that the
/// libc `stdout` buffer of the loaded library is emptied into the file.
fn capture_fd1<F: FnOnce()>(f: F) -> Vec<u8> {
    // fd 1 is process-global, so only one capture may be in flight at a time.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Push anything Rust has buffered out before fd 1 moves.
    std::io::stdout().flush().ok();
    unsafe { fflush(std::ptr::null_mut()) };

    let mut tmp = tempfile();
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
    unsafe { close(saved) };

    let mut out = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek temp file");
    tmp.read_to_end(&mut out).expect("read temp file");
    out
}

fn tempfile() -> std::fs::File {
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir());
    std::fs::create_dir_all(&dir).ok();
    let unique = format!(
        "driver-diff-{}-{:?}-{}.out",
        std::process::id(),
        std::thread::current().id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let path = dir.join(unique);
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("cannot create temp file {}: {e}", path.display()));
    std::fs::remove_file(&path).ok(); // unlinked; the fd keeps it alive
    f
}
static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Capture the stdout bytes produced by `driver` in one library.
pub fn driver_stdout(lib: &Lib, haystack: &[u8]) -> Vec<u8> {
    let s = cstr(haystack);
    let f = lib.driver();
    capture_fd1(|| unsafe { f(s.as_ptr()) })
}

/// Call `driver` in both libraries and assert their stdout matches byte-for-byte.
pub fn diff_driver(haystack: &[u8], ctx: &str) -> Vec<u8> {
    let c_out = driver_stdout(c_lib(), haystack);
    let r_out = driver_stdout(rust_lib(), haystack);
    assert_eq!(
        c_out,
        r_out,
        "driver stdout divergence [{ctx}]: haystack(len={})={:?}\n  C   -> {:?}\n  Rust-> {:?}",
        haystack.len(),
        Preview(haystack),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
    c_out
}

/// Truncating, escaping preview of a byte slice for assertion messages.
pub struct Preview<'a>(pub &'a [u8]);
impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.0.len().min(80);
        write!(f, "\"")?;
        for &b in &self.0[..n] {
            if b.is_ascii_graphic() || b == b' ' {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{b:02x}")?;
            }
        }
        if self.0.len() > n {
            write!(f, "...(+{} bytes)", self.0.len() - n)?;
        }
        write!(f, "\"")
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) - fixed seeds keep every row reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    /// Any non-NUL byte (`1..=255`) - NUL cannot appear inside a C string.
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.range(1, 255)) as u8
    }
    /// Random bytes from `alphabet`.
    pub fn bytes_from(&mut self, alphabet: &[u8], len: usize) -> Vec<u8> {
        (0..len).map(|_| alphabet[self.below(alphabet.len())]).collect()
    }
    /// Random non-NUL bytes over the whole byte range.
    pub fn nonzero_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }
}

/// Reference count, used only as an independent sanity check on top of the
/// C-vs-Rust comparison (never as the ground truth for a divergence).
pub fn count_bytes(haystack: &[u8], needle: u8) -> c_int {
    haystack.iter().filter(|&&b| b == needle).count() as c_int
}
