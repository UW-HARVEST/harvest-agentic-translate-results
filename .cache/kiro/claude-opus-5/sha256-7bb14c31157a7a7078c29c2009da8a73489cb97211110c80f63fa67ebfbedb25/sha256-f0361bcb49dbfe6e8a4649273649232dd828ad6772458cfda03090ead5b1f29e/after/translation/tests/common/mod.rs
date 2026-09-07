//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked only through their exported `driver` symbol. The Rust side is never
//! called as a Rust function, so the `#[no_mangle] extern "C"` wrapper is part
//! of what is under test.

#![allow(dead_code)]

use std::ffi::c_int;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, OnceLock};

/// `void driver(int, int)`
pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

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

/// Serializes everything that touches process-global state (fd 1, `fork`).
static GLOBAL: Mutex<()> = Mutex::new(());

pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    match GLOBAL.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // .../target/<profile>/deps/<test-exe>  ->  .../target/<profile>/libdriver.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe lives in target/<profile>/deps")
        .to_path_buf();
    profile_dir.join("libdriver.so")
}

struct Loaded {
    // Kept alive for the lifetime of the process; the raw fn pointers below
    // point into these mappings.
    _c_lib: libloading::Library,
    _rust_lib: libloading::Library,
    c_driver: DriverFn,
    rust_driver: DriverFn,
}

// The raw fn pointers are plain addresses into two permanently-mapped
// libraries; sharing them across threads is sound.
unsafe impl Send for Loaded {}
unsafe impl Sync for Loaded {}

static LOADED: OnceLock<Loaded> = OnceLock::new();

fn loaded() -> &'static Loaded {
    LOADED.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not found at {}\nBuild it with: cargo build [--release]",
            rust_path.display()
        );

        unsafe {
            let c_lib = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

            // Resolve eagerly so no `dlsym` is ever needed after a `fork`.
            let c_driver: DriverFn = *c_lib
                .get::<DriverFn>(b"driver\0")
                .expect("C .so exports `driver`");
            let rust_driver: DriverFn = *rust_lib
                .get::<DriverFn>(b"driver\0")
                .expect("Rust .so exports `driver`");

            Loaded {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_driver,
                rust_driver,
            }
        }
    })
}

pub fn driver_fn(which: Impl) -> DriverFn {
    let l = loaded();
    match which {
        Impl::C => l.c_driver,
        Impl::Rust => l.rust_driver,
    }
}

/// Flush every `FILE*` in the process plus Rust's own `stdout` buffer, so that
/// no bytes straddle an fd-1 redirection boundary.
fn flush_everything() {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe {
        libc::fflush(ptr::null_mut());
    }
}

fn temp_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "driver-difftest-{}-{}-{}-{}",
        std::process::id(),
        tag,
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    p
}

/// Run `pairs` through `which`'s `driver`, with fd 1 pointed at a regular
/// file, and return every byte written.
pub fn capture_file(which: Impl, pairs: &[(i32, i32)]) -> Vec<u8> {
    let f = driver_fn(which);
    let _g = lock();
    let path = temp_path("out");

    let bytes = {
        let file = fs::File::create(&path).expect("create capture file");
        flush_everything();
        let saved = unsafe { libc::dup(1) };
        assert!(saved >= 0, "dup(1) failed");
        assert!(
            unsafe { libc::dup2(file.as_raw_fd(), 1) } >= 0,
            "dup2 onto fd 1 failed"
        );

        for &(x, y) in pairs {
            unsafe { f(x, y) };
        }

        unsafe {
            libc::fflush(ptr::null_mut());
            libc::dup2(saved, 1);
            libc::close(saved);
        }
        drop(file);
        fs::read(&path).expect("read capture file")
    };

    let _ = fs::remove_file(&path);
    bytes
}

/// Same, but fd 1 is a *pipe*: non-seekable, so glibc selects full block
/// buffering rather than the mode it would pick for a regular file.
pub fn capture_pipe(which: Impl, pairs: &[(i32, i32)]) -> Vec<u8> {
    let f = driver_fn(which);
    let _g = lock();

    let mut fds = [0 as c_int; 2];
    assert!(unsafe { libc::pipe(fds.as_mut_ptr()) } == 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    // Drain concurrently so a full pipe buffer cannot deadlock the writer.
    let reader = std::thread::spawn(move || {
        let mut file = unsafe { <fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(rd) };
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf);
        buf
    });

    flush_everything();
    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { libc::dup2(wr, 1) } >= 0, "dup2 onto fd 1 failed");
    unsafe { libc::close(wr) };

    for &(x, y) in pairs {
        unsafe { f(x, y) };
    }

    unsafe {
        libc::fflush(ptr::null_mut());
        // Dropping the last write-end reference closes the pipe so the reader
        // sees EOF.
        libc::dup2(saved, 1);
        libc::close(saved);
    }

    reader.join().expect("pipe reader thread")
}

/// How the forked child should set up fd 1 before calling `driver`.
#[derive(Copy, Clone, Debug)]
pub enum ChildStdout {
    /// fd 1 is the write end of a pipe the parent drains.
    Pipe,
    /// fd 1 is a read-only descriptor, so every `write` fails with `EBADF`.
    /// Models "printf fails" without involving `SIGPIPE`.
    ReadOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Exited(i32),
    Signaled(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub status: Status,
    pub stdout: Vec<u8>,
}

/// Call `which`'s `driver(x, y)` in a forked child, so that a fatal signal
/// (`SIGFPE`) is observable instead of killing the test runner.
///
/// The libraries are `dlopen`ed and the symbol resolved *before* the fork, so
/// the child never touches the dynamic loader.
pub fn run_isolated(which: Impl, stdout_mode: ChildStdout, x: i32, y: i32) -> Outcome {
    let f = driver_fn(which);
    let _g = lock();

    let mut fds = [0 as c_int; 2];
    assert!(unsafe { libc::pipe(fds.as_mut_ptr()) } == 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    flush_everything();

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork() failed");

    if pid == 0 {
        // ---- child ----
        unsafe {
            libc::close(rd);
            match stdout_mode {
                ChildStdout::Pipe => {
                    libc::dup2(wr, 1);
                }
                ChildStdout::ReadOnly => {
                    let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY);
                    libc::dup2(devnull, 1);
                }
            }
            libc::close(wr);

            f(x, y);

            libc::fflush(ptr::null_mut());
            libc::_exit(0);
        }
    }

    // ---- parent ----
    unsafe { libc::close(wr) };
    let mut out = Vec::new();
    {
        let mut file = unsafe { <fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(rd) };
        let _ = file.read_to_end(&mut out);
    }

    let mut wstatus: c_int = 0;
    loop {
        let r = unsafe { libc::waitpid(pid, &mut wstatus, 0) };
        if r == pid {
            break;
        }
        assert!(
            r < 0 && unsafe { *libc::__errno_location() } == libc::EINTR,
            "waitpid failed"
        );
    }

    let status = if libc::WIFSIGNALED(wstatus) {
        Status::Signaled(libc::WTERMSIG(wstatus))
    } else if libc::WIFEXITED(wstatus) {
        Status::Exited(libc::WEXITSTATUS(wstatus))
    } else {
        panic!("child neither exited nor signaled: {wstatus:#x}");
    };

    Outcome {
        status,
        stdout: out,
    }
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn render(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\n', "\\n")
}

/// Assert C and Rust write byte-identical stdout for every pair in `pairs`,
/// driving each pair individually so a mismatch names the exact input.
pub fn assert_same_each(row: &str, pairs: &[(i32, i32)]) {
    for &(x, y) in pairs {
        let c = capture_file(Impl::C, &[(x, y)]);
        let r = capture_file(Impl::Rust, &[(x, y)]);
        assert_eq!(
            c,
            r,
            "{row}: stdout diverged for driver({x}, {y})\n  C   : \"{}\"\n  Rust: \"{}\"",
            render(&c),
            render(&r)
        );
        assert!(
            !c.is_empty(),
            "{row}: C produced no output for driver({x}, {y}); the test would be vacuous"
        );
    }
}

/// Assert C and Rust agree on the whole concatenated stream for a batch of
/// calls (catches buffering / ordering / trailing-newline divergence).
pub fn assert_same_batch(row: &str, pairs: &[(i32, i32)], via_pipe: bool) {
    let (c, r) = if via_pipe {
        (
            capture_pipe(Impl::C, pairs),
            capture_pipe(Impl::Rust, pairs),
        )
    } else {
        (
            capture_file(Impl::C, pairs),
            capture_file(Impl::Rust, pairs),
        )
    };
    assert_eq!(
        c.len(),
        r.len(),
        "{row}: total stdout length differs ({} vs {} bytes) over {} calls",
        c.len(),
        r.len(),
        pairs.len()
    );
    if c != r {
        let at = c.iter().zip(r.iter()).position(|(a, b)| a != b).unwrap();
        let lo = at.saturating_sub(60);
        panic!(
            "{row}: batch stdout diverged at byte {at}\n  C   : \"{}\"\n  Rust: \"{}\"",
            render(&c[lo..(at + 60).min(c.len())]),
            render(&r[lo..(at + 60).min(r.len())])
        );
    }
    assert!(!c.is_empty(), "{row}: C produced no output; test is vacuous");
}

// ---------------------------------------------------------------------------
// Reproducible randomness (SplitMix64) — fixed seeds, no external dep.
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

    /// Any `i32`, uniform over all 2^32 bit patterns.
    pub fn any_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[lo, hi]` inclusive; works across the full `i32` range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64) as u64 + 1;
        if span == 0 {
            return self.any_i32();
        }
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// Uniform in `[1, hi]`, biased toward small magnitudes half the time so
    /// that both wide and narrow `%d` fields are exercised.
    pub fn magnitude(&mut self, hi: i32) -> i32 {
        debug_assert!(hi >= 1);
        if self.next_u64() & 1 == 0 {
            self.range_i32(1, hi.min(1000))
        } else {
            self.range_i32(1, hi)
        }
    }

    /// Any nonzero `i32`.
    pub fn nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.any_i32();
            if v != 0 {
                return v;
            }
        }
    }
}

/// The two operand pairs on which the C traps (see `ERRORS.md` rows 1–2).
pub fn traps(x: i32, y: i32) -> bool {
    y == 0 || (x == i32::MIN && y == -1)
}

pub const PER_ROW: usize = 256;
