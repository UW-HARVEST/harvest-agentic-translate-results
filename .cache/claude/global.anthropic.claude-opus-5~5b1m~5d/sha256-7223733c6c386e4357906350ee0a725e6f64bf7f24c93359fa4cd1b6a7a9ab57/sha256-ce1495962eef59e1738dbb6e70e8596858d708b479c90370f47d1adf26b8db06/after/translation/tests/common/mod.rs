//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! called only through their exported `slice` symbol, exactly as an external
//! C consumer would. The Rust functions are never called directly, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! `slice` reports its result through *two* channels: the `int` return value
//! and bytes written to `stdout` by the C library's `printf`. To compare the
//! second channel we temporarily redirect file descriptor 1 to a pipe, flush
//! the C stdio buffers, and read back the raw bytes. That happens at the fd
//! level (not through Rust's `print!` capture) because the output originates
//! inside libc, in the loaded `.so`.

#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

/// Signature of the symbol under test:
/// `int slice(char *mystr, int *start_ptr, int *stop_ptr)`
pub type SliceFn = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

/// Which of the two implementations to invoke.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

struct Loaded {
    _c_lib: Library,
    _rust_lib: Library,
    c_slice: SliceFn,
    rust_slice: SliceFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// The raw function pointers are plain code addresses in libraries that are
// never unloaded, so they are safe to share between threads.
unsafe impl Send for Loaded {}
unsafe impl Sync for Loaded {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared object, built via CMake.
pub fn c_so_path() -> PathBuf {
    let root = manifest_dir();
    let candidates = [
        root.join("../c_src/build/libString_Slice.so"),
        root.join("../c_src/build/libString_Slice.dylib"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Build it first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         looked in: {candidates:?}"
    );
}

/// Path to the Rust `cdylib`. Prefers the profile the tests themselves were
/// built with, then falls back to the other one.
pub fn rust_so_path() -> PathBuf {
    let root = manifest_dir();
    let profiles: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    let mut tried = Vec::new();
    for p in profiles {
        for name in ["libString_Slice.so", "libString_Slice.dylib"] {
            let cand = root.join("target").join(p).join(name);
            if cand.exists() {
                return cand;
            }
            tried.push(cand);
        }
    }
    panic!("Rust cdylib not found; run `cargo build`. looked in: {tried:?}");
}

/// Fails if `so` is older than any of the sources it is built from.
///
/// This guard exists because `cargo test --test <name>` does **not** rebuild
/// the `cdylib`: an integration test cannot link a `crate-type = ["cdylib"]`
/// target, so cargo has no dependency edge from the test to the `.so`. Without
/// this check a stale library is loaded silently and the whole suite reports
/// green against code that is no longer on disk — verified the hard way, by
/// mutating `src/lib.rs` and watching every row still pass.
fn assert_fresh(so: &Path, sources: &[PathBuf], hint: &str) {
    let Ok(so_mtime) = std::fs::metadata(so).and_then(|m| m.modified()) else {
        return;
    };
    let mut stale = Vec::new();
    for src in sources {
        if let Ok(t) = std::fs::metadata(src).and_then(|m| m.modified()) {
            if t > so_mtime {
                stale.push(src.display().to_string());
            }
        }
    }
    assert!(
        stale.is_empty(),
        "STALE LIBRARY: {} is older than {stale:?}.\n\
         The tests would have silently verified out-of-date code.\n\
         Rebuild it first:  {hint}",
        so.display(),
    );
}

/// Every file whose contents affect the built Rust `.so`.
fn rust_sources() -> Vec<PathBuf> {
    let root = manifest_dir();
    let mut v = vec![root.join("build.rs"), root.join("Cargo.toml")];
    if let Ok(entries) = std::fs::read_dir(root.join("src")) {
        for e in entries.flatten() {
            if e.path().extension().is_some_and(|x| x == "rs") {
                v.push(e.path());
            }
        }
    }
    v
}

/// Every file whose contents affect the built C `.so`.
fn c_sources() -> Vec<PathBuf> {
    let root = manifest_dir().join("../c_src");
    let mut v = vec![root.join("CMakeLists.txt")];
    for sub in ["src", "include"] {
        if let Ok(entries) = std::fs::read_dir(root.join(sub)) {
            for e in entries.flatten() {
                if e.path()
                    .extension()
                    .is_some_and(|x| x == "c" || x == "h")
                {
                    v.push(e.path());
                }
            }
        }
    }
    v
}

fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        assert_fresh(&rust_path, &rust_sources(), "cargo build [--release]");
        assert_fresh(
            &c_path,
            &c_sources(),
            "cd c_src/build && cmake --build .",
        );

        // SAFETY: both paths point at shared objects whose only exported
        // symbol is the `slice` function we look up below. `Library` is kept
        // alive for the whole process (leaked into a `OnceLock`), so the
        // resolved function pointers stay valid.
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));

            let c_sym: Symbol<SliceFn> = c_lib
                .get(b"slice\0")
                .unwrap_or_else(|e| panic!("C .so does not export `slice`: {e}"));
            let rust_sym: Symbol<SliceFn> = rust_lib
                .get(b"slice\0")
                .unwrap_or_else(|e| panic!("Rust .so does not export `slice`: {e}"));

            let c_slice = *c_sym;
            let rust_slice = *rust_sym;

            Loaded {
                c_slice,
                rust_slice,
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_path,
                rust_path,
            }
        }
    })
}

pub fn slice_fn(which: Impl) -> SliceFn {
    let l = loaded();
    match which {
        Impl::C => l.c_slice,
        Impl::Rust => l.rust_slice,
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so only one capture may be active at a time.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Runs `f` with fd 1 redirected into a pipe and returns everything it wrote.
///
/// The C stdio buffers are flushed before the redirect (so unrelated output
/// does not leak into the capture) and after `f` returns (so buffered output
/// from the `.so` is included).
fn with_captured_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = capture_lock();

    // SAFETY: plain POSIX fd juggling; every fd created here is closed on
    // every path, and the original fd 1 is restored before returning.
    unsafe {
        fflush(std::ptr::null_mut()); // fflush(NULL): flush all output streams
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let mut fds: [c_int; 2] = [-1, -1];
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        let (read_fd, write_fd) = (fds[0], fds[1]);

        let saved = dup(STDOUT_FILENO);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(write_fd, STDOUT_FILENO) >= 0, "dup2 onto stdout failed");
        close(write_fd);

        let result = f();

        fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());

        // Restore stdout, which also closes the pipe's write end so the read
        // below sees EOF instead of blocking.
        assert!(dup2(saved, STDOUT_FILENO) >= 0, "restoring stdout failed");
        close(saved);

        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = read(read_fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n > 0 {
                out.extend_from_slice(&buf[..n as usize]);
            } else {
                break;
            }
        }
        close(read_fd);

        (result, out)
    }
}

/// Writes bytes straight to the real stdout, bypassing capture (for diagnostics).
pub fn raw_stdout(bytes: &[u8]) {
    unsafe {
        let _ = write(STDOUT_FILENO, bytes.as_ptr() as *const c_void, bytes.len());
    }
}

// ---------------------------------------------------------------------------
// Sub-test runner
// ---------------------------------------------------------------------------
//
// fd 1 is a *process-global* resource, and libtest writes its own progress
// output ("test foo ... ok\n") to it from whichever thread finishes a test.
// If two `#[test]`s ran concurrently, one test's capture would swallow the
// harness chatter emitted for the other, producing bogus divergences.
//
// Rather than depend on `--test-threads=1` being passed, each test binary
// exposes exactly ONE `#[test]` that drives its rows through this `Suite`.
// That makes serialization structural. Progress is reported on stderr, which
// is never redirected.

/// Runs named sub-tests sequentially and aggregates failures.
pub struct Suite {
    name: &'static str,
    passed: Vec<String>,
    failed: Vec<(String, String)>,
}

impl Suite {
    pub fn new(name: &'static str) -> Self {
        eprintln!("\n=== {name} ===");
        Suite {
            name,
            passed: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// Runs one sub-test, catching a panic so later rows still execute.
    pub fn row(&mut self, id: &str, f: impl FnOnce()) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match result {
            Ok(()) => {
                eprintln!("  [PASS] {id}");
                self.passed.push(id.to_string());
            }
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "<non-string panic payload>".to_string()
                };
                eprintln!("  [FAIL] {id}\n{msg}");
                self.failed.push((id.to_string(), msg));
            }
        }
    }

    /// Panics with a summary if any row failed.
    pub fn finish(self) {
        // Restore the default hook so the summary panic below is printed.
        let _ = std::panic::take_hook();
        eprintln!(
            "--- {}: {} passed, {} failed ---",
            self.name,
            self.passed.len(),
            self.failed.len()
        );
        if !self.failed.is_empty() {
            let ids: Vec<&str> = self.failed.iter().map(|(i, _)| i.as_str()).collect();
            panic!(
                "{}: {} of {} rows FAILED: {:?}\n\nfirst failure:\n{}",
                self.name,
                self.failed.len(),
                self.failed.len() + self.passed.len(),
                ids,
                self.failed[0].1
            );
        }
    }
}

/// Quietens libtest's default panic printing so `Suite::row` output stays
/// readable; failures are still reported through `Suite::finish`.
pub fn silence_panic_hook() {
    std::panic::set_hook(Box::new(|_| {}));
}

// ---------------------------------------------------------------------------
// One differential call
// ---------------------------------------------------------------------------

/// Everything one `slice` call observably produces.
#[derive(Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The `int` returned by `slice`.
    pub ret: c_int,
    /// Raw bytes the call wrote to fd 1.
    pub stdout: Vec<u8>,
    /// `*start_ptr` after the call (`None` if the pointer was null).
    pub start_after: Option<c_int>,
    /// `*stop_ptr` after the call (`None` if the pointer was null).
    pub stop_after: Option<c_int>,
    /// The `mystr` buffer (including its NUL and any trailing bytes) after the call.
    pub buf_after: Vec<u8>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("ret", &self.ret)
            .field("stdout", &Escaped(&self.stdout))
            .field("start_after", &self.start_after)
            .field("stop_after", &self.stop_after)
            .field("buf_after", &Escaped(&self.buf_after))
            .finish()
    }
}

struct Escaped<'a>(&'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for &b in self.0 {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                b'\0' => write!(f, "\\0")?,
                b'"' => write!(f, "\\\"")?,
                b'\\' => write!(f, "\\\\")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        write!(f, "\" ({} bytes)", self.0.len())
    }
}

/// Invokes one implementation.
///
/// `buf` must be the exact byte image handed to the C function, NUL terminator
/// included; it is copied first so the two implementations always start from
/// identical memory.
pub fn call(which: Impl, buf: &[u8], start: Option<c_int>, stop: Option<c_int>) -> Outcome {
    let f = slice_fn(which);

    let mut owned: Vec<u8> = buf.to_vec();
    let mut start_val = start.unwrap_or(0);
    let mut stop_val = stop.unwrap_or(0);

    let start_p: *mut c_int = if start.is_some() {
        &mut start_val
    } else {
        std::ptr::null_mut()
    };
    let stop_p: *mut c_int = if stop.is_some() {
        &mut stop_val
    } else {
        std::ptr::null_mut()
    };

    let str_p = owned.as_mut_ptr() as *mut c_char;

    // SAFETY: `owned` is NUL-terminated by the callers below, and the bound
    // pointers are either null or point at live `c_int`s. Those are exactly
    // the C function's preconditions.
    let (ret, stdout) = with_captured_stdout(|| unsafe { f(str_p, start_p, stop_p) });

    Outcome {
        ret,
        stdout,
        start_after: start.map(|_| start_val),
        stop_after: stop.map(|_| stop_val),
        buf_after: owned,
    }
}

/// Calls both implementations with identical inputs and asserts every
/// observable is byte-for-byte identical. Returns the (shared) outcome.
#[track_caller]
pub fn assert_same(ctx: &str, buf: &[u8], start: Option<c_int>, stop: Option<c_int>) -> Outcome {
    assert!(
        buf.contains(&0),
        "{ctx}: test input must be NUL-terminated (harness bug)"
    );

    let c = call(Impl::C, buf, start, stop);
    let r = call(Impl::Rust, buf, start, stop);

    if c != r {
        panic!(
            "DIVERGENCE [{ctx}]\n  input buf   = {:?}\n  start       = {start:?}\n  stop        = {stop:?}\n\
             \n  C    ret = {:>3}  stdout = {:?}\n  Rust ret = {:>3}  stdout = {:?}\
             \n  C    start_after={:?} stop_after={:?} buf_after={:?}\
             \n  Rust start_after={:?} stop_after={:?} buf_after={:?}",
            Escaped(buf),
            c.ret,
            Escaped(&c.stdout),
            r.ret,
            Escaped(&r.stdout),
            c.start_after,
            c.stop_after,
            Escaped(&c.buf_after),
            r.start_after,
            r.stop_after,
            Escaped(&r.buf_after),
        );
    }

    // G7: the C code never writes through any of its arguments.
    assert_eq!(&c.buf_after[..], buf, "{ctx}: mystr buffer was mutated");
    if let Some(s) = start {
        assert_eq!(c.start_after, Some(s), "{ctx}: *start_ptr was mutated");
    }
    if let Some(s) = stop {
        assert_eq!(c.stop_after, Some(s), "{ctx}: *stop_ptr was mutated");
    }

    c
}

/// Invokes one implementation with `start_ptr` and `stop_ptr` **aliased** to the
/// same `int`.
///
/// A C caller can legitimately do this (`slice(s, &n, &n)`), and since the C
/// code only reads through the pointers it is well defined: `start == stop`, so
/// the ordering check always rejects it — unless the value is out of range, in
/// which case the start check fires first.
pub fn call_aliased(which: Impl, buf: &[u8], value: c_int) -> Outcome {
    let f = slice_fn(which);
    let mut owned: Vec<u8> = buf.to_vec();
    let mut shared = value;
    let p: *mut c_int = &mut shared;
    let str_p = owned.as_mut_ptr() as *mut c_char;

    // SAFETY: `owned` is NUL-terminated and `p` points at a live `c_int`.
    // Passing it twice is fine because `slice` only ever reads through it.
    let (ret, stdout) = with_captured_stdout(|| unsafe { f(str_p, p, p) });

    Outcome {
        ret,
        stdout,
        start_after: Some(shared),
        stop_after: Some(shared),
        buf_after: owned,
    }
}

/// Differential version of [`call_aliased`].
#[track_caller]
pub fn assert_same_aliased(ctx: &str, payload: &[u8], value: c_int) -> Outcome {
    let mut buf = payload.to_vec();
    buf.push(0);
    let c = call_aliased(Impl::C, &buf, value);
    let r = call_aliased(Impl::Rust, &buf, value);
    assert_eq!(
        c, r,
        "DIVERGENCE [{ctx}] aliased bounds, payload={:?}, value={value}\n  C={c:?}\n  Rust={r:?}",
        Escaped(payload)
    );
    assert_eq!(c.start_after, Some(value), "{ctx}: aliased int was mutated");
    c
}

/// Convenience wrapper taking the payload without its NUL terminator.
#[track_caller]
pub fn assert_same_str(
    ctx: &str,
    payload: &[u8],
    start: Option<c_int>,
    stop: Option<c_int>,
) -> Outcome {
    let mut buf = payload.to_vec();
    buf.push(0);
    assert_same(ctx, &buf, start, stop)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed ⇒ reproducible property-style tests)
// ---------------------------------------------------------------------------

/// xorshift64* — small, dependency-free, and identical on every run.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Avoid the zero fixed point.
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform-ish value in `[0, n)`; returns 0 when `n == 0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// Inclusive range `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }

    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Random payload of `len` bytes drawn from `alphabet` (never contains NUL).
    pub fn payload(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        assert!(!alphabet.is_empty());
        assert!(!alphabet.contains(&0), "alphabet must not contain NUL");
        (0..len)
            .map(|_| alphabet[self.below(alphabet.len())])
            .collect()
    }

    /// Random payload of `len` non-NUL bytes over the full `0x01..=0xFF` range.
    pub fn payload_any(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| 1 + (self.next_u32() % 255) as u8).collect()
    }
}

pub const ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 .,!?";
pub const PERCENTS: &[u8] = b"%sn dx0-.*\\\"'{}";
pub const CONTROLS: &[u8] = &[
    0x01, 0x02, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x1b, 0x1f, b'A', b' ',
];

/// Bytes 0x80..=0xFF (invalid UTF-8 when standing alone).
pub fn high_bytes() -> Vec<u8> {
    (0x80u8..=0xffu8).collect()
}
