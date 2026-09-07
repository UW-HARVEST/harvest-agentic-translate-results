//! Shared differential-test harness.
//!
//! Both the original C shared object and the translated Rust shared object are
//! loaded with `libloading` and driven purely through their exported C symbols.
//! Rust functions are NEVER called directly, so the `#[no_mangle] extern "C"`
//! wrappers are part of what is under test.
//!
//! Because every function in this library communicates only through `stdout`,
//! the harness captures `stdout` around each individual call by redirecting
//! file descriptor 1 to a scratch file, and compares the captured bytes.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits needed for stdout capture (no `libc` crate dependency required)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn ftruncate(fd: c_int, length: i64) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;
const SEEK_SET: c_int = 0;

// ---------------------------------------------------------------------------
// The exported ABI of the library under test
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    print_line: unsafe extern "C" fn(*const c_char),
    print_int_line: unsafe extern "C" fn(c_int),
    bad: unsafe extern "C" fn(),
    good: unsafe extern "C" fn(),
    driver: unsafe extern "C" fn(),
    _lib: &'static Library,
}

impl Api {
    /// `void printLine(const char *)`
    pub fn print_line(&self, line: *const c_char) {
        unsafe { (self.print_line)(line) }
    }
    /// `void printIntLine(int)`
    pub fn print_int_line(&self, n: c_int) {
        unsafe { (self.print_int_line)(n) }
    }
    /// `void bad(void)`
    pub fn bad(&self) {
        unsafe { (self.bad)() }
    }
    /// `void good(void)`
    pub fn good(&self) {
        unsafe { (self.good)() }
    }
    /// `void driver(void)`
    pub fn driver(&self) {
        unsafe { (self.driver)() }
    }

    fn load(name: &'static str, path: &Path) -> Api {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()))
        }));
        unsafe fn sym<T: Copy>(lib: &'static Library, s: &str) -> T {
            let symbol: Symbol<T> = lib
                .get(format!("{s}\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing exported symbol `{s}`: {e}"));
            *symbol
        }
        unsafe {
            Api {
                name,
                print_line: sym(lib, "printLine"),
                print_int_line: sym(lib, "printIntLine"),
                bad: sym(lib, "bad"),
                good: sym(lib, "good"),
                driver: sym(lib, "driver"),
                _lib: lib,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // The test executable lives in target/<profile>/deps/, so the cdylib built
    // by the very same `cargo test` invocation sits next to target/<profile>/.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|s| s == "deps").unwrap_or(false) {
        dir.pop();
    }
    let candidate = dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    for profile in ["release", "debug"] {
        let p = manifest_dir().join("target").join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found (looked at {} and target/{{release,debug}}/libdriver.so). \
         Build it with `cargo build --release`.",
        candidate.display()
    );
}

pub fn c_api() -> &'static Api {
    static C: OnceLock<Api> = OnceLock::new();
    C.get_or_init(|| Api::load("C", &c_so_path()))
}

pub fn rust_api() -> &'static Api {
    static R: OnceLock<Api> = OnceLock::new();
    R.get_or_init(|| Api::load("Rust", &rust_so_path()))
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so captures must never overlap.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Runs `f` with fd 1 redirected to a scratch file and returns everything that
/// was written to `stdout` (including output produced by the C `stdio` buffers
/// of the loaded shared objects).
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_diff_capture_{}_{:?}.bin",
        std::process::id(),
        std::thread::current().id()
    ));
    let cpath = {
        let mut v = path.as_os_str().as_encoded_bytes().to_vec();
        v.push(0);
        v
    };

    unsafe {
        // Make sure nothing already buffered leaks into the capture.
        fflush(std::ptr::null_mut());

        let fd = open(cpath.as_ptr() as *const c_char, O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "cannot create capture file {}", path.display());
        ftruncate(fd, 0);

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 onto stdout failed");

        f();

        // Flush every stdio stream (both .so's share the process stdio) before
        // putting the real stdout back.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "restoring stdout failed");
        close(saved);

        lseek(fd, 0, SEEK_SET);
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fd);
        let _ = std::fs::remove_file(&path);
        out
    }
}

// ---------------------------------------------------------------------------
// differential assertion
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(256) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > 256 {
        s.push_str(&format!("...(+{} bytes)", bytes.len() - 256));
    }
    s
}

/// Runs the same closure against the C API and the Rust API and asserts that
/// the bytes they wrote to `stdout` are identical.
pub fn assert_same<F>(case: &str, f: F)
where
    F: Fn(&'static Api),
{
    let c = capture_stdout(|| f(c_api()));
    let r = capture_stdout(|| f(rust_api()));
    if c != r {
        panic!(
            "DIVERGENCE in case `{case}`\n  C   ({:>5} bytes): \"{}\"\n  Rust({:>5} bytes): \"{}\"",
            c.len(),
            show(&c),
            r.len(),
            show(&r)
        );
    }
}

/// Same as [`assert_same`] but additionally pins the expected byte stream, so a
/// regression that changes *both* implementations cannot slip through.
pub fn assert_same_and_eq<F>(case: &str, expected: &[u8], f: F)
where
    F: Fn(&'static Api),
{
    assert_same(case, &f);
    let c = capture_stdout(|| f(c_api()));
    assert_eq!(
        show(&c),
        show(expected),
        "case `{case}`: C output does not match the hand-derived expectation"
    );
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x0123_4567_89ab_cdef;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Inclusive range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// A NUL-terminated buffer whose interior bytes are arbitrary non-NUL values.
pub struct CBuf(Vec<u8>);

impl CBuf {
    pub fn new(bytes: &[u8]) -> CBuf {
        assert!(!bytes.contains(&0), "interior NUL would truncate the string");
        let mut v = bytes.to_vec();
        v.push(0);
        CBuf(v)
    }
    pub fn as_ptr(&self) -> *const c_char {
        self.0.as_ptr() as *const c_char
    }
}
