//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! invoked only through their exported `extern "C"` symbols, so the
//! `#[no_mangle]` export wrappers are part of what is under test. No Rust
//! function is ever called directly.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type FmaArrayFn = unsafe extern "C" fn(
    *mut c_int,
    *const c_int,
    *const c_int,
    *const c_int,
    c_int,
);
pub type DriverFn = unsafe extern "C" fn(*const c_int, c_int);

/// Which of the two shared objects to exercise.
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

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    crate_root()
        .parent()
        .expect("crate root has a parent")
        .join("c_src/build/libdriver.so")
}

/// The Rust `.so` is the `cdylib` this crate produces. `cargo test` builds it
/// as a dependency of the integration test but does not hand us its path, so we
/// take `DRIVER_RUST_SO` when set (used to pin a specific profile) and
/// otherwise pick the newest of the two profile outputs.
fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(
            p.exists(),
            "DRIVER_RUST_SO points at {} which does not exist",
            p.display()
        );
        return p;
    }
    let root = crate_root();
    let candidates = [
        root.join("target/release/libdriver.so"),
        root.join("target/debug/libdriver.so"),
    ];
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for c in candidates {
        if let Ok(md) = std::fs::metadata(&c) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map_or(true, |(_, bt)| t > *bt) {
                best = Some((c, t));
            }
        }
    }
    best.map(|(p, _)| p).unwrap_or_else(|| {
        panic!(
            "no Rust libdriver.so found under {}/target/{{release,debug}}; \
             run `cargo build --release` first",
            root.display()
        )
    })
}

fn load(path: &PathBuf) -> &'static Library {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    Box::leak(Box::new(lib))
}

fn c_lib() -> &'static Library {
    static L: OnceLock<&'static Library> = OnceLock::new();
    L.get_or_init(|| load(&c_so_path()))
}

fn rust_lib() -> &'static Library {
    static L: OnceLock<&'static Library> = OnceLock::new();
    L.get_or_init(|| load(&rust_so_path()))
}

fn lib(which: Impl) -> &'static Library {
    match which {
        Impl::C => c_lib(),
        Impl::Rust => rust_lib(),
    }
}

pub fn fma_array_of(which: Impl) -> FmaArrayFn {
    let l = lib(which);
    let s: Symbol<FmaArrayFn> = unsafe { l.get(b"fma_array\0") }
        .unwrap_or_else(|e| panic!("{} .so is missing `fma_array`: {e}", which.name()));
    *s
}

pub fn driver_of(which: Impl) -> DriverFn {
    let l = lib(which);
    let s: Symbol<DriverFn> = unsafe { l.get(b"driver\0") }
        .unwrap_or_else(|e| panic!("{} .so is missing `driver`: {e}", which.name()));
    *s
}

/// Force both shared objects to be resolved, so a missing symbol fails loudly
/// and early rather than inside a `fork()`ed child.
pub fn preload_both() {
    for w in [Impl::C, Impl::Rust] {
        let _ = fma_array_of(w);
        let _ = driver_of(w);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds, so every failure reproduces.
// ---------------------------------------------------------------------------

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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform over the whole `i32` range.
    pub fn full_i32(&mut self) -> c_int {
        self.next_u32() as i32
    }

    /// |v| <= 1000, so `m1 * m2 + a` cannot overflow.
    pub fn small_i32(&mut self) -> c_int {
        (self.next_u32() % 2001) as i32 - 1000
    }

    /// Interesting two's-complement boundary values.
    pub fn boundary_i32(&mut self) -> c_int {
        const POOL: [i32; 11] = [
            i32::MIN,
            i32::MIN + 1,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            i32::MAX - 1,
            i32::MAX,
        ];
        POOL[(self.next_u32() as usize) % POOL.len()]
    }
}

/// Which value distribution a configuration row uses.
#[derive(Copy, Clone, Debug)]
pub enum Dist {
    Small,
    Full,
    Boundary,
}

impl Dist {
    pub fn sample(self, rng: &mut Rng) -> c_int {
        match self {
            Dist::Small => rng.small_i32(),
            Dist::Full => rng.full_i32(),
            Dist::Boundary => rng.boundary_i32(),
        }
    }

    pub fn vec(self, rng: &mut Rng, n: usize) -> Vec<c_int> {
        (0..n).map(|_| self.sample(rng)).collect()
    }
}

// ---------------------------------------------------------------------------
// stdout capture — `driver` writes to stdout via libc `printf`, so the only
// way to diff it is to redirect fd 1 to a temp file around the call.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Serializes every fd-1 manipulation, since `cargo test` runs tests on
/// multiple threads and fd 1 is process-global.
fn stdout_lock() -> std::sync::MutexGuard<'static, ()> {
    static M: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    M.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Run `f` in a forked child whose fd 1 is a fresh temp file, and return the
/// bytes it wrote.
///
/// The call is isolated in a child process on purpose: `cargo test` runs tests
/// on several threads and its own harness writes progress text to fd 1, so
/// redirecting fd 1 in-process would splice unrelated harness output into the
/// captured bytes. In the child nothing else is running, so the capture
/// contains exactly what the library under test printed.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;

    let _guard = stdout_lock();

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_difftest_{}_{}.out",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture temp file");

    use std::os::fd::AsRawFd;
    let fd = file.as_raw_fd();

    // Drain anything libc has buffered so the child does not inherit and
    // re-emit it.
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            if dup2(fd, 1) < 0 {
                _exit(101);
            }
        }
        f();
        unsafe { fflush(std::ptr::null_mut()) };
        unsafe { _exit(0) };
    }

    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    let outcome = decode_status(status);
    assert_eq!(
        outcome,
        Outcome::Exited(0),
        "capture child terminated abnormally: {outcome}"
    );

    let mut buf = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut buf)
        .expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&path);
    buf
}

// ---------------------------------------------------------------------------
// fork-based comparison for inputs that legitimately fault (UB in the C).
// ---------------------------------------------------------------------------

/// How a child process ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited(c_int),
    Signaled(c_int),
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Exited(c) => write!(f, "exit({c})"),
            Outcome::Signaled(s) => write!(f, "signal({s})"),
        }
    }
}

/// Decode a `waitpid` status per `<bits/waitstatus.h>`.
fn decode_status(status: c_int) -> Outcome {
    if status & 0x7f == 0 {
        Outcome::Exited((status >> 8) & 0xff) // WIFEXITED / WEXITSTATUS
    } else {
        Outcome::Signaled(status & 0x7f) // WTERMSIG
    }
}

/// Run `f` in a forked child and report how the child terminated. The child
/// exits with code 0 if `f` returns normally. Used to compare the C and Rust
/// implementations on inputs that fault, so "same rejection" means "same
/// signal / same exit code", not merely "both broke somehow".
pub fn outcome_of<F: FnOnce()>(f: F) -> Outcome {
    let _guard = stdout_lock();
    unsafe { fflush(std::ptr::null_mut()) };
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // Child: silence stdout so faulting cases do not spam the test log,
        // run the call, then leave immediately without unwinding or running
        // atexit handlers.
        unsafe {
            if let Ok(devnull) = std::fs::OpenOptions::new().write(true).open("/dev/null") {
                use std::os::fd::AsRawFd;
                dup2(devnull.as_raw_fd(), 1);
            }
        }
        f();
        unsafe { fflush(std::ptr::null_mut()) };
        unsafe { _exit(0) };
    }
    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    decode_status(status)
}

/// Assert that the C and Rust implementations reject an input identically.
pub fn assert_same_outcome<FC, FR>(label: &str, c_call: FC, rust_call: FR)
where
    FC: FnOnce(),
    FR: FnOnce(),
{
    let c = outcome_of(c_call);
    let r = outcome_of(rust_call);
    assert_eq!(
        c, r,
        "[{label}] rejection mismatch: C ended with {c}, Rust ended with {r}"
    );
}
