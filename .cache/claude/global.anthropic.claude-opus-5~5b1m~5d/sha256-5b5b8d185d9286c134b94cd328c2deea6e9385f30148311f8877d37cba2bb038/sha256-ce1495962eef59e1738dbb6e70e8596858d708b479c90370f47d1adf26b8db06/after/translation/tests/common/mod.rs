//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries via `libloading` and exposes them side by side so
//! every test calls the C and the Rust implementation through the *same* FFI
//! surface an external consumer would use. The Rust crate is never linked
//! directly — only its `cdylib` exports are exercised.
//!
//! Build prerequisites (see `run_tests.sh`):
//!   * `c_src/build/libzstd.so`            (cmake)
//!   * `translation/target/release/libzstd.so` (cargo build --release)

#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------- library pair

pub struct Impls {
    pub c: Library,
    pub rs: Library,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn load() -> Impls {
    let root = workspace_root();
    let c_path = root.join("c_src/build/libzstd.so");
    let rs_path = root.join("translation/target/release/libzstd.so");

    for p in [&c_path, &rs_path] {
        assert!(
            p.exists(),
            "missing shared library {}\nrun ./run_tests.sh (it builds both) first",
            p.display()
        );
    }

    unsafe {
        Impls {
            c: Library::new(&c_path).unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display())),
            rs: Library::new(&rs_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display())),
        }
    }
}

/// Process-wide singleton so the two libraries are `dlopen`ed exactly once.
pub fn impls() -> &'static Impls {
    static I: OnceLock<Impls> = OnceLock::new();
    I.get_or_init(load)
}

// ------------------------------------------------- runtime coverage recording
//
// Many tests build symbol names dynamically (`format!("HUFv07_{s}")`), so a
// static grep of the test sources undercounts what is actually exercised.
// Every symbol resolved through `pair()`/`has()` is therefore recorded here and
// appended (once, on first sight) to `$ZSTD_DIFF_COVERAGE` when that env var is
// set. `tools/coverage.sh` runs the suite with it set and diffs the result
// against `nm -D`, giving a TRUE list of exercised exports.

fn record(name: &str) {
    use std::collections::HashSet;
    use std::io::Write;
    use std::sync::Mutex;
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let path = match std::env::var("ZSTD_DIFF_COVERAGE") {
        Ok(p) if !p.is_empty() => p,
        _ => return,
    };
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let mut g = seen.lock().unwrap();
    if g.insert(name.to_string()) {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{name}");
        }
    }
}

impl Impls {
    /// Fetch the same symbol from both libraries.
    ///
    /// Panics with a clear message if either library is missing it, which makes
    /// symbol-parity gaps show up as test failures rather than silent skips.
    pub fn pair<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        record(name);
        let cname = std::ffi::CString::new(name).unwrap();
        unsafe {
            let c: Symbol<T> = self
                .c
                .get(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("C .so is missing symbol `{name}`: {e}"));
            let r: Symbol<T> = self
                .rs
                .get(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("Rust .so is missing symbol `{name}`: {e}"));
            (c, r)
        }
    }

    pub fn has(&self, name: &str) -> bool {
        record(name);
        let cname = std::ffi::CString::new(name).unwrap();
        unsafe {
            self.c
                .get::<*const ()>(cname.as_bytes_with_nul())
                .is_ok()
                && self
                    .rs
                    .get::<*const ()>(cname.as_bytes_with_nul())
                    .is_ok()
        }
    }
}

/// Convenience: bind one `extern "C"` fn signature from both libs.
///
/// ```ignore
/// let (c, r) = sym!(i.pair::<unsafe extern "C" fn() -> u32>("ZSTD_versionNumber"));
/// ```
#[macro_export]
macro_rules! sym {
    ($e:expr) => {
        $e
    };
}

// ------------------------------------------------------------------- rng (PRNG)

/// Deterministic xoshiro-style PRNG so every randomized row is reproducible
/// from its seed. Deliberately not `rand` — keeps dev-dependencies to
/// `libloading` only.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // splitmix64 warm-up so low seeds still give well-mixed state
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut z = self.0;
        z ^= z >> 12;
        z ^= z << 25;
        z ^= z >> 27;
        self.0 = z;
        z.wrapping_mul(0x2545_F491_4F6C_DD1D)
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

    pub fn range(&mut self, lo: usize, hi_incl: usize) -> usize {
        if hi_incl <= lo {
            lo
        } else {
            lo + self.below(hi_incl - lo + 1)
        }
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & (1 << 33) != 0
    }
}

// --------------------------------------------------------------- input shapes

/// The distinct input *shapes* `CONFIGS.md` enumerates. Each one drives the
/// compressor down a different family of code paths (RLE blocks, raw blocks,
/// huffman-coded literals, repeat-offset matches, ...).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Every byte identical -> RLE blocks.
    Constant,
    /// Cryptographically-unfriendly random -> incompressible, raw blocks.
    Random,
    /// Small alphabet, skewed -> huffman-compressible literals.
    SkewedText,
    /// Long repeated pattern -> long matches, repeat offsets.
    Repetitive,
    /// Structured records -> regular offsets.
    Tabular,
    /// Mostly zeros with sparse noise.
    Sparse,
    /// Two alternating halves, tests window/offset behaviour.
    TwoPhase,
    /// Incrementing counter bytes.
    Counter,
}

pub const ALL_SHAPES: [Shape; 8] = [
    Shape::Constant,
    Shape::Random,
    Shape::SkewedText,
    Shape::Repetitive,
    Shape::Tabular,
    Shape::Sparse,
    Shape::TwoPhase,
    Shape::Counter,
];

pub fn gen_shape(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    match shape {
        Shape::Constant => {
            let b = rng.byte();
            v.resize(len, b);
        }
        Shape::Random => {
            while v.len() < len {
                v.extend_from_slice(&rng.next_u64().to_le_bytes());
            }
            v.truncate(len);
        }
        Shape::SkewedText => {
            const AL: &[u8] = b"aaaaabbbbcccdde fgh\n";
            for _ in 0..len {
                let i = rng.below(AL.len());
                v.push(AL[i]);
            }
        }
        Shape::Repetitive => {
            let plen = rng.range(1, 64);
            let mut pat = Vec::with_capacity(plen);
            for _ in 0..plen {
                pat.push(rng.byte());
            }
            while v.len() < len {
                let take = core::cmp::min(pat.len(), len - v.len());
                v.extend_from_slice(&pat[..take]);
            }
        }
        Shape::Tabular => {
            let mut n: u32 = rng.next_u32();
            while v.len() < len {
                let rec = format!("{:08x},{:05},row\n", n, n % 100_000);
                let take = core::cmp::min(rec.len(), len - v.len());
                v.extend_from_slice(&rec.as_bytes()[..take]);
                n = n.wrapping_add(1);
            }
        }
        Shape::Sparse => {
            v.resize(len, 0);
            let hits = len / 64 + 1;
            for _ in 0..hits {
                if len > 0 {
                    let i = rng.below(len);
                    v[i] = rng.byte();
                }
            }
        }
        Shape::TwoPhase => {
            let half = len / 2;
            for _ in 0..half {
                v.push(b'A' + (rng.below(4) as u8));
            }
            while v.len() < len {
                v.extend_from_slice(&rng.next_u32().to_le_bytes());
            }
            v.truncate(len);
        }
        Shape::Counter => {
            for i in 0..len {
                v.push((i & 0xff) as u8);
            }
        }
    }
    debug_assert_eq!(v.len(), len);
    v
}

/// Interesting length boundaries: empty, one, sub-word, block edges, multi-block.
pub const EDGE_LENS: [usize; 18] = [
    0,
    1,
    2,
    3,
    7,
    8,
    9,
    15,
    16,
    31,
    64,
    127,
    128,
    1023,
    1024,
    65535,
    131_072,      // ZSTD_BLOCKSIZE_MAX
    131_073,      // one past a full block
];

// ------------------------------------------------------------------ diff utils

/// Byte-for-byte comparison with a readable first-divergence report.
pub fn assert_bytes_eq(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    if c.len() != r.len() {
        panic!(
            "{ctx}: length mismatch C={} Rust={}\n C[..32]={:02x?}\n R[..32]={:02x?}",
            c.len(),
            r.len(),
            &c[..c.len().min(32)],
            &r[..r.len().min(32)]
        );
    }
    let at = c.iter().zip(r).position(|(a, b)| a != b).unwrap();
    let lo = at.saturating_sub(8);
    let hi = (at + 8).min(c.len());
    panic!(
        "{ctx}: first byte divergence at {at} (len {})\n C[{lo}..{hi}]={:02x?}\n R[{lo}..{hi}]={:02x?}",
        c.len(),
        &c[lo..hi],
        &r[lo..hi]
    );
}

pub fn assert_eq_dbg<T: PartialEq + std::fmt::Debug>(ctx: &str, c: T, r: T) {
    assert!(c == r, "{ctx}: C={c:?} Rust={r:?}");
}

// --------------------------------------------------------- zstd ABI constants

pub const ZSTD_BLOCKSIZE_MAX: usize = 128 * 1024;
pub const ZSTD_CONTENTSIZE_UNKNOWN: u64 = u64::MAX - 0;
pub const ZSTD_CONTENTSIZE_ERROR: u64 = u64::MAX - 1;

// ZSTD_ErrorCode (zstd_errors.h)
pub const ZSTD_error_no_error: i32 = 0;
pub const ZSTD_error_GENERIC: i32 = 1;
pub const ZSTD_error_prefix_unknown: i32 = 10;
pub const ZSTD_error_version_unsupported: i32 = 12;
pub const ZSTD_error_frameParameter_unsupported: i32 = 14;
pub const ZSTD_error_frameParameter_windowTooLarge: i32 = 16;
pub const ZSTD_error_corruption_detected: i32 = 20;
pub const ZSTD_error_checksum_wrong: i32 = 22;
pub const ZSTD_error_literals_headerWrong: i32 = 24;
pub const ZSTD_error_dictionary_corrupted: i32 = 30;
pub const ZSTD_error_dictionary_wrong: i32 = 32;
pub const ZSTD_error_dictionaryCreation_failed: i32 = 34;
pub const ZSTD_error_parameter_unsupported: i32 = 40;
pub const ZSTD_error_parameter_combination_unsupported: i32 = 41;
pub const ZSTD_error_parameter_outOfBound: i32 = 42;
pub const ZSTD_error_tableLog_tooLarge: i32 = 44;
pub const ZSTD_error_maxSymbolValue_tooLarge: i32 = 46;
pub const ZSTD_error_maxSymbolValue_tooSmall: i32 = 48;
pub const ZSTD_error_cannotProduce_uncompressedBlock: i32 = 49;
pub const ZSTD_error_stabilityCondition_notRespected: i32 = 50;
pub const ZSTD_error_stage_wrong: i32 = 60;
pub const ZSTD_error_init_missing: i32 = 62;
pub const ZSTD_error_memory_allocation: i32 = 64;
pub const ZSTD_error_workSpace_tooSmall: i32 = 66;
pub const ZSTD_error_dstSize_tooSmall: i32 = 70;
pub const ZSTD_error_srcSize_wrong: i32 = 72;
pub const ZSTD_error_dstBuffer_null: i32 = 74;
pub const ZSTD_error_noForwardProgress: i32 = 80;
pub const ZSTD_error_frameIndex_tooLarge: i32 = 100;
pub const ZSTD_error_seekableIO: i32 = 102;
pub const ZSTD_error_dstBuffer_wrong: i32 = 104;
pub const ZSTD_error_srcBuffer_wrong: i32 = 105;
pub const ZSTD_error_sequenceProducer_failed: i32 = 106;
pub const ZSTD_error_externalSequences_invalid: i32 = 107;

// ZSTD_cParameter (zstd.h) — values are ABI, verified against the header.
pub const ZSTD_c_compressionLevel: i32 = 100;
pub const ZSTD_c_windowLog: i32 = 101;
pub const ZSTD_c_hashLog: i32 = 102;
pub const ZSTD_c_chainLog: i32 = 103;
pub const ZSTD_c_searchLog: i32 = 104;
pub const ZSTD_c_minMatch: i32 = 105;
pub const ZSTD_c_targetLength: i32 = 106;
pub const ZSTD_c_strategy: i32 = 107;
pub const ZSTD_c_targetCBlockSize: i32 = 130;
pub const ZSTD_c_enableLongDistanceMatching: i32 = 160;
pub const ZSTD_c_ldmHashLog: i32 = 161;
pub const ZSTD_c_ldmMinMatch: i32 = 162;
pub const ZSTD_c_ldmBucketSizeLog: i32 = 163;
pub const ZSTD_c_ldmHashRateLog: i32 = 164;
pub const ZSTD_c_contentSizeFlag: i32 = 200;
pub const ZSTD_c_checksumFlag: i32 = 201;
pub const ZSTD_c_dictIDFlag: i32 = 202;
pub const ZSTD_c_nbWorkers: i32 = 400;
pub const ZSTD_c_jobSize: i32 = 401;
pub const ZSTD_c_overlapLog: i32 = 402;

// experimental cParams — values taken verbatim from the ZSTD_cParameter enum
// in c_src/src/include/zstd.h (ZSTD_c_experimentalParamN aliases).
pub const ZSTD_c_rsyncable: i32 = 500; // experimentalParam1
pub const ZSTD_c_format: i32 = 10; // experimentalParam2
pub const ZSTD_c_forceMaxWindow: i32 = 1000; // experimentalParam3
pub const ZSTD_c_forceAttachDict: i32 = 1001; // experimentalParam4
pub const ZSTD_c_literalCompressionMode: i32 = 1002; // experimentalParam5
pub const ZSTD_c_experimentalParam6: i32 = 1003; // (retired)
pub const ZSTD_c_srcSizeHint: i32 = 1004; // experimentalParam7
pub const ZSTD_c_enableDedicatedDictSearch: i32 = 1005; // experimentalParam8
pub const ZSTD_c_stableInBuffer: i32 = 1006; // experimentalParam9
pub const ZSTD_c_stableOutBuffer: i32 = 1007; // experimentalParam10
pub const ZSTD_c_blockDelimiters: i32 = 1008; // experimentalParam11
pub const ZSTD_c_validateSequences: i32 = 1009; // experimentalParam12
pub const ZSTD_c_splitAfterSequences: i32 = 1010; // experimentalParam13
pub const ZSTD_c_useRowMatchFinder: i32 = 1011; // experimentalParam14
pub const ZSTD_c_deterministicRefPrefix: i32 = 1012; // experimentalParam15
pub const ZSTD_c_prefetchCDictTables: i32 = 1013; // experimentalParam16
pub const ZSTD_c_enableSeqProducerFallback: i32 = 1014; // experimentalParam17
pub const ZSTD_c_maxBlockSize: i32 = 1015; // experimentalParam18
pub const ZSTD_c_repcodeResolution: i32 = 1016; // experimentalParam19
pub const ZSTD_c_searchForExternalRepcodes: i32 = 1016; // alias of the above
pub const ZSTD_c_blockSplitterLevel: i32 = 1017; // experimentalParam20

// ZSTD_dParameter
pub const ZSTD_d_windowLogMax: i32 = 100;
pub const ZSTD_d_format: i32 = 1000; // experimentalParam1
pub const ZSTD_d_stableOutBuffer: i32 = 1001; // experimentalParam2
pub const ZSTD_d_forceIgnoreChecksum: i32 = 1002; // experimentalParam3
pub const ZSTD_d_refMultipleDDicts: i32 = 1003; // experimentalParam4
pub const ZSTD_d_disableHuffmanAssembly: i32 = 1004; // experimentalParam5
pub const ZSTD_d_maxBlockSize: i32 = 1005; // experimentalParam6

// ZSTD_strategy
pub const ZSTD_fast: i32 = 1;
pub const ZSTD_dfast: i32 = 2;
pub const ZSTD_greedy: i32 = 3;
pub const ZSTD_lazy: i32 = 4;
pub const ZSTD_lazy2: i32 = 5;
pub const ZSTD_btlazy2: i32 = 6;
pub const ZSTD_btopt: i32 = 7;
pub const ZSTD_btultra: i32 = 8;
pub const ZSTD_btultra2: i32 = 9;
pub const ALL_STRATEGIES: [i32; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];

// ZSTD_format_e / ZSTD_ResetDirective / EndDirective
pub const ZSTD_f_zstd1: i32 = 0;
pub const ZSTD_f_zstd1_magicless: i32 = 1;
pub const ZSTD_reset_session_only: i32 = 1;
pub const ZSTD_reset_parameters: i32 = 2;
pub const ZSTD_reset_session_and_parameters: i32 = 3;
pub const ZSTD_e_continue: i32 = 0;
pub const ZSTD_e_flush: i32 = 1;
pub const ZSTD_e_end: i32 = 2;

// ZSTD_literalCompressionMode_e / dictAttachPref / paramSwitch / SequenceFormat
pub const ZSTD_lcm_auto: i32 = 0;
pub const ZSTD_lcm_huffman: i32 = 1;
pub const ZSTD_lcm_uncompressed: i32 = 2;
pub const ZSTD_dictDefaultAttach: i32 = 0;
pub const ZSTD_dictForceAttach: i32 = 1;
pub const ZSTD_dictForceCopy: i32 = 2;
pub const ZSTD_dictForceLoad: i32 = 3;
pub const ZSTD_ps_auto: i32 = 0;
pub const ZSTD_ps_enable: i32 = 1;
pub const ZSTD_ps_disable: i32 = 2;
pub const ZSTD_sf_noBlockDelimiters: i32 = 0;
pub const ZSTD_sf_explicitBlockDelimiters: i32 = 1;

// dictionary load method / content type
pub const ZSTD_dlm_byCopy: i32 = 0;
pub const ZSTD_dlm_byRef: i32 = 1;
pub const ZSTD_dct_auto: i32 = 0;
pub const ZSTD_dct_rawContent: i32 = 1;
pub const ZSTD_dct_fullDict: i32 = 2;

pub const ZSTD_MAGICNUMBER: u32 = 0xFD2F_B528;
pub const ZSTD_MAGIC_DICTIONARY: u32 = 0xEC30_A437;
pub const ZSTD_MAGIC_SKIPPABLE_START: u32 = 0x184D_2A50;

// ---------------------------------------------------------------- ZSTD structs

/// `ZSTD_bounds` — { ZSTD_ErrorCode error; int lowerBound; int upperBound; }
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_bounds {
    pub error: usize,
    pub lower_bound: i32,
    pub upper_bound: i32,
}

/// `ZSTD_inBuffer` — { const void* src; size_t size; size_t pos; }
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ZSTD_inBuffer {
    pub src: *const u8,
    pub size: usize,
    pub pos: usize,
}

/// `ZSTD_outBuffer` — { void* dst; size_t size; size_t pos; }
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ZSTD_outBuffer {
    pub dst: *mut u8,
    pub size: usize,
    pub pos: usize,
}

/// `ZSTD_Sequence` — { U32 offset; U32 litLength; U32 matchLength; U32 rep; }
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_Sequence {
    pub offset: u32,
    pub lit_length: u32,
    pub match_length: u32,
    pub rep: u32,
}

/// `ZSTD_frameHeader` (zstd.h, experimental section).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTD_frameHeader {
    pub frame_content_size: u64,
    pub window_size: u64,
    pub block_size_max: u32,
    pub frame_type: u32,   // ZSTD_frameType_e
    pub header_size: u32,
    pub dict_id: u32,
    pub checksum_flag: u32,
    pub _reserved1: u32,
    pub _reserved2: u32,
}
