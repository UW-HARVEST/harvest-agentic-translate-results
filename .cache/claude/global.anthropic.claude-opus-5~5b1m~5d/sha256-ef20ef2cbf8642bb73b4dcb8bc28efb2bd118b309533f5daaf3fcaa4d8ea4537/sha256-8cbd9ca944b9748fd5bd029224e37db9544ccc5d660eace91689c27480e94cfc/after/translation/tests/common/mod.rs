//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and the
//! `driver` symbol is resolved with `dlsym`. The Rust implementation is NEVER
//! called directly as a Rust function — it is always reached through its
//! `#[no_mangle]` export in the built `cdylib`, exactly as an external C caller
//! would reach it. That way the export wrapper itself is under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

/// `void driver(int, int)`
pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

extern "C" {
    /// `int fflush(FILE *)`. Passing NULL flushes *all* open output streams,
    /// which is what we need to force the library's `printf` through the fd.
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;
const STDOUT_FILENO: c_int = 1;

/// Repository root, derived from `CARGO_MANIFEST_DIR` (= `translation/`).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    repo_root().join("c_src/build/libdriver.so")
}

/// Path to the Rust `cdylib` **built for the profile this test binary itself was
/// built with**.
///
/// The test executable lives at `target/<profile>/deps/<name>-<hash>`, so the
/// sibling `cdylib` is `target/<profile>/libdriver.so`. Deriving it this way
/// matters: hardcoding `release` would make a `cargo test` (debug) run silently
/// re-verify the stale release artifact instead of the one it just built.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>
    let profile_dir = exe
        .parent() // deps/
        .and_then(|p| p.parent()) // <profile>/
        .expect("test binary should live in target/<profile>/deps/");
    let p = profile_dir.join("libdriver.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {} (build the cdylib for this profile first)",
        p.display()
    );
    p
}

/// The two implementations under test, each behind its own `dlopen` handle.
pub struct Impls {
    _c_lib: libloading::Library,
    _rust_lib: libloading::Library,
    pub c: DriverFn,
    pub rust: DriverFn,
}

pub fn load() -> Impls {
    unsafe {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        let c_lib = libloading::Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
        let rust_lib = libloading::Library::new(&r_path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", r_path.display()));

        // dlsym("driver") on each handle. This is the ONLY way the Rust code is
        // invoked anywhere in this test suite.
        let c: libloading::Symbol<DriverFn> = c_lib
            .get(b"driver\0")
            .expect("C .so does not export `driver`");
        let rust: libloading::Symbol<DriverFn> = rust_lib
            .get(b"driver\0")
            .expect("Rust .so does not export `driver`");

        let c = *c;
        let rust = *rust;
        Impls {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    }
}

/// A scratch file used as the redirection target for fd 1.
struct Scratch {
    path: std::ffi::CString,
    fd: c_int,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let path = format!(
            "{}/driver_diff_{}_{}_{:?}.out",
            dir.trim_end_matches('/'),
            tag,
            std::process::id(),
            std::thread::current().id()
        );
        let cpath = std::ffi::CString::new(path).unwrap();
        let fd = unsafe { open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int) };
        assert!(fd >= 0, "failed to create scratch file");
        Scratch { path: cpath, fd }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        unsafe {
            close(self.fd);
            unlink(self.path.as_ptr());
        }
    }
}

/// Run `f` with fd 1 redirected to a scratch file and return the exact bytes it
/// wrote. Used for the non-faulting (Phase B) cases: it is fast enough for tens
/// of thousands of calls.
///
/// The library writes through libc `printf`, so `fflush(NULL)` is issued while
/// the redirect is still installed to guarantee every byte has reached the fd.
fn capture(tag: &str, f: impl FnOnce()) -> Vec<u8> {
    let scratch = Scratch::new(tag);
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(STDOUT_FILENO);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(scratch.fd, STDOUT_FILENO) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FILENO) >= 0, "dup2 restore failed");
        close(saved);

        // Read everything the call produced.
        lseek(scratch.fd, 0, 0 /* SEEK_SET */);
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = read(scratch.fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            assert!(n >= 0, "read from scratch failed");
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        out
    }
}

/// stdout produced by the C `driver(x, y)`.
pub fn c_out(imp: &Impls, x: i32, y: i32) -> Vec<u8> {
    capture("c", || unsafe { (imp.c)(x, y) })
}

/// stdout produced by the Rust `driver(x, y)`, reached via `dlsym`.
pub fn rust_out(imp: &Impls, x: i32, y: i32) -> Vec<u8> {
    capture("rust", || unsafe { (imp.rust)(x, y) })
}

/// How a `driver` invocation ended when run in its own process.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// `Some(signal)` if the child was killed by a signal, else `None`.
    pub signal: Option<c_int>,
    /// Normal exit code, if it exited normally.
    pub exit_code: Option<c_int>,
    /// Bytes the call wrote to stdout before ending.
    pub stdout: Vec<u8>,
}

/// Run `driver(x, y)` in a forked child with fd 1 redirected, and report how it
/// terminated. This is how the faulting (Phase C) inputs are compared: the
/// signal number itself is observed, so "both crashed with SIGFPE" is asserted
/// rather than merely "both failed".
pub fn outcome(f: DriverFn, tag: &str, x: i32, y: i32) -> Outcome {
    let scratch = Scratch::new(tag);
    unsafe {
        // Flush before forking so no already-buffered bytes get duplicated into
        // the child's copy of the stdio buffers.
        fflush(std::ptr::null_mut());

        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: redirect, call, flush, leave without running atexit
            // handlers (so the parent's buffers can't be double-flushed).
            dup2(scratch.fd, STDOUT_FILENO);
            f(x, y);
            fflush(std::ptr::null_mut());
            _exit(0);
        }

        let mut status: c_int = 0;
        assert!(waitpid(pid, &mut status, 0) == pid, "waitpid failed");

        lseek(scratch.fd, 0, 0);
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = read(scratch.fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }

        // Decode wait status without pulling in the libc crate.
        let exited = (status & 0x7f) == 0;
        let signaled = ((status & 0x7f) + 1) >> 1 > 0 && !exited;
        Outcome {
            signal: if signaled { Some(status & 0x7f) } else { None },
            exit_code: if exited {
                Some((status >> 8) & 0xff)
            } else {
                None
            },
            stdout: out,
        }
    }
}

/// Deterministic SplitMix64 PRNG so every run uses the identical inputs.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Uniform non-zero `i32`.
    pub fn nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.next_i32();
            if v != 0 {
                return v;
            }
        }
    }
}

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

/// Assert C and Rust produce byte-identical stdout for `driver(x, y)`.
/// Also sanity-checks that output was actually produced (guards against a
/// broken capture harness silently comparing two empty buffers).
pub fn assert_same(imp: &Impls, row: &str, x: i32, y: i32) {
    let c = c_out(imp, x, y);
    let r = rust_out(imp, x, y);
    assert!(
        !c.is_empty(),
        "[{row}] capture harness produced no C output for driver({x}, {y})"
    );
    assert_eq!(
        c,
        r,
        "[{row}] divergence for driver({x}, {y})\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
}
