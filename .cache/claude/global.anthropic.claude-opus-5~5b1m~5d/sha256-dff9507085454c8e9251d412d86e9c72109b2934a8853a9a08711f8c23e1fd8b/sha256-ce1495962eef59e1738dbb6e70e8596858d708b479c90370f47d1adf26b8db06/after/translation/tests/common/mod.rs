//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls their exported
//! `driver` symbol exactly as an external C consumer would.  Nothing in the
//! Rust crate is called directly, so the `#[no_mangle]` export wrapper is part
//! of what is under test.
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
}

pub const LC_ALL: c_int = 6;

/// `void driver(char c)` — the real ABI signature.
pub type DriverFn = unsafe extern "C" fn(c_char);
/// The same symbol viewed with an `int` parameter, so that a caller can push a
/// value wider than `char` across the FFI boundary (which the C ABI permits).
pub type DriverWideFn = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    driver: DriverFn,
    driver_wide: DriverWideFn,
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        assert!(
            path.exists(),
            "{} not found at {}. Build it first \
             (cmake for C, `cargo build` / `cargo build --release` for Rust).",
            name,
            path.display()
        );
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
            let driver: libloading::Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("symbol `driver` missing from {name}: {e}"));
            let driver = *driver;
            let driver_wide: libloading::Symbol<DriverWideFn> = lib.get(b"driver\0").unwrap();
            let driver_wide = *driver_wide;
            Lib {
                name,
                _lib: lib,
                driver,
                driver_wide,
            }
        }
    }

    pub fn call(&self, c: c_char) {
        unsafe { (self.driver)(c) }
    }

    pub fn call_wide(&self, c: c_int) {
        unsafe { (self.driver_wide)(c) }
    }

    /// The raw address of the exported `driver` symbol, for tests that need to
    /// call it through a deliberately different (mis-declared) signature.
    pub fn raw(&self) -> *const c_void {
        self.driver as *const c_void
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn c_so_path() -> PathBuf {
    let root = workspace_root();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/Debug/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

fn rust_so_path() -> PathBuf {
    // An explicit override lets the same suite be run against every build
    // profile of the crate (the optimiser changes which code paths survive, so
    // debug and release must BOTH be verified).
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer the profile the tests themselves were built with, then the other.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut candidates = Vec::new();
    // The test executable lives in target/<profile>/deps/, so derive the
    // profile directory from it when possible.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile_dir) = deps.parent() {
                candidates.push(profile_dir.join("libdriver.so"));
            }
        }
    }
    candidates.push(manifest.join("target/debug/libdriver.so"));
    candidates.push(manifest.join("target/release/libdriver.so"));
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates.pop().unwrap()
}

/// The two libraries, opened once per test process.
pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn libs() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Lib::open("C libdriver.so", &c_so_path()),
        rs: Lib::open("Rust libdriver.so", &rust_so_path()),
    })
}

/// fd 1 is process-global, so every capture must be serialised.
fn capture_lock() -> &'static Mutex<u64> {
    static L: Mutex<u64> = Mutex::new(0);
    &L
}

/// Runs `f` with fd 1 redirected into a temporary regular FILE (fully
/// buffered) and returns every byte that was written.
pub fn capture_to_file<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut guard = capture_lock().lock().unwrap();
    *guard += 1;
    let seq = *guard;
    let mut path = std::env::temp_dir();
    path.push(format!("drv_cap_{}_{}.out", std::process::id(), seq));

    let bytes = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let file = std::fs::File::create(&path).expect("create capture file");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        drop(file);
        std::fs::read(&path).expect("read capture file")
    };
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Runs `f` with fd 1 redirected into a PIPE (fully buffered, but a different
/// `st_mode` than a regular file, which is what glibc inspects when it picks a
/// buffering mode).  The payload is far below the 64 KiB pipe capacity, so a
/// post-hoc read cannot deadlock.
pub fn capture_to_pipe<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap();
    unsafe {
        let mut fds = [0 as c_int; 2];
        assert!(pipe(fds.as_mut_ptr()) == 0, "pipe() failed");
        let (rd, wr) = (fds[0], fds[1]);

        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(dup2(wr, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        close(wr);

        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = read(rd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
            if out.len() > 1 << 20 {
                break;
            }
        }
        close(rd);
        out
    }
}

/// Sets the process locale, returning whether it succeeded.
pub fn set_locale(name: &str) -> bool {
    let mut z: Vec<u8> = name.as_bytes().to_vec();
    z.push(0);
    unsafe { !setlocale(LC_ALL, z.as_ptr() as *const c_char).is_null() }
}

/// Queries the current process locale (`setlocale(LC_ALL, NULL)`).
pub fn query_locale() -> String {
    unsafe {
        let p = setlocale(LC_ALL, std::ptr::null());
        if p.is_null() {
            return String::from("<null>");
        }
        let n = strlen(p);
        let s = std::slice::from_raw_parts(p as *const u8, n);
        String::from_utf8_lossy(s).into_owned()
    }
}

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Asserts that the C and Rust byte streams are identical.
#[track_caller]
pub fn assert_same(ctx: &str, c_out: &[u8], rs_out: &[u8]) {
    if c_out != rs_out {
        panic!(
            "DIVERGENCE [{ctx}]\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
            c_out.len(),
            show(c_out),
            rs_out.len(),
            show(rs_out)
        );
    }
}

/// Runs one `driver(c)` call on each library, each in its own capture, and
/// compares the byte streams.
#[track_caller]
pub fn diff_char(c: c_char) {
    let p = libs();
    let a = capture_to_file(|| p.c.call(c));
    let b = capture_to_file(|| p.rs.call(c));
    assert_same(&format!("driver({c})"), &a, &b);
}

#[track_caller]
pub fn diff_wide(c: c_int) {
    let p = libs();
    let a = capture_to_file(|| p.c.call_wide(c));
    let b = capture_to_file(|| p.rs.call_wide(c));
    assert_same(&format!("driver(wide {c})"), &a, &b);
}

/// Deterministic xorshift64* PRNG — fixed seed for reproducibility.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    /// A uniformly random `char` over the whole `-128 ..= 127` range.
    pub fn next_char(&mut self) -> c_char {
        (self.next_u64() as u8) as c_char
    }
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
}

pub const ALL_CHARS: std::ops::RangeInclusive<i16> = -128..=127;
