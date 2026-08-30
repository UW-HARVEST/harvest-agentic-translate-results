//! Shared differential-test harness.
//!
//! Both implementations are loaded as *shared objects* through `libloading` and
//! called only through their exported `driver` symbol — the Rust crate is never
//! linked or called directly, so the `#[no_mangle] extern "C"` wrapper is part of
//! what gets tested.
//!
//! `driver` returns `void` and its only observable effect is what it writes to
//! `stdout`, so every comparison is a comparison of captured stdout bytes (and,
//! for the error paths, of the child process's termination status).

#![allow(dead_code)]

use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

pub type DriverFn = unsafe extern "C" fn(*const c_char, *const c_char);

/// Serialises everything that touches the process-wide stdout fd or forks.
static STDIO_LOCK: Mutex<()> = Mutex::new(());

pub struct Libs {
    pub c: DriverFn,
    pub rust: DriverFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
    // Keep the libraries loaded for the lifetime of the process.
    _c_lib: Library,
    _rust_lib: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared object not found at {p:?}.\nBuild it first:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    // An explicit override lets the driver script point the same test suite at
    // each build profile / feature combination in turn.
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO points at a missing file: {p:?}");
        return p;
    }
    // Otherwise prefer the profile the test itself was built with, then the other.
    let dir = manifest_dir().join("target");
    let preferred = if cfg!(debug_assertions) { "debug" } else { "release" };
    let other = if cfg!(debug_assertions) { "release" } else { "debug" };
    for profile in [preferred, other] {
        let p = dir.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust shared object not found under {dir:?}/{{debug,release}}/libdriver.so.\n\
         Build it first:  cd translation && cargo build --release"
    );
}

/// Loads both `.so`s once and resolves the `driver` symbol out of each.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        let c_lib = Library::new(&c_path).expect("failed to dlopen the C libdriver.so");
        let rust_lib = Library::new(&rust_path).expect("failed to dlopen the Rust libdriver.so");

        let c_sym: Symbol<DriverFn> = c_lib
            .get(b"driver\0")
            .expect("symbol `driver` missing from the C .so");
        let rust_sym: Symbol<DriverFn> = rust_lib
            .get(b"driver\0")
            .expect("symbol `driver` missing from the Rust .so — check #[no_mangle]");

        let c = *c_sym;
        let rust = *rust_sym;

        Libs {
            c,
            rust,
            c_path,
            rust_path,
            _c_lib: c_lib,
            _rust_lib: rust_lib,
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture (in-process; for calls that are expected not to crash)
// ---------------------------------------------------------------------------

unsafe fn make_temp_fd() -> libc::c_int {
    let mut tmpl: Vec<u8> = b"/tmp/driver-difftest-XXXXXX\0".to_vec();
    // Honour TMPDIR when it is set, so the harness works in sandboxes.
    if let Ok(dir) = std::env::var("TMPDIR") {
        let dir = dir.trim_end_matches('/');
        if !dir.is_empty() {
            tmpl = format!("{dir}/driver-difftest-XXXXXX\0").into_bytes();
        }
    }
    let fd = libc::mkstemp(tmpl.as_mut_ptr() as *mut c_char);
    assert!(fd >= 0, "mkstemp failed");
    // Unlink immediately; we only ever use the fd.
    libc::unlink(tmpl.as_ptr() as *const c_char);
    fd
}

unsafe fn read_all_from_start(fd: libc::c_int) -> Vec<u8> {
    libc::lseek(fd, 0, libc::SEEK_SET);
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    out
}

/// Runs `f` with fd 1 redirected to a scratch file and returns everything it wrote.
///
/// `f` must not crash — use [`fork_capture`] for that.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDIO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Drain anything already buffered so it is not attributed to `f`.
        libc::fflush(std::ptr::null_mut());

        let tmp = make_temp_fd();
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(tmp, 1) >= 0, "dup2 onto fd 1 failed");

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        let out = read_all_from_start(tmp);
        libc::close(tmp);
        out
    }
}

/// Calls the C `driver` and captures its stdout.
pub fn c_out(s1: *const c_char, s2: *const c_char) -> Vec<u8> {
    let f = libs().c;
    capture(|| unsafe { f(s1, s2) })
}

/// Calls the Rust `driver` (via the `.so` export) and captures its stdout.
pub fn rust_out(s1: *const c_char, s2: *const c_char) -> Vec<u8> {
    let f = libs().rust;
    capture(|| unsafe { f(s1, s2) })
}

// ---------------------------------------------------------------------------
// forked capture (for calls that may fault)
// ---------------------------------------------------------------------------

/// How a forked child that called `driver` terminated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Returned normally; the child exited with this code.
    Exited(i32),
    /// Killed by this signal number (e.g. 11 = SIGSEGV, 6 = SIGABRT).
    Signaled(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkResult {
    pub outcome: Outcome,
    pub stdout: Vec<u8>,
}

impl ForkResult {
    pub fn segfaulted(&self) -> bool {
        matches!(self.outcome, Outcome::Signaled(libc::SIGSEGV))
    }
}

/// Forks, runs `driver(s1, s2)` in the child with stdout captured, and reports
/// both the child's termination status and its output.
///
/// This is what makes "the same rejection" checkable for an API that reports
/// failure only by dying: `SIGSEGV` vs `SIGBUS` vs normal exit are distinguished.
pub fn fork_call(f: DriverFn, s1: *const c_char, s2: *const c_char) -> ForkResult {
    let _guard = STDIO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let tmp = make_temp_fd();

        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ---- child ----
            libc::dup2(tmp, 1);
            f(s1, s2);
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }

        // ---- parent ----
        let mut status: libc::c_int = 0;
        while libc::waitpid(pid, &mut status, 0) < 0 {
            if *libc::__errno_location() != libc::EINTR {
                panic!("waitpid failed");
            }
        }

        let outcome = if libc::WIFEXITED(status) {
            Outcome::Exited(libc::WEXITSTATUS(status))
        } else if libc::WIFSIGNALED(status) {
            Outcome::Signaled(libc::WTERMSIG(status))
        } else {
            panic!("child neither exited nor was signalled: status={status}");
        };

        let stdout = read_all_from_start(tmp);
        libc::close(tmp);
        ForkResult { outcome, stdout }
    }
}

pub fn fork_c(s1: *const c_char, s2: *const c_char) -> ForkResult {
    fork_call(libs().c, s1, s2)
}

pub fn fork_rust(s1: *const c_char, s2: *const c_char) -> ForkResult {
    fork_call(libs().rust, s1, s2)
}

/// Asserts C and Rust behave identically for a possibly-faulting call.
pub fn assert_same_fork(s1: *const c_char, s2: *const c_char, what: &str) -> ForkResult {
    let c = fork_c(s1, s2);
    let r = fork_rust(s1, s2);
    assert_eq!(
        c.outcome, r.outcome,
        "termination status differs for {what}: C={:?} Rust={:?}",
        c.outcome, r.outcome
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {what}: C={:?} Rust={:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    c
}

// ---------------------------------------------------------------------------
// helpers for building inputs
// ---------------------------------------------------------------------------

/// A NUL-terminated buffer that owns its bytes.
pub struct CBuf(Vec<u8>);

impl CBuf {
    pub fn new(bytes: &[u8]) -> Self {
        assert!(
            !bytes.contains(&0),
            "interior NUL would truncate the C string"
        );
        let mut v = Vec::with_capacity(bytes.len() + 1);
        v.extend_from_slice(bytes);
        v.push(0);
        CBuf(v)
    }
    pub fn ptr(&self) -> *const c_char {
        self.0.as_ptr() as *const c_char
    }
}

/// Compares C and Rust for a pair of *valid* NUL-terminated strings.
pub fn assert_same(s1: &[u8], s2: &[u8]) {
    let a = CBuf::new(s1);
    let b = CBuf::new(s2);
    let c = c_out(a.ptr(), b.ptr());
    let r = rust_out(a.ptr(), b.ptr());
    assert_eq!(
        c,
        r,
        "divergence!\n  s1 = {:?}\n  s2 = {:?}\n  C    -> {:?}\n  Rust -> {:?}",
        Escaped(s1),
        Escaped(s2),
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    // Sanity: output must be a decimal number followed by exactly one newline.
    let txt = String::from_utf8_lossy(&c);
    assert!(
        txt.ends_with('\n') && txt[..txt.len() - 1].bytes().all(|b| b.is_ascii_digit()),
        "unexpected output shape {txt:?}"
    );
}

/// Same as [`assert_same`] but also checks the value against an independently
/// computed expectation, so a *mutually* wrong pair cannot pass silently.
pub fn assert_same_and_eq(s1: &[u8], s2: &[u8], expected: usize) {
    assert_same(s1, s2);
    let a = CBuf::new(s1);
    let b = CBuf::new(s2);
    let got = c_out(a.ptr(), b.ptr());
    assert_eq!(
        got,
        format!("{expected}\n").into_bytes(),
        "C printed {:?}, expected {expected}\n  s1={:?} s2={:?}",
        String::from_utf8_lossy(&got),
        Escaped(s1),
        Escaped(s2)
    );
}

/// Reference model of `strcspn`, used only to cross-check expectations.
pub fn strcspn_ref(s1: &[u8], s2: &[u8]) -> usize {
    for (i, c) in s1.iter().enumerate() {
        if s2.contains(c) {
            return i;
        }
    }
    s1.len()
}

pub struct Escaped<'a>(pub &'a [u8]);
impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "b\"")?;
        for &b in self.0 {
            if b.is_ascii_graphic() || b == b' ' {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{b:02x}")?;
            }
        }
        write!(f, "\" (len {})", self.0.len())
    }
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64) — fixed seeds keep every run reproducible
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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    /// Any byte in `0x01..=0xFF` (never NUL — that would terminate the string).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
    pub fn byte_from(&mut self, alphabet: &[u8]) -> u8 {
        alphabet[self.below(alphabet.len())]
    }
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }
    pub fn bytes_from(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len).map(|_| self.byte_from(alphabet)).collect()
    }
    /// `bytes` with a length drawn from `lo..=hi` (avoids a nested `&mut` borrow).
    pub fn bytes_range(&mut self, lo: usize, hi_inclusive: usize) -> Vec<u8> {
        let n = self.range(lo, hi_inclusive);
        self.bytes(n)
    }
    /// `bytes` with a length drawn from `0..n`.
    pub fn bytes_below(&mut self, n: usize) -> Vec<u8> {
        let n = self.below(n);
        self.bytes(n)
    }
    /// `bytes_from` with a length drawn from `lo..=hi`.
    pub fn bytes_from_range(&mut self, lo: usize, hi_inclusive: usize, alphabet: &[u8]) -> Vec<u8> {
        let n = self.range(lo, hi_inclusive);
        self.bytes_from(n, alphabet)
    }
}

pub const ASCII_PRINTABLE: &[u8] =
    b" !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

pub fn control_bytes() -> Vec<u8> {
    (0x01u8..=0x1F).collect()
}
pub fn high_bytes() -> Vec<u8> {
    (0x80u8..=0xFF).collect()
}
pub fn all_nonzero_bytes() -> Vec<u8> {
    (0x01u8..=0xFF).collect()
}

// ---------------------------------------------------------------------------
// page-boundary buffers (over-read detectors)
// ---------------------------------------------------------------------------

/// A string placed so that its terminating NUL is the **last readable byte**
/// before an unmapped guard page. Any read past the NUL faults.
pub struct GuardedString {
    base: *mut u8,
    total: usize,
    start: *const c_char,
}

impl GuardedString {
    /// `bytes` must not contain NUL. The layout is
    /// `[ ... padding ... | bytes | NUL ][ PROT_NONE page ]`.
    pub fn new(bytes: &[u8]) -> Self {
        assert!(!bytes.contains(&0));
        unsafe {
            let page = libc::sysconf(libc::_SC_PAGESIZE) as usize;
            let needed = bytes.len() + 1;
            let data_pages = needed.div_ceil(page);
            let total = (data_pages + 1) * page;
            let base = libc::mmap(
                std::ptr::null_mut(),
                total,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            assert_ne!(base, libc::MAP_FAILED, "mmap failed");
            let base = base as *mut u8;
            // Make the trailing page unreadable.
            let guard = base.add(data_pages * page);
            assert_eq!(
                libc::mprotect(guard as *mut libc::c_void, page, libc::PROT_NONE),
                0,
                "mprotect failed"
            );
            // Place the string so its NUL lands on the final readable byte.
            let start = guard.sub(needed);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), start, bytes.len());
            *start.add(bytes.len()) = 0;
            GuardedString {
                base,
                total,
                start: start as *const c_char,
            }
        }
    }
    pub fn ptr(&self) -> *const c_char {
        self.start
    }
}

impl Drop for GuardedString {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, self.total);
        }
    }
}

/// A buffer with **no** NUL terminator, ending flush against a guard page, so
/// scanning it necessarily faults.
pub struct UnterminatedString {
    base: *mut u8,
    total: usize,
    start: *const c_char,
}

impl UnterminatedString {
    pub fn new(bytes: &[u8]) -> Self {
        assert!(!bytes.is_empty() && !bytes.contains(&0));
        unsafe {
            let page = libc::sysconf(libc::_SC_PAGESIZE) as usize;
            let data_pages = bytes.len().div_ceil(page);
            let total = (data_pages + 1) * page;
            let base = libc::mmap(
                std::ptr::null_mut(),
                total,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            assert_ne!(base, libc::MAP_FAILED, "mmap failed");
            let base = base as *mut u8;
            let guard = base.add(data_pages * page);
            assert_eq!(
                libc::mprotect(guard as *mut libc::c_void, page, libc::PROT_NONE),
                0
            );
            let start = guard.sub(bytes.len());
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), start, bytes.len());
            UnterminatedString {
                base,
                total,
                start: start as *const c_char,
            }
        }
    }
    pub fn ptr(&self) -> *const c_char {
        self.start
    }
}

impl Drop for UnterminatedString {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, self.total);
        }
    }
}
