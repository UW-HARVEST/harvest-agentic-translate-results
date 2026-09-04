//! Shared differential-test harness.
//!
//! Loads BOTH the C `libzstd.so` and the Rust `libzstd.so` via `libloading`
//! and exposes every symbol through the FFI boundary only. No Rust function is
//! ever called directly — this is what exercises the `#[no_mangle]` wrappers.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type size_t = usize;

// ---------------------------------------------------------------- structs ----

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_bounds {
    pub error: size_t,
    pub lowerBound: c_int,
    pub upperBound: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_inBuffer {
    pub src: *const c_void,
    pub size: size_t,
    pub pos: size_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_outBuffer {
    pub dst: *mut c_void,
    pub size: size_t,
    pub pos: size_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_Sequence {
    pub offset: c_uint,
    pub litLength: c_uint,
    pub matchLength: c_uint,
    pub rep: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_frameHeader {
    pub frameContentSize: c_ulonglong,
    pub windowSize: c_ulonglong,
    pub blockSizeMax: c_uint,
    pub frameType: c_uint,
    pub headerSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
    pub _reserved1: c_uint,
    pub _reserved2: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_compressionParameters {
    pub windowLog: c_uint,
    pub chainLog: c_uint,
    pub hashLog: c_uint,
    pub searchLog: c_uint,
    pub minMatch: c_uint,
    pub targetLength: c_uint,
    pub strategy: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_frameParameters {
    pub contentSizeFlag: c_int,
    pub checksumFlag: c_int,
    pub noDictIDFlag: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_parameters {
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZDICT_params_t {
    pub compressionLevel: c_int,
    pub notificationLevel: c_uint,
    pub dictID: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ZDICT_cover_params_t {
    pub k: c_uint,
    pub d: c_uint,
    pub steps: c_uint,
    pub nbThreads: c_uint,
    pub splitPoint: f64,
    pub shrinkDict: c_uint,
    pub shrinkDictMaxRegression: c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ZDICT_fastCover_params_t {
    pub k: c_uint,
    pub d: c_uint,
    pub f: c_uint,
    pub steps: c_uint,
    pub nbThreads: c_uint,
    pub splitPoint: f64,
    pub accel: c_uint,
    pub shrinkDict: c_uint,
    pub shrinkDictMaxRegression: c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZDICT_legacy_params_t {
    pub selectivityLevel: c_uint,
    pub zParams: ZDICT_params_t,
}

// ------------------------------------------------------------- constants ----

pub const ZSTD_CONTENTSIZE_UNKNOWN: c_ulonglong = 0u64.wrapping_sub(1);
pub const ZSTD_CONTENTSIZE_ERROR: c_ulonglong = 0u64.wrapping_sub(2);

// compression parameters
pub const ZSTD_c_compressionLevel: c_int = 100;
pub const ZSTD_c_windowLog: c_int = 101;
pub const ZSTD_c_hashLog: c_int = 102;
pub const ZSTD_c_chainLog: c_int = 103;
pub const ZSTD_c_searchLog: c_int = 104;
pub const ZSTD_c_minMatch: c_int = 105;
pub const ZSTD_c_targetLength: c_int = 106;
pub const ZSTD_c_strategy: c_int = 107;
pub const ZSTD_c_targetCBlockSize: c_int = 130;
pub const ZSTD_c_enableLongDistanceMatching: c_int = 160;
pub const ZSTD_c_ldmHashLog: c_int = 161;
pub const ZSTD_c_ldmMinMatch: c_int = 162;
pub const ZSTD_c_ldmBucketSizeLog: c_int = 163;
pub const ZSTD_c_ldmHashRateLog: c_int = 164;
pub const ZSTD_c_contentSizeFlag: c_int = 200;
pub const ZSTD_c_checksumFlag: c_int = 201;
pub const ZSTD_c_dictIDFlag: c_int = 202;
pub const ZSTD_c_nbWorkers: c_int = 400;
pub const ZSTD_c_jobSize: c_int = 401;
pub const ZSTD_c_overlapLog: c_int = 402;
// experimental (macro aliases in zstd.h)
pub const ZSTD_c_rsyncable: c_int = 500; // experimentalParam1
pub const ZSTD_c_format: c_int = 10; // experimentalParam2
pub const ZSTD_c_forceMaxWindow: c_int = 1000; // experimentalParam3
pub const ZSTD_c_forceAttachDict: c_int = 1001; // experimentalParam4
pub const ZSTD_c_literalCompressionMode: c_int = 1002; // experimentalParam5
pub const ZSTD_c_srcSizeHint: c_int = 1004; // experimentalParam7
pub const ZSTD_c_enableDedicatedDictSearch: c_int = 1005; // experimentalParam8
pub const ZSTD_c_stableInBuffer: c_int = 1006; // experimentalParam9
pub const ZSTD_c_stableOutBuffer: c_int = 1007; // experimentalParam10
pub const ZSTD_c_blockDelimiters: c_int = 1008; // experimentalParam11
pub const ZSTD_c_validateSequences: c_int = 1009; // experimentalParam12
pub const ZSTD_c_splitAfterSequences: c_int = 1010; // experimentalParam13
pub const ZSTD_c_useRowMatchFinder: c_int = 1011; // experimentalParam14
pub const ZSTD_c_deterministicRefPrefix: c_int = 1012; // experimentalParam15
pub const ZSTD_c_prefetchCDictTables: c_int = 1013; // experimentalParam16
pub const ZSTD_c_enableSeqProducerFallback: c_int = 1014; // experimentalParam17
pub const ZSTD_c_maxBlockSize: c_int = 1015; // experimentalParam18
pub const ZSTD_c_repcodeResolution: c_int = 1016; // experimentalParam19
pub const ZSTD_c_blockSplitterLevel: c_int = 1017; // experimentalParam20

// decompression parameters
pub const ZSTD_d_windowLogMax: c_int = 100;
pub const ZSTD_d_format: c_int = 1000; // experimentalParam1
pub const ZSTD_d_stableOutBuffer: c_int = 1001; // experimentalParam2
pub const ZSTD_d_forceIgnoreChecksum: c_int = 1002; // experimentalParam3
pub const ZSTD_d_refMultipleDDicts: c_int = 1003; // experimentalParam4
pub const ZSTD_d_disableHuffmanAssembly: c_int = 1004; // experimentalParam5
pub const ZSTD_d_maxBlockSize: c_int = 1005; // experimentalParam6

// enums
pub const ZSTD_fast: c_int = 1;
pub const ZSTD_dfast: c_int = 2;
pub const ZSTD_greedy: c_int = 3;
pub const ZSTD_lazy: c_int = 4;
pub const ZSTD_lazy2: c_int = 5;
pub const ZSTD_btlazy2: c_int = 6;
pub const ZSTD_btopt: c_int = 7;
pub const ZSTD_btultra: c_int = 8;
pub const ZSTD_btultra2: c_int = 9;

pub const ZSTD_e_continue: c_int = 0;
pub const ZSTD_e_flush: c_int = 1;
pub const ZSTD_e_end: c_int = 2;

pub const ZSTD_reset_session_only: c_int = 1;
pub const ZSTD_reset_parameters: c_int = 2;
pub const ZSTD_reset_session_and_parameters: c_int = 3;

pub const ZSTD_dct_auto: c_int = 0;
pub const ZSTD_dct_rawContent: c_int = 1;
pub const ZSTD_dct_fullDict: c_int = 2;

pub const ZSTD_dlm_byCopy: c_int = 0;
pub const ZSTD_dlm_byRef: c_int = 1;

pub const ZSTD_f_zstd1: c_int = 0;
pub const ZSTD_f_zstd1_magicless: c_int = 1;

pub const ZSTD_lcm_auto: c_int = 0;
pub const ZSTD_lcm_huffman: c_int = 1;
pub const ZSTD_lcm_uncompressed: c_int = 2;

pub const ZSTD_urm_auto: c_int = 0;
pub const ZSTD_urm_disableRowMatchFinder: c_int = 1;
pub const ZSTD_urm_enableRowMatchFinder: c_int = 2;

pub const ZSTD_ps_auto: c_int = 0;
pub const ZSTD_ps_enable: c_int = 1;
pub const ZSTD_ps_disable: c_int = 2;

pub const ZSTD_sf_noBlockDelimiters: c_int = 0;
pub const ZSTD_sf_explicitBlockDelimiters: c_int = 1;

pub const ZSTD_d_validateChecksum: c_int = 0;
pub const ZSTD_d_ignoreChecksum: c_int = 1;

pub const ZSTD_rmd_refSingleDDict: c_int = 0;
pub const ZSTD_rmd_refMultipleDDicts: c_int = 1;

pub const ZSTD_MAGICNUMBER: c_uint = 0xFD2FB528;
pub const ZSTD_MAGIC_DICTIONARY: c_uint = 0xEC30A437;
pub const ZSTD_MAGIC_SKIPPABLE_START: c_uint = 0x184D2A50;

// ------------------------------------------------------------ lib loading ----

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

fn root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

pub fn c_so_path() -> PathBuf {
    root().join("c_src/build/libzstd.so")
}

pub fn rs_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libzstd.so")
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c = c_so_path();
        let r = rs_so_path();
        assert!(c.exists(), "missing C .so at {c:?} — build c_src first");
        assert!(r.exists(), "missing Rust .so at {r:?} — cargo build --release");
        unsafe {
            Libs {
                c: Library::new(&c).expect("dlopen C libzstd.so"),
                rs: Library::new(&r).expect("dlopen Rust libzstd.so"),
            }
        }
    })
}

/// Fetch the same symbol from both libraries.
pub fn pair<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let l = libs();
    let n = format!("{name}\0");
    let b = n.as_bytes();
    unsafe {
        let c: Symbol<'static, T> = l
            .c
            .get(b)
            .unwrap_or_else(|e| panic!("C .so missing symbol {name}: {e}"));
        let r: Symbol<'static, T> = l
            .rs
            .get(b)
            .unwrap_or_else(|e| panic!("Rust .so missing symbol {name}: {e}"));
        (c, r)
    }
}

/// `(c_fn, rs_fn)` for a function-pointer type.
#[macro_export]
macro_rules! fnpair {
    ($name:literal, $ty:ty) => {{
        let (c, r) = $crate::common::pair::<$ty>($name);
        (*c, *r)
    }};
}

// -------------------------------------------------------------- utilities ----

/// Deterministic xorshift64* PRNG so every test run is reproducible.
pub struct Rng(pub u64);

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + self.below((hi - lo + 1) as usize) as i32
    }
}

/// Input shape generators — cover the data-dependent branches (all-zero,
/// highly repetitive, low-entropy text, incompressible random, mixed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Zeros,
    Repetitive,
    Text,
    Random,
    Mixed,
    /// long-range matches, to exercise LDM
    LongRange,
    /// single repeated byte value != 0
    SingleByte,
    /// 2-symbol alphabet (RLE / tiny huffman table)
    TwoSymbol,
}

pub const ALL_SHAPES: [Shape; 8] = [
    Shape::Zeros,
    Shape::Repetitive,
    Shape::Text,
    Shape::Random,
    Shape::Mixed,
    Shape::LongRange,
    Shape::SingleByte,
    Shape::TwoSymbol,
];

pub fn gen(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    match shape {
        Shape::Zeros => v.resize(len, 0u8),
        Shape::SingleByte => {
            let b = (rng.next_u32() & 0xFF) as u8;
            v.resize(len, b)
        }
        Shape::TwoSymbol => {
            let a = (rng.next_u32() & 0xFF) as u8;
            let b = a ^ 0x5A;
            for _ in 0..len {
                v.push(if rng.next_u64() & 1 == 0 { a } else { b });
            }
        }
        Shape::Repetitive => {
            let plen = 1 + rng.below(32);
            let pat: Vec<u8> = (0..plen).map(|_| (rng.next_u32() & 0xFF) as u8).collect();
            while v.len() < len {
                v.push(pat[v.len() % plen]);
            }
        }
        Shape::Text => {
            const W: [&str; 12] = [
                "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ", "zstd ",
                "compress ", "data ", "block ",
            ];
            while v.len() < len {
                v.extend_from_slice(W[rng.below(W.len())].as_bytes());
            }
            v.truncate(len);
        }
        Shape::Random => {
            while v.len() < len {
                v.extend_from_slice(&rng.next_u64().to_le_bytes());
            }
            v.truncate(len);
        }
        Shape::Mixed => {
            while v.len() < len {
                if rng.next_u64() & 3 == 0 {
                    v.extend_from_slice(&rng.next_u64().to_le_bytes());
                } else {
                    let n = 1 + rng.below(64);
                    let b = (rng.next_u32() & 0xFF) as u8;
                    for _ in 0..n {
                        v.push(b);
                    }
                }
            }
            v.truncate(len);
        }
        Shape::LongRange => {
            // build a chunk, then repeat it far apart with small mutations
            let chunk: Vec<u8> = {
                let n = 1024.min(len.max(1));
                let mut c = Vec::with_capacity(n);
                while c.len() < n {
                    c.extend_from_slice(&rng.next_u64().to_le_bytes());
                }
                c.truncate(n);
                c
            };
            while v.len() < len {
                v.extend_from_slice(&chunk);
                let pad = rng.below(4096);
                for _ in 0..pad {
                    v.push((rng.next_u32() & 0xFF) as u8);
                }
            }
            v.truncate(len);
        }
    }
    v.truncate(len);
    while v.len() < len {
        v.push(0);
    }
    v
}

/// Pretty diff assertion for byte buffers.
#[track_caller]
pub fn assert_bytes_eq(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    if c.len() != r.len() {
        panic!("{ctx}: length mismatch C={} Rust={}", c.len(), r.len());
    }
    let i = c.iter().zip(r.iter()).position(|(a, b)| a != b).unwrap();
    let lo = i.saturating_sub(8);
    let hi = (i + 8).min(c.len());
    panic!(
        "{ctx}: first byte diff at {i} (len {}): C={:02x?} Rust={:02x?}",
        c.len(),
        &c[lo..hi],
        &r[lo..hi]
    );
}

// ------------------------------------------------ common signature aliases ----

pub type FnVoidPtr = unsafe extern "C" fn() -> *mut c_void;
pub type FnPtrSize = unsafe extern "C" fn(*mut c_void) -> size_t;
pub type FnSizeSize = unsafe extern "C" fn(size_t) -> size_t;
pub type FnIsError = unsafe extern "C" fn(size_t) -> c_uint;
pub type FnErrName = unsafe extern "C" fn(size_t) -> *const c_char;
pub type FnGetErrorCode = unsafe extern "C" fn(size_t) -> c_int;
pub type FnCompress =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
pub type FnDecompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
pub type FnCCtxCompress =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
pub type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
pub type FnGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> size_t;
pub type FnBounds = unsafe extern "C" fn(c_int) -> ZSTD_bounds;
pub type FnStream = unsafe extern "C" fn(
    *mut c_void,
    *mut ZSTD_outBuffer,
    *mut ZSTD_inBuffer,
    c_int,
) -> size_t;
pub type FnDStream = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> size_t;

pub fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    unsafe { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
}
