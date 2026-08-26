//! Shared differential-test harness.
//!
//! Both the C and the Rust implementation are loaded as *shared objects* through
//! `libloading` and driven only through their exported `extern "C"` symbols --
//! exactly as an external consumer would. The Rust functions are never called
//! directly, so the `#[no_mangle]` export wrappers are under test too.
//!
//! `driver` reports its result by writing to the C standard library's `stdout`
//! (`printf` / `putchar`), which is process-global state shared by both `.so`
//! files. To capture a call's bytes without disturbing the test process, the
//! harness `fork()`s and redirects file descriptor 1 **in the child only**:
//!
//!   * the parent's fd 1 is never touched, so libtest's own progress output can
//!     never leak into a capture (this makes captures safe under any
//!     `--test-threads` setting), and
//!   * the child's exit status is reported alongside its bytes, so the C and the
//!     Rust implementation are also compared on *how they terminate* (normal
//!     exit vs. fatal signal), not just on what they print.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    /// `fflush(NULL)` flushes *every* open libc output stream.
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FD: c_int = 1;
/// Child exit code used when the redirection itself fails.
const CHILD_SETUP_FAILED: c_int = 111;

/// Signature of the exported symbol `void driver(int x)`.
pub type DriverFn = unsafe extern "C" fn(c_int);
/// The same symbol viewed as taking a 32-bit unsigned argument.
pub type DriverFnU32 = unsafe extern "C" fn(u32);
/// The same symbol viewed as taking a 64-bit argument, so a test can control the
/// whole argument register and check that only the low 32 bits are read.
pub type DriverFnU64 = unsafe extern "C" fn(u64);

/// The two loaded implementations.
pub struct Impls {
    pub c: Library,
    pub rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

/// Locate the Rust cdylib next to the test binary (`target/<profile>/libdriver.so`).
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    for dir in exe.ancestors() {
        let candidate = dir.join("libdriver.so");
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "could not find the Rust cdylib libdriver.so near {}: run `cargo build` first",
        exe.display()
    );
}

/// Load both `.so` files once per test binary.
///
/// Both libraries export the *same* symbol name `driver`. `libloading` uses
/// `RTLD_LOCAL`, so `dlsym` on a handle resolves within that library's own
/// scope; `sanity_two_distinct_implementations_are_loaded` verifies this.
pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.is_file(),
            "missing C shared library {} -- build it with:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
        Impls {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

pub fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { impls().c.get(b"driver\0") }.expect("C .so does not export `driver`")
}

pub fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { impls().rust.get(b"driver\0") }.expect("Rust .so does not export `driver`")
}

pub fn c_driver_u32() -> Symbol<'static, DriverFnU32> {
    unsafe { impls().c.get(b"driver\0") }.expect("C .so does not export `driver`")
}

pub fn rust_driver_u32() -> Symbol<'static, DriverFnU32> {
    unsafe { impls().rust.get(b"driver\0") }.expect("Rust .so does not export `driver`")
}

pub fn c_driver_u64() -> Symbol<'static, DriverFnU64> {
    unsafe { impls().c.get(b"driver\0") }.expect("C .so does not export `driver`")
}

pub fn rust_driver_u64() -> Symbol<'static, DriverFnU64> {
    unsafe { impls().rust.get(b"driver\0") }.expect("Rust .so does not export `driver`")
}

/// Outcome of one captured run: the bytes written to libc `stdout` plus how the
/// child terminated.
#[derive(Clone, PartialEq, Eq)]
pub struct Run {
    pub bytes: Vec<u8>,
    /// Raw `waitpid` status.
    pub status: c_int,
}

impl Run {
    pub fn exit_code(&self) -> Option<c_int> {
        if self.status & 0x7f == 0 {
            Some((self.status >> 8) & 0xff)
        } else {
            None
        }
    }
    pub fn signal(&self) -> Option<c_int> {
        let sig = self.status & 0x7f;
        if sig == 0 || sig == 0x7f {
            None
        } else {
            Some(sig)
        }
    }
    pub fn terminated_normally(&self) -> bool {
        self.exit_code() == Some(0)
    }
    pub fn describe(&self) -> String {
        let how = match (self.exit_code(), self.signal()) {
            (Some(c), _) => format!("exit {c}"),
            (_, Some(s)) => format!("killed by signal {s}"),
            _ => format!("raw status {:#x}", self.status),
        };
        format!("{how}, output \"{}\"", show(&self.bytes))
    }
}

/// Guard serialising captures. The parent takes glibc's `stdout` lock only in
/// the `fflush` below, so holding this across the `fork` guarantees no thread is
/// inside `printf` when the child is created.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn unique_temp_path() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::var_os("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let _ = fs::create_dir_all(&dir);
    dir.join(format!("driver-capture-{}-{n}.bin", std::process::id()))
}

/// Fork, point the child's fd 1 at `target_fd`, run `f` there, and return the
/// child's raw `waitpid` status. The child performs no allocation and no Rust
/// runtime work beyond calling `f`, then `_exit`s.
fn fork_capture<F: FnOnce()>(target_fd: c_int, f: F) -> c_int {
    unsafe {
        // Nothing of ours may still sit in a libc buffer when we duplicate the
        // address space.
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // ---- child ----
            if dup2(target_fd, STDOUT_FD) < 0 {
                _exit(CHILD_SETUP_FAILED);
            }
            f();
            if fflush(std::ptr::null_mut()) != 0 {
                _exit(CHILD_SETUP_FAILED);
            }
            _exit(0);
        }
        // ---- parent ----
        let mut status: c_int = -1;
        let waited = waitpid(pid, &mut status, 0);
        assert_eq!(waited, pid, "waitpid() failed");
        status
    }
}

/// Run `f` with libc `stdout` on a **regular file** (fully buffered) in a child
/// process and return everything it wrote plus its exit status.
pub fn run_file<F: FnOnce()>(f: F) -> Run {
    let _guard = capture_lock();
    let path = unique_temp_path();
    let file = fs::File::create(&path).expect("create capture file");
    let status = fork_capture(file.as_raw_fd(), f);
    drop(file);
    let bytes = fs::read(&path).expect("read capture file");
    let _ = fs::remove_file(&path);
    assert_ne!(
        status >> 8 & 0xff,
        CHILD_SETUP_FAILED,
        "capture child failed to redirect its stdout"
    );
    Run { bytes, status }
}

/// Run `f` with libc `stdout` on a **pipe** in a child process. Keep the
/// expected output well under the 64 KiB pipe capacity.
pub fn run_pipe<F: FnOnce()>(f: F) -> Run {
    let _guard = capture_lock();
    let mut fds: [c_int; 2] = [-1, -1];
    let mut bytes = Vec::new();
    let status;
    unsafe {
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        let (read_end, write_end) = (fds[0], fds[1]);
        status = fork_capture(write_end, f);
        // The parent must drop its copy of the write end to observe EOF.
        close(write_end);
        let mut buf = [0u8; 4096];
        loop {
            let n = read(read_end, buf.as_mut_ptr().cast(), buf.len());
            if n <= 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..n as usize]);
        }
        close(read_end);
    }
    Run { bytes, status }
}

/// Bytes written by `f`, asserting the capture child exited cleanly.
pub fn capture_file<F: FnOnce()>(f: F) -> Vec<u8> {
    let run = run_file(f);
    assert!(
        run.terminated_normally(),
        "capture child did not exit cleanly: {}",
        run.describe()
    );
    run.bytes
}

/// Bytes written by `f` through a pipe, asserting the child exited cleanly.
pub fn capture_pipe<F: FnOnce()>(f: F) -> Vec<u8> {
    let run = run_pipe(f);
    assert!(
        run.terminated_normally(),
        "capture child did not exit cleanly: {}",
        run.describe()
    );
    run.bytes
}

/// Capture one `driver(x)` call from the C library.
pub fn c_out(x: c_int) -> Vec<u8> {
    let f = c_driver();
    capture_file(|| unsafe { f(x) })
}

/// Capture one `driver(x)` call from the Rust library.
pub fn rust_out(x: c_int) -> Vec<u8> {
    let f = rust_driver();
    capture_file(|| unsafe { f(x) })
}

/// Full outcome (bytes + termination) of one C `driver(x)` call.
pub fn c_run(x: c_int) -> Run {
    let f = c_driver();
    run_file(|| unsafe { f(x) })
}

/// Full outcome (bytes + termination) of one Rust `driver(x)` call.
pub fn rust_run(x: c_int) -> Run {
    let f = rust_driver();
    run_file(|| unsafe { f(x) })
}

/// The byte stream the C is specified to produce: the object representation of
/// `x` in host order, two lowercase hex digits per byte, then a newline.
/// Endianness-agnostic because `to_ne_bytes` mirrors the C `memcpy`.
pub fn expected(x: c_int) -> Vec<u8> {
    let mut s = String::new();
    for b in x.to_ne_bytes() {
        s.push_str(&format!("{b:02x}"));
    }
    s.push('\n');
    s.into_bytes()
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Assert C and Rust agree for a single input -- same bytes *and* the same
/// termination -- and that the output satisfies `print_hex`'s invariants (C20).
pub fn assert_same(x: c_int, row: &str) {
    let c = c_run(x);
    let r = rust_run(x);
    assert_eq!(
        c.bytes,
        r.bytes,
        "[{row}] driver({x} = {x:#010x}) diverged:\n  C    = {}\n  Rust = {}",
        c.describe(),
        r.describe()
    );
    assert_eq!(
        c.status,
        r.status,
        "[{row}] driver({x} = {x:#010x}) terminated differently:\n  C    = {}\n  Rust = {}",
        c.describe(),
        r.describe()
    );
    assert!(
        c.terminated_normally(),
        "[{row}] the C reference did not exit cleanly for {x:#010x}: {}",
        c.describe()
    );
    assert_eq!(
        c.bytes,
        expected(x),
        "[{row}] driver({x} = {x:#010x}): C output \"{}\" is not the host-order hex dump",
        show(&c.bytes)
    );
    assert_shape(&c.bytes, x, row);
}

/// Row C20: 8 lowercase hex digits plus a trailing newline, always.
pub fn assert_shape(out: &[u8], x: c_int, row: &str) {
    assert_eq!(
        out.len(),
        9,
        "[{row}] driver({x:#010x}) emitted {} bytes, expected 9: \"{}\"",
        out.len(),
        show(out)
    );
    assert_eq!(
        out[8], b'\n',
        "[{row}] driver({x:#010x}) missing trailing newline"
    );
    for (i, &b) in out[..8].iter().enumerate() {
        assert!(
            b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
            "[{row}] driver({x:#010x}) byte {i} = {b:#04x} is not a lowercase hex digit"
        );
    }
}

/// Assert C and Rust agree over a whole *sequence* of calls captured together.
/// Stronger than per-call comparison: it also pins down cross-call state and the
/// flush ordering of the shared libc stream. On divergence the failure is
/// bisected down to the first offending input.
pub fn assert_same_batch(xs: &[c_int], row: &str) {
    let cf = c_driver();
    let rf = rust_driver();
    let c = capture_file(|| {
        for &x in xs {
            unsafe { cf(x) }
        }
    });
    let r = capture_file(|| {
        for &x in xs {
            unsafe { rf(x) }
        }
    });
    if c != r {
        // Each call contributes exactly 9 bytes, so the first differing byte
        // identifies the offending input directly -- no rescan of the batch.
        let first_diff = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c.len().min(r.len()));
        let idx = first_diff / 9;
        if let Some(&x) = xs.get(idx) {
            let (c1, r1) = (c_out(x), rust_out(x));
            assert_eq!(
                c1,
                r1,
                "[{row}] divergence in a batch of {} calls, at call #{idx}, driver({x} = {x:#010x}):\n  \
                 C    = \"{}\"\n  Rust = \"{}\"",
                xs.len(),
                show(&c1),
                show(&r1)
            );
        }
        panic!(
            "[{row}] batch of {} calls diverged at byte {first_diff} (call #{idx}) although that \
             call matches in isolation -- cross-call state?\n  C   ({} bytes) = \"{}\"\n  Rust({} bytes) = \"{}\"",
            xs.len(),
            c.len(),
            show(&c[..c.len().min(first_diff + 27)]),
            r.len(),
            show(&r[..r.len().min(first_diff + 27)]),
        );
    }
    let mut want = Vec::new();
    for &x in xs {
        want.extend_from_slice(&expected(x));
    }
    assert_eq!(
        c, want,
        "[{row}] C batch output is not the concatenation of per-call hex dumps"
    );
}

/// Deterministic xorshift64* PRNG so every randomized row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9e3779b97f4a7c15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> c_int {
        self.next_u32() as c_int
    }
    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

/// Number of randomized inputs per property-style row (override with
/// `DRIVER_TEST_CASES`).
pub fn cases() -> usize {
    std::env::var("DRIVER_TEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(256)
}
