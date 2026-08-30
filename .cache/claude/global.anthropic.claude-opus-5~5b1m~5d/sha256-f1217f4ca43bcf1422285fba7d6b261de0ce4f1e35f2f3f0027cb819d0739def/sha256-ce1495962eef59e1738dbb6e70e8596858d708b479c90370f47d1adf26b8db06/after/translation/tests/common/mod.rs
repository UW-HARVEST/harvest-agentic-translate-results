//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked only through their exported `extern "C"` symbols — the Rust crate is
//! never called directly, so the `#[no_mangle]` wrappers are under test too.
//!
//! # Harness symmetry
//!
//! `driver.c`'s `bad()` reads an *indeterminate* stack slot (its CWE-457 defect),
//! so what it prints depends on the bytes the **caller** happened to leave just
//! below its own frame. A differential harness therefore has to invoke the two
//! libraries from a *bit-for-bit identical* calling context, otherwise it reports
//! divergences that are artefacts of the test code rather than of the
//! translation. Two details matter and are handled below:
//!
//! * the C run and the Rust run go through **one** monomorphised code path and
//!   **one** call site (a loop over `[&c, &rust]` plus `&dyn Fn()` erasure).
//!   Passing two distinct closures would instantiate two distinct functions at
//!   two distinct addresses, and their return addresses would leak into the very
//!   slot `bad()` reads;
//! * the scratch file names are fixed-width, so `format!` does not allocate a
//!   differently-shaped string (and scribble the stack differently) between the
//!   two runs.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need for fd-level stdout capture (no `libc` crate needed).
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream in the process,
    /// which covers the `stdout` FILE* that both `.so`s share.
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

const STDOUT_FD: c_int = 1;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("manifest has a parent")
        .to_path_buf()
}

/// `c_src/build/libdriver.so`, produced by the CMake build.
pub fn c_so_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \\\n    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// `translation/target/{debug,release}/libdriver.so`, produced by cargo.
pub fn rust_so_path() -> PathBuf {
    // Prefer the profile the test binary itself was built with, so that we test
    // the artifact that matches this compilation.
    let preferred = if cfg!(debug_assertions) { "debug" } else { "release" };
    let other = if preferred == "debug" { "release" } else { "debug" };
    let target = manifest_dir().join("target");
    for profile in [preferred, other] {
        let p = target.join(profile).join("libdriver.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "Rust shared library not found under {}. Build it with `cargo build` / `cargo build --release`.",
        target.display()
    );
}

// ---------------------------------------------------------------------------
// The loaded pair
// ---------------------------------------------------------------------------

/// The four exported entry points of one `.so`.
pub struct Api {
    pub name: &'static str,
    _lib: Library,
    print_line: unsafe extern "C" fn(*const c_char),
    bad: unsafe extern "C" fn(),
    good: unsafe extern "C" fn(),
    driver: unsafe extern "C" fn(c_int),
    /// The very same `driver` symbol, but typed as taking a 64-bit argument, so
    /// a test can set the whole register and observe that the callee only looks
    /// at the low 32 bits (`%edi`).
    driver_wide: unsafe extern "C" fn(u64),
}

impl Api {
    fn load(name: &'static str, path: &Path) -> Api {
        // SAFETY: the paths point at the two libraries under test.
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
            let print_line: Symbol<unsafe extern "C" fn(*const c_char)> = lib
                .get(b"printLine\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol printLine: {e}"));
            let bad: Symbol<unsafe extern "C" fn()> = lib
                .get(b"bad\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol bad: {e}"));
            let good: Symbol<unsafe extern "C" fn()> = lib
                .get(b"good\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol good: {e}"));
            let driver: Symbol<unsafe extern "C" fn(c_int)> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol driver: {e}"));
            let driver_wide: Symbol<unsafe extern "C" fn(u64)> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol driver: {e}"));
            let (print_line, bad, good, driver, driver_wide) =
                (*print_line, *bad, *good, *driver, *driver_wide);
            Api {
                name,
                _lib: lib,
                print_line,
                bad,
                good,
                driver,
                driver_wide,
            }
        }
    }

    /// `void printLine(const char *line)` — the lowest-level entry point.
    #[inline(never)]
    pub unsafe fn print_line(&self, line: *const c_char) {
        (self.print_line)(line)
    }

    /// `void bad(void)`
    #[inline(never)]
    pub unsafe fn bad(&self) {
        (self.bad)()
    }

    /// `void good(void)`
    #[inline(never)]
    pub unsafe fn good(&self) {
        (self.good)()
    }

    /// `void driver(int useGood)` — the only header-declared entry point.
    #[inline(never)]
    pub unsafe fn driver(&self, use_good: c_int) {
        (self.driver)(use_good)
    }

    /// `driver` called with a full 64-bit register value: the callee is an
    /// `int`-taking function, so it must observe only the low 32 bits.
    #[inline(never)]
    pub unsafe fn driver_wide(&self, use_good: u64) {
        (self.driver_wide)(use_good)
    }
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

impl Pair {
    /// The two implementations in a fixed order, for symmetric iteration.
    pub fn both(&self) -> [&Api; 2] {
        [&self.c, &self.rust]
    }
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// The two loaded libraries. Loaded once per test process.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Api::load("C", &c_so_path()),
        rust: Api::load("Rust", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// scratch files
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so only one capture may be in flight at a time.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

fn capture_lock() -> MutexGuard<'static, ()> {
    CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn scratch_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let d = manifest_dir().join("target").join("difftest-tmp");
        fs::create_dir_all(&d).expect("create scratch dir");
        d
    })
}

static COUNTER: Mutex<u64> = Mutex::new(0);

/// A fresh scratch path. The name is **fixed width** on purpose: a
/// variable-length `format!` would allocate differently between the C run and
/// the Rust run and perturb the stack bytes `bad()` reads.
fn scratch_path(tag: &str) -> PathBuf {
    let n = {
        let mut c = COUNTER.lock().unwrap_or_else(|e| e.into_inner());
        *c += 1;
        *c
    };
    scratch_dir().join(format!("{tag}-{:010}-{:012}.bin", std::process::id(), n))
}

// ---------------------------------------------------------------------------
// stdout capture (in-process)
// ---------------------------------------------------------------------------

/// Run `f` with fd 1 redirected to a temp file and return everything written.
///
/// A temp file (not a pipe) is used deliberately: some cases write 1 MiB, which
/// would deadlock on a 64 KiB pipe buffer.
///
/// Deliberately **not generic**: see the module docs on harness symmetry.
fn capture_dyn(f: &dyn Fn()) -> Vec<u8> {
    let path = scratch_path("cap");
    let file = fs::File::create(&path).expect("create capture file");
    let saved;
    unsafe {
        // Push out anything already buffered so it does not land in our file.
        fflush(std::ptr::null_mut());
        saved = dup(STDOUT_FD);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), STDOUT_FD) >= 0, "dup2 onto fd 1 failed");
    }

    // Run the payload; make sure fd 1 is restored even on panic.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FD) >= 0, "restoring fd 1 failed");
        close(saved);
    }
    drop(file);

    let mut bytes = Vec::new();
    fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut bytes)
        .expect("read capture file");
    let _ = fs::remove_file(&path);

    match result {
        Ok(()) => bytes,
        Err(p) => std::panic::resume_unwind(p),
    }
}

/// Convenience wrapper around [`capture_dyn`] for diagnostics.
pub fn capture<F: Fn()>(f: F) -> Vec<u8> {
    let _guard = capture_lock();
    capture_dyn(&f)
}

// ---------------------------------------------------------------------------
// Crash-safe isolation via fork()
// ---------------------------------------------------------------------------

/// The full result of running a case in a forked child.
#[derive(PartialEq, Eq)]
pub struct Isolated {
    /// Everything the child wrote to fd 1.
    pub out: Vec<u8>,
    /// Raw `waitpid` status: encodes normal exit code *and* fatal signal, so
    /// "both segfaulted" is distinguishable from "both returned normally".
    pub status: c_int,
}

impl Isolated {
    pub fn exit_code(&self) -> Option<i32> {
        if self.status & 0x7f == 0 {
            Some((self.status >> 8) & 0xff)
        } else {
            None
        }
    }
    pub fn signal(&self) -> Option<i32> {
        let s = self.status & 0x7f;
        if s != 0 && s != 0x7f {
            Some(s)
        } else {
            None
        }
    }
    pub fn describe(&self) -> String {
        let term = match (self.exit_code(), self.signal()) {
            (Some(c), _) => format!("exit={c}"),
            (_, Some(s)) => format!("signal={s}{}", if s == 11 { " (SIGSEGV)" } else { "" }),
            _ => format!("raw status={}", self.status),
        };
        format!("{term} stdout={}", preview(&self.out))
    }
}

/// Run `f` in a forked child with fd 1 pointing at a fresh file, and report both
/// the bytes it produced and how it terminated.
///
/// This is required for the `bad()` / `driver(0)` paths: the C reads an
/// indeterminate pointer, so for some caller stacks it genuinely takes SIGSEGV.
/// A crash is a legitimate observable behaviour that the Rust must reproduce, and
/// it cannot be observed from inside the test process.
///
/// Deliberately **not generic**: see the module docs on harness symmetry.
fn run_isolated_dyn(f: &dyn Fn()) -> Isolated {
    use std::io::Write;

    let path = scratch_path("iso");
    // The file is opened *before* forking so the child needs no allocation.
    let file = fs::File::create(&path).expect("create isolation file");
    let out_fd = file.as_raw_fd();

    // Make sure no pending buffered bytes are duplicated into the child.
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");

    if pid == 0 {
        // ---- child ----------------------------------------------------
        // Only dup2 / the payload / _exit happen here: no allocation, no locks.
        unsafe {
            dup2(out_fd, STDOUT_FD);
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            fflush(std::ptr::null_mut());
            _exit(if r.is_ok() { 0 } else { 70 });
        }
    }

    // ---- parent -------------------------------------------------------
    let mut status: c_int = 0;
    loop {
        let r = unsafe { waitpid(pid, &mut status, 0) };
        if r == pid {
            break;
        }
        assert!(r >= 0, "waitpid failed");
    }
    drop(file);

    let mut out = Vec::new();
    fs::File::open(&path)
        .expect("reopen isolation file")
        .read_to_end(&mut out)
        .expect("read isolation file");
    let _ = fs::remove_file(&path);
    Isolated { out, status }
}

/// Convenience wrapper around [`run_isolated_dyn`] for diagnostics.
pub fn run_isolated<F: Fn()>(f: F) -> Isolated {
    let _guard = capture_lock();
    run_isolated_dyn(&f)
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn preview(b: &[u8]) -> String {
    let head: Vec<u8> = b.iter().copied().take(96).collect();
    let mut s = String::new();
    for x in &head {
        match x {
            0x20..=0x7e if *x != b'\\' => s.push(*x as char),
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            _ => s.push_str(&format!("\\x{x:02x}")),
        }
    }
    if b.len() > head.len() {
        s.push_str(&format!("...(+{} bytes)", b.len() - head.len()));
    }
    format!("len={} \"{}\"", b.len(), s)
}

fn first_diff(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.len().min(b.len()))
}

/// Run the same closure against the C API and the Rust API and require the bytes
/// written to stdout to be identical.
///
/// Both runs happen in one loop body, i.e. from one call site through one
/// monomorphisation — see the module docs on harness symmetry.
pub fn assert_same<F>(case: &str, f: F)
where
    F: Fn(&Api),
{
    let outs = capture_both(&f);
    if outs[0] != outs[1] {
        panic!(
            "stdout mismatch for case `{case}` at byte {}\n  C   : {}\n  Rust: {}",
            first_diff(&outs[0], &outs[1]),
            preview(&outs[0]),
            preview(&outs[1])
        );
    }
}

/// Like [`assert_same`] but also checks the produced bytes against a literal
/// expectation, so a test cannot pass by both sides being equally wrong.
pub fn assert_same_and_eq<F>(case: &str, expected: &[u8], f: F)
where
    F: Fn(&Api),
{
    let outs = capture_both(&f);
    if outs[0] != outs[1] {
        panic!(
            "stdout mismatch for case `{case}` at byte {}\n  C   : {}\n  Rust: {}",
            first_diff(&outs[0], &outs[1]),
            preview(&outs[0]),
            preview(&outs[1])
        );
    }
    if outs[0] != expected {
        panic!(
            "case `{case}`: both sides agree but disagree with the expected C semantics\n  got     : {}\n  expected: {}",
            preview(&outs[0]),
            preview(expected)
        );
    }
}

fn capture_both(f: &dyn Fn(&Api)) -> [Vec<u8>; 2] {
    let p = pair();
    let _guard = capture_lock();
    let mut outs: Vec<Vec<u8>> = Vec::with_capacity(2);
    // ONE call site, ONE closure type, invoked twice with different *data*.
    for api in p.both() {
        outs.push(capture_dyn(&|| f(api)));
    }
    let rust = outs.pop().unwrap();
    let c = outs.pop().unwrap();
    [c, rust]
}

/// Differential assertion for cases that may crash: compares BOTH the bytes
/// written and the termination status (exit code / fatal signal).
pub fn assert_same_isolated<F>(case: &str, f: F)
where
    F: Fn(&Api),
{
    let [c, r] = isolate_both(&f);
    assert!(
        c == r,
        "isolated mismatch for case `{case}`\n  C   : {}\n  Rust: {}",
        c.describe(),
        r.describe()
    );
}

fn isolate_both(f: &dyn Fn(&Api)) -> [Isolated; 2] {
    let p = pair();
    let _guard = capture_lock();
    let mut outs: Vec<Isolated> = Vec::with_capacity(2);
    // ONE call site, ONE closure type, invoked twice with different *data*.
    for api in p.both() {
        outs.push(run_isolated_dyn(&|| f(api)));
    }
    let rust = outs.pop().unwrap();
    let c = outs.pop().unwrap();
    [c, rust]
}

/// Crash-safe version of [`assert_same_and_eq`]: runs both libraries in forked
/// children, requires identical bytes *and* identical termination, and requires
/// that termination to be a clean `exit(0)` producing `expected`.
///
/// Use this for inputs the C is supposed to *reject* rather than fault on (e.g.
/// `printLine(NULL)`): if a regression turns the rejection into a fault, this
/// reports it as a readable assertion instead of taking the whole test process
/// down with SIGSEGV.
pub fn assert_same_and_eq_isolated<F>(case: &str, expected: &[u8], f: F)
where
    F: Fn(&Api),
{
    let [c, r] = isolate_both(&f);
    assert!(
        c == r,
        "isolated mismatch for case `{case}`\n  C   : {}\n  Rust: {}",
        c.describe(),
        r.describe()
    );
    assert_eq!(
        c.exit_code(),
        Some(0),
        "case `{case}`: both sides agree, but the C is expected to return cleanly here; got {}",
        c.describe()
    );
    assert_eq!(
        c.out,
        expected,
        "case `{case}`: both sides agree but disagree with the expected C semantics\n  got     : {}\n  expected: {}",
        preview(&c.out),
        preview(expected)
    );
}

/// Diagnostic variant of [`assert_same_isolated`] that returns the two results.
pub fn isolate_pair<F>(f: F) -> [Isolated; 2]
where
    F: Fn(&Api),
{
    isolate_both(&f)
}

// ---------------------------------------------------------------------------
// Deterministic pseudo-randomness (fixed seed, no external crate)
// ---------------------------------------------------------------------------

/// SplitMix64 — small, fast, fully deterministic.
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn byte_in(&mut self, lo: u8, hi: u8) -> u8 {
        lo + (self.below((hi - lo) as usize + 1) as u8)
    }
}

// ---------------------------------------------------------------------------
// Stack dirtying — the axis the uninitialized read in `bad()` is sensitive to
// ---------------------------------------------------------------------------

/// Scribble a recognisable pattern over the stack below the current frame, then
/// return. Anything called afterwards sees this as its "uninitialized" memory.
#[inline(never)]
pub fn dirty_stack(fill: u64, depth: u32) {
    let mut buf = [0u64; 64];
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot = fill.wrapping_add(i as u64);
    }
    // Keep the writes from being optimised away.
    std::hint::black_box(&mut buf);
    if depth > 0 {
        dirty_stack(fill, depth - 1);
    }
}
