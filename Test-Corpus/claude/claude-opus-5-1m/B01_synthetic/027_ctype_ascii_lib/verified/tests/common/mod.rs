//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! called **only** through their exported `driver` symbol, exactly as an
//! external C consumer would.  The Rust crate is never linked directly, so the
//! `#[unsafe(no_mangle)] extern "C"` wrapper is part of what is under test.
//!
//! `driver` communicates by writing to `stdout`, so the harness captures fd 1
//! around each call (`dup`/`dup2`) and compares the raw bytes.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::io::Write;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Loading the two shared objects
// ---------------------------------------------------------------------------

/// `void driver(char c)` — the real prototype from `c_src/include/driver.h`.
pub type DriverChar = unsafe extern "C" fn(c_char);

/// `void driver(int c)` — a deliberately *widened* prototype, used to probe how
/// each implementation truncates an `int` that arrives where a `char` is
/// expected (the FFI-boundary analogue of an out-of-range enum value).
pub type DriverInt = unsafe extern "C" fn(c_int);

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub driver: DriverChar,
    pub driver_int: DriverInt,
    /// Keeps the `dlopen` handle alive for as long as the fn pointers are used.
    _lib: Library,
}

impl Impl {
    /// `dlsym` probe: does this library export `name` (a NUL-terminated byte
    /// string)?  Used to assert neither library has extra/stub entry points.
    pub fn lookup(&self, name: &[u8]) -> bool {
        // SAFETY: only asks whether the symbol resolves; the pointer is never
        // dereferenced or called.
        unsafe { self._lib.get::<*mut c_void>(name).is_ok() }
    }
}

pub struct Both {
    pub c: Impl,
    pub rust: Impl,
}

/// The two loaded libraries; built on first use if they are missing.
pub fn both() -> &'static Both {
    static BOTH: OnceLock<Both> = OnceLock::new();
    BOTH.get_or_init(|| Both {
        c: load("C", ensure_c_so()),
        rust: load("Rust", ensure_rust_so()),
    })
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    // SAFETY: `path` names a plain C ABI shared object with no init side
    // effects beyond the usual CRT registration.
    let lib = unsafe { Library::new(&path) }
        .unwrap_or_else(|e| panic!("[{name}] dlopen({}) failed: {e}", path.display()));

    let driver: DriverChar = unsafe {
        let sym: Symbol<DriverChar> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("[{name}] dlsym(driver) failed: {e}"));
        *sym
    };
    // The very same symbol, viewed through the widened prototype.
    let driver_int: DriverInt = unsafe {
        let sym: Symbol<DriverInt> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("[{name}] dlsym(driver) failed: {e}"));
        *sym
    };

    Impl { name, path, driver, driver_int, _lib: lib }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Builds `c_src` with cmake if `libdriver.so` is not there yet.
fn ensure_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");
    let so = build.join("libdriver.so");
    if so.is_file() {
        return so;
    }
    std::fs::create_dir_all(&build).expect("create c_src/build");
    run(
        Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .current_dir(&build),
        "cmake configure",
    );
    run(Command::new("cmake").args(["--build", "."]).current_dir(&build), "cmake build");
    assert!(so.is_file(), "cmake did not produce {}", so.display());
    so
}

/// Locates the Rust cdylib, building it if necessary.
///
/// `cargo test` does not build a `crate-type = ["cdylib"]` library (integration
/// tests do not link it), so the harness builds it itself.  This keeps a plain
/// `cargo test` self-sufficient.
fn ensure_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    if let Some(p) = find_rust_so() {
        return p;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(&cargo);
    cmd.arg("build").current_dir(manifest_dir());
    if std::env::var_os("CARGO_NET_OFFLINE").is_none() {
        cmd.arg("--offline");
    }
    // Build the same feature set the test binary itself was built with.
    let features = enabled_features();
    cmd.arg("--no-default-features");
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    run(&mut cmd, "cargo build (cdylib)");
    find_rust_so().unwrap_or_else(|| {
        panic!("cargo build did not produce libdriver.so next to the test binary")
    })
}

/// The crate features that are on for this test binary, so the cdylib is built
/// with the identical configuration.  (`Cargo.toml` currently declares none.)
pub fn enabled_features() -> Vec<&'static str> {
    let mut f: Vec<&'static str> = Vec::new();
    // Extend with `#[cfg(feature = "x")] f.push("x");` if features are added.
    f.sort_unstable();
    f
}

fn find_rust_so() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // target/<profile>/deps/<test>-<hash>  ->  target/<profile>/libdriver.so
    for dir in exe.ancestors().skip(1) {
        let cand = dir.join("libdriver.so");
        if cand.is_file() && !cand.starts_with(manifest_dir().join("c_src")) {
            return Some(cand);
        }
    }
    None
}

fn run(cmd: &mut Command, what: &str) {
    let out = cmd.output().unwrap_or_else(|e| panic!("{what}: spawn failed: {e}"));
    assert!(
        out.status.success(),
        "{what} failed: {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Where `stdout` should point while a call is captured.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sink {
    /// A regular temporary file (glibc: fully buffered).
    File,
    /// A pipe (glibc: fully buffered, different `fstat` path).
    Pipe,
    /// fd 1 closed outright — every `printf` must fail with `EBADF`.
    Closed,
    /// fd 1 replaced by a read-only fd — every write must fail with `EBADF`.
    ReadOnly,
    /// `/dev/full` — writes succeed into the buffer, the flush fails `ENOSPC`.
    DevFull,
}

/// `setvbuf` mode to force on `stdout` for the duration of the call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buffering {
    /// Leave whatever glibc picked.
    Default,
    /// `setvbuf(stdout, NULL, _IOFBF, 0)`
    Full,
    /// `setvbuf(stdout, NULL, _IOLBF, 0)`
    Line,
    /// `setvbuf(stdout, NULL, _IONBF, 0)`
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Opts {
    pub sink: Sink,
    pub buffering: Buffering,
}

impl Default for Opts {
    fn default() -> Self {
        Opts { sink: Sink::File, buffering: Buffering::Default }
    }
}

impl Opts {
    pub fn file() -> Self {
        Opts::default()
    }
    pub fn sink(sink: Sink) -> Self {
        Opts { sink, buffering: Buffering::Default }
    }
    pub fn buffering(buffering: Buffering) -> Self {
        Opts { sink: Sink::File, buffering }
    }
}

/// The `libc` crate does not expose glibc's `stdout` global, so bind it here.
mod c_globals {
    unsafe extern "C" {
        #[link_name = "stdout"]
        pub static mut STDOUT: *mut libc::FILE;
    }
}

/// glibc's `FILE *stdout` — the very stream both implementations `printf` to.
pub fn c_stdout() -> *mut libc::FILE {
    // SAFETY: reads the pointer value out of libc's `stdout` global.  libc
    // initialises it before `main`, and it is never reassigned.
    unsafe { c_globals::STDOUT }
}

/// Serializes fd-1 surgery: cargo runs tests in parallel threads by default.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static SEQ: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Serialization of process-global state
// ---------------------------------------------------------------------------

/// Everything under test here lives in process-global state — the locale, fd 1,
/// and `stdout`'s buffering mode — and a differential case is only meaningful
/// if the C run and the Rust run see *identical* state.  Cargo runs tests in
/// parallel threads, so a case must own that state for its whole duration
/// (setup included), not just while fd 1 is redirected.
///
/// The guard is re-entrant per thread so a test may hold it across several
/// [`diff_case`] calls without deadlocking.
static STATE_LOCK: Mutex<()> = Mutex::new(());

std::thread_local! {
    static STATE_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

pub struct StateGuard {
    _held: Option<std::sync::MutexGuard<'static, ()>>,
}

impl Drop for StateGuard {
    fn drop(&mut self) {
        STATE_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

/// Takes exclusive ownership of the process-global state (re-entrantly).
pub fn state_lock() -> StateGuard {
    STATE_DEPTH.with(|d| {
        if d.get() == 0 {
            let held = STATE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
            d.set(1);
            StateGuard { _held: Some(held) }
        } else {
            d.set(d.get() + 1);
            StateGuard { _held: None }
        }
    })
}

/// Everything observable about one captured call.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Captured {
    /// Raw bytes written to `stdout` while the call ran.
    pub bytes: Vec<u8>,
    /// `errno` immediately after the call and the final `fflush`.
    pub errno: c_int,
    /// `ferror(stdout)` immediately after the call and the final `fflush`
    /// (non-zero = the stream is in an error state).
    pub ferror: c_int,
}

/// Runs `f` with `stdout` redirected per `opts` and returns everything written.
pub fn capture_with(opts: Opts, f: impl FnOnce()) -> Vec<u8> {
    capture_with_state(opts, f).bytes
}

/// As [`capture_with`], but also reports the libc error state left behind —
/// needed by the error-path rows, where the *only* observable difference
/// between "silently ignored the failure" and "handled it" is `errno`/`ferror`.
pub fn capture_with_state(opts: Opts, f: impl FnOnce()) -> Captured {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    // Get the harness's own pending output out of the way first.
    std::io::stdout().flush().ok();
    std::io::stderr().flush().ok();
    // SAFETY: `fflush(NULL)` flushes every open C stream; no arguments to get
    // wrong, and the remaining libc calls below are plain fd/FILE plumbing.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    let tmp_path = temp_path();

    unsafe {
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed: {}", errno_string());

        // Install the requested sink on fd 1 and remember how to read it back.
        let mut read_fd: c_int = -1; // fd to drain afterwards (-1 = nothing)
        let mut extra_close: Vec<c_int> = Vec::new();
        match opts.sink {
            Sink::File => {
                let fd = open_rw(&tmp_path);
                assert_eq!(libc::dup2(fd, 1), 1, "dup2 failed: {}", errno_string());
                read_fd = fd;
            }
            Sink::Pipe => {
                let mut fds = [0 as c_int; 2];
                assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe failed");
                assert_eq!(libc::dup2(fds[1], 1), 1, "dup2 failed");
                libc::close(fds[1]);
                read_fd = fds[0];
            }
            Sink::Closed => {
                libc::close(1);
            }
            Sink::ReadOnly => {
                // A read-only fd on /dev/null: writes fail with EBADF.
                let name = CString::new("/dev/null").unwrap();
                let fd = libc::open(name.as_ptr(), libc::O_RDONLY);
                assert!(fd >= 0, "open /dev/null failed");
                assert_eq!(libc::dup2(fd, 1), 1, "dup2 failed");
                extra_close.push(fd);
            }
            Sink::DevFull => {
                let name = CString::new("/dev/full").unwrap();
                let fd = libc::open(name.as_ptr(), libc::O_WRONLY);
                assert!(fd >= 0, "open /dev/full failed");
                assert_eq!(libc::dup2(fd, 1), 1, "dup2 failed");
                extra_close.push(fd);
            }
        }

        set_buffering(opts.buffering);

        // Clear errno so a test can inspect it afterwards if it wants to.
        *libc::__errno_location() = 0;

        f();

        // Push everything the callee buffered into the sink before we unhook it.
        libc::fflush(std::ptr::null_mut());

        // Sample the error state the callee left behind, before we clean up.
        let post_errno = *libc::__errno_location();
        let post_ferror = libc::ferror(c_stdout());

        // Restore fd 1 and the default buffering.
        assert_eq!(libc::dup2(saved, 1), 1, "restoring fd 1 failed");
        libc::close(saved);
        if opts.buffering != Buffering::Default {
            set_buffering(Buffering::Full);
        }
        // The error sinks left `stdout` in an error state; clear it so later
        // captures are unaffected.
        if matches!(opts.sink, Sink::Closed | Sink::ReadOnly | Sink::DevFull) {
            libc::clearerr(c_stdout());
        }
        for fd in extra_close {
            libc::close(fd);
        }

        let data = if read_fd >= 0 {
            if opts.sink == Sink::File {
                libc::lseek(read_fd, 0, libc::SEEK_SET);
            }
            let d = drain_fd(read_fd);
            libc::close(read_fd);
            d
        } else {
            Vec::new()
        };

        let _ = std::fs::remove_file(&tmp_path);
        Captured { bytes: data, errno: post_errno, ferror: post_ferror }
    }
}

/// `capture_with(Opts::default(), f)`
pub fn capture(f: impl FnOnce()) -> Vec<u8> {
    capture_with(Opts::default(), f)
}

/// How many times a capture may be retried when it is polluted (see below).
const CAPTURE_ATTEMPTS: usize = 16;

/// Captures `f`, retrying while the result fails `validate`.
///
/// Why retries are needed: libtest prints its own progress (`test foo ... ok`)
/// to **fd 1** from its runner thread, and it does so whenever *any* test
/// finishes — including while another test has fd 1 redirected.  That is
/// outside this harness's control, so instead of pretending it cannot happen,
/// every capture is validated for structural integrity and retried if foreign
/// bytes landed in it.  A capture that never validates is a hard failure, so
/// pollution can never be mistaken for a passing comparison.
pub fn capture_valid(
    opts: Opts,
    what: &str,
    validate: &dyn Fn(&[u8]) -> Result<(), String>,
    f: &dyn Fn(),
) -> Vec<u8> {
    let mut last = Vec::new();
    let mut why = String::from("<never ran>");
    for _ in 0..CAPTURE_ATTEMPTS {
        let out = capture_with(opts, || f());
        match validate(&out) {
            Ok(()) => return out,
            Err(e) => {
                why = e;
                last = out;
            }
        }
    }
    panic!(
        "{what}: no valid capture after {CAPTURE_ATTEMPTS} attempts: {why}\n  last: {}",
        escape(&last)
    );
}

/// Captures `f` and requires the output to be exactly `n` well-formed `driver`
/// records.
pub fn capture_records(opts: Opts, what: &str, n: usize, f: &dyn Fn()) -> Vec<u8> {
    capture_valid(
        opts,
        what,
        &|out| match parse_records(out) {
            Ok(r) if r.len() == n => Ok(()),
            Ok(r) => Err(format!("{} records, expected {n}", r.len())),
            Err(e) => Err(e),
        },
        f,
    )
}

/// Captures one `driver(c)` call and requires a single well-formed record.
pub fn capture_call(imp: &Impl, c: c_char) -> Vec<u8> {
    capture_records(Opts::file(), &format!("[{}] driver({})", imp.name, show(c)), 1, &|| unsafe {
        (imp.driver)(c)
    })
}

fn temp_path() -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("driver-diff-{}-{}.out", std::process::id(), n))
}

unsafe fn open_rw(path: &Path) -> c_int {
    let name = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let fd = unsafe {
        libc::open(name.as_ptr(), libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC, 0o600 as c_int)
    };
    assert!(fd >= 0, "open({}) failed: {}", path.display(), errno_string());
    fd
}

unsafe fn drain_fd(fd: c_int) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    out
}

unsafe fn set_buffering(mode: Buffering) {
    let m = match mode {
        Buffering::Default => return,
        Buffering::Full => libc::_IOFBF,
        Buffering::Line => libc::_IOLBF,
        Buffering::None => libc::_IONBF,
    };
    unsafe {
        libc::fflush(c_stdout());
        libc::setvbuf(c_stdout(), std::ptr::null_mut(), m, 0);
    }
}

// ---------------------------------------------------------------------------
// Failing-sink runs, executed in a forked child
// ---------------------------------------------------------------------------

/// What a child process reports back after running `driver` with a broken
/// `stdout`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChildReport {
    /// `errno` after the call and the final `fflush`.
    pub errno: c_int,
    /// `ferror(stdout)` after the call and the final `fflush`.
    pub ferror: c_int,
    /// `Some(code)` if the child exited normally.
    pub exit_code: Option<c_int>,
    /// `Some(sig)` if the child was killed (e.g. `SIGABRT` from a Rust abort).
    pub signal: Option<c_int>,
    /// Whether the child managed to send its report at all.
    pub reported: bool,
}

/// Runs `f` in a **forked child** whose `stdout` is wired to a failing sink.
///
/// Forking is what makes these rows possible at all: redirecting fd 1 in-process
/// would also break libtest's own progress output.  It additionally turns "the
/// library did not abort" into a checkable fact — a Rust `panic = "abort"` on the
/// failed `printf` would kill the child with `SIGABRT` and lose the report,
/// whereas C returns normally and the report arrives.
pub fn run_with_failing_sink(sink: Sink, buffering: Buffering, f: &dyn Fn()) -> ChildReport {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    // Pre-open the sink and pre-build anything that allocates, before forking.
    let dev_null = CString::new("/dev/null").unwrap();
    let dev_full = CString::new("/dev/full").unwrap();

    // Flush everything first: the child inherits a copy of the stdio buffers,
    // and pending bytes would otherwise be emitted twice.
    std::io::stdout().flush().ok();
    std::io::stderr().flush().ok();

    // SAFETY: `fork` in a multi-threaded process is only used to run
    // async-signal-safe-ish work here: an `open`/`dup2`, the library call
    // itself, a `write` of 8 bytes and `_exit`.  Nothing that allocates is
    // introduced by the harness, and `alarm` guarantees the child cannot hang
    // the test run.
    unsafe {
        libc::fflush(std::ptr::null_mut());

        let mut report = [0 as c_int; 2];
        assert_eq!(libc::pipe(report.as_mut_ptr()), 0, "pipe failed");
        let (rd, wr) = (report[0], report[1]);

        let pid = libc::fork();
        assert!(pid >= 0, "fork failed: {}", errno_string());

        if pid == 0 {
            // ---- child ----
            libc::close(rd);
            libc::alarm(20); // never hang the test run

            match sink {
                Sink::Closed => {
                    libc::close(1);
                }
                Sink::ReadOnly => {
                    let fd = libc::open(dev_null.as_ptr(), libc::O_RDONLY);
                    if fd < 0 {
                        libc::_exit(91);
                    }
                    libc::dup2(fd, 1);
                }
                Sink::DevFull => {
                    let fd = libc::open(dev_full.as_ptr(), libc::O_WRONLY);
                    if fd < 0 {
                        libc::_exit(92);
                    }
                    libc::dup2(fd, 1);
                }
                Sink::File | Sink::Pipe => {
                    // Not a failing sink; send the output to /dev/null so the
                    // child's stdout still behaves.
                    let fd = libc::open(dev_null.as_ptr(), libc::O_WRONLY);
                    if fd < 0 {
                        libc::_exit(93);
                    }
                    libc::dup2(fd, 1);
                }
            }

            let mode = match buffering {
                Buffering::Default => None,
                Buffering::Full => Some(libc::_IOFBF),
                Buffering::Line => Some(libc::_IOLBF),
                Buffering::None => Some(libc::_IONBF),
            };
            if let Some(m) = mode {
                libc::setvbuf(c_stdout(), std::ptr::null_mut(), m, 0);
            }

            *libc::__errno_location() = 0;
            f();
            libc::fflush(c_stdout());

            let payload =
                [*libc::__errno_location() as i32, libc::ferror(c_stdout()) as i32];
            let bytes = std::slice::from_raw_parts(payload.as_ptr() as *const u8, 8);
            libc::write(wr, bytes.as_ptr() as *const c_void, 8);
            libc::_exit(0);
        }

        // ---- parent ----
        libc::close(wr);
        let mut buf = [0u8; 8];
        let mut got = 0usize;
        while got < 8 {
            let n = libc::read(rd, buf[got..].as_mut_ptr() as *mut c_void, 8 - got);
            if n <= 0 {
                break;
            }
            got += n as usize;
        }
        libc::close(rd);

        let mut status: c_int = 0;
        libc::waitpid(pid, &mut status, 0);

        let exit_code =
            if libc::WIFEXITED(status) { Some(libc::WEXITSTATUS(status)) } else { None };
        let signal =
            if libc::WIFSIGNALED(status) { Some(libc::WTERMSIG(status)) } else { None };

        let (errno, ferror) = if got == 8 {
            (
                c_int::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
                c_int::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
            )
        } else {
            (0, 0)
        };

        ChildReport { errno, ferror, exit_code, signal, reported: got == 8 }
    }
}

pub fn errno_string() -> String {
    // SAFETY: plain reads of errno / strerror's static buffer.
    unsafe {
        let e = *libc::__errno_location();
        let s = libc::strerror(e);
        format!("{} ({})", CStr::from_ptr(s).to_string_lossy(), e)
    }
}

pub fn errno() -> c_int {
    unsafe { *libc::__errno_location() }
}

// ---------------------------------------------------------------------------
// Locale control (the one runtime "option" this API has)
// ---------------------------------------------------------------------------

/// The locales exercised by the configuration table.  `C` is always present;
/// the rest are skipped gracefully if the system lacks them.
pub const LOCALES: &[&str] = &[
    "C",
    "C.utf8",
    "en_US.iso88591",
    "en_US.utf8",
    "de_DE.iso88591",
    "tr_TR.iso88599",
    "ru_RU.koi8r",
    "ja_JP.eucjp",
];

/// `setlocale(LC_ALL, name)`; returns false if the locale is unavailable.
pub fn set_global_locale(name: &str) -> bool {
    let n = CString::new(name).unwrap();
    // SAFETY: `n` is a valid NUL-terminated string; `setlocale` only reads it.
    unsafe { !libc::setlocale(libc::LC_ALL, n.as_ptr()).is_null() }
}

/// `setlocale(LC_ALL, NULL)` — the current global locale's name.
pub fn global_locale() -> String {
    // SAFETY: query form of `setlocale`; the returned string is static.
    unsafe {
        let p = libc::setlocale(libc::LC_ALL, std::ptr::null());
        if p.is_null() {
            "<null>".to_string()
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}

/// glibc's `LC_GLOBAL_LOCALE` (`(locale_t) -1`).
pub fn lc_global_locale() -> libc::locale_t {
    usize::MAX as libc::locale_t
}

/// A per-thread locale installed with `uselocale`, restored on drop.
pub struct ThreadLocale {
    previous: libc::locale_t,
    installed: libc::locale_t,
}

impl ThreadLocale {
    /// Installs `name` as this thread's locale.  `None` if unavailable.
    pub fn install(name: &str) -> Option<ThreadLocale> {
        let n = CString::new(name).unwrap();
        // SAFETY: standard newlocale/uselocale sequence; both pointers are
        // checked before use and released in `Drop`.
        unsafe {
            let loc = libc::newlocale(libc::LC_ALL_MASK, n.as_ptr(), std::ptr::null_mut());
            if loc.is_null() {
                return None;
            }
            let previous = libc::uselocale(loc);
            if previous.is_null() {
                libc::freelocale(loc);
                return None;
            }
            Some(ThreadLocale { previous, installed: loc })
        }
    }

    /// The `locale_t` currently active for this thread (`uselocale(NULL)`).
    pub fn current() -> libc::locale_t {
        // SAFETY: query form of `uselocale`.
        unsafe { libc::uselocale(std::ptr::null_mut()) }
    }
}

impl Drop for ThreadLocale {
    fn drop(&mut self) {
        // SAFETY: restores the locale saved in `install`, then frees ours.
        unsafe {
            libc::uselocale(self.previous);
            libc::freelocale(self.installed);
        }
    }
}

/// The subset of [`LOCALES`] this system can actually install globally.
pub fn available_locales() -> Vec<&'static str> {
    let mut v = Vec::new();
    for &l in LOCALES {
        if set_global_locale(l) {
            v.push(l);
        }
    }
    set_global_locale("C");
    v
}

/// The subset of [`LOCALES`] this system can install as a *thread* locale.
pub fn available_thread_locales() -> Vec<&'static str> {
    let mut v = Vec::new();
    for &l in LOCALES {
        if let Some(tl) = ThreadLocale::install(l) {
            drop(tl);
            v.push(l);
        }
    }
    v
}

/// Emits a line of the *caller's* own output into the captured stream, through
/// the same libc `printf` and the same `FILE *stdout` that `driver` uses.
pub fn caller_printf(marker: &str) {
    let fmt = CString::new("caller[%s]\n").unwrap();
    let arg = CString::new(marker).unwrap();
    // SAFETY: one `%s` in the format, matched by one NUL-terminated `char *`.
    unsafe {
        libc::printf(fmt.as_ptr(), arg.as_ptr());
    }
}

/// Resets both the global and the thread locale to the process default, so
/// each row starts from a known state.
pub fn reset_locale() {
    // SAFETY: both are plain locale calls with static/sentinel arguments.
    unsafe {
        libc::uselocale(lc_global_locale());
    }
    set_global_locale("C");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5D1F_C0DE_1234_5678;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A uniformly random `char` bit pattern.
    pub fn char(&mut self) -> c_char {
        (self.next_u64() & 0xFF) as u8 as c_char
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

/// All 256 `char` bit patterns, in ascending unsigned byte order.
pub fn all_chars() -> Vec<c_char> {
    (0..=255u16).map(|b| b as u8 as c_char).collect()
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

/// The 14 line labels `driver` emits, in order — used to attribute a
/// divergence to the specific `<ctype.h>` interface that caused it.
pub const LABELS: [&str; 14] = [
    "alphanumeric",
    "alphabetic",
    "lowercase",
    "uppercase",
    "digit",
    "hexadecimal",
    "control",
    "graphical",
    "space",
    "blank",
    "printing",
    "punctuation",
    "to lower",
    "to upper",
];

pub fn escape(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7E => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Compares two captures line by line (Axis 1: one line per ctype interface).
///
/// `Err` describes the first differing line, naming the interface it came from.
pub fn compare_lines(c: &[u8], rust: &[u8]) -> Result<(), String> {
    if c == rust {
        return Ok(());
    }
    let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
    let rl: Vec<&[u8]> = rust.split(|&b| b == b'\n').collect();
    if cl.len() != rl.len() {
        return Err(format!(
            "line count differs: C has {} lines, Rust has {}\n  C   : {}\n  Rust: {}",
            cl.len(),
            rl.len(),
            escape(c),
            escape(rust)
        ));
    }
    for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
        if a != b {
            let label = LABELS.get(i % 14).copied().unwrap_or("<unknown>");
            return Err(format!(
                "line {i} (interface `{label}`) differs:\n  C   : {}\n  Rust: {}",
                escape(a),
                escape(b)
            ));
        }
    }
    Err("captures differ but no line differs (impossible)".to_string())
}

/// One `driver` call's output, parsed into its 14 field values.
pub type Record = Vec<Vec<u8>>;

/// Parses a capture into consecutive `driver` records, validating the exact
/// wire format of each.
///
/// The 12 classification lines are `"<label>: <digits>\n"`; the two conversion
/// lines are `"<label>: <exactly one byte>\n"` (`printf("%c")` always emits
/// exactly one byte, which may itself be `\n` or `\0`).  Parsing rather than
/// splitting on `\n` is what makes the structural check correct for
/// `driver('\n')` and `driver('\0')`.
pub fn parse_records(out: &[u8]) -> Result<Vec<Record>, String> {
    let mut records = Vec::new();
    let mut pos = 0usize;
    while pos < out.len() {
        let mut fields: Record = Vec::with_capacity(14);
        for (i, label) in LABELS.iter().enumerate() {
            let prefix = format!("{label}: ").into_bytes();
            if !out[pos..].starts_with(&prefix) {
                return Err(format!(
                    "record #{}: expected `{label}: ` at byte {pos}, found `{}`",
                    records.len(),
                    escape(&out[pos..out.len().min(pos + 24)])
                ));
            }
            pos += prefix.len();
            let value_len = if i < 12 {
                // `%d`: digits up to the newline.
                match out[pos..].iter().position(|&b| b == b'\n') {
                    Some(n) => n,
                    None => {
                        return Err(format!(
                            "record #{}: `{label}` line is not newline-terminated",
                            records.len()
                        ));
                    }
                }
            } else {
                // `%c`: exactly one byte, whatever it is.
                1
            };
            if pos + value_len >= out.len() + 1 {
                return Err(format!("record #{}: truncated `{label}` line", records.len()));
            }
            fields.push(out[pos..pos + value_len].to_vec());
            pos += value_len;
            if out.get(pos) != Some(&b'\n') {
                return Err(format!(
                    "record #{}: `{label}` line not terminated by a newline (found {:?})",
                    records.len(),
                    out.get(pos).map(|&b| escape(&[b]))
                ));
            }
            pos += 1;
        }
        records.push(fields);
    }
    Ok(records)
}

/// Runs one differential case.
///
/// `setup` is applied identically before each implementation runs, then the
/// body drives the loaded `driver` symbol, and the captured bytes must match.
/// `expect_calls`, when given, additionally asserts each capture parses as
/// exactly that many well-formed 14-field `driver` records — a guard against a
/// broken capture silently comparing two empty (or malformed) buffers, and the
/// per-interface (Axis 1) comparison.
/// Returns the C implementation's captured bytes, so a caller can additionally
/// compare them against a baseline.
pub fn diff_case(
    row: &str,
    detail: &str,
    opts: Opts,
    expect_calls: Option<usize>,
    setup: &dyn Fn(),
    body: &dyn Fn(DriverChar),
) -> Vec<u8> {
    let b = both();
    // Own the global state across BOTH runs, so the two see identical setup.
    let _serial = state_lock();

    let (out_c, post_c, out_rust, post_rust) = match expect_calls {
        // Structure known: validate (and retry past libtest's own fd-1 output).
        Some(n) => {
            let out_c = capture_records(opts, &format!("[{row}] {detail} (C)"), n, &|| {
                setup();
                body(b.c.driver)
            });
            let post_c = PostState::snapshot();
            let out_rust = capture_records(opts, &format!("[{row}] {detail} (Rust)"), n, &|| {
                setup();
                body(b.rust.driver)
            });
            let post_rust = PostState::snapshot();
            (out_c, post_c, out_rust, post_rust)
        }
        None => {
            setup();
            let out_c = capture_with(opts, || body(b.c.driver));
            let post_c = PostState::snapshot();
            setup();
            let out_rust = capture_with(opts, || body(b.rust.driver));
            let post_rust = PostState::snapshot();
            (out_c, post_c, out_rust, post_rust)
        }
    };

    if let Some(n) = expect_calls {
        let recs_c = parse_records(&out_c).expect("validated above");
        let recs_rust = parse_records(&out_rust).expect("validated above");
        assert_eq!(recs_c.len(), n);
        assert_eq!(recs_rust.len(), n);
        // Axis 1: attribute any divergence to the exact ctype interface.
        for (k, (rc, rr)) in recs_c.iter().zip(recs_rust.iter()).enumerate() {
            for (i, label) in LABELS.iter().enumerate() {
                assert_eq!(
                    escape(&rc[i]),
                    escape(&rr[i]),
                    "[{row}] {detail}: call #{k}, interface `{label}` diverges"
                );
            }
        }
    }

    if let Err(why) = compare_lines(&out_c, &out_rust) {
        panic!(
            "[{row}] {detail}\n  opts: {opts:?}\n{why}\n  full C   : {}\n  full Rust: {}",
            escape(&out_c),
            escape(&out_rust)
        );
    }

    assert_eq!(
        post_c, post_rust,
        "[{row}] {detail}: observable locale state after the call differs between C and Rust"
    );

    out_c
}

/// Renders a `char` argument for assertion messages.
pub fn show(c: c_char) -> String {
    let b = c as u8;
    if (0x21..=0x7E).contains(&b) {
        format!("0x{b:02x} ({}) '{}'", c, b as char)
    } else {
        format!("0x{b:02x} ({})", c)
    }
}

/// Splits a capture into `\n`-terminated lines (without the trailing empty).
pub fn lines_of(out: &[u8]) -> Vec<&[u8]> {
    let mut v: Vec<&[u8]> = out.split(|&b| b == b'\n').collect();
    if v.last().map(|l| l.is_empty()).unwrap_or(false) {
        v.pop();
    }
    v
}

/// Side effects `driver` is allowed to leave behind, observed after each call.
#[derive(PartialEq, Eq, Debug)]
pub struct PostState {
    pub global_locale: String,
    pub thread_locale_is_global: bool,
}

impl PostState {
    pub fn snapshot() -> PostState {
        PostState {
            global_locale: global_locale(),
            thread_locale_is_global: ThreadLocale::current() == lc_global_locale(),
        }
    }
}

/// Convenience: the byte output of one `driver(c)` call, per implementation.
pub fn run_one(imp: &Impl, c: c_char) -> Vec<u8> {
    capture_call(imp, c)
}
