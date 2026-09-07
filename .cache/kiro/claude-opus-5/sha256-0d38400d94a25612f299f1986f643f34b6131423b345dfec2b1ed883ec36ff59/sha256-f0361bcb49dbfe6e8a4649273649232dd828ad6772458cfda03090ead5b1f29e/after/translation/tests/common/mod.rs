// Shared differential-test harness.
//
// Both the C reference `.so` and the Rust `.so` are loaded with `libloading`
// and every call goes through the exported `slice` symbol -- the Rust
// implementation is NEVER called directly, so the `#[no_mangle]`/`extern "C"`
// wrapper is part of what is under test.
//
// `slice` communicates through TWO channels: its `int` return value and
// whatever it writes to stdout via libc `printf`. Both are captured and
// compared byte-for-byte.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

pub type SliceFn = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

// ---------------------------------------------------------------------------
// libc bits used only by the harness (never by the code under test)
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libString_Slice.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // `cargo test` alone does NOT emit the cdylib, so pick whichever profile's
    // artifact is present and newest, then verify it is not stale (below).
    let base = manifest_dir().join("target");
    let candidates =
        [base.join("debug/libString_Slice.so"), base.join("release/libString_Slice.so")];
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for c in candidates.iter() {
        if let Ok(m) = std::fs::metadata(c).and_then(|m| m.modified()) {
            if best.as_ref().map_or(true, |(_, t)| m > *t) {
                best = Some((c.clone(), m));
            }
        }
    }
    match best {
        Some((p, _)) => p,
        None => panic!(
            "no Rust shared library found under {base:?}. Run `cargo build` (or \
             ./run_tests.sh) before `cargo test` -- `cargo test` on its own does \
             not produce the cdylib."
        ),
    }
}

/// Guard against testing a stale `.so`: the artifact must be at least as new as
/// every source file it is built from. Without this, `cargo test` happily
/// verifies a shared object built before the last edit.
fn assert_not_stale(so: &std::path::Path) {
    let so_time = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {so:?}: {e}"));
    let src_dir = manifest_dir().join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    if let Ok(rd) = std::fs::read_dir(&src_dir) {
        for e in rd.flatten() {
            if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                if newest.as_ref().map_or(true, |(_, n)| t > *n) {
                    newest = Some((e.path(), t));
                }
            }
        }
    }
    if let Some((p, t)) = newest {
        assert!(
            so_time >= t,
            "STALE ARTIFACT: {so:?} is older than {p:?}. `cargo test` does not \
             rebuild the cdylib -- run `cargo build` first (./run_tests.sh does)."
        );
    }
}

pub struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    pub c_slice: SliceFn,
    pub rust_slice: SliceFn,
}

// Safety: the loaded libraries are leaked for the lifetime of the process and
// `slice` is a pure `printf`-ing function with no shared mutable state.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn load() -> Libs {
    require_single_threaded();
    let cp = c_so_path();
    let rp = rust_so_path();
    assert!(cp.exists(), "C shared library not found at {cp:?} -- build it first");
    assert!(rp.exists(), "Rust shared library not found at {rp:?} -- `cargo build` first");
    assert_not_stale(&rp);
    eprintln!("[diff-harness] C   .so: {cp:?}");
    eprintln!("[diff-harness] Rust .so: {rp:?}");

    unsafe {
        let c_lib = Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {cp:?}: {e}"));
        let rust_lib = Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {rp:?}: {e}"));
        let c_sym: Symbol<SliceFn> =
            c_lib.get(b"slice\0").expect("C .so does not export `slice`");
        let r_sym: Symbol<SliceFn> =
            rust_lib.get(b"slice\0").expect("Rust .so does not export `slice`");
        let c_slice = *c_sym;
        let rust_slice = *r_sym;
        Libs { _c_lib: c_lib, _rust_lib: rust_lib, c_slice, rust_slice }
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(load)
}

// fd 1 redirection is process-global, so all captures must serialise.
fn capture_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Redirect fd 1 to a temp file, run `f`, flush every libc stream, restore
/// fd 1, and return the raw bytes written.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = capture_lock();
    capture_stdout_unlocked(f)
}

fn capture_stdout_unlocked<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let mut tmp = tempfile();
    unsafe {
        // Drain BOTH buffers that sit in front of fd 1 before redirecting it:
        // Rust's `std::io::Stdout` (which holds cargo's unterminated
        // "test foo ... " progress line) and every libc `FILE*`.
        drain_rust_stdout();
        fflush(core::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 failed");

        let out = f();

        fflush(core::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        tmp.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = Vec::new();
        tmp.read_to_end(&mut buf).unwrap();
        (out, buf)
    }
}

fn drain_rust_stdout() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

/// The differential tests redirect the process-wide fd 1, so they cannot run
/// concurrently with cargo's own progress output. Every test file calls this
/// once to make an accidental multi-threaded run fail loudly instead of
/// producing bogus divergences.
pub fn require_single_threaded() {
    let n = std::env::var("RUST_TEST_THREADS").ok();
    assert_eq!(
        n.as_deref(),
        Some("1"),
        "these differential tests capture the process-wide stdout fd and MUST run \
         with a single test thread. Run them via ./run_tests.sh, or with \
         `RUST_TEST_THREADS=1 cargo test -- --test-threads=1`."
    );
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir()
        .join(format!("slice_diff_{}_{}_{:?}.out", std::process::id(), n, std::thread::current().id()));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create temp capture file");
    // Unlink immediately; the fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Differential call
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub ret: c_int,
    pub stdout: Vec<u8>,
}

/// One differential invocation.
///
/// `bytes` is the NUL-terminated payload handed to `slice` as `char *mystr`.
/// A **fresh copy** of the buffer and of the index cells is made for each side
/// so neither implementation can observe the other's mutations.
///
/// `start` / `stop` of `None` mean a NULL pointer argument.
pub fn diff_call(
    bytes: &[u8],
    start: Option<c_int>,
    stop: Option<c_int>,
) -> (Outcome, Outcome) {
    let f = libs();
    let c = invoke(f.c_slice, bytes, start, stop);
    let r = invoke(f.rust_slice, bytes, start, stop);
    (c, r)
}

fn invoke(fun: SliceFn, bytes: &[u8], start: Option<c_int>, stop: Option<c_int>) -> Outcome {
    let mut buf: Vec<u8> = Vec::with_capacity(bytes.len() + 1);
    buf.extend_from_slice(bytes);
    buf.push(0);
    let mut s = start.unwrap_or(0);
    let mut e = stop.unwrap_or(0);
    let sp: *mut c_int = if start.is_some() { &mut s } else { core::ptr::null_mut() };
    let ep: *mut c_int = if stop.is_some() { &mut e } else { core::ptr::null_mut() };
    let p = buf.as_mut_ptr() as *mut c_char;
    let (ret, stdout) = capture_stdout(|| unsafe { fun(p, sp, ep) });
    Outcome { ret, stdout }
}

/// Assert the two implementations agree, with a readable failure message.
#[track_caller]
pub fn assert_same(label: &str, bytes: &[u8], start: Option<c_int>, stop: Option<c_int>) {
    let (c, r) = diff_call(bytes, start, stop);
    if c != r {
        panic!(
            "DIVERGENCE [{label}]\n  input   : {:?} (len={})\n  start   : {:?}\n  stop    : {:?}\n  C   ret={} stdout={:?}\n  Rust ret={} stdout={:?}",
            String::from_utf8_lossy(bytes),
            bytes.len(),
            start,
            stop,
            c.ret,
            c.stdout,
            r.ret,
            r.stdout,
        );
    }
}

/// Same as [`assert_same`] but also pins down what the C actually did, so the
/// test fails if the shared expectation drifts (used by the error-path tests).
#[track_caller]
pub fn assert_same_and(
    label: &str,
    bytes: &[u8],
    start: Option<c_int>,
    stop: Option<c_int>,
    expect_ret: c_int,
    expect_stdout: &[u8],
) {
    let (c, r) = diff_call(bytes, start, stop);
    assert_eq!(
        c, r,
        "DIVERGENCE [{label}] input={:?} start={:?} stop={:?}",
        String::from_utf8_lossy(bytes),
        start,
        stop
    );
    assert_eq!(c.ret, expect_ret, "[{label}] C return value changed");
    assert_eq!(
        c.stdout,
        expect_stdout,
        "[{label}] C stdout changed: got {:?} want {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(expect_stdout)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Random bytes in `[lo, hi]`, never 0 (a NUL would shorten the string).
    pub fn bytes(&mut self, len: usize, lo: u8, hi: u8) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let span = (hi - lo) as u64 + 1;
                lo + (self.next_u64() % span) as u8
            })
            .collect()
    }
    pub fn ascii(&mut self, len: usize) -> Vec<u8> {
        self.bytes(len, 0x20, 0x7e)
    }
}

// ---------------------------------------------------------------------------
// Out-of-process crash comparison (for UB inputs, e.g. mystr == NULL)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub enum ChildExit {
    Signalled(c_int),
    Exited(c_int),
}

/// Run `f` in a forked child and report how the child terminated.
pub fn run_in_child(f: impl FnOnce()) -> ChildExit {
    unsafe {
        drain_rust_stdout();
        fflush(core::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        // WIFSIGNALED / WTERMSIG / WEXITSTATUS
        let term_sig = status & 0x7f;
        if term_sig != 0 && term_sig != 0x7f {
            ChildExit::Signalled(term_sig)
        } else {
            ChildExit::Exited((status >> 8) & 0xff)
        }
    }
}
