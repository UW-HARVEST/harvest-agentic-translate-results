//! Shared differential-test harness.
//!
//! Loads BOTH the C `liblz4.so` and the Rust `liblz4.so` via `libloading` and
//! exposes them as a pair. Tests must never call Rust functions directly: every
//! call goes through a `.so` export so the `#[no_mangle]` wrappers are covered.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Library pair
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Library,
    pub r: Library,
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    manifest().join("..").join("c_src").join("build").join("liblz4.so")
}

fn rust_so() -> PathBuf {
    // Prefer the release cdylib; fall back to debug if that is what exists.
    let rel = manifest().join("target").join("release").join("liblz4.so");
    if rel.exists() {
        return rel;
    }
    manifest().join("target").join("debug").join("liblz4.so")
}

/// `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` lib target, because
/// integration tests cannot link against a cdylib. That means
/// `target/release/liblz4.so` can silently be STALE with respect to `src/*.rs`,
/// and the whole suite would then verify an old binary — passing while the
/// current source is broken. Guard against it: refuse to run if any Rust source
/// is newer than the `.so` we are about to load.
fn assert_rust_so_is_fresh(so: &std::path::Path) {
    use std::fs;
    let so_mtime = fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat Rust .so");
    let src = manifest().join("src");
    let mut stale = Vec::new();
    for entry in fs::read_dir(&src).expect("read src/") {
        let p = entry.expect("dir entry").path();
        if p.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let m = fs::metadata(&p)
            .and_then(|m| m.modified())
            .expect("stat source");
        if m > so_mtime {
            stale.push(p);
        }
    }
    assert!(
        stale.is_empty(),
        "STALE Rust .so: {} is older than {:?}.\n\
         `cargo test` does not rebuild a cdylib-only lib target, so these tests \
         would silently verify an out-of-date binary.\n\
         Run `cargo build --offline --release` before `cargo test`.",
        so.display(),
        stale
    );
}

impl Pair {
    pub fn load() -> Pair {
        let c = c_so();
        let r = rust_so();
        assert!(c.exists(), "C shared library not built: {}", c.display());
        assert!(r.exists(), "Rust shared library not built: {}", r.display());
        assert_rust_so_is_fresh(&r);
        unsafe {
            Pair {
                c: Library::new(&c).expect("dlopen C liblz4.so"),
                r: Library::new(&r).expect("dlopen Rust liblz4.so"),
            }
        }
    }

    /// Look up `name` in both libraries, returning `(c_fn, rust_fn)`.
    pub unsafe fn sym<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let cs: Symbol<T> = self
            .c
            .get(name.as_bytes())
            .unwrap_or_else(|e| panic!("C .so missing symbol {name}: {e}"));
        let rs: Symbol<T> = self
            .r
            .get(name.as_bytes())
            .unwrap_or_else(|e| panic!("Rust .so missing symbol {name}: {e}"));
        (cs, rs)
    }

    pub fn has(&self, name: &str) -> (bool, bool) {
        unsafe {
            let a = self.c.get::<*const c_void>(name.as_bytes()).is_ok();
            let b = self.r.get::<*const c_void>(name.as_bytes()).is_ok();
            (a, b)
        }
    }
}

/// Process-wide lazily-initialised pair, so each test binary dlopens once.
pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(Pair::load)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds keep every test reproducible.
// ---------------------------------------------------------------------------

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
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        if hi_inclusive <= lo {
            lo
        } else {
            lo + self.below(hi_inclusive - lo + 1)
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

// ---------------------------------------------------------------------------
// Input-shape generators — these drive the "input shape" axis of CONFIGS.md.
// ---------------------------------------------------------------------------

/// Pure random bytes: essentially incompressible.
pub fn gen_incompressible(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.byte()).collect()
}

/// Highly compressible: long runs of a small number of distinct bytes.
pub fn gen_compressible(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    while v.len() < len {
        let b = rng.byte() & 0x07;
        let run = rng.range(1, 300);
        for _ in 0..run {
            if v.len() == len {
                break;
            }
            v.push(b);
        }
    }
    v
}

/// Text-like data with repeated tokens: realistic mid-ratio input, good at
/// exercising match finding / hash chains.
pub fn gen_textlike(rng: &mut Rng, len: usize) -> Vec<u8> {
    const WORDS: &[&str] = &[
        "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ",
        "lz4 ", "compression ", "block ", "frame ", "stream ", "dictionary ",
        "hash ", "match ", "literal ", "offset ", "checksum ", "buffer ",
    ];
    let mut v = Vec::with_capacity(len + 32);
    while v.len() < len {
        v.extend_from_slice(WORDS[rng.below(WORDS.len())].as_bytes());
    }
    v.truncate(len);
    v
}

/// Data with a long periodic pattern — hits `LZ4HC` pattern analysis and
/// `memcpy_using_offset` small-offset paths in the decoder.
pub fn gen_periodic(rng: &mut Rng, len: usize) -> Vec<u8> {
    let period = rng.range(1, 8);
    let base: Vec<u8> = (0..period).map(|_| rng.byte()).collect();
    (0..len).map(|i| base[i % period]).collect()
}

/// A handful of near-degenerate shapes: all-zero, all-same, ascending.
pub fn gen_degenerate(rng: &mut Rng, len: usize) -> Vec<u8> {
    match rng.below(3) {
        0 => vec![0u8; len],
        1 => vec![rng.byte(); len],
        _ => (0..len).map(|i| (i & 0xFF) as u8).collect(),
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    Incompressible,
    Compressible,
    TextLike,
    Periodic,
    Degenerate,
}

pub const ALL_SHAPES: [Shape; 5] = [
    Shape::Incompressible,
    Shape::Compressible,
    Shape::TextLike,
    Shape::Periodic,
    Shape::Degenerate,
];

pub fn gen(rng: &mut Rng, shape: Shape, len: usize) -> Vec<u8> {
    match shape {
        Shape::Incompressible => gen_incompressible(rng, len),
        Shape::Compressible => gen_compressible(rng, len),
        Shape::TextLike => gen_textlike(rng, len),
        Shape::Periodic => gen_periodic(rng, len),
        Shape::Degenerate => gen_degenerate(rng, len),
    }
}

/// Sizes that matter: tiny values, `LZ4_minLength`, the byU16/byU32 table
/// pivot at `LZ4_64Klimit == 65547`, and the frame block-size boundaries.
pub fn interesting_sizes() -> Vec<usize> {
    vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 13, 14, 15, 16, 17, 19, 20, 31, 32, 33,
        63, 64, 65, 100, 127, 128, 129, 255, 256, 257, 511, 512, 513, 1023,
        1024, 1025, 4095, 4096, 4097, 8191, 8192, 16384, 32768, 65534, 65535,
        65536, 65537, 65546, 65547, 65548, 65600, 131072, 262143, 262144,
        262145, 300000,
    ]
}

// ---------------------------------------------------------------------------
// C type mirrors
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct LZ4F_frameInfo_t {
    pub blockSizeID: c_uint,
    pub blockMode: c_uint,
    pub contentChecksumFlag: c_uint,
    pub frameType: c_uint,
    pub contentSize: u64,
    pub dictID: c_uint,
    pub blockChecksumFlag: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct LZ4F_preferences_t {
    pub frameInfo: LZ4F_frameInfo_t,
    pub compressionLevel: c_int,
    pub autoFlush: c_uint,
    pub favorDecSpeed: c_uint,
    pub reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct LZ4F_compressOptions_t {
    pub stableSrc: c_uint,
    pub reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct LZ4F_decompressOptions_t {
    pub stableDst: c_uint,
    pub skipChecksums: c_uint,
    pub reserved1: c_uint,
    pub reserved0: c_uint,
}

// LZ4F_blockSizeID_t
pub const LZ4F_DEFAULT: c_uint = 0;
pub const LZ4F_MAX64KB: c_uint = 4;
pub const LZ4F_MAX256KB: c_uint = 5;
pub const LZ4F_MAX1MB: c_uint = 6;
pub const LZ4F_MAX4MB: c_uint = 7;

pub const LZ4F_BLOCK_LINKED: c_uint = 0;
pub const LZ4F_BLOCK_INDEPENDENT: c_uint = 1;

pub const LZ4F_NO_CONTENT_CHECKSUM: c_uint = 0;
pub const LZ4F_CONTENT_CHECKSUM_ENABLED: c_uint = 1;

pub const LZ4F_NO_BLOCK_CHECKSUM: c_uint = 0;
pub const LZ4F_BLOCK_CHECKSUM_ENABLED: c_uint = 1;

pub const LZ4F_FRAME: c_uint = 0;
pub const LZ4F_SKIPPABLE_FRAME: c_uint = 1;

pub const LZ4F_VERSION: c_uint = 100;

pub const LZ4_MAX_INPUT_SIZE: usize = 0x7E000000;
pub const LZ4HC_CLEVEL_MIN: c_int = 2;
pub const LZ4HC_CLEVEL_DEFAULT: c_int = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: c_int = 10;
pub const LZ4HC_CLEVEL_MAX: c_int = 12;

/// `(size_t)-code` — how lz4frame reports `LZ4F_errorCodes`.
pub fn err(code: usize) -> usize {
    (0usize).wrapping_sub(code)
}

/// True when `r` is in the lz4frame error range `[(size_t)-23 ..= (size_t)-1]`.
pub fn is_err_range(r: usize) -> bool {
    r > err(24)
}

// ---------------------------------------------------------------------------
// Common FFI signatures
// ---------------------------------------------------------------------------

pub type FnCompressDefault =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type FnCompressFast =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type FnCompressBound = unsafe extern "C" fn(c_int) -> c_int;
pub type FnDecompressSafe =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type FnDecompressSafePartial =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type FnDecompressFast =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
pub type FnCompressDestSize =
    unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
pub type FnVoidToInt = unsafe extern "C" fn() -> c_int;
pub type FnPtrToInt = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type FnVoidToPtr = unsafe extern "C" fn() -> *mut c_void;
pub type FnFreePtr = unsafe extern "C" fn(*mut c_void) -> c_int;

// ---------------------------------------------------------------------------
// Assertion helpers with rich diagnostics
// ---------------------------------------------------------------------------

pub fn hexdump(b: &[u8]) -> String {
    let n = b.len().min(96);
    let mut s = String::new();
    for x in &b[..n] {
        s.push_str(&format!("{x:02x}"));
    }
    if b.len() > n {
        s.push_str(&format!("...(+{} bytes)", b.len() - n));
    }
    s
}

pub fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return Some(i);
        }
    }
    if a.len() != b.len() {
        Some(n)
    } else {
        None
    }
}

/// Assert two returned integers and two output buffers agree byte-for-byte.
#[track_caller]
pub fn same_int_and_bytes(ctx: &str, cr: c_int, rr: c_int, cb: &[u8], rb: &[u8]) {
    assert_eq!(cr, rr, "{ctx}: return value mismatch (C={cr} Rust={rr})");
    if cr > 0 {
        let n = cr as usize;
        let (ca, ra) = (&cb[..n.min(cb.len())], &rb[..n.min(rb.len())]);
        if let Some(i) = first_diff(ca, ra) {
            panic!(
                "{ctx}: output bytes differ at index {i} (ret={cr})\n  C   : {}\n  Rust: {}",
                hexdump(&ca[i.saturating_sub(8)..]),
                hexdump(&ra[i.saturating_sub(8)..])
            );
        }
    }
}

#[track_caller]
pub fn same_usize_and_bytes(ctx: &str, cr: usize, rr: usize, cb: &[u8], rb: &[u8]) {
    assert_eq!(
        cr as i64 as isize, rr as i64 as isize,
        "{ctx}: return value mismatch (C={} Rust={}, as signed C={} Rust={})",
        cr, rr, cr as isize, rr as isize
    );
    if !is_err_range(cr) && cr > 0 {
        let n = cr.min(cb.len()).min(rb.len());
        if let Some(i) = first_diff(&cb[..n], &rb[..n]) {
            panic!(
                "{ctx}: output bytes differ at index {i} (ret={cr})\n  C   : {}\n  Rust: {}",
                hexdump(&cb[i.saturating_sub(8)..n]),
                hexdump(&rb[i.saturating_sub(8)..n])
            );
        }
    }
}

/// Assert whole buffers are identical regardless of the return value — used to
/// prove neither implementation scribbles differently outside the reported
/// output length.
#[track_caller]
pub fn same_full_buffers(ctx: &str, cb: &[u8], rb: &[u8]) {
    if let Some(i) = first_diff(cb, rb) {
        panic!(
            "{ctx}: full destination buffers differ at index {i}\n  C   : {}\n  Rust: {}",
            hexdump(&cb[i.saturating_sub(8)..]),
            hexdump(&rb[i.saturating_sub(8)..])
        );
    }
}
