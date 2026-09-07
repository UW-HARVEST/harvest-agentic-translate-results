//! Shared differential-test harness.
//!
//! Both libraries are loaded through `libloading` — the Rust side is *never*
//! called directly, so the `#[no_mangle] extern "C"` wrappers are part of what
//! is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type TimeT = i64;

/// `ComputationResult` as the C compiler lays it out on x86-64:
/// `int` + 4 pad + `time_t` + `int` + 4 tail pad = 24 bytes, align 8.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: TimeT,
    pub status: c_int,
}

pub const SIZEOF_COMPUTATION_RESULT: usize = 24;

pub type FnMathOp = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

/// Every exported symbol, resolved once per library.
pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub is_valid_operation: unsafe extern "C" fn(c_char) -> u8,
    pub get_operation_priority: unsafe extern "C" fn(c_int) -> c_int,
    pub add_operation: FnMathOp,
    pub multiply_operation: FnMathOp,
    pub subtract_operation: FnMathOp,
    pub divide_operation: FnMathOp,
    pub modulo_operation: FnMathOp,
    pub select_operation: unsafe extern "C" fn(c_int) -> *const c_void,
    pub get_computation_timestamp: unsafe extern "C" fn() -> TimeT,
    pub allocate_results: unsafe extern "C" fn(c_int) -> *mut ComputationResult,
    pub perform_computation_with_history:
        unsafe extern "C" fn(c_int, c_int, c_int, *mut *mut ComputationResult, *mut c_int) -> c_int,
    pub mathop: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

macro_rules! sym {
    ($lib:expr, $t:ty, $n:literal) => {{
        let s: Symbol<$t> = unsafe {
            $lib.get(concat!($n, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $n))
        };
        *s
    }};
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        assert!(path.exists(), "shared library not found: {}", path.display());
        // RTLD_NOW|RTLD_LOCAL so the two libraries' identically named symbols
        // never shadow each other.
        let lib = unsafe {
            libloading::os::unix::Library::open(
                Some(&path),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        }
        .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        let lib: Library = lib.into();

        Lib {
            name,
            is_valid_operation: sym!(lib, unsafe extern "C" fn(c_char) -> u8, "is_valid_operation"),
            get_operation_priority: sym!(
                lib,
                unsafe extern "C" fn(c_int) -> c_int,
                "get_operation_priority"
            ),
            add_operation: sym!(lib, FnMathOp, "add_operation"),
            multiply_operation: sym!(lib, FnMathOp, "multiply_operation"),
            subtract_operation: sym!(lib, FnMathOp, "subtract_operation"),
            divide_operation: sym!(lib, FnMathOp, "divide_operation"),
            modulo_operation: sym!(lib, FnMathOp, "modulo_operation"),
            select_operation: sym!(
                lib,
                unsafe extern "C" fn(c_int) -> *const c_void,
                "select_operation"
            ),
            get_computation_timestamp: sym!(
                lib,
                unsafe extern "C" fn() -> TimeT,
                "get_computation_timestamp"
            ),
            allocate_results: sym!(
                lib,
                unsafe extern "C" fn(c_int) -> *mut ComputationResult,
                "allocate_results"
            ),
            perform_computation_with_history: sym!(
                lib,
                unsafe extern "C" fn(
                    c_int,
                    c_int,
                    c_int,
                    *mut *mut ComputationResult,
                    *mut c_int,
                ) -> c_int,
                "perform_computation_with_history"
            ),
            mathop: sym!(lib, unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int, "mathop"),
            _lib: lib,
        }
    }

    /// Which of the five exported operation functions the given pointer is.
    /// Comparing this *index* is meaningful across libraries; comparing the raw
    /// addresses is not.
    pub fn op_identity(&self, p: *const c_void) -> &'static str {
        let cands: [(&'static str, FnMathOp); 5] = [
            ("add_operation", self.add_operation),
            ("multiply_operation", self.multiply_operation),
            ("subtract_operation", self.subtract_operation),
            ("divide_operation", self.divide_operation),
            ("modulo_operation", self.modulo_operation),
        ];
        for (n, f) in cands {
            if f as *const c_void == p {
                return n;
            }
        }
        if p.is_null() { "NULL" } else { "UNKNOWN" }
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no .so in {} — build the C library first",
        build.display()
    );
    found.remove(0)
}

fn find_rust_so() -> PathBuf {
    // Explicit override so the same suite can be run against the release
    // artifact (opt-level 3, `panic = "abort"`, debug-assertions off) as well as
    // the dev one.
    if let Ok(p) = std::env::var("MATHOP_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "MATHOP_RUST_SO points at a missing file: {}", p.display());
        assert_freshness(&p);
        return p;
    }
    // Prefer the profile the tests were built with, then fall back.
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["debug", "release"] {
        let p = base.join(prof).join("libmathop_lib.so");
        if p.exists() {
            assert_freshness(&p);
            return p;
        }
    }
    panic!(
        "libmathop_lib.so not found under {} — run `cargo build` first",
        base.display()
    );
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact, so a
/// stale `.so` would silently be tested. Refuse to run in that case.
fn assert_freshness(so: &std::path::Path) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let m = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|md| md.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    assert!(
        m(so) >= m(&src),
        "{} is older than src/lib.rs — run `cargo build` before `cargo test`",
        so.display()
    );
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static R_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c() -> &'static Lib {
    C_LIB.get_or_init(|| Lib::open("C", find_c_so()))
}

pub fn r() -> &'static Lib {
    R_LIB.get_or_init(|| Lib::open("RUST", find_rust_so()))
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — fixed seed, reproducible.
// ---------------------------------------------------------------------------

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
    pub fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as c_int
    }
    /// Biased toward small magnitudes and boundary values, so both the
    /// "interesting" and the full 32-bit range get covered.
    pub fn next_i32_mixed(&mut self) -> c_int {
        let r = self.next_u64();
        match r % 8 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => c_int::MIN,
            4 => c_int::MAX,
            5 => ((r >> 8) % 21) as c_int - 10,
            6 => ((r >> 8) % 2001) as c_int - 1000,
            _ => (r >> 32) as u32 as c_int,
        }
    }
    pub fn range(&mut self, lo: i64, hi_incl: i64) -> i64 {
        let span = (hi_incl - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
}

pub const BOUNDARIES: [c_int; 9] = [
    c_int::MIN,
    c_int::MIN + 1,
    -1000,
    -1,
    0,
    1,
    1000,
    c_int::MAX - 1,
    c_int::MAX,
];

// ---------------------------------------------------------------------------
// stdout capture: `mathop` printf()s, and that output is part of behaviour.
// Both .so files share the process's single stdout FILE*, so redirecting fd 1
// captures either side.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Runs `f` with fd 1 redirected into a temporary file and returns the bytes
/// written. Serialised by the caller (`CAPTURE_LOCK`).
pub fn capture_stdout<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "mathop_capture_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp)
        .expect("open capture file");

    unsafe {
        // Flush everything already buffered so it does not land in the capture.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        let out = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        let mut file = file;
        file.seek(SeekFrom::Start(0)).expect("seek");
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read capture");
        drop(file);
        let _ = std::fs::remove_file(&tmp);
        (out, buf)
    }
}

/// stdout redirection is process-global; tests that capture must hold this.
pub static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn bytes_dbg(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace('\n', "\\n")
}

/// Raw byte view of a `ComputationResult` buffer, including padding bytes —
/// this is what proves the struct layout matches.
pub unsafe fn raw_bytes(p: *const ComputationResult, count: usize) -> Vec<u8> {
    unsafe { std::slice::from_raw_parts(p as *const u8, count * SIZEOF_COMPUTATION_RESULT).to_vec() }
}
