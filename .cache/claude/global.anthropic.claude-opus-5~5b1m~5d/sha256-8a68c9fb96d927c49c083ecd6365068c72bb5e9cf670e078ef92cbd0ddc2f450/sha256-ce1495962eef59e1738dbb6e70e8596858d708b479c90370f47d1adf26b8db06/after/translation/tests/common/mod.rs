//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked only through their exported `driver` symbol — the Rust functions are
//! never called directly, so the `#[no_mangle] extern "C"` export wrapper is
//! part of what is under test.
//!
//! `driver` returns `void` and communicates solely by writing to `stdout` via
//! libc `printf`, so a differential comparison means capturing fd 1 around each
//! call. Redirecting the *test process's* own fd 1 is not good enough: the test
//! harness writes its progress output to the same descriptor and would pollute
//! the capture. Instead each measurement runs in a `fork()`ed child that points
//! its private fd 1 at a scratch file, calls the symbol, flushes stdio and
//! `_exit`s. That gives byte-exact isolation and, as a bonus, lets us compare
//! the *termination status* too — so a panic/abort in one implementation is
//! detected instead of silently matching an empty output.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

extern "C" {
    static mut stdout: *mut c_void;
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

/// Signature of the exported entry point: `void driver(int x)`.
pub type DriverFn = unsafe extern "C" fn(c_int);

/// Everything observable about one invocation.
#[derive(PartialEq, Eq, Clone)]
pub struct Outcome {
    /// Bytes written to stdout.
    pub out: Vec<u8>,
    /// Exit code, or `None` if killed by a signal.
    pub exit_code: Option<i32>,
    /// Terminating signal, if any (a Rust panic under `panic = "abort"` shows up
    /// here as SIGABRT/6, and would not match the C library).
    pub signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = String::from_utf8_lossy(&self.out);
        let head: String = text.chars().take(200).collect();
        write!(
            f,
            "Outcome {{ exit: {:?}, signal: {:?}, {} bytes, {} lines, head: {:?} }}",
            self.exit_code,
            self.signal,
            self.out.len(),
            text.lines().count(),
            head
        )
    }
}

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Self {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {name} ({}): {e}", path.display()));
        unsafe {
            // Fail loudly right here if the export wrapper is missing.
            let _probe: Symbol<DriverFn> = lib.get(b"driver\0").unwrap_or_else(|e| {
                panic!("{name}: {} does not export `driver`: {e}", path.display())
            });
        }
        Impl { name, path, lib }
    }

    pub fn driver(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.lib.get(b"driver\0") }.expect("symbol `driver` vanished")
    }

    /// Call `driver(x)` through the `.so` in an isolated child process.
    pub fn run(&self, x: c_int) -> Outcome {
        let f = self.driver();
        capture(|| unsafe { f(x) })
    }

    /// Convenience: just the stdout bytes, asserting a clean exit.
    pub fn run_out(&self, x: c_int) -> Vec<u8> {
        let o = self.run(x);
        assert_eq!(
            (o.exit_code, o.signal),
            (Some(0), None),
            "{}: driver({x}) did not exit cleanly: {o:?}",
            self.name
        );
        o.out
    }
}

pub struct Impls {
    pub c: Impl,
    /// One entry per Rust build profile present on disk (release, debug); both
    /// are compared because `panic`/overflow-check settings differ per profile.
    pub rust: Vec<Impl>,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let c_path = match std::env::var_os("C_DRIVER_SO") {
            Some(p) => PathBuf::from(p),
            None => manifest_dir()
                .parent()
                .expect("crate has a parent dir")
                .join("c_src/build/libdriver.so"),
        };
        assert!(
            c_path.exists(),
            "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );

        let mut rust = Vec::new();
        match std::env::var_os("RUST_DRIVER_SO") {
            Some(p) => rust.push(Impl::load("rust(env)", PathBuf::from(p))),
            None => {
                for (name, rel) in [
                    ("rust(release)", "target/release/libdriver.so"),
                    ("rust(debug)", "target/debug/libdriver.so"),
                ] {
                    let p = manifest_dir().join(rel);
                    if p.exists() {
                        rust.push(Impl::load(name, p));
                    }
                }
            }
        }
        assert!(
            !rust.is_empty(),
            "no Rust shared library under {}/target/{{release,debug}}; run `cargo build --release`.",
            manifest_dir().display()
        );

        Impls { c: Impl::load("c", c_path), rust }
    })
}

/// `fork()` from a multi-threaded process is only safe if no other thread holds
/// an internal libc lock (the child calls `printf`, which allocates). Tests are
/// forced single-threaded via `.cargo/config.toml` (`RUST_TEST_THREADS=1`), and
/// this mutex keeps that true even if the setting is overridden.
fn fork_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn scratch_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("driver_diff_{}_{}.out", std::process::id(), n))
}

/// Run `f` in a forked child with fd 1 redirected to a scratch file; return the
/// bytes it wrote plus how the child terminated.
pub fn capture<F: FnOnce()>(f: F) -> Outcome {
    let _guard = fork_lock();
    let path = scratch_path();

    let file = std::fs::File::create(&path)
        .unwrap_or_else(|e| panic!("cannot create scratch file {}: {e}", path.display()));

    // Empty both buffering layers so nothing of ours is inherited by the child
    // and attributed to the library under test.
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { fflush(stdout) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");

    if pid == 0 {
        // ---- child: must not run Rust destructors or touch Rust's stdout ----
        unsafe {
            if dup2(file.as_raw_fd(), 1) < 0 {
                _exit(101);
            }
        }
        f();
        // `_exit` does not flush stdio, so do it explicitly.
        unsafe {
            fflush(stdout);
            _exit(0);
        }
    }

    let mut status: c_int = 0;
    let waited = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(waited, pid, "waitpid failed for child {pid}");
    drop(file);

    let (exit_code, signal) = if status & 0x7f == 0 {
        (Some((status >> 8) & 0xff), None)
    } else {
        (None, Some(status & 0x7f))
    };
    assert_ne!(exit_code, Some(101), "child failed to redirect fd 1");

    let out = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("cannot read scratch file {}: {e}", path.display()));
    let _ = std::fs::remove_file(&path);

    Outcome { out, exit_code, signal }
}

/// Assert every Rust `.so` behaves identically to the C `.so` for `driver(x)`:
/// same stdout bytes and same termination status.
pub fn assert_same(x: c_int) {
    let impls = impls();
    let expected = impls.c.run(x);
    for r in &impls.rust {
        let actual = r.run(x);
        if actual != expected {
            let at = expected
                .out
                .iter()
                .zip(actual.out.iter())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| expected.out.len().min(actual.out.len()));
            panic!(
                "divergence for driver({x}) between c and {}\n  first differing stdout byte \
                 offset: {at}\n  C   : {expected:?}\n  RUST: {actual:?}",
                r.name
            );
        }
    }
}

pub fn assert_same_all<I: IntoIterator<Item = c_int>>(xs: I) {
    for x in xs {
        assert_same(x);
    }
}

/// The exact bytes `driver.c` specifies: `x` lines of `"%d %d\n"` with `i` and
/// `j == 2*i` (wrapping on signed overflow, as the `-O0` C build does). Used as
/// an independent oracle so a test cannot pass by both implementations being
/// broken in the same way.
pub fn model_output(x: c_int) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i: c_int = 0;
    let mut j: c_int = 0;
    while i < x {
        out.extend_from_slice(format!("{i} {j}\n").as_bytes());
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
    out
}

/// Deterministic PRNG (SplitMix64) so randomised rows are reproducible.
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
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}
