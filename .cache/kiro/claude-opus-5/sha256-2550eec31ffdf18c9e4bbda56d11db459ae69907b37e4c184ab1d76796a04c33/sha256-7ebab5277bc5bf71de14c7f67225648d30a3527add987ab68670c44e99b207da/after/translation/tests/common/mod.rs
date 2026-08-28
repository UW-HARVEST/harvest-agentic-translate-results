//! Shared harness: loads the C and Rust shared libraries via `libloading`
//! and compares `encode_base64` results byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type EncodeBase64 = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

unsafe extern "C" {
    fn free(p: *mut c_void);
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let root = workspace_root();
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
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\nLooked in: {candidates:?}"
    );
}

fn find_rust_so() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut candidates: Vec<PathBuf> = Vec::new();

    // The test binary lives at target/<profile>/deps/<name>; prefer the
    // libdriver.so built with the very same profile.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            candidates.push(profile_dir.join("libdriver.so"));
            candidates.push(profile_dir.join("libdriver.dylib"));
        }
    }
    candidates.extend([
        manifest.join("target/release/libdriver.so"),
        manifest.join("target/debug/libdriver.so"),
        manifest.join("target/release/libdriver.dylib"),
        manifest.join("target/debug/libdriver.dylib"),
    ]);

    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust shared library not found. Build it with `cargo build --release`.\nLooked in: {candidates:?}"
    );
}

pub struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    pub c_encode: EncodeBase64,
    pub rust_encode: EncodeBase64,
}

/// Loads both libraries once per test binary.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c_lib = Library::new(find_c_so()).expect("failed to dlopen C libdriver.so");
        let rust_lib = Library::new(find_rust_so()).expect("failed to dlopen Rust libdriver.so");

        // Resolve through the dynamic symbol table only -- never call the Rust
        // function directly, so the `#[no_mangle]` wrapper is exercised too.
        let c_sym: Symbol<EncodeBase64> = c_lib
            .get(b"encode_base64\0")
            .expect("C .so does not export encode_base64");
        let rust_sym: Symbol<EncodeBase64> = rust_lib
            .get(b"encode_base64\0")
            .expect("Rust .so does not export encode_base64");

        let c_encode = *c_sym;
        let rust_encode = *rust_sym;

        Libs {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_encode,
            rust_encode,
        }
    })
}

/// Replicates the C allocation-size expression `size * 4 / 3 + 4` using
/// wrapping `int` arithmetic, so we know how many bytes are legal to read
/// back out of both buffers.
fn alloc_len(size: c_int) -> c_int {
    size.wrapping_mul(4).wrapping_div(3).wrapping_add(4)
}

/// How many bytes of the result buffer we can safely compare.
fn comparable_len(size: c_int) -> usize {
    let n = alloc_len(size);
    if n <= 0 { 0 } else { n as usize }
}

/// Snapshot of one call: either NULL, or the full allocated buffer contents.
#[derive(PartialEq, Eq)]
enum Result_ {
    Null,
    Buf(Vec<u8>),
}

impl std::fmt::Debug for Result_ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Result_::Null => write!(f, "NULL"),
            Result_::Buf(b) => write!(f, "{:?} (bytes {:?})", String::from_utf8_lossy(b), b),
        }
    }
}

unsafe fn call(f: EncodeBase64, size: c_int, src: *const c_char, read_len: usize) -> Result_ {
    let p = unsafe { f(size, src) };
    if p.is_null() {
        return Result_::Null;
    }
    let buf = unsafe { std::slice::from_raw_parts(p as *const u8, read_len).to_vec() };
    unsafe { free(p as *mut c_void) };
    Result_::Buf(buf)
}

/// Calls both implementations with `(size, src)` and asserts byte equality of
/// the whole allocated buffer (including the zero padding / NUL terminator).
pub fn check_raw(size: c_int, src: *const c_char, label: &str) {
    let l = libs();
    let read_len = comparable_len(size);
    let c = unsafe { call(l.c_encode, size, src, read_len) };
    let r = unsafe { call(l.rust_encode, size, src, read_len) };
    assert_eq!(c, r, "mismatch for {label} (size={size})");
}

/// Convenience wrapper for a byte slice with an explicit size argument.
pub fn check_bytes_size(bytes: &[u8], size: c_int) {
    // NUL-terminate the backing buffer so the strlen path (size == 0) is safe.
    let mut owned = bytes.to_vec();
    owned.push(0);
    let label = format!("bytes={:x?}", bytes);
    check_raw(size, owned.as_ptr() as *const c_char, &label);
}

/// Encode `bytes` passing its true length as `size`.
pub fn check_bytes(bytes: &[u8]) {
    check_bytes_size(bytes, bytes.len() as c_int);
}

/// Deterministic xorshift PRNG so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() >> 1) as usize % n
    }
}
