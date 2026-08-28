//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls `decode_base64`
//! only through the dynamic symbol, exactly as an external C consumer would.
//! The Rust implementation is NEVER called directly as a Rust function — that
//! way the `#[unsafe(no_mangle)] extern "C"` export wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

/// `char *decode_base64(const char *src)`
pub type DecodeBase64 = unsafe extern "C" fn(*const c_char) -> *mut c_char;

extern "C" {
    /// The very same libc `free` that both libraries' `calloc`/`malloc` came
    /// from, so the returned buffers can be released like a C caller does.
    fn free(p: *mut c_void);
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

/// Directory holding this test executable: `<target>/<profile>/deps`.
fn exe_dir() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop(); // deps/
    p
}

/// `<target>/<profile>` — where cargo puts the cdylib artifact.
fn profile_dir() -> PathBuf {
    let d = exe_dir();
    if d.file_name().map(|f| f == "deps").unwrap_or(false) {
        d.parent().unwrap().to_path_buf()
    } else {
        d
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the Rust cdylib. Prefers the artifact built alongside this test
/// binary (same profile), then falls back to the sibling profile directory.
pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let name = "libdriver.so";
    let mut candidates = vec![profile_dir().join(name)];
    let target = profile_dir().parent().map(|p| p.to_path_buf());
    if let Some(t) = target {
        candidates.push(t.join("release").join(name));
        candidates.push(t.join("debug").join(name));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found. Tried: {:#?}\nRun `cargo build --release` first.",
        candidates
    );
}

/// Locate the C shared library built by CMake.
pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().parent().expect("workspace root").to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Tried: {:#?}\nBuild it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        candidates
    );
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Both libraries, loaded once and kept alive for the whole test binary.
pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_lib_path();
        let rust_path = rust_lib_path();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

pub fn c_decode() -> Symbol<'static, DecodeBase64> {
    unsafe { libs().c.get(b"decode_base64\0") }.expect("C decode_base64 symbol")
}

pub fn rust_decode() -> Symbol<'static, DecodeBase64> {
    unsafe { libs().rust.get(b"decode_base64\0") }.expect("Rust decode_base64 symbol")
}

/// What one call produced: either `NULL`, or the full contents of the returned
/// allocation.
///
/// The C allocates `dest` with `calloc(sizeof(char), l + 13)` where
/// `l = strlen(src) + 1`, i.e. `strlen(src) + 14` defined, zero-initialised
/// bytes. We therefore snapshot exactly that many bytes: comparing the *whole
/// allocation* (rather than just up to the first NUL) is what makes embedded
/// NUL bytes in the decoded output — and any stray byte written past the
/// logical end — visible to the assertions.
#[derive(PartialEq, Eq, Clone)]
pub enum Outcome {
    Null,
    Buf(Vec<u8>),
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Null => write!(f, "NULL"),
            Outcome::Buf(b) => {
                let cstr_len = b.iter().position(|&x| x == 0).unwrap_or(b.len());
                write!(
                    f,
                    "Buf(alloc={} bytes, as_cstr={:?}, bytes={:02x?})",
                    b.len(),
                    String::from_utf8_lossy(&b[..cstr_len]),
                    b
                )
            }
        }
    }
}

/// Call one implementation with a NUL-terminated `src` and snapshot the result.
///
/// `src_with_nul` must contain the trailing NUL. `alloc_len` is the number of
/// bytes the implementation is contractually known to have allocated.
unsafe fn call_raw(f: &Symbol<'static, DecodeBase64>, src: *const c_char, alloc_len: usize) -> Outcome {
    let p = f(src);
    if p.is_null() {
        return Outcome::Null;
    }
    let bytes = std::slice::from_raw_parts(p as *const u8, alloc_len).to_vec();
    free(p as *mut c_void);
    Outcome::Buf(bytes)
}

/// Number of bytes `decode_base64` allocates for `dest`, given the input.
fn dest_alloc_len(src_without_nul: &[u8]) -> usize {
    // strlen(src) + 1 + 13
    src_without_nul.len() + 14
}

/// Run BOTH implementations on the same input and return `(c, rust)`.
///
/// `input` must NOT contain the terminating NUL; it is appended here. Interior
/// NUL bytes are allowed (that is CONFIGS row B29) — `strlen` stops at the
/// first one, which is exactly what we want to compare.
pub fn run_both(input: &[u8]) -> (Outcome, Outcome) {
    let mut buf = input.to_vec();
    buf.push(0);
    // The allocation size is driven by strlen(), i.e. the prefix before the
    // first NUL, not by the whole vector.
    let strlen = buf.iter().position(|&b| b == 0).unwrap();
    let alloc = dest_alloc_len(&buf[..strlen]);
    let cf = c_decode();
    let rf = rust_decode();
    unsafe {
        let c = call_raw(&cf, buf.as_ptr() as *const c_char, alloc);
        let r = call_raw(&rf, buf.as_ptr() as *const c_char, alloc);
        (c, r)
    }
}

/// Assert C and Rust agree byte-for-byte for `input`.
pub fn assert_same(row: &str, input: &[u8]) {
    let (c, r) = run_both(input);
    if c != r {
        panic!(
            "[{row}] DIVERGENCE\n  input   ({} bytes): {:02x?}\n           as text: {:?}\n  C   -> {:?}\n  Rust-> {:?}",
            input.len(),
            input,
            String::from_utf8_lossy(input),
            c,
            r
        );
    }
}

/// Assert both returned NULL (a shared, identical rejection).
pub fn assert_both_null(row: &str, input: &[u8]) {
    let (c, r) = run_both(input);
    assert_eq!(c, Outcome::Null, "[{row}] C should return NULL");
    assert_eq!(r, Outcome::Null, "[{row}] Rust should return NULL");
}

/// Assert both returned non-NULL and identical bytes.
pub fn assert_both_ok(row: &str, input: &[u8]) {
    let (c, r) = run_both(input);
    assert!(
        !matches!(c, Outcome::Null),
        "[{row}] C unexpectedly returned NULL for {:02x?}",
        input
    );
    assert_eq!(c, r, "[{row}] byte mismatch for input {:02x?}", input);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds, reproducible property tests.
// ---------------------------------------------------------------------------

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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn pick(&mut self, set: &[u8]) -> u8 {
        set[self.below(set.len())]
    }
    /// Random bytes from `set`.
    pub fn bytes_from(&mut self, set: &[u8], len: usize) -> Vec<u8> {
        (0..len).map(|_| self.pick(set)).collect()
    }
    /// Random non-NUL bytes (`0x01..=0xFF`), the full range a C string can hold.
    pub fn nonnul_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| (self.next_u64() % 255) as u8 + 1)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Character sets, straight from the C source.
// ---------------------------------------------------------------------------

pub const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
pub const DIGIT: &[u8] = b"0123456789";
pub const PLUS: &[u8] = b"+";
pub const SLASH: &[u8] = b"/";
pub const PAD: &[u8] = b"=";
/// The 64 characters `is_base64` accepts, minus `'='`.
pub const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
/// Characters `is_base64` rejects (a representative spread incl. the
/// off-by-one neighbours of every accepted range, and high/negative bytes).
pub const NON_B64: &[u8] = &[
    b'@', b'[', b'`', b'{', b'*', b',', b'.', b'-', b':', b'<', b'>', b'?', b'!', b'#', b'$', b'%',
    b'^', b'&', b'(', b')', b' ', b'\t', b'\n', b'\r', b'"', b'\'', b';', b'\\', b'|', b'~', 0x7f,
    0x01, 0x1f, 0x80, 0x81, 0xfe, 0xff,
];

/// Reference base64 *encoder* (used only to synthesise valid inputs for the
/// round-trip row; never used to predict the C's output).
pub fn b64_encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64[(n >> 18) as usize & 63]);
        out.push(B64[(n >> 12) as usize & 63]);
        if chunk.len() > 1 {
            out.push(B64[(n >> 6) as usize & 63]);
        } else {
            out.push(b'=');
        }
        if chunk.len() > 2 {
            out.push(B64[n as usize & 63]);
        } else {
            out.push(b'=');
        }
    }
    out
}
