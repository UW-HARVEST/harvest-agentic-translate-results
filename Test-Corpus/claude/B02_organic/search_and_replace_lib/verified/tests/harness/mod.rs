//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects with `libloading` and
//! called through their exported `searchAndReplace` symbol — the Rust crate is
//! never called directly, so the `#[unsafe(no_mangle)] extern "C"` wrapper and
//! the C ABI are part of what is under test.
//!
//! * C   `.so`: `c_src/build/libdriver.so` (override with `DRIVER_C_SO`)
//! * Rust `.so`: `target/release/libdriver.so` (override with `DRIVER_RUST_SO`)

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type SarFn =
    unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    match std::env::var("DRIVER_C_SO") {
        Ok(p) => PathBuf::from(p),
        Err(_) => manifest().join("c_src/build/libdriver.so"),
    }
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest().join("target/release/libdriver.so");
    let dbg = manifest().join("target/debug/libdriver.so");
    if rel.exists() {
        rel
    } else if dbg.exists() {
        dbg
    } else {
        panic!(
            "no Rust cdylib found; run `cargo build --release --offline` first \
             (looked at {} and {})",
            rel.display(),
            dbg.display()
        )
    }
}

fn mtime(p: &PathBuf) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}

/// Guard against testing a stale artifact.
fn assert_fresh() {
    let so = rust_so_path();
    let src = manifest().join("src/lib.rs");
    if let (Some(a), Some(b)) = (mtime(&so), mtime(&src)) {
        assert!(
            a >= b,
            "{} is OLDER than src/lib.rs — rebuild the cdylib before testing",
            so.display()
        );
    }
    let cso = c_so_path();
    let csrc = manifest().join("c_src/src/lib.c");
    if let (Some(a), Some(b)) = (mtime(&cso), mtime(&csrc)) {
        assert!(a >= b, "{} is OLDER than c_src/src/lib.c — rebuild it", cso.display());
    }
}

/// Load both `.so`s once (leaked, so the symbols stay valid for the whole run)
/// and return `(c_fn, rust_fn)`.
pub fn fns() -> (SarFn, SarFn) {
    static FNS: OnceLock<(SarFn, SarFn)> = OnceLock::new();
    *FNS.get_or_init(|| {
        assert_fresh();
        unsafe { libc::atexit(report_calls) };
        let cpath = c_so_path();
        let rpath = rust_so_path();
        // Both libraries export `searchAndReplace`; `Library::new` uses
        // RTLD_LOCAL so each handle resolves to its own definition.
        let clib: &'static Library = Box::leak(Box::new(
            unsafe { Library::new(&cpath) }
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", cpath.display())),
        ));
        let rlib: &'static Library = Box::leak(Box::new(
            unsafe { Library::new(&rpath) }
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rpath.display())),
        ));
        let cs: Symbol<SarFn> = unsafe { clib.get(b"searchAndReplace\0") }
            .expect("C .so does not export searchAndReplace");
        let rs: Symbol<SarFn> = unsafe { rlib.get(b"searchAndReplace\0") }
            .expect("Rust .so does not export searchAndReplace");
        (*cs, *rs)
    })
}

/// Number of FFI calls made through this harness (both libraries counted).
pub static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

extern "C" fn report_calls() {
    eprintln!(
        "[harness] {} FFI calls made through the .so exports",
        CALLS.load(std::sync::atomic::Ordering::Relaxed)
    );
}

/// NUL-terminate a byte slice for passing as `const char *`.
pub fn cstr(b: &[u8]) -> Vec<u8> {
    assert!(!b.contains(&0), "test input must not contain an interior NUL");
    let mut v = Vec::with_capacity(b.len() + 1);
    v.extend_from_slice(b);
    v.push(0);
    v
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Outcome {
    pub null: bool,
    pub bytes: Vec<u8>,
}

/// Call one implementation through the FFI boundary; the returned buffer is
/// `free()`d (which also verifies it came from the C allocator, exactly like
/// the C version's `malloc`/`realloc`/`strdup` result).
pub unsafe fn call(f: SarFn, orig: &[u8], search: &[u8], value: &[u8]) -> Outcome {
    let o = cstr(orig);
    let s = cstr(search);
    let v = cstr(value);
    unsafe {
        call_raw(
            f,
            o.as_ptr() as *const c_char,
            s.as_ptr() as *const c_char,
            v.as_ptr() as *const c_char,
        )
    }
}

/// Like [`call`] but with raw, caller-supplied pointers (aliasing, NULL, ...).
pub unsafe fn call_raw(
    f: SarFn,
    o: *const c_char,
    s: *const c_char,
    v: *const c_char,
) -> Outcome {
    CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = unsafe { f(o, s, v) };
    if p.is_null() {
        return Outcome { null: true, bytes: Vec::new() };
    }
    let len = unsafe { libc::strlen(p) };
    let bytes = unsafe { std::slice::from_raw_parts(p as *const u8, len) }.to_vec();
    unsafe { libc::free(p as *mut c_void) };
    Outcome { null: false, bytes }
}

fn esc(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b.iter().take(400) {
        if c.is_ascii_graphic() || c == b' ' {
            s.push(c as char);
        } else {
            s.push_str(&format!("\\x{c:02x}"));
        }
    }
    if b.len() > 400 {
        s.push_str(&format!("...(+{} bytes)", b.len() - 400));
    }
    s
}

/// The core differential assertion: C and Rust must agree on NULL-ness and on
/// every returned byte. Returns the (identical) C outcome.
pub fn check(row: &str, orig: &[u8], search: &[u8], value: &[u8]) -> Outcome {
    // Guard: an empty `search` makes the C loop non-terminating and, when
    // `value` is non-empty, leaks memory until the allocator is exhausted
    // (ERRORS.md rows 10/11). Such inputs must only be run in a forked child
    // with RLIMIT_AS/alarm, never in-process.
    assert!(
        !search.is_empty(),
        "[{row}] empty `search` must not be exercised in-process — use the \
         fork-based helpers in tests/error_paths.rs (ERRORS.md rows 10/11)"
    );
    let (cf, rf) = fns();
    let c = unsafe { call(cf, orig, search, value) };
    let r = unsafe { call(rf, orig, search, value) };
    if c != r {
        panic!(
            "DIVERGENCE [{row}]\n  orig   ({} bytes) = \"{}\"\n  search ({} bytes) = \"{}\"\n  \
             value  ({} bytes) = \"{}\"\n  C   -> null={} len={} \"{}\"\n  RS  -> null={} len={} \"{}\"",
            orig.len(),
            esc(orig),
            search.len(),
            esc(search),
            value.len(),
            esc(value),
            c.null,
            c.bytes.len(),
            esc(&c.bytes),
            r.null,
            r.bytes.len(),
            esc(&r.bytes),
        );
    }
    c
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds keep every row reproducible.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// uniform-ish in `[0, n)`
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    /// inclusive range
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn pick(&mut self, alpha: &[u8]) -> u8 {
        alpha[self.below(alpha.len())]
    }
    pub fn bytes(&mut self, len: usize, alpha: &[u8]) -> Vec<u8> {
        (0..len).map(|_| self.pick(alpha)).collect()
    }
    /// random length in the inclusive range `[lo, hi]`
    pub fn bytes_r(&mut self, lo: usize, hi: usize, alpha: &[u8]) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.bytes(n, alpha)
    }
    /// random length in `[0, n)`
    pub fn bytes_b(&mut self, n: usize, alpha: &[u8]) -> Vec<u8> {
        let n = self.below(n);
        self.bytes(n, alpha)
    }
    /// n random lengths, each in the inclusive range `[lo, hi]`
    pub fn lens(&mut self, n: usize, lo: usize, hi: usize) -> Vec<usize> {
        (0..n).map(|_| self.range(lo, hi)).collect()
    }
}

// Alphabets chosen so that "filler" bytes can never accidentally form a
// `search` occurrence: FILL, SEARCH and VALUE are pairwise disjoint.
pub const FILL: &[u8] = b"abcdefgh";
pub const SEARCH: &[u8] = b"XY";
pub const VALUE: &[u8] = b"0123456789";

/// Count non-overlapping occurrences the way the C consumes them
/// (scan resumes right after each match) — used only to assert the SHAPE of a
/// generated input, never to predict the C's output.
pub fn count_matches(hay: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || needle.len() > hay.len() {
        return 0;
    }
    let mut i = 0;
    let mut n = 0;
    while i + needle.len() <= hay.len() {
        if &hay[i..i + needle.len()] == needle {
            n += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    n
}

pub fn first_match(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}
