//! Shared helpers for the C-vs-Rust differential tests.
//!
//! Design rules:
//! * The Rust side is ALWAYS exercised as an external consumer would: the
//!   executable is spawned as a process, and the library is `dlopen`ed and
//!   called through its exported `main` symbol. No Rust function from this crate
//!   is ever called directly.
//! * Every assertion compares C against Rust. The C result is the ground truth.
//! * Raw libc entry points are declared inline so the tests need no `libc` crate.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------- libc ------

pub const O_RDONLY: c_int = 0;
pub const O_WRONLY: c_int = 1;
pub const O_RDWR: c_int = 2;
pub const O_CREAT: c_int = 64;
pub const O_TRUNC: c_int = 512;
pub const O_APPEND: c_int = 1024;
pub const O_NOCTTY: c_int = 256;

pub const SIGPIPE: c_int = 13;
pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;

extern "C" {
    pub fn close(fd: c_int) -> c_int;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dup2(old: c_int, new: c_int) -> c_int;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    pub fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn fflush(stream: *mut c_void) -> c_int;
    pub fn signal(signum: c_int, handler: usize) -> usize;
    pub fn posix_openpt(flags: c_int) -> c_int;
    pub fn grantpt(fd: c_int) -> c_int;
    pub fn unlockpt(fd: c_int) -> c_int;
    pub fn ptsname(fd: c_int) -> *mut c_char;
}

/// Flush *all* C stdio streams (`fflush(NULL)`).
///
/// Required after calling the C library's `main` through FFI: `puts` only fills
/// glibc's `stdout` buffer, and because the test harness's stdout is not a tty
/// that buffer is block-buffered, so the bytes would not reach the redirected fd
/// until much later. The Rust library flushes explicitly inside `main`, so this
/// makes the two directly comparable.
pub fn fflush_all() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

// ------------------------------------------------------------ artifacts -----

/// C executable built by `build.rs` from the unmodified `c_src/src/main.c`.
pub fn c_exe() -> PathBuf {
    PathBuf::from(env!("C_DRIVER_EXE"))
}

/// The Rust executable this crate ships (built by cargo).
pub fn rust_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// C shared library built by `build.rs` from the unmodified `c_src/src/main.c`.
pub fn c_so() -> PathBuf {
    PathBuf::from(env!("C_DRIVER_SO"))
}

/// Rust shared library (`cdylib`) built from `src/lib.rs`.
pub fn rust_so() -> PathBuf {
    PathBuf::from(env!("RUST_DRIVER_SO"))
}

pub const C_LABEL: &str = "C";
pub const RUST_LABEL: &str = "Rust";

// ------------------------------------------------------------- PRNG ---------

/// Deterministic xorshift64* PRNG — fixed seed so every run is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1EC5;

impl Rng {
    pub fn new() -> Self {
        Rng(SEED)
    }
    pub fn with_seed(s: u64) -> Self {
        Rng(if s == 0 { SEED } else { s })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi.saturating_sub(lo).max(1))
    }
    /// Random bytes, never containing NUL or `=` so they are legal in argv/env.
    pub fn arg_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let b = (self.next_u64() & 0xff) as u8;
                match b {
                    0 | b'=' => b'x',
                    _ => b,
                }
            })
            .collect()
    }
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.next_u64() & 0xff) as u8).collect()
    }
}

// ------------------------------------------------------------ outcome -------

/// Everything externally observable about one run of the program.
#[derive(PartialEq, Eq, Clone)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Outcome {{ stdout: {:?} ({} bytes), stderr: {:?}, exit_code: {:?}, signal: {:?} }}",
            String::from_utf8_lossy(&self.stdout),
            self.stdout.len(),
            String::from_utf8_lossy(&self.stderr),
            self.code,
            self.signal
        )
    }
}

impl Outcome {
    pub fn from_output(o: std::process::Output) -> Self {
        use std::os::unix::process::ExitStatusExt;
        Outcome {
            stdout: o.stdout,
            stderr: o.stderr,
            code: o.status.code(),
            signal: o.status.signal(),
        }
    }
    pub fn from_status(st: std::process::ExitStatus, stdout: Vec<u8>, stderr: Vec<u8>) -> Self {
        use std::os::unix::process::ExitStatusExt;
        Outcome {
            stdout,
            stderr,
            code: st.code(),
            signal: st.signal(),
        }
    }
}

/// The exact bytes the C program prints.
pub const EXPECTED: &[u8] = b"Hello World!\n";

// -------------------------------------------------- differential drivers ----

/// Run `f` for the C executable and then the Rust executable and require the
/// two results to be identical. Returns the (shared) result.
#[track_caller]
pub fn assert_same_exe<T: PartialEq + std::fmt::Debug>(what: &str, f: impl Fn(&Path) -> T) -> T {
    let c = f(&c_exe());
    let r = f(&rust_exe());
    assert!(
        c == r,
        "DIVERGENCE [{what}] between C and Rust executables:\n  {C_LABEL:<5}= {c:?}\n  {RUST_LABEL:<5}= {r:?}"
    );
    c
}

/// The ABI of the only exported symbol: `int main(void)`.
pub type MainFn = unsafe extern "C" fn() -> c_int;
/// Same symbol, deliberately mis-declared with six integer parameters, to prove
/// `main(void)` ignores whatever happens to be in the argument registers.
pub type MainFnJunk = unsafe extern "C" fn(u64, u64, u64, u64, u64, u64) -> c_int;

fn load_main(path: &Path, label: &str) -> MainFn {
    unsafe {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed for {label}: {e}", path.display()));
        let sym: libloading::Symbol<MainFn> = lib
            .get(b"main\0")
            .unwrap_or_else(|e| panic!("dlsym(\"main\") failed for {label}: {e}"));
        let f = *sym;
        // Never unload: the returned pointer must stay valid for the whole run,
        // and dlclose()ing a Rust cdylib would tear down its std runtime.
        std::mem::forget(lib);
        f
    }
}

/// `main` from the C shared library, loaded once per test binary.
pub fn c_main_sym() -> MainFn {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| load_main(&c_so(), C_LABEL) as usize);
    unsafe { std::mem::transmute::<usize, MainFn>(addr) }
}

/// `main` from the Rust shared library, loaded once per test binary.
pub fn rust_main_sym() -> MainFn {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| load_main(&rust_so(), RUST_LABEL) as usize);
    unsafe { std::mem::transmute::<usize, MainFn>(addr) }
}

/// Run `f` against the C `.so`'s `main` and the Rust `.so`'s `main` and require
/// identical results.
#[track_caller]
pub fn assert_same_so<T: PartialEq + std::fmt::Debug>(what: &str, f: impl Fn(MainFn) -> T) -> T {
    let c = f(c_main_sym());
    let r = f(rust_main_sym());
    assert!(
        c == r,
        "DIVERGENCE [{what}] between C and Rust shared libraries:\n  {C_LABEL:<5}= {c:?}\n  {RUST_LABEL:<5}= {r:?}"
    );
    c
}

// ------------------------------------------------- fd-1 capture (FFI) -------

/// fd 1 is process-global, so only one capture may be in flight at a time even
/// though libtest runs tests on multiple threads.
fn fd1_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

pub fn tmp_path(tag: &str) -> PathBuf {
    static N: OnceLock<Mutex<u64>> = OnceLock::new();
    let mut n = N.get_or_init(|| Mutex::new(0)).lock().unwrap();
    *n += 1;
    std::env::temp_dir().join(format!(
        "cdiff-{}-{}-{}-{}",
        tag,
        std::process::id(),
        *n,
        SEED
    ))
}

/// Point fd 1 at a fresh temp file, run `body`, flush C stdio, restore fd 1, and
/// return the bytes that landed in the file.
///
/// This is how the FFI tests observe what the libraries wrote: both libraries
/// write to file descriptor 1, so redirecting it captures either one.
pub fn capture_fd1<R>(tag: &str, body: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::Write;
    use std::os::fd::AsRawFd;

    let _guard = fd1_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = tmp_path(tag);
    let file = std::fs::File::create(&path).expect("create capture file");

    let _ = std::io::stdout().flush();
    fflush_all();

    let (saved, ret) = unsafe {
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
        let ret = body();
        // Push any bytes the C library left sitting in glibc's stdout buffer
        // into the capture file before we swap fd 1 back.
        fflush_all();
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
        (saved, ret)
    };
    let _ = saved;

    drop(file);
    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (ret, bytes)
}

/// Like [`capture_fd1`] but points fd 1 at an arbitrary already-open fd
/// (e.g. a pipe, `/dev/null`, `/dev/full`) instead of a temp file.
pub fn with_fd1<R>(target: c_int, body: impl FnOnce() -> R) -> R {
    use std::io::Write;
    let _guard = fd1_lock().lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::io::stdout().flush();
    fflush_all();
    unsafe {
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(target, 1) >= 0, "dup2 onto fd 1 failed");
        let r = body();
        fflush_all();
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
        r
    }
}

// --------------------------------------------------------- fd plumbing ------

/// `open(2)` wrapper returning a raw fd.
pub fn open_fd(path: &str, flags: c_int, mode: c_int) -> c_int {
    let c = CString::new(path).unwrap();
    unsafe { open(c.as_ptr(), flags, mode) }
}

/// Create a pipe, returning `(read_fd, write_fd)`.
pub fn make_pipe() -> (c_int, c_int) {
    let mut fds = [0 as c_int; 2];
    let rc = unsafe { pipe(fds.as_mut_ptr()) };
    assert_eq!(rc, 0, "pipe() failed");
    (fds[0], fds[1])
}

/// Wrap a raw fd as a `Stdio` for a child process (takes ownership).
pub fn stdio_from(fd: c_int) -> std::process::Stdio {
    use std::os::fd::FromRawFd;
    unsafe { std::process::Stdio::from_raw_fd(fd) }
}
