//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes the six exported symbols of each behind an identical interface.
//!
//! Nothing in the crate under test is ever called directly — every call goes
//! through `dlsym`, exactly as an external C consumer would, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is being tested.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;
use std::process::Command;

// ---------------------------------------------------------------------------
// StringBuffer, as the C declares it.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

pub type CreateBufferFn = unsafe extern "C" fn(c_int) -> *mut StringBuffer;
pub type AppendToBufferFn = unsafe extern "C" fn(*mut StringBuffer, *const c_char) -> c_int;
pub type DestroyBufferFn = unsafe extern "C" fn(*mut StringBuffer);
pub type GetOperationNameFn = unsafe extern "C" fn(c_int) -> *const c_char;
pub type PerformOperationFn = unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int;
pub type BuffappFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub create_buffer: CreateBufferFn,
    pub append_to_buffer: AppendToBufferFn,
    pub destroy_buffer: DestroyBufferFn,
    pub get_operation_name: GetOperationNameFn,
    pub perform_operation: PerformOperationFn,
    pub buffapp: BuffappFn,
}

impl Impl {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        macro_rules! sym {
            ($t:ty, $s:literal) => {{
                let s: Symbol<$t> = lib
                    .get($s)
                    .unwrap_or_else(|e| panic!("{} missing symbol {:?}: {e}", name, $s));
                *s
            }};
        }
        let create_buffer = sym!(CreateBufferFn, b"create_buffer\0");
        let append_to_buffer = sym!(AppendToBufferFn, b"append_to_buffer\0");
        let destroy_buffer = sym!(DestroyBufferFn, b"destroy_buffer\0");
        let get_operation_name = sym!(GetOperationNameFn, b"get_operation_name\0");
        let perform_operation = sym!(PerformOperationFn, b"perform_operation\0");
        let buffapp = sym!(BuffappFn, b"buffapp\0");
        Impl {
            name,
            _lib: lib,
            create_buffer,
            append_to_buffer,
            destroy_buffer,
            get_operation_name,
            perform_operation,
            buffapp,
        }
    }
}

// ---------------------------------------------------------------------------
// Locating and (re)building the two shared objects.
// ---------------------------------------------------------------------------
pub fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    if !build.join("CMakeCache.txt").exists() {
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let ok = Command::new("cmake")
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .current_dir(&build)
            .status()
            .expect("run cmake")
            .success();
        assert!(ok, "cmake configure failed");
        let ok = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .status()
            .expect("run cmake --build")
            .success();
        assert!(ok, "cmake build failed");
    }
    let mut found = None;
    for entry in std::fs::read_dir(&build).expect("read c_src/build") {
        let p = entry.expect("dir entry").path();
        if p.extension().map(|e| e == "so").unwrap_or(false) {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

pub fn rust_so_path() -> PathBuf {
    // The RELEASE cdylib is the artifact under verification.
    //
    // This matters for fidelity, not just convention: with `-C
    // debug-assertions` (cargo's dev profile) rustc inserts null/alignment
    // checks on every raw-pointer dereference, so `append_to_buffer(NULL, ..)`
    // panics -> aborts (SIGABRT) instead of faulting (SIGSEGV) the way the C
    // does. The C `.so` is built by CMake with no such instrumentation, so the
    // release build is the apples-to-apples comparison.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Escape hatch used by tests/profile_divergence.sh to point the same suite
    // at a differently-built cdylib.
    if let Ok(p) = std::env::var("BUFFAPP_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "BUFFAPP_RUST_SO={} does not exist", p.display());
        return p;
    }
    let so = manifest.join("target/release/libbuffapp_lib.so");
    if !so.exists() {
        let ok = Command::new(env!("CARGO"))
            .args(["build", "--release", "--lib"])
            .current_dir(&manifest)
            .status()
            .expect("cargo build --release --lib")
            .success();
        assert!(ok, "cargo build --release --lib failed");
    }
    assert!(so.exists(), "missing {}", so.display());
    so
}

/// Both implementations, loaded once per test binary.
pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Impl::load("C", &c_so_path()),
            rs: Impl::load("Rust", &rust_so_path()),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed → reproducible property-style tests).
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Biased towards interesting values: extremes, small magnitudes, zero.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.next_u32() % 10 {
            0 => 0,
            1 => i32::MIN,
            2 => i32::MAX,
            3 => -1,
            4 => 1,
            5 => (self.next_u32() % 17) as i32 - 8,
            6 => i32::MIN + (self.next_u32() % 8) as i32,
            7 => i32::MAX - (self.next_u32() % 8) as i32,
            _ => self.next_i32(),
        }
    }
    pub fn range(&mut self, lo: u32, hi_inclusive: u32) -> u32 {
        lo + self.next_u32() % (hi_inclusive - lo + 1)
    }
    /// Random NUL-free byte string of `len` bytes (full 1..=255 byte range).
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.range(1, 255) as u8).collect()
    }
    /// Random NUL-terminated C string of `len` payload bytes.
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v = self.bytes(len);
        v.push(0);
        v
    }
    /// Random NUL-terminated C string whose payload length is drawn from
    /// `lo..=hi` (avoids nested `&mut self` borrows at call sites).
    pub fn cstring_len(&mut self, lo: u32, hi: u32) -> Vec<u8> {
        let n = self.range(lo, hi) as usize;
        self.cstring(n)
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers.
// ---------------------------------------------------------------------------
/// Snapshot of a `StringBuffer`'s observable state: the fields plus the bytes
/// of `data` up to `length` inclusive of the terminating NUL.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BufSnapshot {
    pub is_null: bool,
    pub data_is_null: bool,
    pub capacity: c_int,
    pub length: c_int,
    /// `data[0..=length]` when it is safe to read (length >= 0 and data non-null).
    pub bytes: Option<Vec<u8>>,
}

pub unsafe fn snapshot(buf: *mut StringBuffer) -> BufSnapshot {
    if buf.is_null() {
        return BufSnapshot {
            is_null: true,
            data_is_null: true,
            capacity: 0,
            length: 0,
            bytes: None,
        };
    }
    let b = &*buf;
    let bytes = if b.data.is_null() || b.length < 0 {
        None
    } else {
        let n = b.length as usize;
        Some(std::slice::from_raw_parts(b.data as *const u8, n + 1).to_vec())
    };
    BufSnapshot {
        is_null: false,
        data_is_null: b.data.is_null(),
        capacity: b.capacity,
        length: b.length,
        bytes,
    }
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

// ---------------------------------------------------------------------------
// stdout capture, so `buffapp`'s printf output can be compared byte-for-byte.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Runs `f` with fd 1 redirected into a temporary file and returns
/// `(f's value, captured stdout bytes)`.
///
/// fd 1 is process-global, so the redirection is serialized behind a mutex —
/// otherwise concurrently running tests would steal each other's output.
pub fn capture_stdout<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::sync::Mutex;
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut tmp = tempfile();
    unsafe {
        // Drain BOTH buffered writers that target fd 1: C's stdio FILE and
        // Rust's std::io::Stdout. Anything still sitting in a buffer would
        // otherwise be flushed into the capture file and look like library
        // output.
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let tmp_fd = fd_of(&tmp);
        assert!(dup2(tmp_fd, 1) >= 0, "dup2 into stdout failed");
        let out = f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        let mut bytes = Vec::new();
        tmp.seek(SeekFrom::Start(0)).expect("seek capture file");
        tmp.read_to_end(&mut bytes).expect("read capture file");
        (out, bytes)
    }
}

fn fd_of(f: &std::fs::File) -> c_int {
    use std::os::fd::AsRawFd;
    f.as_raw_fd()
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "buffapp_capture_{}_{}.txt",
        std::process::id(),
        n
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");
    // Unlink immediately; the fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Out-of-process runner, for the crashing / UB rows of ERRORS.md.
// ---------------------------------------------------------------------------
/// Outcome of running one scenario in a forked helper process.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Normal exit with this status code.
    Exit(i32),
    /// Killed by this signal number.
    Signal(i32),
}

/// Re-executes this test binary with `BUFFAPP_CRASH_CASE=<case>` and
/// `BUFFAPP_CRASH_IMPL=<impl>` set, and reports how the child terminated.
pub fn run_crash_case(case: &str, which: &str) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .arg("--exact")
        .arg("crash_helper::helper")
        .arg("--nocapture")
        .arg("--ignored")
        .env("BUFFAPP_CRASH_CASE", case)
        .env("BUFFAPP_CRASH_IMPL", which)
        .envs(std::env::var("BUFFAPP_RUST_SO").map(|v| ("BUFFAPP_RUST_SO".to_string(), v)))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn crash helper");
    match status.signal() {
        Some(s) => Outcome::Signal(s),
        None => Outcome::Exit(status.code().unwrap_or(-1)),
    }
}
