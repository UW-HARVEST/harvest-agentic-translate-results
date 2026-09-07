//! Shared differential-testing harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! exclusively through their exported C symbols. No Rust function from the
//! crate under test is ever called directly, so the `#[no_mangle]` /
//! `extern "C"` export wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::{CString, c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// raw libc bindings (the `libc` crate is not a dependency of this crate)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn alarm(seconds: u32) -> u32;
}

const O_WRONLY: c_int = 0o1;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

// ---------------------------------------------------------------------------
// signatures of the symbols under test
// ---------------------------------------------------------------------------

/// `int foo(const char *in, char c)` — declared with the real narrow `char`.
pub type FooFn = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
/// The same symbol, deliberately mis-declared with a full-width `int` needle so
/// we can push values with garbage above bit 7 across the ABI boundary.
pub type FooWideFn = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
/// `void driver(const char *in)`
pub type DriverFn = unsafe extern "C" fn(*const c_char);

/// One loaded implementation.
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    pub fn foo(&self) -> Symbol<'_, FooFn> {
        unsafe { self.lib.get(b"foo\0") }.expect("symbol `foo` missing")
    }
    pub fn foo_wide(&self) -> Symbol<'_, FooWideFn> {
        unsafe { self.lib.get(b"foo\0") }.expect("symbol `foo` missing")
    }
    pub fn driver(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.lib.get(b"driver\0") }.expect("symbol `driver` missing")
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().parent().unwrap().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {p:?}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    // Prefer the profile this test binary was built with, then fall back.
    let target = manifest_dir().join("target");
    let mut candidates = Vec::new();
    if cfg!(debug_assertions) {
        candidates.push(target.join("debug/libdriver.so"));
        candidates.push(target.join("release/libdriver.so"));
    } else {
        candidates.push(target.join("release/libdriver.so"));
        candidates.push(target.join("debug/libdriver.so"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found (looked at {candidates:?}). Build it with \
         `cargo build` and `cargo build --release`."
    )
}

/// Loads both shared objects exactly once for the whole test binary.
pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c = unsafe { Library::new(c_so_path()) }.expect("failed to dlopen C .so");
        let rust = unsafe { Library::new(rust_so_path()) }.expect("failed to dlopen Rust .so");
        Pair {
            c: Impl { name: "C", lib: c },
            rust: Impl { name: "Rust", lib: rust },
        }
    })
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `[0, n)`; `n` must be non-zero.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A byte in `1..=255` (never NUL, so it can go inside a C string).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) as u8) + 1
    }
    pub fn byte_from(&mut self, alphabet: &[u8]) -> u8 {
        alphabet[self.below(alphabet.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// differential assertions
// ---------------------------------------------------------------------------

/// Calls `foo` in both libraries on a NUL-terminated copy of `bytes` and
/// asserts the two return values are identical.
///
/// `bytes` must not contain an interior NUL (`CString` enforces it).
pub fn assert_foo_eq(row: &str, bytes: &[u8], needle: u8) -> c_int {
    let s = CString::new(bytes).expect("interior NUL in test input");
    let p = pair();
    let cf = p.c.foo();
    let rf = p.rust.foo();
    let cv = unsafe { cf(s.as_ptr(), needle as i8 as c_char) };
    let rv = unsafe { rf(s.as_ptr(), needle as i8 as c_char) };
    assert_eq!(
        cv,
        rv,
        "[{row}] foo divergence: C returned {cv}, Rust returned {rv} \
         for needle {needle:#04x} and input {:?} (len {})",
        Preview(bytes),
        bytes.len()
    );
    cv
}

/// Same, but pushes a full-width `int` needle across the ABI boundary.
pub fn assert_foo_wide_eq(row: &str, bytes: &[u8], needle: c_int) -> c_int {
    let s = CString::new(bytes).expect("interior NUL in test input");
    let p = pair();
    let cf = p.c.foo_wide();
    let rf = p.rust.foo_wide();
    let cv = unsafe { cf(s.as_ptr(), needle) };
    let rv = unsafe { rf(s.as_ptr(), needle) };
    assert_eq!(
        cv, rv,
        "[{row}] foo(wide) divergence: C returned {cv}, Rust returned {rv} \
         for needle {needle:#010x} and input {:?}",
        Preview(bytes)
    );
    cv
}

/// Runs `driver` in both libraries with stdout redirected to a temp file and
/// asserts the captured bytes are identical. Returns the shared output.
///
/// The call happens in a forked child so that the only writer to the redirected
/// fd 1 is the library under test — the libtest harness's own progress output
/// is written from other threads of the parent and would otherwise contaminate
/// the capture.
pub fn assert_driver_stdout_eq(row: &str, bytes: &[u8]) -> Vec<u8> {
    let s = CString::new(bytes).expect("interior NUL in test input");
    let p = pair();

    let mut results = Vec::new();
    for imp in [&p.c, &p.rust] {
        let d = imp.driver();
        let path = temp_path(&format!("driver-{}", imp.name));
        let ptr = s.as_ptr();
        let outcome = run_in_child(|| unsafe { d(ptr) }, 60, Some(&path));
        let out = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        results.push((outcome, out));
    }

    let (co, cout) = &results[0];
    let (ro, rout) = &results[1];
    assert_eq!(
        co, ro,
        "[{row}] driver termination divergence for input {:?}: C {co:?} vs Rust {ro:?}",
        Preview(bytes)
    );
    assert_eq!(
        cout,
        rout,
        "[{row}] driver stdout divergence for input {:?} (len {}):\n  C   : {:?}\n  Rust: {:?}",
        Preview(bytes),
        bytes.len(),
        String::from_utf8_lossy(cout),
        String::from_utf8_lossy(rout)
    );
    assert_eq!(
        *co,
        Outcome::Exited(0),
        "[{row}] driver should return normally for a valid input, got {co:?}"
    );
    cout.clone()
}

/// A short, printable rendering of a test input for assertion messages.
pub struct Preview<'a>(pub &'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.0.len().min(64);
        write!(f, "b\"")?;
        for &b in &self.0[..n] {
            if (0x20..0x7f).contains(&b) && b != b'"' && b != b'\\' {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{b:02x}")?;
            }
        }
        if self.0.len() > n {
            write!(f, "...")?;
        }
        write!(f, "\"")
    }
}

// ---------------------------------------------------------------------------
// stdout capture (fd-level, so it catches libc `printf` from the .so)
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: Mutex<()> = Mutex::new(());
    &L
}

fn temp_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-difftest-{}-{}-{}-{tag}",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ))
}

/// Redirects fd 1 to a temp file, runs `f`, flushes all C streams, restores
/// fd 1, and returns everything written.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = temp_path("stdout");
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    unsafe {
        // Flush anything already pending so it lands on the real stdout.
        fflush(std::ptr::null_mut());
        let fd = open(cpath.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open({path:?}) failed");
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        close(fd);

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    let out = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// out-of-process execution, for inputs whose expected behaviour is a fatal
// signal (null dereference, unbounded out-of-bounds walk)
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Outcome {
    /// Normal exit with this status code.
    Exited(c_int),
    /// Killed by this signal number.
    Signalled(c_int),
    /// Neither (should not happen for our cases).
    Other(c_int),
}

/// Forks, runs `f` in the child, and reports how the child terminated.
///
/// `timeout_secs` arms `alarm()` in the child so a pathological walk cannot
/// hang the test run; a `SIGALRM` outcome is therefore distinguishable from a
/// `SIGSEGV` one. `stdout_to` optionally redirects the child's fd 1 to a file
/// so its output can be compared as well.
pub fn run_in_child<F: FnOnce()>(f: F, timeout_secs: u32, stdout_to: Option<&PathBuf>) -> Outcome {
    let redirect = stdout_to.map(|p| CString::new(p.as_os_str().as_encoded_bytes()).unwrap());
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // ---- child ----
            if let Some(cp) = redirect {
                let fd = open(cp.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o600 as c_int);
                if fd >= 0 {
                    dup2(fd, 1);
                    close(fd);
                }
            }
            if timeout_secs > 0 {
                alarm(timeout_secs);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        // ---- parent ----
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert!(r == pid, "waitpid failed");
        decode_status(status)
    }
}

fn decode_status(status: c_int) -> Outcome {
    // glibc wait status layout.
    if status & 0x7f == 0x7f {
        Outcome::Other(status)
    } else if status & 0x7f != 0 {
        Outcome::Signalled(status & 0x7f)
    } else {
        Outcome::Exited((status >> 8) & 0xff)
    }
}

pub const SIGSEGV: c_int = 11;
pub const SIGBUS: c_int = 7;
pub const SIGALRM: c_int = 14;
