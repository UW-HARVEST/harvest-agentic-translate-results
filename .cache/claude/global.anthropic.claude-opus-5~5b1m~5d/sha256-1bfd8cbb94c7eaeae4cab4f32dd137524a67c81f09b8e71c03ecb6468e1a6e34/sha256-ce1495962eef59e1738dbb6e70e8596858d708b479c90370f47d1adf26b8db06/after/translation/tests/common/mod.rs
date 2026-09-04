//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries via `libloading`:
//!   * the reference C build  `c_src/build/libzstd.so`
//!   * the Rust build         `translation/target/release/libzstd.so`
//!
//! Every call in every test goes through `dlsym`, so the `#[no_mangle]`
//! export wrappers of the Rust crate are what is actually exercised.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void, CStr};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------- libraries

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest().join("../c_src/build/libzstd.so")
}

pub fn rs_so_path() -> PathBuf {
    // The cdylib under test. Built by `cargo build --release`.
    let p = manifest().join("target/release/libzstd.so");
    if p.exists() {
        return p;
    }
    manifest().join("target/debug/libzstd.so")
}

static C_LIB: OnceLock<Library> = OnceLock::new();
static R_LIB: OnceLock<Library> = OnceLock::new();

pub fn clib() -> &'static Library {
    C_LIB.get_or_init(|| {
        let p = c_so_path();
        assert!(
            p.exists(),
            "missing C shared library {p:?} — build it with\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {p:?}: {e}"))
    })
}

pub fn rlib() -> &'static Library {
    R_LIB.get_or_init(|| {
        let p = rs_so_path();
        assert!(
            p.exists(),
            "missing Rust shared library {p:?} — build it with `cargo build --release`"
        );
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {p:?}: {e}"))
    })
}

/// Resolve `name` in both libraries and return the two raw function pointers.
///
/// `T` must be a function-pointer type (`unsafe extern "C" fn(..) -> ..`).
pub unsafe fn duo<T: Copy>(name: &str) -> (T, T) {
    let c = *clib()
        .get::<T>(name.as_bytes())
        .unwrap_or_else(|e| panic!("C .so is missing `{name}`: {e}"));
    let r = *rlib()
        .get::<T>(name.as_bytes())
        .unwrap_or_else(|e| panic!("Rust .so is missing `{name}`: {e}"));
    (c, r)
}

/// Address of a symbol in both libraries (works for data symbols of any size).
pub unsafe fn duo_addr<T>(name: &str) -> (*mut T, *mut T) {
    let c = clib()
        .get::<*mut c_void>(name.as_bytes())
        .unwrap_or_else(|e| panic!("C .so is missing symbol `{name}`: {e}"));
    let r = rlib()
        .get::<*mut c_void>(name.as_bytes())
        .unwrap_or_else(|e| panic!("Rust .so is missing symbol `{name}`: {e}"));
    (
        c.into_raw().into_raw() as *mut T,
        r.into_raw().into_raw() as *mut T,
    )
}

/// Read a *data* symbol (by value) out of both libraries.
pub unsafe fn duo_value<T: Copy>(name: &str) -> (T, T) {
    let (c, r) = duo_addr::<T>(name);
    (*c, *r)
}

// ---------------------------------------------------------------- assertions

pub fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

#[track_caller]
pub fn eqv<T: PartialEq + std::fmt::Debug>(what: &str, c: T, r: T) {
    assert_eq!(c, r, "return value mismatch in {what}");
}

#[track_caller]
pub fn eqbuf(what: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    assert_eq!(c.len(), r.len(), "buffer length mismatch in {what}");
    for i in 0..c.len() {
        if c[i] != r[i] {
            panic!(
                "buffer mismatch in {what}: first difference at byte {i} \
                 (C=0x{:02x} Rust=0x{:02x}); len={}",
                c[i], r[i], c.len()
            );
        }
    }
}

// ---------------------------------------------------------------- rng / data

/// xoshiro256** — deterministic, fixed seed per test.
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut z = seed.wrapping_add(0x9E3779B97F4A7C15);
        let mut sm = || {
            z = z.wrapping_add(0x9E3779B97F4A7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
            x ^ (x >> 31)
        };
        Rng { s: [sm(), sm(), sm(), sm()] }
    }
    pub fn next_u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
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
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            lo
        } else {
            lo + self.below((hi - lo + 1) as usize) as i32
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(n);
        while v.len() < n {
            v.extend_from_slice(&self.next_u64().to_le_bytes());
        }
        v.truncate(n);
        v
    }
}

/// The content classes `CONFIGS.md` enumerates.
pub const N_CLASSES: usize = 8;
pub const CLASS_NAMES: [&str; N_CLASSES] = [
    "zeros",
    "single-byte-run",
    "two-byte-pattern",
    "incompressible-random",
    "text-like",
    "long-range-duplicates",
    "rle-runs",
    "already-compressed",
];

pub fn gen_class(class: usize, size: usize, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed ^ ((class as u64) << 40) ^ ((size as u64) << 8));
    match class % N_CLASSES {
        0 => vec![0u8; size],
        1 => {
            let b = rng.byte();
            vec![b; size]
        }
        2 => {
            let a = rng.byte();
            let b = rng.byte();
            (0..size).map(|i| if i % 2 == 0 { a } else { b }).collect()
        }
        3 => rng.bytes(size),
        4 => {
            // low-entropy alphabet, word-like structure
            const AL: &[u8] = b"abcdefghijklmnopqrstuvwxyz ,.\n";
            let mut v = Vec::with_capacity(size);
            while v.len() < size {
                let wl = 1 + rng.below(9);
                for _ in 0..wl {
                    v.push(AL[rng.below(AL.len() - 2)]);
                    if v.len() >= size {
                        break;
                    }
                }
                if v.len() < size {
                    v.push(b' ');
                }
            }
            v.truncate(size);
            v
        }
        5 => {
            // a few distinct blocks repeated far apart: bait for LDM / long matches
            let unit = (size / 8).max(1).min(64 * 1024);
            let a = rng.bytes(unit);
            let b = rng.bytes(unit);
            let mut v = Vec::with_capacity(size);
            let mut i = 0usize;
            while v.len() < size {
                let src = if i % 3 == 2 { &b } else { &a };
                let take = src.len().min(size - v.len());
                v.extend_from_slice(&src[..take]);
                i += 1;
            }
            v
        }
        6 => {
            let mut v = Vec::with_capacity(size);
            while v.len() < size {
                let b = rng.byte();
                let n = (1 + rng.below(300)).min(size - v.len());
                v.extend(std::iter::repeat(b).take(n));
            }
            v
        }
        _ => {
            // pseudo-"already compressed": high entropy with occasional structure
            let mut v = rng.bytes(size);
            let mut i = 0;
            while i + 16 < size {
                let b = v[i];
                for k in 0..8 {
                    v[i + k] = b;
                }
                i += 1 + rng.below(4096);
            }
            v
        }
    }
}

/// The input-size ladder `CONFIGS.md` enumerates (small end; large sizes are
/// used only where the row calls for them).
pub const SIZES: [usize; 13] = [
    0,
    1,
    7,
    128,
    1024,
    8 * 1024,
    64 * 1024,
    128 * 1024 - 1,
    128 * 1024,
    128 * 1024 + 1,
    200_000,
    256 * 1024,
    1024 * 1024,
];

pub const SMALL_SIZES: [usize; 8] = [0, 1, 2, 7, 63, 128, 1024, 8 * 1024];

// ---------------------------------------------------------------- constants

pub const ZSTD_CONTENTSIZE_UNKNOWN: c_ulonglong = 0u64.wrapping_sub(1);
pub const ZSTD_CONTENTSIZE_ERROR: c_ulonglong = 0u64.wrapping_sub(2);

// ZSTD_cParameter
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
// experimental (exact enum values from zstd.h L522-541)
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

/// Every public + experimental compression parameter, in header order.
pub const ALL_CPARAMS: &[(&str, c_int)] = &[
    ("compressionLevel", ZSTD_c_compressionLevel),
    ("windowLog", ZSTD_c_windowLog),
    ("hashLog", ZSTD_c_hashLog),
    ("chainLog", ZSTD_c_chainLog),
    ("searchLog", ZSTD_c_searchLog),
    ("minMatch", ZSTD_c_minMatch),
    ("targetLength", ZSTD_c_targetLength),
    ("strategy", ZSTD_c_strategy),
    ("targetCBlockSize", ZSTD_c_targetCBlockSize),
    ("enableLongDistanceMatching", ZSTD_c_enableLongDistanceMatching),
    ("ldmHashLog", ZSTD_c_ldmHashLog),
    ("ldmMinMatch", ZSTD_c_ldmMinMatch),
    ("ldmBucketSizeLog", ZSTD_c_ldmBucketSizeLog),
    ("ldmHashRateLog", ZSTD_c_ldmHashRateLog),
    ("contentSizeFlag", ZSTD_c_contentSizeFlag),
    ("checksumFlag", ZSTD_c_checksumFlag),
    ("dictIDFlag", ZSTD_c_dictIDFlag),
    ("nbWorkers", ZSTD_c_nbWorkers),
    ("jobSize", ZSTD_c_jobSize),
    ("overlapLog", ZSTD_c_overlapLog),
    ("rsyncable(exp1)", ZSTD_c_rsyncable),
    ("format(exp2)", ZSTD_c_format),
    ("forceMaxWindow(exp3)", ZSTD_c_forceMaxWindow),
    ("forceAttachDict(exp4)", ZSTD_c_forceAttachDict),
    ("literalCompressionMode(exp5)", ZSTD_c_literalCompressionMode),
    ("srcSizeHint(exp7)", ZSTD_c_srcSizeHint),
    ("enableDedicatedDictSearch(exp8)", ZSTD_c_enableDedicatedDictSearch),
    ("stableInBuffer(exp9)", ZSTD_c_stableInBuffer),
    ("stableOutBuffer(exp10)", ZSTD_c_stableOutBuffer),
    ("blockDelimiters(exp11)", ZSTD_c_blockDelimiters),
    ("validateSequences(exp12)", ZSTD_c_validateSequences),
    ("splitAfterSequences(exp13)", ZSTD_c_splitAfterSequences),
    ("useRowMatchFinder(exp14)", ZSTD_c_useRowMatchFinder),
    ("deterministicRefPrefix(exp15)", ZSTD_c_deterministicRefPrefix),
    ("prefetchCDictTables(exp16)", ZSTD_c_prefetchCDictTables),
    ("enableSeqProducerFallback(exp17)", ZSTD_c_enableSeqProducerFallback),
    ("maxBlockSize(exp18)", ZSTD_c_maxBlockSize),
    ("repcodeResolution(exp19)", ZSTD_c_repcodeResolution),
    ("blockSplitterLevel(exp20)", ZSTD_c_blockSplitterLevel),
];

// ZSTD_dParameter
pub const ZSTD_d_windowLogMax: c_int = 100;
pub const ZSTD_d_format: c_int = 1000;
pub const ZSTD_d_stableOutBuffer: c_int = 1001;
pub const ZSTD_d_forceIgnoreChecksum: c_int = 1002;
pub const ZSTD_d_refMultipleDDicts: c_int = 1003;
pub const ZSTD_d_disableHuffmanAssembly: c_int = 1004;
pub const ZSTD_d_maxBlockSize: c_int = 1005;

pub const ALL_DPARAMS: &[(&str, c_int)] = &[
    ("windowLogMax", 100),
    ("format(exp1)", 1000),
    ("stableOutBuffer(exp2)", 1001),
    ("forceIgnoreChecksum(exp3)", 1002),
    ("refMultipleDDicts(exp4)", 1003),
    ("disableHuffmanAssembly(exp5)", 1004),
    ("maxBlockSize(exp6)", 1005),
];

// ZSTD_ResetDirective
pub const ZSTD_reset_session_only: c_int = 1;
pub const ZSTD_reset_parameters: c_int = 2;
pub const ZSTD_reset_session_and_parameters: c_int = 3;

// ZSTD_EndDirective
pub const ZSTD_e_continue: c_int = 0;
pub const ZSTD_e_flush: c_int = 1;
pub const ZSTD_e_end: c_int = 2;

// ZSTD_strategy
pub const ZSTD_fast: c_int = 1;
pub const ZSTD_btultra2: c_int = 9;
pub const ALL_STRATEGIES: [c_int; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];

// ZSTD_dictLoadMethod_e / ZSTD_dictContentType_e
pub const ZSTD_dlm_byCopy: c_int = 0;
pub const ZSTD_dlm_byRef: c_int = 1;
pub const ZSTD_dct_auto: c_int = 0;
pub const ZSTD_dct_rawContent: c_int = 1;
pub const ZSTD_dct_fullDict: c_int = 2;

// ZSTD_ParamSwitch_e
pub const ZSTD_ps_auto: c_int = 0;
pub const ZSTD_ps_enable: c_int = 1;
pub const ZSTD_ps_disable: c_int = 2;

pub const ZSTD_MAGICNUMBER: u32 = 0xFD2FB528;
pub const ZSTD_MAGIC_DICTIONARY: u32 = 0xEC30A437;
pub const ZSTD_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;
pub const ZSTD_BLOCKSIZE_MAX: usize = 128 * 1024;

pub const LEGACY_MAGICS: [u32; 7] = [
    0x1EA92542, // v0.1
    0xFD2FB522, // v0.2
    0xFD2FB523, // v0.3
    0xFD2FB524, // v0.4
    0xFD2FB525, // v0.5
    0xFD2FB526, // v0.6
    0xFD2FB527, // v0.7
];

// ---------------------------------------------------------------- structs

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_bounds {
    pub error: usize,
    pub lowerBound: c_int,
    pub upperBound: c_int,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZSTD_inBuffer {
    pub src: *const c_void,
    pub size: usize,
    pub pos: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZSTD_outBuffer {
    pub dst: *mut c_void,
    pub size: usize,
    pub pos: usize,
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
pub struct ZSTD_frameProgression {
    pub ingested: c_ulonglong,
    pub consumed: c_ulonglong,
    pub produced: c_ulonglong,
    pub flushed: c_ulonglong,
    pub currentJobID: c_uint,
    pub nbActiveWorkers: c_uint,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZSTD_customMem {
    pub customAlloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub customFree: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}

impl Default for ZSTD_customMem {
    fn default() -> Self {
        ZSTD_customMem { customAlloc: None, customFree: None, opaque: std::ptr::null_mut() }
    }
}

// ---------------------------------------------------------------- fn types

pub type FnSizeT0 = unsafe extern "C" fn() -> usize;
pub type FnUint0 = unsafe extern "C" fn() -> c_uint;
pub type FnInt0 = unsafe extern "C" fn() -> c_int;
pub type FnPtr0 = unsafe extern "C" fn() -> *mut c_void;
pub type FnFreePtr = unsafe extern "C" fn(*mut c_void) -> usize;
pub type FnSizeT1 = unsafe extern "C" fn(usize) -> usize;
pub type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
pub type FnErrName = unsafe extern "C" fn(usize) -> *const c_char;

pub type FnCompress =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_int) -> usize;
pub type FnDecompress = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;
pub type FnCompressCCtx =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize, c_int) -> usize;
pub type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;

pub type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
pub type FnGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> usize;
pub type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
pub type FnStream2 =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer, c_int) -> usize;
pub type FnDStream = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize;
pub type FnGetBounds = unsafe extern "C" fn(c_int) -> ZSTD_bounds;

// ---------------------------------------------------------------- helpers

/// A pair of contexts (one per library) created/destroyed together.
pub struct CtxPair {
    pub c: *mut c_void,
    pub r: *mut c_void,
    free_c: FnFreePtr,
    free_r: FnFreePtr,
}

impl CtxPair {
    pub unsafe fn new(create: &str, free: &str) -> CtxPair {
        let (cc, cr) = duo::<FnPtr0>(create);
        let (fc, fr) = duo::<FnFreePtr>(free);
        let c = cc();
        let r = cr();
        assert!(!c.is_null(), "{create} returned NULL in C");
        assert!(!r.is_null(), "{create} returned NULL in Rust");
        CtxPair { c, r, free_c: fc, free_r: fr }
    }
    pub unsafe fn cctx() -> CtxPair {
        CtxPair::new("ZSTD_createCCtx", "ZSTD_freeCCtx")
    }
    pub unsafe fn dctx() -> CtxPair {
        CtxPair::new("ZSTD_createDCtx", "ZSTD_freeDCtx")
    }
    pub unsafe fn cstream() -> CtxPair {
        CtxPair::new("ZSTD_createCStream", "ZSTD_freeCStream")
    }
    pub unsafe fn dstream() -> CtxPair {
        CtxPair::new("ZSTD_createDStream", "ZSTD_freeDStream")
    }
    pub unsafe fn cctx_params() -> CtxPair {
        CtxPair::new("ZSTD_createCCtxParams", "ZSTD_freeCCtxParams")
    }
}

impl Drop for CtxPair {
    fn drop(&mut self) {
        unsafe {
            if !self.c.is_null() {
                (self.free_c)(self.c);
            }
            if !self.r.is_null() {
                (self.free_r)(self.r);
            }
        }
    }
}

/// Run `f` against both libraries' function pointers and compare the returned
/// `usize` plus the destination buffer contents.
///
/// `f(fnptr, dst) -> ret`
#[track_caller]
pub unsafe fn diff_call<T: Copy, F>(what: &str, name: &str, dstlen: usize, mut f: F)
where
    F: FnMut(T, &mut [u8]) -> usize,
{
    let (fc, fr) = duo::<T>(name);
    let mut dc = vec![0xA5u8; dstlen];
    let mut dr = vec![0xA5u8; dstlen];
    let rc = f(fc, &mut dc);
    let rr = f(fr, &mut dr);
    eqv(&format!("{what} [{name}] return"), rc, rr);
    eqbuf(&format!("{what} [{name}] dst"), &dc, &dr);
}

/// Compress `src` with the C library at `level`; panics on error.
pub unsafe fn c_compress(src: &[u8], level: c_int) -> Vec<u8> {
    let (bound, _) = duo::<FnSizeT1>("ZSTD_compressBound");
    let (comp, _) = duo::<FnCompress>("ZSTD_compress");
    let (iserr, _) = duo::<FnIsError>("ZSTD_isError");
    let cap = bound(src.len());
    let mut dst = vec![0u8; cap.max(64)];
    let n = comp(
        dst.as_mut_ptr() as *mut c_void,
        dst.len(),
        src.as_ptr() as *const c_void,
        src.len(),
        level,
    );
    assert_eq!(iserr(n), 0, "helper c_compress failed");
    dst.truncate(n);
    dst
}

pub fn is_err(n: usize) -> bool {
    n > usize::MAX - 130
}
