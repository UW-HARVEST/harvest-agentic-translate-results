//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and drives them exclusively
//! through their exported C symbols — the Rust crate is never called
//! directly, so the `#[no_mangle] extern "C"` wrappers are under test too.
//!
//! ## Why every call is serialised
//!
//! `driver.c` keeps a `static house_t the_house` that *every* call mutates and
//! that lives for the whole process. Each loaded `.so` has its own copy. To
//! keep the two copies in lock-step we execute one logical call at a time
//! under a global mutex: C first, then Rust, comparing the captured stdout of
//! each. Because both libraries then observe the exact same call sequence,
//! their internal state stays identical no matter how the `#[test]` functions
//! are interleaved by the test runner's thread pool.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_long, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture / errno / fork
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn __errno_location() -> *mut c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
}

pub const ERANGE: c_int = 34;
pub const EINVAL: c_int = 22;

pub fn errno() -> c_int {
    unsafe { *__errno_location() }
}
pub fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v }
}

// ---------------------------------------------------------------------------
// Function signatures exported by both .so files
// ---------------------------------------------------------------------------
type RunFn = unsafe extern "C" fn(c_int);
type DriverFn = unsafe extern "C" fn(*const c_char);

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: libloading::Library,
    run: RunFn,
    driver: DriverFn,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("cannot dlopen {} ({}): {e}", path.display(), name));
            let run: libloading::Symbol<RunFn> = lib
                .get(b"run\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `run`: {e}"));
            let driver: libloading::Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `driver`: {e}"));
            let run = *run;
            let driver = *driver;
            Impl {
                name,
                path,
                _lib: lib,
                run,
                driver,
            }
        }
    }

    pub unsafe fn run(&self, extra_bedrooms: c_int) {
        (self.run)(extra_bedrooms)
    }
    pub unsafe fn driver(&self, s: *const c_char) {
        (self.driver)(s)
    }
    pub fn run_ptr(&self) -> RunFn {
        self.run
    }
    pub fn driver_ptr(&self) -> DriverFn {
        self.driver
    }
}

// `libloading::Library` is Send + Sync; the raw fn pointers are too. The
// *libraries' internal state* is what needs protecting, and that is done by
// LOCK below.
unsafe impl Send for Impl {}
unsafe impl Sync for Impl {}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

fn c_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = manifest.parent().unwrap().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // current_exe() is target/<profile>/deps/<testname>-<hash>; the cdylib
    // lives in target/<profile>/.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let p = profile_dir.join("libdriver.so");
    assert!(
        p.is_file(),
        "Rust cdylib not found at {} (build with `cargo build`)",
        p.display()
    );
    p
}

/// Capturing fd 1 is only sound if no other thread writes to it meanwhile —
/// libtest's own progress output would otherwise be mixed into the captured
/// bytes. `translation/.cargo/config.toml` pins `RUST_TEST_THREADS=1`; fail
/// loudly rather than flakily if that was bypassed.
fn assert_single_threaded() {
    let v = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
    assert_eq!(
        v, "1",
        "these differential tests capture fd 1 and MUST run single-threaded.\n\
         Set RUST_TEST_THREADS=1 (translation/.cargo/config.toml does this\n\
         automatically) or pass `-- --test-threads=1`."
    );
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        assert_single_threaded();
        Pair {
            c: Impl::load("C", c_so_path()),
            rust: Impl::load("Rust", rust_so_path()),
        }
    })
}

pub fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both `.so`s and this test binary share one libc, hence one `stdout` FILE
// buffer and one fd 1. Redirecting fd 1 at a temp file around the call
// captures exactly the bytes the library wrote.
// ---------------------------------------------------------------------------
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        // Flush whatever the harness itself may have buffered so it does not
        // land in our capture file: first Rust's own `LineWriter`-backed
        // `Stdout` (libtest writes "test foo ... " through it without a
        // trailing newline, so it stays buffered), then every C stream.
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        fflush(std::ptr::null_mut());

        let mut tmp = tempfile();
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed, errno={}", errno());
        assert!(dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 failed");

        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        if let Err(e) = res {
            std::panic::resume_unwind(e);
        }

        tmp.seek(SeekFrom::Start(0)).unwrap();
        let mut out = Vec::new();
        tmp.read_to_end(&mut out).unwrap();
        out
    }
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir();
    let n = N.fetch_add(1, Ordering::Relaxed);
    let path = dir.join(format!(
        "difftest-{}-{}-{}.out",
        std::process::id(),
        n,
        // thread id is not portable-printable; use the address of a local
        std::thread::current().id().as_u64_unchecked()
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", path.display()));
    let _ = std::fs::remove_file(&path); // unlink; fd keeps it alive
    f
}

// std::thread::ThreadId has no stable accessor; emulate one.
trait ThreadIdExt {
    fn as_u64_unchecked(&self) -> u64;
}
impl ThreadIdExt for std::thread::ThreadId {
    fn as_u64_unchecked(&self) -> u64 {
        // Debug format is "ThreadId(N)"; good enough for a filename, and the
        // file is unlinked immediately anyway.
        let s = format!("{self:?}");
        s.chars().filter(|c| c.is_ascii_digit()).collect::<String>()
            .parse()
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// One logical call, executed on both implementations in lock-step
// ---------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub enum Call {
    /// `run(extra_bedrooms)`
    Run(c_int),
    /// `driver(s)` — `s` is the *payload*; a trailing NUL is appended here.
    Driver(Vec<u8>),
    /// `run(x); run(x);` — mirrors what `driver` does internally.
    RunTwice(c_int),
}

impl Call {
    pub fn describe(&self) -> String {
        match self {
            Call::Run(x) => format!("run({x})"),
            Call::RunTwice(x) => format!("run({x}); run({x})"),
            Call::Driver(s) => format!("driver({:?})", pretty(s)),
        }
    }

    fn invoke(&self, im: &Impl) {
        unsafe {
            match self {
                Call::Run(x) => im.run(*x),
                Call::RunTwice(x) => {
                    im.run(*x);
                    im.run(*x);
                }
                Call::Driver(s) => {
                    let mut buf = s.clone();
                    buf.push(0);
                    im.driver(buf.as_ptr() as *const c_char);
                }
            }
        }
    }
}

fn pretty(s: &[u8]) -> String {
    let shown: Vec<u8> = if s.len() > 80 {
        s.iter().take(80).copied().collect()
    } else {
        s.to_vec()
    };
    let mut out = String::from_utf8_lossy(&shown).escape_debug().to_string();
    if s.len() > 80 {
        out.push_str(&format!("…(+{} bytes)", s.len() - 80));
    }
    out
}

/// Advance BOTH libraries' global state by `n` `run(extra)` calls with the
/// output thrown away.
///
/// Needed to reach deep state cheaply: `the_house.bathrooms` only ever moves
/// by `+1.0` per `run`, so demonstrating that `%.1f` still agrees once
/// `bathrooms` exceeds `f32`'s exactly-representable half-integer range
/// (2^23) requires millions of calls. Each library receives exactly the same
/// number of calls, so they stay in lock-step.
pub fn advance_both_silently(n: u64, extra: c_int) {
    let _g = lock();
    let p = pair();
    unsafe {
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        fflush(std::ptr::null_mut());
        let devnull = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .expect("open /dev/null");
        let saved = dup(1);
        assert!(saved >= 0);
        assert!(dup2(devnull.as_raw_fd(), 1) >= 0);

        for _ in 0..n {
            p.c.run(extra);
        }
        fflush(std::ptr::null_mut());
        for _ in 0..n {
            p.rust.run(extra);
        }
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0);
        close(saved);
    }
}

/// Result of one lock-step step.
pub struct StepOut {
    pub c_out: Vec<u8>,
    pub rust_out: Vec<u8>,
    pub c_errno: c_int,
    pub rust_errno: c_int,
}

/// Execute `call` on C then on Rust, capturing each stdout separately.
/// `errno_before` is installed before each invocation so both see the same
/// starting `errno`.
pub fn step_with_errno(call: &Call, errno_before: c_int) -> StepOut {
    let _g = lock();
    let p = pair();

    set_errno(errno_before);
    let c_out = capture(|| call.invoke(&p.c));
    let c_errno = errno();

    set_errno(errno_before);
    let rust_out = capture(|| call.invoke(&p.rust));
    let rust_errno = errno();

    StepOut {
        c_out,
        rust_out,
        c_errno,
        rust_errno,
    }
}

pub fn step(call: &Call) -> StepOut {
    step_with_errno(call, 0)
}

/// Execute `call` on both and assert byte-identical stdout.
pub fn assert_same(call: &Call) -> Vec<u8> {
    assert_same_errno(call, 0)
}

pub fn assert_same_errno(call: &Call, errno_before: c_int) -> Vec<u8> {
    let s = step_with_errno(call, errno_before);
    if s.c_out != s.rust_out {
        panic!(
            "stdout divergence for {}\n  C   ({} bytes): {:?}\n  Rust({} bytes): {:?}",
            call.describe(),
            s.c_out.len(),
            String::from_utf8_lossy(&s.c_out),
            s.rust_out.len(),
            String::from_utf8_lossy(&s.rust_out),
        );
    }
    assert_eq!(
        s.c_errno,
        s.rust_errno,
        "errno divergence for {} (errno_before={errno_before}): C={} Rust={}",
        call.describe(),
        s.c_errno,
        s.rust_errno
    );
    s.c_out
}

/// Cross-check the call hierarchy: run `a` on C and `b` on Rust and require
/// identical output. Used to prove `driver(s) == run(x); run(x)` holds in both
/// implementations while keeping both libraries' state advancing equally.
pub fn assert_cross(a_on_c: &Call, b_on_rust: &Call) {
    let _g = lock();
    let p = pair();
    set_errno(0);
    let c_out = capture(|| a_on_c.invoke(&p.c));
    set_errno(0);
    let rust_out = capture(|| b_on_rust.invoke(&p.rust));
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
        "cross divergence: C {} vs Rust {}",
        a_on_c.describe(),
        b_on_rust.describe()
    );
}

// ---------------------------------------------------------------------------
// Oracle helpers (mirror of the C, used only to classify expectations)
// ---------------------------------------------------------------------------

/// Exactly what `parse_val` does, using the real libc `strtol`.
pub fn c_parse_val(s: &[u8]) -> Option<c_int> {
    let mut buf = s.to_vec();
    buf.push(0);
    unsafe {
        set_errno(0);
        let base = buf.as_ptr() as *const c_char;
        let mut endp: *mut c_char = base as *mut c_char;
        let tmp = strtol(base, &mut endp, 10);
        if endp != base as *mut c_char
            && errno() == 0
            && tmp >= c_int::MIN as c_long
            && tmp <= c_int::MAX as c_long
        {
            Some(tmp as c_int)
        } else {
            None
        }
    }
}

pub const ERR_MSG: &[u8] = b"An error occurred\n";

/// Number of `printf` lines a successful `driver` emits: 2 runs x 4 lines.
pub const DRIVER_OK_LINES: usize = 8;
pub const RUN_LINES: usize = 4;

pub fn line_count(out: &[u8]) -> usize {
    out.iter().filter(|&&b| b == b'\n').count()
}

// ---------------------------------------------------------------------------
// fork-based crash comparison (for undefined-behaviour inputs such as NULL)
// ---------------------------------------------------------------------------

/// How a forked child that invoked the library terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Term {
    Exited(c_int),
    Signaled(c_int),
}

/// Fork, run `f` in the child, and report how the child terminated.
pub fn fork_probe<F: FnOnce()>(f: F) -> Term {
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed errno={}", errno());
        if pid == 0 {
            // Child: silence stdout/stderr so a crash message does not
            // pollute the test log, then perform the (possibly fatal) call.
            let devnull = std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/null")
                .ok();
            if let Some(dn) = &devnull {
                dup2(dn.as_raw_fd(), 1);
                dup2(dn.as_raw_fd(), 2);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed errno={}", errno());
        // WIFEXITED / WIFSIGNALED, glibc layout
        if status & 0x7f == 0x7f {
            // stopped; should not happen
            Term::Signaled(-1)
        } else if status & 0x7f == 0 {
            Term::Exited((status >> 8) & 0xff)
        } else {
            Term::Signaled(status & 0x7f)
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

    pub fn new() -> Rng {
        Rng(Self::SEED)
    }
    pub fn with_seed(s: u64) -> Rng {
        Rng(s)
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

    /// Uniform in `lo..=hi`.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi as i128 - lo as i128 + 1) as u128;
        lo + (self.next_u64() as u128 % span) as i64
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}
