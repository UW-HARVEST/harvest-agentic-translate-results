//! Bindings to the system zlib (libpng does not implement DEFLATE itself).
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

pub type Bytef = u8;
pub type uInt = c_uint;
pub type uLong = c_ulong;
pub type voidpf = *mut c_void;

pub type alloc_func = Option<unsafe extern "C" fn(voidpf, uInt, uInt) -> voidpf>;
pub type free_func = Option<unsafe extern "C" fn(voidpf, voidpf)>;

#[repr(C)]
pub struct z_stream {
    pub next_in: *const Bytef,
    pub avail_in: uInt,
    pub total_in: uLong,
    pub next_out: *mut Bytef,
    pub avail_out: uInt,
    pub total_out: uLong,
    pub msg: *const c_char,
    pub state: *mut c_void,
    pub zalloc: alloc_func,
    pub zfree: free_func,
    pub opaque: voidpf,
    pub data_type: c_int,
    pub adler: uLong,
    pub reserved: uLong,
}

impl Default for z_stream {
    fn default() -> Self {
        z_stream {
            next_in: core::ptr::null(),
            avail_in: 0,
            total_in: 0,
            next_out: core::ptr::null_mut(),
            avail_out: 0,
            total_out: 0,
            msg: core::ptr::null(),
            state: core::ptr::null_mut(),
            zalloc: None,
            zfree: None,
            opaque: core::ptr::null_mut(),
            data_type: 0,
            adler: 0,
            reserved: 0,
        }
    }
}

pub const Z_NO_FLUSH: c_int = 0;
pub const Z_PARTIAL_FLUSH: c_int = 1;
pub const Z_SYNC_FLUSH: c_int = 2;
pub const Z_FULL_FLUSH: c_int = 3;
pub const Z_FINISH: c_int = 4;
pub const Z_BLOCK: c_int = 5;
pub const Z_TREES: c_int = 6;

pub const Z_OK: c_int = 0;
pub const Z_STREAM_END: c_int = 1;
pub const Z_NEED_DICT: c_int = 2;
pub const Z_ERRNO: c_int = -1;
pub const Z_STREAM_ERROR: c_int = -2;
pub const Z_DATA_ERROR: c_int = -3;
pub const Z_MEM_ERROR: c_int = -4;
pub const Z_BUF_ERROR: c_int = -5;
pub const Z_VERSION_ERROR: c_int = -6;

pub const Z_NO_COMPRESSION: c_int = 0;
pub const Z_BEST_SPEED: c_int = 1;
pub const Z_BEST_COMPRESSION: c_int = 9;
pub const Z_DEFAULT_COMPRESSION: c_int = -1;

pub const Z_FILTERED: c_int = 1;
pub const Z_HUFFMAN_ONLY: c_int = 2;
pub const Z_RLE: c_int = 3;
pub const Z_FIXED: c_int = 4;
pub const Z_DEFAULT_STRATEGY: c_int = 0;

pub const Z_BINARY: c_int = 0;
pub const Z_TEXT: c_int = 1;
pub const Z_UNKNOWN: c_int = 2;

pub const Z_DEFLATED: c_int = 8;

pub const MAX_WBITS: c_int = 15;
pub const MAX_MEM_LEVEL: c_int = 9;

extern "C" {
    pub fn zlibVersion() -> *const c_char;
    pub fn deflateInit2_(
        strm: *mut z_stream,
        level: c_int,
        method: c_int,
        windowBits: c_int,
        memLevel: c_int,
        strategy: c_int,
        version: *const c_char,
        stream_size: c_int,
    ) -> c_int;
    pub fn deflate(strm: *mut z_stream, flush: c_int) -> c_int;
    pub fn deflateEnd(strm: *mut z_stream) -> c_int;
    pub fn deflateReset(strm: *mut z_stream) -> c_int;
    pub fn deflateBound(strm: *mut z_stream, sourceLen: uLong) -> uLong;
    pub fn inflateInit_(strm: *mut z_stream, version: *const c_char, stream_size: c_int) -> c_int;
    pub fn inflateInit2_(
        strm: *mut z_stream,
        windowBits: c_int,
        version: *const c_char,
        stream_size: c_int,
    ) -> c_int;
    pub fn inflate(strm: *mut z_stream, flush: c_int) -> c_int;
    pub fn inflateEnd(strm: *mut z_stream) -> c_int;
    pub fn inflateReset(strm: *mut z_stream) -> c_int;
    pub fn inflateReset2(strm: *mut z_stream, windowBits: c_int) -> c_int;
    pub fn inflateValidate(strm: *mut z_stream, check: c_int) -> c_int;
    pub fn crc32(crc: uLong, buf: *const Bytef, len: uInt) -> uLong;
    pub fn adler32(adler: uLong, buf: *const Bytef, len: uInt) -> uLong;
}

#[inline]
pub unsafe fn deflateInit2(
    strm: *mut z_stream,
    level: c_int,
    method: c_int,
    window_bits: c_int,
    mem_level: c_int,
    strategy: c_int,
) -> c_int {
    deflateInit2_(
        strm,
        level,
        method,
        window_bits,
        mem_level,
        strategy,
        zlibVersion(),
        core::mem::size_of::<z_stream>() as c_int,
    )
}

#[inline]
pub unsafe fn inflateInit(strm: *mut z_stream) -> c_int {
    inflateInit_(
        strm,
        zlibVersion(),
        core::mem::size_of::<z_stream>() as c_int,
    )
}

#[inline]
pub unsafe fn inflateInit2(strm: *mut z_stream, window_bits: c_int) -> c_int {
    inflateInit2_(
        strm,
        window_bits,
        zlibVersion(),
        core::mem::size_of::<z_stream>() as c_int,
    )
}

/// ZLIB_VERNUM of the zlib libpng was configured against.  The reference build
/// used zlib >= 1.2.11.
pub const ZLIB_VERNUM: c_int = 0x12b0;

/// `(uInt)-1`
pub const ZLIB_IO_MAX: uInt = uInt::MAX;

#[allow(unused)]
fn _unused(_: c_long) {}
