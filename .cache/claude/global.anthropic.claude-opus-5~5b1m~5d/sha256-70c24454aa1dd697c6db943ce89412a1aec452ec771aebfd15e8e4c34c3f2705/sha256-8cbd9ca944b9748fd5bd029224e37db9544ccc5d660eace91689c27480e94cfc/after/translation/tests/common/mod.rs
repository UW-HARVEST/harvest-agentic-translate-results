//! Shared differential-test harness.
//!
//! Both the C `libStaticAlias.so` and the Rust `libStaticAlias.so` are loaded
//! with `libloading` and driven **only** through their exported C symbols, so
//! the `#[no_mangle] extern "C"` wrappers are part of what is under test.
//!
//! `static_alias` owns a function-local `static int inner = 1;`. That state is
//! per-mapping, and `dlopen` of the same path returns the same refcounted
//! handle, so a test that needs a virgin `inner` must load a *distinct file*.
//! `Pair::fresh()` therefore copies both shared objects to unique temporary
//! paths before loading them.

#![allow(dead_code)]

use std::ffi::{CString, OsStr};
use std::io::Write as _;
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits used by the harness itself (never by the code under test).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn open(path: *const i8, flags: i32, ...) -> i32;
    fn fflush(stream: *mut core::ffi::c_void) -> i32;
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

const O_WRONLY: i32 = 0o1;
const O_CREAT: i32 = 0o100;
const O_TRUNC: i32 = 0o1000;

// ---------------------------------------------------------------------------
// Function signatures of the library under test.
// ---------------------------------------------------------------------------

pub type StaticAliasFn = unsafe extern "C" fn(*mut i32) -> *mut i32;
pub type DriverFn = unsafe extern "C" fn(i32, i32);

// ---------------------------------------------------------------------------
// Locating the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C shared object built by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libStaticAlias.so");
    assert!(
        p.is_file(),
        "C shared library not found at {p:?}; build it with\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// The Rust `cdylib`. The release artifact is preferred (it is what ships);
/// the debug artifact is accepted as a fallback.
pub fn rust_so_path() -> PathBuf {
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libStaticAlias.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "Rust shared library not found under {base:?}; build it with\n  \
         cd translation && cargo build --release"
    );
}

fn unique_tmp_dir() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "staticalias-diff-{}-{}-{}",
        std::process::id(),
        nanos,
        n
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

// ---------------------------------------------------------------------------
// Observations
// ---------------------------------------------------------------------------

/// Which pointer `static_alias` handed back. The absolute addresses of the two
/// libraries' statics necessarily differ, so the comparable observable is the
/// pointer's *identity class*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetClass {
    /// The very pointer that was passed in (`return outer;`).
    Arg,
    /// A pointer into the library itself (`return &inner;`).
    LibStatic,
}

/// Everything one `static_alias` call can be observed to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Obs {
    pub class: RetClass,
    /// `*ret` after the call.
    pub ret_val: i32,
    /// `*outer` after the call (the caller's cell, whether or not it was the
    /// one that got mutated).
    pub outer_after: i32,
}

/// One loaded library, plus its resolved symbols.
pub struct Lib {
    _lib: Library,
    path: PathBuf,
    pub static_alias: StaticAliasFn,
    pub driver: DriverFn,
}

impl Lib {
    fn load(path: PathBuf) -> Lib {
        // SAFETY: loading a plain C shared object with no initialisers of ours.
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {path:?} failed: {e}"));
        let static_alias: StaticAliasFn = unsafe {
            let s: Symbol<StaticAliasFn> = lib
                .get(b"static_alias\0")
                .unwrap_or_else(|e| panic!("{path:?} does not export `static_alias`: {e}"));
            *s
        };
        let driver: DriverFn = unsafe {
            let s: Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{path:?} does not export `driver`: {e}"));
            *s
        };
        Lib {
            _lib: lib,
            path,
            static_alias,
            driver,
        }
    }

    /// Call `static_alias(&mut cell)` and record all three observables.
    pub fn call(&self, cell: &mut i32) -> Obs {
        let arg: *mut i32 = cell as *mut i32;
        // SAFETY: `arg` points to a live, aligned, initialised `i32`.
        let ret = unsafe { (self.static_alias)(arg) };
        assert!(!ret.is_null(), "{:?}: static_alias returned NULL", self.path);
        let class = if ret == arg {
            RetClass::Arg
        } else {
            RetClass::LibStatic
        };
        // SAFETY: the C contract guarantees `ret` is either `arg` or `&inner`,
        // both live for the lifetime of the library.
        let ret_val = unsafe { *ret };
        Obs {
            class,
            ret_val,
            outer_after: *cell,
        }
    }

    /// Call `static_alias` with a raw pointer (used for the self-alias shape,
    /// where the argument is the library's own static).
    pub fn call_raw(&self, arg: *mut i32) -> (RetClass, i32, *mut i32) {
        // SAFETY: caller guarantees `arg` is a pointer previously produced by
        // this same library (or a pointer to a live local).
        let ret = unsafe { (self.static_alias)(arg) };
        assert!(!ret.is_null(), "{:?}: static_alias returned NULL", self.path);
        let class = if ret == arg {
            RetClass::Arg
        } else {
            RetClass::LibStatic
        };
        (class, unsafe { *ret }, ret)
    }

    /// Run `driver` with fd 1 redirected to a scratch file and return the exact
    /// bytes it printed.
    pub fn driver_stdout(&self, initial_value: i32, iterations: i32) -> Vec<u8> {
        capture_fd1(|| {
            // SAFETY: plain scalar arguments.
            unsafe { (self.driver)(initial_value, iterations) };
        })
    }
}

/// A C library and a Rust library, both freshly mapped so their `inner`
/// statics both start at the initial value `1`.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
    dir: PathBuf,
}

impl Pair {
    /// Copy both shared objects to unique paths and `dlopen` the copies, so the
    /// function-local `static int inner` starts at `1` in both.
    pub fn fresh() -> Pair {
        let dir = unique_tmp_dir();
        let c_dst = dir.join("libC_StaticAlias.so");
        let r_dst = dir.join("libR_StaticAlias.so");
        std::fs::copy(c_so_path(), &c_dst).expect("copy C .so");
        std::fs::copy(rust_so_path(), &r_dst).expect("copy Rust .so");
        Pair {
            c: Lib::load(c_dst),
            rust: Lib::load(r_dst),
            dir,
        }
    }

    /// Differential `static_alias`: call both with the same `*outer` value and
    /// assert the full observation matches.
    #[track_caller]
    pub fn assert_static_alias(&self, value: i32, ctx: &str) -> Obs {
        let mut cc = value;
        let mut rc = value;
        let oc = self.c.call(&mut cc);
        let or = self.rust.call(&mut rc);
        assert_eq!(
            oc, or,
            "static_alias divergence [{ctx}] with *outer = {value} ({value:#010x})\n  \
             C   : {oc:?}\n  Rust: {or:?}"
        );
        oc
    }

    /// Differential `driver`: byte-for-byte stdout comparison.
    #[track_caller]
    pub fn assert_driver(&self, initial_value: i32, iterations: i32, ctx: &str) -> Vec<u8> {
        let c_out = self.c.driver_stdout(initial_value, iterations);
        let r_out = self.rust.driver_stdout(initial_value, iterations);
        assert_eq!(
            c_out,
            r_out,
            "driver stdout divergence [{ctx}] initial_value = {initial_value}, \
             iterations = {iterations}\n  C   ({} bytes): {:?}\n  Rust ({} bytes): {:?}",
            c_out.len(),
            String::from_utf8_lossy(&trunc(&c_out)),
            r_out.len(),
            String::from_utf8_lossy(&trunc(&r_out)),
        );
        c_out
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        // Best-effort cleanup; the libraries are dlclose'd by `Library::drop`
        // which runs before this.
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn trunc(v: &[u8]) -> Vec<u8> {
    v.iter().copied().take(400).collect()
}

// ---------------------------------------------------------------------------
// fd-1 capture (process-global, hence serialised)
// ---------------------------------------------------------------------------

fn fd1_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Redirect file descriptor 1 to a scratch file, run `f`, restore fd 1 and
/// return the bytes written.
///
/// Both libraries call the *same* glibc `printf` in this process, so the
/// stream's buffering mode is identical for both and the captured bytes are
/// directly comparable.
pub fn capture_fd1<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = fd1_lock();

    let dir = unique_tmp_dir();
    let out_path = dir.join("stdout.bin");
    let c_path = CString::new(out_path.as_os_str().as_bytes()).expect("path has no NUL");

    // Don't let anything already buffered land in our capture file.
    let _ = std::io::stdout().flush();
    // SAFETY: valid stream pointer (NULL = "all streams").
    unsafe { fflush(core::ptr::null_mut()) };

    // SAFETY: all raw fd juggling below uses fds we own.
    let bytes = unsafe {
        let fd = open(c_path.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644i32);
        assert!(fd >= 0, "open {out_path:?} failed");
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2(fd, 1) failed");

        f();

        // Force everything the library buffered out through fd 1 *before* the
        // descriptor is pointed back at the real stdout.
        fflush(core::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
        close(fd);

        std::fs::read(&out_path).expect("read captured stdout")
    };

    let _ = std::fs::remove_dir_all(&dir);

    // fd 1 is process-global: if libtest is running tests on several threads,
    // its own progress output ("test foo ... ok") lands in our capture file and
    // corrupts the comparison. `driver` can only ever emit `printf("%d\n", ..)`,
    // i.e. bytes from [0-9], '-' and '\n', so anything else is contamination.
    // Fail loudly rather than reporting a bogus C-vs-Rust divergence.
    if let Some(&bad) = bytes
        .iter()
        .find(|b| !(b.is_ascii_digit() || **b == b'-' || **b == b'\n'))
    {
        panic!(
            "captured stdout was contaminated by concurrent writers to fd 1 \
             (unexpected byte {bad:#04x} = {:?}).\n\
             These differential tests redirect fd 1 and must therefore run \
             serially:\n    cargo test --release -- --test-threads=1\n\
             (or use ./run_tests.sh)\ncaptured: {:?}",
            bad as char,
            String::from_utf8_lossy(&trunc(&bytes))
        );
    }

    bytes
}

// ---------------------------------------------------------------------------
// Fatal-signal probing (for the NULL-pointer row)
// ---------------------------------------------------------------------------

/// How a child process that called into the library terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Ran to completion and returned normally.
    Returned,
    /// Killed by this signal number.
    Signal(i32),
    /// Exited with this non-zero status.
    Exit(i32),
}

/// Run `f` in a forked child and report how the child terminated. Used to
/// compare *fatal* behaviour (e.g. the unguarded `*outer` on a NULL argument)
/// between the two libraries without taking the test process down.
pub fn outcome_in_child<F: FnOnce()>(f: F) -> Outcome {
    let _ = std::io::stdout().flush();
    // SAFETY: fflush before fork so buffered data is not duplicated.
    unsafe { fflush(core::ptr::null_mut()) };

    // SAFETY: the child does nothing but run `f` and `_exit`; it never returns
    // to the test harness or the Rust runtime, so async-signal-safety concerns
    // about forking a threaded process do not apply.
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status: i32 = 0;
        let r = waitpid(pid, &mut status as *mut i32, 0);
        assert_eq!(r, pid, "waitpid failed");
        decode_status(status)
    }
}

fn decode_status(status: i32) -> Outcome {
    // WIFSIGNALED / WTERMSIG / WIFEXITED / WEXITSTATUS, open-coded so the
    // harness needs no libc crate.
    let termsig = status & 0x7f;
    if termsig != 0 && termsig != 0x7f {
        return Outcome::Signal(termsig);
    }
    let code = (status >> 8) & 0xff;
    if code == 0 {
        Outcome::Returned
    } else {
        Outcome::Exit(code)
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) -- fixed seeds keep every row reproducible.
// ---------------------------------------------------------------------------

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

    /// Uniform over the whole `i32` domain.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `lo ..= hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// The `int`-domain corner values every row that says "extremes" uses.
pub const EXTREMES: [i32; 12] = [
    i32::MIN,
    i32::MIN + 1,
    i32::MIN / 2,
    -2,
    -1,
    0,
    1,
    2,
    3,
    i32::MAX / 2,
    i32::MAX - 1,
    i32::MAX,
];

/// Helper for tests that want the symbol lists of the two `.so` files.
pub fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args([OsStr::new("-D"), OsStr::new("--defined-only"), so.as_os_str()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .filter(|s| !s.is_empty())
        .collect();
    v.sort();
    v.dedup();
    v
}
