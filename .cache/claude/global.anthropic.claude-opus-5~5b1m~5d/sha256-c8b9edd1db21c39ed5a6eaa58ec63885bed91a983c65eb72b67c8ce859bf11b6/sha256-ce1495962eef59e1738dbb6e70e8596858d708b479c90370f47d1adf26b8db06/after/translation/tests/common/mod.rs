//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries with `libloading` and calls them only through
//! their exported C symbols:
//!
//!   * C    -> `c_src/build/libdriver.so`
//!   * Rust -> `translation/target/{release,debug}/libdriver.so`
//!
//! No Rust function is ever called directly, so the `#[no_mangle]` export
//! wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::fd::AsRawFd;
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need (declared directly; no `libc` crate dependency)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
    fn alarm(seconds: u32) -> u32;
}

// ---------------------------------------------------------------------------
// Exported signatures
// ---------------------------------------------------------------------------

/// `int foo(const char *in, char c)`
pub type FooFn = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
/// `void driver(const char *in)`
pub type DriverFn = unsafe extern "C" fn(*const c_char);

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub foo: FooFn,
    pub driver: DriverFn,
    _lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
            let foo: Symbol<FooFn> = lib
                .get(b"foo\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `foo`: {e}"));
            let foo = *foo;
            let driver: Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `driver`: {e}"));
            let driver = *driver;
            Impl {
                name,
                path,
                foo,
                driver,
                _lib: lib,
            }
        }
    }
}

pub struct Libs {
    pub c: Impl,
    pub rs: Impl,
}

// `Library` handles + plain `fn` pointers are fine to share across threads;
// all mutation-sensitive work (stdout capture) is serialised by CAPTURE_LOCK.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

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
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {}. Build it with: cargo build --release --offline",
        base.display()
    );
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| Libs {
        c: Impl::load("C", c_so_path()),
        rs: Impl::load("Rust", rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — property-style testing with a fixed seed
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
    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    /// Any byte in `0x01..=0xFF` (never 0, so strings stay NUL-terminated).
    pub fn nonzero_byte(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }
    /// Printable-ish ASCII byte in `0x20..=0x7E`.
    pub fn ascii_byte(&mut self) -> u8 {
        0x20 + (self.next_u64() % (0x7F - 0x20)) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Differential helpers: `foo`
// ---------------------------------------------------------------------------

/// Calls `foo` in both libraries with the same NUL-terminated buffer and
/// asserts the returned counts are identical.
pub fn diff_foo(bytes: &[u8], needle: i8, ctx: &str) -> c_int {
    assert!(
        !bytes.contains(&0),
        "{ctx}: test bug — haystack must not contain an interior NUL"
    );
    let cs = CString::new(bytes).expect("interior NUL");
    let l = libs();
    let (c_res, rs_res) = unsafe {
        // Same pointer handed to both implementations.
        let p = cs.as_ptr();
        ((l.c.foo)(p, needle), (l.rs.foo)(p, needle))
    };
    assert_eq!(
        c_res,
        rs_res,
        "foo divergence [{ctx}] needle={needle} (0x{:02x}) len={} haystack={:?}",
        needle as u8,
        bytes.len(),
        preview(bytes)
    );
    c_res
}

/// Reference count computed independently of both libraries, to make sure the
/// two implementations are not "identically wrong" in a trivial way.
pub fn expected_count(bytes: &[u8], needle: i8) -> c_int {
    if needle == 0 {
        // Not modelled: UB in the C code (see ERRORS.md E3).
        panic!("expected_count is undefined for needle == 0");
    }
    bytes.iter().filter(|&&b| b == needle as u8).count() as c_int
}

pub fn preview(bytes: &[u8]) -> String {
    let n = bytes.len().min(64);
    let mut s = String::new();
    for &b in &bytes[..n] {
        if (0x20..0x7F).contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{b:02x}"));
        }
    }
    if bytes.len() > n {
        s.push_str("...");
    }
    s
}

// ---------------------------------------------------------------------------
// Differential helpers: `driver` (stdout is the observable output)
// ---------------------------------------------------------------------------

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_SEQ: Mutex<u64> = Mutex::new(0);

/// Runs `f` in a forked child whose file descriptor 1 is a fresh temporary
/// file, and returns the exact bytes the child wrote to it.
///
/// A forked child (rather than an in-process `dup2` of fd 1) is used on
/// purpose: the `cargo test` harness keeps writing its own progress lines to
/// the real fd 1 from another thread, and those writes would otherwise land in
/// the capture file and corrupt the comparison. The child gets a private copy
/// of fd 1 and of the libc stdout buffer, so the capture contains *only* what
/// the library under test printed.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap();
    let seq = {
        let mut s = CAPTURE_SEQ.lock().unwrap();
        *s += 1;
        *s
    };
    let path = std::env::temp_dir().join(format!("driver_cap_{}_{seq}.bin", std::process::id()));
    // Everything that allocates happens before the fork.
    let file = std::fs::File::create(&path).expect("create temp capture file");
    let fd = file.as_raw_fd();

    let status = unsafe {
        // Drain the parent's pending libc output so the child does not inherit
        // (and re-emit) a copy of it.
        assert_eq!(fflush(std::ptr::null_mut()), 0, "pre-flush failed");
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // Child: only fd juggling + the library call + flush + _exit.
            if dup2(fd, 1) < 0 {
                _exit(90);
            }
            alarm(20);
            f();
            if fflush(std::ptr::null_mut()) != 0 {
                _exit(91);
            }
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        status
    };
    assert_eq!(
        decode(status),
        Outcome::Exited(0),
        "stdout-capture child terminated abnormally: {:?}",
        decode(status)
    );

    drop(file);
    let data = std::fs::read(&path).expect("read temp capture file");
    let _ = std::fs::remove_file(&path);
    data
}

/// Calls `driver` in both libraries with the same buffer, capturing stdout for
/// each, and asserts the byte streams are identical.
pub fn diff_driver(bytes: &[u8], ctx: &str) -> Vec<u8> {
    assert!(
        !bytes.contains(&0),
        "{ctx}: test bug — input must not contain an interior NUL"
    );
    let cs = CString::new(bytes).expect("interior NUL");
    let l = libs();

    let c_out = capture_stdout(|| unsafe { (l.c.driver)(cs.as_ptr()) });
    let rs_out = capture_stdout(|| unsafe { (l.rs.driver)(cs.as_ptr()) });

    assert_eq!(
        c_out,
        rs_out,
        "driver stdout divergence [{ctx}] len={} input={:?}\n  C   : {:?}\n  Rust: {:?}",
        bytes.len(),
        preview(bytes),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rs_out)
    );
    c_out
}

// ---------------------------------------------------------------------------
// Crash-parity helpers (for the C code's undefined-behaviour inputs)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

fn decode(status: c_int) -> Outcome {
    if status & 0x7f == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else {
        Outcome::Signaled(status & 0x7f)
    }
}

/// Runs `f` in a forked child with fd 1 and fd 2 pointed at `/dev/null` and an
/// `alarm(timeout)` watchdog, then reports how the child terminated.
///
/// `f` must not allocate (it runs after `fork()`); the closure should only call
/// into the loaded `.so`s and then `_exit`.
pub fn child_outcome<F: FnOnce() -> i32>(timeout: u32, f: F) -> Outcome {
    // Everything that could allocate happens BEFORE the fork.
    let devnull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .expect("open /dev/null");
    let dn = devnull.as_raw_fd();

    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            dup2(dn, 1);
            dup2(dn, 2);
            alarm(timeout);
            let code = f();
            _exit(code & 0xff);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        decode(status)
    }
}

/// Signal numbers that indicate a memory-safety fault (what the C code does
/// on its unchecked inputs).
pub const SIGSEGV: i32 = 11;
pub const SIGBUS: i32 = 7;
pub const SIGALRM: i32 = 14;
pub const SIGABRT: i32 = 6;
