//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects via `libloading` and
//! invoked purely through their exported `extern "C"` symbols — the Rust
//! translation is *never* called directly as a Rust function, so the
//! `#[no_mangle]` export wrapper is part of what is under test.
//!
//! `driver` communicates only by writing to the C runtime's `stdout`, so the
//! harness compares behaviour by capturing file descriptor 1 around each call.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed to redirect / flush the shared stdout stream.
// These are plain libc calls, not calls into either implementation.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn fileno(stream: *mut c_void) -> c_int;
}

// glibc exports `stdout` as a real data symbol.
extern "C" {
    static mut stdout: *mut c_void;
}

pub const IOFBF: c_int = 0; // fully buffered
pub const IOLBF: c_int = 1; // line buffered
pub const IONBF: c_int = 2; // unbuffered

/// Which implementation a captured run came from.
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

pub type DriverFn = unsafe extern "C" fn(c_int);

/// Holds both dynamically loaded libraries for the lifetime of a test.
pub struct Libs {
    c_lib: Library,
    rust_lib: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let root = workspace_root();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\nLooked in: {:?}",
        candidates
    );
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Prefer the release cdylib (that is what an external consumer links),
    // fall back to the debug one that `cargo test` produces.
    let candidates = [
        manifest.join("target/release/libdriver.so"),
        manifest.join("target/debug/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust shared library not found. Build it with:\n  cd translation && cargo build --release\n\
         Looked in: {:?}",
        candidates
    );
}

impl Libs {
    pub fn load() -> Libs {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));
            Libs {
                c_lib,
                rust_lib,
                c_path,
                rust_path,
            }
        }
    }

    fn sym(&self, which: Impl) -> Symbol<'_, DriverFn> {
        let lib = match which {
            Impl::C => &self.c_lib,
            Impl::Rust => &self.rust_lib,
        };
        unsafe {
            lib.get(b"driver\0")
                .unwrap_or_else(|e| panic!("symbol `driver` missing from {} .so: {e}", which.name()))
        }
    }

    /// Call `driver(x)` in the chosen implementation, capturing everything it
    /// writes to fd 1, and return the raw bytes.
    pub fn run(&self, which: Impl, x: i32) -> Vec<u8> {
        let f = self.sym(which);
        capture(|| unsafe { f(x) })
    }

    /// Call `driver(x)` `times` times back-to-back inside a single capture.
    pub fn run_repeated(&self, which: Impl, x: i32, times: usize) -> Vec<u8> {
        let f = self.sym(which);
        capture(|| {
            for _ in 0..times {
                unsafe { f(x) }
            }
        })
    }

    /// Call `driver(x)` with `stdout` forced into a specific buffering mode.
    pub fn run_with_bufmode(&self, which: Impl, x: i32, mode: c_int, size: usize) -> Vec<u8> {
        let f = self.sym(which);
        capture_with_bufmode(mode, size, || unsafe { f(x) })
    }

    /// Call `driver(x)` with fd 1 pointing at a descriptor that cannot be
    /// written to, so every `printf` inside the library fails.
    pub fn run_with_failing_stdout(&self, which: Impl, x: i32) -> FailedRun {
        let f = self.sym(which);
        capture_failing(|| unsafe { f(x) })
    }

    /// Stream `driver(x)`'s output through a pipe and hash it instead of
    /// buffering it, for input sizes whose output does not fit in memory.
    pub fn run_digest(&self, which: Impl, x: i32) -> Digest {
        let f = self.sym(which);
        capture_digest(|| unsafe { f(x) })
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn tmp_path() -> PathBuf {
    let n = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ))
}

/// Redirect fd 1 to a temporary file, run `f`, flush the C runtime, restore
/// fd 1, and return whatever bytes landed in the file.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let path = tmp_path();
    let file = std::fs::File::create(&path).expect("create temp capture file");

    unsafe {
        // Flush anything the harness itself may have queued so it cannot leak
        // into the captured region.
        fflush(std::ptr::null_mut());
    }
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }
    drop(file);

    let bytes = std::fs::read(&path).expect("read temp capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Same as [`capture`], but forces the C `stdout` stream into `mode` for the
/// duration of the call and restores full buffering afterwards.
pub fn capture_with_bufmode<F: FnOnce()>(mode: c_int, size: usize, f: F) -> Vec<u8> {
    let path = tmp_path();
    let file = std::fs::File::create(&path).expect("create temp capture file");

    unsafe {
        fflush(std::ptr::null_mut());
    }
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");
    unsafe {
        // setvbuf must follow the redirect and precede any I/O on the stream.
        setvbuf(stdout, std::ptr::null_mut(), mode, size);
    }

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        // Put the stream back into a sane fully-buffered state before the
        // descriptor changes underneath it again.
        setvbuf(stdout, std::ptr::null_mut(), IOFBF, 4096);
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }
    drop(file);

    let bytes = std::fs::read(&path).expect("read temp capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Result of running with a `stdout` that rejects every write.
#[derive(Debug, PartialEq, Eq)]
pub struct FailedRun {
    /// Bytes that reached anywhere observable (expected: none).
    pub bytes: Vec<u8>,
    /// `ferror(stdout)`-equivalent: did the stream end up in an error state?
    pub stream_error: bool,
    /// Did control return from the call at all?
    pub returned: bool,
}

extern "C" {
    fn ferror(stream: *mut c_void) -> c_int;
    fn clearerr(stream: *mut c_void);
}

/// Point fd 1 at `/dev/null` opened read-only, so `write(2)` fails with EBADF
/// on every `printf`, then run `f`.
pub fn capture_failing<F: FnOnce()>(f: F) -> FailedRun {
    // A read-only descriptor: writing to it fails with EBADF.
    let ro = std::fs::File::open("/dev/null").expect("open /dev/null read-only");

    unsafe {
        fflush(std::ptr::null_mut());
    }
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(ro.as_raw_fd(), 1) } >= 0, "dup2 failed");
    unsafe {
        clearerr(stdout);
        // Unbuffered so every single printf attempts (and fails) a write.
        setvbuf(stdout, std::ptr::null_mut(), IONBF, 0);
    }

    f();

    let stream_error = unsafe { ferror(stdout) != 0 };

    unsafe {
        fflush(std::ptr::null_mut());
        clearerr(stdout);
        setvbuf(stdout, std::ptr::null_mut(), IOFBF, 4096);
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }
    drop(ro);

    FailedRun {
        bytes: Vec::new(),
        stream_error,
        returned: true,
    }
}

/// Streamed digest of a capture: FNV-1a 64 over every byte, plus the byte and
/// line counts and the first/last chunk for diagnostics.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Digest {
    pub hash: u64,
    pub bytes: u64,
    pub lines: u64,
    pub head: Vec<u8>,
    pub tail: Vec<u8>,
}

pub fn digest_of(data: &[u8]) -> Digest {
    let mut h = Hasher::new();
    h.update(data);
    h.finish()
}

struct Hasher {
    hash: u64,
    bytes: u64,
    lines: u64,
    head: Vec<u8>,
    tail: std::collections::VecDeque<u8>,
}

const HEAD_TAIL: usize = 128;

impl Hasher {
    fn new() -> Hasher {
        Hasher {
            hash: 0xcbf2_9ce4_8422_2325,
            bytes: 0,
            lines: 0,
            head: Vec::with_capacity(HEAD_TAIL),
            tail: std::collections::VecDeque::with_capacity(HEAD_TAIL + 1),
        }
    }

    fn update(&mut self, buf: &[u8]) {
        let mut h = self.hash;
        let mut nl = 0u64;
        for &b in buf {
            h ^= b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
            nl += (b == b'\n') as u64;
        }
        self.hash = h;
        self.lines += nl;
        self.bytes += buf.len() as u64;

        // Head/tail bookkeeping, slice-wise rather than byte-wise.
        if self.head.len() < HEAD_TAIL {
            let take = (HEAD_TAIL - self.head.len()).min(buf.len());
            self.head.extend_from_slice(&buf[..take]);
        }
        let tail_src = &buf[buf.len().saturating_sub(HEAD_TAIL)..];
        if tail_src.len() >= HEAD_TAIL {
            self.tail.clear();
        } else {
            while self.tail.len() + tail_src.len() > HEAD_TAIL {
                self.tail.pop_front();
            }
        }
        self.tail.extend(tail_src.iter().copied());
    }

    fn finish(self) -> Digest {
        Digest {
            hash: self.hash,
            bytes: self.bytes,
            lines: self.lines,
            head: self.head,
            tail: self.tail.into_iter().collect(),
        }
    }
}

/// Redirect fd 1 into a pipe, hash everything the writer produces on a reader
/// thread, and return the digest. Used for outputs too large to materialise.
pub fn capture_digest<F: FnOnce()>(f: F) -> Digest {
    let mut fds = [0 as c_int; 2];
    assert!(unsafe { pipe(fds.as_mut_ptr()) } == 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    unsafe {
        fflush(std::ptr::null_mut());
    }
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(wr, 1) } >= 0, "dup2 failed");
    unsafe {
        close(wr);
    }

    let reader = std::thread::spawn(move || {
        let mut file = unsafe {
            <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(rd)
        };
        let mut hasher = Hasher::new();
        let mut buf = vec![0u8; 1 << 20];
        loop {
            match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => hasher.update(&buf[..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => panic!("pipe read failed: {e}"),
            }
        }
        hasher.finish()
    });

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        // Dropping our copy of the write end closes the pipe so the reader
        // sees EOF.
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }

    reader.join().expect("digest reader thread panicked")
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, reproducible across runs and platforms)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    /// xorshift64*
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform-ish value in `lo..=hi` (inclusive), for `i64` ranges.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }

    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        self.range_i64(lo as i64, hi as i64) as i32
    }

    pub fn any_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn preview(b: &[u8]) -> String {
    let n = b.len().min(200);
    format!(
        "{}{} (len {})",
        String::from_utf8_lossy(&b[..n]),
        if b.len() > n { "…" } else { "" },
        b.len()
    )
}

/// Assert C and Rust produced byte-identical output for the same input.
pub fn assert_same(label: &str, x: i32, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let first_diff = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| c.len().min(r.len()));
    panic!(
        "[{label}] divergence for driver({x}): C len={} Rust len={} first differing byte at {}\n\
         C   : {}\n\
         Rust: {}",
        c.len(),
        r.len(),
        first_diff,
        preview(&c[first_diff.saturating_sub(40)..(first_diff + 120).min(c.len())]),
        preview(&r[first_diff.saturating_sub(40)..(first_diff + 120).min(r.len())]),
    );
}

/// Independently recompute what the C loop must emit, as a cross-check that the
/// harness itself is capturing real output rather than nothing at all.
pub fn expected(x: i32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        out.extend_from_slice(format!("{i} {j}\n").as_bytes());
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
    out
}
