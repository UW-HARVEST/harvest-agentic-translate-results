//! Shared differential-test harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and driven **only** through their exported C symbols, exactly as
//! an external consumer would.  No Rust function is ever called directly.
//!
//! The library's sole observable is the byte stream it writes to the process
//! `stdout`, so the harness captures fd 1 into a temporary file around each
//! invocation and compares the two byte streams.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need (declared directly so no extra dependency is required)
// ---------------------------------------------------------------------------

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    /// glibc's `stdout` — the `FILE*` that `printf(3)` and `puts(3)` write to.
    static mut stdout: *mut c_void;
}

/// Read glibc's `stdout` `FILE*` without forming a reference to a `static mut`.
unsafe fn get_c_stdout() -> *mut c_void {
    std::ptr::read(std::ptr::addr_of!(stdout))
}

/// Point glibc's `stdout` at a different stream.
unsafe fn set_c_stdout(f: *mut c_void) {
    std::ptr::write(std::ptr::addr_of_mut!(stdout), f);
}

pub const IOFBF: c_int = 0; // fully buffered
pub const IOLBF: c_int = 1; // line buffered
pub const IONBF: c_int = 2; // unbuffered

/// Buffering mode applied to the capture stream that temporarily *is* the libc
/// `stdout` seen by both shared objects.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Buffering {
    /// `A_pipe` — glibc's default for a regular file (fully buffered, `BUFSIZ`).
    Default,
    /// `A_unbuf` — `setvbuf(f, NULL, _IONBF, 0)`.
    Unbuffered,
    /// `A_line` — `setvbuf(f, NULL, _IOLBF, 1024)`.
    LineBuffered,
    /// explicit `_IOFBF` with a 4 KiB buffer.
    FullyBuffered,
}

// ---------------------------------------------------------------------------
// Loading the two shared objects
// ---------------------------------------------------------------------------

/// Raw C-ABI entry points of one shared object.
pub struct Api {
    pub name: &'static str,
    pub print_line: unsafe extern "C" fn(*const c_char),
    pub print_int_line: unsafe extern "C" fn(c_int),
    pub bad: unsafe extern "C" fn(),
    pub good: unsafe extern "C" fn(),
    pub driver: unsafe extern "C" fn(),
}

impl Api {
    /// `printIntLine` re-typed to take a 64-bit argument, so a value wider than
    /// the declared `int` parameter can be pushed through the ABI slot (G10).
    pub fn print_int_line_wide(&self) -> unsafe extern "C" fn(i64) {
        unsafe { std::mem::transmute(self.print_int_line) }
    }

    /// A `void (void)` symbol re-typed to accept three extra register
    /// arguments, so junk can be supplied across the FFI boundary (G11).
    pub fn extra_args(f: unsafe extern "C" fn()) -> unsafe extern "C" fn(c_int, c_int, c_int) {
        unsafe { std::mem::transmute(f) }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?} — build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Locate the Rust `cdylib`.  Preference order: the profile directory the test
/// binary itself lives in, then `release`, then `debug`.
fn rust_so_path() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test bin>
        for anc in exe.ancestors().skip(1).take(3) {
            candidates.push(anc.join("libdriver.so"));
        }
    }
    let target = manifest_dir().join("target");
    candidates.push(target.join("release/libdriver.so"));
    candidates.push(target.join("debug/libdriver.so"));

    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found; looked in {candidates:?}. \
         Build it with `cargo build` / `cargo build --release`."
    );
}

unsafe fn load(path: &PathBuf, name: &'static str) -> Api {
    let lib = libloading::Library::new(path)
        .unwrap_or_else(|e| panic!("failed to dlopen {path:?}: {e}"));
    // Leaked on purpose: the resolved function pointers must stay valid for the
    // whole test-binary lifetime.
    let lib: &'static libloading::Library = Box::leak(Box::new(lib));

    macro_rules! sym {
        ($t:ty, $n:expr) => {{
            let s: libloading::Symbol<'static, $t> = lib
                .get($n)
                .unwrap_or_else(|e| panic!("{} is missing symbol {:?}: {}", name, $n, e));
            *s
        }};
    }

    Api {
        name,
        print_line: sym!(unsafe extern "C" fn(*const c_char), b"printLine\0"),
        print_int_line: sym!(unsafe extern "C" fn(c_int), b"printIntLine\0"),
        bad: sym!(unsafe extern "C" fn(), b"bad\0"),
        good: sym!(unsafe extern "C" fn(), b"good\0"),
        driver: sym!(unsafe extern "C" fn(), b"driver\0"),
    }
}

pub fn c_api() -> &'static Api {
    static C: OnceLock<Api> = OnceLock::new();
    C.get_or_init(|| unsafe { load(&c_so_path(), "C libdriver.so") })
}

pub fn rust_api() -> &'static Api {
    static R: OnceLock<Api> = OnceLock::new();
    R.get_or_init(|| unsafe { load(&rust_so_path(), "Rust libdriver.so") })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// glibc's `stdout` is process-global, so captures must never overlap.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Directory for capture scratch files.  Prefer the crate's own `target/` (always
/// writable) and fall back to `TMPDIR`.
fn scratch_dir() -> PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let mut cands: Vec<PathBuf> = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(profile) = exe.parent().and_then(|p| p.parent()) {
                cands.push(profile.join("driver-difftest"));
            }
        }
        cands.push(manifest_dir().join("target/driver-difftest"));
        if let Some(t) = std::env::var_os("TMPDIR") {
            cands.push(PathBuf::from(t).join("driver-difftest"));
        }
        cands.push(std::env::temp_dir().join("driver-difftest"));
        for c in cands {
            if std::fs::create_dir_all(&c).is_ok() {
                // prove it is really writable
                let probe = c.join(".probe");
                if std::fs::write(&probe, b"x").is_ok() {
                    let _ = std::fs::remove_file(&probe);
                    return c;
                }
            }
        }
        panic!("no writable scratch directory found for capture files");
    })
    .clone()
}

static CAPTURE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Run `f` with glibc's `stdout` temporarily pointing at a fresh `FILE*` over a
/// scratch file, and return the bytes the libraries wrote.
///
/// Swapping the `FILE*` (rather than `dup2`-ing fd 1) is deliberate: `printf` and
/// `puts` inside *both* shared objects go through this global, while Rust's own
/// `std::io::stdout()` — and therefore libtest's progress output — writes to
/// fd 1 directly and cannot contaminate the capture.
///
/// `mode` is applied with `setvbuf` immediately after `fopen`, i.e. before any
/// I/O has happened on the stream, which is the only point at which `setvbuf` is
/// well defined.
pub fn capture_with(mode: Buffering, f: impl FnOnce()) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let n = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = scratch_dir().join(format!("cap_{}_{}.out", std::process::id(), n));
    let cpath = {
        let mut v = path.as_os_str().as_encoded_bytes().to_vec();
        v.push(0);
        v
    };

    let stream = unsafe {
        fopen(
            cpath.as_ptr() as *const c_char,
            b"w\0".as_ptr() as *const c_char,
        )
    };
    assert!(!stream.is_null(), "fopen({path:?}) failed");

    unsafe {
        match mode {
            Buffering::Default => {}
            Buffering::Unbuffered => {
                setvbuf(stream, std::ptr::null_mut(), IONBF, 0);
            }
            Buffering::LineBuffered => {
                setvbuf(stream, std::ptr::null_mut(), IOLBF, 1024);
            }
            Buffering::FullyBuffered => {
                setvbuf(stream, std::ptr::null_mut(), IOFBF, 4096);
            }
        }
    }

    let saved = unsafe { get_c_stdout() };
    unsafe { fflush(saved) };
    unsafe { set_c_stdout(stream) };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe {
        fflush(stream);
        set_c_stdout(saved);
        fclose(stream);
    }

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);

    match result {
        Ok(()) => bytes,
        Err(p) => std::panic::resume_unwind(p),
    }
}

/// Capture with the default (fully buffered onto a file) mode — row `A_pipe`.
pub fn capture(f: impl FnOnce()) -> Vec<u8> {
    capture_with(Buffering::Default, f)
}

/// Run `f` with glibc's `stdout` pointing at a stream that is **open for reading
/// only**, so every `printf`/`puts` inside the library fails (`EBADF`).
///
/// Neither the C nor the Rust implementation checks the `printf` return value, so
/// both must silently ignore the failure, write nothing, and return normally.
/// Returns `(bytes_written, ferror_flag)`.
pub fn run_with_write_failing_stdout(f: impl FnOnce()) -> (Vec<u8>, bool) {
    extern "C" {
        fn ferror(stream: *mut c_void) -> c_int;
    }

    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let n = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = scratch_dir().join(format!("ro_{}_{}.out", std::process::id(), n));
    std::fs::write(&path, b"").expect("create read-only scratch file");
    let cpath = {
        let mut v = path.as_os_str().as_encoded_bytes().to_vec();
        v.push(0);
        v
    };

    // "r" => the underlying fd is O_RDONLY, so write(2) returns EBADF.
    let stream = unsafe {
        fopen(
            cpath.as_ptr() as *const c_char,
            b"r\0".as_ptr() as *const c_char,
        )
    };
    assert!(!stream.is_null(), "fopen({path:?}, \"r\") failed");
    // Unbuffered, so the failure surfaces on every single call rather than only
    // at the final flush.
    unsafe { setvbuf(stream, std::ptr::null_mut(), IONBF, 0) };

    let saved = unsafe { get_c_stdout() };
    unsafe { fflush(saved) };
    unsafe { set_c_stdout(stream) };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    let err = unsafe {
        fflush(stream);
        let e = ferror(stream) != 0;
        set_c_stdout(saved);
        fclose(stream);
        e
    };

    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);

    match result {
        Ok(()) => (bytes, err),
        Err(p) => std::panic::resume_unwind(p),
    }
}

// ---------------------------------------------------------------------------
// Operation script — the vocabulary of calls we can drive both libraries with
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Op {
    /// `printLine(<bytes>)` — a NUL terminator is appended by the harness.
    PrintLine(Vec<u8>),
    /// `printLine(NULL)`
    PrintLineNull,
    /// `printIntLine(<v>)`
    PrintIntLine(i32),
    /// `printIntLine` invoked with a 64-bit value in the `int` ABI slot.
    PrintIntLineWide(i64),
    Bad,
    Good,
    Driver,
    /// `bad()` / `good()` / `driver()` called with three junk register args.
    BadExtraArgs(c_int, c_int, c_int),
    GoodExtraArgs(c_int, c_int, c_int),
    DriverExtraArgs(c_int, c_int, c_int),
}

/// Apply an operation script to one library.  Must be called inside `capture`.
pub fn apply(api: &Api, ops: &[Op]) {
    for op in ops {
        unsafe {
            match op {
                Op::PrintLine(bytes) => {
                    let mut z = bytes.clone();
                    assert!(
                        !z.contains(&0),
                        "test payload must not contain interior NUL bytes"
                    );
                    z.push(0);
                    (api.print_line)(z.as_ptr() as *const c_char);
                }
                Op::PrintLineNull => (api.print_line)(std::ptr::null()),
                Op::PrintIntLine(v) => (api.print_int_line)(*v),
                Op::PrintIntLineWide(v) => (api.print_int_line_wide())(*v),
                Op::Bad => (api.bad)(),
                Op::Good => (api.good)(),
                Op::Driver => (api.driver)(),
                Op::BadExtraArgs(a, b, c) => Api::extra_args(api.bad)(*a, *b, *c),
                Op::GoodExtraArgs(a, b, c) => Api::extra_args(api.good)(*a, *b, *c),
                Op::DriverExtraArgs(a, b, c) => Api::extra_args(api.driver)(*a, *b, *c),
            }
        }
    }
}

fn show(bytes: &[u8]) -> String {
    let head: Vec<u8> = bytes.iter().copied().take(400).collect();
    format!(
        "{} bytes, first {}: {:?}{}",
        bytes.len(),
        head.len(),
        String::from_utf8_lossy(&head),
        if bytes.len() > head.len() { " …" } else { "" }
    )
}

/// Run `ops` against both libraries under `mode` and assert byte equality.
/// Returns the (shared) output for further assertions.
pub fn diff_with(label: &str, mode: Buffering, ops: &[Op]) -> Vec<u8> {
    let c_out = capture_with(mode, || apply(c_api(), ops));
    let r_out = capture_with(mode, || apply(rust_api(), ops));
    if c_out != r_out {
        let at = c_out
            .iter()
            .zip(r_out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c_out.len().min(r_out.len()));
        panic!(
            "DIVERGENCE [{label}] mode={mode:?} first differing byte index {at}\n  ops  = {ops:?}\n  C    = {}\n  RUST = {}",
            show(&c_out),
            show(&r_out)
        );
    }
    c_out
}

pub fn diff(label: &str, ops: &[Op]) -> Vec<u8> {
    diff_with(label, Buffering::Default, ops)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        if span == 0 {
            self.next_u64() as i64
        } else {
            lo + (self.next_u64() % span) as i64
        }
    }
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        self.range_i64(lo as i64, hi as i64) as usize
    }
    /// Random byte in `0x01..=0xFF` (never NUL — that would terminate early).
    pub fn byte_nonzero(&mut self) -> u8 {
        (self.range_i64(1, 255)) as u8
    }
    /// Random printable ASCII byte `0x20..=0x7E`.
    pub fn byte_printable(&mut self) -> u8 {
        (self.range_i64(0x20, 0x7E)) as u8
    }
    pub fn bytes_printable(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte_printable()).collect()
    }
    pub fn bytes_any(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte_nonzero()).collect()
    }
}
