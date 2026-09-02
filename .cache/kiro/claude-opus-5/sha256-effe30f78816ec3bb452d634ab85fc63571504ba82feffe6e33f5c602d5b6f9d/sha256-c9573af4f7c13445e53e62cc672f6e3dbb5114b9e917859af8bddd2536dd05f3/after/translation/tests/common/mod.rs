// Shared differential-test harness.
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
// **only** through their exported symbols, exactly as an external C consumer
// would -- the Rust functions are never called directly, so the
// `#[no_mangle] extern "C"` wrappers are under test too.
//
// The library's only observable output is bytes on `stdout` (via `puts`, which
// is what both GCC and LLVM lower `printf("%s\n", line)` to). Capturing that
// through a plain `dup2` on fd 1 would be process-global and therefore unsafe
// with cargo's parallel test threads, and it could not observe the `SIGSEGV`
// that `driver` is *supposed* to produce for negative `data`. So every batch of
// calls runs in a **forked child** that redirects its own fd 1 to a temp file:
//
//   * perfectly isolated, so tests can run in parallel;
//   * captures the exact byte stream, including stdio buffering effects;
//   * lets us compare the child's *termination status* (exit code vs. signal
//     number), which is how the error rows in `ERRORS.md` are observable.

#![allow(dead_code)]

use std::ffi::CString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed, reproducible across runs and
// across platforms. Deliberately not `rand`, so the corpus never shifts when a
// dependency updates.
// ---------------------------------------------------------------------------

/// The one seed used by every property-style test in this suite.
pub const SEED: u64 = 0x5EED_D00D;

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

    /// Uniform-ish value in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    /// Inclusive `[lo, hi]`, correct even for the full `i32` span.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }

    /// Inclusive `[lo, hi]` byte.
    pub fn byte_in(&mut self, lo: u8, hi: u8) -> u8 {
        (lo as u64 + self.below((hi - lo) as u64 + 1)) as u8
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library built by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so")
}

/// Path to the Rust `cdylib`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("target/release/libdriver.so")
}

fn require(path: &Path, how: &str) {
    if !path.exists() {
        panic!(
            "shared library not found: {}\nBuild it first:\n  {}",
            path.display(),
            how
        );
    }
}

// ---------------------------------------------------------------------------
// Loaded library handles
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(c_int);
type PrintLineFn = unsafe extern "C" fn(*const c_char);

/// A `.so` loaded through `libloading`, with its two exported entry points
/// resolved by name. The `Library` is leaked so the raw function pointers stay
/// valid for the whole test binary.
pub struct DriverLib {
    pub name: &'static str,
    pub path: PathBuf,
    pub driver: DriverFn,
    pub print_line: PrintLineFn,
}

impl DriverLib {
    fn open(name: &'static str, path: PathBuf) -> DriverLib {
        let lib = unsafe { libloading::Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        // Leak so the symbols outlive this scope.
        let lib: &'static libloading::Library = Box::leak(Box::new(lib));

        let driver: libloading::Symbol<DriverFn> = unsafe { lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("{} does not export `driver`: {e}", path.display()));
        let print_line: libloading::Symbol<PrintLineFn> = unsafe { lib.get(b"printLine\0") }
            .unwrap_or_else(|e| panic!("{} does not export `printLine`: {e}", path.display()));

        DriverLib {
            name,
            path,
            driver: *driver,
            print_line: *print_line,
        }
    }
}

/// The pair under comparison.
pub struct Pair {
    pub c: DriverLib,
    pub rust: DriverLib,
}

/// Load both shared objects. Panics with build instructions if either is
/// missing.
pub fn load_pair() -> Pair {
    let c = c_so_path();
    let r = rust_so_path();
    require(
        &c,
        "cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
    );
    require(&r, "cd translation && cargo build --release");
    Pair {
        c: DriverLib::open("C", c),
        rust: DriverLib::open("Rust", r),
    }
}

// ---------------------------------------------------------------------------
// Call scripts
// ---------------------------------------------------------------------------

/// One invocation of the library, replayed identically against both `.so`s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    /// `driver(data)`
    Driver(i32),
    /// `printLine(<bytes>)` -- a NUL terminator is appended by the harness.
    PrintLine(Vec<u8>),
    /// `printLine(NULL)`
    PrintLineNull,
}

impl Call {
    pub fn print_line(s: &[u8]) -> Call {
        assert!(
            !s.contains(&0),
            "PrintLine payload must not contain an interior NUL"
        );
        Call::PrintLine(s.to_vec())
    }
}

// ---------------------------------------------------------------------------
// Outcome of running a call script in a child process
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Exact bytes the child wrote to fd 1.
    pub stdout: Vec<u8>,
    /// `Some(code)` if the child exited normally.
    pub exit: Option<i32>,
    /// `Some(signum)` if the child was killed by a signal.
    pub signal: Option<i32>,
}

impl Outcome {
    pub fn exited_ok(&self) -> bool {
        self.exit == Some(0)
    }
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Outcome {{ ")?;
        match (self.exit, self.signal) {
            (Some(c), _) => write!(f, "exit: {c}, ")?,
            (_, Some(s)) => write!(f, "signal: {s}, ")?,
            _ => write!(f, "status: unknown, ")?,
        }
        write!(f, "stdout: {} bytes = {} }}", self.stdout.len(), render(&self.stdout))
    }
}

/// Compact, readable rendering of a byte stream for assertion messages.
pub fn render(b: &[u8]) -> String {
    let show = |slice: &[u8]| -> String {
        let mut s = String::from("\"");
        for &c in slice {
            match c {
                b'\n' => s.push_str("\\n"),
                b'\t' => s.push_str("\\t"),
                b'\r' => s.push_str("\\r"),
                b'\\' => s.push_str("\\\\"),
                b'"' => s.push_str("\\\""),
                0x20..=0x7e => s.push(c as char),
                _ => s.push_str(&format!("\\x{c:02x}")),
            }
        }
        s.push('"');
        s
    };
    if b.len() <= 96 {
        show(b)
    } else {
        format!(
            "{}...{} (len {})",
            show(&b[..48]),
            show(&b[b.len() - 24..]),
            b.len()
        )
    }
}

// ---------------------------------------------------------------------------
// Forked execution
// ---------------------------------------------------------------------------

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn tmp_capture_path() -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{:?}_{}.out",
        std::process::id(),
        std::thread::current().id(),
        n
    ))
}

/// Run `calls` in order against `lib`, in a forked child whose fd 1 is a temp
/// file, and return the captured bytes plus the child's termination status.
pub fn run_batch(lib: &DriverLib, calls: &[Call]) -> Outcome {
    // Everything that allocates or touches the filesystem happens BEFORE the
    // fork; the child only performs async-signal-safe-ish work.
    let path = tmp_capture_path();
    let c_path = CString::new(path.as_os_str().as_encoded_bytes())
        .expect("temp path has no interior NUL");

    // NUL-terminated payloads for the `printLine` calls, pre-allocated.
    let payloads: Vec<Option<Vec<u8>>> = calls
        .iter()
        .map(|c| match c {
            Call::PrintLine(b) => {
                let mut v = Vec::with_capacity(b.len() + 1);
                v.extend_from_slice(b);
                v.push(0);
                Some(v)
            }
            _ => None,
        })
        .collect();

    // Drain both the Rust-level and the libc-level stdout buffers so the child
    // cannot inherit -- and then duplicate -- pending parent output.
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork() failed");

    if pid == 0 {
        // ---- child ----
        unsafe {
            // The negative-`data` rows in ERRORS.md deliberately provoke a
            // SIGSEGV. Without these two calls every such crash is handed to
            // systemd-coredump (see /proc/sys/kernel/core_pattern), costing
            // ~0.4 s each and dominating the suite's runtime; with them a crash
            // costs ~6 ms. Neither call changes the signal the parent observes
            // via WTERMSIG -- they only suppress the core dump.
            let no_core = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &no_core);
            libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);

            let fd = libc::open(
                c_path.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                0o600 as libc::c_int,
            );
            if fd < 0 {
                libc::_exit(101);
            }
            if libc::dup2(fd, 1) < 0 {
                libc::_exit(102);
            }
            if fd != 1 {
                libc::close(fd);
            }

            for (call, payload) in calls.iter().zip(payloads.iter()) {
                match call {
                    Call::Driver(d) => (lib.driver)(*d as c_int),
                    Call::PrintLine(_) => {
                        let p = payload.as_ref().unwrap();
                        (lib.print_line)(p.as_ptr() as *const c_char)
                    }
                    Call::PrintLineNull => (lib.print_line)(std::ptr::null()),
                }
            }

            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
    }

    // ---- parent ----
    let mut status: c_int = 0;
    let rc = unsafe { libc::waitpid(pid, &mut status, 0) };
    assert_eq!(rc, pid, "waitpid() failed");

    let stdout = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);

    let (exit, signal) = {
        if libc::WIFEXITED(status) {
            (Some(libc::WEXITSTATUS(status)), None)
        } else if libc::WIFSIGNALED(status) {
            (None, Some(libc::WTERMSIG(status)))
        } else {
            (None, None)
        }
    };

    // 101/102 are the harness's own plumbing failures, never library behaviour.
    if let Some(c) = exit {
        assert!(
            c != 101 && c != 102,
            "capture plumbing failed in child (exit {c}) for {}",
            lib.name
        );
    }

    Outcome {
        stdout,
        exit,
        signal,
    }
}

pub fn run_one(lib: &DriverLib, call: Call) -> Outcome {
    run_batch(lib, std::slice::from_ref(&call))
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// Run the same script against both `.so`s and assert the outcomes are
/// byte-for-byte and status-for-status identical. Returns the (shared) outcome.
pub fn assert_same(pair: &Pair, calls: &[Call], ctx: &str) -> Outcome {
    let c = run_batch(&pair.c, calls);
    let r = run_batch(&pair.rust, calls);

    assert_eq!(
        (c.exit, c.signal),
        (r.exit, r.signal),
        "termination status diverged [{ctx}]\n  calls: {}\n  C:    {:?}\n  Rust: {:?}",
        describe(calls),
        c,
        r
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout diverged [{ctx}]\n  calls: {}\n  C    ({} bytes): {}\n  Rust ({} bytes): {}",
        describe(calls),
        c.stdout.len(),
        render(&c.stdout),
        r.stdout.len(),
        render(&r.stdout)
    );
    c
}

/// `assert_same`, plus an assertion that the shared outcome is a clean exit
/// with exactly `expected` on stdout. Pins the *absolute* behaviour so a
/// mutually-broken pair cannot pass silently.
pub fn assert_same_and_output(pair: &Pair, calls: &[Call], expected: &[u8], ctx: &str) {
    let out = assert_same(pair, calls, ctx);
    assert!(
        out.exited_ok(),
        "expected clean exit [{ctx}], got {out:?} for {}",
        describe(calls)
    );
    assert_eq!(
        out.stdout,
        expected,
        "absolute output mismatch [{ctx}] for {}\n  actual   ({} bytes): {}\n  expected ({} bytes): {}",
        describe(calls),
        out.stdout.len(),
        render(&out.stdout),
        expected.len(),
        render(expected)
    );
}

/// `assert_same`, plus an assertion that both were killed by `signum`.
pub fn assert_same_and_signal(pair: &Pair, calls: &[Call], signum: i32, ctx: &str) {
    let out = assert_same(pair, calls, ctx);
    assert_eq!(
        out.signal,
        Some(signum),
        "expected both to die from signal {signum} [{ctx}], got {out:?} for {}",
        describe(calls)
    );
}

pub fn describe(calls: &[Call]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in calls.iter().take(8) {
        parts.push(match c {
            Call::Driver(d) => format!("driver({d})"),
            Call::PrintLine(b) => format!("printLine({})", render(b)),
            Call::PrintLineNull => "printLine(NULL)".to_string(),
        });
    }
    if calls.len() > 8 {
        parts.push(format!("... +{} more", calls.len() - 8));
    }
    format!("[{}]", parts.join(", "))
}

// ---------------------------------------------------------------------------
// Oracles transcribed from the C source (used only for the *absolute* checks;
// the primary assertion is always C-vs-Rust)
// ---------------------------------------------------------------------------

/// Expected stdout of `driver(data)`, per `c_src/src/driver.c`:
/// `source` is 99 `'A'` + NUL; `dest` starts empty; when `data < 100` the first
/// `data` bytes are copied and `dest[data]` is NUL'd; then the line is printed.
/// Only defined for the non-crashing window `0 <= data`.
pub fn oracle_driver(data: i32) -> Vec<u8> {
    assert!(data >= 0, "oracle is undefined for negative data (UB in C)");
    let mut v = Vec::new();
    if data < 100 {
        v.extend(std::iter::repeat(b'A').take(data as usize));
    }
    v.push(b'\n');
    v
}

/// Expected stdout of `printLine(line)` for a non-NULL `line`.
pub fn oracle_print_line(line: &[u8]) -> Vec<u8> {
    let mut v = line.to_vec();
    v.push(b'\n');
    v
}
