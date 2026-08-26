// Differential-test harness.
//
// Loads BOTH shared libraries through `libloading` and drives them only through
// their exported C ABI symbols (`driver`, `printLine`).  The Rust crate is never
// called directly, so the `#[unsafe(no_mangle)] extern "C"` export wrappers are
// part of what is under test.
//
// Because `driver`/`printLine` communicate exclusively by writing to the
// process-wide `stdout`, the harness captures output by temporarily redirecting
// file descriptor 1 to a private temp file around each call.  Both `.so`s are
// linked against the *same* system glibc, hence the *same* `stdout` `FILE*`, so
// `fflush(NULL)` from the test process flushes whichever library just wrote.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

unsafe extern "C" {
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

pub type DriverFn = unsafe extern "C" fn(c_int);
pub type PrintLineFn = unsafe extern "C" fn(*const c_char);

// ---------------------------------------------------------------------------
// Force single-threaded test execution.
//
// `driver`/`printLine` write to the process-wide fd 1, so the harness has to
// redirect fd 1 to capture them.  If libtest ran tests concurrently, its own
// progress output ("test foo ... ok") — emitted from a different thread — would
// land inside somebody else's capture window and corrupt the comparison.
//
// An ELF `.init_array` entry lets us set `RUST_TEST_THREADS=1` *before* libtest
// parses its options in `main`, so the suite is correct no matter how it is
// invoked (`cargo test`, direct execution, ...).  As a bonus, concurrency 1
// makes libtest run each test on the main thread, which is also the safest
// context for the `fork()`-based crash comparisons.
// ---------------------------------------------------------------------------

extern "C" fn force_single_threaded() {
    unsafe {
        setenv(c"RUST_TEST_THREADS".as_ptr(), c"1".as_ptr(), 1);
    }
}

#[used]
#[cfg_attr(target_os = "linux", unsafe(link_section = ".init_array"))]
static FORCE_SINGLE_THREADED: extern "C" fn() = force_single_threaded;

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
    /// Uniform-ish value in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        lo + self.below((hi - lo + 1) as u64) as i64
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    /// Any byte except NUL.
    pub fn nonzero_byte(&mut self) -> u8 {
        let b = self.byte();
        if b == 0 { 1 } else { b }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Newest mtime among the files of `dir` (recursively) with one of `exts`.
fn newest_source_mtime(dir: &Path, exts: &[&str]) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "build" || n == "target") {
                continue;
            }
            if let Some(t) = newest_source_mtime(&p, exts) {
                newest = Some(newest.map_or(t, |n: std::time::SystemTime| n.max(t)));
            }
        } else if p
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| exts.contains(&s))
        {
            if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                newest = Some(newest.map_or(t, |n: std::time::SystemTime| n.max(t)));
            }
        }
    }
    newest
}

/// Refuse to run against a shared object that is older than its sources.
///
/// This matters a great deal here: the crate declares `crate-type = ["cdylib"]`
/// only, so `cargo test` does **not** rebuild the cdylib (integration tests do
/// not link it).  Without this guard a stale `libdriver.so` would be loaded and
/// every differential test would "pass" while testing an old binary.
fn assert_fresh(so: &Path, src_dir: &Path, exts: &[&str], rebuild_hint: &str) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {so:?}: {e}"));
    if let Some(src_mtime) = newest_source_mtime(src_dir, exts) {
        assert!(
            so_mtime >= src_mtime,
            "STALE SHARED OBJECT: {so:?} is older than the newest source in \
             {src_dir:?}.\nDifferential results would be meaningless. Rebuild with:\n  {rebuild_hint}"
        );
    }
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \\\n    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    assert_fresh(
        &p,
        &manifest_dir().join("c_src"),
        &["c", "h"],
        "cd c_src/build && cmake --build .",
    );
    p
}

fn rust_so_path() -> PathBuf {
    // The integration-test binary lives in `<target>/<profile>/deps/`, so the
    // cdylib is one directory up (and, for some cargo versions, alongside it).
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let mut candidates: Vec<PathBuf> = vec![
        deps.join("libdriver.so"),
        deps.parent().unwrap_or(deps).join("libdriver.so"),
    ];
    candidates.push(manifest_dir().join("target/debug/libdriver.so"));
    candidates.push(manifest_dir().join("target/release/libdriver.so"));
    for c in &candidates {
        if c.exists() {
            assert_fresh(
                c,
                &manifest_dir().join("src"),
                &["rs"],
                "cargo build --offline        # `cargo test` alone does NOT rebuild a cdylib-only lib target",
            );
            return c.clone();
        }
    }
    panic!("Rust cdylib libdriver.so not found; looked in {candidates:?}");
}

// ---------------------------------------------------------------------------
// One loaded implementation
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    pub driver: DriverFn,
    pub print_line: PrintLineFn,
    // Keep the library mapped for the whole process lifetime.
    _lib: &'static Library,
}

fn load(name: &'static str, path: &Path) -> Impl {
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"))
    }));
    let driver: DriverFn = unsafe {
        let s: Symbol<DriverFn> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("dlsym driver in {path:?}: {e}"));
        *s
    };
    let print_line: PrintLineFn = unsafe {
        let s: Symbol<PrintLineFn> = lib
            .get(b"printLine\0")
            .unwrap_or_else(|e| panic!("dlsym printLine in {path:?}: {e}"));
        *s
    };
    Impl { name, driver, print_line, _lib: lib }
}

pub struct Harness {
    pub c: Impl,
    pub rust: Impl,
}

/// `(C .so, Rust .so)` filesystem paths, used by tests that inspect the
/// binaries themselves (symbol parity, codegen parity).
pub fn so_paths() -> (PathBuf, PathBuf) {
    (c_so_path(), rust_so_path())
}

static HARNESS: OnceLock<Harness> = OnceLock::new();
static FD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn fd_lock() -> MutexGuard<'static, ()> {
    match FD_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn harness() -> &'static Harness {
    HARNESS.get_or_init(|| {
        let h = Harness {
            c: load("C", &c_so_path()),
            rust: load("Rust", &rust_so_path()),
        };
        // Warm up every lazily-bound PLT slot (memset, strncpy, puts/printf,
        // printLine) in BOTH libraries *before* any fork(), so a forked child
        // never has to run the dynamic-symbol resolver.
        let warm = capture_locked(&mut fd_lock(), || unsafe {
            (h.c.driver)(0);
            (h.c.driver)(100);
            (h.c.print_line)(c"warm".as_ptr());
            (h.c.print_line)(std::ptr::null());
            (h.rust.driver)(0);
            (h.rust.driver)(100);
            (h.rust.print_line)(c"warm".as_ptr());
            (h.rust.print_line)(std::ptr::null());
        });
        assert!(!warm.is_empty(), "warm-up produced no output");
        h
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static SEQ: AtomicU64 = AtomicU64::new(0);

fn capture_locked<F: FnOnce()>(_guard: &mut MutexGuard<'static, ()>, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "cdiff-{}-{}.out",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    let file = std::fs::File::create(&path).expect("create temp capture file");
    let out: Vec<u8>;
    unsafe {
        // Drain anything already pending on fd 1 so it does not land in our file.
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        // Flush the shared glibc `stdout` that the library just wrote to.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    drop(file);
    out = std::fs::read(&path).expect("read temp capture file");
    let _ = std::fs::remove_file(&path);
    out
}

/// Run `f` with fd 1 redirected to a temp file and return the captured bytes.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut g = fd_lock();
    capture_locked(&mut g, f)
}

// ---------------------------------------------------------------------------
// Operation sequences
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// `driver(data)`
    Driver(c_int),
    /// `printLine(buf.as_ptr())` — `buf` must already contain its NUL.
    PrintLine(Vec<u8>),
    /// `printLine(NULL)`
    PrintLineNull,
}

/// Build a NUL-terminated buffer from raw bytes (bytes may contain NUL, which
/// is exactly how the embedded-NUL shape is exercised).
pub fn cbuf(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

fn run_ops(im: &Impl, ops: &[Op]) -> Vec<u8> {
    capture(|| {
        for op in ops {
            unsafe {
                match op {
                    Op::Driver(d) => (im.driver)(*d),
                    Op::PrintLine(buf) => {
                        assert!(buf.contains(&0), "PrintLine buffer must be NUL-terminated");
                        (im.print_line)(buf.as_ptr() as *const c_char)
                    }
                    Op::PrintLineNull => (im.print_line)(std::ptr::null()),
                }
            }
        }
    })
}

fn describe(ops: &[Op]) -> String {
    let mut s = String::new();
    for (i, op) in ops.iter().enumerate() {
        if i > 0 {
            s.push_str("; ");
        }
        match op {
            Op::Driver(d) => s.push_str(&format!("driver({d})")),
            Op::PrintLineNull => s.push_str("printLine(NULL)"),
            Op::PrintLine(buf) => {
                let content = &buf[..buf.len() - 1];
                if content.len() <= 40 {
                    s.push_str(&format!("printLine({:?})", String::from_utf8_lossy(content)));
                } else {
                    s.push_str(&format!(
                        "printLine(len={}, first40={:?})",
                        content.len(),
                        String::from_utf8_lossy(&content[..40])
                    ));
                }
            }
        }
    }
    s
}

/// Run an operation sequence against both `.so`s and assert byte-identical
/// `stdout`.  Returns the (shared) captured bytes.
#[track_caller]
pub fn assert_same(row: &str, ops: &[Op]) -> Vec<u8> {
    let h = harness();
    let got_c = run_ops(&h.c, ops);
    let got_rust = run_ops(&h.rust, ops);
    if got_c != got_rust {
        panic!(
            "[{row}] output mismatch for: {}\n  C    ({} bytes): {:?}\n  Rust ({} bytes): {:?}",
            describe(ops),
            got_c.len(),
            String::from_utf8_lossy(&got_c),
            got_rust.len(),
            String::from_utf8_lossy(&got_rust),
        );
    }
    got_c
}

/// Same as [`assert_same`] but additionally pins the expected bytes derived by
/// hand from the C source, so a *shared* regression in both cannot pass.
#[track_caller]
pub fn assert_same_and_eq(row: &str, ops: &[Op], expected: &[u8]) {
    let got = assert_same(row, ops);
    assert_eq!(
        got,
        expected,
        "[{row}] both impls agreed but disagree with the C source's expected output for {}\n  got:      {:?}\n  expected: {:?}",
        describe(ops),
        String::from_utf8_lossy(&got),
        String::from_utf8_lossy(expected),
    );
}

// ---------------------------------------------------------------------------
// Crash comparison (for the out-of-bounds-write inputs)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited(c_int),
    Signaled(c_int),
}

/// `/dev/null` opened once, in the parent, so a forked child only ever needs
/// `dup2` (async-signal-safe) before invoking the library.
fn devnull_fd() -> c_int {
    static F: OnceLock<c_int> = OnceLock::new();
    *F.get_or_init(|| {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .expect("open /dev/null");
        let fd = f.as_raw_fd();
        std::mem::forget(f); // keep the fd alive for the process lifetime
        fd
    })
}

/// Fork, run `f` in the child with stdout silenced, and report how the child
/// terminated.  This is the only way to observe the C code's out-of-bounds
/// write, which terminates the process.
fn run_in_child<F: FnOnce()>(f: F) -> Outcome {
    let _g = fd_lock();
    let null = devnull_fd();
    unsafe {
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            dup2(null, 1);
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        let termsig = status & 0x7f;
        if termsig == 0 {
            Outcome::Exited((status >> 8) & 0xff)
        } else {
            Outcome::Signaled(termsig)
        }
    }
}

pub fn child_driver(im: &Impl, data: c_int) -> Outcome {
    let f = im.driver;
    run_in_child(move || unsafe { f(data) })
}

pub fn child_print_line(im: &Impl, buf: Option<Vec<u8>>) -> Outcome {
    let f = im.print_line;
    run_in_child(move || unsafe {
        match &buf {
            None => f(std::ptr::null()),
            Some(b) => f(b.as_ptr() as *const c_char),
        }
    })
}

/// Assert both implementations terminate a child process the same way.
#[track_caller]
pub fn assert_same_outcome(row: &str, data: c_int) -> Outcome {
    let h = harness();
    let oc = child_driver(&h.c, data);
    let or = child_driver(&h.rust, data);
    assert_eq!(
        oc, or,
        "[{row}] driver({data}): C terminated as {oc:?} but Rust terminated as {or:?}"
    );
    oc
}

pub const SIGSEGV: c_int = 11;
pub const SIGBUS: c_int = 7;
