//! Shared differential-test harness.
//!
//! Loads BOTH the C shared library (`c_src/build/liblz4.so`) and the Rust
//! shared library (`target/<profile>/liblz4.so`) with `libloading` and calls
//! every function through the FFI boundary, exactly as an external consumer
//! would.  No Rust function is ever called directly.
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("LZ4_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src")
        .join("build")
        .join("liblz4.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("LZ4_RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe is target/<profile>/deps/<testbin>; the cdylib lives in
    // target/<profile>/liblz4.so
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                let cand = profile.join("liblz4.so");
                if cand.exists() {
                    return cand;
                }
            }
        }
    }
    manifest_dir()
        .join("target")
        .join("release")
        .join("liblz4.so")
}

pub struct Lib {
    lib: Library,
    pub which: &'static str,
}

impl Lib {
    /// Fetch an exported symbol.  Panics with a clear message when the symbol
    /// is missing, which is itself a Phase-A/Phase-D failure.
    pub fn get<T>(&self, name: &str) -> Symbol<'_, T> {
        let mut owned = Vec::with_capacity(name.len() + 1);
        owned.extend_from_slice(name.as_bytes());
        owned.push(0);
        unsafe {
            self.lib.get::<T>(&owned).unwrap_or_else(|e| {
                panic!("symbol `{}` missing from {} library: {}", name, self.which, e)
            })
        }
    }

    pub fn has(&self, name: &str) -> bool {
        let mut owned = Vec::with_capacity(name.len() + 1);
        owned.extend_from_slice(name.as_bytes());
        owned.push(0);
        unsafe { self.lib.get::<*const c_void>(&owned).is_ok() }
    }
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static R_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c() -> &'static Lib {
    C_LIB.get_or_init(|| {
        let p = c_so_path();
        let lib = unsafe { Library::new(&p) }
            .unwrap_or_else(|e| panic!("cannot load C library {:?}: {}", p, e));
        Lib { lib, which: "C" }
    })
}

pub fn r() -> &'static Lib {
    R_LIB.get_or_init(|| {
        let p = rust_so_path();
        let lib = unsafe { Library::new(&p) }
            .unwrap_or_else(|e| panic!("cannot load Rust library {:?}: {}", p, e));
        Lib { lib, which: "Rust" }
    })
}

/// Run the same closure against both libraries and return `(c_result, rust_result)`.
pub fn both<T, F: Fn(&'static Lib) -> T>(f: F) -> (T, T) {
    (f(c()), f(r()))
}

// ---------------------------------------------------------------------------
// deterministic RNG (xoshiro256**) and data generators
// ---------------------------------------------------------------------------

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // splitmix64 seeding
        let mut x = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut s = [0u64; 4];
        for slot in s.iter_mut() {
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *slot = z ^ (z >> 31);
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

    /// uniform in `[0, n)`; returns 0 when `n == 0`
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }

    /// uniform in `[lo, hi]`
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            lo
        } else {
            lo + self.below(hi - lo + 1)
        }
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Data shapes the LZ4 code paths distinguish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// pure random bytes — incompressible, forces the "no match" path
    Incompressible,
    /// all one byte value — maximal match lengths, exercises the long-match
    /// / 255-continuation encoding
    Constant,
    /// small alphabet, produces many short matches and long offsets
    Text,
    /// repeated block of `period` bytes — controlled match offsets
    Periodic(usize),
    /// long runs of identical bytes separated by random noise
    Runs,
    /// mostly zero with sparse random bytes (very compressible)
    Sparse,
}

pub const ALL_SHAPES: &[Shape] = &[
    Shape::Incompressible,
    Shape::Constant,
    Shape::Text,
    Shape::Periodic(3),
    Shape::Periodic(64),
    Shape::Periodic(65535),
    Shape::Runs,
    Shape::Sparse,
];

pub fn gen(rng: &mut Rng, len: usize, shape: Shape) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    match shape {
        Shape::Incompressible => {
            while v.len() < len {
                v.extend_from_slice(&rng.next_u64().to_le_bytes());
            }
            v.truncate(len);
        }
        Shape::Constant => {
            let b = rng.byte();
            v.resize(len, b);
        }
        Shape::Text => {
            const ALPHA: &[u8] = b"abcdefgh \n";
            for _ in 0..len {
                v.push(ALPHA[rng.below(ALPHA.len())]);
            }
        }
        Shape::Periodic(p) => {
            let p = p.max(1);
            let mut pat = Vec::with_capacity(p);
            for _ in 0..p {
                pat.push(rng.byte());
            }
            while v.len() < len {
                let n = (len - v.len()).min(p);
                v.extend_from_slice(&pat[..n]);
            }
        }
        Shape::Runs => {
            while v.len() < len {
                if rng.bool() {
                    let n = rng.range(1, 300).min(len - v.len());
                    let b = rng.byte();
                    for _ in 0..n {
                        v.push(b);
                    }
                } else {
                    let n = rng.range(1, 40).min(len - v.len());
                    for _ in 0..n {
                        v.push(rng.byte());
                    }
                }
            }
            v.truncate(len);
        }
        Shape::Sparse => {
            v.resize(len, 0);
            let hits = len / 64 + 1;
            for _ in 0..hits {
                let i = rng.below(len.max(1));
                if i < len {
                    v[i] = rng.byte();
                }
            }
        }
    }
    v
}

// ---------------------------------------------------------------------------
// aligned scratch buffers (for the *_extState / initStream entry points)
// ---------------------------------------------------------------------------

/// Heap buffer with a guaranteed alignment, used for `LZ4_stream_t` /
/// `LZ4_streamHC_t` scratch space handed to the `extState` entry points.
pub struct Aligned {
    ptr: *mut u8,
    layout: std::alloc::Layout,
}

impl Aligned {
    pub fn new(size: usize, align: usize) -> Aligned {
        let layout = std::alloc::Layout::from_size_align(size.max(1), align).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null());
        Aligned { ptr, layout }
    }
    pub fn ptr(&self) -> *mut c_void {
        self.ptr as *mut c_void
    }
    /// A pointer deliberately offset by one byte, i.e. guaranteed misaligned.
    pub fn misaligned(&self) -> *mut c_void {
        unsafe { self.ptr.add(1) as *mut c_void }
    }
    pub fn size(&self) -> usize {
        self.layout.size()
    }
    pub fn zero(&mut self) {
        unsafe { std::ptr::write_bytes(self.ptr, 0, self.layout.size()) }
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.layout.size()) }
    }
}

impl Drop for Aligned {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) }
    }
}

// ---------------------------------------------------------------------------
// lz4frame types (layouts copied verbatim from c_src/include/lz4frame.h)
// ---------------------------------------------------------------------------

pub const LZ4F_VERSION: c_uint = 100;
pub const LZ4F_HEADER_SIZE_MIN: usize = 7;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;
pub const LZ4F_BLOCK_HEADER_SIZE: usize = 4;
pub const LZ4F_BLOCK_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_CONTENT_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_MAGICNUMBER: u32 = 0x184D_2204;
pub const LZ4F_MAGIC_SKIPPABLE_START: u32 = 0x184D_2A50;

pub const LZ4_MAX_INPUT_SIZE: c_int = 0x7E00_0000;
pub const LZ4_ACCELERATION_MAX: c_int = 65537;
pub const LZ4HC_CLEVEL_MIN: c_int = 2;
pub const LZ4HC_CLEVEL_DEFAULT: c_int = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: c_int = 10;
pub const LZ4HC_CLEVEL_MAX: c_int = 12;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LZ4F_frameInfo_t {
    pub blockSizeID: c_int,
    pub blockMode: c_int,
    pub contentChecksumFlag: c_int,
    pub frameType: c_int,
    pub contentSize: c_ulonglong,
    pub dictID: c_uint,
    pub blockChecksumFlag: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LZ4F_preferences_t {
    pub frameInfo: LZ4F_frameInfo_t,
    pub compressionLevel: c_int,
    pub autoFlush: c_uint,
    pub favorDecSpeed: c_uint,
    pub reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LZ4F_compressOptions_t {
    pub stableSrc: c_uint,
    pub reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LZ4F_decompressOptions_t {
    pub stableDst: c_uint,
    pub skipChecksums: c_uint,
    pub reserved1: c_uint,
    pub reserved0: c_uint,
}

// LZ4F_blockSizeID_t
pub const LZ4F_DEFAULT: c_int = 0;
pub const LZ4F_MAX64KB: c_int = 4;
pub const LZ4F_MAX256KB: c_int = 5;
pub const LZ4F_MAX1MB: c_int = 6;
pub const LZ4F_MAX4MB: c_int = 7;
// LZ4F_blockMode_t
pub const LZ4F_BLOCK_LINKED: c_int = 0;
pub const LZ4F_BLOCK_INDEPENDENT: c_int = 1;
// LZ4F_contentChecksum_t
pub const LZ4F_NO_CONTENT_CHECKSUM: c_int = 0;
pub const LZ4F_CONTENT_CHECKSUM_ENABLED: c_int = 1;
// LZ4F_blockChecksum_t
pub const LZ4F_NO_BLOCK_CHECKSUM: c_int = 0;
pub const LZ4F_BLOCK_CHECKSUM_ENABLED: c_int = 1;
// LZ4F_frameType_t
pub const LZ4F_FRAME: c_int = 0;
pub const LZ4F_SKIPPABLE_FRAME: c_int = 1;

pub const LZ4F_ERROR_MAX_CODE: usize = 24;

/// Mirror of `LZ4F_isError`.
pub fn is_error(r: usize) -> bool {
    r > usize::MAX - LZ4F_ERROR_MAX_CODE
}

/// Recover the positive error code from an `LZ4F_*` return value.
pub fn err_code(r: usize) -> i64 {
    if is_error(r) {
        -(r as i64)
    } else {
        0
    }
}

/// Render an `LZ4F_*` return value for assertion messages.
pub fn show(r: usize) -> String {
    if is_error(r) {
        format!("ERR({})", err_code(r))
    } else {
        format!("ok({})", r)
    }
}

// ---------------------------------------------------------------------------
// function-pointer type aliases
// ---------------------------------------------------------------------------

pub type FnU = unsafe extern "C" fn() -> c_uint;
pub type FnI = unsafe extern "C" fn() -> c_int;
pub type FnStr = unsafe extern "C" fn() -> *const c_char;

// lz4.h — simple
pub type Fn_compressBound = unsafe extern "C" fn(c_int) -> c_int;
pub type Fn_compress_default =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_decompress_safe =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_compress_fast =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type Fn_compress_fast_extState =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type Fn_compress_destSize =
    unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
pub type Fn_compress_destSize_extState =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;
pub type Fn_decompress_safe_partial =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type Fn_decompress_fast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
pub type Fn_sizeofState = unsafe extern "C" fn() -> c_int;

// streaming (block)
pub type Fn_createStream = unsafe extern "C" fn() -> *mut c_void;
pub type Fn_freeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type Fn_initStream = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type Fn_resetStream = unsafe extern "C" fn(*mut c_void);
pub type Fn_loadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
pub type Fn_loadDict_internal =
    unsafe extern "C" fn(*mut c_void, *const c_char, c_int, c_int) -> c_int;
pub type Fn_attach_dictionary = unsafe extern "C" fn(*mut c_void, *const c_void);
pub type Fn_compress_fast_continue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type Fn_saveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
pub type Fn_createStreamDecode = unsafe extern "C" fn() -> *mut c_void;
pub type Fn_freeStreamDecode = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type Fn_setStreamDecode = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
pub type Fn_decompress_safe_continue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_decompress_fast_continue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
pub type Fn_decompress_usingDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    *const c_char,
    c_int,
) -> c_int;
pub type Fn_decompress_partial_usingDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_char,
    c_int,
) -> c_int;
pub type Fn_decompress_fast_usingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, *const c_char, c_int) -> c_int;
pub type Fn_decoderRingBufferSize = unsafe extern "C" fn(c_int) -> c_int;

// lz4hc.h
pub type Fn_compress_HC =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type Fn_compress_HC_extStateHC =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type Fn_compress_HC_destSize =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;
pub type Fn_resetStreamHC = unsafe extern "C" fn(*mut c_void, c_int);
pub type Fn_initStreamHC = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type Fn_loadDictHC = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
pub type Fn_compress_HC_continue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_compress_HC_continue_destSize =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
pub type Fn_saveDictHC = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
pub type Fn_setCompressionLevel = unsafe extern "C" fn(*mut c_void, c_int);
pub type Fn_favorDecompressionSpeed = unsafe extern "C" fn(*mut c_void, c_int);
pub type Fn_resetStreamStateHC = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;

// deprecated
pub type Fn_compress = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
pub type Fn_compress_limitedOutput =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_compress_withState =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
pub type Fn_compress_limitedOutput_withState =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_compress_continue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
pub type Fn_compress_limitedOutput_continue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_uncompress = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
pub type Fn_uncompress_unknownOutputSize =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_create = unsafe extern "C" fn(*mut c_char) -> *mut c_void;
pub type Fn_slideInputBuffer = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
pub type Fn_resetStreamState = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
pub type Fn_compressHC2 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_compressHC2_limitedOutput =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type Fn_compressHC_withStateHC =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
pub type Fn_compressHC_limitedOutput_withStateHC =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_compressHC2_withStateHC =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_compressHC2_limitedOutput_withStateHC =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type Fn_createHC = unsafe extern "C" fn(*const c_char) -> *mut c_void;
pub type Fn_freeHC = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type Fn_slideInputBufferHC = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
pub type Fn_compressHC2_continue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type Fn_compressHC2_limitedOutput_continue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type Fn_compress_forceExtDict =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
/// `typedef struct { int off; int len; int back; } LZ4HC_match_t;` (lz4hc.c:357)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LZ4HC_match_t {
    pub off: c_int,
    pub len: c_int,
    pub back: c_int,
}

/// `LZ4HC_match_t LZ4HC_searchExtDict(const BYTE* ip, U32 ipIndex,
///        const BYTE* iLowLimit, const BYTE* iHighLimit,
///        const LZ4HC_CCtx_internal* dictCtx, U32 gDictEndIndex,
///        int currentBestML, int nbAttempts)` (lz4hc.c:363)
pub type Fn_HC_searchExtDict = unsafe extern "C" fn(
    *const u8,    // ip
    u32,          // ipIndex
    *const u8,    // iLowLimit
    *const u8,    // iHighLimit
    *const c_void, // dictCtx (== &LZ4_streamHC_t.internal_donotuse, offset 0)
    u32,          // gDictEndIndex
    c_int,        // currentBestML
    c_int,        // nbAttempts
) -> LZ4HC_match_t;

// internal-but-exported entry points
pub type Fn_F_compressBegin_internal = unsafe extern "C" fn(
    *mut c_void, // cctx
    *mut c_void, // dst
    usize,
    *const c_void, // dictBuffer
    usize,         // dictSize
    *const c_void, // cdict
    *const LZ4F_preferences_t,
) -> usize;
pub type Fn_F_compressBegin_usingDictOnce = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
/// `int LZ4_decompress_safe_forceExtDict(const char*, char*, int, int, const void*, size_t)`
pub type Fn_decompress_safe_forceExtDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    *const c_void,
    usize,
) -> c_int;
/// `int LZ4_decompress_safe_partial_forceExtDict(const char*, char*, int, int, int, const void*, size_t)`
pub type Fn_decompress_safe_partial_forceExtDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_void,
    usize,
) -> c_int;
/// `int LZ4_decompress_safe_withPrefix64k(const char*, char*, int, int)`
pub type Fn_decompress_safe_withPrefix64k =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `int LZ4_decompress_fast_withPrefix64k(const char*, char*, int)`
pub type Fn_decompress_fast_withPrefix64k =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;

// lz4frame.h
pub type Fn_F_getVersion = unsafe extern "C" fn() -> c_uint;
pub type Fn_F_isError = unsafe extern "C" fn(usize) -> c_uint;
pub type Fn_F_getErrorName = unsafe extern "C" fn(usize) -> *const c_char;
pub type Fn_F_getErrorCode = unsafe extern "C" fn(usize) -> c_int;
pub type Fn_F_compressionLevel_max = unsafe extern "C" fn() -> c_int;
pub type Fn_F_compressFrameBound =
    unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
pub type Fn_F_compressFrame = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
pub type Fn_F_compressFrame_usingCDict = unsafe extern "C" fn(
    *mut c_void, // cctx
    *mut c_void, // dst
    usize,
    *const c_void, // src
    usize,
    *const c_void, // cdict
    *const LZ4F_preferences_t,
) -> usize;
pub type Fn_F_createCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
pub type Fn_F_createCDict_advanced =
    unsafe extern "C" fn(CustomMem, *const c_void, usize) -> *mut c_void;
pub type Fn_F_freeCDict = unsafe extern "C" fn(*mut c_void);
pub type Fn_F_createCompressionContext =
    unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
pub type Fn_F_createCompressionContext_advanced =
    unsafe extern "C" fn(CustomMem, c_uint) -> *mut c_void;
pub type Fn_F_freeCompressionContext = unsafe extern "C" fn(*mut c_void) -> usize;
pub type Fn_F_compressBegin = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
pub type Fn_F_compressBegin_usingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
pub type Fn_F_compressBegin_usingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
pub type Fn_F_compressBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
pub type Fn_F_compressUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
pub type Fn_F_uncompressedUpdate = Fn_F_compressUpdate;
pub type Fn_F_flush = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
pub type Fn_F_compressEnd = Fn_F_flush;
pub type Fn_F_getBlockSize = unsafe extern "C" fn(c_int) -> usize;
pub type Fn_F_createDecompressionContext =
    unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
pub type Fn_F_createDecompressionContext_advanced =
    unsafe extern "C" fn(CustomMem, c_uint) -> *mut c_void;
pub type Fn_F_freeDecompressionContext = unsafe extern "C" fn(*mut c_void) -> usize;
pub type Fn_F_headerSize = unsafe extern "C" fn(*const c_void, usize) -> usize;
pub type Fn_F_getFrameInfo = unsafe extern "C" fn(
    *mut c_void,
    *mut LZ4F_frameInfo_t,
    *const c_void,
    *mut usize,
) -> usize;
pub type Fn_F_decompress = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
pub type Fn_F_decompress_usingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const c_void,
    usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
pub type Fn_F_resetDecompressionContext = unsafe extern "C" fn(*mut c_void);

/// `LZ4F_CustomMem` from lz4frame.h (STATIC_LINKING_ONLY section).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CustomMem {
    pub customAlloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub customCalloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub customFree: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaqueState: *mut c_void,
}

impl Default for CustomMem {
    fn default() -> Self {
        CustomMem {
            customAlloc: None,
            customCalloc: None,
            customFree: None,
            opaqueState: std::ptr::null_mut(),
        }
    }
}

// lz4file.h
pub type Fn_F_readOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
pub type Fn_F_read = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
pub type Fn_F_readClose = unsafe extern "C" fn(*mut c_void) -> usize;
pub type Fn_F_writeOpen = unsafe extern "C" fn(
    *mut *mut c_void,
    *mut c_void,
    *const LZ4F_preferences_t,
) -> usize;
pub type Fn_F_write = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
pub type Fn_F_writeClose = unsafe extern "C" fn(*mut c_void) -> usize;

// xxhash.h (namespaced LZ4_)
pub type Fn_XXH_versionNumber = unsafe extern "C" fn() -> c_uint;
pub type Fn_XXH32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> u32;
pub type Fn_XXH64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
pub type Fn_XXH_createState = unsafe extern "C" fn() -> *mut c_void;
pub type Fn_XXH_freeState = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type Fn_XXH_copyState = unsafe extern "C" fn(*mut c_void, *const c_void);
pub type Fn_XXH32_reset = unsafe extern "C" fn(*mut c_void, c_uint) -> c_int;
pub type Fn_XXH64_reset = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
pub type Fn_XXH_update = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
pub type Fn_XXH32_digest = unsafe extern "C" fn(*const c_void) -> u32;
pub type Fn_XXH64_digest = unsafe extern "C" fn(*const c_void) -> u64;
pub type Fn_XXH32_canonicalFromHash = unsafe extern "C" fn(*mut c_void, u32);
pub type Fn_XXH64_canonicalFromHash = unsafe extern "C" fn(*mut c_void, u64);
pub type Fn_XXH32_hashFromCanonical = unsafe extern "C" fn(*const c_void) -> u32;
pub type Fn_XXH64_hashFromCanonical = unsafe extern "C" fn(*const c_void) -> u64;

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

pub fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    unsafe { std::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned()
}

/// Compare two byte slices and produce a short diagnostic on mismatch.
pub fn diff_report(ctx: &str, a: &[u8], b: &[u8]) -> Option<String> {
    if a == b {
        return None;
    }
    if a.len() != b.len() {
        return Some(format!(
            "{}: length differs, C={} Rust={}",
            ctx,
            a.len(),
            b.len()
        ));
    }
    let i = a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap();
    let lo = i.saturating_sub(8);
    let hi = (i + 8).min(a.len());
    Some(format!(
        "{}: first difference at byte {} (C={:#04x} Rust={:#04x})\n  C   [{}..{}] = {:02x?}\n  Rust[{}..{}] = {:02x?}",
        ctx, i, a[i], b[i], lo, hi, &a[lo..hi], lo, hi, &b[lo..hi]
    ))
}

#[macro_export]
macro_rules! assert_bytes_eq {
    ($ctx:expr, $a:expr, $b:expr) => {
        if let Some(m) = $crate::common::diff_report(&$ctx, &$a, &$b) {
            panic!("{}", m);
        }
    };
}
