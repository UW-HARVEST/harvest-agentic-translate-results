//! Shared differential-testing harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
//! purely through their exported `decode_base64` symbol, exactly as an external
//! consumer would. Rust functions are never called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

unsafe extern "C" {
    fn free(p: *mut c_void);
    fn atexit(cb: extern "C" fn()) -> i32;
}

/// Exact number of C-vs-Rust differential comparisons performed, reported at
/// process exit so the coverage figure is measured rather than estimated.
/// Print it with `cargo test -- --nocapture` (it goes to stderr).
pub static COMPARISONS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

extern "C" fn report_comparisons() {
    let n = COMPARISONS.load(std::sync::atomic::Ordering::Relaxed);
    eprintln!("[differential] C-vs-Rust comparisons in this binary: {n}");
}

pub type DecodeBase64 = unsafe extern "C" fn(*const c_char) -> *mut c_char;

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_decode: DecodeBase64,
    pub rust_decode: DecodeBase64,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built: {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Prefer an explicit override, then release, then debug.
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_DRIVER_SO does not exist: {}", p.display());
        return p;
    }
    for candidate in [
        manifest.join("target/release/libdriver.so"),
        manifest.join("target/debug/libdriver.so"),
    ] {
        if candidate.exists() {
            return candidate;
        }
    }
    panic!(
        "Rust shared library not built. Run `cargo build --release` in {}",
        manifest.display()
    );
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("load C libdriver.so");
        let rust = Library::new(rust_so_path()).expect("load Rust libdriver.so");
        let c_sym: Symbol<DecodeBase64> =
            c.get(b"decode_base64\0").expect("C decode_base64 symbol");
        let rust_sym: Symbol<DecodeBase64> = rust
            .get(b"decode_base64\0")
            .expect("Rust decode_base64 symbol (missing #[no_mangle] export?)");
        let c_decode = *c_sym;
        let rust_decode = *rust_sym;
        atexit(report_comparisons);
        Libs {
            _c: c,
            _rust: rust,
            c_decode,
            rust_decode,
        }
    })
}

/// Outcome of one `decode_base64` call, captured so the heap buffer can be
/// freed immediately.
#[derive(PartialEq, Eq)]
pub enum Outcome {
    Null,
    /// The full `strlen(src) + 14` byte allocation (`calloc(1, l + 13)` with
    /// `l = strlen(src) + 1`), captured verbatim including the trailing zeros.
    Buf(Vec<u8>),
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Null => write!(f, "NULL"),
            Outcome::Buf(v) => {
                write!(f, "Buf(len={}, {:02x?}", v.len(), &v[..v.len().min(48)])?;
                if v.len() > 48 {
                    write!(f, " ...")?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Call one implementation on a NUL-terminated input and capture the result.
///
/// `input` must NOT contain an interior NUL (the C API takes a C string).
unsafe fn call_one(f: DecodeBase64, input: &[u8]) -> Outcome {
    let mut cstr = Vec::with_capacity(input.len() + 1);
    cstr.extend_from_slice(input);
    cstr.push(0);
    let ret = unsafe { f(cstr.as_ptr() as *const c_char) };
    if ret.is_null() {
        return Outcome::Null;
    }
    // `dest = calloc(sizeof(char), l + 13)` with `l = strlen(src) + 1`.
    let alloc_len = input.len() + 1 + 13;
    let bytes = unsafe { std::slice::from_raw_parts(ret as *const u8, alloc_len) }.to_vec();
    unsafe { free(ret as *mut c_void) };
    Outcome::Buf(bytes)
}

/// Call the C implementation with a raw pointer (for the NULL-pointer case).
pub unsafe fn call_c_ptr(p: *const c_char) -> bool {
    let r = unsafe { (libs().c_decode)(p) };
    if !r.is_null() {
        unsafe { free(r as *mut c_void) };
        return false;
    }
    true
}

pub unsafe fn call_rust_ptr(p: *const c_char) -> bool {
    let r = unsafe { (libs().rust_decode)(p) };
    if !r.is_null() {
        unsafe { free(r as *mut c_void) };
        return false;
    }
    true
}

pub fn c_call(input: &[u8]) -> Outcome {
    unsafe { call_one(libs().c_decode, input) }
}

pub fn rust_call(input: &[u8]) -> Outcome {
    unsafe { call_one(libs().rust_decode, input) }
}

fn render(input: &[u8]) -> String {
    let mut s = String::new();
    for &b in input.iter().take(120) {
        if b.is_ascii_graphic() {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{b:02x}"));
        }
    }
    if input.len() > 120 {
        s.push_str("...");
    }
    s
}

/// The core differential assertion: C and Rust must agree byte-for-byte.
#[track_caller]
pub fn assert_same(row: &str, input: &[u8]) {
    let c = c_call(input);
    let r = rust_call(input);
    COMPARISONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if c != r {
        let (cd, rd) = (format!("{c:?}"), format!("{r:?}"));
        let first_diff = match (&c, &r) {
            (Outcome::Buf(a), Outcome::Buf(b)) => a
                .iter()
                .zip(b.iter())
                .position(|(x, y)| x != y)
                .map(|i| format!("\n  first differing byte at offset {i}: C=0x{:02x} Rust=0x{:02x}", a[i], b[i]))
                .unwrap_or_else(|| format!("\n  lengths differ: C={} Rust={}", a.len(), b.len())),
            _ => String::new(),
        };
        panic!(
            "DIVERGENCE [{row}]\n  input  (len {}) = \"{}\"\n  C      = {cd}\n  Rust   = {rd}{first_diff}",
            input.len(),
            render(input)
        );
    }
}

/// Deterministic xorshift64* PRNG so every run is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn pick(&mut self, set: &[u8]) -> u8 {
        set[self.below(set.len())]
    }
}

pub const B64_ALPHABET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
pub const DIGITS: &[u8] = b"0123456789";
pub const SPECIALS: &[u8] = b"+/";

/// Every byte in `0x01..=0x7f` that `is_base64()` rejects.
pub fn ascii_junk() -> Vec<u8> {
    (0x01u8..=0x7f)
        .filter(|b| !B64_ALPHABET.contains(b) && *b != b'=')
        .collect()
}

/// Random string of exactly `n` characters drawn from `set`.
pub fn rand_from(rng: &mut Rng, set: &[u8], n: usize) -> Vec<u8> {
    (0..n).map(|_| rng.pick(set)).collect()
}

/// Standard base64 encoder, used to build inputs whose decoded payload is a
/// known arbitrary byte string (rows 24-25).
pub fn b64_encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64_ALPHABET[((n >> 18) & 0x3f) as usize]);
        out.push(B64_ALPHABET[((n >> 12) & 0x3f) as usize]);
        out.push(if chunk.len() > 1 {
            B64_ALPHABET[((n >> 6) & 0x3f) as usize]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            B64_ALPHABET[(n & 0x3f) as usize]
        } else {
            b'='
        });
    }
    out
}
