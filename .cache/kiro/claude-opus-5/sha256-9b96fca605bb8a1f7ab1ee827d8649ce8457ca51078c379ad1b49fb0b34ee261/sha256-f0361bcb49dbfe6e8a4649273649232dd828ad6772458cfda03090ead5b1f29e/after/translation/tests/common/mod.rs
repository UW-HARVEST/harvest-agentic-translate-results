//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries via `libloading` and never calls any Rust
//! function directly, so the `#[no_mangle]`/`extern "C"` export wrappers are
//! part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type Sz = usize;

// ---------------------------------------------------------------- library pair

pub struct Pair {
    pub c: Library,
    pub r: Library,
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest().join("../c_src/build/libzstd.so")
}

pub fn r_so_path() -> PathBuf {
    manifest().join("target/release/libzstd.so")
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| {
        let cp = c_so_path();
        let rp = r_so_path();
        assert!(cp.exists(), "C .so not built: {}", cp.display());
        assert!(
            rp.exists(),
            "Rust .so not built (cargo build --release): {}",
            rp.display()
        );
        unsafe {
            Pair {
                c: Library::new(&cp).expect("load C .so"),
                r: Library::new(&rp).expect("load Rust .so"),
            }
        }
    })
}

impl Pair {
    /// Fetch the same symbol from both libraries. Panics with the symbol name
    /// if either library is missing it.
    pub fn sym<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        let cs: Symbol<T> = unsafe { self.c.get(&bytes) }
            .unwrap_or_else(|e| panic!("C .so missing symbol {name}: {e}"));
        let rs: Symbol<T> = unsafe { self.r.get(&bytes) }
            .unwrap_or_else(|e| panic!("Rust .so missing symbol {name}: {e}"));
        (cs, rs)
    }

    pub fn has(&self, name: &str) -> bool {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        unsafe { self.c.get::<*const c_void>(&bytes) }.is_ok()
            && unsafe { self.r.get::<*const c_void>(&bytes) }.is_ok()
    }
}

// ---------------------------------------------------------------------- PRNG

/// Deterministic xoshiro-style PRNG so every row is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_2718;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

// ------------------------------------------------------------- payload shapes

/// The entropy shapes the C special-cases (RLE block, raw block, huffman
/// literals, FSE `set_basic`/`set_rle`/`set_compressed`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Zeros,
    SingleByte,
    TwoSymbol,
    Uniform256,
    TextLike,
    Repetitive,
    Incompressible,
    MixedEntropy,
}

pub const ALL_SHAPES: [Shape; 8] = [
    Shape::Zeros,
    Shape::SingleByte,
    Shape::TwoSymbol,
    Shape::Uniform256,
    Shape::TextLike,
    Shape::Repetitive,
    Shape::Incompressible,
    Shape::MixedEntropy,
];

pub fn gen(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    match shape {
        Shape::Zeros => v.resize(len, 0),
        Shape::SingleByte => {
            let b = rng.byte();
            v.resize(len, b);
        }
        Shape::TwoSymbol => {
            let (a, b) = (rng.byte(), rng.byte());
            for _ in 0..len {
                v.push(if rng.next_u32() & 1 == 0 { a } else { b });
            }
        }
        Shape::Uniform256 => {
            for i in 0..len {
                v.push((i % 256) as u8);
            }
        }
        Shape::TextLike => {
            const W: [&str; 12] = [
                "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ", "zstd ",
                "compress ", "and ", "decompress ",
            ];
            while v.len() < len {
                v.extend_from_slice(W[rng.below(12) as usize].as_bytes());
            }
            v.truncate(len);
        }
        Shape::Repetitive => {
            let plen = 4 + rng.below(60) as usize;
            let pat: Vec<u8> = (0..plen).map(|_| rng.byte()).collect();
            while v.len() < len {
                v.extend_from_slice(&pat);
            }
            v.truncate(len);
        }
        Shape::Incompressible => {
            for _ in 0..len {
                v.push(rng.byte());
            }
        }
        Shape::MixedEntropy => {
            // entropy changes mid-buffer -> exercises the block splitter
            let mut i = 0usize;
            let mut which = 0u32;
            while i < len {
                let seg = (len / 4).max(1).min(len - i);
                match which % 4 {
                    0 => v.extend(std::iter::repeat(0u8).take(seg)),
                    1 => v.extend((0..seg).map(|_| rng.byte())),
                    2 => v.extend((0..seg).map(|k| (k % 3) as u8)),
                    _ => v.extend((0..seg).map(|k| b'a' + (k % 5) as u8)),
                }
                i += seg;
                which += 1;
            }
            v.truncate(len);
        }
    }
    v
}

/// The size axis the C special-cases.
pub const SIZE_AXIS: [usize; 24] = [
    0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 63, 127, 128, 255, 256, 1023, 1024, 4096, 65535, 65536,
    131_071, 131_072, 131_073, 300_000,
];

/// Smaller axis for rows whose cross-product is large.
pub const SIZE_AXIS_SMALL: [usize; 10] = [0, 1, 3, 8, 63, 256, 1024, 4096, 65536, 131_073];

// --------------------------------------------------------------- diff helpers

#[track_caller]
pub fn eq<T: PartialEq + std::fmt::Debug>(what: &str, c: T, r: T) {
    assert_eq!(c, r, "divergence in {what}: C={c:?} Rust={r:?}");
}

#[track_caller]
pub fn eq_bytes(what: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    if c.len() != r.len() {
        panic!("divergence in {what}: length C={} Rust={}", c.len(), r.len());
    }
    let i = c.iter().zip(r).position(|(a, b)| a != b).unwrap();
    panic!(
        "divergence in {}: first differing byte at {} of {}: C=0x{:02x} Rust=0x{:02x}\n  C[{}..]={:02x?}\n  R[{}..]={:02x?}",
        what,
        i,
        c.len(),
        c[i],
        r[i],
        i,
        &c[i..(i + 16).min(c.len())],
        i,
        &r[i..(i + 16).min(r.len())],
    );
}

// ------------------------------------------------------- common ZSTD bindings

pub const CONTENTSIZE_UNKNOWN: u64 = u64::MAX;
pub const CONTENTSIZE_ERROR: u64 = u64::MAX - 1;

pub type FnCompress =
    unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz, c_int) -> Sz;
pub type FnDecompress = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz) -> Sz;
pub type FnIsError = unsafe extern "C" fn(Sz) -> c_uint;
pub type FnGetErrCode = unsafe extern "C" fn(Sz) -> c_int;
pub type FnGetErrName = unsafe extern "C" fn(Sz) -> *const c_char;
pub type FnCompressBound = unsafe extern "C" fn(Sz) -> Sz;
pub type FnCreateCtx = unsafe extern "C" fn() -> *mut c_void;
pub type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> Sz;
pub type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> Sz;
pub type FnGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> Sz;
pub type FnGetBounds = unsafe extern "C" fn(c_int) -> Bounds;
pub type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> Sz;
pub type FnVoidInt = unsafe extern "C" fn() -> c_int;
pub type FnVoidUint = unsafe extern "C" fn() -> c_uint;
pub type FnVoidSz = unsafe extern "C" fn() -> Sz;
pub type FnVoidStr = unsafe extern "C" fn() -> *const c_char;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Bounds {
    pub error: Sz,
    pub lower: c_int,
    pub upper: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct InBuffer {
    pub src: *const c_void,
    pub size: Sz,
    pub pos: Sz,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct OutBuffer {
    pub dst: *mut c_void,
    pub size: Sz,
    pub pos: Sz,
}

/// `ZSTD_frameHeader` — layout from `c_src/src/include/zstd.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct FrameHeader {
    pub frame_content_size: u64,
    pub window_size: u64,
    pub block_size_max: c_uint,
    pub frame_type: c_uint,
    pub header_size: c_uint,
    pub dict_id: c_uint,
    pub checksum_flag: c_uint,
    pub _reserved1: c_uint,
    pub _reserved2: c_uint,
}

/// `ZSTD_compressionParameters`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CParams {
    pub window_log: c_uint,
    pub chain_log: c_uint,
    pub hash_log: c_uint,
    pub search_log: c_uint,
    pub min_match: c_uint,
    pub target_length: c_uint,
    pub strategy: c_uint,
}

/// `ZSTD_frameParameters`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct FParams {
    pub content_size_flag: c_int,
    pub checksum_flag: c_int,
    pub no_dict_id_flag: c_int,
}

/// `ZSTD_parameters`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Params {
    pub c_params: CParams,
    pub f_params: FParams,
}

/// `ZSTD_Sequence`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Sequence {
    pub offset: c_uint,
    pub lit_length: c_uint,
    pub match_length: c_uint,
    pub rep: c_uint,
}

/// `ZSTD_bounds`-style out-param free helpers used everywhere below.
pub struct Ctx<'a> {
    pub p: &'a Pair,
}

// Convenience: read a `*const c_char` as an owned String (or a marker).
pub fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    unsafe { std::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned()
}

// ---- parameter enum tables (mechanically from zstd.h) ----

/// The 39 valid `ZSTD_cParameter` values (public + experimental aliases).
pub const C_PARAMS: [(c_int, &str); 39] = [
    (100, "compressionLevel"),
    (101, "windowLog"),
    (102, "hashLog"),
    (103, "chainLog"),
    (104, "searchLog"),
    (105, "minMatch"),
    (106, "targetLength"),
    (107, "strategy"),
    (130, "targetCBlockSize"),
    (160, "enableLongDistanceMatching"),
    (161, "ldmHashLog"),
    (162, "ldmMinMatch"),
    (163, "ldmBucketSizeLog"),
    (164, "ldmHashRateLog"),
    (200, "contentSizeFlag"),
    (201, "checksumFlag"),
    (202, "dictIDFlag"),
    (400, "nbWorkers"),
    (401, "jobSize"),
    (402, "overlapLog"),
    (500, "rsyncable"),
    (10, "format"),
    (1000, "forceMaxWindow"),
    (1001, "forceAttachDict"),
    (1002, "literalCompressionMode"),
    (1004, "srcSizeHint"),
    (1005, "enableDedicatedDictSearch"),
    (1006, "stableInBuffer"),
    (1007, "stableOutBuffer"),
    (1008, "blockDelimiters"),
    (1009, "validateSequences"),
    (1010, "splitAfterSequences"),
    (1011, "useRowMatchFinder"),
    (1012, "deterministicRefPrefix"),
    (1013, "prefetchCDictTables"),
    (1014, "enableSeqProducerFallback"),
    (1015, "maxBlockSize"),
    (1016, "repcodeResolution"),
    (1017, "blockSplitterLevel"),
];

/// The valid `ZSTD_dParameter` values (`zstd.h`: `ZSTD_d_windowLogMax=100`,
/// `ZSTD_d_experimentalParam1..6 = 1000..1005`).
pub const D_PARAMS: [(c_int, &str); 7] = [
    (100, "windowLogMax"),
    (1000, "format"),
    (1001, "stableOutBuffer"),
    (1002, "forceIgnoreChecksum"),
    (1003, "refMultipleDDicts"),
    (1004, "disableHuffmanAssembly"),
    (1005, "maxBlockSize"),
];

/// Out-of-range ints deliberately fed across the FFI boundary as enum values.
pub const BAD_ENUM_INTS: [c_int; 12] = [
    -1,
    0,
    3,
    9,
    99,
    108,
    9999,
    100_000,
    -100_000,
    c_int::MIN,
    c_int::MAX,
    c_int::MIN + 1,
];

pub const STRATEGIES: [c_int; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
