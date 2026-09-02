//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C symbols — the Rust functions are never
//! called directly, so the `#[no_mangle]` / `extern "C"` wrappers are under test
//! too.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// libc bits we need in the tests themselves
// ---------------------------------------------------------------------------
unsafe extern "C" {
    pub fn free(p: *mut c_void);
    pub fn fork() -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    pub fn _exit(code: c_int) -> !;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn dup2(old: c_int, new: c_int) -> c_int;
}

pub type DropFn = unsafe extern "C" fn(*const c_char) -> *const c_char;
/// NOTE: the C prototype is `char *w_utf8_filter(const char *, _Bool)`.
/// `_Bool` is one byte, so `u8` is the ABI-identical parameter type and lets the
/// tests pass non-canonical boolean bytes (2, 0xFF, …) across the boundary.
pub type FilterFn = unsafe extern "C" fn(*const c_char, u8) -> *mut c_char;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    crate_root()
        .parent()
        .expect("crate root has a parent")
        .join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let rel = crate_root().join("target/release/libdriver.so");
    if rel.exists() {
        return rel;
    }
    crate_root().join("target/debug/libdriver.so")
}

/// A loaded driver: the `Library` is kept alive alongside the raw function
/// pointers extracted from it.
pub struct Driver {
    _lib: libloading::Library,
    pub drop_fn: DropFn,
    pub filter_fn: FilterFn,
    pub path: PathBuf,
}

impl Driver {
    pub fn open(path: &PathBuf) -> Driver {
        assert!(
            path.exists(),
            "shared object not found: {}\n\
             build the C side with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             build the Rust side with:\n  cd translation && cargo build --release",
            path.display()
        );
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
            let drop_fn: DropFn = *lib
                .get::<DropFn>(b"w_utf8_drop\0")
                .unwrap_or_else(|e| panic!("w_utf8_drop missing from {}: {e}", path.display()));
            let filter_fn: FilterFn = *lib
                .get::<FilterFn>(b"w_utf8_filter\0")
                .unwrap_or_else(|e| panic!("w_utf8_filter missing from {}: {e}", path.display()));
            Driver {
                _lib: lib,
                drop_fn,
                filter_fn,
                path: path.clone(),
            }
        }
    }
}

/// The pair under comparison.
pub struct Pair {
    pub c: Driver,
    pub rs: Driver,
}

pub fn pair() -> Pair {
    Pair {
        c: Driver::open(&c_so_path()),
        rs: Driver::open(&rust_so_path()),
    }
}

// ---------------------------------------------------------------------------
// Calling conveniences. `input` is always a NUL-terminated byte buffer.
// ---------------------------------------------------------------------------

fn as_cstr(input: &[u8]) -> Vec<u8> {
    assert!(
        !input.contains(&0),
        "test inputs must not contain an interior NUL"
    );
    let mut v = Vec::with_capacity(input.len() + 1);
    v.extend_from_slice(input);
    v.push(0);
    v
}

/// Calls `w_utf8_drop` and returns the *offset* of the returned pointer, which is
/// the only implementation-independent way to compare two pointers into two
/// different (but byte-identical) buffers.
pub fn call_drop(f: DropFn, cstr: &[u8]) -> usize {
    let base = cstr.as_ptr() as *const c_char;
    let ret = unsafe { f(base) };
    assert!(!ret.is_null(), "w_utf8_drop returned NULL");
    let off = ret as usize - base as usize;
    assert!(off < cstr.len(), "w_utf8_drop returned out-of-bounds offset");
    off
}

/// Calls `w_utf8_filter`, copies the result out, and `free()`s it (the C
/// contract: the buffer comes from `malloc`/`realloc`/`strdup`).
/// Returns `None` when the callee returned `NULL`.
pub fn call_filter(f: FilterFn, cstr: &[u8], replacement: u8) -> Option<Vec<u8>> {
    let out = unsafe { f(cstr.as_ptr() as *const c_char, replacement) };
    if out.is_null() {
        return None;
    }
    let mut bytes = Vec::new();
    unsafe {
        let mut p = out as *const u8;
        while *p != 0 {
            bytes.push(*p);
            p = p.add(1);
        }
        free(out as *mut c_void);
    }
    Some(bytes)
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 3);
    for x in b {
        s.push_str(&format!("{x:02X} "));
    }
    s.trim_end().to_string()
}

/// One differential assertion over `w_utf8_drop`.
pub fn assert_drop_eq(p: &Pair, input: &[u8], ctx: &str) {
    let a = as_cstr(input);
    let b = as_cstr(input);
    let c_off = call_drop(p.c.drop_fn, &a);
    let r_off = call_drop(p.rs.drop_fn, &b);
    assert_eq!(
        c_off,
        r_off,
        "w_utf8_drop offset mismatch [{ctx}]\n  input (len {}) = {}\n  C   -> {c_off}\n  Rust-> {r_off}",
        input.len(),
        hex(input)
    );
}

/// One differential assertion over `w_utf8_filter`.
pub fn assert_filter_eq(p: &Pair, input: &[u8], replacement: u8, ctx: &str) {
    let a = as_cstr(input);
    let b = as_cstr(input);
    let c_out = call_filter(p.c.filter_fn, &a, replacement);
    let r_out = call_filter(p.rs.filter_fn, &b, replacement);
    match (&c_out, &r_out) {
        (Some(cv), Some(rv)) => assert_eq!(
            cv,
            rv,
            "w_utf8_filter output mismatch [{ctx}] replacement={replacement}\n  \
             input (len {}) = {}\n  C   ({} bytes) = {}\n  Rust({} bytes) = {}",
            input.len(),
            hex(input),
            cv.len(),
            hex(cv),
            rv.len(),
            hex(rv)
        ),
        (None, None) => {}
        _ => panic!(
            "w_utf8_filter NULL-ness mismatch [{ctx}] replacement={replacement}\n  \
             input (len {}) = {}\n  C returned {:?}\n  Rust returned {:?}",
            input.len(),
            hex(input),
            c_out.as_ref().map(|v| v.len()),
            r_out.as_ref().map(|v| v.len())
        ),
    }
}

/// Every mode byte worth exercising: false, canonical true, and non-canonical
/// non-zero bytes (a C `_Bool` parameter accepts any byte over the FFI boundary).
pub const MODES: [u8; 7] = [0, 1, 2, 3, 0x7F, 0x80, 0xFF];
pub const CANONICAL_MODES: [u8; 2] = [0, 1];

/// Run drop + filter (all modes) against one input. This is the composed
/// pipeline check: both the low-level scanner and the wrapper on the same bytes.
pub fn assert_all_eq(p: &Pair, input: &[u8], ctx: &str) {
    assert_drop_eq(p, input, ctx);
    for m in MODES {
        assert_filter_eq(p, input, m, ctx);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    /// Uniform byte in `0x01..=0xFF` (never NUL — that would terminate the
    /// string early and is covered separately by the empty/short cases).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
    pub fn byte_in(&mut self, lo: u8, hi: u8) -> u8 {
        lo + self.below((hi - lo) as usize + 1) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Generators for the input SHAPES enumerated in CONFIGS.md
// ---------------------------------------------------------------------------

/// A valid 2-byte sequence: lead `0xC2..=0xDF`, continuation `0x80..=0xBF`.
pub fn valid2(rng: &mut Rng, out: &mut Vec<u8>) {
    out.push(rng.byte_in(0xC2, 0xDF));
    out.push(rng.byte_in(0x80, 0xBF));
}

/// A valid 3-byte sequence, honouring the `0xE0`/`0xED` second-byte splits.
pub fn valid3(rng: &mut Rng, out: &mut Vec<u8>) {
    let lead = rng.byte_in(0xE0, 0xEF);
    let c1 = match lead {
        0xE0 => rng.byte_in(0xA0, 0xBF),
        0xED => rng.byte_in(0x80, 0x9F),
        _ => rng.byte_in(0x80, 0xBF),
    };
    out.push(lead);
    out.push(c1);
    out.push(rng.byte_in(0x80, 0xBF));
}

/// A valid 4-byte sequence, honouring the `0xF0`/`0xF4` second-byte splits.
pub fn valid4(rng: &mut Rng, out: &mut Vec<u8>) {
    let lead = rng.byte_in(0xF0, 0xF4);
    let c1 = match lead {
        0xF0 => rng.byte_in(0x90, 0xBF),
        0xF4 => rng.byte_in(0x80, 0x8F),
        _ => rng.byte_in(0x80, 0xBF),
    };
    out.push(lead);
    out.push(c1);
    out.push(rng.byte_in(0x80, 0xBF));
    out.push(rng.byte_in(0x80, 0xBF));
}

pub fn valid1(rng: &mut Rng, out: &mut Vec<u8>) {
    out.push(rng.byte_in(0x01, 0x7F));
}

/// A fully valid UTF-8 string of `n` code points with the requested widths.
pub fn valid_mixed(rng: &mut Rng, n: usize, widths: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..n {
        match widths[rng.below(widths.len())] {
            1 => valid1(rng, &mut out),
            2 => valid2(rng, &mut out),
            3 => valid3(rng, &mut out),
            _ => valid4(rng, &mut out),
        }
    }
    out
}

/// A byte that `w_utf8_drop` is guaranteed to reject wherever it appears
/// (a bare continuation byte, or an always-illegal lead).
pub fn definitely_invalid_byte(rng: &mut Rng) -> u8 {
    const CHOICES: [u8; 6] = [0x80, 0xBF, 0xC0, 0xC1, 0xF5, 0xFF];
    CHOICES[rng.below(CHOICES.len())]
}

/// Uniform random non-NUL bytes.
pub fn random_bytes(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.nonzero_byte()).collect()
}

/// Random bytes biased towards `0xC0..=0xFF` so long invalid runs and
/// near-miss multi-byte sequences dominate.
pub fn biased_bytes(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| {
            if rng.below(4) == 0 {
                rng.byte_in(0x01, 0x7F)
            } else {
                rng.byte_in(0x80, 0xFF)
            }
        })
        .collect()
}

/// Exactly `count` bytes that are all individually rejected.
pub fn invalid_run(rng: &mut Rng, count: usize) -> Vec<u8> {
    (0..count).map(|_| definitely_invalid_byte(rng)).collect()
}

/// `valid_mixed` with the code-point count drawn from `lo..=hi` (avoids
/// double-borrowing the RNG at the call site).
pub fn valid_mixed_n(rng: &mut Rng, lo: usize, hi: usize, widths: &[u8]) -> Vec<u8> {
    let n = rng.range(lo, hi);
    valid_mixed(rng, n, widths)
}

/// `invalid_run` with the length drawn from `lo..=hi`.
pub fn invalid_run_n(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let n = rng.range(lo, hi);
    invalid_run(rng, n)
}
