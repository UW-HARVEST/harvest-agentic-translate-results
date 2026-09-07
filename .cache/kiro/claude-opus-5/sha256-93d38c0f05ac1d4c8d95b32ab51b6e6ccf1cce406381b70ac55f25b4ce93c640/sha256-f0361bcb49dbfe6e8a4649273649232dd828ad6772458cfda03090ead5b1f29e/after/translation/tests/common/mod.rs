//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes them behind an identical interface, so every assertion in the
//! test suite compares two *dynamically loaded* libraries. The Rust functions
//! are never called directly — always through the `cdylib`'s exported symbols,
//! exactly as an external C consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;

pub type DropFn = unsafe extern "C" fn(*const c_char) -> *const c_char;
/// NOTE: `replacement` is deliberately typed as `u8` rather than `bool` so the
/// tests can push out-of-range `_Bool` values across the FFI boundary
/// (ERRORS.md row 23).
pub type FilterFn = unsafe extern "C" fn(*const c_char, u8) -> *mut c_char;
pub type FreeFn = unsafe extern "C" fn(*mut c_char);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub drop_fn: DropFn,
    pub filter_fn: FilterFn,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        assert!(
            path.exists(),
            "{} shared library not found at {}. Build it first \
             (C: cmake --build c_src/build ; Rust: cargo build --release).",
            name,
            path.display()
        );
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
            let drop_fn: Symbol<DropFn> = lib
                .get(b"w_utf8_drop\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol w_utf8_drop: {e}"));
            let filter_fn: Symbol<FilterFn> = lib
                .get(b"w_utf8_filter\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol w_utf8_filter: {e}"));
            let drop_fn = *drop_fn;
            let filter_fn = *filter_fn;
            Impl {
                name,
                _lib: lib,
                drop_fn,
                filter_fn,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load("C", c_so_path()),
        rs: Impl::load("Rust", rust_so_path()),
    }
}

extern "C" {
    fn free(p: *mut std::ffi::c_void);
}

/// Build a NUL-terminated C string from arbitrary bytes.
///
/// A `0x00` inside `bytes` would terminate the string early, which is a
/// property of C strings rather than of this library, so callers are expected
/// to pass NUL-free payloads; this is asserted to keep tests honest.
pub fn cstring(bytes: &[u8]) -> Vec<u8> {
    assert!(
        !bytes.contains(&0),
        "test payload must not contain an interior NUL"
    );
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    v
}

/// Call `w_utf8_drop` and return the *byte offset* of the returned pointer,
/// which is the implementation-independent observable.
pub fn call_drop(imp: &Impl, buf: &[u8]) -> usize {
    unsafe {
        let base = buf.as_ptr() as *const c_char;
        let ret = (imp.drop_fn)(base);
        assert!(!ret.is_null(), "{}: w_utf8_drop returned NULL", imp.name);
        let off = (ret as usize).wrapping_sub(base as usize);
        assert!(
            off < buf.len(),
            "{}: w_utf8_drop returned out-of-bounds offset {off} (buf len {})",
            imp.name,
            buf.len()
        );
        off
    }
}

/// Call `w_utf8_filter` and copy the returned C string out, then `free` it with
/// libc `free` (both implementations hand back `malloc`/`realloc`/`strdup`
/// memory, so `free` is the correct deallocator for both).
pub fn call_filter(imp: &Impl, buf: &[u8], replacement: u8) -> Option<Vec<u8>> {
    unsafe {
        let ret = (imp.filter_fn)(buf.as_ptr() as *const c_char, replacement);
        if ret.is_null() {
            return None;
        }
        let mut out = Vec::new();
        let mut p = ret as *const u8;
        while *p != 0 {
            out.push(*p);
            p = p.add(1);
        }
        free(ret as *mut std::ffi::c_void);
        Some(out)
    }
}

/// Deterministic xorshift64* PRNG so every randomized row is reproducible.
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
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// A non-zero byte (interior NUL is not a meaningful C-string input).
    pub fn nonzero_byte(&mut self) -> u8 {
        loop {
            let b = self.byte();
            if b != 0 {
                return b;
            }
        }
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Generators for the input shapes CONFIGS.md enumerates.
// ---------------------------------------------------------------------------

pub fn push_valid_1(out: &mut Vec<u8>, rng: &mut Rng) {
    out.push(rng.range(0x01, 0x7F) as u8);
}

pub fn push_valid_2(out: &mut Vec<u8>, rng: &mut Rng) {
    // lead 0xC2..=0xDF, continuation 0x80..=0xBF
    out.push(rng.range(0xC2, 0xDF) as u8);
    out.push(rng.range(0x80, 0xBF) as u8);
}

pub fn push_valid_3(out: &mut Vec<u8>, rng: &mut Rng) {
    loop {
        let b0 = rng.range(0xE0, 0xEF) as u8;
        let b1 = rng.range(0x80, 0xBF) as u8;
        let b2 = rng.range(0x80, 0xBF) as u8;
        if b0 == 0xE0 && b1 < 0xA0 {
            continue;
        }
        if b0 == 0xED && b1 >= 0xA0 {
            continue;
        }
        out.push(b0);
        out.push(b1);
        out.push(b2);
        return;
    }
}

pub fn push_valid_4(out: &mut Vec<u8>, rng: &mut Rng) {
    loop {
        let b0 = rng.range(0xF0, 0xF4) as u8;
        let b1 = rng.range(0x80, 0xBF) as u8;
        if b0 == 0xF0 && b1 < 0x90 {
            continue;
        }
        if b0 == 0xF4 && b1 > 0x8F {
            continue;
        }
        out.push(b0);
        out.push(b1);
        out.push(rng.range(0x80, 0xBF) as u8);
        out.push(rng.range(0x80, 0xBF) as u8);
        return;
    }
}

/// A sequence of `n` valid code points chosen from all four widths.
pub fn gen_valid_mixed(rng: &mut Rng, n: usize) -> Vec<u8> {    let mut out = Vec::new();
    for _ in 0..n {
        match rng.below(4) {
            0 => push_valid_1(&mut out, rng),
            1 => push_valid_2(&mut out, rng),
            2 => push_valid_3(&mut out, rng),
            _ => push_valid_4(&mut out, rng),
        }
    }
    out
}

/// Like [`gen_valid_mixed`] but picks the code-point count from `lo..=hi`
/// itself, so callers do not need two simultaneous mutable borrows of `rng`.
pub fn gen_valid_mixed_range(rng: &mut Rng, lo: usize, hi_inclusive: usize) -> Vec<u8> {
    let n = rng.range(lo, hi_inclusive);
    gen_valid_mixed(rng, n)
}

/// Bytes that are always rejected no matter what follows them.
pub const ALWAYS_INVALID: [u8; 12] = [
    0x80, 0x8F, 0xA0, 0xBF, // lone continuations
    0xC0, 0xC1, // overlong 2-byte leads
    0xF5, 0xF6, 0xF7, // lead > 0xF4
    0xF8, 0xFE, 0xFF, // invalid leads
];

pub fn invalid_byte(rng: &mut Rng) -> u8 {
    ALWAYS_INVALID[rng.below(ALWAYS_INVALID.len())]
}

/// Boundary-heavy interesting single bytes for exhaustive-ish sweeps.
pub const INTERESTING: [u8; 26] = [
    0x01, 0x41, 0x7F, 0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF, 0xC0, 0xC1, 0xC2, 0xDF, 0xE0, 0xE1,
    0xEC, 0xED, 0xEE, 0xEF, 0xF0, 0xF1, 0xF4, 0xF5, 0xF7, 0xF8, 0xFF,
];
