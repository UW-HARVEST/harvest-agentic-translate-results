// Shared harness for the C-vs-Rust differential tests.
//
// Both implementations are loaded as shared objects through `libloading` and
// called only through their exported C symbols -- the Rust functions are never
// called directly, so the `#[no_mangle]`/`extern "C"` export wrappers are under
// test too.
//
// Every function in this library returns `void` and communicates purely by
// writing to libc's `stdout`. So "compare the outputs byte-for-byte" means
// capturing the library's writes around each call and diffing the raw bytes.
//
// Capture is done in a FORKED CHILD whose fd 1 is a temporary file. An earlier
// version redirected fd 1 in-process, but `cargo test` runs tests on parallel
// threads and libtest writes its own progress text ("test foo ... ok\n") to the
// same fd -- that text leaked into the captured bytes and produced spurious
// mismatches. A forked child's fd 1 is seen only by the call under test, so the
// capture is exactly the library's output and nothing else. Forking also gives
// each measurement a pristine copy of libc's stdio state, so C and Rust are
// compared under identical buffering, and it lets a crash be observed as an
// exit outcome instead of taking down the test process.

#![allow(dead_code)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;
use std::fmt;
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::os::unix::{Library, Symbol, RTLD_LOCAL, RTLD_NOW};

extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    /// `fflush(NULL)` flushes *all* open output streams, which lets us drain
    /// libc's `stdout` buffer without needing the `stdout` global itself.
    fn fflush(stream: *mut c_void) -> c_int;
    /// Used only to pre-allocate libc's stdout buffer in the parent (see
    /// `warm_up_stdio`), never to produce compared output.
    fn puts(s: *const c_char) -> c_int;
}

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Which implementation to drive
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Locating and opening the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_src_dir() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("crate root has a parent")
        .join("c_src")
}

/// `c_src/build/libdriver.so`, produced by the CMake build described in
/// `c_src/CMakeLists.txt`.
fn c_so_path() -> PathBuf {
    let p = c_src_dir().join("build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}\n\
         build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The Rust `cdylib` for the profile this test binary was built in.
///
/// Located relative to `current_exe()` (`target/<profile>/deps/<test binary>`),
/// never by scanning both profiles. Two places can hold the artifact:
///
///   * `target/<profile>/deps/libdriver.so` -- what cargo actually produces,
///     always rebuilt alongside the test binaries;
///   * `target/<profile>/libdriver.so`      -- the "uplifted" copy, which cargo
///     only refreshes for `cargo build`, NOT for `cargo test`.
///
/// We take the NEWEST of the two. This matters: an earlier version of this
/// harness looked only at the uplifted path and fell back across profiles, so
/// when `cargo test` left no uplifted copy it silently loaded a stale
/// `target/release/libdriver.so` -- and a deliberate mutation of `src/lib.rs`
/// went completely undetected. The freshness assertion below is the backstop.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps_dir = exe.parent().expect("target/<profile>/deps/<test binary>");
    let profile_dir = deps_dir.parent().expect("target/<profile>");

    let candidates = [deps_dir.join("libdriver.so"), profile_dir.join("libdriver.so")];

    let newest = candidates
        .iter()
        .filter(|p| p.is_file())
        .max_by_key(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
        .unwrap_or_else(|| {
            panic!(
                "Rust shared library not found in this profile.\n  looked at: {}\n\
                 build it with `cargo build` (matching profile)",
                candidates
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n             ")
            )
        })
        .clone();

    assert_fresh(&newest);
    newest
}

/// Guard against testing a `.so` that predates the current sources.
fn assert_fresh(so: &std::path::Path) {
    let so_mtime = match so.metadata().and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = manifest_dir().join(src);
        if let Ok(src_mtime) = p.metadata().and_then(|m| m.modified()) {
            assert!(
                so_mtime >= src_mtime,
                "{} is OLDER than {} -- the tests would be comparing against a \
                 stale build. Re-run `cargo build` for this profile.",
                so.display(),
                p.display()
            );
        }
    }
}

/// `RTLD_LOCAL` matters: both objects export `printLine`, `bad`, `good` and
/// `driver`. Loading them into local scopes keeps each library's *internal*
/// calls (e.g. C `driver` -> C `printLine`, which goes through the PLT and is
/// therefore interposable) bound to its own definitions, so we never
/// accidentally measure a hybrid of the two.
fn open_lib(path: &PathBuf) -> Library {
    unsafe { Library::open(Some(path), RTLD_NOW | RTLD_LOCAL) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
}

struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| Libs {
        c: open_lib(&c_so_path()),
        rust: open_lib(&rust_so_path()),
    })
}

fn lib(which: Impl) -> &'static Library {
    match which {
        Impl::C => &libs().c,
        Impl::Rust => &libs().rust,
    }
}

pub fn so_path(which: Impl) -> PathBuf {
    match which {
        Impl::C => c_so_path(),
        Impl::Rust => rust_so_path(),
    }
}

/// Look up a symbol, asserting it is actually exported by that `.so`.
fn sym<T>(which: Impl, name: &[u8]) -> Symbol<T> {
    unsafe { lib(which).get::<T>(name) }.unwrap_or_else(|e| {
        panic!(
            "symbol `{}` is not exported by the {} .so: {e}",
            String::from_utf8_lossy(name),
            which.name()
        )
    })
}

/// True if `name` is exported; used to assert that the `static` C helpers stay
/// unexported on both sides.
pub fn exports(which: Impl, name: &[u8]) -> bool {
    unsafe { lib(which).get::<*const c_void>(name) }.is_ok()
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

/// How the forked child terminated. Part of the differential comparison, so
/// that "C segfaults but Rust does not" is a detectable divergence rather than
/// an invisible one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Exited(c) => write!(f, "exited({c})"),
            Outcome::Signaled(s) => write!(f, "killed by signal {s}"),
        }
    }
}

/// The full observable result of one call: the bytes it wrote and how it ended.
#[derive(Clone, PartialEq, Eq)]
pub struct Capture {
    pub stdout: Vec<u8>,
    pub outcome: Outcome,
}

impl Capture {
    /// Convenience for the common "it exited cleanly, give me the bytes" case.
    pub fn ok_stdout(&self) -> &[u8] {
        assert_eq!(
            self.outcome,
            Outcome::Exited(0),
            "call did not terminate cleanly: {}",
            self.outcome
        );
        &self.stdout
    }
}

/// Pre-allocate libc's stdout buffer *in the parent*, so the forked child never
/// has to `malloc` inside `puts`. Forking a multi-threaded process and then
/// allocating can deadlock if another thread held the allocator lock at the
/// instant of the fork; doing the one allocation up front removes that hazard.
/// Also makes stdio's buffering decision identical for every child, so C and
/// Rust are always measured under the same discipline.
fn warm_up_stdio() {
    static WARM: OnceLock<()> = OnceLock::new();
    WARM.get_or_init(|| {
        const O_WRONLY: c_int = 1;
        unsafe {
            let _ = std::io::stdout().flush();
            fflush(std::ptr::null_mut());

            let devnull = open(c"/dev/null".as_ptr(), O_WRONLY);
            if devnull >= 0 {
                let saved = dup2_saving(1);
                dup2(devnull, 1);
                puts(c"".as_ptr()); // forces _IO_file_doallocate for stdout
                fflush(std::ptr::null_mut());
                if saved >= 0 {
                    dup2(saved, 1);
                    close(saved);
                }
                close(devnull);
            }
        }
    });
}

unsafe fn dup2_saving(fd: c_int) -> c_int {
    extern "C" {
        fn dup(oldfd: c_int) -> c_int;
    }
    dup(fd)
}

/// Run `f` in a forked child with fd 1 pointing at a temporary file, and return
/// everything the child wrote plus how it terminated.
///
/// `f` must not allocate or take locks: prepare all buffers *before* calling.
pub fn capture<F: FnOnce()>(f: F) -> Capture {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    warm_up_stdio();

    let path = std::env::temp_dir().join(format!(
        "driver-difftest-{}-{}.bin",
        std::process::id(),
        TMP_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let file =
        fs::File::create(&path).unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
    let out_fd = file.as_raw_fd();

    // Drain both buffering layers so nothing pending is inherited and then
    // duplicated by the child.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");

    if pid == 0 {
        // ---- child ----------------------------------------------------
        // Only `_exit` from here on: running libtest's or Rust's atexit
        // machinery in the child would corrupt the parent's test output.
        unsafe {
            if dup2(out_fd, 1) < 0 {
                _exit(91);
            }
        }
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        unsafe {
            fflush(std::ptr::null_mut());
            _exit(if r.is_ok() { 0 } else { 92 });
        }
    }

    // ---- parent -------------------------------------------------------
    let mut status: c_int = 0;
    loop {
        let w = unsafe { waitpid(pid, &mut status, 0) };
        if w == pid {
            break;
        }
        assert!(w >= 0, "waitpid({pid}) failed");
    }

    let outcome = if status & 0x7f == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else {
        Outcome::Signaled(status & 0x7f)
    };

    drop(file);
    let stdout =
        fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let _ = fs::remove_file(&path);

    assert_ne!(
        outcome,
        Outcome::Exited(91),
        "capture child failed to redirect fd 1"
    );

    Capture { stdout, outcome }
}

// ---------------------------------------------------------------------------
// The four exported entry points, invoked purely via the .so exports
// ---------------------------------------------------------------------------

type VoidFn = unsafe extern "C" fn();
type PrintLineFn = unsafe extern "C" fn(*const c_char);

/// Call `printLine(ptr)` on the chosen implementation.
///
/// `ptr` is passed through completely untouched, so callers can hand over a
/// null pointer, an interior pointer, or a non-UTF-8 buffer.
pub fn print_line_raw(which: Impl, ptr: *const c_char) -> Capture {
    let f: Symbol<PrintLineFn> = sym(which, b"printLine");
    capture(|| unsafe { f(ptr) })
}

/// `printLine` on a NUL-terminated copy of `bytes`.
///
/// # Panics
/// If `bytes` contains an interior NUL (that would not be the same input).
pub fn print_line(which: Impl, bytes: &[u8]) -> Capture {
    assert!(
        !bytes.contains(&0),
        "print_line() takes the string content, without any NUL"
    );
    let mut buf = Vec::with_capacity(bytes.len() + 1);
    buf.extend_from_slice(bytes);
    buf.push(0);
    print_line_raw(which, buf.as_ptr() as *const c_char)
}

pub fn call_void(which: Impl, name: &[u8]) -> Capture {
    let f: Symbol<VoidFn> = sym(which, name);
    capture(|| unsafe { f() })
}

pub fn bad(which: Impl) -> Capture {
    call_void(which, b"bad")
}

pub fn good(which: Impl) -> Capture {
    call_void(which, b"good")
}

pub fn driver(which: Impl) -> Capture {
    call_void(which, b"driver")
}

/// Call a zero-argument entry point through a signature that passes THREE junk
/// arguments anyway.
///
/// The C ABI lets a caller pass extra arguments to a function that ignores
/// them, so these are real inputs reaching the FFI boundary. Values are chosen
/// to be nonsense for any hypothetical enum/int parameter.
pub fn call_void_with_extra_args(which: Impl, name: &[u8]) -> Capture {
    type ExtraArgsFn = unsafe extern "C" fn(c_int, c_int, u64);
    let f: Symbol<ExtraArgsFn> = sym(which, name);
    capture(|| unsafe { f(-1, c_int::MIN, u64::MAX) })
}

/// One step of a composed call sequence (Phase B row 9).
#[derive(Clone, Debug)]
pub enum Step {
    PrintLine(Vec<u8>),
    PrintLineNull,
    Bad,
    Good,
    Driver,
}

/// Run a whole sequence of calls against one implementation inside a *single*
/// capture window, so the result reflects the accumulated stdout stream
/// including any flush-ordering effects between calls.
pub fn run_sequence(which: Impl, steps: &[Step]) -> Capture {
    let print_line_f: Symbol<PrintLineFn> = sym(which, b"printLine");
    let bad_f: Symbol<VoidFn> = sym(which, b"bad");
    let good_f: Symbol<VoidFn> = sym(which, b"good");
    let driver_f: Symbol<VoidFn> = sym(which, b"driver");

    // Materialise the NUL-terminated buffers up front: the child must not
    // allocate, and the pointers must stay valid for the whole window.
    let bufs: Vec<Option<Vec<u8>>> = steps
        .iter()
        .map(|s| match s {
            Step::PrintLine(b) => {
                let mut v = b.clone();
                v.push(0);
                Some(v)
            }
            _ => None,
        })
        .collect();

    capture(|| unsafe {
        for (step, buf) in steps.iter().zip(bufs.iter()) {
            match step {
                Step::PrintLine(_) => {
                    print_line_f(buf.as_ref().unwrap().as_ptr() as *const c_char)
                }
                Step::PrintLineNull => print_line_f(std::ptr::null()),
                Step::Bad => bad_f(),
                Step::Good => good_f(),
                Step::Driver => driver_f(),
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Differential assertion
// ---------------------------------------------------------------------------

fn render(bytes: &[u8]) -> String {
    if bytes.len() > 300 {
        format!(
            "{} bytes, starts {:?}, ends {:?}",
            bytes.len(),
            String::from_utf8_lossy(&bytes[..120]),
            String::from_utf8_lossy(&bytes[bytes.len() - 120..])
        )
    } else {
        format!("{} bytes: {:?}", bytes.len(), String::from_utf8_lossy(bytes))
    }
}

/// Assert the two captures are identical in BOTH written bytes and termination
/// outcome, reporting the first differing offset when they are not.
#[track_caller]
pub fn assert_same(context: &str, c: &Capture, rust: &Capture) {
    if c.outcome != rust.outcome {
        panic!(
            "C and Rust terminated differently for {context}\n\
             C    -> {}\n\
             Rust -> {}",
            c.outcome, rust.outcome
        );
    }
    if c.stdout == rust.stdout {
        return;
    }
    let first_diff = c
        .stdout
        .iter()
        .zip(rust.stdout.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| c.stdout.len().min(rust.stdout.len()));
    panic!(
        "C and Rust stdout differ for {context}\n\
         first difference at byte offset {first_diff}\n\
         C    -> {}\n\
         Rust -> {}",
        render(&c.stdout),
        render(&rust.stdout)
    );
}

/// `printLine` on both implementations, asserting equality.
#[track_caller]
pub fn diff_print_line(context: &str, bytes: &[u8]) {
    let c = print_line(Impl::C, bytes);
    let rust = print_line(Impl::Rust, bytes);
    assert_same(context, &c, &rust);
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) -- fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

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

    /// Uniform-ish in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }

    /// A byte in `1..=255` -- never 0, since a NUL would terminate the string
    /// early and so would not be the input we think we are testing.
    pub fn nonzero_byte(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }

    pub fn printable_byte(&mut self) -> u8 {
        0x20 + (self.next_u64() % (0x7f - 0x20)) as u8
    }

    pub fn bytes_nonzero(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }

    pub fn bytes_printable(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.printable_byte()).collect()
    }

    pub fn pick<'a, T>(&mut self, choices: &'a [T]) -> &'a T {
        &choices[self.range(0, choices.len() - 1)]
    }
}
