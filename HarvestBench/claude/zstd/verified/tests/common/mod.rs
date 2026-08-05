//! Shared test harness: loads BOTH the C .so and the Rust .so via libloading
//! and exposes symbol lookup so tests can call each side through the FFI
//! boundary and compare byte-for-byte.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_uint, c_void};

pub const C_SO: &str = "c_src/build/libzstd.so";
pub const RUST_SO: &str = "target/debug/libzstd.so";

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            let c = Library::new(C_SO).expect("load C .so");
            let rust = Library::new(RUST_SO).expect("load Rust .so");
            Libs { c, rust }
        }
    }
}

/// Deterministic xorshift RNG for reproducible property-style tests.
pub struct Rng(pub u64);
impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15).max(1))
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % ((hi - lo) as u64)) as i64
    }
    pub fn fill_random(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u32() as u8;
        }
    }
    /// Fill with compressible-ish data (limited alphabet + runs).
    pub fn fill_compressible(&mut self, buf: &mut [u8]) {
        let mut i = 0;
        while i < buf.len() {
            let run = 1 + (self.next_u32() % 16) as usize;
            let val = (self.next_u32() % 8) as u8;
            for _ in 0..run {
                if i >= buf.len() {
                    break;
                }
                buf[i] = val;
                i += 1;
            }
        }
    }
}

// ---- Function-pointer type aliases for symbols we call in both libs. ----

pub type FnCompressBound = unsafe extern "C" fn(usize) -> usize;
pub type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
pub type FnGetErrorName = unsafe extern "C" fn(usize) -> *const c_char;
pub type FnGetErrorCode = unsafe extern "C" fn(usize) -> c_int;
pub type FnGetErrorString = unsafe extern "C" fn(c_int) -> *const c_char;
pub type FnVersionNumber = unsafe extern "C" fn() -> c_uint;
pub type FnVersionString = unsafe extern "C" fn() -> *const c_char;
pub type FnClevel = unsafe extern "C" fn() -> c_int;
pub type FnSizeVoid = unsafe extern "C" fn() -> usize;

pub type FnCompress = unsafe extern "C" fn(
    dst: *mut c_void,
    dst_cap: usize,
    src: *const c_void,
    src_size: usize,
    level: c_int,
) -> usize;
pub type FnDecompress = unsafe extern "C" fn(
    dst: *mut c_void,
    dst_cap: usize,
    src: *const c_void,
    src_size: usize,
) -> usize;
pub type FnGetFrameContentSize = unsafe extern "C" fn(*const c_void, usize) -> u64;
pub type FnGetDecompressedSize = unsafe extern "C" fn(*const c_void, usize) -> u64;
pub type FnFindFrameCompressedSize = unsafe extern "C" fn(*const c_void, usize) -> usize;
pub type FnDecompressBound = unsafe extern "C" fn(*const c_void, usize) -> u64;

pub type FnCreateCtx = unsafe extern "C" fn() -> *mut c_void;
pub type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> usize;
pub type FnCompressCCtx = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    c_int,
) -> usize;
pub type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
pub type FnSetParameter = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
pub type FnCctxReset = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
pub type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZstdBounds {
    pub error: usize,
    pub lower: c_int,
    pub upper: c_int,
}
pub type FnGetBounds = unsafe extern "C" fn(c_int) -> ZstdBounds;

#[repr(C)]
pub struct ZstdInBuffer {
    pub src: *const c_void,
    pub size: usize,
    pub pos: usize,
}
#[repr(C)]
pub struct ZstdOutBuffer {
    pub dst: *mut c_void,
    pub size: usize,
    pub pos: usize,
}
pub type FnCompressStream2 = unsafe extern "C" fn(
    *mut c_void,
    *mut ZstdOutBuffer,
    *mut ZstdInBuffer,
    c_int,
) -> usize;
pub type FnDecompressStream =
    unsafe extern "C" fn(*mut c_void, *mut ZstdOutBuffer, *mut ZstdInBuffer) -> usize;

pub type FnXxh64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
pub type FnXxh32 = unsafe extern "C" fn(*const c_void, usize, u32) -> u32;

pub fn sym<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    unsafe { lib.get(name).unwrap_or_else(|e| panic!("symbol {:?}: {}", String::from_utf8_lossy(name), e)) }
}

// Parameter enum constants (from zstd.h)
pub const ZSTD_C_COMPRESSION_LEVEL: c_int = 100;
pub const ZSTD_C_WINDOWLOG: c_int = 101;
pub const ZSTD_C_HASHLOG: c_int = 102;
pub const ZSTD_C_CHAINLOG: c_int = 103;
pub const ZSTD_C_SEARCHLOG: c_int = 104;
pub const ZSTD_C_MINMATCH: c_int = 105;
pub const ZSTD_C_TARGETLENGTH: c_int = 106;
pub const ZSTD_C_STRATEGY: c_int = 107;
pub const ZSTD_C_ENABLE_LDM: c_int = 160;
pub const ZSTD_C_CONTENTSIZEFLAG: c_int = 200;
pub const ZSTD_C_CHECKSUMFLAG: c_int = 201;
pub const ZSTD_C_DICTIDFLAG: c_int = 202;

pub const ZSTD_D_WINDOWLOGMAX: c_int = 100;

pub const ZSTD_E_CONTINUE: c_int = 0;
pub const ZSTD_E_FLUSH: c_int = 1;
pub const ZSTD_E_END: c_int = 2;

pub const ZSTD_RESET_SESSION_ONLY: c_int = 1;
pub const ZSTD_RESET_PARAMETERS: c_int = 2;
pub const ZSTD_RESET_SESSION_AND_PARAMETERS: c_int = 3;

pub const CONTENTSIZE_UNKNOWN: u64 = 0u64.wrapping_sub(1);
pub const CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);
