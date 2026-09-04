//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and never
//! calls Rust functions directly, so the `#[no_mangle]` export wrappers are
//! part of what is under test.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

/* ------------------------------------------------------------------ */
/* library loading                                                     */
/* ------------------------------------------------------------------ */

pub struct Impls {
    pub c: Library,
    pub r: Library,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    root().parent().unwrap().join("c_src/build/liblz4.so")
}

fn rust_so() -> PathBuf {
    root().join("target/release/liblz4.so")
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        let cp = c_so();
        let rp = rust_so();
        assert!(
            cp.exists(),
            "missing C .so at {cp:?} — build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(
            rp.exists(),
            "missing Rust .so at {rp:?} — build it with:\n  cd translation && cargo build --release"
        );
        unsafe {
            Impls {
                c: Library::new(&cp).expect("dlopen C .so"),
                r: Library::new(&rp).expect("dlopen Rust .so"),
            }
        }
    })
}

/// Fetch a symbol of a given signature from a library.
pub unsafe fn sym<'a, T>(lib: &'a Library, name: &str) -> Symbol<'a, T> {
    let mut b = name.as_bytes().to_vec();
    b.push(0);
    unsafe {
        lib.get::<T>(&b)
            .unwrap_or_else(|e| panic!("symbol {name} not found: {e}"))
    }
}

/// Run `f` against both implementations and assert the results are identical.
///
/// The failure message is deliberately bounded: these values can be
/// multi-megabyte byte vectors.
pub fn diff<R, F>(label: &str, f: F)
where
    R: PartialEq + std::fmt::Debug,
    F: Fn(&Library) -> R,
{
    let i = impls();
    let rc = f(&i.c);
    let rr = f(&i.r);
    if rc != rr {
        let sc = format!("{rc:?}");
        let sr = format!("{rr:?}");
        // find the first differing character position for a focused excerpt
        let pos = sc
            .as_bytes()
            .iter()
            .zip(sr.as_bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(sc.len().min(sr.len()));
        let lo = pos.saturating_sub(120);
        let hi_c = (pos + 200).min(sc.len());
        let hi_r = (pos + 200).min(sr.len());
        panic!(
            "DIVERGENCE [{label}]\n  first difference at char {pos} \
             (C len {}, Rust len {})\n  C   ...{}...\n  Rust...{}...",
            sc.len(),
            sr.len(),
            &sc[lo..hi_c],
            &sr[lo..hi_r]
        );
    }
}

/* ------------------------------------------------------------------ */
/* deterministic RNG (xoshiro256**)                                    */
/* ------------------------------------------------------------------ */

pub struct Rng(pub [u64; 4]);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // splitmix64 expansion
        let mut s = seed;
        let mut nxt = || {
            s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        Rng([nxt(), nxt(), nxt(), nxt()])
    }
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.0;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// uniform in [0, n)
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo { lo } else { lo + self.below(hi - lo) }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}

/* ------------------------------------------------------------------ */
/* data generators — cover the entropy axis (A6 in CONFIGS.md)         */
/* ------------------------------------------------------------------ */

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Incompressible: uniform random bytes.
    Random,
    /// Maximally compressible: a single repeated byte (very long matches).
    Constant,
    /// Small alphabet: lots of medium matches.
    LowEntropy,
    /// Repeating pattern with period > 15 (exercises long match codes).
    Periodic,
    /// Long literal runs separated by long matches.
    LiteralsAndMatches,
    /// Text-like.
    Textish,
    /// Sparse: mostly zeros with occasional random bytes.
    Sparse,
}

pub const ALL_SHAPES: [Shape; 7] = [
    Shape::Random,
    Shape::Constant,
    Shape::LowEntropy,
    Shape::Periodic,
    Shape::LiteralsAndMatches,
    Shape::Textish,
    Shape::Sparse,
];

pub fn mkdata(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    // Always keep a real (zeroed) heap allocation behind the vector, even for
    // len == 0: several C entry points (notably the LZ4HC family) dereference
    // `src` without a NULL guard, so `as_ptr()` must stay readable.
    let mut v = Vec::with_capacity(len.max(16));
    v.resize(len.max(16), 0u8);
    v.clear();
    match shape {
        Shape::Random => {
            for _ in 0..len {
                v.push(rng.byte());
            }
        }
        Shape::Constant => {
            let b = rng.byte();
            v.resize(len, b);
        }
        Shape::LowEntropy => {
            let n = 1 + rng.below(4);
            let alpha: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            for _ in 0..len {
                v.push(alpha[rng.below(n)]);
            }
        }
        Shape::Periodic => {
            let p = rng.range(16, 80);
            let pat: Vec<u8> = (0..p).map(|_| rng.byte()).collect();
            for i in 0..len {
                v.push(pat[i % p]);
            }
        }
        Shape::LiteralsAndMatches => {
            while v.len() < len {
                let lit = rng.range(1, 40);
                for _ in 0..lit {
                    if v.len() >= len {
                        break;
                    }
                    v.push(rng.byte());
                }
                if v.is_empty() {
                    continue;
                }
                let back = 1 + rng.below(v.len());
                let mlen = rng.range(4, 300);
                let start = v.len() - back;
                for k in 0..mlen {
                    if v.len() >= len {
                        break;
                    }
                    let b = v[start + (k % back)];
                    v.push(b);
                }
            }
        }
        Shape::Textish => {
            const WORDS: [&str; 12] = [
                "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ", "lz4 ",
                "compress ", "decompress ", "frame ",
            ];
            while v.len() < len {
                v.extend_from_slice(WORDS[rng.below(WORDS.len())].as_bytes());
            }
            v.truncate(len);
        }
        Shape::Sparse => {
            v.resize(len, 0);
            let n = len / 64 + 1;
            for _ in 0..n {
                let i = rng.below(len.max(1));
                if i < len {
                    v[i] = rng.byte();
                }
            }
        }
    }
    v.truncate(len);
    v
}

/* ------------------------------------------------------------------ */
/* constants mirrored from the C headers                               */
/* ------------------------------------------------------------------ */

pub const LZ4_MAX_INPUT_SIZE: i32 = 0x7E00_0000;
pub const LZ4_ACCELERATION_MAX: i32 = 65537;
pub const LZ4HC_CLEVEL_MIN: i32 = 2;
pub const LZ4HC_CLEVEL_DEFAULT: i32 = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: i32 = 10;
pub const LZ4HC_CLEVEL_MAX: i32 = 12;
pub const LZ4F_HEADER_SIZE_MIN: usize = 7;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;
pub const LZ4F_BLOCK_HEADER_SIZE: usize = 4;
pub const LZ4F_BLOCK_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_MAGICNUMBER: u32 = 0x184D_2204;
pub const LZ4F_MAGIC_SKIPPABLE_START: u32 = 0x184D_2A50;

/// Generous over-allocation for `LZ4_stream_t` / `LZ4_streamHC_t` user buffers.
/// The real sizes are queried at runtime via `LZ4_sizeofState*`.
pub const STATE_SLOP: usize = 64;

pub fn compress_bound(isize_: i32) -> i32 {
    if (isize_ as u32) > (LZ4_MAX_INPUT_SIZE as u32) {
        0
    } else {
        isize_ + isize_ / 255 + 16
    }
}

/* ------------------------------------------------------------------ */
/* aligned scratch buffer                                              */
/* ------------------------------------------------------------------ */

/// 64-bit-aligned heap buffer, needed because `LZ4_initStream` /
/// `LZ4_compress_HC_extStateHC` reject misaligned state pointers.
pub struct Aligned {
    ptr: *mut u8,
    layout: std::alloc::Layout,
    pub len: usize,
}

impl Aligned {
    pub fn new(len: usize, align: usize) -> Aligned {
        let layout = std::alloc::Layout::from_size_align(len.max(1), align).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null());
        Aligned { ptr, layout, len }
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }
}

impl Drop for Aligned {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) }
    }
}

/* ------------------------------------------------------------------ */
/* LZ4F structures — layout must match the C headers exactly           */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct LZ4F_frameInfo_t {
    pub blockSizeID: i32,
    pub blockMode: i32,
    pub contentChecksumFlag: i32,
    pub frameType: i32,
    pub contentSize: u64,
    pub dictID: u32,
    pub blockChecksumFlag: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct LZ4F_preferences_t {
    pub frameInfo: LZ4F_frameInfo_t,
    pub compressionLevel: i32,
    pub autoFlush: u32,
    pub favorDecSpeed: u32,
    pub reserved: [u32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct LZ4F_compressOptions_t {
    pub stableSrc: u32,
    pub reserved: [u32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct LZ4F_decompressOptions_t {
    pub stableDst: u32,
    pub skipChecksums: u32,
    pub reserved1: u32,
    pub reserved0: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct LZ4F_CustomMem {
    pub customAlloc: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize) -> *mut std::ffi::c_void>,
    pub customCalloc: Option<unsafe extern "C" fn(*mut std::ffi::c_void, usize) -> *mut std::ffi::c_void>,
    pub customFree: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void)>,
    pub opaque: *mut std::ffi::c_void,
}

/* ------------------------------------------------------------------ */
/* LZ4F error decoding                                                 */
/* ------------------------------------------------------------------ */

/// Mirrors the `LZ4F_LIST_ERRORS` enum order in `lz4frame.h`.
pub const LZ4F_ERROR_NAMES: [&str; 26] = [
    "OK_NoError",
    "ERROR_GENERIC",
    "ERROR_maxBlockSize_invalid",
    "ERROR_blockMode_invalid",
    "ERROR_parameter_invalid",
    "ERROR_compressionLevel_invalid",
    "ERROR_headerVersion_wrong",
    "ERROR_blockChecksum_invalid",
    "ERROR_reservedFlag_set",
    "ERROR_allocation_failed",
    "ERROR_srcSize_tooLarge",
    "ERROR_dstMaxSize_tooSmall",
    "ERROR_frameHeader_incomplete",
    "ERROR_frameType_unknown",
    "ERROR_frameSize_wrong",
    "ERROR_srcPtr_wrong",
    "ERROR_decompressionFailed",
    "ERROR_headerChecksum_invalid",
    "ERROR_contentChecksum_invalid",
    "ERROR_frameDecoding_alreadyStarted",
    "ERROR_compressionState_uninitialized",
    "ERROR_parameter_null",
    "ERROR_io_write",
    "ERROR_io_read",
    "ERROR_maxCode",
    "_dummy",
];

pub fn err_code(name: &str) -> usize {
    let idx = LZ4F_ERROR_NAMES
        .iter()
        .position(|n| *n == name)
        .unwrap_or_else(|| panic!("unknown error name {name}"));
    (0usize).wrapping_sub(idx)
}

/* ------------------------------------------------------------------ */
/* typed symbol aliases used across test files                         */
/* ------------------------------------------------------------------ */

pub type FnI32I32 = unsafe extern "C" fn(i32) -> i32;
pub type FnVoidI32 = unsafe extern "C" fn() -> i32;
pub type CChar = std::ffi::c_char;
pub type CVoid = std::ffi::c_void;

pub mod frame;
