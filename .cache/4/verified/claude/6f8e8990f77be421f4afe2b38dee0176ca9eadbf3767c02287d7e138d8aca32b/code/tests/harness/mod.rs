//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and every call
//! goes through `dlsym`, so the Rust `#[no_mangle] extern "C"` export wrappers
//! are exercised exactly the way an external C caller would exercise them.
//! Rust functions are never called directly.
#![allow(dead_code)]

use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// The exact 13 bytes `helloworld` must emit per call (`puts` appends the `\n`).
pub const HELLO_LINE: &[u8] = b"Hello World!\n";

/// Fixed seed so every randomized (property-style) test is reproducible.
pub const SEED: u64 = 0x5EED_1234_9ABC_DEF0;

unsafe extern "C" {
    /// libc's `FILE *stdout` — the very stream the C library's `printf` writes
    /// to. Both `.so`s share this one object because both import `puts` from
    /// the same libc.
    static stdout: *mut libc::FILE;
}

pub fn c_stdout() -> *mut libc::FILE {
    unsafe { stdout }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — no external crate, fully reproducible.
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

    /// Uniform-ish in `[lo, hi]` (inclusive).
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        assert!(lo <= hi);
        lo + self.next_u64() % (hi - lo + 1)
    }

    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn f64(&mut self) -> f64 {
        // Includes normal values plus the occasional extreme / non-finite.
        match self.next_u64() % 8 {
            0 => 0.0,
            1 => -0.0,
            2 => f64::NAN,
            3 => f64::INFINITY,
            4 => f64::NEG_INFINITY,
            5 => f64::MAX,
            6 => f64::MIN_POSITIVE,
            _ => (self.i32() as f64) / 7.0,
        }
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }

    /// Printable-ASCII blob (never contains `\n`, so line-based checks stay
    /// meaningful), length `len`.
    pub fn ascii_blob(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| b'!' + (self.next_u64() % 93) as u8)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Shared-object locations
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C `libhello.so` built by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HELLO_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libhello.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \\\n    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// The Rust `libhello.so` (cdylib) for the profile this test binary was built
/// with: `target/<profile>/deps/<test> -> target/<profile>/libhello.so`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HELLO_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let candidate = exe
        .parent()
        .and_then(|deps| deps.parent())
        .map(|profile| profile.join("libhello.so"));
    if let Some(c) = candidate {
        if c.exists() {
            return c;
        }
    }
    for p in ["target/debug/libhello.so", "target/release/libhello.so"] {
        let p = manifest_dir().join(p);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found. Build it first:\n  cargo build --offline\n\
         (cargo test does not build cdylib artifacts by itself)"
    );
}

/// The release-profile Rust cdylib (used by the dev-vs-release parity test).
pub fn rust_so_release_path() -> PathBuf {
    if let Ok(p) = std::env::var("HELLO_RUST_SO_RELEASE") {
        return PathBuf::from(p);
    }
    manifest_dir().join("target/release/libhello.so")
}

// ---------------------------------------------------------------------------
// Loading + calling through dlopen/dlsym
// ---------------------------------------------------------------------------

/// `dlopen` with libloading's default flags (`RTLD_LAZY | RTLD_LOCAL`).
pub fn open_lib(path: &Path) -> libloading::Library {
    unsafe { libloading::Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({path:?}) failed: {e}"))
}

/// `dlopen` with explicit flags.
pub fn open_lib_flags(path: &Path, flags: c_int) -> libloading::os::unix::Library {
    unsafe { libloading::os::unix::Library::open(Some(path), flags) }
        .unwrap_or_else(|e| panic!("dlopen({path:?}, {flags:#x}) failed: {e}"))
}

/// `dlsym("helloworld")`, returned as a raw code address so it can be
/// transmuted to each ABI shape under test and moved across threads.
pub fn hello_addr(lib: &libloading::Library) -> usize {
    let sym: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { lib.get(b"helloworld\0") }.expect("dlsym(helloworld) failed");
    (*sym) as usize
}

pub fn hello_addr_os(lib: &libloading::os::unix::Library) -> usize {
    let sym: libloading::os::unix::Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { lib.get(b"helloworld\0") }.expect("dlsym(helloworld) failed");
    (*sym) as usize
}

struct Loaded {
    c: usize,
    rust: usize,
}

static LOADED: OnceLock<Loaded> = OnceLock::new();

/// Addresses of `helloworld` in (C `.so`, Rust `.so`). Both libraries are
/// leaked on purpose so the handles stay valid for the whole test binary.
pub fn addrs() -> (usize, usize) {
    let l = LOADED.get_or_init(|| {
        let c_lib = open_lib(&c_so_path());
        let r_lib = open_lib(&rust_so_path());
        let c = hello_addr(&c_lib);
        let rust = hello_addr(&r_lib);
        assert_ne!(c, 0);
        assert_ne!(rust, 0);
        assert_ne!(
            c, rust,
            "the two handles resolved to the same address: both .so's are the same file?"
        );
        std::mem::forget(c_lib);
        std::mem::forget(r_lib);
        Loaded { c, rust }
    });
    (l.c, l.rust)
}

// --- the ABI shapes a caller can use against `int helloworld();` -----------

pub type Hello0 = unsafe extern "C" fn() -> c_int;
pub type Hello0Long = unsafe extern "C" fn() -> i64;
pub type Hello1I = unsafe extern "C" fn(c_int) -> c_int;
pub type Hello2I = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type Hello3I = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type Hello4I = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type Hello5I = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int) -> c_int;
pub type Hello6I = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;
pub type HelloFloats =
    unsafe extern "C" fn(c_double, c_double, c_double, c_double, c_int, c_int) -> c_int;
pub type HelloVariadic = unsafe extern "C" fn(c_int, ...) -> c_int;
pub type HelloPtrs = unsafe extern "C" fn(*const c_void, *const c_void, *const c_char) -> c_int;

pub unsafe fn call0(addr: usize) -> c_int {
    let f: Hello0 = unsafe { std::mem::transmute(addr) };
    unsafe { f() }
}

pub unsafe fn call0_long(addr: usize) -> i64 {
    let f: Hello0Long = unsafe { std::mem::transmute(addr) };
    unsafe { f() }
}

// ---------------------------------------------------------------------------
// stdout configuration + capture
// ---------------------------------------------------------------------------

/// Buffering configuration applied to libc's `stdout` before a call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufCfg {
    /// glibc's own default block buffer (`setvbuf(NULL, _IOFBF, 0)`).
    Default,
    /// Fully buffered with a caller-supplied buffer of `n` bytes.
    Full(usize),
    /// Line buffered with a caller-supplied buffer of `n` bytes.
    Line(usize),
    /// Unbuffered: one `write(2)` per `puts`.
    NoBuf,
}

fn leak_buf(n: usize) -> *mut c_char {
    assert!(n > 0);
    let b = vec![0u8; n].into_boxed_slice();
    Box::leak(b).as_mut_ptr() as *mut c_char
}

pub fn apply_buf(cfg: BufCfg) {
    unsafe {
        libc::fflush(c_stdout());
        let rc = match cfg {
            BufCfg::Default => libc::setvbuf(c_stdout(), std::ptr::null_mut(), libc::_IOFBF, 0),
            BufCfg::Full(n) => libc::setvbuf(c_stdout(), leak_buf(n), libc::_IOFBF, n),
            BufCfg::Line(n) => libc::setvbuf(c_stdout(), leak_buf(n), libc::_IOLBF, n),
            BufCfg::NoBuf => libc::setvbuf(c_stdout(), std::ptr::null_mut(), libc::_IONBF, 0),
        };
        assert_eq!(rc, 0, "setvbuf({cfg:?}) failed");
    }
}

pub fn clear_stdout_error() {
    unsafe { libc::clearerr(c_stdout()) };
}

pub fn stdout_has_error() -> bool {
    unsafe { libc::ferror(c_stdout()) != 0 }
}

/// Everything that touches process-wide fd 1 / libc `stdout` must be
/// serialized, because cargo runs tests in parallel threads.
static SERIAL: Mutex<()> = Mutex::new(());

pub fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// Redirects fd 1 to `fd` for the duration of `body`, with `buf` buffering
/// applied. Restores fd 1 (and default buffering) afterwards, even on panic.
pub struct Redirect {
    saved: c_int,
}

impl Redirect {
    pub fn to_fd(fd: c_int) -> Self {
        unsafe {
            libc::fflush(c_stdout());
            let saved = libc::dup(1);
            assert!(saved >= 0, "dup(1) failed: {}", errno());
            assert!(libc::dup2(fd, 1) >= 0, "dup2({fd}, 1) failed: {}", errno());
            Redirect { saved }
        }
    }

    /// Closes fd 1 entirely (so every `write(2)` fails with `EBADF`).
    pub fn close_stdout() -> Self {
        unsafe {
            libc::fflush(c_stdout());
            let saved = libc::dup(1);
            assert!(saved >= 0, "dup(1) failed: {}", errno());
            assert_eq!(libc::close(1), 0, "close(1) failed: {}", errno());
            Redirect { saved }
        }
    }
}

impl Drop for Redirect {
    fn drop(&mut self) {
        unsafe {
            libc::fflush(c_stdout());
            assert!(libc::dup2(self.saved, 1) >= 0, "restoring fd 1 failed");
            libc::close(self.saved);
        }
    }
}

pub fn errno() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn tmp_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "hello-diff-{}-{}-{}-{}.bin",
        std::process::id(),
        tag,
        n,
        HELLO_LINE.len()
    ))
}

pub fn open_fd(path: &Path, flags: c_int, mode: libc::mode_t) -> c_int {
    let mut bytes = path.as_os_str().as_encoded_bytes().to_vec();
    bytes.push(0);
    let fd = unsafe { libc::open(bytes.as_ptr() as *const c_char, flags, mode as c_int) };
    assert!(fd >= 0, "open({path:?}, {flags:#x}) failed: {}", errno());
    fd
}

/// Runs `body` with libc `stdout` pointed at a fresh temp file, returns the
/// bytes that ended up in that file.
pub fn capture_file<T>(buf: BufCfg, body: impl FnOnce() -> T) -> (Vec<u8>, T) {
    let path = tmp_path("file");
    let fd = open_fd(
        &path,
        libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
        0o600,
    );
    let out = {
        let _r = Redirect::to_fd(fd);
        apply_buf(buf);
        let t = body();
        // `_r`'s Drop flushes and restores fd 1 here.
        t
    };
    apply_buf(BufCfg::Default);
    unsafe { libc::close(fd) };
    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (bytes, out)
}

/// Same, but `stdout` is a pipe (non-seekable). Output must stay under the
/// 64 KiB pipe capacity, which every caller here respects.
pub fn capture_pipe<T>(buf: BufCfg, body: impl FnOnce() -> T) -> (Vec<u8>, T) {
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    let (rfd, wfd) = (fds[0], fds[1]);
    let out = {
        let _r = Redirect::to_fd(wfd);
        apply_buf(buf);
        body()
    };
    apply_buf(BufCfg::Default);
    unsafe { libc::close(wfd) };
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(rfd, chunk.as_mut_ptr() as *mut c_void, chunk.len()) };
        assert!(n >= 0, "read(pipe) failed: {}", errno());
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n as usize]);
    }
    unsafe { libc::close(rfd) };
    (bytes, out)
}

/// `stdout` is an `O_APPEND` fd on a file that already contains `prefix`.
/// Returns the whole file contents afterwards.
pub fn capture_append<T>(buf: BufCfg, prefix: &[u8], body: impl FnOnce() -> T) -> (Vec<u8>, T) {
    let path = tmp_path("append");
    std::fs::write(&path, prefix).expect("seed append file");
    let fd = open_fd(&path, libc::O_WRONLY | libc::O_APPEND, 0o600);
    let out = {
        let _r = Redirect::to_fd(fd);
        apply_buf(buf);
        body()
    };
    apply_buf(BufCfg::Default);
    unsafe { libc::close(fd) };
    let bytes = std::fs::read(&path).expect("read append file");
    let _ = std::fs::remove_file(&path);
    (bytes, out)
}

/// Runs `body` with `stdout` pointed at a character device (`/dev/null`,
/// `/dev/full`, ...). Nothing is captured; the return value of `body` is.
pub fn with_stdout_device<T>(dev: &str, buf: BufCfg, body: impl FnOnce() -> T) -> T {
    let fd = open_fd(Path::new(dev), libc::O_WRONLY, 0);
    let out = {
        let _r = Redirect::to_fd(fd);
        apply_buf(buf);
        body()
    };
    apply_buf(BufCfg::Default);
    unsafe { libc::close(fd) };
    out
}

/// Writes `bytes` to libc `stdout` through the *caller's* own `fwrite`, i.e.
/// the same `FILE *` buffer the library under test uses.
pub fn caller_write(bytes: &[u8]) {
    let n = unsafe {
        libc::fwrite(
            bytes.as_ptr() as *const c_void,
            1,
            bytes.len(),
            c_stdout(),
        )
    };
    assert_eq!(n, bytes.len(), "fwrite to stdout short/failed");
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

/// `n` back-to-back copies of the expected line.
pub fn expected(n: usize) -> Vec<u8> {
    HELLO_LINE.repeat(n)
}

pub fn assert_same_bytes(what: &str, c: &[u8], rust: &[u8]) {
    if c != rust {
        panic!(
            "{what}: byte streams differ\n  C   ({} bytes): {:?}\n  Rust({} bytes): {:?}",
            c.len(),
            String::from_utf8_lossy(&c[..c.len().min(256)]),
            rust.len(),
            String::from_utf8_lossy(&rust[..rust.len().min(256)]),
        );
    }
}

pub fn assert_same_rets<T: PartialEq + std::fmt::Debug>(what: &str, c: &T, rust: &T) {
    assert_eq!(c, rust, "{what}: return values differ (C vs Rust)");
}

// ---------------------------------------------------------------------------
// Row runner
// ---------------------------------------------------------------------------
//
// Every test binary here exposes exactly ONE `#[test]` function, which drives
// all of its rows through this runner. That is deliberate: libtest prints its
// own progress lines ("test foo ... ok") to fd 1, and with more than one test
// in a binary those lines race into the fd-1 captures these tests rely on. One
// `#[test]` per binary means nothing else can write to fd 1 while a capture
// window is open, no matter what `--test-threads` is set to.

pub struct Rows {
    phase: &'static str,
    ran: usize,
    failed: Vec<String>,
}

impl Rows {
    pub fn new(phase: &'static str) -> Self {
        eprintln!("=== {phase} ===");
        Rows {
            phase,
            ran: 0,
            failed: Vec::new(),
        }
    }

    /// Runs one table row, recording (but not aborting on) a failure so that
    /// every remaining row still gets exercised and reported.
    pub fn row(&mut self, id: &str, f: impl FnOnce()) {
        self.ran += 1;
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        // Always leave libc stdout in a sane state for the next row, even if
        // the row panicked half-way through a redirect.
        clear_stdout_error();
        apply_buf(BufCfg::Default);
        match r {
            Ok(()) => eprintln!("  [x] {id}"),
            Err(_) => {
                eprintln!("  [ ] {id}  <-- FAILED");
                self.failed.push(id.to_string());
            }
        }
    }

    pub fn finish(self) {
        eprintln!(
            "=== {} : {} rows run, {} failed ===",
            self.phase,
            self.ran,
            self.failed.len()
        );
        assert!(
            self.failed.is_empty(),
            "{}: {} of {} rows FAILED: {:?}",
            self.phase,
            self.failed.len(),
            self.ran,
            self.failed
        );
    }
}
