//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` via `libloading` and exposes their
//! exported symbols as raw `extern "C"` function pointers. Rust functions are
//! NEVER called directly — every call goes through the `.so` export table, so the
//! `#[no_mangle]` wrappers are exercised exactly as an external C caller would.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use libloading::Library;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    lib: Library,
    pub tag: &'static str,
}

impl Lib {
    /// Extract an exported symbol as a copyable value (function pointer).
    ///
    /// The returned value outlives the borrow because `Lib` is stored in a
    /// process-lifetime `OnceLock` and therefore never unloaded.
    pub fn sym<T: Copy>(&self, name: &str) -> T {
        unsafe {
            let s = self
                .lib
                .get::<T>(name.as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol `{}`: {}", self.tag, name, e));
            *s
        }
    }

    /// Returns `true` if the symbol is exported at all.
    pub fn has(&self, name: &str) -> bool {
        unsafe { self.lib.get::<*const c_void>(name.as_bytes()).is_ok() }
    }
}

pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

fn crate_root() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

fn rust_so_path() -> String {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    format!("{}/target/{}/liblz4.so", crate_root(), profile)
}

fn c_so_path() -> String {
    format!("{}/c_src/build/liblz4.so", crate_root())
}

static LIBS: OnceLock<Pair> = OnceLock::new();

/// The loaded pair of shared libraries (loaded once per test process).
pub fn libs() -> &'static Pair {
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("cannot load C .so at {}: {}\nDid you run cmake --build c_src/build ?", cp, e));
        let rust = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("cannot load Rust .so at {}: {}", rp, e));
        Pair {
            c: Lib { lib: c, tag: "C" },
            rust: Lib { lib: rust, tag: "Rust" },
        }
    })
}

/// Fetch the same symbol from both libraries, returning `(c_fn, rust_fn)`.
pub fn both<T: Copy>(name: &str) -> (T, T) {
    let l = libs();
    (l.c.sym::<T>(name), l.rust.sym::<T>(name))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xoshiro256**) — no external crate, fully reproducible
// ---------------------------------------------------------------------------

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // SplitMix64 seeding
        let mut z = seed;
        let mut s = [0u64; 4];
        for slot in s.iter_mut() {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *slot = x ^ (x >> 31);
        }
        Rng { s }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    /// Uniform in `[0, n)`. Returns 0 when `n == 0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            lo
        } else {
            lo + self.below(hi - lo + 1)
        }
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Input-shape generators
//
// These deliberately span the data characters the LZ4 encoder branches on:
// incompressible (long literal runs -> literal-length extension bytes),
// highly repetitive (long matches -> match-length extension bytes),
// and mixtures / structured text.
// ---------------------------------------------------------------------------

/// Fully random bytes: incompressible, forces literal runs > 15 and hence the
/// literal-length 255-extension loop.
pub fn gen_random(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.byte()).collect()
}

/// A single repeated byte: maximally compressible, forces very long matches and
/// hence multiple 0xFF match-length extension bytes.
pub fn gen_constant(len: usize, b: u8) -> Vec<u8> {
    vec![b; len]
}

/// Repeating pattern of `period` bytes: long matches at a fixed small offset.
pub fn gen_periodic(rng: &mut Rng, len: usize, period: usize) -> Vec<u8> {
    let period = period.max(1);
    let base: Vec<u8> = (0..period).map(|_| rng.byte()).collect();
    (0..len).map(|i| base[i % period]).collect()
}

/// Text-like data from a small alphabet with repeated words: realistic mix of
/// short and medium matches.
pub fn gen_text(rng: &mut Rng, len: usize) -> Vec<u8> {
    const WORDS: &[&[u8]] = &[
        b"the ", b"quick ", b"brown ", b"fox ", b"jumps ", b"over ", b"lazy ", b"dog ",
        b"lz4 ", b"compression ", b"block ", b"frame ", b"checksum ", b"dictionary ",
    ];
    let mut out = Vec::with_capacity(len + 16);
    while out.len() < len {
        out.extend_from_slice(WORDS[rng.below(WORDS.len())]);
    }
    out.truncate(len);
    out
}

/// Random data interleaved with long runs: alternates the literal-extension and
/// match-extension paths within a single input.
pub fn gen_mixed(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 512);
    while out.len() < len {
        if rng.bool() {
            let n = rng.range(1, 300);
            let b = rng.byte();
            out.extend(std::iter::repeat(b).take(n));
        } else {
            let n = rng.range(1, 64);
            for _ in 0..n {
                out.push(rng.byte());
            }
        }
    }
    out.truncate(len);
    out
}

/// Data built by copying earlier windows of itself at controlled distances,
/// including distances near the 65535 maximum LZ4 offset.
pub fn gen_selfref(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(len + 256);
    // seed with some random bytes
    let seed_len = rng.range(16, 128).min(len.max(1));
    for _ in 0..seed_len {
        out.push(rng.byte());
    }
    while out.len() < len {
        let choice = rng.below(4);
        match choice {
            0 => {
                // literal noise
                let n = rng.range(1, 40);
                for _ in 0..n {
                    out.push(rng.byte());
                }
            }
            _ => {
                // copy from a previous window; bias distances toward 65535
                let maxdist = out.len().min(65535);
                let dist = match rng.below(3) {
                    0 => maxdist,
                    1 => rng.range(1, maxdist.max(1)),
                    _ => rng.range(1, maxdist.min(4096).max(1)),
                }
                .max(1);
                let n = rng.range(4, 600);
                let start = out.len() - dist;
                for k in 0..n {
                    let b = out[start + (k % dist)];
                    out.push(b);
                    if out.len() >= len + 600 {
                        break;
                    }
                }
            }
        }
    }
    out.truncate(len);
    out
}

/// The set of generators, addressed by index, so tests can sweep data characters.
pub const N_SHAPES: usize = 6;

pub fn gen_shape(rng: &mut Rng, shape: usize, len: usize) -> Vec<u8> {
    match shape % N_SHAPES {
        0 => gen_random(rng, len),
        1 => gen_constant(len, rng.byte()),
        2 => {
            let period = rng.range(1, 40);
            gen_periodic(rng, len, period)
        }
        3 => gen_text(rng, len),
        4 => gen_mixed(rng, len),
        _ => gen_selfref(rng, len),
    }
}

pub fn shape_name(shape: usize) -> &'static str {
    match shape % N_SHAPES {
        0 => "random",
        1 => "constant",
        2 => "periodic",
        3 => "text",
        4 => "mixed",
        _ => "selfref",
    }
}

// ---------------------------------------------------------------------------
// Aligned scratch buffers
//
// `LZ4_initStream` / `LZ4_initStreamHC` reject insufficiently aligned buffers
// (returning NULL), and the `*_extState` entry points require a properly
// aligned state block, so tests need controllable alignment.
// ---------------------------------------------------------------------------

/// A heap buffer with a guaranteed alignment, plus an optional byte offset so a
/// test can deliberately produce a MISALIGNED pointer.
pub struct AlignedBuf {
    ptr: *mut u8,
    layout: std::alloc::Layout,
    offset: usize,
    len: usize,
}

impl AlignedBuf {
    /// Allocate `len` usable bytes at `align`-byte alignment, zero-filled.
    pub fn new(len: usize, align: usize) -> Self {
        Self::with_offset(len, align, 0)
    }

    /// Allocate so that `as_mut_ptr()` is `align`-aligned **plus** `offset`
    /// bytes — i.e. deliberately misaligned when `offset != 0`.
    pub fn with_offset(len: usize, align: usize, offset: usize) -> Self {
        let total = len + offset + align;
        let layout = std::alloc::Layout::from_size_align(total, align).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null(), "allocation of {} bytes failed", total);
        AlignedBuf { ptr, layout, offset, len }
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.ptr.add(self.offset) }
    }

    pub fn as_ptr(&self) -> *const u8 {
        unsafe { self.ptr.add(self.offset) }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    pub fn zero(&mut self) {
        unsafe { std::ptr::write_bytes(self.ptr, 0, self.layout.size()) }
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) }
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

pub fn hexdump(b: &[u8], max: usize) -> String {
    let n = b.len().min(max);
    let mut s = String::new();
    for (i, x) in b[..n].iter().enumerate() {
        if i % 32 == 0 && i != 0 {
            s.push('\n');
        }
        s.push_str(&format!("{:02x}", x));
    }
    if b.len() > n {
        s.push_str(&format!(" ...(+{} more)", b.len() - n));
    }
    s
}

/// Compare two byte slices and produce a precise diff message on mismatch.
pub fn assert_bytes_eq(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    if c.len() != r.len() {
        panic!(
            "{}\n  length mismatch: C={} Rust={}\n  C   : {}\n  Rust: {}",
            ctx,
            c.len(),
            r.len(),
            hexdump(c, 128),
            hexdump(r, 128)
        );
    }
    let idx = c.iter().zip(r.iter()).position(|(a, b)| a != b).unwrap();
    let lo = idx.saturating_sub(16);
    let hi = (idx + 16).min(c.len());
    panic!(
        "{}\n  first byte difference at offset {} (len {}): C=0x{:02x} Rust=0x{:02x}\n  C   [{}..{}]: {}\n  Rust[{}..{}]: {}",
        ctx, idx, c.len(), c[idx], r[idx], lo, hi, hexdump(&c[lo..hi], 64), lo, hi, hexdump(&r[lo..hi], 64)
    );
}

// ---------------------------------------------------------------------------
// FFI types mirroring the C headers
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LZ4F_frameInfo_t {
    pub blockSizeID: c_int,
    pub blockMode: c_int,
    pub contentChecksumFlag: c_int,
    pub frameType: c_int,
    pub contentSize: u64,
    pub dictID: c_uint,
    pub blockChecksumFlag: c_int,
}

impl Default for LZ4F_frameInfo_t {
    /// Mirrors `LZ4F_INIT_FRAMEINFO`.
    fn default() -> Self {
        LZ4F_frameInfo_t {
            blockSizeID: LZ4F_max64KB,
            blockMode: LZ4F_blockLinked,
            contentChecksumFlag: LZ4F_noContentChecksum,
            frameType: LZ4F_frame,
            contentSize: 0,
            dictID: 0,
            blockChecksumFlag: LZ4F_noBlockChecksum,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LZ4F_preferences_t {
    pub frameInfo: LZ4F_frameInfo_t,
    pub compressionLevel: c_int,
    pub autoFlush: c_uint,
    pub favorDecSpeed: c_uint,
    pub reserved: [c_uint; 3],
}

impl Default for LZ4F_preferences_t {
    /// Mirrors `LZ4F_INIT_PREFERENCES`.
    fn default() -> Self {
        LZ4F_preferences_t {
            frameInfo: LZ4F_frameInfo_t::default(),
            compressionLevel: 0,
            autoFlush: 0,
            favorDecSpeed: 0,
            reserved: [0; 3],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LZ4F_compressOptions_t {
    pub stableSrc: c_uint,
    pub reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LZ4F_decompressOptions_t {
    pub stableDst: c_uint,
    pub skipChecksums: c_uint,
    pub reserved1: c_uint,
    pub reserved0: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_CustomMem {
    pub customAlloc: Option<extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub customCalloc: Option<extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub customFree: Option<extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaqueState: *mut c_void,
}

impl Default for LZ4F_CustomMem {
    /// Mirrors `LZ4F_defaultCMem` — all NULL, defers to stdlib.
    fn default() -> Self {
        LZ4F_CustomMem {
            customAlloc: None,
            customCalloc: None,
            customFree: None,
            opaqueState: std::ptr::null_mut(),
        }
    }
}

// ---------------------------------------------------------------------------
// Constants from the C headers
// ---------------------------------------------------------------------------

// lz4.h
pub const LZ4_MAX_INPUT_SIZE: usize = 0x7E00_0000;
pub const LZ4_ACCELERATION_MAX: c_int = 65537;
pub const LZ4_ACCELERATION_DEFAULT: c_int = 1;
pub const LZ4_MEMORY_USAGE: usize = 14;
/// `static const int LZ4_64Klimit = ((64 KB) + (MFLIMIT-1));` (lz4.c:710).
/// Inputs `< LZ4_64Klimit` select `tableType == byU16`, `>=` selects `byU32`.
pub const LZ4_64Klimit: usize = 65536 + (MFLIMIT - 1); // 65547
pub const MFLIMIT: usize = 12;
pub const LASTLITERALS: usize = 5;
pub const MINMATCH: usize = 4;
pub const LZ4_DISTANCE_MAX: usize = 65535;

// lz4hc.h
pub const LZ4HC_CLEVEL_MIN: c_int = 2;
pub const LZ4HC_CLEVEL_DEFAULT: c_int = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: c_int = 10;
pub const LZ4HC_CLEVEL_MAX: c_int = 12;
pub const LZ4HC_DICTIONARY_LOGSIZE: usize = 16;
pub const LZ4HC_MAXD: usize = 1 << LZ4HC_DICTIONARY_LOGSIZE;

// lz4frame.h — blockSizeID
pub const LZ4F_default: c_int = 0;
pub const LZ4F_max64KB: c_int = 4;
pub const LZ4F_max256KB: c_int = 5;
pub const LZ4F_max1MB: c_int = 6;
pub const LZ4F_max4MB: c_int = 7;

// blockMode
pub const LZ4F_blockLinked: c_int = 0;
pub const LZ4F_blockIndependent: c_int = 1;

// contentChecksum
pub const LZ4F_noContentChecksum: c_int = 0;
pub const LZ4F_contentChecksumEnabled: c_int = 1;

// blockChecksum
pub const LZ4F_noBlockChecksum: c_int = 0;
pub const LZ4F_blockChecksumEnabled: c_int = 1;

// frameType
pub const LZ4F_frame: c_int = 0;
pub const LZ4F_skippableFrame: c_int = 1;

pub const LZ4F_VERSION: c_uint = 100;
pub const LZ4F_HEADER_SIZE_MIN: usize = 7;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;
pub const LZ4F_BLOCK_HEADER_SIZE: usize = 4;
pub const LZ4F_BLOCK_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_CONTENT_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_MAGICNUMBER: u32 = 0x184D2204;
pub const LZ4F_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;

/// `LZ4F_errorCodes` — numeric values follow the `LZ4F_LIST_ERRORS` macro order.
pub mod err {
    pub const OK_NoError: i32 = 0;
    pub const ERROR_GENERIC: i32 = 1;
    pub const ERROR_maxBlockSize_invalid: i32 = 2;
    pub const ERROR_blockMode_invalid: i32 = 3;
    pub const ERROR_parameter_invalid: i32 = 4;
    pub const ERROR_compressionLevel_invalid: i32 = 5;
    pub const ERROR_headerVersion_wrong: i32 = 6;
    pub const ERROR_blockChecksum_invalid: i32 = 7;
    pub const ERROR_reservedFlag_set: i32 = 8;
    pub const ERROR_allocation_failed: i32 = 9;
    pub const ERROR_srcSize_tooLarge: i32 = 10;
    pub const ERROR_dstMaxSize_tooSmall: i32 = 11;
    pub const ERROR_frameHeader_incomplete: i32 = 12;
    pub const ERROR_frameType_unknown: i32 = 13;
    pub const ERROR_frameSize_wrong: i32 = 14;
    pub const ERROR_srcPtr_wrong: i32 = 15;
    pub const ERROR_decompressionFailed: i32 = 16;
    pub const ERROR_headerChecksum_invalid: i32 = 17;
    pub const ERROR_contentChecksum_invalid: i32 = 18;
    pub const ERROR_frameDecoding_alreadyStarted: i32 = 19;
    pub const ERROR_compressionState_uninitialized: i32 = 20;
    pub const ERROR_parameter_null: i32 = 21;
    pub const ERROR_io_write: i32 = 22;
    pub const ERROR_io_read: i32 = 23;
    pub const ERROR_maxCode: i32 = 24;
}

// xxhash.h
pub const XXH_OK: c_int = 0;
pub const XXH_ERROR: c_int = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct XXH32_canonical_t {
    pub digest: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct XXH64_canonical_t {
    pub digest: [u8; 8],
}

// ---------------------------------------------------------------------------
// Commonly used function-pointer type aliases
// ---------------------------------------------------------------------------

pub type FnCompressDefault =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type FnCompressFast =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type FnDecompressSafe =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type FnCompressBound = unsafe extern "C" fn(c_int) -> c_int;

/// Returns `true` when a `size_t`-returning LZ4F function signalled an error,
/// replicating `LZ4F_isError()`.
pub fn lz4f_is_error(code: usize) -> bool {
    // LZ4F_isError: (code > (LZ4F_errorCode_t)(-LZ4F_ERROR_maxCode))
    code > (0usize.wrapping_sub(err::ERROR_maxCode as usize))
}

/// Decode a `size_t` LZ4F return value into its `LZ4F_errorCodes` number,
/// replicating `LZ4F_getErrorCode()`. Returns 0 (`OK_NoError`) for successes.
pub fn lz4f_error_code(code: usize) -> i32 {
    if !lz4f_is_error(code) {
        0
    } else {
        (0usize.wrapping_sub(code)) as i32
    }
}
