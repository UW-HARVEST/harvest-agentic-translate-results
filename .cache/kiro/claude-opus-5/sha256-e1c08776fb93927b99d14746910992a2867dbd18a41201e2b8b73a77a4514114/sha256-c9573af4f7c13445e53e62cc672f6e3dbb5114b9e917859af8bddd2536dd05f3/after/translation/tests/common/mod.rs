//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported `searchAndReplace` symbol, exactly as an
//! external C consumer would. The Rust functions are never called directly, so
//! the `#[no_mangle]`/`extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

extern "C" {
    fn free(p: *mut c_void);
}

/// Signature of the symbol under test (`c_src/include/lib.h`).
pub type SearchAndReplaceFn =
    unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    /// Kept alive for the whole process so that `f` stays valid.
    _lib: Library,
    pub f: SearchAndReplaceFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`
pub fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

/// Every Rust `.so` we can find: the release artifact (the deliverable) and,
/// when present, the debug artifact (different codegen: overflow checks on,
/// `panic = "unwind"`).
pub fn rust_so_paths() -> Vec<(&'static str, PathBuf)> {
    let mut v = Vec::new();
    let release = manifest_dir().join("target/release/libdriver.so");
    if release.is_file() {
        v.push(("rust-release", release));
    }
    let debug = manifest_dir().join("target/debug/libdriver.so");
    if debug.is_file() {
        v.push(("rust-debug", debug));
    }
    v
}

fn load(name: &'static str, path: &Path) -> Impl {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    let f: SearchAndReplaceFn = unsafe {
        let sym = lib
            .get::<SearchAndReplaceFn>(b"searchAndReplace\0")
            .unwrap_or_else(|e| panic!("{} does not export searchAndReplace: {e}", path.display()));
        *sym
    };
    Impl {
        name,
        path: path.to_path_buf(),
        _lib: lib,
        f,
    }
}

pub fn c_impl() -> &'static Impl {
    static C: OnceLock<Impl> = OnceLock::new();
    C.get_or_init(|| {
        let p = c_so_path();
        assert!(
            p.is_file(),
            "C shared library missing at {} — build it with: cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            p.display()
        );
        load("c", &p)
    })
}

pub fn rust_impls() -> &'static Vec<Impl> {
    static R: OnceLock<Vec<Impl>> = OnceLock::new();
    R.get_or_init(|| {
        let paths = rust_so_paths();
        assert!(
            !paths.is_empty(),
            "no Rust shared library found — build it with: cd translation && cargo build --release"
        );
        paths.iter().map(|(n, p)| load(n, p)).collect()
    })
}

/// Result of one call: `None` for a `NULL` return, otherwise the bytes of the
/// returned C string (excluding the terminating NUL).
pub fn call(imp: &Impl, orig: &[u8], search: &[u8], value: &[u8]) -> Option<Vec<u8>> {
    let o = cstr(orig);
    let s = cstr(search);
    let v = cstr(value);
    unsafe {
        let ret = (imp.f)(o.as_ptr() as *const c_char, s.as_ptr() as *const c_char,
                          v.as_ptr() as *const c_char);
        if ret.is_null() {
            return None;
        }
        let mut out = Vec::new();
        let mut p = ret as *const u8;
        while *p != 0 {
            out.push(*p);
            p = p.add(1);
        }
        // The buffer comes from malloc/realloc/strdup in both implementations,
        // so the process allocator owns it and free() is the correct release.
        free(ret as *mut c_void);
        Some(out)
    }
}

fn cstr(b: &[u8]) -> Vec<u8> {
    assert!(!b.contains(&0), "test input must not contain interior NUL");
    let mut v = Vec::with_capacity(b.len() + 1);
    v.extend_from_slice(b);
    v.push(0);
    v
}

/// Core differential assertion: the C `.so` and every Rust `.so` must return
/// byte-identical results (including identical NULL-ness).
pub fn assert_same(orig: &[u8], search: &[u8], value: &[u8]) {
    let expected = call(c_impl(), orig, search, value);
    for imp in rust_impls() {
        let got = call(imp, orig, search, value);
        if got != expected {
            panic!(
                "divergence [{}] for\n  orig   = {}\n  search = {}\n  value  = {}\n  C    -> {}\n  RUST -> {}",
                imp.name,
                show(orig),
                show(search),
                show(value),
                show_opt(&expected),
                show_opt(&got),
            );
        }
    }
}

pub fn show(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &c in b {
        if c.is_ascii_graphic() || c == b' ' {
            s.push(c as char);
        } else {
            s.push_str(&format!("\\x{c:02x}"));
        }
    }
    s.push('"');
    format!("{s} (len {})", b.len())
}

pub fn show_opt(v: &Option<Vec<u8>>) -> String {
    match v {
        None => "NULL".to_string(),
        Some(b) => show(b),
    }
}

/// SplitMix64: tiny, deterministic, seedable PRNG so every randomized row is
/// reproducible.
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
    /// Uniform in `[0, n)`; `n > 0`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    /// Random byte from `alphabet`.
    pub fn pick(&mut self, alphabet: &[u8]) -> u8 {
        alphabet[self.below(alphabet.len())]
    }
    /// Random string of `len` bytes over `alphabet`.
    pub fn bytes(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len).map(|_| self.pick(alphabet)).collect()
    }
    /// Random string whose length is uniform in `[lo, hi]`, over `alphabet`.
    pub fn bytes_range(&mut self, lo: usize, hi: usize, alphabet: &[u8]) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.bytes(n, alphabet)
    }
}

/// Alphabets used by the rows in `CONFIGS.md`.
pub const AB: &[u8] = b"ab";
pub const ABC: &[u8] = b"abc";
pub const HIGH: &[u8] = &[
    0x80, 0x81, 0x9f, 0xa0, 0xc3, 0xe9, 0xfe, 0xff,
];
pub fn all_bytes() -> Vec<u8> {
    (1u8..=255).collect()
}
