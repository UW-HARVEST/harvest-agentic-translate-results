//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C symbols. No Rust function from the
//! crate under test is ever called directly, so the `#[no_mangle]`/`extern "C"`
//! wrappers are part of what is being verified.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Mirror of the (private) C struct. Layout from lib.c:
//   typedef struct { char *data; int capacity; int length; } StringBuffer;
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

pub type FnCreateBuffer = unsafe extern "C" fn(c_int) -> *mut StringBuffer;
pub type FnAppendToBuffer = unsafe extern "C" fn(*mut StringBuffer, *const c_char) -> c_int;
pub type FnDestroyBuffer = unsafe extern "C" fn(*mut StringBuffer);
pub type FnGetOperationName = unsafe extern "C" fn(c_int) -> *const c_char;
pub type FnPerformOperation = unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int;
pub type FnBuffapp = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub create_buffer: FnCreateBuffer,
    pub append_to_buffer: FnAppendToBuffer,
    pub destroy_buffer: FnDestroyBuffer,
    pub get_operation_name: FnGetOperationName,
    pub perform_operation: FnPerformOperation,
    pub buffapp: FnBuffapp,
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    // Leak the Library so the extracted function pointers stay valid for the
    // whole process lifetime.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&path).unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", path.display(), name))
    }));
    macro_rules! sym {
        ($t:ty, $n:literal) => {
            *unsafe { lib.get::<$t>($n) }
                .unwrap_or_else(|e| panic!("{} missing symbol {:?}: {e}", name, $n))
        };
    }
    Impl {
        name,
        path,
        create_buffer: sym!(FnCreateBuffer, b"create_buffer\0"),
        append_to_buffer: sym!(FnAppendToBuffer, b"append_to_buffer\0"),
        destroy_buffer: sym!(FnDestroyBuffer, b"destroy_buffer\0"),
        get_operation_name: sym!(FnGetOperationName, b"get_operation_name\0"),
        perform_operation: sym!(FnPerformOperation, b"perform_operation\0"),
        buffapp: sym!(FnBuffapp, b"buffapp\0"),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir();
    let candidates = [
        base.join("c_src/build/libtranslated_rust.so"),
        base.join("c_src/build/libc_src.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    // Fall back to any .so directly inside c_src/build.
    if let Ok(rd) = std::fs::read_dir(base.join("c_src/build")) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                return p;
            }
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    // The integration-test executable lives in <target>/<profile>/deps/, so the
    // cdylib is one directory up.
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            dirs.push(deps.to_path_buf());
            if let Some(profile) = deps.parent() {
                dirs.push(profile.to_path_buf());
            }
        }
    }
    let base = manifest_dir();
    dirs.push(base.join("target/debug"));
    dirs.push(base.join("target/release"));
    for d in &dirs {
        let p = d.join("libbuffapp_lib.so");
        if p.exists() {
            // Guard against silently verifying a stale cdylib: `cargo test`
            // alone does not rebuild a `cdylib`-only lib target.
            let src = base.join("src/lib.rs");
            if let (Ok(a), Ok(b)) = (std::fs::metadata(&p), std::fs::metadata(&src)) {
                if let (Ok(ta), Ok(tb)) = (a.modified(), b.modified()) {
                    assert!(
                        ta >= tb,
                        "STALE cdylib: {} is older than src/lib.rs. \
                         Run `cargo build` before `cargo test` \
                         (`cargo test` does not rebuild a cdylib-only lib target).",
                        p.display()
                    );
                }
            }
            return p;
        }
    }
    panic!("Rust cdylib libbuffapp_lib.so not found; searched {dirs:?}. Run `cargo build` first.");
}

/// The two loaded implementations, C first.
pub fn impls() -> &'static (Impl, Impl) {
    static IMPLS: OnceLock<(Impl, Impl)> = OnceLock::new();
    IMPLS.get_or_init(|| (load("C", find_c_so()), load("RUST", find_rust_so())))
}

pub fn c() -> &'static Impl {
    &impls().0
}
pub fn rs() -> &'static Impl {
    &impls().1
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Inclusive range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Mixture of "interesting" magnitudes and uniform ints — the shapes the
    /// arithmetic in `perform_operation` / `buffapp` actually distinguishes.
    pub fn interesting_i32(&mut self) -> i32 {
        const SPECIAL: [i32; 16] = [
            0,
            1,
            -1,
            2,
            -2,
            3,
            -3,
            4,
            -4,
            7,
            -7,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
            65536,
        ];
        match self.below(4) {
            0 => SPECIAL[self.below(SPECIAL.len() as u32) as usize],
            1 => self.range_i32(-16, 16),
            2 => self.range_i32(-100_000, 100_000),
            _ => self.next_i32(),
        }
    }
    pub fn bytes(&mut self, len: usize, lo: u8, hi: u8) -> Vec<u8> {
        let span = (hi - lo) as u32 + 1;
        (0..len).map(|_| lo + self.below(span) as u8).collect()
    }
}

/// Iteration multiplier for the large randomized sweeps (`HARVEST_SOAK=50`
/// turns the default sweeps into a long soak run). Default 1.
pub fn soak() -> usize {
    std::env::var("HARVEST_SOAK")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(1)
}

// ---------------------------------------------------------------------------
// Observation trace: what a scenario records so C and Rust runs can be
// compared even though they operate on different heap addresses.
// ---------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Obs {
    /// Whether a returned pointer was NULL.
    IsNull(bool),
    /// `{capacity, length}` read out of a StringBuffer.
    Fields(c_int, c_int),
    /// An `int` return value.
    Ret(c_int),
    /// Raw bytes read out of a buffer.
    Bytes(Vec<u8>),
    /// A NUL-terminated string read through a returned `const char*`.
    CStr(Vec<u8>),
    /// A free-form marker so traces stay aligned/readable on mismatch.
    Mark(&'static str),
}

pub struct Trace(pub Vec<Obs>);

impl Trace {
    pub fn new() -> Self {
        Trace(Vec::new())
    }
    pub fn push(&mut self, o: Obs) {
        self.0.push(o);
    }
    pub fn mark(&mut self, m: &'static str) {
        self.0.push(Obs::Mark(m));
    }
    /// Record NULL-ness plus (if non-NULL) the struct fields.
    pub fn buf(&mut self, p: *mut StringBuffer) {
        self.push(Obs::IsNull(p.is_null()));
        if !p.is_null() {
            let b = unsafe { *p };
            self.push(Obs::Fields(b.capacity, b.length));
        }
    }
    /// Record `data[0 ..= length]` (the C string content plus its NUL).
    pub fn content(&mut self, p: *mut StringBuffer) {
        if p.is_null() {
            self.push(Obs::Bytes(Vec::new()));
            return;
        }
        let b = unsafe { *p };
        if b.data.is_null() || b.length < 0 {
            self.push(Obs::Bytes(Vec::new()));
            return;
        }
        let n = b.length as usize + 1;
        let s = unsafe { std::slice::from_raw_parts(b.data as *const u8, n) };
        self.push(Obs::Bytes(s.to_vec()));
    }
    /// Record `data[from ..= length]` — used when only a tail is deterministic.
    pub fn content_tail(&mut self, p: *mut StringBuffer, from: usize) {
        let b = unsafe { *p };
        let end = b.length as usize + 1;
        let from = from.min(end);
        let s = unsafe { std::slice::from_raw_parts((b.data as *const u8).add(from), end - from) };
        self.push(Obs::Bytes(s.to_vec()));
    }
}

pub fn read_cstr(p: *const c_char) -> Vec<u8> {
    assert!(!p.is_null(), "unexpected NULL string");
    let mut v = Vec::new();
    let mut i = 0isize;
    loop {
        let b = unsafe { *p.offset(i) as u8 };
        if b == 0 {
            break;
        }
        v.push(b);
        i += 1;
        assert!(i < 1 << 20, "unterminated string");
    }
    v
}

/// Run `scenario` against both implementations and assert identical traces.
pub fn diff<F>(label: &str, mut scenario: F)
where
    F: FnMut(&Impl, &mut Trace),
{
    let mut tc = Trace::new();
    scenario(c(), &mut tc);
    let mut tr = Trace::new();
    scenario(rs(), &mut tr);
    assert_traces(label, &tc, &tr);
}

pub fn assert_traces(label: &str, tc: &Trace, tr: &Trace) {
    if tc.0 == tr.0 {
        return;
    }
    let n = tc.0.len().max(tr.0.len());
    let mut msg = format!(
        "\nDIVERGENCE in `{label}`:\n  C trace has {} entries, RUST trace has {}\n",
        tc.0.len(),
        tr.0.len()
    );
    for i in 0..n {
        let a = tc.0.get(i);
        let b = tr.0.get(i);
        let flag = if a == b { "  " } else { "!!" };
        msg += &format!("{flag} [{i}] C={:?}\n     R={:?}\n", a, b);
    }
    panic!("{msg}");
}

// ---------------------------------------------------------------------------
// stdout capture (buffapp writes with libc `printf`; both .so's share the same
// glibc `stdout` FILE, so the bytes are directly comparable).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// A reusable stdout sink: one temp file, truncated and re-read per capture.
pub struct Cap {
    file: std::fs::File,
    path: PathBuf,
}

impl Cap {
    fn new() -> Cap {
        let path = std::env::temp_dir().join(format!("buffapp_diff_{}.out", std::process::id()));
        let file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("create stdout capture file");
        Cap { file, path }
    }

    fn run<R, F: FnOnce() -> R>(&mut self, f: F) -> (R, Vec<u8>) {
        use std::io::{Read, Seek, SeekFrom, Write};
        use std::os::unix::io::AsRawFd;
        self.file.set_len(0).expect("truncate");
        self.file.seek(SeekFrom::Start(0)).expect("rewind");
        let fd = self.file.as_raw_fd();
        let out;
        unsafe {
            // Flush whatever the process already had pending on the real stdout.
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(fd, 1) >= 0, "dup2 failed");
            out = f();
            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
            close(saved);
        }
        let _ = self.file.flush();
        self.file.seek(SeekFrom::Start(0)).expect("rewind");
        let mut bytes = Vec::new();
        self.file.read_to_end(&mut bytes).expect("read capture");
        (out, bytes)
    }
}

impl Drop for Cap {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn cap() -> &'static Mutex<Cap> {
    static C: OnceLock<Mutex<Cap>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(Cap::new()))
}

/// Redirect fd 1 to a temp file for the duration of `f`, returning `f`'s value
/// and everything written to stdout. Serialised process-wide.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let mut g = cap().lock().unwrap();
    g.run(f)
}

/// Guard that points fd 1 at `/dev/null` until dropped. Used for large
/// randomized sweeps where only return values are compared.
pub struct Discard {
    _g: std::sync::MutexGuard<'static, Cap>,
    saved: c_int,
}

impl Drop for Discard {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
        }
    }
}

pub fn discard_stdout() -> Discard {
    use std::os::unix::io::AsRawFd;
    let g = cap().lock().unwrap();
    let devnull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .expect("open /dev/null");
    let saved = unsafe {
        fflush(std::ptr::null_mut());
        let s = dup(1);
        assert!(s >= 0, "dup(1) failed");
        assert!(dup2(devnull.as_raw_fd(), 1) >= 0, "dup2 failed");
        s
    };
    Discard { _g: g, saved }
}

/// `buffapp` on both implementations: compare return value AND stdout bytes.
pub fn diff_buffapp(p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let (rc, oc) = capture_stdout(|| unsafe { (c().buffapp)(p1, p2, p3, p4) });
    let (rr, or) = capture_stdout(|| unsafe { (rs().buffapp)(p1, p2, p3, p4) });
    assert_eq!(
        rc, rr,
        "buffapp({p1}, {p2}, {p3}, {p4}) return value: C={rc} RUST={rr}"
    );
    if oc != or {
        panic!(
            "buffapp({p1}, {p2}, {p3}, {p4}) stdout mismatch:\n  C    ({} bytes) = {:?}\n  RUST ({} bytes) = {:?}",
            oc.len(),
            String::from_utf8_lossy(&oc),
            or.len(),
            String::from_utf8_lossy(&or)
        );
    }
    assert!(!oc.is_empty(), "buffapp produced no stdout at all — capture broken");
}

/// `buffapp` return value only. Caller must hold a [`Discard`] guard.
pub fn diff_buffapp_ret(p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let rc = unsafe { (c().buffapp)(p1, p2, p3, p4) };
    let rr = unsafe { (rs().buffapp)(p1, p2, p3, p4) };
    assert_eq!(
        rc, rr,
        "buffapp({p1}, {p2}, {p3}, {p4}) return value: C={rc} RUST={rr}"
    );
}

/// NUL-terminate a byte string for passing across FFI.
pub fn cstring(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

pub const OPS: [&[u8]; 4] = [b"add", b"subtract", b"multiply", b"divide"];

// ---------------------------------------------------------------------------
// Minimal sequential test runner (`harness = false`).
//
// The default libtest harness runs tests on several threads and prints its own
// progress lines to fd 1 from the main thread. Since `buffapp` writes to fd 1
// with libc `printf`, that progress output lands *inside* our stdout capture and
// corrupts the byte-for-byte comparison. Running the cases sequentially and
// sending all harness output to **stderr** makes the capture exact.
// ---------------------------------------------------------------------------
pub struct Runner {
    filter: Vec<String>,
    list: bool,
    passed: usize,
    failed: Vec<String>,
    filtered: usize,
}

impl Runner {
    pub fn new() -> Runner {
        let mut filter = Vec::new();
        let mut list = false;
        for a in std::env::args().skip(1) {
            if a == "--list" {
                list = true;
            } else if a.starts_with("--") {
                // ignore libtest-compatible flags (--nocapture, --test-threads, ...)
            } else {
                filter.push(a);
            }
        }
        Runner {
            filter,
            list,
            passed: 0,
            failed: Vec::new(),
            filtered: 0,
        }
    }

    pub fn case(&mut self, name: &str, f: fn()) {
        if self.list {
            println!("{name}: test");
            return;
        }
        if !self.filter.is_empty() && !self.filter.iter().any(|p| name.contains(p.as_str())) {
            self.filtered += 1;
            return;
        }
        eprint!("test {name} ... ");
        let t0 = std::time::Instant::now();
        match std::panic::catch_unwind(f) {
            Ok(()) => {
                self.passed += 1;
                eprintln!("ok ({:.2?})", t0.elapsed());
            }
            Err(_) => {
                self.failed.push(name.to_string());
                eprintln!("FAILED ({:.2?})", t0.elapsed());
            }
        }
    }

    pub fn finish(self) {
        if self.list {
            return;
        }
        eprintln!(
            "\nresult: {}. {} passed; {} failed; {} filtered out\n",
            if self.failed.is_empty() { "ok" } else { "FAILED" },
            self.passed,
            self.failed.len(),
            self.filtered
        );
        if !self.failed.is_empty() {
            for f in &self.failed {
                eprintln!("  FAILED: {f}");
            }
            std::process::exit(1);
        }
    }
}

/// Sanity: both `.so` files exist and were loaded.
pub fn assert_loaded() {
    let (a, b) = impls();
    assert!(Path::new(&a.path).exists());
    assert!(Path::new(&b.path).exists());
}
