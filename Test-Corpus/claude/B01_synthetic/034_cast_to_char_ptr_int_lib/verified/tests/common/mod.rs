//! Shared differential-test harness.
//!
//! Both the C shared object (`c_src/build/libdriver.so`) and the Rust shared
//! object (`target/<profile>/libdriver.so`) are loaded with `libloading` and
//! driven *only* through their exported `driver` symbol — exactly as an external
//! consumer would. No Rust function is ever called directly, so the
//! `#[no_mangle]`/`extern "C"` export wrapper is part of what is under test.
//!
//! `driver` communicates through `stdout` (it calls `printf`), so the harness
//! captures the process's file descriptor 1 around each invocation batch and
//! compares the raw bytes.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bindings (declared directly so we hit the very same glibc `stdout`
// FILE object that both libraries under test write to)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn clearerr(stream: *mut c_void);
    fn ferror(stream: *mut c_void) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn __errno_location() -> *mut c_int;
    static mut stdout: *mut c_void;
}

pub const IOFBF: c_int = 0;
pub const IOLBF: c_int = 1;
pub const IONBF: c_int = 2;

pub const O_WRONLY: c_int = 1;
pub const O_RDONLY: c_int = 0;

pub const EBADF: c_int = 9;
pub const ENOSPC: c_int = 28;
pub const EPIPE: c_int = 32;

pub const LC_NUMERIC: c_int = 1;
pub const LC_ALL: c_int = 6;

fn stdout_file() -> *mut c_void {
    unsafe { stdout }
}

pub fn errno() -> c_int {
    unsafe { *__errno_location() }
}

pub fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v }
}

pub fn stdout_ferror() -> c_int {
    unsafe { ferror(stdout_file()) }
}

pub fn stdout_clearerr() {
    unsafe { clearerr(stdout_file()) }
}

// ---------------------------------------------------------------------------
// The two libraries under test
// ---------------------------------------------------------------------------

/// `void driver(int x)` — the single exported entry point.
pub type DriverFn = unsafe extern "C" fn(c_int);

/// Same symbol, viewed through a 64-bit parameter, to check that the callee
/// ignores dirty upper register bits exactly like the C does.
pub type DriverFn64 = unsafe extern "C" fn(i64);

pub struct Libs {
    pub c_lib: Library,
    pub rust_lib: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Libs {
    pub fn c_driver(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.c_lib.get(b"driver\0") }.expect("C .so must export `driver`")
    }
    pub fn rust_driver(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.rust_lib.get(b"driver\0") }.expect("Rust .so must export `driver`")
    }
    pub fn c_driver64(&self) -> Symbol<'_, DriverFn64> {
        unsafe { self.c_lib.get(b"driver\0") }.expect("C .so must export `driver`")
    }
    pub fn rust_driver64(&self) -> Symbol<'_, DriverFn64> {
        unsafe { self.rust_lib.get(b"driver\0") }.expect("Rust .so must export `driver`")
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // current_exe() == target/<profile>/deps/<testbin>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    let candidate = profile.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    deps.join("libdriver.so")
}

/// `cargo test` does **not** rebuild a `cdylib` (the integration tests do not
/// link against it), so a stale `.so` would silently make every differential
/// test vacuous. Refuse to run in that case.
fn assert_fresh(so: &std::path::Path, sources: &[PathBuf]) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    for src in sources {
        if let Ok(src_mtime) = std::fs::metadata(src).and_then(|m| m.modified()) {
            assert!(
                so_mtime >= src_mtime,
                "{} is OLDER than {} — the shared object is stale and the \
                 differential tests would be meaningless.\nRebuild first:\n  \
                 cargo build --no-default-features\n  (cd c_src/build && cmake --build .)\n\
                 or just run ./run_tests.sh",
                so.display(),
                src.display()
            );
        }
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not found at {}. Build it with `cargo build`.",
            rust_path.display()
        );
        let md = manifest_dir();
        assert_fresh(&rust_path, &[md.join("src/lib.rs"), md.join("Cargo.toml")]);
        assert_fresh(
            &c_path,
            &[
                md.join("c_src/src/driver.c"),
                md.join("c_src/include/driver.h"),
            ],
        );
        // RTLD_LOCAL (libloading's default) keeps the two identically-named
        // `driver` symbols from interfering with each other.
        let c_lib = unsafe { Library::new(&c_path) }.expect("dlopen C .so");
        let rust_lib = unsafe { Library::new(&rust_path) }.expect("dlopen Rust .so");
        Libs {
            c_lib,
            rust_lib,
            c_path,
            rust_path,
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn stdout_lock() -> &'static Mutex<()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

/// Serialises everything that touches file descriptor 1 / the `stdout` FILE.
pub struct CaptureEnv {
    _guard: MutexGuard<'static, ()>,
}

/// Runs `f` with exclusive ownership of the process's `stdout`.
pub fn with_stdout<R>(f: impl FnOnce(&mut CaptureEnv) -> R) -> R {
    let guard = stdout_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut env = CaptureEnv { _guard: guard };
    // Start from a known-good state.
    env.flush_all();
    stdout_clearerr();
    let out = f(&mut env);
    env.flush_all();
    stdout_clearerr();
    out
}

fn unique_tmp_path(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "driver-difftest-{}-{}-{}.bin",
        tag,
        std::process::id(),
        n
    ))
}

impl CaptureEnv {
    pub fn flush_all(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
        }
    }

    /// Sets the buffering mode of the glibc `stdout` stream. Returns the
    /// `setvbuf` result.
    pub fn set_mode(&mut self, mode: c_int, size: usize) -> c_int {
        self.flush_all();
        unsafe { setvbuf(stdout_file(), std::ptr::null_mut(), mode, size) }
    }

    /// Redirects fd 1 to a fresh regular file, runs `body`, flushes, restores
    /// fd 1 and returns everything that was written.
    pub fn capture_file(&mut self, body: impl FnOnce()) -> Vec<u8> {
        let path = unique_tmp_path("file");
        let file = File::create(&path).expect("create temp capture file");
        let bytes = {
            let tfd = file.as_raw_fd();
            self.run_redirected(tfd, body);
            drop(file);
            let mut buf = Vec::new();
            File::open(&path)
                .expect("reopen temp capture file")
                .read_to_end(&mut buf)
                .expect("read temp capture file");
            buf
        };
        let _ = std::fs::remove_file(&path);
        bytes
    }

    /// Same as [`capture_file`] but fd 1 becomes the write end of a pipe.
    ///
    /// The caller must keep the produced output below the pipe capacity
    /// (64 KiB on Linux); this is asserted.
    pub fn capture_pipe(&mut self, body: impl FnOnce()) -> Vec<u8> {
        let mut fds = [0 as c_int; 2];
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
        let (r, w) = (fds[0], fds[1]);
        self.run_redirected(w, body);
        unsafe { close(w) };
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = unsafe { read(r, buf.as_mut_ptr() as *mut c_void, buf.len()) };
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
            if out.len() > 60_000 {
                break;
            }
        }
        unsafe { close(r) };
        assert!(
            out.len() <= 60_000,
            "pipe capture overflowed; keep the batch small"
        );
        out
    }

    /// Points fd 1 at an arbitrary already-open descriptor for the duration of
    /// `body`. Nothing is read back — used by the error-path tests.
    pub fn run_redirected(&mut self, target_fd: c_int, body: impl FnOnce()) {
        self.flush_all();
        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "dup(1) failed");
        assert!(unsafe { dup2(target_fd, 1) } >= 0, "dup2 onto fd 1 failed");
        body();
        self.flush_all();
        assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
        unsafe { close(saved) };
    }

    /// Opens `path` with `flags`, points fd 1 at it, runs `body`, restores fd 1.
    /// Returns `(ferror(stdout), errno)` observed immediately after `body` and
    /// its flush.
    pub fn run_with_stdout_on(
        &mut self,
        path: &str,
        flags: c_int,
        body: impl FnOnce(),
    ) -> (c_int, c_int) {
        let cpath = CString::new(path).unwrap();
        let fd = unsafe { open(cpath.as_ptr(), flags) };
        assert!(fd >= 0, "open({}) failed", path);
        let mut observed = (0, 0);
        self.flush_all();
        stdout_clearerr();
        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "dup(1) failed");
        assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 onto fd 1 failed");
        set_errno(0);
        body();
        unsafe { fflush(stdout_file()) };
        observed.0 = stdout_ferror();
        observed.1 = errno();
        assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
        unsafe { close(saved) };
        unsafe { close(fd) };
        stdout_clearerr();
        observed
    }

    /// 1. Puts the `stdout` `FILE` into a *sticky error state* by running
    ///    `trigger` with fd 1 pointing at `/dev/full` (unbuffered, so the
    ///    failure happens during the call).
    /// 2. Without calling `clearerr`, points fd 1 at a perfectly good file and
    ///    runs `body`.
    ///
    /// Returns `(ferror(stdout) after body, bytes body managed to emit)`.
    pub fn sticky_error_then_capture(
        &mut self,
        trigger: impl FnOnce(),
        body: impl FnOnce(),
    ) -> (c_int, Vec<u8>) {
        let path = unique_tmp_path("sticky");
        let file = File::create(&path).expect("create temp capture file");
        let tfd = file.as_raw_fd();

        let full = CString::new("/dev/full").unwrap();
        let full_fd = unsafe { open(full.as_ptr(), O_WRONLY) };
        assert!(full_fd >= 0, "open(/dev/full) failed");

        self.flush_all();
        stdout_clearerr();
        let prev_mode = self.set_mode(IONBF, 0);
        assert_eq!(prev_mode, 0, "setvbuf(_IONBF) failed");

        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "dup(1) failed");

        // (1) provoke the write error
        assert!(unsafe { dup2(full_fd, 1) } >= 0, "dup2 onto fd 1 failed");
        trigger();
        unsafe { fflush(stdout_file()) };
        let err_after_trigger = stdout_ferror();

        // (2) same stream, healthy fd, error flag deliberately left set
        assert!(unsafe { dup2(tfd, 1) } >= 0, "dup2 onto fd 1 failed");
        body();
        unsafe { fflush(stdout_file()) };
        let err_after_body = stdout_ferror();

        assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
        unsafe { close(saved) };
        unsafe { close(full_fd) };
        stdout_clearerr();
        self.set_mode(IOFBF, 4096);

        drop(file);
        let mut buf = Vec::new();
        File::open(&path)
            .expect("reopen temp capture file")
            .read_to_end(&mut buf)
            .expect("read temp capture file");
        let _ = std::fs::remove_file(&path);

        assert_ne!(
            err_after_trigger, 0,
            "the /dev/full trigger did not put stdout into an error state"
        );
        (err_after_body, buf)
    }

    /// Like [`run_with_stdout_on`] but fd 1 becomes the write end of a pipe
    /// whose read end has already been closed (⇒ `EPIPE`).
    pub fn run_with_stdout_on_broken_pipe(&mut self, body: impl FnOnce()) -> (c_int, c_int) {
        let mut fds = [0 as c_int; 2];
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
        let (r, w) = (fds[0], fds[1]);
        unsafe { close(r) };
        self.flush_all();
        stdout_clearerr();
        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "dup(1) failed");
        assert!(unsafe { dup2(w, 1) } >= 0, "dup2 onto fd 1 failed");
        set_errno(0);
        body();
        unsafe { fflush(stdout_file()) };
        let observed = (stdout_ferror(), errno());
        assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
        unsafe { close(saved) };
        unsafe { close(w) };
        stdout_clearerr();
        observed
    }
}

/// Temporarily switches the C locale, runs `f`, and restores `"C"`.
pub fn with_locale<R>(category: c_int, name: &str, f: impl FnOnce(bool) -> R) -> R {
    let want = CString::new(name).unwrap();
    let applied = unsafe { !setlocale(category, want.as_ptr()).is_null() };
    let out = f(applied);
    let c = CString::new("C").unwrap();
    unsafe {
        setlocale(LC_ALL, c.as_ptr());
    }
    out
}

// ---------------------------------------------------------------------------
// Reference oracle + comparison
// ---------------------------------------------------------------------------

/// Independent model of the C: `print_hex((unsigned char*)&x, sizeof(x))` on a
/// little-endian target — 2 lowercase hex digits per object-representation byte
/// followed by `'\n'`.
pub const RECORD_LEN: usize = 2 * std::mem::size_of::<c_int>() + 1;

pub fn expected_record(x: i32) -> Vec<u8> {
    let mut s = Vec::with_capacity(RECORD_LEN);
    for b in (x as u32).to_le_bytes() {
        s.extend_from_slice(format!("{:02x}", b).as_bytes());
    }
    s.push(b'\n');
    s
}

pub fn expected_stream(inputs: &[i32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(inputs.len() * RECORD_LEN);
    for &x in inputs {
        out.extend_from_slice(&expected_record(x));
    }
    out
}

fn render(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Compares the two captured streams record-by-record and reports the first
/// divergence together with the input that produced it.
pub fn assert_streams_match(label: &str, inputs: &[i32], c_out: &[u8], rust_out: &[u8]) {
    if c_out == rust_out {
        // Also sanity-check the harness itself against the independent oracle.
        let want = expected_stream(inputs);
        assert_eq!(
            c_out,
            want.as_slice(),
            "[{label}] the C library disagrees with the reference oracle \
             (harness bug or unexpected target endianness)"
        );
        return;
    }

    let n = c_out.len().min(rust_out.len());
    let first = (0..n).find(|&i| c_out[i] != rust_out[i]).unwrap_or(n);
    let rec = first / RECORD_LEN;
    let input = inputs.get(rec).copied();
    let lo = rec * RECORD_LEN;
    let hi_c = (lo + RECORD_LEN).min(c_out.len());
    let hi_r = (lo + RECORD_LEN).min(rust_out.len());
    panic!(
        "[{label}] C and Rust output diverge.\n\
         first differing byte : {first} (record #{rec}, offset {} within record)\n\
         input for record     : {input:?} (0x{:08x})\n\
         C     record         : \"{}\"\n\
         Rust  record         : \"{}\"\n\
         expected (oracle)    : \"{}\"\n\
         total lengths        : C={} Rust={} (expected {})\n",
        first - lo,
        input.map(|v| v as u32).unwrap_or(0),
        render(&c_out[lo..hi_c]),
        render(&rust_out[lo..hi_r]),
        render(&input.map(expected_record).unwrap_or_default()),
        c_out.len(),
        rust_out.len(),
        inputs.len() * RECORD_LEN,
    );
}

/// Runs the whole batch through the C `.so`, then through the Rust `.so`, and
/// asserts the captured byte streams are identical.
pub fn diff_batch(label: &str, inputs: &[i32]) {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let (c_out, rust_out) = with_stdout(|env| {
        let c_out = env.capture_file(|| {
            for &x in inputs {
                unsafe { c(x) };
            }
        });
        let rust_out = env.capture_file(|| {
            for &x in inputs {
                unsafe { r(x) };
            }
        });
        (c_out, rust_out)
    });
    assert_streams_match(label, inputs, &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Random `i32` biased towards "interesting" byte patterns as well as
    /// uniform values.
    pub fn next_interesting_i32(&mut self) -> i32 {
        let r = self.next_u64();
        match r % 8 {
            0 => 0,
            1 => -1,
            2 => (r >> 8) as u8 as i32,                    // one low byte only
            3 => (((r >> 8) as u8 as u32) << 24) as i32,   // one high byte only
            4 => ((r >> 8) as u16 as u32) as i32,          // low half only
            5 => (((r >> 8) as u16 as u32) << 16) as i32,  // high half only
            _ => self.next_i32(),                          // uniform
        }
    }
    pub fn sample(&mut self, n: usize) -> Vec<i32> {
        (0..n).map(|_| self.next_interesting_i32()).collect()
    }
}
