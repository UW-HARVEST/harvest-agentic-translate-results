//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls them only through
//! their exported C symbols — the Rust implementation is never called directly,
//! so the `#[no_mangle] extern "C"` export wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type HelloFn = unsafe extern "C" fn() -> c_int;

/// Which implementation to drive.
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

pub const BOTH: [Impl; 2] = [Impl::C, Impl::Rust];

/// The exact bytes the C library writes per call: `printf("Hello World!\n")`.
pub const EXPECTED_LINE: &[u8] = b"Hello World!\n";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_HELLO_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libhello.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_HELLO_SO") {
        return PathBuf::from(p);
    }
    // Prefer whichever cdylib cargo most recently produced for this invocation.
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for profile in ["debug", "release"] {
        let p = manifest_dir().join("target").join(profile).join("libhello.so");
        if let Ok(md) = std::fs::metadata(&p) {
            if let Ok(t) = md.modified() {
                if best.as_ref().map_or(true, |(bt, _)| t > *bt) {
                    best = Some((t, p));
                }
            }
        }
    }
    let (_, p) = best.unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found under {}/target/{{debug,release}}/libhello.so — run `cargo build`",
            manifest_dir().display()
        )
    });
    p
}

struct Libs {
    c: Library,
    rust: Library,
}

// SAFETY: we only ever call the loaded `helloworld`, which is re-entrant apart
// from libc's own internally-locked stdout.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_lib_path()).expect("dlopen C libhello.so");
        let rust = Library::new(rust_lib_path()).expect("dlopen Rust libhello.so");
        Libs { c, rust }
    })
}

/// `dlsym("helloworld")` on the requested library handle.
pub fn helloworld_of(which: Impl) -> HelloFn {
    let lib = match which {
        Impl::C => &libs().c,
        Impl::Rust => &libs().rust,
    };
    unsafe {
        let sym: Symbol<HelloFn> = lib
            .get(b"helloworld\0")
            .unwrap_or_else(|e| panic!("{} .so does not export `helloworld`: {e}", which.name()));
        *sym
    }
}

/// These tests redirect the process-wide fd 1, so the libtest harness must not
/// be printing its own progress output from another thread at the same time.
fn require_serial() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let serial_env = std::env::var("RUST_TEST_THREADS").as_deref() == Ok("1");
        let args: Vec<String> = std::env::args().collect();
        let serial_arg = args.windows(2).any(|w| w[0] == "--test-threads" && w[1] == "1")
            || args.iter().any(|a| a == "--test-threads=1");
        assert!(
            serial_env || serial_arg,
            "these differential tests redirect fd 1 process-wide and MUST run serially.\n\
             Run them as:  RUST_TEST_THREADS=1 cargo test --offline\n\
             or:           cargo test --offline -- --test-threads=1\n\
             (see ./run_tests.sh)"
        );
    });
}

/// Flush the libtest harness' own Rust-level `stdout` buffer so that a pending
/// unterminated progress line ("test foo ... ") cannot be flushed *into* our
/// capture file later on.
pub fn flush_rust_stdout() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

/// A permanently-leaked buffer for `setvbuf`.
///
/// glibc keeps using a caller-supplied `stdout` buffer even after a subsequent
/// `setvbuf(stdout, NULL, _IOFBF, 0)`, so any buffer we hand to libc must live
/// for the rest of the process — otherwise libc writes into freed memory
/// (observed as `malloc(): unaligned fastbin chunk detected`).
pub fn leaked_buf(n: usize) -> *mut libc::c_char {
    let b: &'static mut [u8] = Box::leak(vec![0u8; n.max(1)].into_boxed_slice());
    b.as_mut_ptr() as *mut libc::c_char
}

/// Put `stdout` back to ordinary fully-buffered mode on a fresh leaked buffer.
pub fn restore_default_buffering() {
    unsafe {
        libc::fflush(libc_stdout());
        libc::setvbuf(libc_stdout(), leaked_buf(4096), libc::_IOFBF, 4096);
        libc::clearerr(libc_stdout());
    }
}

/// Serialises every test that fiddles with process-wide fds / stdout state.
pub fn fd_lock() -> MutexGuard<'static, ()> {
    require_serial();
    flush_rust_stdout();
    static LOCK: Mutex<()> = Mutex::new(());
    match LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Buffering mode for the captured `stdout`.
#[derive(Copy, Clone, Debug)]
pub enum Buffering {
    /// leave libc's default alone
    Default,
    /// `setvbuf(stdout, NULL, _IOFBF, 0)`
    FullDefaultBuf,
    /// `setvbuf(stdout, own_buf, _IOFBF, n)`
    FullOwnBuf(usize),
    /// `setvbuf(stdout, own_buf, _IOLBF, n)`
    LineOwnBuf(usize),
    /// `setvbuf(stdout, NULL, _IONBF, 0)`
    None_,
}

/// How the flush is triggered at the end of the captured region.
#[derive(Copy, Clone, Debug)]
pub enum Flush {
    Stdout,
    All,
}

pub struct Capture {
    pub bytes: Vec<u8>,
    /// return values of each `helloworld` call, in order
    pub rets: Vec<c_int>,
    /// return value of the final `fflush`
    pub flush_ret: c_int,
}

fn tmp_path(tag: &str) -> PathBuf {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from(dir).join(format!("hello_diff_{tag}_{n}_{}.out", std::process::id()))
}

/// Redirects fd 1 to a fresh temp file, applies `buffering`, runs `body`,
/// flushes, restores fd 1 and returns everything that was written.
///
/// `open_flags`/`pad` let a caller vary the target-file shape (append mode,
/// non-zero start offset, ...).
pub fn capture_stdout<F>(
    tag: &str,
    buffering: Buffering,
    flush: Flush,
    pad: &[u8],
    append: bool,
    body: F,
) -> Capture
where
    F: FnOnce() -> Vec<c_int>,
{
    let path = tmp_path(tag);
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();

    if !pad.is_empty() {
        std::fs::write(&path, pad).unwrap();
    }

    let mut flags = libc::O_WRONLY | libc::O_CREAT;
    if append {
        flags |= libc::O_APPEND;
    }
    let fd = unsafe { libc::open(cpath.as_ptr(), flags, 0o600 as libc::c_int) };
    assert!(fd >= 0, "open({}) failed", path.display());
    if !pad.is_empty() && !append {
        // start writing after the padding
        unsafe { libc::lseek(fd, pad.len() as libc::off_t, libc::SEEK_SET) };
    }

    flush_rust_stdout();
    let (bytes, rets, flush_ret) = unsafe {
        let saved = libc::dup(1);
        assert!(saved >= 0);
        libc::fflush(libc_stdout());
        assert!(libc::dup2(fd, 1) >= 0);

        match buffering {
            Buffering::Default => {}
            Buffering::FullDefaultBuf => {
                libc::setvbuf(libc_stdout(), std::ptr::null_mut(), libc::_IOFBF, 0);
            }
            Buffering::FullOwnBuf(n) => {
                libc::setvbuf(libc_stdout(), leaked_buf(n), libc::_IOFBF, n);
            }
            Buffering::LineOwnBuf(n) => {
                libc::setvbuf(libc_stdout(), leaked_buf(n), libc::_IOLBF, n);
            }
            Buffering::None_ => {
                libc::setvbuf(libc_stdout(), std::ptr::null_mut(), libc::_IONBF, 0);
            }
        }

        let rets = body();

        let flush_ret = match flush {
            Flush::Stdout => libc::fflush(libc_stdout()),
            Flush::All => libc::fflush(std::ptr::null_mut()),
        };

        // Move stdout onto a *fresh* leaked buffer; the previous one stays alive.
        libc::setvbuf(libc_stdout(), leaked_buf(4096), libc::_IOFBF, 4096);
        libc::clearerr(libc_stdout());

        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(fd);

        let mut bytes = std::fs::read(&path).unwrap_or_default();
        // strip the pre-existing padding so callers compare only new output
        if bytes.len() >= pad.len() && bytes.starts_with(pad) {
            bytes.drain(..pad.len());
        }
        (bytes, rets, flush_ret)
    };

    let _ = std::fs::remove_file(&path);
    Capture { bytes, rets, flush_ret }
}

/// `stdout` as libc sees it.
pub fn libc_stdout() -> *mut libc::FILE {
    // glibc exposes `stdout` as a data symbol; libc crate mirrors it.
    unsafe { libc_stdout_ptr() }
}

unsafe fn libc_stdout_ptr() -> *mut libc::FILE {
    unsafe extern "C" {
        #[link_name = "stdout"]
        static mut C_STDOUT: *mut libc::FILE;
    }
    unsafe { C_STDOUT }
}

/// Redirects fd 1 to `dev_path` (e.g. `/dev/full`, `/dev/null`), runs `body`,
/// restores fd 1. Returns (call return values, final `fflush` result).
pub fn with_stdout_to<F>(dev_path: &str, unbuffered: bool, body: F) -> (Vec<c_int>, c_int)
where
    F: FnOnce() -> Vec<c_int>,
{
    let cpath = CString::new(dev_path).unwrap();
    flush_rust_stdout();
    unsafe {
        let fd = libc::open(cpath.as_ptr(), libc::O_WRONLY);
        assert!(fd >= 0, "open({dev_path}) failed");
        let saved = libc::dup(1);
        libc::fflush(libc_stdout());
        libc::dup2(fd, 1);
        if unbuffered {
            libc::setvbuf(libc_stdout(), std::ptr::null_mut(), libc::_IONBF, 0);
        }
        let rets = body();
        let fr = libc::fflush(libc_stdout());
        libc::setvbuf(libc_stdout(), leaked_buf(4096), libc::_IOFBF, 4096);
        libc::clearerr(libc_stdout());
        // Discard anything still queued for the failing target.
        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(fd);
        (rets, fr)
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64) — fixed seed for reproducibility
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
    /// uniform in `lo..=hi`
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        assert!(hi >= lo);
        lo + self.next_u64() % (hi - lo + 1)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

pub const SEED: u64 = 0x5EED_1234;

/// Expected stream contents for `n` successful calls.
pub fn repeat_line(n: usize) -> Vec<u8> {
    EXPECTED_LINE.repeat(n)
}
