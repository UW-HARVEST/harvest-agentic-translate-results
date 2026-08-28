//! Shared differential-test harness.
//!
//! Both implementations are loaded as **shared objects** with `libloading` and
//! called only through their exported `searchAndReplace` symbol:
//!
//! * C:    `c_src/build/libdriver.so`
//! * Rust: `translation/target/release/libdriver.so` (override with
//!         `RUST_DRIVER_SO=<path>` to test e.g. the debug artifact)
//!
//! No Rust function of the crate under test is ever called directly, so the
//! `#[no_mangle] extern "C"` export wrapper is part of what gets tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

/// ABI of the single public entry point (see `c_src/include/lib.h`).
pub type SearchAndReplaceFn =
    unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;

unsafe extern "C" {
    /// Used to release the buffers returned by either implementation; both
    /// allocate with the process' glibc `malloc`/`realloc`/`strdup`.
    fn free(p: *mut c_void);
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("../c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // Prefer the release artifact (the one that ships); fall back to the debug
    // artifact so that a plain `cargo test` works too.
    let release = manifest_dir().join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    let debug = manifest_dir().join("target/debug/libdriver.so");
    if debug.exists() {
        return debug;
    }
    release
}

fn load(path: PathBuf) -> SearchAndReplaceFn {
    assert!(
        path.exists(),
        "shared object {} not found.\n  build the C side with:\n    cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  build the Rust side with:\n    cd translation && cargo build --release",
        path.display()
    );
    // Leak the `Library` so the returned function pointer stays valid for the
    // whole process lifetime.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    }));
    let sym: Symbol<'static, SearchAndReplaceFn> = unsafe {
        lib.get(b"searchAndReplace\0")
            .unwrap_or_else(|e| panic!("dlsym(searchAndReplace) in {} failed: {e}", path.display()))
    };
    *sym
}

/// The C implementation, loaded from `libdriver.so` built by CMake.
pub fn c_impl() -> SearchAndReplaceFn {
    static F: OnceLock<SearchAndReplaceFn> = OnceLock::new();
    *F.get_or_init(|| load(c_so_path()))
}

/// The Rust implementation, loaded from the crate's `cdylib`.
pub fn rust_impl() -> SearchAndReplaceFn {
    static F: OnceLock<SearchAndReplaceFn> = OnceLock::new();
    *F.get_or_init(|| load(rust_so_path()))
}

unsafe fn strlen(p: *const c_char) -> usize {
    let mut n = 0usize;
    while unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    n
}

/// NUL-terminate `s` for passing across the FFI boundary.
pub fn cstr(s: &[u8]) -> Vec<u8> {
    assert!(
        !s.contains(&0),
        "test inputs must not contain interior NUL bytes"
    );
    let mut v = Vec::with_capacity(s.len() + 1);
    v.extend_from_slice(s);
    v.push(0);
    v
}

/// Call one implementation with already-NUL-terminated buffers.
///
/// Returns `None` when the implementation returned `NULL`, otherwise the bytes
/// of the returned C string (the buffer is `free`d before returning).
pub unsafe fn call_raw(
    f: SearchAndReplaceFn,
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> Option<Vec<u8>> {
    let p = unsafe { f(orig, search, value) };
    if p.is_null() {
        return None;
    }
    let n = unsafe { strlen(p) };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(unsafe { *p.add(i) } as u8);
    }
    unsafe { free(p as *mut c_void) };
    Some(out)
}

/// Call one implementation with byte-slice inputs.
pub fn call(f: SearchAndReplaceFn, orig: &[u8], search: &[u8], value: &[u8]) -> Option<Vec<u8>> {
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

pub fn show(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &c in b.iter().take(160) {
        match c {
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > 160 {
        s.push_str(&format!("\"...(+{} bytes)", b.len() - 160));
    } else {
        s.push('"');
    }
    s
}

fn showopt(o: &Option<Vec<u8>>) -> String {
    match o {
        None => "NULL".to_string(),
        Some(v) => format!("{} (len {})", show(v), v.len()),
    }
}

/// The core differential assertion: run both `.so`s and require byte-identical
/// results (including identical `NULL`-ness).
pub fn assert_same(row: &str, orig: &[u8], search: &[u8], value: &[u8]) -> Option<Vec<u8>> {
    let c = call(c_impl(), orig, search, value);
    let r = call(rust_impl(), orig, search, value);
    assert!(
        c == r,
        "[{row}] divergence\n  orig   = {}\n  search = {}\n  value  = {}\n  C    -> {}\n  Rust -> {}",
        show(orig),
        show(search),
        show(value),
        showopt(&c),
        showopt(&r)
    );
    c
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — property-style testing with a fixed seed.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.range(0, xs.len() - 1)]
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// `n` bytes drawn from `alphabet`.
    pub fn bytes(&mut self, n: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..n).map(|_| *self.pick(alphabet)).collect()
    }

    /// `n` bytes drawn uniformly from `1..=255` (never NUL, may be non-UTF-8).
    pub fn any_bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| (self.next_u64() % 255) as u8 + 1)
            .collect()
    }

    /// `range(lo, hi)` bytes from `alphabet` (avoids nested `&mut self` calls).
    pub fn bytes_range(&mut self, lo: usize, hi: usize, alphabet: &[u8]) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.bytes(n, alphabet)
    }

    /// `range(lo, hi)` bytes from `1..=255`.
    pub fn any_bytes_range(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.any_bytes(n)
    }
}

/// Alphabets used by the property sweeps.
pub const AB: &[u8] = b"ab";
pub const ABC: &[u8] = b"abc";
pub const HIGH: &[u8] = &[0x80, 0x81, 0xfe, 0xff, 0xc3, 0xa9];
