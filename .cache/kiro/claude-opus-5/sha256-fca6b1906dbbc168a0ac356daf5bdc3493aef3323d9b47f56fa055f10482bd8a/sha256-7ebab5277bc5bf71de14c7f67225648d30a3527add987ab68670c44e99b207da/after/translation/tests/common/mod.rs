//! Shared harness: loads the C `.so` and the Rust `.so` via `libloading` and
//! exposes their exported symbols so the two can be compared byte-for-byte.
//!
//! The Rust side is *never* called directly — everything goes through the
//! `cdylib`'s `#[no_mangle]` exports, exactly as an external C caller would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;

pub type DropFn = unsafe extern "C" fn(*const i8) -> *const i8;
pub type FilterFn = unsafe extern "C" fn(*const i8, bool) -> *mut i8;

unsafe extern "C" {
    fn free(p: *mut c_void);
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let root = repo_root();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/libdriver.dylib"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\ntried: {candidates:?}"
    );
}

fn rust_so_path() -> PathBuf {
    // Allows pointing the harness at a specific build (e.g. the debug cdylib,
    // which has integer-overflow checks enabled).
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO does not exist: {}", p.display());
        return p;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Prefer the profile the tests were built with, then fall back.
    let mut candidates = Vec::new();
    for profile in ["release", "debug"] {
        candidates.push(manifest.join(format!("target/{profile}/libdriver.so")));
        candidates.push(manifest.join(format!("target/{profile}/libdriver.dylib")));
    }
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Rust cdylib not found; run `cargo build --release` first. tried: {candidates:?}");
}

/// A loaded pair of implementations.
pub struct Impls {
    _c_lib: Library,
    _rs_lib: Library,
    pub c_drop: DropFn,
    pub rs_drop: FilterFnHolderDrop,
    pub c_filter: FilterFn,
    pub rs_filter: FilterFn,
}

// Small alias so the struct field names read nicely.
pub type FilterFnHolderDrop = DropFn;

impl Impls {
    pub fn load() -> Impls {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("failed to dlopen the C library");
            let rs_lib = Library::new(rust_so_path()).expect("failed to dlopen the Rust library");

            let c_drop: Symbol<DropFn> = c_lib
                .get(b"w_utf8_drop\0")
                .expect("C .so does not export w_utf8_drop");
            let rs_drop: Symbol<DropFn> = rs_lib
                .get(b"w_utf8_drop\0")
                .expect("Rust .so does not export w_utf8_drop");
            let c_filter: Symbol<FilterFn> = c_lib
                .get(b"w_utf8_filter\0")
                .expect("C .so does not export w_utf8_filter");
            let rs_filter: Symbol<FilterFn> = rs_lib
                .get(b"w_utf8_filter\0")
                .expect("Rust .so does not export w_utf8_filter");

            let (c_drop, rs_drop) = (*c_drop, *rs_drop);
            let (c_filter, rs_filter) = (*c_filter, *rs_filter);

            Impls {
                _c_lib: c_lib,
                _rs_lib: rs_lib,
                c_drop,
                rs_drop,
                c_filter,
                rs_filter,
            }
        }
    }

    /// `w_utf8_drop` returns an interior pointer; the observable value is its
    /// offset from the start of the input.
    pub fn drop_offsets(&self, input: &[u8]) -> (isize, isize) {
        let buf = nul_terminated(input);
        let base = buf.as_ptr() as *const i8;
        unsafe {
            let c = (self.c_drop)(base);
            let r = (self.rs_drop)(base);
            (
                c.offset_from(base),
                r.offset_from(base),
            )
        }
    }

    /// Call `w_utf8_filter` on both sides and return the two NUL-terminated
    /// results as owned byte vectors (the C-allocated buffers are freed).
    pub fn filter_outputs(
        &self,
        input: &[u8],
        replacement: bool,
    ) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        let buf = nul_terminated(input);
        let base = buf.as_ptr() as *const i8;
        unsafe {
            let c = (self.c_filter)(base, replacement);
            let r = (self.rs_filter)(base, replacement);
            let cv = take_c_string(c);
            let rv = take_c_string(r);
            (cv, rv)
        }
    }
}

/// Copy `p`'s contents into a `Vec` and `free` the original allocation.
///
/// Both libraries hand back `malloc`ed memory from the process's libc, so the
/// same `free` is valid for either.
unsafe fn take_c_string(p: *mut i8) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    unsafe {
        let mut n = 0usize;
        while *p.add(n) != 0 {
            n += 1;
        }
        let v = std::slice::from_raw_parts(p as *const u8, n).to_vec();
        free(p as *mut c_void);
        Some(v)
    }
}

/// Build a NUL-terminated buffer. Panics if `input` already contains a NUL,
/// which would make the test case meaningless.
pub fn nul_terminated(input: &[u8]) -> Vec<u8> {
    assert!(
        !input.contains(&0),
        "test inputs must not contain interior NUL bytes"
    );
    let mut v = Vec::with_capacity(input.len() + 1);
    v.extend_from_slice(input);
    v.push(0);
    v
}

/// Assert both implementations agree for every relevant call on `input`.
pub fn assert_agree(impls: &Impls, input: &[u8]) {
    let (c_off, r_off) = impls.drop_offsets(input);
    assert_eq!(
        c_off, r_off,
        "w_utf8_drop offset mismatch for input {:02X?} (c={c_off}, rust={r_off})",
        input
    );

    for replacement in [false, true] {
        let (c, r) = impls.filter_outputs(input, replacement);
        assert_eq!(
            c.as_deref().map(hex),
            r.as_deref().map(hex),
            "w_utf8_filter(replacement={replacement}) mismatch for input {:02X?}",
            input
        );
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect()
}

/// Deterministic xorshift PRNG so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// A byte in 1..=255 (never 0, which would terminate the string).
    pub fn byte(&mut self) -> u8 {
        1u8.wrapping_add((self.next_u64() % 255) as u8)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Byte values that sit on every boundary the C macros test.
pub const INTERESTING: &[u8] = &[
    0x01, 0x41, 0x7F, // ASCII / valid_1 edges
    0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF, // continuation-byte edges
    0xC0, 0xC1, 0xC2, 0xDF, // 2-byte lead edges (overlong C0/C1)
    0xE0, 0xE1, 0xEC, 0xED, 0xEE, 0xEF, // 3-byte lead edges (E0 overlong, ED surrogates)
    0xF0, 0xF1, 0xF4, 0xF5, 0xF7, // 4-byte lead edges (F0 overlong, >F4 invalid)
    0xF8, 0xFF, // invalid lead bytes
];
