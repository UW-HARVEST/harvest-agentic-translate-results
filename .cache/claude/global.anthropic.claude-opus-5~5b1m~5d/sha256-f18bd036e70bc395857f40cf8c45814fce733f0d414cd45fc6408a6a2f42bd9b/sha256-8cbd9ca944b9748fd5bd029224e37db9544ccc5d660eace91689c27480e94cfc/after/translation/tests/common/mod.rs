//! Shared differential-testing harness.
//!
//! Loads BOTH the C `.so` (built by cmake from `c_src/`) and the Rust `.so`
//! (`cargo build --release`) with `libloading`, and compares what each one
//! writes to `stdout` byte-for-byte.
//!
//! Nothing here calls a Rust function directly — every call goes through
//! `dlsym` on the Rust cdylib, exactly as an external C consumer would, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::ffi::c_int;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Which implementation is under test
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate root has a parent")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let release = manifest_dir().join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    manifest_dir().join("target/debug/libdriver.so")
}

struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(
            cp.exists(),
            "C shared library not found at {cp:?}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(
            rp.exists(),
            "Rust shared library not found at {rp:?}. Build it with:\n  \
             cd translation && cargo build --release"
        );
        // RTLD_LOCAL (libloading's default) keeps the two libraries' identical
        // symbol names from interposing on one another.
        let c = unsafe { Library::new(&cp) }.expect("dlopen C .so");
        let rust = unsafe { Library::new(&rp) }.expect("dlopen Rust .so");
        Libs { c, rust }
    })
}

fn lib_of(which: Impl) -> &'static Library {
    let l = libs();
    match which {
        Impl::C => &l.c,
        Impl::Rust => &l.rust,
    }
}

/// Every exported symbol of the library, in the C signature it has.
pub type FnVoid = unsafe extern "C" fn();
pub type FnInt = unsafe extern "C" fn(c_int);
pub type FnStr = unsafe extern "C" fn(*const c_char);

fn sym<T>(which: Impl, name: &[u8]) -> Symbol<'static, T> {
    unsafe { lib_of(which).get::<T>(name) }
        .unwrap_or_else(|e| panic!("{} .so is missing symbol {:?}: {e}", which.name(), String::from_utf8_lossy(name)))
}

pub fn f_driver(which: Impl) -> Symbol<'static, FnInt> {
    sym::<FnInt>(which, b"driver\0")
}
pub fn f_bad(which: Impl) -> Symbol<'static, FnVoid> {
    sym::<FnVoid>(which, b"bad\0")
}
pub fn f_good(which: Impl) -> Symbol<'static, FnVoid> {
    sym::<FnVoid>(which, b"good\0")
}
pub fn f_print_int_line(which: Impl) -> Symbol<'static, FnInt> {
    sym::<FnInt>(which, b"printIntLine\0")
}
pub fn f_print_line(which: Impl) -> Symbol<'static, FnStr> {
    sym::<FnStr>(which, b"printLine\0")
}

/// Assert a symbol is present in both `.so`s (used by the symbol-parity test).
pub fn assert_symbol_in_both(name: &[u8]) {
    for w in BOTH {
        let r = unsafe { lib_of(w).get::<*const ()>(name) };
        assert!(
            r.is_ok(),
            "{} .so does not export {:?}",
            w.name(),
            String::from_utf8_lossy(name)
        );
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

extern "C" {
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// stdout is process-global state; captures must not overlap.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Run `f`, capturing everything the C-stdio `stdout` receives while it runs.
///
/// Both libraries print with libc `printf`, i.e. through the *same* process
/// `stdout` FILE, so redirecting fd 1 captures either one identically.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    // Flush anything already pending so it is not attributed to the callee.
    // Both the C stdio buffers (`fflush(NULL)`) and Rust's own `Stdout`
    // LineWriter (which holds libtest's unterminated "test foo ... " progress
    // line) must be drained, or their bytes would land inside the capture.
    unsafe {
        fflush(core::ptr::null_mut());
    }
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
    }

    let file = std::fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    unsafe {
        fflush(core::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);

    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

/// Run the same closure against both implementations and assert the captured
/// stdout is byte-identical.
///
/// `f` receives the implementation selector so it can look up that
/// implementation's exported symbols.
pub fn assert_same<F>(row: &str, mut f: F)
where
    F: FnMut(Impl),
{
    let c_out = capture(|| f(Impl::C));
    let rust_out = capture(|| f(Impl::Rust));
    if c_out != rust_out {
        panic!(
            "[{row}] stdout divergence\n  C    ({} bytes): {}\n  Rust ({} bytes): {}",
            c_out.len(),
            pretty(&c_out),
            rust_out.len(),
            pretty(&rust_out),
        );
    }
}

fn pretty(b: &[u8]) -> String {
    const MAX: usize = 400;
    let shown = &b[..b.len().min(MAX)];
    let mut s = String::from("\"");
    for &c in shown {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    s.push('"');
    if b.len() > MAX {
        s.push_str(&format!(" ... (+{} bytes)", b.len() - MAX));
    }
    s
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed, reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        lo + (self.next_u64() % ((hi - lo) as u64 + 1)) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Build a NUL-terminated C string from arbitrary non-NUL bytes.
pub fn cstr(bytes: &[u8]) -> Vec<c_char> {
    assert!(!bytes.contains(&0), "interior NUL is not representable");
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}
