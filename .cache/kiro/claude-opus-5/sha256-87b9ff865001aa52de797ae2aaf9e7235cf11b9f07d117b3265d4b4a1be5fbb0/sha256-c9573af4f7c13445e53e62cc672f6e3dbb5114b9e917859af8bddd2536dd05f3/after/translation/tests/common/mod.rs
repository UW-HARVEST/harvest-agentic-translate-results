//! Shared differential-test support.
//!
//! Both implementations are always reached through `dlopen`/`dlsym` on their
//! respective shared objects — the Rust side is never called directly as a
//! Rust function, so the `#[no_mangle] extern "C"` export wrapper is under
//! test too.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Read;
use std::os::unix::io::RawFd;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Which implementation a loaded library is.
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
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    // .../<work>/translation -> .../<work>
    crate_root()
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C shared object, building it with CMake if it is not there yet.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HELLO_C_SO") {
        return PathBuf::from(p);
    }
    let root = workspace_root();
    let candidates = [
        root.join("c_src/build/libhello.so"),
        root.join("c_src/build/lib/libhello.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    build_c();
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "could not find or build the C libhello.so; looked in {:?}",
        candidates
    );
}

fn build_c() {
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let ok = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(ok, "cmake configure of c_src failed");
    let ok = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(ok, "cmake build of c_src failed");
}

/// Path to the Rust shared object produced by this crate (`crate-type =
/// ["cdylib"]`).
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HELLO_RUST_SO") {
        return PathBuf::from(p);
    }
    // The integration-test binary lives in target/<profile>/deps/, so the
    // sibling cdylib is one directory up. Fall back to the well-known
    // profile directories.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            candidates.push(deps.join("libhello.so"));
            if let Some(profile) = deps.parent() {
                candidates.push(profile.join("libhello.so"));
            }
        }
    }
    let target = crate_root().join("target");
    candidates.push(target.join("release/libhello.so"));
    candidates.push(target.join("debug/libhello.so"));

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "could not find the Rust libhello.so; run `cargo build --release` first. Looked in {:?}",
        candidates
    );
}

/// Both shared objects, opened at once. `libloading::Library::new` uses
/// `RTLD_LOCAL`, so the two identically-named `helloworld` symbols do not
/// collide: each handle resolves to its own definition.
pub struct Pair {
    pub c: Library,
    pub rust: Library,
}

pub fn load_pair() -> Pair {
    Pair {
        c: open(&c_so_path()),
        rust: open(&rust_so_path()),
    }
}

pub fn open(path: &Path) -> Library {
    unsafe { Library::new(path) }.unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
}

impl Pair {
    pub fn lib(&self, which: Impl) -> &Library {
        match which {
            Impl::C => &self.c,
            Impl::Rust => &self.rust,
        }
    }

    /// `helloworld` with its declared (empty) parameter list.
    pub fn helloworld(&self, which: Impl) -> Symbol<'_, unsafe extern "C" fn() -> c_int> {
        unsafe { self.lib(which).get(b"helloworld\0") }.expect("dlsym(helloworld)")
    }

    /// Raw address of `helloworld`, for calling through deliberately
    /// differently-typed function pointers (the header uses an unprototyped
    /// declaration, `int helloworld();`, so C callers may pass arguments).
    pub fn helloworld_addr(&self, which: Impl) -> *const c_void {
        let s: Symbol<'_, unsafe extern "C" fn() -> c_int> =
            unsafe { self.lib(which).get(b"helloworld\0") }.expect("dlsym(helloworld)");
        unsafe { s.into_raw() }.into_raw() as *const c_void
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// How the captured `stdout` should be buffered while the call runs.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Buffering {
    /// Leave whatever libc chose for the redirected fd.
    Default,
    /// `setvbuf(stdout, buf, _IOFBF, n)`
    Full(usize),
    /// `setvbuf(stdout, buf, _IOLBF, n)`
    Line(usize),
    /// `setvbuf(stdout, NULL, _IONBF, 0)`
    None,
}

/// Where the captured `stdout` should point.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sink {
    /// A regular temporary file.
    File,
    /// A `pipe(2)`.
    Pipe,
}

/// Redirects the process-wide `stdout` (fd 1 *and* the libc `FILE*`) into a
/// capture, runs `body`, then restores it and returns the captured bytes.
///
/// The whole point of routing through fd 1 is that it is neutral: it captures
/// whatever either implementation emits, whether that goes through libc
/// stdio, through Rust's `std::io::stdout`, or straight to the descriptor.
pub fn capture<R>(sink: Sink, buffering: Buffering, body: impl FnOnce() -> R) -> (R, Vec<u8>) {
    // Serialize: fd 1 is process-global state.
    let _guard = capture_lock();

    unsafe {
        // Flush anything the harness itself has pending so it is not swept
        // into the capture. This must happen while fd 1 is still the *real*
        // stdout: libtest writes a partial line ("test foo ... ") before each
        // test runs, and that byte sequence would otherwise be flushed into
        // our capture and counted as library output. Both stdio layers matter
        // — libc's `FILE*` and Rust's own `LineWriter`.
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
        }
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let (write_fd, reader) = make_sink(sink);

        assert!(libc::dup2(write_fd, 1) >= 0, "dup2 -> 1 failed");
        libc::close(write_fd);

        // Apply the requested buffering to the libc stream now that fd 1
        // points at the capture. `setvbuf` must come before any output on the
        // stream, which holds because we just flushed it.
        let mut buf: Vec<u8> = Vec::new();
        match buffering {
            Buffering::Default => {}
            Buffering::None => {
                libc::setvbuf(stdout_file(), std::ptr::null_mut(), libc::_IONBF, 0);
            }
            Buffering::Full(n) | Buffering::Line(n) => {
                let n = n.max(2);
                buf = vec![0u8; n];
                let mode = if matches!(buffering, Buffering::Full(_)) {
                    libc::_IOFBF
                } else {
                    libc::_IOLBF
                };
                libc::setvbuf(stdout_file(), buf.as_mut_ptr() as *mut c_char, mode, n);
            }
        }

        let out = body();

        // Drain both stdio layers while fd 1 still points at the capture, so
        // nothing the implementation emitted is left behind or leaks out
        // later.
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        // Flush every libc stream, then drop any custom buffer *after*
        // pointing the stream back at a default buffer.
        libc::fflush(std::ptr::null_mut());
        libc::setvbuf(stdout_file(), std::ptr::null_mut(), libc::_IONBF, 0);
        drop(buf);

        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
        // Back on the real stdout: restore ordinary buffering.
        libc::setvbuf(stdout_file(), std::ptr::null_mut(), libc::_IOFBF, 0);

        let bytes = reader.finish();
        (out, bytes)
    }
}

/// A sink that has been redirected onto fd 1 and can be drained afterwards.
enum Reader {
    File(std::fs::File),
    Pipe(std::fs::File, std::thread::JoinHandle<Vec<u8>>),
}

impl Reader {
    fn finish(self) -> Vec<u8> {
        match self {
            Reader::File(mut f) => {
                use std::io::Seek;
                f.seek(std::io::SeekFrom::Start(0)).expect("seek capture");
                let mut v = Vec::new();
                f.read_to_end(&mut v).expect("read capture");
                v
            }
            Reader::Pipe(w, joiner) => {
                // Dropping the last writer lets the drain thread see EOF.
                drop(w);
                joiner.join().expect("pipe drain thread")
            }
        }
    }
}

unsafe fn make_sink(sink: Sink) -> (RawFd, Reader) {
    match sink {
        Sink::File => {
            use std::os::unix::io::AsRawFd;
            let path = temp_path();
            let f = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
                .expect("create capture file");
            // Unlink immediately; the open fds keep it alive.
            let _ = std::fs::remove_file(&path);
            let dupd = libc::dup(f.as_raw_fd());
            assert!(dupd >= 0, "dup capture file");
            // `f` keeps the readable handle; `dupd` becomes fd 1.
            (dupd, Reader::File(f))
        }
        Sink::Pipe => {
            use std::os::unix::io::FromRawFd;
            let mut fds = [0 as RawFd; 2];
            assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe() failed");
            let [r, w] = fds;
            let mut rf = std::fs::File::from_raw_fd(r);
            // Drain concurrently so a full pipe buffer can never deadlock the
            // implementation under test.
            let joiner = std::thread::spawn(move || {
                let mut v = Vec::new();
                rf.read_to_end(&mut v).expect("read pipe");
                v
            });
            // Keep a writer alive so the drain thread only sees EOF when we
            // say so; fd 1 gets a duplicate.
            let keep = std::fs::File::from_raw_fd(libc::dup(w));
            assert!(w >= 0);
            (w, Reader::Pipe(keep, joiner))
        }
    }
}

fn temp_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "hello_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ))
}

fn stdout_file() -> *mut libc::FILE {
    // glibc exposes `stdout` as a global variable holding a FILE*.
    extern "C" {
        static mut stdout: *mut libc::FILE;
    }
    unsafe { std::ptr::addr_of!(stdout).read() }
}

/// fd 1 is process-global, so captures must not overlap.
fn capture_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Runs `body` with `stdout` pointing at a descriptor that cannot be written
/// to, so the `printf` inside the library fails. Used by the `ERRORS.md` row
/// that checks the C swallows `printf`'s failure.
///
/// `mode` selects how fd 1 is broken:
/// * `Unwritable` — fd 1 is a read-only `/dev/null` (writes fail `EBADF`).
/// * `Closed` — fd 1 is closed outright.
///
/// Returns `(body's value, whether the stream's error indicator got set)`. The
/// second element exists so the caller can prove the write really did fail and
/// the test is not vacuous.
pub fn with_broken_stdout<R>(mode: BrokenStdout, body: impl FnOnce() -> R) -> (R, bool) {
    let _guard = capture_lock();
    unsafe {
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
        }
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        match mode {
            BrokenStdout::Unwritable => {
                let ro = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_RDONLY);
                assert!(ro >= 0, "open(/dev/null, O_RDONLY) failed");
                assert!(libc::dup2(ro, 1) >= 0, "dup2 -> 1 failed");
                libc::close(ro);
            }
            BrokenStdout::Closed => {
                libc::close(1);
            }
        }
        // Unbuffered, so the failing write happens inside the call rather than
        // at some later flush.
        libc::setvbuf(stdout_file(), std::ptr::null_mut(), libc::_IONBF, 0);
        libc::clearerr(stdout_file());

        let out = body();

        let errored = libc::ferror(stdout_file()) != 0;

        // The stream's error indicator is sticky; clear it before handing the
        // real stdout back, otherwise every later test would fail to print.
        libc::clearerr(stdout_file());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
        libc::clearerr(stdout_file());
        libc::setvbuf(stdout_file(), std::ptr::null_mut(), libc::_IOFBF, 0);
        (out, errored)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BrokenStdout {
    Unwritable,
    Closed,
}

// ---------------------------------------------------------------------------
// Deterministic randomness (fixed seed, reproducible)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0FF_EE00_0001;

/// SplitMix64 — small, deterministic, no external dependency.
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
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        assert!(lo <= hi);
        lo + self.next_u64() % (hi - lo + 1)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A printable ASCII blob of the given length.
    pub fn ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| b'!' + (self.next_u64() % 93) as u8)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Expectations shared by the tests
// ---------------------------------------------------------------------------

/// The exact bytes `c_src/src/hello.c` emits per call:
/// `printf("Hello World!\n")`.
pub const EXPECTED_LINE: &[u8] = b"Hello World!\n";

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Asserts the two captures are byte-identical, with a readable diff.
#[track_caller]
pub fn assert_same_bytes(label: &str, c: &[u8], rust: &[u8]) {
    if c != rust {
        let at = c
            .iter()
            .zip(rust.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c.len().min(rust.len()));
        panic!(
            "{label}: C and Rust stdout differ\n  C    ({} bytes): \"{}\"\n  Rust ({} bytes): \"{}\"\n  first difference at byte {at}",
            c.len(),
            show(c),
            rust.len(),
            show(rust),
        );
    }
}

#[track_caller]
pub fn assert_same_ret(label: &str, c: c_int, rust: c_int) {
    assert_eq!(c, rust, "{label}: return values differ (C={c}, Rust={rust})");
}
