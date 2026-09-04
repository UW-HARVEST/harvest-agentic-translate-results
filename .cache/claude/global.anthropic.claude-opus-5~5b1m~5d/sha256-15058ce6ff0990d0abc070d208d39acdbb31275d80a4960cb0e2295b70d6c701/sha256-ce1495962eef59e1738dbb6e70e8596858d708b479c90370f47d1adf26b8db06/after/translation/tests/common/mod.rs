//! Shared differential-test harness.
//!
//! Loads BOTH the C `liblz4.so` and the Rust `liblz4.so` through `libloading`
//! and exposes them as raw `extern "C"` function pointers, so every call goes
//! through the real dynamic-symbol export path (this also tests the
//! `#[no_mangle]` wrappers).

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    lib: libloading::Library,
    pub name: &'static str,
}

impl Lib {
    /// Fetch a symbol as a raw function pointer. Panics with the symbol name on
    /// failure so a missing export is reported clearly.
    pub fn sym<T: Copy>(&self, name: &str) -> T {
        assert_eq!(
            std::mem::size_of::<T>(),
            std::mem::size_of::<*const c_void>(),
            "sym::<T>() must be used with a function pointer type"
        );
        unsafe {
            let s: libloading::Symbol<T> = self
                .lib
                .get(name.as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol `{}`: {}", self.name, name, e));
            *s
        }
    }

    pub fn has(&self, name: &str) -> bool {
        unsafe {
            self.lib
                .get::<*const c_void>(name.as_bytes())
                .is_ok()
        }
    }
}

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.push("c_src");
    p.push("build");
    p.push("liblz4.so");
    p
}

fn rust_so_path() -> PathBuf {
    // current_exe = target/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().unwrap().to_path_buf(); // deps/
    if dir.file_name().map(|f| f == "deps").unwrap_or(false) {
        dir.pop();
    }
    let cand = dir.join("liblz4.so");
    if cand.exists() {
        return cand;
    }
    // fall back to the release build
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("release");
    p.push("liblz4.so");
    p
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "C shared library not found at {:?}", cp);
        assert!(rp.exists(), "Rust shared library not found at {:?}", rp);
        // RTLD_LOCAL (libloading's default) keeps the two symbol sets separate
        // even though the names are identical.
        let c = unsafe { libloading::Library::new(&cp) }.expect("dlopen C lib");
        let r = unsafe { libloading::Library::new(&rp) }.expect("dlopen Rust lib");
        Pair {
            c: Lib { lib: c, name: "C" },
            r: Lib { lib: r, name: "RUST" },
        }
    })
}

// ---------------------------------------------------------------------------
// C type mirrors
// ---------------------------------------------------------------------------

pub const LZ4F_VERSION: c_uint = 100;
pub const LZ4F_HEADER_SIZE_MIN: usize = 7;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;
pub const LZ4F_BLOCK_HEADER_SIZE: usize = 4;
pub const LZ4F_BLOCK_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_CONTENT_CHECKSUM_SIZE: usize = 4;
pub const LZ4_MAX_INPUT_SIZE: usize = 0x7E00_0000;
pub const LZ4_MEMORY_USAGE: u32 = 14;
pub const LZ4HC_CLEVEL_MIN: c_int = 2;
pub const LZ4HC_CLEVEL_DEFAULT: c_int = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: c_int = 10;
pub const LZ4HC_CLEVEL_MAX: c_int = 12;
pub const LZ4_STREAMSIZE: usize = 16416;
pub const LZ4_STREAMHCSIZE: usize = 262200;
pub const LZ4_STREAMDECODESIZE: usize = 32;

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

/// `LZ4F_errorCodes` enum values, in declaration order.
pub const LZ4F_ERROR_NAMES: &[&str] = &[
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
];

pub fn err_code_of(ret: usize) -> isize {
    -(ret as isize)
}

/// Human readable rendering of an LZ4F return value (for assert messages).
pub fn fmt_lz4f(ret: usize) -> String {
    let code = err_code_of(ret);
    if (1..=(LZ4F_ERROR_NAMES.len() as isize)).contains(&code) {
        format!("{} (=LZ4F_{}) [raw {:#x}]", ret as i64, LZ4F_ERROR_NAMES[code as usize], ret)
    } else {
        format!("{}", ret)
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG + data generators
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            lo
        } else {
            lo + self.below(hi - lo)
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
    pub fn fill(&mut self, out: &mut [u8]) {
        let mut i = 0;
        while i < out.len() {
            let x = self.next_u64().to_le_bytes();
            let take = std::cmp::min(8, out.len() - i);
            out[i..i + take].copy_from_slice(&x[..take]);
            i += take;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// uniformly random bytes — essentially incompressible
    Random,
    /// long runs of a single byte — extremely compressible
    Runs,
    /// small alphabet, word-like structure — realistically compressible
    Text,
    /// repeating periodic pattern — exercises long matches / offsets
    Periodic(usize),
    /// all zeroes
    Zeroes,
    /// mixture of the above, concatenated
    Mixed,
}

pub const ALL_SHAPES: &[Shape] = &[
    Shape::Random,
    Shape::Runs,
    Shape::Text,
    Shape::Periodic(7),
    Shape::Periodic(255),
    Shape::Zeroes,
    Shape::Mixed,
];

pub fn gen(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    // NOTE: always reserve at least one byte so that `as_ptr()` is a real
    // allocation even for len == 0. A dangling (0x1) pointer would make the C
    // library's `iend - MFLIMIT` pointer arithmetic underflow, which is UB in
    // C too — not a behaviour worth diffing.
    let mut v = Vec::with_capacity(len.max(64));
    match shape {
        Shape::Random => {
            v.resize(len, 0);
            rng.fill(&mut v);
        }
        Shape::Zeroes => v.resize(len, 0),
        Shape::Runs => {
            while v.len() < len {
                let b = rng.next_u32() as u8;
                let n = rng.range(1, 300).min(len - v.len());
                for _ in 0..n {
                    v.push(b);
                }
            }
        }
        Shape::Text => {
            const WORDS: &[&str] = &[
                "the ", "quick ", "brown ", "fox ", "jumps ", "over ", "lazy ", "dog ", "lz4 ",
                "compression ", "algorithm ", "block ", "frame ", "stream ", "dictionary ",
                "a ", "of ", "and ", "\n",
            ];
            while v.len() < len {
                let w = WORDS[rng.below(WORDS.len())].as_bytes();
                let take = std::cmp::min(w.len(), len - v.len());
                v.extend_from_slice(&w[..take]);
            }
        }
        Shape::Periodic(p) => {
            let period: Vec<u8> = (0..p.max(1)).map(|_| rng.next_u32() as u8).collect();
            while v.len() < len {
                let take = std::cmp::min(period.len(), len - v.len());
                v.extend_from_slice(&period[..take]);
            }
        }
        Shape::Mixed => {
            let sub = [Shape::Runs, Shape::Random, Shape::Text, Shape::Periodic(13)];
            while v.len() < len {
                let s = sub[rng.below(sub.len())];
                let n = rng.range(1, 1 + (len - v.len()).min(4096));
                let part = gen(s, n, rng);
                v.extend_from_slice(&part);
            }
        }
    }
    v.truncate(len);
    v
}

/// Sizes that straddle every interesting internal boundary of lz4.
pub const BOUNDARY_SIZES: &[usize] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14, 15, 16, 17, 19, 20, 31, 32, 33, 63, 64, 65, 127,
    128, 129, 255, 256, 511, 512, 1023, 1024, 4095, 4096, 65535, 65536, 65537,
];

pub fn hexdump(b: &[u8]) -> String {
    let n = b.len().min(64);
    let mut s = String::new();
    for x in &b[..n] {
        s.push_str(&format!("{:02x}", x));
    }
    if b.len() > n {
        s.push_str("...");
    }
    s
}

/// Assert two byte buffers are identical, printing the first difference.
pub fn assert_bytes_eq(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let at = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(c.len().min(r.len()));
    panic!(
        "{}: byte mismatch\n  C   len={} {}\n  RUST len={} {}\n  first diff at {}",
        ctx,
        c.len(),
        hexdump(c),
        r.len(),
        hexdump(r),
        at
    );
}

// ---------------------------------------------------------------------------
// common function pointer typedefs
// ---------------------------------------------------------------------------

pub type FnCompressDefault =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type FnCompressFast =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type FnDecompressSafe =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type FnDecompressPartial =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type FnCompressBound = unsafe extern "C" fn(c_int) -> c_int;
pub type FnVoidPtr = unsafe extern "C" fn() -> *mut c_void;
pub type FnFreePtr = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type FnIntVoid = unsafe extern "C" fn() -> c_int;
pub type FnUIntVoid = unsafe extern "C" fn() -> c_uint;
pub type FnStrVoid = unsafe extern "C" fn() -> *const c_char;

pub unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
}
