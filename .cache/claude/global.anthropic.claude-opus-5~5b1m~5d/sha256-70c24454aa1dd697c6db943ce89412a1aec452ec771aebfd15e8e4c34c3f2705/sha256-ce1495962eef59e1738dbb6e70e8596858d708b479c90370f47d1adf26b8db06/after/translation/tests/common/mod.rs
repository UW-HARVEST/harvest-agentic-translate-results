//! Shared differential-testing harness.
//!
//! Both the C `.so` (built by `c_src/CMakeLists.txt`) and the Rust `.so`
//! (`crate-type = ["cdylib"]`) are loaded with `libloading` and driven through
//! their exported symbols only. No Rust function is ever called directly, so
//! the `#[no_mangle]`/`extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::{CString, c_int};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type StaticAliasFn = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

pub struct Lib {
    pub name: &'static str,
    pub static_alias: StaticAliasFn,
    pub driver: DriverFn,
    /// Stable address of the library's private `static int inner`.
    pub inner_addr: *mut c_int,
    /// The value `inner` had when the library was first loaded, i.e. the
    /// initialiser `static int inner = 1;` as it lives in the `.so`'s data
    /// segment. Captured once at load time so it stays observable no matter
    /// which test runs first.
    pub inner_at_load: c_int,
    _lib: libloading::Library,
}

/// The dlopen handle, the extracted function pointers and the address of the
/// library's private static are all plain data that stays valid for the whole
/// process lifetime; access is serialised by [`lock`].
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

pub struct Libs {
    pub c: Lib,
    pub rust: Lib,
}

static LIBS: OnceLock<Libs> = OnceLock::new();
/// The libraries hold *mutable global state* (`static int inner`) and the tests
/// redirect the process-wide stdout, so every test body takes this lock.
static LOCK: Mutex<()> = Mutex::new(());

pub fn lock() -> MutexGuard<'static, ()> {
    match LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("STATICALIAS_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libStaticAlias.so")
}

/// The Rust `.so` that belongs to the *currently running* test profile
/// (`target/debug/libStaticAlias.so` or `target/release/libStaticAlias.so`),
/// derived from the test executable's own location.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("STATICALIAS_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>/
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("test binary lives in target/<profile>/deps");
    let candidate = profile_dir.join("libStaticAlias.so");
    if candidate.exists() {
        return candidate;
    }
    for p in ["target/release", "target/debug"] {
        let c = manifest_dir().join(p).join("libStaticAlias.so");
        if c.exists() {
            return c;
        }
    }
    candidate
}

unsafe fn open(name: &'static str, path: &PathBuf) -> Lib {
    let lib = unsafe { libloading::Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {name} .so at {}: {e}", path.display()));
    let static_alias: StaticAliasFn = unsafe {
        *lib.get::<StaticAliasFn>(b"static_alias\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol `static_alias`: {e}"))
    };
    let driver: DriverFn = unsafe {
        *lib.get::<DriverFn>(b"driver\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol `driver`: {e}"))
    };
    // Discover the address of the private `static int inner` through the public
    // API: `*outer >= inner` holds for `*outer == INT_MAX` whatever `inner` is,
    // so the `if` arm is taken and `&inner` is returned. That call also does
    // `inner += INT_MAX`, which the caller below undoes.
    let mut probe: c_int = c_int::MAX;
    let inner_addr = unsafe { static_alias(&mut probe) };
    assert!(!inner_addr.is_null(), "{name}: static_alias returned NULL");
    assert!(
        inner_addr != &mut probe as *mut c_int,
        "{name}: expected the `if` arm to return &inner, not the caller's pointer"
    );
    assert_eq!(
        probe,
        c_int::MAX,
        "{name}: the `if` arm must not modify *outer"
    );
    // Undo the probe's `inner += INT_MAX` to recover -- and record -- the value
    // the library was loaded with.
    let inner_at_load = unsafe {
        *inner_addr = (*inner_addr).wrapping_sub(c_int::MAX);
        *inner_addr
    };
    Lib {
        name,
        static_alias,
        driver,
        inner_addr,
        inner_at_load,
        _lib: lib,
    }
}

/// Guard against testing a stale shared object: `cargo test` does not always
/// relink the `cdylib`, and silently comparing an out-of-date `.so` would make
/// the whole differential suite meaningless.
fn assert_fresher_than(artifact: &PathBuf, source: &PathBuf) {
    let (Ok(a), Ok(s)) = (artifact.metadata(), source.metadata()) else {
        return;
    };
    let (Ok(a), Ok(s)) = (a.modified(), s.modified()) else {
        return;
    };
    assert!(
        a >= s,
        "{} is OLDER than {} -- rebuild before testing (`cargo build` / rebuild the C library)",
        artifact.display(),
        source.display()
    );
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {} -- build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not found at {} -- build it with `cargo build`",
            rust_path.display()
        );
        assert_fresher_than(&rust_so_path(), &manifest_dir().join("src/lib.rs"));
        assert_fresher_than(&c_so_path(), &manifest_dir().parent().unwrap().join("c_src/src/staticalias.c"));
        let c = unsafe { open("C", &c_path) };
        let rust = unsafe { open("Rust", &rust_path) };
        // The two libraries must be distinct objects with distinct state.
        assert_ne!(
            c.inner_addr, rust.inner_addr,
            "C and Rust `inner` must be separate objects"
        );
        Libs { c, rust }
    })
}

/// Freshly-loaded value of the C `static int inner = 1;`
pub const INNER_INITIAL: c_int = 1;

pub fn set_inner(l: &Lib, v: c_int) {
    unsafe { *l.inner_addr = v };
}

pub fn get_inner(l: &Lib) -> c_int {
    unsafe { *l.inner_addr }
}

/// Everything a single `static_alias` call can be observed to do.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AliasObs {
    /// The returned pointer is the library's private `&inner`.
    pub ret_is_inner: bool,
    /// The returned pointer is exactly the pointer that was passed in.
    pub ret_is_outer: bool,
    /// Value behind the returned pointer.
    pub ret_val: c_int,
    /// The caller's variable after the call.
    pub outer_after: c_int,
    /// The library's `inner` after the call.
    pub inner_after: c_int,
}

/// `static_alias(&outer)` with `inner` preset, observing every visible effect.
pub fn call_alias(l: &Lib, inner_init: c_int, outer_init: c_int) -> AliasObs {
    set_inner(l, inner_init);
    let mut outer: c_int = outer_init;
    let outer_ptr: *mut c_int = &mut outer;
    let ret = unsafe { (l.static_alias)(outer_ptr) };
    assert!(!ret.is_null(), "{}: static_alias returned NULL", l.name);
    AliasObs {
        ret_is_inner: ret == l.inner_addr,
        ret_is_outer: ret == outer_ptr,
        ret_val: unsafe { *ret },
        outer_after: outer,
        inner_after: get_inner(l),
    }
}

/// Feed the returned pointer back in, exactly like `driver` does, `steps` times.
/// Records the observation of every step.
pub fn call_alias_chain(l: &Lib, inner_init: c_int, outer_init: c_int, steps: usize) -> Vec<AliasObs> {
    set_inner(l, inner_init);
    let mut outer: c_int = outer_init;
    let outer_ptr: *mut c_int = &mut outer;
    let mut cur: *mut c_int = outer_ptr;
    let mut out = Vec::with_capacity(steps);
    for _ in 0..steps {
        let ret = unsafe { (l.static_alias)(cur) };
        assert!(!ret.is_null(), "{}: static_alias returned NULL", l.name);
        out.push(AliasObs {
            ret_is_inner: ret == l.inner_addr,
            ret_is_outer: ret == cur,
            ret_val: unsafe { *ret },
            outer_after: outer,
            inner_after: get_inner(l),
        });
        cur = ret;
    }
    out
}

/// Everything a single `driver` call can be observed to do.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DriverObs {
    /// Exact bytes `printf` emitted.
    pub stdout: Vec<u8>,
    /// The library's `inner` afterwards.
    pub inner_after: c_int,
    /// The caller's `initial_value` variable afterwards (by-value parameter, so
    /// it must be untouched).
    pub caller_arg_after: c_int,
}

pub fn call_driver(l: &Lib, inner_init: c_int, initial_value: c_int, iterations: c_int) -> DriverObs {
    set_inner(l, inner_init);
    let caller_arg: c_int = initial_value;
    let stdout = capture_stdout(l.name, || unsafe { (l.driver)(caller_arg, iterations) });
    DriverObs {
        stdout,
        inner_after: get_inner(l),
        caller_arg_after: caller_arg,
    }
}

/// Redirect file descriptor 1 to a temporary file for the duration of `f` and
/// return the bytes written. Both libraries `printf` to the process-wide libc
/// `stdout`, so this captures either one identically.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let data = capture_stdout_raw(tag, f);
    check_not_polluted(&data);
    data
}

/// Like [`capture_stdout`] but without the "only digits and newlines" sanity
/// check, for tests that deliberately mix in their own output.
pub fn capture_stdout_raw<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    use std::io::Write;
    unsafe {
        // Flush anything already buffered so it is not attributed to `f`.
        // Both buffers matter: libc's `stdout` FILE (what the libraries under
        // test write through) and Rust's own `Stdout` LineWriter (what the test
        // harness writes through -- a half-finished "test foo ... " line would
        // otherwise be flushed into our capture file).
        let _ = std::io::stdout().flush();
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let mut path = std::env::temp_dir();
        path.push(format!(
            "staticalias_cap_{}_{}_{:p}.txt",
            tag,
            std::process::id(),
            &saved as *const _
        ));
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

        f();

        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
        libc::close(fd);

        let data = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        data
    }
}

/// Write `s` through **libc** `printf`, i.e. into the very same `FILE *stdout`
/// buffer the libraries under test use. Deliberately not flushed.
pub fn libc_print(s: &str) {
    let c = CString::new(s).unwrap();
    let fmt = CString::new("%s").unwrap();
    unsafe {
        libc::printf(fmt.as_ptr(), c.as_ptr());
    }
}

/// `driver` only ever emits `printf("%d\n", ...)`, so a captured buffer that
/// contains anything other than digits, `-` and `\n` means some *other* writer
/// (the test harness's own progress output from a concurrent test thread) got
/// into the capture window. Fail loudly instead of reporting a bogus divergence.
fn check_not_polluted(data: &[u8]) {
    if let Some(&bad) = data
        .iter()
        .find(|b| !(b.is_ascii_digit() || **b == b'-' || **b == b'\n'))
    {
        panic!(
            "stdout capture was polluted by a foreign writer (unexpected byte {:?}). \
             These tests must run single-threaded; use RUST_TEST_THREADS=1 or \
             `cargo test -- --test-threads=1`.\ncaptured: {}",
            bad as char,
            preview(data)
        );
    }
}

/// How a forked child terminated. Used for the crashing error paths, where the
/// requirement is not "both failed somehow" but "both died from the same
/// signal".
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Exit {
    Code(i32),
    Signal(i32),
}

/// Run `f` in a forked child and report exactly how the child terminated.
pub fn run_in_child<F: FnOnce()>(f: F) -> Exit {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            libc::_exit(0);
        }
        let mut status: c_int = 0;
        let r = libc::waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Exit::Signal(libc::WTERMSIG(status))
        } else {
            Exit::Code(libc::WEXITSTATUS(status))
        }
    }
}

/// Deterministic SplitMix64 so every randomized row is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x5A71C_A11A5;

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
    /// Uniform over the whole `int` range.
    pub fn int(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
    /// Uniform in `[lo, hi]` (inclusive), computed in `i64` to avoid overflow.
    pub fn int_in(&mut self, lo: c_int, hi: c_int) -> c_int {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as c_int
    }
    /// A mix of small magnitudes and full-range values, plus the interesting
    /// boundaries -- a single uniform `int` almost never lands on a boundary.
    pub fn int_biased(&mut self) -> c_int {
        match self.next_u64() % 8 {
            0 => *pick(&BOUNDARIES, self.next_u64() as usize),
            1 => self.int_in(-4, 4),
            2 => self.int_in(-1000, 1000),
            3 => self.int_in(c_int::MAX - 32, c_int::MAX),
            4 => self.int_in(c_int::MIN, c_int::MIN + 32),
            _ => self.int(),
        }
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

fn pick<T>(xs: &[T], i: usize) -> &T {
    &xs[i % xs.len()]
}

/// The values the signed comparison / overflow behaviour actually hinges on.
pub const BOUNDARIES: [c_int; 8] = [
    c_int::MIN,
    c_int::MIN + 1,
    -1,
    0,
    1,
    2,
    c_int::MAX - 1,
    c_int::MAX,
];

/// Assert that C and Rust agree for one `static_alias` configuration.
#[track_caller]
pub fn assert_alias_eq(row: &str, inner_init: c_int, outer_init: c_int) -> AliasObs {
    let l = libs();
    let got_c = call_alias(&l.c, inner_init, outer_init);
    let got_rust = call_alias(&l.rust, inner_init, outer_init);
    assert_eq!(
        got_c, got_rust,
        "[{row}] static_alias divergence for inner={inner_init}, *outer={outer_init}\n  C   : {got_c:?}\n  Rust: {got_rust:?}"
    );
    got_c
}

/// Assert that C and Rust agree for one `static_alias` feed-back chain.
#[track_caller]
pub fn assert_chain_eq(row: &str, inner_init: c_int, outer_init: c_int, steps: usize) {
    let l = libs();
    let got_c = call_alias_chain(&l.c, inner_init, outer_init, steps);
    let got_rust = call_alias_chain(&l.rust, inner_init, outer_init, steps);
    assert_eq!(
        got_c.len(),
        got_rust.len(),
        "[{row}] chain length differs"
    );
    for (i, (a, b)) in got_c.iter().zip(got_rust.iter()).enumerate() {
        assert_eq!(
            a, b,
            "[{row}] chain divergence at step {i} for inner={inner_init}, *outer={outer_init}, steps={steps}\n  C   : {a:?}\n  Rust: {b:?}"
        );
    }
}

/// Assert that C and Rust agree for one `driver` configuration, comparing the
/// emitted stdout byte-for-byte.
#[track_caller]
pub fn assert_driver_eq(
    row: &str,
    inner_init: c_int,
    initial_value: c_int,
    iterations: c_int,
) -> DriverObs {
    let l = libs();
    let got_c = call_driver(&l.c, inner_init, initial_value, iterations);
    let got_rust = call_driver(&l.rust, inner_init, initial_value, iterations);
    assert_eq!(
        got_c.inner_after, got_rust.inner_after,
        "[{row}] driver `inner` divergence for inner={inner_init}, initial_value={initial_value}, iterations={iterations}: C={} Rust={}",
        got_c.inner_after, got_rust.inner_after
    );
    assert_eq!(
        got_c.caller_arg_after, got_rust.caller_arg_after,
        "[{row}] driver caller-argument divergence for inner={inner_init}, initial_value={initial_value}, iterations={iterations}"
    );
    if got_c.stdout != got_rust.stdout {
        panic!(
            "[{row}] driver stdout divergence for inner={inner_init}, initial_value={initial_value}, iterations={iterations}\n  C   ({} bytes): {}\n  Rust({} bytes): {}\n  first difference at byte {}",
            got_c.stdout.len(),
            preview(&got_c.stdout),
            got_rust.stdout.len(),
            preview(&got_rust.stdout),
            first_diff(&got_c.stdout, &got_rust.stdout),
        );
    }
    got_c
}

pub fn preview(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(&bytes[..bytes.len().min(400)]);
    let mut s = s.replace('\n', "\\n");
    if bytes.len() > 400 {
        s.push_str(" ...");
    }
    s
}

pub fn first_diff(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()))
}
