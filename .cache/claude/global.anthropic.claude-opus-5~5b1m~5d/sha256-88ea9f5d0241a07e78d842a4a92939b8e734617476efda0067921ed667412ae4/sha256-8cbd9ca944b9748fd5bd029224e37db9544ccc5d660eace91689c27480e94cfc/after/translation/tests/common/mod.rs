//! Shared differential-test harness.
//!
//! Both the C library and the Rust library are loaded *as shared objects* via
//! `libloading`, and every call goes through the exported C ABI symbols. The
//! Rust functions are never called directly, so the `#[unsafe(no_mangle)]
//! extern "C"` wrappers are part of what is under test.
//!
//! Both libraries write to the process-global glibc `stdout`, so output is
//! captured by temporarily `dup2`-ing a temp file over file descriptor 1.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub type DriverFn = unsafe extern "C" fn(c_int);
pub type PrintLineFn = unsafe extern "C" fn(*const c_char);

/// Which implementation a call is directed at.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("translation has a parent").to_path_buf()
}

fn must_exist(p: PathBuf, how: &str) -> PathBuf {
    assert!(
        p.exists(),
        "missing shared library {}\nbuild it first with:\n  {}",
        p.display(),
        how
    );
    p
}

pub fn c_so_path() -> PathBuf {
    let root = repo_root();
    for cand in [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    must_exist(
        root.join("c_src/build/libdriver.so"),
        "cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
    )
}

pub fn rust_so_path() -> PathBuf {
    let root = repo_root();
    must_exist(
        root.join("translation/target/release/libdriver.so"),
        "cd translation && cargo build --release",
    )
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            let c = Library::new(c_so_path()).expect("failed to dlopen the C libdriver.so");
            let rust =
                Library::new(rust_so_path()).expect("failed to dlopen the Rust libdriver.so");
            Libs { c, rust }
        }
    }

    fn lib(&self, which: Impl) -> &Library {
        match which {
            Impl::C => &self.c,
            Impl::Rust => &self.rust,
        }
    }

    pub fn driver(&self, which: Impl) -> Symbol<'_, DriverFn> {
        unsafe {
            self.lib(which)
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("symbol `driver` missing from {:?} .so: {e}", which))
        }
    }

    pub fn print_line(&self, which: Impl) -> Symbol<'_, PrintLineFn> {
        unsafe {
            self.lib(which)
                .get(b"printLine\0")
                .unwrap_or_else(|e| panic!("symbol `printLine` missing from {:?} .so: {e}", which))
        }
    }

    /// Call `driver(data)` in one implementation and return the raw stdout bytes.
    pub fn call_driver(&self, which: Impl, data: c_int) -> Vec<u8> {
        let f = self.driver(which);
        capture_stdout(|| unsafe { f(data) })
    }

    /// Call `printLine(ptr)` in one implementation and return the raw stdout bytes.
    ///
    /// # Safety
    /// `ptr` must be NUL-terminated or null.
    pub unsafe fn call_print_line(&self, which: Impl, ptr: *const c_char) -> Vec<u8> {
        let f = self.print_line(which);
        capture_stdout(|| unsafe { f(ptr) })
    }
}

static SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_path() -> PathBuf {
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    dir.join(format!("driver_difftest_{}_{}.out", std::process::id(), n))
}

/// Run `f` in a forked child whose fd 1 is a fresh temp file, then return the
/// bytes the child wrote plus how it terminated.
///
/// The child is used (rather than `dup2` inside this process) so that libtest's
/// own progress output, emitted concurrently from other threads, can never be
/// mixed into the captured stream. Everything the callee emits through the C
/// runtime's `stdout` (`puts`, `printf`, …) is captured, including its
/// buffering behaviour, since the stream is flushed inside the child before it
/// exits. A regular file is used instead of a pipe so that large outputs cannot
/// deadlock on the pipe capacity.
pub fn capture_stdout_status<F: FnOnce()>(f: F) -> (Vec<u8>, Exit) {
    let path = temp_path();
    let file = std::fs::File::create(&path).expect("create temp capture file");
    let tmp_fd = file.as_raw_fd();

    let exit = unsafe {
        // Nothing of ours must be inherited in the child's stdio buffers.
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            if libc::dup2(tmp_fd, 1) < 0 {
                libc::_exit(101);
            }
            f();
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
        let mut status: c_int = 0;
        assert!(libc::waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Exit::Signal(libc::WTERMSIG(status))
        } else {
            Exit::Code(libc::WEXITSTATUS(status))
        }
    };
    drop(file);

    let mut buf = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen temp capture file")
        .read_to_end(&mut buf)
        .expect("read temp capture file");
    let _ = std::fs::remove_file(&path);
    (buf, exit)
}

/// Like [`capture_stdout_status`] but asserts the call returned normally.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let (buf, exit) = capture_stdout_status(f);
    assert_eq!(
        exit,
        Exit::Code(0),
        "the captured call did not return normally (output so far: {:?})",
        String::from_utf8_lossy(&buf)
    );
    buf
}

/// Assert two byte streams are identical, with a readable diff on failure.
pub fn assert_same(ctx: &str, c_out: &[u8], rust_out: &[u8]) {
    if c_out != rust_out {
        panic!(
            "output divergence for {ctx}\n  C    ({} bytes): {:?}\n  Rust ({} bytes): {:?}",
            c_out.len(),
            String::from_utf8_lossy(c_out),
            rust_out.len(),
            String::from_utf8_lossy(rust_out),
        );
    }
}

/// Deterministic PRNG (xorshift64*) so every randomized row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    /// Uniform-ish value in `[lo, hi]` (inclusive).
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn byte_in(&mut self, lo: u8, hi: u8) -> u8 {
        (lo as u64 + self.next_u64() % (hi as u64 - lo as u64 + 1)) as u8
    }
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }
}

/// Run `body` in a forked child and report how the child terminated.
/// Used for the deliberately-undefined-behaviour rows, which crash the process.
#[derive(Debug, PartialEq, Eq)]
pub enum Exit {
    Code(i32),
    Signal(i32),
}

pub fn fork_and_run<F: FnOnce()>(body: F) -> Exit {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: silence output, run the UB, exit cleanly if it survives.
            let devnull = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 1);
                libc::dup2(devnull, 2);
            }
            body();
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
        let mut status: c_int = 0;
        assert!(libc::waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Exit::Signal(libc::WTERMSIG(status))
        } else {
            Exit::Code(libc::WEXITSTATUS(status))
        }
    }
}

/// Path helper for the symbol-parity test.
pub fn nm_dynamic_defined(so: &Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("running `nm` failed");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}
