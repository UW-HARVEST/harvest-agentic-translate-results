//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls `custom_strdup` through the FFI boundary in both, so the
//! `#[no_mangle]` export wrapper is exercised exactly as an external C caller
//! would exercise it. No Rust function is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type CustomStrdupFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

unsafe extern "C" {
    pub fn free(p: *mut std::ffi::c_void);
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workdir() -> PathBuf {
    crate_root().parent().expect("crate has a parent dir").to_path_buf()
}

/// Path to the C shared library produced by `c_src/CMakeLists.txt`.
fn c_so_path() -> PathBuf {
    let candidates = [
        workdir().join("c_src/build/libdriver.so"),
        workdir().join("c_src/build/lib/libdriver.so"),
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

/// Path to the Rust `cdylib`. Prefers whichever profile dir the current test
/// run produced, but accepts either.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return p;
        }
    }
    // The test executable lives in target/<profile>/deps/, so the sibling
    // cdylib is one level up.
    let mut from_exe: Option<PathBuf> = None;
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                let cand = profile.join("libdriver.so");
                if cand.is_file() {
                    from_exe = Some(cand);
                }
            }
        }
    }
    if let Some(p) = from_exe {
        return p;
    }
    let candidates = [
        crate_root().join("target/release/libdriver.so"),
        crate_root().join("target/debug/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("Rust cdylib libdriver.so not found; run `cargo build` first. looked in {candidates:?}");
}

pub struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: CustomStrdupFn,
    pub rs: CustomStrdupFn,
    pub c_path: PathBuf,
    pub rs_path: PathBuf,
}

// The function pointers are plain `extern "C"` code addresses in libraries that
// are never unloaded (leaked below), so sharing them across threads is sound.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn load_sym(lib: &Library, name: &[u8]) -> CustomStrdupFn {
    unsafe {
        let s: Symbol<CustomStrdupFn> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("symbol {} missing: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rs_path = rust_so_path();
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rs_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rs_path.display()));
        let c = load_sym(&c_lib, b"custom_strdup");
        let rs = load_sym(&rust_lib, b"custom_strdup");
        Libs { _c_lib: c_lib, _rust_lib: rust_lib, c, rs, c_path, rs_path }
    })
}

pub fn so_paths() -> (PathBuf, PathBuf) {
    let l = libs();
    (l.c_path.clone(), l.rs_path.clone())
}

/// Deterministic PRNG (SplitMix64) so every randomized row is reproducible.
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
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo) as u64 + 1)) as usize
    }
    /// A byte in `0x01..=0xFF` (never a NUL, so it cannot terminate a string).
    pub fn nonnul_byte(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }
    pub fn ascii_printable(&mut self) -> u8 {
        0x20 + (self.next_u64() % (0x7F - 0x20)) as u8
    }
}

/// A NUL-terminated byte buffer we hand to the library as `const char *`.
pub struct CStrBuf {
    pub bytes: Vec<u8>,
}

impl CStrBuf {
    /// `body` must not contain a NUL if you want the whole thing copied.
    pub fn new(body: &[u8]) -> Self {
        let mut bytes = body.to_vec();
        bytes.push(0);
        CStrBuf { bytes }
    }
    pub fn ptr(&self) -> *const c_char {
        self.bytes.as_ptr() as *const c_char
    }
    /// Bytes the C implementation will copy: prefix up to the first NUL, plus
    /// that NUL (`strlen(str) + 1`).
    pub fn expected_copy(&self) -> &[u8] {
        let n = self.bytes.iter().position(|&b| b == 0).expect("NUL-terminated");
        &self.bytes[..=n]
    }
}

fn read_out(p: *mut c_char, n: usize) -> Vec<u8> {
    unsafe { std::slice::from_raw_parts(p as *const u8, n).to_vec() }
}

/// Calls both implementations on the same `const char *` and asserts the
/// results are byte-identical over `strlen(input) + 1` bytes, then frees both.
///
/// Returns the copied bytes (including the terminator).
pub fn assert_same(input: &CStrBuf, ctx: &str) -> Vec<u8> {
    let l = libs();
    let expect = input.expected_copy();
    let n = expect.len();

    let cp = unsafe { (l.c)(input.ptr()) };
    let rp = unsafe { (l.rs)(input.ptr()) };

    assert!(!cp.is_null(), "{ctx}: C returned NULL for a valid input");
    assert!(!rp.is_null(), "{ctx}: Rust returned NULL where C returned non-NULL");

    // The C returns a *fresh* buffer, never aliasing the input.
    assert_ne!(cp as *const c_char, input.ptr(), "{ctx}: C aliased the input");
    assert_ne!(rp as *const c_char, input.ptr(), "{ctx}: Rust aliased the input");
    assert_ne!(cp, rp, "{ctx}: C and Rust returned the same pointer");

    let cout = read_out(cp, n);
    let rout = read_out(rp, n);

    assert_eq!(
        cout, rout,
        "{ctx}: C/Rust output differs over {n} bytes\n  C   ={:x?}\n  Rust={:x?}",
        &cout[..n.min(64)],
        &rout[..n.min(64)]
    );
    assert_eq!(cout, expect, "{ctx}: C output does not match strlen+1 copy of the input");
    assert_eq!(cout[n - 1], 0, "{ctx}: result not NUL-terminated");

    // Allocator-ownership parity: both pointers must be free()-able by the
    // caller through libc `free`.
    unsafe {
        free(cp as *mut _);
        free(rp as *mut _);
    }

    cout
}

/// Asserts both implementations return NULL for the given pointer.
pub fn assert_both_null(p: *const c_char, ctx: &str) {
    let l = libs();
    let cp = unsafe { (l.c)(p) };
    let rp = unsafe { (l.rs)(p) };
    assert!(cp.is_null(), "{ctx}: C did not return NULL (got {cp:p})");
    assert!(rp.is_null(), "{ctx}: Rust did not return NULL (got {rp:p})");
    assert_eq!(cp as usize, 0usize, "{ctx}: C sentinel is not exactly (char*)NULL");
    assert_eq!(rp as usize, 0usize, "{ctx}: Rust sentinel is not exactly (char*)NULL");
}
