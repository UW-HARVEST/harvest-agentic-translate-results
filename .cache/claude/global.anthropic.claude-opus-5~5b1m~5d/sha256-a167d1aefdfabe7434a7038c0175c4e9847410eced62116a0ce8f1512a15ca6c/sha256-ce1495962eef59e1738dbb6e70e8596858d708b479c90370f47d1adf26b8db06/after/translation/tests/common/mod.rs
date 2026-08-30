// Shared differential-test harness.
//
// Both the C reference library and the Rust translation are loaded as shared
// objects with `libloading` and driven exclusively through `dlsym`-resolved
// `extern "C"` function pointers. No Rust function in the crate under test is
// ever called directly, so the `#[no_mangle]` export wrappers are part of what
// gets verified.
//
// Every function in this library returns `void`; the only observable behaviour
// is the bytes it writes to file descriptor 1 (via libc stdio). The harness
// therefore captures fd 1 around each call by `dup2`-ing a temporary file over
// it, flushing all libc streams before and after so that nothing leaks between
// captures.

#![allow(dead_code)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need for the fd-1 capture. Declared by hand so the harness has
// no dependency beyond `libloading`.
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, which is exactly what
    /// we need: it drains whatever the loaded `.so`s buffered in the shared
    /// glibc `stdout` without us having to name the `stdout` global.
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// The library-under-test handle
// ---------------------------------------------------------------------------

/// The four symbols the C `.so` exports, resolved out of one shared object.
pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    print_line: unsafe extern "C" fn(*const c_char),
    bad_fn: unsafe extern "C" fn(),
    good_fn: unsafe extern "C" fn(),
    driver_fn: unsafe extern "C" fn(c_int),
}

impl Lib {
    fn load(name: &'static str, path: PathBuf) -> Lib {
        // Leaked on purpose: the resolved function pointers must stay valid for
        // the whole test binary, so the `Library` must never be dropped
        // (dropping it would `dlclose` and invalidate them).
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(&path).unwrap_or_else(|e| panic!("dlopen {} ({:?}) failed: {e}", name, path))
        }));

        unsafe fn sym<T: Copy>(lib: &'static Library, which: &str, raw: &[u8]) -> T {
            let s: Symbol<T> = lib
                .get(raw)
                .unwrap_or_else(|e| panic!("dlsym {:?} in {which} failed: {e}", raw));
            *s
        }

        unsafe {
            Lib {
                name,
                print_line: sym(lib, name, b"printLine\0"),
                bad_fn: sym(lib, name, b"bad\0"),
                good_fn: sym(lib, name, b"good\0"),
                driver_fn: sym(lib, name, b"driver\0"),
                path,
            }
        }
    }

    // --- raw FFI calls, no capture -----------------------------------------

    pub unsafe fn print_line_raw(&self, p: *const c_char) {
        (self.print_line)(p)
    }
    pub unsafe fn bad_raw(&self) {
        (self.bad_fn)()
    }
    pub unsafe fn good_raw(&self) {
        (self.good_fn)()
    }
    pub unsafe fn driver_raw(&self, use_good: c_int) {
        (self.driver_fn)(use_good)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let root = manifest_dir().parent().expect("crate has a parent dir").to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
        root.join("c_src/build/Release/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C reference .so not found. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         looked in: {candidates:#?}"
    );
}

fn rust_so_path() -> PathBuf {
    // `std::env::current_exe()` is <target>/<profile>/deps/<test binary>, so the
    // cdylib built for this same profile sits two directories up. Fall back to
    // scanning the usual profile directories.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                candidates.push(profile.join("libdriver.so"));
            }
        }
    }
    let target = manifest_dir().join("target");
    candidates.push(target.join("debug/libdriver.so"));
    candidates.push(target.join("release/libdriver.so"));

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found. Build it with `cargo build` first.\nlooked in: {candidates:#?}"
    );
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c_lib() -> &'static Lib {
    C_LIB.get_or_init(|| Lib::load("C", c_so_path()))
}

pub fn rust_lib() -> &'static Lib {
    RUST_LIB.get_or_init(|| Lib::load("Rust", rust_so_path()))
}

// ---------------------------------------------------------------------------
// fd-1 capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global state, so only one capture may be in flight at a
/// time. All captures go through this lock.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn lock() -> MutexGuard<'static, ()> {
    match CAPTURE_LOCK.lock() {
        Ok(g) => g,
        // A previously panicking test poisoned the lock; the fd was already
        // restored by then, so the state is still usable.
        Err(p) => p.into_inner(),
    }
}

fn scratch_path() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "driver-difftest-{}-{}-{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ))
}

/// Run `f` with fd 1 redirected to a scratch file and return everything it
/// wrote. Works for output produced by any library in the process, because it
/// operates on the file descriptor rather than on any Rust-side wrapper.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = lock();
    let path = scratch_path();
    let file = std::fs::File::create(&path)
        .unwrap_or_else(|e| panic!("cannot create scratch file {path:?}: {e}"));

    // Drain Rust's own `io::stdout()` LineWriter first. It is a buffer that
    // lives *above* libc, so `fflush(NULL)` cannot see it; if a partial line
    // were left sitting there it would be written to fd 1 later -- i.e. into
    // our scratch file -- and corrupt the comparison.
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let saved: c_int;
    unsafe {
        // Drain anything already buffered so it lands on the *real* stdout.
        fflush(std::ptr::null_mut());
        saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
    }

    f();

    let mut out = Vec::new();
    unsafe {
        // Push the library's buffered bytes into the scratch file before we
        // detach it, then put the real stdout back.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }
    drop(file);

    std::fs::File::open(&path)
        .and_then(|mut f| f.read_to_end(&mut out))
        .unwrap_or_else(|e| panic!("cannot read scratch file {path:?}: {e}"));
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// Differential comparison helpers
// ---------------------------------------------------------------------------

fn render(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b.iter().take(400) {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > 400 {
        s.push_str(&format!("... (+{} bytes)", b.len() - 400));
    }
    s
}

/// Run the same closure against the C `.so` and the Rust `.so` and assert the
/// captured stdout bytes are identical.
#[track_caller]
pub fn assert_same<F>(case: &str, mut op: F)
where
    F: FnMut(&'static Lib),
{
    let c_out = capture(|| op(c_lib()));
    let r_out = capture(|| op(rust_lib()));
    if c_out != r_out {
        panic!(
            "differential mismatch in case `{case}`\n\
             C    ({} bytes): {}\n\
             Rust ({} bytes): {}",
            c_out.len(),
            render(&c_out),
            r_out.len(),
            render(&r_out)
        );
    }
}

/// Same as [`assert_same`] but also asserts the exact expected byte string, so
/// a test cannot pass by both sides being identically empty by accident.
#[track_caller]
pub fn assert_same_and_eq<F>(case: &str, expected: &[u8], op: F)
where
    F: FnMut(&'static Lib),
{
    let mut op = op;
    let c_out = capture(|| op(c_lib()));
    let r_out = capture(|| op(rust_lib()));
    assert_eq!(
        c_out,
        r_out,
        "differential mismatch in `{case}`: C={} Rust={}",
        render(&c_out),
        render(&r_out)
    );
    assert_eq!(
        c_out,
        expected,
        "C reference output for `{case}` is not what the C source implies: got {}, expected {}",
        render(&c_out),
        render(expected)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed, reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

    pub fn new(extra: u64) -> Rng {
        Rng(Self::SEED ^ extra)
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

    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }

    /// A byte in `1..=255` — never 0, since a 0 would terminate the string.
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }

    pub fn nonzero_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }
}

/// NUL-terminate `payload` and hand a `*const c_char` to `f`.
pub fn with_cstr<R>(payload: &[u8], f: impl FnOnce(*const c_char) -> R) -> R {
    assert!(!payload.contains(&0), "payload must not contain an interior NUL");
    let mut buf: Vec<u8> = Vec::with_capacity(payload.len() + 1);
    buf.extend_from_slice(payload);
    buf.push(0);
    let r = f(buf.as_ptr() as *const c_char);
    drop(buf);
    r
}

/// The exact bytes `printLine(s)` must emit: `s` then a single `\n`.
pub fn expected_line(payload: &[u8]) -> Vec<u8> {
    let mut v = payload.to_vec();
    v.push(b'\n');
    v
}

/// What `good()` prints, per `c_src/src/driver.c:49`.
pub const GOOD_OUTPUT: &[u8] = b"helperGood1 string\n";

/// What `bad()` prints. `helperBad()` returns the address of an automatic
/// array; the reference build emits `mov $0x0,%eax`, so `printLine` receives
/// NULL and its `if (line != NULL)` guard suppresses all output.
pub const BAD_OUTPUT: &[u8] = b"";

// ---------------------------------------------------------------------------
// Minimal sequential test runner
// ---------------------------------------------------------------------------
//
// The differential suites use `harness = false` and this runner instead of
// libtest. Reason: libtest runs `#[test]` functions on several threads and
// writes its own progress lines ("test foo ... ok") to fd 1. Because `capture`
// redirects fd 1 for the whole process, a progress line emitted by another
// thread lands inside the captured bytes and corrupts the comparison. Running
// the cases sequentially from a single thread, and flushing Rust's stdout
// before each capture, removes that race entirely.

pub struct Runner {
    filter: Option<String>,
    passed: usize,
    skipped: usize,
    failures: Vec<(String, String)>,
    suite: &'static str,
}

impl Runner {
    pub fn new(suite: &'static str) -> Runner {
        // Accept the same positional filter argument `cargo test -- <name>` uses,
        // and ignore libtest flags cargo passes through.
        let filter = std::env::args()
            .skip(1)
            .find(|a| !a.starts_with('-'));
        println!("\nrunning suite `{suite}`");
        Runner {
            filter,
            passed: 0,
            skipped: 0,
            failures: Vec::new(),
            suite,
        }
    }

    pub fn case<F: FnOnce() + std::panic::UnwindSafe>(&mut self, name: &str, f: F) {
        if let Some(flt) = &self.filter {
            if !name.contains(flt.as_str()) {
                self.skipped += 1;
                return;
            }
        }
        print!("test {name} ... ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        // Silence the default panic hook so a deliberate failure does not print
        // a backtrace *into* a capture; the message is reported by us instead.
        let prev = std::panic::take_hook();
        let sink = std::sync::Arc::new(Mutex::new(String::new()));
        let sink2 = sink.clone();
        std::panic::set_hook(Box::new(move |info| {
            let mut g = sink2.lock().unwrap_or_else(|p| p.into_inner());
            *g = format!("{info}");
        }));
        let result = std::panic::catch_unwind(f);
        std::panic::set_hook(prev);
        match result {
            Ok(()) => {
                println!("ok");
                self.passed += 1;
            }
            Err(_) => {
                println!("FAILED");
                let msg = sink.lock().unwrap_or_else(|p| p.into_inner()).clone();
                self.failures.push((name.to_string(), msg));
            }
        }
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }

    pub fn finish(self) {
        println!(
            "\nsuite `{}`: {} passed; {} failed; {} filtered out",
            self.suite,
            self.passed,
            self.failures.len(),
            self.skipped
        );
        if !self.failures.is_empty() {
            println!("\nfailures:");
            for (name, msg) in &self.failures {
                println!("---- {name} ----\n{msg}\n");
            }
            let _ = std::io::Write::flush(&mut std::io::stdout());
            std::process::exit(1);
        }
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
}

// ---------------------------------------------------------------------------
// Loading an arbitrary shared object / comparing two arbitrary libraries
// ---------------------------------------------------------------------------

/// Load a `.so` from an explicit path. Used by the optimization-level suite,
/// which rebuilds the C source at several `-O` levels to prove the reference
/// behaviour (notably `helperBad`'s NULL return) is not an `-O0` artifact.
pub fn load_from(name: &'static str, path: PathBuf) -> &'static Lib {
    Box::leak(Box::new(Lib::load(name, path)))
}

/// Differential comparison between two explicitly chosen libraries.
/// `expected` additionally pins the absolute bytes when supplied.
#[track_caller]
pub fn assert_same_between<F>(
    case: &str,
    a: &'static Lib,
    b: &'static Lib,
    expected: Option<&[u8]>,
    mut op: F,
) where
    F: FnMut(&'static Lib),
{
    let a_out = capture(|| op(a));
    let b_out = capture(|| op(b));
    if a_out != b_out {
        panic!(
            "differential mismatch in case `{case}`\n\
             {:<5}({} bytes): {}\n\
             {:<5}({} bytes): {}",
            a.name,
            a_out.len(),
            render(&a_out),
            b.name,
            b_out.len(),
            render(&b_out)
        );
    }
    if let Some(exp) = expected {
        assert_eq!(
            a_out,
            exp,
            "`{case}`: reference output is not what the C source implies: got {}, expected {}",
            render(&a_out),
            render(exp)
        );
    }
}
