//! Shared differential-testing harness.
//!
//! Loads BOTH the C `libzstd.so` and the Rust `libzstd.so` via `libloading` and
//! exposes typed symbol lookup so every call crosses the real FFI boundary
//! (exercising the `#[no_mangle] extern "C"` export wrappers).
//!
//! Never call Rust functions directly — always go through `rs()`.
#![allow(dead_code, non_snake_case, non_camel_case_types, non_upper_case_globals)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type size_t = usize;

// ---------------------------------------------------------------- library load

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn c_so_path() -> PathBuf {
    repo_root().join("c_src/build/libzstd.so")
}

fn rs_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/release/libzstd.so");
    p
}

static C_LIB: OnceLock<Library> = OnceLock::new();
static RS_LIB: OnceLock<Library> = OnceLock::new();

pub fn c() -> &'static Library {
    C_LIB.get_or_init(|| {
        let p = c_so_path();
        assert!(p.exists(), "C .so not built: {}", p.display());
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()))
    })
}

pub fn rs() -> &'static Library {
    RS_LIB.get_or_init(|| {
        let p = rs_so_path();
        assert!(
            p.exists(),
            "Rust .so not built ({}). Run `cargo build --release` first.",
            p.display()
        );
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()))
    })
}

/// Look up `name` in `lib`, panicking with a clear message if absent.
pub unsafe fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    let mut owned = name.as_bytes().to_vec();
    owned.push(0);
    lib.get::<T>(&owned)
        .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"))
}

/// Both libraries at once. `.0` = C, `.1` = Rust.
pub unsafe fn both<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    (sym::<T>(c(), name), sym::<T>(rs(), name))
}

/// True if `name` is resolvable in both libraries.
pub fn has_both(name: &str) -> bool {
    let mut owned = name.as_bytes().to_vec();
    owned.push(0);
    unsafe {
        c().get::<*const c_void>(&owned).is_ok() && rs().get::<*const c_void>(&owned).is_ok()
    }
}

// -------------------------------------------------------------------- typedefs

pub type FnIsError = unsafe extern "C" fn(size_t) -> c_uint;
pub type FnGetErrorCode = unsafe extern "C" fn(size_t) -> c_int;
pub type FnGetErrorName = unsafe extern "C" fn(size_t) -> *const c_char;
pub type FnVoidToUint = unsafe extern "C" fn() -> c_uint;
pub type FnVoidToSize = unsafe extern "C" fn() -> size_t;
pub type FnVoidToInt = unsafe extern "C" fn() -> c_int;
pub type FnVoidToPtr = unsafe extern "C" fn() -> *mut c_void;
pub type FnPtrToSize = unsafe extern "C" fn(*mut c_void) -> size_t;
pub type FnSizeToSize = unsafe extern "C" fn(size_t) -> size_t;
pub type FnIntToSize = unsafe extern "C" fn(c_int) -> size_t;

// ------------------------------------------------------------------- error rep

/// Canonical, library-independent description of a `size_t` return value.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Ret {
    Ok(size_t),
    Err { code: c_int, name: String },
}

pub struct ErrApi {
    is_error: Symbol<'static, FnIsError>,
    get_code: Symbol<'static, FnGetErrorCode>,
    get_name: Symbol<'static, FnGetErrorName>,
}

impl ErrApi {
    pub fn new(lib: &'static Library) -> Self {
        unsafe {
            ErrApi {
                is_error: sym::<FnIsError>(lib, "ZSTD_isError"),
                get_code: sym::<FnGetErrorCode>(lib, "ZSTD_getErrorCode"),
                get_name: sym::<FnGetErrorName>(lib, "ZSTD_getErrorName"),
            }
        }
    }
    pub fn is_err(&self, r: size_t) -> bool {
        unsafe { (self.is_error)(r) != 0 }
    }
    pub fn classify(&self, r: size_t) -> Ret {
        unsafe {
            if (self.is_error)(r) != 0 {
                let code = (self.get_code)(r);
                let name = cstr((self.get_name)(r));
                Ret::Err { code, name }
            } else {
                Ret::Ok(r)
            }
        }
    }
}

pub unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
}

/// The two error APIs, one per library.
pub struct Err2 {
    pub c: ErrApi,
    pub r: ErrApi,
}
impl Err2 {
    pub fn new() -> Self {
        Err2 { c: ErrApi::new(c()), r: ErrApi::new(rs()) }
    }
    /// Assert that a C return value and a Rust return value are equivalent:
    /// both OK with the same value, or both errors with the same error code.
    #[track_caller]
    pub fn eq(&self, ctx: &str, cr: size_t, rr: size_t) {
        let a = self.c.classify(cr);
        let b = self.r.classify(rr);
        assert_eq!(a, b, "{ctx}: C={a:?} RS={b:?} (raw C={cr:#x} RS={rr:#x})");
    }

    /// Same as [`Err2::eq`] but tolerates a `memory_allocation` result on
    /// either side.
    ///
    /// Both `.so`s are loaded into the SAME process, so a configuration that
    /// asks for a multi-gigabyte workspace (e.g. `ldmHashLog = 29` ⇒ a 4 GiB
    /// LDM table) can succeed in whichever library runs first and then OOM in
    /// the other. That outcome is a property of the host's free memory, not of
    /// the translation, so it carries no information. Every other error code is
    /// still compared strictly.
    ///
    /// Returns `true` if the values were compared, `false` if the comparison
    /// was skipped because of an allocation failure.
    #[track_caller]
    pub fn eq_or_oom(&self, ctx: &str, cr: size_t, rr: size_t) -> bool {
        let a = self.c.classify(cr);
        let b = self.r.classify(rr);
        let oom = |r: &Ret| matches!(r, Ret::Err { code, .. } if *code == E_memory_allocation);
        if oom(&a) || oom(&b) {
            return false;
        }
        assert_eq!(a, b, "{ctx}: C={a:?} RS={b:?} (raw C={cr:#x} RS={rr:#x})");
        true
    }
}

// ------------------------------------------------------------------------ rng

/// Deterministic xorshift64* PRNG — fixed seed => reproducible property tests.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
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
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Data-shape generators covering the shapes the C code special-cases.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Empty,
    Zeros,
    Constant,
    Random,
    Text,
    LowEntropy,
    Repeating,
    Incompressible,
    TwoSymbols,
    Sequential,
    LongMatches,
}

pub const ALL_SHAPES: &[Shape] = &[
    Shape::Empty,
    Shape::Zeros,
    Shape::Constant,
    Shape::Random,
    Shape::Text,
    Shape::LowEntropy,
    Shape::Repeating,
    Shape::Incompressible,
    Shape::TwoSymbols,
    Shape::Sequential,
    Shape::LongMatches,
];

pub fn gen(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    match shape {
        Shape::Empty => Vec::new(),
        Shape::Zeros => vec![0u8; len],
        Shape::Constant => vec![rng.byte(); len],
        Shape::Random | Shape::Incompressible => (0..len).map(|_| rng.byte()).collect(),
        Shape::Text => {
            const W: &[&str] = &[
                "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ", "and ",
                "then ", "zstd ", "compresses ", "it ", "very ", "well ", "indeed ", "a ", "of ",
            ];
            let mut v = Vec::with_capacity(len + 16);
            while v.len() < len {
                v.extend_from_slice(W[rng.below(W.len())].as_bytes());
            }
            v.truncate(len);
            v
        }
        Shape::LowEntropy => (0..len).map(|_| rng.byte() & 0x07).collect(),
        Shape::Repeating => {
            let plen = 1 + rng.below(64);
            let pat: Vec<u8> = (0..plen).map(|_| rng.byte()).collect();
            (0..len).map(|i| pat[i % plen]).collect()
        }
        Shape::TwoSymbols => (0..len).map(|_| if rng.bool() { b'a' } else { b'b' }).collect(),
        Shape::Sequential => (0..len).map(|i| (i & 0xff) as u8).collect(),
        Shape::LongMatches => {
            // Long repeated blocks with occasional noise — exercises the match
            // finders, LDM, and repeat-offset code paths.
            let mut v = Vec::with_capacity(len + 256);
            let blk: Vec<u8> = (0..1024).map(|_| rng.byte()).collect();
            while v.len() < len {
                v.extend_from_slice(&blk);
                if rng.bool() {
                    for _ in 0..rng.below(32) {
                        v.push(rng.byte());
                    }
                }
            }
            v.truncate(len);
            v
        }
    }
}

/// Interesting length values: boundaries + a few random sizes.
pub const LENS: &[usize] = &[
    0, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 63, 64, 100, 127, 128, 129, 255, 256, 257, 511, 512,
    1000, 1023, 1024, 1025, 4095, 4096, 4097, 8192, 16384, 20000, 65535, 65536, 65537, 100_000,
    131_072, 200_000,
];

// -------------------------------------------------------------- misc constants

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
// experimental (documented aliases)
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

// ZSTD_dParameter
pub const ZSTD_d_windowLogMax: c_int = 100;
pub const ZSTD_d_format: c_int = 1000; // experimentalParam1
pub const ZSTD_d_stableOutBuffer: c_int = 1001; // experimentalParam2
pub const ZSTD_d_forceIgnoreChecksum: c_int = 1002; // experimentalParam3
pub const ZSTD_d_refMultipleDDicts: c_int = 1003; // experimentalParam4
pub const ZSTD_d_disableHuffmanAssembly: c_int = 1004; // experimentalParam5
pub const ZSTD_d_maxBlockSize: c_int = 1005; // experimentalParam6

// ZSTD_EndDirective
pub const ZSTD_e_continue: c_int = 0;
pub const ZSTD_e_flush: c_int = 1;
pub const ZSTD_e_end: c_int = 2;

// ZSTD_ResetDirective
pub const ZSTD_reset_session_only: c_int = 1;
pub const ZSTD_reset_parameters: c_int = 2;
pub const ZSTD_reset_session_and_parameters: c_int = 3;

// error codes
pub const E_no_error: c_int = 0;
pub const E_GENERIC: c_int = 1;
pub const E_prefix_unknown: c_int = 10;
pub const E_version_unsupported: c_int = 12;
pub const E_frameParameter_unsupported: c_int = 14;
pub const E_frameParameter_windowTooLarge: c_int = 16;
pub const E_corruption_detected: c_int = 20;
pub const E_checksum_wrong: c_int = 22;
pub const E_literals_headerWrong: c_int = 24;
pub const E_dictionary_corrupted: c_int = 30;
pub const E_dictionary_wrong: c_int = 32;
pub const E_dictionaryCreation_failed: c_int = 34;
pub const E_parameter_unsupported: c_int = 40;
pub const E_parameter_combination_unsupported: c_int = 41;
pub const E_parameter_outOfBound: c_int = 42;
pub const E_tableLog_tooLarge: c_int = 44;
pub const E_maxSymbolValue_tooLarge: c_int = 46;
pub const E_maxSymbolValue_tooSmall: c_int = 48;
pub const E_cannotProduce_uncompressedBlock: c_int = 49;
pub const E_stabilityCondition_notRespected: c_int = 50;
pub const E_stage_wrong: c_int = 60;
pub const E_init_missing: c_int = 62;
pub const E_memory_allocation: c_int = 64;
pub const E_workSpace_tooSmall: c_int = 66;
pub const E_dstSize_tooSmall: c_int = 70;
pub const E_srcSize_wrong: c_int = 72;
pub const E_dstBuffer_null: c_int = 74;
pub const E_noForwardProgress_destFull: c_int = 80;
pub const E_noForwardProgress_inputEmpty: c_int = 82;
pub const E_dstBuffer_wrong: c_int = 104;
pub const E_srcBuffer_wrong: c_int = 105;
pub const E_sequenceProducer_failed: c_int = 106;
pub const E_externalSequences_invalid: c_int = 107;

// ---------------------------------------------------------------- FFI structs

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZSTD_bounds {
    pub error: size_t,
    pub lowerBound: c_int,
    pub upperBound: c_int,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ZSTD_inBuffer {
    pub src: *const c_void,
    pub size: size_t,
    pub pos: size_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ZSTD_outBuffer {
    pub dst: *mut c_void,
    pub size: size_t,
    pub pos: size_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ZSTD_Sequence {
    pub offset: c_uint,
    pub litLength: c_uint,
    pub matchLength: c_uint,
    pub rep: c_uint,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ZSTD_frameParameters {
    pub contentSizeFlag: c_int,
    pub checksumFlag: c_int,
    pub noDictIDFlag: c_int,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ZSTD_parameters {
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,
}

// ------------------------------------------------------------------ utilities

pub fn hexdump(b: &[u8], max: usize) -> String {
    let n = b.len().min(max);
    let mut s = String::new();
    for x in &b[..n] {
        s.push_str(&format!("{x:02x}"));
    }
    if b.len() > n {
        s.push_str("...");
    }
    s
}

/// Compare two byte buffers, reporting the first differing offset.
#[track_caller]
pub fn assert_bytes_eq(ctx: &str, a: &[u8], b: &[u8]) {
    if a == b {
        return;
    }
    if a.len() != b.len() {
        panic!("{ctx}: length differs C={} RS={}\n  C={}\n RS={}", a.len(), b.len(),
               hexdump(a, 64), hexdump(b, 64));
    }
    let i = a.iter().zip(b).position(|(x, y)| x != y).unwrap();
    panic!(
        "{ctx}: first diff at byte {i} (len {}): C=0x{:02x} RS=0x{:02x}\n  C={}\n RS={}",
        a.len(), a[i], b[i],
        hexdump(&a[i.saturating_sub(8)..(i + 24).min(a.len())], 64),
        hexdump(&b[i.saturating_sub(8)..(i + 24).min(b.len())], 64)
    );
}

// ------------------------------------------------- parameter id / value tables

/// Every `ZSTD_cParameter` id the C `switch` recognises.
pub const ALL_CPARAMS: &[(&str, c_int)] = &[
    ("compressionLevel", 100),
    ("windowLog", 101),
    ("hashLog", 102),
    ("chainLog", 103),
    ("searchLog", 104),
    ("minMatch", 105),
    ("targetLength", 106),
    ("strategy", 107),
    ("targetCBlockSize", 130),
    ("enableLongDistanceMatching", 160),
    ("ldmHashLog", 161),
    ("ldmMinMatch", 162),
    ("ldmBucketSizeLog", 163),
    ("ldmHashRateLog", 164),
    ("contentSizeFlag", 200),
    ("checksumFlag", 201),
    ("dictIDFlag", 202),
    ("nbWorkers", 400),
    ("jobSize", 401),
    ("overlapLog", 402),
    ("format(exp2)", 10),
    ("rsyncable(exp1)", 500),
    ("forceMaxWindow(exp3)", 1000),
    ("forceAttachDict(exp4)", 1001),
    ("literalCompressionMode(exp5)", 1002),
    ("srcSizeHint(exp7)", 1004),
    ("enableDedicatedDictSearch(exp8)", 1005),
    ("stableInBuffer(exp9)", 1006),
    ("stableOutBuffer(exp10)", 1007),
    ("blockDelimiters(exp11)", 1008),
    ("validateSequences(exp12)", 1009),
    ("splitAfterSequences(exp13)", 1010),
    ("useRowMatchFinder(exp14)", 1011),
    ("deterministicRefPrefix(exp15)", 1012),
    ("prefetchCDictTables(exp16)", 1013),
    ("enableSeqProducerFallback(exp17)", 1014),
    ("maxBlockSize(exp18)", 1015),
    ("repcodeResolution(exp19)", 1016),
    ("blockSplitterLevel(exp20)", 1017),
];

/// Ids that the C `switch` does NOT recognise (out-of-range enum values).
pub const BAD_CPARAMS: &[c_int] = &[
    0, 1, 2, 9, 11, 12, 99, 108, 109, 110, 129, 131, 159, 165, 199, 203, 204, 399, 403, 404, 499,
    501, 502, 999, 1003, 1018, 1019, 1020, 2000, -1, -100, i32::MIN, i32::MAX, i32::MIN + 1,
    i32::MAX - 1,
];

pub const ALL_DPARAMS: &[(&str, c_int)] = &[
    ("windowLogMax", 100),
    ("format(exp1)", 1000),
    ("stableOutBuffer(exp2)", 1001),
    ("forceIgnoreChecksum(exp3)", 1002),
    ("refMultipleDDicts(exp4)", 1003),
    ("disableHuffmanAssembly(exp5)", 1004),
    ("maxBlockSize(exp6)", 1005),
];

pub const BAD_DPARAMS: &[c_int] = &[
    0, 1, 99, 101, 102, 200, 400, 999, 1006, 1007, 1100, -1, -100, i32::MIN, i32::MAX,
];

/// Values worth trying for any parameter: bounds ± 1, sentinels, extremes.
pub fn param_probe_values(lo: c_int, hi: c_int, rng: &mut Rng) -> Vec<c_int> {
    let mut v = vec![
        0,
        1,
        -1,
        lo,
        hi,
        lo.saturating_sub(1),
        lo.saturating_add(1),
        hi.saturating_sub(1),
        hi.saturating_add(1),
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    if hi > lo {
        let mid = lo.wrapping_add((hi.wrapping_sub(lo)) / 2);
        v.push(mid);
        for _ in 0..8 {
            v.push(rng.range(lo as i64, hi as i64) as c_int);
        }
    }
    for _ in 0..4 {
        v.push(rng.next_u32() as c_int);
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// All `ZSTD_strategy` values plus out-of-range ones.
pub const STRATEGIES: &[c_int] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
pub const BAD_STRATEGIES: &[c_int] = &[0, 10, 11, -1, 100, i32::MIN, i32::MAX];
