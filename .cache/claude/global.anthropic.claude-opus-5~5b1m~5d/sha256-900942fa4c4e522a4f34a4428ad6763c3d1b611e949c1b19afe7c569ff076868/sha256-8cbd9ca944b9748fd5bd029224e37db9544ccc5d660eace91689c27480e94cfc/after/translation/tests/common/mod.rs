//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and calls `encode_base64` across the FFI boundary in both, so
//! the `#[no_mangle]` export wrapper itself is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type EncodeBase64 = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

unsafe extern "C" {
    fn free(p: *mut c_void);
}

pub struct Libs {
    c: Library,
    rs: Library,
}

// SAFETY: we only ever read function pointers out of the loaded libraries and
// the underlying code is re-entrant.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let root = repo_root();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/libdriver.dylib"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         looked in: {candidates:?}"
    );
}

fn find_rust_so() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib sits one directory up. Fall back to both profiles.
    let mut here = std::env::current_exe().expect("current_exe");
    here.pop(); // deps/
    here.pop(); // <profile>/
    let root = repo_root();
    let names = ["libdriver.so", "libdriver.dylib", "driver.dll"];
    let dirs = [
        here.clone(),
        root.join("translation/target/release"),
        root.join("translation/target/debug"),
    ];
    for d in &dirs {
        for n in &names {
            let p = d.join(n);
            if p.is_file() {
                return p;
            }
        }
    }
    panic!("Rust cdylib not found; run `cargo build` first. looked in: {dirs:?}");
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = find_c_so();
        let rp = find_rust_so();
        unsafe {
            Libs {
                c: Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {cp:?}: {e}")),
                rs: Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {rp:?}: {e}")),
            }
        }
    })
}

impl Libs {
    pub fn c_encode(&self) -> Symbol<'_, EncodeBase64> {
        unsafe { self.c.get(b"encode_base64\0") }.expect("C encode_base64 symbol")
    }
    pub fn rs_encode(&self) -> Symbol<'_, EncodeBase64> {
        unsafe { self.rs.get(b"encode_base64\0") }.expect("Rust encode_base64 symbol")
    }
}

/// Result of one call: either NULL, or the exact `n = size*4/3+4` bytes of the
/// returned allocation (`n` clamped at 0 so a `calloc(1,0)` result reads as an
/// empty byte string).
#[derive(PartialEq, Eq)]
pub enum Ret {
    Null,
    Buf(Vec<u8>),
}

impl std::fmt::Debug for Ret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ret::Null => write!(f, "NULL"),
            Ret::Buf(b) => write!(f, "Buf({:?} = {:?})", String::from_utf8_lossy(b), b),
        }
    }
}

/// Number of bytes the C `calloc(sizeof(char), size * 4 / 3 + 4)` requests,
/// reproducing the `int` arithmetic (including wraparound) exactly.
pub fn alloc_len(size: c_int) -> c_int {
    size.wrapping_mul(4).wrapping_div(3).wrapping_add(4)
}

/// The effective length the C loop uses: `size`, unless `size == 0`, in which
/// case `strlen(src)`.
fn effective_size(size: c_int, src: &[u8]) -> c_int {
    if size == 0 {
        let n = src.iter().position(|&b| b == 0).unwrap_or(src.len());
        n as c_int
    } else {
        size
    }
}

/// Upper bound on how many bytes of a returned allocation we touch.
///
/// `int` wraparound in `size * 4 / 3 + 4` can turn a negative `size` into a
/// multi-gigabyte `n` (e.g. `size = INT_MIN / 2` wraps to a huge positive).
/// `calloc` may well satisfy such a request under overcommit, but faulting in
/// gigabytes of pages just to memcmp zeros is pointless: the interesting
/// observable in those cases is NULL-vs-non-NULL plus the leading bytes. This
/// cap is far above every real payload used in the tests (max `n` ~5.5 KiB).
pub const MAX_COMPARE: usize = 1 << 20;

/// How many bytes of the returned allocation are meaningful to compare.
///
/// `alloc_len(effective_size)` when the allocation succeeded, clamped to
/// `0..=MAX_COMPARE`.
fn compare_len(size: c_int, src: &[u8]) -> usize {
    let eff = effective_size(size, src);
    let n = alloc_len(eff);
    if n <= 0 {
        0
    } else {
        (n as usize).min(MAX_COMPARE)
    }
}

unsafe fn call(f: &Symbol<'_, EncodeBase64>, size: c_int, src: *const c_char, n: usize) -> Ret {
    let p = unsafe { f(size, src) };
    if p.is_null() {
        return Ret::Null;
    }
    let bytes = unsafe { std::slice::from_raw_parts(p as *const u8, n) }.to_vec();
    unsafe { free(p as *mut c_void) };
    Ret::Buf(bytes)
}

/// Call C and Rust with the same `(size, src)` and return both results.
///
/// `src` is passed as a raw pointer to `payload`'s bytes. `payload` should
/// already contain any NUL terminator the configuration needs.
pub fn both(size: c_int, payload: &[u8]) -> (Ret, Ret) {
    let l = libs();
    let cf = l.c_encode();
    let rf = l.rs_encode();
    let n = compare_len(size, payload);
    let p = payload.as_ptr() as *const c_char;
    unsafe { (call(&cf, size, p, n), call(&rf, size, p, n)) }
}

/// Call both with a NULL `src`.
pub fn both_null(size: c_int) -> (Ret, Ret) {
    let l = libs();
    let cf = l.c_encode();
    let rf = l.rs_encode();
    unsafe {
        (
            call(&cf, size, std::ptr::null(), 0),
            call(&rf, size, std::ptr::null(), 0),
        )
    }
}

/// Assert byte-for-byte agreement, with a descriptive label.
#[track_caller]
pub fn assert_same(label: &str, size: c_int, payload: &[u8], c: Ret, r: Ret) {
    assert_eq!(
        c, r,
        "DIVERGENCE [{label}]: size={size} payload={payload:?}\n  C  = {c:?}\n  Rust = {r:?}"
    );
}

/// Run one differential case end to end.
#[track_caller]
pub fn check(label: &str, size: c_int, payload: &[u8]) {
    let (c, r) = both(size, payload);
    assert_same(label, size, payload, c, r);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every row is property-tested with a
// fixed seed and is fully reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// `len` random bytes over the full 0x00..=0xFF range.
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte()).collect()
    }
    /// `len` random bytes restricted to printable ASCII (never 0, never >=0x80).
    pub fn ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| 0x20 + (self.byte() % 0x5F)).collect()
    }
    /// `len` random bytes with the high bit set on roughly half of them, and
    /// never a NUL (safe for `strlen`-terminated payloads).
    pub fn high_bit(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let b = self.byte();
                if b == 0 { 0x80 } else { b }
            })
            .collect()
    }
}

/// Number of randomized samples per configuration row.
pub const SAMPLES: usize = 64;
